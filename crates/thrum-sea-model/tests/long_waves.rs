//! Closed forms, world derivatives and extended-precision fixtures.
use thrum_sea_model::{Component, GRAVITY, LongWaves, SeaError};

#[test]
fn one_component_matches_closed_form_at_a_displaced_label() {
    // Label u=3, k=.2, a=2, q=.9, phase chosen so theta=pi/3.
    // World x = u-q*a*sin(theta), h=a*cos(theta).
    // This steepness (.36) needs inversion; evaluating at x is wrong.
    let theta = std::f64::consts::PI / 3.0;
    let omega = (GRAVITY * 0.2).sqrt();
    let waves = LongWaves::new(
        vec![Component {
            wave_vector: [0.2, 0.0],
            amplitude: 2.0,
            phase: theta - 0.6,
        }],
        0.9,
    )
    .unwrap();
    let x = 3.0 - 1.8 * 3.0_f64.sqrt() / 2.0;
    let sample = waves.evaluate([x, 0.0], 0.0).unwrap();
    let slope = -0.2 * 3.0_f64.sqrt() / (1.0 - 0.18);
    let vx = 0.9 * omega;
    let vy = omega * 3.0_f64.sqrt();
    assert!((sample.height - 1.0).abs() < 1e-11);
    assert!((sample.slope[0] - slope).abs() < 1e-11);
    assert!(sample.slope[1].abs() < 1e-14);
    assert!((sample.velocity[0] - vx).abs() < 1e-11);
    assert!((sample.velocity[1] - vy).abs() < 1e-11);
    assert!((sample.height_dt - (vy - slope * vx)).abs() < 1e-11);
    assert!(
        LongWaves::new(
            vec![Component {
                wave_vector: [0.5, 0.0],
                amplitude: 2.0,
                phase: 0.0
            }],
            1.0
        )
        .is_err()
    );
    assert_eq!(
        waves.evaluate([f64::NAN, 0.0], 0.0),
        Err(SeaError::InvalidInput)
    );
}

#[test]
fn spatial_and_time_derivatives_match_world_finite_differences() {
    let waves = LongWaves::new(
        vec![
            Component {
                wave_vector: [0.17, 0.08],
                amplitude: 1.4,
                phase: 0.7,
            },
            Component {
                wave_vector: [-0.04, 0.11],
                amplitude: 0.8,
                phase: 2.1,
            },
        ],
        1.0,
    )
    .unwrap();
    let p = [17.3, -9.0];
    let t = 33.0;
    let e = 1e-4;
    let s = waves.evaluate(p, t).unwrap();
    for axis in 0..2 {
        let mut a = p;
        let mut b = p;
        a[axis] += e;
        b[axis] -= e;
        let slope = (waves.evaluate(a, t).unwrap().height - waves.evaluate(b, t).unwrap().height)
            / (2.0 * e);
        assert!((slope - s.slope[axis]).abs() < 1e-8);
    }
    let dt = (waves.evaluate(p, t + e).unwrap().height - waves.evaluate(p, t - e).unwrap().height)
        / (2.0 * e);
    assert!((dt - s.height_dt).abs() < 1e-7);
}

#[test]
fn far_point_after_a_day_matches_extended_precision() {
    // Decimal 70-digit, with exact binary64 inputs, generated independently by
    // extended_reference.py. The fixture includes nonlinear world inversion.
    let waves = LongWaves::new(
        vec![
            Component {
                wave_vector: [0.17, 0.08],
                amplitude: 1.4,
                phase: 0.7,
            },
            Component {
                wave_vector: [-0.04, 0.11],
                amplitude: 0.8,
                phase: 2.1,
            },
        ],
        1.0,
    )
    .unwrap();
    let sample = waves.evaluate([20000.0, -17000.0], 86400.0).unwrap();
    let expected: Vec<f64> = include_str!("fixtures/extended.txt")
        .split_whitespace()
        .map(|v| v.parse().unwrap())
        .collect();
    assert_eq!(expected.len(), 7);
    let actual = [
        sample.height,
        sample.slope[0],
        sample.slope[1],
        sample.velocity[0],
        sample.velocity[1],
        sample.velocity[2],
        sample.height_dt,
    ];
    for (a, b) in actual.into_iter().zip(expected) {
        assert!((a - b).abs() < 1e-6, "{a} versus {b}");
    }
}
