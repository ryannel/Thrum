//! Bounded fixed-step clock. The platform supplies monotonic time and a CPU
//! admission count; this module never sleeps or uses a display callback as time.

use std::{num::NonZeroU32, time::Duration};

const NANOS_PER_SECOND: u128 = 1_000_000_000;

/// Fixed-step rate and finite work limits for one authoritative world.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClockConfig {
    /// World updates per second, independent of presented frames.
    pub tick_hz: NonZeroU32,
    /// Safety limit on updates attempted during one host call.
    pub max_ticks_per_call: NonZeroU32,
    /// Maximum due updates retained before explicit overload.
    pub max_queued_ticks: NonZeroU32,
}

impl Default for ClockConfig {
    fn default() -> Self {
        Self {
            tick_hz: NonZeroU32::new(60).unwrap(),
            max_ticks_per_call: NonZeroU32::new(4).unwrap(),
            max_queued_ticks: NonZeroU32::new(8).unwrap(),
        }
    }
}

/// Local clock state; online authorities need their own overload policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClockState {
    /// Monotonic elapsed time accrues world updates.
    Active,
    /// Hidden/background world time is stopped.
    Paused,
    /// Due work exceeded admission; time is stopped until backlog drains and
    /// the host explicitly resumes from a fresh wall-time origin.
    Overloaded,
}

/// Invalid clock operation. Rejection leaves clock state unchanged.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClockError {
    /// A safety limit exceeds the finite queue.
    InvalidConfig,
    /// A running clock cannot reset its wall-time origin through resume.
    AlreadyActive,
    /// Platform timestamp moved backwards.
    NonMonotonicTime,
    /// A timestamp/rate product cannot be represented.
    TimeOverflow,
    /// Pending updates must complete before a new wall-time origin begins.
    PendingTicks,
}

/// Account of one host call. A retryable `step_error` retains the failed tick;
/// the host must terminate on a game-contract fault.
#[derive(Debug)]
pub struct ClockReport<E> {
    /// State after the call.
    pub state: ClockState,
    /// Number of successful world updates.
    pub completed: u32,
    /// Due updates still waiting for service.
    pub queued: u32,
    /// New due updates admitted from elapsed active time.
    pub admitted: u32,
    /// Due updates refused when the clock entered overload. This is an
    /// explicit stopped-time event, never a silent skip or stretched step.
    pub refused_on_overload: u64,
    /// Error from the first attempted update that did not commit.
    pub step_error: Option<E>,
}

/// One world's monotonic, fixed-step time account. No per-call heap storage.
pub struct TickClock {
    config: ClockConfig,
    state: ClockState,
    last_time: Duration,
    fractional_tick: u128,
    queued: u32,
}

impl TickClock {
    /// Begin at a host-supplied monotonic timestamp.
    pub fn new(config: ClockConfig, start: Duration) -> Result<Self, ClockError> {
        if config.max_ticks_per_call > config.max_queued_ticks {
            return Err(ClockError::InvalidConfig);
        }
        Ok(Self {
            config,
            state: ClockState::Active,
            last_time: start,
            fractional_tick: 0,
            queued: 0,
        })
    }

    /// Current clock state.
    pub fn state(&self) -> ClockState {
        self.state
    }

    /// Due updates retained for service.
    pub fn queued_ticks(&self) -> u32 {
        self.queued
    }

    /// Delay from the last host callback until the next world update is due.
    /// Pending work is ready now; a stopped clock with no pending work has no
    /// deadline. The host may choose a later bounded retry for backpressure.
    pub fn next_due_in(&self) -> Option<Duration> {
        if self.queued != 0 {
            return Some(Duration::ZERO);
        }
        if self.state != ClockState::Active {
            return None;
        }
        let remaining = NANOS_PER_SECOND - self.fractional_tick;
        let hz = u128::from(self.config.tick_hz.get());
        let nanos = remaining.div_ceil(hz);
        Some(Duration::from_nanos(nanos as u64))
    }

    pub(crate) fn report_cpu_overload(&mut self) {
        self.state = ClockState::Overloaded;
        self.fractional_tick = 0;
    }

    /// Stop advancing world time immediately. Already admitted updates remain.
    pub fn pause(&mut self) {
        self.state = ClockState::Paused;
        self.fractional_tick = 0;
    }

    /// Restart from a new wall-time origin after prior work is complete.
    pub fn resume(&mut self, now: Duration) -> Result<(), ClockError> {
        if self.state == ClockState::Active {
            return Err(ClockError::AlreadyActive);
        }
        if now < self.last_time {
            return Err(ClockError::NonMonotonicTime);
        }
        if self.queued != 0 {
            return Err(ClockError::PendingTicks);
        }
        self.last_time = now;
        self.fractional_tick = 0;
        self.state = ClockState::Active;
        Ok(())
    }

    /// Admit elapsed ticks, then attempt at most the configured and host CPU
    /// budgets. The host must derive `cpu_tick_budget` from its measured or
    /// bounded per-tick cost and callback reserve. A retryable failed `step`
    /// must leave its authoritative world unchanged, as `Authority::step` does
    /// on backpressure. A terminal error must stop the host. This method does
    /// not preempt a slow callback.
    pub fn advance<E>(
        &mut self,
        now: Duration,
        cpu_tick_budget: u32,
        mut step: impl FnMut() -> Result<(), E>,
    ) -> Result<ClockReport<E>, ClockError> {
        self.advance_controlled(now, cpu_tick_budget, || step().map(|()| true))
    }

    /// `Ok(false)` means the tick committed but the host must stop this call,
    /// for example after a terminal post-commit consumer fault.
    pub(crate) fn advance_controlled<E>(
        &mut self,
        now: Duration,
        cpu_tick_budget: u32,
        mut step: impl FnMut() -> Result<bool, E>,
    ) -> Result<ClockReport<E>, ClockError> {
        if now < self.last_time {
            return Err(ClockError::NonMonotonicTime);
        }
        let mut admitted = 0;
        let mut refused_on_overload = 0;
        if self.state == ClockState::Active {
            let elapsed = (now - self.last_time).as_nanos();
            let numerator = elapsed
                .checked_mul(u128::from(self.config.tick_hz.get()))
                .and_then(|value| value.checked_add(self.fractional_tick))
                .ok_or(ClockError::TimeOverflow)?;
            let due = numerator / NANOS_PER_SECOND;
            let due_u64 = u64::try_from(due).map_err(|_| ClockError::TimeOverflow)?;
            if due > u128::from(self.config.max_queued_ticks.get() - self.queued) {
                self.state = ClockState::Overloaded;
                self.fractional_tick = 0;
                refused_on_overload = due_u64;
            } else {
                admitted = u32::try_from(due).expect("bounded by queue");
                self.queued += admitted;
                self.fractional_tick = numerator % NANOS_PER_SECOND;
            }
            self.last_time = now;
        } else {
            self.last_time = now;
        }

        let mut completed = 0;
        let mut step_error = None;
        let allowed = self
            .queued
            .min(self.config.max_ticks_per_call.get())
            .min(cpu_tick_budget);
        for _ in 0..allowed {
            match step() {
                Ok(keep_running) => {
                    self.queued -= 1;
                    completed += 1;
                    if !keep_running {
                        break;
                    }
                }
                Err(error) => {
                    step_error = Some(error);
                    break;
                }
            }
        }
        Ok(ClockReport {
            state: self.state,
            completed,
            queued: self.queued,
            admitted,
            refused_on_overload,
            step_error,
        })
    }
}
