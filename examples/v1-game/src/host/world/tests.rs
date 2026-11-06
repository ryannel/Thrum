use std::time::Duration;

use thrum_core::sim::WorldTask;

use super::WorldDriver;

#[test]
fn threaded_control_equality_preserves_inline_signed_zero_admission() {
    let positive_zero = [0.0_f32; 3];
    let negative_zero = [-0.0_f32, 0.0, 0.0];
    assert!(
        !<WorldDriver as WorldTask>::same_control(positive_zero, negative_zero),
        "resident mailbox must admit the same bitwise control change as inline world"
    );

    let mut world = WorldDriver::new().expect("fixture world");
    world
        .advance(Duration::from_millis(17))
        .expect("first tick");
    let before = world.input_sequence;
    world
        .set_controls(negative_zero)
        .expect("valid signed zero");
    world.advance(Duration::from_millis(34)).expect("next tick");
    assert_eq!(world.input_sequence, before + 1);
}
