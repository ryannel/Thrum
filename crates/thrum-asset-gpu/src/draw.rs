//! Generation-bound pipelines and per-set bind groups for drawing cooked products in the scene.

use thrum_render::binding;
use thrum_render::contribution::{
    ContributionFrame, SceneShadowSurface, SceneSurface, SecondaryShadowCascade, ShadowBounds,
};

use std::cell::Cell;

use crate::load::ResidentSet;
use crate::material::{MeshPipelines, Target};
use crate::{CookedDrawPath, CookedMaterial, MeshPlacement, VolumePlacement};

const CULL_BYTES: usize = size_of::<crate::records::Cull>();
const VOLUME_BYTES: usize = size_of::<crate::records::CookedVolume>();
pub(crate) const DRAW_ARGS_BYTES: u64 = 20;

mod secondary;
use secondary::SecondaryCull;

/// Pipelines and fixed resources for one scene surface.
pub(crate) struct Pipelines {
    #[cfg(feature = "draw-probe")]
    probe: Option<crate::draw_probe::Probe>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    mesh: MeshPipelines,
    stage_layout: Option<crate::mesh_stages::Layout>,
    stage_cull: bool,
    pub(crate) indexed_path: CookedDrawPath,
    shadow: Option<wgpu::RenderPipeline>,
    stage_shadow: Option<wgpu::RenderPipeline>,
    secondary_shadows: Vec<Option<wgpu::RenderPipeline>>,
    secondary_cull: Option<SecondaryCull>,
    volume: wgpu::RenderPipeline,
    volume_layout: wgpu::BindGroupLayout,
    material_layout: wgpu::BindGroupLayout,
    cull: Option<(wgpu::ComputePipeline, wgpu::BindGroupLayout)>,
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
    pub(crate) visible: Option<wgpu::Buffer>,
    /// This set was drawn in the last submitted frame, so its geometry has motion history.
    pub(crate) continued: Cell<bool>,
    draw_counts: (usize, usize, usize),
    indirect_meshlets: u32,
}

impl Bound {
    pub(crate) fn ray_instances(
        &self,
        instances: &mut thrum_render::contribution::RayInstances<'_>,
    ) {
        for mesh in &self.meshes {
            if let Some(blas) = &mesh.ray_mesh {
                let transform = std::array::from_fn(|index| mesh.model.row(index / 4)[index % 4]);
                instances.offer(blas, transform, 0b11);
            }
        }
    }

    /// Whether a frame drawing this set dispatches any cull work.
    pub(crate) fn culls(&self, views: &[thrum_render::contribution::SecondaryViewFrame]) -> bool {
        self.visible.is_some() || self.culls_views(views)
    }

    pub(crate) fn has_meshes(&self) -> bool {
        !self.meshes.is_empty()
    }

    #[cfg(feature = "draw-probe")]
    pub(crate) fn probe_resources(&self) -> (usize, bool) {
        let count = self
            .meshes
            .iter()
            .flat_map(|mesh| {
                mesh.indirect
                    .as_ref()
                    .map(|indirect| &indirect.draws)
                    .into_iter()
                    .chain(
                        mesh.probe
                            .as_ref()
                            .and_then(crate::draw_probe::Bound::indirect_buffer),
                    )
            })
            .filter(|buffer| buffer.usage().contains(wgpu::BufferUsages::INDIRECT))
            .count();
        (
            count,
            self.visible.is_some() || self.meshes.iter().any(|mesh| mesh.indirect.is_some()),
        )
    }

    #[cfg(feature = "draw-probe")]
    fn draw_residency(&self) -> crate::draw_probe::DrawResidency {
        let mut sizes = crate::draw_probe::DrawResidency {
            visibility: self.visible.as_ref().map_or(0, wgpu::Buffer::size),
            volume_uniform: self.volume.as_ref().map_or(0, |(_, buffer)| buffer.size()),
            ..Default::default()
        };
        for mesh in &self.meshes {
            sizes.instances += mesh.instance.size();
            sizes.materials += mesh.material_bytes;
            for indirect in mesh.indirect.iter().chain(&mesh.secondary) {
                sizes.indirect_arguments += indirect.draws.size();
                sizes.cull_uniforms += indirect.cull_uniform.size();
            }
            if let Some(buffer) = mesh
                .probe
                .as_ref()
                .and_then(crate::draw_probe::Bound::indirect_buffer)
            {
                sizes.indirect_arguments += buffer.size();
            }
        }
        sizes
    }

    #[cfg(feature = "draw-probe")]
    pub(crate) fn probe_commands(&self) -> Vec<String> {
        self.meshes
            .iter()
            .enumerate()
            .map(|(index, mesh)| {
                let command = if let Some(probe) = &mesh.probe {
                    probe.description()
                } else if mesh.stage.is_some() {
                    format!(
                        "draw_mesh_tasks x={}",
                        crate::mesh_stages::groups(mesh.meshlets)
                    )
                } else if mesh.indirect.is_some() {
                    format!(
                        "multi_draw_indexed_indirect offset=0 count={}",
                        mesh.meshlets
                    )
                } else {
                    format!(
                        "draw_indexed range=0..{} base_vertex=0 instances=0..1",
                        mesh.index_count
                    )
                };
                format!("placement {index}: {command}")
            })
            .collect()
    }

    /// Meshlets submitted to the main view's indexed compute cull.
    pub(crate) fn meshlets(&self) -> u32 {
        self.indirect_meshlets
    }

    pub(crate) fn draw_counts(&self) -> (usize, usize, usize) {
        self.draw_counts
    }
}

struct BoundMesh {
    ray_mesh: Option<wgpu::Blas>,
    #[cfg(feature = "draw-probe")]
    probe: Option<crate::draw_probe::Bound>,
    material: wgpu::BindGroup,
    #[cfg(feature = "draw-probe")]
    material_bytes: u64,
    stage: Option<wgpu::BindGroup>,
    indirect: Option<IndirectMesh>,
    secondary: Vec<IndirectMesh>,
    index_count: u32,
    instance: wgpu::Buffer,
    model: glam::Mat4,
    max_scale: f32,
    /// Whether the instance buffer's history flag currently says "valid".
    history: Cell<bool>,
    meshlets: u32,
    vertex: [wgpu::Buffer; 2],
    indices: wgpu::Buffer,
    caster_bounds: Option<ShadowBounds>,
}

struct IndirectMesh {
    cull: wgpu::BindGroup,
    cull_uniform: wgpu::Buffer,
    draws: wgpu::Buffer,
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
pub(crate) const INSTANCE_BYTES: u64 = size_of::<crate::records::Instance>() as u64;

/// The ABI revision 2 material record (80 bytes) of one placement: its base factor times the
/// cooked base-colour texture, its roughness and metalness, no emission, full occlusion.
fn material_record(material: &CookedMaterial) -> crate::records::Material {
    crate::records::Material {
        base_rgba: material.base_colour,
        emissive_cutoff: [0.0; 4],
        surface: [1.0, material.roughness, material.metalness, 1.0],
        flags: [0; 4],
        textures: [0; 4],
    }
}
impl Pipelines {
    /// Pipelines for the scene surface and its cooked contribution.
    pub(crate) fn new(
        surface: &SceneSurface<'_>,
        target: &mut Target,
        indexed_path: CookedDrawPath,
        config: &crate::CookedConfig,
        fallback: &mut Option<String>,
    ) -> Self {
        let device = surface.device;
        let cooked = cooked_source(surface.shader_source, surface.sun_optical_depth);
        let source = format!("{}\n{cooked}", surface.shader_source);
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cooked scene entries"),
            source: wgpu::ShaderSource::Wgsl(source.as_str().into()),
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
        let secondary_source = target.secondary_source.as_ref().map(|source| {
            let cooked = cooked_source(source, surface.sun_optical_depth);
            format!("{source}\n{cooked}")
        });
        let mut mesh = target.meshes(&module, secondary_source.as_deref());
        let mut stage_shadow = None;
        if config.draw_path == CookedDrawPath::MeshStages && fallback.is_none() {
            match crate::mesh_stages::prepare(surface, target, &source, config) {
                Ok(prepared) => {
                    mesh.receiver_stages = prepared.receiver;
                    target.stages = Some(prepared.layout);
                    mesh.stages = Some(prepared.forward);
                    stage_shadow = prepared.shadow;
                }
                Err(reason) => *fallback = Some(reason),
            }
        }
        let volume_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("cooked volume"),
            entries: &[
                binding::texture(
                    8,
                    wgpu::ShaderStages::FRAGMENT,
                    wgpu::TextureSampleType::Float { filterable: true },
                    wgpu::TextureViewDimension::D3,
                ),
                binding::sampler(
                    9,
                    wgpu::ShaderStages::FRAGMENT,
                    wgpu::SamplerBindingType::Filtering,
                ),
                binding::uniform(10, wgpu::ShaderStages::VERTEX_FRAGMENT, 0),
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
        let mut volume_targets = target.main.color_formats.clone();
        volume_targets[0]
            .as_mut()
            .expect("scene colour target")
            .blend = Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING);
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
                targets: &volume_targets,
            }),
            multiview_mask: None,
            cache: None,
        });
        let cull = (indexed_path == CookedDrawPath::Indirect).then(|| {
            let cull_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("cooked meshlet cull"),
                entries: &[
                    binding::uniform(0, wgpu::ShaderStages::COMPUTE, 0),
                    binding::storage(1, wgpu::ShaderStages::COMPUTE, true, 0),
                    binding::storage(2, wgpu::ShaderStages::COMPUTE, true, 0),
                    binding::storage(3, wgpu::ShaderStages::COMPUTE, false, 0),
                    binding::storage(4, wgpu::ShaderStages::COMPUTE, false, 0),
                ],
            });
            let cull_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("cooked meshlet cull"),
                source: wgpu::ShaderSource::Wgsl(crate::records::CULL_SHADER.into()),
            });
            let cull_pipeline_layout =
                device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
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
            (cull, cull_layout)
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
        let secondary_count = surface.secondary_surfaces.len();
        let secondary_cull = if CookedDrawPath::select(surface.downlevel)
            == CookedDrawPath::Indirect
            && secondary_count != 0
        {
            SecondaryCull::new(surface, secondary_count)
        } else {
            None
        };
        Self {
            #[cfg(feature = "draw-probe")]
            probe: None,
            device: wgpu::Device::clone(device),
            queue: surface.queue.clone(),
            mesh,
            stage_layout: target.stages.clone(),
            stage_cull: config.mesh_stage_cull,
            indexed_path,
            secondary_shadows: surface
                .secondary_surfaces
                .iter()
                .map(|view| {
                    view.shadow
                        .as_ref()
                        .map(|shadow| shadow_pipeline(device, shadow))
                })
                .collect(),
            secondary_cull,
            stage_shadow,
            shadow: surface
                .shadow
                .as_ref()
                .map(|shadow| shadow_pipeline(device, shadow)),
            volume,
            volume_layout,
            material_layout: surface.material_layout.clone(),
            cull,
            defaults,
            sampler,
            volume_sampler,
        }
    }

    pub(crate) fn uses_mesh_stages(&self) -> bool {
        self.stage_layout.is_some()
    }

    #[cfg(feature = "draw-probe")]
    pub(crate) fn probe(&mut self, probe: Option<crate::draw_probe::Probe>) {
        self.probe = probe;
    }

    /// Draw cooked meshes with a promoted replacement from now on.
    pub(crate) fn install_mesh(&mut self, mesh: MeshPipelines) {
        self.mesh = mesh;
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
        let visible = self
            .cull
            .as_ref()
            .filter(|_| {
                set.meshes.iter().any(|(mesh, _)| {
                    mesh.ranges.is_some()
                        && (self.stage_layout.is_none() || mesh.mesh_stage_streams.is_none())
                })
            })
            .map(|_| {
                self.buffer(
                    "cooked visible meshlets",
                    4,
                    wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                )
            });
        let mut shadow_bounds: Option<ShadowBounds> = None;
        let secondary_cull = self.secondary_cull.as_ref().filter(|cull| cull.admits(set));
        let mut draw_counts = (0, 0, 0);
        let mut indirect_meshlets = 0_u32;
        let meshes = set
            .meshes
            .iter()
            .zip(placements)
            .enumerate()
            .map(|(placement_index, ((mesh, texture), placement))| {
                #[cfg(not(feature = "draw-probe"))]
                let _ = placement_index;
                let base = texture.texture.create_view(&wgpu::TextureViewDescriptor {
                    dimension: Some(wgpu::TextureViewDimension::D2Array),
                    ..Default::default()
                });
                let material_buffer = self.buffer(
                    "cooked material",
                    thrum_render::abi::MATERIAL_BYTES as u64,
                    wgpu::BufferUsages::STORAGE,
                );
                self.queue.write_buffer(
                    &material_buffer,
                    0,
                    bytemuck::bytes_of(&material_record(&placement.material)),
                );
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
                let (scale, _, _) = model.to_scale_rotation_translation();
                let max_scale = scale.abs().max_element();
                let layout = self
                    .stage_layout
                    .as_ref()
                    .zip(mesh.mesh_stage_streams.as_ref());
                let instance = self.buffer(
                    "cooked instance",
                    INSTANCE_BYTES,
                    wgpu::BufferUsages::VERTEX
                        | if layout.is_some() {
                            wgpu::BufferUsages::UNIFORM
                        } else {
                            wgpu::BufferUsages::empty()
                        },
                );
                // History flag (byte 64) starts at zero: the first submitted frame has no history.
                self.queue.write_buffer(
                    &instance,
                    0,
                    bytemuck::bytes_of(&crate::records::Instance {
                        model: model.to_cols_array_2d(),
                        history: self.instance_draw(
                            layout.is_some(),
                            mesh.meshlet_count,
                            false,
                            max_scale,
                        ),
                    }),
                );
                let indirect = self
                    .cull
                    .as_ref()
                    .zip(visible.as_ref())
                    .filter(|_| layout.is_none() && mesh.ranges.is_some())
                    .map(|((_, layout), visible)| self.bind_cull(mesh, layout, 4, visible));
                let stage = layout.map(|(layout, streams)| {
                    layout.bind(
                        &self.device,
                        [
                            &instance,
                            mesh.bounds
                                .as_ref()
                                .expect("mesh stage loader stages meshlet bounds"),
                            &mesh.positions,
                            &mesh.attributes,
                            &streams[0],
                            &streams[1],
                            &streams[2],
                        ],
                    )
                });
                if stage.is_some() {
                    draw_counts.0 += 1;
                } else if indirect.is_some() {
                    draw_counts.1 += mesh.meshlet_count as usize;
                    indirect_meshlets = indirect_meshlets.saturating_add(mesh.meshlet_count);
                } else {
                    draw_counts.2 += 1;
                }
                let secondary = secondary_cull
                    .into_iter()
                    .flat_map(|cull| {
                        cull.records
                            .iter()
                            .map(|record| self.bind_cull(mesh, &cull.layout, 5, record))
                    })
                    .collect();
                let caster_bounds = if self.shadow.is_some()
                    && mesh.index_count != 0
                    && let Some((min, max)) = mesh.caster_box
                    && let Some(bounds) = world_caster_bounds(model, min, max)
                {
                    shadow_bounds = Some(shadow_bounds.map_or(bounds, |all| all.union(bounds)));
                    Some(bounds)
                } else {
                    None
                };
                BoundMesh {
                    ray_mesh: mesh.ray_mesh.as_ref().map(|mesh| mesh.blas.clone()),
                    #[cfg(feature = "draw-probe")]
                    probe: self.probe.as_ref().and_then(|probe| {
                        probe.bind(&self.device, &self.queue, placement_index, mesh.index_count)
                    }),
                    material,
                    #[cfg(feature = "draw-probe")]
                    material_bytes: material_buffer.size(),
                    stage,
                    indirect,
                    secondary,
                    index_count: mesh.index_count,
                    instance,
                    model,
                    max_scale,
                    history: Cell::new(false),
                    meshlets: mesh.meshlet_count,
                    vertex: [mesh.positions.clone(), mesh.attributes.clone()],
                    indices: mesh.indices.clone(),
                    caster_bounds,
                }
            })
            .collect();
        let volume = set
            .volume
            .as_ref()
            .zip(volume)
            .map(|(texture, placement)| self.bind_volume(texture, placement));
        Bound {
            meshes,
            shadow_bounds,
            volume,
            visible,
            continued: Cell::new(false),
            draw_counts,
            indirect_meshlets,
        }
    }

    fn instance_draw(&self, stages: bool, count: u32, history: bool, scale: f32) -> [u32; 4] {
        // The first word is also the indexed vertex history attribute.
        if stages {
            [
                u32::from(history),
                count,
                u32::from(self.stage_cull),
                scale.to_bits(),
            ]
        } else {
            [u32::from(history), 0, 0, 0]
        }
    }

    fn bind_cull(
        &self,
        mesh: &crate::load::GpuMesh,
        layout: &wgpu::BindGroupLayout,
        extra_binding: u32,
        extra: &wgpu::Buffer,
    ) -> IndirectMesh {
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
        let inputs = [
            &cull_uniform,
            mesh.bounds
                .as_ref()
                .expect("indirect loader stages meshlet bounds"),
            mesh.ranges
                .as_ref()
                .expect("indirect loader stages meshlet ranges"),
            &draws,
            extra,
        ];
        let bindings = [0, 1, 2, 3, extra_binding];
        let entries: [_; 5] = std::array::from_fn(|i| wgpu::BindGroupEntry {
            binding: bindings[i],
            resource: inputs[i].as_entire_binding(),
        });
        let cull = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("cooked meshlet cull"),
            layout,
            entries: &entries,
        });
        IndirectMesh {
            cull,
            cull_uniform,
            draws,
        }
    }

    fn bind_volume(
        &self,
        texture: &crate::load::GpuTexture,
        placement: &VolumePlacement,
    ) -> (wgpu::BindGroup, wgpu::Buffer) {
        let view = texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let uniform = self.buffer(
            "cooked volume",
            VOLUME_BYTES as u64,
            wgpu::BufferUsages::UNIFORM,
        );
        let model = glam::Mat4::from_cols_array_2d(&placement.transform);
        let record = crate::records::CookedVolume {
            model: model.to_cols_array_2d(),
            inverse_model: model.inverse().to_cols_array_2d(),
            params: [
                placement.extinction,
                placement.steps as f32,
                placement.light_steps as f32,
                0.0,
            ],
        };
        self.queue
            .write_buffer(&uniform, 0, bytemuck::bytes_of(&record));
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
    }

    /// History derives only from committed state, on both draw paths. Queue writes from a
    /// discarded recording are overwritten from that state by the next recording.
    pub(crate) fn history(&self, bound: &Bound, frame: &ContributionFrame) -> bool {
        let continued = bound.continued.get() && !frame.history_reset;
        for mesh in &bound.meshes {
            if mesh.history.get() != continued {
                self.queue.write_buffer(
                    &mesh.instance,
                    0,
                    bytemuck::bytes_of(&crate::records::Instance {
                        model: mesh.model.to_cols_array_2d(),
                        history: self.instance_draw(
                            mesh.stage.is_some(),
                            mesh.meshlets,
                            continued,
                            mesh.max_scale,
                        ),
                    }),
                );
                mesh.history.set(continued);
            }
        }
        continued
    }

    /// Record the unchanged meshlet cull on the indirect path only.
    pub(crate) fn cull(
        &self,
        bound: &Bound,
        pass: &mut wgpu::ComputePass<'_>,
        frame: &ContributionFrame,
    ) {
        let Some((pipeline, _)) = &self.cull else {
            return;
        };
        if bound.visible.is_none() {
            return;
        }
        pass.set_pipeline(pipeline);
        for mesh in &bound.meshes {
            let Some(indirect) = &mesh.indirect else {
                continue;
            };
            self.cull_mesh(pass, mesh, indirect, frame.view_proj, frame.camera);
        }
    }

    fn cull_mesh(
        &self,
        pass: &mut wgpu::ComputePass<'_>,
        mesh: &BoundMesh,
        indirect: &IndirectMesh,
        view_proj: [[f32; 4]; 4],
        [x, y, z]: [f32; 3],
    ) {
        if mesh.meshlets == 0 {
            return;
        }
        let record = crate::records::Cull {
            view_proj,
            model: mesh.model.to_cols_array_2d(),
            camera_scale: [x, y, z, mesh.max_scale],
            counts: [mesh.meshlets, 0, 0, 0],
        };
        self.queue
            .write_buffer(&indirect.cull_uniform, 0, bytemuck::bytes_of(&record));
        pass.set_bind_group(0, &indirect.cull, &[]);
        pass.dispatch_workgroups(mesh.meshlets.div_ceil(64), 1, 1);
    }

    #[cfg(feature = "draw-probe")]
    pub(crate) fn draw_residency(
        &self,
        bound: &Bound,
        set: &ResidentSet,
        readback: Option<&wgpu::Buffer>,
    ) -> crate::draw_probe::DrawResidency {
        let mut sizes = bound.draw_residency();
        sizes.cull_uniforms += self
            .secondary_cull
            .iter()
            .flat_map(|cull| &cull.records)
            .map(wgpu::Buffer::size)
            .sum::<u64>();
        sizes.visibility += readback.map_or(0, wgpu::Buffer::size);
        sizes.sampled_textures = self
            .defaults
            .iter()
            .map(wgpu::TextureView::texture)
            .chain(set.meshes.iter().map(|(_, texture)| &texture.texture))
            .chain(set.volume.iter().map(|volume| &volume.texture))
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .map(crate::draw_probe::texture_bytes)
            .sum();
        sizes
    }

    pub(crate) fn draw_shadow(&self, bound: &Bound, pass: &mut wgpu::RenderPass<'_>) -> usize {
        Self::draw_casters(
            bound,
            pass,
            self.shadow.as_ref(),
            self.stage_shadow.as_ref(),
            None,
        )
    }

    fn draw_casters(
        bound: &Bound,
        pass: &mut wgpu::RenderPass<'_>,
        pipeline: Option<&wgpu::RenderPipeline>,
        stage_pipeline: Option<&wgpu::RenderPipeline>,
        cascade: Option<SecondaryShadowCascade>,
    ) -> usize {
        let Some(pipeline) = pipeline else { return 0 };
        let mut stages = None;
        let mut draws = 0;
        for mesh in bound.meshes.iter().filter(|mesh| {
            mesh.caster_bounds
                .is_some_and(|bounds| cascade.is_none_or(|cascade| cascade.overlaps(bounds)))
        }) {
            draws += 1;
            let stage = mesh.stage.as_ref().zip(stage_pipeline);
            if stages != Some(stage.is_some()) {
                pass.set_pipeline(stage.map_or(pipeline, |(_, pipeline)| pipeline));
                stages = Some(stage.is_some());
            }
            if let Some((group, _)) = stage {
                crate::mesh_stages::draw(pass, group, mesh.meshlets, 0);
                continue;
            }
            pass.set_vertex_buffer(0, mesh.vertex[0].slice(..));
            pass.set_vertex_buffer(1, mesh.instance.slice(..));
            pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
            // Camera meshlet culling cannot determine visibility from the Sun.
            pass.draw_indexed(0..mesh.index_count, 0, 0..1);
        }
        draws
    }

    /// Draw meshes with the scene's fragment (through `mesh` when a replacement is drawn this
    /// frame), then the volume. Scene groups 0, 1 and 3 are bound.
    pub(crate) fn draw(
        &self,
        bound: &Bound,
        pass: &mut wgpu::RenderPass<'_>,
        mesh: Option<&MeshPipelines>,
        view_offset: u32,
    ) {
        let pipelines = mesh.unwrap_or(&self.mesh);
        Self::draw_meshes(
            bound,
            pass,
            &pipelines.main,
            pipelines.stages.as_ref(),
            view_offset,
        );
        if let Some((group, _)) = &bound.volume {
            pass.set_pipeline(&self.volume);
            pass.set_bind_group(2, group, &[]);
            pass.draw(0..36, 0..1);
        }
    }

    pub(crate) fn draw_ambient_receivers(
        &self,
        bound: &Bound,
        pass: &mut wgpu::RenderPass<'_>,
        mesh: Option<&MeshPipelines>,
        view_offset: u32,
    ) {
        let pipelines = mesh.unwrap_or(&self.mesh);
        if let Some(receiver) = &pipelines.receiver {
            Self::draw_meshes(
                bound,
                pass,
                receiver,
                pipelines.receiver_stages.as_ref(),
                view_offset,
            );
        }
    }

    fn draw_meshes(
        bound: &Bound,
        pass: &mut wgpu::RenderPass<'_>,
        pipeline: &wgpu::RenderPipeline,
        stage_pipeline: Option<&wgpu::RenderPipeline>,
        view_offset: u32,
    ) {
        let mut stages = None;
        for mesh in &bound.meshes {
            let stage = mesh.stage.as_ref().zip(stage_pipeline);
            if stages != Some(stage.is_some()) {
                pass.set_pipeline(stage.map_or(pipeline, |(_, pipeline)| pipeline));
                stages = Some(stage.is_some());
            }
            if let Some((group, _)) = stage {
                pass.set_bind_group(2, &mesh.material, &[]);
                crate::mesh_stages::draw(pass, group, mesh.meshlets, view_offset);
                continue;
            }
            mesh.bind_draw(pass);
            #[cfg(feature = "draw-probe")]
            if let Some(probe) = &mesh.probe {
                probe.draw(pass);
                continue;
            }
            mesh.draw(pass, mesh.indirect.as_ref());
        }
    }
}

impl BoundMesh {
    fn bind_draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_bind_group(2, &self.material, &[]);
        pass.set_vertex_buffer(0, self.vertex[0].slice(..));
        pass.set_vertex_buffer(1, self.vertex[1].slice(..));
        pass.set_vertex_buffer(2, self.instance.slice(..));
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
    }

    fn draw(&self, pass: &mut wgpu::RenderPass<'_>, indirect: Option<&IndirectMesh>) {
        if let Some(indirect) = indirect {
            pass.multi_draw_indexed_indirect(&indirect.draws, 0, self.meshlets);
        } else {
            pass.draw_indexed(0..self.index_count, 0, 0..1);
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
        primitive: caster_primitive(),
        depth_stencil: Some(caster_depth(surface)),
        multisample: wgpu::MultisampleState::default(),
        fragment: None,
        multiview_mask: None,
        cache: None,
    })
}

pub(crate) fn caster_primitive() -> wgpu::PrimitiveState {
    // At low Sun the terrain's Sun-facing faces are grazing slivers. The
    // faces aimed away from the Sun must also rasterize to close the caster.
    wgpu::PrimitiveState {
        cull_mode: None,
        ..Default::default()
    }
}

pub(crate) fn caster_depth(surface: &SceneShadowSurface<'_>) -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: surface.depth_format,
        depth_write_enabled: Some(true),
        depth_compare: Some(wgpu::CompareFunction::Greater),
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    }
}

pub(crate) fn cooked_vertex_source(scene: &str) -> String {
    let source = include_str!("cooked.wgsl");
    if scene.contains("fn shadow_tangent_surface(") {
        return source.replace(
            "    output.uv = uv;",
            "    output.shadow_surface = shadow_tangent_surface(model, position, output.normal);\n    output.uv = uv;",
        );
    }
    source.into()
}

/// The cooked entries for `scene`; with `cloud`, the volume's Sun term is also
/// multiplied by the scene's cloud factor.
///
/// # Panics
/// Panics if the cloud-factor variant's replacement does not match exactly once
/// in `cooked.wgsl`, which would silently draw volumes without cloud shadow.
fn cooked_source(scene: &str, cloud: bool) -> String {
    const SUN_TERM: &str = "light.irradiance.rgb * exp(-optical)";
    let source = cooked_vertex_source(scene);
    if cloud {
        assert_eq!(
            source.matches(SUN_TERM).count(),
            1,
            "the cooked volume's Sun term for the cloud-factor variant"
        );
        source.replacen(
            SUN_TERM,
            "light.irradiance.rgb * cloud_factor((cooked_volume.model * vec4(p, 1.0)).xyz) * exp(-optical)",
            1,
        )
    } else {
        source
    }
}

#[cfg(test)]
mod tests;
