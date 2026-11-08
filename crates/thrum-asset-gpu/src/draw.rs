//! Generation-bound pipelines and per-set bind groups for drawing cooked products in the scene.

use thrum_render::contribution::{
    ContributionFrame, SceneShadowSurface, SceneSurface, ShadowBounds,
};

use std::cell::Cell;

use crate::load::ResidentSet;
use crate::material::{self, Target};
use crate::{CookedMaterial, MeshPlacement, VolumePlacement};

const CULL_BYTES: usize = 160;
const VOLUME_BYTES: usize = 144;
const DRAW_ARGS_BYTES: u64 = 20;

/// Pipelines and fixed resources for one scene surface.
pub(crate) struct Pipelines {
    device: wgpu::Device,
    queue: wgpu::Queue,
    mesh: wgpu::RenderPipeline,
    mesh_has_hook: bool,
    receiver: Option<wgpu::RenderPipeline>,
    shadow: Option<wgpu::RenderPipeline>,
    volume: wgpu::RenderPipeline,
    volume_layout: wgpu::BindGroupLayout,
    material_layout: wgpu::BindGroupLayout,
    cull: wgpu::ComputePipeline,
    cull_layout: wgpu::BindGroupLayout,
    defaults: [wgpu::TextureView; 3],
    sampler: wgpu::Sampler,
    volume_sampler: wgpu::Sampler,
}

/// Per-set GPU bindings, rebuilt whenever the set or the surface changes.
pub(crate) struct Bound {
    meshes: Vec<BoundMesh>,
    pub(crate) shadow_bounds: Option<ShadowBounds>,
    volume: Option<(wgpu::BindGroup, wgpu::Buffer)>,
    /// Meshlets the last cull kept (one `u32`), copied out for [`crate::CookedStats`] and
    /// cleared after each frame's cull.
    pub(crate) visible: wgpu::Buffer,
    /// This set was drawn in the last submitted frame, so its geometry has motion history.
    pub(crate) continued: Cell<bool>,
}

impl Bound {
    /// Whether a frame drawing this set dispatches any cull work.
    pub(crate) fn culls(&self) -> bool {
        !self.meshes.is_empty()
    }

    /// Meshlets across every placed mesh.
    pub(crate) fn meshlets(&self) -> u32 {
        self.meshes.iter().map(|mesh| mesh.meshlets).sum()
    }
}

struct BoundMesh {
    material: wgpu::BindGroup,
    cull: wgpu::BindGroup,
    cull_uniform: wgpu::Buffer,
    draws: wgpu::Buffer,
    instance: wgpu::Buffer,
    model: glam::Mat4,
    max_scale: f32,
    /// Whether the instance buffer's history flag currently says "valid".
    history: Cell<bool>,
    meshlets: u32,
    vertex: [wgpu::Buffer; 2],
    indices: wgpu::Buffer,
    shadow_indices: u32,
}

fn texture_entry(
    binding: u32,
    dimension: wgpu::TextureViewDimension,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: dimension,
            multisampled: false,
        },
        count: None,
    }
}

fn storage_entry(binding: u32, read_only: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn uniform_entry(binding: u32, visibility: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

/// One 1×1 texel used for the normal, ORM and emissive classes the cooked material leaves unset.
fn default_texel(device: &wgpu::Device, queue: &wgpu::Queue, texel: [u8; 4]) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("cooked default texel"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        texture.as_image_copy(),
        &texel,
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
}

/// Model matrix and history flag of one placement.
pub(crate) const INSTANCE_BYTES: u64 = 80;

/// The ABI revision 2 material record (80 bytes) of one placement: its base factor times the
/// cooked base-colour texture, its roughness and metalness, no emission, full occlusion.
fn material_record(material: &CookedMaterial) -> [u8; thrum_render::abi::MATERIAL_BYTES] {
    let [r, g, b, a] = material.base_colour;
    let floats: [f32; 12] = [
        r,
        g,
        b,
        a,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
        material.roughness,
        material.metalness,
        1.0,
    ];
    let mut bytes = [0; thrum_render::abi::MATERIAL_BYTES];
    bytes[..48].copy_from_slice(bytemuck::cast_slice(&floats));
    bytes
}

impl Pipelines {
    /// Pipelines for `surface`, drawing the built-in material. An accepted material hook is
    /// restored by [`crate::MaterialReload`] once it validates against the surface.
    pub(crate) fn new(surface: &SceneSurface<'_>, target: &Target) -> Self {
        let device = surface.device;
        let source = format!("{}\n{}", surface.shader_source, include_str!("cooked.wgsl"));
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cooked scene entries"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let depth = |write| {
            Some(wgpu::DepthStencilState {
                format: surface.depth_format,
                depth_write_enabled: Some(write),
                depth_compare: Some(wgpu::CompareFunction::Greater),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            })
        };
        let mesh = material::mesh_pipeline(target, &module);
        let receiver = surface.receiver.map(|receiver| {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("cooked Sun receiver geometry"),
                bind_group_layouts: &[Some(receiver.view_layout), Some(surface.light_layout)],
                immediate_size: 0,
            });
            material::mesh_pipeline_for(
                target,
                &module,
                &layout,
                receiver.fragment_entry,
                &[Some(material::mesh_color(receiver.guide_format))],
            )
        });
        let volume_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("cooked volume"),
            entries: &[
                texture_entry(8, wgpu::TextureViewDimension::D3),
                wgpu::BindGroupLayoutEntry {
                    binding: 9,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                uniform_entry(10, wgpu::ShaderStages::VERTEX_FRAGMENT),
            ],
        });
        let volume_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("cooked volume"),
                bind_group_layouts: &[
                    Some(surface.view_layout),
                    Some(surface.light_layout),
                    Some(&volume_layout),
                ],
                immediate_size: 0,
            });
        let volume = device.render(&wgpu::RenderPipelineDescriptor {
            label: Some("cooked volume"),
            layout: Some(&volume_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("cooked_volume_vertex"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Front),
                ..Default::default()
            },
            depth_stencil: depth(false),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("cooked_volume_fragment"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[
                    Some(wgpu::ColorTargetState {
                        format: surface.color_formats[0],
                        blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    Some(wgpu::ColorTargetState {
                        format: surface.color_formats[1],
                        blend: None,
                        write_mask: wgpu::ColorWrites::empty(),
                    }),
                ],
            }),
            multiview_mask: None,
            cache: None,
        });
        let cull_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("cooked meshlet cull"),
            entries: &[
                uniform_entry(0, wgpu::ShaderStages::COMPUTE),
                storage_entry(1, true),
                storage_entry(2, true),
                storage_entry(3, false),
                storage_entry(4, false),
            ],
        });
        let cull_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cooked meshlet cull"),
            source: wgpu::ShaderSource::Wgsl(include_str!("cull.wgsl").into()),
        });
        let cull_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("cooked meshlet cull"),
            bind_group_layouts: &[Some(&cull_layout)],
            immediate_size: 0,
        });
        let cull = device.compute(&wgpu::ComputePipelineDescriptor {
            label: Some("cooked meshlet cull"),
            layout: Some(&cull_pipeline_layout),
            module: &cull_module,
            entry_point: Some("cull_meshlets"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let defaults = [
            [128, 128, 255, 255],
            [255, 255, 255, 255],
            [255, 255, 255, 255],
        ]
        .map(|texel| default_texel(device, surface.queue, texel));
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("cooked material"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            anisotropy_clamp: 8,
            ..Default::default()
        });
        let volume_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("cooked volume"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Self {
            device: wgpu::Device::clone(device),
            queue: surface.queue.clone(),
            mesh,
            mesh_has_hook: false,
            receiver,
            shadow: surface
                .shadow
                .as_ref()
                .map(|shadow| shadow_pipeline(device, shadow)),
            volume,
            volume_layout,
            material_layout: surface.material_layout.clone(),
            cull,
            cull_layout,
            defaults,
            sampler,
            volume_sampler,
        }
    }

    /// Draw cooked meshes with a promoted replacement from now on.
    pub(crate) fn install_mesh(&mut self, mesh: wgpu::RenderPipeline) {
        self.mesh = mesh;
        self.mesh_has_hook = true;
    }

    fn buffer(&self, label: &str, size: u64, usage: wgpu::BufferUsages) -> wgpu::Buffer {
        self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size,
            usage: usage | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    /// Bind a resident set's resources for this surface.
    pub(crate) fn bind(
        &self,
        set: &ResidentSet,
        placements: &[MeshPlacement],
        volume: Option<&VolumePlacement>,
    ) -> Bound {
        let visible = self.buffer(
            "cooked visible meshlets",
            4,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        );
        let mut shadow_bounds: Option<ShadowBounds> = None;
        let meshes = set
            .meshes
            .iter()
            .zip(placements)
            .map(|((mesh, texture), placement)| {
                let base = texture.texture.create_view(&wgpu::TextureViewDescriptor {
                    dimension: Some(wgpu::TextureViewDimension::D2Array),
                    ..Default::default()
                });
                let material_buffer = self.buffer(
                    "cooked material",
                    thrum_render::abi::MATERIAL_BYTES as u64,
                    wgpu::BufferUsages::STORAGE,
                );
                self.queue
                    .write_buffer(&material_buffer, 0, &material_record(&placement.material));
                let material = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("cooked material"),
                    layout: &self.material_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: material_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&base),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::TextureView(&self.defaults[0]),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::TextureView(&self.defaults[1]),
                        },
                        wgpu::BindGroupEntry {
                            binding: 4,
                            resource: wgpu::BindingResource::TextureView(&self.defaults[2]),
                        },
                        wgpu::BindGroupEntry {
                            binding: 5,
                            resource: wgpu::BindingResource::Sampler(&self.sampler),
                        },
                    ],
                });
                let model = crate::placement::placed_model(placement, &mesh.rest_points);
                let instance = self.buffer(
                    "cooked instance",
                    INSTANCE_BYTES,
                    wgpu::BufferUsages::VERTEX,
                );
                // History flag (byte 64) starts at zero: the first submitted frame has no history.
                self.queue
                    .write_buffer(&instance, 0, bytemuck::cast_slice(&model.to_cols_array()));
                let cull_uniform = self.buffer(
                    "cooked cull",
                    CULL_BYTES as u64,
                    wgpu::BufferUsages::UNIFORM,
                );
                let draws = self.buffer(
                    "cooked meshlet draws",
                    u64::from(mesh.meshlet_count.max(1)) * DRAW_ARGS_BYTES,
                    wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::INDIRECT,
                );
                let cull = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("cooked meshlet cull"),
                    layout: &self.cull_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: cull_uniform.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: mesh.bounds.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: mesh.ranges.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: draws.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 4,
                            resource: visible.as_entire_binding(),
                        },
                    ],
                });
                let (scale, _, _) = model.to_scale_rotation_translation();
                let shadow_indices = if self.shadow.is_some()
                    && mesh.index_count != 0
                    && let Some((min, max)) = mesh.caster_box
                    && let Some(bounds) = world_caster_bounds(model, min, max)
                {
                    shadow_bounds = Some(shadow_bounds.map_or(bounds, |all| all.union(bounds)));
                    mesh.index_count
                } else {
                    0
                };
                BoundMesh {
                    material,
                    cull,
                    cull_uniform,
                    draws,
                    instance,
                    model,
                    max_scale: scale.abs().max_element(),
                    history: Cell::new(false),
                    meshlets: mesh.meshlet_count,
                    vertex: [mesh.positions.clone(), mesh.attributes.clone()],
                    indices: mesh.indices.clone(),
                    shadow_indices,
                }
            })
            .collect();
        let volume = set.volume.as_ref().zip(volume).map(|(texture, placement)| {
            let view = texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let uniform = self.buffer(
                "cooked volume",
                VOLUME_BYTES as u64,
                wgpu::BufferUsages::UNIFORM,
            );
            let model = glam::Mat4::from_cols_array_2d(&placement.transform);
            let mut bytes = Vec::with_capacity(VOLUME_BYTES);
            bytes.extend_from_slice(bytemuck::cast_slice(&model.to_cols_array()));
            bytes.extend_from_slice(bytemuck::cast_slice(&model.inverse().to_cols_array()));
            bytes.extend_from_slice(bytemuck::cast_slice(&[
                placement.extinction,
                placement.steps as f32,
                placement.light_steps as f32,
                0.0,
            ]));
            self.queue.write_buffer(&uniform, 0, &bytes);
            let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("cooked volume"),
                layout: &self.volume_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 8,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 9,
                        resource: wgpu::BindingResource::Sampler(&self.volume_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 10,
                        resource: uniform.as_entire_binding(),
                    },
                ],
            });
            (group, uniform)
        });
        Bound {
            meshes,
            shadow_bounds,
            volume,
            visible,
            continued: Cell::new(false),
        }
    }

    /// Cull every placed mesh's meshlets for this frame's camera into `pass`, and set each
    /// instance's history flag: valid only when it was drawn in the previous submitted frame
    /// (`continued`) and the scene keeps its view history. Queue writes land with the next
    /// submission even when this frame is discarded; they derive only from committed state and
    /// this frame's view, and the next recording rewrites the cull uniform, so a discarded frame
    /// changes nothing the next frame draws. Returns the flag.
    pub(crate) fn cull(
        &self,
        bound: &Bound,
        pass: &mut wgpu::ComputePass<'_>,
        frame: &ContributionFrame,
    ) -> bool {
        let continued = bound.continued.get() && !frame.history_reset;
        pass.set_pipeline(&self.cull);
        for mesh in &bound.meshes {
            if mesh.history.get() != continued {
                self.queue
                    .write_buffer(&mesh.instance, 64, &u32::from(continued).to_le_bytes());
                mesh.history.set(continued);
            }
            let mut bytes = [0u8; CULL_BYTES];
            let view_proj = glam::Mat4::from_cols_array_2d(&frame.view_proj);
            bytes[..64].copy_from_slice(bytemuck::cast_slice(&view_proj.to_cols_array()));
            bytes[64..128].copy_from_slice(bytemuck::cast_slice(&mesh.model.to_cols_array()));
            let camera = [
                frame.camera[0],
                frame.camera[1],
                frame.camera[2],
                mesh.max_scale,
            ];
            bytes[128..144].copy_from_slice(bytemuck::cast_slice(&camera));
            bytes[144..148].copy_from_slice(&mesh.meshlets.to_le_bytes());
            self.queue.write_buffer(&mesh.cull_uniform, 0, &bytes);
            pass.set_bind_group(0, &mesh.cull, &[]);
            pass.dispatch_workgroups(mesh.meshlets.div_ceil(64), 1, 1);
        }
        continued
    }

    pub(crate) fn draw_shadow(&self, bound: &Bound, pass: &mut wgpu::RenderPass<'_>) {
        let Some(pipeline) = &self.shadow else { return };
        pass.set_pipeline(pipeline);
        for mesh in &bound.meshes {
            if mesh.shadow_indices == 0 {
                continue;
            }
            pass.set_vertex_buffer(0, mesh.vertex[0].slice(..));
            pass.set_vertex_buffer(1, mesh.instance.slice(..));
            pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
            // Camera meshlet culling cannot determine visibility from the Sun.
            pass.draw_indexed(0..mesh.shadow_indices, 0, 0..1);
        }
    }

    /// Draw meshes with the scene's fragment (through `mesh` when a replacement is drawn this
    /// frame), then the volume. Scene groups 0, 1 and 3 are bound.
    pub(crate) fn draw(
        &self,
        bound: &Bound,
        pass: &mut wgpu::RenderPass<'_>,
        mesh: Option<&wgpu::RenderPipeline>,
    ) {
        Self::draw_meshes(bound, pass, mesh.unwrap_or(&self.mesh), true);
        if let Some((group, _)) = &bound.volume {
            pass.set_pipeline(&self.volume);
            pass.set_bind_group(2, group, &[]);
            pass.draw(0..36, 0..1);
        }
    }

    pub(crate) fn supports_receivers(&self) -> bool {
        !self.mesh_has_hook && self.receiver.is_some()
    }

    pub(crate) fn draw_receivers(&self, bound: &Bound, pass: &mut wgpu::RenderPass<'_>) {
        if let Some(receiver) = self.receiver.as_ref() {
            Self::draw_meshes(bound, pass, receiver, false);
        }
    }

    fn draw_meshes(
        bound: &Bound,
        pass: &mut wgpu::RenderPass<'_>,
        pipeline: &wgpu::RenderPipeline,
        bind_material: bool,
    ) {
        if !bound.meshes.is_empty() {
            pass.set_pipeline(pipeline);
        }
        for mesh in &bound.meshes {
            if bind_material {
                pass.set_bind_group(2, &mesh.material, &[]);
            }
            pass.set_vertex_buffer(0, mesh.vertex[0].slice(..));
            pass.set_vertex_buffer(1, mesh.vertex[1].slice(..));
            pass.set_vertex_buffer(2, mesh.instance.slice(..));
            pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.multi_draw_indexed_indirect(&mesh.draws, 0, mesh.meshlets);
        }
    }
}

fn world_caster_bounds(
    model: glam::Mat4,
    min: glam::Vec3,
    max: glam::Vec3,
) -> Option<ShadowBounds> {
    let mut low = glam::Vec3::splat(f32::INFINITY);
    let mut high = glam::Vec3::splat(f32::NEG_INFINITY);
    for corner in 0..8 {
        let local = glam::Vec3::new(
            if corner & 1 == 0 { min.x } else { max.x },
            if corner & 2 == 0 { min.y } else { max.y },
            if corner & 4 == 0 { min.z } else { max.z },
        );
        let world = model.transform_point3(local);
        if !world.is_finite() {
            return None;
        }
        low = low.min(world);
        high = high.max(world);
    }
    ShadowBounds::new(low.to_array(), high.to_array())
}

fn shadow_pipeline(
    device: &thrum_render::runtime::DeviceSite,
    surface: &SceneShadowSurface<'_>,
) -> wgpu::RenderPipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("cooked Sun casters"),
        bind_group_layouts: &[Some(surface.view_layout)],
        immediate_size: 0,
    });
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("cooked Sun casters"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shadow.wgsl").into()),
    });
    device.render(&wgpu::RenderPipelineDescriptor {
        label: Some("cooked Sun casters"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vertex"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[
                Some(wgpu::VertexBufferLayout {
                    array_stride: 12,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3],
                }),
                Some(wgpu::VertexBufferLayout {
                    array_stride: INSTANCE_BYTES,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4],
                }),
            ],
        },
        // Cooked terrain is an open heightfield. Near or below the horizon its
        // Sun-facing faces are grazing slivers that rasterize with holes, while
        // the faces a ray enters through face away from the Sun. Both bound the
        // solid, so draw both.
        primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: surface.depth_format,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Greater),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: None,
        multiview_mask: None,
        cache: None,
    })
}
