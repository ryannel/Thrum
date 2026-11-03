//! Prepared in-memory instanced diagnostic. It exercises the real material and
//! instance tables, four texture-class bindings, graph declaration and optional
//! timestamps. It deliberately makes no PBR, CSM or cooked-product claim.

use crate::abi::{self, AbiError, InstanceRecord, MaterialRecord, TextureClasses};
use crate::graph::{
    Access, AccessKind, CompiledGraph, FrameGraph, GraphError, Pass, PassScope, PipelineKey,
    ResourceDescriptor, ResourceScope,
};
use crate::timing::{GpuTimings, TimingError};
use std::collections::BTreeSet;
use std::sync::mpsc::{self, Receiver, TryRecvError};

/// Synthetic source revision; not a substitute for an F4 product identity.
pub const SYNTHETIC_REVISION: u64 = 1;
/// Explicit output encoding: linear unorm diagnostic colour, no display transform.
pub const OUTPUT_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// Immutable CPU scene, converted at prepare time rather than each draw.
#[derive(Clone, Debug)]
pub struct SyntheticScene {
    /// Nonempty storage-buffer material table.
    pub materials: Vec<MaterialRecord>,
    /// Nonempty indexed draw instances.
    pub instances: Vec<InstanceRecord>,
}
impl SyntheticScene {
    /// Generate a bounded regular grid. Instance IDs remain stable; one draw
    /// renders all quads. Extent is clip space for this diagnostic geometry.
    pub fn grid(count: u32) -> Result<Self, RenderError> {
        if count == 0 || count > 1_000_000 {
            return Err(RenderError::SceneCapacity);
        }
        let side = (f64::from(count).sqrt().ceil()) as u32;
        let cell = 1.8 / side as f32;
        let materials = vec![
            MaterialRecord {
                base_rgba: [0.8, 0.2, 0.1, 1.0],
                ..Default::default()
            },
            MaterialRecord {
                base_rgba: [0.1, 0.5, 0.9, 1.0],
                ..Default::default()
            },
        ];
        let instances = (0..count)
            .map(|index| {
                let mut instance = InstanceRecord::identity(index, index % 2);
                instance.current[0][0] = cell * 0.4;
                instance.current[1][1] = cell * 0.4;
                instance.current[3][0] = -0.9 + cell * (index % side) as f32 + cell * 0.5;
                instance.current[3][1] = -0.9 + cell * (index / side) as f32 + cell * 0.5;
                instance.previous = instance.current;
                instance
            })
            .collect();
        Ok(Self {
            materials,
            instances,
        })
    }
}

/// Expected failures before a view can be encoded or published ready.
#[derive(Debug)]
pub enum RenderError {
    /// ABI validation failed.
    Abi(AbiError),
    /// Typed graph failed validation.
    Graph(GraphError),
    /// Optional timestamp lifecycle failure.
    Timing(TimingError),
    /// Pipeline/resource creation was rejected by the device.
    Validation(String),
    /// Empty/unreasonably large synthetic fixture.
    SceneCapacity,
    /// Unsupported semantic flags, skinning or mapped normals in this diagnostic.
    UnsupportedSyntheticMaterial,
    /// Target/readback belongs to another generation or grid.
    StaleTarget,
    /// Target dimensions exceed device limits or overflow readback arithmetic.
    TargetExtent,
    /// Readback is already pending or failed.
    Readback,
}
impl From<AbiError> for RenderError {
    fn from(error: AbiError) -> Self {
        Self::Abi(error)
    }
}
impl From<GraphError> for RenderError {
    fn from(error: GraphError) -> Self {
        Self::Graph(error)
    }
}
impl From<TimingError> for RenderError {
    fn from(error: TimingError) -> Self {
        Self::Timing(error)
    }
}

/// Fully prepared closure for one opaque static diagnostic draw role. Required
/// pipeline creation finishes before this object is returned; encode never builds it.
pub struct PreparedSynthetic {
    key: PipelineKey,
    pipeline: wgpu::RenderPipeline,
    empty: wgpu::BindGroup,
    materials: wgpu::BindGroup,
    instances: wgpu::BindGroup,
    material_bytes: u64,
    instance_bytes: u64,
    instance_count: u32,
}
impl PreparedSynthetic {
    /// Prepare on the V1 executor's existing device and upload queue. No private
    /// adapter, device, submission or cook authority is created here.
    pub async fn prepare(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        generation: u64,
        scene: &SyntheticScene,
    ) -> Result<Self, RenderError> {
        let limits = device.limits();
        let classes = TextureClasses::defaults(&limits)?;
        let (material_bytes, instance_bytes) =
            abi::table_bytes(scene.materials.len(), scene.instances.len(), &limits)?;
        let mut packed_materials = Vec::with_capacity(material_bytes as usize);
        for material in &scene.materials {
            // This is a single opaque static closure, not admission for all PBR flags.
            if material.flags != [0; 4] || material.texture_layers != [0; 4] {
                return Err(RenderError::UnsupportedSyntheticMaterial);
            }
            packed_materials.extend_from_slice(&material.pack(&classes)?);
        }
        let mut packed_instances = Vec::with_capacity(instance_bytes as usize);
        for instance in &scene.instances {
            if instance.ids[2] != 0 {
                return Err(RenderError::UnsupportedSyntheticMaterial);
            }
            packed_instances.extend_from_slice(&instance.pack(scene.materials.len() as u32)?);
        }
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let material_buffer =
            storage_buffer(device, queue, "synthetic materials", &packed_materials);
        let instance_buffer =
            storage_buffer(device, queue, "synthetic instances", &packed_instances);
        let empty_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("reserved view/scene groups"),
            entries: &[],
        });
        let empty = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("reserved view/scene"),
            layout: &empty_layout,
            entries: &[],
        });
        let mut entries = vec![storage_layout(
            0,
            wgpu::ShaderStages::FRAGMENT,
            abi::MATERIAL_BYTES as u64,
        )];
        for binding in 1..=4 {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2Array,
                    multisampled: false,
                },
                count: None,
            });
        }
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: 5,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        });
        let material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ABI2 material table and classes"),
            entries: &entries,
        });
        let instance_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ABI2 instance table"),
            entries: &[storage_layout(
                0,
                wgpu::ShaderStages::VERTEX,
                abi::INSTANCE_BYTES as u64,
            )],
        });
        let views: Vec<wgpu::TextureView> = classes
            .arrays()
            .iter()
            .map(|array| {
                let texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("typed default texture class"),
                    size: wgpu::Extent3d {
                        width: 1,
                        height: 1,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: array.class.format(),
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                });
                queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    &array.class.default_texel(),
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(4),
                        rows_per_image: Some(1),
                    },
                    wgpu::Extent3d {
                        width: 1,
                        height: 1,
                        depth_or_array_layers: 1,
                    },
                );
                texture.create_view(&wgpu::TextureViewDescriptor {
                    dimension: Some(wgpu::TextureViewDimension::D2Array),
                    ..Default::default()
                })
            })
            .collect();
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
        let materials = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("synthetic material classes"),
            layout: &material_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: material_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&views[0]),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&views[1]),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&views[2]),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&views[3]),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        let instances = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("synthetic instances"),
            layout: &instance_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: instance_buffer.as_entire_binding(),
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("synthetic ABI2"),
            bind_group_layouts: &[
                Some(&empty_layout),
                Some(&empty_layout),
                Some(&material_layout),
                Some(&instance_layout),
            ],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("prepared synthetic shader ABI2"),
            source: wgpu::ShaderSource::Wgsl(include_str!("synthetic.wgsl").into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("prepared opaque synthetic role"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: OUTPUT_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        if let Some(error) = scope.pop().await {
            return Err(RenderError::Validation(error.to_string()));
        }
        Ok(Self {
            key: PipelineKey {
                device_generation: generation,
                revision: SYNTHETIC_REVISION,
                role: 0,
            },
            pipeline,
            empty,
            materials,
            instances,
            material_bytes,
            instance_bytes,
            instance_count: scene.instances.len() as u32,
        })
    }
    /// Prepared generation-local finite draw tuple.
    pub fn pipeline_key(&self) -> PipelineKey {
        self.key
    }
    /// Compile the actual draw + optional readback declaration. Allocation and
    /// graph analysis happen at preparation/target changes, not per instance.
    pub fn graph(
        &self,
        target: &OffscreenTarget,
        readback: bool,
    ) -> Result<CompiledGraph, RenderError> {
        if target.generation != self.key.device_generation {
            return Err(RenderError::StaleTarget);
        }
        let mut graph = FrameGraph::default();
        let material = graph.resource(
            ResourceDescriptor::Buffer {
                bytes: self.material_bytes,
                usage: wgpu::BufferUsages::STORAGE,
            },
            ResourceScope::Imported { initialized: true },
        )?;
        let instance = graph.resource(
            ResourceDescriptor::Buffer {
                bytes: self.instance_bytes,
                usage: wgpu::BufferUsages::STORAGE,
            },
            ResourceScope::Imported { initialized: true },
        )?;
        let mut accesses = vec![
            Access {
                resource: material,
                range: 0..self.material_bytes,
                kind: AccessKind::StorageRead,
                store: true,
            },
            Access {
                resource: instance,
                range: 0..self.instance_bytes,
                kind: AccessKind::StorageRead,
                store: true,
            },
        ];
        for class in TextureClasses::defaults(&wgpu::Limits::default())?.arrays() {
            let texture = graph.resource(
                ResourceDescriptor::Texture {
                    width: 1,
                    height: 1,
                    layers: 1,
                    mips: 1,
                    samples: 1,
                    format: class.class.format(),
                    usage: wgpu::TextureUsages::TEXTURE_BINDING,
                },
                ResourceScope::Imported { initialized: true },
            )?;
            accesses.push(Access {
                resource: texture,
                range: 0..1,
                kind: AccessKind::SampledRead,
                store: true,
            });
        }
        let output = graph.resource(
            ResourceDescriptor::Texture {
                width: target.width,
                height: target.height,
                layers: 1,
                mips: 1,
                samples: 1,
                format: OUTPUT_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            },
            ResourceScope::Imported { initialized: false },
        )?;
        accesses.push(Access {
            resource: output.clone(),
            range: 0..1,
            kind: AccessKind::AttachmentClear,
            store: true,
        });
        graph.pass(Pass {
            name: "synthetic instances".into(),
            scope: PassScope::View(1),
            pipeline: Some(self.key),
            accesses,
            external_side_effect: !readback,
        });
        if readback {
            let bytes = target.readback_bytes();
            let destination = graph.resource(
                ResourceDescriptor::Buffer {
                    bytes,
                    usage: wgpu::BufferUsages::COPY_DST,
                },
                ResourceScope::Imported { initialized: false },
            )?;
            graph.pass(Pass {
                name: "output readback".into(),
                scope: PassScope::View(1),
                pipeline: None,
                accesses: vec![
                    Access {
                        resource: output,
                        range: 0..1,
                        kind: AccessKind::CopyRead,
                        store: true,
                    },
                    Access {
                        resource: destination,
                        range: 0..bytes,
                        kind: AccessKind::CopyWrite,
                        store: true,
                    },
                ],
                external_side_effect: true,
            });
        }
        Ok(graph.compile(self.key.device_generation, &BTreeSet::from([self.key]))?)
    }
    /// Encode an already prepared draw. The caller owns submission and device
    /// polling. Skipping or failing this method cannot advance view histories.
    pub fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        generation: u64,
        target: &OffscreenTarget,
        timings: Option<&mut GpuTimings>,
    ) -> Result<(), RenderError> {
        if generation != self.key.device_generation || target.generation != generation {
            return Err(RenderError::StaleTarget);
        }
        let timestamps = match timings {
            Some(timings) => timings.pass_writes(generation, "synthetic instances")?,
            None => None,
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("synthetic instances"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target.view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.02,
                        g: 0.03,
                        b: 0.04,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            timestamp_writes: timestamps,
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.empty, &[]);
        pass.set_bind_group(1, &self.empty, &[]);
        pass.set_bind_group(2, &self.materials, &[]);
        pass.set_bind_group(3, &self.instances, &[]);
        pass.draw(0..6, 0..self.instance_count);
        Ok(())
    }
}
fn storage_layout(
    binding: u32,
    visibility: wgpu::ShaderStages,
    stride: u64,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: wgpu::BufferSize::new(stride),
        },
        count: None,
    }
}
fn storage_buffer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &str,
    data: &[u8],
) -> wgpu::Buffer {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: data.len() as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&buffer, 0, data);
    buffer
}

/// Generation-bound offscreen target. Retain while encoded/submitted work uses
/// it; actual old/new allocation retirement belongs to the executor's fences.
pub struct OffscreenTarget {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    width: u32,
    height: u32,
    generation: u64,
    row_bytes: u32,
}
impl OffscreenTarget {
    /// Allocate a nonempty bounded target on the executor's device.
    pub fn new(
        device: &wgpu::Device,
        generation: u64,
        width: u32,
        height: u32,
    ) -> Result<Self, RenderError> {
        let limits = device.limits();
        if width == 0
            || height == 0
            || width > limits.max_texture_dimension_2d
            || height > limits.max_texture_dimension_2d
        {
            return Err(RenderError::TargetExtent);
        }
        let row_bytes = width
            .checked_mul(4)
            .and_then(|value| value.checked_add(255))
            .map(|value| value / 256 * 256)
            .ok_or(RenderError::TargetExtent)?;
        if u64::from(row_bytes) * u64::from(height) > limits.max_buffer_size {
            return Err(RenderError::TargetExtent);
        }
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("synthetic offscreen target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: OUTPUT_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Ok(Self {
            texture,
            view,
            width,
            height,
            generation,
            row_bytes,
        })
    }
    /// Target extent in pixels.
    pub fn extent(&self) -> [u32; 2] {
        [self.width, self.height]
    }
    fn readback_bytes(&self) -> u64 {
        u64::from(self.row_bytes) * u64::from(self.height)
    }
    /// Allocate one bounded CPU readback slot for this exact generation/grid.
    pub fn readback(&self, device: &wgpu::Device) -> PixelReadback {
        PixelReadback {
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("synthetic pixel readback"),
                size: self.readback_bytes(),
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }),
            generation: self.generation,
            extent: self.extent(),
            row_bytes: self.row_bytes,
            pending: None,
        }
    }
}

/// One bounded asynchronous readback slot, reusable after collection.
pub struct PixelReadback {
    buffer: wgpu::Buffer,
    generation: u64,
    extent: [u32; 2],
    row_bytes: u32,
    pending: Option<Receiver<Result<(), wgpu::BufferAsyncError>>>,
}
impl PixelReadback {
    /// Encode a copy and map on submission; no CPU wait or private submission.
    pub fn encode(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &OffscreenTarget,
    ) -> Result<(), RenderError> {
        if self.pending.is_some() {
            return Err(RenderError::Readback);
        }
        if self.generation != target.generation || self.extent != target.extent() {
            return Err(RenderError::StaleTarget);
        }
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &target.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.row_bytes),
                    rows_per_image: Some(self.extent[1]),
                },
            },
            wgpu::Extent3d {
                width: self.extent[0],
                height: self.extent[1],
                depth_or_array_layers: 1,
            },
        );
        let (send, receive) = mpsc::channel();
        encoder.map_buffer_on_submit(&self.buffer, wgpu::MapMode::Read, .., move |result| {
            let _ = send.send(result);
        });
        self.pending = Some(receive);
        Ok(())
    }
    /// Nonblocking collection, removing row padding. The caller polls the device.
    pub fn try_collect(&mut self, generation: u64) -> Result<Option<Vec<u8>>, RenderError> {
        if self.generation != generation {
            return Err(RenderError::StaleTarget);
        }
        let Some(pending) = &self.pending else {
            return Ok(None);
        };
        match pending.try_recv() {
            Err(TryRecvError::Empty) => return Ok(None),
            Err(TryRecvError::Disconnected) | Ok(Err(_)) => {
                self.pending = None;
                return Err(RenderError::Readback);
            }
            Ok(Ok(())) => {}
        }
        self.pending = None;
        let mapped = self
            .buffer
            .slice(..)
            .get_mapped_range()
            .map_err(|_| RenderError::Readback)?;
        let mut pixels = Vec::with_capacity(self.extent[0] as usize * self.extent[1] as usize * 4);
        for row in mapped.chunks_exact(self.row_bytes as usize) {
            pixels.extend_from_slice(&row[..self.extent[0] as usize * 4]);
        }
        drop(mapped);
        self.buffer.unmap();
        Ok(Some(pixels))
    }
}
