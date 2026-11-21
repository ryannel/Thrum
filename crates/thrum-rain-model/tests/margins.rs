//! M2.3: a damp band above a falling waterline. A fall of the level adds to it, a rise
//! takes from it, and it shrinks by `exp(-dt * drying / pore capacity)` under the hollow's
//! rim. The test works each expected band from the rates; a hollow's band in the rain
//! state follows the rule on the levels the test reads; a shore's band is kept by the same
//! rule from its stage, above the wave support `thrum-water-model` gives; and neither
//! holds water in the account.
use crate::support;
use std::f64::consts::{LN_2, TAU};
use support::{mm_h, pothole, prepare};
use thrum_rain_model::{
    Canopy, GroundClass, GroundStores, Rain, RainState, TickReport, band_fill, band_step,
    shore_fill,
};
use thrum_sea_model::{Component, LongWaves};
use thrum_water_model::{Bed, Body, BodyKind, Field, Map, Profile, Stage, limit};

const DT: f64 = 1.0;
/// Paving's pores, metres, and a drying rate of 6 mm/h.
const PORES: f64 = 0.3e-3;

fn drying() -> f64 {
    mm_h(6.0)
}

/// The band's decay rate, per second: drying in metres a second over the pores.
fn k() -> f64 {
    drying() / 1000.0 / PORES
}

/// A band stepped by the rule through levels, from nothing.
fn run(levels: impl Iterator<Item = f64>, drying: f64, rim: f64) -> Vec<f64> {
    let mut band = 0.0;
    let mut last = None;
    levels
        .map(|l| {
            if let Some(before) = last {
                band = band_step(band, before, l, DT, drying, PORES, rim);
            }
            last = Some(l);
            band
        })
        .collect()
}

#[test]
fn a_steady_fall_leaves_a_band_of_its_rate_times_the_pores_over_the_drying() {
    // A tick is under 0.02 of the pore capacity over the drying rate, the range in which
    // the band settles within 2% of the steady value.
    assert!(k() * DT < 0.02);
    for fall in [1.0e-3 / 600.0, 20.0e-3 / 600.0] {
        let bands = run((0..5000).map(|t| -fall * f64::from(t)), drying(), 1.0);
        let steady = fall / k();
        let got = bands[bands.len() - 1];
        println!(
            "fall {:.2} mm/min: band {:.3} mm, the rates give {:.3} mm",
            fall * 6e4,
            got * 1e3,
            steady * 1e3
        );
        assert!((got / steady - 1.0).abs() < 0.02);
    }
    // A slow fall leaves a thin band and a fast one a wide band.
    let slow = run((0..5000).map(|t| -1e-6 * f64::from(t)), drying(), 1.0);
    let fast = run((0..5000).map(|t| -1e-5 * f64::from(t)), drying(), 1.0);
    assert!(fast[4999] > 9.0 * slow[4999]);
}

#[test]
fn a_rise_leaves_none_a_stop_halves_it_and_without_drying_it_stays() {
    // A rising puddle leaves none.
    let rising = run((0..1000).map(|t| 1e-6 * f64::from(t)), drying(), 1.0);
    assert!(rising.iter().all(|&b| b == 0.0));
    // A rise takes from the band, down to nothing.
    assert!((band_step(2e-3, 0.0, 0.5e-3, DT, 0.0, PORES, 1.0) - 1.5e-3).abs() < 1e-18);
    assert_eq!(band_step(2e-3, 0.0, 3e-3, DT, 0.0, PORES, 1.0), 0.0);
    // A rise as high as the band leaves none, and a smaller one the rest less a tick's
    // drying.
    assert_eq!(band_step(2e-3, 0.0, 2e-3, DT, drying(), PORES, 1.0), 0.0);
    let rest = band_step(2e-3, 0.0, 0.5e-3, DT, drying(), PORES, 1.0);
    assert!((rest - 1.5e-3 * (-k() * DT).exp()).abs() < 1e-18);
    // When the level stops falling, the band halves in ln 2 times the pores over the
    // drying.
    let fall = 1e-5;
    let levels = (0..3000).map(|t| -fall * f64::from(t.min(2000)));
    let bands = run(levels, drying(), 1.0);
    let at_stop = bands[2000];
    let half = LN_2 / k();
    let t_half = bands[2000..]
        .iter()
        .position(|&b| b <= 0.5 * at_stop)
        .expect("it halves");
    println!("halves in {t_half} s; the rates give {half:.1} s");
    assert!((t_half as f64 - half).abs() <= DT);
    // With drying at nothing it stays.
    let kept = run((0..3000).map(|t| -fall * f64::from(t.min(2000))), 0.0, 1.0);
    assert!((kept[2999] - 2000.0 * fall).abs() < 1e-15);
    // It stays under the rim.
    let rimmed = run((0..3000).map(|t| -fall * f64::from(t)), 0.0, 4e-3);
    assert_eq!(rimmed[2999], 4e-3);
}

/// A pothole drains by percolation and drying until it is empty; its hollow's band in the
/// rain state is the rule on the levels the test reads before and after each tick, and
/// it is kept after the hollow has emptied until it has faded.
#[test]
fn a_hollow_s_band_follows_its_level_and_outlasts_its_water() {
    let tree = prepare(&pothole(8, 8));
    let class = GroundClass {
        soak: mm_h(10.0),
        percolation: mm_h(5.0),
        pores: PORES,
        relief: 1e-3,
        steepest: 1.0,
        share: 0.47,
        porosity: 1.0,
        film_roughness: 0.05,
    };
    let mut ground = GroundStores::new(&[class]).expect("valid test case");
    let mut state = RainState::new(&tree);
    let mut report = TickReport::default();
    let hollow = tree.labels()[tree.pit(0)];
    let h = hollow as usize;
    let mut own = 0.0;
    let mut emptied = None;
    let mut account = Vec::new();
    for t in 0..30_000 {
        if emptied.is_some_and(|(e, _)| t > e + 200) {
            break;
        }
        let rain = if t < 2000 {
            Rain {
                flux: mm_h(40.0),
                drying: 0.0,
            }
        } else {
            Rain {
                flux: 0.0,
                drying: drying(),
            }
        };
        let before = state.levels(&tree)[h];
        state
            .step_with(&tree, rain, &ground, 0, DT, 1, &mut report)
            .expect("valid test case");
        ground.step(rain, DT).expect("valid test case");
        let after = state.levels(&tree)[h];
        let rim = tree.node(hollow).sill - after;
        own = band_step(own, before, after, DT, rain.drying, PORES, rim);
        assert_eq!(state.bands()[h], own, "at {t} s");
        if emptied.is_none() && t > 2000 && state.held() == 0.0 {
            emptied = Some((t, own));
        }
        let a = state.account();
        account.push((a.rain, a.sinks(), a.initial));
    }
    let (t, band) = emptied.expect("the pothole empties");
    println!("emptied at {t} s with a band of {:.3} mm", band * 1e3);
    assert!(band > 0.0, "an emptied hollow keeps its band");
    let left = state.bands()[h];
    assert!(left < band && left > 0.0, "it fades: {left}");
    // The wetness at the pit reads the band: dry ground, its pores damp from the band.
    let pit = tree.ground().centre(tree.pit(hollow));
    let z = tree.ground().heights()[tree.pit(hollow)];
    let w = state
        .wetness(
            &tree,
            &ground,
            0,
            &Canopy::open(),
            [pit[0], z + 1e-4, pit[1]],
            [0.0, 1.0, 0.0],
            None,
        )
        .expect("on the body");
    let level = state.levels(&tree)[h];
    assert!(
        (w.pores - band_fill(z + 1e-4 - level, left).max(ground.store(0, 0).fill)).abs() < 1e-15
    );
    // A band holds no water: the account closes on the puddles, the ground and the sinks.
    let held = state.held() + state.ground_held(&tree, &ground, 0);
    let (rain, sinks, sources) = account[account.len() - 1];
    assert!((rain + sources - held - sinks).abs() < 1e-9 * rain);
}

/// A bowl's basin with a long wave whose swash washes its shore. The stage falls; the
/// shore's band is the rule on the stage's fall; inside the wave support the pores are
/// full, and above it they fall off over the band.
#[test]
fn a_shore_s_band_stands_above_the_wave_support_the_water_model_gives() {
    let n = 41;
    let heights = (0..n * n)
        .map(|i| {
            let (x, z) = ((i % n) as f64 - 20.0, (i / n) as f64 - 20.0);
            0.05 * (x * x + z * z)
        })
        .collect();
    let bed = Bed::new([0.0, 0.0], 1.0, [n, n], heights).expect("valid test case");
    let profile = Profile {
        margin_height: 1.5,
        swash_ratio: 0.7,
    };
    let mut map = Map::prepare(
        bed,
        vec![Body {
            name: "pond".into(),
            kind: BodyKind::Basin { seed: [20.0, 20.0] },
            level: 3.0,
            stage: Stage {
                min: -1.0,
                max: 0.0,
                value: 0.0,
            },
            fetch: 1e3,
            gains: [1.0, 0.0, 0.0, 0.0],
        }],
        profile,
    )
    .expect("valid test case");
    let field = Field {
        layers: std::array::from_fn(|i| {
            let c = if i == 0 {
                vec![Component {
                    wave_vector: [TAU / 60.0, 0.0],
                    amplitude: 0.1,
                    phase: 0.0,
                }]
            } else {
                Vec::new()
            };
            LongWaves::new(c, 0.0).expect("valid test case")
        }),
    };
    // The stage falls 0.5 m over 500 s, then holds.
    let mut band = 0.0;
    let mut stage = 0.0;
    for t in 0..520 {
        let next = -0.5 * (f64::from(t + 1) / 500.0).min(1.0);
        band = band_step(
            band,
            3.0 + stage,
            3.0 + next,
            DT,
            drying(),
            PORES,
            profile.margin_height,
        );
        map.set_stage(0, next).expect("valid test case");
        stage = next;
    }
    assert!(band > 0.0);
    let (mut inside, mut above) = (0, 0);
    for i in 0..200 {
        let p = [20.0 + 6.0 + 0.02 * f64::from(i), 20.0];
        let fill = shore_fill(&map, &field, p, &[band]);
        // The test's own support: the row and depth at the point, and the swash depth
        // the water model's depth limit gives there.
        let Some(s) = map.sample(p) else {
            assert_eq!(fill, 0.0);
            continue;
        };
        let swash = limit(s.row, s.depth, field.variances(), profile).swash_depth;
        assert!(swash > 0.0);
        if s.depth > -swash {
            assert_eq!(fill, 1.0, "inside the wave support at {p:?}");
            inside += 1;
        } else {
            let want = (1.0 - (-s.depth - swash) / band).max(0.0);
            assert!(
                (fill - want).abs() < 1e-15,
                "{fill} against {want} at {p:?}"
            );
            above += usize::from(want > 0.0 && want < 1.0);
        }
    }
    assert!(
        inside > 5 && above > 5,
        "inside {inside}, in the band {above}"
    );
}
