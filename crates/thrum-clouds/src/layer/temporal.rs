use super::{COMPOSE, Layer, SceneSurface, Uniform};
use thrum_render::binding;
use thrum_render::contribution::{EffectState, HistoryBinding};

pub(super) const PASS: &str = "cloud resolve";

pub(super) struct Temporal {
    pub(super) clock: EffectState,
    pub(super) targets: Targets,
    pipeline: wgpu::ComputePipeline,
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) layout: wgpu::BindGroupLayout,
}

pub(super) struct Targets {
    pub(super) textures: [[wgpu::Texture; 2]; 2],
    groups: [wgpu::BindGroup; 2],
    compose: [wgpu::BindGroup; 2],
}

impl Temporal {
    pub(super) fn new(
        surface: &SceneSurface<'_>,
        params: &wgpu::Buffer,
        fresh: &[wgpu::Texture; 2],
        compose_layout: &wgpu::BindGroupLayout,
        accepted: Option<&Self>,
    ) -> Self {
        let device = surface.device;
        let mut entries = vec![binding::uniform(
            0,
            wgpu::ShaderStages::COMPUTE,
            size_of::<Uniform>() as u64,
        )];
        for binding in 1..=6 {
            entries.push(if binding >= 5 {
                binding::storage_texture(
                    binding,
                    wgpu::ShaderStages::COMPUTE,
                    wgpu::StorageTextureAccess::WriteOnly,
                    wgpu::TextureFormat::Rgba16Float,
                    wgpu::TextureViewDimension::D2Array,
                )
            } else {
                binding::texture(
                    binding,
                    wgpu::ShaderStages::COMPUTE,
                    wgpu::TextureSampleType::Float { filterable: false },
                    wgpu::TextureViewDimension::D2Array,
                )
            });
        }
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(PASS),
            entries: &entries,
        });
        let input = super::declaration(
            PASS,
            thrum_render::contribution::AfterOpaqueKind::Compute,
            Vec::new(),
        )
        .input_layout(device);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(PASS),
            bind_group_layouts: &[Some(surface.view_layout), Some(&input), Some(&layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(PASS),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "{}\n{}\n{}\n{}",
                    super::StopMode::shader(),
                    include_str!("params.wgsl"),
                    include_str!("view.wgsl"),
                    include_str!("resolve.wgsl")
                )
                .into(),
            ),
        });
        let pipeline = device.compute(&wgpu::ComputePipelineDescriptor {
            label: Some(PASS),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("resolve"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let targets = Targets::new(
            device,
            params,
            fresh,
            compose_layout,
            &layout,
            accepted.map(|temporal| &temporal.targets.textures),
        );
        let mut clock = EffectState::new(2);
        clock.rebuild(surface.generation, 1);
        Self {
            clock,
            targets,
            pipeline,
            #[cfg(not(target_arch = "wasm32"))]
            layout: layout.into_raw(),
        }
    }

    pub(super) fn encode(&mut self, pass: &mut wgpu::ComputePass<'_>, extent: [u32; 2]) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(
            2,
            &self.targets.groups[self.clock.side(0, HistoryBinding::Previous)],
            &[],
        );
        pass.dispatch_workgroups(extent[0].div_ceil(8), extent[1].div_ceil(8), 1);
        self.clock.produced(0);
        self.clock.produced(1);
    }
    pub(super) fn result_group(&self) -> &wgpu::BindGroup {
        &self.targets.compose[self.clock.side(0, HistoryBinding::Result)]
    }
}

impl Targets {
    pub(super) fn new(
        device: &wgpu::Device,
        params: &wgpu::Buffer,
        fresh: &[wgpu::Texture; 2],
        compose_layout: &wgpu::BindGroupLayout,
        layout: &wgpu::BindGroupLayout,
        accepted: Option<&[[wgpu::Texture; 2]; 2]>,
    ) -> Self {
        let textures = accepted.map_or_else(
            || {
                std::array::from_fn(|_| {
                    std::array::from_fn(|_| {
                        device.create_texture(&wgpu::TextureDescriptor {
                            label: Some("cloud layer history"),
                            size: fresh[0].size(),
                            mip_level_count: 1,
                            sample_count: 1,
                            dimension: wgpu::TextureDimension::D2,
                            format: wgpu::TextureFormat::Rgba16Float,
                            usage: wgpu::TextureUsages::TEXTURE_BINDING
                                | wgpu::TextureUsages::STORAGE_BINDING
                                | wgpu::TextureUsages::COPY_SRC,
                            view_formats: &[],
                        })
                    })
                })
            },
            Clone::clone,
        );
        let groups =
            std::array::from_fn(|side| {
                let views = [
                    &fresh[0],
                    &fresh[1],
                    &textures[side][0],
                    &textures[side][1],
                    &textures[1 - side][0],
                    &textures[1 - side][1],
                ]
                .map(super::stop_view);
                let mut entries = vec![wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params.as_entire_binding(),
                }];
                entries.extend(views.iter().zip(1..=6).map(|(view, binding)| {
                    wgpu::BindGroupEntry {
                        binding,
                        resource: wgpu::BindingResource::TextureView(view),
                    }
                }));
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some(PASS),
                    layout,
                    entries: &entries,
                })
            });
        let compose = std::array::from_fn(|side| {
            compose_group(device, params, &textures[side], compose_layout)
        });
        Self {
            textures,
            groups,
            compose,
        }
    }
}

pub(super) fn compose_group(
    device: &wgpu::Device,
    params: &wgpu::Buffer,
    textures: &[wgpu::Texture; 2],
    layout: &wgpu::BindGroupLayout,
) -> wgpu::BindGroup {
    let views = textures.each_ref().map(super::stop_view);
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(COMPOSE),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: params.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&views[0]),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(&views[1]),
            },
        ],
    })
}

impl Layer {
    pub(crate) fn submitted(&mut self) {
        if let Some(sample) = self.pending.take() {
            self.committed = Some(sample);
            self.ordinal = self.ordinal.wrapping_add(1);
        }
        self.temporal.clock.submitted();
    }
}
