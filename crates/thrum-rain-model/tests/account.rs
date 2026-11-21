//! M1.4: the water account closes through spills, merges and partings; a catchment six
//! times a pothole's area fills it in a sixth of the time; and a puddle on ground that
//! passes the rain down does not grow.
use crate::support;
use support::{design_bed, mm_h, pothole, prepare, residual};
use thrum_rain_model::{Canopy, GroundClass, Profile, Rain, RainState, Shelter, TickReport, Tree};

#[test]
fn the_account_closes_over_ten_thousand_ticks() {
    let bed = design_bed();
    // A crown that lets half through over part of the bed, so the canopy's sink is used.
    let crown = Shelter {
        name: "crown".into(),
        triangles: vec![
            [[0.2, 2.0, 0.2], [1.4, 2.0, 0.2], [1.4, 2.0, 1.4]],
            [[0.2, 2.0, 0.2], [1.4, 2.0, 1.4], [0.2, 2.0, 1.4]],
        ],
        share: 0.5,
    };
    let canopy = Canopy::prepare([0.0, 0.0], [32, 32], &[crown], &Profile::default())
        .expect("valid test case");
    let tree = Tree::prepare(&bed, &canopy, &Profile::default()).expect("valid test case");
    let mut state = RainState::new(&tree);
    let mut report = TickReport::default();
    let class = GroundClass::bare(mm_h(1.0));
    let (mut merges, mut partings, mut worst) = (0, 0, 0.0_f64);
    for t in 0..10_000 {
        // Two showers, each followed by drying; the second lighter and longer.
        let rain = match t {
            0..2600 => Rain {
                flux: mm_h(30.0),
                drying: 0.0,
            },
            2600..5000 => Rain {
                flux: 0.0,
                drying: mm_h(40.0),
            },
            5000..8000 => Rain {
                flux: mm_h(12.0),
                drying: mm_h(2.0),
            },
            _ => Rain {
                flux: 0.0,
                drying: mm_h(60.0),
            },
        };
        state
            .step(&tree, rain, class, 1.0, 1, &mut report)
            .expect("valid test case");
        merges += report.merges;
        partings += report.partings;
        worst = worst.max(residual(&state));
    }
    let a = state.account();
    println!(
        "rain {:.6} m3, held {:.6}, canopy {:.6}, ground {:.6}, percolated {:.6}, dried {:.6}, edge {:.6}; merges {merges}, partings {partings}, worst residual {worst:e}",
        a.rain,
        state.held(),
        a.canopy,
        a.ground,
        a.percolated,
        a.dried,
        a.edge
    );
    assert!(worst <= 1e-9, "residual {worst:e} of the rain");
    assert!(
        merges >= 2 && partings >= 2,
        "merges {merges}, partings {partings}"
    );
    for v in [a.canopy, a.ground, a.percolated, a.dried, a.edge] {
        assert!(v > 0.0, "every sink takes water");
    }
}

/// Ticks of one second until the pothole first spills, at 36 mm/h on ground that takes
/// nothing, and the size of its catchment in cells.
fn fill_time(width: usize, depth: usize) -> (usize, usize) {
    let tree = prepare(&pothole(width, depth));
    assert_eq!(tree.hollow_count(), 1);
    let catchment = tree.labels().iter().filter(|&&l| l == 0).count();
    let mut state = RainState::new(&tree);
    let mut report = TickReport::default();
    for t in 1..10_000 {
        let rain = Rain {
            flux: mm_h(36.0),
            drying: 0.0,
        };
        state
            .step(&tree, rain, GroundClass::bare(0.0), 1.0, 1, &mut report)
            .expect("valid test case");
        if report.edge_spill > 0.0 {
            return (t, catchment);
        }
    }
    panic!("never filled");
}

#[test]
fn six_times_the_catchment_fills_in_a_sixth_of_the_time() {
    let (alone, a) = fill_time(4, 4);
    let (drained, b) = fill_time(8, 12);
    assert_eq!(
        (a, b),
        (16, 96),
        "the catchments are the pothole and six times it"
    );
    println!("fills in {alone} s alone and {drained} s with six times its area draining to it");
    // 0.8 l at 36 mm/h on 0.04 m2 is 2,000 s; a tick of rounding either way.
    assert!((1999..=2001).contains(&alone), "{alone}");
    assert!(
        alone.abs_diff(6 * drained) <= 6,
        "{alone} against 6 x {drained}"
    );
}

#[test]
fn a_puddle_on_ground_that_passes_the_rain_down_does_not_grow() {
    let tree = prepare(&pothole(4, 4));
    let rain = Rain {
        flux: mm_h(36.0),
        drying: 0.0,
    };
    for percolation in [mm_h(36.0), mm_h(50.0)] {
        let class = GroundClass::bare(percolation);
        // Dry, it stays dry.
        let mut state = RainState::new(&tree);
        support::run(&tree, &mut state, rain, class, 1.0, 1000);
        assert_eq!(state.held(), 0.0);
        // Half full, it never grows.
        let mut state = RainState::new(&tree);
        state
            .pour(&tree, 0, 0.5 * tree.capacity(0))
            .expect("valid test case");
        let mut report = TickReport::default();
        let mut last = state.held();
        for _ in 0..1000 {
            state
                .step(&tree, rain, class, 1.0, 1, &mut report)
                .expect("valid test case");
            assert!(state.held() <= last, "{} grew past {last}", state.held());
            last = state.held();
        }
        assert!(residual(&state) <= 1e-9);
    }
    // With no percolation the same puddle does grow, so the case can fail.
    let mut state = RainState::new(&tree);
    support::run(&tree, &mut state, rain, GroundClass::bare(0.0), 1.0, 10);
    assert!(state.held() > 0.0);
}
