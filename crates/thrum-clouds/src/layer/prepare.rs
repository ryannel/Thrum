use std::fmt::Write as _;

use super::{
    AfterOpaqueKind, COMPOSE, Error, FIELD, Layer, SceneSurface, SkyProfile, TRACE, Uniform,
    WeatherMap, declaration,
};
use crate::{
    ground::{
        GroundField,
        band::Band,
        lighting::{self, FIRST, LightTable},
    },
    noise::{DETAIL_SIZE, Noise},
};
use thrum_render::{binding, runtime::PipelineInput};

fn validate_profile(profile: SkyProfile, max_extent: u32) -> Result<(), Error> {
    if profile.extent.contains(&0)
        || !profile.step.is_finite()
        || profile.step <= 0.0
        || profile.light_samples > 3
        || ![1, 2, 4].contains(&profile.refresh_stride)
    {
        return Err(Error::Unsupported("cloud layer extent or march controls"));
    }
    if profile.extent.into_iter().any(|side| side > max_extent) {
        return Err(Error::DeviceLimit("cloud layer texture extent"));
    }
    Ok(())
}

pub(super) fn transport_textures(
    device: &wgpu::Device,
    extent: [u32; 2],
    ground: bool,
) -> [wgpu::Texture; 2] {
    std::array::from_fn(|_| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("cloud layer transport"),
            size: wgpu::Extent3d {
                width: extent[0].div_ceil(2),
                height: extent[1].div_ceil(2),
                depth_or_array_layers: if ground { 4 } else { 1 },
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        })
    })
}

fn prepare_detail_noise(
    device: &thrum_render::runtime::DeviceSite,
    seed: u32,
    accepted: Option<&Layer>,
) -> (Noise, Option<wgpu::CommandBuffer>) {
    if let Some(layer) = accepted {
        (layer.detail_noise.clone(), None)
    } else {
        let noise = Noise::new(device, DETAIL_SIZE, "cloud detail noise");
        let command = noise.prepare(device, seed, "detail");
        (noise, Some(command))
    }
}

impl Layer {
    #[expect(
        clippy::too_many_lines,
        reason = "prepare the layer's texture bindings and fixed pipelines together"
    )]
    pub(crate) fn new(
        surface: &SceneSurface<'_>,
        weather: &WeatherMap,
        sun: &crate::SunPath,
        target: (&wgpu::Texture, &wgpu::Buffer),
        placed_map: Option<&wgpu::Texture>,
        profile: SkyProfile,
        shapes: Option<&crate::shapes::Shapes>,
        accepted: Option<&Self>,
        ground: Option<(&GroundField, &LightTable, &Band, &wgpu::Buffer)>,
    ) -> Result<(Self, Option<wgpu::CommandBuffer>), Error> {
        validate_profile(profile, surface.limits.max_texture_dimension_2d)?;
        let device = surface.device;
        let (detail_noise, initialization) = prepare_detail_noise(device, weather.seed(), accepted);
        let environment = if ground.is_some() && shapes.is_some() {
            surface.environment_transport_products()
        } else {
            surface.environment_products()
        }
        .ok_or(Error::Unsupported(
            "cloud Sky requires the physical environment",
        ))?;
        let params = ground.map_or_else(
            || super::parameter_buffer(device),
            |(_, _, _, params)| params.clone(),
        );
        let lighting = super::light::Lighting::new(surface, weather, &environment)?;
        let ground_stops = ground.is_some();
        let render_capacity = surface.render_capacity.unwrap_or(profile.extent);
        let capacity = if ground_stops {
            std::array::from_fn(|i| profile.extent[i].max(render_capacity[i]))
        } else {
            profile.extent
        };
        let retained = accepted.filter(|layer| {
            let texture = &layer.textures[0];
            [texture.width(), texture.height()] == capacity.map(|side| side.div_ceil(2))
        });
        let textures = retained.map_or_else(
            || transport_textures(device, capacity, ground_stops),
            |layer| layer.textures.clone(),
        );
        let uniform_entry = binding::uniform(
            0,
            wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::FRAGMENT,
            size_of::<Uniform>() as u64,
        );
        let texture_entry = |binding, storage, filterable| {
            let visibility = wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::FRAGMENT;
            if storage {
                binding::storage_texture(
                    binding,
                    visibility,
                    wgpu::StorageTextureAccess::WriteOnly,
                    wgpu::TextureFormat::Rgba16Float,
                    wgpu::TextureViewDimension::D2Array,
                )
            } else {
                binding::texture(
                    binding,
                    visibility,
                    wgpu::TextureSampleType::Float { filterable },
                    wgpu::TextureViewDimension::D2Array,
                )
            }
        };
        let mut trace_entries = vec![
            uniform_entry,
            texture_entry(1, false, false),
            texture_entry(2, false, false),
            texture_entry(3, true, false),
            texture_entry(4, true, false),
            texture_entry(5, false, true),
            texture_entry(6, false, true),
            texture_entry(7, false, true),
            binding::storage(9, wgpu::ShaderStages::COMPUTE, true, 0),
            binding::storage(10, wgpu::ShaderStages::COMPUTE, true, 0),
            binding::texture(
                11,
                wgpu::ShaderStages::COMPUTE,
                wgpu::TextureSampleType::Float { filterable: true },
                wgpu::TextureViewDimension::D3,
            ),
            binding::texture(
                12,
                wgpu::ShaderStages::COMPUTE,
                wgpu::TextureSampleType::Float { filterable: true },
                wgpu::TextureViewDimension::D3,
            ),
            binding::sampler(
                13,
                wgpu::ShaderStages::COMPUTE,
                wgpu::SamplerBindingType::Filtering,
            ),
            binding::uniform(8, wgpu::ShaderStages::COMPUTE, 96),
        ];
        if ground.is_some() {
            trace_entries.extend(GroundField::layout(FIRST));
            trace_entries.extend(LightTable::layout());
        }
        let trace_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(TRACE),
            entries: &trace_entries,
        });
        let views: Vec<_> = weather.textures()[..2]
            .iter()
            .map(|t| {
                t.create_view(&wgpu::TextureViewDescriptor {
                    dimension: Some(wgpu::TextureViewDimension::D2Array),
                    ..Default::default()
                })
            })
            .chain(textures.iter().map(super::stop_view))
            .chain(sun.products().iter().map(|t| {
                t.create_view(&wgpu::TextureViewDescriptor {
                    dimension: Some(wgpu::TextureViewDimension::D2Array),
                    ..wgpu::TextureViewDescriptor::default()
                })
            }))
            .collect();
        let mut entries = vec![wgpu::BindGroupEntry {
            binding: 0,
            resource: params.as_entire_binding(),
        }];
        entries.extend(views.iter().enumerate().map(|(i, v)| wgpu::BindGroupEntry {
            binding: [1, 2, 3, 4, 5, 6, 7][i],
            resource: wgpu::BindingResource::TextureView(v),
        }));
        entries.push(wgpu::BindGroupEntry {
            binding: 8,
            resource: target.1.as_entire_binding(),
        });
        entries.push(wgpu::BindGroupEntry {
            binding: 9,
            resource: lighting.phase.as_entire_binding(),
        });
        entries.push(wgpu::BindGroupEntry {
            binding: 10,
            resource: lighting.ambient.as_entire_binding(),
        });
        entries.extend([
            sun.shape_noise.bind(11),
            detail_noise.bind(12),
            wgpu::BindGroupEntry {
                binding: 13,
                resource: wgpu::BindingResource::Sampler(&sun.shape_noise.sampler),
            },
        ]);
        if let Some((field, phases, band, _)) = ground {
            entries.extend(field.entries(FIRST));
            entries.extend(phases.entries(band));
        }
        #[cfg(not(target_arch = "wasm32"))]
        let trace_bindings = super::resize::TraceBindings::new(&trace_layout, &entries);
        let trace_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(TRACE),
            layout: &trace_layout,
            entries: &entries,
        });
        let input = declaration(TRACE, AfterOpaqueKind::Compute, Vec::new()).input_layout(device);
        let mut layouts = vec![Some(&environment.layout), Some(&input), Some(&trace_layout)];
        if let Some(shapes) = shapes {
            layouts.push(Some(&shapes.layout));
        }
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(TRACE),
            bind_group_layouts: &layouts,
            immediate_size: 0,
        });
        let source = trace_shader(&environment, shapes.is_some(), ground_stops);
        let overlay = surface.sky_overlay_target().map(|target| {
            super::overlay::Overlay::new(
                surface,
                target,
                &environment,
                &trace_layout,
                shapes,
                ground.is_some(),
            )
        });
        let trace = trace_pipeline(device, &source, &pipeline_layout);
        let (field_layout, field_group) = field_bindings(
            device,
            weather,
            target.0,
            placed_map,
            environment.exposure_buffers(),
        );
        let (compose, compose_group, compose_layout) = compose(
            surface,
            &params,
            &textures,
            uniform_entry,
            |binding, storage| texture_entry(binding, storage, false),
            &input,
        );
        let field_pipeline = field_pipeline(surface, &compose_layout, &field_layout);
        let temporal = super::temporal::Temporal::new(
            surface,
            &params,
            &textures,
            &compose_layout,
            retained.map(|layer| &layer.temporal),
        );
        #[cfg(not(target_arch = "wasm32"))]
        let resize_bindings = super::resize::Bindings {
            trace: trace_bindings,
            compose_layout,
        };
        Ok((
            Self {
                quality: accepted.map_or(
                    super::Quality {
                        stride: profile.refresh_stride,
                        light_samples: profile.light_samples,
                        layer_percent: 100,
                        step: profile.step,
                        cap: super::MAX_TRACE_SAMPLES,
                    },
                    |layer| layer.quality,
                ),
                environment,
                lighting,
                temporal,
                committed: retained.and_then(|layer| layer.committed),
                pending: None,
                ordinal: accepted.map_or(0, |layer| layer.ordinal),
                trace_stride: accepted.map_or(1, |layer| layer.trace_stride),
                params,
                queue: surface.queue.clone(),
                textures,
                detail_noise,
                trace,
                compose,
                field_pipeline,
                field_overlay: crate::FieldOverlay::Off,
                trace_group,
                #[cfg(not(target_arch = "wasm32"))]
                resize_bindings,
                compose_group,
                field_group,
                has_placed_map: placed_map.is_some(),
                per_pixel_stops: ground_stops,
                profile_extent: profile.extent,
                render_extent: render_capacity,
                overlay,
            },
            initialization,
        ))
    }
}

fn trace_pipeline(
    device: &thrum_render::runtime::DeviceSite,
    source: &str,
    pipeline_layout: &wgpu::PipelineLayout,
) -> wgpu::ComputePipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(TRACE),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    device.compute(&wgpu::ComputePipelineDescriptor {
        label: Some(TRACE),
        layout: Some(pipeline_layout),
        module: &shader,
        entry_point: Some("trace"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    })
}

const TRACE_SHADER: &str = concat!(
    include_str!("../layer.wgsl"),
    include_str!("billow.wgsl"),
    "@group(1) @binding(0) var scene_depth:texture_depth_2d;\n",
    include_str!("trace_library.wgsl"),
    include_str!("stop_pixels.wgsl"),
    include_str!("trace_stops.wgsl"),
    include_str!("trace.wgsl"),
);
const OVERLAY_SHADER: &str = concat!(
    include_str!("../layer.wgsl"),
    include_str!("billow.wgsl"),
    include_str!("trace_library.wgsl"),
    "fn cloud_placed(point: vec3<f32>, footprint: f32) -> vec4<f32> { return cloud_coarse_placed(point, footprint); }",
    include_str!("overlay.wgsl"),
);

pub(super) fn trace_shader(
    environment: &thrum_render::atmosphere::EnvironmentProducts,
    shapes: bool,
    ground: bool,
) -> String {
    let source = layer_shader(environment, TRACE_SHADER, shapes, ground, false);
    format!("{source}\nconst SURFACE_STOPS: bool = {ground};")
}

fn layer_shader(
    environment: &thrum_render::atmosphere::EnvironmentProducts,
    transport: &str,
    shapes: bool,
    ground: bool,
    coarse: bool,
) -> String {
    let placed = placed_source(3, shapes, !coarse, coarse || ground);
    format!(
        "{}\n{}\n{transport}\n{}\n{}\n{}\n{}\n{}\n{}\n{placed}\n{}",
        environment.shader_source,
        include_str!("params.wgsl"),
        density_and_light_source(2),
        include_str!("light.wgsl"),
        super::SCATTERING,
        include_str!("transport.wgsl"),
        super::sun_source(2),
        include_str!("../erosion.wgsl"),
        lighting::trace_source(ground),
    )
}

// Generate the coarse sampler for the view volume and the Ground handover
// from the same code used by the overlay.
pub(crate) fn placed_source(group: u32, shapes: bool, detailed: bool, coarse: bool) -> String {
    let mut source = if shapes {
        crate::shapes::shader(group)
    } else {
        String::new()
    };
    let variants = [
        (
            detailed,
            "cloud_placed",
            "60.0 * cloud.physical.y",
            include_str!("placed.wgsl"),
        ),
        (
            coarse,
            "cloud_coarse_placed",
            "0.0",
            include_str!("placed_coarse.wgsl"),
        ),
    ];
    for (_, name, edge, erosion) in variants.into_iter().filter(|variant| variant.0) {
        writeln!(
            source,
            "\nfn {name}(point: vec3<f32>, footprint: f32) -> vec4<f32> {{"
        )
        .expect("shader function header");
        if shapes {
            writeln!(
                source,
                "    let medium = placed_sample(point - vec3(cloud.weather[0].x, 0.0, cloud.weather[0].y), footprint, {edge});"
            )
            .expect("placed-volume sample");
            source.push_str(include_str!("placed_column.wgsl"));
            source.push_str(erosion);
        } else {
            source.push_str("    return vec4(0.0);\n}\n");
        }
    }
    source
}

pub(super) fn overlay_source(
    environment: &thrum_render::atmosphere::EnvironmentProducts,
    shapes: bool,
    ground: bool,
) -> String {
    layer_shader(environment, OVERLAY_SHADER, shapes, ground, true)
}

fn field_bindings(
    device: &thrum_render::runtime::DeviceSite,
    weather: &WeatherMap,
    sun: &wgpu::Texture,
    placed_map: Option<&wgpu::Texture>,
    exposure: &[wgpu::Buffer; 2],
) -> (PipelineInput<wgpu::BindGroupLayout>, [wgpu::BindGroup; 2]) {
    use wgpu::TextureViewDimension::{D2, D2Array};
    let fields = [
        (&weather.textures()[0], D2Array),
        (&weather.textures()[1], D2Array),
        (placed_map.unwrap_or(&weather.textures()[0]), D2Array),
        (sun, D2),
    ];
    let mut entries: Vec<_> = fields
        .map(|(_, dimension)| dimension)
        .into_iter()
        .zip(0..)
        .map(|(dimension, binding)| {
            binding::texture(
                binding,
                wgpu::ShaderStages::FRAGMENT,
                wgpu::TextureSampleType::Float { filterable: false },
                dimension,
            )
        })
        .collect();
    entries.push(binding::uniform(4, wgpu::ShaderStages::FRAGMENT, 16));
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("cloud internal fields"),
        entries: &entries,
    });
    let views = fields.map(|(texture, dimension)| {
        texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(dimension),
            ..Default::default()
        })
    });
    let groups = std::array::from_fn(|side| {
        let mut entries: Vec<_> = views
            .iter()
            .zip(0..)
            .map(|(view, binding)| wgpu::BindGroupEntry {
                binding,
                resource: wgpu::BindingResource::TextureView(view),
            })
            .collect();
        entries.push(wgpu::BindGroupEntry {
            binding: 4,
            resource: exposure[side].as_entire_binding(),
        });
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("cloud internal fields"),
            layout: &layout,
            entries: &entries,
        })
    });
    (layout, groups)
}

fn compose(
    surface: &SceneSurface<'_>,
    params: &wgpu::Buffer,
    textures: &[wgpu::Texture; 2],
    uniform_entry: wgpu::BindGroupLayoutEntry,
    texture_entry: impl Fn(u32, bool) -> wgpu::BindGroupLayoutEntry,
    input: &wgpu::BindGroupLayout,
) -> (wgpu::RenderPipeline, wgpu::BindGroup, wgpu::BindGroupLayout) {
    let device = surface.device;
    let mut sampled = [texture_entry(1, false), texture_entry(2, false)];
    for entry in &mut sampled {
        entry.ty = wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: false },
            view_dimension: wgpu::TextureViewDimension::D2Array,
            multisampled: false,
        };
    }
    let compose_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some(COMPOSE),
        entries: &[uniform_entry, sampled[0], sampled[1]],
    });
    let compose_group = super::temporal::compose_group(device, params, textures, &compose_layout);
    let compose_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(COMPOSE),
        bind_group_layouts: &[
            Some(surface.view_layout),
            Some(input),
            Some(&compose_layout),
        ],
        immediate_size: 0,
    });
    let compose_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(COMPOSE),
        source: wgpu::ShaderSource::Wgsl(
            format!(
                "{}\n{}\n{}",
                include_str!("params.wgsl"),
                concat!(include_str!("view.wgsl"), include_str!("stop_pixels.wgsl")),
                include_str!("compose.wgsl")
            )
            .into(),
        ),
    });
    let mut targets = surface.color_formats.map(|format| {
        Some(wgpu::ColorTargetState {
            format,
            blend: Some(wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::SrcAlpha,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent::REPLACE,
            }),
            write_mask: wgpu::ColorWrites::ALL,
        })
    });
    if let Some(motion) = &mut targets[1] {
        motion.blend = Some(wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent::REPLACE,
        });
    }
    if let Some(reactive) = &mut targets[2] {
        reactive.blend = Some(wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Max,
            },
            alpha: wgpu::BlendComponent::REPLACE,
        });
    }
    let compose = device.render(&wgpu::RenderPipelineDescriptor {
        label: Some(COMPOSE),
        layout: Some(&compose_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &compose_shader,
            entry_point: Some("vertex"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &compose_shader,
            entry_point: Some("fragment"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &targets,
        }),
        multiview_mask: None,
        cache: None,
    });
    (compose, compose_group, compose_layout.into_raw())
}

fn field_pipeline(
    surface: &SceneSurface<'_>,
    layer_layout: &wgpu::BindGroupLayout,
    field_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let device = surface.device;
    let input = declaration(FIELD, AfterOpaqueKind::PreTone, Vec::new()).input_layout(device);
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(FIELD),
        bind_group_layouts: &[
            Some(surface.view_layout),
            Some(&input),
            Some(layer_layout),
            Some(field_layout),
        ],
        immediate_size: 0,
    });
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(FIELD),
        source: wgpu::ShaderSource::Wgsl(
            concat!(
                include_str!("params.wgsl"),
                "\n",
                include_str!("field_overlay.wgsl")
            )
            .into(),
        ),
    });
    device.render(&wgpu::RenderPipelineDescriptor {
        label: Some(FIELD),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vertex"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fragment"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: surface.color_formats[0],
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

pub(crate) fn density_and_light_source(group: u32) -> String {
    format!(
        "@group({group}) @binding(1) var columns: texture_2d_array<f32>;
         @group({group}) @binding(2) var optical: texture_2d_array<f32>;
         @group({group}) @binding(9) var<storage, read> cloud_phase: array<f32>;
         @group({group}) @binding(10) var<storage, read> cloud_ambient: array<vec4<f32>, 2>;
         {}\n{}",
        include_str!("column.wgsl"),
        include_str!("source_light.wgsl"),
    )
}
