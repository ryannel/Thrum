//! Submitted V1 effect times, independently checked against consumed targets.
pub mod support;
#[cfg(target_os = "macos")]
use std::time::{Duration, Instant};
#[cfg(target_os = "macos")]
use thrum_render::runtime::{FrameConfig, FrameSample};
use v1_game::{GameHost, GameSetup, effect::DiagnosticEffect};

#[cfg(target_os = "macos")]
fn complete(mut poll: impl FnMut() -> Option<FrameSample>, device: &wgpu::Device) -> FrameSample {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        if let Some(sample) = poll() {
            return sample;
        }
        assert!(
            Instant::now() < deadline,
            "submitted frame did not complete"
        );
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("Metal");
    }
}

#[cfg(target_os = "macos")]
fn identity(sample: &FrameSample) {
    assert_eq!(
        sample.presentation_alpha.to_bits(),
        sample.display_phase.min(1.0).to_bits()
    );
    if sample.clock_interpolating && sample.display_phase < 4.0 {
        let expected_due = Duration::from_nanos(
            (u128::from(sample.tick) * 1_000_000_000)
                .div_ceil(60)
                .try_into()
                .expect("short test"),
        );
        assert_eq!(sample.tick_due_time, Some(expected_due));
        // Tick coordinates are exact 1/60; f32 phase costs < 0.01 us here.
        // Allow 2 us, including whole-nanosecond clock target arithmetic.
        let display = (sample.tick as f64 - 1.0 + f64::from(sample.display_phase)) / 60.0;
        let expected = (sample.observation + sample.presentation_lead).as_secs_f64() - 1.0 / 60.0;
        assert!(
            (display - expected).abs() < 0.000_002,
            "{sample:?}: {display} != {expected}"
        );
    } else if !sample.clock_interpolating {
        assert_eq!(sample.tick_due_time, None);
        assert_eq!(sample.display_phase, 1.0);
    }
}

#[test]
#[cfg(target_os = "macos")]
fn delayed_observation_still_describes_the_consumed_target() {
    let Some(test_gpu) = thrum_test_support::gpu(
        "delayed_observation_still_describes_the_consumed_target",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    pollster::block_on(async {
        let (_, device, queue, _) = support::device(&test_gpu, "portable", false);
        let (view, profile) = support::presentation(&device, [96, 54]);
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
            .expect("V1 game");
        let (effect, handle) = DiagnosticEffect::new();
        host.set_game_contribution(Box::new(effect))
            .expect("public effect seam");
        let origin = Instant::now();
        let (mut host, controls) = host.into_threaded(origin, || {}).expect("world lane");
        let observation = origin.elapsed();
        // Model preemption between reading the observation and polling the lane.
        std::thread::sleep(Duration::from_millis(40));
        let progress_deadline = Instant::now() + Duration::from_secs(3);
        while controls.check_health().expect("running world") < 2 {
            assert!(Instant::now() < progress_deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        host.set_presentation_lead(Duration::ZERO);
        let report = host
            .drive(
                observation,
                Some(&thrum_render::runtime::SurfaceOutput {
                    view: &view,
                    extent: [96, 54],
                }),
            )
            .expect("delayed render owner");
        assert!(report.submitted, "delayed frame: {report:?}");
        let sample = complete(|| host.poll_completed().expect("completion"), &device);
        assert!(
            sample.clock_interpolating,
            "the counterexample needs an active clock"
        );
        identity(&sample);
        let time = handle.lock().expect("effect").time.expect("delivered time");
        assert_eq!(time.tick, sample.tick);
        assert_eq!(time.display_phase, sample.display_phase);
        assert_eq!(time.alpha, sample.presentation_alpha);
        assert_eq!(
            time.tick, 0,
            "the observation preceded the first tick's due time"
        );
        let authority_tick = controls
            .check_health()
            .expect("authority continues independently");
        assert!(authority_tick >= 2);
        let output =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tmp/effect-graph-m4");
        std::fs::create_dir_all(&output).expect("output");
        std::fs::write(output.join("delayed-observation-fixed.json"),
            serde_json::to_vec_pretty(&serde_json::json!({"observation_ns":observation.as_nanos(),
                "lead_ns":sample.presentation_lead.as_nanos(),"frame_tick":time.tick,"authority_tick":authority_tick,
                "phase":time.display_phase,"alpha":time.alpha,"interpolating":sample.clock_interpolating,
                "precision_ns":2000})).expect("JSON")).expect("record");
    });
}

#[test]
#[cfg(target_os = "macos")]
fn submitted_effects_consume_unclamped_clock_phase_inline_and_threaded() {
    let Some(test_gpu) = thrum_test_support::gpu(
        "submitted_effects_consume_unclamped_clock_phase_inline_and_threaded",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    pollster::block_on(async {
        let (instance, device, queue, caps) = support::device(&test_gpu, "portable", false);
        let (view, profile) = support::presentation(&device, [96, 54]);
        let surface = thrum_render::runtime::SurfaceOutput {
            view: &view,
            extent: [96, 54],
        };
        let config = FrameConfig {
            render_extent: [96, 54],
            surface: Some(profile),
        };
        let mut records = Vec::new();
        for refresh in [120_u64, 60] {
            let mut host = GameSetup::default()
                .prepare_on(&device, &queue, config)
                .await
                .expect("V1 game");
            let (effect, handle) = DiagnosticEffect::new();
            host.set_game_contribution(Box::new(effect))
                .expect("effect seam");
            let mut previous: Option<(f64, f64)> = None;
            let mut past_endpoint = 0;
            let mut previous_field: Option<(u64, Vec<u32>)> = None;
            let mut nonempty = false;
            for lead_ms in [0, 10, 20, 30, 40, 50] {
                let lead = Duration::from_millis(lead_ms);
                for i in 0..8_u64 {
                    let index = lead_ms / 10 * 8 + i;
                    let observation = Duration::from_nanos(index * 1_000_000_000 / refresh);
                    host.set_presentation_lead(lead);
                    host.drive(observation, Some(&surface))
                        .expect("scripted frame");
                    let sample = complete(|| host.poll_completed().expect("completion"), &device);
                    assert_eq!(sample.observation, observation);
                    assert_eq!(sample.presentation_lead, lead);
                    identity(&sample);
                    let time = handle.lock().expect("effect").time.expect("delivered time");
                    assert_eq!(time.tick, sample.tick);
                    assert_eq!(time.alpha, sample.presentation_alpha);
                    assert_eq!(time.display_phase, sample.display_phase);
                    assert!(!time.capture);
                    let texture = handle.lock().expect("effect").textures[2][0].clone();
                    let field =
                        v1_game::effect::qualification::read_texture(&device, &queue, &texture, 0)
                            .await
                            .expect("consumer GPU contents");
                    nonempty |= field.iter().any(|&value| value != 0);
                    if let Some((old_tick, old_field)) = &previous_field
                        && *old_tick == time.tick
                    {
                        assert_eq!(
                            *old_field, field,
                            "repeated tick retains state with a different phase"
                        );
                    }
                    previous_field = Some((time.tick, field));
                    if time.display_phase > 2.0 {
                        past_endpoint += 1;
                    }
                    let display = (time.tick as f64 - 1.0 + f64::from(time.display_phase)) / 60.0;
                    let target = (observation + lead).as_secs_f64();
                    if let Some((old_display, old_target)) = previous {
                        assert!((display - old_display - (target - old_target)).abs() < 0.000_002);
                    }
                    previous = Some((display, target));
                    records.push(serde_json::json!({"mode":"inline","refresh":refresh,"tick":time.tick,
                        "observation_ns":observation.as_nanos(),"lead_ns":lead.as_nanos(),"phase":time.display_phase}));
                }
            }
            assert!(past_endpoint > 0);
            assert!(nonempty, "the kept field was not empty");
            host.world_mut().pause().expect("pause inline world");
            host.drive(Duration::from_secs(1), Some(&surface))
                .expect("paused");
            let paused = complete(|| host.poll_completed().expect("completion"), &device);
            identity(&paused);
            assert!(!paused.clock_interpolating);
            let texture = handle.lock().expect("effect").textures[2][0].clone();
            let paused_field =
                v1_game::effect::qualification::read_texture(&device, &queue, &texture, 0)
                    .await
                    .expect("paused GPU contents");
            assert_eq!(paused_field, previous_field.expect("running field").1);
            let committed = handle
                .lock()
                .expect("effect")
                .state
                .committed()
                .expect("submitted");
            let mut pending = host.begin_capture().expect("endpoint capture");
            let time = handle.lock().expect("effect").time.expect("capture time");
            assert!(time.capture);
            assert_eq!((time.alpha, time.display_phase), (1.0, 1.0));
            assert_eq!(
                handle.lock().expect("effect").state.committed(),
                Some(committed)
            );
            while host
                .try_finish_capture(&mut pending)
                .expect("readback")
                .is_none()
            {
                device
                    .poll(wgpu::PollType::wait_indefinitely())
                    .expect("Metal");
            }
        }
        // The autonomous world is genuinely on its own lane. Observation is the
        // actual world-origin timestamp, while targets follow the runner's schedule.
        let mut host = GameSetup::default()
            .prepare_on(&device, &queue, config)
            .await
            .expect("V1 game");
        let (effect, handle) = DiagnosticEffect::new();
        host.set_game_contribution(Box::new(effect))
            .expect("effect");
        let origin = Instant::now();
        let (mut host, controls) = host.into_threaded(origin, || {}).expect("world lane");
        let mut interpolating = 0;
        for refresh in [120_u64, 60] {
            let start = origin.elapsed() + Duration::from_millis(20);
            for i in 0..24_u64 {
                let due = start + Duration::from_nanos(i * 1_000_000_000 / refresh);
                if let Some(wait) = due.checked_sub(origin.elapsed()) {
                    std::thread::sleep(wait);
                }
                let target = due + Duration::from_millis(30);
                let admission_deadline = Instant::now() + Duration::from_secs(3);
                let (observation, lead) = loop {
                    let observation = origin.elapsed();
                    let lead = match i % 12 {
                        0 => Duration::ZERO,
                        1 => Duration::from_millis(50),
                        _ => target.saturating_sub(observation),
                    };
                    host.set_presentation_lead(lead);
                    let report = host
                        .drive(observation, Some(&surface))
                        .expect("threaded frame");
                    assert!(
                        report.overload.is_none(),
                        "threaded clock refusal: {report:?}"
                    );
                    if report.submitted {
                        assert!(report.surface_presented, "threaded surface: {report:?}");
                        break (observation, lead);
                    }
                    // A threaded drive can omit the optional view while the world
                    // has no matching publication. Wait for admission before GPU completion.
                    controls
                        .check_health()
                        .expect("threaded world after skipped view");
                    assert!(
                        Instant::now() < admission_deadline,
                        "threaded frame was not submitted at {refresh} Hz, step {i}: {report:?}"
                    );
                    std::thread::sleep(Duration::from_millis(1));
                };
                let sample = complete(|| host.poll_completed().expect("completion"), &device);
                identity(&sample);
                let time = handle.lock().expect("effect").time.expect("delivered time");
                assert_eq!(sample.observation, observation);
                assert_eq!(sample.presentation_lead, lead);
                assert_eq!(time.tick, sample.tick);
                assert_eq!(time.alpha, sample.presentation_alpha);
                assert_eq!(time.display_phase, sample.display_phase);
                interpolating +=
                    usize::from(sample.clock_interpolating && sample.display_phase < 4.0);
                records.push(serde_json::json!({"mode":"threaded","refresh":refresh,"tick":time.tick,
                    "scheduled_target_ns":target.as_nanos(),
                    "observation_ns":observation.as_nanos(),"lead_ns":lead.as_nanos(),"phase":time.display_phase,
                    "interpolating":sample.clock_interpolating}));
            }
        }
        assert!(interpolating > 0);
        let output =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tmp/effect-graph-m4");
        std::fs::create_dir_all(&output).expect("output");
        std::fs::write(
            output.join("effect-frames.json"),
            serde_json::to_vec_pretty(&records).expect("JSON"),
        )
        .expect("record");
        // Existing literal state-table oracle also reads GPU contents through
        // repeated ticks, pause, rollback, discard, bounded hitch and replacement.
        let state_record = Box::pin(v1_game::effect::qualification::run(
            &device, &queue, &caps, &instance,
        ))
        .await
        .expect("diagnostic state-table regression");
        std::fs::write(output.join("state-table.txt"), state_record).expect("state record");
    });
}
