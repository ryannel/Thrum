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

    let mut world = WorldDriver::new(
        crate::scene::GameContent::Grid,
        crate::scene::RouteOverlay::Off,
    )
    .expect("fixture world");
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
    let mut driver = WorldDriver::new(
        crate::scene::GameContent::Grid,
        crate::scene::RouteOverlay::Off,
    )
    .expect("fixture world");
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

#[test]
fn inline_overload_reports_once_and_drains_over_display_drives() {
    let mut driver = WorldDriver::new(
        crate::scene::GameContent::Grid,
        crate::scene::RouteOverlay::Off,
    )
    .expect("fixture world");
    driver.world = driver.world.with_tick_work_slice(Duration::ZERO);
    let first = driver
        .advance(Duration::from_millis(100))
        .expect("catch up");
    assert_eq!(
        (first.tick, first.completed_ticks, first.queued_ticks),
        (1, 1, 5)
    );

    let now = Duration::from_millis(500);
    let overloaded = driver.advance(now).expect("overload");
    assert_eq!(overloaded.overload, Some(24));
    assert_eq!((overloaded.tick, overloaded.queued_ticks), (2, 4));
    driver.pause();
    driver.resume_after_drain(now).expect("request resume");
    assert_eq!(driver.tick(), 2);

    for tick in 3..=6 {
        let later = Duration::from_secs(tick);
        let drained = driver.advance(later).expect("drain");
        assert_eq!(drained.tick, tick);
        assert_eq!(drained.completed_ticks, 1);
        assert_eq!(drained.overload, None);
    }
    let resumed = driver
        .advance(Duration::from_millis(6017))
        .expect("resumed");
    assert_eq!((resumed.tick, resumed.queued_ticks), (7, 0));
}

#[test]
fn a_new_inline_pause_cancels_deferred_resume() {
    let mut driver = WorldDriver::new(
        crate::scene::GameContent::Grid,
        crate::scene::RouteOverlay::Off,
    )
    .expect("fixture world");
    driver.world = driver.world.with_tick_work_slice(Duration::ZERO);
    let now = Duration::from_millis(50);
    driver.advance(now).expect("first of three ticks");
    driver.pause();
    driver.resume_after_drain(now).expect("request resume");
    driver.pause();

    for tick in 2..=3 {
        let drained = driver.advance(Duration::from_secs(tick)).expect("drain");
        assert_eq!(drained.tick, tick);
    }
    let held = driver.advance(Duration::from_secs(4)).expect("held");
    assert_eq!((held.tick, held.queued_ticks), (3, 0));
}
