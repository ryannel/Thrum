//! Requested capture opportunities; never an authoritative world clock.
use std::time::{Duration, Instant};

/// Explicit distinction between synthetic sampling and elapsed runtime proof.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Timebase {
    /// Actual monotonic elapsed time, including oversleep and stalls.
    Elapsed,
    /// Explicit synthetic sample times; never live-motion evidence.
    Fixed,
}

/// A bounded-rate opportunity source replacing only the OS callback source.
pub struct OpportunityClock {
    origin: Instant,
    index: u32,
    hz: u32,
    timebase: Timebase,
}

/// A capture opportunity rate outside the supported range.
#[derive(Debug)]
pub struct OpportunityRateError;

impl std::fmt::Display for OpportunityRateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("capture opportunity rate must be 1..=1000 Hz")
    }
}

impl std::error::Error for OpportunityRateError {}

/// One requested opportunity. Missing indices represent elapsed scheduling gaps.
pub struct Opportunity {
    /// Requested opportunity index, independent of submission IDs.
    pub index: u32,
    /// Scenario time on the declared timebase.
    pub scenario_time: Duration,
    /// Actual request observation, always monotonic elapsed host time.
    pub requested: Duration,
    /// Scheduled indices skipped while the driver was busy or overslept.
    pub skipped_before: u32,
    /// Platform target requested by the runner, on its monotonic clock.
    pub target: Instant,
}

impl OpportunityClock {
    /// Discard elapsed opportunities before an admission wait or after it.
    /// The caller records the returned count with the reason for that interval.
    /// Fixed clocks never discard opportunities.
    pub fn discard_elapsed(&mut self) -> u32 {
        if self.timebase == Timebase::Fixed {
            return 0;
        }
        let next =
            u32::try_from(self.origin.elapsed().as_nanos() * u128::from(self.hz) / 1_000_000_000)
                .unwrap_or(u32::MAX)
                .saturating_add(1);
        let skipped = next.saturating_sub(self.index);
        self.index = self.index.max(next);
        skipped
    }

    /// Start after a primed initial submission at time zero.
    /// # Errors
    /// Refuses zero or implausibly high opportunity rates.
    pub fn new(origin: Instant, hz: u32, timebase: Timebase) -> Result<Self, OpportunityRateError> {
        if !(1..=1000).contains(&hz) {
            return Err(OpportunityRateError);
        }
        Ok(Self {
            origin,
            index: 1,
            hz,
            timebase,
        })
    }

    /// Request the next opportunity. Elapsed mode skips expired indices rather
    /// than bursting synthetic callbacks after a stall. Fixed mode may wait for
    /// readback between calls; its scenario time remains explicitly synthetic.
    pub fn next_opportunity(&mut self) -> Opportunity {
        self.next_with_pacing(false)
    }

    /// Diagnostic cost arm: actively wait for exactly the same opportunity times.
    /// This keeps the render owner runnable to control scheduler/frequency changes;
    /// it burns CPU and is never the default or a display-cadence claim.
    pub fn next_active_opportunity(&mut self) -> Opportunity {
        self.next_with_pacing(true)
    }

    fn next_with_pacing(&mut self, active: bool) -> Opportunity {
        let previous = self.index;
        if self.timebase == Timebase::Elapsed {
            self.index = self.next_target_index(self.origin.elapsed());
        }
        let due = Duration::from_secs_f64(f64::from(self.index) / f64::from(self.hz));
        if active {
            while self.origin.elapsed() < due {
                std::hint::spin_loop();
            }
        } else if let Some(wait) = due.checked_sub(self.origin.elapsed()) {
            std::thread::sleep(wait);
        }
        let requested = self.origin.elapsed();
        let index = self.index;
        self.index = self.index.saturating_add(1);
        Opportunity {
            target: self.origin + due + Duration::from_secs_f64(1.0 / f64::from(self.hz)),
            index,
            skipped_before: index - previous,
            requested,
            scenario_time: if self.timebase == Timebase::Elapsed {
                requested
            } else {
                due
            },
        }
    }

    fn next_target_index(&self, elapsed: Duration) -> u32 {
        // Each target is one period after its request is due. A late request
        // can still use that target while the target itself is in the future.
        let elapsed_periods =
            u32::try_from(elapsed.as_nanos() * u128::from(self.hz) / 1_000_000_000)
                .unwrap_or(u32::MAX);
        self.index.max(elapsed_periods)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_late_request_can_still_use_its_future_target() {
        let mut clock = OpportunityClock::new(Instant::now(), 120, Timebase::Elapsed).unwrap();
        // Index 2 becomes due at 16.667 ms and targets 25 ms.
        clock.index = 2;
        assert_eq!(clock.next_target_index(Duration::from_millis(19)), 2);
        assert_eq!(clock.next_target_index(Duration::from_millis(25)), 3);
        assert_eq!(clock.next_target_index(Duration::from_millis(43)), 5);
    }
}
