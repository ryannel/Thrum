//! Frame-slot ownership and publication of delayed GPU timestamp batches.

use super::{
    FrameHost, FrameSample, FrameSlot, FrameTarget, FrameTimingBatch, FrameWorld, SceneSource,
    SlotState, retain_sample,
};
use crate::{
    sprite::graph::validate_sprite_timings,
    synthetic::validate_pass_timings,
    timing::{FrameCounters, TimingError, TimingFrame, TimingRecords, TimingRefusal, TimingStats},
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
            .flat_map(FrameSlot::pending_timing_batches)
            .count();
        stats
    }

    pub(super) fn retire_device_timings(&mut self) {
        for _ in self
            .slots
            .iter()
            .chain(&self.retired_slots)
            .flat_map(FrameSlot::pending_timing_batches)
        {
            self.timing_stats.refuse(TimingRefusal::RetiredDevice);
        }
        self.frame_counters.reset();
    }

    pub(super) fn track_frame_counters(&mut self, slot_index: usize, order: u64) {
        let mut oldest_floor: Option<u64> = None;
        let mut pending_capacity = 0;
        for (frame, batch) in self
            .slots
            .iter()
            .chain(&self.retired_slots)
            .flat_map(FrameSlot::pending_timing_batches)
        {
            pending_capacity += batch.queries.capacity() as usize;
            if frame.submission_order != order {
                oldest_floor = Some(
                    oldest_floor
                        .map_or(batch.counter_floor, |floor| floor.min(batch.counter_floor)),
                );
            }
        }
        self.slots[slot_index].timing.counter_floor =
            self.frame_counters
                .submit(order, oldest_floor, pending_capacity);
    }

    pub(super) fn collect_frame_timings(&mut self) {
        // The queue submits frames in order. Callback delivery and slot order
        // can differ. Validate the earlier batch first so that if a pair
        // appears twice, it is the later submission that refuses it.
        while let Some((_, deferred, slot)) = self
            .slots
            .iter_mut()
            .chain(&mut self.retired_slots)
            .filter_map(|slot| {
                let (order, deferred) = slot.oldest_timing()?;
                Some((order, deferred, slot))
            })
            .min_by_key(|(order, _, _)| *order)
        {
            let (batch, sample) = if deferred {
                let deferred = slot.deferred_timing.as_mut().expect("deferred timing");
                (
                    &mut deferred.batch,
                    deferred.sample.as_mut().expect("deferred sample"),
                )
            } else {
                let SlotState::Resolving(sample) = &mut slot.state else {
                    // The oldest render has not completed yet.
                    break;
                };
                sample.camera_culling = sample
                    .camera_culling
                    .or_else(|| slot.target.take_culling_counts_if_3d());
                (&mut slot.timing, sample)
            };
            if !batch.collect_or_refuse(
                &self.device,
                self.generation,
                &slot.target,
                &mut slot.timing_records,
                sample,
                &mut self.timing_stats,
                &mut self.frame_counters,
            ) {
                break;
            }
            let sample = if deferred {
                slot.deferred_timing
                    .as_mut()
                    .and_then(|timing| timing.sample.take())
                    .expect("collected deferred sample")
            } else {
                let SlotState::Resolving(sample) =
                    std::mem::replace(&mut slot.state, SlotState::Free)
                else {
                    unreachable!();
                };
                sample
            };
            if let Some(governor) = &mut self.governor {
                governor.observe(&sample);
            }
            retain_sample(&mut self.completed, &mut self.dropped_reports, sample);
        }
    }
}

impl FrameSlot {
    fn pending_timing_batches(&self) -> impl Iterator<Item = (TimingFrame, &FrameTimingBatch)> {
        std::iter::once(&self.timing)
            .chain(
                self.deferred_timing
                    .as_ref()
                    .map(|deferred| &deferred.batch),
            )
            .filter_map(|batch| Some((batch.queries.pending_frame()?, batch)))
    }

    fn oldest_timing(&self) -> Option<(u64, bool)> {
        let active = self
            .timing
            .queries
            .pending_frame()
            .map(|frame| (frame.submission_order, false));
        let deferred = self
            .deferred_sample()
            .map(|sample| (sample.submission_order, true));
        active.into_iter().chain(deferred).min()
    }

    // Render and native completion are already known. Rotate the prepared
    // readback batches so mapping can finish independently of target reuse.
    pub(super) fn defer_timing(&mut self, sample: FrameSample) {
        if let Some(deferred) = &mut self.deferred_timing
            && deferred.sample.is_none()
        {
            self.timing
                .queries
                .swap_readback(&mut deferred.batch.queries);
            std::mem::swap(&mut self.timing.layout, &mut deferred.batch.layout);
            std::mem::swap(&mut self.timing.readback, &mut deferred.batch.readback);
            std::mem::swap(
                &mut self.timing.counter_floor,
                &mut deferred.batch.counter_floor,
            );
            deferred.sample = Some(sample);
        } else {
            self.state = SlotState::Resolving(sample);
        }
    }

    pub(super) fn deferred_sample(&self) -> Option<&FrameSample> {
        self.deferred_timing.as_ref()?.sample.as_ref()
    }

    pub(super) fn active_sample(&self) -> Option<&FrameSample> {
        match &self.state {
            SlotState::Rendering(sample) | SlotState::Resolving(sample) => Some(sample),
            SlotState::Free => None,
        }
    }

    pub(super) fn samples(&self) -> impl Iterator<Item = &FrameSample> {
        self.active_sample()
            .into_iter()
            .chain(self.deferred_sample())
    }

    pub(super) fn is_pending(&self) -> bool {
        self.active_sample().is_some() || self.deferred_sample().is_some()
    }

    pub(super) fn discard_timing(
        &mut self,
        generation: u64,
        stats: &mut TimingStats,
    ) -> Result<(), TimingError> {
        if self.timing.queries.discard_recorded(generation)? {
            stats.gathered += 1;
            stats.refuse(TimingRefusal::Discarded);
        }
        Ok(())
    }
}

impl FrameTimingBatch {
    #[expect(
        clippy::too_many_arguments,
        reason = "publication uses the slot metadata and host counter history"
    )]
    fn collect_or_refuse(
        &mut self,
        device: &wgpu::Device,
        generation: u64,
        target: &FrameTarget,
        records: &mut TimingRecords,
        sample: &mut FrameSample,
        stats: &mut TimingStats,
        counters: &mut FrameCounters,
    ) -> bool {
        match self.collect_timing(generation, target, records, sample, stats, counters) {
            Ok(ready) => ready,
            Err(error) => {
                self.refuse_timing(sample, stats, error);
                if !matches!(error, TimingError::InvalidTimestamp { .. }) {
                    self.queries.recover_readback(device);
                }
                true
            }
        }
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
        self.queries.clear_frame();
        stats.refuse(reason);
        sample.timing_fault = true;
    }

    fn collect_timing(
        &mut self,
        generation: u64,
        target: &FrameTarget,
        records: &mut TimingRecords,
        sample: &mut FrameSample,
        stats: &mut TimingStats,
        counters: &mut FrameCounters,
    ) -> Result<bool, TimingError> {
        let frame = self.queries.pending_frame();
        let Some(passes) = self.queries.collect_in_submission_order(
            generation,
            counters,
            self.counter_floor,
            sample.submission_order,
        )?
        else {
            return Ok(false);
        };
        let presented = sample.surface_presented;
        let durations = match target {
            FrameTarget::ThreeD(_) => self
                .layout
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
        let timing = records.publish(|record| match target {
            FrameTarget::ThreeD(target) => {
                let boundary = target
                    .first_late_pass()
                    .and_then(|name| self.queries.effect_boundary(name));
                self.queries.move_frame_results(record);
                record.effect_passes(target.effect_pass_names());
                record.set_effect_boundary(boundary);
                record.unmeasured(target.untimed_pass_names(self.readback, presented));
            }
            FrameTarget::TwoD(target) => {
                self.queries.move_frame_results(record);
                target.timing_metadata(record, self.readback, presented);
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
