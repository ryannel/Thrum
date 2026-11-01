//! Fixed-step host clock behavior and failure cases.

use std::num::NonZeroU32;
use std::time::Duration;

use thrum_core::{ClockConfig, ClockError, ClockState, TickClock};

#[test]
fn display_callbacks_do_not_set_simulation_rate() {
    for frames in [30_u64, 60, 120, 240] {
        let mut clock = TickClock::new(ClockConfig::default(), Duration::ZERO).unwrap();
        let mut steps = 0;
        for frame in 1..=frames {
            let now = Duration::from_nanos(frame * 1_000_000_000 / frames);
            let report = clock
                .advance(now, 4, || -> Result<(), ()> { Ok(()) })
                .unwrap();
            steps += report.completed;
            assert_eq!(report.state, ClockState::Active);
            assert_eq!(report.refused_on_overload, 0);
            assert!(report.step_error.is_none());
        }
        assert_eq!(steps, 60, "display callbacks: {frames}");
        assert_eq!(clock.queued_ticks(), 0);
    }
}

#[test]
fn a_game_can_select_a_higher_world_update_rate() {
    let config = ClockConfig {
        tick_hz: NonZeroU32::new(120).unwrap(),
        ..ClockConfig::default()
    };
    let mut clock = TickClock::new(config, Duration::ZERO).unwrap();
    let mut steps = 0;
    for frame in 1..=240_u64 {
        let now = Duration::from_nanos(frame * 1_000_000_000 / 240);
        steps += clock
            .advance(now, 4, || -> Result<(), ()> { Ok(()) })
            .unwrap()
            .completed;
    }
    assert_eq!(steps, 120);
}

#[test]
fn failed_step_is_retained_and_retry_runs_once() {
    let mut clock = TickClock::new(ClockConfig::default(), Duration::ZERO).unwrap();
    let mut attempts = 0;
    let report = clock
        .advance(Duration::from_millis(17), 4, || {
            attempts += 1;
            Err("reader still holds publication")
        })
        .unwrap();
    assert_eq!(report.completed, 0);
    assert_eq!(report.queued, 1);
    assert_eq!(report.step_error, Some("reader still holds publication"));
    assert_eq!(attempts, 1);
    let report = clock
        .advance(Duration::from_millis(17), 4, || -> Result<(), ()> {
            attempts += 1;
            Ok(())
        })
        .unwrap();
    assert_eq!(report.completed, 1);
    assert_eq!(report.queued, 0);
    assert_eq!(attempts, 2);
}

#[test]
fn overload_is_explicit_and_needs_a_fresh_origin() {
    let mut clock = TickClock::new(ClockConfig::default(), Duration::ZERO).unwrap();
    let report = clock
        .advance(Duration::from_millis(134), 0, || -> Result<(), ()> {
            Ok(())
        })
        .unwrap();
    assert_eq!(report.queued, 8);
    assert_eq!(report.state, ClockState::Active);
    let report = clock
        .advance(Duration::from_millis(151), 0, || -> Result<(), ()> {
            Ok(())
        })
        .unwrap();
    assert_eq!(report.state, ClockState::Overloaded);
    assert_eq!(report.refused_on_overload, 1);
    assert_eq!(report.queued, 8);
    assert_eq!(
        clock.resume(Duration::from_millis(151)),
        Err(ClockError::PendingTicks)
    );
    let mut completed = 0;
    for now in [Duration::from_secs(1), Duration::from_secs(2)] {
        let report = clock
            .advance(now, 4, || -> Result<(), ()> { Ok(()) })
            .unwrap();
        completed += report.completed;
        assert_eq!(report.refused_on_overload, 0);
    }
    assert_eq!(completed, 8);
    assert_eq!(clock.queued_ticks(), 0);
    clock.resume(Duration::from_secs(2)).unwrap();
    assert_eq!(clock.state(), ClockState::Active);
    let report = clock
        .advance(
            Duration::from_secs(2) + Duration::from_millis(17),
            4,
            || -> Result<(), ()> { Ok(()) },
        )
        .unwrap();
    assert_eq!(report.completed, 1);
}

#[test]
fn pause_ignores_hidden_time_and_rejects_clock_reversal() {
    let mut clock = TickClock::new(ClockConfig::default(), Duration::from_secs(2)).unwrap();
    assert_eq!(
        clock.resume(Duration::from_secs(100)),
        Err(ClockError::AlreadyActive)
    );
    clock.pause();
    let report = clock
        .advance(Duration::from_secs(100), 4, || -> Result<(), ()> {
            panic!("paused world ran")
        })
        .unwrap();
    assert_eq!(report.completed, 0);
    assert_eq!(report.queued, 0);
    assert_eq!(
        clock.resume(Duration::from_secs(99)),
        Err(ClockError::NonMonotonicTime)
    );
    assert_eq!(clock.state(), ClockState::Paused);
    clock.resume(Duration::from_secs(101)).unwrap();
    assert_eq!(
        clock
            .advance(Duration::from_secs(100), 4, || -> Result<(), ()> { Ok(()) })
            .err(),
        Some(ClockError::NonMonotonicTime)
    );
    let report = clock
        .advance(
            Duration::from_secs(101) + Duration::from_millis(17),
            4,
            || Ok::<_, ()>(()),
        )
        .unwrap();
    assert_eq!(report.completed, 1);
}

#[test]
fn pause_preserves_and_drains_already_admitted_work() {
    let mut clock = TickClock::new(ClockConfig::default(), Duration::ZERO).unwrap();
    let report = clock
        .advance(Duration::from_millis(34), 0, || -> Result<(), ()> {
            Ok(())
        })
        .unwrap();
    assert_eq!(report.queued, 2);
    clock.pause();
    let report = clock
        .advance(Duration::from_secs(10), 4, || -> Result<(), ()> { Ok(()) })
        .unwrap();
    assert_eq!(report.state, ClockState::Paused);
    assert_eq!(report.completed, 2);
    assert_eq!(report.queued, 0);
    assert_eq!(report.admitted, 0);
    clock.resume(Duration::from_secs(10)).unwrap();
}
