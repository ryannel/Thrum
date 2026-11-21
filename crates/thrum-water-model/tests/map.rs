//! Map oracles and named refusal cases.
use crate::support;
use support::fixtures::{body, close, grid, profile};
use thrum_water_model::{BodyKind, Chart, Map};

#[test]
fn basin_seed_selects_only_its_bilinear_component() {
    // h = -8(u-1/2)(v-1/2), with saddle h=0. At level -1
    // its two wet lobes are disconnected and distinguished by u < 1/2.
    for flipped in [false, true] {
        for (origin, spacing) in [([0.0; 2], 1.0), ([10.0, -3.0], 2.0)] {
            let world = |u, v| {
                [
                    origin[0] + spacing * (1.0 + u),
                    origin[1] + spacing * (1.0 + v),
                ]
            };
            for [u, v] in [
                [0.0, 0.0],
                [0.01, 0.01],
                [0.0, 0.01],
                [0.01, 0.0],
                [1.0, 1.0],
                [0.99, 0.99],
            ] {
                let seed_u = if flipped { 1.0 - u } else { u };
                let mut p = profile();
                p.margin_height = 0.0;
                let map = Map::prepare(
                    support::fixtures::saddle_bed(origin, spacing, flipped),
                    vec![body(
                        "seeded",
                        BodyKind::Basin {
                            seed: world(seed_u, v),
                        },
                        -1.0,
                    )],
                    p,
                )
                .unwrap();
                for u in [0.0, 0.01, 0.1, 0.25, 0.4, 0.6, 0.75, 0.9, 0.99, 1.0] {
                    for v in [0.0, 0.01, 0.1, 0.25, 0.4, 0.6, 0.75, 0.9, 0.99, 1.0] {
                        let sign = if flipped { 8.0 } else { -8.0 };
                        let height: f64 = sign * (u - 0.5) * (v - 0.5);
                        if (height + 1.0).abs() < 1e-9 {
                            continue;
                        }
                        let expected = height < -1.0 && (u - 0.5) * (seed_u - 0.5) > 0.0;
                        let answer = map.sample(world(u, v));
                        assert_eq!(
                            answer.is_some(),
                            expected,
                            "flipped={flipped}, seed={seed_u},{v}, point={u},{v}"
                        );
                        if let Some(answer) = answer {
                            close(answer.bed.unwrap(), height);
                            close(answer.depth, -1.0 - height);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn diagonal_necks_propagate_beyond_the_seed_cell_only_when_open() {
    for level in [-1.0, 0.0, 0.5] {
        let mut p = profile();
        p.margin_height = 0.0;
        let map = Map::prepare(
            support::fixtures::saddle_chain(7),
            vec![body("chain", BodyKind::Basin { seed: [1.0, 1.0] }, level)],
            p,
        )
        .unwrap();
        for i in [2.0, 3.0] {
            assert_eq!(
                map.sample([i, i]).is_some(),
                level > 0.0,
                "saddle=0, level={level}, point={i}"
            );
        }
    }
}

#[test]
fn basin_refuses_a_diagonal_connection_to_exterior_sea() {
    // The boundary well (3,3) connects to (1,1) through two h=0 saddles
    // at sea level 0.5, although all intervening orthogonal vertices are h=2.
    let error = Map::prepare(
        support::fixtures::saddle_chain(4),
        vec![
            body("sea", BodyKind::Sea, 0.5),
            body("open-hollow", BodyKind::Basin { seed: [1.0, 1.0] }, -1.0),
        ],
        profile(),
    )
    .err()
    .unwrap();
    assert_eq!(error.name, "open-hollow");
    assert!(error.reason.contains("sea"));
}

#[test]
fn a_shoreline_corner_cannot_flood_the_downhill_hollow() {
    let bed = grid([7, 5], |x, z| {
        if !(0.5..=5.5).contains(&x) || !(0.5..=3.5).contains(&z) {
            10.0
        } else if (x - 2.0).abs() < 0.5 {
            0.0
        } else {
            -2.0
        }
    });
    let map = Map::prepare(
        bed,
        vec![body("closed", BodyKind::Basin { seed: [1.0, 2.0] }, 0.0)],
        profile(),
    )
    .unwrap();
    assert!(
        map.sample([2.0, 2.0]).is_some(),
        "the sill is a shore margin"
    );
    for x in [2.01, 2.5, 2.99, 3.0] {
        assert!(
            map.sample([x, 2.0]).is_none(),
            "ground descends beyond the level-zero sill at {x}"
        );
    }
}

#[test]
fn cone_radius_and_disconnected_hollow_are_independent_oracles() {
    let map = support::fixtures::basin();
    for x in [10.0, 10.3, 11.7, 12.999] {
        let s = map.sample([x, 10.0]).unwrap();
        assert_eq!(s.body, 0);
        close(s.row.level, 3.0);
        close(s.bed.unwrap(), x - 10.0);
        close(s.depth, 13.0 - x);
    }
    // Cone h=r has shoreline r=level=3, rather than a cell boundary.
    close(map.sample([13.0, 10.0]).unwrap().depth, 0.0);
    assert!(
        map.sample([30.0, 10.0]).is_none(),
        "the equally deep, disconnected cone must stay dry"
    );
    assert!(map.sample([20.0, 10.0]).is_none());
}

#[test]
fn sealed_basin_with_subsea_floor_takes_precedence_without_spilling() {
    let bed = grid([21, 21], |x, z| {
        let r = (x - 10.0).hypot(z - 10.0);
        if r < 4.0 {
            -3.0 + 0.5 * r
        } else if r < 7.0 {
            10.0
        } else {
            -2.0
        }
    });
    let map = Map::prepare(
        bed,
        vec![
            body("sea", BodyKind::Sea, 0.0),
            body("sealed", BodyKind::Basin { seed: [10.0, 10.0] }, 1.0),
        ],
        profile(),
    )
    .unwrap();
    let lake = map.sample([10.0, 10.0]).unwrap();
    assert_eq!(lake.body, 1);
    close(lake.row.level, 1.0);
    close(lake.depth, 4.0);
    let sea = map.sample([0.0, 0.0]).unwrap();
    assert_eq!(sea.body, 0);
    close(sea.depth, 2.0);
}

#[test]
fn chart_margin_needs_reachable_wet_ground_and_does_not_descend() {
    let chart = Chart {
        levels: vec![0.0; 169],
        currents: vec![[1.0, 0.0]; 169],
        footprint: (0..169)
            .map(|i| (3..=9).contains(&(i % 13)) && (3..=5).contains(&(i / 13)))
            .collect(),
        ends: vec![],
    };
    let bed = grid([13, 13], |x, z| {
        if !(3.0..=9.0).contains(&x) || !(3.0..=5.0).contains(&z) {
            5.0
        } else if x < 6.0 {
            -1.0
        } else if x < 7.0 {
            1.0
        } else {
            0.5
        }
    });
    let map = Map::prepare(
        bed,
        vec![body("flow", BodyKind::Chart(chart.clone()), 0.0)],
        profile(),
    )
    .unwrap();
    assert!(map.sample([4.0, 4.0]).unwrap().depth > 0.0);
    assert!(map.sample([6.0, 4.0]).is_some());
    for z in [4.0, 4.3] {
        assert!(map.sample([5.9, z]).is_some());
        for x in [6.1, 6.5, 6.9] {
            assert!(
                map.sample([x, z]).is_none(),
                "downhill chart margin {x}, {z}"
            );
        }
    }
    assert!(map.sample([8.0, 4.0]).is_none());
    let dry = || grid([13, 13], |_, _| 1.0);
    let map = Map::prepare(
        dry(),
        vec![body("dry", BodyKind::Chart(chart.clone()), 0.0)],
        profile(),
    )
    .unwrap();
    assert!(map.sample([4.0, 4.0]).is_none());
    let error = Map::prepare(
        dry(),
        vec![
            body("a", BodyKind::Chart(chart.clone()), 0.0),
            body("b", BodyKind::Chart(chart), 0.0),
        ],
        profile(),
    )
    .err()
    .unwrap();
    assert_eq!(error.name, "b");
    assert!(error.reason.contains("overlap"));
}

#[test]
fn sea_is_unbounded_and_its_plane_bed_is_continuous() {
    let map = Map::prepare(
        grid([11, 11], |x, z| x * 0.2 + z * 0.1 - 2.0),
        vec![body("ocean", BodyKind::Sea, 0.0)],
        profile(),
    )
    .unwrap();
    for x in [2.999_999, 3.0, 3.000_001] {
        let s = map.sample([x, 4.2]).unwrap();
        close(s.bed.unwrap(), x * 0.2 + 0.42 - 2.0);
    }
    let s = map.sample([-1e6, 9e5]).unwrap();
    assert_eq!(s.body, 0);
    assert_eq!(s.bed, None);
    assert!(s.depth.is_infinite());
    assert!(map.sample([f64::NAN, 0.0]).is_none());
}

#[test]
fn tilted_chart_has_analytic_level_current_and_precedence() {
    let bed = grid([11, 7], |x, z| {
        if (2.0..=8.0).contains(&x) && (2.0..=4.0).contains(&z) {
            -2.0
        } else {
            5.0
        }
    });
    let chart = Chart {
        levels: (0..77).map(|i| 1.0 - 0.05 * f64::from(i % 11)).collect(),
        currents: (0..77).map(|i| [0.1 * f64::from(i % 11), 0.0]).collect(),
        footprint: (0..77)
            .map(|i| (2..=8).contains(&(i % 11)) && (2..=4).contains(&(i / 11)))
            .collect(),
        ends: vec![],
    };
    let map = Map::prepare(
        bed,
        vec![
            body("ocean", BodyKind::Sea, -3.0),
            body("flow", BodyKind::Chart(chart), 0.0),
        ],
        profile(),
    )
    .unwrap();
    for x in [3.999_999, 4.0, 4.000_001, 5.7] {
        let s = map.sample([x, 3.0]).unwrap();
        assert_eq!(s.body, 1);
        close(s.row.level, 1.0 - 0.05 * x);
        close(s.row.current[0], 0.1 * x);
        close(s.depth, 3.0 - 0.05 * x);
    }
}

#[test]
fn named_refusals_for_overlaps_spills_and_open_banks() {
    let rejects = |bed, bodies: Vec<thrum_water_model::Body>, name, reason| {
        let error = Map::prepare(bed, bodies, profile()).err().unwrap();
        assert_eq!(error.name, name);
        assert!(error.reason.contains(reason), "{}", error.reason);
    };
    rejects(
        grid([9, 9], |_, _| -1.0),
        vec![body("a", BodyKind::Sea, 0.0), body("b", BodyKind::Sea, 1.0)],
        "b",
        "overlap",
    );
    rejects(
        grid([9, 9], |_, _| -1.0),
        vec![body("spill", BodyKind::Basin { seed: [4.0, 4.0] }, 0.0)],
        "spill",
        "edge",
    );
    let bowl = || grid([11, 11], |x, z| (x - 5.0).hypot(z - 5.0));
    rejects(
        bowl(),
        vec![
            body("a", BodyKind::Basin { seed: [5.0, 5.0] }, 2.0),
            body("b", BodyKind::Basin { seed: [5.0, 5.0] }, 3.0),
        ],
        "b",
        "overlap",
    );
    rejects(
        bowl(),
        vec![
            body("sea", BodyKind::Sea, 6.0),
            body("wet", BodyKind::Basin { seed: [5.0, 5.0] }, 2.0),
        ],
        "wet",
        "sea",
    );
    let c = Chart {
        levels: vec![0.0; 81],
        currents: vec![[0.0; 2]; 81],
        footprint: (0..81)
            .map(|i| (3..=5).contains(&(i % 9)) && (3..=5).contains(&(i / 9)))
            .collect(),
        ends: vec![],
    };
    rejects(
        grid([9, 9], |_, _| -1.0),
        vec![body("unheld", BodyKind::Chart(c.clone()), 0.0)],
        "unheld",
        "bank",
    );
    rejects(
        grid([9, 9], |x, z| {
            if (3.0..=5.0).contains(&x) && (3.0..=5.0).contains(&z) {
                -1.0
            } else {
                5.0
            }
        }),
        vec![
            body("a", BodyKind::Chart(c.clone()), 0.0),
            body("b", BodyKind::Chart(c), 0.0),
        ],
        "b",
        "overlap",
    );
}
