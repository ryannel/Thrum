//! Submitted V1 effect times, independently checked against consumed targets.
pub mod support;
use std::time::{Duration, Instant};
use v1_game::{FrameConfig, FrameHost, FrameSample, effect::DiagnosticEffect};

fn complete(host: &mut FrameHost, device: &wgpu::Device) -> FrameSample {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        if let Some(sample) = host.poll_completed().expect("completion") {
            return sample;
        }
        assert!(Instant::now() < deadline);
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("Metal");
    }
}

fn identity(sample: &FrameSample) {
    assert_eq!(
        sample.presentation_alpha.to_bits(),
        sample.display_phase.min(1.0).to_bits()
    );
    if sample.clock_interpolating && sample.display_phase < 4.0 {
        // Tick coordinates are exact 1/60; f32 phase costs < 0.01 us here.
        // Allow 2 us, including whole-nanosecond clock target arithmetic.
        let display = (sample.tick as f64 - 1.0 + f64::from(sample.display_phase)) / 60.0;
        let expected = (sample.observation + sample.presentation_lead).as_secs_f64() - 1.0 / 60.0;
        assert!(
            (display - expected).abs() < 0.000_002,
            "{sample:?}: {display} != {expected}"
        );
    } else if !sample.clock_interpolating {
        assert_eq!(sample.display_phase, 1.0);
    }
}

#[test]
#[cfg(target_os = "macos")]
fn submitted_effects_consume_unclamped_clock_phase_inline_and_threaded() {
    pollster::block_on(async {
        let (_, device, queue, caps) = support::device("portable", false).await;
        let (view, profile) = support::presentation(&device, [96, 54]);
        let surface = v1_game::SurfaceOutput {
            view: &view,
            extent: [96, 54],
        };
        let mut records = Vec::new();
        for refresh in [120_u64, 60] {
            let mut host = FrameHost::new(
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
            host.set_scene_contribution(Box::new(effect))
                .expect("effect seam");
            let mut previous: Option<(f64, f64)> = None;
            let mut past_endpoint = 0;
            for lead_ms in [0, 10, 20, 30, 40, 50] {
                let lead = Duration::from_millis(lead_ms);
                for i in 0..8_u64 {
                    let index = lead_ms / 10 * 8 + i;
                    let observation = Duration::from_nanos(index * 1_000_000_000 / refresh);
                    host.set_presentation_lead(lead);
                    host.drive(observation, Some(&surface))
                        .expect("scripted frame");
                    let sample = complete(&mut host, &device);
                    identity(&sample);
                    let time = handle.lock().expect("effect").time.expect("delivered time");
                    assert_eq!(time.tick, sample.tick);
                    assert_eq!(time.alpha, sample.presentation_alpha);
                    assert_eq!(time.display_phase, sample.display_phase);
                    assert!(!time.capture);
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
            host.pause().expect("pause inline world");
            host.drive(Duration::from_secs(1), Some(&surface))
                .expect("paused");
            let paused = complete(&mut host, &device);
            identity(&paused);
            assert!(!paused.clock_interpolating);
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
        let mut host = FrameHost::new(
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
        host.set_scene_contribution(Box::new(effect))
            .expect("effect");
        let origin = Instant::now();
        let (mut host, _) = host.into_threaded(origin, || {}).expect("world lane");
        let mut interpolating = 0;
        for refresh in [120_u64, 60] {
            let start = origin.elapsed() + Duration::from_millis(20);
            for i in 0..24_u64 {
                let due = start + Duration::from_nanos(i * 1_000_000_000 / refresh);
                if let Some(wait) = due.checked_sub(origin.elapsed()) {
                    std::thread::sleep(wait);
                }
                let observation = origin.elapsed();
                let target = due + Duration::from_millis(30);
                let lead = target.saturating_sub(observation);
                host.set_presentation_lead(lead);
                host.drive(observation, Some(&surface))
                    .expect("threaded frame");
                let deadline = Instant::now() + Duration::from_secs(60);
                let sample = loop {
                    if let Some(s) = host.poll_completed().expect("complete") {
                        break s;
                    }
                    assert!(Instant::now() < deadline);
                    device
                        .poll(wgpu::PollType::wait_indefinitely())
                        .expect("Metal");
                };
                identity(&sample);
                let time = handle.lock().expect("effect").time.expect("delivered time");
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
        Box::pin(v1_game::effect::qualification::run(
            &device,
            &queue,
            &caps,
            &wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle()),
        ))
        .await
        .expect("diagnostic state-table regression");
    });
}
