//! M1.8: `WATERS`' conformance model answers on a hollows body: in a puddle, the body and
//! its depth; on dry ground between two puddles, dry; and either side of a merge the
//! answers differ only by the level's change.
//!
//! M2.9: with rain bands on the hollows body, the harness reports nothing for the model
//! against itself, and reports a provider that ignores exposure by the points under the
//! roof where rings stand and by the differences there.
use crate::support;
use std::sync::Arc;
use support::{mm_h, pair, prepare};
use thrum_rain_model::{
    Canopy, GroundClass, Rain, RainState, RingProfile, Shelter, TickReport, Tree,
};
use thrum_sea_model::LongWaves;
use thrum_water_model::{
    Bed, Body, BodyKind, Exposure, Field, Map, OpenSky, Profile, RainBands, RainBandsProfile,
    RateHistory, Stage, compare,
};

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
            .step(&tree, rain, GroundClass::bare(0.0), 1.0, 1, &mut report)
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

const TICK: f64 = 1.0 / 60.0;

/// A roof over the second hollow of the pair.
fn roofed() -> Canopy {
    let (x0, z0, x1, z1) = (0.6, -1.0, 1.4, 1.0);
    let roof = Shelter {
        name: "yard roof".into(),
        triangles: vec![
            [[x0, 3.0, z0], [x1, 3.0, z0], [x1, 3.0, z1]],
            [[x0, 3.0, z0], [x1, 3.0, z1], [x0, 3.0, z1]],
        ],
        share: 0.0,
    };
    Canopy::prepare(
        [-2.0, -2.0],
        [40, 40],
        &[roof],
        &thrum_rain_model::Profile::default(),
    )
    .expect("valid test case")
}

/// The hollows body holding a puddle in each hollow, with rain bands committed to a tick
/// of steady rain and read through an exposure.
fn rained(tree: &Tree, levels: &[f64], tick: u64, exposure: Arc<dyn Exposure>) -> Map {
    let mut m = map(tree, levels);
    let history = RateHistory {
        tick: TICK,
        last: i64::try_from(tick).expect("a small tick"),
        rates: vec![mm_h(20.0); 61],
    };
    let table = Arc::new(RingProfile::default().table());
    m.set_rain(Some(
        RainBands::new(RainBandsProfile::default(), table, history, exposure)
            .expect("valid test case"),
    ));
    m
}

#[test]
fn the_harness_passes_rain_bands_against_themselves_and_reports_ignored_exposure() {
    let tree = prepare(&pair());
    let field = still_field();
    let mut state = RainState::new(&tree);
    let pit = |h: u32| tree.ground().heights()[tree.pit(h)];
    state
        .pour(&tree, 0, tree.volume(0, pit(0) + 0.020))
        .expect("valid test case");
    state
        .pour(&tree, 1, tree.volume(1, pit(1) + 0.020))
        .expect("valid test case");
    let levels = state.levels(&tree);
    let ticks: Vec<u64> = (600..606).collect();
    let canopy = Arc::new(roofed());
    let model: Vec<Map> = ticks
        .iter()
        .map(|&t| rained(&tree, &levels, t, Arc::<Canopy>::clone(&canopy)))
        .collect();
    let open: Vec<Map> = ticks
        .iter()
        .map(|&t| rained(&tree, &levels, t, Arc::new(OpenSky)))
        .collect();
    let at = |maps: &[Map], p, t: u64| {
        #[expect(clippy::cast_precision_loss, reason = "a small tick")]
        let time = t as f64 * TICK;
        let i = usize::try_from(t - ticks[0]).expect("a small tick");
        maps[i].surface(&field, p, time)
    };
    let reference = |p, t| at(&model, p, t);
    let again = |p, t| at(&model, p, t);
    let ignores = |p, t| at(&open, p, t);
    let g = tree.ground();
    let points: Vec<[f64; 2]> = [0, 1]
        .into_iter()
        .flat_map(|h| {
            let c = g.centre(tree.pit(h));
            (-2..=2).flat_map(move |i| {
                (-2..=2).map(move |j| [c[0] + 0.03 * f64::from(i), c[1] + 0.03 * f64::from(j)])
            })
        })
        .collect();
    // The model against itself: nothing.
    let same = compare(&reference, &again, &points, &ticks).expect("valid test case");
    assert!(same.surface_differences.is_empty() && same.body_mismatches.is_empty());
    assert_eq!(same.wet_mismatches, Vec::new());
    assert_eq!(same.surface.height, 0.0);
    assert_eq!(same.surface.slope, [0.0; 2]);
    // A provider that reads open sky everywhere: reported at every point and tick under
    // the roof where its rings move the water, and nowhere in the open.
    let c = compare(&reference, &ignores, &points, &ticks).expect("valid test case");
    let mut want = Vec::new();
    let mut largest: f64 = 0.0;
    for &t in &ticks {
        for &p in &points {
            let (a, b) = (
                reference(p, t).expect("valid"),
                ignores(p, t).expect("valid"),
            );
            let s = model[0].sample(p).expect("on the map");
            let shaded = canopy.exposure([p[0], s.row.level, p[1]]) < 1.0;
            if a.sample != b.sample {
                assert!(shaded, "a difference in the open at {p:?}");
                want.push((p, t));
                largest = largest.max((a.sample.height - b.sample.height).abs());
            }
        }
    }
    println!(
        "{} of {} reported; largest height difference {largest:.3e} m",
        want.len(),
        points.len() * ticks.len()
    );
    assert_eq!(c.surface_differences, want);
    assert!(!want.is_empty() && want.iter().all(|&(p, _)| p[0] > 0.6 && p[0] < 1.4));
    assert_eq!(c.surface.height, largest);
    assert!(largest > 0.0);
    assert!(c.body_mismatches.is_empty() && c.wet_mismatches.is_empty());
}
