//! Opt-in ground-slice oracle for the submitted browser lifecycle check.

use crate::{RowStamp, SunPath, WeatherMap};
use thrum_render::runtime::DeviceSite;

#[derive(Clone)]
pub(crate) struct GroundReference {
    pipeline: wgpu::ComputePipeline,
    groups: Vec<wgpu::BindGroup>,
    rows: Vec<wgpu::Buffer>,
    output: wgpu::Texture,
}

impl GroundReference {
    pub(crate) fn new(raw: &wgpu::Device, sun: &SunPath, weather: &WeatherMap) -> Self {
        let device = DeviceSite::untracked(raw.clone());
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cloud ground reference"),
            source: wgpu::ShaderSource::Wgsl(super::GROUND_REFERENCE_SHADER.into()),
        });
        let pipeline = device.compute(&wgpu::ComputePipelineDescriptor {
            label: Some("cloud ground reference"),
            layout: None,
            module: &module,
            entry_point: Some("replay"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let size = sun.profile().size;
        let output = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("cloud ground reference"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 3,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let weather = weather.textures()[..2]
            .iter()
            .map(|texture| {
                texture.create_view(&wgpu::TextureViewDescriptor {
                    dimension: Some(wgpu::TextureViewDimension::D2Array),
                    ..Default::default()
                })
            })
            .collect::<Vec<_>>();
        let mut groups = Vec::new();
        let mut rows = Vec::new();
        for layer in 0..3 {
            let view = output.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2),
                base_array_layer: layer,
                array_layer_count: Some(1),
                ..Default::default()
            });
            let mask = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("cloud ground reference row mask"),
                size: u64::from(size) * 4,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            groups.push(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("cloud ground reference level"),
                layout: &pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: sun.uniforms()[layer as usize].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&weather[0]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&weather[1]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: mask.as_entire_binding(),
                    },
                    sun.shape_noise.bind(9),
                    wgpu::BindGroupEntry {
                        binding: 10,
                        resource: wgpu::BindingResource::Sampler(&sun.shape_noise.sampler),
                    },
                ],
            }));
            rows.push(mask);
        }
        Self {
            pipeline,
            groups,
            rows,
            output,
        }
    }

    pub(crate) fn replay(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        stamps: &[Vec<RowStamp>; 3],
        tick: u64,
    ) -> &wgpu::Texture {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("replay cloud ground slices"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&self.pipeline);
            let tiles = self.output.width().div_ceil(8);
            for ((mask, group), stamps) in self.rows.iter().zip(&self.groups).zip(stamps) {
                let current = stamps
                    .iter()
                    .map(|stamp| u32::from(stamp.tick == tick))
                    .collect::<Vec<_>>();
                queue.write_buffer(mask, 0, bytemuck::cast_slice(&current));
                pass.set_bind_group(0, group, &[]);
                pass.dispatch_workgroups(tiles, tiles, 1);
            }
        }
        queue.submit([encoder.finish()]);
        &self.output
    }
}
