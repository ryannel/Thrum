//! Submitted fixed checkpoints and paired timings for the rain ground.
use serde_json::{Value, json};
use std::path::Path;
use thrum_render::runtime::{FrameWorld, SceneSource};
use water_scene::island::{Wind, build_rain_ground, build_surf, build_wet_shore};

use super::{limits, sample_json, support::Fixture, take_image};

const EXTENT: [u32; 2] = [1280, 720];
const RATE: u32 = 10;
const DRYING: u32 = 100;
const SCALE: u32 = 600;

fn weather(f: &mut Fixture, rate: u32, drying: u32, scale: u32) {
    f.parameters(&format!(
        r#"{{"rain.rate":{rate},"rain.drying":{drying},"rain.time_scale":{scale}}}"#
    ));
}

fn advance(f: &mut Fixture, time: &mut f64, requested_ticks: u32) {
    for _ in 0..requested_ticks {
        *time += 1.0 / 60.0;
        f.frame_without_timings(*time);
    }
}

fn checkpoint(f: &mut Fixture, time: &mut f64, name: &str, directory: &Path) -> Value {
    *time += 1.0 / 60.0;
    f.host
        .enable_submitted_capture_with_limits(2, limits())
        .expect("T2 fixed checkpoint");
    let sample = f.frame(*time);
    let image = take_image(f);
    assert_eq!(sample.submission_order, image.sample.submission_order);
    let file = format!("{name}.rgba");
    std::fs::write(directory.join(&file), &image.rgba).expect("fixed submitted pixels");
    let view = f
        .host
        .source()
        .frame(
            f.host.world().source(),
            sample.presentation_alpha,
            f.extent,
            None,
        )
        .expect("submitted camera")
        .view;
    let record = json!({
        "name":name, "file":file, "camera":view.camera_position(),
        "sample":sample_json(&sample), "exposure_words":image.exposure_gpu_words,
        "submitted_elapsed":image.submitted_elapsed.as_secs_f64(),
    });
    f.host.recycle_submitted_image(image);
    f.host.disable_submitted_capture();
    record
}

pub(super) fn fixed(gpu: &thrum_test_support::Gpu, directory: &Path, shore: Option<Wind>) -> Value {
    let mut images = Vec::new();
    if let Some(wind) = shore {
        let (game, report, _, _) = build_wet_shore(wind, true).expect("exposed beach");
        let mut f = Fixture::from_game(gpu, EXTENT, game, report);
        f.parameters(r#"{"camera.island":0,"sun.elevation":37,"sea.stage":0}"#);
        f.warm_cooked();
        let mut time = 1.0;
        for (tick, name) in [(6, "first"), (12, "second"), (12, "third")] {
            advance(&mut f, &mut time, tick);
            images.push(checkpoint(&mut f, &mut time, name, directory));
        }
    } else {
        let (game, report, map, _) =
            build_rain_ground(Wind::Calm, true, true).expect("rain ground");
        let mut f = Fixture::from_game(gpu, EXTENT, game, report);
        f.parameters(r#"{"rain.rate":0,"rain.drying":0,"rain.time_scale":1}"#);
        f.warm_cooked();
        let mut time = 1.0;
        for (name, rain_ticks) in [("dry", 0), ("damp", 13), ("sheen", 136)] {
            weather(&mut f, RATE, 0, SCALE);
            advance(&mut f, &mut time, rain_ticks);
            weather(&mut f, 0, 0, 1);
            for (sun, light) in [(64, "noon"), (8, "low")] {
                for (camera, view) in [
                    (4, "half-down"),
                    (13, "half-graze"),
                    (14, "eye-down"),
                    (15, "eye-graze"),
                ] {
                    f.parameters(&format!(
                        r#"{{"camera.island":{camera},"sun.elevation":{sun}}}"#
                    ));
                    let label = format!("{name}-{light}-{view}");
                    images.push(checkpoint(&mut f, &mut time, &label, directory));
                }
            }
            if name != "damp" {
                for (camera, view) in [
                    (16, "roof-top"),
                    (17, "wall-rock-metal"),
                    (18, "wall-reflection"),
                ] {
                    f.parameters(&format!(
                        r#"{{"camera.island":{camera},"sun.elevation":35}}"#
                    ));
                    images.push(checkpoint(
                        &mut f,
                        &mut time,
                        &format!("{name}-{view}"),
                        directory,
                    ));
                }
            }
        }
        f.parameters(r#"{"sun.elevation":35}"#);
        for (name, ticks, speed) in [
            ("soaked", 0, 120),
            ("slope-clears", 8, 120),
            ("film-clears", 12, 120),
            ("margin-recedes", 54, 600),
            ("dry-finish", 210, 600),
        ] {
            weather(&mut f, 0, DRYING, speed);
            advance(&mut f, &mut time, ticks);
            weather(&mut f, 0, 0, 1);
            for (camera, view) in [(13, "joints"), (14, "margin")] {
                f.parameters(&format!(r#"{{"camera.island":{camera}}}"#));
                let mut image = checkpoint(&mut f, &mut time, &format!("{name}-{view}"), directory);
                image["yard_levels"] =
                    json!(super::rain_cost::levels(&map.lock().expect("yard map")));
                images.push(image);
            }
        }
    }
    json!({"target":"mac-metal","mode":"fixed","extent":EXTENT,
        "commit":std::env::var("THRUM_WATER_COMMIT").expect("revision"),
        "rain_rate_mm_h":RATE,"drying_rate_mm_h":DRYING,"wind":shore.map(|w|format!("{w:?}")),"images":images})
}

pub(super) fn lake_film(gpu: &thrum_test_support::Gpu, directory: &Path) -> Value {
    let extent = [960, 540];
    let camera = thrum_app::sky::Camera::look_at([164.5, 13.2, -12.5], [159.0, 12.0, -10.0], 50.0);
    let capture = |label, wet, water| {
        let (game, report, _, _) = if wet {
            build_wet_shore(Wind::Gale, water)
        } else {
            build_surf(Wind::Gale, water)
        }
        .expect("matched lake view");
        let game = game.with_presentation_camera(move |_| camera);
        let mut f = Fixture::from_game(gpu, extent, game, report);
        f.parameters(r#"{"sun.elevation":22}"#);
        f.warm_cooked();
        for _ in 0..8 {
            f.frame_without_timings(2.0);
        }
        f.host
            .enable_submitted_capture_with_limits(1, limits())
            .expect("lake submitted capture");
        let frame = f.frame(2.0);
        let timing = frame
            .gpu_pass_ns
            .as_ref()
            .expect("lake frame's Metal timestamps");
        assert_eq!(
            timing
                .passes()
                .iter()
                .any(|pass| pass.name.ends_with("shore film")),
            water
        );
        let image = take_image(&mut f);
        assert_eq!(image.sample.submission_order, frame.submission_order);
        let file = format!("{label}.rgba");
        std::fs::write(directory.join(&file), &image.rgba).expect("lake review capture");
        (
            image.rgba,
            json!({"name":label,"file":file,"sample":sample_json(&frame)}),
        )
    };
    let [
        (ordinary, ordinary_frame),
        (opaque, opaque_frame),
        (wet, wet_frame),
        (wet_opaque, wet_opaque_frame),
    ] = [
        ("ordinary", false, true),
        ("opaque", false, false),
        ("wet", true, true),
        ("wet-opaque", true, false),
    ]
    .map(|(label, wet, water)| capture(label, wet, water));
    let different = |a: &[u8], b: &[u8]| a.iter().zip(b).any(|(a, b)| a.abs_diff(*b) > 5);
    // Sample the landward shore next to the witness point. Subtract each
    // provider's own opaque arm so darkened sand cannot count as film.
    let mut reference = 0;
    let mut retained = 0;
    for y in 269..277 {
        for x in 445..525 {
            let at = ((y * extent[0] + x) * 4) as usize;
            let rgb = at..at + 3;
            if different(&ordinary[rgb.clone()], &opaque[rgb.clone()]) {
                reference += 1;
                retained += usize::from(different(&wet[rgb.clone()], &wet_opaque[rgb]));
            }
        }
    }
    assert!(
        reference >= 30 && retained * 10 >= reference * 9,
        "Gale lake shore retained {retained}/{reference} water contributions"
    );
    json!({"target":"mac-metal","mode":"fixed","extent":extent,"clock_seconds":2.0,
        "commit":std::env::var("THRUM_WATER_COMMIT").expect("revision"),
        "box":[445,269,80,8],"channel_delta_threshold":5,
        "reference":reference,"retained":retained,"images":
        [ordinary_frame,opaque_frame,wet_frame,wet_opaque_frame]})
}

pub(super) fn cost(gpu: &thrum_test_support::Gpu) -> Value {
    let extent = [2560, 1440];
    let arms: Vec<_> = [false, true]
        .into_iter()
        .map(|provider| {
            let (game, report, map, _) =
                build_rain_ground(Wind::Calm, true, provider).expect("cost pair");
            let mut f = Fixture::from_game(gpu, extent, game, report);
            f.parameters(
                r#"{"camera.island":14,"sun.elevation":55,"rain.rate":0,"rain.drying":0}"#,
            );
            f.warm_cooked();
            let mut time = 1.0;
            weather(&mut f, RATE, 0, SCALE);
            advance(&mut f, &mut time, 149);
            let levels = super::rain_cost::levels(&map.lock().expect("yard map"));
            weather(&mut f, RATE, 0, 1);
            let frames: Vec<_> = (1..=20)
                .map(|i| {
                    let sample = f.frame(time + f64::from(i) / 120.0);
                    let mut record = sample_json(&sample);
                    record["completion_upper_ms"] = json!(f.last_elapsed.as_secs_f64() * 1000.0);
                    record
                })
                .collect();
            let report = f.report.lock().expect("water report");
            let forward = f
                .host
                .scene_shader_source()
                .expect("submitted forward shader");
            let counts = fragment_textures(forward, "shadowed_fragment");
            json!({
                "provider":provider, "levels":levels, "frames":frames,
                "forward_sampled_textures":counts,
                "surface_sampled_textures":report.surface_sampled_textures,
                "sampled_texture_limit":f.caps.limits().max_sampled_textures_per_shader_stage,
                "resources":report.resources,
            })
        })
        .collect();
    json!({"target":"mac-metal","extent":extent,"capture":false,"rain_rate_mm_h":RATE,"arms":arms})
}

fn fragment_textures(source: &str, entry: &str) -> usize {
    use wgpu::naga;
    let module = naga::front::wgsl::parse_str(source).expect("forward WGSL");
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .expect("forward shader validation");
    let index = module
        .entry_points
        .iter()
        .position(|point| point.name == entry)
        .expect("opaque fragment");
    module
        .global_variables
        .iter()
        .filter(|(handle, global)| {
            matches!(
                module.types[global.ty].inner,
                naga::TypeInner::Image {
                    class: naga::ImageClass::Sampled { .. } | naga::ImageClass::Depth { .. },
                    ..
                }
            ) && !info.get_entry_point(index)[*handle].is_empty()
        })
        .count()
}
