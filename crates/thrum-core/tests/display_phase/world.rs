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
    type Error = &'static str;
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
