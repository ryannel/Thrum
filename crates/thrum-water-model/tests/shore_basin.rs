//! A disconnected, non-flat lake with its own wave sources and fetch growth.
use crate::support;
use std::f64::consts::FRAC_PI_4;
use support::shore::{bed, config};
use thrum_water_model::{
    Body, BodyKind, Lag, Map, Profile, ShoreConfig, ShoreTransform, Source, SurfProfile, shelter,
};

/// A round lake 20 m deep at its centre in a ring of land, on 8 m cells, with the lake's width
/// as its declared fetch.
struct Lake {
    radius: f64,
    cells: usize,
    origin: f64,
}

impl Lake {
    /// The lake the fetch energy is held to: 700 m across a 221-cell square. Its 2 m isobath,
    /// where the upwind energy is read, lies four and a half cells from the shore, which the
    /// energy's 5% bound needs.
    const FULL: Self = Self {
        radius: 700.0,
        cells: 221,
        origin: -880.0,
    };
    /// A compact basin on the same cells and depth, with dry ground around its coast.
    const COMPACT: Self = Self {
        radius: 250.0,
        cells: 81,
        origin: -320.0,
    };

    fn ground(&self, x: f64, z: f64) -> f64 {
        let r = x.hypot(z);
        if r < self.radius {
            -20.0 * (1.0 - (r / self.radius).powi(2))
        } else {
            (0.06 * 700.0 / self.radius * (r - self.radius)).min(6.0)
        }
    }

    /// Cells from the centre to the edge of the square the deep-water turning is read over:
    /// 480 m for the full lake, scaled with it.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a whole count of 8 m cells"
    )]
    fn deep_cells(&self) -> i32 {
        (60.0 * self.radius / 700.0) as i32
    }

    fn prepare(&self) -> (thrum_water_model::Bed, ShoreConfig) {
        let b = bed([self.origin; 2], 8.0, [self.cells, self.cells], |x, z| {
            self.ground(x, z)
        });
        let mut c = config();
        c.seed = Some([0.0; 2]);
        c.fetch = 2.0 * self.radius;
        (b, c)
    }
}

/// Lags are finite in enclosed water at every stage and absent where no edge source reaches,
/// deep water does not turn, waves turn towards the shore, and exposure follows the analytic
/// circle. The compact lake shows each of these.
#[test]
fn enclosed_water_has_finite_lags_turning_and_exposure() {
    let lake = Lake::COMPACT;
    let (b, c) = lake.prepare();
    let mut edge = c;
    edge.source = Source::EdgeOnly;
    let absent = Lag::solve(&b, edge, 40.0, [1.0, 0.0], 0.0).unwrap();
    for (i, value) in absent.values().iter().enumerate() {
        let p = [
            lake.origin + (i % lake.cells) as f64 * 8.0,
            lake.origin + (i / lake.cells) as f64 * 8.0,
        ];
        if lake.ground(p[0], p[1]) < 0.0 {
            assert!(value.is_infinite());
        }
    }
    let transform = ShoreTransform::prepare(b.clone(), c).unwrap();
    for direction in 0..8 {
        for rung in 0..6 {
            for &stage in transform.stages() {
                for z in 0..lake.cells {
                    for x in 0..lake.cells {
                        let p = [lake.origin + x as f64 * 8.0, lake.origin + z as f64 * 8.0];
                        if lake.ground(p[0], p[1]) < 0.0 {
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
    for z in -lake.deep_cells()..=lake.deep_cells() {
        for x in -lake.deep_cells()..=lake.deep_cells() {
            let p = [f64::from(x) * 8.0, f64::from(z) * 8.0];
            if -lake.ground(p[0], p[1]) <= 10.0 {
                continue;
            }
            let g = gradient(&transform, 20.0, p);
            turn = turn.max(g[1].atan2(g[0]).abs().to_degrees());
            stretch = stretch.max((1.0 / g[0].hypot(g[1]) - 1.0).abs());
        }
    }
    eprintln!("lake deep-water turn {turn:.2} deg, wavelength error {stretch:.5}");
    assert!(turn < 15.0 && stretch < 0.03);
    let r = lake.radius * 0.9_f64.sqrt();
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
                    dot + (dot * dot + lake.radius.powi(2) - p[0].powi(2) - p[1].powi(2)).sqrt();
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
}

/// A lake with nonzero long-wave gain is not flat, and its wave energy at the upwind 2 m
/// isobath, its centre and the downwind 2 m isobath follows the fetch share of the analytic
/// circle. The surface reads only the lowest stage, and the transform prepares each stage
/// apart, so the full lake is prepared at that one stage.
#[test]
fn enclosed_water_has_its_fetch_energy() {
    let lake = Lake::FULL;
    let (b, mut c) = lake.prepare();
    c.stage.max = c.stage.min;
    let r = lake.radius * 0.9_f64.sqrt();
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
                ((dot + (dot * dot + lake.radius.powi(2) - p[0].powi(2) - p[1].powi(2)).sqrt())
                    / c.fetch)
                    .min(1.0)
            })
            .sum::<f64>()
            / 17.0;
        let depth = -lake.ground(p[0], p[1]);
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
