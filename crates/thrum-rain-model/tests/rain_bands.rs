//! M2.6: ruling 3 on the CPU. `thrum-water-model`'s surface model sums two rain bands
//! through its one sum. The bands are a function of time and of the committed rate
//! history; a drop is born by the rate at its own tick, so rings run out their second
//! after a stop and none is born in a dry spell; scheduled rings follow the rain up to the
//! profile's count; the drops below the cut add their squared slope to the moments over
//! the same history; the gain is exposure times the depth response on every body; and
//! with no rain bands every answer is the same to the bit.
use crate::support;
use std::sync::Arc;
use support::{mm_h, pair, prepare};
use thrum_rain_model::{Canopy, Profile, RainState, RingProfile, Shelter};
use thrum_sea_model::LongWaves;
use thrum_water_model::{
    Bed, Body, BodyKind, Field, Map, RainBands, RainBandsProfile, RateHistory, RingTable, Stage,
};

const TICK: f64 = 1.0 / 60.0;

fn still() -> Field {
    Field {
        layers: std::array::from_fn(|_| LongWaves::new(Vec::new(), 0.0).expect("valid test case")),
    }
}

fn table() -> Arc<RingTable> {
    static TABLE: std::sync::OnceLock<Arc<RingTable>> = std::sync::OnceLock::new();
    Arc::clone(TABLE.get_or_init(|| Arc::new(RingProfile::default().table())))
}

/// The history committed at the tick a time falls in: 61 ticks, from a rate by tick.
fn history(time: f64, rate: impl Fn(i64) -> f64) -> RateHistory {
    #[expect(clippy::cast_possible_truncation, reason = "a tick of a small time")]
    let last = (time / TICK).floor() as i64;
    RateHistory {
        tick: TICK,
        last,
        rates: (last - 60..=last).map(rate).collect(),
    }
}

/// A roof over part of the sea and over the hollows body's second hollow.
fn canopy() -> Canopy {
    let roof = |name: &str, x0: f64, z0: f64, x1: f64, z1: f64| Shelter {
        name: name.into(),
        triangles: vec![
            [[x0, 3.0, z0], [x1, 3.0, z0], [x1, 3.0, z1]],
            [[x0, 3.0, z0], [x1, 3.0, z1], [x0, 3.0, z1]],
        ],
        share: 0.0,
    };
    Canopy::prepare(
        [-8.0, -2.0],
        [100, 40],
        &[
            roof("sea roof", -6.0, 0.0, -4.0, 1.0),
            roof("yard roof", 0.6, -1.0, 1.4, 1.0),
        ],
        &Profile::default(),
    )
    .expect("valid test case")
}

/// The sea west of x = -3, ground east of it, and a hollows body on the ground holding
/// puddles. With rain, two bands over a history.
fn map(rain: Option<RateHistory>) -> (Map, Vec<f64>) {
    let n = 20;
    let heights = (0..n * n)
        .map(|i| {
            if -10.0 + (i % n) as f64 <= -3.0 {
                -5.0
            } else {
                1.0
            }
        })
        .collect();
    let bed = Bed::new([-10.0, -10.0], 1.0, [n, n], heights).expect("valid test case");
    let tree = prepare(&pair());
    let mut state = RainState::new(&tree);
    // A puddle in each hollow: 20 mm and 2 mm at their pits.
    let pit = |h: u32| tree.ground().heights()[tree.pit(h)];
    state
        .pour(&tree, 0, tree.volume(0, pit(0) + 0.020))
        .expect("valid test case");
    state
        .pour(&tree, 1, tree.volume(1, pit(1) + 0.002))
        .expect("valid test case");
    let levels = state.levels(&tree);
    let body = |name: &str, kind| Body {
        name: name.into(),
        kind,
        level: 0.0,
        stage: Stage {
            min: 0.0,
            max: 0.0,
            value: 0.0,
        },
        fetch: 1e3,
        gains: [0.0; 4],
    };
    let mut map = Map::prepare(
        bed,
        vec![
            body("sea", BodyKind::Sea),
            body(
                "yard",
                BodyKind::Hollows(tree.water_hollows(&levels).expect("valid test case")),
            ),
        ],
        thrum_water_model::Profile {
            margin_height: 0.2,
            swash_ratio: 0.7,
        },
    )
    .expect("valid test case");
    if let Some(h) = rain {
        map.set_rain(Some(bands(h)));
    }
    let depths = vec![levels[0] - pit(0), levels[1] - pit(1)];
    (map, depths)
}

fn bands(h: RateHistory) -> RainBands {
    RainBands::new(RainBandsProfile::default(), table(), h, Arc::new(canopy()))
        .expect("valid test case")
}

/// The test's own sum of a band's rings at a point: each live drop's ring from the
/// table at its distance, the nearest image in the patch.
fn own_sum(b: &RainBands, point: [f64; 2], time: f64) -> f64 {
    let t = b.table();
    let mut h = 0.0;
    for band in 0..2 {
        let l = b.profile().patches[band];
        for d in b.drops(band, time) {
            let wrap = |v: f64| v - l * (v / l).round();
            let dx = wrap(point[0].rem_euclid(l) - d.place[0]);
            let dz = wrap(point[1].rem_euclid(l) - d.place[1]);
            h += d.weight * t.ring(d.class, dx.hypot(dz), time - d.time)[0];
        }
    }
    h
}

fn points() -> Vec<[f64; 2]> {
    (0..40)
        .map(|i| [-9.0 + 0.37 * f64::from(i), 0.21 + 0.113 * f64::from(i % 7)])
        .collect()
}

#[test]
fn with_no_rain_bands_every_answer_is_the_same_to_the_bit() {
    let field = still();
    let (plain, _) = map(None);
    let (mut rained, _) = map(Some(history(5.0, |_| mm_h(30.0))));
    rained.set_rain(None);
    for p in points() {
        for t in [0.0, 1.7, 5.0] {
            assert_eq!(plain.surface(&field, p, t), rained.surface(&field, p, t));
            assert_eq!(rained.rain_moments(&field, p, t), 0.0);
        }
    }
}

#[test]
fn the_bands_are_a_function_of_time_and_the_committed_history() {
    let field = still();
    let rain = |j: i64| mm_h(if j % 7 < 3 { 40.0 } else { 5.0 });
    let (a, _) = map(Some(history(7.5, rain)));
    let (mut b, _) = map(Some(history(3.0, |_| mm_h(80.0))));
    // Evaluate b at other times and histories first, then give it a's history.
    for p in points() {
        b.surface(&field, p, 3.0).expect("valid test case");
    }
    b.set_rate_history(history(7.5, rain))
        .expect("valid test case");
    let mut order: Vec<(usize, f64)> = (0..40).flat_map(|i| [(i, 7.5), (i, 7.51)]).collect();
    order.reverse();
    let pts = points();
    for &(i, t) in &order {
        let x = a.surface(&field, pts[i], t).expect("valid test case");
        let y = b.surface(&field, pts[i], t).expect("valid test case");
        assert_eq!(x, y, "at {:?}, {t} s", pts[i]);
        assert_eq!(
            a.rain_moments(&field, pts[i], t),
            b.rain_moments(&field, pts[i], t)
        );
    }
    // Some of them are raised by rings.
    let (plain, _) = map(None);
    let moved = pts
        .iter()
        .filter(|&&p| a.surface(&field, p, 7.5) != plain.surface(&field, p, 7.5))
        .count();
    assert!(moved > 10, "{moved} points moved by rings");
}

#[test]
fn rings_run_out_after_a_stop_and_none_is_born_in_the_dry_spell() {
    let rate = mm_h(30.0);
    // Rain to 2.0 s (tick 120), dry to 2.6 s (tick 156), rain again.
    let stop_restart = |j: i64| if (120..156).contains(&j) { 0.0 } else { rate };
    let stop = |j: i64| if j >= 120 { 0.0 } else { rate };
    let at = |t: f64, f: &dyn Fn(i64) -> f64| bands(history(t, f));
    let before = at(1.999, &stop_restart);
    for band in 0..2 {
        let born_before: Vec<_> = before.drops(band, 1.999);
        for t in [2.05, 2.3, 2.59] {
            let b = at(t, &stop_restart);
            let alive = b.drops(band, t);
            let (mut kept, mut present_rate_births) = (0, 0);
            for d in &born_before {
                if d.time <= t - 1.0 {
                    continue;
                }
                // Each ring born before the stop is still there, at its own weight.
                let same = alive
                    .iter()
                    .find(|e| {
                        e.time.to_bits() == d.time.to_bits()
                            && e.place.map(f64::to_bits) == d.place.map(f64::to_bits)
                    })
                    .expect("a ring born before the stop is alive after it");
                assert_eq!(same.weight, d.weight);
                kept += usize::from(d.weight > 0.0);
                // Births read from the present rate would give it no weight now.
                present_rate_births += usize::from(b.history().rate(b.history().last) > 0.0);
            }
            assert!(
                kept > 20,
                "{kept} rings born before the stop alive at {t} s"
            );
            assert_eq!(present_rate_births, 0);
            // No ring is born at a tick with no rain.
            for d in &alive {
                if d.weight > 0.0 {
                    assert!(d.time < 2.0, "a ring born at {} s in the dry spell", d.time);
                }
            }
        }
        // After the restart: rings alive, none with a birth in the dry spell; births read
        // from the present rate would give weight to the dry spell's scheduled drops.
        let t = 2.65;
        let b = at(t, &stop_restart);
        let alive = b.drops(band, t);
        let dry: Vec<_> = alive
            .iter()
            .filter(|d| (2.0..2.6).contains(&d.time))
            .collect();
        assert!(dry.iter().all(|d| d.weight == 0.0));
        assert!(alive.iter().any(|d| d.weight > 0.0 && d.time >= 2.6));
        assert!(
            !dry.is_empty(),
            "the dry spell has scheduled drops that did not fall"
        );
    }
    // The field is the table's rings of those drops, through the dry spell.
    for t in [2.05, 2.3, 2.59] {
        let b = at(t, &stop_restart);
        for p in points() {
            let own = own_sum(&b, p, t);
            let got = b.sample(p, t).height;
            assert!(
                (got - own).abs() <= 1e-12 * own.abs().max(1e-9),
                "{got} against {own}"
            );
        }
    }
    // A second after a stop with no restart the field is exactly nothing.
    let field = still();
    let (plain, _) = map(None);
    for t in [3.0 + TICK, 3.5, 4.2] {
        let (m, _) = map(Some(history(t, stop)));
        for p in points() {
            assert_eq!(
                m.surface(&field, p, t),
                plain.surface(&field, p, t),
                "{t} s"
            );
            assert_eq!(m.rain_moments(&field, p, t), 0.0);
        }
    }
    // Just before that second is out, rings remain.
    let (m, _) = map(Some(history(2.9, stop)));
    assert!(
        points()
            .iter()
            .any(|&p| m.surface(&field, p, 2.9) != plain.surface(&field, p, 2.9))
    );
}

/// Marshall and Palmer's drops of 2 mm and over on a square metre each second, by the
/// test's own closed form: the integral of `8000 e^(-ΛD) v(D)` from 2 mm.
fn big_drops(rate: f64) -> f64 {
    let l = 4.1 * rate.powf(-0.21);
    8000.0 * (9.65 * (-2.0 * l).exp() / l - 10.3 * (-2.0 * (l + 0.6)).exp() / (l + 0.6))
}

#[test]
fn scheduled_rings_follow_the_rain_up_to_the_profile_s_count() {
    let p = RainBandsProfile::default();
    for rate in [2.0, 5.0, 10.0, 50.0] {
        let share = (big_drops(rate) / p.count).min(1.0);
        for band in 0..2 {
            let l = p.patches[band];
            // Disjoint seconds: each drop counted in the second it lands.
            let mut n = 0.0;
            let seconds = 25;
            for s in 0..seconds {
                let t = 30.0 - f64::from(s);
                let b = bands(history(t, |_| mm_h(rate)));
                n += b.drops(band, t).iter().map(|d| d.weight).sum::<f64>();
            }
            let want = 0.5 * p.count * l * l * f64::from(seconds) * share;
            // Counting noise, three deviations, and the soft threshold's bias.
            let tol = 3.0 * want.sqrt() + 0.02 * want;
            println!("{rate} mm/h band {band}: {n:.0} rings, the rate gives {want:.0}");
            assert!(
                (n - want).abs() <= tol,
                "{rate} mm/h band {band}: {n} against {want}"
            );
        }
    }
    // Moments: at a steady rate, the table's steady squared slope; and they run out
    // over the second after a stop, to exactly nothing.
    let t = table();
    let steady = bands(history(30.0, |_| mm_h(10.0)));
    let m = steady.moments(30.0);
    let read = {
        let k = t.moments.partition_point(|q| q[0] <= mm_h(10.0));
        let ([x0, y0], [x1, y1]) = (t.moments[k - 1], t.moments[k]);
        y0 + (y1 - y0) * (mm_h(10.0) - x0) / (x1 - x0)
    };
    assert!((m / read - 1.0).abs() < 0.03, "{m} against {read}");
    let stop = |j: i64| if j >= 120 { 0.0 } else { mm_h(10.0) };
    let mut last = f64::INFINITY;
    for k in 0..=60 {
        let time = 2.0 + f64::from(k) / 60.0;
        let v = bands(history(time, stop)).moments(time);
        assert!(
            v <= last * (1.0 + 1e-12),
            "moments grow after a stop at {time} s"
        );
        last = v;
    }
    assert_eq!(bands(history(3.0 + TICK, stop)).moments(3.0 + TICK), 0.0);
}

#[test]
fn the_gain_is_exposure_times_the_depth_response_on_every_body() {
    let field = still();
    let time = 6.0;
    let h = history(time, |_| mm_h(15.0));
    let (m, depths) = map(Some(h.clone()));
    let (plain, _) = map(None);
    let b = bands(h);
    let c = canopy();
    let t = table();
    let tree = prepare(&pair());
    let pit = |k: u32| tree.ground().centre(tree.pit(k));
    // The sea, open and under its roof; the yard's deep puddle in the open and its
    // shallow one under the roof.
    let cases = [
        ([-8.3, 4.6], 1.0, 1.0),
        ([-5.0, 0.5], 0.0, 1.0),
        (pit(0), 1.0, t.response(depths[0])),
        (pit(1), 0.0, t.response(depths[1])),
    ];
    for (p, exposure, response) in cases {
        let s = m.sample(p).expect("on the map");
        let open = c.exposure([p[0], s.row.level, p[1]]);
        assert!((open - exposure).abs() < 1e-12, "exposure {open} at {p:?}");
        let gain = m.rain_gain(&field, p, time, s);
        assert!(
            (gain - open * response).abs() < 1e-12,
            "gain {gain} at {p:?}"
        );
        let with = m.surface(&field, p, time).expect("valid test case");
        let without = plain.surface(&field, p, time).expect("valid test case");
        let rings = b.sample(p, time);
        assert!((with.sample.height - without.sample.height - gain * rings.height).abs() < 1e-15);
        assert!(
            (with.sample.slope[0] - without.sample.slope[0] - gain * rings.slope[0]).abs() < 1e-12
        );
        if exposure == 0.0 {
            assert_eq!(with, without, "none under a roof at {p:?}");
        }
    }
    assert!((t.response(depths[0]) - 1.0).abs() < 1e-12);
    assert!((t.response(depths[1]) - 0.41).abs() < 0.05);
    // On dry ground between the puddles: no rings.
    let sill = tree.ground().centre(tree.pit(0) + 5);
    let s = m.sample(sill).expect("on the map");
    assert_eq!(m.rain_gain(&field, sill, time, s), 0.0);
    // The depth limit a wave layer has. The response keeps rings under a tenth of it on
    // any depth (part 3), so a table whose response is 1 on any depth shows it: on water
    // a tenth of a millimetre deep the gain is held at 0.6 of the depth over the bands'
    // significant height.
    let mut flat = (*t).clone();
    flat.depth_response = vec![[0.0, 1.0], [1.0, 1.0]];
    let mut m = m;
    m.set_rain(Some(
        RainBands::new(
            RainBandsProfile::default(),
            Arc::new(flat),
            b.history().clone(),
            Arc::new(canopy()),
        )
        .expect("valid test case"),
    ));
    let mut shallow = s;
    shallow.depth = 1e-4;
    let hs = 4.0 * b.variance(time).sqrt();
    let held = m.rain_gain(&field, sill, time, shallow);
    let open = c.exposure([sill[0], s.row.level, sill[1]]);
    println!(
        "depth limit: free gain {open}, cap over Hs {}, held {held}",
        0.6e-4 / hs
    );
    assert!(
        hs > 0.0 && 0.6e-4 / hs < open,
        "the limit binds in this case"
    );
    assert!((held - 0.6e-4 / hs).abs() < 1e-15);
    // And the real response stays under it there.
    assert!(t.response(1e-4) < 0.6e-4 / hs);
}
