use super::{
    AfterOpaqueKind, COMPOSE, Error, Layer, SceneSurface, SkyProfile, TRACE, Uniform, WeatherMap,
    declaration,
};
use crate::noise::{DETAIL_SIZE, Noise};
use thrum_render::binding;

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

fn transport_textures(device: &wgpu::Device, extent: [u32; 2]) -> [wgpu::Texture; 2] {
    std::array::from_fn(|_| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("cloud layer transport"),
            size: wgpu::Extent3d {
                width: extent[0].div_ceil(2),
                height: extent[1].div_ceil(2),
                depth_or_array_layers: 1,
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
    pub(crate) fn new(
        surface: &SceneSurface<'_>,
        weather: &WeatherMap,
        sun: &crate::SunPath,
        mapping: &wgpu::Buffer,
        profile: SkyProfile,
        shapes: Option<&crate::shapes::Shapes>,
        accepted: Option<&Self>,
    ) -> Result<(Self, Option<wgpu::CommandBuffer>), Error> {
        validate_profile(profile, surface.limits.max_texture_dimension_2d)?;
        let device = surface.device;
        let (detail_noise, initialization) = prepare_detail_noise(device, weather.seed(), accepted);
        let environment = surface.environment_products().ok_or(Error::Unsupported(
            "cloud Sky requires the physical environment",
        ))?;
        let params = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cloud layer parameters"),
            size: size_of::<Uniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let lighting = super::light::Lighting::new(surface, weather, &environment)?;
        let textures = accepted.map_or_else(
            || transport_textures(device, profile.extent),
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
                    wgpu::TextureViewDimension::D2,
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
        let trace_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(TRACE),
            entries: &[
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
            ],
        });
        let views: Vec<_> = weather.textures()[..2]
            .iter()
            .map(|t| {
                t.create_view(&wgpu::TextureViewDescriptor {
                    dimension: Some(wgpu::TextureViewDimension::D2Array),
                    ..Default::default()
                })
            })
            .chain(
                textures
                    .iter()
                    .map(|t| t.create_view(&wgpu::TextureViewDescriptor::default())),
            )
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
            resource: mapping.as_entire_binding(),
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
        let source = if shapes.is_some() {
            shaped_trace_shader
        } else {
            trace_shader
        }(&environment);
        let overlay = surface.sky_overlay_target().map(|target| {
            super::overlay::Overlay::new(surface, target, &environment, &trace_layout, shapes)
        });
        let trace = trace_pipeline(device, &source, &pipeline_layout);
        let (compose, compose_group, compose_layout) = compose(
            surface,
            &params,
            &textures,
            uniform_entry,
            |binding, storage| texture_entry(binding, storage, false),
            &input,
        );
        let temporal = super::temporal::Temporal::new(
            surface,
            &params,
            &textures,
            &compose_layout,
            accepted.map(|layer| &layer.temporal),
        );
        Ok((
            Self {
                profile,
                environment,
                lighting,
                temporal,
                committed: accepted.and_then(|layer| layer.committed),
                pending: None,
                ordinal: accepted.map_or(0, |layer| layer.ordinal),
                trace_stride: accepted.map_or(1, |layer| layer.trace_stride),
                params,
                queue: surface.queue.clone(),
                textures,
                detail_noise,
                trace,
                compose,
                trace_group,
                compose_group,
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

const UNPLACED: &str =
    "fn cloud_placed(point: vec3<f32>, footprint: f32) -> vec4<f32> { return vec4(0.0); }";
const TRACE_SHADER: &str = concat!(
    include_str!("../layer.wgsl"),
    "@group(1) @binding(0) var scene_depth:texture_depth_2d;\n",
    include_str!("trace_library.wgsl"),
    include_str!("trace.wgsl"),
);
const OVERLAY_SHADER: &str = concat!(
    include_str!("../layer.wgsl"),
    include_str!("trace_library.wgsl"),
    include_str!("overlay.wgsl"),
);

pub(super) fn trace_shader(environment: &thrum_render::atmosphere::EnvironmentProducts) -> String {
    trace_source(environment, TRACE_SHADER, UNPLACED)
}

fn trace_source(
    environment: &thrum_render::atmosphere::EnvironmentProducts,
    trace: &str,
    placed: &str,
) -> String {
    format!(
        "{}\n{}\n{trace}\n{}\n{}\n{}\n{}\n{}\n{placed}",
        environment.shader_source,
        include_str!("params.wgsl"),
        include_str!("light.wgsl"),
        include_str!("scattering.wgsl"),
        include_str!("transport.wgsl"),
        include_str!("sun.wgsl"),
        include_str!("../erosion.wgsl"),
    )
}

fn shaped_trace_shader(environment: &thrum_render::atmosphere::EnvironmentProducts) -> String {
    trace_source(
        environment,
        TRACE_SHADER,
        &format!(
            "{}\n{}",
            crate::shapes::shader(3),
            concat!(
                include_str!("placed_column.wgsl"),
                include_str!("placed.wgsl"),
                "fn cloud_placed_edge_footprint() -> f32 { return 140.0 * cloud.physical.y; }\n"
            )
        ),
    )
}

pub(super) fn overlay_source(
    environment: &thrum_render::atmosphere::EnvironmentProducts,
    shapes: bool,
) -> String {
    if shapes {
        trace_source(
            environment,
            OVERLAY_SHADER,
            &format!(
                "{}\n{}",
                crate::shapes::shader(3),
                concat!(
                    include_str!("placed_column.wgsl"),
                    include_str!("placed_coarse.wgsl"),
                    "fn cloud_placed_edge_footprint() -> f32 { return 0.0; }\n"
                ),
            ),
        )
    } else {
        trace_source(environment, OVERLAY_SHADER, UNPLACED)
    }
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
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        };
    }
    let compose_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some(COMPOSE),
        entries: &[uniform_entry, sampled[0], sampled[1]],
    });
    let compose_views = textures
        .each_ref()
        .map(|t| t.create_view(&wgpu::TextureViewDescriptor::default()));
    let compose_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(COMPOSE),
        layout: &compose_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: params.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&compose_views[0]),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(&compose_views[1]),
            },
        ],
    });
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
                include_str!("view.wgsl"),
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
