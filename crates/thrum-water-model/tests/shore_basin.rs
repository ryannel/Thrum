//! A disconnected, non-flat lake with its own wave sources and fetch growth.
#[expect(
    dead_code,
    reason = "independent dispersion helper belongs to Snell test"
)]
#[path = "support/shore.rs"]
mod support;
use std::f64::consts::FRAC_PI_4;
use support::{bed, config};
use thrum_water_model::{
    Body, BodyKind, Lag, Map, Profile, ShoreTransform, Source, SurfProfile, shelter,
};

fn ground(x: f64, z: f64) -> f64 {
    let r = x.hypot(z);
    if r < 700.0 {
        -20.0 * (1.0 - (r / 700.0).powi(2))
    } else {
        (0.06 * (r - 700.0)).min(6.0)
    }
}

#[test]
fn enclosed_water_has_finite_lags_turning_and_fetch_energy() {
    let b = bed([-880.0; 2], 8.0, [221, 221], ground);
    let mut c = config();
    c.seed = Some([0.0; 2]);
    let mut edge = c;
    edge.source = Source::EdgeOnly;
    let absent = Lag::solve(&b, edge, 40.0, [1.0, 0.0], 0.0).unwrap();
    for (i, value) in absent.values().iter().enumerate() {
        let p = [
            -880.0 + (i % 221) as f64 * 8.0,
            -880.0 + (i / 221) as f64 * 8.0,
        ];
        if ground(p[0], p[1]) < 0.0 {
            assert!(value.is_infinite());
        }
    }
    let transform = ShoreTransform::prepare(b.clone(), c).unwrap();
    for direction in 0..8 {
        for rung in 0..6 {
            for &stage in transform.stages() {
                for z in 0..221 {
                    for x in 0..221 {
                        let p = [-880.0 + f64::from(x) * 8.0, -880.0 + f64::from(z) * 8.0];
                        if ground(p[0], p[1]) < 0.0 {
                            assert!(
                                transform
                                    .sample(direction, 20.0 * 2.0_f64.powi(rung), stage, p)
                                    .0
                                    .is_finite()
                            );
                        }
                    }
                }
            }
        }
    }
    let mut turn = 0.0_f64;
    let mut stretch = 0.0_f64;
    for z in -60..=60 {
        for x in -60..=60 {
            let p = [f64::from(x) * 8.0, f64::from(z) * 8.0];
            if -ground(p[0], p[1]) <= 10.0 {
                continue;
            }
            let g = gradient(&transform, 20.0, p);
            turn = turn.max(g[1].atan2(g[0]).abs().to_degrees());
            stretch = stretch.max((1.0 / g[0].hypot(g[1]) - 1.0).abs());
        }
    }
    eprintln!("lake deep-water turn {turn:.2} deg, wavelength error {stretch:.5}");
    assert!(turn < 15.0 && stretch < 0.03);
    let r = 700.0 * 0.9_f64.sqrt();
    for az in [30.0_f64, 60.0, 90.0] {
        let a = az.to_radians();
        let p = [r * a.cos(), r * a.sin()];
        let g = gradient(&transform, 40.0, p);
        let angle = ((g[0] * a.cos() + g[1] * a.sin()) / g[0].hypot(g[1]))
            .clamp(-1.0, 1.0)
            .acos()
            .to_degrees();
        assert!(angle < az - 10.0, "{az}: {angle}");
        let expected = (0..17)
            .map(|s| {
                let ray = FRAC_PI_4 * (f64::from(s) / 16.0 - 0.5);
                let dot = p[0] * ray.cos() + p[1] * ray.sin();
                let length =
                    dot + (dot * dot + 700.0_f64.powi(2) - p[0].powi(2) - p[1].powi(2)).sqrt();
                (length / c.fetch).min(1.0)
            })
            .sum::<f64>()
            / 17.0;
        assert!((transform.exposure(0, 0.0, p) - expected).abs() < 0.02);
    }
    assert!(transform.exposure(0, 0.0, [-r, 0.0]) < 0.1);
    // With a declared fetch shorter than the available upwind water, the energy is whole.
    // With the width as fetch this point is just under one, as its sector fans out.
    assert!(transform.exposure(0, 0.0, [r, 0.0]) > 0.9);
    let forcing = thrum_sea_model::Forcing {
        wind_speed: 10.0,
        wind_direction: 0.0,
        fetch: 1e6,
        swells: vec![],
    };
    let gains = shelter(&forcing, c.fetch).unwrap();
    assert!(gains[0] > 0.0);
    let water = Body {
        name: "hollow".into(),
        kind: BodyKind::Basin { seed: [0.0; 2] },
        level: 0.0,
        stage: c.stage,
        fetch: c.fetch,
        gains,
    };
    let mut map = Map::prepare(
        b,
        vec![water],
        Profile {
            margin_height: c.margin,
            swash_ratio: 0.7,
        },
    )
    .unwrap();
    map.enable_surf(SurfProfile::default()).unwrap();
    let field = thrum_water_model::Field {
        layers: std::array::from_fn(|i| {
            thrum_sea_model::LongWaves::new(
                if i == 0 {
                    vec![thrum_sea_model::Component {
                        wave_vector: [std::f64::consts::TAU / 40.0, 0.0],
                        amplitude: 0.5,
                        phase: 0.0,
                    }]
                } else {
                    vec![]
                },
                0.0,
            )
            .expect("contractive regular wave")
        }),
    };
    let mut maximum = 0.0_f64;
    for t in 0..32 {
        maximum = maximum.max(
            map.surface(&field, [r, 0.0], f64::from(t) * 0.25)
                .unwrap()
                .sample
                .height,
        );
    }
    assert!(
        maximum > gains[0] * 0.1,
        "a lake with nonzero long-wave gain must not be flat"
    );
    let period = std::f64::consts::TAU / field.layers[0].components()[0].frequency();
    for p in [[-r, 0.0], [0.0; 2], [r, 0.0]] {
        let samples: Vec<_> = (0..128)
            .map(|i| {
                map.surface(&field, p, period * f64::from(i) / 128.0)
                    .expect("finite wet lake surface")
                    .sample
                    .height
            })
            .collect();
        let mean = samples.iter().sum::<f64>() / 128.0;
        let power = samples.iter().map(|h| (h - mean).powi(2)).sum::<f64>() / 128.0;
        // Expected fetch from the analytic circle, independent of the model's ray.
        let share = (0..17)
            .map(|s| {
                let a = FRAC_PI_4 * (f64::from(s) / 16.0 - 0.5);
                let dot = p[0] * a.cos() + p[1] * a.sin();
                ((dot + (dot * dot + 700.0_f64.powi(2) - p[0].powi(2) - p[1].powi(2)).sqrt())
                    / c.fetch)
                    .min(1.0)
            })
            .sum::<f64>()
            / 17.0;
        let depth = -ground(p[0], p[1]);
        let k0 = std::f64::consts::TAU / 40.0;
        let expected =
            0.5 * (0.5 * gains[0] * thrum_water_model::shoaling(k0, depth)).powi(2) * share;
        assert!(
            (power / expected - 1.0).abs() < 0.05,
            "fetch energy at {p:?}: {power} vs {expected}"
        );
    }
}

fn gradient(t: &ShoreTransform, length: f64, p: [f64; 2]) -> [f64; 2] {
    std::array::from_fn(|axis| {
        let mut lo = p;
        lo[axis] -= 8.0;
        let mut hi = p;
        hi[axis] += 8.0;
        f64::from(axis == 0)
            + (t.sample(0, length, 0.0, hi).0 - t.sample(0, length, 0.0, lo).0) / 16.0
    })
}
