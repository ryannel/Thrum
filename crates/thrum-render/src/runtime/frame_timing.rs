//! Frame-slot ownership and publication of delayed GPU timestamp batches.

use super::{FrameError, FrameHost, FrameSample, FrameSlot, FrameTarget, FrameWorld, SceneSource};
use crate::{
    capabilities::DeviceCapabilities,
    sprite::graph::validate_sprite_timings,
    synthetic::validate_pass_timings,
    timing::{GpuTimings, TimingError, TimingFrame, TimingRefusal, TimingStats},
};

impl<W: FrameWorld, V: SceneSource<W::Page>> FrameHost<W, V> {
    /// GPU timestamp outcomes, including active batches on this device.
    #[must_use]
    pub fn timing_stats(&self) -> TimingStats {
        let mut stats = self.timing_stats;
        stats.pending = self
            .slots
            .iter()
            .chain(&self.retired_slots)
            .filter(|slot| slot.timings.pending_frame().is_some())
            .count();
        stats
    }

    pub(super) fn retire_device_timings(&mut self) {
        for slot in self.slots.iter().chain(&self.retired_slots) {
            if slot.timings.pending_frame().is_some() {
                self.timing_stats.refuse(TimingRefusal::RetiredDevice);
            }
        }
    }
}

impl FrameSlot {
    pub(super) fn discard_timing(
        &mut self,
        generation: u64,
        stats: &mut TimingStats,
    ) -> Result<(), TimingError> {
        if self.timings.discard_recorded(generation)? {
            stats.gathered += 1;
            stats.refuse(TimingRefusal::Discarded);
        }
        Ok(())
    }

    pub(super) fn refuse_timing(
        &mut self,
        sample: &mut FrameSample,
        stats: &mut TimingStats,
        error: TimingError,
    ) {
        let reason = match error {
            TimingError::InvalidTimestamp { reason, .. } => reason,
            TimingError::StaleGeneration => TimingRefusal::RetiredDevice,
            _ => TimingRefusal::Readback,
        };
        self.reject_sample(sample, stats, reason);
    }

    fn reject_sample(
        &mut self,
        sample: &mut FrameSample,
        stats: &mut TimingStats,
        reason: TimingRefusal,
    ) {
        self.timings.clear_frame();
        stats.refuse(reason);
        sample.timing_fault = true;
    }

    pub(super) fn recover_timings(
        &mut self,
        capabilities: &DeviceCapabilities,
        queue: &wgpu::Queue,
    ) -> Result<(), FrameError> {
        self.timings = GpuTimings::new(capabilities, queue, self.timings.capacity())
            .map_err(|error| FrameError::gpu("GPU timing recovery", error))?;
        Ok(())
    }

    pub(super) fn collect_timing(
        &mut self,
        generation: u64,
        sample: &mut FrameSample,
        stats: &mut TimingStats,
    ) -> Result<bool, TimingError> {
        let frame = self.timings.pending_frame();
        let Some(passes) = self.timings.try_collect(generation)? else {
            return Ok(false);
        };
        let presented = sample.surface_presented;
        let durations = match &self.target {
            FrameTarget::ThreeD(_) => self
                .timing_layout
                .and_then(|layout| validate_pass_timings(passes, presented, layout)),
            FrameTarget::TwoD(_) => validate_sprite_timings(passes, presented),
        };
        let Some((present, contribution)) = durations else {
            self.reject_sample(sample, stats, TimingRefusal::Layout);
            return Ok(true);
        };
        if frame
            != Some(TimingFrame {
                device_generation: generation,
                submission_order: sample.submission_order,
            })
        {
            self.reject_sample(sample, stats, TimingRefusal::Layout);
            return Ok(true);
        }
        let timing = self.timing_records.publish(|record| match &self.target {
            FrameTarget::ThreeD(target) => {
                let boundary = target
                    .first_late_pass()
                    .and_then(|name| self.timings.effect_boundary(name));
                self.timings.move_frame_results(record);
                record.effect_passes(target.effect_pass_names());
                record.set_effect_boundary(boundary);
                record.unmeasured(target.untimed_pass_names(self.timing_readback, presented));
            }
            FrameTarget::TwoD(target) => {
                self.timings.move_frame_results(record);
                target.timing_metadata(record, self.timing_readback, presented);
            }
        });
        if let Some(timing) = timing {
            stats.published += 1;
            sample.gpu_pass_ns = Some(timing);
            sample.gpu_present_ns = present;
            sample.gpu_contribution_ns = contribution;
        } else {
            self.reject_sample(sample, stats, TimingRefusal::ReportsHeld);
        }
        Ok(true)
    }
}
