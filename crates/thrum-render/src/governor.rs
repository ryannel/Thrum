//! Frame-boundary quality policy. All time and lifecycle observations come from the host.

use std::time::Duration;

use crate::synthetic::RenderScale;

const MINIMUM_PERIOD: Duration = Duration::from_secs(1);
const PANIC_FRAMES: u8 = 3;
const RECOVERY_FRAMES: u32 = 12;

/// Platform thermal pressure, independent of application or simulation time.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ThermalState {
    /// No platform pressure.
    #[default]
    Nominal,
    /// The platform advises reducing work.
    Fair,
    /// The platform reports high thermal pressure.
    Serious,
    /// The platform reports critical thermal pressure.
    Critical,
}

impl ThermalState {
    const fn minimum_tier(self) -> u8 {
        match self {
            Self::Nominal => 0,
            Self::Fair => 1,
            Self::Serious | Self::Critical => 2,
        }
    }
}

/// Display cadence and thermal pressure sampled by the platform.
#[derive(Clone, Copy, Debug)]
pub struct DisplayRecord {
    /// Nominal interval of the current display cadence.
    pub interval: Duration,
    /// Current platform thermal state.
    pub thermal: ThermalState,
}

/// The measurement that supplied a policy observation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimingSource {
    /// Resolved pass timestamps and native segments.
    Gpu,
    /// Render-owner CPU duration on a platform without GPU timestamps.
    RenderCpu,
    /// Elapsed browser animation-frame interval.
    AnimationFrame,
    /// Display link ticks without a call to present, a render-side proxy.
    DisplayLink,
}

/// Resolved time and the part expected to grow with the drawn pixel area.
#[derive(Clone, Copy, Debug)]
pub struct FrameCost {
    /// Total time measured for the observation.
    pub elapsed: Duration,
    /// Time spent in passes that can grow with the render subrectangle.
    pub scale_dependent: Duration,
    /// Measurement used for the observation.
    pub source: TimingSource,
}

/// Why committed quality last changed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChangeReason {
    /// Initial recipe quality.
    Initial,
    /// Three consecutive observations exceeded the budget.
    OverBudget,
    /// The platform reported thermal pressure.
    Thermal,
    /// Sustained headroom permitted one quality step up.
    Recovery,
}

/// The policy's committed values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QualityState {
    /// Prepared render scale.
    pub scale: RenderScale,
    /// Effect tier, where zero is the recipe's highest tier.
    pub tier: u8,
    /// Reason for the last value change.
    pub reason: ChangeReason,
}

/// A mark carried by a frame until its asynchronous sample resolves.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SampleMark {
    revision: u64,
    held: bool,
}

impl SampleMark {
    /// This frame was encoded during a build or as its first new-generation frame.
    #[must_use]
    pub const fn held(self) -> bool {
        self.held
    }
}

/// Governor over timestamped observations and generation hold marks.
pub struct Governor {
    scale: usize,
    tier: u8,
    reason: ChangeReason,
    recovery_since: Duration,
    revision: u64,
    held: bool,
    last_order: Option<u64>,
    over: u8,
    headroom: u16,
    missed: u8,
    source: Option<TimingSource>,
    display: DisplayRecord,
}

impl Governor {
    /// Start at one of the recipe's prepared render scales.
    #[must_use]
    pub fn new(initial: RenderScale, display: DisplayRecord) -> Self {
        Self {
            scale: initial as usize,
            tier: 0,
            reason: ChangeReason::Initial,
            recovery_since: Duration::ZERO,
            revision: 0,
            held: false,
            last_order: None,
            over: 0,
            headroom: 0,
            missed: 0,
            source: None,
            display,
        }
    }

    /// Current display budget for measured work.
    #[must_use]
    pub fn budget(&self) -> Duration {
        self.display.interval.mul_f64(0.85)
    }

    /// Committed choices.
    #[must_use]
    pub fn state(&self) -> QualityState {
        QualityState {
            scale: RenderScale::STEPS[self.scale],
            tier: self.tier,
            reason: self.reason,
        }
    }

    /// Last admitted measurement source.
    #[must_use]
    pub const fn source(&self) -> Option<TimingSource> {
        self.source
    }

    /// Mark an encoded frame or a display observation with the current policy revision.
    #[must_use]
    pub const fn mark(&self) -> SampleMark {
        SampleMark {
            revision: self.revision,
            held: self.held,
        }
    }

    /// Update platform facts and the generation boundary before accepting observations.
    /// The first frame after a switch is marked held by the host.
    pub fn boundary(&mut self, now: Duration, display: DisplayRecord, held: bool) {
        self.display = display;
        if self.held != held {
            self.held = held;
            self.clear_window(now);
        }
    }

    /// Admit one resolved frame or animation interval from the current policy revision.
    pub fn observe(&mut self, order: u64, mark: SampleMark, cost: FrameCost) {
        if !self.admits(mark) || self.last_order.is_some_and(|last| order <= last) {
            return;
        }
        if self.last_order.is_some_and(|last| order != last + 1) {
            self.over = 0;
            self.headroom = 0;
        }
        self.last_order = Some(order);
        self.source = Some(cost.source);
        let interval = self.display.interval;
        let budget = self.budget();
        // A normally paced RAF is one full display interval, including idle.
        let (over, under) = if cost.source == TimingSource::AnimationFrame {
            (
                cost.elapsed > interval.mul_f64(1.25),
                cost.elapsed <= interval.mul_f64(1.10),
            )
        } else {
            let mut projected = cost.elapsed;
            if self.tier <= self.display.thermal.minimum_tier() && self.scale > 0 {
                // Only passes sized to the subrectangle can spend its additional pixels.
                let current = f64::from(RenderScale::STEPS[self.scale].percent());
                let next = f64::from(RenderScale::STEPS[self.scale - 1].percent());
                projected += cost
                    .scale_dependent
                    .min(cost.elapsed)
                    .mul_f64((next / current).powi(2) - 1.0);
            }
            (cost.elapsed > budget, projected <= budget.mul_f64(0.93))
        };
        self.over = if over { self.over.saturating_add(1) } else { 0 };
        self.headroom = (self.headroom << 1) | u16::from(under);
    }

    /// Admit a count of consecutive display-link ticks without a present.
    /// Zero means the preceding opportunity called present.
    pub fn observe_missed(&mut self, mark: SampleMark, count: u64, presented: bool) {
        if self.admits(mark) {
            if presented {
                self.missed = 0;
            }
            self.missed = self
                .missed
                .saturating_add(u8::try_from(count).unwrap_or(u8::MAX));
            if count != 0 {
                self.headroom <<= 1;
            }
        }
    }

    pub(crate) fn interrupt_display_observations(&mut self, mark: SampleMark) {
        if self.admits(mark) {
            self.missed = 0;
            self.headroom = 0;
        }
    }

    /// Commit at most one prepared step for the coming frame.
    pub fn commit(&mut self, now: Duration) -> bool {
        if self.held || self.display.interval.is_zero() {
            return false;
        }
        let thermal = self.display.thermal.minimum_tier();
        let pressure = self.over >= PANIC_FRAMES || self.missed >= PANIC_FRAMES;
        let reason = if self.tier < thermal {
            self.tier += 1;
            ChangeReason::Thermal
        } else if pressure && self.scale + 1 < RenderScale::STEPS.len() {
            self.scale += 1;
            if self.over < PANIC_FRAMES {
                self.source = Some(TimingSource::DisplayLink);
            }
            ChangeReason::OverBudget
        } else if pressure && self.tier < 2 {
            self.tier += 1;
            if self.over < PANIC_FRAMES {
                self.source = Some(TimingSource::DisplayLink);
            }
            ChangeReason::OverBudget
        } else if self.headroom.count_ones() >= RECOVERY_FRAMES
            && self.missed == 0
            && now.saturating_sub(self.recovery_since) >= MINIMUM_PERIOD
        {
            if self.tier > thermal {
                self.tier -= 1;
            } else if self.scale > 0 {
                self.scale -= 1;
            } else {
                return false;
            }
            ChangeReason::Recovery
        } else {
            return false;
        };
        self.reason = reason;
        self.clear_window(now);
        true
    }

    fn admits(&self, mark: SampleMark) -> bool {
        !self.held
            && !mark.held
            && mark.revision == self.revision
            && !self.display.interval.is_zero()
    }

    fn clear_window(&mut self, now: Duration) {
        self.revision = self.revision.wrapping_add(1);
        self.over = 0;
        self.headroom = 0;
        self.last_order = None;
        self.missed = 0;
        self.recovery_since = now;
    }
}
