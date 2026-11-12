//! The public game host replaces its GPU while keeping one resident world.

use std::time::{Duration, Instant};

use thrum_core::sim::WorldLifecycle;
use thrum_render::{
    capabilities::DeviceCapabilities, present::SurfaceProfile, synthetic::DisplayOverlay,
};

use super::{
    super::{FrameConfig, FrameHost, ThreadedFrameHost},
    gpu_support::{draw, retain, transition},
};

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    capabilities: DeviceCapabilities,
    target: wgpu::Texture,
}

impl Gpu {
    async fn new(instance: &wgpu::Instance, generation: u64) -> Self {
        // A replacement requests a fresh Adapter, as required by wgpu's API.
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("real GPU required for device recovery");
        #[cfg(target_os = "macos")]
        assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: DeviceCapabilities::requested_timing_features(&adapter),
                ..Default::default()
            })
            .await
            .expect("new rendering device");
        let capabilities = DeviceCapabilities::capture(&adapter, &device, generation);
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("recovered presentation proxy, not a drawable"),
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
        Self {
            device,
            queue,
            capabilities,
            target,
        }
    }
}

fn surface() -> SurfaceProfile {
    SurfaceProfile {
        format: wgpu::TextureFormat::Bgra8Unorm,
        extent: [640, 360],
    }
}

fn controlled_x(host: &mut ThreadedFrameHost) -> f64 {
    // Observe the retained published authority; do not add a production probe.
    let page = host
        .inner
        .inner
        .world_mut()
        .published_page()
        .expect("world publication");
    let slot = u32::try_from(crate::scene::ENTITY_COUNT - 1).expect("controlled slot");
    let handle = page.handle(slot).expect("controlled actor handle");
    page.position(handle).expect("controlled actor position").0[0]
}

#[test]
fn destroyed_gpu_replacement_keeps_paused_world_camera_sky_and_held_input() {
    prove_paused_recovery(0.76, false, DisplayOverlay::Off);
    prove_paused_recovery(0.0, true, DisplayOverlay::Off);
}

#[test]
fn destroyed_gpu_replacement_keeps_the_exposure_zones_overlay() {
    prove_paused_recovery(0.76, false, DisplayOverlay::ExposureZones);
}

fn prove_paused_recovery(phase: f32, midnight: bool, overlay: DisplayOverlay) {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let old = Gpu::new(&instance, 1).await;
        let mut host = FrameHost::new_with_capabilities(
            &old.device,
            &old.queue,
            &old.capabilities,
            FrameConfig {
                surface: Some(surface()),
                ..FrameConfig::default()
            },
        )
        .await
        .expect("10k game scene");
        let camera = crate::SceneCamera::perspective(
            [0.0, 0.2, 2.0],
            [0.0, 0.1, 0.0],
            46.0_f32.to_radians(),
            0.1,
            30_000.0,
        )
        .expect("authored camera");
        host.set_scene_camera(camera).expect("no route installed");
        let orientation = host.camera_orientation();
        assert!((orientation[0] - (-0.1_f32).atan2(2.0)).abs() < 1e-6);
        assert_eq!(orientation[1], 0.0);
        let origin = Instant::now();
        let (mut host, controls) = host.into_threaded(origin, || {}).expect("resident world");
        let paused = transition(&mut host, &controls, &old.device, WorldLifecycle::Pause);
        let (before, _) = draw(&mut host, origin, &old.device, &old.queue, &old.target);
        let inputs = host.device_prepare_inputs().expect("retained CPU inputs");
        let replacement = Gpu::new(&instance, 2).await;
        let mut candidate = inputs
            .prepare(
                &replacement.capabilities,
                &replacement.queue,
                Some(surface()),
            )
            .await
            .expect("detached replacement");
        // Detached preparation must not freeze the old presentation policy.
        host.set_camera_yaw(0.13)
            .expect("camera during preparation");
        host.set_time_of_day(phase)
            .expect("solar edit during preparation");
        host.set_display_overlay(overlay);
        let (expected, prior) = draw(&mut host, origin, &old.device, &old.queue, &old.target);
        assert_ne!(before, expected);
        if overlay == DisplayOverlay::ExposureZones {
            // Zones are 13 undithered palette codes, so a zoned frame has few colours.
            let colours = expected
                .as_chunks::<4>()
                .0
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len();
            assert!(
                (2..=13).contains(&colours),
                "a zoned frame must use only palette codes, found {colours} colours"
            );
        }
        if midnight {
            // The fixture's local lights and grid can fill the left of the
            // image. The upper right looks above the geometry into the sky.
            let mut sky = expected
                .as_chunks::<{ 640 * 4 }>()
                .0
                .iter()
                .take(32)
                .flat_map(|row| row[560 * 4..].as_chunks::<4>().0)
                .flat_map(|pixel| &pixel[..3]);
            assert!(
                sky.all(|channel| (40..90).contains(channel)),
                "the upper-right patch must be dim, nonblack midnight sky"
            );
        }
        let before_x = controlled_x(&mut host);
        let view = old
            .target
            .create_view(&wgpu::TextureViewDescriptor::default());
        assert!(
            host.drive(
                origin.elapsed(),
                Some(&super::super::SurfaceOutput {
                    view: &view,
                    extent: [640, 360],
                }),
            )
            .expect("submit before device loss")
            .submitted
        );
        let pending = host.pending_frames();
        let dropped = host.dropped_reports();
        assert!(pending > 0, "replace a host with unretired GPU work");
        old.device.destroy();
        controls
            .set_controls([1.0, 0.0, 0.0])
            .expect("controls remain usable without rendering");
        // No call into the destroyed device is required to install its successor.
        host.publish_device(&mut candidate)
            .expect("replace only GPU resources");
        assert_eq!(host.inner.camera_orientation(), [orientation[0], 0.13]);
        assert_eq!(host.pending_frames(), 0);
        assert!(host.dropped_reports() >= dropped + u64::try_from(pending).expect("bounded slots"));
        assert_eq!(
            controls.check_health().expect("same paused owner"),
            paused.id.tick
        );
        let (restored, sample) = draw(
            &mut host,
            origin,
            &replacement.device,
            &replacement.queue,
            &replacement.target,
        );
        assert_eq!(sample.tick, paused.id.tick);
        assert_eq!(sample.presentation_alpha, 1.0);
        assert!(sample.submission_order > prior.submission_order);
        assert_eq!(sample.visual_revision, prior.visual_revision);
        assert_eq!(
            sample.uploaded_rows, 0,
            "installation already uploaded the unchanged paused scene"
        );
        let (consecutive, stable_sample) = draw(
            &mut host,
            origin,
            &replacement.device,
            &replacement.queue,
            &replacement.target,
        );
        assert_eq!(stable_sample.tick, paused.id.tick);
        assert_eq!(
            consecutive, restored,
            "consecutive replacement frames are identical"
        );
        let peak = expected
            .iter()
            .zip(&restored)
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .expect("pixels");
        assert!(
            peak <= 1,
            "replacement changed fixed-view pixels by {peak} SDR codes"
        );
        if overlay == DisplayOverlay::ExposureZones {
            assert_eq!(
                peak, 0,
                "the replacement device must redraw the same zone codes"
            );
        }
        eprintln!(
            "recovery paused: peak_sdr_codes={peak}, tick={}, uploaded_rows={}, abandoned_reports={}",
            sample.tick,
            sample.uploaded_rows,
            host.dropped_reports() - dropped
        );
        let resumed = transition(
            &mut host,
            &controls,
            &replacement.device,
            WorldLifecycle::Resume,
        );
        assert_eq!(resumed.id, paused.id);
        assert_eq!(resumed.timeline, paused.timeline);
        let deadline = Instant::now() + Duration::from_secs(10);
        while controls.check_health().expect("resumed owner") == paused.id.tick {
            assert!(Instant::now() < deadline, "world did not resume");
            std::thread::sleep(Duration::from_millis(1));
        }
        let next = transition(
            &mut host,
            &controls,
            &replacement.device,
            WorldLifecycle::Pause,
        );
        assert_eq!(next.id.epoch, paused.id.epoch);
        assert_eq!(next.timeline, paused.timeline);
        assert!(next.id.tick > paused.id.tick);
        let (moved, _) = draw(
            &mut host,
            origin,
            &replacement.device,
            &replacement.queue,
            &replacement.target,
        );
        assert_ne!(moved, restored, "moving entities resume in the same world");
        assert!(
            controlled_x(&mut host) > before_x,
            "held input reached the controlled actor after GPU replacement"
        );
        let label = match (midnight, overlay) {
            (_, DisplayOverlay::TemporalInputs) => "recovery-temporal",
            (_, DisplayOverlay::ExposureZones) => "recovery-zones",
            (_, DisplayOverlay::Motion) => "recovery-motion",
            (true, DisplayOverlay::Off) => "recovery-midnight",
            (false, DisplayOverlay::Off) => "recovery",
        };
        retain(&format!("{label}-before"), &expected);
        retain(&format!("{label}-after"), &restored);
        retain(&format!("{label}-resumed"), &moved);
    });
}

#[test]
fn running_world_advances_while_its_destroyed_gpu_is_replaced() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let old = Gpu::new(&instance, 11).await;
        let host = FrameHost::new_with_capabilities(
            &old.device,
            &old.queue,
            &old.capabilities,
            FrameConfig {
                surface: Some(surface()),
                ..FrameConfig::default()
            },
        )
        .await
        .expect("platform generation is not fixed at one");
        let origin = Instant::now();
        let (mut host, controls) = host.into_threaded(origin, || {}).expect("resident world");
        let baseline = transition(&mut host, &controls, &old.device, WorldLifecycle::Pause);
        transition(&mut host, &controls, &old.device, WorldLifecycle::Resume);
        let inputs = host.device_prepare_inputs().expect("CPU recovery inputs");
        old.device.destroy();
        controls
            .set_controls([0.5, 0.0, 0.0])
            .expect("input while GPU unavailable");
        let deadline = Instant::now() + Duration::from_secs(10);
        while controls.check_health().expect("world independent of GPU") == baseline.id.tick {
            assert!(Instant::now() < deadline, "GPU loss blocked the world");
            std::thread::sleep(Duration::from_millis(1));
        }
        let replacement = Gpu::new(&instance, 12).await;
        let mut candidate = inputs
            .prepare(
                &replacement.capabilities,
                &replacement.queue,
                Some(surface()),
            )
            .await
            .expect("prepare successor after destruction");
        host.publish_device(&mut candidate)
            .expect("retain running owner");
        let paused = transition(
            &mut host,
            &controls,
            &replacement.device,
            WorldLifecycle::Pause,
        );
        assert_eq!(paused.id.epoch, baseline.id.epoch);
        assert_eq!(paused.timeline, baseline.timeline);
        assert!(paused.id.tick > baseline.id.tick);
        let (pixels, sample) = draw(
            &mut host,
            origin,
            &replacement.device,
            &replacement.queue,
            &replacement.target,
        );
        assert_eq!(sample.tick, paused.id.tick);
        eprintln!(
            "recovery running: tick_before={}, tick_after={}, epoch_unchanged=true, uploaded_rows={}",
            baseline.id.tick, paused.id.tick, sample.uploaded_rows
        );
        retain("recovery-running", &pixels);
    });
}

/// The mixed field keeps its authored meshes through device loss: the
/// replacement prepares the same per-slot selection from the retained scene
/// and redraws the paused frame unchanged, then the world resumes on it.
#[test]
fn mixed_mesh_field_keeps_its_meshes_through_gpu_replacement() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let old = Gpu::new(&instance, 21).await;
        let host = FrameHost::new_with_setup(
            &old.device,
            &old.queue,
            &old.capabilities,
            FrameConfig {
                surface: Some(surface()),
                ..FrameConfig::default()
            },
            crate::GameSetup {
                display_overlay: DisplayOverlay::Off,
                diagnostic_clouds: false,
                content: crate::GameContent::Field,
                camera_culling: thrum_render::synthetic::CullMode::Frustum,
                valley_stars: false,
                route_overlay: crate::RouteOverlay::Off,
                field_meshes: true,
            },
        )
        .await
        .expect("mixed-mesh field");
        let origin = Instant::now();
        let (mut host, controls) = host.into_threaded(origin, || {}).expect("resident world");
        // Let the flythrough run before pausing on a fixed frame.
        let deadline = Instant::now() + Duration::from_secs(10);
        while controls.check_health().expect("running world") < 30 {
            assert!(Instant::now() < deadline, "world did not run");
            std::thread::sleep(Duration::from_millis(1));
        }
        let paused = transition(&mut host, &controls, &old.device, WorldLifecycle::Pause);
        let (before, _) = draw(&mut host, origin, &old.device, &old.queue, &old.target);
        let inputs = host.device_prepare_inputs().expect("retained CPU inputs");
        old.device.destroy();
        let replacement = Gpu::new(&instance, 22).await;
        let mut candidate = inputs
            .prepare(
                &replacement.capabilities,
                &replacement.queue,
                Some(surface()),
            )
            .await
            .expect("replacement after destruction");
        host.publish_device(&mut candidate)
            .expect("replace only GPU resources");
        let (restored, sample) = draw(
            &mut host,
            origin,
            &replacement.device,
            &replacement.queue,
            &replacement.target,
        );
        assert_eq!(sample.tick, paused.id.tick);
        let peak = before
            .iter()
            .zip(&restored)
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .expect("pixels");
        let differing = before
            .as_chunks::<4>()
            .0
            .iter()
            .zip(restored.as_chunks::<4>().0)
            .filter(|(a, b)| a != b)
            .count();
        eprintln!(
            "mixed field recovery: tick={}, peak_sdr_codes={peak}, differing_px={differing}",
            sample.tick
        );
        assert!(
            peak <= 1,
            "the replacement redraws the same meshes: {peak} codes over {differing} px"
        );
        transition(
            &mut host,
            &controls,
            &replacement.device,
            WorldLifecycle::Resume,
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        while controls.check_health().expect("resumed owner") == paused.id.tick {
            assert!(Instant::now() < deadline, "world did not resume");
            std::thread::sleep(Duration::from_millis(1));
        }
        let (moved, _) = draw(
            &mut host,
            origin,
            &replacement.device,
            &replacement.queue,
            &replacement.target,
        );
        assert_ne!(moved, restored, "the flythrough resumes on the replacement");
        retain("mixed-field-recovery-before", &before);
        retain("mixed-field-recovery-after", &restored);
        retain("mixed-field-recovery-resumed", &moved);
    });
}
