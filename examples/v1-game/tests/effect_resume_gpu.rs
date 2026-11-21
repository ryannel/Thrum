//! Submitted effect times across inline and resident-world lifecycle boundaries.
use crate::support;
#[cfg(target_os = "macos")]
use std::time::{Duration, Instant};
#[cfg(target_os = "macos")]
use thrum_core::sim::{WorldLifecycle, WorldLifecycleReply};
#[cfg(target_os = "macos")]
use thrum_render::runtime::{FrameConfig, FrameSample};
use v1_game::{GameHost, GameSetup, WorldControls, effect::DiagnosticEffect};

#[track_caller]
#[cfg(target_os = "macos")]
fn complete(mut poll: impl FnMut() -> Option<FrameSample>, device: &wgpu::Device) -> FrameSample {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        if let Some(sample) = poll() {
            return sample;
        }
        assert!(Instant::now() < deadline);
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("Metal");
    }
}

#[cfg(target_os = "macos")]
fn lifecycle(
    controls: &WorldControls,
    request: WorldLifecycle,
    origin: Instant,
) -> (Duration, WorldLifecycleReply, Duration) {
    let asked = origin.elapsed();
    controls.request_lifecycle(request).expect("request");
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(reply) = controls.poll_lifecycle_reply().expect("reply") {
            return (asked, reply, origin.elapsed());
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_micros(200));
    }
}

#[cfg(target_os = "macos")]
fn display(sample: &FrameSample) -> f64 {
    assert_eq!(
        sample.presentation_alpha.to_bits(),
        sample.display_phase.min(1.0).to_bits()
    );
    if sample.clock_interpolating {
        let due = sample.tick_due_time.expect("interpolating due time");
        let world = (sample.tick as f64 - 1.0 + f64::from(sample.display_phase)) / 60.0;
        if sample.display_phase < 4.0 {
            let independent = (sample.tick as f64 - 1.0) / 60.0
                + (sample.observation + sample.presentation_lead).as_secs_f64()
                - due.as_secs_f64();
            assert!((world - independent).abs() < 0.000_002);
        }
        world
    } else {
        assert_eq!(
            (sample.presentation_alpha, sample.display_phase),
            (1.0, 1.0)
        );
        assert_eq!(sample.tick_due_time, None);
        sample.tick as f64 / 60.0
    }
}

#[test]
#[cfg(target_os = "macos")]
fn submitted_frames_exclude_stopped_time_and_discarded_fraction() {
    let Some(test_gpu) = thrum_test_support::gpu(
        "submitted_frames_exclude_stopped_time_and_discarded_fraction",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    pollster::block_on(async {
        let (_, device, queue, _) = support::device(&test_gpu, "portable", false);
        let (view, profile) = support::presentation(&device, [96, 54]);
        let surface = thrum_render::runtime::SurfaceOutput {
            view: &view,
            extent: [96, 54],
        };
        let mut records = Vec::new();
        for lead_ms in [0, 10, 50] {
            let mut host = GameSetup::default()
                .prepare_on(
                    &device,
                    &queue,
                    FrameConfig {
                        render_extent: [96, 54],
                        surface: Some(profile),
                    },
                )
                .await
                .expect("V1 host");
            let (effect, handle) = DiagnosticEffect::new();
            host.set_game_contribution(Box::new(effect))
                .expect("effect");
            host.set_presentation_lead(Duration::from_millis(lead_ms));
            let report = host
                .drive(Duration::from_millis(58), Some(&surface))
                .expect("partial tick");
            assert!(report.submitted, "initial partial tick: {report:?}");
            let before = complete(|| host.poll_completed().expect("completion"), &device);
            assert_eq!(before.tick, 3);
            host.world_mut().pause().expect("pause");
            for ms in [58, 958, 1058] {
                let report = host
                    .drive(Duration::from_millis(ms), Some(&surface))
                    .expect("paused");
                assert!(report.submitted, "inline paused: {report:?}");
                let s = complete(|| host.poll_completed().expect("completion"), &device);
                assert_eq!(s.tick, 3);
                assert!(!s.clock_interpolating);
                assert_eq!(display(&s), 0.050);
                let time = handle.lock().expect("effect").time.expect("time");
                assert_eq!(
                    (time.tick, time.alpha, time.display_phase),
                    (s.tick, s.presentation_alpha, s.display_phase)
                );
            }
            let resume = Duration::from_millis(1058);
            host.world_mut().resume(resume).expect("resume");
            for after_ms in [0, 5, 16, 25, 40, 55, 75] {
                let observation = resume + Duration::from_millis(after_ms);
                let report = host
                    .drive(observation, Some(&surface))
                    .expect("resumed frame");
                assert!(report.submitted, "inline resumed: {report:?}");
                let s = complete(|| host.poll_completed().expect("completion"), &device);
                let time = handle.lock().expect("effect").time.expect("time");
                assert_eq!(
                    (time.tick, time.alpha, time.display_phase),
                    (s.tick, s.presentation_alpha, s.display_phase)
                );
                let world = display(&s);
                if after_ms < 17 {
                    assert!(!s.clock_interpolating);
                    assert_eq!(s.tick, 3);
                    assert_eq!(world, 0.050);
                } else {
                    let ticks_since_resume = after_ms * 60 / 1000;
                    assert_eq!(s.tick, 3 + ticks_since_resume);
                    // Scripted resume and elapsed intervals, never the returned phase/due time.
                    let due_ns = resume.as_nanos()
                        + (u128::from(ticks_since_resume) * 1_000_000_000).div_ceil(60);
                    assert_eq!(s.tick_due_time.expect("due").as_nanos(), due_ns);
                    let expected = 0.050 + (after_ms as f64 + lead_ms as f64) / 1000.0 - 1.0 / 60.0;
                    assert!((world - expected).abs() < 0.000_002);
                    assert!(
                        world >= 0.050,
                        "resume never moves behind its committed tick"
                    );
                }
                records.push(serde_json::json!({"mode":"inline","lead_ms":lead_ms,
                    "resume_ns":resume.as_nanos(),"observation_ns":observation.as_nanos(),
                    "tick":s.tick,"due_ns":s.tick_due_time.map(|d| d.as_nanos()),
                    "phase":s.display_phase,"alpha":s.presentation_alpha,"interpolating":s.clock_interpolating}));
            }
        }
        for gap_ms in [0, 20] {
            let mut host = GameSetup::default()
                .prepare_on(
                    &device,
                    &queue,
                    FrameConfig {
                        render_extent: [96, 54],
                        surface: Some(profile),
                    },
                )
                .await
                .expect("V1 host");
            let (effect, handle) = DiagnosticEffect::new();
            host.set_game_contribution(Box::new(effect))
                .expect("effect");
            let origin = Instant::now();
            let (mut host, controls) = host.into_threaded(origin, || {}).expect("resident world");
            let (_, paused, _) = lifecycle(&controls, WorldLifecycle::Pause, origin);
            let report = host
                .drive(origin.elapsed(), Some(&surface))
                .expect("paused");
            assert!(report.submitted, "resident paused: {report:?}");
            let s = complete(|| host.poll_completed().expect("completion"), &device);
            assert_eq!(s.tick, paused.id.tick);
            assert!(!s.clock_interpolating);
            display(&s);
            let (asked, resumed, answered) = lifecycle(&controls, WorldLifecycle::Resume, origin);
            assert_eq!(resumed.id.tick, paused.id.tick);
            let mut first_origin: Option<f64> = None;
            let mut interpolating = 0;
            let mut submitted = 0;
            let mut attempts = 0;
            let deadline = Instant::now() + Duration::from_secs(60);
            // A dropped opportunity does not wait for GPU completion. Give the
            // world owner time to publish instead of exhausting a tight retry cap.
            while submitted < 24 && Instant::now() < deadline {
                let observation = origin.elapsed();
                std::thread::sleep(Duration::from_millis(gap_ms));
                host.set_presentation_lead(Duration::from_millis([0, 10, 50][attempts % 3]));
                attempts += 1;
                let report = host
                    .drive(observation, Some(&surface))
                    .expect("resumed resident frame");
                if !report.submitted {
                    assert!(
                        report.overload.is_none(),
                        "world remains active: {report:?}"
                    );
                    records.push(serde_json::json!({"mode":"threaded","gap_ms":gap_ms,
                        "observation_ns":observation.as_nanos(),"submitted":false,
                        "tick":report.tick,"dropped_view":report.dropped_view}));
                    std::thread::sleep(Duration::from_millis(1));
                    continue;
                }
                submitted += 1;
                let s = complete(|| host.poll_completed().expect("completion"), &device);
                let time = handle.lock().expect("effect").time.expect("time");
                assert_eq!(
                    (time.tick, time.alpha, time.display_phase),
                    (s.tick, s.presentation_alpha, s.display_phase)
                );
                display(&s);
                if s.clock_interpolating {
                    assert!(s.tick > resumed.id.tick);
                    let offset = s.tick_due_time.expect("due").as_secs_f64() - s.tick as f64 / 60.0;
                    let first = *first_origin.get_or_insert(offset);
                    assert!((offset - first).abs() < 0.000_002);
                    let anchor = offset + resumed.id.tick as f64 / 60.0;
                    assert!(
                        anchor >= asked.as_secs_f64() - 0.000_002
                            && anchor <= answered.as_secs_f64() + 0.000_002
                    );
                    interpolating += 1;
                }
                records.push(serde_json::json!({"mode":"threaded","gap_ms":gap_ms,
                    "submitted":true,"dropped_view":report.dropped_view,
                    "resume_tick":resumed.id.tick,"asked_ns":asked.as_nanos(),"answered_ns":answered.as_nanos(),
                    "tick":s.tick,"due_ns":s.tick_due_time.map(|d| d.as_nanos()),
                    "observation_ns":s.observation.as_nanos(),"lead_ns":s.presentation_lead.as_nanos(),
                    "phase":s.display_phase,"alpha":s.presentation_alpha,"interpolating":s.clock_interpolating}));
                std::thread::sleep(Duration::from_millis(8));
            }
            assert!(
                interpolating > 0 && submitted == 24,
                "bounded submitted-frame coverage: gap_ms={gap_ms}, attempts={attempts}, \
                 submitted={submitted}, interpolating={interpolating}"
            );
        }
        let output =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tmp/effect-graph-m4");
        std::fs::create_dir_all(&output).expect("output");
        std::fs::write(
            output.join("resume-frames.json"),
            serde_json::to_vec_pretty(&records).expect("JSON"),
        )
        .expect("record");
    });
}
