use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use thrum_app::sky::{Camera, ContentStatus, Controls, SkyGame};
use water_scene::{channel, island::Wind, river};

use crate::support::{self, Fixture};

const EXTENT: [u32; 2] = [96, 12];
const STEP: Duration = Duration::from_nanos(16_666_667);
const EPS_DEPTH: f64 = 0.000_04;

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
    let joins: Vec<_> = files
        .iter()
        .flat_map(|file| &file.reaches)
        .flat_map(|reach| &reach.ends)
        .filter(|end| end.kind != thrum_water_model::EndKind::Brink)
        .collect();
    assert_eq!(joins.len(), 3);
    scene.frame_without_timings(0.0);
    let mut time = 0.0;
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
                12.0,
            );
            *camera.lock().expect("proof camera") = view;
            let matrix = support::depth::matrix(view, EXTENT);
            // The displayed sample lies one fixed step behind the observation clock.
            scene.frame_without_timings(time + STEP.as_secs_f64());
            let depth = tap.read(&scene);
            let map = map.lock().expect("river map");
            let row = EXTENT[1] / 2;
            let [left, right] = [EXTENT[0] / 2 - 2, EXTENT[0] / 2 + 1].map(|column| {
                let d = depth[(row * EXTENT[0] + column) as usize];
                assert!(
                    d > 0.0,
                    "rendered join at {point:?}, lake {lake}, tide {tide}, column {column}"
                );
                let p = support::world_at(matrix, EXTENT, [column, row], d);
                let place = [f64::from(p.x), f64::from(p.z)];
                let expected = map.surface(&field, place, time).expect("model surface");
                assert!(expected.wet);
                let error = (f64::from(p.y) - expected.sample.height).abs();
                assert!(
                    error < EPS_DEPTH,
                    "depth residual at {place:?}: {error:.7} m"
                );
                (expected.body, f64::from(p.y), expected.sample.height)
            });
            assert_ne!(
                left.0, right.0,
                "the pixels must straddle ownership at {point:?}"
            );
            let residual = ((right.1 - left.1) - (right.2 - left.2)).abs();
            assert!(residual < EPS_DEPTH, "step at {point:?}: {residual:.7} m");
            if join.body == "Sea" {
                max_swell = max_swell.max((centre.sample.height - tide).abs());
            }
            println!(
                "Metal join {point:?} at lake {lake}, tide {tide}: owners {:?}/{:?}, seam residual {residual:.7} m",
                left.0, right.0
            );
        }
    }
    assert!(max_swell > 0.01, "the sea join was crossed with a swell");
}
