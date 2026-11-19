use super::{
    Profile, SURFACE_PASS,
    interaction::{self, Interaction},
    map_gpu::{self, MapGpu},
    query::{self, QueryMailbox, Readback},
    records::Parameters,
};
use std::ops::Range;
use thrum_render::{
    atmosphere::EnvironmentProducts,
    contribution::{AfterOpaqueKind, AfterOpaquePass, EffectCadence, EffectGraph, SceneSurface},
};
use wgpu::util::DeviceExt;

pub(super) struct Prepared {
    pub(super) ambient: thrum_sea::Ambient,
    pub(super) ambient_passes: usize,
    pub(super) query: Option<Readback>,
    pub(super) interaction: Option<Interaction>,
    pub(super) environment: EnvironmentProducts,
    pub(super) pipeline: wgpu::RenderPipeline,
    pub(super) parameters: wgpu::Buffer,
    pub(super) group: wgpu::BindGroup,
    pub(super) mesh: wgpu::Buffer,
    pub(super) vertices: u32,
    pub(super) queue: wgpu::Queue,
    pub(super) map: Option<MapGpu>,
    pub(super) bodies: Vec<(String, u32)>,
    pub(super) indices: Option<wgpu::Buffer>,
    pub(super) draws: Vec<Range<u32>>,
}
fn declaration() -> AfterOpaquePass {
    AfterOpaquePass {
        name: SURFACE_PASS.into(),
        kind: AfterOpaqueKind::Render,
        cadence: EffectCadence::EveryFrame,
        accesses: Vec::new(),
        depth: false,
        write_depth: true,
        refraction: false,
    }
}
impl Prepared {
    pub(super) fn graph(&self) -> EffectGraph {
        let mut graph = self.ambient.graph();
        let wake = self
            .interaction
            .as_ref()
            .map(|interaction| interaction.declare(&mut graph));
        if self.query.is_some() {
            Readback::declare(&self.ambient, &mut graph, wake.clone());
        }
        let mut surface = declaration();
        surface.accesses = self.ambient.filtered_accesses();
        surface.accesses.extend(wake);
        graph.opaque_reads.clear();
        graph.after_opaque.push(surface);
        graph
    }
}
pub(super) fn new(
    surface: &SceneSurface<'_>,
    profile: Profile,
    map: &thrum_water_model::Map,
    mapped: bool,
    mailbox: Option<QueryMailbox>,
    interaction: Option<interaction::Mailbox>,
    previous_interaction: Option<interaction::Report>,
) -> Result<Prepared, String> {
    let device = surface.device;
    let map_gpu = mapped
        .then_some(map)
        .map(|map| MapGpu::new(device, surface.queue, map))
        .transpose()
        .map_err(|e| e.to_string())?;
    let ambient =
        thrum_sea::Ambient::new(device, profile.waves).map_err(|error| error.to_string())?;
    let ambient_passes = ambient.graph().passes.len();
    let interaction = interaction
        .map(|mailbox| Interaction::new(device, mailbox, previous_interaction.unwrap_or_default()));
    let query = if let Some(mailbox) = mailbox {
        query::validate(
            profile.query.expect("enabled query limits"),
            &surface.limits,
        )?;
        Some(Readback::new(
            device,
            &ambient,
            mailbox,
            interaction.as_ref(),
        ))
    } else {
        None
    };
    let environment = surface
        .environment_products()
        .ok_or("water needs the scene's physical environment")?;
    let parameters = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("water parameters"),
        size: size_of::<Parameters>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut entries: Vec<_> = (0..5)
        .map(|binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: if binding == 0 {
                wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                }
            } else if binding <= 2 {
                wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2Array,
                    multisampled: false,
                }
            } else {
                wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering)
            },
            count: None,
        })
        .collect();
    if mapped {
        entries.extend(map_gpu::bindings(wgpu::ShaderStages::VERTEX_FRAGMENT));
    }
    if interaction.is_some() {
        entries.extend(Interaction::sample_layout(
            7,
            wgpu::ShaderStages::VERTEX_FRAGMENT,
        ));
    }
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("water field"),
        entries: &entries,
    });
    let sampler = |mode| {
        device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("water anisotropic field"),
            address_mode_u: mode,
            address_mode_v: mode,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            anisotropy_clamp: if surface
                .downlevel
                .is_some_and(|flags| flags.contains(wgpu::DownlevelFlags::ANISOTROPIC_FILTERING))
            {
                16
            } else {
                1
            },
            ..Default::default()
        })
    };
    let repeat = sampler(wgpu::AddressMode::Repeat);
    let clamp = sampler(wgpu::AddressMode::ClampToEdge);
    let views = ambient.filtered_views();
    let mut bindings = vec![
        wgpu::BindGroupEntry {
            binding: 0,
            resource: parameters.as_entire_binding(),
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
            resource: wgpu::BindingResource::Sampler(&repeat),
        },
        wgpu::BindGroupEntry {
            binding: 4,
            resource: wgpu::BindingResource::Sampler(&clamp),
        },
    ];
    let map_view = map_gpu.as_ref().filter(|_| mapped).map(MapGpu::view);
    if let (Some(map), Some(view)) = (&map_gpu, &map_view) {
        bindings.extend(map.entries(view));
    }
    if let Some(interaction) = &interaction {
        bindings.extend(interaction.sample_entries(7));
    }
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("water field"),
        layout: &layout,
        entries: &bindings,
    });
    let inputs = declaration().input_layout(device);
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(SURFACE_PASS),
        bind_group_layouts: &[
            Some(surface.view_layout),
            Some(&inputs),
            Some(&environment.layout),
            Some(&layout),
        ],
        immediate_size: 0,
    });
    let pipeline = create_pipeline(
        surface,
        &pipeline_layout,
        &environment,
        mapped,
        interaction.is_some(),
    );
    let geometry = surface_mesh(mapped.then_some(map));
    let count = u32::try_from(geometry.vertices.len()).map_err(|e| e.to_string())?;
    let (mesh, indices) = geometry.upload(device)?;
    Ok(Prepared {
        ambient,
        ambient_passes,
        query,
        interaction,
        environment,
        pipeline,
        parameters,
        group,
        mesh,
        vertices: count,
        queue: surface.queue.clone(),
        map: map_gpu,
        bodies: geometry.bodies,
        draws: geometry.draws,
        indices,
    })
}

fn create_pipeline(
    surface: &SceneSurface<'_>,
    pipeline_layout: &wgpu::PipelineLayout,
    environment: &EnvironmentProducts,
    mapped: bool,
    wake: bool,
) -> wgpu::RenderPipeline {
    let device = surface.device;
    let source = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        environment.shader_source.replace("@group(0)", "@group(2)"),
        include_str!("fresnel.wgsl"),
        include_str!("sampling.wgsl"),
        Interaction::sample_source(3, 7, 8, wake),
        include_str!("surface.wgsl"),
        if mapped {
            map_gpu::declarations() + include_str!("mapped_surface.wgsl")
        } else {
            include_str!("open_surface.wgsl").to_owned()
        }
    );
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(SURFACE_PASS),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let targets: Vec<_> = surface
        .color_formats
        .iter()
        .enumerate()
        .map(|(i, f)| {
            Some(wgpu::ColorTargetState {
                format: *f,
                blend: (i == 0 && mapped).then_some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })
        })
        .collect();
    device.render(&wgpu::RenderPipelineDescriptor {
        label: Some(SURFACE_PASS),
        layout: Some(pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("water_vertex"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: 16,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 0,
                }],
            })],
        },
        primitive: wgpu::PrimitiveState {
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: surface.depth_format,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::GreaterEqual),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("water_fragment"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &targets,
        }),
        multiview_mask: None,
        cache: None,
    })
}

#[derive(Default)]
struct Geometry {
    vertices: Vec<[f32; 4]>,
    indices: Vec<u32>,
    bodies: Vec<(String, u32)>,
    draws: Vec<Range<u32>>,
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "GPU vertices and at most 16 body slots"
)]
fn surface_mesh(map: Option<&thrum_water_model::Map>) -> Geometry {
    let Some(map) = map else {
        return Geometry {
            vertices: mesh(),
            ..Geometry::default()
        };
    };
    let mut geometry = Geometry::default();
    for body in 0..map_gpu::BODY_LIMIT {
        let Some(declaration) = map.body(body) else {
            break;
        };
        let first = geometry.vertices.len() as u32;
        let first_index = geometry.indices.len() as u32;
        if matches!(declaration.kind, thrum_water_model::BodyKind::Sea) {
            for level in 0..17 {
                geometry.grid(
                    [-32; 2],
                    [32; 2],
                    0.25 * 2.0_f32.powi(level),
                    0.0,
                    level > 0,
                );
            }
        } else if let Some([min, max]) = map.surface_bounds(body) {
            // Tile fine enough for geometry in the near view; shading filters
            // the unresolved part of each layer separately.
            let span = (max[0] - min[0]).max(max[1] - min[1]);
            let spacing = 0.5_f32.max((span / 1024.0) as f32);
            let start = min.map(|x| (x / f64::from(spacing)).floor() as i32);
            let end = max.map(|x| (x / f64::from(spacing)).ceil() as i32);
            geometry.grid(start, end, spacing, (body + 1) as f32, false);
        }
        geometry.bodies.push((
            declaration.name.clone(),
            geometry.vertices.len() as u32 - first,
        ));
        geometry
            .draws
            .push(first_index..geometry.indices.len() as u32);
    }
    geometry
}

impl Geometry {
    fn upload(
        &self,
        device: &wgpu::Device,
    ) -> Result<(wgpu::Buffer, Option<wgpu::Buffer>), String> {
        let vertices = bytemuck::cast_slice(&self.vertices);
        let indices = bytemuck::cast_slice(&self.indices);
        if vertices.len().max(indices.len()) as u64 > device.limits().max_buffer_size {
            return Err("water mesh exceeds the maximum buffer size".into());
        }
        let buffer = |label, contents, usage| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents,
                usage,
            })
        };
        Ok((
            buffer(
                "water surface vertices",
                vertices,
                wgpu::BufferUsages::VERTEX,
            ),
            (!indices.is_empty()).then(|| {
                buffer(
                    "water surface triangle indices",
                    indices,
                    wgpu::BufferUsages::INDEX,
                )
            }),
        ))
    }

    fn grid(&mut self, start: [i32; 2], end: [i32; 2], spacing: f32, body: f32, ring: bool) {
        let base = u32::try_from(self.vertices.len()).expect("bounded mesh");
        let width = u32::try_from(end[0] - start[0] + 1).expect("grid width");
        for (row, z) in (0..).zip(start[1]..=end[1]) {
            for (column, x) in (0..).zip(start[0]..=end[0]) {
                self.vertices
                    .push([x as f32 * spacing, z as f32 * spacing, spacing, body]);
                let centre = ring && (-16..16).contains(&x) && (-16..16).contains(&z);
                if x < end[0] && z < end[1] && !centre {
                    let a = base + row * width + column;
                    self.indices
                        .extend([a, a + width, a + 1, a + 1, a + width, a + width + 1]);
                }
            }
        }
    }
}
fn mesh() -> Vec<[f32; 4]> {
    let mut vertices = Vec::new();
    // Nested world-snapped square rings. Every outer boundary has the next ring's
    // even vertices; boundary heights are interpolated at the shared coarse edge.
    for level in 0_i32..17 {
        let spacing = 0.25 * 2.0_f32.powi(level);
        for z in -32_i16..32 {
            for x in -32_i16..32 {
                if level > 0 && (-16..16).contains(&x) && (-16..16).contains(&z) {
                    continue;
                }
                for [dx, dz] in [[0, 0], [0, 1], [1, 0], [1, 0], [0, 1], [1, 1]] {
                    let px = f32::from(x + dx);
                    let pz = f32::from(z + dz);
                    vertices.push([px * spacing, pz * spacing, spacing, 0.0]);
                }
            }
        }
    }
    vertices
}
