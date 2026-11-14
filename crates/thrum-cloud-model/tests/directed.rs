//! Directed authority: ramps, additive columns, rejection and travel round-trip.
pub mod support;
use support::{close, directed, field, parameters, shape};
use thrum_cloud_model::{Error, State};

fn life(dt: f64) -> f64 {
    let shape = shape();
    let field = field(7, 0.0);
    let mut state = State::new(parameters(0.0, 0.0), 10.0, 16).expect("state");
    state.add(directed(&shape, 1, [0.0; 2])).expect("shape");
    let mut largest = 0.0_f64;
    let mut previous = 0.0;
    let mut tick = 0;
    let mut elapsed = 0.0;
    while elapsed < 30.0 {
        if elapsed >= 15.0 {
            state.direct(1, 0.0, 0.0).ok();
        }
        tick += 1;
        state.step(tick, dt).expect("step");
        let value = field.query([-20.0, 0.0, 10.0], &state).optical_depth;
        largest = largest.max((value - previous).abs());
        previous = value;
        elapsed += dt;
    }
    assert_eq!(state.shapes(), []);
    largest
}

#[test]
fn directed_life_is_continuous() {
    close(life(0.125) / life(0.25), 0.5, 1e-12);
}

#[test]
fn directed_columns_add_and_refusals_preserve_shapes() {
    let shape = shape();
    let field = field(7, 0.0);
    let mut state = State::new(parameters(0.8, 0.5), 10.0, 2).expect("state");
    state.step(1, 8000.0).expect("time");
    let centre = [-20.0, 10.0];
    let before = field.query([centre[0], 0.0, centre[1]], &state);
    assert!(before.optical_depth > 0.0);
    state.add(directed(&shape, 1, [0.0; 2])).expect("add");
    let snapshots = state.shapes().to_vec();
    assert_eq!(
        state.add(directed(&shape, 2, [10.0, 10.0])),
        Err(Error::Overlap)
    );
    assert_eq!(state.shapes(), snapshots);
    state
        .add(directed(&shape, 2, [1000.0, 0.0]))
        .expect("second");
    let snapshots = state.shapes().to_vec();
    assert_eq!(
        state.add(directed(&shape, 3, [2000.0, 0.0])),
        Err(Error::Capacity)
    );
    assert_eq!(state.shapes(), snapshots);
    assert_eq!(state.refusals(), 2);
    state.step(2, 10.0).expect("form");
    let after = field.query([centre[0], 0.0, centre[1]], &state);
    let base = field.column(centre, state.time(), state.parameters());
    let expected = shape.footprint().column(centre)[0] * 0.05;
    close(after.optical_depth, base.optical_depth + expected, 1e-12);
    assert_eq!(state.shapes()[0].growth, 1.0);
    let mut p = parameters(0.0, 0.0);
    p.wind = [40.0, -40.0];
    let mut moved = State::new(p, 10.0, 0).expect("state");
    moved.step(1, 86_400.0).expect("day");
    let point = [123.456, -789.123];
    for (a, b) in moved.to_world(moved.to_air(point)).into_iter().zip(point) {
        close(a, b, 2.0 * f64::EPSILON * 40.0 * 86_400.0);
    }
}
