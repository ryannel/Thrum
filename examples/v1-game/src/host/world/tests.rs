use std::time::Duration;

use thrum_core::sim::{WorldLifecycle, WorldLifecyclePolicy, WorldTask};

use super::{GameRules, WorldDriver};
use thrum_core::sim::ClockedWorld;

#[test]
#[cfg(feature = "proving")]
fn reconstruction_fixture_replaces_the_prop_on_authoritative_tick_sixty() {
    use thrum_core::{StableId, sim::WorldGame};
    let mut driver = WorldDriver::new(
        crate::scene::GameContent::Grid,
        crate::scene::RouteOverlay::Reconstruction,
    )
    .unwrap();
    let old_id = StableId(crate::scene::MOTION_SUBJECT_SLOT as u128);
    for _ in 0..59 {
        driver.world.game_mut().step().unwrap();
    }
    let sim = &driver.world.game().sim;
    assert!(sim.handle(old_id).is_some());
    assert!(sim.handle(StableId(20_000)).is_none());
    driver.world.game_mut().step().unwrap();
    let sim = &driver.world.game().sim;
    assert_eq!(sim.tick(), 60);
    assert!(sim.handle(old_id).is_none());
    let spawned = sim.get(sim.handle(StableId(20_000)).unwrap()).unwrap();
    assert!(spawned.position.0[1] > 240.0);
    assert_eq!(spawned.velocity.0, [3.0, 0.0, 0.0]);
}

#[test]
fn a_recovered_page_is_republished_at_the_latest_tick() {
    use thrum_core::sim::WorldGame;
    let mut driver = WorldDriver::new(
        crate::scene::GameContent::Grid,
        crate::scene::RouteOverlay::Off,
    )
    .expect("fixture world");
    // Lease every page the ring offers, so a step finds no free page and skips publication.
    let mut held = vec![driver.published_page().expect("tick zero")];
    let skipped = loop {
        let step = driver.world.game_mut().step().expect("step");
        if step.skipped_publication {
            break driver.world.game().sim.tick();
        }
        held.push(driver.published_page().expect("held page"));
        assert!(held.len() < 64, "the page ring never filled");
    };
    assert!(!driver.has_current_page());
    let rules = driver.world.game_mut();
    assert_eq!(rules.recover_publication().expect("busy recovery"), None);
    drop(held);
    assert_eq!(rules.recover_publication().expect("recovery"), Some(0));
    assert!(driver.has_current_page());
    let page = driver.published_page().expect("republished page");
    assert_eq!(page.id().tick, skipped);
}

#[test]
fn threaded_control_equality_preserves_inline_signed_zero_admission() {
    let positive_zero = [0.0_f32; 3];
    let negative_zero = [-0.0_f32, 0.0, 0.0];
    assert!(
        !<ClockedWorld<GameRules> as WorldTask>::same_control(positive_zero, negative_zero),
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
    let mut lifecycle = WorldLifecyclePolicy::default();
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
    lifecycle.overloaded(overloaded.overload.expect("refused ticks"));
    reconcile(&mut driver, &mut lifecycle, overloaded.queued_ticks, now);
    assert_eq!(driver.tick(), 2);

    for tick in 3..=6 {
        let later = Duration::from_secs(tick);
        let drained = driver.advance(later).expect("drain");
        assert_eq!(drained.tick, tick);
        assert_eq!(drained.completed_ticks, 1);
        assert_eq!(drained.overload, None);
        reconcile(&mut driver, &mut lifecycle, drained.queued_ticks, later);
    }
    let resumed = driver
        .advance(Duration::from_millis(6017))
        .expect("resumed");
    assert_eq!((resumed.tick, resumed.queued_ticks), (7, 0));
}

#[test]
fn a_new_inline_pause_cancels_deferred_resume() {
    let mut lifecycle = WorldLifecyclePolicy::default();
    let mut driver = WorldDriver::new(
        crate::scene::GameContent::Grid,
        crate::scene::RouteOverlay::Off,
    )
    .expect("fixture world");
    driver.world = driver.world.with_tick_work_slice(Duration::ZERO);
    let now = Duration::from_millis(50);
    let first = driver.advance(now).expect("first of three ticks");
    for paused in [true, false, true] {
        lifecycle.request(paused);
        reconcile(&mut driver, &mut lifecycle, first.queued_ticks, now);
    }

    for tick in 2..=3 {
        let now = Duration::from_secs(tick);
        let drained = driver.advance(now).expect("drain");
        assert_eq!(drained.tick, tick);
        reconcile(&mut driver, &mut lifecycle, drained.queued_ticks, now);
    }
    let held = driver.advance(Duration::from_secs(4)).expect("held");
    assert_eq!((held.tick, held.queued_ticks), (3, 0));
}

fn reconcile(
    driver: &mut WorldDriver,
    policy: &mut WorldLifecyclePolicy,
    queued: u32,
    now: Duration,
) {
    policy
        .drive_inline(queued, |request| match request {
            WorldLifecycle::Pause => {
                driver.pause();
                Ok(())
            }
            WorldLifecycle::Resume => driver.resume(now),
        })
        .expect("apply lifecycle request");
}
