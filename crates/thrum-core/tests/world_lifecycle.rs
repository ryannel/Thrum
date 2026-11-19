//! Host lifecycle policy exercised with an inline clock and a resident world lane.

use std::time::{Duration, Instant};

use thrum_core::{
    ClockConfig, WorldEpoch,
    sim::{
        ClockedWorld, SnapshotId, WorldGame, WorldLane, WorldLaneFault,
        WorldLifecycle::{Pause, Resume},
        WorldLifecyclePolicy, WorldLifecycleStatus, WorldPublication, WorldStep,
    },
};

#[derive(Default)]
struct Counter(u64);

impl WorldGame for Counter {
    type Control = ();
    type Page = u64;
    type Error = std::io::Error;

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
        Ok(WorldStep {
            projected_changes: 0,
            skipped_publication: false,
        })
    }

    fn recover_publication(&mut self) -> Result<Option<usize>, Self::Error> {
        Ok(None)
    }

    fn publication(&mut self) -> Result<Option<WorldPublication<Self::Page>>, Self::Error> {
        let (id, timeline) = self.endpoint();
        Ok(Some(WorldPublication {
            id,
            timeline,
            page: self.0,
        }))
    }

    fn projection_lost_ticks(&self) -> u64 {
        0
    }
}

fn world() -> ClockedWorld<Counter> {
    ClockedWorld::new(Counter::default(), ClockConfig::default(), Duration::ZERO)
        .expect("clocked world")
        .with_tick_work_slice(Duration::ZERO)
}

#[test]
fn inline_overload_drains_accepted_ticks_and_holds_until_explicit_resume() {
    let mut world = world();
    let mut policy = WorldLifecyclePolicy::default();
    let first = world
        .advance_inline(Duration::from_millis(100))
        .expect("admit six ticks");
    assert_eq!((first.tick, first.queued_ticks), (1, 5));

    let now = Duration::from_millis(500);
    let overloaded = world.advance_inline(now).expect("refuse later ticks");
    let refused = overloaded.overload.expect("overload reported");
    policy.overloaded(refused);
    policy.overloaded(refused);
    policy
        .drive_inline(overloaded.queued_ticks, |request| {
            assert_eq!(request, Pause);
            world.pause();
            Ok::<_, std::io::Error>(())
        })
        .expect("start drain");
    assert!(policy.needs_reply());

    // A user pause during recovery prevents the automatic restart.
    policy.request(true);
    for tick in 3..=6 {
        let report = world
            .advance_inline(Duration::from_secs(tick))
            .expect("drain");
        assert_eq!(report.tick, tick);
        assert_eq!(report.overload, None);
        policy
            .drive_inline(report.queued_ticks, |_| -> Result<(), std::io::Error> {
                panic!("no request should be issued while draining or held")
            })
            .expect("observe drain");
    }
    assert_eq!(policy.status(), WorldLifecycleStatus::OverloadPaused);
    assert_eq!(
        (policy.overload_count(), policy.refused_ticks()),
        (1, refused)
    );
    assert_eq!(
        world
            .advance_inline(Duration::from_secs(9))
            .expect("held")
            .tick,
        6
    );

    policy.request(false);
    let now = Duration::from_secs(10);
    policy
        .drive_inline(0, |request| {
            assert_eq!(request, Resume);
            world.resume(now)
        })
        .expect("resume");
    assert_eq!(
        world
            .advance_inline(Duration::from_millis(10_017))
            .expect("one fresh tick")
            .tick,
        7
    );
}

#[test]
fn threaded_overload_round_trip_through_the_lane() {
    let (send, receive) = std::sync::mpsc::channel();
    let origin = Instant::now() - Duration::from_secs(1);
    let (_lane, controls) = WorldLane::start(world(), (), origin, move || {
        let _ = send.send(());
    })
    .expect("start resident world");
    let until = Instant::now() + Duration::from_secs(5);
    let wait = || {
        receive
            .recv_timeout(until.saturating_duration_since(Instant::now()))
            .expect("resident world wake");
    };
    let mut policy = WorldLifecyclePolicy::default();
    let refused = loop {
        match controls.check_health() {
            Err(WorldLaneFault::Overload(refused)) => break refused,
            Ok(_) => wait(),
            Err(error) => panic!("world failed: {error}"),
        }
    };
    policy.overloaded(refused);
    policy.request(true);
    for request in [Pause, Resume] {
        assert_eq!(policy.next_request(), Some(request));
        controls
            .request_lifecycle(request)
            .expect("submit lifecycle request");
        policy.accepted(request);
        let reply = loop {
            if let Some(reply) = controls.poll_lifecycle_reply().expect("lifecycle slot") {
                break reply;
            }
            wait();
        };
        assert_eq!(reply.request, request);
        assert_eq!(reply.id.epoch, WorldEpoch(1));
        policy.acknowledge(reply.request).expect("matching reply");
        if request == Pause {
            assert!(policy.confirmed_paused());
            assert_eq!(policy.next_request(), None);
            policy.request(false);
        }
    }
    assert_eq!(policy.status(), WorldLifecycleStatus::Running);
    assert_eq!(
        (policy.overload_count(), policy.refused_ticks()),
        (1, refused)
    );
}

#[test]
fn pending_resume_defers_racing_health_to_the_next_fresh_observation() {
    let mut policy = WorldLifecyclePolicy::default();
    policy.overloaded(12);
    // A busy slot leaves the same pause due and a mismatched reply leaves it pending.
    assert_eq!(policy.next_request(), Some(Pause));
    assert_eq!(policy.next_request(), Some(Pause));
    policy.accepted(Pause);
    assert!(policy.acknowledge(Resume).is_err());
    policy.acknowledge(Pause).expect("drained");
    policy.overloaded(12);
    policy.accepted(Resume);
    policy.overloaded(11);
    assert_eq!((policy.overload_count(), policy.refused_ticks()), (1, 12));
    assert_eq!(policy.next_request(), None);
    policy.acknowledge(Resume).expect("restart completed");
    policy.overloaded(11);
    policy.overloaded(11);
    assert_eq!(policy.next_request(), Some(Pause));
    assert_eq!((policy.overload_count(), policy.refused_ticks()), (2, 23));
}
