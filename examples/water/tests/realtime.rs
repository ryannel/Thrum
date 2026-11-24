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
const WRITER_QUEUE: usize = 192;
const CPU_LEASES: usize = 200;
const PIXEL_ALLOWANCE: u64 = 3 * 1024 * 1024 * 1024;
const _: () = assert!(MAX_SUBMISSIONS < WRITER_QUEUE);
const _: () = assert!(MAX_SUBMISSIONS < CPU_LEASES);
const _: () = assert!(MAX_SUBMISSIONS as u64 * 2560 * 1440 * 4 < PIXEL_ALLOWANCE);

pub(super) fn capture(gpu: &thrum_test_support::Gpu, directory: &Path) -> Value {
    let (device, queue, caps) = super::support::device(gpu);
    let island = super::island_wind();
    let floating = super::floating();
    let wake = super::wake_route();
    let whitewater = super::whitewater();
    let surf = super::surf_route();
    let route = std::env::var("THRUM_WATER_ROUTE").unwrap_or_default();
    let mut fleet = None;
    let (game, report) = if let Some(floating) = floating {
        let scene = super::floating_game(floating, true, true, whitewater);
        fleet = Some((scene.observer(), scene.mailbox.clone()));
        (scene.game, scene.report)
    } else {
        super::proof_game(true)
    };
    let (extent, interval, opportunities, max_submissions) = if surf {
        ([2560, 1440], 12.0, 10.0, MAX_SUBMISSIONS)
    } else if whitewater && route == "stop" {
        ([1280, 720], 72.0, 10.0, 740)
    } else if whitewater || wake {
        ([1280, 720], 12.0, 10.0, MAX_SUBMISSIONS)
    } else if floating.is_some() {
        ([2560, 1440], 3.7, 30.0, MAX_SUBMISSIONS)
    } else if island.is_some() {
        ([1280, 720], 7.0, 16.0, MAX_SUBMISSIONS)
    } else {
        ([2560, 1440], 1.0, 120.0, MAX_SUBMISSIONS)
    };
    let mut runtime =
        Offscreen::new(game, &device, &caps, &queue, extent).expect("ordinary game runtime");
    // The island's camera index is a meaning only this capture needs, so it has a variable of its own.
    let island_view = std::env::var("THRUM_WATER_ISLAND_CAMERA").unwrap_or_else(|_| "0".into());
    if island.is_some() {
        runtime
            .parameters(&format!(r#"{{"camera.island":{island_view}}}"#))
            .expect("island view");
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
    let (file, encoding) = if floating.is_some() || whitewater {
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
    while begin.elapsed() < Duration::from_secs_f64(interval) && submitted < max_submissions {
        let elapsed = begin.elapsed().as_secs_f64();
        if island.is_none() && floating.is_none() {
            runtime
                .parameters(&format!(
                    r#"{{"camera.bearing":{}}}"#,
                    70.0 + elapsed * 12.0
                ))
                .expect("pan control");
        }
        if surf && route == "tide" {
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
        if runtime.step().expect("ordinary submit").submitted {
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
    let prepared = report.lock().expect("report").clone();
    json!({"schema":1,"target":"mac-metal","mode":"realtime","commit":std::env::var("THRUM_WATER_COMMIT").unwrap_or_else(|_|"working tree diagnostic".into()),"extent":extent,"vertices":prepared.vertices,"resources":prepared.resources,"wind":island.or(floating.map(|f| f.wind)).map(|w| format!("{w:?}")),"island_view":island.map(|_|&island_view),"floating_view":floating.map(|f| f.wake.map_or_else(|| if f.chase {"chase".into()} else {String::new()}, |wake| format!("{:?}", wake.view).to_lowercase())),"craft_speed":floating.map(|f| f.speed),"clock":"ordinary Offscreen::step on actual monotonic world time; asynchronous in-flight frames","opportunities_hz":opportunities,"frames":frames,"hashes":written.frame_hashes,"encoding":format!("{encoding:?}"),"omissions":runtime.omissions(),"capture_limits":{"interval_seconds":interval,"max_submissions":max_submissions,"gpu_staging_slots":6,"cpu_image_leases":CPU_LEASES,"pixel_byte_allowance":PIXEL_ALLOWANCE,"writer_queue":WRITER_QUEUE,"raw_spool_allowance":6_u64*1024*1024*1024},"writer_seconds":written.write_time.as_secs_f64(),"body_tick_poses":poses,"query":fleet.map(|(_,mailbox)|format!("{:?}",mailbox.stats())), "wake_route": (wake || whitewater).then_some(route), "interaction":prepared.interaction.map(|wake| format!("{wake:?}")), "foam":prepared.foam.map(|foam| format!("{foam:?}")), "foam_pattern":prepared.foam_pattern})
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
