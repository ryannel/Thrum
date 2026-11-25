//! Virtual-time tests for driver-owned tick admission.
use std::{convert::Infallible, num::NonZeroU32, time::Duration};

use thrum_core::{
    ClockSample, WorldEpoch,
    sim::{DrivenAdvance, DrivenWorld, SnapshotId, WorldDriver, WorldPublication, WorldTask},
};

#[derive(Default)]
struct Script {
    tick: u64,
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
        let (requested, fraction, rate) = match now.as_secs() {
            11 => (1, 0.25, 5.0),
            12 => (7, 0.5, 2.0),
            _ => (0, 0.75, 0.0),
        };
        let completed_ticks = requested.min(cap.get());
        for _ in 0..completed_ticks {
            self.tick += 1;
        }
        Ok(DrivenAdvance {
            completed_ticks,
            presentation: ClockSample::driven(now, fraction, rate).expect("script phase"),
            next_due: Duration::from_millis(5),
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
    let mut world = DrivenWorld::new(Script::default(), NonZeroU32::new(3).expect("cap"));
    let initial = WorldTask::initial(&mut world).expect("initial publication");
    assert_eq!(initial.publication.expect("tick zero").id.tick, 0);
    for (time, completed, tick, alpha) in [
        (10, 0, 0, 0.75),
        (11, 1, 1, 0.5),
        (12, 3, 4, 0.6),
        (4_000, 0, 4, 0.75),
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
    }
}

#[test]
fn pausing_holds_the_pair_and_stops_polling_the_driver() {
    let mut world = DrivenWorld::new(Script::default(), NonZeroU32::MIN);
    world.advance(Duration::from_secs(11)).expect("first tick");
    world.pause();
    let paused = Duration::from_millis(11_050);
    let update = WorldTask::advance(&mut world, paused).expect("pause");
    assert_eq!(update.tick, 1);
    assert_eq!(update.next_due, None);
    assert!((update.clock.alpha_at(Duration::from_secs(90)) - 0.5).abs() < f32::EPSILON);
    assert_eq!(
        world.advance(Duration::from_secs(12)).expect("paused").tick,
        1
    );
    world.resume();
    assert_eq!(
        world
            .advance(Duration::from_secs(12))
            .expect("resumed")
            .tick,
        2
    );
}
