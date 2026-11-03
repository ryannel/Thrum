//! Opt-in real-device synthetic renderer integration test.
#![cfg(not(target_arch = "wasm32"))]

use thrum_render::abi::MaterialRecord;
use thrum_render::synthetic::{OffscreenTarget, PreparedSynthetic, SyntheticLight, SyntheticScene};
use thrum_render::timing::GpuTimings;

/// Opt-in integration test: a real adapter is required, never replaced by a mock.
#[test]
#[ignore = "requires a real native GPU adapter; run explicitly for device evidence"]
#[expect(
    clippy::too_many_lines,
    reason = "one explicitly opted-in adapter/device lifecycle proves the coupled render, readback and timing paths"
)]
fn prepared_scene_pixels_timestamps_and_disabled_path() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("native adapter required for explicit GPU test");
        let info = adapter.get_info();
        let features = GpuTimings::requested_features(&adapter);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: features,
                ..Default::default()
            })
            .await
            .unwrap();
        let mut scene = SyntheticScene::grid(1).unwrap();
        // Axis-aligned front face gives an independent, constant-normal oracle.
        scene.instances[0].current = [
            [0.6, 0.0, 0.0, 0.0],
            [0.0, 0.6, 0.0, 0.0],
            [0.0, 0.0, 0.1, 0.0],
            [0.0, 0.0, 0.5, 1.0],
        ];
        scene.instances[0].previous = scene.instances[0].current;
        scene.instances[0].normal = [
            [1.0 / 0.6, 0.0, 0.0, 0.0],
            [0.0, 1.0 / 0.6, 0.0, 0.0],
            [0.0, 0.0, 10.0, 0.0],
        ];
        let prepared = PreparedSynthetic::prepare(&device, &queue, 41, &scene)
            .await
            .unwrap();
        // Non-square/non-aligned row extent catches padded-row copy mistakes.
        let target = OffscreenTarget::new(&device, 41, 65, 33).unwrap();
        let graph = prepared.graph(&target, true).unwrap();
        assert_eq!(graph.dependencies, [(0, 1)]);
        let mut pixels = target.readback(&device);
        let mut timings = GpuTimings::new(&device, &queue, 41, 1).unwrap();
        let enabled = timings.is_enabled();
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        for _ in 0..2 {
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            assert!(prepared.encode(&mut encoder, 42, &target, None).is_err());
            prepared
                .encode(&mut encoder, 41, &target, Some(&mut timings))
                .unwrap();
            pixels.encode(&mut encoder, &target).unwrap();
            assert!(pixels.encode(&mut encoder, &target).is_err());
            timings.finish_render(41, &encoder).unwrap();
            // The render encoder has not been submitted; collection must return
            // immediately without recording a same-submission resolve.
            let mut early =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            assert!(!timings.resolve_if_ready(41, &mut early).unwrap());
            let submission = queue.submit([encoder.finish()]);
            device
                .poll(wgpu::PollType::Wait {
                    submission_index: Some(submission),
                    timeout: Some(std::time::Duration::from_secs(10)),
                })
                .unwrap();
            let rgba = pixels
                .try_collect(41)
                .unwrap()
                .expect("mapped pixel output");
            assert_eq!(rgba.len(), 65 * 33 * 4);
            let center = (16 * 65 + 32) * 4;
            let expected = reference_pixel(
                &scene.materials[0],
                [0.0, 0.0, -1.0],
                SyntheticLight::default(),
            );
            for (actual, expected) in rgba[center..center + 4].iter().zip(expected) {
                assert!(
                    actual.abs_diff(expected) <= 1,
                    "center {actual} expected {expected}"
                );
            }
            for (actual, expected) in rgba[..4].iter().zip([5_u8, 8, 10, 255]) {
                assert!(
                    actual.abs_diff(expected) <= 1,
                    "background {actual} expected {expected}"
                );
            }
            if enabled {
                let mut resolve =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                assert!(timings.resolve_if_ready(41, &mut resolve).unwrap());
                let submission = queue.submit([resolve.finish()]);
                device
                    .poll(wgpu::PollType::Wait {
                        submission_index: Some(submission),
                        timeout: Some(std::time::Duration::from_secs(10)),
                    })
                    .unwrap();
            }
            let measured = timings.try_collect(41).unwrap();
            if enabled {
                let measured = measured.unwrap();
                assert_eq!(measured.len(), 1);
                assert!(measured[0].nanoseconds.is_finite() && measured[0].nanoseconds >= 0.0);
                eprintln!("GPU pass diagnostic: {} ns", measured[0].nanoseconds);
            } else {
                assert!(measured.is_none());
            }
        }
        assert!(scope.pop().await.is_none());
        assert!(timings.try_collect(42).is_err());
        assert!(pixels.try_collect(42).is_err());
        // Nonzero instance/material rows exercise both 192/80-byte GPU strides.
        let indexed =
            PreparedSynthetic::prepare(&device, &queue, 41, &SyntheticScene::grid(2).unwrap())
                .await
                .unwrap();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        indexed.encode(&mut encoder, 41, &target, None).unwrap();
        pixels.encode(&mut encoder, &target).unwrap();
        let submission = queue.submit([encoder.finish()]);
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(std::time::Duration::from_secs(10)),
            })
            .unwrap();
        let rgba = pixels.try_collect(41).unwrap().unwrap();
        let grid = SyntheticScene::grid(2).unwrap();
        let normal = grid.instances[0].normal[2][..3]
            .iter()
            .map(|v| -f64::from(*v))
            .collect::<Vec<_>>();
        for (row, x) in [(0, 18), (1, 47)] {
            let expected = reference_pixel(
                &grid.materials[row],
                [normal[0], normal[1], normal[2]],
                SyntheticLight::default(),
            );
            assert_pixel(&rgba, 65, x, 24, expected);
        }
        // Material and source changes must affect transport, with emission
        // remaining visible when direct illumination is disabled/back-facing.
        for (roughness, metalness, light) in [
            (0.0, 0.0, SyntheticLight::default()),
            (
                0.0,
                0.0,
                SyntheticLight {
                    direction_to_light: [0.0, 0.0, -1.0],
                    irradiance: [0.001; 3],
                },
            ),
            (0.3, 1.0, SyntheticLight::default()),
            (
                1.0,
                0.0,
                SyntheticLight {
                    irradiance: [0.0; 3],
                    ..Default::default()
                },
            ),
            (
                0.4,
                0.5,
                SyntheticLight {
                    direction_to_light: [0.0, 0.0, 1.0],
                    ..Default::default()
                },
            ),
        ] {
            let mut variant = scene.clone();
            variant.materials[0].surface = [0.0, roughness, metalness, 1.0];
            variant.materials[0].emissive_cutoff = [0.03, 0.02, 0.01, 0.0];
            let prepared =
                PreparedSynthetic::prepare_with_light(&device, &queue, 41, &variant, light)
                    .await
                    .unwrap();
            let rgba = render_pixels(&device, &queue, &prepared, &target);
            assert_pixel(
                &rgba,
                65,
                32,
                16,
                reference_pixel(&variant.materials[0], [0.0, 0.0, -1.0], light),
            );
        }
        // Nearly antiparallel half-vector and grazing cosines remain finite;
        // zero-source emission must survive even when squared lengths underflow.
        let mut grazing = scene.clone();
        grazing.instances[0].normal[2] = [-1.0, 0.0, 1e-24, 0.0];
        grazing.materials[0].emissive_cutoff = [0.2, 0.1, 0.05, 0.0];
        for irradiance in [[0.0; 3], [2.0; 3]] {
            let light = SyntheticLight {
                direction_to_light: [1e-23, 0.0, 1.0],
                irradiance,
            };
            let prepared =
                PreparedSynthetic::prepare_with_light(&device, &queue, 41, &grazing, light)
                    .await
                    .unwrap();
            let rgba = render_pixels(&device, &queue, &prepared, &target);
            assert_pixel(
                &rgba,
                65,
                32,
                16,
                reference_pixel(&grazing.materials[0], [1.0, 0.0, -1e-24], light),
            );
        }
        // Submit near geometry FIRST; a later far draw must not overwrite it.
        // Reversing order also preserves the near material. Emission is exact
        // with the source disabled, independent of the BRDF oracle.
        for reversed in [false, true] {
            let mut overlap = scene.clone();
            overlap.materials[0].emissive_cutoff = [0.2, 0.1, 0.0, 0.0];
            overlap.materials[1].emissive_cutoff = [0.0, 0.1, 0.4, 0.0];
            let mut far = overlap.instances[0].clone();
            far.ids = [1, 1, 0, 1];
            far.current[3][2] = 0.8;
            far.previous = far.current;
            overlap.instances.push(far);
            if reversed {
                overlap.instances.reverse();
            }
            let prepared = PreparedSynthetic::prepare_with_light(
                &device,
                &queue,
                41,
                &overlap,
                SyntheticLight {
                    irradiance: [0.0; 3],
                    ..Default::default()
                },
            )
            .await
            .unwrap();
            let rgba = render_pixels(&device, &queue, &prepared, &target);
            assert_pixel(&rgba, 65, 32, 16, [51, 26, 0, 255]);
        }
        // 10k real lit cubes at 1080p, prepared/graph-compiled before timing.
        let large =
            PreparedSynthetic::prepare(&device, &queue, 41, &SyntheticScene::grid(10_000).unwrap())
                .await
                .unwrap();
        let large_target = OffscreenTarget::new(&device, 41, 1920, 1080).unwrap();
        let _graph = large.graph(&large_target, false).unwrap();
        let mut gpu_ns = Vec::new();
        let mut enabled_cpu_ns = Vec::new();
        let mut disabled_cpu_ns = Vec::new();
        for frame in 0..37 {
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            let start = std::time::Instant::now();
            large
                .encode(&mut encoder, 41, &large_target, Some(&mut timings))
                .unwrap();
            timings.finish_render(41, &encoder).unwrap();
            let cpu_ns = start.elapsed().as_nanos() as f64;
            submit_wait(&device, &queue, encoder);
            if enabled {
                let mut resolve =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                assert!(timings.resolve_if_ready(41, &mut resolve).unwrap());
                submit_wait(&device, &queue, resolve);
                let values = timings.try_collect(41).unwrap().unwrap();
                assert_eq!(values[0].name.as_ref(), "synthetic forward opaque");
                if frame >= 5 {
                    gpu_ns.push(values[0].nanoseconds);
                }
            }
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            let start = std::time::Instant::now();
            large.encode(&mut encoder, 41, &large_target, None).unwrap();
            let plain_cpu_ns = start.elapsed().as_nanos() as f64;
            submit_wait(&device, &queue, encoder);
            if frame >= 5 {
                enabled_cpu_ns.push(cpu_ns);
                disabled_cpu_ns.push(plain_cpu_ns);
            }
        }
        report("10k cubes 1920x1080 GPU forward opaque ns", &gpu_ns);
        report(
            if enabled {
                "CPU encode+finish_render timestamp-enabled ns"
            } else {
                "CPU encode+finish_render unsupported-timing ns"
            },
            &enabled_cpu_ns,
        );
        report("CPU encode timestamp-disabled ns", &disabled_cpu_ns);
        let rgba = render_pixels(&device, &queue, &large, &large_target);
        assert!(
            rgba.as_chunks::<4>()
                .0
                .iter()
                .filter(|p| p[0] > 20 || p[1] > 20 || p[2] > 20)
                .count()
                > 100_000,
            "10k lit geometry must cover meaningful pixels"
        );
        if let Ok(path) = std::env::var("THRUM_RENDER_CAPTURE") {
            let mut ppm = b"P6\n1920 1080\n255\n".to_vec();
            for pixel in rgba.as_chunks::<4>().0 {
                ppm.extend_from_slice(&pixel[..3]);
            }
            std::fs::write(path, ppm).unwrap();
        }
        // Feature not enabled on a device means successful unsupported behavior,
        // even when the physical adapter does support timestamps.
        let (plain_device, plain_queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .unwrap();
        let mut disabled = GpuTimings::new(&plain_device, &plain_queue, 42, 1).unwrap();
        assert!(!disabled.is_enabled());
        assert!(disabled.pass_writes(42, "unsupported").unwrap().is_none());
        let mut encoder =
            plain_device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        disabled.finish_render(42, &encoder).unwrap();
        assert!(!disabled.resolve_if_ready(42, &mut encoder).unwrap());
        assert!(disabled.try_collect(42).unwrap().is_none());
        eprintln!(
            "adapter={} backend={:?} driver={} timestamps={enabled}; directional BRDF/emission/depth pixel checks and 10,000-cube 1080p run passed",
            info.name, info.backend, info.driver
        );
    });
}

fn submit_wait(device: &wgpu::Device, queue: &wgpu::Queue, encoder: wgpu::CommandEncoder) {
    let submission = queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(std::time::Duration::from_secs(10)),
        })
        .expect("explicit GPU host operation must succeed");
}
fn render_pixels(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    prepared: &PreparedSynthetic,
    target: &OffscreenTarget,
) -> Vec<u8> {
    let mut pixels = target.readback(device);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    prepared
        .encode(&mut encoder, 41, target, None)
        .expect("explicit GPU host operation must succeed");
    pixels
        .encode(&mut encoder, target)
        .expect("explicit GPU host operation must succeed");
    submit_wait(device, queue, encoder);
    pixels
        .try_collect(41)
        .expect("pixel map must succeed")
        .expect("submission completed pixel map")
}
fn assert_pixel(rgba: &[u8], width: usize, x: usize, y: usize, expected: [u8; 4]) {
    let offset = (y * width + x) * 4;
    for (actual, expected) in rgba[offset..offset + 4].iter().zip(expected) {
        assert!(
            actual.abs_diff(expected) <= 2,
            "pixel ({x},{y}) got {:?}, expected channel {expected}",
            &rgba[offset..offset + 4]
        );
    }
}
#[expect(
    clippy::print_stderr,
    reason = "the opt-in GPU evidence harness explicitly reports bounded timing diagnostics"
)]
fn report(name: &str, samples: &[f64]) {
    if samples.is_empty() {
        return;
    }
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    eprintln!(
        "{name}: n={} min={:.0} median={:.0} p95={:.0} max={:.0}",
        sorted.len(),
        sorted[0],
        sorted[sorted.len() / 2],
        sorted[(sorted.len() * 95 / 100).min(sorted.len() - 1)],
        sorted[sorted.len() - 1]
    );
}
/// Independent f64 evaluation of the literal specification, without the shader's
/// Smith cancellation. Default texture texels are all multiplicative identities.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the independent oracle explicitly clamps and rounds its linear unorm output to [0,255]"
)]
fn reference_pixel(material: &MaterialRecord, normal: [f64; 3], light: SyntheticLight) -> [u8; 4] {
    let dot = |a: [f64; 3], b: [f64; 3]| a.into_iter().zip(b).map(|(x, y)| x * y).sum::<f64>();
    let normalize = |a: [f64; 3]| {
        let length = dot(a, a).sqrt();
        a.map(|v| v / length)
    };
    let n = normalize(normal);
    let l = normalize(light.direction_to_light.map(f64::from));
    let v = [0.0, 0.0, -1.0];
    let ci = dot(n, l).max(0.0);
    let co = dot(n, v).max(0.0);
    let mut out = [0_u8; 4];
    for (channel, output) in out[..3].iter_mut().enumerate() {
        let mut radiance = f64::from(material.emissive_cutoff[channel]);
        if ci > 0.0 && co > 0.0 {
            let h = normalize(std::array::from_fn(|i| l[i] + v[i]));
            let ch = dot(n, h).clamp(0.0, 1.0);
            let vh = dot(v, h).clamp(0.0, 1.0);
            let a2 = f64::from(material.surface[1]).max(0.045).powi(4);
            let d = a2 / (std::f64::consts::PI * (1.0 + ch * ch * (a2 - 1.0)).powi(2));
            let g = 2.0 * ci * co
                / (ci * (a2 + (1.0 - a2) * co * co).sqrt()
                    + co * (a2 + (1.0 - a2) * ci * ci).sqrt());
            let m = f64::from(material.surface[2]);
            let base = f64::from(material.base_rgba[channel]);
            let f0 = 0.04 * (1.0 - m) + base * m;
            let f = f0 + (1.0 - f0) * (1.0 - vh).powi(5);
            radiance += ((1.0 - m) * (1.0 - f) * base / std::f64::consts::PI
                + d * g * f / (4.0 * ci * co))
                * f64::from(light.irradiance[channel])
                * ci;
        }
        *output = (radiance.clamp(0.0, 1.0) * 255.0).round() as u8;
    }
    out[3] = 255;
    out
}
