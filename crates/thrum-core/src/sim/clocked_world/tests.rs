use std::{num::NonZeroU32, time::Duration};

use crate::{ClockConfig, ClockError, WorldEpoch, sim::SnapshotId};

use super::{ClockedWorld, ClockedWorldError, WorldGame, WorldPublication, WorldStep};

#[derive(Default)]
enum Recovery {
    #[default]
    Idle,
    Ready(usize),
    Refuse,
}

#[derive(Default)]
struct Game {
    tick: u64,
    control: u32,
    control_applications: u32,
    attempts: u32,
    fail_next: bool,
    publication_busy: bool,
    publication_tick: u64,
    recovery: Recovery,
    recovery_calls: u32,
    lost_ticks: u64,
}

impl WorldGame for Game {
    type Control = u32;
    type Page = u64;
    type Error = &'static str;

    fn validate_control(control: Self::Control) -> Result<(), Self::Error> {
        (control <= 1).then_some(()).ok_or("invalid control")
    }

    fn same_control(left: Self::Control, right: Self::Control) -> bool {
        left == right
    }

    fn apply_control(&mut self, control: Self::Control) -> Result<(), Self::Error> {
        self.control = control;
        self.control_applications += 1;
        Ok(())
    }

    fn endpoint(&self) -> (SnapshotId, u64) {
        (
            SnapshotId {
                epoch: WorldEpoch(17),
                tick: self.tick,
            },
            3,
        )
    }

    fn step(&mut self) -> Result<WorldStep, Self::Error> {
        self.attempts += 1;
        if self.fail_next {
            self.fail_next = false;
            return Err("retry tick");
        }
        self.tick += 1;
        if !self.publication_busy {
            self.publication_tick = self.tick;
        }
        Ok(WorldStep {
            projected_changes: usize::from(!self.publication_busy),
            skipped_publication: self.publication_busy,
        })
    }

    fn recover_publication(&mut self) -> Result<Option<usize>, Self::Error> {
        self.recovery_calls += 1;
        match std::mem::take(&mut self.recovery) {
            Recovery::Idle => Ok(None),
            Recovery::Refuse => Err("publication recovery failed"),
            Recovery::Ready(changes) => {
                self.publication_busy = false;
                self.publication_tick = self.tick;
                Ok(Some(changes))
            }
        }
    }

    fn publication(&mut self) -> Result<Option<WorldPublication<Self::Page>>, Self::Error> {
        let (mut id, timeline) = self.endpoint();
        id.tick = self.publication_tick;
        Ok(Some(WorldPublication {
            id,
            timeline,
            page: self.publication_tick,
        }))
    }

    fn projection_lost_ticks(&self) -> u64 {
        self.lost_ticks
    }
}

fn config(cap: u32) -> ClockConfig {
    ClockConfig {
        tick_hz: NonZeroU32::new(10).expect("nonzero tick rate"),
        max_ticks_per_call: NonZeroU32::new(cap).expect("nonzero tick cap"),
        max_queued_ticks: NonZeroU32::new(16).expect("nonzero backlog cap"),
    }
}

fn world(game: Game, cap: u32) -> ClockedWorld<Game> {
    ClockedWorld::new(game, config(cap), Duration::ZERO).expect("clocked game")
}

#[test]
fn failed_tick_remains_queued_and_prior_commits_are_not_repeated() {
    let mut world = world(Game::default(), 2);
    let first = world
        .advance(Duration::from_millis(300))
        .expect("first cap");
    assert_eq!(
        (first.tick, first.completed_ticks, first.queued_ticks),
        (2, 2, 1)
    );
    world.game_mut().fail_next = true;
    let fault = world.advance(Duration::from_millis(300));
    assert!(matches!(fault, Err(ClockedWorldError::Game("retry tick"))));
    assert_eq!(world.game().tick, 2);
    assert_eq!(world.game().attempts, 3);
    let recovered = world.advance(Duration::from_millis(300)).expect("retry");
    assert_eq!(
        (
            recovered.tick,
            recovered.completed_ticks,
            recovered.queued_ticks
        ),
        (3, 1, 0)
    );
    assert_eq!(world.game().attempts, 4);
    assert_eq!(
        world
            .advance(Duration::from_millis(300))
            .expect("idle")
            .completed_ticks,
        0
    );
}

#[test]
fn configured_cap_controls_each_call_and_pause_preserves_accepted_backlog() {
    let mut world = world(Game::default(), 1);
    let first = world
        .advance(Duration::from_millis(350))
        .expect("admission");
    assert_eq!((first.completed_ticks, first.queued_ticks), (1, 2));
    world.pause();
    assert!(matches!(
        world.resume(Duration::from_secs(10)),
        Err(ClockedWorldError::Clock(ClockError::PendingTicks))
    ));
    let second = world.advance(Duration::from_secs(10)).expect("drain one");
    assert_eq!((second.completed_ticks, second.queued_ticks), (1, 1));
    let third = world.advance(Duration::from_secs(10)).expect("drain two");
    assert_eq!(
        (third.tick, third.completed_ticks, third.queued_ticks),
        (3, 1, 0)
    );
    assert_eq!(world.next_due_in(), None);
    world.resume(Duration::from_secs(10)).expect("resume");
    assert_eq!(
        world
            .advance(Duration::from_secs(10))
            .expect("new origin")
            .completed_ticks,
        0
    );
    assert_eq!(
        world
            .advance(Duration::from_millis(10_100))
            .expect("new tick")
            .tick,
        4
    );
}

#[test]
fn optional_publication_recovers_without_an_additional_tick_or_duplicate_output() {
    let mut world = world(
        Game {
            publication_busy: true,
            lost_ticks: 9,
            ..Game::default()
        },
        2,
    );
    let busy = world
        .advance(Duration::from_millis(200))
        .expect("busy pages");
    assert_eq!(
        (busy.tick, busy.skipped_publications, busy.projected_changes),
        (2, 2, 0)
    );
    assert!(!busy.recovered_publication);
    assert_eq!(busy.projection_lost_ticks, 9);
    world.game_mut().recovery = Recovery::Ready(7);
    let recovered = world
        .advance(Duration::from_millis(200))
        .expect("page recovery");
    assert_eq!(
        (
            recovered.tick,
            recovered.completed_ticks,
            recovered.projected_changes
        ),
        (2, 0, 7)
    );
    assert!(recovered.recovered_publication);
    let idle = world
        .advance(Duration::from_millis(200))
        .expect("recovery consumed");
    assert!(!idle.recovered_publication);
    assert_eq!(idle.projected_changes, 0);
    assert_eq!(world.game().attempts, 2);
}

#[test]
fn zero_change_recovery_is_still_a_new_publication() {
    let mut world = world(
        Game {
            recovery: Recovery::Ready(0),
            ..Game::default()
        },
        2,
    );
    let recovered = world.advance(Duration::ZERO).expect("empty recovery");
    assert!(recovered.recovered_publication);
    assert_eq!(recovered.projected_changes, 0);
    assert_eq!(recovered.completed_ticks, 0);
}

#[test]
fn recovery_failure_does_not_repeat_previously_committed_ticks() {
    let mut world = world(
        Game {
            recovery: Recovery::Refuse,
            ..Game::default()
        },
        2,
    );
    assert!(matches!(
        world.advance(Duration::from_millis(200)),
        Err(ClockedWorldError::Game("publication recovery failed"))
    ));
    assert_eq!((world.game().tick, world.game().attempts), (2, 2));
    let retried = world
        .advance(Duration::from_millis(200))
        .expect("retry recovery");
    assert_eq!(
        (retried.tick, retried.completed_ticks, retried.queued_ticks),
        (2, 0, 0)
    );
    assert_eq!(world.game().attempts, 2);
}

#[test]
fn invalid_controls_are_refused_before_game_control_replacement() {
    let mut world = world(Game::default(), 2);
    assert!(matches!(
        world.set_controls(9),
        Err(ClockedWorldError::Game("invalid control"))
    ));
    assert_eq!(world.game().control_applications, 0);
    world.set_controls(1).expect("valid replacement");
    assert_eq!(
        (world.game().control, world.game().control_applications),
        (1, 1)
    );
}

#[test]
fn display_samples_do_not_admit_world_time_and_clock_faults_preserve_state() {
    let mut world = world(Game::default(), 2);
    world
        .advance(Duration::from_millis(25))
        .expect("clock fraction");
    let alpha = world.presentation_alpha_at(Duration::from_millis(25));
    assert!((alpha - 0.5).abs() < f32::EPSILON);
    let sample = world.presentation_sample();
    assert!((sample.alpha_at(Duration::from_millis(50)) - alpha).abs() < f32::EPSILON);
    assert_eq!(world.game().tick, 0);
    assert!(matches!(
        world.advance(Duration::ZERO),
        Err(ClockedWorldError::Clock(ClockError::NonMonotonicTime))
    ));
    assert_eq!(world.game().attempts, 0);
    assert_eq!(world.next_due_in(), Some(Duration::from_millis(75)));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn native_updates_preserve_actual_publication_identity_and_recovery_signal() {
    use crate::sim::WorldTask;

    let mut world = world(
        Game {
            publication_busy: true,
            ..Game::default()
        },
        2,
    );
    let update = WorldTask::advance(&mut world, Duration::from_millis(100)).expect("tick");
    assert_eq!(update.tick, 1);
    let publication = update.publication.expect("retained old page");
    assert_eq!(publication.id.tick, 0);
    assert_eq!(publication.timeline, 3);
    assert_eq!(publication.page, 0);
    let idle = WorldTask::advance(&mut world, Duration::from_millis(100)).expect("idle");
    assert!(idle.publication.is_none());
    world.game_mut().recovery = Recovery::Ready(0);
    let recovered = WorldTask::advance(&mut world, Duration::from_millis(100)).expect("recover");
    assert_eq!(recovered.completed_ticks, 0);
    assert_eq!(
        recovered
            .publication
            .expect("zero-change recovery page")
            .id
            .tick,
        1
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn resident_lane_adopts_the_existing_clock_and_phase() {
    use std::time::Instant;

    use crate::sim::WorldLane;

    let mut world = world(Game::default(), 2);
    world
        .advance(Duration::from_millis(25))
        .expect("fraction before move");
    let expected = world
        .presentation_sample()
        .alpha_at(Duration::from_millis(25));
    let origin = Instant::now()
        .checked_sub(Duration::from_millis(25))
        .expect("existing monotonic origin");
    let (lane, controls) = WorldLane::start(world, 1, origin, || {}).expect("resident clock");
    // Before the first poll, the reader's sample time is zero. Sampling an
    // absolute 25 ms target checks its adopted phase without racing the owner.
    assert!((lane.presentation_alpha_at(Duration::from_millis(25)) - expected).abs() < 0.000_001);
    assert_eq!(lane.tick(), 0);
    assert_eq!(lane.current_page(), Some(0));
    assert!(controls.check_health().is_ok());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn paused_adoption_drains_only_the_already_accepted_backlog() {
    use crate::sim::WorldLane;
    use std::time::Instant;

    let mut world = world(Game::default(), 1);
    let admitted = world
        .advance(Duration::from_millis(350))
        .expect("accept three ticks");
    assert_eq!((admitted.tick, admitted.queued_ticks), (1, 2));
    world.pause();
    let origin = Instant::now()
        .checked_sub(Duration::from_millis(350))
        .expect("same time origin");
    let (mut lane, _controls) = WorldLane::start(world, 0, origin, || {}).expect("paused adoption");
    let deadline = Instant::now() + Duration::from_secs(1);
    let progress = loop {
        let progress = lane.poll(origin.elapsed()).expect("paused world progress");
        if progress.tick == 3 {
            break progress;
        }
        assert!(progress.tick < 3, "paused clock admitted new time");
        assert!(Instant::now() < deadline, "accepted backlog did not drain");
        std::thread::yield_now();
    };
    assert_eq!(progress.queued_ticks, 0);
    assert_eq!(lane.current_page(), Some(3));
    // Observing a later display time cannot restart a paused authority.
    let idle = lane
        .poll(origin.elapsed() + Duration::from_secs(2))
        .expect("future display");
    assert_eq!((idle.tick, idle.completed_ticks), (3, 0));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn first_overload_survives_native_adoption_and_clears_only_after_resume() {
    use std::time::Instant;

    use crate::sim::{WorldLane, WorldLaneFault, WorldTask};

    let mut world = world(Game::default(), 2);
    let overloaded = world
        .advance(Duration::from_secs(2))
        .expect("explicit overload");
    assert_eq!(overloaded.overload, Some(20));
    assert_eq!(
        world
            .advance(Duration::from_secs(2))
            .expect("stopped clock")
            .overload,
        Some(20)
    );
    assert_eq!(
        WorldTask::initial(&mut world)
            .expect("adoption snapshot")
            .overload,
        Some(20)
    );
    let origin = Instant::now()
        .checked_sub(Duration::from_secs(2))
        .expect("overloaded monotonic origin");
    let (mut lane, controls) = WorldLane::start(world, 0, origin, || {}).expect("overloaded lane");
    assert_eq!(
        lane.poll(Duration::from_secs(2))
            .expect("overload progress")
            .overload,
        Some(20)
    );
    assert!(matches!(
        controls.check_health(),
        Err(WorldLaneFault::Overload(20))
    ));
    drop(lane);

    let mut world =
        ClockedWorld::new(Game::default(), config(2), Duration::ZERO).expect("second world");
    world.advance(Duration::from_secs(2)).expect("overload");
    world.pause();
    assert!(matches!(
        world.resume(Duration::ZERO),
        Err(ClockedWorldError::Clock(ClockError::NonMonotonicTime))
    ));
    assert_eq!(
        WorldTask::initial(&mut world)
            .expect("refused resume")
            .overload,
        Some(20)
    );
    world.resume(Duration::from_secs(2)).expect("new episode");
    assert_eq!(
        WorldTask::initial(&mut world)
            .expect("resumed snapshot")
            .overload,
        None
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn task_lifecycle_is_idempotent_without_weakening_the_strict_clock_api() {
    use crate::sim::WorldTask;

    let mut world = world(Game::default(), 1);
    world
        .advance(Duration::from_millis(50))
        .expect("active fraction");
    WorldTask::resume(&mut world, Duration::from_secs(10)).expect("already running");
    assert!((world.presentation_alpha_at(Duration::ZERO) - 0.5).abs() < f32::EPSILON);
    assert!(matches!(
        world.resume(Duration::from_millis(50)),
        Err(ClockedWorldError::Clock(ClockError::AlreadyActive))
    ));
    WorldTask::pause(&mut world);
    WorldTask::pause(&mut world);
    assert_eq!(world.presentation_alpha_at(Duration::ZERO), 1.0);
    WorldTask::resume(&mut world, Duration::from_secs(10)).expect("new running origin");
    WorldTask::resume(&mut world, Duration::from_secs(20)).expect("already running retains origin");
    assert_eq!(
        world
            .advance(Duration::from_millis(10_100))
            .expect("one new tick")
            .tick,
        1
    );
}
