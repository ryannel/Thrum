//! Power response and continuity through changes in forcing.
use thrum_sea_model::{Forcing, LongWaves, Spectrum, Swell, Transition};

#[test]
fn every_power_follows_response_and_every_phase_survives_retargeting() {
    let a = Forcing {
        wind_speed: 0.5,
        wind_direction: 0.0,
        fetch: 1e5,
        swells: vec![],
    };
    let b = Forcing {
        wind_speed: 20.0,
        wind_direction: 2.0,
        fetch: 1e6,
        swells: vec![Swell {
            significant_height: 2.0,
            wavelength: 110.0,
            direction: -1.0,
        }],
    };
    let start = Spectrum::build(&a).unwrap();
    let target = Spectrum::build(&b).unwrap();
    let transition = Transition::new(start.clone(), target.clone(), 10.0, 7.0).unwrap();
    for elapsed in [0.0, 0.001, 1.0, 7.0, 70.0] {
        let value = transition.at(10.0 + elapsed).unwrap();
        let weight = 1.0 - (-elapsed / 7.0).exp();
        let all = |s: &Spectrum| {
            s.long
                .iter()
                .chain(
                    s.cascades
                        .iter()
                        .flat_map(thrum_sea_model::Cascade::components),
                )
                .copied()
                .collect::<Vec<_>>()
        };
        for ((a, b), v) in all(&start).into_iter().zip(all(&target)).zip(all(&value)) {
            assert_eq!(a.phase, v.phase);
            assert_eq!(a.wave_vector, v.wave_vector);
            let expected = a.amplitude.powi(2) * (1.0 - weight) + b.amplitude.powi(2) * weight;
            assert!((v.amplitude.powi(2) - expected).abs() < 1e-12);
        }
    }
    let snapshot = transition.at(13.0).unwrap();
    let again = Transition::new(snapshot.clone(), start.clone(), 13.0, 4.0).unwrap();
    assert_eq!(again.at(13.0).unwrap().long, snapshot.long);
    let p = [17.0, 29.0];
    let before = LongWaves::new(start.long.clone(), 0.1)
        .unwrap()
        .evaluate(p, 10.0)
        .unwrap();
    let at = LongWaves::new(transition.at(10.0).unwrap().long, 0.1)
        .unwrap()
        .evaluate(p, 10.0)
        .unwrap();
    assert_eq!(before, at);
    // Continuity from the right for both nonlinear long waves and every bin.
    let near = transition.at(10.0 + 1e-10).unwrap();
    let after = LongWaves::new(near.long, 0.1)
        .unwrap()
        .evaluate(p, 10.0 + 1e-10)
        .unwrap();
    assert!((after.height - before.height).abs() < 1e-4);
    for (a, b) in start.cascades.iter().zip(&near.cascades) {
        assert!(
            (a.direct(p, 10.0).displacement[1] - b.direct(p, 10.0 + 1e-10).displacement[1]).abs()
                < 1e-4
        );
    }
    assert!(transition.at(f64::NAN).is_err());
    assert!(Transition::new(start, target, 0.0, 0.0).is_err());
}
