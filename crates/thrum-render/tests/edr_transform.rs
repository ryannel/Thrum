//! Compare the shared tone function's Metal output with the exposed-colour formula.
use thrum_test_support::{Profile, Restriction, gpu};

const KNEE: f64 = 0.76;
const HEADROOM: [f32; 6] = [1.0, 1.5, 2.0, 4.0, 8.0, 16.0];
const CHROMA: [[f32; 3]; 4] = [
    [1.0, 1.0, 1.0],
    [1.0, 0.2, 0.0],
    [0.0, 1.0, 0.3],
    [0.4, 0.0, 1.0],
];

fn reference(rgb: [f64; 3], headroom: f64) -> [f64; 3] {
    let x = rgb.into_iter().fold(0.0, f64::max);
    if x <= KNEE {
        return rgb;
    }
    let d = headroom - KNEE;
    let peak = headroom - d * d / (x + d - KNEE);
    let g = 1.0 / (0.15 * (x - peak) + 1.0);
    rgb.map(|channel| (1.0 - g) * peak + channel / x * peak * g)
}

fn sdr_reference(rgb: [f64; 3]) -> [f64; 3] {
    let x = rgb.into_iter().fold(0.0, f64::max);
    if x <= KNEE {
        return rgb;
    }
    let peak = 1.0 - 0.24 * 0.24 / (x - 0.52);
    let g = 1.0 / (0.15 * (x - peak) + 1.0);
    rgb.map(|channel| (1.0 - g) * peak + channel / x * peak * g)
}

fn peaks() -> Vec<f32> {
    let mut values = vec![0.0, 0.76 - 0.001, 0.76, 0.76 + 0.001];
    values.extend((-128_i16..=288).map(|step| (f32::from(step) / 16.0).exp2()));
    values.sort_by(f32::total_cmp);
    values
}

fn samples() -> Vec<[f32; 4]> {
    let peaks = peaks();
    HEADROOM
        .into_iter()
        .flat_map(|headroom| {
            CHROMA.into_iter().flat_map({
                let peaks = &peaks;
                move |chroma| {
                    peaks.iter().map(move |x| {
                        let [r, g, b] = chroma.map(|channel| x * channel);
                        [r, g, b, headroom]
                    })
                }
            })
        })
        .collect()
}

#[test]
fn cpu_reference_has_the_sdr_identity_and_matches_knee_slopes() {
    let step = 1e-6;
    for headroom in HEADROOM.map(f64::from) {
        for chroma in CHROMA.map(|rgb| rgb.map(f64::from)) {
            let close = |x| reference(chroma.map(|c| c * x), headroom);
            let below = close(KNEE - step);
            let at = close(KNEE);
            let above = close(KNEE + step);
            for channel in 0..3 {
                assert!((at[channel] - chroma[channel] * KNEE).abs() < 1e-15);
                let left = (at[channel] - below[channel]) / step;
                let right = (above[channel] - at[channel]) / step;
                assert!(
                    (left - right).abs() < 5e-6,
                    "H={headroom} channel={channel}"
                );
            }
        }
    }
    for [r, g, b, h] in samples() {
        let rgb = [r, g, b].map(f64::from);
        let mapped = reference(rgb, f64::from(h));
        assert!(
            mapped
                .into_iter()
                .all(|v| (0.0..=f64::from(h)).contains(&v))
        );
        if h.to_bits() == 1.0_f32.to_bits() {
            for (a, b) in mapped.into_iter().zip(sdr_reference(rgb)) {
                assert!((a - b).abs() <= f64::EPSILON);
            }
        }
    }
}

#[test]
fn metal_float_attachment_matches_the_cpu_curve_and_is_monotone() {
    let Some(gpu) = gpu("EDR transform", Some(wgpu::Backend::Metal)) else {
        return;
    };
    let (device, queue) = gpu.device(Profile::Portable, &Restriction::default());
    let samples = samples();
    let width = u32::try_from(peaks().len()).unwrap();
    let height = u32::try_from(HEADROOM.len() * CHROMA.len()).unwrap();
    let output = read_float_target(&device, &queue, &samples, width, height);
    for (inputs, colours) in samples
        .chunks_exact(width as usize)
        .zip(output.chunks_exact(width as usize))
    {
        let mut prior = [0.0; 3];
        for (&[r, g, b, h], &[red, green, blue, _]) in inputs.iter().zip(colours) {
            let rgb = [r, g, b];
            let expected = reference(rgb.map(f64::from), f64::from(h));
            let actual = [red, green, blue];
            for channel in 0..3 {
                let value = actual[channel];
                assert!((0.0..=h).contains(&value), "{rgb:?} H={h}: {actual:?}");
                assert!((f64::from(value) - expected[channel]).abs() < 4e-6);
                if rgb.into_iter().all(|v| f64::from(v) <= KNEE) {
                    assert_eq!(value.to_bits(), rgb[channel].to_bits());
                }
                assert!(
                    value + 2e-6 >= prior[channel],
                    "{rgb:?} H={h}: {actual:?} after {prior:?}"
                );
            }
            prior = actual;
        }
    }
}

fn read_float_target(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    samples: &[[f32; 4]],
    width: u32,
    height: u32,
) -> Vec<[f32; 4]> {
    use wgpu::util::DeviceExt as _;

    let source = format!(
        "{}\nconst WIDTH: u32 = {width}u;\n{}",
        include_str!("../src/synthetic/display/neutral_highlights.wgsl"),
        r"
@group(0) @binding(0) var<storage, read> samples: array<vec4<f32>>;
@vertex fn vertex(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    return vec4(array(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0))[i], 0.0, 1.0);
}
@fragment fn fragment(@builtin(position) p: vec4<f32>) -> @location(0) vec4<f32> {
    let sample = samples[u32(p.y) * WIDTH + u32(p.x)];
    return vec4(neutral_highlights(sample.xyz, 1.0, sample.w), 1.0);
}
"
    );
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("EDR curve samples"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("EDR float reference comparison"),
        layout: None,
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
            targets: &[Some(wgpu::TextureFormat::Rgba32Float.into())],
        }),
        multiview_mask: None,
        cache: None,
    });
    let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Independent EDR samples"),
        contents: bytemuck::cast_slice(samples),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: input.as_entire_binding(),
        }],
    });
    let extent = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("EDR output before display quantization"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.draw(0..3, 0..1);
    }
    let readback = thrum_test_support::readback::Readback::texture(
        device,
        &mut encoder,
        &texture,
        thrum_test_support::readback::Mip::level(0),
        ..,
    );
    let submission = queue.submit([encoder.finish()]);
    let bytes = readback.wait(device, submission);
    bytemuck::cast_slice::<_, [f32; 4]>(&bytes).to_vec()
}
