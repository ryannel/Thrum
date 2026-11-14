//! Synthetic inputs shared by independent criterion tests.
use std::sync::Arc;
use thrum_cloud_model::{Directed, Field, Parameters, Profile, Shape, State, Type};

/// Stationary controls with the requested coverage and type coordinate.
pub fn parameters(coverage: f64, kind: f64) -> Parameters {
    Parameters {
        coverage,
        kind,
        wind: [0.0; 2],
        time_scale: 1.0,
    }
}

/// Tick-zero state with a ten-second response.
/// # Panics
/// Panics if the caller supplies invalid controls.
pub fn state(coverage: f64, kind: f64) -> State {
    State::new(parameters(coverage, kind), 10.0, 16).expect("valid test state")
}

/// Synthetic two-row field, independent of any game.
/// # Panics
/// Panics if erosion is invalid.
pub fn field(seed: u32, erosion: f64) -> Field {
    // Synthetic rows for numerical tests, with no game names or looks.
    Field::new(
        seed,
        Profile {
            pitch: 1300.0,
            period: 1600.0,
            detail_mean: 0.5,
            erosion,
        },
        vec![
            Type {
                base: 700.0,
                depth: [220.0, 950.0],
                liquid_water: 0.0003,
                droplet_radius: 9e-6,
                deck: 0.0,
                profile: [0.2, 0.3],
            },
            Type {
                base: 700.0,
                depth: [220.0, 950.0],
                liquid_water: 0.0003,
                droplet_radius: 9e-6,
                deck: 1.0,
                profile: [0.2, 0.3],
            },
        ],
    )
    .expect("valid test field")
}

/// An asymmetric hollow shape with an overhang.
/// # Panics
/// Panics if the fixture grid is malformed.
pub fn shape() -> Shape {
    let mut values = vec![0.0; 5 * 6 * 4];
    for z in 0..4 {
        for y in 0..6 {
            for x in 0..5 {
                // Asymmetric overhang, pierced by a vertical hollow.
                if (y < 2 && x < 3 || y >= 2 && x > 0) && !(x == 2 && z == 2) {
                    values[(z * 6 + y) * 5 + x] = if x == 4 { 0.4 } else { 1.0 };
                }
            }
        }
    }
    Shape::new([5, 6, 4], 20.0, values).expect("test shape")
}

/// New shape admitted at zero envelope, targeting full growth and amplitude.
pub fn directed(shape: &Shape, id: u64, centre: [f64; 2]) -> Directed {
    Directed {
        id,
        centre,
        yaw: 0.0,
        scale: 1.0,
        footprint: Arc::new(shape.footprint().clone()),
        envelope: 0.0,
        target: 1.0,
        growth: 0.0,
        growth_target: 1.0,
    }
}

/// A fully formed directed shape in otherwise clear weather.
/// # Panics
/// Panics if the fixture cannot be admitted or stepped.
pub fn formed(shape: &Shape) -> State {
    let mut state = state(0.0, 0.0);
    state.add(directed(shape, 1, [0.0; 2])).expect("admit");
    state.step(1, 10.0).expect("form");
    state
}

/// Assert absolute error in the quantity's stated units.
/// # Panics
/// Panics if the quantities differ by more than tolerance.
pub fn close(a: f64, b: f64, tolerance: f64) {
    assert!(
        (a - b).abs() <= tolerance,
        "{a:.12} != {b:.12}, tolerance {tolerance}"
    );
}
