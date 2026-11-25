//! Coordinator admission and reporting for the optional native provider.
use super::{FrameError, FrameHost, FrameSlot, FrameWorld, SceneSource};
use crate::{
    apple_tile::{NativeFrame, NativeOutcome, NativeProviderReport},
    synthetic::NativeWitnessReport,
};

impl<W: FrameWorld, V: SceneSource<W::Page>> FrameHost<W, V> {
    /// Outcomes of the optional native provider on this view.
    #[must_use]
    pub fn native_provider_report(&self) -> Option<NativeProviderReport> {
        self.slots
            .first()
            .and_then(|slot| slot.target.three_d()?.native.as_ref())
            .map(|binding| binding.report())
    }

    /// Diagnostic reset argument passed to MetalFX on its most recent encode.
    /// Returns `None` before the first native encode on the active scaler.
    #[cfg(all(target_os = "macos", feature = "creation-counters"))]
    #[must_use]
    pub fn metalfx_last_encoded_reset(&self) -> Option<bool> {
        let binding = self.slots.first()?.target.three_d()?.native.as_ref()?;
        crate::apple_tile::metalfx::last_encoded_reset(binding)
    }

    /// Outcomes of the optional labelled native copy/select diagnostic.
    #[must_use]
    pub fn native_witness_report(&self) -> Option<NativeWitnessReport> {
        self.slots
            .first()
            .and_then(|slot| slot.target.three_d()?.native_witness_report())
    }

    /// Reject the optional provider at a free frame boundary. Its prepared twin
    /// reuses initialized output/history resources and resets before next use.
    /// Busy providers defer; rejection alone is never a device-loss signal.
    pub fn reject_native_provider(&mut self) -> bool {
        if self.pending_frames() != 0 {
            return false;
        }
        let mut rejected = false;
        for slot in &self.slots {
            if let Some(binding) = slot
                .target
                .three_d()
                .and_then(|target| target.native.as_ref())
            {
                binding.reject();
                rejected = true;
            }
        }
        self.history_reset_pending |= rejected;
        rejected
    }

    /// Labelled fault injection for the native-boundary diagnostic. The next
    /// encoded native segment performs its real writes, then completion reports
    /// an error through the same generation recovery path as a raw Metal error.
    /// Ordinary rendering has no native binding and is unaffected.
    pub fn inject_native_completion_error(&self) {
        if let Some(binding) = self
            .slots
            .first()
            .and_then(|slot| slot.target.three_d()?.native.as_ref())
        {
            binding.inject_completion_error();
        }
    }

    pub(super) fn discard_native_presentation(
        &mut self,
        slot_index: usize,
        capture_slot: Option<usize>,
        native: NativeFrame,
        refused_acquisition: bool,
    ) -> Result<(), FrameError> {
        self.history_reset_pending |= if refused_acquisition {
            !native.refuse_acquisition()
        } else {
            drop(native);
            true
        };
        if let Some(index) = capture_slot
            && let Some(tap) = self.submitted_capture.as_mut()
        {
            tap.abandon(index);
        }
        self.slots[slot_index]
            .discard_timing(self.generation, &mut self.timing_stats)
            .map_err(|error| FrameError::gpu("native presentation discard", error))?;
        self.flush_staged_uploads()
    }

    pub(super) fn check_native_completion(&mut self) -> Result<(), FrameError> {
        if self.native_failed_generation.is_none() {
            self.native_failed_generation = self
                .slots
                .iter()
                .chain(&self.retired_slots)
                .flat_map(FrameSlot::samples)
                .find_map(|sample| {
                    sample.native_segment.as_ref().and_then(|segment| {
                        (segment.outcome() == NativeOutcome::Failed).then_some(segment.generation)
                    })
                });
        }
        if let Some(generation) = self.native_failed_generation {
            self.history_reset_pending = true;
            return Err(FrameError::NativeCommandBuffer { generation });
        }
        Ok(())
    }
}
