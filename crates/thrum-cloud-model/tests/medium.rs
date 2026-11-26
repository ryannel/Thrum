//! Partitioned integrations and independent light-coefficient expectations.
use crate::ground_support;
use crate::support;
use thrum_cloud_model::{
    Coefficients, Conformance, Handover, Light, Phase, PhaseTable, Sample, Segment, Weather,
};

#[test]
fn coefficients_use_the_beam_and_two_stream_diffuse_term() {
    let phase = Phase::fit(20.0).expect("phase");
    let table = PhaseTable::new(phase, 0.00465, 1025).expect("table");
    let light = Light {
        phases: [table.reference(0.25), 0.1],
        sun_depths: [2.0, 4.0],
        forward_share: 0.75,
        broad_weight: 0.5,
        plane_cosines: [0.5, 0.0],
        vertical_depths: [3.0, 5.0],
        ambient: [0.2, 0.1],
        mean_cosine: phase.mean_cosine(),
    };
    let field = support::field(1, 0.0);
    let surface = ground_support::flat(0.0);
    let ground = ground_support::table(vec![ground_support::row()]);
    let mut state = support::state(0.0, 0.0);
    ground_support::controls(&mut state, vec![1.0], 0.0);
    let w = Weather {
        field: &field,
        surface: Some(&surface),
        ground: Some(&ground),
    };
    let actual = w.medium([0.0, 10.0, 0.0], &state, |_| None, light);
    let beam =
        (-1.0_f64).exp() * table.reference(0.25) + 0.1 * ((-0.5_f64).exp() - (-1.0_f64).exp());
    let reduced_cosine = 0.5 * (1.0 - phase.mean_cosine());
    let escape = 6.0 / 7.0;
    let flux = (escape + (-1.0_f64).exp() / 7.0) / (1.0 + 3.0 * reduced_cosine);
    let mean = (1.0 - flux * (1.0 + 3.0 * reduced_cosine)
        + (2.0 * escape - 1.0) * (1.0 - (-0.5_f64).exp()))
        / (4.0 * std::f64::consts::PI);
    let expected = Coefficients {
        extinction: 0.02,
        solar_kernel: 0.02 * (beam + mean),
        ambient_source: 0.02
            * (0.2 / (1.0 + 0.75 * (1.0 - phase.mean_cosine()) * 3.0)
                + 0.1 / (1.0 + 0.75 * (1.0 - phase.mean_cosine()) * 5.0)),
    };
    let sample = Sample {
        tick: 0,
        origin: [0.0, 10.0, 0.0],
        direction: [1.0, 0.0, 0.0],
        length: 1.0,
        steps: 1,
    };
    assert!(
        Conformance::medium(&[sample], |_| expected, |_| actual)
            .expect("provider")
            .maximum
            < 1e-15
    );
    assert!(
        Conformance::medium(
            &[sample],
            |_| expected,
            |_| Coefficients {
                extinction: f64::NAN,
                ..actual
            }
        )
        .is_err()
    );
    let weighted = actual.weighted(0.4);
    support::close(weighted.extinction, 0.008, 1e-15);
    support::close(weighted.solar_kernel, actual.solar_kernel * 0.4, 1e-15);
    support::close(weighted.ambient_source, actual.ambient_source * 0.4, 1e-15);
    let dim = Light {
        ambient: [0.0; 2],
        phases: [0.0; 2],
        sun_depths: [0.0; 2],
        ..light
    };
    let mixed = w.medium_sources([0.0, 10.0, 0.0], &state, |_| None, dim, |_| light);
    support::close(mixed.extinction, actual.extinction, 1e-15);
    support::close(mixed.solar_kernel, actual.solar_kernel, 1e-15);
    support::close(mixed.ambient_source, actual.ambient_source, 1e-15);
}

#[test]
fn two_shares_compose_and_both_wrong_substitutes_fail() {
    let field = support::field(3, 0.0);
    let surface = ground_support::flat(0.0);
    let mut row = ground_support::row();
    row.drifting = true;
    row.centre = [400.0, 0.0];
    row.radius = [150.0, 1000.0];
    row.liquid_water = (3.912 / 400.0) * 2.0 * 1000.0 * row.droplet_radius / 3.0;
    let ground = ground_support::table(vec![row]);
    let mut state = support::state(0.0, 0.0);
    ground_support::controls(&mut state, vec![1.0], 0.0);
    let w = Weather {
        field: &field,
        surface: Some(&surface),
        ground: Some(&ground),
    };
    let handover = Handover::new(300.0, 500.0).expect("ramp");
    for shadow in [false, true] {
        let identity = Segment {
            transmittance: 1.0,
            scattering: 0.0,
        };
        let mut near = identity;
        let mut far = identity;
        let mut whole = identity;
        for i in 0..40_000 {
            let distance = (f64::from(i) + 0.5) * 0.05;
            let [n, f] = handover.weights(distance);
            assert_eq!(n + f, 1.0);
            let sigma = w.extinction([distance, 10.0, 0.0], &state, |_| None).value;
            let source = if shadow && distance >= 400.0 {
                0.1
            } else {
                1.0
            };
            near = near.then(Segment::beer(sigma * n, sigma * n * source, 0.05).expect("near"));
            far = far.then(Segment::beer(sigma * f, sigma * f * source, 0.05).expect("far"));
            whole = whole.then(Segment::beer(sigma, sigma * source, 0.05).expect("whole"));
        }
        let combined = near.then(far);
        support::close(combined.transmittance, whole.transmittance, 3e-12);
        let ceiling = 0.05 * whole.scattering + 0.005 * whole.scattering;
        assert!((combined.scattering - whole.scattering).abs() <= ceiling);
        let doubled = whole.then(whole);
        assert!(
            (doubled.transmittance - whole.transmittance).abs()
                > 0.05 * whole.transmittance + 0.005
        );
        let unattenuated = near.scattering + far.scattering;
        assert!((unattenuated - whole.scattering).abs() > ceiling);
        println!(
            "shadow={shadow}: whole T {:.6}, S {:.6}; split S error {:.6}, ceiling {ceiling:.6}; both whole T error {:.6}, unattenuated far S error {:.6}",
            whole.transmittance,
            whole.scattering,
            (combined.scattering - whole.scattering).abs(),
            (doubled.transmittance - whole.transmittance).abs(),
            (unattenuated - whole.scattering).abs()
        );
    }
}
