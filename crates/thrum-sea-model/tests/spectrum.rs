//! Spectrum partition, quadrature error and physical forcing checks.
use std::f64::consts::{PI, TAU};
use thrum_sea_model::{Forcing, GRAVITY, Spectrum, Swell};

fn forcing(speed: f64, swell: bool) -> Forcing {
    Forcing {
        wind_speed: speed,
        wind_direction: 0.71,
        fetch: 1e8,
        swells: if swell {
            vec![Swell {
                significant_height: 1.7,
                wavelength: 120.0,
                direction: -0.4,
            }]
        } else {
            vec![]
        },
    }
}

fn midpoint(f: &Forcing, low: f64, high: f64, count: u32) -> f64 {
    let step = (high / low).ln() / f64::from(count);
    (0..count)
        .map(|i| f.log_density(low * ((f64::from(i) + 0.5) * step).exp()) * step)
        .sum()
}

#[test]
fn disjoint_layers_conserve_independently_integrated_variance() {
    for speed in [0.0, 8.0, 20.0] {
        for swell in [false, true] {
            let f = forcing(speed, swell);
            let sea = Spectrum::build(&f).unwrap();
            assert_eq!(sea.long.len(), 192);
            let split = TAU / 20.0;
            assert!(
                sea.long
                    .iter()
                    .all(|c| c.wave_vector[0].hypot(c.wave_vector[1]) < split)
            );
            let mut lower = split;
            let mut shares = vec![
                sea.long
                    .iter()
                    .map(|c| 0.5 * c.amplitude.powi(2))
                    .sum::<f64>(),
            ];
            for cascade in &sea.cascades {
                let upper = PI * 256.0 / cascade.length();
                assert!(cascade.components().iter().all(|c| {
                    let k = c.wave_vector[0].hypot(c.wave_vector[1]);
                    k >= lower && k < upper
                }));
                shares.push(cascade.variance());
                lower = upper;
            }
            // Independent global grids, neither aligned to production cells.
            // Midpoint has O(h²) error. Richardson's /3 estimates the fine
            // quadrature error; 16x covers nonasymptotic/rounding uncertainty.
            let coarse = midpoint(&f, 1e-5, lower, 32768);
            let fine = midpoint(&f, 1e-5, lower, 65536);
            let reference_error = (fine - coarse).abs() / 3.0;
            let production_coarse = midpoint(&f, 1e-5, split, 6144)
                + sea
                    .cascades
                    .iter()
                    .enumerate()
                    .map(|(i, c)| {
                        let low = if i == 0 {
                            split
                        } else {
                            PI * 256.0 / sea.cascades[i - 1].length()
                        };
                        midpoint(&f, low, PI * 256.0 / c.length(), 4096)
                    })
                    .sum::<f64>();
            let budget = 16.0
                * ((sea.variance() - production_coarse).abs() / 3.0 + reference_error)
                + 256.0 * f64::EPSILON * fine;
            let smallest = shares
                .into_iter()
                .filter(|s| *s > 0.0)
                .fold(f64::INFINITY, f64::min);
            assert!(
                budget < smallest * 0.01,
                "quadrature cannot hide even 1% of the smallest band: {budget} / {smallest}"
            );
            assert!(
                (sea.variance() - fine).abs() <= budget,
                "{speed}, {swell}: {} versus {fine}, budget {budget}",
                sea.variance()
            );
            for c in &sea.long {
                assert!(
                    (c.frequency().powi(2) - GRAVITY * c.wave_vector[0].hypot(c.wave_vector[1]))
                        .abs()
                        < 1e-12
                );
            }
        }
    }
}

#[test]
fn wind_height_and_fetch_have_physical_response() {
    let heights: Vec<_> = [0.0, 0.5, 0.99, 8.0, 20.0]
        .map(|v| {
            Spectrum::build(&forcing(v, false))
                .unwrap()
                .significant_height()
        })
        .into();
    assert!(heights[2] < 0.05);
    assert!(heights[4] >= 5.0);
    assert!(heights.windows(2).all(|h| h[1] > h[0]));
    let mut limited = forcing(20.0, false);
    limited.fetch = 1000.0;
    assert!(Spectrum::build(&limited).unwrap().significant_height() < heights[4]);
    limited.fetch = f64::NAN;
    assert!(Spectrum::build(&limited).is_err());
}
