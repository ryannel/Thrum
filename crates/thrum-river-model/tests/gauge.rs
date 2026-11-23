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
use thrum_river_model::{Profile, Samples, key, key_for_samples, solve, solve_samples};
use thrum_water_model::{Body, Gauge, Map, RiverFile, Stage};

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

/// How far across the bed the second valley lies from the first.
const ACROSS: f64 = 16.0;

/// A cell's level in sample `k` of a reach.
fn reach_level(reach: &thrum_water_model::RiverReach, k: usize, i: usize) -> f64 {
    if k == 0 {
        reach.levels[i]
    } else {
        reach.upper[k - 1].levels[i]
    }
}

/// Two solved rivers on one bed: the lake's outlet, and a river whose source body is the
/// outlet's pool.
struct Two {
    up_samples: Samples,
    up: RiverFile,
    /// The upper river's reach that holds the pool, and the four cells round the place.
    pool: usize,
    cells: Vec<usize>,
    place: [f64; 2],
    /// The levels the pool takes at the place over the lake's range.
    range: [f64; 2],
    low: thrum_river_model::Grid,
    declaration: thrum_river_model::Declaration,
    down_samples: Samples,
    down: RiverFile,
    bed: thrum_water_model::Bed,
    map: Map,
    source_chart: usize,
    up_charts: Vec<usize>,
    down_charts: Vec<usize>,
}

/// The pool's level at the place at a lake level, from the upper river's stored samples alone.
fn pool_level(
    samples: &Samples,
    reach: &thrum_water_model::RiverReach,
    cells: &[usize],
    lake: f64,
) -> f64 {
    let k = usize::from(lake > samples.gauges[1]);
    let w = (lake - samples.gauges[k]) / (samples.gauges[k + 1] - samples.gauges[k]);
    cells
        .iter()
        .map(|&i| 0.25 * ((1.0 - w) * reach_level(reach, k, i) + w * reach_level(reach, k + 1, i)))
        .sum::<f64>()
}

fn two_rivers() -> Two {
    // The river above: the lake's outlet, whose gauge is the lake's level.
    let (grid, up_samples, up) = sampled(HIGH);
    let nx = grid.size()[0];
    // The place the river below leaves it: mid-pool, between four cell centres.
    let place = [15.0, CENTRE];
    let cells: Vec<usize> = [-0.125, 0.125]
        .into_iter()
        .flat_map(|dz| [-0.125, 0.125].map(|dx| grid.cell([place[0] + dx, place[1] + dz]).unwrap()))
        .collect();
    let pool = up
        .reaches
        .iter()
        .position(|r| cells.iter().all(|&i| r.footprint[i]))
        .unwrap();
    let range = RANGE.map(|lake| pool_level(&up_samples, &up.reaches[pool], &cells, lake));
    assert!(range[1] - range[0] > 0.2, "{range:?}");
    // The river below: the same valley beside the first, lowered so its sill lies 0.1 m under
    // the pool's lowest level, and solved at the pool's levels across their range. The two are
    // not joined in space: this is the gauge alone, read through the shared map. A still pond
    // stands at the second valley's head for the map's join there; nothing moves it.
    let drop = SILL_OVER - range[0];
    let (g0, mut declaration) = lake(DX, HIGH, [range[0] + drop, range[1] + drop]);
    let low = thrum_river_model::Grid::new(
        [g0.origin()[0], g0.origin()[1] + ACROSS],
        DX,
        g0.size(),
        g0.heights().iter().map(|h| h - drop).collect(),
    )
    .unwrap();
    declaration.name = "branch".into();
    let source = &mut declaration.sources[0];
    source.name.clone_from(&up.reaches[pool].name);
    source.seed[1] += ACROSS;
    source.extent[0][1] += ACROSS;
    source.extent[1][1] += ACROSS;
    source.level = range[0];
    source.range = range;
    let tarn = &mut declaration.receivers[0];
    tarn.name = "low tarn".into();
    tarn.seed[1] += ACROSS;
    tarn.extent[0][1] += ACROSS;
    tarn.extent[1][1] += ACROSS;
    tarn.level -= drop;
    let profile = Profile::default();
    let down_samples = solve_samples(&low, &declaration, &profile, &[]).unwrap();
    let down = down_samples
        .file(key_for_samples(&low, &declaration, &profile, &[]))
        .unwrap();
    assert_eq!(down.gauge_range(), range);
    assert!(down_samples.gauges.len() >= 2);

    // One bed and one map hold both.
    let mut heights = grid.heights().to_vec();
    heights.extend_from_slice(low.heights());
    let bed =
        thrum_water_model::Bed::new(grid.origin(), DX, [nx, 2 * grid.size()[1]], heights.clone())
            .unwrap();
    let still = Stage {
        min: 0.0,
        max: 0.0,
        value: 0.0,
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
        support::lake::basin("tarn", [30.0, CENTRE], support::lake::TARN, still),
        support::lake::basin(
            "low tarn",
            [30.0, CENTRE + ACROSS],
            support::lake::TARN - drop,
            still,
        ),
        support::lake::basin(
            "pond",
            [3.5, CENTRE + ACROSS],
            0.0,
            Stage {
                min: range[0],
                max: range[1],
                value: range[0],
            },
        ),
    ];
    let first = bodies.len();
    for r in &up.reaches {
        let chart = r
            .chart(&bed, |name| match name {
                "lake" => Some(0),
                "tarn" => Some(1),
                _ => None,
            })
            .unwrap();
        bodies.push(chart_body(&r.name, chart));
    }
    let up_charts: Vec<usize> = (first..bodies.len()).collect();
    let source_chart = first + pool;
    let pool_name = up.reaches[pool].name.clone();
    for r in &down.reaches {
        let chart = r
            .chart(&bed, |name| {
                if name == pool_name {
                    Some(3)
                } else {
                    (name == "low tarn").then_some(2)
                }
            })
            .unwrap();
        bodies.push(chart_body(&format!("low {}", r.name), chart));
    }
    let down_charts: Vec<usize> = (first + up.reaches.len()..bodies.len()).collect();
    let map = Map::prepare(bed.clone(), bodies, WATER).unwrap();
    Two {
        up_samples,
        up,
        pool,
        cells,
        place,
        range,
        low,
        declaration,
        down_samples,
        down,
        bed,
        map,
        source_chart,
        up_charts,
        down_charts,
    }
}

#[test]
fn a_river_whose_source_is_another_river_s_chart_follows_that_chart_s_level() {
    let Two {
        up_samples,
        up,
        pool,
        cells,
        place,
        range,
        low,
        declaration,
        down_samples,
        down,
        bed,
        mut map,
        source_chart,
        up_charts,
        down_charts,
    } = two_rivers();
    let n = low.heights().len();
    let profile = Profile::default();
    let pool_level = |lake: f64| pool_level(&up_samples, &up.reaches[pool], &cells, lake);
    let chart_gauge = Gauge::Chart {
        body: source_chart,
        place,
    };
    let branch = |gauge| down.river(&bed, "branch", &down_charts, gauge).unwrap();

    // Before the river above is in the map its chart is one sample, the lowest: a gauge that
    // reads it is refused only where it could not be read at all.
    let refused = |map: &mut Map, gauge, reason: &str| {
        let error = map.add_river(branch(gauge)).unwrap_err();
        assert!(format!("{error:?}").contains(reason), "{error:?}");
    };
    refused(&mut map, Gauge::Body(source_chart), "needs a place");
    refused(&mut map, Gauge::Chart { body: 0, place }, "not a chart");
    refused(
        &mut map,
        Gauge::Chart {
            body: source_chart,
            place: [15.0, CENTRE + 6.0],
        },
        "footprint",
    );
    refused(
        &mut map,
        Gauge::Chart {
            body: down_charts[0],
            place: [15.0, CENTRE + ACROSS],
        },
        "river's own",
    );
    let upper = map
        .add_river(
            up.river(&bed, "outlet", &up_charts, Gauge::Body(0))
                .unwrap(),
        )
        .unwrap();
    // A river whose samples do not cover the levels the pool can take is refused.
    let mut short = branch(chart_gauge);
    short.gauges.pop();
    for c in &mut short.charts {
        c.samples.pop();
        c.wetting.pop();
    }
    let error = map.add_river(short).unwrap_err();
    assert!(
        format!("{error:?}").contains("outside the samples"),
        "{error:?}"
    );
    let lower = map.add_river(branch(chart_gauge)).unwrap();
    assert!(map.set_gauge(lower, range[0]).is_err());

    // Cells of the river below that no join blends, and that every sample wets.
    let z = low.heights();
    let cell_of = |i: usize| i;
    let samples_of = |r: usize, k: usize, i: usize| reach_level(&down.reaches[r], k, i);
    let count = down_samples.gauges.len();
    let mut seen = Vec::new();
    let mut last_fall = 0.0;
    for step in 0..=12 {
        let lake_level = RANGE[0] + (RANGE[1] - RANGE[0]) * f64::from(step) / 12.0;
        map.set_stage(0, lake_level).unwrap();
        assert_eq!(map.gauge(upper), lake_level);
        // The gauge of the river below is the pool's level there: the map's own row of the
        // chart above, and the mix worked out here from the upper river's stored samples.
        let g = map.gauge(lower);
        assert_eq!(g, map.row(source_chart, place).level);
        assert!((g - pool_level(lake_level)).abs() < 1e-9, "{g}");
        if let Some(&before) = seen.last() {
            assert!(
                g > before,
                "the lower gauge rises with the lake: {before} then {g}"
            );
        }
        seen.push(g);
        // Every chart of the river below is the mix of its own samples at that gauge.
        let k = (0..count - 1)
            .rev()
            .find(|&k| down_samples.gauges[k] <= g)
            .unwrap_or(0);
        let w = ((g - down_samples.gauges[k])
            / (down_samples.gauges[k + 1] - down_samples.gauges[k]))
            .clamp(0.0, 1.0);
        let mut checked = 0;
        for (r, &body) in down_charts.iter().enumerate() {
            for i in 0..n {
                let j = cell_of(i);
                let p = low.point(i);
                // Away from the reach's ends, where a join blends the row.
                if !(10.0..24.0).contains(&p[0]) {
                    continue;
                }
                let (a, b) = (samples_of(r, k, i), samples_of(r, k + 1, i));
                if a > z[j] && b > z[j] {
                    let expected = (1.0 - w) * a + w * b;
                    let row = map.row(body, p);
                    assert!((row.level - expected).abs() < 1e-9, "{p:?}");
                    checked += 1;
                }
            }
        }
        assert!(checked > 300, "{checked}");
        // Its falls are read at the same gauge, and run harder as the lake rises.
        let falls = down.falls_at(g);
        let fall: f64 = falls
            .iter()
            .flat_map(|f| &f.segments)
            .map(|s| s.discharge)
            .sum();
        assert!(fall > last_fall, "{last_fall} then {fall}");
        last_fall = fall;
    }
    // Between its samples the river below is near a solve at the level its gauge reads.
    let lake_level = 3.25;
    map.set_stage(0, lake_level).unwrap();
    let g = map.gauge(lower);
    assert!(
        down_samples.gauges.iter().all(|s| (s - g).abs() > 0.05),
        "{g} against {:?}",
        down_samples.gauges
    );
    let mut at = declaration.clone();
    at.sources[0].level = g;
    let fresh = solve(&low, &at, &profile).unwrap();
    let sp = |c: [f64; 2]| c[0].hypot(c[1]);
    let (mut depth, mut speed, mut cells_checked) = (0.0_f64, 0.0_f64, 0);
    for (r, &body) in down_charts.iter().enumerate() {
        for i in 0..n {
            let p = low.point(i);
            let every = (0..count).all(|k| samples_of(r, k, i) > z[cell_of(i)]);
            if !every || !fresh.wet[i] || !(10.0..24.0).contains(&p[0]) {
                continue;
            }
            let row = map.row(body, p);
            let u = sp(fresh.current(i));
            depth = depth.max((row.level - fresh.level(i)).abs());
            speed = speed.max((sp(row.current) - u).abs() / u);
            cells_checked += 1;
        }
    }
    println!(
        "a river below a pool: the lake from {:.2} to {:.2} m moves the pool, and so the lower river's \
         gauge, from {:.3} to {:.3} m; its falls reach {last_fall:.3} m3/s; at a lake of {lake_level} m \
         (gauge {g:.3} m) {cells_checked} cells of its mix are off a fresh solve by {:.1} mm and {:.1}%",
        RANGE[0],
        RANGE[1],
        seen[0],
        seen[seen.len() - 1],
        1000.0 * depth,
        100.0 * speed
    );
    assert!(cells_checked > 300, "{cells_checked}");
    assert!(depth <= 0.01, "{depth}");
    assert!(speed <= 0.25, "{speed}");
    // A chart an earlier river's gauge reads cannot join a later river.
    let mut again = up.river(&bed, "again", &up_charts, Gauge::Body(0)).unwrap();
    again.charts.retain(|c| c.body == source_chart);
    let error = map.add_river(again).unwrap_err();
    assert!(
        format!("{error:?}").contains("already in a river"),
        "{error:?}"
    );
}

/// The level of the lowest sample over the second valley's sill, before it is lowered.
const SILL_OVER: f64 = 3.1;

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
    let chart = thrum_water_model::Chart {
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
    let sample = |level: f64| thrum_water_model::FlowSample {
        levels: vec![level; n],
        currents: vec![[0.0; 2]; n],
        loss: vec![0.0; n],
        gains: vec![[0.0; 2]; n],
    };
    let river = |upper: thrum_water_model::FlowSample, name: &str| thrum_water_model::River {
        name: name.into(),
        gauges: vec![1.0, 3.0],
        gauge: Gauge::Stage(Stage {
            min: 1.0,
            max: 3.0,
            value: 1.0,
        }),
        charts: vec![thrum_water_model::ChartSamples {
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
