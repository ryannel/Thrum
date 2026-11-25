//! Estimate the display interval from browser animation callbacks.

use std::time::Duration;

const STARTUP_INTERVALS: u8 = 20;

#[derive(Default)]
pub(crate) struct FrameCadence {
    previous: Option<Duration>,
    interval: Option<Duration>,
    warmup: u8,
    candidate: Option<(Duration, u8)>,
}

impl FrameCadence {
    pub(crate) fn interval(&self) -> Option<Duration> {
        self.interval.filter(|_| self.warmup >= STARTUP_INTERVALS)
    }

    pub(crate) fn observe(&mut self, now: Duration) -> Option<Duration> {
        let sample = self
            .previous
            .replace(now)
            .map(|previous| now.saturating_sub(previous))
            .filter(|sample| !sample.is_zero());
        if let Some(sample) = sample {
            if self.warmup < STARTUP_INTERVALS {
                self.interval = Some(self.interval.map_or(sample, |old| old.min(sample)));
                self.warmup += 1;
            } else if let Some(interval) = self.interval {
                self.track_change(interval, sample);
            }
        }
        self.interval()
    }

    fn track_change(&mut self, interval: Duration, sample: Duration) {
        // Browser timestamps are quantized. Allow jitter within a cadence and
        // require consecutive evidence before changing the budget.
        if sample.abs_diff(interval) <= interval.mul_f64(0.12)
            || !(Duration::from_millis(4)..=Duration::from_millis(34)).contains(&sample)
        {
            self.candidate = None;
            return;
        }
        let (first, count) = self
            .candidate
            .filter(|(first, _)| sample.abs_diff(*first) <= first.mul_f64(0.06))
            .map_or((sample, 1), |(first, count)| (first.min(sample), count + 1));
        // Adopt the new rhythm before the RAF fallback could accumulate
        // three over-budget intervals at the old refresh rate.
        if count == 3 {
            self.interval = Some(first);
            self.candidate = None;
        } else {
            self.candidate = Some((first, count));
        }
    }

    pub(crate) fn suspend(&mut self) {
        self.previous = None;
        self.candidate = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sustained_refresh_change_updates_the_budget_but_callback_stalls_do_not() {
        let mut cadence = FrameCadence::default();
        let fast = Duration::from_micros(8_333);
        let slow = Duration::from_micros(16_667);
        let mut now = Duration::ZERO;
        let mut observe = |cadence: &mut FrameCadence, gap| {
            now += gap;
            cadence.observe(now)
        };
        assert_eq!(observe(&mut cadence, Duration::ZERO), None);
        for _ in 0..19 {
            assert_eq!(observe(&mut cadence, fast), None);
        }
        assert_eq!(observe(&mut cadence, fast), Some(fast));
        for gap in [slow, fast * 3, Duration::from_millis(90), fast] {
            assert_eq!(observe(&mut cadence, gap), Some(fast));
        }
        for expected in [fast, fast, slow] {
            assert_eq!(observe(&mut cadence, slow), Some(expected));
        }
        assert_eq!(observe(&mut cadence, fast), Some(slow));
        cadence.suspend();
        assert_eq!(observe(&mut cadence, Duration::from_secs(10)), Some(slow));
        for expected in [slow, slow, fast] {
            assert_eq!(observe(&mut cadence, fast), Some(expected));
        }
    }
}
