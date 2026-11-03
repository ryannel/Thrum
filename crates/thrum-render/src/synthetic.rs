//! Prepared in-memory instanced diagnostic. It exercises the real material and
//! instance tables, four texture-class bindings, graph declaration and optional
//! timestamps. One directional metallic-roughness closure and opaque depth are
//! real; clustered lights, shadows, HDR/display and cooked products remain unbuilt.

use crate::abi::{self, AbiError, InstanceRecord, MaterialRecord, TextureClasses};
use crate::graph::{
    Access, AccessKind, CompiledGraph, FrameGraph, GraphError, Pass, PassScope, PipelineKey,
    ResourceDescriptor, ResourceScope,
};
use crate::timing::{GpuTimings, TimingError};
use std::collections::BTreeSet;
use std::sync::mpsc::{self, Receiver, TryRecvError};

/// Synthetic source revision; not a substitute for an F4 product identity.
pub const SYNTHETIC_REVISION: u64 = 2;
/// Portable opaque depth; no sampling or separate prepass is needed.
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;
const VIEW_BYTES: usize = 160;
const LIGHT_BYTES: usize = 48;
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
    /// renders all cubes. World coordinates also form the fixed orthographic clip
    /// grid; +Z points away from the camera and depth lies in [0,1].
    /// # Errors
    /// Refuses zero instances or more than one million diagnostic instances.
    pub fn grid(count: u32) -> Result<Self, RenderError> {
        if count == 0 || count > 1_000_000 {
            return Err(RenderError::SceneCapacity);
        }
        let side = (count - 1).isqrt() + 1;
        let cell = 1.8 / side as f32;
        let materials = vec![
            MaterialRecord {
                base_rgba: [0.8, 0.2, 0.1, 1.0],
                ..Default::default()
            },
            MaterialRecord {
                base_rgba: [0.1, 0.5, 0.9, 1.0],
                surface: [1.0, 0.3, 0.2, 1.0],
                ..Default::default()
            },
        ];
        let instances = (0..count)
            .map(|index| {
                let mut instance = InstanceRecord::identity(index, index % 2);
                // A fixed orthonormal rotation exposes three differently lit faces.
                let (sx, cx) = 0.35_f32.sin_cos();
                let (sy, cy) = 0.5_f32.sin_cos();
                let columns = [
                    [cy, 0.0, -sy],
                    [sy * sx, cx, cy * sx],
                    [sy * cx, -sx, cy * cx],
                ];
                let scale = (cell * 0.3).min(0.3);
                for (axis, column) in columns.into_iter().enumerate() {
                    for (row, value) in column.into_iter().enumerate() {
                        instance.current[axis][row] = value * scale;
                        instance.normal[axis][row] = value / scale;
                    }
                }
                instance.current[3][2] = 0.5;
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

/// Bounded in-memory directional illumination, in linear working RGB. Direction
/// points from the receiver toward the source; irradiance is for a perpendicular
/// receiver, before cosine. No physical-world light registry is created here.
#[derive(Clone, Copy, Debug)]
pub struct SyntheticLight {
    /// Nonzero finite direction; normalized once during preparation.
    pub direction_to_light: [f32; 3],
    /// Finite nonnegative RGB illuminance, each at most 10,000 diagnostic units.
    pub irradiance: [f32; 3],
}
impl Default for SyntheticLight {
    fn default() -> Self {
        Self {
            direction_to_light: [0.4, -0.6, -1.0],
            irradiance: [4.0; 3],
        }
    }
}
impl SyntheticLight {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "finite normalized source invariants intentionally round once to the f32 GPU ABI"
    )]
    fn pack(self) -> Result<[u8; LIGHT_BYTES], RenderError> {
        let length = self
            .direction_to_light
            .iter()
            .map(|v| f64::from(*v).powi(2))
            .sum::<f64>()
            .sqrt();
        if !length.is_finite()
            || length == 0.0
            || !self
                .irradiance
                .iter()
                .all(|v| v.is_finite() && (0.0..=10_000.0).contains(v))
        {
            return Err(RenderError::InvalidLighting);
        }
        let direction = self
            .direction_to_light
            .map(|value| f64::from(value) / length);
        // Orthographic receiver-to-camera direction is fixed at -Z, so the
        // half-vector and Schlick angular power are invariant across fragments.
        // f64 preparation avoids f32 squared-length underflow near opposition.
        let half = [direction[0], direction[1], direction[2] - 1.0];
        let half_length = half.iter().map(|v| v * v).sum::<f64>().sqrt();
        let half = if half_length > 0.0 {
            half.map(|v| v / half_length)
        } else {
            [0.0, 0.0, -1.0]
        };
        let angular_fresnel = (1.0 + half[2]).clamp(0.0, 1.0).powi(5) as f32;
        let mut bytes = [0; LIGHT_BYTES];
        for i in 0..3 {
            bytes[i * 4..i * 4 + 4].copy_from_slice(&(direction[i] as f32).to_le_bytes());
            bytes[32 + i * 4..36 + i * 4].copy_from_slice(&(half[i] as f32).to_le_bytes());
        }
        bytes[44..48].copy_from_slice(&angular_fresnel.to_le_bytes());
        for (i, value) in self.irradiance.into_iter().enumerate() {
            bytes[16 + i * 4..20 + i * 4].copy_from_slice(&value.to_le_bytes());
        }
        Ok(bytes)
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
    /// Invalid directional source or degenerate/unsupported diagnostic transform.
    InvalidLighting,
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
    view: wgpu::BindGroup,
    light: wgpu::BindGroup,
    materials: wgpu::BindGroup,
    instances: wgpu::BindGroup,
    material_bytes: u64,
    instance_bytes: u64,
    instance_count: u32,
}
impl PreparedSynthetic {
    /// Prepare on the V1 executor's existing device and upload queue. No private
    /// adapter, device, submission or cook authority is created here.
    /// # Errors
    /// Refuses invalid/unsupported scene data, device capacity or GPU validation failure.
    pub async fn prepare(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        generation: u64,
        scene: &SyntheticScene,
    ) -> Result<Self, RenderError> {
        Self::prepare_with_light(device, queue, generation, scene, SyntheticLight::default()).await
    }
    /// Prepare the same finite draw role with an explicit directional source.
    /// Zero irradiance disables direct lighting; emission remains independent.
    /// # Errors
    /// Refuses invalid lights/transforms/materials, device capacity or GPU validation failure.
    #[expect(
        clippy::too_many_lines,
        reason = "one finite pipeline preparation transaction keeps resources, layouts and its validation scope together"
    )]
    pub async fn prepare_with_light(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        generation: u64,
        scene: &SyntheticScene,
        light: SyntheticLight,
    ) -> Result<Self, RenderError> {
        let light_data = light.pack()?;
        let limits = device.limits();
        let classes = TextureClasses::defaults(&limits)?;
        let (material_bytes, instance_bytes) =
            abi::table_bytes(scene.materials.len(), scene.instances.len(), &limits)?;
        let material_count =
            u32::try_from(scene.materials.len()).map_err(|_| RenderError::SceneCapacity)?;
        let instance_count =
            u32::try_from(scene.instances.len()).map_err(|_| RenderError::SceneCapacity)?;
        let mut packed_materials = Vec::with_capacity(
            usize::try_from(material_bytes).map_err(|_| RenderError::SceneCapacity)?,
        );
        for material in &scene.materials {
            // This is a single opaque static closure, not admission for all PBR flags.
            if material.flags != [0; 4] || material.texture_layers != [0; 4] {
                return Err(RenderError::UnsupportedSyntheticMaterial);
            }
            packed_materials.extend_from_slice(&material.pack(&classes)?);
        }
        let mut packed_instances = Vec::with_capacity(
            usize::try_from(instance_bytes).map_err(|_| RenderError::SceneCapacity)?,
        );
        for instance in &scene.instances {
            if instance.ids[2] != 0 {
                return Err(RenderError::UnsupportedSyntheticMaterial);
            }
            // The bounded cube evaluator needs finite nondegenerate normal axes.
            // General affine/singular-transform policy belongs to cooked geometry.
            if instance.normal.iter().any(|column| {
                let length2 = column[..3]
                    .iter()
                    .map(|v| f64::from(*v).powi(2))
                    .sum::<f64>();
                !(1e-12..=1e12).contains(&length2)
            }) || instance
                .current
                .iter()
                .flatten()
                .any(|v| !v.is_finite() || v.abs() > 1e6)
                || instance.current[0][3] != 0.0
                || instance.current[1][3] != 0.0
                || instance.current[2][3] != 0.0
                || instance.current[3][3].to_bits() != 1.0_f32.to_bits()
            {
                return Err(RenderError::InvalidLighting);
            }
            packed_instances.extend_from_slice(&instance.pack(material_count)?);
        }
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let material_buffer =
            storage_buffer(device, queue, "synthetic materials", &packed_materials);
        let instance_buffer =
            storage_buffer(device, queue, "synthetic forward opaque", &packed_instances);
        // ABI2 ViewRecord: identity current/previous orthographic view projection,
        // camera at negative Z, unit pre-exposure; all unused padding stays zero.
        let mut view_data = [0_u8; VIEW_BYTES];
        for offset in [0, 64] {
            for diagonal in [0, 20, 40, 60] {
                view_data[offset + diagonal..offset + diagonal + 4]
                    .copy_from_slice(&1.0_f32.to_le_bytes());
            }
        }
        view_data[136..140].copy_from_slice(&(-3.0_f32).to_le_bytes());
        view_data[144..148].copy_from_slice(&1.0_f32.to_le_bytes());
        view_data[148..152].copy_from_slice(&1.0_f32.to_le_bytes());
        let (view_layout, view) = uniform_group(
            device,
            queue,
            "synthetic ABI2 view",
            &view_data,
            wgpu::ShaderStages::VERTEX,
        );
        let (light_layout, light) = uniform_group(
            device,
            queue,
            "synthetic directional source",
            &light_data,
            wgpu::ShaderStages::FRAGMENT,
        );
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
            label: Some("synthetic forward opaque"),
            layout: &instance_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: instance_buffer.as_entire_binding(),
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("synthetic ABI2"),
            bind_group_layouts: &[
                Some(&view_layout),
                Some(&light_layout),
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
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
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
            view,
            light,
            materials,
            instances,
            material_bytes,
            instance_bytes,
            instance_count,
        })
    }
    /// Prepared generation-local finite draw tuple.
    pub fn pipeline_key(&self) -> PipelineKey {
        self.key
    }
    /// Compile the actual draw + optional readback declaration. Allocation and
    /// graph analysis happen at preparation/target changes, not per instance.
    /// # Errors
    /// Refuses a stale target generation or invalid typed resource/pass declarations.
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
        for bytes in [VIEW_BYTES as u64, LIGHT_BYTES as u64] {
            let uniform = graph.resource(
                ResourceDescriptor::Buffer {
                    bytes,
                    usage: wgpu::BufferUsages::UNIFORM,
                },
                ResourceScope::Imported { initialized: true },
            )?;
            accesses.push(Access {
                resource: uniform,
                range: 0..bytes,
                kind: AccessKind::UniformRead,
                store: true,
            });
        }
        let depth = graph.resource(
            ResourceDescriptor::Texture {
                width: target.width,
                height: target.height,
                layers: 1,
                mips: 1,
                samples: 1,
                format: DEPTH_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            },
            ResourceScope::Imported { initialized: false },
        )?;
        accesses.push(Access {
            resource: depth,
            range: 0..1,
            kind: AccessKind::AttachmentClear,
            store: false,
        });
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
            name: "synthetic forward opaque".into(),
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
    /// # Errors
    /// Refuses a stale target/device generation or unavailable timestamp batch capacity.
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
            Some(timings) => timings.pass_writes(generation, "synthetic forward opaque")?,
            None => None,
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("synthetic forward opaque"),
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
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &target.depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: timestamps,
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.view, &[]);
        pass.set_bind_group(1, &self.light, &[]);
        pass.set_bind_group(2, &self.materials, &[]);
        pass.set_bind_group(3, &self.instances, &[]);
        pass.draw(0..36, 0..self.instance_count);
        Ok(())
    }
}
fn uniform_group(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &str,
    data: &[u8],
    visibility: wgpu::ShaderStages,
) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some(label),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(data.len() as u64),
            },
            count: None,
        }],
    });
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: data.len() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&buffer, 0, data);
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout: &layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }],
    });
    (layout, group)
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
    depth: wgpu::TextureView,
    width: u32,
    height: u32,
    generation: u64,
    row_bytes: u32,
}
impl OffscreenTarget {
    /// Allocate a nonempty bounded target on the executor's device.
    /// # Errors
    /// Refuses empty/overflowing targets or extents beyond the device limits.
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
        let depth = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("synthetic opaque depth"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default());
        Ok(Self {
            texture,
            view,
            depth,
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
    /// # Errors
    /// Refuses a pending readback or mismatched target generation/extent.
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
    /// # Errors
    /// Refuses stale generations, failed mapping or a disconnected completion callback.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_preparation_normalizes_extremes_and_refuses_bad_sources() {
        for direction in [[f32::MAX, 0.0, 0.0], [f32::MIN_POSITIVE, 0.0, 0.0]] {
            let bytes = SyntheticLight {
                direction_to_light: direction,
                ..Default::default()
            }
            .pack()
            .unwrap();
            assert_eq!(f32::from_le_bytes(bytes[..4].try_into().unwrap()), 1.0);
            assert_eq!(&bytes[12..16], &[0; 4]);
            assert_eq!(&bytes[28..32], &[0; 4]);
        }
        for direction in [[0.0; 3], [f32::NAN, 0.0, 1.0], [f32::INFINITY, 0.0, 1.0]] {
            assert!(matches!(
                SyntheticLight {
                    direction_to_light: direction,
                    ..Default::default()
                }
                .pack(),
                Err(RenderError::InvalidLighting)
            ));
        }
        for value in [-1.0, f32::NAN, f32::INFINITY, 10_001.0] {
            assert!(matches!(
                SyntheticLight {
                    irradiance: [value; 3],
                    ..Default::default()
                }
                .pack(),
                Err(RenderError::InvalidLighting)
            ));
        }
        assert!(
            SyntheticLight {
                irradiance: [0.0; 3],
                ..Default::default()
            }
            .pack()
            .is_ok()
        );
    }
}
