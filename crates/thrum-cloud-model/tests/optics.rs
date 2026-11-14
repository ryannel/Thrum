//! Optical oracle values and independent sphere quadrature.
pub mod support;
use std::f64::consts::PI;
use support::close;
use thrum_cloud_model::{Orders, Phase, Segment};

/// Independent midpoint integration under u=sin(theta/2)^(1/2).
pub fn sphere(phase: impl Fn(f64) -> f64, n: u32) -> [f64; 2] {
    let mut total = 0.0;
    let mut moment = 0.0;
    for i in 0..n {
        let u = (f64::from(i) + 0.5) / f64::from(n);
        let s2 = u.powi(4);
        let mass = phase(s2) * 16.0 * PI * u.powi(3) / f64::from(n);
        total += mass;
        moment += mass * (1.0 - 2.0 * s2);
    }
    [total, moment / total]
}

#[test]
fn beer_composition_and_later_orders_match_the_independent_oracle() {
    // Printed by scripts/reference/cloud/optical_reference.py; command retained
    // in docs/evidence/CLOUDS/M1-optical-reference.py and M1-optical-reference.txt.
    let a = Segment::beer(0.04, 0.03, 25.0).expect("segment");
    close(a.transmittance, 0.367_879_441_171_442_33, 1e-15);
    close(a.scattering, 0.474_090_419_121_418_25, 1e-15);
    let b = Segment::beer(0.02, 0.01, 40.0).expect("segment");
    let ab = a.then(b);
    close(ab.transmittance, 0.165_298_888_221_586_56, 1e-15);
    close(ab.scattering, 0.575_380_695_596_346_1, 1e-15);
    assert!((ab.scattering - b.then(a).scattering).abs() > 0.01);
    close(
        Segment::beer(0.0, 0.03, 25.0).expect("limit").scattering,
        0.75,
        0.0,
    );
    close(
        Segment::beer(1e-20, 0.03, 25.0).expect("small").scattering,
        0.75,
        1e-15,
    );
    assert_eq!(Orders::WEIGHTS, [0.7, 0.2, 0.07, 0.03]);
    assert_eq!(Orders::EXTINCTION, [1.0, 0.5, 0.25, 0.125]);
    assert_eq!(Orders::LATER_G, [0.43, 0.215, 0.1075]);
}

#[test]
fn point_phase_is_normalised_and_has_its_closed_form_moment() {
    for diameter in [5.0, 20.0, 50.0] {
        let phase = Phase::fit(diameter).expect("fit");
        let result = sphere(|s2| phase.point(s2), 100_000);
        close(result[0], 1.0, 2e-8);
        close(result[1], phase.mean_cosine(), 2e-8);
    }
    let fit = Phase::fit(20.0).expect("fit").parameters();
    // Published supplemental equations 7–10 evaluated independently at d=20.
    for (actual, expected) in fit
        .into_iter()
        .zip([0.994_610, 0.593_791, 27.113_694, 0.498_159])
    {
        close(actual, expected, 0.000_001);
    }
}
