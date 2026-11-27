pub(crate) mod hollow_tiles;

use super::{
    Profile, SURFACE_PASS,
    foam::{self, Foam},
    interaction::{self, Interaction},
    map_gpu::{self, MapGpu},
    optics::{self, Gather},
    query::{self, QueryMailbox, Readback},
    rain::Rain,
    records::Parameters,
    surf::{SurfGpu, cache::Cache},
    underwater::cut_gpu::{self, Cut},
};
use std::ops::Range;
use thrum_render::binding;
use thrum_render::{
    atmosphere::EnvironmentProducts,
    contribution::{AfterOpaqueKind, AfterOpaquePass, EffectCadence, EffectGraph, SceneSurface},
    runtime::DeviceSite,
};
use wgpu::util::DeviceExt;

const WET_GROUND_BINDING: u32 = 26;

pub(crate) const SEA_RING_HALF_WIDTH: i32 = 32;
// A joined chart uses the same half-metre lattice as the sea throughout
// the near view. This keeps the two displaced triangulations coincident
// where the draw switches at the drowned edge.
pub(crate) const TIDAL_RING_HALF_WIDTHS: [i32; 4] = [184, 104, 64, 40];
pub(crate) const SURF_FINE_RING_HALF_WIDTH: i32 = 64;
pub(crate) const SURF_HALF_METRE_RING_HALF_WIDTH: i32 = 240;
pub(crate) const SURF_METRE_RING_HALF_WIDTH: i32 = 128;
pub(crate) const SURF_TWO_METRE_RING_HALF_WIDTH: i32 = 72;
pub(crate) const SURF_FOUR_METRE_RING_HALF_WIDTH: i32 = 40;
pub(crate) const SURF_EIGHT_METRE_RING_HALF_WIDTH: i32 = 40;

pub(super) struct Prepared {
    pub(super) ambient: thrum_sea::Ambient,
    pub(super) ambient_passes: usize,
    pub(super) rain: Option<Rain>,
    pub(super) query: Option<Readback>,
    pub(super) interaction: Option<Interaction>,
    pub(super) foam: Option<Foam>,
    pub(super) environment: EnvironmentProducts,
    pub(super) gather: Option<Gather>,
    pub(super) pipeline: wgpu::RenderPipeline,
    pub(super) river_pipeline: Option<wgpu::RenderPipeline>,
    pub(super) film: Option<wgpu::RenderPipeline>,
    pub(super) surface_sampled_textures: Option<usize>,
    wet_ground: Option<thrum_render::wetness::PreparedWetness>,
    standing_film: bool,
    pub(super) underwater: Option<UnderwaterPipelines>,
    pub(super) parameters: wgpu::Buffer,
    pub(super) group: wgpu::BindGroup,
    pattern: Option<Pattern>,
    pub(super) mesh: wgpu::Buffer,
    pub(super) vertices: u32,
    pub(super) rest_mesh: Option<super::underwater::RestMesh>,
    pub(super) lens_cut: Option<Cut>,
    pub(super) queue: wgpu::Queue,
    pub(super) map: Option<MapGpu>,
    pub(super) surf: Option<SurfGpu>,
    pub(super) surf_cache: Option<Cache>,
    pub(super) bodies: Vec<(String, u32)>,
    pub(super) indices: Option<wgpu::Buffer>,
    pub(super) draws: Vec<WaterDraw>,
    pub(super) hollow_tiles: Option<hollow_tiles::HollowTiles>,
}
pub(super) struct UnderwaterPipelines {
    pub(super) surface: wgpu::RenderPipeline,
    pub(super) river: Option<wgpu::RenderPipeline>,
    pub(super) through_lens: wgpu::RenderPipeline,
}

struct SurfacePipelines {
    sampled_textures: Option<usize>,
    sheet: wgpu::RenderPipeline,
    river: Option<wgpu::RenderPipeline>,
    film: Option<wgpu::RenderPipeline>,
    underwater: Option<UnderwaterPipelines>,
}

struct Pattern {
    device: DeviceSite,
    layout: wgpu::BindGroupLayout,
    samplers: [wgpu::Sampler; 2],
    bound: Option<(u64, wgpu::BindGroup)>,
    submitted: Option<u64>,
}
fn declaration(gather: bool) -> AfterOpaquePass {
    AfterOpaquePass {
        name: SURFACE_PASS.into(),
        kind: AfterOpaqueKind::Render,
        cadence: EffectCadence::EveryFrame,
        accesses: Vec::new(),
        depth: false,
        write_depth: true,
        refraction: false,
        images: if gather {
            optics::sampled(0)
        } else {
            thrum_render::contribution::AfterOpaqueImages::None
        },
    }
}
fn film_declaration(standing_water: bool) -> AfterOpaquePass {
    AfterOpaquePass {
        name: "shore film".into(),
        depth: true,
        write_depth: false,
        images: if standing_water {
            optics::sampled(0)
        } else {
            thrum_render::contribution::AfterOpaqueImages::None
        },
        ..declaration(false)
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
                    self.ambient.surface_views(),
                    &pattern.samplers,
                    self.map.as_ref().zip(view.as_ref()),
                    self.interaction.as_ref(),
                );
                if let Some(rain) = &self.rain {
                    entries.extend(rain.entries());
                }
                entries.extend(self.foam.as_ref().expect("foam pattern").sample_entries());
                entries.push(pattern_entry(&product.view));
                if let Some(cache) = &self.surf_cache {
                    entries.push(cache.surface_entry());
                }
                if let Some(gather) = &self.gather {
                    entries.push(optics::entry(&gather.buffer));
                }
                if let Some(wet) = &self.wet_ground {
                    entries.push(wet.entry(WET_GROUND_BINDING));
                }
                if let Some(cut) = &self.lens_cut {
                    entries.extend(cut.render_entries(&self.mesh));
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

    pub(super) fn surf_cache_index(&self) -> usize {
        self.ambient_passes + usize::from(self.interaction.is_some())
    }

    pub(super) fn rain_offset(&self) -> usize {
        self.surf_cache_index()
            + usize::from(self.surf_cache.is_some())
            + usize::from(self.foam.is_some())
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
        let mut surface = declaration(self.gather.is_some());
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
        if let Some(rain) = &self.rain {
            surface.accesses.extend(rain.bands.declare(&mut graph));
        }
        graph.opaque_reads.clear();
        if self.gather.is_some() {
            graph.after_opaque.push(optics::declaration());
        }
        if self.film.is_some() {
            let mut film = film_declaration(self.standing_film);
            film.accesses.clone_from(&surface.accesses);
            graph.after_opaque.push(film);
        }
        surface.accesses.extend(surf_cache);
        if let Some(cut) = &self.lens_cut {
            use thrum_render::{
                contribution::EffectAccess,
                graph::{AccessKind, ResourceDescriptor, ResourceScope},
            };
            let buffer = cut.output();
            let resource = graph.resources.len();
            graph.resources.push((
                ResourceDescriptor::Buffer {
                    bytes: buffer.size(),
                    usage: buffer.usage(),
                },
                ResourceScope::Persistent {
                    owner: 0x6c65_6e73,
                    epoch: 0,
                    initialized: true,
                },
            ));
            let read = EffectAccess {
                resource,
                range: 0..buffer.size(),
                kind: AccessKind::StorageRead,
                view: None,
                history: None,
            };
            let mut pass = Cut::declaration();
            pass.accesses.clone_from(&surface.accesses);
            pass.accesses.push(EffectAccess {
                kind: AccessKind::StorageWrite,
                ..read.clone()
            });
            // Its roster index follows the gather, before either water draw.
            graph
                .after_opaque
                .insert(usize::from(self.gather.is_some()), pass);
            surface.accesses.push(read);
        }
        graph.after_opaque.push(surface);
        graph
    }
}
#[expect(
    clippy::too_many_lines,
    reason = "one preparation transaction for shared water resources and pipelines"
)]
pub(super) fn new(
    surface: &SceneSurface<'_>,
    profile: Profile,
    map: &thrum_water_model::Map,
    mapped: bool,
    long_components: usize,
    mailbox: Option<QueryMailbox>,
    interaction: Option<interaction::Mailbox>,
    previous_interaction: Option<interaction::Report>,
    whitewater: Option<(foam::Whitewater, foam::Report)>,
    measure_surf_work: bool,
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
    let tidal_grid = mapped && map.surf_profile().is_none() && has_tidal_join(map);
    let geometry = surface_mesh(mapped.then_some(map), tidal_grid, profile.underwater);
    let count = u32::try_from(geometry.vertices.len()).map_err(|e| e.to_string())?;
    let (mesh, indices) = geometry.upload(device, surf.is_some() || profile.underwater)?;
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
                measure_surf_work,
            )
        })
        .transpose()
        .map_err(|error| error.to_string())?;
    let flow_profile = mapped.then(|| map.flow_bands()).flatten();
    let ambient = if let Some(flow) = flow_profile {
        thrum_sea::Ambient::with_flow_bands(
            device,
            surface.queue,
            profile.waves,
            map.rain().is_some(),
            flow,
        )
    } else if map.rain().is_some() {
        thrum_sea::Ambient::with_rain(device, profile.waves)
    } else {
        thrum_sea::Ambient::new(device, profile.waves)
    }
    .map_err(|error| error.to_string())?;
    let ambient_passes = ambient.graph().passes.len();
    let flow = flow_profile
        .zip(ambient.flow_first_layer())
        .map(|(flow, first)| (flow.profile(), first));
    let wind_samplers = field_samplers(surface, 0.0);
    let rain = map
        .rain()
        .map(|model| {
            Rain::new(
                device,
                surface.queue,
                model,
                ambient.filtered_texture(0).expect("cascade array"),
                wind_samplers[0].clone(),
            )
        })
        .transpose()?;
    let interaction = interaction
        .map(|mailbox| Interaction::new(device, mailbox, previous_interaction.unwrap_or_default()));
    let query = if let Some(mailbox) = mailbox {
        mailbox.set_rain_tracks(map_gpu.as_ref());
        let rivers = map_gpu.as_ref().is_some_and(MapGpu::has_rivers);
        query::validate(
            profile.query.expect("enabled query limits"),
            &surface.limits,
            surf.is_some(),
            interaction.is_some(),
            rivers,
            flow,
        )?;
        let query_map = surf
            .as_ref()
            .map(|surf| MapGpu::new(device, surface.queue, map).map(|map| (map, surf)))
            .transpose()
            .map_err(|error| error.to_string())?;
        Some(Readback::new(
            device,
            &ambient,
            mailbox,
            interaction.as_ref(),
            query_map,
            long_components,
            map.body(1).is_none(),
            rivers,
            flow,
        ))
    } else {
        None
    };
    let environment = surface
        .environment_products_with_cloud()
        .ok_or("water needs the scene's physical environment")?;
    let library = environment.shader_source.replace("@group(0)", "@group(2)");
    let projection = surface
        .wetness
        .filter(|wet| wet.uses(super::wet_ground::SOURCE));
    let standing_film = mapped
        && projection.is_some()
        && profile.optics.is_some()
        && map_gpu
            .as_ref()
            .is_some_and(|map| map.hollows_entry().is_some());
    let wet_ground = projection.filter(|_| standing_film || mapped && whitewater.is_some());
    let gather = profile.optics.map(|optics| {
        Gather::new(
            surface,
            &environment,
            optics,
            &library,
            profile.underwater,
            standing_film,
        )
    });
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
        let stages = wgpu::ShaderStages::VERTEX_FRAGMENT;
        entries.extend(map_gpu.as_ref().expect("mapped water").bindings(stages));
        entries.extend(
            map_gpu
                .as_ref()
                .and_then(|map| map.hollows_bindings(stages)),
        );
    }
    if interaction.is_some() {
        entries.extend(Interaction::sample_layout(
            7,
            wgpu::ShaderStages::VERTEX_FRAGMENT,
        ));
    }
    let map_view = map_gpu.as_ref().filter(|_| mapped).map(MapGpu::view);
    let mut bindings = field_entries(
        &parameters,
        ambient.filtered_views(),
        &wind_samplers,
        map_gpu.as_ref().zip(map_view.as_ref()),
        interaction.as_ref(),
    );
    let foam = whitewater.map(|(settings, report)| {
        if flow.is_some() {
            bindings[1].resource = wgpu::BindingResource::TextureView(ambient.query_view());
        }
        Foam::new(
            device,
            settings,
            report,
            &entries,
            &bindings,
            interaction.is_some(),
            surf.as_ref(),
            map_gpu.as_ref(),
            flow,
        )
    });
    let shared_samplers = rain
        .as_ref()
        .map(|_| field_samplers(surface, f32::from(thrum_sea::rain::WIND_BASE_MIP)));
    let samplers = shared_samplers.as_ref().unwrap_or(&wind_samplers);
    if rain.is_some() || flow.is_some() {
        bindings[1].resource = wgpu::BindingResource::TextureView(&ambient.surface_views()[0]);
        bindings[3].resource = wgpu::BindingResource::Sampler(&samplers[0]);
    }
    let fallback = foam.as_ref().map(|_| fallback_pattern(device));
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
    bind_optics(gather.as_ref(), &mut entries, &mut bindings);
    if let Some(wet) = wet_ground {
        entries.push(thrum_render::wetness::PreparedWetness::layout(
            WET_GROUND_BINDING,
        ));
        bindings.push(wet.entry(WET_GROUND_BINDING));
    }
    if let Some(rain) = &rain {
        entries.extend(Rain::bindings());
        bindings.extend(rain.entries());
    }
    let lens_buffers = geometry
        .rest_mesh
        .as_ref()
        .map(|rest| cut_gpu::Buffers::new(device, rest, mesh.size()))
        .transpose()?;
    if let Some(buffers) = &lens_buffers {
        entries.extend(cut_gpu::Buffers::render_bindings());
        bindings.extend(buffers.render_entries(&mesh));
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
    let (pipelines, shader) = surface_pipelines(
        surface,
        &layout,
        &environment,
        &library,
        map_gpu.as_ref(),
        interaction.as_ref(),
        foam.as_ref(),
        gather.as_ref(),
        surf_cache.as_ref(),
        has_shore(map),
        tidal_grid,
        rain.is_some(),
        flow,
        (wet_ground, standing_film),
    );
    let lens_cut = lens_buffers
        .as_ref()
        .zip(shader.as_ref())
        .map(|(buffers, shader)| Cut::new(surface, shader, buffers.clone(), &entries, &bindings));
    let pattern = foam.as_ref().map(|_| Pattern {
        device: device.clone(),
        layout: layout.into_raw(),
        samplers: samplers.clone(),
        bound: None,
        submitted: None,
    });
    Ok(Prepared {
        ambient,
        ambient_passes,
        rain,
        query,
        interaction,
        foam,
        environment,
        gather,
        pipeline: pipelines.sheet,
        river_pipeline: pipelines.river,
        film: pipelines.film,
        surface_sampled_textures: pipelines.sampled_textures,
        wet_ground: wet_ground.cloned(),
        standing_film,
        underwater: pipelines.underwater,
        parameters,
        group,
        pattern,
        mesh,
        vertices: count,
        rest_mesh: geometry.rest_mesh,
        lens_cut,
        queue: surface.queue.clone(),
        map: map_gpu,
        surf,
        surf_cache,
        bodies: geometry.bodies,
        draws: geometry.draws,
        hollow_tiles: hollow_tiles::HollowTiles::new(device, geometry.hollow_tiles)?,
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

fn field_samplers(surface: &SceneSurface<'_>, wind_min_mip: f32) -> [wgpu::Sampler; 2] {
    let device = surface.device;
    let sampler = |mode, lod_min_clamp| {
        device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("water anisotropic field"),
            address_mode_u: mode,
            address_mode_v: mode,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            lod_min_clamp,
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
        sampler(wgpu::AddressMode::Repeat, wind_min_mip),
        sampler(wgpu::AddressMode::ClampToEdge, 0.0),
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
        bindings.extend(map.hollows_entry());
    }
    if let Some(interaction) = interaction {
        bindings.extend(interaction.sample_entries(7));
    }
    bindings
}

fn fallback_pattern(device: &DeviceSite) -> wgpu::TextureView {
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
}

fn pattern_entry(view: &wgpu::TextureView) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding: 12,
        resource: wgpu::BindingResource::TextureView(view),
    }
}

fn bind_optics<'a>(
    gather: Option<&'a Gather>,
    entries: &mut Vec<wgpu::BindGroupLayoutEntry>,
    bindings: &mut Vec<wgpu::BindGroupEntry<'a>>,
) {
    if let Some(gather) = gather {
        entries.push(optics::uniform(wgpu::ShaderStages::FRAGMENT));
        bindings.push(optics::entry(&gather.buffer));
    }
}

const UNUSED_UNDERSIDE: &str = "fn water_underside(input: VertexOut, n: vec3<f32>, v: vec3<f32>, variance: f32, foam: f32) -> vec4<f32> { return vec4(0.0); }";

fn surface_pipelines(
    surface: &SceneSurface<'_>,
    layout: &wgpu::BindGroupLayout,
    environment: &EnvironmentProducts,
    library: &str,
    map: Option<&MapGpu>,
    interaction: Option<&Interaction>,
    foam: Option<&Foam>,
    gather: Option<&Gather>,
    cache: Option<&Cache>,
    shore: bool,
    tidal_grid: bool,
    rain: bool,
    flow: Option<(thrum_water_model::FlowBandsProfile, u32)>,
    (wet_ground, standing_film): (Option<&thrum_render::wetness::PreparedWetness>, bool),
) -> (
    SurfacePipelines,
    Option<thrum_render::runtime::PipelineInput<wgpu::ShaderModule>>,
) {
    use std::fmt::Write;

    let under = gather.is_some_and(|gather| gather.underwater);
    let mut source = shader_source(
        surface,
        library,
        map,
        interaction.is_some(),
        foam,
        gather,
        rain,
        flow.is_some(),
    ) + &Cache::source(cache, tidal_grid);
    let wind_base_mip = if rain {
        thrum_sea::rain::WIND_BASE_MIP
    } else {
        0
    };
    writeln!(&mut source, "\nconst WIND_BASE_MIP = {wind_base_mip};").expect("write shader source");
    if rain {
        writeln!(
            &mut source,
            "\nconst RAIN_FIRST_LAYER = {}u;\nconst RAIN_SHARED_LAYERS = {}u;",
            thrum_sea::rain::FIRST_LAYER,
            thrum_sea::rain::SHARED_LAYERS + if flow.is_some() { 10 } else { 0 }
        )
        .expect("write shader source");
        source.push_str(super::RAIN_GAIN_WGSL);
        source.push_str(include_str!("rain.wgsl"));
    }
    let has_rivers = map.is_some_and(MapGpu::has_rivers);
    if has_rivers {
        let sea_receives_river = map.is_some_and(MapGpu::sea_receives_river);
        writeln!(
            &mut source,
            "\nconst SEA_RECEIVES_RIVER: bool = {sea_receives_river};"
        )
        .expect("write shader source");
        source.push_str(concat!(
            include_str!("map_gpu/river_carriage.wgsl"),
            include_str!("map_gpu/river_surface_waves.wgsl"),
            include_str!("map_gpu/river_surface_depth.wgsl"),
        ));
        source.push_str(if flow.is_some() {
            include_str!("flow/river_column.wgsl")
        } else {
            include_str!("map_gpu/river_column.wgsl")
        });
        source.push_str(include_str!("map_gpu/river_depth_refine.wgsl"));
        source.push_str(if flow.is_some() {
            include_str!("map_gpu/flow_depth_steps.wgsl")
        } else {
            include_str!("map_gpu/river_depth_steps.wgsl")
        });
        source.push_str(include_str!("map_gpu/river_depth_iteration.wgsl"));
        source.push_str(if flow.is_some() {
            include_str!("map_gpu/flow_depth_exit.wgsl")
        } else {
            include_str!("map_gpu/river_depth_exit.wgsl")
        });
        source.push_str(include_str!("map_gpu/river_depth_bracket.wgsl"));
        source.push_str(if rain {
            "\nfn river_rain_height(point: vec2<f32>, spacing: f32, row: SurfaceRow, motion: CascadeMotion) -> f32 {
                return rain_geometry(point, spacing, row, motion).x;
            }"
        } else {
            "\nfn river_rain_height(_point: vec2<f32>, _spacing: f32, _row: SurfaceRow, _motion: CascadeMotion) -> f32 {
                return 0.0;
            }"
        });
    }
    if let Some((profile, first)) = flow {
        source.push_str(&crate::flow::constants(profile, first));
        source.push_str(include_str!("flow/surface.wgsl"));
    }
    let module = |source: String| {
        surface
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(SURFACE_PASS),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            })
    };
    let sheet = SurfaceDraw::Sheet {
        mapped: map.is_some(),
        gather: gather.is_some(),
    };
    let river = SurfaceDraw::River {
        gather: gather.is_some(),
    };
    let ordinary_source = source.clone() + UNUSED_UNDERSIDE;
    #[cfg(not(target_arch = "wasm32"))]
    let sampled_textures = rain.then(|| sampled_textures(&ordinary_source, "water_fragment"));
    #[cfg(target_arch = "wasm32")]
    let sampled_textures = None;
    let ordinary = module(ordinary_source);
    let pipeline = create_pipeline(surface, layout, environment, &ordinary, sheet);
    let river_pipeline =
        has_rivers.then(|| create_pipeline(surface, layout, environment, &ordinary, river));
    let film = if let Some(wet) = wet_ground.filter(|_| standing_film || foam.is_some() && shore) {
        let shader = if standing_film {
            compose_rain_film(source.clone(), wet, foam.is_some(), rain)
        } else {
            compose_shore_support(&source, wet)
        };
        let shader = module(shader + UNUSED_UNDERSIDE);
        Some(create_pipeline(
            surface,
            layout,
            environment,
            &shader,
            SurfaceDraw::Film {
                standing_water: standing_film,
            },
        ))
    } else {
        (foam.is_some() && shore).then(|| {
            create_pipeline(
                surface,
                layout,
                environment,
                &ordinary,
                SurfaceDraw::Film {
                    standing_water: false,
                },
            )
        })
    };
    let shader = under.then(|| {
        let sheet = if has_rivers {
            include_str!("underwater/river_sheet.wgsl")
        } else {
            include_str!("underwater/rest_sheet.wgsl")
        };
        module(
            source
                + include_str!("underwater/cut_gpu.wgsl")
                + sheet
                + include_str!("underwater.wgsl"),
        )
    });
    let underwater = shader.as_ref().map(|shader| UnderwaterPipelines {
        surface: create_pipeline(surface, layout, environment, shader, sheet),
        river: has_rivers.then(|| create_pipeline(surface, layout, environment, shader, river)),
        through_lens: create_pipeline(surface, layout, environment, shader, SurfaceDraw::Lens),
    });
    (
        SurfacePipelines {
            sampled_textures,
            sheet: pipeline,
            river: river_pipeline,
            film,
            underwater,
        },
        shader,
    )
}

fn shader_source(
    surface: &SceneSurface<'_>,
    library: &str,
    map: Option<&MapGpu>,
    wake: bool,
    foam: Option<&Foam>,
    gather: Option<&Gather>,
    rain: bool,
    flow: bool,
) -> String {
    let gather = gather.is_some();
    let sun = if surface.sun_optical_depth {
        "fn water_sun(point: vec3<f32>) -> vec3<f32> { return sun_irradiance(point) * cloud_factor(point); }"
    } else {
        "fn water_sun(point: vec3<f32>) -> vec3<f32> { return sun_irradiance(point); }"
    };
    let has_hollows = map.is_some_and(|map| map.hollows_entry().is_some());
    // The opt-out composition reproduces the existing surface source byte for byte.
    let (rain_geometry, rain_normal) = if rain {
        (
            include_str!("rain_geometry_sum.wgsl"),
            include_str!("rain_normal_sum.wgsl"),
        )
    } else {
        ("", "")
    };
    let vertex_route = if has_hollows {
        "    if vertex.z < 0.0 { return hollows_vertex(vertex, body); }\n"
    } else {
        ""
    };
    let has_rivers = map.is_some_and(MapGpu::has_rivers);
    let (varying, vertex_output, fragment_entry, fragment_row) = if has_rivers {
        (
            "    @location(1) rest:vec2<f32>, @location(2) river:vec2<f32>,\n",
            "vec2(row.level,vertex.z)",
            include_str!("map_gpu/river_fragment_entry.wgsl"),
            "    var row=source;\n",
        )
    } else {
        (
            "    @location(1) rest:vec2<f32>, @location(2) velocity:vec3<f32>,\n",
            "f.velocity.xyz",
            "@fragment fn water_fragment(input:VertexOut)->Output {\n",
            "    var row=surface_pixel(input.rest,input.body);\n",
        )
    };
    format!(
        "{}\n{sun}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        library,
        super::VIEW_RECORDS,
        include_str!("fresnel.wgsl"),
        if rain {
            if flow {
                concat!(include_str!("sampling_weights.wgsl"),
                    include_str!("flow/sampling_wind.wgsl"), include_str!("sampling_shared.wgsl"))
            } else {
                concat!(include_str!("sampling_weights.wgsl"),
                    include_str!("sampling_wind.wgsl"), include_str!("sampling_shared.wgsl"))
            }
        } else {
            concat!(
                include_str!("sampling_weights.wgsl"),
                include_str!("sampling.wgsl")
            )
        },
        include_str!("field.wgsl"),
        Interaction::sample_source(3, 7, 8, wake),
        concat!(include_str!("carriage.wgsl"), include_str!("surface.wgsl")).to_owned()
            + if has_rivers {
                "    let motion = river_surface_motion(point, row);\n"
            } else {
                "    let motion = cascade_motion(point, row.current, row.track, water.carriage.w, water.carriage.y);\n"
            }
            + include_str!("surface_cascades.wgsl")
            + rain_geometry
            + if flow { include_str!("flow/geometry.wgsl") } else { "" }
            + include_str!("surface_vertex.wgsl")
            + if has_rivers { "    body: u32,\n" } else { "" }
            + if flow { "    flow_gains: vec2<f32>,\n" } else { "" }
            + "}\nstruct VertexOut {\n    "
            // Mixed maps draw the same geometry through regular and refined
            // entry points. Keep their raster positions invariant on Metal.
            + if has_rivers { "@invariant " } else { "" }
            + include_str!("surface_vertex_output.wgsl")
            + varying
            + include_str!("surface_vertex_entry.wgsl")
            + vertex_route
            + include_str!("surface_vertex_geometry.wgsl")
            + "    return VertexOut(clip,world,rest,"
            + vertex_output
            + include_str!("surface_vertex_return.wgsl")
            + include_str!("surface_fragment.wgsl")
            + fragment_entry
            + include_str!("surface_fragment_derivatives.wgsl")
            + fragment_row
            + if has_hollows && gather {
                include_str!("map_gpu/hollows_contact.wgsl")
            } else {
                "    let hollow_surface = false;\n"
            }
            + if has_rivers {
                "    let motion = river_surface_motion(input.rest, row);\n"
            } else {
                "    let motion = cascade_motion(input.rest, row.current, row.track, water.carriage.w, water.carriage.y);\n"
            }
            + include_str!("surface_shading.wgsl")
            + rain_normal
            + if flow { include_str!("flow/normal.wgsl") } else { "" }
            + include_str!("surface_lighting.wgsl")
            + if has_hollows && gather {
                include_str!("map_gpu/hollows_transport.wgsl")
            } else {
                ""
            }
            + include_str!("surface_atmosphere.wgsl")
            + if has_hollows && gather {
                include_str!("map_gpu/hollows_composite.wgsl")
            } else {
                ""
            }
            + include_str!("surface_composite.wgsl"),
        if gather {
            concat!(
                include_str!("optics/regions.wgsl"),
                include_str!("optics/paths.wgsl")
            )
        } else {
            include_str!("optics/disabled.wgsl")
        },
        map.map_or_else(
            || include_str!("open_surface.wgsl").to_owned(),
            |map| {
                let mut source = map.source(3)
                    + include_str!("mapped_surface.wgsl")
                    + if flow { include_str!("flow/surface_row.wgsl") } else { include_str!("map_gpu/surface_row.wgsl") }
                    + "        owned, swash, loss, loss_without_long, row.water.yz, map_wave_track(point, body, row.water.yz)"
                    + if flow { ", body, limit_flow(raw_flow, depth, swash));\n" } else if has_rivers { ", body);\n" } else { ");\n" }
                    + include_str!("mapped_surface_geometry.wgsl");
                if has_hollows {
                    let motion = if rain {
                        include_str!("map_gpu/hollows_rain_motion.wgsl")
                    } else {
                        include_str!("map_gpu/hollows_motion.wgsl")
                    };
                    source.push_str(include_str!("map_gpu/hollows_surface.wgsl"));
                    source.push_str(motion);
                    source.push_str(include_str!("map_gpu/hollows_surface_pixel.wgsl"));
                } else {
                    source.push_str(include_str!("mapped_surface_pixel.wgsl"));
                }
                source
            },
        ),
        if let Some(foam) = foam {
            foam.sample_source() + include_str!("foam/appearance.wgsl")
                + if flow { include_str!("foam/river_lace.wgsl") } else { include_str!("foam/lace.wgsl") }
                + include_str!("foam/film.wgsl")
                + if flow { include_str!("foam/river_appearance.wgsl") } else { "" }
        } else {
            "const FOAM_ENABLED: bool = false;
             fn foam_surface(_p: vec3<f32>, _r: SurfaceRow, _v: f32, _d: f32, _face: f32) -> vec4<f32> { return vec4(0.0); }"
                .into()
        },
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn sampled_textures(source: &str, entry: &str) -> usize {
    use wgpu::naga;
    let module = naga::front::wgsl::parse_str(source).expect("water WGSL");
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .expect("validate water surface");
    let index = module
        .entry_points
        .iter()
        .position(|point| point.name == entry)
        .expect("water surface entry");
    module
        .global_variables
        .iter()
        .filter(|(handle, global)| {
            matches!(
                module.types[global.ty].inner,
                naga::TypeInner::Image {
                    class: naga::ImageClass::Sampled { .. } | naga::ImageClass::Depth { .. },
                    ..
                }
            ) && !info.get_entry_point(index)[*handle].is_empty()
        })
        .count()
}

fn compose_shore_support(source: &str, wet: &thrum_render::wetness::PreparedWetness) -> String {
    const REACH: &str = "let reach = 0.175 * row.swash;";
    let (before, after) = source.split_once(REACH).expect("shore film support");
    let field = wet.shader(3, WET_GROUND_BINDING);
    format!(
        "{before}let authored = wet_shore(ground.xz);
    let reach = select(0.175 * row.swash, authored.y, authored.w > 0.0);{after}\n{field}"
    )
}

fn compose_rain_film(
    mut source: String,
    wet: &thrum_render::wetness::PreparedWetness,
    shore: bool,
    rain: bool,
) -> String {
    const APPEARANCE: &str = include_str!("foam/appearance.wgsl");
    const VERTEX: &str = "@vertex fn film_vertex(";
    const SHORE: &str = "    let footprint = length(ground - view.camera_xyz_time.xyz)";
    let (_, film) = APPEARANCE.split_once(VERTEX).expect("film entry point");
    let (header, tail) = film.split_once(SHORE).expect("film support");
    if shore {
        source.truncate(source.find(VERTEX).expect("shore film entry point"));
    } else {
        source.push_str("\n@group(1) @binding(0) var shore_depth: texture_depth_2d;\n");
    }
    source.push_str(VERTEX);
    source.push_str(header);
    source.push_str(
        "    let normal = film_normal(pixel.xy, ground);
    let wet = ground_wetness_footprint(ground, normal, film_footprint(ground, normal));
    if wet.hollows_ground {
        if wet.cover <= 0.0 { discard; }
        return standing_film(pixel.xy, ground, normal, wet);
    }
",
    );
    if shore {
        source.push_str(SHORE);
        source.push_str(tail);
    } else {
        source.push_str("    discard;\n    return Output(vec4(0.0), vec2(0.0), 0.0);\n}\n");
    }
    source.push_str(&wet.shader(3, WET_GROUND_BINDING));
    source.push_str(include_str!("wet_film.wgsl"));
    source.push_str(if rain {
        include_str!("wet_film_rings.wgsl")
    } else {
        "fn film_rings(_ground: vec3<f32>, _n: vec3<f32>, _footprint: f32) -> vec2<f32> { return vec2(0.0); }"
    });
    source
}

#[derive(Clone, Copy)]
enum SurfaceDraw {
    Sheet { mapped: bool, gather: bool },
    River { gather: bool },
    Film { standing_water: bool },
    Lens,
}

fn create_pipeline(
    surface: &SceneSurface<'_>,
    layout: &wgpu::BindGroupLayout,
    environment: &EnvironmentProducts,
    shader: &wgpu::ShaderModule,
    draw: SurfaceDraw,
) -> wgpu::RenderPipeline {
    let device = surface.device;
    let (pass, vertex, fragment) = match draw {
        SurfaceDraw::Sheet { gather, .. } => {
            (declaration(gather), "water_vertex", "water_fragment")
        }
        SurfaceDraw::River { gather } => (declaration(gather), "water_vertex", "river_fragment"),
        SurfaceDraw::Film { standing_water } => (
            film_declaration(standing_water),
            "film_vertex",
            "film_fragment",
        ),
        SurfaceDraw::Lens => (
            declaration(true),
            "underwater_vertex",
            "underwater_fragment",
        ),
    };
    let film = matches!(draw, SurfaceDraw::Film { .. });
    let underwater = matches!(draw, SurfaceDraw::Lens);
    let mapped = matches!(
        draw,
        SurfaceDraw::Sheet { mapped: true, .. }
            | SurfaceDraw::River { .. }
            | SurfaceDraw::Film { .. }
    );
    let name = if underwater {
        super::underwater::DRAW
    } else {
        &pass.name
    };
    let inputs = pass.input_layout(device);
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(name),
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
                    0 if mapped || underwater => Some(wgpu::BlendState::ALPHA_BLENDING),
                    2 if film || underwater => {
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
                write_mask: if (film || underwater) && i == 1 {
                    wgpu::ColorWrites::empty()
                } else {
                    wgpu::ColorWrites::ALL
                },
            })
        })
        .collect();
    device.render(&wgpu::RenderPipelineDescriptor {
        label: Some(name),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some(vertex),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: if film || underwater {
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
            depth_write_enabled: Some(!underwater),
            depth_compare: Some(if underwater {
                wgpu::CompareFunction::Always
            } else {
                wgpu::CompareFunction::GreaterEqual
            }),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(fragment),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &targets,
        }),
        multiview_mask: None,
        cache: None,
    })
}

pub(super) struct WaterDraw {
    pub(super) indices: Range<u32>,
    pub(super) river: bool,
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
    hollow_tiles: Vec<hollow_tiles::Rectangle>,
    rest_mesh: Option<super::underwater::RestMesh>,
}

fn has_tidal_join(map: &thrum_water_model::Map) -> bool {
    use thrum_water_model::BodyKind;
    (0..map_gpu::BODY_LIMIT)
        .map_while(|body| map.body(body))
        .any(|body| {
            matches!(&body.kind, BodyKind::Chart(chart)
            if chart.ends.iter().any(|end| end.mouth && map.body(end.body).is_some_and(|joined|
                matches!(joined.kind, BodyKind::Sea))))
        })
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "GPU vertices and at most 16 body slots"
)]
fn surface_mesh(
    map: Option<&thrum_water_model::Map>,
    tidal_grid: bool,
    retain_rest_mesh: bool,
) -> Geometry {
    let mut geometry = Geometry {
        rest_mesh: retain_rest_mesh.then(super::underwater::RestMesh::default),
        ..Geometry::default()
    };
    let Some(map) = map else {
        geometry.vertices = mesh(geometry.rest_mesh.as_mut());
        return geometry;
    };
    let surf = map.surf_profile().is_some();
    for (body, receiver) in map_gpu::river_receivers(map).into_iter().enumerate() {
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
            let first_level = i32::from(tidal_grid);
            for level in first_level..17 {
                let half_width = match (surf, level) {
                    (true, 0) => SURF_FINE_RING_HALF_WIDTH,
                    (true, 1) => SURF_HALF_METRE_RING_HALF_WIDTH,
                    (true, 2) => SURF_METRE_RING_HALF_WIDTH,
                    (true, 3) => SURF_TWO_METRE_RING_HALF_WIDTH,
                    (true, 4) => SURF_FOUR_METRE_RING_HALF_WIDTH,
                    // Keep the mesh transition apart from the coarse foam window's edge.
                    (true, 5) => SURF_EIGHT_METRE_RING_HALF_WIDTH,
                    _ if tidal_grid => TIDAL_RING_HALF_WIDTHS
                        .get(usize::try_from(level - first_level).expect("tidal ring"))
                        .copied()
                        .unwrap_or(SEA_RING_HALF_WIDTH),
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
                    river: receiver && level <= 3,
                    offshore: (radius < 524_288.0).then_some(OffshoreRing {
                        radius,
                        levels,
                        bed,
                    }),
                });
                previous_half_width = half_width;
            }
        } else if let thrum_water_model::BodyKind::Hollows(hollows) = &declaration.kind {
            geometry
                .hollow_tiles
                .push(hollow_tiles::Rectangle::new(first, hollows));
            geometry.hollows(hollows, (body + 1) as f32);
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
                river: map_gpu::sampled_river(map, body) || receiver,
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
    #[expect(
        clippy::cast_possible_truncation,
        reason = "bounded mesh using GPU metres"
    )]
    fn hollows(&mut self, hollows: &thrum_water_model::Hollows, slot: f32) {
        let ([x, z], cell, [width, height]) = hollows.lattice();
        if let Some(rest) = &mut self.rest_mesh {
            rest.hollows(
                [x, z],
                cell,
                [width, height],
                u32::try_from(self.vertices.len()).expect("bounded mesh"),
            );
        }
        for row in 0..height {
            for column in 0..width {
                for (corner, [dx, dz]) in [[0, 0], [1, 0], [0, 1], [1, 1]].into_iter().enumerate() {
                    self.vertices.push([
                        ((column + dx) as f64).mul_add(cell, x) as f32,
                        ((row + dz) as f64).mul_add(cell, z) as f32,
                        -(corner as f32 + 1.0),
                        slot,
                    ]);
                }
            }
        }
    }

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
        if let Some(rest) = &mut self.rest_mesh {
            rest.grid(
                start,
                end,
                spacing,
                inner_half_width,
                body == 0.0,
                super::underwater::Vertices::Indexed {
                    first: base,
                    surf_diagonal: surf,
                },
            );
        }
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
fn mesh(mut rest: Option<&mut super::underwater::RestMesh>) -> Vec<[f32; 4]> {
    let mut vertices = Vec::new();
    let half_width = i16::try_from(SEA_RING_HALF_WIDTH).expect("bounded sea ring");
    // Nested world-snapped square rings. Every outer boundary has the next ring's
    // even vertices; boundary heights are interpolated at the shared coarse edge.
    for level in 0_i32..17 {
        let spacing = 0.25 * 2.0_f32.powi(level);
        let inner = if level == 0 { 0 } else { half_width / 2 };
        if let Some(rest) = &mut rest {
            rest.grid(
                [-i32::from(half_width); 2],
                [i32::from(half_width); 2],
                spacing,
                i32::from(inner),
                true,
                super::underwater::Vertices::Expanded {
                    first: u32::try_from(vertices.len()).expect("bounded mesh"),
                },
            );
        }
        for z in -half_width..half_width {
            for x in -half_width..half_width {
                if (-inner..inner).contains(&x) && (-inner..inner).contains(&z) {
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
