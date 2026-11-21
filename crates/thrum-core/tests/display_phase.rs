//! Scripted consumed-target expectations, independent of clock sampling code.
mod world;
use std::{num::NonZeroU32, time::Duration};
use thrum_core::{ClockConfig, TickClock};

fn advance(clock: &mut TickClock, millis: u64, budget: u32) -> u32 {
    clock
        .advance(Duration::from_millis(millis), budget, || Ok::<_, ()>(()))
        .expect("monotonic script")
        .completed
}

fn stopped(clock: &TickClock) {
    let sample = clock.presentation_sample();
    assert!(!sample.is_interpolating());
    for target in [Duration::ZERO, Duration::from_secs(100)] {
        assert_eq!(sample.display_phase_at(target), 1.0);
        assert_eq!(sample.alpha_at(target), 1.0);
    }
}

#[test]
fn consumed_target_extends_alpha_without_mutating_authority() {
    for (hz, expected) in [(60, [0.5, 1.4, 2.9, 4.0]), (100, [0.5, 2.0, 4.5, 6.0])] {
        let mut clock = TickClock::new(
            ClockConfig {
                tick_hz: NonZeroU32::new(hz).expect("rate"),
                ..ClockConfig::default()
            },
            Duration::ZERO,
        )
        .expect("clock");
        // 25 ms is tick 1 + .5 at 60 Hz, tick 2 + .5 at 100 Hz.
        let ticks = advance(&mut clock, 25, 4);
        assert_eq!(ticks, if hz == 60 { 1 } else { 2 });
        let sample = clock.presentation_sample();
        assert!(sample.is_interpolating());
        let deadline = clock.next_due_in();
        for (lead_ms, expected) in [0, 15, 40, 100].into_iter().zip(expected) {
            let lead = Duration::from_millis(lead_ms);
            let target = Duration::from_millis(25) + lead;
            let phase = sample.display_phase_at(target);
            assert!(
                (phase - expected).abs() < 1e-6,
                "{hz} Hz, {lead_ms} ms: {phase}"
            );
            assert_eq!(sample.alpha_at(target).to_bits(), phase.min(1.0).to_bits());
            assert_eq!(clock.display_phase_at(lead), phase);
        }
        // Exactly at the bound: 1 + 50 ms in ticks, independently specified.
        let bound_target = if hz == 60 {
            Duration::from_nanos(83_333_334)
        } else {
            Duration::from_millis(80)
        };
        assert_eq!(
            sample.display_phase_at(bound_target),
            if hz == 60 { 4.0 } else { 6.0 }
        );
        assert_eq!(sample.display_phase_at(Duration::ZERO), 0.0);
        assert_eq!(clock.queued_ticks(), 0);
        assert_eq!(clock.next_due_in(), deadline);
        // No sampling has admitted or consumed any work.
        assert_eq!(advance(&mut clock, 25, 4), 0);
    }
}

#[test]
fn every_endpoint_only_state_reports_one() {
    let mut clock = TickClock::new(ClockConfig::default(), Duration::ZERO).expect("clock");
    clock.pause();
    stopped(&clock);
    clock.resume(Duration::from_millis(10)).expect("resume");
    stopped(&clock); // hold survives until a tick commits
    assert_eq!(advance(&mut clock, 20, 4), 0);
    stopped(&clock);
    assert_eq!(advance(&mut clock, 30, 4), 1);
    assert!(clock.presentation_sample().is_interpolating());
    advance(&mut clock, 60, 0);
    assert!(clock.queued_ticks() > 0);
    stopped(&clock);
    advance(&mut clock, 60, 4);
    assert!(clock.presentation_sample().is_interpolating());
    advance(&mut clock, 1000, 4);
    stopped(&clock); // overload
}
