mod lifecycle;

use std::{
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
        mpsc::{self, Receiver},
    },
    thread,
    time::{Duration, Instant},
};

use crate::{ClockConfig, TickClock, WorldEpoch, sim::SnapshotId};

use super::{WorldLane, WorldPublication, WorldTask, WorldUpdate};

struct ScriptedWorld {
    steps: Receiver<Step>,
    tick: u64,
    clock: TickClock,
    applied_control: Arc<AtomicU32>,
}

enum Step {
    Publish(bool),
    Panic,
}

impl ScriptedWorld {
    fn update(&self, publication: bool, completed: u32) -> WorldUpdate<u64> {
        WorldUpdate {
            epoch: WorldEpoch(7),
            tick: self.tick,
            timeline: 3,
            clock: self.clock.presentation_sample(),
            completed_ticks: completed,
            queued_ticks: 0,
            tick_cpu: Duration::ZERO,
            skipped_publications: u64::from(!publication),
            projected_changes: 0,
            projection_lost_ticks: 0,
            overload: None,
            publication: publication.then_some(WorldPublication {
                id: SnapshotId {
                    epoch: WorldEpoch(7),
                    tick: self.tick,
                },
                timeline: 3,
                page: self.tick,
            }),
            next_due: Some(Duration::ZERO),
        }
    }
}

impl WorldTask for ScriptedWorld {
    type Control = u32;
    type Page = u64;
    type Error = &'static str;

    fn validate_control(control: Self::Control) -> Result<(), Self::Error> {
        (control <= 1).then_some(()).ok_or("invalid control")
    }

    fn same_control(left: Self::Control, right: Self::Control) -> bool {
        left == right
    }

    fn initial(&mut self) -> Result<WorldUpdate<Self::Page>, Self::Error> {
        Ok(self.update(true, 0))
    }

    fn apply_control(&mut self, control: Self::Control) -> Result<(), Self::Error> {
        self.applied_control.store(control, Ordering::Release);
        Ok(())
    }

    fn pause(&mut self) {
        self.clock.pause();
    }

    fn resume(&mut self, now: Duration) -> Result<(), Self::Error> {
        if self.clock.state() == crate::ClockState::Active {
            return Ok(());
        }
        self.clock.resume(now).map_err(|_| "resume refused")
    }

    fn advance(&mut self, _now: Duration) -> Result<WorldUpdate<Self::Page>, Self::Error> {
        let publication = match self.steps.recv().map_err(|_| "script ended")? {
            Step::Publish(publish) => publish,
            Step::Panic => panic!("scripted world failed outside its mailbox lock"),
        };
        self.tick += 1;
        Ok(self.update(publication, 1))
    }
}

fn poll_until(lane: &mut WorldLane<ScriptedWorld>, origin: Instant, tick: u64) -> u32 {
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        let report = lane.poll(origin.elapsed()).expect("healthy world");
        if report.tick == tick {
            return report.completed_ticks;
        }
        assert!(Instant::now() < deadline, "scripted world did not advance");
        thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn stale_page_never_borrows_newer_tick_and_progress_is_consumed_once() {
    let (steps, requests) = mpsc::channel();
    let origin = Instant::now();
    let applied_control = Arc::new(AtomicU32::new(0));
    let task = ScriptedWorld {
        steps: requests,
        tick: 0,
        clock: TickClock::new(ClockConfig::default(), Duration::ZERO).expect("clock"),
        applied_control: Arc::clone(&applied_control),
    };
    let (mut lane, controls) = WorldLane::start(task, 1, origin, || {}).expect("world lane");
    assert_eq!(
        applied_control.load(Ordering::Acquire),
        1,
        "initial control reached task"
    );
    assert_eq!(lane.current_page(), Some(0));
    assert!(
        controls.set(9).is_err(),
        "invalid control is refused before mailbox"
    );

    steps
        .send(Step::Publish(false))
        .expect("deferred first publication");
    assert_eq!(poll_until(&mut lane, origin, 1), 1);
    assert_eq!(
        lane.current_page(),
        None,
        "old page cannot impersonate tick one"
    );
    assert_eq!(
        lane.poll(origin.elapsed())
            .expect("second poll")
            .completed_ticks,
        0
    );

    steps.send(Step::Publish(true)).expect("second publication");
    assert_eq!(poll_until(&mut lane, origin, 2), 1);
    assert_eq!(lane.current_page(), Some(2));
    assert_eq!(
        lane.poll(origin.elapsed())
            .expect("fourth poll")
            .completed_ticks,
        0
    );
    drop(steps);
    drop(lane);
    assert!(
        controls.set(1).is_err(),
        "control clones cannot revive a stopped owner"
    );
}

#[test]
fn panicking_owner_wakes_health_only_observer_and_stops_controls() {
    let (steps, requests) = mpsc::channel();
    let (wakes, notified) = mpsc::sync_channel(1);
    let task = ScriptedWorld {
        steps: requests,
        tick: 0,
        clock: TickClock::new(ClockConfig::default(), Duration::ZERO).expect("clock"),
        applied_control: Arc::new(AtomicU32::new(0)),
    };
    let (lane, controls) = WorldLane::start(task, 0, Instant::now(), move || {
        let _ = wakes.try_send(());
    })
    .expect("world lane");
    controls
        .request_lifecycle(super::WorldLifecycle::Pause)
        .expect("pending pause before panic");
    steps.send(Step::Panic).expect("inject task panic");
    notified
        .recv_timeout(Duration::from_secs(1))
        .expect("terminal wake");
    assert!(
        controls.check_health().is_err(),
        "panic must be visible without render polling"
    );
    assert!(controls.set(1).is_err(), "dead owner must refuse controls");
    assert!(
        matches!(
            controls.poll_lifecycle_reply(),
            Err(super::WorldLaneFault::Stopped)
        ),
        "panic resolves a pending lifecycle poll to a retained fault"
    );
    drop(lane);
}

#[test]
fn panicking_wake_stops_owner_without_retrying_callback_during_unwind() {
    let (steps, requests) = mpsc::channel();
    let (wakes, notified) = mpsc::sync_channel(1);
    let wake_count = Arc::new(AtomicU32::new(0));
    let count = Arc::clone(&wake_count);
    let task = ScriptedWorld {
        steps: requests,
        tick: 0,
        clock: TickClock::new(ClockConfig::default(), Duration::ZERO).expect("clock"),
        applied_control: Arc::new(AtomicU32::new(0)),
    };
    let (lane, controls) = WorldLane::start(task, 0, Instant::now(), move || {
        count.fetch_add(1, Ordering::AcqRel);
        let _ = wakes.try_send(());
        panic!("a failed observer must not abort the process");
    })
    .expect("world lane");
    steps.send(Step::Publish(true)).expect("one update");
    notified
        .recv_timeout(Duration::from_secs(1))
        .expect("wake callback entered");
    let deadline = Instant::now() + Duration::from_secs(1);
    while controls.check_health().is_ok() {
        assert!(
            Instant::now() < deadline,
            "failed callback did not stop owner"
        );
        thread::yield_now();
    }
    assert_eq!(
        wake_count.load(Ordering::Acquire),
        1,
        "failed callback was not retried"
    );
    drop(lane);
}
