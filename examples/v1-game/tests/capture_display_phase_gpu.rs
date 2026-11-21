//! Realtime T2 submitted images from the ordinary V1 offscreen runner.
use crate::support;
#[cfg(target_os = "macos")]
use std::time::{Duration, Instant};
#[cfg(target_os = "macos")]
use thrum_render::frame::TargetLeadStatus;

#[cfg(target_os = "macos")]
fn check_status(
    status: TargetLeadStatus,
    consumed: Duration,
    target: Duration,
    observation: Duration,
    counts: &mut [usize; 6],
) {
    match status {
        TargetLeadStatus::Future => {
            counts[0] += 1;
            // f64 seconds subtraction/Duration conversion may round by one ns.
            assert!(consumed.abs_diff(target) <= Duration::from_nanos(2));
        }
        TargetLeadStatus::Late => {
            counts[1] += 1;
            assert_eq!(consumed, observation);
            assert!(observation >= target);
        }
        TargetLeadStatus::Invalid => counts[2] += 1,
        TargetLeadStatus::Clamped => counts[3] += 1,
    }
}

#[test]
#[cfg(target_os = "macos")]
fn realtime_records_consumed_targets_including_expired_opportunity() {
    use std::sync::{Arc, Mutex};
    use thrum_app::offscreen::{
        Offscreen,
        clock::{OpportunityClock, Timebase},
    };
    let Some(test_gpu) = thrum_test_support::gpu(
        "realtime_records_consumed_targets_including_expired_opportunity",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    pollster::block_on(async {
        let (_, device, queue, caps) = support::device(&test_gpu, "portable", false);
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
        let mut opportunities = Vec::new();
        let mut counts = [0_usize; 6]; // future, late, invalid, clamped, bound, stopped
        let mut previous: Option<(f64, f64, f64)> = None;
        let mut last_origin: Option<f64> = None;
        let mut origin_changes = 0;
        let mut paused = false;
        let mut resumed = false;
        let mut held_seen = false;
        let mut preemption_seen = false;
        let callback_interval = Arc::new(Mutex::new(None));
        for _ in 0..256 {
            if records.len() >= 16 && !paused {
                driver
                    .parameters(r#"{"world.paused":true}"#)
                    .expect("pause control");
                paused = true;
            }
            if records.len() >= 20 && !resumed {
                driver
                    .parameters(r#"{"world.paused":false}"#)
                    .expect("resume control");
                resumed = true;
            }
            let opportunity = clock.next_opportunity();
            let target = opportunity.target.saturating_duration_since(origin);
            let held = records.len() >= 12 && !held_seen;
            let preempt = records.len() >= 8 && !preemption_seen;
            if held {
                // Hold this very opportunity until it expires; don't ask for a new one.
                if let Some(wait) = opportunity.target.checked_duration_since(Instant::now()) {
                    std::thread::sleep(wait + Duration::from_millis(2));
                }
            }
            if preempt {
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
            let preemption = if preempt {
                Some(
                    callback_interval
                        .lock()
                        .expect("callback interval")
                        .take()
                        .expect("callback ran while driving this opportunity"),
                )
            } else {
                None
            };
            opportunities.push(serde_json::json!({"id":opportunity.index,
                "platform_target_ns":target.as_nanos(),"submitted":report.submitted,
                "tick":report.tick,"dropped_view":report.dropped_view,"held":held,
                "render_preemption_ns":preemption.map(|p| p.map(|t| t.as_nanos()))}));
            // The lane may have no matching optional publication on this poll.
            // Keep that opportunity on the record; qualify every submitted frame.
            if !report.submitted {
                continue;
            }
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
            if let Some(interval) = preemption {
                assert!(sample.observation <= interval[0]);
                assert!(origin + interval[1] <= image.submitted_at);
                preemption_seen = true;
            }
            check_status(status, consumed, target, sample.observation, &mut counts);
            if held {
                assert_eq!(status, TargetLeadStatus::Late);
                held_seen = true;
            }
            assert_eq!(sample.presentation_alpha, sample.display_phase.min(1.0));
            let frame_origin = sample
                .tick_due_time
                .map(|due| due.as_secs_f64() - sample.tick as f64 / 60.0);
            if let Some(offset) = frame_origin {
                if let Some(old) = last_origin
                    && (offset - old).abs() > 0.000_002
                {
                    origin_changes += 1;
                }
                last_origin = Some(offset);
                if !paused {
                    assert!(offset.abs() < 0.000_002, "initial span starts at zero");
                }
            }
            if !sample.clock_interpolating {
                assert_eq!(sample.tick_due_time, None);
                counts[5] += 1;
                assert_eq!(sample.display_phase, 1.0);
            } else if sample.display_phase >= 4.0 {
                counts[4] += 1;
            } else {
                let display = (sample.tick as f64 - 1.0 + f64::from(sample.display_phase)) / 60.0;
                let consumed_seconds = consumed.as_secs_f64();
                let offset = frame_origin.expect("interpolating origin");
                let independent = (sample.tick as f64 - 1.0) / 60.0 + consumed_seconds
                    - sample.tick_due_time.expect("due time").as_secs_f64();
                assert!((display - independent).abs() < 0.000_002);
                if let Some((old_display, old_consumed, old_origin)) = previous
                    && (offset - old_origin).abs() < 0.000_002
                {
                    assert!(
                        (display - old_display - (consumed_seconds - old_consumed)).abs()
                            < 0.000_004
                    );
                }
                previous = Some((display, consumed_seconds, offset));
            }
            records.push(serde_json::json!({"opportunity":opportunity.index,"held":held,
                "platform_target_ns":target.as_nanos(),"observation_ns":sample.observation.as_nanos(),
                "presentation_lead_ns":sample.presentation_lead.as_nanos(),"lead_status":format!("{status:?}").to_lowercase(),
                "tick":sample.tick,"display_phase":sample.display_phase,"alpha":sample.presentation_alpha,
                "clock_interpolating":sample.clock_interpolating,"submission":sample.submission_order,
                "tick_due_time_ns":sample.tick_due_time.map(|d| d.as_nanos()),
                "origin_seconds":frame_origin,
                "pixel_bytes":image.rgba.len(),"render_preemption_ns":preemption.map(|p| p.map(|t| t.as_nanos()))}));
            driver.recycle(image);
            if records.len() == 32 {
                break;
            }
        }
        assert_eq!(
            records.len(),
            32,
            "bounded run captured 32 actual submitted frames"
        );
        assert!(
            preemption_seen,
            "one submitted frame included render-owner preemption"
        );
        assert!(held_seen && counts[0] > 0 && counts[1] > 0);
        assert!(
            counts[5] >= 4 && origin_changes >= 1,
            "capture retained stop and origin change"
        );
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
            "counts":{"future":counts[0],"late":counts[1],"invalid":counts[2],"clamped":counts[3],"bound":counts[4],"stopped":counts[5],"origin_changes":origin_changes},
            "opportunities":opportunities,"frames":records})).expect("JSON")).expect("record");
    });
}
