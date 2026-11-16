//! Rest-sheet derivatives and world-point queries describe the same displaced surface.
use thrum_sea_model::{Component, LongWaves};
#[test]
fn forward_sheet_has_the_same_height_and_material_rate_as_the_world_query() {
    let waves = LongWaves::new(
        vec![
            Component {
                wave_vector: [0.2, 0.1],
                amplitude: 0.6,
                phase: 0.7,
            },
            Component {
                wave_vector: [-0.05, 0.13],
                amplitude: 0.4,
                phase: 1.8,
            },
        ],
        2.0,
    )
    .expect("contractive");
    for time in [0.0, 10.125, 86400.125] {
        for rest in [[0.0, 0.0], [1234.5, -6789.0]] {
            let f = waves.forward(rest, time).expect("forward");
            let query = waves
                .evaluate(
                    [rest[0] + f.displacement[0], rest[1] + f.displacement[2]],
                    time,
                )
                .expect("query");
            assert!((query.height - f.displacement[1]).abs() < 1e-10);
            for (a, b) in query.velocity.into_iter().zip(f.velocity) {
                assert!((a - b).abs() < 1e-10);
            }
            let [x, c, z] = f.horizontal_derivative;
            let slope = [
                (f.slope[0] * (1.0 + z) - f.slope[1] * c) / f.jacobian,
                (f.slope[1] * (1.0 + x) - f.slope[0] * c) / f.jacobian,
            ];
            for (a, b) in query.slope.into_iter().zip(slope) {
                assert!((a - b).abs() < 1e-10);
            }
            for axis in 0..2 {
                let mut left = rest;
                let mut right = rest;
                left[axis] -= 0.0001;
                right[axis] += 0.0001;
                let l = waves.forward(left, time).expect("left");
                let r = waves.forward(right, time).expect("right");
                assert!(
                    ((r.displacement[1] - l.displacement[1]) / 0.0002 - f.slope[axis]).abs() < 1e-7
                );
            }
        }
    }
    assert!(waves.forward([f64::NAN, 0.0], 0.0).is_err());
    assert!(waves.forward([0.0, 0.0], f64::INFINITY).is_err());
}
