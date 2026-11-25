//! Virtual-time tests for driver-owned tick admission.
use std::{
    convert::Infallible,
    num::NonZeroU32,
    sync::mpsc,
    time::{Duration, Instant},
};

use thrum_core::{
    ClockSample, WorldEpoch,
    sim::{
        DrivenAdvance, DrivenWorld, SnapshotId, WorldDriver, WorldLane, WorldLifecycle,
        WorldPublication, WorldTask,
    },
};

const TICK: Duration = Duration::from_millis(40);

#[derive(Default)]
struct Script {
    tick: u64,
    resident_anchor: Option<mpsc::Sender<Duration>>,
}

impl WorldDriver for Script {
    type Control = ();
    type Page = u64;
    type Error = Infallible;

    fn validate_control((): ()) -> Result<(), Self::Error> {
        Ok(())
    }

    fn same_control((): (), (): ()) -> bool {
        true
    }

    fn apply_control(&mut self, (): ()) -> Result<(), Self::Error> {
        Ok(())
    }

    fn endpoint(&self) -> (SnapshotId, u64) {
        (
            SnapshotId {
                epoch: WorldEpoch(4),
                tick: self.tick,
            },
            0,
        )
    }

    fn advance(&mut self, now: Duration, cap: NonZeroU32) -> Result<DrivenAdvance, Self::Error> {
        let (requested, fraction, rate) = if let Some(anchor) = &self.resident_anchor {
            anchor.send(now).expect("record scripted sample");
            (1, 0.25, 25.0)
        } else {
            match now.as_secs() {
                11 => (1, 0.25, 5.0),
                12 => (7, 0.5, 2.0),
                _ => (0, 0.75, 0.0),
            }
        };
        let completed_ticks = requested.min(cap.get());
        for _ in 0..completed_ticks {
            self.tick += 1;
        }
        Ok(DrivenAdvance {
            completed_ticks,
            presentation: ClockSample::driven(now, fraction, rate, TICK).expect("script phase"),
            next_due: Duration::from_secs(30),
        })
    }

    fn publication(&mut self) -> Result<Option<WorldPublication<u64>>, Self::Error> {
        let (id, timeline) = self.endpoint();
        Ok(Some(WorldPublication {
            id,
            timeline,
            page: self.tick,
        }))
    }
}

#[test]
fn only_the_driver_admits_ticks_and_supplies_the_phase() {
    let mut world = DrivenWorld::new(Script::default(), NonZeroU32::new(3).expect("cap"), TICK);
    let initial = WorldTask::initial(&mut world).expect("initial publication");
    assert_eq!(initial.publication.expect("tick zero").id.tick, 0);
    assert_eq!(initial.clock.tick_duration(), TICK);
    assert!(!initial.clock.is_interpolating());
    for (time, completed, tick, alpha, due_ms) in [
        (10, 0, 0, 0.75, None),
        (11, 1, 1, 0.5, Some(10_950)),
        (12, 3, 4, 0.6, Some(11_750)),
        (4_000, 0, 4, 0.75, None),
    ] {
        let now = Duration::from_secs(time);
        let update = WorldTask::advance(&mut world, now).expect("scripted advance");
        assert_eq!(update.tick, tick);
        assert_eq!(update.completed_ticks, completed);
        assert_eq!(update.queued_ticks, 0);
        assert_eq!(update.overload, None);
        assert_eq!(
            update.publication.map(|page| page.id.tick),
            (completed != 0).then_some(tick)
        );
        let at = now + Duration::from_millis(50);
        assert!((update.clock.alpha_at(at) - alpha).abs() < f32::EPSILON);
        assert_eq!(update.clock.tick_duration(), TICK);
        assert_eq!(update.clock.is_interpolating(), due_ms.is_some());
        assert_eq!(
            update.clock.tick_due_time(),
            due_ms.map(Duration::from_millis)
        );
    }
}

#[test]
fn pausing_holds_the_pair_and_stops_polling_the_driver() {
    let mut world = DrivenWorld::new(Script::default(), NonZeroU32::MIN, TICK);
    world.advance(Duration::from_secs(11)).expect("first tick");
    world.pause();
    let paused = Duration::from_millis(11_050);
    let update = WorldTask::advance(&mut world, paused).expect("pause");
    assert_eq!(update.tick, 1);
    assert_eq!(update.next_due, None);
    assert!(!update.clock.is_interpolating());
    assert_eq!(update.clock.tick_due_time(), None);
    assert_eq!(update.clock.tick_duration(), TICK);
    assert!((update.clock.alpha_at(Duration::from_secs(90)) - 1.0).abs() < f32::EPSILON);
    assert!((update.clock.display_phase_at(paused) - 1.0).abs() < f32::EPSILON);
    assert_eq!(
        world.advance(Duration::from_secs(12)).expect("paused").tick,
        1
    );
    world.resume();
    let resumed = WorldTask::advance(&mut world, Duration::from_secs(12)).expect("resumed");
    assert_eq!(resumed.tick, 2);
    assert!(resumed.clock.is_interpolating());
    assert_eq!(
        resumed.clock.tick_due_time(),
        Some(Duration::from_millis(11_750))
    );
}

#[test]
fn resident_observer_uses_the_tick_due_time_to_select_its_pair() {
    let (anchors, samples) = mpsc::channel();
    let (wakes, changed) = mpsc::channel();
    let world = DrivenWorld::new(
        Script {
            tick: 0,
            resident_anchor: Some(anchors),
        },
        NonZeroU32::MIN,
        TICK,
    );
    let origin = Instant::now().checked_sub(TICK).expect("test clock origin");
    let (mut lane, controls) = WorldLane::start(world, (), origin, move || {
        let _ = wakes.send(());
    })
    .expect("resident world");
    let timeout = Duration::from_secs(2);
    let anchor = samples.recv_timeout(timeout).expect("driver observation");
    changed.recv_timeout(timeout).expect("worker published");
    let due = anchor
        .checked_sub(Duration::from_millis(10))
        .expect("due time");
    let before_due = due
        .checked_sub(Duration::from_nanos(1))
        .expect("before due time");

    assert_eq!(lane.poll(before_due).expect("refused pair").tick, 0);
    assert_eq!(
        lane.poll(due + Duration::from_millis(5))
            .expect("eligible pair")
            .tick,
        1
    );
    assert_eq!(lane.current_page(), Some(1));
    assert_eq!(lane.tick_duration(), TICK);
    assert_eq!(lane.tick_due_time(), Some(due));
    assert!(lane.is_interpolating());
    let lead = Duration::from_millis(40);
    assert!((lane.display_phase_at(lead) - 1.125).abs() < f32::EPSILON);
    assert!((lane.presentation_alpha_at(lead) - 1.0).abs() < f32::EPSILON);

    controls
        .request_lifecycle(WorldLifecycle::Pause)
        .expect("pause on worker");
    changed.recv_timeout(timeout).expect("worker paused");
    lane.poll(due + TICK).expect("held pair");
    assert_eq!(lane.tick_due_time(), None);
    assert!(!lane.is_interpolating());
    assert!((lane.display_phase_at(lead) - 1.0).abs() < f32::EPSILON);
}
