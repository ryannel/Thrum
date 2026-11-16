//! Opt-in submitted-output capture and paired GPU measurement driver.
mod realtime;
mod support;
use serde_json::json;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use thrum_render::runtime::{CaptureLimits, FrameSample, SubmittedImage};
#[test]
fn proof_driver() {
    if std::env::var_os("THRUM_WATER_PROOF").is_none() {
        return;
    }
    driver();
}
fn driver() {
    let mode = std::env::var("THRUM_WATER_PROOF").expect("mode");
    let directory = PathBuf::from(std::env::var("THRUM_WATER_OUTPUT").expect("output"));
    std::fs::create_dir_all(&directory).expect("output folder");
    if mode == "realtime" {
        let result = realtime::capture(&directory);
        std::fs::write(
            directory.join("realtime.json"),
            serde_json::to_vec_pretty(&result).expect("json"),
        )
        .expect("record");
        return;
    }
    let mut fixture = support::Fixture::new([2560, 1440]);
    for index in 0..8 {
        fixture.frame(f64::from(index) / 120.0);
    }
    let report = fixture.report.lock().expect("report").clone();
    let mut result = json!({"schema":1,"target":"mac-metal","mode":mode,"extent":fixture.extent,"vertices":report.vertices,"resources":report.resources});
    result["commit"] = json!(
        std::env::var("THRUM_WATER_COMMIT").unwrap_or_else(|_| "working tree diagnostic".into())
    );
    match mode.as_str() {
        "cost" => {
            let mut arms = Vec::new();
            for present in [true, false] {
                let mut fixture = support::Fixture::paired([2560, 1440], present);
                for index in 0..8 {
                    fixture.frame(f64::from(index) / 120.0);
                }
                let mut frames = Vec::new();
                for index in 0..32 {
                    let start = 1.0;
                    let sample = fixture.frame(start + f64::from(index) / 120.0);
                    if index >= 8 {
                        let mut record = sample_json(&sample);
                        record["completion_upper_ms"] =
                            json!(fixture.last_elapsed.as_secs_f64() * 1000.0);
                        frames.push(record);
                    }
                }
                let pixels = if present {
                    fixture
                        .witness()
                        .iter()
                        .filter(|p| p[0] > 0.1 && p[1] > 0.0)
                        .count()
                } else {
                    0
                };
                arms.push(json!({"water":present,"calibration_float":true,"frames":frames,"water_pixels_shaded":pixels,"pixel_count_method":"visible coverage from submitted reactive/depth witness; helper and hidden invocations excluded","storage":fixture.host.storage_report()}));
            }
            result["capture"] = json!(false);
            result["arms"] = json!(arms);
        }
        "fixed" => {
            fixture
                .host
                .enable_submitted_capture_with_limits(2, limits())
                .expect("tap");
            let mut images = Vec::new();
            for (name, height, bearing, sun) in [
                ("noon-2-along", 2, 90, 60),
                ("noon-2-across", 2, 0, 60),
                ("noon-40-along", 40, 90, 60),
                ("noon-40-across", 40, 0, 60),
                ("low-sun", 2, 90, 6),
                ("twilight", 2, 90, -4),
            ] {
                fixture.parameters(&format!(r#"{{"camera.height":{height},"camera.bearing":{bearing},"sun.elevation":{sun}}}"#));
                let sample = fixture.frame(10.0);
                let image = take_image(&mut fixture);
                let file = format!("{name}.rgba");
                std::fs::write(directory.join(&file), &image.rgba).expect("fixed image");
                images.push(json!({"name":name,"file":file,"sample":sample_json(&sample),"exposure_words":image.exposure_gpu_words,"submitted_elapsed":image.submitted_elapsed.as_secs_f64()}));
                fixture.host.recycle_submitted_image(image);
            }
            result["images"] = json!(images);
        }
        _ => panic!("unknown proof mode"),
    }
    std::fs::write(
        directory.join(format!("{mode}.json")),
        serde_json::to_vec_pretty(&result).expect("json"),
    )
    .expect("record");
}
fn limits() -> CaptureLimits {
    CaptureLimits {
        staging_slots: 3,
        cpu_buffers: 16,
        byte_allowance: 300 * 1024 * 1024,
    }
}
fn take_image(fixture: &mut support::Fixture) -> SubmittedImage {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        fixture
            .device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("tap poll");
        if let Some(image) = fixture.host.try_submitted_image().expect("image") {
            return image;
        }
        assert!(Instant::now() < deadline, "tap timeout");
        std::thread::yield_now();
    }
}
fn sample_json(sample: &FrameSample) -> serde_json::Value {
    let Some(gpu) = sample.gpu_pass_ns.as_ref() else {
        return json!({"tick":sample.tick,"alpha":sample.presentation_alpha,"tick_frame":sample.completed_ticks>0,"submission":sample.submission_order,"gpu_ms":null,"passes_ms":{},"timing_fault":sample.timing_fault,"timing_requested":sample.timing_requested,"cpu_wall_ms":sample.wall.as_secs_f64()*1000.0});
    };
    let mut passes: std::collections::BTreeMap<_, _> = gpu
        .passes()
        .iter()
        .map(|p| (p.name.to_string(), p.nanoseconds / 1e6))
        .collect();
    if let Some(present) = sample.gpu_present_ns {
        passes.insert(thrum_render::present::PASS_NAME.into(), present / 1e6);
    }
    json!({"tick":sample.tick,"alpha":sample.presentation_alpha,"tick_frame":sample.completed_ticks>0,"submission":sample.submission_order,"gpu_ms":(gpu.sum()+sample.gpu_present_ns.unwrap_or(0.0))/1e6,"scene_gpu_ms":gpu.sum()/1e6,"present_gpu_ms":sample.gpu_present_ns.map(|v|v/1e6),"gpu_scope":"timed scene and surface-present pass sum; untimed copies and query readback are not imputed as zero","cpu_wall_ms":sample.wall.as_secs_f64()*1000.0,"passes_ms":passes})
}
