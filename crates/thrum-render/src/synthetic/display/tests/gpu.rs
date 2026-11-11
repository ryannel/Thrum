use super::*;
use std::{sync::mpsc, time::Duration};
use wgpu::util::DeviceExt;

fn half(bits: u16) -> f64 {
    let exponent = (bits >> 10) & 31;
    let fraction = f64::from(bits & 1023);
    if exponent == 0 {
        fraction * 2.0_f64.powi(-24)
    } else {
        (1.0 + fraction / 1024.0) * 2.0_f64.powi(i32::from(exponent) - 15)
    }
}

fn legacy_pipeline(device: &wgpu::Device, layout: &wgpu::BindGroupLayout) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("70cc9d3 exact-pixel oracle"),
        source: wgpu::ShaderSource::Wgsl(include_str!("../legacy_display.wgsl").into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
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
                format: OUTPUT_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn render(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pipeline: &wgpu::RenderPipeline,
    group: &wgpu::BindGroup,
    width: u32,
) -> Vec<u8> {
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: OUTPUT_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let row_bytes = (width * 4).div_ceil(256) * 256;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: u64::from(row_bytes),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
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
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, group, &[]);
        pass.draw(0..3, 0..1);
    }
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row_bytes),
                rows_per_image: Some(1),
            },
        },
        target.size(),
    );
    let (send, receive) = mpsc::channel();
    encoder.map_buffer_on_submit(&readback, wgpu::MapMode::Read, .., move |result| {
        send.send(result).unwrap();
    });
    let submission = queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(Duration::from_secs(10)),
        })
        .unwrap();
    receive.recv().unwrap().unwrap();
    readback.slice(..).get_mapped_range().unwrap()[..width as usize * 4].to_vec()
}

#[test]
fn gpu_sdr_ramps_match_f64_and_legacy_pixels_across_checked_gain_range() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .unwrap();
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        // Exact half input: black/subnormal, knee±one half ULP, neutral and
        // saturated ramps, dominant-channel ties, and the finite fp16 maximum.
        let mut cases = Vec::new();
        for bits in [
            0x0000, 0x0001, 0x0400, 0x1400, 0x3000, 0x3800, 0x3a00, 0x3a14, 0x3a15, 0x3c00, 0x4000,
            0x4400, 0x5c00, 0x7bff,
        ] {
            cases.extend([[bits; 3], [bits, 0, 0], [bits, bits, 0], [0, bits, bits]]);
        }
        let width = u32::try_from(cases.len()).unwrap();
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("exact half tone ramp"),
            size: wgpu::Extent3d {
                width,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let bytes: Vec<u8> = cases
            .iter()
            .flat_map(|rgb| rgb.iter().copied().chain([0x3c00]))
            .flat_map(u16::to_le_bytes)
            .collect();
        queue.write_texture(
            texture.as_image_copy(),
            &bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 8),
                rows_per_image: Some(1),
            },
            texture.size(),
        );
        let display = DisplayPrepared::new(&crate::runtime::DeviceSite::untracked(
            wgpu::Device::clone(&device),
        ));
        let original = legacy_pipeline(&device, &display.layout);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        for gain in [
            f32::MIN_POSITIVE * 0.5,
            1.0 / 16_000.0,
            PRE_EXPOSURE,
            1.0,
            f32::MAX * PRE_EXPOSURE,
        ] {
            crate::synthetic::checked_display_gain(gain).unwrap();
            let inverse = 1.0 / gain;
            let mut words = [0_u32; 8];
            words[0] = PRE_EXPOSURE.to_bits();
            words[1] = inverse.to_bits();
            let sample = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: words.map(u32::to_le_bytes).as_flattened(),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
            let group = prepare_group(&device, &display.layout, &view, &view, &sample);
            let expected_legacy = render(&device, &queue, &original, &group, width);
            let actual_legacy = render(&device, &queue, &display.pipeline, &group, width);
            // The display adds a fixed one-step triangular dither; the legacy reference has none.
            for (index, (actual, expected)) in
                actual_legacy.iter().zip(&expected_legacy).enumerate()
            {
                assert!(
                    actual.abs_diff(*expected) <= 1,
                    "legacy pixels within the dither step at gain {gain}, byte {index}: {actual} vs {expected}"
                );
                if matches!(*expected, 0 | 255) {
                    assert_eq!(
                        actual, expected,
                        "dither leaves black and white exact at gain {gain}"
                    );
                }
            }
            words[7] = SdrToneMap::NeutralHighlights as u32;
            queue.write_buffer(&sample, 0, words.map(u32::to_le_bytes).as_flattened());
            let actual = render(&device, &queue, &display.pipeline, &group, width);
            for (rgb, pixel) in cases.iter().zip(actual.as_chunks::<4>().0) {
                let expected = reference(rgb.map(half), inverse).map(display_byte);
                assert_eq!(pixel[3], 255);
                for (actual, expected) in pixel[..3].iter().zip(expected) {
                    assert!(
                        actual.abs_diff(expected) <= 1,
                        "gain={gain}, half={rgb:?}, pixel={pixel:?}, expected={expected}"
                    );
                }
                if rgb[0] == rgb[1] && rgb[1] == rgb[2] {
                    assert_eq!(pixel[0], pixel[1]);
                    assert_eq!(pixel[1], pixel[2]);
                }
            }
        }
        assert!(scope.pop().await.is_none());
    });
}

/// The display codes the shader writes for bands -6 to +6.
const ZONE_CODES: [[u8; 3]; 13] = [
    [20, 0, 40],
    [90, 20, 140],
    [40, 60, 200],
    [20, 140, 230],
    [20, 190, 180],
    [90, 170, 90],
    [128, 128, 128],
    [190, 220, 70],
    [250, 220, 40],
    [250, 140, 20],
    [230, 40, 30],
    [250, 110, 200],
    [255, 255, 255],
];

#[test]
fn gpu_exposure_zones_band_exposed_luminance_for_every_tone_map_and_gain() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .unwrap();
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let reference = 0.18 / (std::f64::consts::PI * 1.2 * 0.4);
        // Exposed luminance in stops from the reference, each 0.3 stop inside a band, plus
        // black and values beyond both ends. Colour must not matter, only luminance.
        let mut stops = vec![None];
        for band in -8_i32..=8 {
            for offset in [-0.3, 0.0, 0.3] {
                stops.push(Some(f64::from(band) + offset));
            }
        }
        let tints = [[1.0, 1.0, 1.0], [2.0, 0.5, 0.25]];
        let display = DisplayPrepared::new(&crate::runtime::DeviceSite::untracked(
            wgpu::Device::clone(&device),
        ));
        for inverse in [1.0_f32 / 16_000.0, 1.0, 2_500.0] {
            let gain = f64::from(PRE_EXPOSURE) * f64::from(inverse);
            let mut texels = Vec::new();
            let mut expected = Vec::new();
            for stop in &stops {
                for tint in tints {
                    let luminance = stop.map_or(0.0, |stop| reference * 2.0_f64.powf(stop) * gain);
                    let weight = 0.2126 * tint[0] + 0.7152 * tint[1] + 0.0722 * tint[2];
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "test texels are stored at the f32 precision the shader loads"
                    )]
                    texels.push(tint.map(|channel| (channel * luminance / weight) as f32));
                    #[expect(
                        clippy::cast_possible_truncation,
                        clippy::cast_sign_loss,
                        reason = "the rounded band is clamped into the 13-entry palette"
                    )]
                    let band =
                        stop.map_or(0, |stop| (stop.round().clamp(-6.0, 6.0) + 6.0) as usize);
                    expected.push(ZONE_CODES[band]);
                }
            }
            let width = u32::try_from(texels.len()).unwrap();
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("exposed luminance bands"),
                size: wgpu::Extent3d {
                    width,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba32Float,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let bytes: Vec<u8> = texels
                .iter()
                .flat_map(|rgb| rgb.iter().copied().chain([1.0]))
                .flat_map(f32::to_le_bytes)
                .collect();
            queue.write_texture(
                texture.as_image_copy(),
                &bytes,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(width * 16),
                    rows_per_image: Some(1),
                },
                texture.size(),
            );
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let mut words = [0_u32; 8];
            words[0] = PRE_EXPOSURE.to_bits();
            words[1] = inverse.to_bits();
            let sample = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: words.map(u32::to_le_bytes).as_flattened(),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
            let group = prepare_group(&device, &display.layout, &view, &view, &sample);
            let mut outputs = Vec::new();
            for tone_map in [SdrToneMap::Reinhard, SdrToneMap::NeutralHighlights] {
                words[7] = display_selectors(tone_map, DisplayOverlay::ExposureZones);
                queue.write_buffer(&sample, 0, words.map(u32::to_le_bytes).as_flattened());
                let actual = render(&device, &queue, &display.pipeline, &group, width);
                for (index, (pixel, expected)) in
                    actual.as_chunks::<4>().0.iter().zip(&expected).enumerate()
                {
                    assert_eq!(
                        pixel,
                        &[expected[0], expected[1], expected[2], 255],
                        "inverse={inverse}, {tone_map:?}, texel {index}: {:?} stops",
                        stops[index / tints.len()]
                    );
                }
                outputs.push(actual);
            }
            assert_eq!(outputs[0], outputs[1], "the overlay ignores the tone map");
        }
        assert!(scope.pop().await.is_none());
    });
}
