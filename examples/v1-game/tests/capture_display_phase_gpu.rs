//! Realtime T2 submitted images from the ordinary V1 offscreen runner.
pub mod support;
#[cfg(target_os = "macos")]
use std::time::{Duration, Instant};
#[cfg(target_os = "macos")]
use thrum_render::frame::TargetLeadStatus;

#[test]
#[cfg(target_os = "macos")]
fn realtime_records_consumed_targets_including_expired_opportunity() {
    use std::sync::{Arc, Mutex};
    use thrum_app::offscreen::{
        Offscreen,
        clock::{OpportunityClock, Timebase},
    };
    pollster::block_on(async {
        let (_, device, queue, caps) = support::device("portable", false).await;
        let mut driver = Offscreen::new(
            v1_game::capture_game().expect("ordinary V1 game"),
            &device,
            &caps,
            &queue,
            [160, 90],
        )
        .expect("ordinary offscreen driver");
        let origin = driver.origin();
        // Prime shader work before the short observed interval.
        driver.step().expect("prime");
        while driver.pending_frames() != 0 {
            drop(driver.completed().expect("prime complete"));
        }
        driver.enable_capture(4).expect("T2 submitted tap");
        let mut clock =
            OpportunityClock::new(origin, 120, Timebase::Elapsed).expect("120 Hz opportunities");
        let mut records = Vec::new();
        let mut counts = [0_usize; 6]; // future, late, invalid, clamped, bound, stopped
        let mut previous: Option<(f64, f64)> = None;
        let mut held_seen = false;
        let callback_interval = Arc::new(Mutex::new(None));
        for index in 0..32 {
            let opportunity = clock.next_opportunity();
            let target = opportunity.target.saturating_duration_since(origin);
            if index == 12 {
                // Hold this very opportunity until it expires; don't ask for a new one.
                if let Some(wait) = opportunity.target.checked_duration_since(Instant::now()) {
                    std::thread::sleep(wait + Duration::from_millis(2));
                }
            }
            if index == 8 {
                // A real completion callback delays the render owner's poll
                // after step_for reads its observation, while authority runs.
                let interval = Arc::clone(&callback_interval);
                queue.on_submitted_work_done(move || {
                    let start = origin.elapsed();
                    std::thread::sleep(Duration::from_millis(40));
                    *interval.lock().expect("callback interval") = Some([start, origin.elapsed()]);
                });
            }
            let report = driver.step_for(&opportunity).expect("ordinary frame");
            assert!(
                report.submitted,
                "serially drained runner submits each opportunity"
            );
            let deadline = Instant::now() + Duration::from_secs(60);
            let image = loop {
                drop(driver.completed().expect("completion"));
                if let Some(image) = driver.image().expect("submitted image") {
                    break image;
                }
                assert!(Instant::now() < deadline);
                device
                    .poll(wgpu::PollType::wait_indefinitely())
                    .expect("Metal");
            };
            let sample = &image.sample;
            assert_eq!(sample.platform_target, Some(target));
            assert_eq!(sample.presentation_lead, image.presentation_lead);
            let status = sample.lead_status.expect("runner lead status");
            let consumed = sample.observation + sample.presentation_lead;
            let preemption = if index == 8 {
                let interval = callback_interval
                    .lock()
                    .expect("callback interval")
                    .take()
                    .expect("callback ran while driving this frame");
                assert!(sample.observation <= interval[0]);
                assert!(origin + interval[1] <= image.submitted_at);
                Some(interval.map(|time| time.as_nanos()))
            } else {
                None
            };
            match status {
                TargetLeadStatus::Future => {
                    counts[0] += 1;
                    // f64 seconds subtraction/Duration conversion may round by one ns.
                    assert!(consumed.abs_diff(target) <= Duration::from_nanos(2));
                }
                TargetLeadStatus::Late => {
                    counts[1] += 1;
                    assert_eq!(consumed, sample.observation);
                    assert!(sample.observation >= target);
                }
                TargetLeadStatus::Invalid => {
                    counts[2] += 1;
                }
                TargetLeadStatus::Clamped => {
                    counts[3] += 1;
                }
            }
            if index == 12 {
                assert_eq!(status, TargetLeadStatus::Late);
                held_seen = true;
            }
            assert_eq!(sample.presentation_alpha, sample.display_phase.min(1.0));
            if !sample.clock_interpolating {
                counts[5] += 1;
                assert_eq!(sample.display_phase, 1.0);
                previous = None;
            } else if sample.display_phase >= 4.0 {
                counts[4] += 1;
                previous = None;
            } else {
                let display = (sample.tick as f64 - 1.0 + f64::from(sample.display_phase)) / 60.0;
                let consumed_seconds = consumed.as_secs_f64();
                assert!((display - (consumed_seconds - 1.0 / 60.0)).abs() < 0.000_002);
                if let Some((old_display, old_consumed)) = previous {
                    assert!(
                        (display - old_display - (consumed_seconds - old_consumed)).abs()
                            < 0.000_004
                    );
                }
                previous = Some((display, consumed_seconds));
            }
            records.push(serde_json::json!({"opportunity":opportunity.index,"held":index==12,
                "platform_target_ns":target.as_nanos(),"observation_ns":sample.observation.as_nanos(),
                "presentation_lead_ns":sample.presentation_lead.as_nanos(),"lead_status":format!("{status:?}").to_lowercase(),
                "tick":sample.tick,"display_phase":sample.display_phase,"alpha":sample.presentation_alpha,
                "clock_interpolating":sample.clock_interpolating,"submission":sample.submission_order,
                "pixel_bytes":image.rgba.len(),"render_preemption_ns":preemption}));
            driver.recycle(image);
        }
        assert!(held_seen && counts[0] > 0 && counts[1] > 0);
        assert!(
            records.len() > counts[4] + counts[5],
            "running interpolating world was observed"
        );
        assert_eq!(driver.omissions(), 0);
        let output =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tmp/effect-graph-m4");
        std::fs::create_dir_all(&output).expect("output");
        std::fs::write(output.join("realtime-record.json"), serde_json::to_vec_pretty(&serde_json::json!({
            "mode":"realtime","opportunity_hz":120,"precision_ns":2000,
            "counts":{"future":counts[0],"late":counts[1],"invalid":counts[2],"clamped":counts[3],"bound":counts[4],"stopped":counts[5]},
            "frames":records})).expect("JSON")).expect("record");
    });
}
