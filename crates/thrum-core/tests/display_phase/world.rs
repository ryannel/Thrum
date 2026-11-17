//! Clocked world and resident lane retain the same phase as their clock sample.
use std::time::{Duration, Instant};
use thrum_core::{
    ClockConfig, WorldEpoch,
    sim::{ClockedWorld, SnapshotId, WorldGame, WorldLane, WorldPublication, WorldStep},
};

struct Game(u64);
impl WorldGame for Game {
    type Control = ();
    type Page = u64;
    type Error = std::convert::Infallible;
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
                epoch: WorldEpoch(1),
                tick: self.0,
            },
            1,
        )
    }
    fn step(&mut self) -> Result<WorldStep, Self::Error> {
        self.0 += 1;
        Ok(WorldStep::default())
    }
    fn recover_publication(&mut self) -> Result<Option<usize>, Self::Error> {
        Ok(None)
    }
    fn publication(&mut self) -> Result<Option<WorldPublication<u64>>, Self::Error> {
        let (id, timeline) = self.endpoint();
        Ok(Some(WorldPublication {
            id,
            timeline,
            page: self.0,
        }))
    }
}

#[test]
fn world_and_lane_sample_their_clock_without_advancing_it() {
    let mut world =
        ClockedWorld::new(Game(0), ClockConfig::default(), Duration::ZERO).expect("world");
    world.advance(Duration::from_millis(25)).expect("tick one");
    assert_eq!(world.game().0, 1);
    assert!((world.display_phase_at(Duration::from_millis(15)) - 1.4).abs() < 1e-6);
    assert_eq!(world.presentation_alpha_at(Duration::from_millis(15)), 1.0);
    assert_eq!(world.game().0, 1);
    let origin = Instant::now();
    let world = ClockedWorld::new(Game(0), ClockConfig::default(), Duration::ZERO).expect("world");
    let (mut lane, _) = WorldLane::start(world, (), origin, || {}).expect("resident lane");
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut observed = 0;
    while observed < 6 {
        let observation = origin.elapsed();
        lane.poll(observation).expect("snapshot");
        if lane.is_interpolating() {
            let phase = lane.display_phase_at(Duration::from_millis(30));
            if phase < 4.0 {
                // Expected directly from observation + 30 ms, never alpha.
                let display = (lane.tick() as f64 - 1.0 + f64::from(phase)) / 60.0;
                assert!((display - (observation.as_secs_f64() + 0.030 - 1.0 / 60.0)).abs() < 1e-6);
                assert_eq!(
                    lane.presentation_alpha_at(Duration::from_millis(30)),
                    phase.min(1.0)
                );
                assert_eq!(lane.current_page(), Some(lane.tick()));
                observed += 1;
            }
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn delayed_observation_retains_a_causal_pair_and_later_poll_reports_all_progress() {
    let origin = Instant::now();
    let world = ClockedWorld::new(Game(0), ClockConfig::default(), Duration::ZERO).expect("world");
    let (mut lane, controls) = WorldLane::start(world, (), origin, || {}).expect("world lane");
    let observation = Duration::ZERO;
    let deadline = Instant::now() + Duration::from_secs(3);
    while controls.check_health().expect("authority remains active") < 2 {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
    let delayed = lane.poll(observation).expect("old observation");
    assert_eq!(delayed.tick, 0);
    assert_eq!(delayed.completed_ticks, 0);
    assert_eq!(lane.current_page(), Some(0));
    assert!(lane.is_interpolating());
    assert_eq!(lane.display_phase_at(Duration::ZERO), 0.0);
    // A future lead is presentation only; it still cannot select a future
    // authority endpoint for the observation that admitted this frame.
    assert_eq!(lane.display_phase_at(Duration::from_millis(50)), 3.0);
    assert_eq!(lane.presentation_alpha_at(Duration::from_millis(50)), 1.0);
    assert!(controls.check_health().expect("owner was never stopped") >= 2);

    let mut completed = 0_u64;
    let (fresh_observation, latest) = loop {
        let observation = origin.elapsed();
        let report = lane.poll(observation).expect("fresh observation");
        completed += u64::from(report.completed_ticks);
        if report.tick >= 2 {
            break (observation, report);
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    };
    assert_eq!(completed, latest.tick);
    assert_eq!(lane.current_page(), Some(latest.tick));
    let repeated = lane.poll(fresh_observation).expect("retained pair");
    // The owner may commit more already-due work between these observations;
    // count it once, even when both polls sample the same target.
    assert_eq!(
        u64::from(repeated.completed_ticks),
        repeated.tick - latest.tick
    );
}

#[test]
fn every_delayed_poll_advances_to_at_least_the_previously_seen_pair() {
    let origin = Instant::now();
    let world = ClockedWorld::new(Game(0), ClockConfig::default(), Duration::ZERO).expect("world");
    let (mut lane, controls) = WorldLane::start(world, (), origin, || {}).expect("world lane");
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut previous_newest = 0;
    let mut previous_shown = 0;
    let mut completed = 0_u64;
    for _ in 0..12 {
        // This is read AFTER the preceding poll. Every poll below is delayed:
        // wait for authority to commit an endpoint due after this observation.
        let observation = origin.elapsed();
        let first_future_tick = u64::try_from(observation.as_nanos() * 60 / 1_000_000_000 + 1)
            .expect("short running-clock test");
        while controls.check_health().expect("running owner") < first_future_tick {
            assert!(Instant::now() < deadline, "owner must keep committing");
            std::thread::sleep(Duration::from_millis(1));
        }
        let newest = controls.check_health().expect("published progress");
        let report = lane.poll(observation).expect("delayed poll");
        assert!(report.tick < newest, "newest endpoint is not yet due");
        assert!(
            report.tick >= previous_newest,
            "must retain the refused pair"
        );
        assert!(report.tick >= previous_shown, "never go backwards");
        completed += u64::from(report.completed_ticks);
        assert_eq!(completed, report.tick, "count each completed tick once");
        assert_eq!(lane.current_page(), Some(report.tick));
        assert!(lane.is_interpolating());
        for lead in [
            Duration::ZERO,
            Duration::from_millis(8),
            Duration::from_millis(50),
        ] {
            let phase = lane.display_phase_at(lead);
            assert_eq!(lane.presentation_alpha_at(lead), phase.min(1.0));
            if phase < 4.0 {
                let display = (report.tick as f64 - 1.0 + f64::from(phase)) / 60.0;
                let expected = (observation + lead).as_secs_f64() - 1.0 / 60.0;
                assert!((display - expected).abs() < 1e-6);
            }
        }
        previous_newest = newest;
        previous_shown = report.tick;
    }
    assert!(previous_shown >= 11);
}
