//! M2.1: the ground's two stores step with the tick for each class and reach. Every
//! expected time and fill is worked out here from the class's rates, as parts 5 and 8 of
//! the design proofs work them, and not read from the model; and the water account closes
//! with the stores, the marks' transfers and the pores under the marks in it.
use crate::support;
use support::{design_bed, mm_h};
use thrum_rain_model::{
    Canopy, GroundClass, GroundStores, Profile, REACHES, Rain, RainState, Shelter, TickReport, Tree,
};
use thrum_water_model::NO_HOLLOW;

/// Millimetres as metres.
fn mm(v: f64) -> f64 {
    v / 1000.0
}

fn class(soak: f64, percolation: f64, pores: f64, relief: f64) -> GroundClass {
    GroundClass {
        soak: mm_h(soak),
        percolation: mm_h(percolation),
        pores: mm(pores),
        relief: mm(relief),
        steepest: 1.0,
        share: 0.47,
        porosity: 1.0,
        film_roughness: 0.05,
    }
}

const PAVING: usize = 0;
const EARTH: usize = 1;
const SAND: usize = 2;

fn table() -> Vec<GroundClass> {
    vec![
        // Part 8's paving, earth and sand.
        class(30.0, 0.0, 0.3, 1.0),
        class(15.0, 5.0, 3.0, 0.5),
        class(60.0, 60.0, 8.0, 0.2),
    ]
}

/// The first tick (1-based, of `dt` seconds) at which a predicate holds.
fn first(series: &[(f64, f64)], pred: impl Fn(f64, f64) -> bool) -> Option<usize> {
    series.iter().position(|&(s, f)| pred(s, f)).map(|i| i + 1)
}

#[test]
fn earth_takes_a_film_when_its_pores_are_full_and_runs_off_when_its_relief_is() {
    let dt = 1.0;
    let rain = Rain {
        flux: mm_h(10.0),
        drying: 0.0,
    };
    let mut stores = GroundStores::new(&table()).expect("valid test case");
    let mut series = vec![Vec::new(); 3];
    let mut runoff_from = None;
    for t in 1..=7200 {
        let before = stores.totals(EARTH, 0).runoff;
        stores.step(rain, dt).expect("valid test case");
        for (r, s) in series.iter_mut().enumerate() {
            let st = stores.store(EARTH, r);
            s.push((st.surface, st.fill));
        }
        if runoff_from.is_none() && stores.totals(EARTH, 0).runoff > before {
            runoff_from = Some(t);
        }
        // Sand passes the rain down faster than it falls: no surface water, ever.
        for r in 0..3 {
            assert_eq!(stores.store(SAND, r).surface, 0.0, "sand at {t} s");
        }
    }
    let c = table()[EARTH];
    for (r, s) in series.iter().enumerate() {
        // The rain this reach takes, and the times that follow from it, in seconds.
        let fall = REACHES[r] * rain.flux;
        let into = fall.min(c.soak);
        let full = c.pores * 1000.0 / into;
        let standing = fall - c.percolation;
        let got_full = first(s, |_, f| f >= 1.0).expect("the pores fill");
        assert!((got_full as f64 - full).abs() <= 2.0 * dt);
        if standing <= 0.0 {
            // Percolation at least the rain this reach takes: no surface water, ever.
            assert!(s.iter().all(|&(sw, _)| sw == 0.0), "reach {}", REACHES[r]);
            continue;
        }
        let relief_full = full + c.relief * 1000.0 / standing;
        let got_film = first(s, |sw, _| sw > 0.0).expect("a film stands");
        let got_relief = first(s, |sw, _| sw >= c.relief * (1.0 - 1e-12)).expect("relief fills");
        for (what, got, want) in [
            ("pores full", got_full, full),
            ("first film", got_film, full),
            ("relief full", got_relief, relief_full),
        ] {
            assert!(
                (got as f64 - want).abs() <= 2.0 * dt,
                "reach {}: {what} at {got} s, the rates give {want} s",
                REACHES[r]
            );
        }
        // No surface water while the pores fill.
        assert!(s[..got_full - 1].iter().all(|&(sw, _)| sw == 0.0));
        if r == 0 {
            assert_eq!(full, 1080.0);
            assert_eq!(relief_full, 1440.0);
            let from = runoff_from.expect("runoff starts");
            assert!(
                (f64::from(from) - relief_full).abs() <= 2.0 * dt,
                "runoff from {from} s"
            );
        }
    }
}

#[test]
fn paving_fills_its_pores_at_the_soak_rate_and_dries_its_sheen_before_its_pores() {
    let dt = 1.0;
    let c = table()[PAVING];
    let rain = Rain {
        flux: mm_h(60.0),
        drying: 0.0,
    };
    let dry = Rain {
        flux: 0.0,
        drying: mm_h(6.0),
    };
    let mut stores = GroundStores::new(&table()).expect("valid test case");
    // Rain above the soak rate: the pores take the soak rate and the rest stands.
    let full = c.pores / (c.soak / 1000.0);
    let relief_full =
        full + (c.relief - (rain.flux - c.soak) / 1000.0 * full) / (rain.flux / 1000.0);
    let mut runoff = 0.0;
    for t in 1..=600 {
        stores.step(rain, dt).expect("valid test case");
        let s = stores.store(PAVING, 0);
        let t = f64::from(t);
        if t < full - dt {
            let fill = c.soak / 1000.0 * t / c.pores;
            assert!((s.fill - fill).abs() < 1e-12, "fill {} at {t} s", s.fill);
            let standing = (rain.flux - c.soak) / 1000.0 * t;
            assert!((s.surface - standing).abs() < 1e-15);
        }
        if t < relief_full - dt {
            assert_eq!(stores.totals(PAVING, 0).runoff, 0.0, "no runoff at {t} s");
        }
        if t > relief_full + dt {
            assert_eq!(s.surface, c.relief);
            let r = stores.totals(PAVING, 0).runoff;
            assert!(
                (r - runoff - rain.flux / 1000.0 * dt).abs() < 1e-15,
                "the rest runs off"
            );
            runoff = r;
        } else {
            runoff = stores.totals(PAVING, 0).runoff;
        }
    }
    // After it: the surface water goes at the drying rate, and the pores stay full until
    // it has gone; then they fall in proportion to their fill.
    let gone = c.relief / (dry.drying / 1000.0);
    let mut was = 1.0;
    for t in 1..=3000 {
        stores.step(dry, dt).expect("valid test case");
        let s = stores.store(PAVING, 0);
        let t = f64::from(t);
        if t < gone - dt {
            let left = c.relief - dry.drying / 1000.0 * t;
            assert!((s.surface - left).abs() < 1e-15, "{} at {t} s", s.surface);
            assert_eq!(s.fill, 1.0, "the pores stay full at {t} s");
        }
        if t > gone + dt {
            assert_eq!(s.surface, 0.0);
            let step = (-dry.drying / 1000.0 * dt / c.pores).exp();
            assert!(
                (s.fill - was * step).abs() < 1e-15,
                "fill {} at {t} s",
                s.fill
            );
        }
        was = s.fill;
    }
    assert!(was < 1e-3, "the pores have nearly dried: {was}");
}

#[test]
fn every_class_steps_each_reach_by_its_share_of_the_rain() {
    let mut stores = GroundStores::new(&table()).expect("valid test case");
    let rain = Rain {
        flux: mm_h(20.0),
        drying: mm_h(1.0),
    };
    for _ in 0..3000 {
        stores.step(rain, 0.5).expect("valid test case");
    }
    let total_rain = rain.flux / 1000.0 * 1500.0;
    for (k, c) in table().iter().enumerate() {
        for (r, reach) in REACHES.iter().enumerate() {
            let t = stores.totals(k, r);
            assert!((t.rain - reach * total_rain).abs() < 1e-12);
            // Each store's own water balance.
            let held = stores.store(k, r).water(c.pores);
            let out = held + t.percolated + t.dried + t.runoff;
            assert!((t.rain - out).abs() < 1e-14, "class {k} reach {reach}");
        }
    }
    // Less rain, less held: the open store leads the half and the quarter.
    let e = |r| stores.store(EARTH, r).fill;
    assert!(e(0) >= e(1) && e(1) > e(2), "{} {} {}", e(0), e(1), e(2));
    assert!(GroundStores::new(&[table()[0]; 9]).is_err());
}

/// Ground that percolates 15 mm/h and soaks 5, with 3 mm of pores and 0.5 mm of relief,
/// under 10 mm/h: while its pores fill it takes only the soak rate, so a film stands, 5 mm/h
/// of it up to the relief; once they are full it passes the rain and the film down. The
/// times are the rates': the relief full at 0.5 mm over 5 mm/h, the pores at 3 mm over
/// 5 mm/h, and the film gone 0.5 mm over (15 - 10) mm/h after that.
#[test]
fn ground_that_percolates_faster_than_it_soaks_films_while_its_pores_fill() {
    let c = class(5.0, 15.0, 3.0, 0.5);
    let rain = Rain {
        flux: mm_h(10.0),
        drying: 0.0,
    };
    let mut stores = GroundStores::new(&[c]).expect("valid test case");
    let relief_full = c.relief * 1000.0 / (rain.flux - c.soak);
    let full = c.pores * 1000.0 / c.soak;
    let gone = full + c.relief * 1000.0 / (c.percolation - rain.flux);
    assert_eq!((relief_full, full, gone), (360.0, 2160.0, 2520.0));
    for t in 1..=7200 {
        stores.step(rain, 1.0).expect("valid test case");
        let s = stores.store(0, 0);
        let t = f64::from(t);
        if t < full - 2.0 {
            let film = ((rain.flux - c.soak) / 1000.0 * t).min(c.relief);
            assert!(
                (s.surface - film).abs() < 1e-15,
                "film {} at {t} s",
                s.surface
            );
            assert!((s.fill - c.soak / 1000.0 * t / c.pores).abs() < 1e-12);
            assert!(s.surface > 0.0);
        }
        if t > full + 2.0 {
            assert_eq!(s.fill, 1.0);
        }
        if t > full + 2.0 && t < gone - 2.0 {
            assert!(
                s.surface > 0.0 && s.surface < c.relief,
                "the film goes down at {t} s"
            );
        }
        if t > gone + 2.0 {
            assert_eq!(s.surface, 0.0, "no film over full pores at {t} s");
        }
    }
    // Soak and percolation both at least the rain: no film through two hours, with the
    // percolation the greater.
    let mut stores = GroundStores::new(&[class(10.0, 15.0, 3.0, 0.5)]).expect("valid test case");
    for t in 0..7200 {
        stores.step(rain, 1.0).expect("valid test case");
        assert_eq!(stores.store(0, 0).surface, 0.0, "at {t} s");
    }
}

/// The floor of M2's third review: 8 by 8 level cells of 5 cm inside a rim 4 cm high.
fn floor() -> (Tree, Vec<f64>) {
    let n = 10;
    let heights: Vec<f64> = (0..n * n)
        .map(|i| {
            if i % n == 0 || i % n == n - 1 || i / n == 0 || i / n == n - 1 {
                0.04
            } else {
                0.0
            }
        })
        .collect();
    let bed = thrum_rain_model::Ground::new([0.0, 0.0], 0.05, [n, n], heights.clone())
        .expect("valid test case");
    let tree = Tree::prepare(&bed, &Canopy::open(), &Profile::default()).expect("valid test case");
    (tree, heights)
}

/// The test's own sum of what a body's ground holds: each open cell, at or above its
/// hollow's mark or draining over the edge, from the stores at its exposure, and the pores
/// under every mark.
fn ground_water(
    tree: &Tree,
    exposure: &[f64],
    state: &RainState,
    ground: &GroundStores,
    c: &GroundClass,
) -> f64 {
    let g = tree.ground();
    let area = g.spacing() * g.spacing();
    let marks = state.marks();
    let mut sum: f64 = state.pores_under().iter().sum();
    for (i, &z) in g.heights().iter().enumerate() {
        let l = tree.labels()[i];
        if l != NO_HOLLOW && z < marks[l as usize] {
            continue;
        }
        let e = exposure[i];
        let s = |r: usize| ground.store(0, r).water(c.pores);
        let v = if e >= 0.5 {
            s(1) + (s(0) - s(1)) * (e - 0.5) / 0.5
        } else if e >= 0.25 {
            s(2) + (s(1) - s(2)) * (e - 0.25) / 0.25
        } else {
            s(2) * e / 0.25
        };
        sum += v * area;
    }
    sum
}

/// What an account case found.
struct Totals {
    worst: f64,
    merges: u32,
    partings: u32,
    /// Summed entries: film joined, pore water under, film left, topped up, soaked, the
    /// beds' take into pores, and rain on held ground.
    entries: [f64; 7],
    held_ticks: u32,
    account: thrum_rain_model::Account,
}

/// M1.4's account, with a class that stores water, on the design's bed under a crown:
/// each tick the test sums what the ground holds cell by cell and closes rain against the
/// puddles, the ground and every sink. Rain is the only source: what is held and sunk
/// never grows in a tick by more than the tick's rain. No pores under a mark hold less
/// than none or more than they have room for.
fn account_case(classes: [GroundClass; 1]) -> Totals {
    let bed = design_bed();
    let crown = Shelter {
        name: "crown".into(),
        triangles: vec![
            [[0.2, 2.0, 0.2], [1.4, 2.0, 0.2], [1.4, 2.0, 1.4]],
            [[0.2, 2.0, 0.2], [1.4, 2.0, 1.4], [0.2, 2.0, 1.4]],
        ],
        share: 0.4,
    };
    let canopy = Canopy::prepare([0.0, 0.0], [32, 32], &[crown], &Profile::default())
        .expect("valid test case");
    let tree = Tree::prepare(&bed, &canopy, &Profile::default()).expect("valid test case");
    let g = tree.ground();
    let area = g.spacing() * g.spacing();
    let exposure: Vec<f64> = (0..g.heights().len())
        .map(|i| {
            let [x, z] = g.centre(i);
            canopy.exposure([x, g.heights()[i], z])
        })
        .collect();
    let c = classes[0];
    let mut ground = GroundStores::new(&classes).expect("valid test case");
    let mut state = RainState::new(&tree);
    let mut report = TickReport::default();
    let (mut worst, mut merges, mut partings, mut held_ticks) = (0.0_f64, 0, 0, 0);
    let mut entries = [0.0; 7];
    let mut there = 0.0;
    for t in 0..10_000 {
        let rain = match t {
            0..2600 => Rain {
                flux: mm_h(40.0),
                drying: 0.0,
            },
            2600..5000 => Rain {
                flux: 0.0,
                drying: mm_h(40.0),
            },
            5000..8000 => Rain {
                flux: mm_h(15.0),
                drying: mm_h(2.0),
            },
            _ => Rain {
                flux: 0.0,
                drying: mm_h(60.0),
            },
        };
        let sunk = state.account().sinks();
        state
            .step_with(&tree, rain, &ground, 0, 1.0, 1, &mut report)
            .expect("valid test case");
        ground.step(rain, 1.0).expect("valid test case");
        merges += report.merges;
        partings += report.partings;
        let mut held = false;
        for h in &report.hollows {
            for (sum, v) in entries.iter_mut().zip([
                h.joined, h.under, h.left, h.topped, h.soaked, h.into, h.on_held,
            ]) {
                *sum += v;
            }
            held |= h.short > 0.0;
        }
        held_ticks += u32::from(held);
        let a = state.account();
        let own = ground_water(&tree, &exposure, &state, &ground, &c);
        assert!((own - state.ground_held(&tree, &ground, 0)).abs() <= 1e-12 * a.rain);
        let now = state.held() + own;
        worst = worst.max((a.rain + a.initial - now - a.sinks()).abs() / a.rain);
        let fell = rain.flux / 1000.0 * area * g.heights().len() as f64;
        assert!(
            now + a.sinks() - sunk <= there + fell + 1e-12 * a.rain,
            "water made at {t} s"
        );
        there = now;
        for (l, &u) in state.pores_under().iter().enumerate() {
            let cells = g
                .heights()
                .iter()
                .zip(tree.labels())
                .filter(|&(&z, &label)| label as usize == l && z < state.marks()[l])
                .count();
            assert!(
                u >= 0.0 && u <= c.pores * cells as f64 * area * (1.0 + 1e-9) + 1e-18,
                "pores under hollow {l}'s mark at {t} s: {u:e}"
            );
        }
    }
    Totals {
        worst,
        merges,
        partings,
        entries,
        held_ticks,
        account: *state.account(),
    }
}

#[test]
fn the_account_closes_with_the_stores_and_the_marks_transfers() {
    let t = account_case([class(20.0, 1.0, 1.0, 0.6)]);
    let [joined, under, left, topped, soaked, into, on_held] = t.entries;
    println!(
        "worst residual {:e}; merges {}, partings {}; joined {joined:e}, pore water under {under:e}, left {left:e}, topped up {topped:e}, soaked {soaked:e}, beds into pores {into:e}; held in {} ticks, rain on held ground {on_held:e}",
        t.worst, t.merges, t.partings, t.held_ticks
    );
    assert!(t.worst < 1e-9, "worst residual {:e}", t.worst);
    assert!(t.merges > 0 && t.partings > 0);
    assert!(joined > 0.0 && under > 0.0 && left > 0.0 && soaked > 0.0 && into > 0.0);
    let a = t.account;
    assert!(a.ground_dried > 0.0 && a.canopy > 0.0 && a.edge > 0.0);
}

/// The same account on ground whose pores hold more than the first runoff brings and
/// soak slowly, through merges and partings: the pores under the puddles fill at the soak
/// rate and nothing is owed.
#[test]
fn the_account_closes_on_ground_with_deep_slow_pores() {
    let t = account_case([class(5.0, 0.0, 2.0, 0.1)]);
    println!(
        "worst residual {:e}; merges {}, partings {}; beds into pores {:e} m3; held in {} ticks",
        t.worst, t.merges, t.partings, t.entries[5], t.held_ticks
    );
    assert!(t.worst < 1e-9 && t.merges > 0 && t.partings > 0 && t.entries[5] > 0.0);
}

/// The first review's case: from dry, a shower on the flat floor at a 60 Hz step. The
/// ground has 3 mm of pores that soak at 1 mm/h, 0.1 mm of relief and no percolation; the
/// rain is 10 mm/h with drying off. No water is made: the puddle, the ground and the sinks
/// hold the rain, every tick, within 1e-9 of it. After 120 s the floor's 64 cells have
/// taken 0.16 m2 of the rain: 0.1 mm of it stood as film and joined the puddle, and the
/// pores have taken 1 mm/h, first from the rain and then under the puddle.
#[test]
fn a_shower_on_a_dry_flat_floor_makes_no_water() {
    let (tree, _) = floor();
    let c = class(1.0, 0.0, 3.0, 0.1);
    let mut ground = GroundStores::new(&[c]).expect("valid test case");
    let mut state = RainState::new(&tree);
    let mut report = TickReport::default();
    let rain = Rain {
        flux: mm_h(10.0),
        drying: 0.0,
    };
    let dt = 1.0 / 60.0;
    let exposure = vec![1.0; 100];
    let mut worst = 0.0_f64;
    for _ in 0..7200 {
        state
            .step_with(&tree, rain, &ground, 0, dt, 1, &mut report)
            .expect("valid test case");
        ground.step(rain, dt).expect("valid test case");
        let own = ground_water(&tree, &exposure, &state, &ground, &c);
        assert!((own - state.ground_held(&tree, &ground, 0)).abs() < 1e-15);
        let acc = state.account();
        let held = state.held() + own;
        worst = worst.max((acc.rain + acc.initial - held - acc.sinks()).abs() / acc.rain);
        assert!(held + acc.sinks() <= acc.rain * (1.0 + 1e-9), "water made");
    }
    let floor_area = 64.0 * 0.05 * 0.05;
    let fell = mm_h(10.0) / 1000.0 * 120.0 * floor_area;
    let soaked = mm_h(1.0) / 1000.0 * 120.0 * floor_area;
    println!(
        "worst residual {worst:e}; of {fell:.4e} m3 on the floor the puddle holds {:.4e} and the pores under it {:.4e}",
        state.held(),
        state.pores_under()[0]
    );
    assert!(worst < 1e-9, "worst residual {worst:e}");
    // The floor's rain is in its puddle and its pores, to within a tick's worth.
    let tick = mm_h(10.0) / 1000.0 * dt * floor_area;
    assert!((state.held() + state.pores_under()[0] - fell).abs() <= 2.0 * tick);
    assert!((state.pores_under()[0] - soaked).abs() <= 2.0 * tick);
}

/// The third review's cases: from dry, a shower at 60 mm/h on the flat floor, then drying
/// at 20 mm/h for three hours, for two soak rates, two reliefs and three showers. Every
/// tick the puddle, the pores under the mark and the ground hold no less than none, a
/// tick's sinks never take more than the water there was at its start and the tick's
/// rain, and the account closes; by the end the ground is dry and the sinks hold all the
/// rain.
#[test]
fn a_short_shower_then_drying_never_spends_water_that_is_not_there() {
    let (tree, _) = floor();
    for soak in [1.0, 30.0] {
        for relief in [0.1, 3.0] {
            for shower in [50, 150, 500] {
                let c = class(soak, 0.0, 3.0, relief);
                let mut ground = GroundStores::new(&[c]).expect("valid test case");
                let mut state = RainState::new(&tree);
                let mut report = TickReport::default();
                let mut worst = 0.0_f64;
                let mut there = 0.0;
                for t in 0..(shower + 3 * 3600) {
                    let rain = if t < shower {
                        Rain {
                            flux: mm_h(60.0),
                            drying: 0.0,
                        }
                    } else {
                        Rain {
                            flux: 0.0,
                            drying: mm_h(20.0),
                        }
                    };
                    let before = state.account().sinks();
                    state
                        .step_with(&tree, rain, &ground, 0, 1.0, 1, &mut report)
                        .expect("valid test case");
                    ground.step(rain, 1.0).expect("valid test case");
                    let a = state.account();
                    let fell = rain.flux / 1000.0 * 100.0 * 0.05 * 0.05;
                    let case = format!("soak {soak}, relief {relief}, shower {shower} s, at {t} s");
                    assert!(
                        a.sinks() - before <= there + fell + 1e-15,
                        "sinks took water not there: {case}"
                    );
                    for l in 0..tree.hollow_count() {
                        assert!(state.pores_under()[l] >= 0.0, "{case}");
                        assert!(
                            state.hollow_ground(&tree, &ground, 0, l) >= 0.0,
                            "negative ground: {case}"
                        );
                    }
                    let held = state.held() + state.ground_held(&tree, &ground, 0);
                    assert!(state.held() >= 0.0, "{case}");
                    worst = worst.max((a.rain - held - a.sinks()).abs() / a.rain);
                    there = held;
                }
                let a = state.account();
                println!(
                    "soak {soak} mm/h, relief {relief} mm, shower {shower} s: worst residual {worst:.1e}; after drying ground {:.1e} of the rain",
                    state.ground_held(&tree, &ground, 0) / a.rain
                );
                assert!(worst < 1e-9);
                assert_eq!(state.held(), 0.0);
                assert!(
                    state.ground_held(&tree, &ground, 0) < 1e-6 * a.rain,
                    "the ground dries"
                );
                assert!((a.sinks() - a.rain).abs() < 1e-6 * a.rain);
            }
        }
    }
}
