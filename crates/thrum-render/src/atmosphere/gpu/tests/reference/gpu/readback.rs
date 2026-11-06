//! Read the production LUTs through sampled access, without adding test-only
//! resource usages or paths to the renderer.

use std::sync::mpsc;
use std::time::Duration;

use crate::atmosphere::AtmosphereFrame;
use crate::atmosphere::gpu::{AtmospherePrepared, EXTENTS};
use crate::synthetic::SyntheticView;

const READ_SHADER: &str = r"
@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var<storage,read_write> values: array<vec4<f32>>;
@compute @workgroup_size(8,8)
fn read(@builtin(global_invocation_id) id: vec3<u32>) {
    let size=textureDimensions(source);
    if any(id.xy>=size) { return; }
    values[id.y*size.x+id.x]=textureLoad(source,vec2<i32>(id.xy),0);
}
";

pub(super) struct Sky {
    pub full: Vec<[f64; 3]>,
    pub single: Vec<[f64; 3]>,
    pub rays_full: Vec<[f64; 3]>,
    pub rays_single: Vec<[f64; 3]>,
}

pub(super) struct Probe {
    atmosphere: AtmospherePrepared,
    reader: wgpu::ComputePipeline,
    single_group: wgpu::BindGroup,
    transport_ready: bool,
    rays: super::query::Rays,
}

impl Probe {
    pub(super) fn new(device: &wgpu::Device, queue: &wgpu::Queue, directions: &[[f32; 4]]) -> Self {
        let directional = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("reference directional output"),
            size: crate::synthetic::LIGHT_BYTES as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let atmosphere = AtmospherePrepared::new(
            device,
            queue,
            1,
            frame(45),
            &SyntheticView::default(),
            &directional,
        );
        let zero = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("zero higher-order input for first-order comparison"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let views = [
            atmosphere.textures[0].create_view(&wgpu::TextureViewDescriptor::default()),
            zero.create_view(&wgpu::TextureViewDescriptor::default()),
            atmosphere.textures[2].create_view(&wgpu::TextureViewDescriptor::default()),
        ];
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let single_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("production single-scatter resource substitution"),
            layout: &atmosphere.compute[2].get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: atmosphere.uniform.as_entire_binding(),
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
                    resource: wgpu::BindingResource::TextureView(&views[2]),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    // Indirect ground paths are excluded from the first-order
                    // observation, alongside the zero multiple-scattering input.
                    resource: wgpu::BindingResource::TextureView(&views[1]),
                },
            ],
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("reference texture readback"),
            source: wgpu::ShaderSource::Wgsl(READ_SHADER.into()),
        });
        let reader = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("reference texture readback"),
            layout: None,
            module: &module,
            entry_point: Some("read"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        Self {
            atmosphere,
            reader,
            single_group,
            transport_ready: false,
            rays: super::query::Rays::new(device, directions),
        }
    }

    pub(super) fn sky(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        sun_degrees: i16,
    ) -> Sky {
        self.atmosphere
            .update(queue, frame(sun_degrees), &SyntheticView::default(), [1, 1]);
        let full = Snapshot::new(device, &self.reader, &self.atmosphere.textures[2]);
        let single = Snapshot::new(device, &self.reader, &self.atmosphere.textures[2]);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        if !self.transport_ready {
            for index in 0..2 {
                self.dispatch(&mut encoder, index, &self.atmosphere.groups[index]);
            }
            self.dispatch(&mut encoder, 3, &self.atmosphere.groups[3]);
        }
        self.dispatch(&mut encoder, 2, &self.atmosphere.groups[2]);
        full.encode(&mut encoder, &self.reader);
        let rays_full = self.rays.observe(
            device,
            &mut encoder,
            &self.atmosphere.uniform,
            &self.atmosphere.textures[2],
        );
        self.dispatch(&mut encoder, 2, &self.single_group);
        single.encode(&mut encoder, &self.reader);
        let rays_single = self.rays.observe(
            device,
            &mut encoder,
            &self.atmosphere.uniform,
            &self.atmosphere.textures[2],
        );
        let receivers =
            [&full.readback, &single.readback, &rays_full, &rays_single].map(|buffer| {
                let (send, receive) = mpsc::channel();
                encoder.map_buffer_on_submit(buffer, wgpu::MapMode::Read, .., move |result| {
                    send.send(result)
                        .expect("reference readback receiver remains alive");
                });
                receive
            });
        let submission = queue.submit([encoder.finish()]);
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(Duration::from_secs(10)),
            })
            .expect("reference GPU work completes");
        for receiver in receivers {
            receiver
                .recv()
                .expect("reference map callback runs")
                .expect("reference GPU buffer maps");
        }
        self.transport_ready = true;
        Sky {
            full: full.collect(1024.0),
            single: single.collect(1024.0),
            rays_full: decode(&rays_full, 1024.0),
            rays_single: decode(&rays_single, 1024.0),
        }
    }

    pub(super) fn multiple(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<[f64; 3]> {
        assert!(
            self.transport_ready,
            "profile has executed before observation"
        );
        let snapshot = Snapshot::new(device, &self.reader, &self.atmosphere.textures[1]);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        snapshot.encode(&mut encoder, &self.reader);
        let (send, receive) = mpsc::channel();
        encoder.map_buffer_on_submit(&snapshot.readback, wgpu::MapMode::Read, .., move |result| {
            send.send(result).expect("M observation receiver alive");
        });
        let submitted = queue.submit([encoder.finish()]);
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submitted),
                timeout: Some(Duration::from_secs(10)),
            })
            .expect("M observation completes");
        receive
            .recv()
            .expect("M mapping callback")
            .expect("M mapped");
        snapshot.collect(1.0)
    }

    fn dispatch(&self, encoder: &mut wgpu::CommandEncoder, index: usize, group: &wgpu::BindGroup) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&self.atmosphere.compute[index]);
        pass.set_bind_group(0, group, &[]);
        let [width, height] = EXTENTS[index];
        if index == 3 {
            // Ground irradiance cooperates across 64 lanes per solar entry.
            pass.dispatch_workgroups(width, 1, 1);
        } else {
            pass.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), 1);
        }
    }
}

fn frame(degrees: i16) -> AtmosphereFrame {
    let (sin, cos) = f32::from(degrees).to_radians().sin_cos();
    AtmosphereFrame::new([cos, sin, 0.0], 1.0, [1.0; 3], 2.0, 1)
        .expect("fixed reference frame is valid")
}

struct Snapshot {
    output: wgpu::Buffer,
    readback: wgpu::Buffer,
    group: wgpu::BindGroup,
    width: u32,
    height: u32,
}

impl Snapshot {
    fn new(
        device: &wgpu::Device,
        pipeline: &wgpu::ComputePipeline,
        texture: &wgpu::Texture,
    ) -> Self {
        let width = texture.width();
        let height = texture.height();
        let size = u64::from(width) * u64::from(height) * 16;
        let output = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("reference float LUT snapshot"),
            size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("reference mapped LUT snapshot"),
            size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("reference readback resources"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output.as_entire_binding(),
                },
            ],
        });
        Self {
            output,
            readback,
            group,
            width,
            height,
        }
    }

    fn encode(&self, encoder: &mut wgpu::CommandEncoder, pipeline: &wgpu::ComputePipeline) {
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &self.group, &[]);
            pass.dispatch_workgroups(self.width.div_ceil(8), self.height.div_ceil(8), 1);
        }
        encoder.copy_buffer_to_buffer(&self.output, 0, &self.readback, 0, self.output.size());
    }

    fn collect(&self, gain: f64) -> Vec<[f64; 3]> {
        decode(&self.readback, gain)
    }
}

fn decode(readback: &wgpu::Buffer, gain: f64) -> Vec<[f64; 3]> {
    let mapped = readback
        .slice(..)
        .get_mapped_range()
        .expect("reference buffer is mapped");
    mapped
        .as_chunks::<16>()
        .0
        .iter()
        .map(|pixel| {
            std::array::from_fn(|channel| {
                let encoded = f32::from_le_bytes(pixel.as_chunks::<4>().0[channel]);
                assert!(
                    encoded.is_finite() && encoded >= 0.0,
                    "invalid production sky radiance"
                );
                f64::from(encoded) / gain
            })
        })
        .collect()
}
