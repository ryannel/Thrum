use super::{Host, Policy, gpu, host};
use std::time::{Duration, Instant};

fn retire_scene(host: &mut Host) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let timeout = deadline.saturating_duration_since(Instant::now());
        assert!(
            !timeout.is_zero(),
            "camera-history test scene did not retire"
        );
        host.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(timeout),
            })
            .expect("wait for scene or its timing readback");
        if host.poll_completed().expect("collect scene").is_some() {
            assert_eq!(host.pending_frames(), 0);
            return;
        }
    }
}

#[test]
fn a_skipped_camera_cut_resets_only_the_first_submitted_view_in_its_epoch() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (capabilities, queue) = gpu(&instance, 7).await;
        let mut host = host(
            &capabilities,
            &queue,
            Policy {
                camera_cut_tick: Some(2),
                empty: true,
                ..Policy::default()
            },
        )
        .await;
        assert!(
            host.advance(Duration::ZERO)
                .expect("initial authority")
                .encode_scene()
                .expect("initial recording")
                .submit(None)
                .expect("initial submission")
                .submitted
        );
        retire_scene(&mut host);
        host.world.authority.tick = 3;
        host.world.publication.tick = 3;
        let abandoned = host
            .advance(Duration::ZERO)
            .expect("post-cut authority")
            .encode_scene()
            .expect("post-cut recording");
        assert!(abandoned.can_acquire_surface());
        drop(abandoned);
        assert!(host.scene.view.history_reset());
        assert_eq!(host.submitted_view_history_epoch, 0);

        let pixels = host.capture_pixels().expect("post-cut capture");
        assert_eq!(pixels.len(), 64 * 64 * 4);
        assert!(host.scene.view.history_reset());
        assert_eq!(host.submitted_view_history_epoch, 0);

        for reset in [true, false] {
            assert!(
                host.advance(Duration::ZERO)
                    .expect("same authority")
                    .encode_scene()
                    .expect("same-epoch recording")
                    .submit(None)
                    .expect("same-epoch submission")
                    .submitted
            );
            assert_eq!(host.scene.view.history_reset(), reset);
            assert_eq!(host.submitted_view_history_epoch, 1);
            retire_scene(&mut host);
        }
    });
}

#[test]
fn m2_history_commits_in_submission_order_with_two_in_flight_and_cancelled_acquisition() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (capabilities, queue) = gpu(&instance, 7).await;
        let mut host = host(&capabilities, &queue, Policy::default()).await;
        let encoded_a = host
            .advance(Duration::ZERO)
            .unwrap()
            .encode_scene()
            .unwrap();
        let a = encoded_a.history_identity().unwrap();
        let a_view = encoded_a.frame_extent().unwrap().view_projection;
        assert_eq!(a[1], 0);
        encoded_a.submit(None).unwrap();
        assert_eq!(host.pending_frames(), 1);
        // Admit B without polling A's completion. This is the real admission
        // and encoding path with completion deliberately held at the host boundary.
        let (report, advance) = host.admit_world(Duration::ZERO).unwrap();
        let held = crate::runtime::AdvancedFrame {
            host: Some(&mut host),
            report: Some(report),
            tick_cpu: advance.tick_cpu,
            wall_start: Instant::now(),
            recovered_publication: advance.recovered_publication,
        }
        .encode_scene()
        .unwrap();
        let b = held.history_identity().unwrap();
        assert_eq!(
            held.frame_extent().unwrap().previous_view_projection,
            a_view,
            "previous matrices are A before completion"
        );
        assert_eq!(
            b,
            [a[0] + 1, a[0]],
            "held second sample reads A before completion"
        );
        held.submit(None).unwrap();
        assert_eq!(host.pending_frames(), 2);
        assert_eq!(
            host.prepared.history_identity()[1],
            b[0],
            "submit accepts without CPU completion"
        );
        println!("M2 D2.4 two in flight and held submission: A={a:?} B={b:?}");
        drain_history_frames(&mut host, 2);
        host.source.camera = 0.2;
        let cancelled = host
            .advance(Duration::ZERO)
            .unwrap()
            .encode_scene()
            .unwrap();
        let c = cancelled.history_identity().unwrap();
        assert_eq!(
            cancelled.frame_extent().unwrap().previous_view_projection,
            a_view
        );
        assert_eq!(c[1], b[0]);
        cancelled.skip();
        assert_eq!(
            host.prepared.history_identity(),
            c,
            "failed acquisition does not advance"
        );
        let successor = host
            .advance(Duration::ZERO)
            .unwrap()
            .encode_scene()
            .unwrap();
        assert_eq!(successor.history_identity(), Some(c));
        assert_eq!(
            successor.frame_extent().unwrap().previous_view_projection,
            a_view,
            "cancelled pose must not become previous matrices"
        );
        successor.submit(None).unwrap();
        println!("M2 D2.4 cancelled acquisition counted=1; successor={c:?}");
        drain_history_frames(&mut host, 1);
        // A background lease refusal uses the lost/forbidden-queue cancellation:
        // no queue call, and a reset at the next accepted foreground sample.
        host.advance(Duration::ZERO)
            .unwrap()
            .encode_scene()
            .unwrap()
            .cancel_background();
        assert!(host.history_reset_pending);
        let resumed = host
            .advance(Duration::ZERO)
            .unwrap()
            .encode_scene()
            .unwrap();
        assert_eq!(resumed.history_identity().unwrap()[1], 0);
        resumed.submit(None).unwrap();
        println!("M2 D2.4 background/forbidden-queue drop counted=1; reset predecessor=0");
    });
}

#[test]
fn m2_forced_submission_refusal_resets_without_publishing_the_candidate() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (capabilities, queue) = gpu(&instance, 7).await;
        let mut host = host(&capabilities, &queue, Policy::default()).await;
        host.advance(Duration::ZERO)
            .unwrap()
            .encode_scene()
            .unwrap()
            .submit(None)
            .unwrap();
        drain_history_frames(&mut host, 1);
        let prior = host.prepared.history_identity();
        let mut advanced = host.advance(Duration::ZERO).unwrap();
        let mut report = advanced.report.take().unwrap();
        let host = advanced.host.take().unwrap();
        let frame = host.extract_serial(Duration::ZERO, 0, 0).unwrap();
        let sample = host
            .prepare_extracted(0, &mut report, frame)
            .unwrap()
            .unwrap();
        let recorded = host.encode_scene_only(0, sample).unwrap();
        let result = host.finish_encoded_with(recorded, &mut report, None, Instant::now(), || {
            Err(super::FrameError::Gpu("forced submission refusal".into()))
        });
        assert!(result.is_err());
        assert_eq!(host.prepared.history_identity(), prior);
        assert!(host.history_reset_pending);
        assert_eq!(host.pending_frames(), 0);
        let next = host
            .advance(Duration::ZERO)
            .unwrap()
            .encode_scene()
            .unwrap();
        assert_eq!(next.history_identity().unwrap()[1], 0);
        next.submit(None).unwrap();
        println!(
            "M2 D2.4 forced submission refusal counted=1; previous identity retained and next sample reset"
        );
    });
}

#[test]
fn m2_temporal_passes_preserve_named_timing_layout() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (capabilities, queue) = gpu(&instance, 7).await;
        let mut host = host(&capabilities, &queue, Policy::default()).await;
        host.advance(Duration::ZERO)
            .unwrap()
            .encode_scene()
            .unwrap()
            .submit(None)
            .unwrap();
        host.device
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();
        host.poll_gpu().unwrap();
        host.device
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();
        let slot = &mut host.slots[0];
        let passes = slot
            .timings
            .try_collect(7)
            .expect("real timestamp batch")
            .expect("resolved batch");
        println!("M2 actual named timings: {passes:?}");
        assert!(
            crate::synthetic::pass_durations(passes, false, slot.timing_layout).is_some(),
            "M2 pass layout"
        );
    });
}

fn drain_history_frames(host: &mut Host, count: usize) {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut collected = 0;
    while collected < count {
        if let Some(sample) = host.poll_completed().expect("history frame completion") {
            assert!(!sample.timing_fault, "history frame timing failed");
            assert!(
                sample.gpu_pass_ns.is_some(),
                "Metal history proof needs valid timing too"
            );
            collected += 1;
        }
        assert!(
            Instant::now() < deadline,
            "history frame completion timed out"
        );
        std::thread::yield_now();
    }
    assert_eq!(host.pending_frames(), 0);
}
