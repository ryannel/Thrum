//! Fixed-step host clock behavior and failure cases.

use std::num::NonZeroU32;
use std::time::Duration;

use thrum_core::{ClockConfig, ClockError, ClockState, TickClock};

#[test]
fn display_callbacks_do_not_set_simulation_rate() {
    for frames in [30_u64, 60, 120, 240] {
        let mut clock = TickClock::new(ClockConfig::default(), Duration::ZERO)
            .expect("test setup or assertion requires a value");
        let mut steps = 0;
        for frame in 1..=frames {
            let now = Duration::from_nanos(frame * 1_000_000_000 / frames);
            let report = clock
                .advance(now, 4, || -> Result<(), ()> { Ok(()) })
                .expect("test setup or assertion requires a value");
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
        tick_hz: NonZeroU32::new(120).expect("test setup or assertion requires a value"),
        ..ClockConfig::default()
    };
    let mut clock =
        TickClock::new(config, Duration::ZERO).expect("test setup or assertion requires a value");
    let mut steps = 0;
    for frame in 1..=240_u64 {
        let now = Duration::from_nanos(frame * 1_000_000_000 / 240);
        steps += clock
            .advance(now, 4, || -> Result<(), ()> { Ok(()) })
            .expect("test setup or assertion requires a value")
            .completed;
    }
    assert_eq!(steps, 120);
}

#[test]
fn next_due_delay_tracks_fractional_time_rate_and_stopped_backlog() {
    let mut clock = TickClock::new(ClockConfig::default(), Duration::ZERO)
        .expect("test setup or assertion requires a value");
    assert_eq!(clock.next_due_in(), Some(Duration::from_nanos(16_666_667)));
    clock
        .advance(Duration::from_millis(10), 4, || Ok::<_, ()>(()))
        .expect("test setup or assertion requires a value");
    assert_eq!(clock.next_due_in(), Some(Duration::from_nanos(6_666_667)));
    clock
        .advance(Duration::from_millis(34), 0, || Ok::<_, ()>(()))
        .expect("test setup or assertion requires a value");
    assert_eq!(clock.queued_ticks(), 2);
    assert_eq!(clock.next_due_in(), Some(Duration::ZERO));
    clock.pause();
    assert_eq!(clock.next_due_in(), Some(Duration::ZERO));
    clock
        .advance(Duration::from_secs(1), 4, || Ok::<_, ()>(()))
        .expect("test setup or assertion requires a value");
    assert_eq!(clock.next_due_in(), None);
    clock
        .resume(Duration::from_secs(1))
        .expect("test setup or assertion requires a value");
    assert_eq!(clock.next_due_in(), Some(Duration::from_nanos(16_666_667)));

    let custom = TickClock::new(
        ClockConfig {
            tick_hz: NonZeroU32::new(240).expect("test setup or assertion requires a value"),
            ..ClockConfig::default()
        },
        Duration::ZERO,
    )
    .expect("test setup or assertion requires a value");
    assert_eq!(custom.next_due_in(), Some(Duration::from_nanos(4_166_667)));
}

#[test]
fn deadline_driven_callbacks_do_one_update_each_without_idle_polling() {
    for hz in [60_u32, 240] {
        let mut clock = TickClock::new(
            ClockConfig {
                tick_hz: NonZeroU32::new(hz).expect("test setup or assertion requires a value"),
                ..ClockConfig::default()
            },
            Duration::ZERO,
        )
        .expect("test setup or assertion requires a value");
        let mut now = Duration::ZERO;
        for _ in 0..hz {
            now += clock
                .next_due_in()
                .expect("test setup or assertion requires a value");
            let report = clock
                .advance(now, 4, || Ok::<_, ()>(()))
                .expect("test setup or assertion requires a value");
            assert_eq!(report.completed, 1);
            assert_eq!(report.queued, 0);
            assert_eq!(report.state, ClockState::Active);
        }
        assert_eq!(now, Duration::from_secs(1));
    }
}

#[test]
fn failed_step_is_retained_and_retry_runs_once() {
    let mut clock = TickClock::new(ClockConfig::default(), Duration::ZERO)
        .expect("test setup or assertion requires a value");
    let mut attempts = 0;
    let report = clock
        .advance(Duration::from_millis(17), 4, || {
            attempts += 1;
            Err("reader still holds publication")
        })
        .expect("test setup or assertion requires a value");
    assert_eq!(report.completed, 0);
    assert_eq!(report.queued, 1);
    assert_eq!(report.step_error, Some("reader still holds publication"));
    assert_eq!(attempts, 1);
    let report = clock
        .advance(Duration::from_millis(17), 4, || -> Result<(), ()> {
            attempts += 1;
            Ok(())
        })
        .expect("test setup or assertion requires a value");
    assert_eq!(report.completed, 1);
    assert_eq!(report.queued, 0);
    assert_eq!(attempts, 2);
}

#[test]
fn overload_is_explicit_and_needs_a_fresh_origin() {
    let mut clock = TickClock::new(ClockConfig::default(), Duration::ZERO)
        .expect("test setup or assertion requires a value");
    let report = clock
        .advance(Duration::from_millis(134), 0, || -> Result<(), ()> {
            Ok(())
        })
        .expect("test setup or assertion requires a value");
    assert_eq!(report.queued, 8);
    assert_eq!(report.state, ClockState::Active);
    let report = clock
        .advance(Duration::from_millis(151), 0, || -> Result<(), ()> {
            Ok(())
        })
        .expect("test setup or assertion requires a value");
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
            .expect("test setup or assertion requires a value");
        completed += report.completed;
        assert_eq!(report.refused_on_overload, 0);
    }
    assert_eq!(completed, 8);
    assert_eq!(clock.queued_ticks(), 0);
    clock
        .resume(Duration::from_secs(2))
        .expect("test setup or assertion requires a value");
    assert_eq!(clock.state(), ClockState::Active);
    let report = clock
        .advance(
            Duration::from_secs(2) + Duration::from_millis(17),
            4,
            || -> Result<(), ()> { Ok(()) },
        )
        .expect("test setup or assertion requires a value");
    assert_eq!(report.completed, 1);
}

#[test]
fn pause_ignores_hidden_time_and_rejects_clock_reversal() {
    let mut clock = TickClock::new(ClockConfig::default(), Duration::from_secs(2))
        .expect("test setup or assertion requires a value");
    assert_eq!(
        clock.resume(Duration::from_secs(100)),
        Err(ClockError::AlreadyActive)
    );
    clock.pause();
    let report = clock
        .advance(Duration::from_secs(100), 4, || -> Result<(), ()> {
            panic!("paused world ran")
        })
        .expect("test setup or assertion requires a value");
    assert_eq!(report.completed, 0);
    assert_eq!(report.queued, 0);
    assert_eq!(
        clock.resume(Duration::from_secs(99)),
        Err(ClockError::NonMonotonicTime)
    );
    assert_eq!(clock.state(), ClockState::Paused);
    clock
        .resume(Duration::from_secs(101))
        .expect("test setup or assertion requires a value");
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
        .expect("test setup or assertion requires a value");
    assert_eq!(report.completed, 1);
}

#[test]
fn pause_preserves_and_drains_already_admitted_work() {
    let mut clock = TickClock::new(ClockConfig::default(), Duration::ZERO)
        .expect("test setup or assertion requires a value");
    let report = clock
        .advance(Duration::from_millis(34), 0, || -> Result<(), ()> {
            Ok(())
        })
        .expect("test setup or assertion requires a value");
    assert_eq!(report.queued, 2);
    clock.pause();
    let report = clock
        .advance(Duration::from_secs(10), 4, || -> Result<(), ()> { Ok(()) })
        .expect("test setup or assertion requires a value");
    assert_eq!(report.state, ClockState::Paused);
    assert_eq!(report.completed, 2);
    assert_eq!(report.queued, 0);
    assert_eq!(report.admitted, 0);
    clock
        .resume(Duration::from_secs(10))
        .expect("test setup or assertion requires a value");
}
