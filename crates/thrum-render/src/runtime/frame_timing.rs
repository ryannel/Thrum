//! Frame-slot ownership and publication of delayed GPU timestamp batches.

use super::{
    FrameError, FrameHost, FrameSample, FrameSlot, FrameTarget, FrameWorld, SceneSource, SlotState,
    retain_sample,
};
use crate::{
    capabilities::DeviceCapabilities,
    sprite::graph::validate_sprite_timings,
    synthetic::validate_pass_timings,
    timing::{FrameCounters, GpuTimings, TimingError, TimingFrame, TimingRefusal, TimingStats},
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
        self.frame_counters.reset();
    }

    pub(super) fn track_frame_counters(&mut self, slot_index: usize, order: u64) {
        let mut oldest_floor: Option<u64> = None;
        let mut pending_capacity = 0;
        for slot in self.slots.iter().chain(&self.retired_slots) {
            let Some(frame) = slot.timings.pending_frame() else {
                continue;
            };
            pending_capacity += slot.timings.capacity() as usize;
            if frame.submission_order != order {
                oldest_floor = Some(oldest_floor.map_or(slot.timing_counter_floor, |floor| {
                    floor.min(slot.timing_counter_floor)
                }));
            }
        }
        self.slots[slot_index].timing_counter_floor =
            self.frame_counters
                .submit(order, oldest_floor, pending_capacity);
    }

    pub(super) fn collect_frame_timings(&mut self) -> Result<(), FrameError> {
        // The queue submits frames in order. Callback delivery and slot order
        // can differ. Validate the earlier batch first so that if a pair
        // appears twice, it is the later submission that refuses it.
        while let Some((_, slot)) = self
            .slots
            .iter_mut()
            .chain(&mut self.retired_slots)
            .filter_map(|slot| Some((slot.timings.pending_frame()?.submission_order, slot)))
            .min_by_key(|(order, _)| *order)
        {
            let state = std::mem::replace(&mut slot.state, SlotState::Free);
            let SlotState::Resolving(mut sample) = state else {
                // An older frame is still waiting for render completion.
                slot.state = state;
                break;
            };
            sample.camera_culling = sample
                .camera_culling
                .or_else(|| slot.target.take_culling_counts_if_3d());
            match slot.collect_timing(
                self.generation,
                &mut sample,
                &mut self.timing_stats,
                &mut self.frame_counters,
            ) {
                Ok(false) => {
                    slot.state = SlotState::Resolving(sample);
                    break;
                }
                Ok(true) => {}
                Err(error) => {
                    slot.refuse_timing(&mut sample, &mut self.timing_stats, error);
                    if !matches!(error, TimingError::InvalidTimestamp { .. }) {
                        slot.recover_timings(self.prepared.capabilities(), &self.queue)?;
                    }
                }
            }
            if let Some(governor) = &mut self.governor {
                governor.observe(&sample);
            }
            retain_sample(&mut self.completed, &mut self.dropped_reports, sample);
        }
        Ok(())
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
        counters: &mut FrameCounters,
    ) -> Result<bool, TimingError> {
        let frame = self.timings.pending_frame();
        let Some(passes) = self.timings.collect_in_submission_order(
            generation,
            counters,
            self.timing_counter_floor,
            sample.submission_order,
        )?
        else {
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
