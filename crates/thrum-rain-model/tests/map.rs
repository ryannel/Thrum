//! M1.6, ruling 1 on the CPU: the map takes a hollows body and answers by the point test,
//! with no current and no wind gain; its identity and generation hold through a merge; it
//! refuses a body in another's water or margin and one over the profile's counts; and it
//! holds a table that stands two levels in wet cells that touch.
use crate::support;
use support::{mm_h, pair, prepare};
use thrum_rain_model::{GroundClass, Rain, RainState, TickReport, Tree};
use thrum_water_model::{Bed, Body, BodyKind, Hollows, HollowsProfile, Map, Profile, Stage};

fn profile() -> Profile {
    Profile {
        margin_height: 0.2,
        swash_ratio: 0.7,
    }
}

fn body(name: &str, kind: BodyKind, level: f64, stage: Stage) -> Body {
    Body {
        name: name.into(),
        kind,
        level,
        stage,
        fetch: 1e3,
        gains: [1.0; 4],
    }
}

fn still() -> Stage {
    Stage {
        min: 0.0,
        max: 0.0,
        value: 0.0,
    }
}

/// The map's own bed: 20 m square at 1 m, low ground at x < 6 and high ground beyond.
fn map_bed() -> Bed {
    let heights = (0..20 * 20)
        .map(|i| if i % 20 < 6 { -1.0 } else { 1.0 })
        .collect();
    Bed::new([-10.0, -10.0], 1.0, [20, 20], heights).expect("valid test case")
}

fn declare(tree: &Tree, levels: &[f64]) -> Body {
    body(
        "hollows",
        BodyKind::Hollows(tree.water_hollows(levels).expect("valid test case")),
        0.0,
        still(),
    )
}

#[test]
fn the_map_answers_by_the_point_test_through_a_merge() {
    let tree = prepare(&pair());
    assert_eq!(tree.hollow_count(), 2);
    let g = tree.ground().clone();
    let mut state = RainState::new(&tree);
    let mut map = Map::prepare(
        map_bed(),
        vec![declare(&tree, &state.levels(&tree))],
        profile(),
    )
    .expect("valid test case");
    let id = 0;
    let generation = map.generation(id);
    let mut report = TickReport::default();
    let mut merged_at = None;
    for t in 0..3000 {
        state
            .step(
                &tree,
                Rain {
                    flux: mm_h(60.0),
                    drying: 0.0,
                },
                GroundClass::bare(0.0),
                1.0,
                1,
                &mut report,
            )
            .expect("valid test case");
        let levels = state.levels(&tree);
        assert!(
            map.set_levels(id, &levels).expect("valid test case"),
            "the rain model's table is never held"
        );
        if merged_at.is_none()
            && state
                .puddles()
                .any(|(k, _)| tree.node(k).children.is_some())
        {
            merged_at = Some(t);
        }
        if t % 50 != 0 && merged_at != Some(t) {
            continue;
        }
        // Every cell's centre, and a point near its corner, answer by the point test.
        for i in 0..g.heights().len() {
            let [x, z] = g.centre(i);
            for p in [[x, z], [x - 0.024, z + 0.024]] {
                let label = tree.labels()[i];
                let wet = label != thrum_water_model::NO_HOLLOW
                    && levels[label as usize] > g.heights()[i];
                let s = map.sample(p);
                if wet {
                    let s = s.expect("a puddle answers");
                    assert_eq!(s.body, id);
                    assert_eq!(s.row.level, levels[label as usize]);
                    assert_eq!(s.bed, Some(g.heights()[i]));
                    assert_eq!(s.depth, s.row.level - g.heights()[i]);
                    assert_eq!(s.row.current, [0.0; 2]);
                    assert_eq!(
                        s.row.gains, [0.0; 4],
                        "no wind gain, though the body declared gains"
                    );
                } else if let Some(s) = s {
                    // Dry: at most a margin beside water, below its level.
                    assert_eq!(s.body, id);
                    assert!(s.depth <= 0.0);
                }
            }
        }
        assert_eq!(map.generation(id), generation);
    }
    assert!(merged_at.is_some(), "the two puddles join");
    assert_eq!(map.held_level_tables, 0);
    // Outside the rectangle the map's own bed answers as before: nothing here.
    assert!(map.sample([5.0, 5.0]).is_none());
}

#[test]
fn a_table_with_two_levels_in_touching_wet_cells_is_held() {
    let tree = prepare(&pair());
    let mut state = RainState::new(&tree);
    let mut report = TickReport::default();
    let mut map = Map::prepare(
        map_bed(),
        vec![declare(&tree, &state.levels(&tree))],
        profile(),
    )
    .expect("valid test case");
    // Fill until they have joined, so wet cells of both hollows touch across the sill.
    while !state
        .puddles()
        .any(|(k, _)| tree.node(k).children.is_some())
    {
        state
            .step(
                &tree,
                Rain {
                    flux: mm_h(60.0),
                    drying: 0.0,
                },
                GroundClass::bare(0.0),
                1.0,
                1,
                &mut report,
            )
            .expect("valid test case");
    }
    for _ in 0..60 {
        state
            .step(
                &tree,
                Rain {
                    flux: mm_h(60.0),
                    drying: 0.0,
                },
                GroundClass::bare(0.0),
                1.0,
                1,
                &mut report,
            )
            .expect("valid test case");
    }
    let good = state.levels(&tree);
    assert!(map.set_levels(0, &good).expect("valid test case"));
    let probe = tree.ground().centre(tree.pit(0));
    let before = map.sample(probe).expect("valid test case");
    let mut bad = good.clone();
    bad[1] += 0.001;
    assert!(!map.set_levels(0, &bad).expect("valid test case"), "held");
    assert_eq!(map.held_level_tables, 1);
    assert_eq!(
        map.sample(probe).expect("valid test case"),
        before,
        "the last table holds"
    );
    assert!(
        map.set_levels(0, &[0.0]).is_err(),
        "a wrong count is refused"
    );
}

#[test]
fn a_hollows_body_in_another_bodys_water_or_margin_is_refused() {
    let tree = prepare(&pair());
    let levels = state_levels(&tree);
    // The rectangle lies at x 0 to 1.3, z 0 to 0.5. A basin in the low ground at x < -4
    // reaches it only when its stage lifts it over the high ground at x >= -4.
    let basin = |max: f64| {
        body(
            "pond",
            BodyKind::Basin { seed: [-8.0, 0.0] },
            0.5,
            Stage {
                min: 0.0,
                max,
                value: 0.0,
            },
        )
    };
    let walled = |heights: Vec<f64>| {
        Bed::new([-10.0, -10.0], 1.0, [20, 20], heights).expect("valid test case")
    };
    // Walls round the map's bed keep the basin's water off its edge.
    let heights: Vec<f64> = (0..400)
        .map(|i| match (i % 20, i / 20) {
            (0 | 19, _) | (_, 0 | 19) => 5.0,
            (x, _) if x < 6 => -1.0,
            _ => 1.0,
        })
        .collect();
    // In range at its lowest stage only: accepted.
    let ok = Map::prepare(
        walled(heights.clone()),
        vec![basin(0.0), declare(&tree, &levels)],
        profile(),
    );
    assert!(ok.is_ok(), "{:?}", ok.err());
    // Its highest stage floods to the rectangle: refused, naming the hollows body.
    let err = Map::prepare(
        walled(heights.clone()),
        vec![basin(1.0), declare(&tree, &levels)],
        profile(),
    )
    .err()
    .expect("valid test case");
    assert_eq!(err.name, "hollows");
    assert!(err.reason.contains("water or margin"), "{}", err.reason);
    // A highest stage short of flooding, whose shore margin crosses the high ground to the
    // rectangle: refused too.
    let err = Map::prepare(
        walled(heights),
        vec![basin(0.35), declare(&tree, &levels)],
        profile(),
    )
    .err()
    .expect("valid test case");
    assert!(err.reason.contains("water or margin"), "{}", err.reason);
}

fn state_levels(tree: &Tree) -> Vec<f64> {
    RainState::new(tree).levels(tree)
}

#[test]
fn a_hollows_body_over_the_profile_is_refused() {
    let tree = prepare(&pair());
    let levels = state_levels(&tree);
    let small = |max_cells, max_hollows| HollowsProfile {
        max_cells,
        max_hollows,
    };
    let n = tree.ground().heights().len();
    assert!(
        Map::prepare_with(
            map_bed(),
            vec![declare(&tree, &levels)],
            profile(),
            small(n, 2)
        )
        .is_ok()
    );
    let err = Map::prepare_with(
        map_bed(),
        vec![declare(&tree, &levels)],
        profile(),
        small(n - 1, 2),
    )
    .err()
    .expect("valid test case");
    assert!(err.reason.contains("cells"), "{}", err.reason);
    let err = Map::prepare_with(
        map_bed(),
        vec![declare(&tree, &levels)],
        profile(),
        small(n, 1),
    )
    .err()
    .expect("valid test case");
    assert!(err.reason.contains("hollows"), "{}", err.reason);
    // And a declaration whose labels name no hollow is refused before the map.
    let g = tree.ground();
    assert!(
        Hollows::new(
            g.origin(),
            g.spacing(),
            g.size(),
            g.heights().to_vec(),
            vec![7; n],
            levels
        )
        .is_err()
    );
}
