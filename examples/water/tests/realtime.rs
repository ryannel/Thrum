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

pub(super) fn capture(directory: &Path) -> Value {
    let (device, queue, caps) = super::support::device();
    let (game, report) =
        water_scene::build_with_report(std::env::var_os(thrum_app::sky::STORE_VARIABLE).is_some())
            .expect("scene");
    let mut runtime =
        Offscreen::new(game, &device, &caps, &queue, [2560, 1440]).expect("ordinary game runtime");
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
                cpu_buffers: 200,
                byte_allowance: 3 * 1024 * 1024 * 1024,
            },
        )
        .expect("tap");
    let output = std::fs::File::create(directory.join("motion.spool.raw")).expect("spool");
    let writer = ImageWriter::start(output, 192, 6 * 1024 * 1024 * 1024, SpoolEncoding::Raw)
        .expect("writer");
    let begin = Instant::now();
    let mut submitted = 0;
    let mut frames = Vec::new();
    let mut completed = BTreeMap::new();
    while begin.elapsed() < Duration::from_secs(3) {
        let elapsed = begin.elapsed().as_secs_f64();
        runtime
            .parameters(&format!(
                r#"{{"camera.bearing":{}}}"#,
                70.0 + elapsed * 12.0
            ))
            .expect("pan control");
        if runtime.step().expect("ordinary submit").submitted {
            submitted += 1;
        }
        drain(&mut runtime, &writer, &mut frames, &mut completed);
        if let Some(wait) = Duration::from_secs_f64((elapsed * 120.0).floor() / 120.0 + 1.0 / 120.0)
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
        drain(&mut runtime, &writer, &mut frames, &mut completed);
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
    json!({"schema":1,"target":"mac-metal","mode":"realtime","commit":std::env::var("THRUM_WATER_COMMIT").unwrap_or_else(|_|"working tree diagnostic".into()),"extent":[2560,1440],"vertices":prepared.vertices,"resources":prepared.resources,"clock":"ordinary Offscreen::step on actual monotonic world time; 120 Hz opportunities, asynchronous in-flight frames","frames":frames,"hashes":written.frame_hashes,"encoding":"Raw","omissions":runtime.omissions(),"capture_limits":{"gpu_staging_slots":6,"cpu_image_leases":200,"pixel_byte_allowance":3_u64*1024*1024*1024,"writer_queue":192,"raw_spool_allowance":6_u64*1024*1024*1024},"writer_seconds":written.write_time.as_secs_f64()})
}
fn drain(
    runtime: &mut Offscreen<SkyGame>,
    writer: &ImageWriter,
    frames: &mut Vec<Value>,
    completed: &mut BTreeMap<u64, FrameSample>,
) {
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
        assert!(writer.submit(image).is_none(), "bounded writer overflow");
    }
    while let Some(image) = writer.recycled() {
        runtime.recycle(image);
    }
}
