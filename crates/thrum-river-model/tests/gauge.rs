//! M3.2, ruling 2: a river chart's row is the mix, linear in the gauge, of the two samples that
//! bracket it; the gauge is the source body's level where the river has one and the river's own
//! stage where it has none; every chart of a river takes the same gauge. Between samples 0.3 m
//! apart the mixed row is within 1 cm in depth and 25% in speed of a solve at that level, in the
//! cells both samples wet, as part 3 of the design proof compares them; a chart left at one
//! sample with its level raised is not. As the gauge moves 1 cm the waterline on the channel's
//! banks, of 6 in 1, moves by at most one cell. A chart with one sample answers to the bit as
//! before, and its stage adds to its level. A source body may be another river's chart: the river
//! below then takes that chart's level at the place it leaves it, so the lake that moves the
//! river above moves the river below, its charts and its falls, through the one map.
#![expect(
    clippy::unwrap_used,
    clippy::float_cmp,
    reason = "test arithmetic; the test reports what it checked"
)]
use crate::support;
use support::fall_samples;
use support::lake::{
    CENTRE, HIGH, WATER, bed_of, chart_body, lake, sampled, solve_at_gauges, water,
};
use thrum_river_model::{Declaration, Grid, Manning, Profile, Receiver, Source, key, solve};
use thrum_water_model::{
    Body, Chart, ChartSamples, FlowSample, Gauge, Map, River, RiverFile, Stage,
};

const DX: f64 = 0.25;
const RANGE: [f64; 2] = [3.1, 3.7];

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
    grid: &Grid,
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
    let (grid, mut declaration) = lake(0.5, HIGH, RANGE);
    let profile = Profile::default();
    let (samples, file) = solve_at_gauges(&grid, &declaration, &profile);
    let z = grid.heights();
    let (mut map, river) = water(&grid, &file, RANGE, RANGE[0], true);
    let river = river.unwrap();
    let chart = 2;
    for (k, g) in [(0, 3.25), (1, 3.55)] {
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
        // Keep the minimum checked area as the grid gets coarser.
        assert!(f64::from(cells) * grid.spacing().powi(2) > 300.0 * DX.powi(2));
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
    // The outlet and its terrace both take the lake's gauge.
    let (grid, file) = fall_samples::prepared();
    let gauges = file.gauges();
    assert!(file.reaches.len() >= 2, "{}", file.reaches.len());
    let (mut map, river) = fall_samples::water(&grid, &file, true);
    let river = river.unwrap();
    let z = grid.heights();
    let g = 3.6;
    let w = (g - gauges[1]) / (gauges[2] - gauges[1]);
    let mut checked = vec![0; file.reaches.len()];
    for (r, reach) in file.reaches.iter().enumerate() {
        assert_eq!(map.river_of(2 + r), Some((river, r)));
        // Cells no join blends at either sample.
        let mut own = vec![true; z.len()];
        for (gauge, row) in [(gauges[1], &reach.upper[0]), (gauges[2], &reach.upper[1])] {
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
    // Take the prepared main reach with two flow samples and give it its own stage.
    let (grid, mut file) = fall_samples::prepared();
    file.reaches.truncate(1);
    file.reaches[0].upper.truncate(1);
    file.upper.truncate(1);
    let [lo, hi] = file.gauge_range();
    let (mut map, _) = fall_samples::water(&grid, &file, false);
    let chart = 2;
    let river = map
        .add_river(
            file.river(
                map.bed(),
                "stream",
                &[chart],
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
            *o &= map.row(chart, grid.point(i)).level == levels[i];
        }
    }
    let g = f64::midpoint(lo, hi);
    map.set_gauge(river, g).unwrap();
    assert_eq!(map.gauge(river), g);
    map.set_stage(0, RANGE[1]).unwrap();
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
            assert!((map.row(chart, p).level - f64::midpoint(a, b)).abs() < 1e-9);
            checked += 1;
        }
    }
    println!("{checked} cells of the spring-fed stream mixed halfway at its stage {g:.4} m");
    assert_eq!(checked, 11, "all cells wet in both samples");
    // A stage outside the samples is held at the range's end and counted.
    map.set_gauge(river, hi + 1.0).unwrap();
    assert_eq!(map.gauge(river), hi);
    assert_eq!(map.clamped_stages, 1);
}

const SOURCE_CHART: usize = 3;
const PLACE: [f64; 2] = [5.25, 2.75];
// Bilinear weights at PLACE, in the row-major order of the four pool cells.
const POOL_WEIGHTS: [f64; 4] = [3.0 / 16.0, 1.0 / 16.0, 9.0 / 16.0, 3.0 / 16.0];
const LAKE_GAUGES: [f64; 3] = [RANGE[0], 3.4, RANGE[1]];
const POOL_LEVELS: [f64; 3] = [10.14, 10.30, 10.50];
const POOL_OFFSETS: [f64; 4] = [-0.05, 0.03, -0.01, 0.07];

/// Prepared upper pool on the map's grid. Its four wet cells surround the
/// gauge place, with distinct levels along both axes at all three samples.
fn upper_pool(grid: &Grid, cells: [usize; 4]) -> (Body, River) {
    let n = grid.heights().len();
    let samples: Vec<_> = POOL_LEVELS
        .iter()
        .map(|&level| {
            let mut levels: Vec<_> = grid.heights().iter().map(|h| h - 0.05).collect();
            for (i, offset) in cells.into_iter().zip(POOL_OFFSETS) {
                levels[i] = level + offset;
            }
            FlowSample {
                levels,
                currents: vec![[0.0; 2]; n],
                loss: vec![0.0; n],
                gains: vec![[0.0; 2]; n],
            }
        })
        .collect();
    let pool = chart_body(
        "outlet pool",
        Chart {
            levels: samples[0].levels.clone(),
            currents: samples[0].currents.clone(),
            footprint: (0..n).map(|i| cells.contains(&i)).collect(),
            ends: vec![],
        },
    );
    let river = River {
        name: "outlet".into(),
        gauges: LAKE_GAUGES.to_vec(),
        gauge: Gauge::Body(0),
        charts: vec![ChartSamples {
            body: SOURCE_CHART,
            samples,
            wetting: vec![vec![None; n]; 2],
        }],
    };
    (pool, river)
}

/// The lower reach has a pond at its source join and a cliff into the tarn.
/// The pond supplies the solver's boundary level. The map reads the moving
/// gauge independently, from the upper pool across the bank.
fn branch() -> (Grid, Declaration) {
    let grid = Grid::new(
        [0.0, 6.0],
        1.0,
        [25, 10],
        (0..250)
            .map(|i| match (i % 25, i / 25) {
                (3..=5, 3..=6) => 9.0,
                (6..=13, 3..=6) => 10.0,
                (14..=21, 3..=6) => 7.0,
                _ => 15.0,
            })
            .collect(),
    )
    .unwrap();
    let declaration = Declaration {
        name: "branch".into(),
        springs: vec![],
        sources: vec![Source {
            name: "pond".into(),
            seed: [4.0, 10.0],
            extent: [[2.5, 8.5], [5.5, 12.5]],
            level: POOL_LEVELS[0],
            range: [POOL_LEVELS[0], POOL_LEVELS[2]],
        }],
        receivers: vec![Receiver {
            name: "tarn".into(),
            seed: [19.0, 10.0],
            extent: [[13.5, 8.5], [21.5, 12.5]],
            level: 8.0,
        }],
        manning: Manning::Uniform(0.04),
    };
    (grid, declaration)
}

fn chained_map(
    low: &Grid,
    declaration: &Declaration,
    down: &RiverFile,
) -> (Map, River, Vec<usize>) {
    let [nx, nz] = low.size();
    let mut heights = vec![15.0; nx * 6];
    let pool_cells = [2 * nx + 5, 2 * nx + 6, 3 * nx + 5, 3 * nx + 6];
    for i in pool_cells {
        heights[i] = 9.0;
    }
    heights[2 * nx + 2] = 2.0;
    heights.extend_from_slice(low.heights());
    let grid = Grid::new([0.0; 2], 1.0, [nx, 6 + nz], heights).unwrap();
    let bed = bed_of(&grid);
    let (pool, upper) = upper_pool(&grid, pool_cells);
    let stage = |[min, max]: [f64; 2]| Stage {
        min,
        max,
        value: min,
    };
    let mut bodies = vec![
        support::lake::basin("lake", [2.0; 2], 0.0, stage(RANGE)),
        support::lake::basin(
            "pond",
            declaration.sources[0].seed,
            0.0,
            stage(down.gauge_range()),
        ),
        support::lake::basin("tarn", declaration.receivers[0].seed, 8.0, stage([0.0; 2])),
        pool,
    ];
    for reach in &down.reaches {
        let chart = reach
            .chart(&bed, |name| match name {
                "pond" => Some(1),
                "tarn" => Some(2),
                _ => None,
            })
            .unwrap();
        bodies.push(chart_body(&reach.name, chart));
    }
    let down_charts = (SOURCE_CHART + 1..bodies.len()).collect();
    (
        Map::prepare(bed, bodies, WATER).unwrap(),
        upper,
        down_charts,
    )
}

fn assert_chart_mix(
    map: &Map,
    file: &RiverFile,
    grid: &Grid,
    charts: &[usize],
    gauge: f64,
) -> usize {
    let gauges = file.gauges();
    let k = support::lake::lower_sample(&gauges, gauge);
    let w = (gauge - gauges[k]) / (gauges[k + 1] - gauges[k]);
    let mut total = 0;
    for (r, &body) in charts.iter().enumerate() {
        let mut checked = 0;
        for (i, &bed) in grid.heights().iter().enumerate() {
            let p = grid.point(i);
            let (a, ca) = support::lake::stored(file, r, k, i);
            let (b, cb) = support::lake::stored(file, r, k + 1, i);
            if a > bed && b > bed && support::lake::unjoined(file, r, p) {
                let row = map.row(body, p);
                assert!((row.level - ((1.0 - w) * a + w * b)).abs() < 1e-9, "{p:?}");
                for axis in 0..2 {
                    assert!(
                        (row.current[axis] - ((1.0 - w) * ca[axis] + w * cb[axis])).abs() < 1e-9,
                        "{p:?}"
                    );
                }
                checked += 1;
            }
        }
        assert!(checked > 0, "chart {body} has no wet, unblended cells");
        total += checked;
    }
    total
}

#[test]
fn a_river_whose_source_is_another_river_s_chart_follows_that_chart_s_level() {
    let (low, declaration) = branch();
    let profile = Profile {
        threads: 1,
        ..Profile::default()
    };
    let (samples, down) = solve_at_gauges(&low, &declaration, &profile);
    let gauges = &samples.gauges;
    assert_eq!(gauges.len(), 3);
    println!(
        "lower flow samples at {gauges:?} m take {:?} steps",
        samples
            .solutions
            .iter()
            .map(|s| s.report.steps)
            .collect::<Vec<_>>(),
    );
    let (mut map, outlet, down_charts) = chained_map(&low, &declaration, &down);
    let bed = map.bed().clone();
    let chart_gauge = Gauge::Chart {
        body: SOURCE_CHART,
        place: PLACE,
    };
    let branch = |gauge| down.river(&bed, "branch", &down_charts, gauge).unwrap();
    let refused = |map: &mut Map, gauge, reason: &str| {
        let error = map.add_river(branch(gauge)).unwrap_err();
        assert!(error.reason.contains(reason), "{error:?}");
    };
    refused(&mut map, Gauge::Body(SOURCE_CHART), "needs a place");
    refused(
        &mut map,
        Gauge::Chart {
            body: 0,
            place: PLACE,
        },
        "not a chart",
    );
    refused(
        &mut map,
        Gauge::Chart {
            body: SOURCE_CHART,
            place: [5.5, 3.5],
        },
        "footprint",
    );
    refused(
        &mut map,
        Gauge::Chart {
            body: down_charts[0],
            place: [8.0, 10.0],
        },
        "river's own",
    );
    let upper = map.add_river(outlet.clone()).unwrap();
    let mut short = branch(chart_gauge);
    short.gauges.pop();
    for chart in &mut short.charts {
        chart.samples.pop();
        chart.wetting.pop();
    }
    let error = map.add_river(short).unwrap_err();
    assert!(error.reason.contains("outside the samples"), "{error:?}");
    let lower = map.add_river(branch(chart_gauge)).unwrap();
    let error = map.set_gauge(lower, POOL_LEVELS[0]).unwrap_err();
    assert!(error.reason.contains("source"), "{error:?}");
    let error = map.add_river(outlet).unwrap_err();
    assert!(error.reason.contains("already in a river"), "{error:?}");

    let mut last_gauge = 0.0;
    let mut last_fall = 0.0;
    let mut intervals = [false; 2];
    for lake in [RANGE[0], 3.25, 3.4, 3.55, RANGE[1]] {
        map.set_stage(0, lake).unwrap();
        assert_eq!(map.gauge(upper), lake);
        let k = support::lake::lower_sample(&LAKE_GAUGES, lake);
        let w = (lake - LAKE_GAUGES[k]) / (LAKE_GAUGES[k + 1] - LAKE_GAUGES[k]);
        let expected: f64 = POOL_OFFSETS
            .iter()
            .zip(POOL_WEIGHTS)
            .map(|(&offset, weight)| {
                weight * ((1.0 - w) * (POOL_LEVELS[k] + offset) + w * (POOL_LEVELS[k + 1] + offset))
            })
            .sum();
        let gauge = map.gauge(lower);
        assert_eq!(gauge, map.row(SOURCE_CHART, PLACE).level);
        assert!((gauge - expected).abs() < 1e-9, "{gauge} != {expected}");
        assert!(gauge > last_gauge, "{last_gauge} then {gauge}");
        last_gauge = gauge;
        for (k, interval) in gauges.windows(2).enumerate() {
            intervals[k] |= gauge > interval[0] + 1e-9 && gauge < interval[1] - 1e-9;
        }
        let checked = assert_chart_mix(&map, &down, &low, &down_charts, gauge);
        let fall: f64 = down
            .falls_at(gauge)
            .iter()
            .flat_map(|f| &f.segments)
            .map(|s| s.discharge)
            .sum();
        assert!(fall > last_fall, "{last_fall} then {fall}");
        last_fall = fall;
        println!(
            "lake {lake:.2} m: lower gauge {gauge:.3} m, {checked} unblended cells, fall discharge {fall:.4} m³/s"
        );
    }
    assert!(intervals.into_iter().all(|seen| seen));

    map.set_stage(0, 3.25).unwrap();
    let gauge = map.gauge(lower);
    assert!(
        gauges.iter().all(|s| (s - gauge).abs() > 0.05),
        "{gauge} against {gauges:?}"
    );
    let mut at = declaration;
    at.sources[0].level = gauge;
    let fresh = solve(&low, &at, &profile).unwrap();
    let speed = |v: [f64; 2]| v[0].hypot(v[1]);
    let (mut depth_error, mut speed_error, mut checked) = (0.0_f64, 0.0_f64, 0);
    for (r, &body) in down_charts.iter().enumerate() {
        for (i, &bed) in low.heights().iter().enumerate() {
            let p = low.point(i);
            let wet = (0..gauges.len()).all(|k| support::lake::stored(&down, r, k, i).0 > bed);
            if wet && fresh.wet[i] && support::lake::unjoined(&down, r, p) {
                let row = map.row(body, p);
                let u = speed(fresh.current(i));
                assert!(u > 0.0, "{p:?}");
                depth_error = depth_error.max((row.level - fresh.level(i)).abs());
                speed_error = speed_error.max((speed(row.current) - u).abs() / u);
                checked += 1;
            }
        }
    }
    println!(
        "fresh solve at {gauge:.3} m: {} steps, {checked} cells, {:.1} mm depth error and {:.1}% speed error",
        fresh.report.steps,
        depth_error * 1000.0,
        speed_error * 100.0,
    );
    assert!(checked > 0);
    assert!(depth_error <= 0.01, "{depth_error}");
    assert!(speed_error <= 0.25, "{speed_error}");
}

#[test]
fn a_refused_river_leaves_the_map_answering_as_before() {
    // A chart in a trough three cells square with a low bank on one side, as the second
    // review's probe made it: the upper sample dries most of it and overtops one bank.
    let n = 49;
    let mut heights = vec![2.0; n];
    let mut footprint = vec![false; n];
    for z in 2..=4 {
        for x in 2..=4 {
            heights[z * 7 + x] = if x == 4 { 1.1 } else { 0.0 };
            footprint[z * 7 + x] = true;
        }
    }
    let bed = thrum_water_model::Bed::new([0.0; 2], 1.0, [7, 7], heights).unwrap();
    let chart = Chart {
        levels: vec![1.0; n],
        currents: vec![[0.0; 2]; n],
        footprint,
        ends: vec![],
    };
    let mut map = Map::prepare(bed, vec![chart_body("reach", chart)], WATER).unwrap();
    let points: Vec<[f64; 2]> = (0..=60)
        .flat_map(|z| (0..=60).map(move |x| [f64::from(x) / 10.0, f64::from(z) / 10.0]))
        .collect();
    let answers = |map: &Map| -> Vec<_> {
        points
            .iter()
            .map(|&p| (map.sample(p), map.row(0, p), map.flow(0, p)))
            .collect()
    };
    let before = answers(&map);
    let sample = |level: f64| FlowSample {
        levels: vec![level; n],
        currents: vec![[0.0; 2]; n],
        loss: vec![0.0; n],
        gains: vec![[0.0; 2]; n],
    };
    let river = |upper: FlowSample, name: &str| River {
        name: name.into(),
        gauges: vec![1.0, 3.0],
        gauge: Gauge::Stage(Stage {
            min: 1.0,
            max: 3.0,
            value: 1.0,
        }),
        charts: vec![ChartSamples {
            body: 0,
            samples: vec![sample(1.0), upper],
            wetting: vec![vec![None; n]],
        }],
    };
    let mut overtopping = sample(-1.0);
    overtopping.levels[2 * 7 + 2] = 3.0;
    let mut nonfinite = sample(1.0);
    nonfinite.loss[0] = f64::NAN;
    let mut refused = 0;
    for (upper, reason) in [(overtopping, "bank"), (nonfinite, "invalid sample grid")] {
        let error = map.add_river(river(upper, "refused")).unwrap_err();
        assert!(format!("{error:?}").contains(reason), "{error:?}");
        assert!(map.river(0).is_none());
        assert_eq!(map.river_of(0), None);
        let after = answers(&map);
        let changed = before.iter().zip(&after).filter(|(a, b)| a != b).count();
        assert_eq!(changed, 0, "{reason}");
        refused += 1;
    }
    // The map still takes a river its banks hold, and then answers at its gauge.
    let id = map.add_river(river(sample(1.05), "held")).unwrap();
    map.set_gauge(id, 3.0).unwrap();
    assert_eq!(map.row(0, [3.0, 3.0]).level, 1.05);
    println!(
        "{refused} refused rivers: {} samples, rows and flow rows each the same as before",
        before.len()
    );
}
