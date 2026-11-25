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

    /// Read the native normalization repair totals after a diagnostic run.
    /// The three channels count high clamps, low clamps and non-finite repairs.
    /// # Errors
    /// Returns a diagnostic readback or device wait error.
    #[cfg(all(feature = "creation-counters", not(target_arch = "wasm32")))]
    pub fn metalfx_repairs_for_probe(&self) -> Result<Option<[u64; 3]>, FrameError> {
        use crate::records::MetalFxRepairs;

        let buffers: Vec<_> = self
            .slots
            .iter()
            .filter_map(|slot| slot.target.three_d()?.metalfx_repairs())
            .collect();
        if buffers.is_empty() {
            return Ok(None);
        }
        let stride = size_of::<MetalFxRepairs>() as u64;
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("MetalFX repair totals"),
            size: stride * buffers.len() as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        for (index, source) in buffers.iter().enumerate() {
            encoder.copy_buffer_to_buffer(source, 0, &readback, stride * index as u64, stride);
        }
        let submission = self.queue.submit([encoder.finish()]);
        let slice = readback.slice(..);
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        slice.map_async(wgpu::MapMode::Read, move |result| {
            drop(sender.send(result));
        });
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(std::time::Duration::from_secs(10)),
            })
            .map_err(|error| FrameError::gpu("MetalFX repair totals", error))?;
        receiver
            .try_recv()
            .ok()
            .and_then(Result::ok)
            .ok_or(FrameError::Render(crate::error::RenderError::Readback))?;
        let mapped = slice
            .get_mapped_range()
            .map_err(|_| FrameError::Render(crate::error::RenderError::Readback))?;
        let mut total = [0_u64; 3];
        for bytes in mapped.chunks_exact(size_of::<MetalFxRepairs>()) {
            let repairs = bytemuck::pod_read_unaligned::<MetalFxRepairs>(bytes);
            for (total, count) in
                total
                    .iter_mut()
                    .zip([repairs.high, repairs.low, repairs.nonfinite])
            {
                *total += u64::from(count);
            }
        }
        Ok(Some(total))
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
