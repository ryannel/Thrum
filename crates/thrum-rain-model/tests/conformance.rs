//! M1.8: `WATERS`' conformance model answers on a hollows body: in a puddle, the body and
//! its depth; on dry ground between two puddles, dry; and either side of a merge the
//! answers differ only by the level's change.
pub mod support;
use support::{mm_h, pair, prepare};
use thrum_rain_model::{GroundClass, Rain, RainState, TickReport, Tree};
use thrum_sea_model::LongWaves;
use thrum_water_model::{Bed, Body, BodyKind, Field, Map, Profile, Stage, compare};

fn still_field() -> Field {
    Field {
        layers: std::array::from_fn(|_| LongWaves::new(Vec::new(), 0.0).expect("valid test case")),
    }
}

fn map(tree: &Tree, levels: &[f64]) -> Map {
    let bed = Bed::new([-10.0, -10.0], 1.0, [20, 20], vec![-1.0; 400]).expect("valid test case");
    let body = Body {
        name: "hollows".into(),
        kind: BodyKind::Hollows(tree.water_hollows(levels).expect("valid test case")),
        level: 0.0,
        stage: Stage {
            min: 0.0,
            max: 0.0,
            value: 0.0,
        },
        fetch: 1e3,
        gains: [0.0; 4],
    };
    Map::prepare(
        bed,
        vec![body],
        Profile {
            margin_height: 0.2,
            swash_ratio: 0.7,
        },
    )
    .expect("valid test case")
}

#[test]
fn the_conformance_model_answers_on_hollows_through_a_merge() {
    let tree = prepare(&pair());
    let field = still_field();
    let mut state = RainState::new(&tree);
    let mut report = TickReport::default();
    let rain = Rain {
        flux: mm_h(60.0),
        drying: 0.0,
    };
    let joined = |s: &RainState| s.puddles().any(|(k, _)| tree.node(k).children.is_some());
    // Fill until both puddles stand but have not joined.
    let mut before = state.clone();
    while !joined(&state) {
        before = state.clone();
        state
            .step(
                &tree,
                rain,
                GroundClass { percolation: 0.0 },
                1.0,
                1,
                &mut report,
            )
            .expect("valid test case");
    }
    let after = state;
    let (lb, la) = (before.levels(&tree), after.levels(&tree));
    let (mb, ma) = (map(&tree, &lb), map(&tree, &la));
    let g = tree.ground();
    let (pa, pb) = (g.centre(tree.pit(0)), g.centre(tree.pit(1)));
    let sill = g.centre(tree.pit(0) + 5); // the sill between them, on the middle row
    // In a puddle: the body and its depth.
    let s = mb.surface(&field, pa, 0.0).expect("valid test case");
    assert_eq!(s.body, Some(0));
    assert!(s.wet);
    assert!((s.depth - (lb[0] - g.heights()[tree.pit(0)])).abs() < 1e-15);
    // On dry ground between two puddles, before they join: dry.
    let dry = mb.surface(&field, sill, 0.0).expect("valid test case");
    assert!(!dry.wet, "{dry:?}");
    assert!(dry.depth <= 0.0 || dry.body.is_none());
    // Either side of the merge, wet points in both answer alike but for the level's change.
    let reference = |p, _t| mb.surface(&field, p, 0.0);
    let provider = |p, _t| ma.surface(&field, p, 0.0);
    for (pit, h) in [(pa, 0), (pb, 1)] {
        let near: Vec<[f64; 2]> = (-2..=2)
            .map(|d| [pit[0] + f64::from(d) * 0.05, pit[1]])
            .collect();
        let c = compare(&reference, &provider, &near, &[0]).expect("valid test case");
        let change = (la[h] - lb[h]).abs();
        assert!(
            (c.surface.height - change).abs() < 1e-15,
            "{} against {change}",
            c.surface.height
        );
        assert!((c.depth - change).abs() < 1e-15);
        assert_eq!(c.surface.slope, [0.0; 2]);
        assert_eq!(c.current, [0.0; 2]);
        assert!(c.body_mismatches.is_empty() && c.wet_mismatches.is_empty());
        assert!(
            change < 0.002,
            "the merge moves each level by under a tick's rise: {change}"
        );
    }
}
