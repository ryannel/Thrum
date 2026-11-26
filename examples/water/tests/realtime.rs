use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::Path,
    time::{Duration, Instant},
};
use thrum_app::{
    offscreen::{
        Offscreen,
        writer::{ImageWriter, SpoolEncoding},
    },
    sky::SkyGame,
};
use thrum_render::runtime::{CaptureLimits, FrameSample};

// Even a stalled writer can retain this complete interval in its finite queue.
// Keep ordinary elapsed-time stepping and pan speed; bound the observer, not time.
const MAX_SUBMISSIONS: usize = 120;
const CHANNEL_SUBMISSIONS: usize = 180;
const WRITER_QUEUE: usize = 192;
const CPU_LEASES: usize = 200;
const PIXEL_ALLOWANCE: u64 = 3 * 1024 * 1024 * 1024;
const _: () = assert!(MAX_SUBMISSIONS < WRITER_QUEUE);
const _: () = assert!(MAX_SUBMISSIONS < CPU_LEASES);
const _: () = assert!(CHANNEL_SUBMISSIONS < WRITER_QUEUE);
const _: () = assert!(CHANNEL_SUBMISSIONS < CPU_LEASES);
const _: () = assert!(CHANNEL_SUBMISSIONS as u64 * 1280 * 720 * 4 < PIXEL_ALLOWANCE);
const _: () = assert!(MAX_SUBMISSIONS as u64 * 2560 * 1440 * 4 < PIXEL_ALLOWANCE);

#[expect(
    clippy::too_many_lines,
    reason = "one timeline coordinates submitted frames, stage edits and the capture spool"
)]
pub(super) fn capture(gpu: &thrum_test_support::Gpu, directory: &Path) -> Value {
    let (device, queue, caps) = super::support::device(gpu);
    let island = super::island_wind();
    let yard = matches!(super::settings(), super::Settings::RainYard(_));
    let channel = super::channel_route();
    let channel_view = std::env::var("THRUM_WATER_CHANNEL_CAMERA").unwrap_or_else(|_| "2".into());
    let floating = super::floating();
    let wake = super::wake_route();
    let whitewater = super::whitewater();
    let surf = super::surf_route();
    let route = std::env::var("THRUM_WATER_ROUTE").unwrap_or_default();
    let optics_height = std::env::var("THRUM_WATER_OPTICS_HEIGHT").unwrap_or_else(|_| "2".into());
    let late_channel = channel && route == "late";
    let handoff_cut = surf && route == "handoff-cut";
    let island_view = std::env::var("THRUM_WATER_ISLAND_CAMERA").unwrap_or_else(|_| "0".into());
    if handoff_cut {
        assert_eq!(island_view, "7", "start at the foam handoff camera");
    }
    let mut fleet = None;
    let mut yard_map = None;
    let (game, report) = if let Some(floating) = floating {
        let scene = super::floating_game(floating, true, true, whitewater);
        fleet = Some((scene.observer(), scene.mailbox.clone()));
        (scene.game, scene.report)
    } else if let super::Settings::RainYard(wind) = super::settings() {
        let (game, report, map, _) =
            water_scene::island::build_rain_yard(wind, true).expect("rain yard");
        yard_map = Some(map);
        (game, report)
    } else {
        super::proof_game(true)
    };
    let (extent, interval, opportunities, max_submissions) = if yard {
        ([1280, 720], 20.0, 9.0, CHANNEL_SUBMISSIONS)
    } else if surf && island_view == "2" {
        // A complete set of unequal breaks takes longer than a nearshore take.
        ([2560, 1440], 22.0, 5.0, MAX_SUBMISSIONS)
    } else if surf && island_view == "9" {
        ([2560, 1440], 18.0, 6.5, MAX_SUBMISSIONS)
    } else if channel {
        ([1280, 720], 9.0, 20.0, CHANNEL_SUBMISSIONS)
    } else if surf {
        ([2560, 1440], 12.0, 10.0, MAX_SUBMISSIONS)
    } else if whitewater && route == "stop" {
        ([1280, 720], 72.0, 10.0, 740)
    } else if whitewater || wake {
        ([1280, 720], 12.0, 10.0, MAX_SUBMISSIONS)
    } else if floating.is_some() {
        ([2560, 1440], 3.7, 30.0, MAX_SUBMISSIONS)
    } else if island.is_some() {
        // Inspect the lake reflection and its moving silhouettes at the stills' native size.
        let extent = if matches!(route.as_str(), "optics-pan" | "optics-breeze") {
            [2560, 1440]
        } else {
            [1280, 720]
        };
        (extent, 7.0, 16.0, MAX_SUBMISSIONS)
    } else {
        ([2560, 1440], 1.0, 120.0, MAX_SUBMISSIONS)
    };
    let mut runtime =
        Offscreen::new(game, &device, &caps, &queue, extent).expect("ordinary game runtime");
    if yard {
        runtime
            .parameters(&format!(
                r#"{{"camera.island":{island_view},"sun.elevation":8,"rain.rate":0}}"#
            ))
            .expect("initial dry yard");
    }
    // The island's camera index is a meaning only this capture needs, so it has a variable of its own.
    if island.is_some() {
        runtime
            .parameters(&format!(r#"{{"camera.island":{island_view}}}"#))
            .expect("island view");
    }
    if channel {
        runtime
            .parameters(&format!(
                r#"{{"camera.channel":{channel_view},"sea.stage":{}}}"#,
                if matches!(route.as_str(), "tide" | "low-tide") {
                    -0.5
                } else {
                    0.0
                }
            ))
            .expect("channel view");
    }
    for _ in 0..8 {
        runtime.step().expect("warm submit");
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(30)),
            })
            .expect("warm poll");
        while runtime.completed().expect("warm completion").is_some() {}
    }
    runtime
        .enable_capture_with_limits(
            3,
            CaptureLimits {
                staging_slots: 6,
                cpu_buffers: CPU_LEASES,
                byte_allowance: PIXEL_ALLOWANCE,
            },
        )
        .expect("tap");
    let (file, encoding) = if floating.is_some() || whitewater || channel || yard {
        ("motion.spool.zst", SpoolEncoding::ZstdDelta)
    } else {
        ("motion.spool.raw", SpoolEncoding::Raw)
    };
    let output = std::fs::File::create(directory.join(file)).expect("spool");
    let writer =
        ImageWriter::start(output, WRITER_QUEUE, 6 * 1024 * 1024 * 1024, encoding).expect("writer");
    let begin = Instant::now();
    let mut submitted = 0;
    let mut frames = Vec::new();
    let mut completed = BTreeMap::new();
    let mut poses = Vec::new();
    let mut applied = "";
    let mut cut = None;
    let mut weather_edits = Vec::new();
    let mut yard_levels = Vec::new();
    while begin.elapsed() < Duration::from_secs_f64(interval) && submitted < max_submissions {
        let elapsed = begin.elapsed().as_secs_f64();
        if yard {
            let change = match elapsed {
                t if t >= 7.8 => r#"{"rain.rate":0,"rain.drying":100,"rain.time_scale":600}"#,
                t if t >= 3.9 => r#"{"rain.rate":100,"rain.drying":0,"rain.time_scale":600}"#,
                t if t >= 1.10 => r#"{"rain.rate":100,"rain.drying":0,"rain.time_scale":150}"#,
                t if t >= 0.25 => r#"{"rain.rate":100,"rain.drying":0,"rain.time_scale":15}"#,
                _ => "",
            };
            if !change.is_empty() && change != applied {
                runtime.parameters(change).expect("yard weather controls");
                weather_edits.push(json!({"elapsed":elapsed, "parameters":serde_json::from_str::<Value>(change).expect("weather")}));
                applied = change;
            }
        }
        if !yard && !channel && island.is_none() && floating.is_none() {
            runtime
                .parameters(&format!(
                    r#"{{"camera.bearing":{}}}"#,
                    70.0 + elapsed * 12.0
                ))
                .expect("pan control");
        }
        if channel && route == "tide" {
            let height = ((elapsed - 1.0) / 6.0).clamp(0.0, 1.0);
            runtime
                .parameters(&format!(
                    r#"{{"lake.stage":{height:.3},"sea.stage":{:.3}}}"#,
                    height - 0.5
                ))
                .expect("channel stages");
        }
        if route == "optics-pan" {
            if floating.is_some() {
                let yaw = -56.0 + elapsed * 30.0;
                runtime.parameters(&format!(r#"{{"camera.optics_height":{optics_height},"camera.optics_pan":{yaw},"sun.elevation":6}}"#))
                    .expect("fleet pan");
            } else if island.is_some() {
                let yaw = elapsed * 14.0;
                let tilt = elapsed * 4.0;
                runtime
                    .parameters(&format!(
                        r#"{{"camera.pan":{yaw},"camera.tilt":{tilt},"sun.elevation":12}}"#
                    ))
                    .expect("lake pan");
            }
        } else if route == "optics-breeze" {
            let gain = ((elapsed - 1.0) / 4.0).clamp(0.0, 1.0);
            runtime
                .parameters(&format!(r#"{{"lake.waves":{gain}}}"#))
                .expect("breeze");
        }
        if handoff_cut && elapsed >= 5.0 && cut.is_none() {
            runtime
                .parameters(r#"{"camera.island":8}"#)
                .expect("approach view");
            cut = Some(
                json!({"from":7,"to":8,"driver_elapsed":elapsed,"next_capture_index":submitted}),
            );
        } else if surf && route == "tide" {
            let stage = if elapsed <= 6.0 {
                elapsed / 6.0 - 0.5
            } else {
                1.5 - elapsed / 6.0
            };
            runtime
                .parameters(&format!(r#"{{"sea.stage":{:.3}}}"#, stage.clamp(-0.5, 0.5)))
                .expect("tide control");
        } else if wake || whitewater {
            let change = match (route.as_str(), elapsed) {
                ("turn", t) if t >= 4.0 => r#"{"craft.turn":65}"#,
                ("restart", t) if t >= 8.0 => r#"{"craft.speed":6}"#,
                ("restart", t) if t >= 4.0 => r#"{"craft.speed":0}"#,
                ("stop", t) if t >= 7.0 => r#"{"craft.speed":0}"#,
                _ => "",
            };
            if !change.is_empty() && change != applied {
                runtime.parameters(change).expect("wake route controls");
                applied = change;
            }
        }
        let submitted_frame = if late_channel {
            runtime.step_at(Duration::from_secs_f64(60.0 + elapsed))
        } else {
            runtime.step()
        };
        if submitted_frame.expect("ordinary submit").submitted {
            if let Some(map) = &yard_map {
                let map = map.lock().expect("yard map");
                yard_levels.push(super::rain_cost::levels(&map));
            }
            if let Some((observe, _)) = &fleet {
                poses.push(observe().into_iter().map(|body| json!({
                    "id": body.id, "position": body.position.to_array(),
                    "rotation": body.rotation.to_array(), "velocity": body.velocity.to_array(),
                })).collect::<Vec<_>>());
            }
            submitted += 1;
        }
        assert!(
            drain(&mut runtime, &writer, &mut frames, &mut completed).is_ok(),
            "writer rejected a frame; underlying outcome: {:?}",
            writer.finish(Duration::from_secs(60))
        );
        if let Some(wait) = Duration::from_secs_f64(
            (elapsed * opportunities).floor() / opportunities + 1.0 / opportunities,
        )
        .checked_sub(begin.elapsed())
        {
            std::thread::sleep(wait);
        }
    }
    let drained = thrum_test_support::wait::until("capture drain", Duration::from_secs(30), || {
        if runtime.pending_frames() == 0 && frames.len() >= submitted {
            return Some(true);
        }
        device.poll(wgpu::PollType::Poll).expect("final poll");
        // A rejected frame ends the wait; the writer's outcome is reported below.
        drain(&mut runtime, &writer, &mut frames, &mut completed)
            .is_err()
            .then_some(false)
    });
    assert!(
        drained,
        "writer rejected a frame; underlying outcome: {:?}",
        writer.finish(Duration::from_secs(60))
    );
    for frame in &mut frames {
        let serial = frame["sample"]["submission"].as_u64().expect("serial");
        frame["sample"] =
            super::sample_json(completed.get(&serial).expect("matched GPU completion"));
    }
    let written = writer
        .finish(Duration::from_secs(60))
        .expect("lossless writer");
    assert_eq!(written.frames, frames.len() as u64);
    assert_eq!(runtime.omissions(), 0, "complete submitted interval");
    if yard {
        assert_eq!(
            yard_levels.len(),
            frames.len(),
            "one level pair per submitted frame"
        );
        let joined = |levels: &[Option<f64>; 2]| matches!(levels, [Some(a), Some(b)] if a.to_bits() == b.to_bits());
        assert_eq!(yard_levels.first(), Some(&[None; 2]), "the yard starts dry");
        assert_eq!(yard_levels.last(), Some(&[None; 2]), "the yard dries out");
        assert!(
            yard_levels.iter().copied().any(super::rain_cost::at_brim),
            "the paving reaches its outlet before drying"
        );
        let merge = yard_levels
            .iter()
            .position(joined)
            .expect("observed a joined puddle");
        assert!(
            yard_levels[..merge]
                .iter()
                .any(|row| matches!(row, [Some(lower), Some(upper)] if lower > upper)),
            "the lower hollow spills into the upper hollow before joining"
        );
        assert!(
            yard_levels[merge + 1..]
                .iter()
                .any(|row| row.iter().all(Option::is_some) && !joined(row)),
            "observed a parting"
        );
    }
    let prepared = report.lock().expect("report").clone();
    assert!(
        !handoff_cut || cut.is_some(),
        "capture reached the camera change"
    );
    json!({"schema":1,"target":"mac-metal","mode":"realtime","commit":std::env::var("THRUM_WATER_COMMIT").unwrap_or_else(|_|"working tree diagnostic".into()),"extent":extent,"vertices":prepared.vertices,"resources":prepared.resources,"wind":island.or(floating.map(|f| f.wind)).map(|w| format!("{w:?}")),"island_view":(yard || island.is_some()).then_some(&island_view),"weather_edits":weather_edits,"yard_levels":yard_levels,"camera_cut":cut,"channel_view":channel.then_some(channel_view),"channel_route":channel.then_some(&route),"floating_view":floating.map(|f| f.wake.map_or_else(|| if f.chase {"chase".into()} else {String::new()}, |wake| format!("{:?}", wake.view).to_lowercase())),"craft_speed":floating.map(|f| f.speed),"clock":if late_channel {"ordinary submits at 60 seconds plus actual elapsed time"} else {"ordinary Offscreen::step on actual monotonic world time; asynchronous in-flight frames"},"opportunities_hz":opportunities,"frames":frames,"hashes":written.frame_hashes,"encoding":format!("{encoding:?}"),"omissions":runtime.omissions(),"capture_limits":{"interval_seconds":interval,"max_submissions":max_submissions,"gpu_staging_slots":6,"cpu_image_leases":CPU_LEASES,"pixel_byte_allowance":PIXEL_ALLOWANCE,"writer_queue":WRITER_QUEUE,"raw_spool_allowance":6_u64*1024*1024*1024},"writer_seconds":written.write_time.as_secs_f64(),"body_tick_poses":poses,"query":fleet.map(|(_,mailbox)|format!("{:?}",mailbox.stats())), "wake_route": (wake || whitewater).then_some(route), "interaction":prepared.interaction.map(|wake| format!("{wake:?}")), "foam":prepared.foam.map(|foam| format!("{foam:?}")), "foam_pattern":prepared.foam_pattern})
}
fn drain(
    runtime: &mut Offscreen<SkyGame>,
    writer: &ImageWriter,
    frames: &mut Vec<Value>,
    completed: &mut BTreeMap<u64, FrameSample>,
) -> Result<(), ()> {
    while let Some(sample) = runtime.completed().expect("completed") {
        completed.insert(sample.submission_order, sample);
    }
    while let Some(image) = runtime.image().expect("submitted image") {
        if let Some(previous) = frames.last() {
            assert!(
                previous["sample"]["submission"].as_u64().expect("serial")
                    < image.sample.submission_order,
                "ordered submitted sources"
            );
        }
        frames.push(json!({"elapsed":image.submitted_elapsed.as_secs_f64(),"sample":super::sample_json(&image.sample),"submitted_elapsed":image.submitted_elapsed.as_secs_f64(),"exposure_words":image.exposure_gpu_words}));
        if let Some(image) = writer.submit(image) {
            runtime.recycle(image);
            return Err(());
        }
    }
    while let Some(image) = writer.recycled() {
        runtime.recycle(image);
    }
    Ok(())
}
