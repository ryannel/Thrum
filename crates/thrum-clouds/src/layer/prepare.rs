use super::{
    AfterOpaqueKind, COMPOSE, Error, Layer, SceneSurface, SkyProfile, TRACE, Uniform, WeatherMap,
    declaration,
};

impl Layer {
    pub(crate) fn new(
        surface: &SceneSurface<'_>,
        weather: &WeatherMap,
        sun: &crate::SunPath,
        profile: SkyProfile,
    ) -> Result<Self, Error> {
        if profile.extent.contains(&0)
            || profile
                .extent
                .iter()
                .any(|&v| v > surface.limits.max_texture_dimension_2d)
            || !profile.step.is_finite()
            || profile.step <= 0.0
            || profile.light_samples > 3
            || ![1, 2, 4].contains(&profile.refresh_stride)
        {
            return Err(Error::Unsupported("cloud layer extent or march controls"));
        }
        let device = surface.device;
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
        let textures = std::array::from_fn(|_| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some("cloud layer transport"),
                size: wgpu::Extent3d {
                    width: profile.extent[0].div_ceil(2),
                    height: profile.extent[1].div_ceil(2),
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
        });
        let uniform_entry = wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(size_of::<Uniform>() as u64),
            },
            count: None,
        };
        let texture_entry = |binding, storage| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::FRAGMENT,
            ty: if storage {
                wgpu::BindingType::StorageTexture {
                    access: wgpu::StorageTextureAccess::WriteOnly,
                    format: wgpu::TextureFormat::Rgba16Float,
                    view_dimension: wgpu::TextureViewDimension::D2,
                }
            } else {
                wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2Array,
                    multisampled: false,
                }
            },
            count: None,
        };
        let trace_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(TRACE),
            entries: &[
                uniform_entry,
                texture_entry(1, false),
                texture_entry(2, false),
                texture_entry(3, true),
                texture_entry(4, true),
                texture_entry(5, false),
                texture_entry(6, false),
                texture_entry(7, false),
                super::light::storage_entry(9, true),
                super::light::storage_entry(10, true),
                wgpu::BindGroupLayoutEntry {
                    binding: 8,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(96),
                    },
                    count: None,
                },
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
            resource: sun.mapping().as_entire_binding(),
        });
        entries.push(wgpu::BindGroupEntry {
            binding: 9,
            resource: lighting.phase.as_entire_binding(),
        });
        entries.push(wgpu::BindGroupEntry {
            binding: 10,
            resource: lighting.ambient.as_entire_binding(),
        });
        let trace_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(TRACE),
            layout: &trace_layout,
            entries: &entries,
        });
        let input = declaration(TRACE, AfterOpaqueKind::Compute, Vec::new()).input_layout(device);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(TRACE),
            bind_group_layouts: &[Some(&environment.layout), Some(&input), Some(&trace_layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(TRACE),
            source: wgpu::ShaderSource::Wgsl(trace_shader(&environment).into()),
        });
        let trace = device.compute(&wgpu::ComputePipelineDescriptor {
            label: Some(TRACE),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("trace"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let (compose, compose_group, compose_layout) = compose(
            surface,
            &params,
            &textures,
            uniform_entry,
            texture_entry,
            &input,
        );
        let temporal = super::temporal::Temporal::new(surface, &params, &textures, &compose_layout);
        Ok(Self {
            profile,
            environment,
            lighting,
            temporal,
            committed: None,
            pending: None,
            ordinal: 0,
            params,
            queue: surface.queue.clone(),
            textures,
            trace,
            compose,
            trace_group,
            compose_group,
        })
    }
}

pub(super) fn trace_shader(environment: &thrum_render::atmosphere::EnvironmentProducts) -> String {
    format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        environment.shader_source,
        include_str!("params.wgsl"),
        include_str!("../layer.wgsl"),
        super::light::shader(),
        include_str!("transport.wgsl"),
        include_str!("sun.wgsl"),
    )
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
    (compose, compose_group, compose_layout)
}
