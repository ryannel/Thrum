use std::{
    error::Error,
    time::{Duration, Instant},
};
use thrum_core::{
    ClockConfig, TickClock, WorldEpoch,
    sim::{
        SnapshotId, WorldLane, WorldLaneControls, WorldLaneFault, WorldLifecycle, WorldPublication,
        WorldTask, WorldUpdate,
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Failure {
    Validate,
    Apply,
    Initial,
    Advance,
    Resume,
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "injected world refusal: {self:?}")
    }
}
impl Error for Failure {}

struct Task {
    failure: Option<Failure>,
    publish: bool,
    clock: TickClock,
}

impl Task {
    fn new(failure: Option<Failure>) -> Self {
        Self {
            failure,
            publish: true,
            clock: TickClock::new(ClockConfig::default(), Duration::ZERO).expect("clock"),
        }
    }
    fn update(&self) -> WorldUpdate<u64> {
        WorldUpdate {
            epoch: WorldEpoch(1),
            tick: 0,
            timeline: 1,
            clock: self.clock.presentation_sample(),
            completed_ticks: 0,
            queued_ticks: 0,
            tick_cpu: Duration::ZERO,
            skipped_publications: 0,
            projected_changes: 0,
            projection_lost_ticks: 0,
            overload: None,
            publication: self.publish.then_some(WorldPublication {
                id: SnapshotId {
                    epoch: WorldEpoch(1),
                    tick: 0,
                },
                timeline: 1,
                page: 0,
            }),
            next_due: None,
        }
    }
}

impl WorldTask for Task {
    type Control = u8;
    type Page = u64;
    type Error = Failure;
    fn validate_control(control: u8) -> Result<(), Failure> {
        if control == 255 {
            Err(Failure::Validate)
        } else {
            Ok(())
        }
    }
    fn same_control(left: u8, right: u8) -> bool {
        left == right
    }
    fn initial(&mut self) -> Result<WorldUpdate<u64>, Failure> {
        if self.failure == Some(Failure::Initial) {
            Err(Failure::Initial)
        } else {
            Ok(self.update())
        }
    }
    fn apply_control(&mut self, control: u8) -> Result<(), Failure> {
        if control == 254 {
            Err(Failure::Apply)
        } else {
            Ok(())
        }
    }
    fn advance(&mut self, _: Duration) -> Result<WorldUpdate<u64>, Failure> {
        if self.failure == Some(Failure::Advance) {
            Err(Failure::Advance)
        } else {
            Ok(self.update())
        }
    }
    fn pause(&mut self) {}
    fn resume(&mut self, _: Duration) -> Result<(), Failure> {
        if self.failure == Some(Failure::Resume) {
            Err(Failure::Resume)
        } else {
            Ok(())
        }
    }
}

fn assert_source(fault: &WorldLaneFault, expected: Failure) {
    let source = fault
        .source()
        .expect("original task error")
        .downcast_ref::<Failure>()
        .expect("injected error type");
    assert_eq!(*source, expected);
    let cloned = fault.clone();
    let cloned_source = cloned
        .source()
        .expect("source survives clone")
        .downcast_ref::<Failure>()
        .expect("original error type survives clone");
    assert!(
        std::ptr::eq(source, cloned_source),
        "cloning shares the actual cause"
    );
}

fn wait_fault(controls: &WorldLaneControls<Task>, lifecycle: bool) -> WorldLaneFault {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let result = if lifecycle {
            controls.poll_lifecycle_reply().map(|_| ())
        } else {
            controls.check_health().map(|_| ())
        };
        if let Err(error) = result {
            return error;
        }
        assert!(
            Instant::now() < deadline,
            "worker did not return the injected failure"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn lane_start_preserves_validation_apply_and_initial_causes() {
    for (control, failure, expected) in [
        (255, None, Failure::Validate),
        (254, None, Failure::Apply),
        (0, Some(Failure::Initial), Failure::Initial),
    ] {
        let Err(fault) = WorldLane::start(Task::new(failure), control, Instant::now(), || {})
        else {
            panic!("startup must refuse");
        };
        if expected == Failure::Validate {
            assert!(matches!(fault, WorldLaneFault::InvalidControl(_)));
        } else {
            assert!(matches!(fault, WorldLaneFault::Task(_)));
        }
        assert_source(&fault, expected);
    }
    let mut task = Task::new(None);
    task.publish = false;
    let Err(fault) = WorldLane::start(task, 0, Instant::now(), || {}) else {
        panic!("initial publication required");
    };
    assert!(matches!(fault, WorldLaneFault::MissingPublication));
    assert!(
        fault.source().is_none(),
        "a plain missing endpoint has no artificial cause"
    );
}

#[test]
fn lane_mailbox_and_worker_keep_causes_after_the_owner_is_dropped() {
    for worker_failure in [Failure::Apply, Failure::Advance] {
        let stage = (worker_failure == Failure::Advance).then_some(Failure::Advance);
        let (lane, controls) =
            WorldLane::start(Task::new(stage), 0, Instant::now(), || {}).expect("start");
        let validation = controls.set(255).expect_err("invalid mailbox input");
        assert!(matches!(validation, WorldLaneFault::InvalidControl(_)));
        assert_source(&validation, Failure::Validate);
        if worker_failure == Failure::Apply {
            controls.set(254).expect("validated mailbox input");
        }
        let fault = wait_fault(&controls, false);
        assert!(matches!(fault, WorldLaneFault::Task(_)));
        drop(lane);
        drop(controls);
        assert_source(&fault, worker_failure);
    }
}

#[test]
fn lifecycle_refusal_keeps_its_cause_without_stopping_the_lane() {
    let (lane, controls) =
        WorldLane::start(Task::new(Some(Failure::Resume)), 0, Instant::now(), || {})
            .expect("start");
    controls
        .request_lifecycle(WorldLifecycle::Resume)
        .expect("request resume");
    let fault = wait_fault(&controls, true);
    assert!(matches!(fault, WorldLaneFault::Lifecycle(_)));
    assert!(
        controls.check_health().is_ok(),
        "lifecycle refusal leaves the owner available"
    );
    drop(lane);
    assert_source(&fault, Failure::Resume);
}
