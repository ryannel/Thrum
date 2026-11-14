//! Loop dispersion and independent finite-difference velocity checks.
use std::f64::consts::TAU;
use thrum_sea_model::{Forcing, LOOP_SECONDS, Spectrum, cascade_frequency, loop_cycles};

#[test]
fn cascade_loop_preserves_dispersion_and_long_waves_do_not_loop() {
    let sea = Spectrum::build(&Forcing {
        wind_speed: 20.0,
        wind_direction: 0.4,
        fetch: 1e6,
        swells: vec![],
    })
    .unwrap();
    for band in &sea.cascades {
        for c in band.components() {
            let omega = cascade_frequency(*c);
            assert!((omega - c.frequency()).abs() <= TAU / (2.0 * LOOP_SECONDS));
            assert!((omega * LOOP_SECONDS / TAU - f64::from(loop_cycles(*c))).abs() < 1e-11);
        }
        let p = [3.2, -17.0];
        let a = band.direct(p, 13.375);
        let b = band.direct(p, 13.375 + LOOP_SECONDS);
        assert_eq!(a.displacement, b.displacement);
        let h = 1e-5;
        let before = band.direct(p, 13.375 - h);
        let after = band.direct(p, 13.375 + h);
        for axis in 0..3 {
            assert!(
                ((after.displacement[axis] - before.displacement[axis]) / (2.0 * h)
                    - a.velocity[axis])
                    .abs()
                    < 1e-7
            );
        }
    }
    for c in &sea.long {
        assert!(
            (c.frequency().powi(2)
                - thrum_sea_model::GRAVITY * c.wave_vector[0].hypot(c.wave_vector[1]))
            .abs()
                < 1e-14
        );
    }
    assert!(sea.long.iter().any(|c| {
        (c.anchored_phase([0.0; 2], 0.0) - c.anchored_phase([0.0; 2], LOOP_SECONDS)).abs() > 0.1
    }));
}
