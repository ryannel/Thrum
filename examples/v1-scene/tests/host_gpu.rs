//! Opt-in real-device coverage for the fixed-clock simulation-to-GPU host.
#![cfg(not(target_arch = "wasm32"))]

use std::time::{Duration, Instant};

use thrum_render::timing::GpuTimings;
use v1_scene::FrameHost;

#[test]
#[ignore = "requires a real native GPU adapter; run explicitly for device evidence"]
fn committed_ticks_drive_latest_gpu_frame_and_overload_is_explicit() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("native adapter required for explicit GPU test");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: GpuTimings::requested_features(&adapter),
                ..Default::default()
            })
            .await
            .expect("device request succeeds");
        let mut host = FrameHost::new(&device, &queue, None)
            .await
            .expect("prepared 10k scene");
        let idle = host.drive(Duration::ZERO, None).expect("idle callback");
        assert_eq!(idle.completed_ticks, 0);
        assert!(!idle.submitted);
        assert_eq!(host.tick(), 0);
        let first = host
            .drive(Duration::from_nanos(16_666_667), None)
            .expect("first due callback");
        assert_eq!(
            (
                first.tick,
                first.completed_ticks,
                first.queued_ticks,
                first.submitted
            ),
            (1, 1, 0, true)
        );
        let catch_up = host
            .drive(Duration::from_nanos(50_000_001), None)
            .expect("bounded catch-up");
        assert_eq!(
            (
                catch_up.tick,
                catch_up.completed_ticks,
                catch_up.queued_ticks,
                catch_up.submitted,
            ),
            (3, 2, 0, true)
        );
        assert_eq!(host.tick(), 3);
        let deadline = Instant::now() + Duration::from_secs(10);
        for expected_tick in [1, 3] {
            let sample = loop {
                if let Some(sample) = host.poll_completed().expect("GPU progress") {
                    break sample;
                }
                assert!(Instant::now() < deadline, "GPU completion timed out");
                device
                    .poll(wgpu::PollType::Wait {
                        submission_index: None,
                        timeout: Some(Duration::from_millis(100)),
                    })
                    .expect("GPU wait");
            };
            assert_eq!(sample.tick, expected_tick);
            assert_eq!(sample.uploaded_rows, 1_500);
            assert_eq!(sample.dropped_presentation, 0);
            assert!(!sample.timing_fault);
            assert!(sample.gpu_pass_ns.is_some());
        }
        assert_eq!(host.pending_frames(), 0);
        assert_eq!(host.dropped_views(), 0);
        assert_eq!(
            host.capture_pixels().expect("visible capture").len(),
            640 * 360 * 4
        );
        let error = host
            .drive(Duration::from_millis(200), None)
            .expect_err("nine due updates exceed eight queued slots");
        assert!(error.contains("fixed-clock overload: 9 ticks refused"));
        assert_eq!(host.tick(), 3, "overload cannot skip or repeat a tick");
        assert!(host.next_due_in().is_none());

        let (plain_device, plain_queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("device without timestamp queries");
        let mut plain = FrameHost::new(&plain_device, &plain_queue, None)
            .await
            .expect("same prepared scene without timestamps");
        assert!(
            plain
                .drive(Duration::from_nanos(16_666_667), None)
                .expect("plain first tick")
                .submitted
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        let plain_sample = loop {
            if let Some(sample) = plain.poll_completed().expect("plain GPU progress") {
                break sample;
            }
            assert!(Instant::now() < deadline, "plain GPU completion timed out");
            plain_device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(Duration::from_millis(100)),
                })
                .expect("plain GPU wait");
        };
        assert_eq!(plain_sample.tick, 1);
        assert_eq!(plain_sample.gpu_pass_ns, None);
        assert!(!plain_sample.timing_fault);
        assert_eq!(plain.pending_frames(), 0);
    });
}
