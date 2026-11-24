//! Hold the first-arrival solve to an independently integrated Snell phase.
use crate::support;
use std::f64::consts::TAU;
use support::shore::{bed, config, wave_number};
use thrum_water_model::{Lag, ShoreTransform};

#[test]
fn snell_phase_and_angle_on_two_metre_cells() {
    // Sufficient alongshore extent keeps side sources outside the measured rays.
    let bed = bed([0.0, -800.0], 2.0, [601, 801], |x, _| (x - 800.0) / 10.0);
    let mut config = config();
    config.stage.max = 0.0;
    let k0 = TAU / 80.0;
    for degrees in [0.0_f64, 30.0, 60.0] {
        let angle = degrees.to_radians();
        let dir = [angle.cos(), angle.sin()];
        let lag = Lag::solve(&bed, config, 80.0, dir, 0.0).unwrap();
        let mut integral = 0.0;
        let mut worst = 0.0_f64;
        let mut wrong_plain = 0.0_f64;
        let mut wrong_local = 0.0_f64;
        for i in 1..400 {
            for s in 0..40 {
                let x = (f64::from(i - 1) + (f64::from(s) + 0.5) / 40.0) * 2.0;
                let d = ((800.0 - x) / 10.0).max(0.5);
                let k = wave_number(k0, d);
                integral += (k * k - (k0 * dir[1]).powi(2)).sqrt() * 2.0 / 40.0;
            }
            let x = f64::from(i) * 2.0;
            let actual = k0 * (dir[0] * x + lag.sample(&bed, [x, 0.0]).0);
            worst = worst.max((actual - integral).abs());
            wrong_plain = wrong_plain.max((k0 * dir[0] * x - integral).abs());
            let local = wave_number(k0, ((800.0 - x) / 10.0).max(0.5));
            wrong_local = wrong_local
                .max(((local * local - (k0 * dir[1]).powi(2)).sqrt() * x - integral).abs());
        }
        let l = lag.sample(&bed, [778.0, 0.0]).0;
        let r = lag.sample(&bed, [782.0, 0.0]).0;
        let b = lag.sample(&bed, [780.0, -2.0]).0;
        let t = lag.sample(&bed, [780.0, 2.0]).0;
        let theta = (dir[1] + (t - b) / 4.0).atan2(dir[0] + (r - l) / 4.0);
        let expected = (k0 * dir[1] / wave_number(k0, 2.0)).asin();
        eprintln!(
            "incidence {degrees}: worst {worst:.4} rad, angle {:.3} vs {:.3}",
            theta.to_degrees(),
            expected.to_degrees()
        );
        assert!(worst < 0.5);
        assert!((theta - expected).abs().to_degrees() < 1.0);
        assert!(wrong_plain > 4.0 && wrong_local > 20.0);
        for p in [[0.0, 0.0], [0.0, -800.0], [-2.0, 0.0]] {
            assert_eq!(lag.sample(&bed, p).0, 0.0);
        }
    }
}

#[test]
fn preparation_covers_all_directions_wavelengths_and_stages() {
    let b = bed([0.0; 2], 2.0, [9, 9], |_, _| -20.0);
    let transform = ShoreTransform::prepare(b, config()).unwrap();
    assert_eq!(transform.stages(), &[0.0, 0.5, 1.0, 1.5, 2.0]);
    assert_eq!(transform.layers(), 240);
    for direction in 0..8 {
        for stage in transform.stages() {
            for r in 0..6 {
                assert!(
                    transform
                        .sample(direction, 20.0 * 2.0_f64.powi(r), *stage, [8.0; 2])
                        .0
                        .is_finite()
                );
            }
        }
    }
}

#[test]
fn spectrum_sector_boundaries_have_the_same_tie_rule_at_every_wavelength() {
    use std::f64::consts::{FRAC_PI_4, TAU};
    use thrum_water_model::shore_sector;
    for sector in 0_u32..8 {
        let boundary = (f64::from(sector) + 0.5) * FRAC_PI_4;
        for wavelength in [20.0, 31.7, 156.0, 640.0] {
            let vector = |angle: f64| {
                let (z, x) = angle.sin_cos();
                [x * TAU / wavelength, z * TAU / wavelength]
            };
            let following = (sector + 1) % 8;
            assert_eq!(shore_sector(vector(boundary - 1e-9)), sector);
            assert_eq!(shore_sector(vector(boundary)), following);
            assert_eq!(shore_sector(vector(boundary + 1e-9)), following);
        }
    }
}

#[test]
fn regular_wave_swash_uses_one_phase_and_keeps_both_waterline_extrema() {
    use thrum_sea_model::{Component, LongWaves};
    use thrum_water_model::{Body, BodyKind, Field, Map, Profile, Sheet, Stage, SurfProfile};

    let bed = bed([-40.0, -16.0], 1.0, [45, 33], |x, _| x / 10.0);
    let mut map = Map::prepare(
        bed,
        vec![Body {
            name: "Plane beach".into(),
            kind: BodyKind::Sea,
            level: 0.0,
            stage: Stage {
                min: 0.0,
                max: 0.0,
                value: 0.0,
            },
            fetch: 1_000.0,
            gains: [1.0, 0.0, 0.0, 0.0],
        }],
        Profile {
            margin_height: 1.0,
            swash_ratio: 0.7,
        },
    )
    .expect("plane beach");
    map.enable_surf(SurfProfile::default())
        .expect("shore transform");
    let contour = [0.0, 0.0];
    let wavelength = 40.0;
    let k = TAU / wavelength;
    let amplitude = 0.4;
    let shore = map.shore_transform(0).expect("prepared shore");
    let (lag, _) = shore.sample(0, wavelength, 0.0, contour);
    assert!(lag.is_finite() && (k * lag).cos() < 0.8, "{lag}");
    let field = Field {
        layers: [
            vec![Component {
                wave_vector: [k, 0.0],
                amplitude,
                phase: -k * (contour[0] + lag),
            }],
            Vec::new(),
            Vec::new(),
            Vec::new(),
        ]
        .map(|components| LongWaves::new(components, 0.0).expect("wave")),
    };
    let swash = 0.7 * 2.0 * 2.0_f64.sqrt() * amplitude;
    let c = 0.6 / (2.0 * 2.0_f64.sqrt());
    let half_period = std::f64::consts::PI / (thrum_sea_model::GRAVITY * k).sqrt();
    for (time, q) in [(0.0, 1.0), (half_period, -1.0)] {
        let sheet = map.surf_sheet(&field, 0, time).expect("sheet");
        let edge = 10.0 * c * swash * q / (1.0 + c * q);
        let mut previous = f64::INFINITY;
        for i in -32..=32 {
            let rest = [edge + f64::from(i) * 0.01, 0.0];
            let sample = sheet.forward(rest);
            let thickness = sample.height - rest[0] / 10.0;
            assert!(thickness < previous, "{rest:?} {thickness}");
            assert!((sample.world[0] - rest[0]).abs() < 1e-12);
            if i == 0 {
                assert!(thickness.abs() < 1e-8, "{sample:?} {edge}");
            }
            previous = thickness;
        }
    }
}

#[test]
fn a_wave_shadow_joins_the_ambient_sea_at_the_bed_edge() {
    let b = bed([-20.0; 2], 1.0, [81, 41], |x, z| {
        if x.abs() <= 1.0 && z.abs() <= 17.0 {
            5.0
        } else {
            -4.0
        }
    });
    let mut sea = config();
    sea.stage.max = 0.0;
    let transform = ShoreTransform::prepare(b, sea).expect("open sea and island");
    assert_eq!(transform.exposure(0, 0.0, [5.0, 0.0]), 0.0);
    let near_edge = transform.exposure(0, 0.0, [59.0, 0.0]);
    assert!((0.92..1.0).contains(&near_edge), "{near_edge}");
    for x in [60.0, 61.0] {
        assert_eq!(transform.exposure(0, 0.0, [x, 0.0]), 1.0);
    }
}
