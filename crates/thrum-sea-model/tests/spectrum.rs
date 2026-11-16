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

#[test]
fn a_narrow_swell_keeps_its_height_when_it_turns() {
    for direction in [0.0, 0.2, 0.5, 1.0, 2.5] {
        let mut input = forcing(0.0, true);
        input.swells[0].direction = direction;
        let sea = Spectrum::build(&input).expect("swell");
        let mut total = 0.0;
        let mut outside = 0.0;
        for c in &sea.long {
            let angle = c.wave_vector[1].atan2(c.wave_vector[0]) - direction;
            let energy = c.amplitude.powi(2) * 0.5;
            total += energy;
            if angle.cos() < (PI / 4.0).cos() {
                outside += energy;
            }
        }
        // The earlier cos^4(theta/2) puts substantial energy outside this cone.
        // A narrow swell must not turn into several comparable crossing trains.
        assert!(
            outside / total < 0.05,
            "direction {direction}: {}",
            outside / total
        );
        assert!((sea.significant_height() - 1.7).abs() < 0.0001);
    }
}

#[test]
#[expect(
    clippy::cast_possible_truncation,
    reason = "the reference recovers the admitted integer Fourier indices in minus 127 through 127"
)]
fn the_short_wave_energy_is_distributed_over_the_patch() {
    let spectrum = Spectrum::build(&forcing(8.0, false)).expect("forcing");
    let cascade = &spectrum.cascades[2];
    let share = patch_concentration(cascade);
    assert!(
        share < 0.20,
        "one sixteenth of the sea holds {share:.3} of its energy"
    );
    // The former linearly increasing phases are a translated coherent packet.
    // Retain that actual failure so the regression cannot pass both mechanisms.
    let modes: Vec<_> = cascade
        .components()
        .iter()
        .map(|c| {
            let x = (c.wave_vector[0] * cascade.length() / TAU).round() as i32;
            let z = (c.wave_vector[1] * cascade.length() / TAU).round() as i32;
            let id = (z + 127) * 255 + x + 127;
            let phase = TAU * (f64::from(id + 1) * 0.618_033_988_749_894_9).fract();
            (x, z, 0.5 * c.amplitude.powi(2), phase)
        })
        .collect();
    let coherent =
        thrum_sea_model::Cascade::new(cascade.length(), &modes).expect("historical phases");
    let old_share = patch_concentration(&coherent);
    assert!(
        old_share > 0.20,
        "the former packet must fail this check: {old_share:.3}"
    );
}
fn patch_concentration(cascade: &thrum_sea_model::Cascade) -> f64 {
    let field = cascade.inverse_transform(0.0);
    let mut tiles = [0.0; 16];
    for (index, sample) in field.iter().enumerate() {
        let tile = (index / 256 / 64) * 4 + (index % 256 / 64);
        tiles[tile] += sample.displacement[1].powi(2);
    }
    let total: f64 = tiles.iter().sum();
    tiles.iter().copied().fold(0.0, f64::max) / total
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
