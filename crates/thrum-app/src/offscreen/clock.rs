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
    pub(super) target: Instant,
}

impl OpportunityClock {
    /// Start after a primed initial submission at time zero.
    /// # Errors
    /// Refuses zero or implausibly high opportunity rates.
    pub fn new(origin: Instant, hz: u32, timebase: Timebase) -> Result<Self, String> {
        if !(1..=1000).contains(&hz) {
            return Err("capture opportunity rate must be 1..=1000 Hz".to_owned());
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
            let elapsed = u32::try_from(
                self.origin.elapsed().as_nanos() * u128::from(self.hz) / 1_000_000_000,
            )
            .unwrap_or(u32::MAX);
            self.index = self.index.max(elapsed.saturating_add(1));
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
}
