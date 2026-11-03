//! Opt-in real-device synthetic renderer integration test.
#![cfg(not(target_arch = "wasm32"))]

use thrum_render::synthetic::{OffscreenTarget, PreparedSynthetic, SyntheticScene};
use thrum_render::timing::GpuTimings;

/// Opt-in integration test: a real adapter is required, never replaced by a mock.
#[test]
#[ignore = "requires a real native GPU adapter; run explicitly for device evidence"]
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
        let scene = SyntheticScene::grid(1).unwrap();
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
            let mut encoder = device.create_command_encoder(&Default::default());
            assert!(prepared.encode(&mut encoder, 42, &target, None).is_err());
            prepared
                .encode(&mut encoder, 41, &target, Some(&mut timings))
                .unwrap();
            pixels.encode(&mut encoder, &target).unwrap();
            assert!(pixels.encode(&mut encoder, &target).is_err());
            timings.finish_render(41, &encoder).unwrap();
            // The render encoder has not been submitted; collection must return
            // immediately without recording a same-submission resolve.
            let mut early = device.create_command_encoder(&Default::default());
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
            for (actual, expected) in rgba[center..center + 4].iter().zip([204_u8, 51, 26, 255]) {
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
                let mut resolve = device.create_command_encoder(&Default::default());
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
        let mut encoder = device.create_command_encoder(&Default::default());
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
        for (x, expected) in [(18, [204_u8, 51, 26, 255]), (47, [26_u8, 128, 230, 255])] {
            let offset = (24 * 65 + x) * 4;
            for (actual, expected) in rgba[offset..offset + 4].iter().zip(expected) {
                assert!(
                    actual.abs_diff(expected) <= 1,
                    "indexed pixel {x} got {actual}, expected {expected}"
                );
            }
        }
        // 10k ABI/table/instance draw on the same executor, prepared before encode.
        let large =
            PreparedSynthetic::prepare(&device, &queue, 41, &SyntheticScene::grid(10_000).unwrap())
                .await
                .unwrap();
        let mut encoder = device.create_command_encoder(&Default::default());
        large.encode(&mut encoder, 41, &target, None).unwrap();
        let submission = queue.submit([encoder.finish()]);
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(std::time::Duration::from_secs(10)),
            })
            .unwrap();
        // Feature not enabled on a device means successful unsupported behavior,
        // even when the physical adapter does support timestamps.
        let (plain_device, plain_queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .unwrap();
        let mut disabled = GpuTimings::new(&plain_device, &plain_queue, 42, 1).unwrap();
        assert!(!disabled.is_enabled());
        assert!(disabled.pass_writes(42, "unsupported").unwrap().is_none());
        let mut encoder = plain_device.create_command_encoder(&Default::default());
        disabled.finish_render(42, &encoder).unwrap();
        assert!(!disabled.resolve_if_ready(42, &mut encoder).unwrap());
        assert!(disabled.try_collect(42).unwrap().is_none());
        eprintln!(
            "adapter={} backend={:?} driver={} timestamps={enabled}; 1/2-instance exact pixel checks and 10,000-instance submission passed",
            info.name, info.backend, info.driver
        );
    });
}
