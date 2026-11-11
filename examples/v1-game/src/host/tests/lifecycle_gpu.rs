//! The real game keeps rendering/camera control while its resident world pauses.

use std::time::{Duration, Instant};

use thrum_core::sim::WorldLifecycle;

use super::super::{FrameConfig, FrameHost};
use super::gpu_support::{draw, retain, transition};
use thrum_render::present::SurfaceProfile;

#[test]
fn threaded_pause_keeps_pixels_still_camera_live_and_resume_advances_the_same_world() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("real GPU required for threaded pause rendering");
        #[cfg(target_os = "macos")]
        assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("game rendering device");
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("offscreen presentation proxy, not a drawable"),
            size: wgpu::Extent3d {
                width: 640,
                height: 360,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let host = FrameHost::new(
            &device,
            &queue,
            FrameConfig {
                surface: Some(SurfaceProfile {
                    format: wgpu::TextureFormat::Bgra8Unorm,
                    extent: [640, 360],
                }),
                ..FrameConfig::default()
            },
        )
        .await
        .expect("10k game scene");
        let origin = Instant::now();
        let (mut host, controls) = host.into_threaded(origin, || {}).expect("resident world");
        let paused = transition(&mut host, &controls, &device, WorldLifecycle::Pause);
        let (before, sample) = draw(&mut host, origin, &device, &queue, &target);
        assert_eq!(sample.tick, paused.id.tick);
        assert_eq!(sample.presentation_alpha, 1.0);
        std::thread::sleep(Duration::from_millis(40));
        // M1 advances jitter even while the world is paused. Compare the same
        // phase after a complete cycle, asserting every intervening world tick.
        for _ in 0..31 {
            let (_, sample) = draw(&mut host, origin, &device, &queue, &target);
            assert_eq!(sample.tick, paused.id.tick);
            assert_eq!(sample.completed_ticks, 0);
        }
        let (still, sample) = draw(&mut host, origin, &device, &queue, &target);
        assert_eq!(sample.tick, paused.id.tick);
        assert_eq!(sample.completed_ticks, 0);
        assert_eq!(sample.presentation_alpha, 1.0);
        assert_eq!(
            still, before,
            "paused world is stable across submitted frames"
        );
        host.set_camera_yaw(0.1).expect("camera while paused");
        let (camera, sample) = draw(&mut host, origin, &device, &queue, &target);
        assert_eq!(sample.tick, paused.id.tick);
        assert_eq!(sample.presentation_alpha, 1.0);
        assert_ne!(camera, still, "presentation remains interactive");
        host.set_camera_yaw(0.0).expect("restore fixed camera");
        controls
            .set_controls([1.0, 0.0, 0.0])
            .expect("held input while paused");
        let resumed = transition(&mut host, &controls, &device, WorldLifecycle::Resume);
        assert_eq!(resumed.id, paused.id, "resume excludes the hidden interval");
        assert_eq!(resumed.timeline, paused.timeline);
        let deadline = Instant::now() + Duration::from_secs(10);
        while controls.check_health().expect("world progresses") == paused.id.tick {
            assert!(Instant::now() < deadline, "resumed world did not advance");
            std::thread::sleep(Duration::from_millis(1));
        }
        let next = transition(&mut host, &controls, &device, WorldLifecycle::Pause);
        assert_eq!(next.id.epoch, paused.id.epoch);
        assert_eq!(next.timeline, paused.timeline);
        assert!(next.id.tick > paused.id.tick);
        let (after, sample) = draw(&mut host, origin, &device, &queue, &target);
        assert_eq!(sample.tick, next.id.tick);
        assert_ne!(after, before, "the same game resumes its moving entities");
        retain("paused", &before);
        retain("camera-paused", &camera);
        retain("resumed", &after);
    });
}
