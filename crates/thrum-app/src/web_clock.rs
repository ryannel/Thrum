//! The browser shell's page clock policy, free of browser and GPU types so it is tested natively.
//!
//! Page timestamps become world time from the first observed timestamp. A timestamp earlier than
//! the last one admitted is clamped to it: animation frames report their frame-start time, while a
//! visibility event samples `performance.now()` later in the same frame. The world runs only while
//! the page is visible and the person has not paused it; the clock says when that composition
//! starts or stops, and the world decides how its queued ticks drain.

use std::time::Duration;

/// What the world must do after a play-state change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Transition {
    /// The composed state did not change.
    None,
    /// Stop admitting wall time, keeping accepted ticks.
    Pause,
    /// Resume from now, or once the accepted ticks have drained.
    Resume,
}

#[derive(Debug)]
pub(crate) struct PageClock {
    first_ms: Option<f64>,
    last: Duration,
    visible: bool,
    user_paused: bool,
    clamps: u64,
}

impl PageClock {
    pub(crate) fn new() -> Self {
        Self {
            first_ms: None,
            last: Duration::ZERO,
            visible: true,
            user_paused: false,
            clamps: 0,
        }
    }

    /// World time for a page timestamp in milliseconds.
    pub(crate) fn now(&mut self, timestamp_ms: f64) -> Result<Duration, &'static str> {
        if !timestamp_ms.is_finite() || timestamp_ms < 0.0 {
            return Err("animation timestamp must be finite and nonnegative");
        }
        let first = *self.first_ms.get_or_insert(timestamp_ms);
        let now = Duration::try_from_secs_f64((timestamp_ms - first).max(0.0) / 1_000.0)
            .map_err(|_| "animation timestamp outside supported range")?;
        if now < self.last {
            self.clamps += 1;
            return Ok(self.last);
        }
        Ok(now)
    }

    /// Record an admitted frame callback at `now`.
    pub(crate) fn mark(&mut self, now: Duration) {
        self.last = now;
    }

    /// Compose visibility with the person's pause and say what the world must do.
    pub(crate) fn set_play(
        &mut self,
        visible: bool,
        user_paused: bool,
        now: Duration,
    ) -> Transition {
        let was_running = self.visible && !self.user_paused;
        let will_run = visible && !user_paused;
        self.visible = visible;
        self.user_paused = user_paused;
        self.last = now;
        match (was_running, will_run) {
            (false, true) => Transition::Resume,
            (true, false) => Transition::Pause,
            _ => Transition::None,
        }
    }

    pub(crate) fn visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn user_paused(&self) -> bool {
        self.user_paused
    }

    /// Time of the last admitted callback or play-state change, since the first timestamp.
    pub(crate) fn last(&self) -> Duration {
        self.last
    }

    pub(crate) fn clamps(&self) -> u64 {
        self.clamps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_timestamp_is_the_origin() {
        let mut clock = PageClock::new();
        assert_eq!(clock.now(5_000.0), Ok(Duration::ZERO));
        assert_eq!(clock.now(5_250.0), Ok(Duration::from_millis(250)));
    }

    #[test]
    fn invalid_timestamps_are_refused() {
        let mut clock = PageClock::new();
        assert!(clock.now(f64::NAN).is_err());
        assert!(clock.now(-1.0).is_err());
        assert!(clock.now(f64::INFINITY).is_err());
    }

    #[test]
    fn a_timestamp_before_the_last_admitted_one_is_clamped_and_counted() {
        let mut clock = PageClock::new();
        clock.now(0.0).unwrap();
        let later = clock.now(100.0).unwrap();
        clock.mark(later);
        assert_eq!(clock.now(90.0), Ok(later));
        assert_eq!(clock.clamps(), 1);
        assert_eq!(clock.now(120.0), Ok(Duration::from_millis(120)));
        assert_eq!(clock.clamps(), 1);
    }

    #[test]
    fn visibility_and_pause_compose() {
        let mut clock = PageClock::new();
        let t = Duration::from_millis(10);
        assert_eq!(clock.set_play(false, false, t), Transition::Pause);
        // Pausing while hidden changes nothing the world sees.
        assert_eq!(clock.set_play(false, true, t), Transition::None);
        // Becoming visible cannot override the person's pause.
        assert_eq!(clock.set_play(true, true, t), Transition::None);
        assert_eq!(clock.set_play(true, false, t), Transition::Resume);
        assert_eq!(clock.set_play(true, false, t), Transition::None);
        assert_eq!(clock.set_play(true, true, t), Transition::Pause);
        assert!(clock.visible() && clock.user_paused());
    }

    #[test]
    fn a_play_change_moves_the_last_admitted_time() {
        let mut clock = PageClock::new();
        clock.now(0.0).unwrap();
        clock.set_play(false, false, Duration::from_millis(400));
        assert_eq!(clock.last(), Duration::from_millis(400));
        assert_eq!(clock.now(300.0), Ok(Duration::from_millis(400)));
    }
}
