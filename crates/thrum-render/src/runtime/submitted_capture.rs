//! Optional bounded readback of ordinary submitted frames.

use super::Instant;
use super::{FrameError, FrameHost, FrameSample, FrameWorld, SceneSource};
use crate::extract::SourceTick;
use crate::synthetic::PixelReadback;

/// Operational capture working-set policy, validated before allocation.
#[derive(Clone, Copy, Debug)]
pub struct CaptureLimits {
    /// Concurrent generation-bound GPU readbacks.
    pub staging_slots: usize,
    /// Reusable tightly packed CPU image leases, including the writer queue.
    pub cpu_buffers: usize,
    /// Maximum combined staging and CPU pixel bytes for this session.
    pub byte_allowance: u64,
}

impl Default for CaptureLimits {
    fn default() -> Self {
        Self {
            staging_slots: 3,
            cpu_buffers: 8,
            byte_allowance: 128 * 1024 * 1024,
        }
    }
}

/// Pixels from one ordinary submission, with its immutable render-time identity.
pub struct SubmittedImage {
    owner: std::sync::Arc<()>,
    /// The sample recorded at submission, before any subsequent world advance.
    pub sample: FrameSample,
    /// Tightly packed SDR RGBA8, with the host's render extent.
    pub rgba: Vec<u8>,
    /// Capture session selected by the caller.
    pub session: u64,
    /// Exact committed publication sampled by extraction.
    pub source: SourceTick,
    /// View history epoch used by this submission.
    pub history_epoch: u64,
    /// Monotonic host observation immediately following the queue submission.
    pub submitted_at: Instant,
}

impl AsRef<[u8]> for SubmittedImage {
    fn as_ref(&self) -> &[u8] {
        &self.rgba
    }
}

struct Slot {
    readback: PixelReadback,
    reserved: bool,
    sample: Option<(FrameSample, SourceTick, u64, Instant)>,
}

pub(super) struct Tap {
    owner: std::sync::Arc<()>,
    session: u64,
    generation: u64,
    extent: [u32; 2],
    slots: Vec<Slot>,
    omitted: u64,
    free_cpu: Vec<Vec<u8>>,
    cpu_capacity: usize,
}

pub(super) fn reserve(
    tap: &mut Option<Tap>,
    generation: u64,
    extent: [u32; 2],
) -> Option<(usize, &mut PixelReadback)> {
    tap.as_mut().and_then(|tap| tap.reserve(generation, extent))
}

impl Tap {
    pub(super) fn reserve(
        &mut self,
        generation: u64,
        extent: [u32; 2],
    ) -> Option<(usize, &mut PixelReadback)> {
        if self.generation != generation || self.extent != extent {
            self.omitted = self.omitted.saturating_add(1);
            return None;
        }
        if let Some(index) = self.slots.iter().position(|slot| !slot.reserved) {
            let slot = &mut self.slots[index];
            slot.reserved = true;
            Some((index, &mut slot.readback))
        } else {
            self.omitted = self.omitted.saturating_add(1);
            None
        }
    }

    pub(super) fn submitted(
        &mut self,
        index: usize,
        sample: &FrameSample,
        source: SourceTick,
        history: u64,
    ) {
        self.slots[index].sample = Some((*sample, source, history, Instant::now()));
    }

    pub(super) fn abandon(&mut self, index: usize) {
        let slot = &mut self.slots[index];
        slot.readback.cancel_unsubmitted();
        slot.reserved = false;
        slot.sample = None;
        self.omitted = self.omitted.saturating_add(1);
    }
}

impl<W: FrameWorld, V: SceneSource<W::Page>> FrameHost<W, V> {
    /// Enable a three-slot submitted-image tap. This never renders or submits.
    /// The caller must drain images into its own bounded writer or mark them
    /// missing. Replacing a session abandons its outstanding images.
    /// # Errors
    /// Refuses an already enabled session.
    pub fn enable_submitted_capture(&mut self, session: u64) -> Result<(), FrameError> {
        self.enable_submitted_capture_with_limits(session, CaptureLimits::default())
    }

    /// Enable with a declared finite pixel-byte allowance, before allocating.
    /// # Errors
    /// Refuses zero capacities, overflow, allowance exhaustion or an active session.
    pub fn enable_submitted_capture_with_limits(
        &mut self,
        session: u64,
        limits: CaptureLimits,
    ) -> Result<(), FrameError> {
        if self.submitted_capture.is_some() {
            return Err(FrameError::InvalidState("capture session already enabled"));
        }
        let row_bytes = (u64::from(self.render_extent[0]) * 4).div_ceil(256) * 256;
        let staging_bytes = row_bytes
            .checked_mul(u64::from(self.render_extent[1]))
            .and_then(|bytes| bytes.checked_mul(u64::try_from(limits.staging_slots).ok()?));
        let cpu_bytes = u64::from(self.render_extent[0])
            .checked_mul(u64::from(self.render_extent[1]))
            .and_then(|bytes| bytes.checked_mul(4))
            .and_then(|bytes| bytes.checked_mul(u64::try_from(limits.cpu_buffers).ok()?));
        let bytes = staging_bytes
            .zip(cpu_bytes)
            .and_then(|(a, b)| a.checked_add(b));
        if limits.staging_slots == 0
            || limits.cpu_buffers == 0
            || bytes.is_none_or(|bytes| bytes > limits.byte_allowance)
        {
            return Err(FrameError::InvalidState(
                "capture pixel-byte allowance exhausted",
            ));
        }
        self.submitted_capture = Some(Tap {
            owner: std::sync::Arc::new(()),
            session,
            generation: self.generation,
            extent: self.render_extent,
            slots: (0..limits.staging_slots)
                .map(|_| Slot {
                    readback: self.slots[0].target.readback(&self.device),
                    reserved: false,
                    sample: None,
                })
                .collect(),
            omitted: 0,
            cpu_capacity: limits.cpu_buffers,
            free_cpu: (0..limits.cpu_buffers)
                .map(|_| {
                    Vec::with_capacity(
                        self.render_extent[0] as usize * self.render_extent[1] as usize * 4,
                    )
                })
                .collect(),
        });
        Ok(())
    }

    /// Collect one image without waiting for GPU completion. No endpoint redraw
    /// or temporal-history change occurs. The caller polls normal GPU progress.
    /// # Errors
    /// Refuses old-device sessions and mapping failure. The interval must then
    /// be reported incomplete; lost devices are never polled here.
    pub fn try_submitted_image(&mut self) -> Result<Option<SubmittedImage>, FrameError> {
        let Some(tap) = self.submitted_capture.as_mut() else {
            return Err(FrameError::InvalidState("no capture session"));
        };
        if tap.generation != self.generation || tap.extent != self.render_extent {
            return Err(FrameError::InvalidState("retired capture generation"));
        }
        let Some(mut rgba) = tap.free_cpu.pop() else {
            return Ok(None);
        };
        for slot in &mut tap.slots {
            let Some((sample, source, history_epoch, submitted_at)) = slot.sample else {
                continue;
            };
            if slot
                .readback
                .try_collect_into(self.generation, &mut rgba)
                .map_err(|error| FrameError::Gpu(format!("submitted readback: {error:?}")))?
            {
                slot.sample = None;
                slot.reserved = false;
                return Ok(Some(SubmittedImage {
                    owner: std::sync::Arc::clone(&tap.owner),
                    sample,
                    rgba,
                    session: tap.session,
                    source,
                    history_epoch,
                    submitted_at,
                }));
            }
        }
        tap.free_cpu.push(rgba);
        Ok(None)
    }

    /// Return a writer's CPU lease to the bounded pool. Retired
    /// session buffers are discarded and never populate a new session.
    pub fn recycle_submitted_image(&mut self, image: SubmittedImage) {
        if let Some(tap) = self.submitted_capture.as_mut()
            && tap.session == image.session
            && std::sync::Arc::ptr_eq(&tap.owner, &image.owner)
            && tap.generation == self.generation
            && tap.free_cpu.len() < tap.cpu_capacity
            && image.rgba.capacity()
                <= self.render_extent[0] as usize * self.render_extent[1] as usize * 4
        {
            tap.free_cpu.push(image.rgba);
        }
    }

    /// Number of samples omitted by ring exhaustion or unsubmitted abandonment.
    /// Any nonzero count makes continuous-image evidence incomplete.
    pub fn submitted_capture_omissions(&self) -> u64 {
        self.submitted_capture.as_ref().map_or(0, |tap| tap.omitted)
    }

    /// Current bounded staging slots and outstanding CPU leases, or no
    /// resources when capture is disabled.
    pub fn submitted_capture_usage(&self) -> Option<(usize, usize)> {
        self.submitted_capture.as_ref().map(|tap| {
            (
                tap.slots.iter().filter(|slot| slot.reserved).count(),
                tap.cpu_capacity - tap.free_cpu.len(),
            )
        })
    }

    /// End capture without waiting for pending GPU work or mapping callbacks.
    /// wgpu retains resources needed by already submitted commands. Late
    /// callbacks own only the discarded readback mailboxes.
    pub fn disable_submitted_capture(&mut self) {
        self.submitted_capture = None;
    }
}
