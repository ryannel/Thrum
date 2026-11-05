use std::{
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use super::WorldClient;
use crate::{host::world::WorldDriver, scene};

#[test]
fn world_advances_and_accepts_controls_while_render_is_idle() {
    let origin = Instant::now();
    let render_thread = thread::current().id();
    let (sender, worker_wake) = mpsc::sync_channel(1);
    let (mut client, controls) = WorldClient::start(
        WorldDriver::new().expect("fixture world"),
        origin,
        move || {
            let _ = sender.try_send(thread::current().id());
        },
    )
    .expect("resident world");
    let initial = client.published_page().expect("initial page");
    let slot = u32::try_from(scene::ENTITY_COUNT - 1).expect("fixture slot");
    let handle = initial.handle(slot).expect("controlled handle");
    let initial_x = initial.position(handle).expect("initial position").0[0];
    drop(initial);

    assert!(controls.set_controls([2.0, 0.0, 0.0]).is_err());
    controls.set_controls([1.0, 0.0, 0.0]).expect("valid input");
    // Deliberately leave the render client unpolled while the world has time
    // to commit several updates on its own deadline.
    thread::sleep(Duration::from_millis(85));
    let deadline = Instant::now() + Duration::from_secs(1);
    let advanced = loop {
        let advanced = client.poll(origin.elapsed()).expect("world status");
        if advanced.tick >= 3 {
            break advanced;
        }
        assert!(
            Instant::now() < deadline,
            "world failed to advance without render"
        );
        thread::sleep(Duration::from_millis(5));
    };
    assert!(advanced.completed_ticks >= 3);
    assert!(
        controls
            .check_health()
            .expect("world remains healthy without render callbacks")
            >= 3
    );
    assert_ne!(
        worker_wake.try_recv().expect("world wake thread"),
        render_thread,
        "the world must not run on the render owner"
    );
    let page = client.published_page().expect("latest complete page");
    assert_eq!(page.id().tick, advanced.tick);
    let current_x = page.position(handle).expect("controlled position").0[0];
    assert!(
        current_x > initial_x,
        "valid controls reached the world owner"
    );
    drop(page);

    let repeated = client.poll(origin.elapsed()).expect("next observed status");
    assert_eq!(
        u64::from(repeated.completed_ticks),
        repeated.tick - advanced.tick,
        "each world tick is reported once"
    );
    drop(client);
    assert!(controls.set_controls([0.0; 3]).is_err());
}

#[test]
fn conversion_preserves_held_controls_until_a_new_valid_input() {
    let mut driver = WorldDriver::new().expect("fixture world");
    driver
        .set_controls([1.0, 0.0, 0.0])
        .expect("held control before handoff");
    let (client, controls) =
        WorldClient::start(driver, Instant::now(), || {}).expect("resident world");
    controls
        .set_controls([0.0; 3])
        .expect("reset earlier held control");
    let state = controls.shared.state.lock().expect("control state");
    assert_eq!(state.control_sequence, 1);
    assert_eq!(state.controls, [0.0; 3]);
    drop(state);
    drop(client);
}

#[test]
fn first_overload_refusal_remains_visible_after_stopped_clock_polls() {
    let origin = Instant::now()
        .checked_sub(Duration::from_millis(200))
        .expect("monotonic clock has more than 200 ms of history");
    let (mut client, controls) =
        WorldClient::start(WorldDriver::new().expect("fixture world"), origin, || {})
            .expect("resident world");
    let deadline = Instant::now() + Duration::from_secs(1);
    let refused = loop {
        if let Some(refused) = client
            .poll(origin.elapsed())
            .expect("world status")
            .overload
        {
            break refused;
        }
        assert!(Instant::now() < deadline, "overload was not reported");
        thread::sleep(Duration::from_millis(5));
    };
    assert!(refused > 0);
    assert!(
        controls
            .check_health()
            .expect_err("world event exposes overload without a drawable")
            .contains("fixed-clock overload")
    );
    thread::sleep(Duration::from_millis(150));
    assert_eq!(
        client
            .poll(origin.elapsed())
            .expect("stopped clock")
            .overload,
        Some(refused)
    );
}
