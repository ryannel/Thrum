//! Optional bounded readback of ordinary submitted frames.

use super::Instant;
use super::{FrameError, FrameHost, FrameSample, FrameWorld, SceneSource};
use crate::extract::SourceTick;
use crate::present::SurfacePresenter;
use crate::synthetic::{CaptureSource, PixelReadback, readback::CopyTiming};

/// Operational capture working-set policy, validated before allocation.
#[derive(Clone, Copy, Debug)]
pub struct CaptureLimits {
    /// Concurrent generation-bound GPU readbacks across the session.
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
            // The default ring also fits a 1080p RGBA16Float drawable.
            byte_allowance: 256 * 1024 * 1024,
        }
    }
}

/// Pixels from one ordinary submission, with its immutable render-time identity.
pub struct SubmittedImage {
    /// The sample recorded at submission, before any subsequent world advance.
    pub sample: FrameSample,
    /// Actual source grid of this submitted image.
    pub extent: [u32; 2],
    /// Tightly packed channel bytes at the captured source extent.
    /// SDR is RGBA8. Scene-linear and EDR use little-endian RGBA16Float.
    pub rgba: Vec<u8>,
    /// Engine stage and stored pixel format of `rgba`.
    pub pixel_source: CaptureSource,
    /// Exact GPU-resolved exposure words from the same submission, before float decoding.
    /// Values encode storage scale, inverse gain, EV100, incident lux, flags,
    /// submitted serial lo/hi and display selectors.
    pub exposure_gpu_words: Option<[u32; 8]>,
    /// Capture session selected by the caller.
    pub session: u64,
    /// Exact committed publication sampled by extraction.
    pub source: SourceTick,
    /// View history epoch used by this submission.
    pub history_epoch: u64,
    /// Monotonic host observation immediately following the queue submission.
    pub submitted_at: Instant,
    /// Submission time relative to this capture session's monotonic origin.
    pub submitted_elapsed: std::time::Duration,
    /// GPU copy upper bracket, including the dependent marker/encoder overhead.
    pub copy_gpu_ns: Option<f64>,
    /// Device generation captured by the render owner at submission.
    pub device_generation: u64,
    /// Engine presenter incarnation (initial prepare, resize, device publish).
    /// This does not identify an OS drawable or compositor lease.
    pub presenter_generation: u64,
    /// Lead actually consumed by world interpolation for this submission.
    pub presentation_lead: std::time::Duration,
    // Drop the lease after its pixels, including when a writer simply drops the image.
    lease: std::sync::Arc<()>,
}

impl SubmittedImage {
    /// Highest finite nonnegative display-linear component in an EDR drawable.
    /// Returns `None` for a different capture source or invalid float pixels.
    #[must_use]
    pub fn edr_max_rgb(&self) -> Option<f32> {
        if self.pixel_source != CaptureSource::Edr {
            return None;
        }
        let mut maximum = 0_u16;
        for pixel in self.rgba.as_chunks::<8>().0 {
            for [low, high] in pixel[..6].as_chunks::<2>().0 {
                let bits = u16::from_le_bytes([*low, *high]);
                if bits >= 0x7c00 {
                    return None;
                }
                maximum = maximum.max(bits);
            }
        }
        Some(half::f16::from_bits(maximum).to_f32())
    }
}

impl AsRef<[u8]> for SubmittedImage {
    fn as_ref(&self) -> &[u8] {
        &self.rgba
    }
}

#[derive(Clone)]
pub(super) struct Stamp {
    pub sample: FrameSample,
    pub source: SourceTick,
    pub history_epoch: u64,
    pub submitted_at: Instant,
    pub device_generation: u64,
    pub presenter_generation: u64,
    pub presentation_lead: std::time::Duration,
}

struct Slot {
    readback: PixelReadback,
    reserved: bool,
    sample: Option<Stamp>,
    retired: bool,
    timing_collected: bool,
    copy_ns: Option<f64>,
}

#[derive(Clone, Copy)]
pub(super) struct TapSpec {
    source: Option<CaptureSource>,
    session: u64,
    started_at: Instant,
    limits: CaptureLimits,
}

impl TapSpec {
    pub(super) fn source_for_format(self, format: Option<wgpu::TextureFormat>) -> CaptureSource {
        self.source.unwrap_or_else(|| default_source(format))
    }
}

fn default_source(format: Option<wgpu::TextureFormat>) -> CaptureSource {
    if format == Some(wgpu::TextureFormat::Rgba16Float) {
        CaptureSource::Edr
    } else {
        CaptureSource::Sdr
    }
}

pub(super) struct Tap {
    limits: CaptureLimits,
    spec: TapSpec,
    pub(super) source: CaptureSource,
    #[cfg(not(target_arch = "wasm32"))]
    owner: std::sync::Arc<()>,
    leases: std::sync::Arc<()>,
    session: u64,
    generation: u64,
    extent: [u32; 2],
    started_at: Instant,
    accepting: bool,
    slots: Vec<Slot>,
    omitted: u64,
    free_cpu: Vec<Vec<u8>>,
    cpu_capacity: usize,
    spare_timers: Vec<CopyTiming>,
}

pub(super) fn reserve(
    tap: &mut Option<Tap>,
    generation: u64,
    extent: [u32; 2],
    source: CaptureSource,
    timing_requested: bool,
) -> Option<(usize, &mut PixelReadback)> {
    tap.as_mut()
        .filter(|tap| tap.source != CaptureSource::Edr)
        .and_then(|tap| tap.reserve(generation, extent, source, timing_requested))
}

pub(super) fn copy_drawable(
    tap: Option<&mut Tap>,
    generation: u64,
    surface: Option<&super::SurfaceOutput<'_>>,
    prepared: &super::PreparedRenderer,
    encoder: &mut wgpu::CommandEncoder,
    timing_requested: bool,
) -> Result<Option<usize>, FrameError> {
    let (Some(tap), Some(surface)) = (tap, surface) else {
        return Ok(None);
    };
    let texture = surface.view.texture();
    if !texture.usage().contains(wgpu::TextureUsages::COPY_SRC) {
        tap.omitted += 1;
        return Ok(None);
    }
    let Some((index, readback)) = tap.reserve(
        generation,
        surface.extent,
        CaptureSource::Edr,
        timing_requested,
    ) else {
        return Ok(None);
    };
    let exposure = prepared
        .three_d()
        .map(crate::synthetic::PreparedSynthetic::pending_exposure_sample);
    if let Err(error) = readback.copy_texture(encoder, texture, exposure) {
        tap.abandon(index);
        return Err(FrameError::gpu("EDR drawable copy", error));
    }
    Ok(Some(index))
}

impl Tap {
    pub(super) fn spec_for_generation(&self, generation: u64) -> Option<TapSpec> {
        (self.generation == generation).then_some(self.spec)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn owner(&self) -> std::sync::Arc<()> {
        std::sync::Arc::clone(&self.owner)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn owns(&self, owner: &std::sync::Arc<()>) -> bool {
        std::sync::Arc::ptr_eq(&self.owner, owner)
    }

    pub(super) fn pending(&self) -> bool {
        self.slots.iter().any(|slot| slot.reserved)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn prepare_extent(
        spec: TapSpec,
        format: wgpu::TextureFormat,
        generation: u64,
        extent: [u32; 2],
    ) -> Result<Self, FrameError> {
        let source = spec.source_for_format(Some(format));
        let budget = validate_capture_budget(extent, source, spec.limits);
        if budget.is_err() && spec.source.is_some() {
            budget?;
        }
        Ok(Self::replacement(spec, source, generation, extent))
    }

    fn replacement(
        spec: TapSpec,
        source: CaptureSource,
        generation: u64,
        extent: [u32; 2],
    ) -> Self {
        let mut limits = spec.limits;
        // Leave presentation eligible even when the new grid cannot fit the tap.
        if validate_capture_budget(extent, source, limits).is_err() {
            limits.staging_slots = 0;
            limits.cpu_buffers = 0;
        }
        Self::empty(spec, source, limits, generation, extent)
    }

    fn empty(
        spec: TapSpec,
        source: CaptureSource,
        limits: CaptureLimits,
        generation: u64,
        extent: [u32; 2],
    ) -> Self {
        Self {
            source,
            limits,
            #[cfg(not(target_arch = "wasm32"))]
            owner: std::sync::Arc::new(()),
            leases: std::sync::Arc::new(()),
            session: spec.session,
            generation,
            extent,
            started_at: spec.started_at,
            accepting: true,
            slots: Vec::new(),
            omitted: 0,
            cpu_capacity: 0,
            free_cpu: Vec::new(),
            spec,
            spare_timers: Vec::new(),
        }
    }

    fn grow(
        &mut self,
        staging_slots: usize,
        cpu_buffers: usize,
        target: &super::preparation::FrameTarget,
        device: &wgpu::Device,
    ) {
        for _ in self.slots.len()..staging_slots {
            let mut readback = if self.source == CaptureSource::Edr {
                let row = (self.extent[0] * self.source.bytes_per_pixel())
                    .next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
                PixelReadback::new(
                    device,
                    self.generation,
                    self.extent,
                    self.source,
                    row,
                    "EDR drawable readback",
                )
            } else {
                match target {
                    super::FrameTarget::TwoD(target) => target.readback_from(device, self.source),
                    super::FrameTarget::ThreeD(_) => target.readback(device),
                }
            };
            readback.copy_timing = self.spare_timers.pop();
            self.slots.push(Slot {
                readback,
                reserved: false,
                sample: None,
                retired: false,
                timing_collected: false,
                copy_ns: None,
            });
        }
        self.free_cpu.extend(
            (self.cpu_capacity..cpu_buffers).map(|_| Self::cpu_buffer(self.extent, self.source)),
        );
        self.cpu_capacity = cpu_buffers;
    }

    fn outstanding(&self) -> usize {
        std::sync::Arc::strong_count(&self.leases) - 1
    }

    fn trim_retired(&mut self, spare_timers: &mut Vec<CopyTiming>) {
        self.slots.retain_mut(|slot| {
            if !slot.reserved && !slot.retired {
                spare_timers.extend(slot.readback.copy_timing.take());
            }
            slot.reserved
        });
        let outstanding = self.outstanding();
        // Preserve enough capacity to drain pending slots, even if those
        // buffers are currently held by the writer and need to be returned first.
        self.cpu_capacity = outstanding.max(self.cpu_capacity.min(self.slots.len()));
        self.free_cpu.truncate(self.cpu_capacity - outstanding);
    }

    fn allocation_bytes(&self) -> u64 {
        let (staging, cpu) = capture_buffer_bytes(self.extent, self.source);
        staging * self.slots.len() as u64 + cpu * self.cpu_capacity as u64
    }

    fn cpu_buffer(extent: [u32; 2], source: CaptureSource) -> Vec<u8> {
        Vec::with_capacity(
            extent[0] as usize * extent[1] as usize * source.bytes_per_pixel() as usize,
        )
    }

    fn take_cpu(&mut self) -> Option<Vec<u8>> {
        self.free_cpu.pop().or_else(|| {
            (self.outstanding() < self.cpu_capacity)
                .then(|| Self::cpu_buffer(self.extent, self.source))
        })
    }

    pub(super) fn reserve(
        &mut self,
        generation: u64,
        extent: [u32; 2],
        source: CaptureSource,
        timing_requested: bool,
    ) -> Option<(usize, &mut PixelReadback)> {
        if !self.accepting {
            return None;
        }
        if self.generation != generation || self.extent != extent || self.source != source {
            self.omitted = self.omitted.saturating_add(1);
            return None;
        }
        if let Some(index) = self
            .slots
            .iter()
            .position(|slot| !slot.reserved && !slot.retired)
        {
            let slot = &mut self.slots[index];
            slot.reserved = true;
            slot.timing_collected = false;
            slot.copy_ns = None;
            slot.readback.set_copy_timing_requested(timing_requested);
            Some((index, &mut slot.readback))
        } else {
            self.omitted = self.omitted.saturating_add(1);
            None
        }
    }

    pub(super) fn submitted(&mut self, index: usize, stamp: &Stamp) {
        self.slots[index].sample = Some(stamp.clone());
    }

    pub(super) fn finish_timing(
        &mut self,
        index: usize,
        encoder: &wgpu::CommandEncoder,
    ) -> Result<(), FrameError> {
        self.slots[index]
            .readback
            .finish_copy_timing(encoder)
            .map_err(|e| FrameError::gpu("copy timing", e))
    }
    pub(super) fn abandon(&mut self, index: usize) {
        let slot = &mut self.slots[index];
        slot.readback.cancel_unsubmitted();
        slot.reserved = false;
        slot.sample = None;
        self.omitted = self.omitted.saturating_add(1);
    }
}

pub(super) fn capture_extent(
    source: CaptureSource,
    target: &super::preparation::FrameTarget,
    presenter: Option<&SurfacePresenter>,
) -> [u32; 2] {
    if source == CaptureSource::Edr
        && let Some(presenter) = presenter
    {
        return presenter.extent();
    }
    target.capture_extent()
}

impl<W: FrameWorld, V: SceneSource<W::Page>> FrameHost<W, V> {
    /// Enable a three-slot submitted-image tap that follows the output mode.
    /// This never renders or submits.
    /// The caller must drain images into its own bounded writer or mark them
    /// missing. Replacing a session abandons its outstanding images.
    /// # Errors
    /// Refuses an already enabled session.
    pub fn enable_submitted_capture(&mut self, session: u64) -> Result<(), FrameError> {
        self.enable_submitted_capture_with_limits(session, CaptureLimits::default())
    }

    /// Enable with a declared finite pixel-byte allowance, before allocating.
    /// After an extent or format transition, frames that exceed the allowance are omitted
    /// until the session reaches a grid and format that fit again.
    /// # Errors
    /// Refuses zero capacities, overflow, allowance exhaustion or an active session.
    pub fn enable_submitted_capture_with_limits(
        &mut self,
        session: u64,
        limits: CaptureLimits,
    ) -> Result<(), FrameError> {
        self.enable_submitted_capture_at(session, limits, None)
    }

    /// Enable the existing tap at a prepared engine stage. Stage changes prepare
    /// and publish replacement targets only after all ordinary frames retire.
    /// A float-mode sprite host can enable the 8-bit stage with frames in flight.
    /// A subsequent visual/extent replacement that changes the target stage
    /// refuses capture admission and increments omissions, while rendering continues.
    /// # Errors
    /// Refuses active sessions, pending frames during a stage change, invalid
    /// capacities, allowance overflow or target preparation failure.
    pub fn enable_submitted_capture_from(
        &mut self,
        session: u64,
        limits: CaptureLimits,
        source: CaptureSource,
    ) -> Result<(), FrameError> {
        self.enable_submitted_capture_at(session, limits, Some(source))
    }

    fn enable_submitted_capture_at(
        &mut self,
        session: u64,
        limits: CaptureLimits,
        source: Option<CaptureSource>,
    ) -> Result<(), FrameError> {
        if self.submitted_capture.is_some() {
            return Err(FrameError::InvalidState("capture session already enabled"));
        }
        let spec = TapSpec {
            source,
            session,
            started_at: Instant::now(),
            limits,
        };
        let source = spec.source_for_format(self.presenter.as_ref().map(|p| p.profile().format));
        let _region = self
            .creations
            .region(super::CreationPhase::Transition, "capture-stage");
        if source == CaptureSource::Edr && !self.is_edr() {
            return Err(FrameError::InvalidState("capture requires an EDR drawable"));
        }
        let sprite_sdr = source == CaptureSource::Sdr
            && self.is_edr()
            && matches!(self.prepared, super::PreparedRenderer::TwoD(_));
        let replacement = if sprite_sdr
            || source == CaptureSource::Edr
            || self.slots[0].target.capture_source() == source
        {
            let extent = capture_extent(source, &self.slots[0].target, self.presenter.as_ref());
            validate_capture_budget(extent, source, limits)?;
            None
        } else {
            if self.pending_frames() != 0 {
                return Err(FrameError::InvalidState(
                    "capture stage change requires retired frames",
                ));
            }
            let mut slots = super::preparation::TargetPreparation::new(&self.prepared)
                .prepare_slots(
                    &self.queue,
                    self.render_extent,
                    self.presenter.is_some(),
                    self.presenter.as_ref().map(SurfacePresenter::profile),
                    source,
                    None,
                    Some(limits),
                )?;
            super::preparation::bind_presenter(
                &self.prepared,
                &mut slots,
                self.presenter.as_ref(),
            )?;
            if let Some(prepared) = self.prepared.three_d() {
                let inputs = prepared.target_inputs();
                for slot in &mut slots {
                    slot.target
                        .share_view_history(&inputs, &self.slots[0].target);
                }
            }
            Some(slots)
        };
        let target = &replacement.as_ref().unwrap_or(&self.slots)[0].target;
        let extent = capture_extent(source, target, self.presenter.as_ref());
        let mut tap = Tap::empty(spec, source, limits, self.generation, extent);
        tap.spare_timers = CopyTiming::prepare(
            self.prepared.capabilities(),
            &self.queue,
            limits.staging_slots,
        )
        .map_err(|error| {
            FrameError::gpu(
                "copy timing prepare",
                crate::error::RenderError::from(error),
            )
        })?;
        tap.grow(
            tap.limits.staging_slots,
            tap.limits.cpu_buffers,
            target,
            &self.device,
        );
        if let Some(slots) = replacement {
            self.slots = slots;
            self.presenter_generation += 1;
        }
        self.submitted_capture = Some(tap);
        if sprite_sdr {
            self.sync_sprite_capture_stage();
        }
        Ok(())
    }

    /// Collect one image without waiting for GPU completion. No endpoint redraw
    /// or temporal-history change occurs. The caller polls normal GPU progress.
    /// # Errors
    /// Refuses old-device sessions and mapping failure. The interval must then
    /// be reported incomplete; lost devices are never polled here.
    pub fn try_submitted_image(&mut self) -> Result<Option<SubmittedImage>, FrameError> {
        // The tap may be drained before normal frame reports. Its timestamp
        // resolution is queue work too, and must respect native failure admission.
        self.check_native_completion()?;
        if let Some(tap) = self.retired_capture.iter_mut().find(|tap| tap.pending()) {
            let before = tap.omitted;
            let result = tap.try_image(&self.device, &self.queue, self.generation);
            if let Some(active) = &mut self.submitted_capture {
                active.omitted += tap.omitted - before;
                tap.trim_retired(&mut active.spare_timers);
            }
            return result;
        }
        let Some(tap) = self.submitted_capture.as_mut() else {
            return Err(FrameError::InvalidState("no capture session"));
        };
        if tap.generation != self.generation
            || tap.extent
                != capture_extent(tap.source, &self.slots[0].target, self.presenter.as_ref())
        {
            return Err(FrameError::InvalidState("retired capture generation"));
        }
        tap.try_image(&self.device, &self.queue, self.generation)
    }

    /// Return a writer's CPU lease to the bounded pool that issued it.
    pub fn recycle_submitted_image(&mut self, image: SubmittedImage) {
        if let Some(tap) = self
            .submitted_capture
            .iter_mut()
            .chain(&mut self.retired_capture)
            .find(|tap| std::sync::Arc::ptr_eq(&tap.leases, &image.lease))
            && tap.generation == self.generation
            && image.rgba.capacity()
                == tap.extent[0] as usize
                    * tap.extent[1] as usize
                    * tap.source.bytes_per_pixel() as usize
        {
            tap.free_cpu.push(image.rgba);
        }
    }

    pub(super) fn switch_capture_extent(&mut self, capture: Option<Tap>) {
        // An invalidated device session remains visible to its caller until closed.
        // An extent publication cannot reactivate or retire that old-device tap.
        let Some(mut next) = capture else {
            return;
        };
        if let Some(mut previous) = self.submitted_capture.take() {
            if previous.source == next.source && previous.extent == next.extent {
                self.submitted_capture = Some(previous);
                return;
            }
            next.accepting = previous.accepting;
            next.omitted = previous.omitted;
            next.spare_timers = std::mem::take(&mut previous.spare_timers);
            previous.trim_retired(&mut next.spare_timers);
            if previous.pending() || previous.outstanding() != 0 {
                self.retired_capture.push_back(previous);
            }
        }
        self.submitted_capture = Some(next);
    }

    pub(super) fn resize_drawable_capture(&mut self) {
        // The 2D scene grid follows the canvas. Its SDR and scene-linear taps
        // change extent with the scene even when the drawable is also SDR.
        let replacement = self.submitted_capture.as_ref().and_then(|tap| {
            let extent = capture_extent(tap.source, &self.slots[0].target, self.presenter.as_ref());
            ((tap.source == CaptureSource::Edr
                || matches!(self.prepared, super::PreparedRenderer::TwoD(_)))
                && tap.generation == self.generation
                && tap.extent != extent)
                .then(|| Tap::replacement(tap.spec, tap.source, tap.generation, extent))
        });
        self.switch_capture_extent(replacement);
    }

    pub(super) fn sprite_capture_source_for_format(
        &self,
        format: wgpu::TextureFormat,
    ) -> CaptureSource {
        let format = Some(format);
        self.submitted_capture
            .as_ref()
            .and_then(|tap| tap.spec_for_generation(self.generation))
            .map_or_else(
                || default_source(format),
                |spec| spec.source_for_format(format),
            )
    }

    pub(super) fn sprite_sdr_capture_enabled(&self) -> bool {
        self.submitted_capture.as_ref().is_some_and(|tap| {
            tap.generation == self.generation && tap.source == CaptureSource::Sdr
        })
    }

    pub(super) fn sync_sprite_capture_stage(&mut self) {
        if !matches!(self.prepared, super::PreparedRenderer::TwoD(_)) || !self.is_edr() {
            return;
        }
        let enabled = self.sprite_sdr_capture_enabled();
        // Keep an in-flight slot's graph and image until its frame and timing
        // retire. Free slots reflect the tap state before their next recording.
        for slot in &mut self.slots {
            if !slot.is_pending()
                && let super::FrameTarget::TwoD(target) = &mut slot.target
            {
                if enabled {
                    target.enable_sdr(&self.device);
                } else {
                    target.release_sdr();
                }
            }
        }
    }

    pub(super) fn refresh_capture_capacity(&mut self) {
        let Some(tap) = self.submitted_capture.as_mut() else {
            return;
        };
        let extent = capture_extent(tap.source, &self.slots[0].target, self.presenter.as_ref());
        let mut available = tap.limits;
        self.retired_capture.retain_mut(|retired| {
            retired.trim_retired(&mut tap.spare_timers);
            if tap.limits.staging_slots != 0 {
                available.staging_slots -= retired.slots.len();
                available.cpu_buffers -= retired.cpu_capacity;
                available.byte_allowance -= retired.allocation_bytes();
            }
            retired.pending() || retired.outstanding() != 0
        });
        if !tap.accepting
            || tap.generation != self.generation
            || tap.extent != extent
            || tap.limits.staging_slots == 0
            || (tap.slots.len() == tap.limits.staging_slots
                && tap.cpu_capacity == tap.limits.cpu_buffers)
        {
            return;
        }
        let target = &self.slots[0].target;
        if tap.source != CaptureSource::Edr && target.capture_source() != tap.source {
            return;
        }
        let (staging_bytes, cpu_bytes) = capture_buffer_bytes(tap.extent, tap.source);
        // Keep every active allocation and find room for one CPU buffer before
        // adding staging slots. A queue of copies with no CPU capacity can't drain.
        let min_cpu = tap.cpu_capacity.max(1);
        if available.cpu_buffers < min_cpu
            || available.staging_slots == 0
            || available.byte_allowance < staging_bytes + cpu_bytes * min_cpu as u64
        {
            return;
        }
        let staging_slots = available.staging_slots.min(
            ((available.byte_allowance - cpu_bytes * min_cpu as u64) / staging_bytes)
                .try_into()
                .unwrap_or(usize::MAX),
        );
        let cpu_buffers = available.cpu_buffers.min(
            ((available.byte_allowance - staging_bytes * staging_slots as u64) / cpu_bytes)
                .try_into()
                .unwrap_or(usize::MAX),
        );
        tap.grow(staging_slots, cpu_buffers, target, &self.device);
    }

    pub(super) fn invalidate_submitted_capture(&mut self) {
        for tap in self
            .submitted_capture
            .iter_mut()
            .chain(&mut self.retired_capture)
        {
            tap.slots.clear();
            tap.spare_timers.clear();
            tap.cpu_capacity = tap.outstanding();
            tap.free_cpu.clear();
        }
    }

    /// Stop admitting copies while pending mappings drain. Does not submit or wait.
    pub fn stop_submitted_capture(&mut self) {
        if let Some(tap) = &mut self.submitted_capture {
            tap.accepting = false;
        }
    }

    /// Number of samples omitted by ring exhaustion or unsubmitted abandonment.
    /// Any nonzero count makes continuous-image evidence incomplete.
    pub fn submitted_capture_omissions(&self) -> u64 {
        self.submitted_capture.as_ref().map_or(0, |tap| tap.omitted)
    }

    /// Reserved staging slots and outstanding CPU leases across the session,
    /// or no resources when capture is disabled.
    pub fn submitted_capture_usage(&self) -> Option<(usize, usize)> {
        self.submitted_capture.as_ref().map(|active| {
            std::iter::once(active).chain(&self.retired_capture).fold(
                (0, 0),
                |(staging, cpu), tap| {
                    (
                        staging + tap.slots.iter().filter(|slot| slot.reserved).count(),
                        cpu + tap.outstanding(),
                    )
                },
            )
        })
    }

    /// End capture without waiting for pending GPU work or mapping callbacks.
    /// wgpu retains resources needed by already submitted commands. Late
    /// callbacks own only the discarded readback mailboxes. Free sprite slots
    /// release their optional 8-bit display image here; pending slots keep it
    /// until retirement.
    pub fn disable_submitted_capture(&mut self) {
        self.submitted_capture = None;
        self.retired_capture.clear();
        self.sync_sprite_capture_stage();
    }
}

pub(super) fn validate_capture_budget(
    extent: [u32; 2],
    source: CaptureSource,
    limits: CaptureLimits,
) -> Result<(), FrameError> {
    let (staging_bytes, cpu_bytes) = capture_buffer_bytes(extent, source);
    let bytes = staging_bytes
        .checked_mul(limits.staging_slots as u64)
        .zip(cpu_bytes.checked_mul(limits.cpu_buffers as u64))
        .and_then(|(a, b)| a.checked_add(b));
    if limits.staging_slots == 0
        || limits.cpu_buffers == 0
        || bytes.is_none_or(|bytes| bytes > limits.byte_allowance)
    {
        return Err(FrameError::InvalidState(
            "capture pixel-byte allowance exhausted",
        ));
    }
    Ok(())
}

fn capture_buffer_bytes(extent: [u32; 2], source: CaptureSource) -> (u64, u64) {
    let row = u64::from(extent[0]) * u64::from(source.bytes_per_pixel());
    let height = u64::from(extent[1]);
    let padded_row = row.next_multiple_of(u64::from(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT));
    (
        padded_row.saturating_mul(height).saturating_add(32),
        row.saturating_mul(height),
    )
}

impl Tap {
    fn try_image(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        generation: u64,
    ) -> Result<Option<SubmittedImage>, FrameError> {
        let tap = self;
        if let Some(segment) = tap
            .slots
            .iter()
            .filter_map(|slot| slot.sample.as_ref())
            .min_by_key(|stamp| stamp.sample.submission_order)
            .and_then(|stamp| stamp.sample.native_segment.as_ref())
        {
            match segment.outcome() {
                crate::synthetic::NativeOutcome::Pending => return Ok(None),
                crate::synthetic::NativeOutcome::Failed => {
                    return Err(FrameError::NativeCommandBuffer {
                        generation: segment.generation,
                    });
                }
                crate::synthetic::NativeOutcome::Timed(_)
                | crate::synthetic::NativeOutcome::Unmeasured => {}
            }
        }
        // Resolve only after completion, in later render-coordinator work.
        if tap
            .slots
            .iter()
            .any(|s| s.sample.is_some() && s.readback.copy_timing_ready_to_resolve())
        {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("submitted copy timestamp resolution"),
            });
            for slot in &mut tap.slots {
                if slot.sample.is_some() {
                    slot.readback
                        .resolve_copy_timing(&mut encoder)
                        .map_err(|e| FrameError::gpu("copy timestamp resolution", e))?;
                }
            }
            queue.submit([encoder.finish()]);
        }
        let oldest = tap
            .slots
            .iter()
            .enumerate()
            .filter_map(|(i, slot)| slot.sample.as_ref().map(|stamp| (i, stamp.clone())))
            .min_by_key(|(_, stamp)| stamp.sample.submission_order);
        if let Some((index, stamp)) = oldest {
            let Some(mut rgba) = tap.take_cpu() else {
                return Ok(None);
            };
            let slot = &mut tap.slots[index];
            let collect = (|| {
                if !slot.timing_collected {
                    let std::task::Poll::Ready(copy_ns) = slot.readback.copy_duration()? else {
                        return Ok(false);
                    };
                    slot.copy_ns = copy_ns;
                    slot.timing_collected = true;
                }
                slot.readback.try_collect_into(generation, &mut rgba)
            })();
            match collect {
                Ok(false) => {
                    tap.free_cpu.push(rgba);
                    return Ok(None);
                }
                Err(error) => {
                    // A failed mapping cannot be reused. Retire the GPU slot and
                    // return its CPU lease; the session is explicitly incomplete.
                    slot.retired = true;
                    slot.reserved = false;
                    slot.sample = None;
                    tap.omitted += 1;
                    tap.free_cpu.push(rgba);
                    return Err(FrameError::gpu("submitted readback", error));
                }
                Ok(true) => {}
            }
            slot.sample = None;
            slot.reserved = false;
            return Ok(Some(SubmittedImage {
                lease: std::sync::Arc::clone(&tap.leases),
                extent: tap.extent,
                sample: stamp.sample,
                rgba,
                pixel_source: tap.source,
                exposure_gpu_words: slot.readback.exposure_gpu_words(),
                session: tap.session,
                source: stamp.source,
                history_epoch: stamp.history_epoch,
                submitted_at: stamp.submitted_at,
                submitted_elapsed: stamp.submitted_at.duration_since(tap.started_at),
                device_generation: stamp.device_generation,
                presenter_generation: stamp.presenter_generation,
                presentation_lead: stamp.presentation_lead,
                copy_gpu_ns: slot.copy_ns,
            }));
        }
        Ok(None)
    }
}
