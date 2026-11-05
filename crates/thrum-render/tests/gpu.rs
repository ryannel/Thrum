//! Opt-in real-device synthetic renderer integration test.
#![cfg(not(target_arch = "wasm32"))]

use thrum_render::abi::{InstanceRecord, MaterialRecord};
use thrum_render::synthetic::{
    DEPTH_FORMAT, OffscreenTarget, PreparedSynthetic, SyntheticLight, SyntheticPointLight,
    SyntheticScene, SyntheticView,
};
use thrum_render::timing::GpuTimings;

/// Opt-in integration test: a real adapter is required, never replaced by a mock.
#[test]
#[ignore = "requires a real native GPU adapter; run explicitly for device evidence"]
#[expect(
    clippy::too_many_lines,
    reason = "one explicitly opted-in adapter/device lifecycle proves the coupled render, readback and timing paths"
)]
fn prepared_scene_pixels_timestamps_and_disabled_path() {
    assert_eq!(DEPTH_FORMAT, wgpu::TextureFormat::Depth32Float);
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
        let prepare_start = std::time::Instant::now();
        let prepared = PreparedSynthetic::prepare(&device, &queue, 41, &scene)
            .await
            .unwrap();
        eprintln!(
            "cold prepare forward+display pipelines: {} ns",
            prepare_start.elapsed().as_nanos()
        );
        // Non-square/non-aligned row extent catches padded-row copy mistakes.
        let target = OffscreenTarget::new(&device, &prepared, 65, 33).unwrap();
        let graph = prepared.graph(&target, true).unwrap();
        assert_eq!(graph.dependencies, [(0, 1), (1, 2)]);
        assert_eq!(
            prepared.graph(&target, false).unwrap().dependencies,
            [(0, 1)]
        );
        let mut pixels = target.readback(&device);
        let mut timings = GpuTimings::new(&device, &queue, 41, 2).unwrap();
        let enabled = timings.is_enabled();
        if enabled {
            let mut one_pass = GpuTimings::new(&device, &queue, 41, 1).unwrap();
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            assert!(matches!(
                prepared.encode(&mut encoder, 41, &target, Some(&mut one_pass)),
                Err(thrum_render::synthetic::RenderError::Timing(
                    thrum_render::timing::TimingError::Capacity
                ))
            ));
            // No timestamp labels were reserved by the rejected whole-view check.
            assert!(one_pass.pass_writes(41, "still unused").unwrap().is_some());
            prepared
                .encode(&mut encoder, 41, &target, Some(&mut timings))
                .unwrap();
            // This is deliberately unsubmitted preparation validation. Start a
            // fresh timing batch for the submitted measurements below.
            timings = GpuTimings::new(&device, &queue, 41, 2).unwrap();
        }

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
            for (actual, expected) in rgba[..4]
                .iter()
                .zip(reference_display([0.02, 0.03, 0.04], 1.0))
            {
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
                assert_eq!(measured.len(), 2);
                assert_eq!(measured[0].name.as_ref(), "synthetic forward opaque");
                assert_eq!(measured[1].name.as_ref(), "synthetic tone encode");
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
            assert_pixel(&rgba, 65, 32, 16, reference_display([0.2, 0.1, 0.0], 1.0));
        }
        // Above-one radiance must retain brightness contrast through fp16;
        // pre-clamping the forward output would make both cards identical.
        for exposure in [0.25, 1.0] {
            let mut emitted = Vec::new();
            for radiance in [2.0, 8.0] {
                let mut card = scene.clone();
                card.materials[0].emissive_cutoff = [radiance; 4];
                card.materials[0].emissive_cutoff[3] = 0.0;
                let prepared = PreparedSynthetic::prepare_with_light_and_exposure(
                    &device,
                    &queue,
                    41,
                    &card,
                    SyntheticLight {
                        irradiance: [0.0; 3],
                        ..Default::default()
                    },
                    exposure,
                )
                .await
                .unwrap();
                let card_target = OffscreenTarget::new(&device, &prepared, 65, 33).unwrap();
                let rgba = render_pixels(&device, &queue, &prepared, &card_target);
                assert_pixel(
                    &rgba,
                    65,
                    32,
                    16,
                    reference_display([f64::from(radiance); 3], f64::from(exposure)),
                );
                assert_pixel(
                    &rgba,
                    65,
                    0,
                    0,
                    reference_display([0.02, 0.03, 0.04], f64::from(exposure)),
                );
                emitted.push(rgba[(16 * 65 + 32) * 4]);
                if exposure.to_bits() != 1.0_f32.to_bits() {
                    let mut encoder =
                        device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                    assert!(prepared.encode(&mut encoder, 41, &target, None).is_err());
                    assert!(prepared.graph(&target, false).is_err());
                }
            }
            assert!(
                emitted[1] > emitted[0] + 10,
                "HDR contrast survives: {emitted:?}"
            );
        }
        // Saturated HDR must preserve channel ratios through the shared gamut
        // compression rather than clipping each exposed channel separately.
        // Each prepared pipeline also renders two extents without preparation.
        // Finite maximum emission tests bounded white at the SDR asymptote;
        // the oracle need not reproduce the intermediate fp16 clamp to know it.
        for emission in [[8.0, 0.5, 0.125], [0.25, 0.5, 16.0], [f32::MAX; 3]] {
            let mut card = scene.clone();
            card.materials[0].emissive_cutoff = [emission[0], emission[1], emission[2], 0.0];
            let prepared = PreparedSynthetic::prepare_with_light(
                &device,
                &queue,
                41,
                &card,
                SyntheticLight {
                    irradiance: [0.0; 3],
                    ..Default::default()
                },
            )
            .await
            .unwrap();
            let expected = reference_display(emission.map(f64::from), 1.0);
            assert_eq!(expected[..3].iter().copied().max(), Some(255));
            for [width, height] in [[65, 33], [97, 49]] {
                let resized = OffscreenTarget::new(&device, &prepared, width, height).unwrap();
                assert_eq!(
                    prepared.graph(&resized, true).unwrap().dependencies,
                    [(0, 1), (1, 2)]
                );
                let rgba = render_pixels(&device, &queue, &prepared, &resized);
                assert_eq!(rgba.len(), width as usize * height as usize * 4);
                assert_pixel(
                    &rgba,
                    width as usize,
                    (width / 2) as usize,
                    (height / 2) as usize,
                    expected,
                );
                assert_pixel(
                    &rgba,
                    width as usize,
                    0,
                    0,
                    reference_display([0.02, 0.03, 0.04], 1.0),
                );
            }
        }
        // 10k real lit cubes at 1080p, prepared/graph-compiled before timing.
        let large =
            PreparedSynthetic::prepare(&device, &queue, 41, &SyntheticScene::grid(10_000).unwrap())
                .await
                .unwrap();
        let large_target = OffscreenTarget::new(&device, &large, 1920, 1080).unwrap();
        let _graph = large.graph(&large_target, false).unwrap();
        let mut gpu_ns = Vec::new();
        let mut display_gpu_ns = Vec::new();
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
                    assert_eq!(values[1].name.as_ref(), "synthetic tone encode");
                    display_gpu_ns.push(values[1].nanoseconds);
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
        report("1920x1080 GPU tone encode ns", &display_gpu_ns);
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
                .filter(|p| p[0] > 60 || p[1] > 60 || p[2] > 60)
                .count()
                > 100_000,
            "10k lit geometry must cover meaningful pixels"
        );
        if let Ok(path) = std::env::var("THRUM_RENDER_CAPTURE") {
            let mut ppm = b"P6\n# sRGB encoded after authored exposure and tone/gamut mapping\n1920 1080\n255\n".to_vec();
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
            "adapter={} backend={:?} driver={} timestamps={enabled}; directional BRDF/emission/depth, HDR contrast, authored exposure and tone/sRGB pixel checks plus 10,000-cube 1080p run passed",
            info.name, info.backend, info.driver
        );
    });
}

#[test]
#[ignore = "requires a real native GPU adapter; run explicitly for device evidence"]
fn clustered_local_light_changes_only_its_receiver_region() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("native adapter");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("device");
        let mut scene = SyntheticScene::grid(1).expect("fixture");
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
        let unlit = SyntheticLight {
            direction_to_light: [0.0, 0.0, -1.0],
            irradiance: [0.0; 3],
        };
        let baseline = PreparedSynthetic::prepare_with_light(&device, &queue, 41, &scene, unlit)
            .await
            .expect("baseline prepare");
        let baseline_target = OffscreenTarget::new(&device, &baseline, 65, 33).expect("target");
        let dark = render_pixels(&device, &queue, &baseline, &baseline_target);
        scene.local_lights.push(SyntheticPointLight {
            position: [0.0, 0.0, 0.0],
            radius: 0.9,
            candela: [4.0, 0.2, 0.1],
        });
        let lit = PreparedSynthetic::prepare_with_light(&device, &queue, 41, &scene, unlit)
            .await
            .expect("clustered prepare");
        assert_eq!(lit.dropped_light_references(), 0);
        let lit_target = OffscreenTarget::new(&device, &lit, 65, 33).expect("lit target");
        let bright = render_pixels(&device, &queue, &lit, &lit_target);
        let center = (16 * 65 + 32) * 4;
        assert_eq!(&dark[center..center + 3], &[0, 0, 0]);
        assert!(bright[center] > 100, "local source must light cube center");
        assert!(bright[center] > bright[center + 1]);
        assert_eq!(&dark[..4], &bright[..4], "background stays unlit");
        scene.local_lights[0].position = [0.0, 0.0, 0.4];
        let coincident = PreparedSynthetic::prepare_with_light(&device, &queue, 41, &scene, unlit)
            .await
            .expect("coincident light prepare");
        let coincident_target =
            OffscreenTarget::new(&device, &coincident, 65, 33).expect("coincident target");
        let near = render_pixels(&device, &queue, &coincident, &coincident_target);
        assert!(
            near[center] > 100,
            "near-field source must not create a dark core"
        );
        scene.local_lights[0].candela = [0.0; 3];
        let disabled = PreparedSynthetic::prepare_with_light(&device, &queue, 41, &scene, unlit)
            .await
            .expect("disabled source prepare");
        let disabled_target =
            OffscreenTarget::new(&device, &disabled, 65, 33).expect("disabled target");
        let disabled_pixels = render_pixels(&device, &queue, &disabled, &disabled_target);
        assert_eq!(
            disabled_pixels, dark,
            "zero-candela light must not illuminate"
        );
    });
}

#[test]
#[ignore = "requires a real native GPU adapter; run explicitly for device evidence"]
fn perspective_cluster_and_target_aux_light_visible_pixels() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("native adapter");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("device");
        let mut scene = SyntheticScene::grid(1).expect("fixture");
        scene.view = SyntheticView::perspective(
            [0.0, 0.0, 2.0],
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            1.0,
            65.0 / 33.0,
            0.1,
            20.0,
        )
        .expect("finite perspective");
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
        let unlit = SyntheticLight {
            direction_to_light: [0.0, 0.0, 1.0],
            irradiance: [0.0; 3],
        };
        let dark = PreparedSynthetic::prepare_with_light(&device, &queue, 41, &scene, unlit)
            .await
            .expect("dark prepare");
        let dark_target = OffscreenTarget::new(&device, &dark, 65, 33).expect("dark target");
        let baseline = render_pixels(&device, &queue, &dark, &dark_target);
        scene.local_lights.push(SyntheticPointLight {
            position: [0.35, 0.0, 0.8],
            radius: 0.35,
            candela: [4.0, 0.2, 0.1],
        });
        let lit = PreparedSynthetic::prepare_with_light(&device, &queue, 41, &scene, unlit)
            .await
            .expect("perspective prepare");
        assert_eq!(lit.dropped_light_references(), 0);
        let lit_target = OffscreenTarget::new(&device, &lit, 65, 33).expect("lit target");
        let pixels = render_pixels(&device, &queue, &lit, &lit_target);
        let center = (16 * 65 + 32) * 4;
        let receiver = (16 * 65 + 40) * 4;
        assert_eq!(&baseline[center..center + 3], &[0, 0, 0]);
        assert_eq!(&pixels[center..center + 3], &baseline[center..center + 3]);
        assert!(
            pixels[receiver] > 100,
            "off-axis local light reaches only its projected receiver"
        );
        assert_eq!(&baseline[..4], &pixels[..4], "background remains unchanged");
        let resized = OffscreenTarget::new(&device, &lit, 130, 66).expect("resized target");
        let resized_pixels = render_pixels(&device, &queue, &lit, &resized);
        assert!(
            resized_pixels[(33 * 130 + 80) * 4] > 100,
            "resized target has correct view aux"
        );
        let original_view = scene.view;
        scene.view = SyntheticView::perspective(
            [0.1, 0.0, 2.0],
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            1.0,
            65.0 / 33.0,
            0.1,
            20.0,
        )
        .expect("moved view");
        let moved = PreparedSynthetic::prepare_with_light(&device, &queue, 41, &scene, unlit)
            .await
            .expect("moved prepare");
        let mut stale_encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        assert!(matches!(
            moved.encode(&mut stale_encoder, 41, &lit_target, None),
            Err(thrum_render::synthetic::RenderError::StaleTarget)
        ));
        scene.view = original_view;
        scene.local_lights[0].position = [5.0, 0.0, 0.8];
        let offscreen = PreparedSynthetic::prepare_with_light(&device, &queue, 41, &scene, unlit)
            .await
            .expect("offscreen prepare");
        let offscreen_target = OffscreenTarget::new(&device, &offscreen, 65, 33).expect("target");
        assert_eq!(
            render_pixels(&device, &queue, &offscreen, &offscreen_target),
            baseline
        );
    });
}

fn place_box(instance: &mut InstanceRecord, center: [f32; 3], scale: [f32; 3]) {
    instance.current = [
        [scale[0], 0.0, 0.0, 0.0],
        [0.0, scale[1], 0.0, 0.0],
        [0.0, 0.0, scale[2], 0.0],
        [center[0], center[1], center[2], 1.0],
    ];
    instance.previous = instance.current;
    instance.normal = [
        [1.0 / scale[0], 0.0, 0.0, 0.0],
        [0.0, 1.0 / scale[1], 0.0, 0.0],
        [0.0, 0.0, 1.0 / scale[2], 0.0],
    ];
}

#[test]
#[ignore = "requires a real native GPU adapter; run explicitly for device evidence"]
fn two_sun_cascades_shadow_near_and_far_receivers_from_off_camera_casters() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("native adapter");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("device");
        let light = SyntheticLight {
            direction_to_light: [1.0, 0.0, 1.0],
            irradiance: [4.0; 3],
        };
        for (receiver, caster, receiver_scale, sample_x) in [
            ([0.0, 0.0, 0.0], [1.5, 0.0, 1.0], [1.0, 1.0, 0.01], 84),
            ([4.0, 0.0, -8.0], [9.0, 0.0, -3.0], [1.0, 1.0, 0.01], 107),
        ] {
            let mut scene = SyntheticScene::grid(2).expect("two boxes");
            scene.view = SyntheticView::perspective(
                [0.0, 0.0, 3.0],
                [0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                1.0,
                1.0,
                0.1,
                20.0,
            )
            .expect("perspective camera");
            place_box(&mut scene.instances[0], receiver, receiver_scale);
            place_box(&mut scene.instances[1], caster, [0.2; 3]);
            let unshadowed =
                PreparedSynthetic::prepare_with_light(&device, &queue, 41, &scene, light)
                    .await
                    .expect("direct Sun prepare");
            let unshadowed_target =
                OffscreenTarget::new(&device, &unshadowed, 129, 129).expect("direct target");
            let direct = render_pixels(&device, &queue, &unshadowed, &unshadowed_target);
            scene.sun_shadows = true;
            let mut shadowed =
                PreparedSynthetic::prepare_with_light(&device, &queue, 41, &scene, light)
                    .await
                    .expect("cascades prepare");
            let target = OffscreenTarget::new(&device, &shadowed, 129, 129).expect("shadow target");
            let graph = shadowed
                .graph(&target, true)
                .expect("four passes plus readback");
            assert!(graph.dependencies.contains(&(0, 2)));
            assert!(graph.dependencies.contains(&(1, 2)));
            assert!(graph.dependencies.contains(&(2, 3)));
            let shaded = render_pixels(&device, &queue, &shadowed, &target);
            if let Some(dir) = std::env::var_os("THRUM_SHADOW_CAPTURE_DIR") {
                let name = if sample_x == 84 { "near" } else { "far" };
                for (suffix, pixels) in [("direct", &direct), ("shadow", &shaded)] {
                    let mut ppm = Vec::with_capacity(129 * 129 * 3 + 32);
                    ppm.extend_from_slice(b"P6\n129 129\n255\n");
                    for rgba in pixels.as_chunks::<4>().0 {
                        ppm.extend_from_slice(&rgba[..3]);
                    }
                    std::fs::write(
                        std::path::Path::new(&dir).join(format!("{name}-{suffix}.ppm")),
                        ppm,
                    )
                    .expect("optional shadow proof capture");
                }
            }
            let pixel = (64 * 129 + sample_x) * 4;
            assert!(
                direct[pixel] > 80,
                "receiver is Sun-lit before shadowing: {direct_pixel:?}",
                direct_pixel = &direct[pixel..pixel + 4]
            );
            assert!(
                u16::from(shaded[pixel]) + 25 < u16::from(direct[pixel]),
                "off-camera caster shadows receiver: direct={direct_pixel:?}, shadow={shadow_pixel:?}",
                direct_pixel = &direct[pixel..pixel + 4],
                shadow_pixel = &shaded[pixel..pixel + 4]
            );
            if sample_x == 84 {
                let other_light = SyntheticLight {
                    direction_to_light: [-1.0, 0.0, 1.0],
                    ..light
                };
                let other =
                    PreparedSynthetic::prepare_with_light(&device, &queue, 41, &scene, other_light)
                        .await
                        .expect("changed Sun prepare");
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                assert!(
                    matches!(
                        other.encode(&mut encoder, 41, &target, None),
                        Err(thrum_render::synthetic::RenderError::StaleTarget)
                    ),
                    "target cannot sample a previous Sun projection"
                );

                scene.instances[1].current[3][0] = 50.0;
                scene.instances[1].current[3][2] = 50.0;
                let report = shadowed
                    .upload_instances(&queue, &scene.instances, &[1])
                    .expect("out-of-envelope presentation update is accepted");
                assert_eq!(report.changed_rows, 1);
                assert_eq!(report.clamped_shadow_casters, 1);
                assert!(
                    shadowed
                        .upload_instances(&queue, &scene.instances, &[1, 1])
                        .is_err()
                );
                let steady = shadowed
                    .upload_instances(&queue, &scene.instances, &[])
                    .expect("unchanged frame retains clamp telemetry");
                assert_eq!((steady.changed_rows, steady.clamped_shadow_casters), (0, 1));
                assert_ne!(
                    render_pixels(&device, &queue, &shadowed, &target),
                    shaded,
                    "moved caster must not remain at its previous GPU pose"
                );
            }
        }
        let mut off = SyntheticScene::grid(1).expect("off-state fixture");
        off.view = SyntheticView::perspective(
            [0.0, 0.0, 3.0],
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            1.0,
            1.0,
            0.1,
            20.0,
        )
        .expect("camera");
        off.sun_shadows = true;
        let off_light = SyntheticLight {
            irradiance: [0.0; 3],
            ..light
        };
        let off_prepared =
            PreparedSynthetic::prepare_with_light(&device, &queue, 41, &off, off_light)
                .await
                .expect("disabled Sun prepare");
        let off_target =
            OffscreenTarget::new(&device, &off_prepared, 65, 33).expect("off-state target");
        assert_eq!(
            off_prepared
                .graph(&off_target, false)
                .expect("off-state graph")
                .dependencies,
            [(0, 1)],
            "disabled Sun has no shadow passes or map readers"
        );
    });
}

#[test]
#[ignore = "requires a real native GPU adapter; run explicitly for device evidence"]
fn cascade_overlap_keeps_caster_occlusion_across_split() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("native adapter");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("device");
        let light = SyntheticLight {
            direction_to_light: [1.0, 0.0, 1.0],
            irradiance: [4.0; 3],
        };
        for depth in [5.5_f32, 5.732, 5.95] {
            let mut scene = SyntheticScene::grid(2).expect("two boxes");
            scene.view = SyntheticView::perspective(
                [0.0, 0.0, 3.0],
                [0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                1.0,
                1.0,
                0.1,
                20.0,
            )
            .expect("camera");
            let receiver_z = 3.0 - depth;
            place_box(
                &mut scene.instances[0],
                [0.0, 0.0, receiver_z],
                [0.5, 0.5, 0.01],
            );
            place_box(
                &mut scene.instances[1],
                [1.0, 0.0, receiver_z + 1.0],
                [0.2; 3],
            );
            let direct = PreparedSynthetic::prepare_with_light(&device, &queue, 41, &scene, light)
                .await
                .expect("direct prepare");
            let direct_target = OffscreenTarget::new(&device, &direct, 129, 129).expect("target");
            let before = render_pixels(&device, &queue, &direct, &direct_target);
            scene.sun_shadows = true;
            let shadowed =
                PreparedSynthetic::prepare_with_light(&device, &queue, 41, &scene, light)
                    .await
                    .expect("shadowed prepare");
            let shadow_target = OffscreenTarget::new(&device, &shadowed, 129, 129).expect("target");
            let after = render_pixels(&device, &queue, &shadowed, &shadow_target);
            let center = (64 * 129 + 64) * 4;
            assert!(before[center] > 80, "direct receiver at depth {depth}");
            assert!(
                u16::from(after[center]) + 25 < u16::from(before[center]),
                "caster must stay visible through cascade blend at depth {depth}"
            );
        }
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
    let mut out = [0.0; 3];
    for (channel, output) in out.iter_mut().enumerate() {
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
        *output = radiance;
    }
    reference_display(out, 1.0)
}

/// Independent literal f64 scene-to-SDR policy, with no shader rearrangement.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the oracle clamps display output and rounds it to [0,255]"
)]
fn reference_display(radiance: [f64; 3], exposure: f64) -> [u8; 4] {
    let exposed = radiance.map(|v| v * exposure);
    let luminance = exposed[0] * 0.2126 + exposed[1] * 0.7152 + exposed[2] * 0.0722;
    let tone = exposed.map(|v| v / (1.0 + luminance));
    let peak = tone.into_iter().fold(1.0, f64::max);
    let rgb = tone.map(|v| {
        let linear = (v / peak).clamp(0.0, 1.0);
        let encoded = if linear <= 0.003_130_8 {
            12.92 * linear
        } else {
            1.055 * linear.powf(1.0 / 2.4) - 0.055
        };
        (encoded * 255.0).round() as u8
    });
    [rgb[0], rgb[1], rgb[2], 255]
}
