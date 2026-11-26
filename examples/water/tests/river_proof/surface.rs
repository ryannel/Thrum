use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use thrum_app::sky::{Camera, ContentStatus, Controls, SkyGame};
use water_scene::{channel, island::Wind, river};

use crate::support::{self, Fixture};

const EXTENT: [u32; 2] = [95, 13];
const STEP: Duration = Duration::from_nanos(16_666_667);
const EPS_DEPTH: f64 = 0.000_04;

fn transect_points() -> impl Iterator<Item = (usize, [f64; 2])> {
    let spring = (0..56).map(|station| {
        let z = f64::from(station) * 1.09 - 91.4;
        (3, [179.35 + f64::from(station % 3) * 0.67, z])
    });
    let outlet = (0..60).map(|station| {
        let x = f64::from(station) * 2.07 + 215.3;
        (5, [x, 4.1 + f64::from(station % 4) * 0.69])
    });
    spring.chain(outlet)
}

fn ray_hit(inverse: glam::DMat4, column: u32, row: u32, depth: f32) -> glam::DVec3 {
    let u = (f64::from(column) + 0.5) / f64::from(EXTENT[0]);
    let v = (f64::from(row) + 0.5) / f64::from(EXTENT[1]);
    inverse.project_point3(glam::dvec3(2.0 * u - 1.0, 1.0 - 2.0 * v, f64::from(depth)))
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::print_stdout,
    reason = "authored camera coordinates use GPU metres and logs retain target measurements"
)]
pub(crate) fn check(gpu: &thrum_test_support::Gpu) {
    let forcing = channel::forcing(Wind::Trade);
    let (map, files) = river::map(&forcing, Some(&support::river_store())).expect("island rivers");
    let mut spectrum = thrum_sea_model::Spectrum::build(&forcing).expect("trade spectrum");
    spectrum.cascades = [160.0, 37.0, 9.0].map(|length| {
        thrum_sea_model::Cascade::new(length, &[]).expect("empty calibration cascade")
    });
    let mut profile = water_scene::profile(&spectrum);
    profile.waves.choppiness = 0.0;
    let field = water_scene::floating::fallback(&spectrum, 0.0).expect("column field");
    let map = Arc::new(Mutex::new(map));
    let water =
        thrum_water::Water::mapped(Arc::clone(&map), spectrum, profile).expect("mapped rivers");
    let report = water.report();
    let camera = Arc::new(Mutex::new(river::VIEWS[0].1));
    let on_tick = Arc::clone(&camera);
    let tap = support::depth::DepthTap::default();
    let game = SkyGame::new(Controls::new(), river::VIEWS[0].1, |_, _| {})
        .with_presentation_camera(move |_| *on_tick.lock().expect("proof camera"))
        .with_tick_duration(STEP)
        .expect("elapsed surface clock")
        .with_ground(None)
        .with_contribution(Box::new(water), || ContentStatus::Drawn(1))
        .with_contribution(Box::new(tap.contribution(EXTENT)), || {
            ContentStatus::Drawn(1)
        });
    let mut scene = Fixture::from_game(gpu, EXTENT, game, report);
    scene.frame_without_timings(0.0);
    let mut time = 0.0;
    let [low, middle, high] = files[0].gauges().try_into().expect("three spring samples");
    let mut count = [0; 6];
    let mut max_error: f64 = 0.0;
    for (spring, stage) in [
        (low, 0.0),
        ((high - low).mul_add(0.37, low), 0.37),
        (middle, 0.5),
        (high, 1.0),
    ] {
        {
            let mut map = map.lock().expect("river stage");
            map.set_gauge(0, spring).expect("spring stage");
            map.set_stage(1, stage).expect("lake stage");
        }
        // Sweep the two main reaches through the incoming swell, then sample
        // the short reach below the fall and the previously failing outlet site.
        let close = transect_points()
            .chain([(4, [180.05, -24.65]), (5, [320.87, 6.17])])
            .map(|(owner, point)| (owner, point, [0.14, 0.25, 0.08]));
        let from_bank = [
            (3, [179.35, -61.97], [7.0, 2.0, 0.8]),
            (5, [237.98, 6.17], [7.0, 2.0, 0.8]),
        ];
        for (owner, point, [dx, dy, dz]) in close.chain(from_bank) {
            if !map
                .lock()
                .expect("river map")
                .sample(point)
                .is_some_and(|s| s.body == owner && s.depth > 0.08)
            {
                continue;
            }
            time += 0.25;
            let height = map
                .lock()
                .expect("river map")
                .surface(&field, point, time)
                .expect("model surface")
                .sample
                .height as f32;
            let [x, z] = point.map(|p| p as f32);
            let view = Camera::look_at(
                [x + dx, (height + dy).max(0.12), z + dz],
                [x, height, z],
                4.0,
            );
            *camera.lock().expect("proof camera") = view;
            let inverse = support::depth::matrix(view, EXTENT).as_dmat4().inverse();
            scene.frame_without_timings(time + STEP.as_secs_f64());
            let depth = tap.read(&scene);
            let map = map.lock().expect("river map");
            let row = EXTENT[1] / 2;
            for column in [EXTENT[0] / 2 - 8, EXTENT[0] / 2, EXTENT[0] / 2 + 8] {
                let d = depth[(row * EXTENT[0] + column) as usize];
                assert!(d > 0.0, "submitted depth at {point:?}, stage {stage}");
                let p = ray_hit(inverse, column, row, d);
                let place = [p.x, p.z];
                let expected = map.surface(&field, place, time).expect("model surface");
                assert!(expected.wet, "model water at {place:?}, stage {stage}");
                assert_eq!(expected.body, Some(owner), "model river at {place:?}");
                let error = (p.y - expected.sample.height).abs();
                assert!(
                    error <= EPS_DEPTH,
                    "depth at {place:?}, stage {stage}, t {time:.2}: {error:.8} m"
                );
                count[owner] += 1;
                max_error = max_error.max(error);
            }
        }
    }
    assert!(
        count[3] >= 640 && count[4] >= 12 && count[5] >= 700,
        "{count:?}"
    );
    println!(
        "Metal river transects: {} spring, {} plunge reach, {} outlet hits, maximum residual {max_error:.7} m",
        count[3], count[4], count[5],
    );
    let joins: Vec<_> = files
        .iter()
        .flat_map(|file| &file.reaches)
        .flat_map(|reach| &reach.ends)
        .filter(|end| end.kind != thrum_water_model::EndKind::Brink)
        .collect();
    assert_eq!(joins.len(), 3);
    let mut max_swell: f64 = 0.0;
    for (lake, tide) in [(0.0, -0.5), (1.0, -0.5), (0.0, 0.5), (1.0, 0.5)] {
        {
            let mut map = map.lock().expect("river stage");
            map.set_stage(1, lake).expect("lake stage");
            map.set_stage(0, tide).expect("tide");
        }
        for join in &joins {
            time += 0.25;
            let [a, b] = join.segment;
            let point = [a[0].midpoint(b[0]), a[1].midpoint(b[1])];
            let centre = {
                let map = map.lock().expect("river map");
                let bed = map.bed().height(point).expect("join bed");
                let (at, surface) = (0_u16..=90)
                    .map(|offset| time + f64::from(offset) * 0.20)
                    .find_map(|at| {
                        let surface = map.surface(&field, point, at).expect("join surface");
                        let high = surface.sample.height;
                        (surface.wet
                            && high > bed + 0.03
                            && (join.body != "Sea" || high > tide + 0.02))
                            .then_some((at, surface))
                    })
                    .expect("a submerged join under the passing swell");
                time = at;
                surface
            };
            let [x, z] = point.map(|p| p as f32);
            let y = centre.sample.height as f32;
            let offset = if (a[0] - b[0]).abs() > 1e-8 {
                [0.002, 0.0]
            } else {
                [0.0, 0.002]
            };
            let view = Camera::look_at(
                [x + offset[0], (y + 0.25).max(0.12), z + offset[1]],
                [x, y, z],
                4.0,
            );
            *camera.lock().expect("proof camera") = view;
            let inverse = support::depth::matrix(view, EXTENT).as_dmat4().inverse();
            // The displayed sample lies one fixed step behind the observation clock.
            scene.frame_without_timings(time + STEP.as_secs_f64());
            let depth = tap.read(&scene);
            let map = map.lock().expect("river map");
            let row = EXTENT[1] / 2;
            // An odd target samples the seam line through the centre of the
            // middle row. Adjacent columns then resolve the two ownership
            // sides within millimetres of the shared mesh edge.
            let [left, right] = [EXTENT[0] / 2 - 1, EXTENT[0] / 2 + 1].map(|column| {
                let d = depth[(row * EXTENT[0] + column) as usize];
                assert!(
                    d > 0.0,
                    "rendered join at {point:?}, lake {lake}, tide {tide}, column {column}"
                );
                let p = ray_hit(inverse, column, row, d);
                let place = [p.x, p.z];
                let expected = map.surface(&field, place, time).expect("model surface");
                assert!(expected.wet);
                let error = (p.y - expected.sample.height).abs();
                assert!(
                    error < EPS_DEPTH,
                    "depth residual at {place:?}: {error:.7} m"
                );
                (expected.body, p.y, expected.sample.height)
            });
            assert_ne!(
                left.0, right.0,
                "the pixels must straddle ownership at {point:?}"
            );
            // Sea wave height is a common displacement across this join.
            // Compare the step itself, after accounting for the reference
            // wave slope between the two distinct ray hits.
            let residual = ((right.1 - left.1) - (right.2 - left.2)).abs();
            println!(
                "Metal join {point:?}, lake {lake}, tide {tide}, t {time:.3}: owners {:?}/{:?}, GPU {:.7}/{:.7} m, model {:.7}/{:.7} m, seam residual {residual:.7} m",
                left.0, right.0, left.1, right.1, left.2, right.2,
            );
            assert!(residual < EPS_DEPTH, "step at {point:?}: {residual:.7} m");
            if join.body == "Sea" {
                assert!([left.1, right.1].iter().all(|y| y - tide > 0.01));
                max_swell = max_swell.max((centre.sample.height - tide).abs());
            }
        }
    }
    assert!(max_swell > 0.01, "the sea join was crossed with a swell");
}
