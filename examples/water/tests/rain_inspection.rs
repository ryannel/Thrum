//! Metal records from the authored rain yard and its island's open bodies.
use serde_json::{Value, json};
use std::{path::Path, time::Instant};
use thrum_water_model::Map;
use water_scene::island::{Wind, build_rain_bands_cost, build_rain_footfall_cost, build_rain_yard};

use super::support::Fixture;

mod fleet;

fn write(directory: &Path, name: &str, data: &Value) {
    std::fs::write(
        directory.join(name),
        serde_json::to_vec_pretty(data).expect("encode rain record"),
    )
    .expect("write rain record");
}

fn controls(fixture: &mut Fixture, camera: u32, rate: f64, scale: u32) {
    let sun = match camera {
        6 => 38,
        7 => 7,
        _ => 28,
    };
    fixture.parameters(&format!(
        r#"{{"camera.island":{camera},"rain.rate":{rate},"rain.time_scale":{scale},"sun.elevation":{sun}}}"#
    ));
}

fn wet_paving(map: &Map) -> bool {
    super::rain_cost::at_brim(super::rain_cost::levels(map))
}

fn fill(fixture: &mut Fixture, map: &water_scene::SharedMap, camera: u32, rate: f64) -> f64 {
    controls(fixture, camera, rate, 600);
    let mut time = 1.0;
    for _ in 0..90 {
        fixture.frame_without_timings(time);
        if wet_paving(&map.lock().expect("yard map")) {
            return time;
        }
        time += 0.1;
    }
    panic!("paving did not reach its outlet");
}

pub(super) fn costs(gpu: &thrum_test_support::Gpu, directory: &Path) {
    let extent = [2560, 1440];
    let mut arms = Vec::new();
    for (water, rings) in [(false, false), (true, false), (true, true)] {
        let (game, report, map, _) =
            build_rain_bands_cost(Wind::Calm, water, rings).expect("rain cost arm");
        let mut fixture = Fixture::from_game(gpu, extent, game, report);
        controls(&mut fixture, 3, 0.0, 600);
        fixture.warm_cooked();
        controls(&mut fixture, 3, 50.0, 600);
        let mut fills = Vec::new();
        let mut time = 1.0;
        for _ in 0..120 {
            time += 1.0 / 15.0;
            let frame = fixture.frame_without_timings(time);
            let report = fixture.report.lock().expect("water report");
            let regions = report.interaction;
            fills.push(json!({
                "tick": frame.tick, "label_ms": regions.map_or(0.0, |report| report.labelling_ms),
                "regions": regions.map(|report| report.regions),
                "levels": super::rain_cost::levels(&map.lock().expect("yard map")),
            }));
            if wet_paving(&map.lock().expect("yard map")) {
                break;
            }
        }
        assert!(
            wet_paving(&map.lock().expect("yard map")),
            "paving reached the outlet"
        );
        let mut views = Vec::new();
        for (camera, name) in [(3, "yard"), (6, "lake")] {
            controls(&mut fixture, camera, 50.0, 1);
            time += 0.25;
            drop(fixture.frame_without_timings(time));
            let mut frames = Vec::new();
            for _ in 0..12 {
                time += 1.0 / 120.0;
                let sample = fixture.frame(time);
                let mut record = super::sample_json(&sample);
                record["completion_upper_ms"] = json!(fixture.last_elapsed.as_secs_f64() * 1000.0);
                record["upload_cpu_ms"] = json!(sample.upload_cpu.as_secs_f64() * 1000.0);
                frames.push(record);
            }
            views.push(json!({"name": name, "frames": frames}));
        }
        let report = fixture.report.lock().expect("water report");
        arms.push(
            json!({"water":water, "rings": rings, "resources": report.resources, "fills": fills, "views": views}),
        );
    }
    write(
        directory,
        "rain-rings-cost.json",
        &json!({
            "target":"mac-metal", "extent":extent, "capture":false, "rate_mm_h":50, "arms":arms, "fleet":fleet::drains(gpu),
        }),
    );
}

pub(super) fn footfall_costs(gpu: &thrum_test_support::Gpu, directory: &Path) {
    let extent = [2560, 1440];
    let mut arms = Vec::new();
    for fine in [false, true] {
        let (game, report, map, _) =
            build_rain_footfall_cost(Wind::Calm, fine).expect("footfall cost arm");
        let mut fixture = Fixture::from_game(gpu, extent, game, report);
        controls(&mut fixture, 8, 0.0, 600);
        fixture.warm_cooked();
        let start = fill(&mut fixture, &map, 8, 100.0);
        controls(&mut fixture, 8, 10.0, 1);
        for tick in 1..=64 {
            fixture.frame_without_timings(start + f64::from(tick) / 60.0);
        }
        let start = start + 64.0 / 60.0;
        fixture.parameters(r#"{"rain.walk":1}"#);
        let mut frames = Vec::new();
        for sample in 1..=528 {
            let time = start + f64::from(sample) / 120.0;
            // Follow the whole marker path at 60 Hz. Sample the same 24
            // consecutive 120 Hz opportunities in each of the two puddles.
            let measure = (9..33).contains(&sample) || (475..499).contains(&sample);
            if !measure {
                if sample % 2 == 0 {
                    fixture.frame_without_timings(time);
                }
                continue;
            }
            let frame = fixture.frame(time);
            let mut record = super::sample_json(&frame);
            let report = fixture.report.lock().expect("water report");
            let waves = report.interaction.expect("yard interaction");
            let fine = waves.fine.map(|fine| {
                json!({
                    "regions": fine.regions, "solves": fine.solves, "steps": fine.steps,
                    "dropped_ticks": fine.dropped_ticks, "refused_regions": fine.refused_regions,
                })
            });
            record["elapsed_s"] = json!(f64::from(sample) / 120.0);
            record["completion_upper_ms"] = json!(fixture.last_elapsed.as_secs_f64() * 1000.0);
            record["upload_cpu_ms"] = json!(frame.upload_cpu.as_secs_f64() * 1000.0);
            record["labelling_cpu_ms"] = json!(waves.labelling_ms);
            record["fine"] = json!(fine);
            assert!(report.rain_drops.is_some());
            frames.push(record);
        }
        let report = fixture.report.lock().expect("water report");
        arms.push(json!({"fine": fine, "resources": report.resources, "frames": frames}));
    }
    write(
        directory,
        "rain-footfall-cost.json",
        &json!({
            "target":"mac-metal", "extent":extent, "capture":false, "rate_mm_h":10,
            "arms":arms,
        }),
    );
}

pub(super) fn sequences(gpu: &thrum_test_support::Gpu, directory: &Path) {
    let camera = variable("THRUM_RAIN_CAMERA", "4")
        .parse()
        .expect("numeric rain capture setting");
    let rate = variable("THRUM_RAIN_RATE", "50")
        .parse()
        .expect("numeric rain capture setting");
    let fixed = variable("THRUM_RAIN_MODE", "realtime") == "fixed";
    let transition = variable("THRUM_RAIN_MODE", "realtime") == "stop";
    let breeze = variable("THRUM_RAIN_WIND", "calm") == "trade";
    let extent = [1280, 720];
    let (game, report, map, _) =
        build_rain_yard(if breeze { Wind::Trade } else { Wind::Calm }, true)
            .expect("rain inspection scene");
    let mut fixture = Fixture::from_game(gpu, extent, game, report);
    controls(&mut fixture, camera, 0.0, 600);
    fixture.warm_cooked();
    let start = fill(&mut fixture, &map, camera, 100.0);
    controls(&mut fixture, camera, rate, 1);
    fixture.frame_without_timings(start + 1.1);
    let start = start + 1.1;
    let clock = Instant::now();
    let mut records = Vec::new();
    let mut phase = 0;
    let duration = if transition { 4.0 } else { 1.1 };
    loop {
        let index = records.len();
        let elapsed = if fixed {
            index as f64 / 24.0
        } else {
            clock.elapsed().as_secs_f64()
        };
        if elapsed >= duration && index > 0 {
            break;
        }
        if transition && phase == 0 && elapsed >= 0.65 {
            controls(&mut fixture, camera, 0.0, 1);
            phase = 1;
        } else if transition && phase == 1 && elapsed >= 2.65 {
            controls(&mut fixture, camera, rate, 1);
            phase = 2;
        }
        let frame = fixture.frame_without_timings(start + elapsed);
        let file = format!("{index:04}.rgba");
        let image = if fixed {
            fixture
                .host
                .capture_pixels()
                .expect("capture selected world tick")
        } else {
            fixture.image()
        };
        std::fs::write(directory.join(&file), image).expect("submitted RGBA frame");
        let drops = fixture.report.lock().expect("water report").rain_drops;
        records.push(json!({
            "file":file, "elapsed":elapsed, "phase":phase, "tick":frame.tick,
            "alpha":if fixed { 1.0 } else { frame.presentation_alpha }, "drops":drops,
        }));
    }
    write(
        directory,
        "record.json",
        &json!({
            "target":"mac-metal", "extent":extent, "camera":camera, "rate":rate, "breeze":breeze,
            "mode":variable("THRUM_RAIN_MODE","realtime"), "frames":records,
            "commit":variable("THRUM_WATER_COMMIT","working tree"),
        }),
    );
}

fn variable(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.into())
}
