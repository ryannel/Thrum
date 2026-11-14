//! Exact world/air-frame drift identity over a day.
pub mod support;
use support::{field, parameters};
use thrum_cloud_model::State;

#[test]
fn queries_are_bit_identical_in_the_translated_air_frame() {
    let field = field(7, 0.3);
    let mut p = parameters(0.6, 0.5);
    p.wind = [40.0, -19.0];
    let mut moved = State::new(p, 10.0, 0).expect("state");
    p.wind = [0.0; 2];
    let mut still = State::new(p, 10.0, 0).expect("state");
    for tick in 1..=86_400 {
        moved.step(tick, 1.0).expect("step");
        still.step(tick, 1.0).expect("step");
        if tick % 97 == 0 {
            for x in [-1347.2, 0.0, 65432.123] {
                let point = [x, 900.0, x * 0.33];
                let air = moved.to_air([point[0], point[2]]);
                assert_eq!(
                    field.query(point, &moved),
                    field.query([air[0], point[1], air[1]], &still)
                );
            }
        }
    }
}
