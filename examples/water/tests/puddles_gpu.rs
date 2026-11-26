//! Submitted depth and continuous level proofs for the selected rain yard.
#[cfg(target_os = "macos")]
mod support;

thrum_test_support::macos_gpu_test! {
fn submitted_yard_depth_matches_the_hollows_map() {
    let Some(gpu) = thrum_test_support::gpu(
        "submitted_yard_depth_matches_the_hollows_map",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    support::island::with_cooked_island("submitted_yard_depth_matches_the_hollows_map", || {
        check_yard_depth(&gpu);
        check_yard_transitions(&gpu);
    });
}
}

#[cfg(target_os = "macos")]
fn check_yard_transitions(gpu: &thrum_test_support::Gpu) {
    use std::time::Duration;
    use thrum_app::Game;
    use water_scene::island::{Wind, build_rain_yard};
    const EXTENT: [u32; 2] = [96, 64];
    const PITS: [[f64; 2]; 2] = [[147.2, 66.2], [149.75, 66.35]];
    const FLOAT: [f64; 2] = [147.575, 66.225];
    let (game, report, map, _) = build_rain_yard(Wind::Calm, true).unwrap();
    let mut scene = support::Fixture::from_game(gpu, EXTENT, game, report);
    scene.parameters(r#"{"camera.island":3,"rain.rate":0}"#);
    scene.warm_cooked();
    let frame_time = |tick: u32| (Duration::from_nanos(16_666_667) * tick).as_secs_f64();
    scene.frame_without_timings(frame_time(60));
    scene.frame_without_timings(frame_time(61));
    let creations = scene.host.pipeline_creation_count();
    let storage = scene.report.lock().unwrap().resources.clone();
    let identity = map.lock().unwrap().generation(3);
    let float_line = |scene: &mut support::Fixture| {
        scene.game.poll_status();
        let controls: serde_json::Value =
            serde_json::from_str(&scene.game.controls_mut().describe_json()).unwrap();
        controls["controls"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["key"] == "rain.float_level")
            .unwrap()["value"]
            .as_f64()
            .unwrap()
    };
    let initial = float_line(&mut scene);
    scene.parameters(r#"{"rain.rate":100,"rain.time_scale":600}"#);
    let mut merged = false;
    let mut drying = false;
    let mut parted = false;
    let mut rising = false;
    let mut last_line = initial;
    for tick in 62..=240 {
        let sample = scene.frame_without_timings(frame_time(tick));
        assert_eq!(sample.completed_ticks, 1);
        let line = float_line(&mut scene);
        assert!(
            (line - last_line).abs() < 0.015,
            "the float stepped at {tick}"
        );
        last_line = line;
        let (levels, float) = {
            let published = map.lock().unwrap();
            let wet = |point| published.sample(point).filter(|hit| hit.depth > 0.0);
            (
                PITS.map(|point| wet(point).map(|hit| hit.row.level)),
                wet(FLOAT),
            )
        };
        if line > initial + 0.0001
            && let Some(float) = float
        {
            assert!(
                (line - float.row.level).abs() < 0.000_11,
                "float waterline at {tick}"
            );
            rising = true;
        }
        if let [Some(low), Some(high)] = levels {
            let joined = low.to_bits() == high.to_bits();
            if joined && !merged {
                merged = true;
                assert!(
                    scene
                        .witness()
                        .iter()
                        .all(|pixel| pixel[2].abs() < 65_000.0),
                    "the merge invalidated history"
                );
            } else if drying && !joined {
                parted = true;
                assert!(
                    scene
                        .witness()
                        .iter()
                        .all(|pixel| pixel[2].abs() < 65_000.0),
                    "the parting invalidated history"
                );
                break;
            }
        }
        if merged && rising && !drying {
            scene.parameters(r#"{"rain.rate":0,"rain.drying":100}"#);
            drying = true;
        }
    }
    assert!(merged && rising && parted);
    assert_eq!(scene.host.pipeline_creation_count(), creations);
    assert_eq!(scene.report.lock().unwrap().resources, storage);
    let published = map.lock().unwrap();
    assert_eq!(published.generation(3), identity);
    assert_eq!(published.held_level_tables, 0);
}

#[cfg(target_os = "macos")]
#[expect(clippy::print_stderr, reason = "retain the actual Metal depth witness")]
fn check_yard_depth(gpu: &thrum_test_support::Gpu) {
    use support::depth::DepthTap;
    use thrum_app::sky::{Camera, ContentStatus};
    use thrum_water_model::Map;
    use water_scene::island::{Wind, build_rain_yard};

    const EXTENT: [u32; 2] = [320, 200];
    const EPS_METRES: f64 = 0.000_04;
    const YARD_BODY: usize = 3;
    const RAIN: &str = r#"{"camera.island":3,"rain.rate":100,"rain.time_scale":600}"#;
    let camera = Camera::look_at([149.0, 33.12, 64.94], [149.0, 18.12, 65.0], 45.0);
    let matrix = support::depth::matrix(camera, EXTENT);
    let make = |draw_water| {
        let (game, report, map, _) = build_rain_yard(Wind::Calm, draw_water).expect("rain yard");
        let tap = DepthTap::default();
        let game = game.with_contribution(Box::new(tap.contribution(EXTENT)), || {
            ContentStatus::Drawn(1)
        });
        let mut scene = support::Fixture::from_game(gpu, EXTENT, game, report);
        scene.parameters(r#"{"camera.island":3,"rain.rate":0}"#);
        scene.warm_cooked();
        scene.frame(1.0);
        scene.parameters(RAIN);
        (scene, tap, map)
    };
    let (mut wet, depth, map) = make(true);
    let (mut opaque, opaque_depth, _) = make(false);
    let mut split = false;
    let mut joined = false;
    for time in [1.5, 2.0] {
        wet.frame(time);
        opaque.frame(time);
        let surface = depth.read(&wet);
        let floor = opaque_depth.read(&opaque);
        let map = map.lock().expect("yard map");
        let owner = |map: &Map, point| {
            map.sample(point)
                .filter(|s| s.body == YARD_BODY && s.depth > 0.0)
        };
        let pits = [[147.2, 66.2], [149.75, 66.35]];
        if let [Some(a), Some(b)] = pits.map(|point| owner(&map, point)) {
            split |= a.row.level.to_bits() != b.row.level.to_bits();
            joined |= a.row.level.to_bits() == b.row.level.to_bits();
        }
        let mut checked_water = 0;
        let mut checked_dry = 0;
        let mut maximum_error = 0.0_f64;
        for y in 0..EXTENT[1] {
            for x in 0..EXTENT[0] {
                let index = (y * EXTENT[0] + x) as usize;
                if floor[index] == 0.0 {
                    continue;
                }
                let point = support::world_at(matrix, EXTENT, [x, y], floor[index]);
                let [x_world, z_world] = [f64::from(point.x), f64::from(point.z)];
                if !(144.0..154.0).contains(&x_world) || !(61.0..69.0).contains(&z_world) {
                    continue;
                }
                let expected = owner(&map, [x_world, z_world]);
                let is_wet = expected.is_some();
                // The CPU label is a cell-centre test. Ground clips the
                // surface continuously at the edge of the same cells.
                if [-0.05, 0.0, 0.05].into_iter().any(|dx| {
                    [-0.05, 0.0, 0.05]
                        .into_iter()
                        .any(|dz| owner(&map, [x_world + dx, z_world + dz]).is_some() != is_wet)
                }) {
                    continue;
                }
                let expect_visible_water =
                    expected.filter(|hit| f64::from(point.y) < hit.row.level);
                if let Some(hit) = expect_visible_water {
                    assert!(
                        surface[index] > floor[index],
                        "missing water at {point:?} at {time}"
                    );
                    let submitted = support::world_at(matrix, EXTENT, [x, y], surface[index]);
                    let error = (f64::from(submitted.y) - hit.row.level).abs();
                    maximum_error = maximum_error.max(error);
                    assert!(
                        error <= EPS_METRES,
                        "sloping surface at {submitted:?} vs {}",
                        hit.row.level
                    );
                    checked_water += 1;
                } else {
                    assert_eq!(
                        surface[index].to_bits(),
                        floor[index].to_bits(),
                        "unexpected puddle at {point:?}"
                    );
                    checked_dry += 1;
                }
            }
        }
        assert!(checked_water > 100 && checked_dry > 100);
        assert!(owner(&map, [151.75, 67.8]).is_none(), "under the roof");
        assert!(
            owner(&map, [152.4, 65.0]).is_none(),
            "beside the paving puddle"
        );
        eprintln!(
            "Metal yard at {time:.2}s: {checked_water} wet, {checked_dry} dry; maximum level error {maximum_error:.8} m"
        );
    }
    assert!(split && joined, "samples must straddle the paving merge");
}
