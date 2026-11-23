use super::*;
use crate::{
    error::RenderError,
    present::{PresentError, SurfacePresenter},
    runtime::{FrameTarget, SpriteSceneRequests, SurfaceFault, SurfaceOutput},
    sprite::{EmbeddedSheet, SpriteRect, SpriteScene},
    timing::{GpuTimings, TimingError},
};

async fn sprite_host(capabilities: &DeviceCapabilities, queue: &wgpu::Queue) -> Host {
    let source = SourceTick {
        epoch: 5,
        tick: 0,
        timeline: 3,
    };
    FrameHost::new_2d(
        capabilities,
        queue,
        FrameConfig {
            render_extent: surface().extent,
            surface: Some(surface()),
        },
        StaticWorld {
            authority: source,
            publication: source,
        },
        Policy::default(),
        SpriteScene::new(
            EmbeddedSheet {
                extent: [1, 1],
                rgba: Arc::from([255; 4]),
                frames: Arc::from([SpriteRect {
                    origin: [0; 2],
                    size: [1; 2],
                }]),
                texels_per_unit: 1.0,
            },
            1,
        ),
        SpriteSceneRequests::default(),
    )
    .await
    .expect("sprite host")
}

#[test]
fn surface_encode_faults_reach_both_routes_and_completed_samples() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let Some(test_gpu) = thrum_test_support::gpu_on(
            "surface_encode_faults_reach_both_routes_and_completed_samples",
            &instance,
        ) else {
            return;
        };
        let (capabilities, queue) = gpu(&test_gpu, 7).await;
        let device = capabilities.device();
        let profile = surface();
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("late surface encode fault drawable"),
            size: wgpu::Extent3d {
                width: profile.extent[0],
                height: profile.extent[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: profile.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        for (sprites, late) in [(false, false), (false, true), (true, false), (true, true)] {
            for fault in [
                SurfaceFault::StaleBinding,
                SurfaceFault::ExtentMismatch,
                SurfaceFault::Timing(TimingError::Capacity),
            ] {
                let mut host = if sprites {
                    sprite_host(&capabilities, &queue).await
                } else {
                    host(&capabilities, &queue, Policy::default()).await
                };
                let extent = match fault {
                    SurfaceFault::StaleBinding => {
                        let old_presenter =
                            SurfacePresenter::prepare(&capabilities, profile.extent, profile)
                                .await
                                .unwrap();
                        let binding = match &host.slots[0].target {
                            FrameTarget::TwoD(target) => old_presenter.bind_output(
                                device,
                                capabilities.generation(),
                                target.output(),
                            ),
                            FrameTarget::ThreeD(target) => {
                                old_presenter.bind_target(device, target)
                            }
                        };
                        host.slots[0].presentation = Some(binding.expect("retired binding"));
                        profile.extent
                    }
                    SurfaceFault::ExtentMismatch => [profile.extent[0] / 2, profile.extent[1]],
                    SurfaceFault::Timing(TimingError::Capacity) => {
                        // Exactly enough queries for the recorded scene. The
                        // late presentation pass must make its own reservation.
                        let passes = host
                            .scene_graph()
                            .unwrap()
                            .pipeline_pass_count_before_acquire();
                        let queries =
                            GpuTimings::new(&capabilities, &queue, u32::try_from(passes).unwrap())
                                .unwrap();
                        assert!(queries.is_enabled(), "Metal timestamp queries required");
                        host.slots[0].timings = queries;
                        profile.extent
                    }
                    _ => unreachable!(),
                };
                let output = SurfaceOutput {
                    view: &view,
                    extent,
                };
                let outcome = if late {
                    host.advance(Duration::ZERO)
                        .unwrap()
                        .encode_scene()
                        .unwrap()
                        .submit(Some(&output))
                } else {
                    host.drive(Duration::ZERO, Some(&output))
                };
                let report = match fault {
                    SurfaceFault::Timing(TimingError::Capacity) => match outcome {
                        Err(FrameError::SurfaceEncode {
                            report,
                            error: RenderError::Present(PresentError::Timing(TimingError::Capacity)),
                        }) => *report,
                        other => panic!("the caller must get timing capacity, got {other:?}"),
                    },
                    _ => outcome.expect("a stale drawable is recoverable"),
                };
                assert!(report.submitted && report.dropped_view && !report.surface_presented);
                assert_eq!(report.surface_fault, Some(fault));
                assert_eq!(host.dropped_surface(), 1);
                let sample = (0..2)
                    .find_map(|_| {
                        device
                            .poll(wgpu::PollType::Wait {
                                submission_index: None,
                                timeout: Some(Duration::from_secs(10)),
                            })
                            .unwrap();
                        host.poll_completed().unwrap()
                    })
                    .expect("scene completion after render and optional timing resolve");
                assert!(!sample.surface_presented);
                assert_eq!(sample.surface_fault, Some(fault));
            }
        }
    });
}
