use super::*;
use crate::sim::{
    ClockedWorld, WorldGame, WorldLaneFault, WorldLifecycle, WorldLifecycleReply, WorldStep,
};
use std::{
    num::NonZeroU32,
    sync::{Mutex, atomic::AtomicBool},
};

#[derive(Default)]
struct Observed {
    attempts: u32,
    control: u32,
    committed_controls: Vec<u32>,
}

struct Game {
    tick: u64,
    page_tick: u64,
    observed: Arc<Mutex<Observed>>,
    publication_busy: Arc<AtomicBool>,
    entered: mpsc::Sender<u64>,
    release: Receiver<()>,
    gated_ticks: u64,
    fail_tick: Option<u64>,
}

impl WorldGame for Game {
    type Control = u32;
    type Page = u64;
    type Error = &'static str;

    fn validate_control(control: u32) -> Result<(), Self::Error> {
        (control <= 1).then_some(()).ok_or("invalid control")
    }

    fn same_control(left: u32, right: u32) -> bool {
        left == right
    }

    fn apply_control(&mut self, control: u32) -> Result<(), Self::Error> {
        self.observed.lock().expect("observer").control = control;
        Ok(())
    }

    fn endpoint(&self) -> (SnapshotId, u64) {
        (
            SnapshotId {
                epoch: WorldEpoch(19),
                tick: self.tick,
            },
            7,
        )
    }

    fn step(&mut self) -> Result<WorldStep, Self::Error> {
        self.observed.lock().expect("observer").attempts += 1;
        let next = self.tick + 1;
        if next <= self.gated_ticks {
            self.entered.send(next).map_err(|_| "observer closed")?;
            self.release.recv().map_err(|_| "release closed")?;
        }
        if self.fail_tick == Some(next) {
            return Err("failed tick");
        }
        let mut observed = self.observed.lock().expect("observer");
        let control = observed.control;
        observed.committed_controls.push(control);
        self.tick = next;
        if !self.publication_busy.load(Ordering::Acquire) {
            self.page_tick = self.tick;
        }
        Ok(WorldStep::default())
    }

    fn recover_publication(&mut self) -> Result<Option<usize>, Self::Error> {
        if !self.publication_busy.load(Ordering::Acquire) && self.page_tick != self.tick {
            self.page_tick = self.tick;
            Ok(Some(0))
        } else {
            Ok(None)
        }
    }

    fn publication(&mut self) -> Result<Option<WorldPublication<u64>>, Self::Error> {
        Ok(Some(WorldPublication {
            id: SnapshotId {
                epoch: WorldEpoch(19),
                tick: self.page_tick,
            },
            timeline: 7,
            page: self.page_tick,
        }))
    }
}

type Lane = WorldLane<ClockedWorld<Game>>;
type Controls = super::super::WorldLaneControls<ClockedWorld<Game>>;

struct Fixture {
    // Release a blocked test step before joining the lane during unwinding.
    release: mpsc::Sender<()>,
    lane: Lane,
    controls: Controls,
    entered: Receiver<u64>,
    wakes: Receiver<()>,
    observed: Arc<Mutex<Observed>>,
    publication_busy: Arc<AtomicBool>,
    origin: Instant,
}

fn fixture(fail_tick: Option<u64>, publication_busy: bool, paused_backlog: bool) -> Fixture {
    let observed = Arc::new(Mutex::new(Observed::default()));
    let busy = Arc::new(AtomicBool::new(publication_busy));
    let (entered, attempts) = mpsc::channel();
    let (release, proceed) = mpsc::channel();
    let (notify, wakes) = mpsc::channel();
    let game = Game {
        tick: 0,
        page_tick: 0,
        observed: Arc::clone(&observed),
        publication_busy: Arc::clone(&busy),
        entered,
        release: proceed,
        gated_ticks: 3,
        fail_tick,
    };
    let config = ClockConfig {
        tick_hz: NonZeroU32::new(10).expect("rate"),
        max_ticks_per_call: NonZeroU32::new(1).expect("cap"),
        max_queued_ticks: NonZeroU32::new(8).expect("queue"),
    };
    let mut world = ClockedWorld::new(game, config, Duration::ZERO).expect("clock");
    if paused_backlog {
        world.game_mut().gated_ticks = 0;
        let initial = world
            .advance(Duration::from_millis(350))
            .expect("accepted backlog");
        assert_eq!((initial.tick, initial.queued_ticks), (1, 2));
        world.game_mut().gated_ticks = 3;
        world.pause();
    }
    let origin = Instant::now()
        .checked_sub(Duration::from_millis(350))
        .expect("origin");
    let (lane, controls) = WorldLane::start(world, 0, origin, move || {
        let _ = notify.send(());
    })
    .expect("lane");
    Fixture {
        lane,
        controls,
        entered: attempts,
        release,
        wakes,
        observed,
        publication_busy: busy,
        origin,
    }
}

fn wait_until(mut ready: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while !ready() {
        assert!(
            Instant::now() < deadline,
            "world owner did not make progress"
        );
        thread::yield_now();
    }
}

fn reply(controls: &Controls) -> WorldLifecycleReply {
    let mut answer = None;
    wait_until(|| {
        answer = controls.poll_lifecycle_reply().expect("healthy lifecycle");
        answer.is_some()
    });
    answer.expect("ready reply")
}

fn drain_paused(fixture: &Fixture) -> u64 {
    assert_eq!(
        fixture
            .entered
            .recv_timeout(Duration::from_secs(2))
            .expect("first tick"),
        1
    );
    fixture
        .controls
        .request_lifecycle(WorldLifecycle::Pause)
        .expect("queue pause");
    assert!(matches!(
        fixture.controls.request_lifecycle(WorldLifecycle::Resume),
        Err(WorldLaneFault::Busy)
    ));
    fixture.release.send(()).expect("commit first");
    fixture
        .wakes
        .recv_timeout(Duration::from_secs(2))
        .expect("first admitted batch");
    // Read the accepted backlog while the next step is gated. Scheduling may
    // delay the first sample; the proof follows its actual admissions, not an
    // assumed thread-start latency or the final acknowledgement's tick count.
    let accepted = {
        let state = fixture.lane.shared.state.lock().expect("first update");
        assert_eq!(state.snapshot.tick, 1);
        assert!(
            state.snapshot.queued_ticks >= 2,
            "backlog exceeds the per-call cap"
        );
        state.snapshot.tick + u64::from(state.snapshot.queued_ticks)
    };
    for tick in 2..=3 {
        assert_eq!(
            fixture
                .entered
                .recv_timeout(Duration::from_secs(2))
                .expect("draining tick"),
            tick
        );
        assert!(matches!(
            fixture.controls.request_lifecycle(WorldLifecycle::Resume),
            Err(WorldLaneFault::Busy)
        ));
        fixture.release.send(()).expect("commit accepted tick");
    }
    for _ in 1..accepted {
        fixture
            .wakes
            .recv_timeout(Duration::from_secs(2))
            .expect("committed/acknowledged wake");
    }
    assert!(
        matches!(
            fixture.controls.request_lifecycle(WorldLifecycle::Resume),
            Err(WorldLaneFault::Busy)
        ),
        "unconsumed reply still owns slot"
    );
    accepted
}

#[test]
fn ordered_pause_drains_real_clock_backlog_and_resume_excludes_paused_time() {
    let mut fixture = fixture(None, false, false);
    let accepted = drain_paused(&fixture);
    let accepted_count = usize::try_from(accepted).expect("bounded queue");
    let paused = reply(&fixture.controls);
    assert_eq!(
        (paused.request, paused.id.tick, paused.timeline),
        (WorldLifecycle::Pause, accepted, 7)
    );
    assert_eq!(paused.id.epoch, WorldEpoch(19));
    assert_eq!(paused.clock.alpha_at(Duration::from_secs(100)), 1.0);
    fixture
        .lane
        .poll(fixture.origin.elapsed())
        .expect("paused snapshot");
    assert_eq!(
        fixture.lane.presentation_alpha_at(Duration::from_secs(10)),
        1.0
    );
    fixture.controls.set(1).expect("controls while paused");
    wait_until(|| fixture.observed.lock().expect("observer").control == 1);
    thread::sleep(Duration::from_millis(120));
    fixture
        .controls
        .request_lifecycle(WorldLifecycle::Pause)
        .expect("repeated pause");
    fixture
        .wakes
        .recv_timeout(Duration::from_secs(2))
        .expect("zero-tick repeated-pause acknowledgement wakes observer");
    assert_eq!(reply(&fixture.controls).id.tick, accepted);
    fixture
        .controls
        .request_lifecycle(WorldLifecycle::Resume)
        .expect("resume");
    // Leave the resume reply unconsumed while authority advances. Its summary
    // must come from acknowledgement, not whichever snapshot polling sees later.
    wait_until(|| {
        fixture
            .observed
            .lock()
            .expect("observer")
            .committed_controls
            .len()
            > accepted_count
    });
    let resumed = reply(&fixture.controls);
    assert_eq!(
        (resumed.request, resumed.id.tick),
        (WorldLifecycle::Resume, accepted),
        "no catch-up from paused wall time"
    );
    assert_eq!(resumed.clock.alpha_at(fixture.origin.elapsed()), 1.0);
    let observed = fixture.observed.lock().expect("observer");
    assert!(
        observed.committed_controls[..accepted_count]
            .iter()
            .all(|control| *control == 0)
    );
    assert_eq!(observed.committed_controls[accepted_count], 1);
    assert_eq!(
        observed.attempts as usize,
        observed.committed_controls.len()
    );
    drop(observed);
    assert_eq!(
        paused.id.tick, accepted,
        "late immutable reply does not follow the latest snapshot"
    );
    assert_eq!(resumed.id.tick, accepted);
}

#[test]
fn paused_recovery_publishes_and_wakes_without_another_tick() {
    let mut fixture = fixture(None, true, false);
    let accepted = drain_paused(&fixture);
    assert_eq!(reply(&fixture.controls).id.tick, accepted);
    fixture
        .lane
        .poll(fixture.origin.elapsed())
        .expect("busy publication");
    assert!(fixture.lane.current_page().is_none());
    fixture.publication_busy.store(false, Ordering::Release);
    fixture
        .wakes
        .recv_timeout(Duration::from_secs(2))
        .expect("zero-tick publication wake");
    let recovered = fixture
        .lane
        .poll(fixture.origin.elapsed())
        .expect("recovered publication");
    assert_eq!((recovered.tick, recovered.completed_ticks), (accepted, 0));
    assert!(recovered.recovered_publication);
    assert_eq!(fixture.lane.current_page(), Some(accepted));
    assert_eq!(
        u64::from(fixture.observed.lock().expect("observer").attempts),
        accepted
    );
    let controls = fixture.controls.clone();
    drop(fixture.lane);
    assert!(matches!(
        controls.poll_lifecycle_reply(),
        Err(WorldLaneFault::Stopped)
    ));
    assert!(matches!(
        controls.request_lifecycle(WorldLifecycle::Resume),
        Err(WorldLaneFault::Stopped)
    ));
}

#[test]
fn failed_drain_never_acknowledges_success_and_releases_pending_poll_to_fault() {
    let mut fixture = fixture(Some(2), false, false);
    fixture
        .entered
        .recv_timeout(Duration::from_secs(2))
        .expect("first tick");
    fixture
        .controls
        .request_lifecycle(WorldLifecycle::Pause)
        .expect("pause");
    fixture.release.send(()).expect("first commit");
    assert_eq!(
        fixture
            .entered
            .recv_timeout(Duration::from_secs(2))
            .expect("failed tick entered"),
        2
    );
    let accepted_queue = fixture
        .lane
        .shared
        .state
        .lock()
        .expect("first update")
        .snapshot
        .queued_ticks;
    assert!(accepted_queue >= 2);
    fixture.release.send(()).expect("fail second");
    wait_until(|| fixture.controls.check_health().is_err());
    assert!(matches!(
        fixture.controls.poll_lifecycle_reply(),
        Err(WorldLaneFault::Task(_))
    ));
    assert!(matches!(
        fixture.controls.request_lifecycle(WorldLifecycle::Resume),
        Err(WorldLaneFault::Task(_))
    ));
    assert!(fixture.lane.poll(fixture.origin.elapsed()).is_err());
    let observed = fixture.observed.lock().expect("observer");
    assert_eq!(
        (observed.attempts, observed.committed_controls.len()),
        (2, 1)
    );
    assert_eq!(
        fixture
            .lane
            .shared
            .state
            .lock()
            .expect("snapshot")
            .snapshot
            .queued_ticks,
        accepted_queue,
        "failed tick remains accepted, never acknowledged"
    );
}

#[test]
fn refused_resume_is_consumed_without_faulting_authority_or_leaving_busy() {
    let fixture = fixture(None, false, true);
    assert_eq!(
        fixture
            .entered
            .recv_timeout(Duration::from_secs(2))
            .expect("second tick"),
        2
    );
    fixture
        .controls
        .request_lifecycle(WorldLifecycle::Resume)
        .expect("resume before drain completes");
    fixture.release.send(()).expect("commit second");
    assert_eq!(
        fixture
            .entered
            .recv_timeout(Duration::from_secs(2))
            .expect("third tick"),
        3
    );
    fixture
        .release
        .send(())
        .expect("commit third after refused resume");
    wait_until(|| match fixture.controls.poll_lifecycle_reply() {
        Err(WorldLaneFault::Lifecycle(_)) => true,
        Ok(None) => false,
        other => panic!("expected isolated pending-ticks refusal, got {other:?}"),
    });
    assert!(
        fixture.controls.check_health().is_ok(),
        "transition refusal is not an authority failure"
    );
    fixture
        .controls
        .request_lifecycle(WorldLifecycle::Resume)
        .expect("consumed error frees request slot");
    let resumed = reply(&fixture.controls);
    assert_eq!(
        (resumed.request, resumed.id.tick),
        (WorldLifecycle::Resume, 3)
    );
    assert_eq!(fixture.observed.lock().expect("observer").attempts, 3);
}

#[test]
fn shutdown_with_a_pending_request_is_observable_without_waiting_for_task() {
    let (steps, requests) = mpsc::channel();
    let task = ScriptedWorld {
        steps: requests,
        tick: 0,
        clock: TickClock::new(ClockConfig::default(), Duration::ZERO).expect("clock"),
        applied_control: Arc::new(AtomicU32::new(0)),
    };
    let (lane, controls) = WorldLane::start(task, 0, Instant::now(), || {}).expect("lane");
    controls
        .request_lifecycle(WorldLifecycle::Pause)
        .expect("pending pause");
    let shutdown = thread::spawn(move || drop(lane));
    wait_until(|| {
        matches!(
            controls.poll_lifecycle_reply(),
            Err(WorldLaneFault::Stopped)
        )
    });
    assert!(matches!(
        controls.request_lifecycle(WorldLifecycle::Resume),
        Err(WorldLaneFault::Stopped)
    ));
    drop(steps);
    shutdown.join().expect("shutdown wakes and joins owner");
}
