//! Feed the clock-free quality policy from the frame owner's lifecycle.

use super::{FrameHost, FrameSample, FrameWorld, SceneSource};
use crate::{
    contribution::QualityKnob,
    governor::{DisplayRecord, FrameCost, Governor, QualityState, SampleMark, TimingSource},
    synthetic::{NativeOutcome, NativeSegment, PreparedSynthetic},
    timing::TimingCategory,
};
use std::{sync::Arc, time::Duration};

/// Quality committed for one encoded frame.
#[derive(Clone, Debug)]
pub struct GovernorFrame {
    /// Scale, effect tier and last change.
    pub state: QualityState,
    /// Committed values of the controls governed at this tier.
    pub knobs: Arc<[QualityKnob]>,
    /// Most recently admitted measurement source.
    pub source: Option<TimingSource>,
    /// Budget calculated from the platform's display interval.
    pub budget: Duration,
    /// Display interval and thermal record used to encode the frame.
    pub display: DisplayRecord,
    /// Frame lifecycle mark, retained with a delayed sample.
    pub mark: SampleMark,
    /// Age in submitted frames of the primary aerial lookup used by this frame.
    pub aerial_age: Option<u32>,
}

pub(super) struct GovernorHost {
    policy: Governor,
    display: DisplayRecord,
    preparing: bool,
    frame_transition: (u64, u64),
    first_frame: bool,
    knobs: Arc<[QualityKnob]>,
    refresh_knobs: bool,
}

#[derive(Clone, Copy)]
pub(super) struct DisplayTick {
    number: u64,
    presented: bool,
    mark: Option<SampleMark>,
}

impl<W: FrameWorld, V: SceneSource<W::Page>> FrameHost<W, V> {
    /// List contribution quality controls with committed values and declared invalidations.
    #[must_use]
    pub fn quality_knobs(&self) -> Vec<QualityKnob> {
        let mut knobs = self
            .contribution
            .as_ref()
            .map_or_else(Vec::new, |entry| entry.quality_knobs());
        knobs.extend_from_slice(self.prepared.lamp_quality());
        if let Some(scene) = self.prepared.three_d() {
            knobs.extend_from_slice(scene.sun_penumbra_quality());
        }
        if self.governor.is_some()
            && let Some(aerial) = self
                .prepared
                .three_d()
                .and_then(PreparedSynthetic::aerial_quality)
        {
            knobs.push(aerial);
        }
        knobs
    }
    /// Stage a quality value for the next frame boundary.
    /// # Errors
    /// Refuses an absent contribution, unknown name or out-of-range value.
    pub fn set_quality_knob(
        &mut self,
        name: &str,
        value: u32,
    ) -> Result<(), crate::contribution::QualityError> {
        if let Some(result) = self.prepared.set_lamp_quality(name, value) {
            return result;
        }
        if let Some(result) = self
            .prepared
            .three_d_mut()
            .and_then(|scene| scene.set_sun_penumbra_quality(name, value))
        {
            return result;
        }
        if self.governor.is_some()
            && let Some(scene) = self.prepared.three_d_mut()
            && let Some(result) = scene.set_aerial_quality(name, value)
        {
            return result;
        }
        self.contribution
            .as_mut()
            .ok_or(crate::contribution::QualityError::NoContribution)?
            .set_quality_knob(name, value)
    }
    /// Enable dynamic quality on a prepared 3D scene with render-scale support.
    /// Returns whether the scene admitted it.
    pub fn enable_governor(&mut self, display: DisplayRecord) -> bool {
        if !self
            .prepared
            .three_d()
            .is_some_and(PreparedSynthetic::supports_render_scale)
        {
            return false;
        }
        self.governor = Some(GovernorHost {
            policy: Governor::new(self.render_scale, display),
            display,
            preparing: false,
            frame_transition: (self.generation, self.presenter_generation),
            first_frame: false,
            knobs: Arc::from([]),
            refresh_knobs: true,
        });
        true
    }

    /// Feed current platform facts and whether an extent candidate is being prepared or deferred.
    pub fn governor_display(&mut self, display: DisplayRecord, preparing: bool) {
        if let Some(governor) = &mut self.governor {
            governor.display = display;
            governor.preparing = preparing;
        }
    }

    /// A new numbered display-link opportunity. Sequence gaps and opportunities with no
    /// present call are counted as a render-side proxy.
    pub fn governor_display_tick(&mut self, now: Duration, number: u64) {
        if self
            .governor_display_tick
            .is_some_and(|tick| tick.number == number)
        {
            return;
        }
        self.governor_boundary(now);
        let mark = self
            .governor
            .as_ref()
            .map(|governor| governor.policy.mark());
        if let Some(previous) = self.governor_display_tick {
            let gap = number.saturating_sub(previous.number).saturating_sub(1);
            let count = gap + u64::from(!previous.presented);
            self.governor_missed_ticks = self.governor_missed_ticks.saturating_add(count);
            // A gap straddling a build has no reliably attributable observations.
            if let Some(governor) = &mut self.governor
                && let Some(mark) = mark
                && previous.mark == Some(mark)
            {
                governor
                    .policy
                    .observe_missed(mark, count, previous.presented);
            }
        }
        self.governor_display_tick = Some(DisplayTick {
            number,
            presented: false,
            mark,
        });
    }

    /// A present call for the current numbered display opportunity.
    pub fn governor_present_called(&mut self) {
        if let Some(tick) = self.governor_display_tick.as_mut() {
            tick.presented = true;
        }
    }

    /// Total observed display-link ticks without a present call, a render-side proxy.
    #[must_use]
    pub fn governor_missed_ticks(&self) -> u64 {
        self.governor_missed_ticks
    }

    /// Feed an elapsed RAF interval when the browser device cannot report GPU timestamps.
    pub fn governor_animation_frame(&mut self, order: u64, interval: Duration) {
        if let Some(governor) = &mut self.governor {
            let mark = governor.policy.mark();
            governor.policy.observe(
                order,
                mark,
                FrameCost {
                    elapsed: interval,
                    scale_dependent: Duration::ZERO,
                    source: TimingSource::AnimationFrame,
                },
            );
        }
    }

    pub(super) fn governor_boundary(&mut self, now: Duration) {
        let Some(governor) = &mut self.governor else {
            return;
        };
        let generation = (self.generation, self.presenter_generation);
        if governor.frame_transition != generation {
            governor.frame_transition = generation;
            governor.first_frame = true;
            governor.refresh_knobs = true;
        }
        governor.policy.boundary(
            now,
            governor.display,
            governor.preparing || governor.first_frame,
        );
    }

    pub(super) fn commit_governor(&mut self, now: Duration) -> Result<(), super::FrameError> {
        let Some(governor) = &mut self.governor else {
            return Ok(());
        };
        let before = governor.policy.state();
        let changed = governor.policy.commit(now);
        let state = governor.policy.state();
        if let Some(tick) = &mut self.governor_display_tick {
            tick.mark = Some(governor.policy.mark());
        }
        self.render_scale = state.scale;
        if governor.policy.mark().held()
            || (!governor.refresh_knobs && (!changed || before.tier == state.tier))
        {
            return Ok(());
        }
        let mut knobs = self.quality_knobs();
        knobs.retain(|knob| knob.governor_steps.is_some());
        for knob in &mut knobs {
            let Some(steps) = knob.governor_steps else {
                continue;
            };
            let value = steps[usize::from(state.tier)];
            if value != knob.value {
                self.set_quality_knob(&knob.name, value)
                    .map_err(|error| super::FrameError::Scene(error.to_string()))?;
                knob.value = value;
            }
        }
        if let Some(governor) = &mut self.governor {
            governor.knobs = knobs.into();
            governor.refresh_knobs = false;
        }
        Ok(())
    }

    /// Current governor values, also attached to encoded frames.
    #[must_use]
    pub fn governor_frame(&self) -> Option<GovernorFrame> {
        let governor = self.governor.as_ref()?;
        Some(GovernorFrame {
            state: governor.policy.state(),
            knobs: Arc::clone(&governor.knobs),
            source: governor.policy.source(),
            budget: governor.policy.budget(),
            display: governor.display,
            mark: governor.policy.mark(),
            aerial_age: self
                .prepared
                .three_d()
                .and_then(PreparedSynthetic::aerial_age),
        })
    }
}

impl GovernorHost {
    pub(super) fn submitted(&mut self) {
        self.first_frame = false;
    }

    pub(super) fn refresh(&mut self) {
        self.refresh_knobs = true;
    }

    pub(super) fn observe(&mut self, sample: &FrameSample) {
        let Some(frame) = &sample.governor else {
            return;
        };
        let gpu = sample.gpu_pass_ns.as_ref().and_then(|timing| {
            let segment = match sample.native_segment.as_ref().map(NativeSegment::outcome) {
                Some(NativeOutcome::Timed([start, end])) => (end - start) * 1e9,
                Some(NativeOutcome::Pending | NativeOutcome::Failed) => return None,
                _ => 0.0,
            };
            let measured = timing.sum() + sample.gpu_present_ns.unwrap_or_default();
            // Passes can overlap. Use the timed output interval when available,
            // with a floor from passes that can outlast the output consumer.
            let span = timing
                .timed_pass_span_nanoseconds()
                .map_or(measured, |span| {
                    timing
                        .passes()
                        .iter()
                        .fold(span, |span, pass| span.max(pass.nanoseconds))
                });
            let scale_work =
                timing.bucket(TimingCategory::Forward) + timing.bucket(TimingCategory::Effects);
            // Discount the scale-dependent share by the same overlap. Gaps
            // and the separately timed native segment stay fixed on recovery.
            let scale_work = if span < measured {
                scale_work * (span / measured)
            } else {
                scale_work
            };
            let ns = |value| Duration::try_from_secs_f64(value * 1e-9).ok();
            let elapsed = ns(span + segment)?;
            // The native owner records this frame before kicking its GPU
            // work. Both costs use the frame's delivery headroom.
            #[cfg(not(target_arch = "wasm32"))]
            let elapsed = elapsed.saturating_add(sample.render_cpu);
            let scale_dependent = ns(scale_work)?;
            Some(FrameCost {
                elapsed,
                scale_dependent,
                source: TimingSource::Gpu,
            })
        });
        #[cfg(not(target_arch = "wasm32"))]
        let gpu = gpu.or(Some(FrameCost {
            elapsed: sample.render_cpu,
            scale_dependent: Duration::ZERO,
            source: TimingSource::RenderCpu,
        }));
        if let Some(cost) = gpu {
            self.policy
                .observe(sample.submission_order, frame.mark, cost);
        }
    }
}
