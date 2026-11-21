//! Independent fetch spectrum quadrature and saturation checks.
use crate::support;
use std::f64::consts::{PI, TAU};
use support::fixtures::{body, close, grid, profile};
use thrum_sea_model::{Forcing, GRAVITY, Swell};
use thrum_water_model::{BodyKind, Map, Row, limit, shelter};

// Independent frequency-domain JONSWAP integration. No spectrum/model call produces
// the expected values. Simpson's rule in log frequency uses the dω Jacobian.
fn energies(wind: f64, fetch: f64) -> [f64; 4] {
    let lengths = [160.0, 37.0 * 1.001_f64.sqrt(), 9.0 * 1.003_f64.sqrt()];
    let k = [
        1e-5,
        TAU / 20.0,
        PI * 256.0 / lengths[0],
        PI * 256.0 / lengths[1],
        PI * 256.0 / lengths[2],
    ];
    let x = fetch * GRAVITY / wind.powi(2);
    let peak = (22.0 * GRAVITY / wind * x.powf(-1.0 / 3.0)).max(0.877 * GRAVITY / wind);
    let alpha = (0.076 * x.powf(-0.22)).max(0.0081);
    let gamma: f64 = if peak > 0.877 * GRAVITY / wind {
        3.3
    } else {
        1.0
    };
    std::array::from_fn(|band| {
        let lo = (GRAVITY * k[band]).sqrt().ln();
        let hi = (GRAVITY * k[band + 1]).sqrt().ln();
        let n = 24000;
        let step = (hi - lo) / f64::from(n);
        let density = |i| {
            let omega = (lo + f64::from(i) * step).exp();
            let sigma: f64 = if omega <= peak { 0.07 } else { 0.09 };
            let r = (-0.5 * ((omega / peak - 1.0) / sigma).powi(2)).exp();
            alpha
                * GRAVITY.powi(2)
                * omega.powi(-4)
                * (-1.25 * (peak / omega).powi(4)).exp()
                * gamma.powf(r)
        };
        (0..=n)
            .map(|i| {
                density(i)
                    * if i == 0 || i == n {
                        1.0
                    } else if i % 2 == 0 {
                        2.0
                    } else {
                        4.0
                    }
            })
            .sum::<f64>()
            * step
            / 3.0
    })
}

#[test]
fn fetch_gains_match_independent_band_integrals_and_exclude_swell() {
    let forcing = Forcing {
        wind_speed: 10.0,
        wind_direction: 0.4,
        fetch: 1e6,
        swells: vec![],
    };
    let reference = energies(10.0, 1e6);
    let mut previous = [0.0; 4];
    for fetch in [200.0, 1000.0, 5000.0, 50000.0, 1e6] {
        let gains = shelter(&forcing, fetch).unwrap();
        let own = energies(10.0, fetch);
        for i in 0..4 {
            let expected = (own[i] / reference[i]).sqrt().min(1.0);
            assert!(
                (gains[i] - expected).abs() < 2e-5,
                "band {i}: {} vs {expected}",
                gains[i]
            );
            assert!(gains[i] >= previous[i] - 1e-12 && gains[i] <= 1.0);
        }
        if fetch < 201.0 {
            assert!(gains[3] > 0.9);
        }
        if (fetch - 5000.0).abs() < 1.0 {
            assert!(gains[0] < 0.05);
        }
        previous = gains;
    }
    for g in previous {
        close(g, 1.0);
    }
    let swell = Swell {
        significant_height: 1.5,
        wavelength: 100.0,
        direction: 0.0,
    };
    let windy = Forcing {
        swells: vec![swell],
        ..forcing.clone()
    };
    let plain = shelter(&forcing, 50000.0).unwrap();
    let with = shelter(&windy, 50000.0).unwrap();
    // Swell's variance is Hs²/16, all in the long band. The product of squared
    // gain and total energy must remain the local wind-only energy.
    assert!(
        (plain[0].powi(2) * reference[0]
            - with[0].powi(2) * (reference[0] + 1.5_f64.powi(2) / 16.0))
            .abs()
            < 1e-6
    );
    let calm = Forcing {
        wind_speed: 0.0,
        ..windy
    };
    assert_eq!(shelter(&calm, 50000.0).unwrap(), [0.0; 4]);
}

#[test]
fn depth_limit_reports_removed_power_and_row_derived_swash() {
    let row = Row {
        level: 0.0,
        current: [0.0; 2],
        gains: [1.0, 0.5, 0.25, 1.0],
    };
    let power = [0.25, 0.04, 0.01, 0.001];
    let p = profile();
    let expected_swash =
        (0.7 * 4.0 * (0.25_f64 + 0.04 * 0.25 + 0.01 * 0.0625 + 0.001).sqrt()).min(1.5);
    let deep = limit(row, 100.0, power, p);
    assert_eq!(deep.gains, row.gains);
    assert_eq!(deep.lost_power, [0.0; 4]);
    close(deep.swash_depth, expected_swash);
    for depth in [2.0, 0.1, 0.0, -0.2, -expected_swash, -2.0] {
        let a = limit(row, depth, power, p);
        let cap = 0.6 * (depth + expected_swash).max(0.0);
        for (i, &power) in power.iter().enumerate() {
            close(a.gains[i], row.gains[i].min(cap / (4.0 * power.sqrt())));
            close(
                a.lost_power[i],
                power * (row.gains[i].powi(2) - a.gains[i].powi(2)),
            );
        }
    }
    assert!(limit(row, 0.0, power, p).gains[0] > 0.0);
    let a = limit(row, 0.001, power, p);
    let b = limit(row, -0.001, power, p);
    assert!((a.gains[0] - b.gains[0]).abs() < 0.001);
    close(limit(row, 0.0, [0.0; 4], p).swash_depth, 0.0);
}

#[test]
fn body_fetch_drives_its_row_and_forcing_changes_are_transactional() {
    let sea = body("exposed", BodyKind::Sea, 0.0);
    let mut basin = body("sheltered", BodyKind::Basin { seed: [10.0, 10.0] }, 5.0);
    basin.fetch = 5000.0;
    let bed = grid([21, 21], |x, z| 2.0 + (x - 10.0).hypot(z - 10.0));
    let mut map = Map::prepare(bed, vec![sea, basin], profile()).unwrap();
    let wind = Forcing {
        wind_speed: 10.0,
        wind_direction: 0.0,
        fetch: 1e6,
        swells: vec![Swell {
            significant_height: 1.0,
            wavelength: 100.0,
            direction: 0.0,
        }],
    };
    map.set_forcing(&wind).unwrap();
    assert_eq!(map.row(0, [-1.0, 0.0]).gains, [1.0; 4]);
    assert_eq!(
        map.row(1, [10.0, 10.0]).gains,
        shelter(&wind, 5000.0).unwrap()
    );
    let old = map.row(1, [10.0, 10.0]);
    assert!(
        map.set_forcing(&Forcing {
            wind_speed: f64::NAN,
            ..wind.clone()
        })
        .is_err()
    );
    assert_eq!(map.row(1, [10.0, 10.0]), old);
    map.set_forcing(&Forcing {
        wind_speed: 0.0,
        ..wind
    })
    .unwrap();
    close(map.row(1, [10.0, 10.0]).gains[0], 0.0);
}
