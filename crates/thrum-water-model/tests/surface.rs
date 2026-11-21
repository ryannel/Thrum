//! Surface values and time/spatial derivatives against analytic waves and SEA.
use crate::support;
use support::fixtures::{basin, body, close, field, grid, profile};
use thrum_water_model::{BodyKind, Chart, Map};

#[test]
fn calm_tilted_chart_reports_the_analytic_surface_slope() {
    let chart = Chart {
        levels: (0..81)
            .map(|i| 1.0 - 0.05 * f64::from(i % 9) + 0.02 * f64::from(i / 9))
            .collect(),
        currents: vec![[1.0, 0.0]; 81],
        footprint: (0..81)
            .map(|i| (2..=6).contains(&(i % 9)) && (2..=6).contains(&(i / 9)))
            .collect(),
        ends: vec![],
    };
    let bed = grid([9, 9], |x, z| {
        if (2.0..=6.0).contains(&x) && (2.0..=6.0).contains(&z) {
            -5.0
        } else {
            10.0
        }
    });
    let map = Map::prepare(
        bed,
        vec![body("flow", BodyKind::Chart(chart), 0.0)],
        profile(),
    )
    .unwrap();
    for p in [[3.2, 4.3], [4.0, 4.0], [5.1, 2.9]] {
        let answer = map.surface(&field(0.0), p, 0.0).unwrap();
        close(answer.sample.height, 1.0 - 0.05 * p[0] + 0.02 * p[1]);
        assert!((answer.sample.slope[0] + 0.05).abs() < 1e-9);
        assert!((answer.sample.slope[1] - 0.02).abs() < 1e-9);
    }
}

#[test]
fn open_water_is_the_sea_evaluator_and_calm_basin_is_its_level() {
    let mut field = field(0.4);
    field.layers[0] =
        thrum_sea_model::LongWaves::new(field.layers[0].components().to_vec(), 0.8).unwrap();
    let mut sea = body("water", BodyKind::Sea, 2.0);
    sea.gains = [1.0; 4];
    let map = Map::prepare(grid([5, 5], |_, _| -100.0), vec![sea], profile()).unwrap();
    for point in [[1.2, 3.1], [-100.0, 240.0], [1e6, -2e6]] {
        for t in [0.0, 1.0, 137.0] {
            let a = map.surface(&field, point, t).unwrap();
            let b = field.layers[0].evaluate(point, t).unwrap();
            close(a.sample.height, 2.0 + b.height);
            assert_eq!(a.sample.slope, b.slope);
            assert_eq!(a.sample.velocity, b.velocity);
            close(a.sample.height_dt, b.height_dt);
            assert!(a.wet);
        }
    }
    let basin = basin();
    let s = basin.surface(&field, [10.0, 10.0], 12.0).unwrap();
    close(s.sample.height, 3.0);
    assert_eq!(s.sample.slope, [0.0; 2]);
    assert_eq!(s.sample.velocity, [0.0; 3]);
    assert!(!basin.surface(&field, [30.0, 10.0], 12.0).unwrap().wet);
}

#[test]
fn surface_derivatives_follow_wave_and_current_is_added_once() {
    let f = field(0.5);
    let mut b = body("water", BodyKind::Sea, 0.0);
    b.gains = [1.0; 4];
    let map = Map::prepare(grid([5, 5], |_, _| -100.0), vec![b], profile()).unwrap();
    let p = [1.3, 2.4];
    let t = 0.7;
    let e = 1e-5;
    let a = map.surface(&f, p, t).unwrap();
    let dx = (map.surface(&f, [p[0] + e, p[1]], t).unwrap().sample.height
        - map.surface(&f, [p[0] - e, p[1]], t).unwrap().sample.height)
        / (2.0 * e);
    let dt = (map.surface(&f, p, t + e).unwrap().sample.height
        - map.surface(&f, p, t - e).unwrap().sample.height)
        / (2.0 * e);
    assert!((a.sample.slope[0] - dx).abs() < 1e-8);
    assert!((a.sample.height_dt - dt).abs() < 1e-8);
    let mut row = map.sample(p).unwrap();
    row.row.current = [2.0, -0.5];
    let moving = map.surface_on_row(&f, p, t, row).unwrap();
    close(moving.sample.velocity[0], a.sample.velocity[0] + 2.0);
    close(moving.sample.velocity[2], a.sample.velocity[2] - 0.5);
}
