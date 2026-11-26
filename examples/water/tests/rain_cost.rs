//! Capture-off, same-view paired yard costs at three fills.
use serde_json::{Value, json};
use std::time::Duration;
use thrum_test_support::Gpu;
use water_scene::island::{Wind, build_rain_yard};

const EXTENT: [u32; 2] = [2560, 1440];

pub(super) fn measure(gpu: &Gpu) -> Value {
    let mut fills: Vec<_> = ["first", "separate", "brim"]
        .map(|name| json!({"name": name, "arms": []}))
        .into_iter()
        .collect();
    for present in [false, true] {
        let (game, report, map, _) = build_rain_yard(Wind::Calm, present).expect("yard");
        let mut fixture = super::support::Fixture::from_game(gpu, EXTENT, game, report);
        fixture.parameters(r#"{"camera.island":3,"rain.rate":0}"#);
        fixture.warm_cooked();
        fixture.frame(1.0);
        fixture.parameters(r#"{"rain.rate":100,"rain.time_scale":600}"#);
        for (index, fill) in (0_u32..).zip(&mut fills) {
            let time = [1.2, 1.48, 2.05][index as usize];
            fixture.frame(time);
            // Freeze this fill. Simulated and repeated draws share the same
            // puddle footprint and tick endpoints for both arms.
            fixture.parameters(r#"{"rain.rate":0}"#);
            let frames: Vec<_> = (1..=12)
                .map(|step| {
                    let sample = fixture.frame(time + f64::from(step) / 120.0);
                    let mut row = super::sample_json(&sample);
                    row["completion_upper_ms"] = json!(fixture.last_elapsed.as_secs_f64() * 1000.0);
                    row
                })
                .collect();
            let pixels = fixture
                .witness()
                .iter()
                .filter(|p| p[0] > 0.1 && p[1] > 0.0)
                .count();
            let report = fixture.report.lock().expect("yard water report");
            let count = |name: &str| {
                report
                    .resources
                    .iter()
                    .find_map(|(label, size)| (label == name).then_some(*size))
                    .unwrap_or(0)
            };
            let vertices = report
                .bodies
                .iter()
                .find_map(|(name, vertices)| (name == "Rain yard").then_some(*vertices))
                .unwrap_or(0);
            let map = map.lock().expect("yard map");
            let levels = [[147.2, 66.2], [149.75, 66.35]].map(|p| {
                map.sample(p)
                    .filter(|hit| hit.depth > 0.0)
                    .map(|hit| hit.row.level)
            });
            fill["arms"].as_array_mut().expect("paired arms").push(json!({
                "water": present, "frames": frames, "reactive_pixels": pixels, "vertices": vertices,
                "hollows_bytes": count("water hollows"), "levels": levels,
            }));
            drop(map);
            drop(report);
            fixture.parameters(r#"{"rain.rate":100}"#);
        }
    }
    json!({
        "target": "mac-metal", "extent": EXTENT, "capture": false, "fills": fills,
        "frame_stride_ns": Duration::from_secs_f64(1.0 / 120.0).as_nanos(),
    })
}
