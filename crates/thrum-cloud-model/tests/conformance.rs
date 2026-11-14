//! Weather-column and arbitrary-direction Sun-depth providers share one harness.
pub mod support;
use support::{close, field, state};
use thrum_cloud_model::{Conformance, Density, Error, Sample};

#[test]
fn conformance_finds_an_injected_column_and_path_error() {
    let field = field(7, 0.3);
    let mut state = state(0.8, 0.4);
    state.step(1, 8000.0).expect("time");
    let samples: Vec<_> = (0..20)
        .map(|i| Sample {
            tick: 1,
            origin: [f64::from(i) * 71.0, 700.0, 417.0],
            direction: [0.6, 0.8, 0.0],
            length: 1800.0,
            steps: 256,
        })
        .collect();
    for paths in [false, true] {
        for fidelity in [Density::Full, Density::Coarse] {
            let reference = |s: &Sample| {
                if paths {
                    field.optical_depth(s, &state, fidelity, |_| None, 0.0)
                } else {
                    field.query(s.origin, &state).optical_depth
                }
            };
            let result = Conformance::compare(&samples, reference, reference).expect("finite");
            close(result.maximum, 0.0, 0.0);
            let bad = |s: &Sample| {
                reference(s)
                    + if (s.origin[0] - 213.0).abs() < 0.001 {
                        0.75
                    } else {
                        0.0
                    }
            };
            let result = Conformance::compare(&samples, reference, bad).expect("finite");
            close(result.maximum, 0.75, 1e-12);
            assert_eq!(result.index, 3);
            assert!(matches!(
                Conformance::compare(&samples, reference, |_| f64::NAN),
                Err(Error::Invalid)
            ));
        }
    }
}
