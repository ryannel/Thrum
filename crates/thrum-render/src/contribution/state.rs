//! Submission-owned effect clock and independently committed ping-pong histories.

use std::time::Duration;

/// Committed world endpoint sampled by an effect.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EffectTime {
    /// World epoch and replay branch are both discontinuities.
    pub epoch: u128,
    /// Replay branch identity.
    pub timeline: u64,
    /// Complete source tick, never a display-frame counter.
    pub tick: u64,
    /// Authoritative fixed timestep.
    pub tick_duration: Duration,
    /// Presentation interpolation; never advances simulation.
    pub alpha: f32,
    /// Read-only capture; cannot advance or reset state.
    pub capture: bool,
}

/// Work selected from the design's state table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectStep {
    /// Keep committed contents exactly, including at a repeated tick or pause.
    Keep,
    /// Seed every texel without reading previous state.
    Reset,
    /// Execute once, integrating this many ticks and accounting for omitted work.
    Advance {
        /// Bounded ticks to integrate.
        ticks: u64,
        /// Older ticks omitted by the effect's declared bound.
        dropped: u64,
    },
}

/// The three bindings of an independently committed history pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryBinding {
    /// The most recently submitted side.
    Previous,
    /// The alternate side; only its producer may write it.
    Next,
    /// Alternate when the producer ran, otherwise the committed side.
    Result,
}

/// Tentative clock, wrapping anchor and history sides for one effect.
///
/// `begin` replaces the preceding tentative recording. Only `submitted` changes
/// committed data. A generation or descriptor change is represented by a new epoch.
#[derive(Debug)]
pub struct EffectState {
    generation: u64,
    resource_epoch: u64,
    committed: Option<EffectTime>,
    anchor: [i64; 2],
    sides: Vec<usize>,
    ran: Vec<bool>,
    pending: Option<(EffectTime, [i64; 2])>,
    step: EffectStep,
    dropped: u64,
}

impl EffectState {
    /// Allocate the bookkeeping once for a fixed number of history pairs.
    pub fn new(histories: usize) -> Self {
        Self {
            generation: 0,
            resource_epoch: 0,
            committed: None,
            anchor: [0; 2],
            sides: vec![0; histories],
            ran: vec![false; histories],
            pending: None,
            step: EffectStep::Keep,
            dropped: 0,
        }
    }

    /// Keep state on same-generation resize; restart on device or descriptor epoch change.
    pub fn rebuild(&mut self, generation: u64, resource_epoch: u64) {
        if self.generation != generation || self.resource_epoch != resource_epoch {
            self.generation = generation;
            self.resource_epoch = resource_epoch;
            self.committed = None;
            self.sides.fill(0);
            self.ran.fill(false);
            self.pending = None;
        }
    }

    /// Start a tentative recording using the committed endpoint, never the last recording.
    pub fn begin(&mut self, time: EffectTime, bound: u64, anchor: [i64; 2]) -> EffectStep {
        self.ran.fill(false);
        self.step = if time.capture {
            EffectStep::Keep
        } else {
            match self.committed {
                None => EffectStep::Reset,
                Some(old)
                    if old.epoch != time.epoch
                        || old.timeline != time.timeline
                        || time.tick < old.tick =>
                {
                    EffectStep::Reset
                }
                Some(old) if time.tick == old.tick => EffectStep::Keep,
                Some(old) => {
                    let elapsed = time.tick - old.tick;
                    EffectStep::Advance {
                        ticks: elapsed.min(bound),
                        dropped: elapsed.saturating_sub(bound),
                    }
                }
            }
        };
        self.pending = (!time.capture).then_some((time, anchor));
        self.step
    }

    /// Mark a whole-output producer as recorded. A capture never marks a producer.
    pub fn produced(&mut self, history: usize) {
        if self.pending.is_some() {
            self.ran[history] = true;
        }
    }

    /// Resolve one pair without allocating or creating bindings in the frame.
    pub fn side(&self, history: usize, binding: HistoryBinding) -> usize {
        let previous = self.sides[history];
        match binding {
            HistoryBinding::Previous => previous,
            HistoryBinding::Next => 1 - previous,
            HistoryBinding::Result => {
                if self.ran[history] {
                    1 - previous
                } else {
                    previous
                }
            }
        }
    }

    /// Commit only after the encoder is submitted. A discarded recording calls this zero times.
    pub fn submitted(&mut self) {
        if let Some((time, anchor)) = self.pending.take() {
            self.committed = Some(time);
            self.anchor = anchor;
            if let EffectStep::Advance { dropped, .. } = self.step {
                self.dropped = self.dropped.saturating_add(dropped);
            }
            for (side, ran) in self.sides.iter_mut().zip(&self.ran) {
                if *ran {
                    *side = 1 - *side;
                }
            }
        }
        self.ran.fill(false);
    }

    /// Last submitted endpoint, absent before the first full frame.
    pub fn committed(&self) -> Option<EffectTime> {
        self.committed
    }
    /// Last submitted integer world anchor.
    pub fn anchor(&self) -> [i64; 2] {
        self.anchor
    }
    /// Ticks omitted by bounded advances of submitted frames.
    pub fn dropped_ticks(&self) -> u64 {
        self.dropped
    }
}
