//! The spectral exponent steps on the weather clock without entering physical queries.
pub mod ground_support;
pub mod support;

use thrum_cloud_model::{Density, Weather};

#[test]
fn angstrom_target_changes_only_the_spectral_exponent() {
    let field = support::field(3, 0.0);
    let surface = ground_support::flat(0.0);
    let ground = ground_support::table(vec![ground_support::row()]);
    let weather = Weather {
        field: &field,
        surface: Some(&surface),
        ground: Some(&ground),
    };
    let mut fixed = support::state(0.6, 0.25);
    ground_support::controls(&mut fixed, vec![1.0], 0.7);
    let mut edited = fixed.clone();
    for bad in [-0.01, 2.01, f64::NAN, f64::INFINITY] {
        assert!(edited.ground_mut().set_angstrom_target(bad).is_err());
        assert_eq!(edited, fixed);
    }
    assert_eq!(
        edited.ground().angstrom_exponent().to_bits(),
        0.0_f64.to_bits()
    );
    edited
        .ground_mut()
        .set_angstrom_target(2.0)
        .expect("valid exponent");
    let mut previous = 0.0;
    for tick in 1..=64 {
        fixed.step(tick, 0.25).expect("control step");
        edited.step(tick, 0.25).expect("edited step");
        let exponent = edited.ground().angstrom_exponent();
        assert!((previous..=2.0).contains(&exponent));
        if tick == 1 {
            assert!(exponent > 0.0 && exponent < 2.0);
        }
        previous = exponent;
        assert_eq!(edited.parameters(), fixed.parameters());
        assert_eq!(
            edited.ground().visual_range().to_bits(),
            fixed.ground().visual_range().to_bits()
        );
        for point in [
            [0.0, 20.0, 0.0],
            [32.0, 805.0, -12.0],
            [500.0, 1200.0, -400.0],
        ] {
            let a = weather.column(point, &fixed);
            let b = weather.column(point, &edited);
            let bits = |c: thrum_cloud_model::Column| {
                [
                    c.coverage,
                    c.kind,
                    c.base,
                    c.top,
                    c.age,
                    c.optical_depth,
                    c.profile[0],
                    c.profile[1],
                ]
                .map(f64::to_bits)
            };
            assert_eq!(bits(a), bits(b));
            for (a, b) in [
                (
                    weather.extinction(point, &fixed, |_| None),
                    weather.extinction(point, &edited, |_| None),
                ),
                (
                    weather.visual_range(point, &fixed, |_| None),
                    weather.visual_range(point, &edited, |_| None),
                ),
            ] {
                assert_eq!(a.value.to_bits(), b.value.to_bits());
                assert_eq!(a.fidelity, b.fidelity);
            }
        }
        let optical_depth = |state| {
            weather
                .optical_depth(
                    [0.0, 4.0, 0.0],
                    [12.0, 140.0, 0.0],
                    state,
                    32,
                    Density::Coarse,
                    |_| None,
                )
                .expect("ground sight line")
        };
        assert_eq!(
            optical_depth(&fixed).value.to_bits(),
            optical_depth(&edited).value.to_bits()
        );
    }
    assert_eq!(previous.to_bits(), 2.0_f64.to_bits());
}
