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
                    CaptureSource::Sdr,
                    true
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
    output_at_extent(host, format, host.render_extent)
}

fn output_at_extent(
    host: &FrameHost<StaticWorld, StaticScene>,
    format: wgpu::TextureFormat,
    [width, height]: [u32; 2],
) -> wgpu::Texture {
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
                extent: [output.width(), output.height()],
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
fn submitted_copy_timing_follows_each_frames_demand_in_sdr_and_edr() {
    let Some(test_gpu) =
        thrum_test_support::gpu("submitted copy timing demand", Some(wgpu::Backend::Metal))
    else {
        return;
    };
    pollster::block_on(async {
        for (format, source) in [
            (wgpu::TextureFormat::Bgra8Unorm, CaptureSource::Sdr),
            (wgpu::TextureFormat::Rgba16Float, CaptureSource::Edr),
        ] {
            let (mut host, output) = Box::pin(host_and_output(&test_gpu, format)).await;
            host.set_timing_requested(false);
            host.enable_submitted_capture(1).unwrap();
            let view = output.create_view(&wgpu::TextureViewDescriptor::default());
            let surface = SurfaceOutput {
                view: &view,
                extent: [output.width(), output.height()],
            };
            let mut expected_pixels = None;
            for (order, requested) in [true, false, true].into_iter().enumerate() {
                host.set_timing_requested(requested);
                let before = host.timing_stats().gathered;
                assert!(
                    host.drive(Duration::ZERO, Some(&surface))
                        .unwrap()
                        .submitted
                );
                host.set_timing_requested(!requested);
                let captured = image(&mut host);
                assert_eq!(captured.sample.submission_order, order as u64);
                assert_eq!(captured.sample.timing_requested, requested);
                assert_eq!(captured.pixel_source, source);
                if requested {
                    assert!(captured.copy_gpu_ns.is_some_and(|time| time > 0.0));
                    expected_pixels.get_or_insert_with(|| captured.rgba.clone());
                } else {
                    assert_eq!(captured.copy_gpu_ns, None);
                    assert_eq!(Some(&captured.rgba), expected_pixels.as_ref());
                }
                assert_eq!(host.timing_stats().gathered, before + u64::from(requested));
                host.recycle_submitted_image(captured);
            }
            assert_eq!(host.submitted_capture_omissions(), 0);
            println!("{source:?}: timed, untimed and timed captures delivered");
        }
    });
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
fn surface_resize_admits_only_the_configured_drawable_in_sdr_and_edr() {
    let Some(test_gpu) = thrum_test_support::gpu(
        "surface_resize_admits_only_the_configured_drawable_in_sdr_and_edr",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    pollster::block_on(async {
        for format in [
            wgpu::TextureFormat::Bgra8Unorm,
            wgpu::TextureFormat::Rgba16Float,
        ] {
            let (mut host, old) = Box::pin(host_and_output(&test_gpu, format)).await;
            for readback in [false, true] {
                let graph = host
                    .prepared
                    .graph(&host.slots[0].target, readback)
                    .unwrap();
                assert_eq!(
                    graph
                        .pass_bindings()
                        .any(|(_, pass)| pass.key == Some(crate::pass_key::PassKey::Display)),
                    readback || format != wgpu::TextureFormat::Rgba16Float,
                );
            }
            let original = host.render_extent;
            let resized = [103, 43];
            let new = output_at_extent(&host, format, resized);
            host.set_timing_requested(true);
            let sample = submit(&mut host, &old);
            assert!(sample.surface_presented);
            assert!(sample.gpu_pass_ns.is_some());
            host.resize_surface(resized).unwrap();
            for (texture, extent, presented) in [
                (&old, original, false),
                (&old, resized, false),
                (&new, original, false),
                (&new, resized, true),
            ] {
                let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                let output = SurfaceOutput {
                    view: &view,
                    extent,
                };
                let report = host.drive(Duration::ZERO, Some(&output)).unwrap();
                assert_eq!(report.surface_presented, presented, "{format:?} {extent:?}");
                assert_eq!(
                    report.surface_fault,
                    (!presented).then_some(crate::runtime::SurfaceFault::ExtentMismatch),
                );
                thrum_test_support::wait::until(
                    "resized submission retires",
                    thrum_test_support::readback::WAIT,
                    || {
                        drop(host.poll_completed().unwrap());
                        (host.pending_frames() == 0).then_some(())
                    },
                );
            }
            let pixels = thrum_test_support::readback::texture(
                &host.device,
                &host.queue,
                &new,
                thrum_test_support::readback::Mip::level(0),
                ..,
            );
            let stride = if format == wgpu::TextureFormat::Rgba16Float {
                8
            } else {
                4
            };
            let left = &pixels[..stride];
            let centre = (resized[0] as usize / 2) * stride;
            let alpha = if stride == 8 {
                &[0, 60][..]
            } else {
                &[255][..]
            };
            assert_eq!(
                left.split_at(stride - alpha.len()),
                (&[0; 6][..stride - alpha.len()], alpha)
            );
            assert_ne!(
                &pixels[centre..centre + stride - alpha.len()],
                &[0; 6][..stride - alpha.len()]
            );
        }
    });
}

#[test]
fn capture_matches_resized_drawable_in_edr_and_scene_grid_in_sdr() {
    let Some(test_gpu) = thrum_test_support::gpu(
        "capture_matches_resized_drawable_in_edr_and_scene_grid_in_sdr",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    pollster::block_on(async {
        for (format, source) in [
            (wgpu::TextureFormat::Bgra8Unorm, CaptureSource::Sdr),
            (wgpu::TextureFormat::Rgba16Float, CaptureSource::Edr),
        ] {
            for capture_before_resize in [false, true] {
                let (mut host, original) = Box::pin(host_and_output(&test_gpu, format)).await;
                let render_extent = host.render_extent;
                let drawable_extent = [103, 43];
                let output = output_at_extent(&host, format, drawable_extent);
                let earlier = capture_before_resize.then(|| {
                    host.enable_submitted_capture(1).unwrap();
                    submit(&mut host, &original)
                });
                host.resize_surface(drawable_extent).unwrap();
                if !capture_before_resize {
                    host.enable_submitted_capture(1).unwrap();
                }
                let frame = submit(&mut host, &output);
                assert!(frame.surface_presented);
                if let Some(earlier) = earlier {
                    let first = image(&mut host);
                    assert_eq!(first.sample.submission_order, earlier.submission_order);
                    assert_eq!(first.extent, render_extent);
                    host.recycle_submitted_image(first);
                }
                let captured = image(&mut host);
                assert_eq!(captured.sample.submission_order, frame.submission_order);
                assert_eq!(captured.session, 1);
                assert_eq!(captured.pixel_source, source);
                assert!(captured.copy_gpu_ns.is_some_and(|time| time > 0.0));
                assert_eq!(host.submitted_capture_omissions(), 0);
                if source == CaptureSource::Edr {
                    let actual = thrum_test_support::readback::texture(
                        &host.device,
                        &host.queue,
                        &output,
                        thrum_test_support::readback::Mip::level(0),
                        ..,
                    );
                    assert_eq!(captured.extent, drawable_extent);
                    assert_eq!(captured.rgba, actual);
                    let black = [0, 0, 0, 0, 0, 0, 0, 60];
                    let row_bytes = drawable_extent[0] as usize * black.len();
                    for row in captured.rgba.chunks_exact(row_bytes) {
                        assert_eq!(row[..8], black);
                        assert_eq!(row[row_bytes - 8..], black);
                    }
                    let centre = (drawable_extent[1] as usize / 2) * row_bytes + row_bytes / 2 - 4;
                    assert_ne!(captured.rgba[centre..centre + 8], black);
                } else {
                    assert_eq!(captured.extent, render_extent);
                    assert_eq!(
                        captured.rgba.len(),
                        (render_extent[0] * render_extent[1] * 4) as usize
                    );
                }
            }
        }
    });
}

#[test]
fn promotion_preserves_session_limits_until_writer_releases_its_image() {
    let Some(test_gpu) = thrum_test_support::gpu(
        "promotion_preserves_session_limits_until_writer_releases_its_image",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    pollster::block_on(async {
        for recycle in [false, true] {
            let (mut host, sdr) =
                Box::pin(host_and_output(&test_gpu, wgpu::TextureFormat::Bgra8Unorm)).await;
            let limits = crate::runtime::CaptureLimits {
                staging_slots: 1,
                cpu_buffers: 1,
                // One float staging slot, its exposure sample, and one float CPU image.
                byte_allowance: 768 * 37 + 32 + 65 * 37 * 8,
            };
            host.enable_submitted_capture_with_limits(42, limits)
                .unwrap();
            #[cfg(feature = "creation-counters")]
            let before = capture_timing_creations(&host);
            let first = submit(&mut host, &sdr);
            let float = wgpu::TextureFormat::Rgba16Float;
            let edr = output_texture(&host, float);
            let candidate = host
                .extent_prepare_inputs_for_format(host.render_extent, float)
                .unwrap()
                .build(&std::sync::atomic::AtomicBool::new(false))
                .unwrap()
                .unwrap();
            assert!(host.publish_extent(candidate));
            assert_eq!(host.submitted_capture_usage(), Some((1, 0)));
            assert!(submit(&mut host, &edr).surface_presented);
            assert_eq!(host.submitted_capture_usage(), Some((1, 0)));
            let old = image(&mut host);
            assert_eq!(old.sample.submission_order, first.submission_order);
            assert_eq!(old.pixel_source, CaptureSource::Sdr);

            assert!(submit(&mut host, &edr).surface_presented);
            assert_eq!(host.submitted_capture_usage(), Some((0, 1)));
            assert_eq!(host.submitted_capture_omissions(), 2);
            assert!(host.try_submitted_image().unwrap().is_none());
            if recycle {
                host.recycle_submitted_image(old);
            } else {
                drop(old);
            }
            assert_eq!(host.submitted_capture_usage(), Some((0, 0)));
            let resumed = submit(&mut host, &edr);
            assert!(resumed.surface_presented);
            let image = image(&mut host);
            assert_eq!(image.session, 42);
            assert_eq!(image.sample.submission_order, resumed.submission_order);
            assert_eq!(image.pixel_source, CaptureSource::Edr);
            assert!(image.copy_gpu_ns.is_some_and(|time| time > 0.0));
            assert_eq!(host.submitted_capture_usage(), Some((0, 1)));
            #[cfg(feature = "creation-counters")]
            assert_eq!(capture_timing_creations(&host), before);
        }
    });
}

#[cfg(feature = "creation-counters")]
fn capture_timing_creations(host: &FrameHost<StaticWorld, StaticScene>) -> (u64, i64, i64) {
    let report = host.creation_report();
    let pipelines = report
        .pipelines
        .iter()
        .filter(|row| row.label == "capture-copy-timing")
        .map(|row| row.compute)
        .sum();
    let (groups, queries) = report
        .rows
        .iter()
        .filter(|row| row.label == "capture-copy-timing")
        .fold((0, 0), |(groups, queries), row| {
            (
                groups + row.counts.bind_groups,
                queries + row.counts.query_sets,
            )
        });
    (pipelines, groups, queries)
}

#[test]
fn retired_pixel_bytes_limit_capture_even_when_a_slot_and_cpu_lease_are_available() {
    let Some(test_gpu) = thrum_test_support::gpu(
        "retired_pixel_bytes_limit_capture_even_when_a_slot_and_cpu_lease_are_available",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    pollster::block_on(async {
        let byte = wgpu::TextureFormat::Bgra8Unorm;
        let float = wgpu::TextureFormat::Rgba16Float;
        let (mut host, _) = Box::pin(host_and_output(&test_gpu, byte)).await;
        let old_extent = [64, 37];
        let new_extent = [33, 25];
        assert!(host.resize_scene_surface(old_extent).unwrap());
        let sdr = output_at_extent(&host, byte, old_extent);
        let edr = output_at_extent(&host, float, new_extent);
        host.enable_submitted_capture_with_limits(
            42,
            crate::runtime::CaptureLimits {
                staging_slots: 1,
                cpu_buffers: 2,
                // The SDR pool fits exactly. The float staging row has 248
                // padding bytes, so one float slot and CPU image plus the
                // writer's old bytes would need 28,904 bytes.
                byte_allowance: 3 * 256 * 37 + 32,
            },
        )
        .unwrap();
        assert!(submit(&mut host, &sdr).surface_presented);
        let old = image(&mut host);
        let next = host
            .extent_prepare_inputs_for_format(new_extent, float)
            .unwrap()
            .build(&std::sync::atomic::AtomicBool::new(false))
            .unwrap()
            .unwrap();
        assert!(host.publish_extent(next));

        assert!(submit(&mut host, &edr).surface_presented);
        assert_eq!(host.submitted_capture_usage(), Some((0, 1)));
        assert_eq!(host.submitted_capture_omissions(), 1);
        assert!(host.try_submitted_image().unwrap().is_none());
        host.recycle_submitted_image(old);

        let resumed = submit(&mut host, &edr);
        assert!(resumed.surface_presented);
        let current = image(&mut host);
        assert_eq!(current.session, 42);
        assert_eq!(current.sample.submission_order, resumed.submission_order);
        assert_eq!(current.rgba.len(), 33 * 25 * 8);
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
    for format in [
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureFormat::Rgba16Float,
    ] {
        if format == wgpu::TextureFormat::Rgba16Float && !cfg!(target_os = "macos") {
            continue;
        }
        pollster::block_on(async {
            let (mut host, output) = Box::pin(host_and_output(&test_gpu, format)).await;
            host.enable_submitted_capture(1).unwrap();
            let view = output.create_view(&wgpu::TextureViewDescriptor::default());
            assert!(
                host.drive(
                    Duration::ZERO,
                    Some(&SurfaceOutput {
                        view: &view,
                        extent: [65, 37],
                    })
                )
                .unwrap()
                .surface_presented
            );
            host.device.destroy();
            let inputs = host.device_prepare_inputs().unwrap();
            let (capabilities, queue) = device::gpu(&test_gpu.fresh_adapter(), 8).await;
            let mut replacement = inputs
                .prepare(
                    &capabilities,
                    &queue,
                    Some(SurfaceProfile {
                        format,
                        extent: [65, 37],
                    }),
                )
                .await
                .unwrap();
            host.publish_device(&mut replacement).unwrap();
            let validation = host.device.push_error_scope(wgpu::ErrorFilter::Validation);
            let output = output_texture(&host, format);
            assert!(submit(&mut host, &output).surface_presented);
            assert_eq!(host.submitted_capture_omissions(), 1, "{format:?}");
            assert!(matches!(
                host.try_submitted_image(),
                Err(super::super::FrameError::InvalidState(
                    "retired capture generation"
                ))
            ));

            // A later extent publication must also leave the old session invalidated.
            let extent = host
                .extent_prepare_inputs(host.render_extent)
                .unwrap()
                .build(&std::sync::atomic::AtomicBool::new(false))
                .unwrap()
                .unwrap();
            assert!(host.publish_extent(extent));
            assert!(submit(&mut host, &output).surface_presented);
            assert_eq!(host.submitted_capture_omissions(), 2, "{format:?}");
            assert!(matches!(
                host.try_submitted_image(),
                Err(super::super::FrameError::InvalidState(
                    "retired capture generation"
                ))
            ));

            host.disable_submitted_capture();
            host.enable_submitted_capture(2).unwrap();
            assert!(submit(&mut host, &output).surface_presented);
            let fresh = image(&mut host);
            assert_eq!(fresh.session, 2);
            assert_eq!(fresh.device_generation, 8);
            assert_eq!(fresh.presenter_generation, 3);
            assert!(validation.pop().await.is_none(), "{format:?}");
        });
    }
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
fn edr_graph_retains_the_sdr_encode_only_for_an_sdr_readback() {
    use crate::runtime::CaptureLimits;

    let Some(test_gpu) = thrum_test_support::gpu(
        "edr_graph_retains_the_sdr_encode_only_for_an_sdr_readback",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    pollster::block_on(async {
        let (mut host, output) =
            Box::pin(host_and_output(&test_gpu, wgpu::TextureFormat::Rgba16Float)).await;
        let check_graph = |host: &FrameHost<StaticWorld, StaticScene>, readback, tone| {
            let graph = host.scene_graph_variant(readback).unwrap();
            assert_eq!(
                graph
                    .pass_names()
                    .any(|pass| pass == "synthetic tone encode"),
                tone,
            );
            assert_eq!(
                graph
                    .pass_bindings()
                    .any(|(_, pass)| pass.kind == Some(crate::graph::PassKind::Copy)),
                readback,
            );
        };
        check_graph(&host, false, false);
        for (session, source) in [(1, CaptureSource::SceneLinear), (2, CaptureSource::Sdr)] {
            host.enable_submitted_capture_from(session, CaptureLimits::default(), source)
                .unwrap();
            check_graph(&host, false, false);
            check_graph(&host, true, source == CaptureSource::Sdr);
            submit(&mut host, &output);
            let collected = image(&mut host);
            assert_eq!(collected.pixel_source, source);
            host.recycle_submitted_image(collected);
            host.disable_submitted_capture();
            for _ in 0..3 {
                check_graph(&host, false, false);
                let frame = submit(&mut host, &output);
                assert!(frame.surface_presented);
                assert_eq!(host.submitted_capture_usage(), None);
            }
        }
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
        let (mut host, output) =
            Box::pin(host_and_output(&test_gpu, wgpu::TextureFormat::Rgba8Unorm)).await;
        let limits = crate::runtime::CaptureLimits {
            staging_slots: 1,
            cpu_buffers: 1,
            byte_allowance: 30_000,
        };
        let allocations = crate::synthetic::target_allocation::starts();
        assert!(matches!(
            host.enable_submitted_capture_from(1, limits, CaptureSource::SceneLinear),
            Err(super::super::FrameError::InvalidState(
                "capture pixel-byte allowance exhausted"
            ))
        ));
        assert_eq!(crate::synthetic::target_allocation::starts(), allocations);
        assert_eq!(host.slots[0].target.capture_source(), CaptureSource::Sdr);
        assert!(host.submitted_capture.is_none());
        host.enable_submitted_capture_with_limits(2, limits)
            .unwrap();
        submit(&mut host, &output);
        let captured = image(&mut host);
        assert_eq!(captured.pixel_source, CaptureSource::Sdr);
        assert_eq!(captured.rgba.len(), 65 * 37 * 4);
    });
}
