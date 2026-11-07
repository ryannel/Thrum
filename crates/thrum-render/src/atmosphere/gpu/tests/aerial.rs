//! Isolate aerial storage error from transport error: all variants execute the
//! production integrator over the same half-precision transmittance/M inputs.
use super::super::{AtmospherePrepared, EARTH, EXTENTS};
use crate::{atmosphere::AtmosphereFrame, synthetic::SyntheticView};
use std::io::Write as _;
use std::{fmt::Write, sync::mpsc, time::Duration};

use wgpu::BindingResource::{Sampler, TextureView};
use wgpu::TextureFormat::{Rgba16Float, Rgba32Float};

const SIDE: u32 = 32;
const LAYERS: u32 = 33;
const GAIN: f32 = 1024.0;
const HALF_STEP: f32 = 1.0 / 16_777_216.0;

#[derive(Clone, Copy)]
enum Storage {
    ReferenceFloat,
    UnscaledHalf,
    Production,
}

struct Output {
    scatter: wgpu::Texture,
    pipeline: wgpu::ComputePipeline,
    group: wgpu::BindGroup,
    gain: f32,
}

impl Output {
    fn new(device: &wgpu::Device, atmosphere: &AtmospherePrepared, storage: Storage) -> Self {
        let (format, gain) = match storage {
            Storage::ReferenceFloat => (Rgba32Float, 1.0),
            Storage::UnscaledHalf => (Rgba16Float, 1.0),
            Storage::Production => (Rgba16Float, GAIN),
        };
        let scatter = texture(device, format, SIDE);
        let trans = texture(device, Rgba16Float, SIDE);
        let array_view = |texture: &wgpu::Texture| {
            texture.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2Array),
                ..Default::default()
            })
        };
        // Only storage declarations and final RGB encoding change. In
        // particular, this is not another numerical transport implementation.
        let production = include_str!("../../aerial.wgsl");
        let aerial = if matches!(storage, Storage::Production) {
            production.to_owned()
        } else {
            production.replace(" * SKY_LUT_GAIN", "")
        }
        .replacen(
            "scatter_out: texture_storage_2d_array<rgba16float, write>",
            if format == Rgba32Float {
                "scatter_out: texture_storage_2d_array<rgba32float, write>"
            } else {
                "scatter_out: texture_storage_2d_array<rgba16float, write>"
            },
            1,
        );
        let source = format!(
            "{}\n{aerial}",
            concat!(
                include_str!("../../storage.wgsl"),
                "\n",
                concat!(
                    include_str!("../../geometry.wgsl"),
                    "\n",
                    include_str!("../../common.wgsl"),
                    "\n",
                    include_str!("../../transport.wgsl"),
                    "\n",
                    include_str!("../../night.wgsl")
                )
            )
        );
        let pipeline = pipeline(device, &EARTH.shader(&source), "aerial_main");
        let views = [
            atmosphere.textures[0].create_view(&wgpu::TextureViewDescriptor::default()),
            atmosphere.textures[1].create_view(&wgpu::TextureViewDescriptor::default()),
            array_view(&scatter),
            array_view(&trans),
        ];
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let entries = [
            entry(0, atmosphere.uniform.as_entire_binding()),
            entry(1, TextureView(&views[0])),
            entry(2, TextureView(&views[1])),
            entry(3, TextureView(&views[2])),
            entry(4, TextureView(&views[3])),
            entry(5, Sampler(&sampler)),
        ];
        let entries = entries
            .into_iter()
            .chain(atmosphere.night.bindings())
            .collect::<Vec<_>>();
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("identical aerial transport inputs"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &entries,
        });
        Self {
            scatter,
            pipeline,
            group,
            gain,
        }
    }

    fn observe(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<f32> {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.group, &[]);
            pass.dispatch_workgroups(SIDE / 8, SIDE / 8, 1);
        }
        let wide = self.scatter.format() == Rgba32Float;
        let row_bytes = SIDE * if wide { 16 } else { 8 };
        let output = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("aerial storage bytes"),
            size: u64::from(row_bytes * SIDE * LAYERS),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            self.scatter.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &output,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row_bytes),
                    rows_per_image: Some(SIDE),
                },
            },
            self.scatter.size(),
        );
        submit(device, queue, encoder, &output);
        let mapped = output
            .slice(..)
            .get_mapped_range()
            .expect("aerial storage mapped");
        mapped
            .chunks_exact(if wide { 4 } else { 2 })
            .map(|value| {
                if wide {
                    f32::from_le_bytes(value.try_into().expect("f32 storage channel"))
                } else {
                    half(u16::from_le_bytes(
                        value.try_into().expect("f16 storage channel"),
                    )) / self.gain
                }
            })
            .collect()
    }
}

fn entry(binding: u32, resource: wgpu::BindingResource<'_>) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry { binding, resource }
}

fn texture(device: &wgpu::Device, format: wgpu::TextureFormat, side: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("aerial storage precision fixture"),
        size: wgpu::Extent3d {
            width: side,
            height: side,
            depth_or_array_layers: LAYERS,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::STORAGE_BINDING
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

fn pipeline(device: &wgpu::Device, source: &str, entry: &str) -> wgpu::ComputePipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(entry),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(entry),
        layout: None,
        module: &module,
        entry_point: Some(entry),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    })
}

fn submit(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    encoder: wgpu::CommandEncoder,
    output: &wgpu::Buffer,
) {
    let (send, receive) = mpsc::channel();
    encoder.map_buffer_on_submit(output, wgpu::MapMode::Read, .., move |result| {
        send.send(result).expect("aerial precision receiver alive");
    });
    let submission = queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(Duration::from_secs(20)),
        })
        .expect("aerial precision GPU completion");
    receive
        .recv()
        .expect("aerial mapping callback")
        .expect("aerial mapped");
}

fn half(bits: u16) -> f32 {
    let exponent = i32::from((bits >> 10) & 31);
    let fraction = bits & 1023;
    let mantissa = f32::from(fraction);
    let magnitude = match exponent {
        0 => mantissa * HALF_STEP,
        31 => {
            if fraction == 0 {
                f32::INFINITY
            } else {
                f32::NAN
            }
        }
        _ => (1.0 + mantissa / 1024.0) * 2.0_f32.powi(exponent - 15),
    };
    if bits & 0x8000 == 0 {
        magnitude
    } else {
        -magnitude
    }
}

fn report(degrees: f32, reference: &[f32], raw: &[f32], encoded: &[f32]) -> String {
    let mut low_count = 0;
    let mut lost_count = 0;
    let mut over_one_percent = 0;
    let mut worst = (0.0_f32, 0, 0.0_f32, 0.0_f32, 0.0_f32);
    let mut raw_error_sum = 0.0;
    let mut gain_error_sum = 0.0;
    for (index, ((truth, raw), encoded)) in reference.iter().zip(raw).zip(encoded).enumerate() {
        assert!(
            truth.is_finite()
                && *truth >= 0.0
                && raw.is_finite()
                && *raw >= 0.0
                && encoded.is_finite()
                && *encoded >= 0.0
        );
        if index % 4 == 3 {
            continue;
        }
        let raw_error = (raw - truth).abs();
        let gain_error = (encoded - truth).abs();
        // Full half ULP accommodates conversion rounding modes; f32 math is
        // identical. This oracle measures storage, not physical model accuracy.
        assert!(
            raw_error <= truth * 0.001 + HALF_STEP,
            "raw index{index}: {raw} vs {truth}"
        );
        assert!(
            gain_error <= truth * 0.001 + HALF_STEP / GAIN,
            "gain index{index}: {encoded} vs {truth}"
        );
        if (1e-9..1e-5).contains(truth) {
            low_count += 1;
            lost_count += usize::from(*raw == 0.0);
            let relative = raw_error / truth;
            over_one_percent += usize::from(relative > 0.01);
            raw_error_sum += raw_error;
            gain_error_sum += gain_error;
            if relative > worst.0 {
                worst = (relative, index, *truth, *raw, *encoded);
            }
        }
    }
    let mut text = format!(
        "Sun={degrees:+}deg low[1e-9,1e-5)={low_count} raw_zero={lost_count} raw_error>1%={over_one_percent} raw_L1={raw_error_sum:e} gain_L1={gain_error_sum:e}\n"
    );
    let (relative, index, truth, raw, encoded) = worst;
    let texel = index / 4;
    writeln!(text, "worst layer={} xy=({}, {}) channel={} ref={truth:e} raw={raw:e} gain1024={encoded:e} raw_rel={relative:e} gain_rel={:e} raw_scene_error_at_127klux={:e}", texel / 1024, texel % 32, texel / 32 % 32, index % 4, (encoded - truth).abs() / truth.max(1e-30), (raw-truth).abs()*127_000.0).expect("write diagnostic");
    assert!(
        low_count > 100,
        "fixture must exercise meaningful low-radiance values: {text}"
    );
    text
}

#[test]
fn aerial_storage_matches_float32_transport_and_consumer_units() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("aerial adapter");
        #[cfg(target_os = "macos")]
        assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("aerial device");
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let directional = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: crate::synthetic::LIGHT_BYTES as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let frame = AtmosphereFrame::new([1.0, 0.0, 0.0], 127_000.0, [1.0; 3], 245.0, 1)
            .expect("aerial frame");
        let atmosphere = AtmospherePrepared::new(
            &device,
            &queue,
            1,
            frame,
            &SyntheticView::default(),
            &directional,
            &super::sky_samples(&device),
        );
        let outputs = [
            Output::new(&device, &atmosphere, Storage::ReferenceFloat),
            Output::new(&device, &atmosphere, Storage::UnscaledHalf),
            Output::new(&device, &atmosphere, Storage::Production),
        ];
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        for (index, extent) in EXTENTS.iter().take(2).enumerate() {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&atmosphere.compute[index]);
            pass.set_bind_group(0, &atmosphere.groups[index], &[]);
            pass.dispatch_workgroups(extent[0].div_ceil(8), extent[1].div_ceil(8), 1);
        }
        queue.submit([encoder.finish()]);
        let mut diagnostics = String::new();
        for degrees in [2.0_f32, 0.0, -2.0, -4.0, -6.0, -9.0] {
            let mut packed = [0.0_f32; 32];
            // Inverse project to rays near +X: ±26.5deg azimuth, ±1.1deg
            // elevation, with camera fixed at 245m. No renderer camera helper.
            packed[2] = 500.0;
            packed[5] = 20.0;
            packed[12..16].copy_from_slice(&[1000.0, 245.0, 0.0, 1.0]);
            packed[16..20].copy_from_slice(&[0.0, 245.0, 0.0, 245.0]);
            let (sin, cos) = degrees.to_radians().sin_cos();
            packed[20..24].copy_from_slice(&[cos, sin, 0.0, 127_000.0]);
            packed[24..28].copy_from_slice(&[1.0, 1.0, 1.0, 1.0 / 1024.0]);
            queue.write_buffer(
                &atmosphere.uniform,
                0,
                packed.map(f32::to_le_bytes).as_flattened(),
            );
            let [reference, raw, encoded] = outputs
                .each_ref()
                .map(|output| output.observe(&device, &queue));
            diagnostics.push_str(&report(degrees, &reference, &raw, &encoded));
        }
        consumer_units(&device, &queue, &atmosphere.uniform);
        assert!(scope.pop().await.is_none(), "aerial pipelines validate");
        std::io::Write::write_all(&mut std::io::stdout(), diagnostics.as_bytes())
            .expect("report aerial precision");
    });
}

fn consumer_units(device: &wgpu::Device, queue: &wgpu::Queue, uniform: &wgpu::Buffer) {
    let scatter = texture(device, Rgba16Float, 1);
    let transmission = texture(device, Rgba16Float, 1);
    // Physical S=2^-16 stored at gain1024 as2^-6, T=1/2 unscaled. The
    // consumer must recover the same physical result, not a1024x lighting gain.
    for (texture, value) in [(&scatter, 0x2400_u16), (&transmission, 0x3800_u16)] {
        let bytes: Vec<_> = (0..LAYERS)
            .flat_map(|_| [value, value, value, 0])
            .flat_map(u16::to_le_bytes)
            .collect();
        queue.write_texture(
            texture.as_image_copy(),
            &bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(8),
                rows_per_image: Some(1),
            },
            texture.size(),
        );
    }
    let views = [&scatter, &transmission].map(|texture| {
        texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        })
    });
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
    let encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let values = observe_consumer(
        device,
        queue,
        encoder,
        ([&views[0], &views[1]], &sampler, uniform),
        "consumer",
    );
    for (channel, surface) in [0.2_f32, 0.3, 0.4].into_iter().enumerate() {
        let actual = values[channel];
        let expected = 0.5 * surface + 127_000.0 / 65_536.0;
        assert!(
            (actual - expected).abs() < expected * 1e-5,
            "consumer channel{channel}: {actual} vs {expected}"
        );
    }
}

fn observe_consumer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    mut encoder: wgpu::CommandEncoder,
    (views, sampler, uniform): ([&wgpu::TextureView; 2], &wgpu::Sampler, &wgpu::Buffer),
    entry_point: &str,
) -> [f32; 4] {
    let source = format!(
        "{}\n{}\n{}",
        r"
struct ViewAux { viewport_inv_viewport: vec4<f32> }
const view_aux=ViewAux(vec4(32.0,32.0,1.0/32.0,1.0/32.0));
@group(0) @binding(0) var<storage,read_write> result: array<vec4<f32>>;
",
        concat!(
            include_str!("../../storage.wgsl"),
            "\n",
            include_str!("../../../synthetic_aerial.wgsl"),
            "\n",
            include_str!("../../../synthetic_aerial_clear.wgsl")
        ),
        r"
@compute @workgroup_size(1)
fn consumer() {
    result[0]=vec4(aerial_colour(vec3(0.2,0.3,0.4),vec3(1000.0,245.0,0.0),vec4(16.0,16.0,0.0,1.0)),1.0);
}
@compute @workgroup_size(1)
fn finite_night() {
    let uv = vec2(0.5);
    let near = aerial_sample(1000.0, uv);
    let middle = aerial_sample(10000.0, uv);
    let far = aerial_sample(30000.0, uv);
    let a = composite_aerial(near, vec3(0.0), near.scatter.rgb);
    let b = composite_aerial(middle, vec3(0.0), middle.scatter.rgb);
    let c = composite_aerial(far, vec3(0.0), far.scatter.rgb);
    result[0] = vec4(a.x, b.x, c.x, 0.0);
}
"
    );
    let pipeline = pipeline(device, &source, entry_point);
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 16,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 16,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let first = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[entry(0, output.as_entire_binding())],
    });
    let second = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(1),
        entries: &[
            entry(4, TextureView(views[0])),
            entry(5, TextureView(views[1])),
            entry(6, Sampler(sampler)),
            entry(7, uniform.as_entire_binding()),
        ],
    });
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &first, &[]);
        pass.set_bind_group(1, &second, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
    encoder.copy_buffer_to_buffer(&output, 0, &readback, 0, 16);
    submit(device, queue, encoder, &readback);
    let mapped = readback
        .slice(..)
        .get_mapped_range()
        .expect("consumer units mapped");
    std::array::from_fn(|index| f32::from_le_bytes(mapped.as_chunks::<4>().0[index]))
}

#[test]
fn finite_night_radiance_survives_the_aerial_composite() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("real GPU required for finite night transport");
        #[cfg(target_os = "macos")]
        assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("finite night device");
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let directional = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: crate::synthetic::LIGHT_BYTES as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let height = 1800.0;
        let view = SyntheticView::perspective(
            [0.0, height, 0.0],
            [1000.0, height, 0.0],
            [0.0, 1.0, 0.0],
            0.25,
            1.0,
            0.1,
            32_000.0,
        )
        .expect("horizontal observer view");
        let frame = AtmosphereFrame::new([0.0, -1.0, 0.0], 0.0, [1.0; 3], height, 1)
            .expect("moonless atmosphere frame");
        let mut atmosphere = AtmospherePrepared::new(
            &device,
            &queue,
            1,
            frame,
            &view,
            &directional,
            &super::sky_samples(&device),
        );
        let [off, on] = [frame, frame.with_nightglow()].map(|frame| {
            atmosphere.update(&queue, frame, &view, [1, 1]);
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            {
                // Only the night source is under test. The zero-initialized
                // solar inputs and zero Sun lux isolate its composite.
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
                pass.set_pipeline(&atmosphere.aerial_pipeline);
                pass.set_bind_group(0, &atmosphere.aerial_group, &[]);
                pass.dispatch_workgroups(SIDE / 8, SIDE / 8, 1);
            }
            observe_consumer(
                &device,
                &queue,
                encoder,
                atmosphere.aerial_sampled(),
                "finite_night",
            )
        });
        assert_eq!(off.map(f32::to_bits), [0; 4]);
        // Sum of the dense shell and diffuse f64 reference paths, mixed-2.csv.
        let expected = [1.241_665_2e-6, 1.178_059_1e-5, 3.150_029e-5];
        for ((distance, actual), expected) in [1, 10, 30].into_iter().zip(on).zip(expected) {
            let relative = (actual - expected).abs() / expected;
            assert!(
                relative <= 0.10,
                "{distance} km: {actual:e} vs {expected:e}"
            );
            writeln!(
                std::io::stdout().lock(),
                "{distance} km: {actual:e} vs {expected:e} ({:.3}%)",
                relative * 100.0
            )
            .expect("write finite night comparison");
        }
        assert!(
            scope.pop().await.is_none(),
            "finite night pipelines validate"
        );
    });
}
