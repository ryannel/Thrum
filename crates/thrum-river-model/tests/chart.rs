//! M1.6: the solve's reach as a chart `thrum-water-model` declares, prepared in the map with a
//! basin at its mouth. The expected joined row is worked out here from `WATERS`' documented
//! join weight and the two rows, at every footprint cell, with the basin's stage at both ends
//! of its range.
#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::unwrap_used,
    reason = "test arithmetic over cell indices"
)]
use crate::support;
use support::valley::{self, HOLLOW, LAKE};
use thrum_river_model::{Profile, Solution, solve};
use thrum_water_model::{Bed, Body, BodyKind, Chart, Map, Profile as WaterProfile, Stage};

const MARGIN_HEIGHT: f64 = 0.3;
const RISE: f64 = 0.3;

fn smooth(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

fn distance(segment: [[f64; 2]; 2], p: [f64; 2]) -> f64 {
    let [a, b] = segment;
    let d = [b[0] - a[0], b[1] - a[1]];
    let t = (((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / (d[0] * d[0] + d[1] * d[1]))
        .clamp(0.0, 1.0);
    (p[0] - a[0] - t * d[0]).hypot(p[1] - a[1] - t * d[1])
}

fn prepared() -> (Solution, Bed, Chart, Map) {
    let (grid, declaration) = valley::valley(0.5);
    let s = solve(&grid, &declaration, &Profile::default()).unwrap();
    let bed = Bed::new(
        grid.origin(),
        grid.spacing(),
        grid.size(),
        grid.heights().to_vec(),
    )
    .unwrap();
    let chart = s.chart(&bed, |name| (name == "lake").then_some(0)).unwrap();
    let lake = Body {
        name: "lake".into(),
        kind: BodyKind::Basin {
            seed: [45.0, valley::CENTRE],
        },
        level: LAKE,
        stage: Stage {
            min: 0.0,
            max: RISE,
            value: 0.0,
        },
        fetch: 1e6,
        gains: [0.0; 4],
    };
    let flow = Body {
        name: "valley".into(),
        kind: BodyKind::Chart(chart.clone()),
        level: 0.0,
        stage: Stage {
            min: 0.0,
            max: 0.0,
            value: 0.0,
        },
        fetch: 1e6,
        gains: [0.0; 4],
    };
    let profile = WaterProfile {
        margin_height: MARGIN_HEIGHT,
        swash_ratio: 0.7,
    };
    let map = Map::prepare(bed.clone(), vec![lake, flow], profile)
        .expect("the map takes the solved chart");
    (s, bed, chart, map)
}

#[test]
fn the_stored_chart_is_the_solve_and_its_wet_cells_are_the_solvers() {
    let (s, _, chart, map) = prepared();
    let z = s.grid().heights();
    let mut wet = 0;
    for (i, _) in z.iter().enumerate() {
        let chart_wet = chart.footprint[i] && chart.levels[i] > z[i];
        if s.is_held(i) {
            // The lake's cells the river runs into are the lake's water in the solve.
            assert_eq!(chart.footprint[i], chart_wet, "held cell {i}");
            continue;
        }
        assert_eq!(chart_wet, s.wet[i], "cell {i} at {:?}", s.grid().point(i));
        if s.wet[i] {
            wet += 1;
            assert_eq!(chart.levels[i], s.level(i));
            assert_eq!(chart.currents[i], s.current(i));
        } else {
            assert!(
                chart.levels[i] < z[i],
                "a dry cell's level lies under its bed"
            );
            assert_eq!(chart.currents[i], [0.0; 2]);
        }
    }
    println!("{wet} wet cells, all the solver's");
    // The hollow, cut off by ground, is dry in the chart and in the map.
    for (i, _) in z.iter().enumerate() {
        let p = s.grid().point(i);
        if (p[0] - HOLLOW[0]).hypot(p[1] - HOLLOW[1]) < 1.0 {
            assert!(!(chart.footprint[i] && chart.levels[i] > z[i]));
            assert!(
                map.sample(p).is_none_or(|m| m.depth <= 0.0),
                "the map holds water in the hollow at {p:?}"
            );
        }
    }
}

#[test]
fn the_map_takes_the_chart_and_joins_it_to_the_lake_at_both_ends_of_its_stage() {
    let (s, bed, chart, mut map) = prepared();
    let z = s.grid().heights();
    let end = &chart.ends[0];
    assert!(end.mouth && end.body == 0 && chart.ends.len() == 1);
    for stage in [0.0, RISE] {
        map.set_stage(0, stage).unwrap();
        let basin = LAKE + stage;
        let (mut joined, mut plain) = (0, 0);
        for (i, _) in z.iter().enumerate() {
            if !chart.footprint[i] {
                continue;
            }
            let p = s.grid().point(i);
            let own = chart.levels[i];
            let mut w = 1.0 - smooth(distance(end.segment, p) / end.length);
            w = w.max(1.0 - smooth((own - basin - 0.175 * MARGIN_HEIGHT) / 0.25));
            let (level, current) = if w >= 1.0 {
                (basin, [0.0; 2])
            } else {
                (
                    own * (1.0 - w) + basin * w,
                    chart.currents[i].map(|c| c * (1.0 - w)),
                )
            };
            let row = map.row(1, p);
            assert!(
                (row.level - level).abs() < 1e-12,
                "stage {stage}, cell {i} at {p:?}: {} against {level}",
                row.level
            );
            assert!(
                (row.current[0] - current[0]).abs() < 1e-12
                    && (row.current[1] - current[1]).abs() < 1e-12
            );
            if w == 0.0 {
                plain += 1;
                assert_eq!(row.level, own);
                assert_eq!(row.current, chart.currents[i]);
            } else {
                joined += 1;
            }
            // Across the mouth the river's row is the lake's: no step.
            if distance(end.segment, p) < 1e-9 {
                assert_eq!(row.level, map.row(0, p).level);
            }
        }
        println!("stage {stage}: {plain} cells the chart's own, {joined} joined to the lake");
        assert!(joined > 0 && plain > 0);
    }
    // The waterline is where the level meets the bed: across the channel at four places.
    map.set_stage(0, 0.0).unwrap();
    let inside = |p: [f64; 2]| {
        let g = s.grid();
        let fx = ((p[0] - g.origin()[0]) / g.spacing()).floor() as usize;
        let fz = ((p[1] - g.origin()[1]) / g.spacing()).floor() as usize;
        let nx = g.size()[0];
        [0, 1, nx, nx + 1]
            .iter()
            .all(|&k| chart.footprint[fz * nx + fx + k])
    };
    for x in [5.06, 14.06, 27.06, 33.06] {
        let (mut wet, mut dry) = (0, 0);
        for k in 0..650 {
            let p = [x, 0.5 + 0.02 * f64::from(k)];
            if !inside(p) {
                continue;
            }
            let water = map.row(1, p).level > bed.height(p).unwrap();
            let sampled = map.sample(p).is_some_and(|m| m.depth > 0.0);
            assert_eq!(water, sampled, "waterline at {p:?}");
            if water { wet += 1 } else { dry += 1 }
        }
        assert!(
            wet > 0 && dry > 0,
            "the transect at x = {x} crosses the waterline"
        );
    }
}
