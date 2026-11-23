//! Real-Metal submitted-image identity, ordinary presenter and bounded leases.

use super::*;
use crate::{present::SurfaceProfile, runtime::SurfaceOutput, synthetic::CaptureSource};

#[global_allocator]
static ALLOCATOR: thrum_test_support::CountingAllocator = thrum_test_support::CountingAllocator;

#[test]
fn disabled_capture_frame_hook_allocates_nothing_and_has_no_readback_graph_pass() {
    let Some(test_gpu) = thrum_test_support::gpu(
        "disabled_capture_frame_hook_allocates_nothing_and_has_no_readback_graph_pass",
        None,
    ) else {
        return;
    };
    let mut disabled = None;
    let ((), allocations) = thrum_test_support::allocations::count(|| {
        for _ in 0..10000 {
            assert!(
                super::super::submitted_capture::reserve(
                    &mut disabled,
                    1,
                    [65, 37],
                    CaptureSource::Sdr
                )
                .is_none()
            );
        }
    });
    assert_eq!(allocations.count, 0);
    pollster::block_on(async {
        let (host, _) = Box::pin(host_and_output(&test_gpu, wgpu::TextureFormat::Rgba8Unorm)).await;
        let off = host.prepared.graph(&host.slots[0].target, false).unwrap();
        let on = host.prepared.graph(&host.slots[0].target, true).unwrap();
        assert!(
            !off.pass_bindings()
                .any(|(_, pass)| pass.kind == Some(crate::graph::PassKind::Copy))
        );
        assert!(
            on.pass_bindings()
                .any(|(_, pass)| pass.kind == Some(crate::graph::PassKind::Copy))
        );
        assert_eq!(host.submitted_capture_usage(), None);
    });
}

async fn host_and_output(
    test_gpu: &thrum_test_support::Gpu,
    format: wgpu::TextureFormat,
) -> (FrameHost<StaticWorld, StaticScene>, wgpu::Texture) {
    let (capabilities, queue) = device::gpu(test_gpu, 7).await;
    let extent = [65, 37];
    let source = SourceTick {
        epoch: 2,
        tick: 0,
        timeline: 1,
    };
    let host = FrameHost::new(
        &capabilities,
        &queue,
        FrameConfig {
            render_extent: extent,
            surface: Some(SurfaceProfile { format, extent }),
        },
        StaticWorld {
            authority: source,
            publication: source,
        },
        StaticScene::default(),
        SyntheticScene::grid(1).unwrap(),
        1.0,
    )
    .await
    .unwrap();
    let output = output_texture(&host, format);
    (host, output)
}

fn output_texture(
    host: &FrameHost<StaticWorld, StaticScene>,
    format: wgpu::TextureFormat,
) -> wgpu::Texture {
    let [width, height] = host.render_extent;
    host.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("submitted-image reference output"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

fn submit(host: &mut FrameHost<StaticWorld, StaticScene>, output: &wgpu::Texture) -> FrameSample {
    let view = output.create_view(&wgpu::TextureViewDescriptor::default());
    let report = host
        .drive(
            Duration::ZERO,
            Some(&SurfaceOutput {
                view: &view,
                extent: [65, 37],
            }),
        )
        .unwrap();
    assert!(
        report.submitted,
        "ordinary paused-world surface gate must submit"
    );
    assert_eq!(report.tick, 0);
    let mut completed = None;
    thrum_test_support::wait::until(
        "ordinary submitted frame completes",
        Duration::from_secs(5),
        || {
            if host.pending_frames() == 0 {
                return Some(());
            }
            completed = host.poll_completed().unwrap().or(completed.take());
            None
        },
    );
    completed.expect("ordinary submitted frame completed")
}

fn image(host: &mut FrameHost<StaticWorld, StaticScene>) -> super::super::SubmittedImage {
    thrum_test_support::wait::until("submitted image", Duration::from_secs(5), || {
        if let Some(image) = host.try_submitted_image().unwrap() {
            return Some(image);
        }
        drop(host.poll_completed().unwrap());
        None
    })
}

#[test]
fn replacement_visual_applies_latest_headroom_to_first_drawable() {
    let Some(test_gpu) = thrum_test_support::gpu(
        "replacement_visual_applies_latest_headroom_to_first_drawable",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    pollster::block_on(async {
        let (mut host, output) =
            Box::pin(host_and_output(&test_gpu, wgpu::TextureFormat::Rgba16Float)).await;
        let lighting = FrameLighting::Photometric(crate::synthetic::PhotometricEnvironment {
            toward_sun: [0.0, 1.0, 0.0],
            sun_lux: 0.0,
            sun_colour: [1.0; 3],
            ambient_lux: [0.0; 3],
            background_radiance: [500_000.0; 3],
        });
        let mut visual = Box::pin(host.visual_prepare_inputs().prepare(lighting, 1.0, None))
            .await
            .unwrap();
        assert!(host.publish_visual(&mut visual).unwrap());
        host.enable_submitted_capture(1).unwrap();
        host.set_display_headroom(8.0);
        submit(&mut host, &output);
        let high = image(&mut host);
        let high_peak = high.edr_max_rgb().unwrap();
        assert!((4.0..=8.0).contains(&high_peak), "H=8 peak was {high_peak}");
        host.recycle_submitted_image(high);

        let mut visual = Box::pin(host.visual_prepare_inputs().prepare(lighting, 1.0, None))
            .await
            .unwrap();
        host.set_display_headroom(4.0);
        assert!(host.publish_visual(&mut visual).unwrap());
        host.set_display_headroom(4.0);
        submit(&mut host, &output);
        let first = image(&mut host);
        assert_eq!(host.display_headroom(), 4.0);
        let peak = first.edr_max_rgb().unwrap();
        assert!((3.0..=4.0).contains(&peak), "first H=4 peak was {peak}");
    });
}

#[test]
fn default_tap_follows_promotion_and_drains_old_pixels_in_submission_order() {
    use crate::runtime::CaptureLimits;

    let Some(test_gpu) = thrum_test_support::gpu(
        "default_tap_follows_promotion_and_drains_old_pixels_in_submission_order",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    pollster::block_on(async {
        let float_format = wgpu::TextureFormat::Rgba16Float;
        let limits = CaptureLimits::default();
        let sdr_only_budget = CaptureLimits {
            staging_slots: 1,
            cpu_buffers: 1,
            byte_allowance: 32 * 1024,
        };
        for (requested, limits, expected) in [
            (None, limits, Some(CaptureSource::Edr)),
            (Some(CaptureSource::Sdr), limits, Some(CaptureSource::Sdr)),
            (None, sdr_only_budget, None),
        ] {
            let (mut host, sdr_output) =
                Box::pin(host_and_output(&test_gpu, wgpu::TextureFormat::Bgra8Unorm)).await;
            let float_output = output_texture(&host, float_format);
            if let Some(source) = requested {
                host.enable_submitted_capture_from(42, limits, source)
                    .unwrap();
            } else {
                host.enable_submitted_capture_with_limits(42, limits)
                    .unwrap();
            }
            let first = submit(&mut host, &sdr_output);
            let extent = host
                .extent_prepare_inputs_for_format(host.render_extent, float_format)
                .unwrap()
                .build(&std::sync::atomic::AtomicBool::new(false))
                .unwrap()
                .unwrap();
            assert!(host.publish_extent(extent));
            let second = submit(&mut host, &float_output);
            let old = image(&mut host);
            assert_eq!(old.session, 42);
            assert_eq!(old.presenter_generation, 1);
            assert_eq!(old.sample.submission_order, first.submission_order);
            assert_eq!(old.pixel_source, CaptureSource::Sdr);
            assert_eq!(old.rgba.len(), 65 * 37 * 4);
            host.recycle_submitted_image(old);
            if let Some(source) = expected {
                let current = image(&mut host);
                assert_eq!(current.session, 42);
                assert_eq!(current.presenter_generation, 2);
                assert_eq!(current.sample.submission_order, second.submission_order);
                assert_eq!(current.pixel_source, source);
                assert_eq!(
                    current.rgba.len(),
                    (65 * 37 * source.bytes_per_pixel()) as usize
                );
                assert_eq!(host.submitted_capture_omissions(), 0);
            } else {
                assert!(host.try_submitted_image().unwrap().is_none());
                assert_eq!(host.submitted_capture_omissions(), 1);
            }
        }
    });
}

#[test]
fn submitted_tap_matches_the_actual_odd_width_surface_and_preserves_history() {
    let Some(test_gpu) = thrum_test_support::gpu(
        "submitted_tap_matches_the_actual_odd_width_surface_and_preserves_history",
        None,
    ) else {
        return;
    };
    for format in [
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureFormat::Bgra8Unorm,
    ] {
        pollster::block_on(async {
            let (mut host, output) = Box::pin(host_and_output(&test_gpu, format)).await;
            host.enable_submitted_capture(42).unwrap();
            #[cfg(feature = "creation-counters")]
            {
                let report = host.creation_report();
                assert_eq!(report.violations, 0);
                assert!(
                    report
                        .pipelines
                        .iter()
                        .any(|row| row.label == "capture-copy-timing"
                            && row.phase == crate::runtime::CreationPhase::Transition
                            && row.compute > 0)
                );
            }
            host.set_presentation_lead(Duration::from_millis(7));
            submit(&mut host, &output);
            let image = image(&mut host);
            assert!(
                image
                    .copy_gpu_ns
                    .is_some_and(|ns| ns.is_finite() && ns > 0.0)
            );
            assert_eq!(image.device_generation, 7);
            assert_eq!(image.presenter_generation, 1);
            assert_eq!(image.presentation_lead, Duration::from_millis(7));
            assert_eq!(image.rgba.len(), 65 * 37 * 4);
            assert_eq!(image.session, 42);
            assert_eq!(image.source, host.world.source());
            assert_eq!(image.sample.submission_order, 0);
            assert_eq!(image.sample.tick, 0);
            assert!(image.sample.surface_presented);
            assert_eq!(image.history_epoch, host.submitted_view_history_epoch);
            // Independently copy the exact output consumed by the presenter; no
            // second scene render or endpoint capture participates in comparison.
            let mut expected = thrum_test_support::readback::texture(
                &host.device,
                &host.queue,
                &output,
                thrum_test_support::readback::Mip::level(0),
                ..,
            );
            if format == wgpu::TextureFormat::Bgra8Unorm {
                for pixel in expected.as_chunks_mut::<4>().0 {
                    pixel.swap(0, 2);
                }
            }
            assert_eq!(
                image.rgba, expected,
                "tap must match the final present texture byte for byte"
            );
            let epoch = host.submitted_view_history_epoch;
            host.recycle_submitted_image(image);
            host.disable_submitted_capture();
            submit(&mut host, &output);
            assert_eq!(host.submitted_view_history_epoch, epoch);
            assert!(host.submitted_capture.is_none());
            assert_eq!(host.submitted_capture_omissions(), 0);
        });
    }
}

#[test]
fn saturated_cpu_and_gpu_leases_do_not_stop_ordinary_submissions() {
    let Some(test_gpu) = thrum_test_support::gpu(
        "saturated_cpu_and_gpu_leases_do_not_stop_ordinary_submissions",
        None,
    ) else {
        return;
    };
    pollster::block_on(async {
        let (mut host, output) =
            Box::pin(host_and_output(&test_gpu, wgpu::TextureFormat::Rgba8Unorm)).await;
        host.enable_submitted_capture(1).unwrap();
        let mut held = Vec::new();
        for _ in 0..8 {
            submit(&mut host, &output);
            held.push(image(&mut host));
        }
        for _ in 0..7 {
            submit(&mut host, &output);
        }
        assert!(host.try_submitted_image().unwrap().is_none());
        assert_eq!(host.submitted_capture_omissions(), 4);
        assert_eq!(host.next_submission_order, 15);
        assert_eq!(host.submitted_capture_usage(), Some((3, 8)));
        for image in held {
            host.recycle_submitted_image(image);
        }
        for _ in 0..3 {
            let image = image(&mut host);
            host.recycle_submitted_image(image);
        }
        host.disable_submitted_capture();
        host.enable_submitted_capture(2).unwrap();
        submit(&mut host, &output);
        let pending = image(&mut host);
        host.disable_submitted_capture();
        host.enable_submitted_capture(2).unwrap();
        submit(&mut host, &output);
        let fresh = image(&mut host);
        host.recycle_submitted_image(pending);
        assert_eq!(host.submitted_capture_usage(), Some((0, 1)));
        host.recycle_submitted_image(fresh);
        assert_eq!(host.submitted_capture_usage(), Some((0, 0)));
        submit(&mut host, &output);
        assert_eq!(image(&mut host).session, 2);
    });
}

#[test]
fn pending_capture_device_loss_is_rejected_without_polling_the_lost_queue() {
    let Some(test_gpu) = thrum_test_support::gpu(
        "pending_capture_device_loss_is_rejected_without_polling_the_lost_queue",
        None,
    ) else {
        return;
    };
    pollster::block_on(async {
        let (mut host, output) =
            Box::pin(host_and_output(&test_gpu, wgpu::TextureFormat::Rgba8Unorm)).await;
        host.enable_submitted_capture(1).unwrap();
        let view = output.create_view(&wgpu::TextureViewDescriptor::default());
        assert!(
            host.drive(
                Duration::ZERO,
                Some(&SurfaceOutput {
                    view: &view,
                    extent: [65, 37]
                })
            )
            .unwrap()
            .submitted
        );
        host.device.destroy();
        let inputs = host.device_prepare_inputs().unwrap();
        let (capabilities, queue) = device::gpu(&test_gpu.fresh_adapter(), 8).await;
        let mut replacement = inputs
            .prepare(
                &capabilities,
                &queue,
                Some(SurfaceProfile {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    extent: [65, 37],
                }),
            )
            .await
            .unwrap();
        host.publish_device(&mut replacement).unwrap();
        assert!(matches!(
            host.try_submitted_image(),
            Err(super::super::FrameError::InvalidState(
                "retired capture generation"
            ))
        ));
        let output = host.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: 65,
                height: 37,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        // An invalidated observer never prevents the continuing app submitting
        // on its replacement device, even before the caller ends capture.
        submit(&mut host, &output);
        assert_eq!(host.submitted_capture_omissions(), 1);
        host.disable_submitted_capture();
        host.enable_submitted_capture(2).unwrap();
        submit(&mut host, &output);
        let fresh = image(&mut host);
        assert_eq!(fresh.session, 2);
        assert_eq!(fresh.device_generation, 8);
        assert_eq!(fresh.presenter_generation, 2);
    });
}

#[test]
fn abandoned_unsubmitted_capture_releases_its_slot_without_lost_queue_work() {
    let Some(test_gpu) = thrum_test_support::gpu(
        "abandoned_unsubmitted_capture_releases_its_slot_without_lost_queue_work",
        None,
    ) else {
        return;
    };
    pollster::block_on(async {
        let (mut host, output) =
            Box::pin(host_and_output(&test_gpu, wgpu::TextureFormat::Rgba8Unorm)).await;
        host.enable_submitted_capture(1).unwrap();
        let before = super::super::tokens::staging_flush::attempts();
        let encoded = host
            .advance(Duration::ZERO)
            .unwrap()
            .encode_scene()
            .unwrap();
        encoded.abandon_lost_device();
        assert_eq!(super::super::tokens::staging_flush::attempts(), before);
        assert_eq!(host.submitted_capture_usage(), Some((0, 0)));
        assert_eq!(host.submitted_capture_omissions(), 1);
        submit(&mut host, &output);
        assert_eq!(image(&mut host).sample.submission_order, 0);
    });
}

#[test]
fn pending_scene_grid_pixels_keep_their_identity_across_surface_viewport_resize() {
    let Some(test_gpu) = thrum_test_support::gpu(
        "pending_scene_grid_pixels_keep_their_identity_across_surface_viewport_resize",
        None,
    ) else {
        return;
    };
    pollster::block_on(async {
        let (mut host, output) =
            Box::pin(host_and_output(&test_gpu, wgpu::TextureFormat::Rgba8Unorm)).await;
        assert!(
            host.enable_submitted_capture_with_limits(
                1,
                super::super::CaptureLimits {
                    staging_slots: 3,
                    cpu_buffers: 8,
                    byte_allowance: 1,
                }
            )
            .is_err()
        );
        assert_eq!(host.submitted_capture_usage(), None);
        host.enable_submitted_capture(1).unwrap();
        let view = output.create_view(&wgpu::TextureViewDescriptor::default());
        assert!(
            host.drive(
                Duration::ZERO,
                Some(&SurfaceOutput {
                    view: &view,
                    extent: [65, 37]
                })
            )
            .unwrap()
            .submitted
        );
        host.resize_surface([83, 41]).unwrap();
        let old = image(&mut host);
        assert_eq!(old.sample.submission_order, 0);
        assert_eq!(old.presenter_generation, 1);
        assert_eq!(
            old.rgba.len(),
            65 * 37 * 4,
            "scene-grid capture is not relabelled as the resized platform viewport"
        );
        host.recycle_submitted_image(old);
        host.resize_surface([65, 37]).unwrap();
        // The loop had no deadline; the kit's longest wait bounds it.
        thrum_test_support::wait::until(
            "resized frames retired",
            thrum_test_support::readback::WAIT,
            || {
                if host.pending_frames() == 0 {
                    return Some(());
                }
                drop(host.poll_completed().unwrap());
                None
            },
        );
        submit(&mut host, &output);
        let fresh = image(&mut host);
        assert_eq!(fresh.sample.submission_order, 1);
        assert_eq!(fresh.presenter_generation, 3);
        assert_eq!(host.submitted_capture_omissions(), 0);
    });
}

#[test]
fn stopping_admission_drains_in_submission_order_while_rendering_continues() {
    let Some(test_gpu) = thrum_test_support::gpu(
        "stopping_admission_drains_in_submission_order_while_rendering_continues",
        None,
    ) else {
        return;
    };
    pollster::block_on(async {
        let (mut host, output) =
            Box::pin(host_and_output(&test_gpu, wgpu::TextureFormat::Rgba8Unorm)).await;
        host.enable_submitted_capture(71).unwrap();
        let first = submit(&mut host, &output).submission_order;
        submit(&mut host, &output);
        let frame = image(&mut host);
        assert_eq!(frame.sample.submission_order, first);
        assert_eq!(frame.extent, [65, 37]);
        host.recycle_submitted_image(frame);
        submit(&mut host, &output); // Reuse an earlier ring slot while an older image is pending.
        host.stop_submitted_capture();
        submit(&mut host, &output);
        for order in [first + 1, first + 2] {
            let frame = image(&mut host);
            assert_eq!(frame.sample.submission_order, order);
            host.recycle_submitted_image(frame);
        }
        assert!(host.try_submitted_image().unwrap().is_none());
        assert_eq!(host.submitted_capture_usage(), Some((0, 0)));
        assert_eq!(host.submitted_capture_omissions(), 0);
    });
}

#[test]
fn scene_linear_capture_uses_the_ordinary_submission_and_can_restore_sdr() {
    let Some(test_gpu) = thrum_test_support::gpu(
        "scene_linear_capture_uses_the_ordinary_submission_and_can_restore_sdr",
        None,
    ) else {
        return;
    };
    pollster::block_on(async {
        let (mut host, output) =
            Box::pin(host_and_output(&test_gpu, wgpu::TextureFormat::Rgba8Unorm)).await;
        host.enable_submitted_capture(1).unwrap();
        submit(&mut host, &output);
        let baseline = image(&mut host);
        let sdr = baseline.rgba.clone();
        let epoch = baseline.history_epoch;
        host.recycle_submitted_image(baseline);
        host.disable_submitted_capture();
        host.enable_submitted_capture_from(
            2,
            crate::runtime::CaptureLimits::default(),
            CaptureSource::SceneLinear,
        )
        .unwrap();
        submit(&mut host, &output);
        let first = image(&mut host);
        assert_eq!(first.pixel_source, CaptureSource::SceneLinear);
        assert_eq!(first.rgba.len(), 65 * 37 * 8);
        assert_eq!(first.history_epoch, epoch);
        assert!(first.sample.surface_presented);
        let linear = first.rgba.clone();
        let first_exposure = first
            .exposure_gpu_words
            .expect("same-submission exposure metadata");
        host.recycle_submitted_image(first);
        submit(&mut host, &output);
        let repeat = image(&mut host);
        assert_eq!(repeat.rgba, linear);
        let repeat_exposure = repeat
            .exposure_gpu_words
            .expect("same-submission exposure metadata");
        assert_eq!(first_exposure[..4], repeat_exposure[..4]);
        assert_ne!(first_exposure[5..7], repeat_exposure[5..7]);
        host.recycle_submitted_image(repeat);
        host.disable_submitted_capture();
        host.enable_submitted_capture(3).unwrap();
        submit(&mut host, &output);
        let restored = image(&mut host);
        assert_eq!(restored.pixel_source, CaptureSource::Sdr);
        assert_eq!(restored.rgba, sdr);
        assert_eq!(restored.history_epoch, epoch);
        assert_eq!(host.submitted_capture_omissions(), 0);
    });
}

#[test]
fn linear_capture_allowance_is_validated_before_target_replacement() {
    let Some(test_gpu) = thrum_test_support::gpu(
        "linear_capture_allowance_is_validated_before_target_replacement",
        None,
    ) else {
        return;
    };
    pollster::block_on(async {
        let (mut host, _output) =
            Box::pin(host_and_output(&test_gpu, wgpu::TextureFormat::Rgba8Unorm)).await;
        let limits = crate::runtime::CaptureLimits {
            staging_slots: 1,
            cpu_buffers: 1,
            byte_allowance: 30_000,
        };
        assert!(
            host.enable_submitted_capture_from(1, limits, CaptureSource::SceneLinear)
                .is_err()
        );
        assert_eq!(host.slots[0].target.capture_source(), CaptureSource::Sdr);
        assert!(host.submitted_capture.is_none());
        host.enable_submitted_capture_with_limits(2, limits)
            .unwrap();
    });
}
