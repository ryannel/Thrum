use super::{
    Profile, SURFACE_PASS,
    foam::{self, Foam},
    interaction::{self, Interaction},
    map_gpu::{self, MapGpu},
    query::{self, QueryMailbox, Readback},
    records::Parameters,
    surf::{SurfGpu, cache::Cache},
};
use std::ops::Range;
use thrum_render::binding;
use thrum_render::{
    atmosphere::EnvironmentProducts,
    contribution::{AfterOpaqueKind, AfterOpaquePass, EffectCadence, EffectGraph, SceneSurface},
    runtime::DeviceSite,
};
use wgpu::util::DeviceExt;

pub(crate) const SEA_RING_HALF_WIDTH: i32 = 32;
pub(crate) const SURF_FINE_RING_HALF_WIDTH: i32 = 64;
pub(crate) const SURF_HALF_METRE_RING_HALF_WIDTH: i32 = 240;
pub(crate) const SURF_METRE_RING_HALF_WIDTH: i32 = 128;
pub(crate) const SURF_TWO_METRE_RING_HALF_WIDTH: i32 = 72;
pub(crate) const SURF_FOUR_METRE_RING_HALF_WIDTH: i32 = 40;
pub(crate) const SURF_EIGHT_METRE_RING_HALF_WIDTH: i32 = 40;

pub(super) struct Prepared {
    pub(super) ambient: thrum_sea::Ambient,
    pub(super) ambient_passes: usize,
    pub(super) query: Option<Readback>,
    pub(super) interaction: Option<Interaction>,
    pub(super) foam: Option<Foam>,
    pub(super) environment: EnvironmentProducts,
    pub(super) pipeline: wgpu::RenderPipeline,
    pub(super) film: Option<wgpu::RenderPipeline>,
    pub(super) parameters: wgpu::Buffer,
    pub(super) group: wgpu::BindGroup,
    pattern: Option<Pattern>,
    pub(super) mesh: wgpu::Buffer,
    pub(super) vertices: u32,
    pub(super) queue: wgpu::Queue,
    pub(super) map: Option<MapGpu>,
    pub(super) surf: Option<SurfGpu>,
    pub(super) surf_cache: Option<Cache>,
    pub(super) bodies: Vec<(String, u32)>,
    pub(super) indices: Option<wgpu::Buffer>,
    pub(super) draws: Vec<WaterDraw>,
}
struct Pattern {
    device: DeviceSite,
    layout: wgpu::BindGroupLayout,
    samplers: [wgpu::Sampler; 2],
    bound: Option<(u64, wgpu::BindGroup)>,
    submitted: Option<u64>,
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
fn film_declaration() -> AfterOpaquePass {
    AfterOpaquePass {
        name: "shore film".into(),
        depth: true,
        write_depth: false,
        ..declaration()
    }
}
impl Prepared {
    pub(super) fn render_group(&self) -> &wgpu::BindGroup {
        self.pattern
            .as_ref()
            .and_then(|pattern| pattern.bound.as_ref())
            .map_or(&self.group, |(_, group)| group)
    }

    pub(super) fn record_pattern(
        &mut self,
        lease: &thrum_asset_gpu::ProductLease,
        generation: u64,
    ) -> bool {
        let Some(pattern) = &mut self.pattern else {
            return false;
        };
        let offer = lease.offered();
        let product = offer
            .as_ref()
            .and_then(|set| set.products.first())
            .filter(|product| product.device_generation == generation);
        let serial = product.map(|product| product.serial);
        if pattern.bound.as_ref().map(|(serial, _)| *serial) != serial {
            pattern.bound = product.map(|product| {
                let view = self.map.as_ref().map(MapGpu::view);
                let mut entries = field_entries(
                    &self.parameters,
                    self.ambient.filtered_views(),
                    &pattern.samplers,
                    self.map.as_ref().zip(view.as_ref()),
                    self.interaction.as_ref(),
                );
                entries.extend(self.foam.as_ref().expect("foam pattern").sample_entries());
                entries.push(pattern_entry(&product.view));
                if let Some(cache) = &self.surf_cache {
                    entries.push(cache.surface_entry());
                }
                (
                    product.serial,
                    pattern
                        .device
                        .create_bind_group(&wgpu::BindGroupDescriptor {
                            label: Some("water field with foam pattern"),
                            layout: &pattern.layout,
                            entries: &entries,
                        }),
                )
            });
        }
        serial != pattern.submitted
    }

    pub(super) fn submit_pattern(&mut self) -> (Option<u64>, bool) {
        self.pattern.as_mut().map_or((None, false), |pattern| {
            let serial = pattern.bound.as_ref().map(|(serial, _)| *serial);
            let changed = serial != pattern.submitted;
            pattern.submitted = serial;
            (serial, changed)
        })
    }

    pub(super) fn query_index(&self) -> usize {
        self.ambient_passes + usize::from(self.interaction.is_some())
    }

    pub(super) fn surf_cache_index(&self) -> usize {
        self.query_index() + usize::from(self.query.is_some())
    }

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
        let surf_cache = self
            .surf_cache
            .as_ref()
            .map(|cache| cache.declare(&mut graph));
        if let Some(foam) = &self.foam {
            let read = foam.declare(&mut graph, surface.accesses.clone());
            surface.accesses.push(read);
        }
        graph.opaque_reads.clear();
        if self.film.is_some() {
            let mut film = film_declaration();
            film.accesses.clone_from(&surface.accesses);
            graph.after_opaque.push(film);
        }
        surface.accesses.extend(surf_cache);
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
    whitewater: Option<(foam::Whitewater, foam::Report)>,
) -> Result<Prepared, String> {
    let device = surface.device;
    let map_gpu = mapped
        .then_some(map)
        .map(|map| MapGpu::new(device, surface.queue, map))
        .transpose()
        .map_err(|e| e.to_string())?;
    let surf = map
        .surf_profile()
        .filter(|_| mapped)
        .map(|profile| SurfGpu::new(device, surface.queue, map, profile))
        .transpose()
        .map_err(|error| error.to_string())?;
    let geometry = surface_mesh(mapped.then_some(map));
    let count = u32::try_from(geometry.vertices.len()).map_err(|e| e.to_string())?;
    let (mesh, indices) = geometry.upload(device, surf.is_some())?;
    let surf_cache = surf
        .as_ref()
        .map(|surf| {
            Cache::new(
                device,
                map_gpu.as_ref().expect("mapped surf"),
                surf,
                &mesh,
                count,
                whitewater.as_ref().map(|_| map.bed().lattice().2),
            )
        })
        .transpose()
        .map_err(|error| error.to_string())?;
    let ambient =
        thrum_sea::Ambient::new(device, profile.waves).map_err(|error| error.to_string())?;
    let ambient_passes = ambient.graph().passes.len();
    let interaction = interaction
        .map(|mailbox| Interaction::new(device, mailbox, previous_interaction.unwrap_or_default()));
    let query = if let Some(mailbox) = mailbox {
        query::validate(
            profile.query.expect("enabled query limits"),
            &surface.limits,
            surf.is_some(),
        )?;
        Some(Readback::new(
            device,
            &ambient,
            mailbox,
            interaction.as_ref(),
            map_gpu.as_ref().zip(surf.as_ref()),
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
        .map(|binding| {
            let visibility = wgpu::ShaderStages::VERTEX_FRAGMENT;
            if binding == 0 {
                binding::uniform(binding, visibility, 0)
            } else if binding <= 2 {
                binding::texture(
                    binding,
                    visibility,
                    wgpu::TextureSampleType::Float { filterable: true },
                    wgpu::TextureViewDimension::D2Array,
                )
            } else {
                binding::sampler(binding, visibility, wgpu::SamplerBindingType::Filtering)
            }
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
    let samplers = field_samplers(surface);
    let map_view = map_gpu.as_ref().filter(|_| mapped).map(MapGpu::view);
    let mut bindings = field_entries(
        &parameters,
        ambient.filtered_views(),
        &samplers,
        map_gpu.as_ref().zip(map_view.as_ref()),
        interaction.as_ref(),
    );
    let foam = whitewater.map(|(settings, report)| {
        Foam::new(
            device,
            settings,
            report,
            &entries,
            &bindings,
            interaction.is_some(),
            surf.as_ref(),
        )
    });
    let fallback = foam.as_ref().map(|_| {
        device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("absent foam pattern"),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default())
    });
    if let (Some(foam), Some(fallback)) = (&foam, &fallback) {
        entries.extend(Foam::sample_layout());
        entries.push(binding::texture(
            12,
            wgpu::ShaderStages::FRAGMENT,
            wgpu::TextureSampleType::Float { filterable: true },
            wgpu::TextureViewDimension::D2,
        ));
        bindings.extend(foam.sample_entries());
        bindings.push(pattern_entry(fallback));
    }
    if let Some(cache) = &surf_cache {
        entries.push(cache.surface_binding());
        bindings.push(cache.surface_entry());
    }
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("water field"),
        entries: &entries,
    });
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("water field"),
        layout: &layout,
        entries: &bindings,
    });
    let source = shader_source(&environment, mapped, interaction.is_some(), foam.as_ref())
        + &Cache::source(surf_cache.as_ref());
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(SURFACE_PASS),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let pipeline = create_pipeline(surface, &layout, &environment, &shader, mapped, false);
    let film = (foam.is_some() && has_shore(map))
        .then(|| create_pipeline(surface, &layout, &environment, &shader, true, true));
    let pattern = foam.as_ref().map(|_| Pattern {
        device: device.clone(),
        layout,
        samplers,
        bound: None,
        submitted: None,
    });
    Ok(Prepared {
        ambient,
        ambient_passes,
        query,
        interaction,
        foam,
        environment,
        pipeline,
        film,
        parameters,
        group,
        pattern,
        mesh,
        vertices: count,
        queue: surface.queue.clone(),
        map: map_gpu,
        surf,
        surf_cache,
        bodies: geometry.bodies,
        draws: geometry.draws,
        indices,
    })
}

fn has_shore(map: &thrum_water_model::Map) -> bool {
    map.body(1).is_some()
        || map.body(0).is_some_and(|body| {
            !matches!(body.kind, thrum_water_model::BodyKind::Sea)
                || map
                    .bed()
                    .heights()
                    .iter()
                    .any(|bed| *bed >= body.level + body.stage.min - map.profile().margin_height)
        })
}

fn field_samplers(surface: &SceneSurface<'_>) -> [wgpu::Sampler; 2] {
    let device = surface.device;
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
    [
        sampler(wgpu::AddressMode::Repeat),
        sampler(wgpu::AddressMode::ClampToEdge),
    ]
}

fn field_entries<'a>(
    parameters: &'a wgpu::Buffer,
    views: &'a [wgpu::TextureView; 2],
    samplers: &'a [wgpu::Sampler; 2],
    map: Option<(&'a MapGpu, &'a wgpu::TextureView)>,
    interaction: Option<&'a Interaction>,
) -> Vec<wgpu::BindGroupEntry<'a>> {
    use wgpu::BindingResource::{Sampler, TextureView};
    let mut bindings = vec![
        wgpu::BindGroupEntry {
            binding: 0,
            resource: parameters.as_entire_binding(),
        },
        wgpu::BindGroupEntry {
            binding: 1,
            resource: TextureView(&views[0]),
        },
        wgpu::BindGroupEntry {
            binding: 2,
            resource: TextureView(&views[1]),
        },
        wgpu::BindGroupEntry {
            binding: 3,
            resource: Sampler(&samplers[0]),
        },
        wgpu::BindGroupEntry {
            binding: 4,
            resource: Sampler(&samplers[1]),
        },
    ];
    if let Some((map, view)) = map {
        bindings.extend(map.entries(view));
    }
    if let Some(interaction) = interaction {
        bindings.extend(interaction.sample_entries(7));
    }
    bindings
}

fn pattern_entry(view: &wgpu::TextureView) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding: 12,
        resource: wgpu::BindingResource::TextureView(view),
    }
}

fn shader_source(
    environment: &EnvironmentProducts,
    mapped: bool,
    wake: bool,
    foam: Option<&Foam>,
) -> String {
    format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        environment.shader_source.replace("@group(0)", "@group(2)"),
        include_str!("fresnel.wgsl"),
        include_str!("sampling.wgsl"),
        include_str!("field.wgsl"),
        Interaction::sample_source(3, 7, 8, wake),
        include_str!("surface.wgsl"),
        if mapped {
            map_gpu::declarations() + include_str!("mapped_surface.wgsl")
        } else {
            include_str!("open_surface.wgsl").to_owned()
        },
        if let Some(foam) = foam {
            foam.sample_source() + include_str!("foam/appearance.wgsl")
        } else {
            "const FOAM_ENABLED: bool = false;
             fn foam_surface(_p: vec3<f32>, _r: SurfaceRow, _v: f32, _d: f32, _face: f32) -> vec4<f32> { return vec4(0.0); }"
                .into()
        },
    )
}

fn create_pipeline(
    surface: &SceneSurface<'_>,
    layout: &wgpu::BindGroupLayout,
    environment: &EnvironmentProducts,
    shader: &wgpu::ShaderModule,
    mapped: bool,
    film: bool,
) -> wgpu::RenderPipeline {
    let device = surface.device;
    let pass = if film {
        film_declaration()
    } else {
        declaration()
    };
    let inputs = pass.input_layout(device);
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(&pass.name),
        bind_group_layouts: &[
            Some(surface.view_layout),
            Some(&inputs),
            Some(&environment.layout),
            Some(layout),
        ],
        immediate_size: 0,
    });
    let targets: Vec<_> = surface
        .color_formats
        .iter()
        .enumerate()
        .map(|(i, f)| {
            Some(wgpu::ColorTargetState {
                format: *f,
                blend: match i {
                    0 if mapped => Some(wgpu::BlendState::ALPHA_BLENDING),
                    2 if film => {
                        let max = wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Max,
                        };
                        Some(wgpu::BlendState {
                            color: max,
                            alpha: max,
                        })
                    }
                    _ => None,
                },
                write_mask: if film && i == 1 {
                    wgpu::ColorWrites::empty()
                } else {
                    wgpu::ColorWrites::ALL
                },
            })
        })
        .collect();
    device.render(&wgpu::RenderPipelineDescriptor {
        label: Some(&pass.name),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some(if film { "film_vertex" } else { "water_vertex" }),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: if film {
                &[]
            } else {
                &[Some(wgpu::VertexBufferLayout {
                    array_stride: 16,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 0,
                        shader_location: 0,
                    }],
                })]
            },
        },
        primitive: wgpu::PrimitiveState {
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: (!film).then_some(wgpu::DepthStencilState {
            format: surface.depth_format,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::GreaterEqual),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(if film {
                "film_fragment"
            } else {
                "water_fragment"
            }),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &targets,
        }),
        multiview_mask: None,
        cache: None,
    })
}

pub(super) struct WaterDraw {
    pub(super) indices: Range<u32>,
    offshore: Option<OffshoreRing>,
}

struct OffshoreRing {
    radius: f32,
    levels: [f32; 2],
    bed: [[f32; 2]; 2],
}

impl WaterDraw {
    pub(super) fn visible(
        &self,
        frame: &thrum_render::contribution::ContributionFrame,
        waves: [f32; 2],
    ) -> bool {
        let Some(sea) = &self.offshore else {
            return true;
        };
        let [x, _, z] = frame.camera;
        let centre = [x.floor(), z.floor()];
        // Every node in the ring must be outside the prepared bed. On
        // the bed, the surf sheet has additional displacement and swash.
        if !(0..2).any(|axis| {
            centre[axis] + sea.radius < sea.bed[0][axis]
                || centre[axis] - sea.radius > sea.bed[1][axis]
        }) {
            return true;
        }
        let radius = sea.radius + waves[0];
        let lo = glam::Vec3::new(
            centre[0] - radius,
            sea.levels[0] - waves[1],
            centre[1] - radius,
        );
        let hi = glam::Vec3::new(
            centre[0] + radius,
            sea.levels[1] + waves[1],
            centre[1] + radius,
        );
        let matrix = glam::Mat4::from_cols_array_2d(&frame.view_proj);
        let [x, y, _, w] = std::array::from_fn(|row| matrix.row(row));
        // The horizon shader clamps z. Reject only outside the raster's
        // horizontal and vertical planes, using the frame's jittered view.
        ![w + x, w - x, w + y, w - y].iter().any(|plane| {
            let axis = plane.truncate();
            let high = (lo * axis).max(hi * axis).element_sum();
            let guard = 16.0
                * f32::EPSILON
                * (lo.abs().max(hi.abs()).dot(axis.abs()) + plane.w.abs() + 1.0);
            high + plane.w < -guard
        })
    }
}

#[derive(Default)]
struct Geometry {
    vertices: Vec<[f32; 4]>,
    indices: Vec<u32>,
    bodies: Vec<(String, u32)>,
    draws: Vec<WaterDraw>,
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
    let surf = map.surf_profile().is_some();
    for body in 0..map_gpu::BODY_LIMIT {
        let Some(declaration) = map.body(body) else {
            break;
        };
        let first = geometry.vertices.len() as u32;
        if matches!(declaration.kind, thrum_water_model::BodyKind::Sea) {
            let (origin, step, extent) = map.bed().lattice();
            let upper: [f64; 2] =
                std::array::from_fn(|a| origin[a] + (extent[a] - 1) as f64 * step);
            let bed = [origin, upper].map(|p| p.map(|v| v as f32));
            let levels = [declaration.stage.min, declaration.stage.max]
                .map(|offset| (declaration.level + offset) as f32);
            let mut previous_half_width = 0;
            for level in 0..17 {
                let half_width = match (surf, level) {
                    (true, 0) => SURF_FINE_RING_HALF_WIDTH,
                    (true, 1) => SURF_HALF_METRE_RING_HALF_WIDTH,
                    (true, 2) => SURF_METRE_RING_HALF_WIDTH,
                    (true, 3) => SURF_TWO_METRE_RING_HALF_WIDTH,
                    (true, 4) => SURF_FOUR_METRE_RING_HALF_WIDTH,
                    // Keep the mesh transition apart from the coarse foam window's edge.
                    (true, 5) => SURF_EIGHT_METRE_RING_HALF_WIDTH,
                    _ => SEA_RING_HALF_WIDTH,
                };
                let first_index = geometry.indices.len() as u32;
                let spacing = 0.25 * 2.0_f32.powi(level);
                geometry.grid(
                    [-half_width; 2],
                    [half_width; 2],
                    spacing,
                    0.0,
                    previous_half_width / 2,
                    surf,
                );
                let radius = half_width as f32 * spacing;
                geometry.draws.push(WaterDraw {
                    indices: first_index..geometry.indices.len() as u32,
                    offshore: (radius < 524_288.0).then_some(OffshoreRing {
                        radius,
                        levels,
                        bed,
                    }),
                });
                previous_half_width = half_width;
            }
        } else if let Some([min, max]) = map.surface_bounds(body) {
            // Tile fine enough for geometry in the near view; shading filters
            // the unresolved part of each layer separately.
            let span = (max[0] - min[0]).max(max[1] - min[1]);
            let spacing = 0.5_f32.max((span / 1024.0) as f32);
            let start = min.map(|x| (x / f64::from(spacing)).floor() as i32);
            let end = max.map(|x| (x / f64::from(spacing)).ceil() as i32);
            let first_index = geometry.indices.len() as u32;
            geometry.grid(start, end, spacing, (body + 1) as f32, 0, surf);
            geometry.draws.push(WaterDraw {
                indices: first_index..geometry.indices.len() as u32,
                offshore: None,
            });
        }
        geometry.bodies.push((
            declaration.name.clone(),
            geometry.vertices.len() as u32 - first,
        ));
    }
    geometry
}

impl Geometry {
    fn upload(
        &self,
        device: &wgpu::Device,
        surf: bool,
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
        let mut vertex_usage = wgpu::BufferUsages::VERTEX;
        if surf {
            vertex_usage |= wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC;
        }
        Ok((
            buffer("water surface vertices", vertices, vertex_usage),
            (!indices.is_empty()).then(|| {
                buffer(
                    "water surface triangle indices",
                    indices,
                    wgpu::BufferUsages::INDEX,
                )
            }),
        ))
    }

    fn grid(
        &mut self,
        start: [i32; 2],
        end: [i32; 2],
        spacing: f32,
        body: f32,
        inner_half_width: i32,
        surf: bool,
    ) {
        let base = u32::try_from(self.vertices.len()).expect("bounded mesh");
        let width = u32::try_from(end[0] - start[0] + 1).expect("grid width");
        for (row, z) in (0..).zip(start[1]..=end[1]) {
            for (column, x) in (0..).zip(start[0]..=end[0]) {
                self.vertices
                    .push([x as f32 * spacing, z as f32 * spacing, spacing, body]);
                let inner = -inner_half_width..inner_half_width;
                let centre = inner.contains(&x) && inner.contains(&z);
                if x < end[0] && z < end[1] && !centre {
                    let a = base + row * width + column;
                    if surf {
                        // Match the diagonal used by the highest-surface cut.
                        let d = a + width + 1;
                        self.indices.extend([a, a + width, d, a, d, a + 1]);
                    } else {
                        self.indices
                            .extend([a, a + width, a + 1, a + 1, a + width, a + width + 1]);
                    }
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
