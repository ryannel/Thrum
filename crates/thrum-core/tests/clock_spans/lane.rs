//! Actual worker lifecycle with a manually stepped world clock.
use std::{
    sync::mpsc::{self, Receiver, Sender},
    time::{Duration, Instant},
};
use thrum_core::{
    ClockConfig, WorldEpoch,
    sim::{
        ClockedWorld, ClockedWorldError, SnapshotId, WorldGame, WorldLane, WorldLaneControls,
        WorldLifecycle, WorldLifecycleReply, WorldPublication, WorldStep, WorldTask, WorldUpdate,
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

const TICK_SECONDS: f64 = 1.0 / 60.0;
const FIRST_TICK: Duration = Duration::from_nanos(16_666_667);
const SECOND_TICK: Duration = Duration::from_nanos(33_333_334);
const TOLERANCE_SECONDS: f64 = 2e-6;
const LEADS_MS: [u64; 4] = [0, 8, 30, 50];
const WORKER_TIMEOUT: Duration = Duration::from_secs(2);

/// The lane still owns lifecycle, publication and pair selection. The test
/// supplies the clocked world's time through the lane's control mailbox.
struct SteppedWorld {
    world: World,
    now: Duration,
    samples: Sender<WorldSample>,
}

enum WorldSample {
    Advance {
        at: Duration,
        completed: u32,
        published: bool,
        interpolating: bool,
    },
    Resume(Duration),
}

impl WorldTask for SteppedWorld {
    type Control = Duration;
    type Page = u64;
    type Error = ClockedWorldError<std::convert::Infallible>;

    fn validate_control(_: Duration) -> Result<(), Self::Error> {
        Ok(())
    }

    fn same_control(left: Duration, right: Duration) -> bool {
        left == right
    }

    fn initial(&mut self) -> Result<WorldUpdate<Self::Page>, Self::Error> {
        self.world.initial()
    }

    fn apply_control(&mut self, now: Duration) -> Result<(), Self::Error> {
        self.now = now;
        Ok(())
    }

    fn pause(&mut self) {
        WorldTask::pause(&mut self.world);
    }

    fn resume(&mut self, worker_sample: Duration) -> Result<(), Self::Error> {
        // Keep the real lane's host-time sample independently observable, even
        // though the clocked world receives the scripted sample.
        let _ = self.samples.send(WorldSample::Resume(worker_sample));
        WorldTask::resume(&mut self.world, self.now)
    }

    fn advance(&mut self, _: Duration) -> Result<WorldUpdate<Self::Page>, Self::Error> {
        let mut update = WorldTask::advance(&mut self.world, self.now)?;
        let _ = self.samples.send(WorldSample::Advance {
            at: self.now,
            completed: update.completed_ticks,
            published: update.publication.is_some(),
            interpolating: update.clock.is_interpolating(),
        });
        // The fixture advances through mailbox wakes. A wall deadline cannot
        // supply a new sample of this clock.
        update.next_due = Some(Duration::from_secs(60));
        Ok(update)
    }
}

struct LaneRun {
    // Dropped before the lane so a worker stopping during unwinding can exit.
    wakes: Receiver<Sender<()>>,
    samples: Receiver<WorldSample>,
    controls: WorldLaneControls<SteppedWorld>,
    lane: WorldLane<SteppedWorld>,
    host_origin: Instant,
}

impl LaneRun {
    fn new() -> Self {
        let (notify, wakes) = mpsc::channel();
        let (samples, observed_samples) = mpsc::channel();
        let host_origin = Instant::now();
        let task = SteppedWorld {
            world: World::new(Game(0), ClockConfig::default(), Duration::ZERO).expect("world"),
            now: Duration::ZERO,
            samples,
        };
        let (lane, controls) = WorldLane::start(task, Duration::ZERO, host_origin, move || {
            // The wake comes after the lane stores the update. Keep the worker
            // off the mailbox until the observation has polled it.
            let (release, parked) = mpsc::channel();
            if notify.send(release).is_ok() {
                let _ = parked.recv();
            }
        })
        .expect("lane");
        Self {
            wakes,
            samples: observed_samples,
            controls,
            lane,
            host_origin,
        }
    }

    fn committed_update(&self) -> Sender<()> {
        self.wakes.recv_timeout(WORKER_TIMEOUT).expect("lane wake")
    }

    fn step(&self, at: Duration) -> Sender<()> {
        self.controls.set(at).expect("step world clock");
        self.committed_update()
    }

    fn step_without_publication(&self, now: Duration) {
        self.controls.set(now).expect("step held clock");
        loop {
            let sample = self
                .samples
                .recv_timeout(WORKER_TIMEOUT)
                .expect("world sample");
            if let WorldSample::Advance {
                at,
                completed,
                published,
                interpolating,
            } = sample
                && at == now
            {
                assert_eq!(completed, 0, "no commit at {now:?}");
                assert!(!published, "no publication at {now:?}");
                assert!(!interpolating, "no consumed target at {now:?}");
                return;
            }
        }
    }

    fn request(&self, request: WorldLifecycle) -> (WorldLifecycleReply, Sender<()>) {
        let host_asked = self.host_origin.elapsed();
        self.controls.request_lifecycle(request).expect("request");
        let release = self.committed_update();
        let reply = self
            .controls
            .poll_lifecycle_reply()
            .expect("reply")
            .expect("acknowledged");
        let host_answered = self.host_origin.elapsed();
        assert_eq!(reply.request, request);
        if request == WorldLifecycle::Resume {
            loop {
                if let WorldSample::Resume(at) = self
                    .samples
                    .recv_timeout(WORKER_TIMEOUT)
                    .expect("resume sample")
                {
                    assert!(
                        (host_asked..=host_answered).contains(&at),
                        "worker read {at:?} outside {host_asked:?}..{host_answered:?}"
                    );
                    break;
                }
            }
        }
        (reply, release)
    }

    fn frame(&mut self, observation: Duration, tick: u64, interpolates: bool) -> Option<f64> {
        let report = self.lane.poll(observation).expect("poll");
        assert_eq!(report.tick, tick, "tick shown at {observation:?}");
        assert_eq!(self.lane.current_page(), Some(tick), "matching publication");
        assert_eq!(self.lane.is_interpolating(), interpolates);
        for lead_ms in LEADS_MS {
            let lead = Duration::from_millis(lead_ms);
            let phase = self.lane.display_phase_at(lead);
            assert_eq!(
                self.lane.presentation_alpha_at(lead).to_bits(),
                phase.min(1.0).to_bits()
            );
            if !interpolates {
                assert_eq!(phase, 1.0);
            }
        }
        if !interpolates {
            assert_eq!(self.lane.tick_due_time(), None);
            return None;
        }

        assert!(self.lane.display_phase_at(Duration::ZERO) < 4.0);
        let due = self
            .lane
            .tick_due_time()
            .expect("interpolating due time")
            .as_secs_f64();
        assert!(
            due <= observation.as_secs_f64() + TOLERANCE_SECONDS,
            "tick due by its observation"
        );
        Some(due - tick as f64 * TICK_SECONDS)
    }

    fn span_origin(&mut self, observation: Duration, tick: u64) -> f64 {
        self.frame(observation, tick, true).expect("running span")
    }
}

#[test]
fn a_resumed_lane_keeps_one_origin_inside_the_resume_bracket() {
    for (gap_ms, pause_ms) in [(0, 300), (3, 40), (20, 700)] {
        let mut run = LaneRun::new();
        let gap = Duration::from_millis(gap_ms);
        let initial = run.span_origin(Duration::ZERO, 0);
        let release = run.step(FIRST_TICK);
        let first = run.span_origin(FIRST_TICK, 1);
        drop(release);

        let observation = SECOND_TICK;
        let before_pause = observation + gap;
        let delayed = gap > FIRST_TICK;
        let paused_tick = if delayed { 3 } else { 2 };
        let release = run.step(before_pause);
        let shown = run.span_origin(observation, if delayed { 1 } else { 2 });
        let latest = run.span_origin(before_pause, paused_tick);
        for origin in [initial, first, shown, latest] {
            assert!(origin.abs() < TOLERANCE_SECONDS, "initial origin: {origin}");
        }
        drop(release);

        let (paused, release) = run.request(WorldLifecycle::Pause);
        assert_eq!(paused.id.tick, paused_tick);
        assert_eq!(run.frame(before_pause, paused_tick, false), None);
        drop(release);

        let asked = before_pause + Duration::from_millis(pause_ms);
        run.step_without_publication(asked);
        assert_eq!(run.frame(asked, paused_tick, false), None);

        let resume_sample = asked + Duration::from_millis(1);
        let answered = resume_sample + Duration::from_millis(1);
        // Queue the sampled time before the lifecycle request. Either the time
        // is installed first or the worker picks up both in the same iteration.
        run.controls
            .set(resume_sample)
            .expect("resume clock sample");
        let (resumed, release) = run.request(WorldLifecycle::Resume);
        assert_eq!(resumed.id, paused.id, "nothing commits while paused");
        assert_eq!(run.frame(answered, paused_tick, false), None);
        drop(release);

        let first_due = resume_sample + FIRST_TICK;
        let last_held = first_due.saturating_sub(Duration::from_nanos(1));
        run.step_without_publication(last_held);
        assert_eq!(run.frame(last_held, paused_tick, false), None);

        let release = run.step(first_due);
        let first_origin = run.span_origin(first_due, paused_tick + 1);
        drop(release);

        let later_observation = resume_sample + SECOND_TICK;
        let later_update = later_observation + gap;
        let release = run.step(later_update);
        let shown_origin =
            run.span_origin(later_observation, paused_tick + if delayed { 1 } else { 2 });
        let latest_origin =
            run.span_origin(later_update, paused_tick + if delayed { 3 } else { 2 });
        let low = first_origin.min(shown_origin).min(latest_origin);
        let high = first_origin.max(shown_origin).max(latest_origin);
        let spread = high - low;
        assert!(
            spread < TOLERANCE_SECONDS,
            "one origin for the span: {low} {high}"
        );
        let anchor = low + paused_tick as f64 * TICK_SECONDS;
        assert!(
            anchor >= asked.as_secs_f64() - TOLERANCE_SECONDS
                && anchor <= answered.as_secs_f64() + TOLERANCE_SECONDS,
            "{anchor} outside {asked:?}..{answered:?}"
        );
        println!(
            "gap {gap_ms:>2} ms, pause {pause_ms:>3} ms, held tick {paused_tick}, first resumed tick {}, resume anchor {anchor:.9} s in {asked:?}..{answered:?}, spread {:.0} ns",
            paused_tick + 1,
            spread * 1e9
        );
        drop(release);
    }
}
