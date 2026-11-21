//! Linear theory's break depth, independently from group velocity and dispersion.
use crate::support;
use std::f64::consts::TAU;
use support::shore::wave_number;
use thrum_water_model::shoaling;

fn expected_gain(k0: f64, d: f64) -> f64 {
    let k = wave_number(k0, d);
    let omega = (9.81 * k0).sqrt();
    let group = omega / k * 0.5 * (1.0 + 2.0 * k * d / (2.0 * k * d).sinh());
    (omega / (2.0 * k0) / group).sqrt()
}

fn break_depth(k0: f64, amplitude: f64, gain: impl Fn(f64, f64) -> f64) -> f64 {
    let (mut lo, mut hi) = (0.5, 10.0);
    for _ in 0..60 {
        let mid = f64::midpoint(lo, hi);
        if 2.0 * amplitude * gain(k0, mid) / (0.78 * mid) > 1.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    f64::midpoint(lo, hi)
}

#[test]
fn regular_wave_and_sets_break_at_the_linear_theory_depth() {
    let k0 = (TAU / 8.0).powi(2) / 9.81;
    let expected = break_depth(k0, 0.5, expected_gain);
    let actual = break_depth(k0, 0.5, shoaling);
    assert!((actual - expected).abs() < 1e-10);
    assert!((actual - 1.64).abs() < 0.01);
    let height = 0.78 * actual;
    let komar = 0.56 * (1.0 / (TAU / k0)).powf(-0.2);
    assert!((height / komar - 1.0).abs() < 0.1);
    let k = (TAU / 9.5).powi(2) / 9.81;
    // Two coherent trains: envelope varies from |a-b| to a+b.
    let (a, b) = (0.45, 0.25);
    let small = break_depth(k, a - b, shoaling);
    let high = break_depth(k, a + b, shoaling);
    assert!(high > small + 1.0);
    eprintln!(
        "regular: depth {actual:.5} m, height {height:.5} m, Komar {komar:.5} m; set {small:.3}..{high:.3} m"
    );
}
