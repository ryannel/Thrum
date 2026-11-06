use std::time::Duration;

use thrum_core::sim::WorldTask;

use super::{GameWorld, WorldDriver};
use thrum_core::sim::ClockedWorld;

#[test]
fn threaded_control_equality_preserves_inline_signed_zero_admission() {
    let positive_zero = [0.0_f32; 3];
    let negative_zero = [-0.0_f32, 0.0, 0.0];
    assert!(
        !<ClockedWorld<GameWorld> as WorldTask>::same_control(positive_zero, negative_zero),
        "resident mailbox must admit the same bitwise control change as inline world"
    );

    let mut world = WorldDriver::new().expect("fixture world");
    world
        .advance(Duration::from_millis(17))
        .expect("first tick");
    let before = world.world.game().input_sequence;
    world
        .set_controls(negative_zero)
        .expect("valid signed zero");
    world.advance(Duration::from_millis(34)).expect("next tick");
    assert_eq!(world.world.game().input_sequence, before + 1);
}

#[test]
fn exhausted_sequence_refuses_before_admitting_input_and_keeps_the_tick_queued() {
    let mut driver = WorldDriver::new().expect("fixture world");
    let initial = driver.controlled_position().expect("controlled actor");
    driver.world.game_mut().input_sequence = u64::MAX;
    driver.set_controls([1.0, 0.0, 0.0]).expect("valid axes");
    let error = driver
        .advance(Duration::from_millis(17))
        .expect_err("exhausted sequence");
    assert!(error.contains("input sequence exhausted"));
    assert_eq!(driver.tick(), 0);
    assert_eq!(driver.world.game().admitted_axes, [0.0; 3]);

    // A retry must not find a partially admitted max-sequence frame.
    driver.world.game_mut().input_sequence = 1;
    driver.set_controls([0.0; 3]).expect("replace held axes");
    let retried = driver
        .advance(Duration::from_millis(17))
        .expect("same due tick");
    assert_eq!(
        (retried.tick, retried.completed_ticks, retried.queued_ticks),
        (1, 1, 0)
    );
    assert_eq!(driver.controlled_position(), Some(initial));
}
