//! Host due times and world display times through lifecycle spans.
mod lane;
use std::{num::NonZeroU32, time::Duration};
use thrum_core::{ClockConfig, ClockError, ClockState, TickClock};

const E9: i128 = 1_000_000_000;
const LEADS_MS: [u64; 4] = [0, 8, 30, 50];

/// The script's own account of the span it is in: tick `tick` fell due at `due_hz / hz` ns.
/// `due_hz` is host nanoseconds times the tick rate, so one tick is exactly `E9`.
#[derive(Clone, Copy)]
struct Anchor {
    due_hz: i128,
    tick: u64,
}

struct Run {
    hz: u32,
    clock: TickClock,
    tick: u64,
    anchor: Anchor,
    /// Whether the script expects the clock to consume a target.
    running: bool,
    held: bool,
    last: Option<(f64, f64)>, // display time, consumed target, both seconds
    frames: u32,
    worst_ns: f64,
    past_one: u32,
    at_bound: u32,
    stopped: u32,
}

impl Run {
    fn new(hz: u32, start: Duration, tick: u64) -> Self {
        let config = ClockConfig {
            tick_hz: NonZeroU32::new(hz).expect("valid scripted clock operation"),
            ..ClockConfig::default()
        };
        Self {
            hz,
            clock: TickClock::new(config, start).expect("valid scripted clock operation"),
            tick,
            anchor: Anchor {
                due_hz: i128::try_from(start.as_nanos()).expect("short script") * i128::from(hz),
                tick,
            },
            running: true,
            held: false,
            last: None,
            frames: 0,
            worst_ns: 0.0,
            past_one: 0,
            at_bound: 0,
            stopped: 0,
        }
    }
    fn d(&self) -> f64 {
        1.0 / f64::from(self.hz)
    }
    fn bound(&self) -> f64 {
        1.0 + 0.05 * f64::from(self.hz)
    }
    fn origin_ms(&self) -> f64 {
        (self.anchor.due_hz - i128::from(self.anchor.tick) * E9) as f64 / f64::from(self.hz) / 1e6
    }
    /// Advance the inline world to `o` and return the ticks it committed.
    fn advance(&mut self, o: Duration, budget: u32) -> u32 {
        let tick = &mut self.tick;
        let report = self
            .clock
            .advance(o, budget, || {
                *tick += 1;
                Ok::<_, ()>(())
            })
            .expect("valid scripted clock operation");
        if report.completed > 0 {
            self.held = false;
        }
        report.completed
    }
    /// One frame: observe at `o`, sample at `o + lead`, and hold the clock to the model.
    /// Returns (display time, consumed target) in seconds when the frame interpolates.
    fn frame(&mut self, o: Duration, lead_ms: u64, budget: u32) -> Option<(f64, f64)> {
        self.advance(o, budget);
        let lead = Duration::from_millis(lead_ms);
        let sample = self.clock.presentation_sample();
        let phase = self.clock.display_phase_at(lead);
        let alpha = self.clock.presentation_alpha_at(lead);
        assert_eq!(
            alpha.to_bits(),
            phase.min(1.0).to_bits(),
            "alpha is the phase held to 1"
        );
        let expect = self.running && !self.held && self.clock.queued_ticks() == 0;
        assert_eq!(sample.is_interpolating(), expect, "interpolating at {o:?}");
        self.frames += 1;
        let world = self.tick as f64 * self.d();
        if !expect {
            assert_eq!(sample.tick_due_time(), None);
            assert_eq!(phase, 1.0);
            self.stopped += 1;
            self.last = Some((world, (o + lead).as_secs_f64()));
            return None;
        }
        // The model: p = (T - t_C) / d, with t_C from the script's anchor alone.
        let target_hz =
            i128::try_from((o + lead).as_nanos()).expect("short script") * i128::from(self.hz);
        let due_hz = self.anchor.due_hz + i128::from(self.tick - self.anchor.tick) * E9;
        assert!(
            due_hz <= i128::try_from(o.as_nanos()).expect("short script") * i128::from(self.hz),
            "tick due by its observation"
        );
        let expected_due_ns = u128::try_from(due_hz)
            .expect("valid scripted clock operation")
            .div_ceil(u128::from(self.hz));
        let due = sample.tick_due_time().expect("interpolating due time");
        assert_eq!(
            due.as_nanos(),
            expected_due_ns,
            "scripted anchor determines due time"
        );

        let exact = (target_hz - due_hz) as f64 / E9 as f64;
        let model = exact.min(self.bound());
        assert!(
            (f64::from(phase) - model).abs() < 1e-6,
            "{o:?} +{lead_ms}: {phase} != {model}"
        );
        if exact > self.bound() {
            self.at_bound += 1;
            self.last = None;
            return None;
        }
        self.past_one += u32::from(phase > 1.0);
        // Display time is world time: the tick's, less one tick, plus host time since it fell due.
        let display = (self.tick as f64 - 1.0 + f64::from(phase)) * self.d();
        let required = (self.tick as f64 - 1.0) * self.d()
            + (target_hz - due_hz) as f64 / E9 as f64 * self.d();
        let error = (display - required).abs() * 1e9;
        assert!(error < 2000.0, "{error} ns");
        self.worst_ns = self.worst_ns.max(error);
        let now = (display, (o + lead).as_secs_f64());
        self.last = Some(now);
        Some(now)
    }
    fn resume(&mut self, at: Duration) {
        self.clock
            .resume(at)
            .expect("valid scripted clock operation");
        self.anchor = Anchor {
            due_hz: i128::try_from(at.as_nanos()).expect("short script") * i128::from(self.hz),
            tick: self.tick,
        };
        self.running = true;
        self.held = true;
    }
}

fn steady(run: &mut Run, mut o: Duration, frames: u32, step: Duration) -> Duration {
    let mut previous: [Option<(f64, f64)>; 4] = [None; 4];
    for i in 0..frames {
        o += step;
        let slot = (i % 4) as usize;
        if let Some((display, target)) = run.frame(o, LEADS_MS[slot], 4) {
            // In one span equal intervals between consumed targets are equal steps of display time.
            if let Some((old_display, old_target)) = previous[slot] {
                let uneven: f64 = (display - old_display) - (target - old_target);
                assert!(uneven.abs() < 4e-6, "uneven by {uneven}");
            }
            previous[slot] = Some((display, target));
        } else {
            previous[slot] = None;
        }
    }
    o
}

#[expect(clippy::print_stdout, reason = "retained timing model evidence")]
fn lifecycle(hz: u32) {
    let frame = Duration::from_nanos(1_000_000_000 / 120);
    let start = Duration::from_nanos(7_300_000_123);
    let mut run = Run::new(hz, start, 100);
    println!("{hz} Hz, clock started at {start:?} on tick 100");
    println!("  origin at the start        {:>14.6} ms", run.origin_ms());

    // 1. Running from a start that is neither host time zero nor tick zero.
    let mut o = steady(&mut run, start, 48, frame);

    // 2. Ticks queue (no CPU budget), then drain: no interpolation meanwhile, same origin after.
    o += Duration::from_millis(40);
    assert!(run.frame(o, 30, 0).is_none());
    assert!(run.clock.queued_ticks() > 0);
    let queued_origin = run.origin_ms();
    o = steady(&mut run, o, 24, frame);
    assert_eq!(queued_origin, run.origin_ms());

    // 3. Pause partway through a tick. The fraction is discarded with the paused time.
    let before = run
        .frame(o + frame, 50, 4)
        .expect("running before the pause");
    o += frame;
    let committed = run.tick as f64 * run.d();
    run.clock.pause();
    run.running = false;
    let paused = o + Duration::from_millis(200);
    assert!(run.frame(paused, 50, 4).is_none());
    let into_stop = committed - before.0;
    assert!(
        into_stop >= -0.050_000_1 && into_stop < run.d(),
        "{into_stop}"
    );
    let old_origin = run.origin_ms();
    let resume = paused + Duration::from_millis(800) + Duration::from_nanos(3);
    run.resume(resume);
    println!(
        "  after pause and resume     {:>14.6} ms   moved {:.6} ms; the clock was stopped {:.6} ms",
        run.origin_ms(),
        run.origin_ms() - old_origin,
        resume.saturating_sub(o).as_secs_f64() * 1e3
    );
    assert!(run.origin_ms() - old_origin >= resume.saturating_sub(o).as_secs_f64() * 1e3 - 1e-6);
    // The hold: no target is consumed until the first tick after the resume commits,
    // however far a lead reaches past that tick's due time.
    let tick_ns = 1_000_000_000 / u64::from(hz);
    for (after_ns, lead) in [(1_000_000, 0), (tick_ns - 1_000, 50), (tick_ns / 2, 50)] {
        let mut probe = Run {
            clock: clone(&run.clock, hz, resume),
            ..copy(&run)
        };
        assert!(
            probe
                .frame(resume + Duration::from_nanos(after_ns), lead, 4)
                .is_none()
        );
    }
    // First interpolating frame: display time goes on from the committed tick.
    let first = resume + Duration::from_nanos(tick_ns + tick_ns / 2);
    let held_at = run.tick as f64 * run.d();
    let (display, target) = run
        .frame(first, 30, 4)
        .expect("interpolating after the hold");
    let onward = display - held_at;
    let due = (resume + Duration::from_nanos(tick_ns)).as_secs_f64();
    assert!((onward - (target - due)).abs() < 2e-6);
    println!(
        "  first frame after the hold moved display time on by {:.3} ms: {:.3} ms since the tick fell due, 30 ms of it lead",
        onward * 1e3,
        (target - due) * 1e3
    );
    o = steady(&mut run, first, 48, frame);

    // 4. Overload: more ticks fall due than the queue admits. Time stops until a resume.
    o += Duration::from_secs(2);
    run.running = false; // the script knows two seconds is more than the queue admits
    assert!(run.frame(o, 8, 4).is_none());
    assert_eq!(run.clock.state(), ClockState::Overloaded);
    let old_origin = run.origin_ms();
    o += Duration::from_millis(5);
    run.resume(o);
    println!(
        "  after overload and resume  {:>14.6} ms   moved {:.6} ms",
        run.origin_ms(),
        run.origin_ms() - old_origin
    );
    o = steady(&mut run, o, 48, frame);

    // 5. Overload with ticks still queued: they drain while stopped, then the resume is accepted.
    o += Duration::from_millis(45);
    assert!(run.frame(o, 0, 0).is_none());
    let queued = run.clock.queued_ticks();
    assert!(queued >= 2);
    o += Duration::from_secs(1);
    run.advance(o, 0);
    assert_eq!(run.clock.state(), ClockState::Overloaded);
    run.running = false;
    assert_eq!(run.clock.resume(o), Err(ClockError::PendingTicks));
    assert!(run.frame(o, 0, 1).is_none()); // one drains, shown as the committed tick
    assert!(run.frame(o, 0, 4).is_none());
    assert_eq!(run.clock.queued_ticks(), 0);
    run.resume(o);
    o = steady(&mut run, o, 48, frame);

    // 6. The world rewinds ten ticks while the clock runs: the tick it lands on takes the due
    //    time of the tick it left, and the frames after it follow the new numbering.
    let old_origin = run.origin_ms();
    run.anchor = Anchor {
        due_hz: run.anchor.due_hz + i128::from(run.tick - run.anchor.tick) * E9,
        tick: run.tick - 10,
    };
    run.tick -= 10;
    println!(
        "  after a ten-tick rewind    {:>14.6} ms   moved {:.6} ms",
        run.origin_ms(),
        run.origin_ms() - old_origin
    );
    steady(&mut run, o, 48, frame);
    println!(
        "  {} frames: {} stopped, {} past 1, {} at the bound, worst {:.3} ns from the model",
        run.frames, run.stopped, run.past_one, run.at_bound, run.worst_ns
    );
    assert!(run.past_one > 0 && run.stopped > 0);
}

fn copy(run: &Run) -> Run {
    Run {
        hz: run.hz,
        clock: TickClock::new(ClockConfig::default(), Duration::ZERO)
            .expect("valid scripted clock operation"),
        tick: run.tick,
        anchor: run.anchor,
        running: run.running,
        held: run.held,
        last: run.last,
        frames: 0,
        worst_ns: 0.0,
        past_one: 0,
        at_bound: 0,
        stopped: 0,
    }
}

/// A second clock in the state the first is in just after its resume, for frames that
/// must not disturb the run.
fn clone(_: &TickClock, hz: u32, resume: Duration) -> TickClock {
    let config = ClockConfig {
        tick_hz: NonZeroU32::new(hz).expect("valid scripted clock operation"),
        ..ClockConfig::default()
    };
    let mut clock = TickClock::new(config, Duration::ZERO).expect("valid scripted clock operation");
    clock.pause();
    clock
        .resume(resume)
        .expect("valid scripted clock operation");
    clock
}

/// The review's two arms, on a clock that starts at host time zero on tick zero.
#[expect(clippy::print_stdout, reason = "retained timing model evidence")]
fn review_arms() {
    println!("the review's frames (60 Hz, started at zero on tick 0)");
    for pause_ms in [50_u64, 58] {
        let mut run = Run::new(60, Duration::ZERO, 0);
        run.frame(Duration::from_millis(pause_ms), 0, 4);
        assert_eq!(run.tick, 3);
        run.clock.pause();
        run.running = false;
        let resume = Duration::from_millis(1000 + pause_ms);
        run.resume(resume);
        for lead in [0, 10, 50] {
            let mut arm = Run {
                clock: clone(&run.clock, 60, resume),
                ..copy(&run)
            };
            let o = resume + Duration::from_millis(25);
            let (display, target) = arm.frame(o, lead, 4).expect("interpolating");
            let literal = target - 1.0 / 60.0;
            let mapped = target - arm.origin_ms() / 1e3 - 1.0 / 60.0;
            println!(
                "  paused at {pause_ms} ms, lead {lead:>2} ms: tick {} display {:.6} s; literal identity off by {:+.6} s; with the origin {:.3} ms off by {:+.9} s",
                arm.tick,
                display,
                display - literal,
                arm.origin_ms(),
                display - mapped
            );
            assert!((display - mapped).abs() < 2e-6);
        }
    }
}

#[test]
fn scripted_clock_spans() {
    review_arms();
    for hz in [60, 100] {
        lifecycle(hz);
    }
    println!("every frame met the model");
}
