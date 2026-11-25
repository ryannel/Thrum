use super::*;
use crate::runtime::SlotState;
use crate::synthetic::NativeOutcome;

#[test]
fn metal_native_surface_and_rejection_reset_without_fixture_cuts() {
    pollster::block_on(async {
        let Some(test_gpu) = thrum_test_support::gpu(
            "metal_native_surface_and_rejection_reset_without_fixture_cuts",
            Some(wgpu::Backend::Metal),
        ) else {
            return;
        };
        let (capabilities, queue) = gpu(&test_gpu, 7).await;
        let mut host = host(
            &capabilities,
            &queue,
            Policy {
                native: NativeWitness::Metal,
                ..Policy::default()
            },
        )
        .await;
        host.enable_submitted_capture_from(
            1,
            CaptureLimits::default(),
            CaptureSource::NativeOutput,
        )
        .unwrap();
        let pipelines = host.pipeline_creation_count();
        let view = output(capabilities.device(), [64, 64]);
        host.advance(Duration::ZERO)
            .unwrap()
            .encode_scene()
            .unwrap()
            .submit(Some(&SurfaceOutput {
                view: &view,
                extent: [64, 64],
            }))
            .unwrap();
        let first = image(&mut host);
        assert_eq!(&first[..8], &[0, 60, 0, 60, 0, 60, 0, 60]);
        assert_eq!(host.native_witness_report().unwrap().resets, 1);
        host.resize_surface([96, 48]).unwrap();
        host.resize_surface([64, 64]).unwrap();
        let replacement_view = output(capabilities.device(), [64, 64]);
        host.advance(Duration::ZERO)
            .unwrap()
            .encode_scene()
            .unwrap()
            .submit(Some(&SurfaceOutput {
                view: &replacement_view,
                extent: [64, 64],
            }))
            .unwrap();
        let replaced = image(&mut host);
        assert_eq!(
            &replaced[..8],
            &[0, 60, 0, 60, 0, 60, 0, 60],
            "surface incarnation must reset native state without a fixture cut"
        );
        assert!(
            replaced == first,
            "surface replacement preserves exact full native output after its own reset"
        );
        assert_eq!(host.native_witness_report().unwrap().resets, 2);
        assert!(host.reject_native_provider());
        host.advance(Duration::ZERO)
            .unwrap()
            .encode_scene()
            .unwrap()
            .submit(Some(&SurfaceOutput {
                view: &replacement_view,
                extent: [64, 64],
            }))
            .unwrap();
        let rejected = image(&mut host);
        assert!(
            rejected == first,
            "runtime rejection selects the ready twin and resets without a fixture cut"
        );
        assert_eq!(
            host.native_witness_report().unwrap().selection,
            NativeWitness::Wgsl
        );
        assert_eq!(host.native_witness_report().unwrap().resets, 3);
        assert_eq!(host.pipeline_creation_count(), pipelines);
        println!(
            "M11 natural resets: real replacement consumer, exact full output and binary tile 1 after surface and rejection; reset counts 1/2/3; pipeline count unchanged; no fixture invalidate_history call"
        );
    });
}

#[test]
fn metal_native_detached_extent_preparation_admission_and_overlap() {
    pollster::block_on(async {
        let Some(test_gpu) = thrum_test_support::gpu(
            "metal_native_detached_extent_preparation_admission_and_overlap",
            Some(wgpu::Backend::Metal),
        ) else {
            return;
        };
        let (capabilities, queue) = gpu(&test_gpu, 7).await;
        let mut host = host(
            &capabilities,
            &queue,
            Policy {
                native: NativeWitness::Metal,
                ..Policy::default()
            },
        )
        .await;
        let old = Arc::downgrade(host.slots[0].target.native.as_ref().unwrap());
        let old_bytes = host.native_witness_report().unwrap().resident_bytes;
        let lighting = FrameLighting::Diagnostic(SyntheticLight::default());
        assert!(
            host.visual_prepare_inputs()
                .prepare_extent([0, 48], surface(), lighting, 1.0)
                .await
                .is_err()
        );
        assert_eq!(host.render_extent, [64, 64]);
        assert!(old.upgrade().is_some());
        let inputs = host.visual_prepare_inputs();
        let worker = std::thread::spawn(move || {
            pollster::block_on(inputs.prepare_extent([96, 48], surface(), lighting, 1.0))
        });
        let view = output(capabilities.device(), [64, 64]);
        host.advance(Duration::ZERO)
            .unwrap()
            .encode_scene()
            .unwrap()
            .submit(Some(&SurfaceOutput {
                view: &view,
                extent: [64, 64],
            }))
            .unwrap();
        let mut candidate = worker.join().unwrap().unwrap();
        let new_bytes = candidate.scene.as_ref().unwrap().slots[0]
            .target
            .native_witness_report()
            .unwrap()
            .resident_bytes;
        assert_eq!(old_bytes, 64 * 64 * 24 + 8);
        assert_eq!(new_bytes, 96 * 48 * 24 + 8);
        let mut sibling = host
            .visual_prepare_inputs()
            .prepare_extent([80, 40], surface(), lighting, 1.0)
            .await
            .unwrap();
        drain(&mut host);
        assert!(host.publish_visual(&mut candidate).unwrap());
        assert!(old.upgrade().is_none(), "retire only after completion");
        assert!(host.publish_visual(&mut sibling).is_err());
        assert_eq!(host.render_extent, [96, 48]);
        assert_eq!(host.native_witness_report().unwrap().submitted, 0);
        let view = output(capabilities.device(), [96, 48]);
        host.advance(Duration::ZERO)
            .unwrap()
            .encode_scene()
            .unwrap()
            .submit(Some(&SurfaceOutput {
                view: &view,
                extent: [96, 48],
            }))
            .unwrap();
        drain(&mut host);
        assert_eq!(host.native_witness_report().unwrap().resets, 1);
        #[cfg(feature = "creation-counters")]
        assert_eq!(host.creation_report().violations, 0);
        println!(
            "M11 detached extent: worker preparation while live frame submits; failed candidate preserves live provider; old/new unique provider overlap={} bytes (includes shared prepared twin); stale sibling refused; retirement after completion; new extent reset",
            old_bytes + new_bytes
        );
    });
}

#[test]
fn metal_native_timing_is_completion_gated_and_submission_identified() {
    pollster::block_on(async {
        let Some(test_gpu) = thrum_test_support::gpu(
            "metal_native_timing_is_completion_gated_and_submission_identified",
            Some(wgpu::Backend::Metal),
        ) else {
            return;
        };
        let (capabilities, queue) = gpu(&test_gpu, 7).await;
        for (selection, timing) in [
            (NativeWitness::Metal, true),
            (NativeWitness::Metal, false),
            (NativeWitness::Wgsl, true),
        ] {
            let mut host = host(
                &capabilities,
                &queue,
                Policy {
                    native: selection,
                    ..Policy::default()
                },
            )
            .await;
            host.set_timing_requested(timing);
            let view = output(capabilities.device(), [64, 64]);
            host.advance(Duration::ZERO)
                .unwrap()
                .encode_scene()
                .unwrap()
                .submit(Some(&SurfaceOutput {
                    view: &view,
                    extent: [64, 64],
                }))
                .unwrap();
            let SlotState::Rendering(sample) = &host.slots[0].state else {
                panic!("rendering slot")
            };
            let segment = sample.native_segment.clone().unwrap();
            assert_eq!(segment.outcome(), NativeOutcome::Pending);
            assert_eq!(segment.submission, sample.submission_order);
            assert_eq!(segment.generation, 7);
            assert_eq!(segment.pass_name(), "native stateful copy/select witness");
            drain(&mut host);
            match segment.outcome() {
                NativeOutcome::Timed([start, end])
                    if timing && selection == NativeWitness::Metal =>
                {
                    assert!(start > 0.0 && end > start);
                    println!(
                        "M11 actual native row: pass={} generation={} submission={} Metal-host-seconds=[{start:.9},{end:.9}] duration_ns={:.3}; no absolute correlation with wgpu query counters",
                        segment.pass_name(),
                        segment.generation,
                        segment.submission,
                        (end - start) * 1e9
                    );
                }
                NativeOutcome::Unmeasured if !timing || selection == NativeWitness::Wgsl => {}
                outcome => panic!("unexpected native timing: {outcome:?}"),
            }
        }
    });
}

#[test]
fn metal_native_rejection_is_exact_twin_and_completion_error_requires_rebuild() {
    pollster::block_on(async {
        let Some(test_gpu) = thrum_test_support::gpu(
            "metal_native_rejection_is_exact_twin_and_completion_error_requires_rebuild",
            Some(wgpu::Backend::Metal),
        ) else {
            return;
        };
        let (capabilities, queue) = gpu(&test_gpu, 7).await;
        for source in [
            CaptureSource::Sdr,
            CaptureSource::SceneLinear,
            CaptureSource::NativeOutput,
            CaptureSource::NativeHistory,
        ] {
            let twin = sequence(&capabilities, &queue, NativeWitness::Wgsl, source, true).await;
            assert_eq!(
                sequence(&capabilities, &queue, NativeWitness::Rejected, source, true).await,
                twin
            );
        }
        let mut host = host(
            &capabilities,
            &queue,
            Policy {
                native: NativeWitness::Metal,
                ..Policy::default()
            },
        )
        .await;
        let inputs = host.device_prepare_inputs().unwrap();
        host.enable_submitted_capture_from(
            1,
            CaptureLimits::default(),
            CaptureSource::NativeOutput,
        )
        .unwrap();
        host.inject_native_completion_error();
        let view = output(capabilities.device(), [64, 64]);
        host.advance(Duration::ZERO)
            .unwrap()
            .encode_scene()
            .unwrap()
            .submit(Some(&SurfaceOutput {
                view: &view,
                extent: [64, 64],
            }))
            .unwrap();
        let device = host.device.clone();
        let (report, advance) = host.admit_world(Duration::ZERO).unwrap();
        let held = crate::runtime::AdvancedFrame {
            host: Some(&mut host),
            report: Some(report),
            tick_cpu: advance.tick_cpu,
            wall_start: std::time::Instant::now(),
            recovered_publication: advance.recovered_publication,
        }
        .encode_scene()
        .unwrap();
        let flushes = crate::runtime::tokens::staging_flush::attempts();
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(10)),
            })
            .unwrap();
        held.skip();
        assert_eq!(
            crate::runtime::tokens::staging_flush::attempts(),
            flushes,
            "a candidate discarded after a prior native error must abandon staging without an empty submit"
        );
        assert!(
            matches!(
                host.try_submitted_image(),
                Err(FrameError::NativeCommandBuffer { generation: 7 })
            ),
            "tap-first draining must refuse timestamp-resolution queue work and failed pixels"
        );
        assert!(matches!(
            host.poll_completed(),
            Err(FrameError::NativeCommandBuffer { generation: 7 })
        ));
        let submits = host.native_witness_report().unwrap().submitted;
        assert!(matches!(
            host.advance(Duration::ZERO),
            Err(FrameError::NativeCommandBuffer { generation: 7 })
        ));
        assert_eq!(host.native_witness_report().unwrap().submitted, submits);
        let (second, queue2) = gpu(&test_gpu.fresh_adapter(), 8).await;
        let mut candidate = inputs
            .prepare(&second, &queue2, Some(surface()))
            .await
            .unwrap();
        host.disable_submitted_capture();
        host.publish_device(&mut candidate).unwrap();
        assert_eq!(host.native_failed_generation, None);
        assert_eq!(host.native_witness_report().unwrap().submitted, 0);
        let view = output(second.device(), [64, 64]);
        host.advance(Duration::ZERO)
            .unwrap()
            .encode_scene()
            .unwrap()
            .submit(Some(&SurfaceOutput {
                view: &view,
                extent: [64, 64],
            }))
            .unwrap();
        drain(&mut host);
        assert_eq!(host.native_witness_report().unwrap().completed, 1);
        assert_eq!(host.native_witness_report().unwrap().resets, 1);
        println!(
            "M11 rejection: four exact submitted tap streams match prepared twin; injected raw completion error reaches generation 7 owner, refuses further queue work; generation 8 rebuild clears fault"
        );
    });
}

#[test]
fn metal_native_witness_report_uses_the_prepared_binding_after_capture_rebind() {
    pollster::block_on(async {
        let Some(test_gpu) = thrum_test_support::gpu(
            "metal_native_witness_report_uses_the_prepared_binding_after_capture_rebind",
            Some(wgpu::Backend::Metal),
        ) else {
            return;
        };
        let (capabilities, queue) = gpu(&test_gpu, 7).await;
        let mut host = host(
            &capabilities,
            &queue,
            Policy {
                native: NativeWitness::Metal,
                empty: true,
                ..Policy::default()
            },
        )
        .await;
        // The selection belongs to preparation. The submitted camera can use
        // the default without disabling the bound diagnostic.
        host.source.native = NativeWitness::Off;
        let view = output(capabilities.device(), [64, 64]);
        let surface = SurfaceOutput {
            view: &view,
            extent: [64, 64],
        };
        host.advance(Duration::ZERO)
            .unwrap()
            .encode_scene()
            .unwrap()
            .submit(Some(&surface))
            .unwrap();
        drain(&mut host);
        let before = host.native_witness_report().unwrap();
        assert_eq!(before.selection, NativeWitness::Metal);
        assert_eq!(before.submitted, 1);
        host.enable_submitted_capture_from(
            1,
            CaptureLimits::default(),
            CaptureSource::NativeOutput,
        )
        .unwrap();
        assert_eq!(host.native_witness_report(), Some(before));
        assert_eq!(
            host.native_provider_report().unwrap().pass,
            crate::synthetic::native_witness::PASS
        );
        assert!(host.reject_native_provider());
        host.advance(Duration::ZERO)
            .unwrap()
            .encode_scene()
            .unwrap()
            .submit(Some(&surface))
            .unwrap();
        assert_eq!(image(&mut host).len(), 64 * 64 * 8);
        let after = host.native_witness_report().unwrap();
        assert_eq!(after.selection, NativeWitness::Wgsl);
        assert_eq!(after.submitted, 2);
        assert_eq!(after.resident_bytes, before.resident_bytes);
    });
}
