//! M1.3: puddles fill, spill, merge and part, and in every tick a hollow's level moves by
//! no more than the water that entered or left its puddle's hollows over the smallest water
//! surface any of those puddles had before or after, taken as at least one cell.
//!
//! The bed is a strip of five hollows in a row, A to E, between walls. Their sills join A
//! with B (0 mm), C with D (2 mm), then E (4 mm), then the two sides (10 mm); the last
//! leaves over a pass at 15 mm. E lies under a roof, so it fills only by D's spill.
#![expect(
    clippy::cast_possible_truncation,
    reason = "test counts of cells, hollows and nodes are far below u32::MAX"
)]
use crate::support;
use support::{ground, mm_h};
use thrum_rain_model::{Canopy, GroundClass, Profile, Rain, RainState, Shelter, TickReport, Tree};

const CELL: f64 = 0.05;
const ROWS: usize = 12;

fn profile_height(x: f64) -> f64 {
    // Control points: cell column and height in metres.
    let points = [
        (0.0, 0.10),
        (1.0, 0.06),
        (6.0, -0.020),
        (11.0, 0.000),
        (16.0, -0.025),
        (21.0, 0.010),
        (26.0, -0.020),
        (31.0, 0.002),
        (36.0, -0.022),
        (41.0, 0.004),
        (46.0, -0.015),
        (51.0, 0.015),
        (55.0, -0.05),
    ];
    let c = x / CELL - 0.5;
    for w in points.windows(2) {
        let ((x0, z0), (x1, z1)) = (w[0], w[1]);
        if c <= x1 {
            return z0 + (z1 - z0) * ((c - x0) / (x1 - x0)).clamp(0.0, 1.0);
        }
    }
    -0.05
}

fn strip() -> Tree {
    let g = ground([56, ROWS], CELL, |x, y| {
        let row = y / CELL - 0.5;
        if row < 0.5 || row > ROWS as f64 - 1.5 {
            return 0.10;
        }
        // A slight bowl across the strip, so each hollow has one pit.
        profile_height(x) + 0.002 * ((row - 5.5) / 5.5).powi(2)
    });
    // A roof over E's catchment and the soft edge round it.
    let (x0, x1) = (41.0 * CELL - 0.2, 52.0 * CELL + 0.2);
    let roof = Shelter {
        name: "roof".into(),
        triangles: vec![
            [[x0, 2.5, -0.5], [x1, 2.5, -0.5], [x1, 2.5, 1.5]],
            [[x0, 2.5, -0.5], [x1, 2.5, 1.5], [x0, 2.5, 1.5]],
        ],
        share: 0.0,
    };
    let canopy = Canopy::prepare([-1.0, -1.0], [50, 30], &[roof], &Profile::default())
        .expect("valid test case");
    Tree::prepare(&g, &canopy, &Profile::default()).expect("valid test case")
}

/// The five hollows, A to E, by their pits' columns.
fn hollows(tree: &Tree) -> [u32; 5] {
    let w = tree.ground().size()[0];
    let by_column = |c: usize| {
        (0..tree.hollow_count() as u32)
            .find(|&h| tree.pit(h) % w == c)
            .expect("valid test case")
    };
    [
        by_column(6),
        by_column(16),
        by_column(26),
        by_column(36),
        by_column(46),
    ]
}

fn within(tree: &Tree, mut node: u32, ancestor: u32) -> bool {
    loop {
        if node == ancestor {
            return true;
        }
        match tree.node(node).parent {
            Some(p) => node = p,
            None => return false,
        }
    }
}

/// The puddles inside a node's subtree: (node, volume, surface).
fn puddles_in(tree: &Tree, state: &RainState, top: u32) -> Vec<(u32, f64, f64)> {
    state
        .puddles()
        .filter(|&(k, _)| within(tree, k, top))
        .map(|(k, v)| (k, v, tree.surface(k, state.level(tree, k))))
        .collect()
}

/// The bound for one hollow across one tick, from that tick's own volumes and surfaces:
/// returns the level's change and the most it may be.
fn bound(tree: &Tree, before: &RainState, after: &RainState, hollow: u32) -> (f64, f64) {
    let (p0, p1) = (before.holder(tree, hollow), after.holder(tree, hollow));
    let top = if within(tree, p0, p1) { p1 } else { p0 };
    let (b, a) = (puddles_in(tree, before, top), puddles_in(tree, after, top));
    let moved = (a.iter().map(|p| p.1).sum::<f64>() - b.iter().map(|p| p.1).sum::<f64>()).abs();
    let cell = CELL * CELL;
    let smallest = b
        .iter()
        .chain(&a)
        .map(|p| p.2)
        .fold(f64::INFINITY, f64::min)
        .max(cell);
    let change = (after.level(tree, p1) - before.level(tree, p0)).abs();
    (change, moved / smallest)
}

fn check_bound(tree: &Tree, before: &RainState, after: &RainState) {
    for h in 0..tree.hollow_count() as u32 {
        let (change, most) = bound(tree, before, after, h);
        assert!(
            change <= most * (1.0 + 1e-9) + 1e-12,
            "hollow {h}: level moved {change} > {most}"
        );
    }
}

fn level(tree: &Tree, s: &RainState, h: u32) -> f64 {
    s.level(tree, s.holder(tree, h))
}

#[test]
fn puddles_fill_spill_merge_and_part() {
    let tree = strip();
    assert_eq!(tree.hollow_count(), 5);
    let [a, b, c, d, e] = hollows(&tree);
    let ab = tree.node(a).parent.expect("valid test case");
    let cd = tree.node(c).parent.expect("valid test case");
    let cde = tree.node(e).parent.expect("valid test case");
    let top = tree.node(ab).parent.expect("valid test case");
    assert_eq!(tree.node(d).parent, Some(cd));
    assert_eq!(tree.node(cd).parent, Some(cde));
    assert_eq!(tree.node(cde).parent, Some(top));
    assert!(tree.node(top).parent.is_none());

    let mut state = RainState::new(&tree);
    let mut report = TickReport::default();
    let class = GroundClass::bare(0.0);
    let rain = Rain {
        flux: mm_h(60.0),
        drying: 0.0,
    };
    let (mut spilled_a, mut merged_ab, mut merged_top, mut e_wet_before_spill) =
        (false, false, false, false);
    let mut d_spilled = false;
    for _ in 0..6000 {
        let before = state.clone();
        state
            .step(&tree, rain, class, 1.0, 1, &mut report)
            .expect("valid test case");
        check_bound(&tree, &before, &state);
        let held = |s: &RainState, k: u32| s.puddles().find(|p| p.0 == k).map(|p| p.1);
        // A full hollow's excess goes to the hollow across its sill.
        if held(&before, a) == Some(tree.capacity(a))
            && held(&state, a) == Some(tree.capacity(a))
            && let (Some(b0), Some(b1)) = (held(&before, b), held(&state, b))
        {
            {
                let ga = report.hollows[a as usize];
                let gb = report.hollows[b as usize];
                let expected = b0 + gb.on_water + gb.runoff + ga.on_water + ga.runoff;
                assert!(
                    (b1 - expected).abs() <= 1e-15 * expected.max(1e-9),
                    "{b1} != {expected}"
                );
                spilled_a = true;
            }
        }
        // Two full to one sill are one puddle at one level.
        if held(&state, ab).is_some() && !merged_ab {
            merged_ab = true;
            assert_eq!(level(&tree, &state, a), level(&tree, &state, b));
            assert!(level(&tree, &state, a) >= tree.node(a).sill);
        }
        // E stays dry until D spills into it.
        let d_full = state.holder(&tree, d) != d || held(&state, d) == Some(tree.capacity(d));
        if !d_spilled && !d_full && state.holder(&tree, e) == e && held(&state, e) != Some(0.0) {
            e_wet_before_spill = true;
        }
        d_spilled |= d_full;
        // A merge of two puddles that are each several hollows.
        if held(&before, ab).is_some()
            && held(&before, cde).is_some()
            && held(&state, top).is_some()
        {
            merged_top = true;
        }
    }
    assert!(
        spilled_a && merged_ab && merged_top && d_spilled,
        "{spilled_a} {merged_ab} {merged_top} {d_spilled}"
    );
    assert!(!e_wet_before_spill, "E took water before D spilled");
    assert!(
        report.edge_spill > 0.0,
        "the full tree spills over its last pass"
    );

    // Drying: they part at their sills, each side keeping its own water.
    let drying = Rain {
        flux: 0.0,
        drying: mm_h(80.0),
    };
    let mut parted = Vec::new();
    for _ in 0..8000 {
        let before = state.clone();
        state
            .step(&tree, drying, class, 1.0, 1, &mut report)
            .expect("valid test case");
        check_bound(&tree, &before, &state);
        for k in [top, cde, cd, ab] {
            let was = before.puddles().find(|p| p.0 == k);
            if let (Some((_, v)), true) = (was, state.puddles().all(|p| p.0 != k)) {
                let [x, y] = tree.node(k).children.expect("valid test case");
                let side = |n: u32| {
                    puddles_in(&tree, &state, n)
                        .iter()
                        .map(|p| p.1)
                        .sum::<f64>()
                };
                let (vx, vy) = (side(x), side(y));
                assert!(vx <= tree.capacity(x) + 1e-15 && vy <= tree.capacity(y) + 1e-15);
                assert!(
                    vx + vy <= v && vx > 0.0 && vy > 0.0,
                    "each side keeps water"
                );
                assert!(level(&tree, &state, leaf(&tree, x)) <= tree.node(x).sill);
                parted.push(k);
            }
        }
    }
    assert_eq!(parted.len(), 4, "every joined puddle parts: {parted:?}");
    assert!(state.held() < 1e-6, "the bed dries");
}

fn leaf(tree: &Tree, mut node: u32) -> u32 {
    while let Some([x, _]) = tree.node(node).children {
        node = x;
    }
    node
}

#[test]
fn a_fill_that_meets_at_the_mean_level_fails_the_bound() {
    // At the tick A first spills into B, a fill that joined them at their mean level would
    // lift B by half the difference of their levels on that tick's small inflow.
    let tree = strip();
    let [a, b, ..] = hollows(&tree);
    let mut state = RainState::new(&tree);
    let mut report = TickReport::default();
    let rain = Rain {
        flux: mm_h(60.0),
        drying: 0.0,
    };
    loop {
        let before = state.clone();
        state
            .step(&tree, rain, GroundClass::bare(0.0), 1.0, 1, &mut report)
            .expect("valid test case");
        if state
            .puddles()
            .any(|p| p.0 == a && p.1.to_bits() == tree.capacity(a).to_bits())
        {
            let (_, most) = bound(&tree, &before, &state, b);
            let mean = f64::midpoint(level(&tree, &state, a), level(&tree, &state, b));
            let wrong = (mean - level(&tree, &before, b)).abs();
            assert!(
                wrong > 10.0 * most,
                "a mean-level merge moves B {wrong}, bound {most}"
            );
            return;
        }
    }
}
