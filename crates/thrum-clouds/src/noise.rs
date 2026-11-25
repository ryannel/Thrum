use thrum_render::{
    contribution::EffectAccess,
    graph::{AccessKind, ResourceDescriptor, ResourceScope, TextureKind},
    runtime::DeviceSite,
};

pub(crate) use thrum_cloud_model::{DETAIL_PERIOD, DETAIL_SIZE, SHAPE_PERIOD, SHAPE_SIZE};

#[expect(
    clippy::cast_possible_truncation,
    reason = "wrapped noise phase fits the GPU record"
)]
pub(crate) fn bank_noise(origin: [f64; 2], deck: f64) -> [f32; 4] {
    let period = thrum_cloud_model::BANK_NOISE_PERIOD;
    [
        (origin[0].rem_euclid(period) / period) as f32,
        (origin[1].rem_euclid(period) / period) as f32,
        period as f32,
        deck as f32,
    ]
}

pub(crate) struct Noise {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    pub(crate) sampler: wgpu::Sampler,
}

impl Noise {
    pub(crate) fn new(device: &DeviceSite, size: u32, label: &str) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: size,
            },
            mip_level_count: size.ilog2() + 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            min_filter: wgpu::FilterMode::Linear,
            mag_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        Self {
            texture,
            view,
            sampler,
        }
    }

    pub(crate) fn prepare(
        &self,
        device: &DeviceSite,
        seed: u32,
        entry: &str,
    ) -> wgpu::CommandBuffer {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cloud noise volume"),
            source: wgpu::ShaderSource::Wgsl(include_str!("noise.wgsl").into()),
        });
        let pipeline = |entry, constants| {
            device.compute(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: None,
                module: &shader,
                entry_point: Some(entry),
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants,
                    ..Default::default()
                },
                cache: None,
            })
        };
        let constants = [("seed", f64::from(seed))];
        let base = pipeline(entry, &constants);
        let mip = pipeline("mip", &[]);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("prepare cloud noise volume"),
        });
        let view = |level| {
            self.texture.create_view(&wgpu::TextureViewDescriptor {
                base_mip_level: level,
                mip_level_count: Some(1),
                ..Default::default()
            })
        };
        for level in 0..self.texture.mip_level_count() {
            let output = view(level);
            let finer = view(level.saturating_sub(1));
            let mut bindings = vec![wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&output),
            }];
            let pipeline = if level == 0 {
                &base
            } else {
                bindings.push(wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&finer),
                });
                &mip
            };
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("cloud noise mip"),
                layout: &pipeline.get_bind_group_layout(0),
                entries: &bindings,
            });
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &group, &[]);
            let count = (self.texture.width() >> level).div_ceil(4);
            pass.dispatch_workgroups(count, count, count);
        }
        encoder.finish()
    }

    pub(crate) fn resource(&self) -> (ResourceDescriptor, ResourceScope) {
        let texture = &self.texture;
        (
            ResourceDescriptor::Texture {
                dimension: TextureKind::D3 {
                    depth: texture.depth_or_array_layers(),
                },
                width: texture.width(),
                height: texture.height(),
                layers: 1,
                mips: texture.mip_level_count(),
                samples: 1,
                format: texture.format(),
                usage: texture.usage(),
            },
            ResourceScope::Imported { initialized: true },
        )
    }

    pub(crate) fn read(&self, resource: usize) -> EffectAccess {
        EffectAccess {
            resource,
            range: 0..u64::from(self.texture.mip_level_count()),
            kind: AccessKind::SampledRead,
            view: None,
            history: None,
        }
    }

    pub(crate) fn bytes(&self) -> u64 {
        (0..self.texture.mip_level_count())
            .map(|level| u64::from(self.texture.width() >> level).pow(3) * 4)
            .sum()
    }

    pub(crate) fn bind(&self, binding: u32) -> wgpu::BindGroupEntry<'_> {
        wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::TextureView(&self.view),
        }
    }
}
