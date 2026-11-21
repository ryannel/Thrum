//! M2.3, ruling 1: a chart may declare a brink end, where the ground drops away below its level.
//! The map prepares such a chart, gives its own row up to the brink with no mix, and names no
//! body on the ground below; the same chart without the brink end is refused as before. The
//! chart below the fall has no source body. The expected rows are worked out here from the
//! chart's own levels and `WATERS`' bilinear weights.
#![expect(
    clippy::unwrap_used,
    reason = "test arithmetic; the test reports what it checked"
)]
use crate::support;
use support::fall::{Channel, Cliff, Foot, channel};
use thrum_river_model::{Profile, Solution, solve};
use thrum_water_model::{
    BRINK, Bed, Body, BodyKind, Chart, EndKind, Map, Profile as WaterProfile, Stage,
};

const DX: f64 = 0.25;

fn solved() -> (Channel, Solution, Bed) {
    let c = channel(
        DX,
        60,
        0.005,
        0.15,
        0.04,
        Cliff {
            height: 3.0,
            face_cells: 0,
        },
        Foot::Ground {
            slope: 0.02,
            cells: 40,
        },
    );
    let s = solve(&c.grid, &c.declaration, &Profile::default()).unwrap();
    let bed = Bed::new(
        c.grid.origin(),
        c.grid.spacing(),
        c.grid.size(),
        c.grid.heights().to_vec(),
    )
    .unwrap();
    (c, s, bed)
}

fn still(stage: f64) -> Stage {
    Stage {
        min: 0.0,
        max: stage,
        value: 0.0,
    }
}

fn chart_body(name: &str, chart: Chart) -> Body {
    Body {
        name: name.into(),
        kind: BodyKind::Chart(chart),
        level: 0.0,
        stage: still(0.0),
        fetch: 1e6,
        gains: [0.0; 4],
    }
}

fn lake(c: &Channel) -> Body {
    Body {
        name: "pool".into(),
        kind: BodyKind::Basin {
            seed: c.declaration.receivers[0].seed,
        },
        level: c.level,
        stage: still(0.0),
        fetch: 1e6,
        gains: [0.0; 4],
    }
}

const WATER: WaterProfile = WaterProfile {
    margin_height: 0.3,
    swash_ratio: 0.7,
};

#[test]
fn the_map_prepares_a_chart_with_a_brink_end_and_its_row_up_to_the_brink_is_its_own() {
    let (c, s, bed) = solved();
    let reaches = s.reaches().unwrap();
    assert_eq!(reaches.len(), 2);
    let upper = reaches[0].chart(&bed, |_| None).unwrap();
    let brinks: Vec<_> = upper.ends.iter().filter(|e| e.body == BRINK).collect();
    assert_eq!(brinks.len(), 1, "one straight lip");
    assert_eq!(
        upper.ends.len(),
        1,
        "a spring-fed reach above a fall has only its brink"
    );
    let map = Map::prepare(bed.clone(), vec![chart_body("upper", upper.clone())], WATER)
        .expect("the map takes a chart with a brink end");
    // On the lip cells and between them and the face, the row is the chart's own levels.
    for z in c.rows() {
        let lip = c.cell(c.lip, z);
        let p = c.grid.point(lip);
        for f in [0.0, 0.25, 0.5] {
            let q = [p[0] + f * DX, p[1]];
            let w = [(1.0 - f / 1.0, lip), (f, lip + 1)];
            let expected: f64 = w.iter().map(|&(a, i)| a * upper.levels[i]).sum();
            assert_eq!(map.row(0, q).level, expected, "row at {q:?}");
        }
        assert_eq!(map.row(0, p).current, upper.currents[lip]);
        let sample = map.sample(p).unwrap();
        assert_eq!(sample.body, 0);
        assert!(sample.depth > 0.0);
    }
    // The ground below the brink, from the face to the far end, is in no body.
    let mut asked = 0;
    for x in c.lip + 1..c.columns().end {
        for z in c.rows() {
            for half in [false, true] {
                let f = if half { 0.5 } else { 0.0 };
                let q = [
                    c.grid.point(c.cell(x, z))[0] - f * DX,
                    c.grid.point(c.cell(x, z))[1],
                ];
                if x == c.lip + 1 && half {
                    // The face itself: still no body.
                    assert!(
                        map.sample(q).is_none_or(|m| m.depth <= 0.0),
                        "face at {q:?}"
                    );
                } else {
                    assert!(map.sample(q).is_none(), "ground below at {q:?}");
                }
                asked += 1;
            }
        }
    }
    println!("upper reach with a brink end prepared; {asked} points below the brink in no body");
}

#[test]
fn the_same_chart_without_its_brink_end_is_refused_as_before() {
    let (_, s, bed) = solved();
    let mut upper = s.reaches().unwrap()[0].chart(&bed, |_| None).unwrap();
    upper.ends.retain(|e| e.body != BRINK);
    let refused = Map::prepare(bed, vec![chart_body("upper", upper)], WATER)
        .err()
        .unwrap();
    assert_eq!(refused.reason, "chart bank does not contain water");
}

#[test]
fn the_chart_below_a_fall_has_no_source_body_and_both_reaches_prepare_with_the_lake() {
    let (c, s, bed) = solved();
    let reaches = s.reaches().unwrap();
    assert!(reaches[1].ends.iter().all(|e| e.kind != EndKind::Source));
    let body = |name: &str| (name == "pool").then_some(0);
    let upper = reaches[0].chart(&bed, body).unwrap();
    let lower = reaches[1].chart(&bed, body).unwrap();
    assert!(lower.ends.iter().all(|e| e.mouth && e.body == 0));
    let map = Map::prepare(
        bed,
        vec![
            lake(&c),
            chart_body("upper", upper),
            chart_body("lower", lower.clone()),
        ],
        WATER,
    )
    .expect("the map takes both reaches and the lake");
    // Below the fall the water is the lower reach's, never the upper's.
    for x in c.foot..c.columns().end {
        for z in c.rows() {
            let i = c.cell(x, z);
            if let Some(m) = map.sample(c.grid.point(i)) {
                assert_ne!(m.body, 1, "the upper reach at {:?}", c.grid.point(i));
            }
            if s.reach_of[i] == Some(1) {
                assert_eq!(map.sample(c.grid.point(i)).unwrap().body, 2);
            }
        }
    }
    println!("lower reach: {} ends, no source", lower.ends.len());
}
