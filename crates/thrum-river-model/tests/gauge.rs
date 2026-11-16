//! M3.2, ruling 2: a river chart's row is the mix, linear in the gauge, of the two samples that
//! bracket it; the gauge is the source body's level where the river has one and the river's own
//! stage where it has none; every chart of a river takes the same gauge. Between samples 0.3 m
//! apart the mixed row is within 1 cm in depth and 25% in speed of a solve at that level, in the
//! cells both samples wet, as part 3 of the design proof compares them; a chart left at one
//! sample with its level raised is not. As the gauge moves 1 cm the waterline on the channel's
//! banks, of 6 in 1, moves by at most one cell. A chart with one sample answers to the bit as
//! before, and its stage adds to its level.
#![expect(
    clippy::unwrap_used,
    clippy::float_cmp,
    clippy::cast_precision_loss,
    reason = "test arithmetic; the test reports what it checked"
)]
mod support;
use support::lake::{BANKS, CENTRE, HIGH, WATER, bed_of, chart_body, lake, water};
use support::valley::{DISCHARGE, LAKE, valley};
use thrum_river_model::{Profile, Samples, key, key_for_samples, solve, solve_samples};
use thrum_water_model::{Body, BodyKind, Gauge, Map, RiverFile, Stage};

const DX: f64 = 0.25;
const RANGE: [f64; 2] = [3.1, 3.7];

fn sampled(banks: support::lake::Banks) -> (thrum_river_model::Grid, Samples, RiverFile) {
    let (grid, declaration) = lake(DX, banks, RANGE);
    let profile = Profile::default();
    let s = solve_samples(&grid, &declaration, &profile, &[]).unwrap();
    let file = s
        .file(key_for_samples(&grid, &declaration, &profile, &[]))
        .unwrap();
    (grid, s, file)
}

/// A cell's level in sample `k` of the file's first reach.
fn level(file: &RiverFile, k: usize, i: usize) -> f64 {
    if k == 0 {
        file.reaches[0].levels[i]
    } else {
        file.reaches[0].upper[k - 1].levels[i]
    }
}

/// The cells of the first chart whose row at each of two sample gauges is that sample's own
/// stored level: those no join blends there.
fn unjoined(
    map: &mut Map,
    grid: &thrum_river_model::Grid,
    file: &RiverFile,
    gauges: [f64; 2],
    samples: [usize; 2],
) -> Vec<bool> {
    let n = grid.heights().len();
    let mut own = vec![true; n];
    for (g, k) in gauges.into_iter().zip(samples) {
        map.set_stage(0, g).unwrap();
        for (i, o) in own.iter_mut().enumerate() {
            *o &= map.row(2, grid.point(i)).level == level(file, k, i);
        }
    }
    own
}

#[test]
fn the_mixed_row_is_near_a_solve_at_the_level_between_and_one_sample_raised_is_not() {
    let (grid, samples, file) = sampled(HIGH);
    let z = grid.heights();
    let (mut map, river) = water(&grid, &file, RANGE, RANGE[0], true);
    let river = river.unwrap();
    let chart = 2;
    for (k, g) in [(0, 3.25), (1, 3.55)] {
        let (mut declaration, profile) = (lake(DX, HIGH, RANGE).1, Profile::default());
        declaration.sources[0].level = g;
        let fresh = solve(&grid, &declaration, &profile).unwrap();
        // The water map does not check a file's key; the store's tests do.
        let (g0, g1) = (samples.gauges[k], samples.gauges[k + 1]);
        let w = (g - g0) / (g1 - g0);
        // Cells no join blends: where the map gives each bracketing sample's own level.
        let own = unjoined(&mut map, &grid, &file, [g0, g1], [k, k + 1]);
        map.set_stage(0, g).unwrap();
        assert_eq!(map.gauge(river), g);
        let sp = |c: [f64; 2]| c[0].hypot(c[1]);
        let (mut depth, mut speed, mut cells) = (0.0_f64, 0.0_f64, 0);
        let (mut raised_depth, mut raised_speed) = (0.0_f64, 0.0_f64);
        for i in 0..z.len() {
            // Cells both samples wet, which the fresh solve wets too, and no join blends.
            let both = level(&file, k, i) > z[i] && level(&file, k + 1, i) > z[i];
            if !both || !fresh.wet[i] || !own[i] {
                continue;
            }
            let row = map.row(chart, grid.point(i));
            // The map's row is the mix, worked out here from the stored samples.
            let expected = (1.0 - w) * level(&file, k, i) + w * level(&file, k + 1, i);
            assert!((row.level - expected).abs() < 1e-9);
            let (l, u) = (fresh.level(i), sp(fresh.current(i)));
            depth = depth.max((row.level - l).abs());
            speed = speed.max((sp(row.current) - u).abs() / u);
            // The middle sample alone, its level raised by the gauge's move from it.
            let raised = level(&file, 1, i) + (g - samples.gauges[1]);
            let raised_current = file.reaches[0].upper[0].currents[i];
            raised_depth = raised_depth.max((raised - l).abs());
            raised_speed = raised_speed.max((sp(raised_current) - u).abs() / u);
            cells += 1;
        }
        println!(
            "gauge {g:.2} m, between {g0:.2} and {g1:.2}: {cells} cells; the mix is off a fresh solve by \
             {:.1} mm in depth and {:.1}% in speed; the middle sample raised, by {:.1} mm and {:.1}%",
            1000.0 * depth,
            100.0 * speed,
            1000.0 * raised_depth,
            100.0 * raised_speed
        );
        assert!(cells > 300, "{cells}");
        assert!(depth <= 0.01, "{depth}");
        assert!(speed <= 0.25, "{speed}");
        assert!(raised_depth > 0.01 || raised_speed > 0.25);
    }
}

/// Where the water first ends going out from the centre line at `x`, on side `side`.
fn waterline(map: &Map, chart: usize, x: f64, side: f64) -> f64 {
    let step = DX / 64.0;
    let mut off = 0.0;
    while off < 4.0 {
        let p = [x, CENTRE + side * off];
        let water = map
            .sample(p)
            .is_some_and(|s| s.body == chart && s.depth > 0.0);
        if !water {
            return off;
        }
        off += step;
    }
    off
}

#[test]
fn the_waterline_moves_at_most_a_cell_for_each_centimetre_of_gauge() {
    let (grid, _, file) = sampled(HIGH);
    let (mut map, _) = water(&grid, &file, RANGE, RANGE[0], true);
    let sections = [8.0, 10.0, 11.0, 20.0, 22.0, 24.0];
    let mut last: Vec<f64> = Vec::new();
    let mut worst = 0.0_f64;
    for step in 0..=60 {
        let g = RANGE[0] + 0.01 * f64::from(step);
        map.set_stage(0, g.min(RANGE[1])).unwrap();
        let now: Vec<f64> = sections
            .iter()
            .flat_map(|&x| [1.0, -1.0].map(|side| waterline(&map, 2, x, side)))
            .collect();
        if !last.is_empty() {
            for (a, b) in last.iter().zip(&now) {
                worst = worst.max((a - b).abs());
            }
        }
        last = now;
    }
    println!(
        "the waterline on six sections' banks moves at most {worst:.3} m for 1 cm of gauge (a cell is {DX} m)"
    );
    assert!(worst <= DX, "{worst}");
}

#[test]
fn a_chart_with_one_sample_answers_to_the_bit_and_its_stage_adds() {
    let (grid, declaration) = lake(DX, HIGH, RANGE);
    let s = solve(&grid, &declaration, &Profile::default()).unwrap();
    let file = s
        .file(key(&grid, &declaration, &Profile::default()))
        .unwrap();
    let at = [RANGE[0]; 2];
    let (plain, _) = water(&grid, &file, at, RANGE[0], false);
    let (one, river) = water(&grid, &file, at, RANGE[0], true);
    let river = river.unwrap();
    assert_eq!(one.river(river).unwrap().gauges.len(), 1);
    let mut gains = 0;
    for zc in 0..64 {
        for xc in 0..136 {
            let p = [0.13 + 0.25 * f64::from(xc), 0.11 + 0.25 * f64::from(zc)];
            assert_eq!(plain.row(2, p), one.row(2, p), "{p:?}");
            assert_eq!(plain.sample(p), one.sample(p), "{p:?}");
            if one.flow_gains(2, p)[0] > 0.0 {
                gains += 1;
            }
        }
    }
    assert!(gains > 100);
    // A plain chart's stage still adds to its level.
    let bed = bed_of(&grid);
    let chart = file.reaches[0]
        .chart(&bed, |n| match n {
            "lake" => Some(0),
            "tarn" => Some(1),
            _ => None,
        })
        .unwrap();
    let staged = Body {
        stage: Stage {
            min: 0.0,
            max: 0.05,
            value: 0.0,
        },
        ..chart_body("staged", chart.clone())
    };
    let mut bodies = vec![
        support::lake::basin(
            "lake",
            [3.5, CENTRE],
            0.0,
            Stage {
                min: RANGE[0],
                max: RANGE[1],
                value: RANGE[0],
            },
        ),
        support::lake::basin(
            "tarn",
            [30.0, CENTRE],
            support::lake::TARN,
            Stage {
                min: 0.0,
                max: 0.0,
                value: 0.0,
            },
        ),
    ];
    bodies.push(staged);
    let mut map = Map::prepare(bed, bodies, WATER).unwrap();
    map.set_stage(2, 0.05).unwrap();
    let i = grid.cell([20.0, CENTRE + 0.1]).unwrap();
    assert_eq!(map.row(2, grid.point(i)).level, chart.levels[i] + 0.05);
}

#[test]
fn every_chart_of_a_river_takes_the_one_gauge() {
    // The lake's outlet with its second way out has two reaches at its highest sample.
    let (grid, samples, file) = sampled(BANKS);
    assert!(file.reaches.len() >= 2, "{}", file.reaches.len());
    let (mut map, river) = water(&grid, &file, RANGE, RANGE[0], true);
    let river = river.unwrap();
    let z = grid.heights();
    let g = 3.6;
    let w = (g - samples.gauges[1]) / (samples.gauges[2] - samples.gauges[1]);
    let mut checked = vec![0; file.reaches.len()];
    for (r, reach) in file.reaches.iter().enumerate() {
        assert_eq!(map.river_of(2 + r), Some((river, r)));
        // Cells no join blends at either sample.
        let mut own = vec![true; z.len()];
        for (gauge, row) in [
            (samples.gauges[1], &reach.upper[0]),
            (samples.gauges[2], &reach.upper[1]),
        ] {
            map.set_stage(0, gauge).unwrap();
            for (i, o) in own.iter_mut().enumerate() {
                *o &= map.row(2 + r, grid.point(i)).level == row.levels[i];
            }
        }
        map.set_stage(0, g).unwrap();
        for i in 0..z.len() {
            let (a, b) = (reach.upper[0].levels[i], reach.upper[1].levels[i]);
            if a > z[i] && b > z[i] && own[i] {
                let expected = (1.0 - w) * a + w * b;
                assert!((map.row(2 + r, grid.point(i)).level - expected).abs() < 1e-9);
                checked[r] += 1;
            }
        }
    }
    println!("cells of each reach checked against the mix at {g} m: {checked:?}");
    assert!(checked.iter().all(|&c| c > 0));
    // The gauge follows the lake's stage, so it is not set on its own.
    assert!(map.set_gauge(river, 3.5).is_err());
}

#[test]
fn a_spring_fed_river_s_gauge_is_its_own_stage() {
    let (grid, declaration) = valley(DX);
    let flows = vec![vec![0.5 * DISCHARGE], vec![DISCHARGE]];
    let profile = Profile::default();
    let s = solve_samples(&grid, &declaration, &profile, &flows).unwrap();
    let file = s
        .file(key_for_samples(&grid, &declaration, &profile, &flows))
        .unwrap();
    let bed = bed_of(&grid);
    let lake = Body {
        name: "lake".into(),
        kind: BodyKind::Basin {
            seed: declaration.receivers[0].seed,
        },
        level: LAKE,
        stage: Stage {
            min: 0.0,
            max: 0.0,
            value: 0.0,
        },
        fetch: 1e6,
        gains: [0.0; 4],
    };
    let chart = file.reaches[0]
        .chart(&bed, |n| (n == "lake").then_some(0))
        .unwrap();
    let mut map =
        Map::prepare(bed.clone(), vec![lake, chart_body("valley", chart)], WATER).unwrap();
    let [lo, hi] = file.gauge_range();
    let river = map
        .add_river(
            file.river(
                &bed,
                "valley",
                &[1],
                Gauge::Stage(Stage {
                    min: lo,
                    max: hi,
                    value: lo,
                }),
            )
            .unwrap(),
        )
        .unwrap();
    let mut own = vec![true; grid.heights().len()];
    for (gauge, levels) in [
        (lo, &file.reaches[0].levels),
        (hi, &file.reaches[0].upper[0].levels),
    ] {
        map.set_gauge(river, gauge).unwrap();
        for (i, o) in own.iter_mut().enumerate() {
            *o &= map.row(1, grid.point(i)).level == levels[i];
        }
    }
    let g = f64::midpoint(lo, hi);
    map.set_gauge(river, g).unwrap();
    assert_eq!(map.gauge(river), g);
    let z = grid.heights();
    let mut checked = 0;
    for i in 0..z.len() {
        let (a, b) = (
            file.reaches[0].levels[i],
            file.reaches[0].upper[0].levels[i],
        );
        let p = grid.point(i);
        if a > z[i] && b > z[i] && own[i] {
            assert!((map.row(1, p).level - f64::midpoint(a, b)).abs() < 1e-9);
            checked += 1;
        }
    }
    println!("{checked} cells of the spring-fed stream mixed halfway at its stage {g:.4} m");
    assert!(checked > 500);
    // A stage outside the samples is held at the range's end and counted.
    map.set_gauge(river, hi + 1.0).unwrap();
    assert_eq!(map.gauge(river), hi);
    assert_eq!(map.clamped_stages, 1);
}
