//! Opt-in real-device coverage for the fixed-clock simulation-to-GPU host.
#![cfg(not(target_arch = "wasm32"))]

use std::time::Duration;

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
        let mut host = FrameHost::new(&device, &queue)
            .await
            .expect("prepared 10k scene");
        assert!(host.drive(Duration::ZERO).expect("idle callback").is_none());
        assert_eq!(host.tick(), 0);
        let first = host
            .drive(Duration::from_nanos(16_666_667))
            .expect("first due callback")
            .expect("one committed frame");
        assert_eq!(
            (first.tick, first.completed_ticks, first.queued_ticks),
            (1, 1, 0)
        );
        assert_eq!(first.uploaded_rows, 1_500);
        assert_eq!(first.dropped_presentation, 0);
        let catch_up = host
            .drive(Duration::from_nanos(50_000_001))
            .expect("bounded catch-up")
            .expect("latest committed frame");
        assert_eq!(
            (
                catch_up.tick,
                catch_up.completed_ticks,
                catch_up.queued_ticks
            ),
            (3, 2, 0)
        );
        assert_eq!(host.tick(), 3);
        assert_eq!(
            host.capture_pixels().expect("visible capture").len(),
            640 * 360 * 4
        );
        let error = host
            .drive(Duration::from_millis(200))
            .expect_err("nine due updates exceed eight queued slots");
        assert!(error.contains("fixed-clock overload: 9 ticks refused"));
        assert_eq!(host.tick(), 3, "overload cannot skip or repeat a tick");
        assert!(host.next_due_in().is_none());
    });
}
