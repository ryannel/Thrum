//! Provider conformance and deliberately injected answer faults.
use thrum_sea_model::{Component, Conformance, LongWaves, Sample, SeaError, SeaProvider, compare};

#[test]
fn identical_model_is_zero_and_phase_fault_has_analytic_height() {
    let c = Component {
        wave_vector: [0.2, 0.0],
        amplitude: 1.7,
        phase: 0.0,
    };
    let model = LongWaves::new(vec![c], 0.0).unwrap();
    let faulty = LongWaves::new(
        vec![Component {
            phase: std::f64::consts::PI / 2.0,
            ..c
        }],
        0.0,
    )
    .unwrap();
    let a = Conformance {
        waves: &model,
        tick_seconds: 1.0 / 60.0,
    };
    let b = Conformance {
        waves: &faulty,
        tick_seconds: 1.0 / 60.0,
    };
    let points = [
        [0.0, 0.0],
        [std::f64::consts::PI / 0.8, 0.0],
        [20000.0, -10000.0],
    ];
    let ticks = [0, 1, 100, 5_184_000];
    assert_eq!(compare(&a, &a, &points, &ticks).unwrap(), Sample::default());
    let report = compare(&a, &b, &points, &ticks).unwrap();
    let mut height: f64 = 0.0;
    let mut slope: f64 = 0.0;
    let mut velocity: f64 = 0.0;
    for tick in ticks {
        for p in points {
            let theta = 0.2 * p[0] - c.frequency() * tick as f64 / 60.0;
            height = height
                .max((1.7 * (theta.cos() - (theta + std::f64::consts::PI / 2.0).cos())).abs());
            slope = slope
                .max((0.34 * (theta.sin() - (theta + std::f64::consts::PI / 2.0).sin())).abs());
            velocity = velocity.max(
                (1.7 * c.frequency() * (theta.sin() - (theta + std::f64::consts::PI / 2.0).sin()))
                    .abs(),
            );
        }
    }
    assert!((report.height - height).abs() < 1e-9);
    assert!((report.slope[0] - slope).abs() < 1e-9);
    assert!((report.velocity[1] - velocity).abs() < 1e-8);
    assert!(compare(&a, &a, &[], &ticks).is_err());
}

struct Invalid;
impl SeaProvider for Invalid {
    fn answer(&self, _: [f64; 2], _: u64) -> Result<Sample, SeaError> {
        Ok(Sample {
            height: f64::NAN,
            ..Sample::default()
        })
    }
}

#[test]
fn invalid_answers_cannot_silently_pass() {
    assert_eq!(
        compare(&Invalid, &Invalid, &[[0.0, 0.0]], &[0]),
        Err(SeaError::InvalidInput)
    );
}
