use std::{
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use super::{WorldClient, WorldControls};
use crate::{host::world::WorldDriver, scene};
use thrum_core::sim::{WorldLaneFault, WorldLifecycle, WorldLifecycleReply};

fn transition(controls: &WorldControls, request: WorldLifecycle) -> WorldLifecycleReply {
    controls
        .request_lifecycle(request)
        .expect("admit transition");
    assert!(matches!(
        controls.request_lifecycle(request),
        Err(WorldLaneFault::Busy)
    ));
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if let Some(reply) = controls.poll_lifecycle_reply().expect("world transition") {
            assert_eq!(reply.request, request);
            return reply;
        }
        assert!(
            Instant::now() < deadline,
            "world transition did not complete"
        );
        thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn world_advances_and_accepts_controls_while_render_is_idle() {
    let driver = WorldDriver::new(scene::GameContent::Grid, scene::RouteOverlay::Off)
        .expect("fixture world");
    let render_thread = thread::current().id();
    let (sender, worker_wake) = mpsc::sync_channel(1);
    // The window host starts its clock once the world and assets are prepared.
    let origin = Instant::now();
    let (mut client, controls) = WorldClient::start(driver, origin, move || {
        let _ = sender.try_send(thread::current().id());
    })
    .expect("resident world");
    let initial = client.published_page().expect("initial page");
    let initial_tick = initial.id().tick;
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
    let mut observed_tick = initial_tick;
    let advanced = loop {
        let advanced = client.poll(origin.elapsed()).expect("world status");
        assert_eq!(
            u64::from(advanced.completed_ticks),
            advanced.tick - observed_tick,
            "each world tick is reported once"
        );
        observed_tick = advanced.tick;
        if observed_tick - initial_tick >= 3 {
            break advanced;
        }
        assert!(
            Instant::now() < deadline,
            "world failed to advance without render: {:?}",
            controls.check_health()
        );
        thread::sleep(Duration::from_millis(5));
    };
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
    let mut driver = WorldDriver::new(scene::GameContent::Grid, scene::RouteOverlay::Off)
        .expect("fixture world");
    driver
        .set_controls([1.0, 0.0, 0.0])
        .expect("held control before handoff");
    let origin = Instant::now();
    let (mut client, controls) = WorldClient::start(driver, origin, || {}).expect("resident world");
    let initial = client.published_page().expect("initial page");
    let slot = u32::try_from(scene::ENTITY_COUNT - 1).expect("fixture slot");
    let handle = initial.handle(slot).expect("controlled handle");
    let initial_x = initial.position(handle).expect("initial position").0[0];
    drop(initial);
    // Observe actual progress; scheduling under concurrent GPU preparation
    // does not promise the worker has run after a fixed sleep.
    let deadline = Instant::now() + Duration::from_secs(3);
    while client
        .poll(origin.elapsed())
        .expect("held control progress")
        .tick
        == 0
    {
        assert!(Instant::now() < deadline, "held controls did not advance");
        thread::sleep(Duration::from_millis(1));
    }
    let moved = client.published_page().expect("moved page");
    let moved_x = moved.position(handle).expect("moved position").0[0];
    assert!(moved_x > initial_x, "handoff kept the held control");
    drop(moved);
    controls.set_controls([0.0; 3]).expect("reset held control");
    let target = client.tick() + 3;
    let deadline = Instant::now() + Duration::from_secs(1);
    while client
        .poll(origin.elapsed())
        .expect("reset control progress")
        .tick
        < target
    {
        assert!(Instant::now() < deadline, "reset controls did not advance");
        thread::sleep(Duration::from_millis(5));
    }
    let stopped = client.published_page().expect("stopped page");
    let stopped_x = stopped.position(handle).expect("stopped position").0[0];
    drop(stopped);
    let target = client.tick() + 1;
    while client
        .poll(origin.elapsed())
        .expect("stable control progress")
        .tick
        < target
    {
        assert!(Instant::now() < deadline, "stable controls did not advance");
        thread::sleep(Duration::from_millis(5));
    }
    let still = client.published_page().expect("stable page");
    assert_eq!(
        still.position(handle).expect("stable position").0[0],
        stopped_x
    );
    drop(client);
}

#[test]
fn first_overload_refusal_remains_visible_after_stopped_clock_polls() {
    let origin = Instant::now()
        .checked_sub(Duration::from_millis(200))
        .expect("monotonic clock has more than 200 ms of history");
    let (mut client, controls) = WorldClient::start(
        WorldDriver::new(scene::GameContent::Grid, scene::RouteOverlay::Off)
            .expect("fixture world"),
        origin,
        || {},
    )
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
    assert!(matches!(
        controls
            .check_health()
            .expect_err("world event exposes overload without a drawable"),
        WorldLaneFault::Overload(count) if count == refused
    ));
    thread::sleep(Duration::from_millis(150));
    assert_eq!(
        client
            .poll(origin.elapsed())
            .expect("stopped clock")
            .overload,
        Some(refused)
    );
}

#[test]
fn native_pause_retains_authority_and_controls_without_render_callbacks() {
    let mut driver =
        WorldDriver::new(scene::GameContent::Grid, scene::RouteOverlay::Off).expect("10k fixture");
    driver.set_controls([1.0, 0.0, 0.0]).unwrap();
    let origin = Instant::now();
    let (mut client, controls) = WorldClient::start(driver, origin, || {}).unwrap();
    let initial = client.published_page().unwrap();
    let handle = initial
        .handle(u32::try_from(scene::ENTITY_COUNT - 1).unwrap())
        .unwrap();
    let initial_x = initial.position(handle).unwrap().0[0];
    let initial_id = initial.id();
    drop(initial);
    let deadline = Instant::now() + Duration::from_secs(3);
    while client.poll(origin.elapsed()).unwrap().tick < 2 {
        assert!(Instant::now() < deadline, "initial world progress");
        thread::sleep(Duration::from_millis(1));
    }
    let paused = transition(&controls, WorldLifecycle::Pause);
    assert_eq!(paused.id.epoch, initial_id.epoch);
    assert_eq!(paused.clock.alpha_at(Duration::MAX), 1.0);
    let progress = client.poll(origin.elapsed()).unwrap();
    assert_eq!(progress.tick, paused.id.tick);
    assert_eq!(progress.queued_ticks, 0);
    assert_eq!(client.timeline(), paused.timeline);
    let page = client.published_page().unwrap();
    assert_eq!(page.id(), paused.id);
    let paused_x = page.position(handle).unwrap().0[0];
    assert!(paused_x > initial_x);
    drop(page);

    controls.set_controls([-1.0, 0.0, 0.0]).unwrap();
    assert!(controls.set_controls([2.0, 0.0, 0.0]).is_err());
    // No render/poll callback supplies time, and controls cannot resume time.
    thread::sleep(Duration::from_millis(70));
    let idle = client.poll(origin.elapsed()).unwrap();
    assert_eq!((idle.tick, idle.completed_ticks), (paused.id.tick, 0));
    assert_eq!(client.presentation_alpha_at(Duration::from_secs(1)), 1.0);
    let page = client.published_page().unwrap();
    assert_eq!(page.position(handle).unwrap().0[0], paused_x);
    drop(page);
    let repeated = transition(&controls, WorldLifecycle::Pause);
    assert_eq!(
        (repeated.id, repeated.timeline),
        (paused.id, paused.timeline)
    );

    let resumed = transition(&controls, WorldLifecycle::Resume);
    assert_eq!((resumed.id, resumed.timeline), (paused.id, paused.timeline));
    assert_eq!(resumed.clock.alpha_at(Duration::MAX), 1.0);
    let target = resumed.id.tick + 2;
    let deadline = Instant::now() + Duration::from_secs(3);
    while client.poll(origin.elapsed()).unwrap().tick < target {
        assert!(Instant::now() < deadline, "resumed world progress");
        thread::sleep(Duration::from_millis(1));
    }
    let final_pause = transition(&controls, WorldLifecycle::Pause);
    client.poll(origin.elapsed()).unwrap();
    let page = client.published_page().unwrap();
    assert_eq!(page.id(), final_pause.id);
    assert!(page.position(handle).unwrap().0[0] < paused_x);
    assert_eq!(final_pause.id.epoch, initial_id.epoch);
    assert_eq!(final_pause.timeline, paused.timeline);
    drop(page);
    drop(client);
    assert!(matches!(
        controls.poll_lifecycle_reply(),
        Err(WorldLaneFault::Stopped)
    ));
}

#[test]
fn inline_lifecycle_facade_explicitly_refuses_a_threaded_world() {
    use crate::host::world::WorldPort;

    let origin = Instant::now();
    let (client, controls) = WorldClient::start(
        WorldDriver::new(scene::GameContent::Grid, scene::RouteOverlay::Off).unwrap(),
        origin,
        || {},
    )
    .unwrap();
    let mut port = WorldPort::Threaded(Box::new(client));
    assert!(port.pause().unwrap_err().contains("inline world lifecycle"));
    assert!(
        port.resume(origin.elapsed())
            .unwrap_err()
            .contains("inline world lifecycle")
    );
    assert!(port.inline_lifecycle().is_err());
    // Refusing an inline API does not kill or implicitly pause its world owner.
    let reply = transition(&controls, WorldLifecycle::Pause);
    assert_eq!(port.advance(origin.elapsed()).unwrap().tick, reply.id.tick);
}
