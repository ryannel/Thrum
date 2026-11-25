//! Read the production LUTs through sampled access, without adding test-only
//! resource usages or paths to the renderer.

use crate::atmosphere::AtmosphereFrame;
use crate::atmosphere::gpu::AtmospherePrepared;
use crate::atmosphere::stage::Chart;
use crate::light::EnvironmentLight;
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
    pub rays_full: Vec<[f64; 3]>,
}

pub(super) struct Probe {
    atmosphere: AtmospherePrepared,
    environment: wgpu::Buffer,
    reader: wgpu::ComputePipeline,
    transport_ready: bool,
    rays: super::query::Rays,
}

impl Probe {
    pub(super) fn new(device: &wgpu::Device, queue: &wgpu::Queue, directions: &[[f32; 4]]) -> Self {
        let environment = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("reference environment output"),
            size: crate::light::ENVIRONMENT_BYTES,
            usage: wgpu::BufferUsages::UNIFORM
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let atmosphere = AtmospherePrepared::new(
            &crate::runtime::DeviceSite::untracked(wgpu::Device::clone(device)),
            queue,
            1,
            frame(45.0, 2.0),
            &SyntheticView::default(),
            &environment,
            (&super::super::sky_samples(device), None),
        );
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
            environment,
            reader,
            transport_ready: false,
            rays: super::query::Rays::new(device, directions),
        }
    }

    pub(super) fn sky(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        sun_degrees: f32,
        height: f32,
    ) -> Sky {
        self.atmosphere.update(
            queue,
            frame(sun_degrees, height),
            &SyntheticView::default(),
            [1, 1],
            None,
        );
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        if !self.transport_ready {
            for chart in [
                Chart::Transmittance,
                Chart::MultipleScattering,
                Chart::Ground,
            ] {
                self.dispatch(&mut encoder, chart);
            }
        }
        self.dispatch(&mut encoder, Chart::SkyView);
        let rays_full = self.rays.observe(
            device,
            &mut encoder,
            &self.atmosphere.uniform,
            &self.atmosphere.textures.sky_view,
        );
        let submission = queue.submit([encoder.finish()]);
        self.transport_ready = true;
        Sky {
            rays_full: decode(&rays_full.wait(device, submission), 1024.0),
        }
    }

    pub(super) fn multiple(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<[f64; 4]> {
        assert!(
            self.transport_ready,
            "profile has executed before observation"
        );
        let snapshot = Snapshot::new(
            device,
            &self.reader,
            &self.atmosphere.textures.multiple_scattering,
        );
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let readback = snapshot.encode(device, &mut encoder, &self.reader);
        let submission = queue.submit([encoder.finish()]);
        decode(&readback.wait(device, submission), 1.0)
    }

    pub(super) fn night(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        height: f32,
        enabled: bool,
    ) -> NightSky {
        let day = frame(45.0, height);
        let frame = if enabled { day.with_nightglow() } else { day };
        self.atmosphere
            .update(queue, frame, &SyntheticView::default(), [1, 1], None);
        let texture = &self.atmosphere.textures.sky_view;
        let snapshot = Snapshot::new(device, &self.reader, texture);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        if !self.transport_ready {
            for chart in [
                Chart::Transmittance,
                Chart::MultipleScattering,
                Chart::Ground,
            ] {
                self.dispatch(&mut encoder, chart);
            }
        }
        self.dispatch(&mut encoder, Chart::SkyView);
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&self.atmosphere.diffuse_pipeline);
            pass.set_bind_group(0, &self.atmosphere.diffuse_group, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }
        let grid = snapshot.encode(device, &mut encoder, &self.reader);
        let rays = self
            .rays
            .observe(device, &mut encoder, &self.atmosphere.uniform, texture);
        let environment = thrum_test_support::readback::Readback::buffer(
            device,
            &mut encoder,
            &self.environment,
            ..,
        );
        let submission = queue.submit([encoder.finish()]);
        self.transport_ready = true;
        NightSky {
            grid: decode(&grid.wait(device, submission.clone()), 1.0),
            rays: decode(&rays.wait(device, submission.clone()), 1.0),
            environment: bytemuck::pod_read_unaligned(&environment.wait(device, submission)),
        }
    }

    fn dispatch(&self, encoder: &mut wgpu::CommandEncoder, chart: Chart) {
        let (pipeline, group) = self.atmosphere.chart_stages.get(chart);
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, group, &[]);
        let [width, height] = chart.extent();
        if chart == Chart::Ground {
            // Ground irradiance cooperates across 64 lanes per solar entry.
            pass.dispatch_workgroups(width, 1, 1);
        } else {
            pass.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), 1);
        }
    }
}

pub(super) struct NightSky {
    pub grid: Vec<[f64; 4]>,
    pub rays: Vec<[f64; 4]>,
    pub environment: EnvironmentLight,
}

fn frame(degrees: f32, height: f32) -> AtmosphereFrame {
    let (sin, cos) = degrees.to_radians().sin_cos();
    AtmosphereFrame::new([cos, sin, 0.0], 1.0, [1.0; 3], height, 1)
        .expect("fixed reference frame is valid")
}

struct Snapshot {
    output: wgpu::Buffer,
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
            group,
            width,
            height,
        }
    }

    fn encode(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        pipeline: &wgpu::ComputePipeline,
    ) -> thrum_test_support::readback::Readback {
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &self.group, &[]);
            pass.dispatch_workgroups(self.width.div_ceil(8), self.height.div_ceil(8), 1);
        }
        thrum_test_support::readback::Readback::buffer(device, encoder, &self.output, ..)
    }
}

fn decode<const N: usize>(mapped: &[u8], gain: f64) -> Vec<[f64; N]> {
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
