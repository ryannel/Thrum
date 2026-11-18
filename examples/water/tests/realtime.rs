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

pub(super) fn capture(directory: &Path) -> Value {
    let (device, queue, caps) = super::support::device();
    let (game, report) = super::proof_game(true);
    let island = super::island_wind();
    let (extent, interval, opportunities) = if island.is_some() {
        ([1280, 720], 7.0, 16.0)
    } else {
        ([2560, 1440], 1.0, 120.0)
    };
    let mut runtime =
        Offscreen::new(game, &device, &caps, &queue, extent).expect("ordinary game runtime");
    let island_view = std::env::var("THRUM_WATER_VIEW").unwrap_or_else(|_| "0".into());
    if island.is_some() {
        runtime
            .parameters(&format!(r#"{{"camera.island":{island_view}}}"#))
            .expect("island view");
    }
    for _ in 0..8 {
        runtime.step().expect("warm submit");
        device
            .poll(wgpu::PollType::wait_indefinitely())
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
    let output = std::fs::File::create(directory.join("motion.spool.raw")).expect("spool");
    let writer = ImageWriter::start(
        output,
        WRITER_QUEUE,
        6 * 1024 * 1024 * 1024,
        SpoolEncoding::Raw,
    )
    .expect("writer");
    let begin = Instant::now();
    let mut submitted = 0;
    let mut frames = Vec::new();
    let mut completed = BTreeMap::new();
    while begin.elapsed() < Duration::from_secs_f64(interval) && submitted < MAX_SUBMISSIONS {
        let elapsed = begin.elapsed().as_secs_f64();
        if island.is_none() {
            runtime
                .parameters(&format!(
                    r#"{{"camera.bearing":{}}}"#,
                    70.0 + elapsed * 12.0
                ))
                .expect("pan control");
        }
        if runtime.step().expect("ordinary submit").submitted {
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
    let deadline = Instant::now() + Duration::from_secs(30);
    while runtime.pending_frames() > 0 || frames.len() < submitted {
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("final poll");
        assert!(
            drain(&mut runtime, &writer, &mut frames, &mut completed).is_ok(),
            "writer rejected a frame; underlying outcome: {:?}",
            writer.finish(Duration::from_secs(60))
        );
        assert!(Instant::now() < deadline, "capture drain timeout");
    }
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
    json!({"schema":1,"target":"mac-metal","mode":"realtime","commit":std::env::var("THRUM_WATER_COMMIT").unwrap_or_else(|_|"working tree diagnostic".into()),"extent":extent,"vertices":prepared.vertices,"resources":prepared.resources,"wind":island.map(|w| format!("{w:?}")),"island_view":island.map(|_|island_view),"clock":"ordinary Offscreen::step on actual monotonic world time; asynchronous in-flight frames","opportunities_hz":opportunities,"frames":frames,"hashes":written.frame_hashes,"encoding":"Raw","omissions":runtime.omissions(),"capture_limits":{"interval_seconds":interval,"max_submissions":MAX_SUBMISSIONS,"gpu_staging_slots":6,"cpu_image_leases":CPU_LEASES,"pixel_byte_allowance":PIXEL_ALLOWANCE,"writer_queue":WRITER_QUEUE,"raw_spool_allowance":6_u64*1024*1024*1024},"writer_seconds":written.write_time.as_secs_f64()})
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
