//! Real Metal completion → qualification observer → platform reply regression.
use super::*;
use crate::{
    app::render_actor::{
        FrameCounters, FrameReadback, RenderOutcome, RenderReply, SurfaceStatus, collect_observer,
    },
    sky::StillWorld,
};
use std::time::Duration;
use thrum_render::{
    capabilities::{DeviceProfile, DeviceRecipe, OpenedDevice},
    extract::{ExtractReport, RenderExtractor, SourceTick},
    present::SurfaceProfile,
    runtime::{DriveReport, FrameConfig, SceneFrame, SurfaceOutput},
    synthetic::{FrameLighting, NativeWitness, SyntheticLight, SyntheticScene, SyntheticView},
};

struct NativeScene;
impl SceneSource<SourceTick> for NativeScene {
    fn extract(&self, into: &mut RenderExtractor, page: &SourceTick) -> ExtractReport {
        into.extract(*page, [0.0; 3], |sink| {
            sink.push(0, 1, 1, [0.0; 3], 0);
        })
    }
    fn frame(
        &self,
        _: SourceTick,
        _: f32,
        _: [u32; 2],
        _: Option<FrameLighting>,
    ) -> Result<SceneFrame, String> {
        Ok(SceneFrame {
            view: SyntheticView::default().with_native_witness(NativeWitness::Metal),
            lighting: FrameLighting::Diagnostic(SyntheticLight::default()),
        })
    }
}

// Construct the same active writer/tap state as THRUM_MAC_QUALIFY + TAP=on,
// without mutating process-global environment in a parallel test harness.
fn observer(host: &mut FrameHost<StillWorld, NativeScene>) -> Observer {
    host.enable_submitted_capture_with_limits(1, CaptureLimits::default())
        .unwrap();
    Observer {
        output: PathBuf::new(),
        extent_markers: None,
        writer: Some(
            ImageWriter::start(std::io::sink(), 4, 1024 * 1024, SpoolEncoding::ZstdDelta).unwrap(),
        ),
        images: Vec::with_capacity(RECORDS),
        frames: Vec::with_capacity(RECORDS),
        refused: 0,
        origin: Instant::now(),
        replace_surface: false,
        replacement_pending: false,
        session: 1,
        retired: 0,
        retirements: Vec::new(),
        stale_image: None,
        stale_recycle: None,
    }
}

fn reply(report: DriveReport, lost: bool) -> RenderReply<()> {
    RenderReply {
        sequence: 1,
        device_generation: 7,
        outcome: if lost {
            RenderOutcome::DeviceLost
        } else {
            RenderOutcome::Frame(Ok(report))
        },
        samples: [None, None],
        target_status: None,
        acquire_wall: None,
        surface_extent: [64, 64],
        platform: FrameReadback {
            profile_revision: 0,
            profile_minimum: 0,
            counters: FrameCounters::default(),
            worker_wall: Duration::ZERO,
            surface_status: SurfaceStatus::Presented,
            transition_error: None,
            readback: (),
        },
    }
}

fn output(device: &wgpu::Device, surface: SurfaceProfile) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("Mac actor reply test output"),
            size: wgpu::Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: surface.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

async fn recover_observer(
    original_gpu: &thrum_test_support::Gpu,
    host: &mut FrameHost<StillWorld, NativeScene>,
    observer: &mut Observer,
    surface: SurfaceProfile,
) {
    let inputs = host.device_prepare_inputs().unwrap();
    let OpenedDevice {
        device,
        queue,
        capabilities,
        watch,
        ..
    } = DeviceRecipe {
        profile: DeviceProfile::Portable,
        generation: 8,
        ..Default::default()
    }
    .open(&original_gpu.instance, None)
    .await
    .unwrap();
    let mut prepared = inputs
        .prepare(&capabilities, &queue, Some(surface))
        .await
        .unwrap();
    host.publish_device(&mut prepared).unwrap();
    // Same handoff as Actor::recover, without any old-device tap polling.
    observer.replace_device(host).unwrap();
    assert_eq!(observer.retired, 1);
    assert_eq!(observer.session, 2);
    let view = output(&device, surface);
    let report = host
        .advance(Duration::ZERO)
        .unwrap()
        .encode_scene()
        .unwrap()
        .submit(Some(&SurfaceOutput {
            view: &view,
            extent: surface.extent,
        }))
        .unwrap();
    observer.frame(host, 2, 2, true).unwrap();
    let mut reply = reply(report, false);
    reply.device_generation = 8;
    thrum_test_support::wait::until(
        "replacement tap did not deliver an image",
        Duration::from_secs(10),
        || {
            if !observer.images.is_empty() {
                return Some(());
            }
            device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(Duration::from_secs(10)),
                })
                .unwrap();
            collect_observer(&mut reply, &watch, host, Some(observer));
            assert!(matches!(reply.outcome, RenderOutcome::Frame(Ok(_))));
            None
        },
    );
    assert_eq!(reply.device_generation, 8);
    assert_eq!(observer.images.len(), 1);
    assert_eq!(observer.images[0].3, 8);
    assert_eq!(host.native_witness_report().unwrap().resets, 1);
    assert_eq!(observer.refused, 0);
    assert!(!watch.is_lost());
    println!(
        "Mac actor observer recovery: generation=8 session=2 platform_reply=Frame(Ok) image_generation=8 retired_failed_frames=1"
    );
}

#[test]
fn metal_observer_preserves_and_discovers_native_loss_in_platform_reply() {
    let Some(gpu) = thrum_test_support::gpu("Mac observer native loss", Some(wgpu::Backend::Metal))
    else {
        return;
    };
    pollster::block_on(async {
        let OpenedDevice {
            device,
            queue,
            capabilities,
            watch,
            ..
        } = DeviceRecipe {
            profile: DeviceProfile::Portable,
            generation: 7,
            ..Default::default()
        }
        .open(&gpu.instance, None)
        .await
        .unwrap();
        let surface = SurfaceProfile {
            format: wgpu::TextureFormat::Rgba8Unorm,
            extent: [64, 64],
        };
        let output = output(&device, surface);
        // Both poll-first (the review's overwrite) and tap-first completion order.
        // The third arm verifies optional rejection still delivers a normal frame.
        for case in ["poll-first", "tap-first", "rejection"] {
            let mut scene = SyntheticScene::grid(1).unwrap();
            scene.view = scene.view.with_native_witness(NativeWitness::Metal);
            let mut host = FrameHost::new(
                &capabilities,
                &queue,
                FrameConfig {
                    render_extent: surface.extent,
                    surface: Some(surface),
                },
                StillWorld::default(),
                NativeScene,
                scene,
                1.0,
            )
            .await
            .unwrap();
            let mut observer = observer(&mut host);
            if case == "rejection" {
                assert!(host.reject_native_provider());
            } else {
                host.inject_native_completion_error();
            }
            let report = host
                .advance(Duration::ZERO)
                .unwrap()
                .encode_scene()
                .unwrap()
                .submit(Some(&SurfaceOutput {
                    view: &output,
                    extent: surface.extent,
                }))
                .unwrap();
            assert!(report.submitted);
            observer.frame(&host, 1, 1, true).unwrap();
            device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(Duration::from_secs(10)),
                })
                .unwrap();
            let poll_first = case == "poll-first";
            if poll_first {
                assert!(matches!(
                    host.poll_completed(),
                    Err(FrameError::NativeCommandBuffer { generation: 7 })
                ));
            }
            let mut reply = reply(report, poll_first);
            assert!(!watch.is_lost());
            // This is the production Actor::work epilogue, including the real
            // active observer and renderer tap, not just frame_failure mapping.
            collect_observer(&mut reply, &watch, &mut host, Some(&mut observer));
            assert_eq!(reply.device_generation, 7);
            assert!(!watch.is_lost());
            if case == "rejection" {
                assert!(matches!(reply.outcome, RenderOutcome::Frame(Ok(_))));
                assert_eq!(
                    host.native_witness_report().unwrap().selection,
                    NativeWitness::Wgsl
                );
            } else {
                assert!(matches!(reply.outcome, RenderOutcome::DeviceLost), "{case}");
                assert!(
                    observer.images.is_empty(),
                    "failed pixels cannot enter the writer"
                );
                assert!(matches!(
                    host.try_submitted_image(),
                    Err(FrameError::NativeCommandBuffer { generation: 7 })
                ));
                // Replacement can also be the first collector; it must preserve
                // the typed error rather than wrap it as a game/scene failure.
                assert!(matches!(
                    observer.replace(&mut host),
                    Err(FrameError::NativeCommandBuffer { generation: 7 })
                ));
                recover_observer(&gpu, &mut host, &mut observer, surface).await;
            }
            observer
                .writer
                .take()
                .unwrap()
                .finish(Duration::from_secs(10))
                .unwrap();
            println!(
                "Mac actor observer {case}: wgpu_lost=false generation=7 platform_reply={}",
                if case == "rejection" {
                    "Frame(Ok), prepared WGSL twin"
                } else {
                    "DeviceLost, no failed images"
                }
            );
        }
    });
}
