//! M3.5, ruling 3 on the CPU: `thrum-water-model` holds the two flow bands' spectrum and an f64
//! evaluator on the cascades' loop, and its surface adds them at a chart's flow gains through the
//! same sum, depth limit and moments as a cascade.
//! - With no wind, the surface over the riffle of M1.1's valley has the chop's rms height, by the
//!   design's law at the map's own depth and speed, within 10%, over a whole loop.
//! - Over a sea, a basin and a chart that is no river's, the surface with no wind is the level,
//!   and none of them has a flow gain.
//! - The bands hold no power that depends on the wind: their gains at a point are the same under
//!   a calm and a gale, and their part of the surface is the same.
//! - Without the bands, every answer is what it was to the bit: the water model's own tests run
//!   unchanged beside this one, and here the surface, the moments and the wave support of the
//!   same map with no bands named match an untouched copy's.
#![expect(
    clippy::unwrap_used,
    reason = "test arithmetic; the test reports what it checked"
)]
use crate::support;
use support::lake::{bed_of, chart_body};
use support::valley::{CENTRE, DISCHARGE, LAKE, MANNING, valley};
use thrum_river_model::{Grid, Profile, key_for_samples, solve_samples};
use thrum_sea_model::{Component, LOOP_SECONDS, LongWaves};
use thrum_water_model::{
    Bed, Body, BodyKind, Field, FlowBands, FlowBandsProfile, Gauge, Map, Profile as WaterProfile,
    Stage,
};

const WATER: WaterProfile = WaterProfile {
    margin_height: 0.3,
    swash_ratio: 0.7,
};

fn still() -> Field {
    Field {
        layers: std::array::from_fn(|_| LongWaves::new(vec![], 0.0).unwrap()),
    }
}

/// A wind's field: a long swell and a wind sea in each short band.
fn windy(height: f64) -> Field {
    Field {
        layers: std::array::from_fn(|i| {
            let k = std::f64::consts::TAU / [40.0, 6.0, 1.5, 0.4][i];
            LongWaves::new(
                vec![Component {
                    wave_vector: [k * 0.8, k * 0.6],
                    amplitude: height / [1.0, 3.0, 9.0, 27.0][i],
                    phase: 0.3 * i as f64,
                }],
                0.0,
            )
            .unwrap()
        }),
    }
}

fn basin(name: &str, seed: [f64; 2], level: f64) -> Body {
    Body {
        name: name.into(),
        kind: BodyKind::Basin { seed },
        level,
        stage: Stage {
            min: 0.0,
            max: 0.0,
            value: 0.0,
        },
        fetch: 1e6,
        gains: [0.0; 4],
    }
}

/// M1.1's valley solved at its spring's discharge: the lake (body 0) and the stream's chart
/// (body 1), with the stream a river of one sample when `river`.
fn stream(river: bool) -> (Grid, Map) {
    let (grid, declaration) = valley(0.25);
    let profile = Profile::default();
    let flows = vec![vec![DISCHARGE]];
    let s = solve_samples(&grid, &declaration, &profile, &flows).unwrap();
    let file = s
        .file(key_for_samples(&grid, &declaration, &profile, &flows))
        .unwrap();
    let bed = bed_of(&grid);
    let chart = file.reaches[0]
        .chart(&bed, |n| (n == "lake").then_some(0))
        .unwrap();
    let lake = basin("lake", declaration.receivers[0].seed, LAKE);
    let mut map =
        Map::prepare(bed.clone(), vec![lake, chart_body("valley", chart)], WATER).unwrap();
    if river {
        let g = file.gauges()[0];
        map.add_river(
            file.river(
                &bed,
                "valley",
                &[1],
                Gauge::Stage(Stage {
                    min: g,
                    max: g,
                    value: g,
                }),
            )
            .unwrap(),
        )
        .unwrap();
    }
    (grid, map)
}

fn bands() -> FlowBands {
    FlowBands::new(FlowBandsProfile::default()).unwrap()
}

/// The chop's rms height by the design's law, `c d min(S, S_cap)`.
fn law(depth: f64, speed: f64) -> f64 {
    let s = MANNING * MANNING * speed * speed / depth.powf(4.0 / 3.0);
    2.0 * depth * s.min(0.6 / 8.0)
}

#[test]
fn with_no_wind_the_riffle_has_the_chop_s_height() {
    let (_, mut map) = stream(true);
    map.set_flow_bands(Some(bands()));
    let field = still();
    // Over the loop at a step under half the fastest mode's period, so every mode's square
    // averages to a half.
    let steps = 16_384;
    let mut worst = 0.0f64;
    for x in [4.0, 5.5, 7.0, 8.5] {
        let p = [x, CENTRE + 0.3];
        let s = map.sample(p).unwrap();
        let level = s.row.level;
        let speed = s.row.current[0].hypot(s.row.current[1]);
        let want = law(s.depth, speed);
        let mut sum = 0.0;
        for k in 0..steps {
            let t = LOOP_SECONDS * f64::from(k) / f64::from(steps);
            let h = map.surface(&field, p, t).unwrap().sample.height - level;
            sum += h * h;
        }
        let got = (sum / f64::from(steps)).sqrt();
        let off = got / want - 1.0;
        worst = worst.max(off.abs());
        println!(
            "riffle at x = {x} m: {:.3} m deep at {:.2} m/s; the law gives {:.2} mm rms, the \
             surface over the loop {:.2} mm ({:+.1}%)",
            s.depth,
            speed,
            want * 1e3,
            got * 1e3,
            100.0 * off
        );
    }
    assert!(worst < 0.1, "{worst}");
}

#[test]
fn a_sea_a_basin_and_a_chart_that_is_no_river_s_lie_at_their_level() {
    // The lake and the stream's chart without a river.
    let (_, mut map) = stream(false);
    map.set_flow_bands(Some(bands()));
    let field = still();
    let lake = [44.0, CENTRE];
    let chart = [5.5, CENTRE + 0.3];
    for (name, p, body) in [("the lake", lake, 0), ("a chart of no river", chart, 1)] {
        let s = map.sample(p).unwrap();
        assert_eq!(s.body, body);
        assert_eq!(map.flow_band_gains(&field, p, s), [0.0; 2]);
        assert_eq!(map.flow_moments(&field, p), 0.0);
        for t in [0.0, 1.7, 333.3] {
            assert_eq!(
                map.surface(&field, p, t).unwrap().sample.height,
                s.row.level
            );
        }
        println!(
            "{name}: no flow gain, and the surface is its level {:.4} m",
            s.row.level
        );
    }
    // A sea over a flat bed.
    let bed = Bed::new([0.5, 0.5], 1.0, [8, 8], vec![-5.0; 64]).unwrap();
    let sea = Body {
        name: "sea".into(),
        kind: BodyKind::Sea,
        level: 0.25,
        stage: Stage {
            min: 0.0,
            max: 0.0,
            value: 0.0,
        },
        fetch: 1e6,
        gains: [1.0; 4],
    };
    let mut map = Map::prepare(bed, vec![sea], WATER).unwrap();
    map.set_flow_bands(Some(bands()));
    let p = [3.3, 4.1];
    let s = map.sample(p).unwrap();
    assert_eq!(map.flow_band_gains(&field, p, s), [0.0; 2]);
    for t in [0.0, 1.7, 333.3] {
        assert_eq!(map.surface(&field, p, t).unwrap().sample.height, 0.25);
    }
    println!("the sea: no flow gain, and the surface is its level 0.25 m");
}

#[test]
fn the_bands_hold_no_power_that_follows_the_wind() {
    let (_, mut map) = stream(true);
    map.set_flow_bands(Some(bands()));
    let mut bare = stream(true).1;
    bare.set_flow_bands(None);
    let p = [5.5, CENTRE + 0.3];
    let s = map.sample(p).unwrap();
    let (calm, gale) = (still(), windy(0.004));
    let g_calm = map.flow_band_gains(&calm, p, s);
    let g_gale = map.flow_band_gains(&gale, p, s);
    println!("flow gains at the riffle: {g_calm:?} in a calm, {g_gale:?} under a wind");
    assert_eq!(g_calm, g_gale);
    // The bands' part of the surface: the surface with them less the surface without.
    let mut worst = 0.0f64;
    for t in [0.0, 2.5, 100.0] {
        let part = |f: &Field| {
            map.surface(f, p, t).unwrap().sample.height
                - bare.surface(f, p, t).unwrap().sample.height
        };
        worst = worst.max((part(&calm) - part(&gale)).abs());
    }
    println!("the bands' part of the surface differs between the two by {worst:e} m");
    assert!(worst < 1e-12);
    // The spectrum is made from its profile alone: two made alike are alike.
    let (a, b) = (bands(), bands());
    assert_eq!(a.variances(), b.variances());
    assert_eq!(a.mode_counts(), b.mode_counts());
    assert_eq!(a.sample(1, p, 7.0), b.sample(1, p, 7.0));
}

#[test]
fn without_the_bands_every_answer_is_what_it_was() {
    let (grid, plain) = stream(true);
    let (_, mut unnamed) = stream(true);
    unnamed.set_flow_bands(Some(bands()));
    unnamed.set_flow_bands(None);
    let field = windy(0.004);
    let z = grid.heights();
    let mut points = 0;
    for i in (0..z.len()).step_by(7) {
        let p = grid.point(i);
        let p = [p[0] + 0.07, p[1] - 0.05];
        for t in [0.0, 1.25, 40.0] {
            let a = plain.surface(&field, p, t).unwrap();
            let b = unnamed.surface(&field, p, t).unwrap();
            assert_eq!(a.sample, b.sample);
            assert_eq!(
                (a.body, a.depth.to_bits(), a.wet),
                (b.body, b.depth.to_bits(), b.wet)
            );
        }
        assert_eq!(plain.flow_moments(&field, p), 0.0);
        assert_eq!(unnamed.flow_moments(&field, p), 0.0);
        assert_eq!(
            plain.wave_support(&field, p),
            unnamed.wave_support(&field, p)
        );
        points += 1;
    }
    println!("{points} points at three times: with the bands taken away, every answer is the same");
}
