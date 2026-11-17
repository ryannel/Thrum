//! Actual worker lifecycle, independently bracketed on its host clock.
use std::time::{Duration, Instant};
use thrum_core::{
    ClockConfig, WorldEpoch,
    sim::{
        ClockedWorld, SnapshotId, WorldGame, WorldLane, WorldLaneControls, WorldLifecycle,
        WorldLifecycleReply, WorldPublication, WorldStep,
    },
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

type World = ClockedWorld<Game>;

#[test]
fn clocked_world_rewind_changes_tick_but_keeps_due_time() {
    let start = Duration::from_millis(7300);
    let mut world = ClockedWorld::new(Game(100), ClockConfig::default(), start).expect("world");
    world
        .advance(start + Duration::from_millis(50))
        .expect("three ticks");
    assert_eq!(world.game().0, 103);
    let due = start + Duration::from_millis(50);
    assert_eq!(world.tick_due_time(), Some(due));
    world.game_mut().0 = 93;
    assert_eq!(world.tick_due_time(), Some(due));
    assert_eq!(world.presentation_sample().tick_due_time(), Some(due));
    world
        .advance(start + Duration::from_millis(75))
        .expect("one tick");
    assert_eq!(world.game().0, 94);
    assert_eq!(
        world.tick_due_time(),
        Some(start + Duration::from_nanos(66_666_667))
    );
}
const D: f64 = 1.0 / 60.0;
const LEADS_MS: [u64; 4] = [0, 8, 30, 50];

fn lifecycle(
    controls: &WorldLaneControls<World>,
    request: WorldLifecycle,
    origin: Instant,
) -> (Duration, WorldLifecycleReply, Duration) {
    let asked = origin.elapsed();
    controls.request_lifecycle(request).expect("request");
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(reply) = controls.poll_lifecycle_reply().expect("reply") {
            return (asked, reply, origin.elapsed());
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_micros(200));
    }
}

/// Polls `frames` times at about 120 a second, each with a `gap` between reading the
/// observation and polling. Returns the origin of every interpolating frame under the
/// bound, in seconds: the independently reported due time of its tick, less the
/// tick's world time.
fn frames(
    lane: &mut WorldLane<World>,
    origin: Instant,
    count: usize,
    gap: Duration,
    stopped: &mut usize,
) -> Vec<(u64, f64)> {
    let mut origins = Vec::new();
    for i in 0..count {
        let observation = origin.elapsed();
        std::thread::sleep(gap);
        let report = lane.poll(observation).expect("poll");
        let lead = Duration::from_millis(LEADS_MS[i % 4]);
        let phase = lane.display_phase_at(lead);
        assert_eq!(
            lane.presentation_alpha_at(lead).to_bits(),
            phase.min(1.0).to_bits()
        );
        if !lane.is_interpolating() {
            assert_eq!(phase, 1.0);
            assert_eq!(lane.tick_due_time(), None);
            *stopped += 1;
        } else if phase < 4.0 {
            let due = lane
                .tick_due_time()
                .expect("interpolating due time")
                .as_secs_f64();
            assert!(
                due <= observation.as_secs_f64() + 2e-6,
                "tick due by its observation"
            );
            origins.push((report.tick, due - report.tick as f64 * D));
        }
        std::thread::sleep(Duration::from_micros(8_333).saturating_sub(gap));
    }
    origins
}

fn spread(origins: &[(u64, f64)]) -> (f64, f64) {
    let low = origins.iter().map(|o| o.1).fold(f64::INFINITY, f64::min);
    let high = origins
        .iter()
        .map(|o| o.1)
        .fold(f64::NEG_INFINITY, f64::max);
    (low, high)
}

#[test]
fn a_resumed_lane_keeps_one_origin_inside_the_resume_bracket() {
    for (gap_ms, pause_ms) in [(0_u64, 300_u64), (3, 40), (20, 700)] {
        let gap = Duration::from_millis(gap_ms);
        let origin = Instant::now();
        let world =
            ClockedWorld::new(Game(0), ClockConfig::default(), Duration::ZERO).expect("world");
        let (mut lane, controls) = WorldLane::start(world, (), origin, || {}).expect("lane");
        let mut stopped = 0;
        let first = frames(&mut lane, origin, 40, gap, &mut stopped);
        let (low, high) = spread(&first);
        // Started at host time zero on tick 0: the first span's origin is zero.
        assert!(low.abs() < 2e-6 && high.abs() < 2e-6, "{low} {high}");
        assert_eq!(stopped, 0);

        let (_, paused, _) = lifecycle(&controls, WorldLifecycle::Pause, origin);
        std::thread::sleep(Duration::from_millis(pause_ms));
        let before = frames(&mut lane, origin, 6, gap, &mut stopped);
        assert!(before.is_empty(), "a paused world consumes no target");
        assert_eq!(stopped, 6);
        assert_eq!(lane.tick(), paused.id.tick);

        let (asked, resumed, answered) = lifecycle(&controls, WorldLifecycle::Resume, origin);
        assert_eq!(
            resumed.id.tick, paused.id.tick,
            "nothing commits while paused"
        );
        let second = frames(&mut lane, origin, 60, gap, &mut stopped);
        assert!(stopped > 6, "the hold shows the committed tick");
        let (tick, _) = second[0];
        assert!(
            tick > resumed.id.tick,
            "the first target is consumed a tick after the resume"
        );
        let (low, high) = spread(&second);
        assert!(high - low < 2e-6, "one origin for the span: {low} {high}");
        // The span's anchor: tick `resumed.id.tick` is due where the worker resumed.
        let anchor = low + resumed.id.tick as f64 * D;
        assert!(
            anchor >= asked.as_secs_f64() - 2e-6 && anchor <= answered.as_secs_f64() + 2e-6,
            "{anchor} outside {asked:?}..{answered:?}"
        );
        println!(
            "gap {gap_ms:>2} ms, paused {pause_ms:>3} ms at tick {}: {} frames at origin 0; then {} frames at origin {:.6} s, \
             spread {:.0} ns; resume asked {:.6} s, anchor {:.6} s, answered {:.6} s; first tick shown {}",
            paused.id.tick,
            first.len(),
            second.len(),
            low,
            (high - low) * 1e9,
            asked.as_secs_f64(),
            anchor,
            answered.as_secs_f64(),
            tick
        );
    }
}
