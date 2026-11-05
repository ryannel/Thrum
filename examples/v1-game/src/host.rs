//! Bounded offscreen frame host for the first assembled V1 workload.

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

#[cfg(not(target_arch = "wasm32"))]
use thrum_content_runtime::EnvironmentProduct;
use thrum_core::{ClockConfig, ClockState, TickClock};
#[cfg(not(target_arch = "wasm32"))]
use thrum_render::extract::SourceTick;
#[cfg(not(target_arch = "wasm32"))]
use thrum_render::synthetic::PhotometricEnvironment;
use thrum_render::{
    capabilities::DeviceCapabilities,
    extract::{ExtractReport, RenderExtractor},
    present::{PresentationBinding, SurfacePresenter, SurfaceProfile},
    synthetic::{
        FrameLighting, OffscreenTarget, PreparedSynthetic, SyntheticLight, SyntheticPointLight,
        SyntheticScene, SyntheticView,
    },
    timing::GpuTimings,
};

use crate::{
    projection::{GameProjection, ProjectionGap, ProjectionOutbox},
    scene::{self, ApplyReport, ENTITY_COUNT},
    simulation::{SimError, Simulation},
};

#[cfg(not(target_arch = "wasm32"))]
mod native_worker;
#[cfg(not(target_arch = "wasm32"))]
use native_worker::{CompletedExtract, ExtractRequest, NativeWorker};

const GENERATION: u64 = 1;
const DEFAULT_RENDER_EXTENT: [u32; 2] = [640, 360];
const GPU_WAIT: Duration = Duration::from_secs(10);
const IN_FLIGHT: usize = 2;
const GAME_SUN: SyntheticLight = SyntheticLight {
    direction_to_light: [0.4, -0.6, 1.0],
    irradiance: [4.0; 3],
};

#[cfg(not(target_arch = "wasm32"))]
fn photometric(product: &EnvironmentProduct) -> PhotometricEnvironment {
    let values = product.values();
    PhotometricEnvironment {
        toward_sun: values.toward_sun(),
        sun_lux: values.sun_lux(),
        sun_colour: values.sun_colour(),
        ambient_lux: values.ambient_lux(),
        background_radiance: values.background_radiance(),
    }
}

fn frame_phase(tick: u64, alpha: f32) -> f32 {
    if tick == 0 {
        return 0.0;
    }
    let cycle = u16::try_from(tick.saturating_sub(1) % 360).expect("phase is less than 360");
    (f32::from(cycle) + alpha) * (std::f32::consts::TAU / 360.0)
}

fn frame_view(tick: u64, alpha: f32, extent: [u32; 2]) -> Result<SyntheticView, String> {
    let phase = frame_phase(tick, alpha);
    SyntheticView::perspective(
        [phase.sin() * 0.08, (phase.cos() - 1.0) * 0.04, 2.0],
        [0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        1.0,
        extent[0] as f32 / extent[1] as f32,
        0.1,
        20.0,
    )
    .map_err(|error| format!("live camera: {error:?}"))
}

/// One completed simulation-to-GPU frame. GPU pass values are absent when the
/// adapter lacks timestamp queries or a timing readback fails. Callback time
/// excludes deferred GPU completion and must not be added to GPU pass times.
#[derive(Clone, Copy, Debug)]
pub struct FrameSample {
    /// Last complete world tick used by the draw.
    pub tick: u64,
    /// GPU submission order; independent of the visual source tick.
    pub submission_order: u64,
    /// Committed updates served in this host call, including catch-up work.
    pub completed_ticks: u32,
    /// Due updates still queued after this call.
    pub queued_ticks: u32,
    /// CPU time spent inside authoritative simulation callbacks.
    pub tick_cpu: Duration,
    /// CPU time for committed-world visit and render-column extraction.
    pub extract_cpu: Duration,
    /// Native worker queue delay; zero for serial extraction.
    pub worker_queue_ns: u32,
    /// Time a completed native CPU view waited before submission.
    pub worker_ready_ns: u32,
    /// Committed ticks newer than this view at submission.
    pub visual_age_ticks: u32,
    /// A completed scene target was presented again while native extraction was busy.
    pub reused_visual: bool,
    /// Fraction between the previous and current committed world ticks.
    pub presentation_alpha: f32,
    /// CPU time for scene mapping, ABI packing and queue writes.
    pub upload_cpu: Duration,
    /// Nanoseconds within upload for live camera, local lights and cascade refit.
    pub view_update_ns: u32,
    /// CPU time through command encoding, finish, submit and completion registration.
    pub encode_cpu: Duration,
    /// CPU wall time for this drive call, including nonblocking GPU polling.
    /// Separate completed-report draining is outside this value.
    pub wall: Duration,
    /// Actual GPU pass durations: near/far Sun shadows, local clusters,
    /// forward and display. Disabled passes report zero in their slots.
    pub gpu_pass_ns: Option<[f64; 6]>,
    /// GPU time for the optional surface blit, when it was submitted and timed.
    pub gpu_present_ns: Option<f64>,
    /// This frame also wrote an acquired platform surface.
    pub surface_presented: bool,
    /// The GPU frame completed but its optional timing batch was unusable.
    pub timing_fault: bool,
    /// Records whose changed ABI bytes were enqueued for upload.
    pub uploaded_rows: u32,
    /// Queue payload bytes for this update.
    pub uploaded_bytes: usize,
    /// Accepted movers with valid adjacent-tick previous positions.
    pub moving: u32,
    /// Presentation samples, updates or local-light sources dropped.
    pub dropped_presentation: u32,
    /// Local-light references omitted from this prepared view after a cluster
    /// exceeds its capacity. The fixed fixture prepares them once.
    pub dropped_light_references: usize,
    /// Authored presentation light channels clamped during preparation.
    pub clamped_light_channels: u32,
    /// Active casters flattened to a prepared shadow depth plane.
    pub clamped_shadow_casters: u32,
    /// Cooked visual product used by this submitted frame, if selected.
    pub visual_revision: Option<[u8; 32]>,
}

/// One clock callback. A busy presentation ring drops only its optional view;
/// committed simulation updates are never dropped or applied twice.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DriveReport {
    /// Last complete authoritative tick after this callback.
    pub tick: u64,
    /// Refused elapsed ticks when the bounded clock overloaded. `Some(0)`
    /// means it remains overloaded and requires explicit pause/drain/resume.
    pub overload: Option<u64>,
    /// World updates committed in this callback.
    pub completed_ticks: u32,
    /// Due updates left queued by the fixed clock.
    pub queued_ticks: u32,
    /// Changed game values copied from the last committed page for future replication.
    pub projected_changes: usize,
    /// Tick projections overwritten before a consumer drained them; baseline reset required.
    pub projection_lost_ticks: u64,
    /// A GPU view was submitted; `tick` remains the latest authoritative tick.
    /// The submitted source tick is reported by the eventual `FrameSample`.
    pub submitted: bool,
    /// An acquired platform surface received this frame in the same submission.
    pub surface_presented: bool,
    /// A fresh optional view was skipped; a stale scene may still be submitted.
    pub dropped_view: bool,
    /// CPU wall time for the entire callback, including nonblocking GPU polling.
    pub wall: Duration,
}

/// One admitted simulation callback awaiting optional presentation. Consuming
/// it once keeps a delayed surface acquisition tied to the exact world step.
pub struct AdvancedFrame<'a> {
    host: Option<&'a mut FrameHost>,
    report: Option<DriveReport>,
    tick_cpu: Duration,
    wall_start: Instant,
    recovered_publication: bool,
}

impl<'a> AdvancedFrame<'a> {
    /// Whether a serial frame can use a drawable after world admission.
    /// Native extraction may finish later and remains separately scheduled.
    pub fn can_acquire_serial(&self) -> bool {
        let Some(host) = self.host.as_deref() else {
            return false;
        };
        #[cfg(not(target_arch = "wasm32"))]
        if host.worker.is_some() {
            return false;
        }
        let Some(report) = self.report.as_ref() else {
            return false;
        };
        report.overload.is_none()
            && host.sim.has_current_page()
            && host.presenter.is_some()
            && host
                .slots
                .iter()
                .any(|slot| matches!(slot.state, SlotState::Free))
    }

    /// Encode and submit the admitted frame once, with an optional acquired surface.
    /// # Errors
    /// Returns a presentation fault after simulation admission; committed ticks
    /// remain committed even when optional rendering fails.
    pub fn submit(mut self, surface: Option<&SurfaceOutput<'_>>) -> Result<DriveReport, String> {
        let report = self
            .report
            .take()
            .ok_or("advanced frame already consumed")?;
        self.host
            .take()
            .ok_or("advanced host already consumed")?
            .finish_advance(
                report,
                self.tick_cpu,
                self.wall_start,
                self.recovered_publication,
                surface,
            )
    }

    /// Encode the serial scene while the platform has not yet acquired its
    /// drawable. The returned token owns the encoder and its frame slot.
    /// # Errors
    /// Returns an extraction or GPU encoding fault; a failed optional view
    /// never rolls back an admitted world update.
    pub fn encode_scene(mut self) -> Result<EncodedFrame<'a>, String> {
        if !self.can_acquire_serial() {
            return Err("serial scene is not ready for encoding".to_owned());
        }
        let host = self.host.as_deref_mut().ok_or("advanced host consumed")?;
        let slot_index = host
            .slots
            .iter()
            .position(|slot| matches!(slot.state, SlotState::Free))
            .ok_or("no free scene slot")?;
        let report = self.report.as_mut().ok_or("advanced report consumed")?;
        let frame =
            host.extract_serial(self.tick_cpu, report.completed_ticks, report.queued_ticks)?;
        let prepared = match host.prepare_extracted(slot_index, report, frame) {
            Ok(Some(prepared)) => prepared,
            Ok(None) => return Err("serial extraction had no current source".to_owned()),
            Err(error) => {
                host.queue.submit([]);
                return Err(error);
            }
        };
        let (encoder, scene_encode_cpu) = host.encode_scene_only(slot_index)?;
        Ok(EncodedFrame {
            host: self.host.take().ok_or("advanced host consumed")?,
            report: self.report.take(),
            encoder: Some(encoder),
            sample: Some(prepared.sample),
            slot_index,
            wall_start: self.wall_start,
            scene_encode_cpu,
            #[cfg(not(target_arch = "wasm32"))]
            source: prepared.source,
        })
    }
}

impl Drop for AdvancedFrame<'_> {
    fn drop(&mut self) {
        let Some((host, report)) = self.host.as_deref_mut().zip(self.report.as_ref()) else {
            return;
        };
        if report.overload.is_some() {
            host.dropped_views = host
                .dropped_views
                .saturating_add(u64::from(report.completed_ticks > 0));
        } else if host.sim.has_current_page()
            && (report.completed_ticks > 0 || self.recovered_publication)
        {
            host.dropped_views = host.dropped_views.saturating_add(1);
            if host.presenter.is_some() {
                host.dropped_surface = host.dropped_surface.saturating_add(1);
            }
        }
    }
}

/// One offscreen scene encoded before the platform acquires a drawable.
/// Dropping it discards the unsubmitted encoder and its timestamp reservations.
pub struct EncodedFrame<'a> {
    host: &'a mut FrameHost,
    report: Option<DriveReport>,
    encoder: Option<wgpu::CommandEncoder>,
    sample: Option<FrameSample>,
    slot_index: usize,
    wall_start: Instant,
    scene_encode_cpu: Duration,
    #[cfg(not(target_arch = "wasm32"))]
    source: SourceTick,
}

impl EncodedFrame<'_> {
    /// Discard an encoded scene when drawable acquisition failed. The world
    /// update remains committed; queued uploads are flushed without a scene
    /// pass or temporal-history publication.
    /// # Panics
    /// Panics only if the frame's privately owned report was taken without
    /// consuming the frame, which no public operation permits.
    pub fn skip(mut self) -> DriveReport {
        let mut report = self.report.take().expect("encoded report belongs to frame");
        report.dropped_view = true;
        report.wall = self.wall_start.elapsed();
        report
    }

    /// Append the presentation pass to the existing encoder and submit once.
    /// A missing drawable submits only the scene and counts the missed surface.
    /// # Errors
    /// Returns a GPU submission fault; committed world ticks remain committed.
    pub fn submit(mut self, surface: Option<&SurfaceOutput<'_>>) -> Result<DriveReport, String> {
        if surface.is_some() && self.host.presenter.is_none() {
            return Err("encoded host has no surface presenter".to_owned());
        }
        let mut encoder = self.encoder.take().ok_or("encoded scene consumed")?;
        let present_start = Instant::now();
        let slot = &mut self.host.slots[self.slot_index];
        let presented = if let (Some(presenter), Some(binding), Some(surface)) = (
            self.host.presenter.as_ref(),
            slot.presentation.as_ref(),
            surface,
        ) {
            presenter
                .encode(
                    &mut encoder,
                    binding,
                    surface.view,
                    surface.extent,
                    Some(&mut slot.timings),
                )
                .is_ok()
        } else {
            false
        };
        if self.host.presenter.is_some() && !presented {
            self.host.dropped_surface = self.host.dropped_surface.saturating_add(1);
            if let Some(report) = self.report.as_mut() {
                report.dropped_view = true;
            }
        }
        let encode_cpu = self.scene_encode_cpu + present_start.elapsed();
        let mut report = self.report.take().ok_or("encoded report consumed")?;
        let sample = self.sample.take().ok_or("encoded sample consumed")?;
        #[cfg(not(target_arch = "wasm32"))]
        let presentation_alpha = sample.presentation_alpha;
        self.host.finish_encoded(
            self.slot_index,
            &mut report,
            encoder,
            sample,
            self.wall_start,
            encode_cpu,
            presented,
        )?;
        self.host.prepared.commit_frame();
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.host.last_view_source = Some(self.source);
            self.host.last_render_slot = Some(self.slot_index);
            self.host.last_presentation_alpha = Some(presentation_alpha);
        }
        Ok(report)
    }
}

impl Drop for EncodedFrame<'_> {
    fn drop(&mut self) {
        if let Some(encoder) = self.encoder.take() {
            drop(encoder);
            let discarded = self.host.slots[self.slot_index]
                .timings
                .discard_unsubmitted(GENERATION);
            debug_assert!(discarded.is_ok(), "unsubmitted scene timing state changed");
            // Queue::write_buffer stages bytes until submit, even if the
            // command encoder is dropped. Empty submit releases that staging
            // after GPU completion without rendering the hidden scene.
            self.host.queue.submit([]);
            self.host.dropped_views = self.host.dropped_views.saturating_add(1);
            if self.host.presenter.is_some() {
                self.host.dropped_surface = self.host.dropped_surface.saturating_add(1);
            }
        }
    }
}

/// An acquired texture view supplied by the platform's single surface owner.
/// The platform calls `present` only when `DriveReport::surface_presented` is true.
pub struct SurfaceOutput<'a> {
    /// View of the exact acquired surface texture.
    pub view: &'a wgpu::TextureView,
    /// Its configured physical pixel extent.
    pub extent: [u32; 2],
}

/// Internal scene resolution and optional surface selected by the platform.
#[derive(Clone, Copy, Debug)]
pub struct FrameConfig {
    /// Scene/shadow-independent render extent; output may have another size.
    pub render_extent: [u32; 2],
    /// Current presentation target, absent for headless frames.
    pub surface: Option<SurfaceProfile>,
}

impl Default for FrameConfig {
    fn default() -> Self {
        Self {
            render_extent: DEFAULT_RENDER_EXTENT,
            surface: None,
        }
    }
}

enum SlotState {
    Free,
    Rendering(FrameSample),
    Resolving(FrameSample),
}

struct FrameSlot {
    target: OffscreenTarget,
    presentation: Option<PresentationBinding>,
    timings: GpuTimings,
    render_done: Arc<AtomicBool>,
    state: SlotState,
}

#[derive(Clone, Copy)]
struct SubmitFrame {
    extracted: ExtractReport,
    extract_cpu: Duration,
    worker_queue_cpu: Duration,
    worker_ready_cpu: Duration,
    tick_cpu: Duration,
    completed_ticks: u32,
    queued_ticks: u32,
    frame_tick: u64,
    presentation_alpha: f32,
}

struct PreparedSample {
    sample: FrameSample,
    #[cfg(not(target_arch = "wasm32"))]
    source: SourceTick,
}

/// A detached copy of the scene and executor handles for preparing one cooked
/// visual generation without borrowing the ticking host.
#[cfg(not(target_arch = "wasm32"))]
pub struct VisualPrepareInputs {
    device: wgpu::Device,
    queue: wgpu::Queue,
    capabilities: DeviceCapabilities,
    scene: SyntheticScene,
    surface: bool,
    render_extent: [u32; 2],
    owner: Arc<()>,
}

/// Prepared product and GPU targets awaiting an extraction-barrier selection.
/// Keep this value across a busy frame ring and retry publication later.
#[cfg(not(target_arch = "wasm32"))]
pub struct PreparedEnvironment {
    product: Option<EnvironmentProduct>,
    prepared: Option<PreparedSynthetic>,
    slots: Option<Vec<FrameSlot>>,
    owner: Arc<()>,
}

#[cfg(not(target_arch = "wasm32"))]
impl VisualPrepareInputs {
    /// Build a complete presentation generation off the authoritative tick path.
    ///
    /// # Errors
    ///
    /// Returns a GPU preparation fault without affecting the running host.
    pub async fn prepare(self, product: EnvironmentProduct) -> Result<PreparedEnvironment, String> {
        let light = photometric(&product);
        let prepared = PreparedSynthetic::prepare_with_capabilities(
            &self.capabilities,
            &self.queue,
            &self.scene,
            FrameLighting::Photometric(light),
            1.0,
        )
        .await
        .map_err(|error| format!("cooked visual prepare: {error:?}"))?;
        let mut slots = Vec::with_capacity(IN_FLIGHT);
        for _ in 0..IN_FLIGHT {
            slots.push(FrameSlot {
                target: OffscreenTarget::new(
                    &self.device,
                    &prepared,
                    self.render_extent[0],
                    self.render_extent[1],
                )
                .map_err(|error| format!("cooked target: {error:?}"))?,
                presentation: None,
                timings: GpuTimings::new(
                    &self.capabilities,
                    &self.queue,
                    prepared.scene_pass_count() + u32::from(self.surface),
                )
                .map_err(|error| format!("cooked timings: {error:?}"))?,
                render_done: Arc::new(AtomicBool::new(false)),
                state: SlotState::Free,
            });
        }
        Ok(PreparedEnvironment {
            product: Some(product),
            prepared: Some(prepared),
            slots: Some(slots),
            owner: self.owner,
        })
    }
}

/// One fixed-clock simulation and prepared GPU scene. The device and queue
/// belong to the caller's executor; this host creates no independent GPU owner.
/// The prepared target ring bounds in-flight work and defers GPU timing readback.
/// A platform adapter may also supply an acquired surface view for the
/// prepared presentation pass; the host never owns the surface lifecycle.
pub struct FrameHost {
    device: wgpu::Device,
    queue: wgpu::Queue,
    clock: TickClock,
    sim: Simulation,
    projections: Option<ProjectionOutbox>,
    desired_axes: [f32; 3],
    admitted_axes: [f32; 3],
    input_sequence: u64,
    extractor: Option<RenderExtractor>,
    #[cfg(not(target_arch = "wasm32"))]
    worker: Option<NativeWorker>,
    #[cfg(not(target_arch = "wasm32"))]
    worker_ready: Option<CompletedExtract>,
    #[cfg(not(target_arch = "wasm32"))]
    visual_generation: u64,
    #[cfg(not(target_arch = "wasm32"))]
    last_view_source: Option<SourceTick>,
    #[cfg(not(target_arch = "wasm32"))]
    last_render_slot: Option<usize>,
    #[cfg(not(target_arch = "wasm32"))]
    last_presentation_alpha: Option<f32>,
    scene: SyntheticScene,
    render_extent: [u32; 2],
    changed_slots: Vec<u32>,
    prepared: PreparedSynthetic,
    presenter: Option<SurfacePresenter>,
    slots: Vec<FrameSlot>,
    completed: VecDeque<FrameSample>,
    next_submission_order: u64,
    dropped_views: u64,
    dropped_stale_views: u64,
    #[cfg(not(target_arch = "wasm32"))]
    reused_views: u64,
    dropped_reports: u64,
    dropped_capture_rows: u64,
    dropped_surface: u64,
    #[cfg(not(target_arch = "wasm32"))]
    visual_product: Option<EnvironmentProduct>,
    #[cfg(not(target_arch = "wasm32"))]
    visual_owner: Arc<()>,
}

impl FrameHost {
    /// Maximum submitted scene frames awaiting GPU completion.
    pub const MAX_IN_FLIGHT: usize = IN_FLIGHT;
    /// Device generation shared by this provisional V1 host and its pipelines.
    pub const DEVICE_GENERATION: u64 = GENERATION;

    #[cfg_attr(
        target_arch = "wasm32",
        expect(
            clippy::unused_self,
            reason = "the browser has no cooked visual product; native frames select it through self"
        )
    )]
    fn frame_lighting(&self, tick: u64, alpha: f32) -> FrameLighting {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(product) = &self.visual_product {
            return FrameLighting::Photometric(photometric(product));
        }
        FrameLighting::Diagnostic(SyntheticLight {
            direction_to_light: [0.4 + frame_phase(tick, alpha).sin() * 0.1, -0.6, 1.0],
            ..GAME_SUN
        })
    }

    /// Prepare one 10k-entity scene, all four scene passes and an optional
    /// surface presentation pass before the first draw.
    /// # Errors
    /// Returns setup, device or scene preparation errors without a partial host.
    /// # Panics
    /// Panics only if the compiled prepared-frame ring size is zero.
    pub async fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: FrameConfig,
    ) -> Result<Self, String> {
        Self::new_inner(device, queue, None, config).await
    }

    /// Build the live V1 host from the platform's immutable adapter record.
    ///
    /// # Errors
    /// Refuses a mismatched device generation or ordinary scene preparation failure.
    pub async fn new_with_capabilities(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        capabilities: &DeviceCapabilities,
        config: FrameConfig,
    ) -> Result<Self, String> {
        if !capabilities.matches(device) || capabilities.generation() != GENERATION {
            return Err("capability record does not match the host device".to_owned());
        }
        Self::new_inner(device, queue, Some(capabilities), config).await
    }

    async fn new_inner(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        capabilities: Option<&DeviceCapabilities>,
        config: FrameConfig,
    ) -> Result<Self, String> {
        let fallback_capabilities = capabilities
            .is_none()
            .then(|| DeviceCapabilities::capture_device(device, GENERATION));
        let capabilities = capabilities
            .or(fallback_capabilities.as_ref())
            .expect("the caller supplies or constructs one capability record");
        let [width, height] = config.render_extent;
        if width == 0 || height == 0 {
            return Err("render extent must be nonzero".to_owned());
        }
        let mut sim = scene::simulation()?;
        let mut extractor = RenderExtractor::new(ENTITY_COUNT, ENTITY_COUNT);
        let initial = sim
            .published_page()
            .map_err(|error| format!("initial world page: {error:?}"))?;
        scene::extract(&mut extractor, &initial);
        drop(initial);
        let count = u32::try_from(ENTITY_COUNT).map_err(|_| "fixture exceeds u32 slots")?;
        let mut scene =
            SyntheticScene::grid(count).map_err(|error| format!("synthetic scene: {error:?}"))?;
        scene.view = SyntheticView::perspective(
            [0.0, 0.0, 2.0],
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            1.0,
            width as f32 / height as f32,
            0.1,
            20.0,
        )
        .map_err(|error| format!("camera: {error:?}"))?;
        scene.sun_shadows = true;
        scene.local_lights = [
            ([-0.55, -0.55, 1.0], [5.0, 0.8, 0.4]),
            ([0.55, -0.55, 1.0], [0.4, 4.0, 0.8]),
            ([-0.55, 0.55, 1.0], [0.7, 1.0, 5.0]),
            ([0.55, 0.55, 1.0], [4.0, 3.0, 0.4]),
        ]
        .map(|(position, candela)| SyntheticPointLight {
            position,
            radius: 0.6,
            candela,
        })
        .to_vec();
        let mut changed_slots = Vec::with_capacity(ENTITY_COUNT);
        scene::apply_frame_to_scene(&mut scene, extractor.frame(), &mut changed_slots);
        let prepared = PreparedSynthetic::prepare_with_capabilities(
            capabilities,
            queue,
            &scene,
            FrameLighting::Diagnostic(GAME_SUN),
            1.0,
        )
        .await
        .map_err(|error| format!("prepare: {error:?}"))?;
        let presenter = match config.surface {
            Some(profile) => Some(
                SurfacePresenter::prepare(capabilities, config.render_extent, profile)
                    .await
                    .map_err(|error| format!("surface prepare: {error:?}"))?,
            ),
            None => None,
        };
        let mut slots = Vec::with_capacity(IN_FLIGHT);
        for _ in 0..IN_FLIGHT {
            let target = OffscreenTarget::new(device, &prepared, width, height)
                .map_err(|error| format!("target: {error:?}"))?;
            slots.push(FrameSlot {
                presentation: presenter
                    .as_ref()
                    .map(|presenter| presenter.bind_target(device, &target))
                    .transpose()
                    .map_err(|error| format!("surface binding: {error:?}"))?,
                target,
                timings: GpuTimings::new(
                    capabilities,
                    queue,
                    prepared.scene_pass_count() + u32::from(presenter.is_some()),
                )
                .map_err(|error| format!("GPU timings: {error:?}"))?,
                render_done: Arc::new(AtomicBool::new(false)),
                state: SlotState::Free,
            });
        }
        let clock = TickClock::new(ClockConfig::default(), Duration::ZERO)
            .map_err(|error| format!("clock: {error:?}"))?;
        #[cfg(not(target_arch = "wasm32"))]
        let initial_source = SourceTick {
            epoch: sim.epoch().0,
            tick: sim.tick(),
            timeline: sim.timeline(),
        };
        Ok(Self {
            device: device.clone(),
            queue: queue.clone(),
            clock,
            sim,
            projections: None,
            desired_axes: [0.0; 3],
            admitted_axes: [0.0; 3],
            input_sequence: 1,
            extractor: Some(extractor),
            #[cfg(not(target_arch = "wasm32"))]
            worker: None,
            #[cfg(not(target_arch = "wasm32"))]
            worker_ready: None,
            #[cfg(not(target_arch = "wasm32"))]
            visual_generation: 0,
            #[cfg(not(target_arch = "wasm32"))]
            last_view_source: Some(initial_source),
            #[cfg(not(target_arch = "wasm32"))]
            #[cfg(not(target_arch = "wasm32"))]
            last_render_slot: None,
            #[cfg(not(target_arch = "wasm32"))]
            last_presentation_alpha: None,
            scene,
            render_extent: config.render_extent,
            changed_slots,
            prepared,
            presenter,
            slots,
            completed: VecDeque::with_capacity(IN_FLIGHT),
            next_submission_order: 0,
            dropped_views: 0,
            dropped_stale_views: 0,
            #[cfg(not(target_arch = "wasm32"))]
            reused_views: 0,
            dropped_reports: 0,
            dropped_capture_rows: 0,
            dropped_surface: 0,
            #[cfg(not(target_arch = "wasm32"))]
            visual_product: None,
            #[cfg(not(target_arch = "wasm32"))]
            visual_owner: Arc::new(()),
        })
    }

    /// Copy presentation inputs once for an asynchronous visual preparation.
    /// This copies the finite scene only on accepted edits, never on ticks.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn visual_prepare_inputs(&self) -> VisualPrepareInputs {
        VisualPrepareInputs {
            device: self.device.clone(),
            queue: self.queue.clone(),
            capabilities: self.prepared.capabilities().clone(),
            scene: self.scene.clone(),
            surface: self.presenter.is_some(),
            render_extent: self.render_extent,
            owner: Arc::clone(&self.visual_owner),
        }
    }

    /// Move native CPU extraction to one long-lived worker. World ticks remain
    /// on the caller and never wait for this optional presentation task.
    /// `wake` must notify the platform loop after a result becomes available.
    ///
    /// # Errors
    ///
    /// Returns a native worker startup fault, leaving serial extraction active.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn enable_native_worker(
        &mut self,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> Result<(), String> {
        if self.worker.is_some() {
            return Err("frame worker already enabled".to_owned());
        }
        self.worker = Some(NativeWorker::new(wake)?);
        Ok(())
    }

    /// A completed CPU view is waiting for a surface and free GPU slot.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn has_ready_view(&self) -> bool {
        self.worker_ready.is_some()
    }

    /// A previously prepared scene can be shown while the native worker is late.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn has_reusable_view(&self) -> bool {
        self.worker.as_ref().is_some_and(NativeWorker::is_busy)
            && self.last_view_source.is_some_and(|source| {
                source.epoch == self.sim.epoch().0 && source.timeline == self.sim.timeline()
            })
            && self.last_render_slot.is_some_and(|index| {
                self.slots
                    .get(index)
                    .is_some_and(|slot| matches!(slot.state, SlotState::Free))
            })
    }

    /// Surface frames drawn from an older prepared scene while extraction was busy.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn reused_views(&self) -> u64 {
        self.reused_views
    }

    /// Select a fully prepared cooked view at a GPU-safe extraction barrier.
    /// `false` retains the candidate for retry while older packets are live.
    ///
    /// # Errors
    ///
    /// Returns a binding or full-table upload fault without replacing the active view.
    ///
    /// # Panics
    ///
    /// Panics only if a checked candidate loses an owned resource during the
    /// final single-threaded move into the active host.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn publish_environment(
        &mut self,
        candidate: &mut PreparedEnvironment,
    ) -> Result<bool, String> {
        if !Arc::ptr_eq(&candidate.owner, &self.visual_owner) {
            return Err("visual candidate belongs to another host".to_owned());
        }
        self.poll_gpu()?;
        if self.pending_frames() != 0 {
            return Ok(false);
        }
        let slots = candidate
            .slots
            .as_mut()
            .ok_or_else(|| "visual candidate already published".to_owned())?;
        for slot in slots.iter_mut() {
            slot.presentation = self
                .presenter
                .as_ref()
                .map(|presenter| presenter.bind_target(&self.device, &slot.target))
                .transpose()
                .map_err(|error| format!("cooked surface binding: {error:?}"))?;
        }
        self.changed_slots.clear();
        for index in 0..self.scene.instances.len() {
            self.changed_slots
                .push(u32::try_from(index).map_err(|_| "visual instance slot exceeds u32")?);
        }
        candidate
            .prepared
            .as_mut()
            .ok_or_else(|| "visual candidate already published".to_owned())?
            .upload_instances(&self.queue, &self.scene.instances, &self.changed_slots)
            .map_err(|error| format!("cooked full upload: {error:?}"))?;
        self.prepared = candidate.prepared.take().expect("checked candidate");
        self.slots = candidate.slots.take().expect("checked candidate");
        self.visual_product = candidate.product.take();
        self.visual_generation = self.visual_generation.wrapping_add(1);
        self.last_view_source = None;
        self.last_render_slot = None;
        self.last_presentation_alpha = None;
        if let Some(extractor) = self.extractor.as_mut() {
            extractor.invalidate_history();
        }
        Ok(true)
    }

    /// Delay until the next world update is due; the platform owns actual sleep.
    pub fn next_due_in(&self) -> Option<Duration> {
        self.clock.next_due_in()
    }

    /// Pause wall-time admission while retaining already queued world updates.
    pub fn pause(&mut self) {
        self.clock.pause();
    }

    /// Resume from a new monotonic time origin after queued work has drained.
    ///
    /// # Errors
    /// Returns a clock error for an active clock, pending updates or backwards time.
    pub fn resume(&mut self, now: Duration) -> Result<(), String> {
        self.clock
            .resume(now)
            .map_err(|error| format!("clock resume: {error:?}"))
    }

    /// Finish previously admitted work before starting a fresh platform time
    /// origin. Hidden time and explicitly refused overload time stay outside
    /// the authoritative tick history.
    /// # Errors
    /// Returns a host fault if admitted ticks cannot drain or resume is invalid.
    pub fn drain_and_resume(&mut self, now: Duration) -> Result<(), String> {
        loop {
            let report = self.drive(now, None)?;
            if report.queued_ticks == 0 {
                break;
            }
            if report.completed_ticks == 0 {
                return Err("paused world updates could not drain".to_owned());
            }
        }
        self.resume(now)
    }

    /// Last committed world tick.
    pub fn tick(&self) -> u64 {
        self.sim.tick()
    }

    /// Sample continuous local controls for the next authoritative tick.
    /// Multiple UI changes before that tick use the latest normalized value.
    /// Local and remote inputs use the same `Simulation::admit_frame` gate.
    ///
    /// # Errors
    /// Returns a normalized-axis validation error without changing the held input.
    pub fn set_controls(&mut self, axes: [f32; 3]) -> Result<(), String> {
        scene::control_frame(scene::LOCAL_PEER, axes, 0, 0)
            .validate_axes()
            .map_err(|error| format!("local controls: {error:?}"))?;
        self.desired_axes = axes;
        Ok(())
    }

    /// Committed world position of the fixture's controlled actor.
    pub fn controlled_position(&self) -> Option<[f64; 3]> {
        self.sim
            .handle(scene::CONTROLLED_ID)
            .and_then(|handle| self.sim.get(handle).ok())
            .map(|entity| entity.position.0)
    }

    /// Number of optional views skipped because all GPU packet slots were held.
    pub fn dropped_views(&self) -> u64 {
        self.dropped_views
    }

    /// Completed native views discarded after their source or visual generation expired.
    pub fn dropped_stale_views(&self) -> u64 {
        self.dropped_stale_views
    }

    /// Completed profiler samples discarded when the caller did not drain the
    /// bounded result queue. This never affects authoritative ticks or GPU work.
    pub fn dropped_reports(&self) -> u64 {
        self.dropped_reports
    }

    /// Presentation rows omitted while refreshing an explicit capture.
    pub fn dropped_capture_rows(&self) -> u64 {
        self.dropped_capture_rows
    }

    /// Optional surface views omitted because acquisition or encode was unavailable.
    pub fn dropped_surface(&self) -> u64 {
        self.dropped_surface
    }

    /// Recompile the surface extent declaration after the platform configures
    /// a nonzero replacement; in-flight old textures remain GPU-owned.
    /// # Errors
    /// Refuses an invalid extent or a host without a surface presenter.
    pub fn resize_surface(&mut self, extent: [u32; 2]) -> Result<(), String> {
        self.presenter
            .as_mut()
            .ok_or_else(|| "host has no surface presenter".to_owned())?
            .resize(&self.device, extent)
            .map_err(|error| format!("surface resize: {error:?}"))
    }

    /// Number of submitted render packets awaiting GPU completion or readback.
    pub fn pending_frames(&self) -> usize {
        self.slots
            .iter()
            .filter(|slot| !matches!(slot.state, SlotState::Free))
            .count()
    }

    /// Progress native callbacks without blocking and return the oldest completed
    /// frame, if any. Browser WebGPU delivers callbacks through its event loop.
    /// # Errors
    /// Returns a device polling fault; optional timestamp failures are reported
    /// on the completed frame and do not stop the simulation.
    pub fn poll_completed(&mut self) -> Result<Option<FrameSample>, String> {
        self.poll_gpu()?;
        let oldest_pending = self
            .slots
            .iter()
            .filter_map(|slot| match slot.state {
                SlotState::Free => None,
                SlotState::Rendering(sample) | SlotState::Resolving(sample) => {
                    Some(sample.submission_order)
                }
            })
            .min();
        Ok(pop_report_before_pending(
            &mut self.completed,
            oldest_pending,
        ))
    }

    /// Start a bounded projection output from the current complete world page.
    /// G6 will attach its transport consumer here; the local game remains lean
    /// when no session is active.
    ///
    /// # Errors
    /// Returns a world publication fault without enabling the output.
    pub fn enable_projection(&mut self) -> Result<(), String> {
        if self.projections.is_none() {
            let page = self
                .sim
                .published_page()
                .map_err(|error| format!("world page: {error:?}"))?;
            let mut outbox = ProjectionOutbox::new(ENTITY_COUNT);
            outbox.reset_full(&page);
            self.projections = Some(outbox);
        }
        Ok(())
    }

    /// Consume the oldest detached projection in source-tick order. A gap
    /// blocks delivery until `reset_projection_baseline` publishes a full state.
    /// Returns `None` when projection output has not been enabled.
    ///
    /// # Errors
    /// Returns the explicit number of overwritten ticks requiring a baseline.
    pub fn poll_projection(
        &mut self,
        consume: impl FnOnce(&GameProjection),
    ) -> Result<Option<bool>, ProjectionGap> {
        self.projections
            .as_mut()
            .map(|outbox| outbox.poll(self.sim.timeline(), consume))
            .transpose()
    }

    /// Reserved detached projection payload bytes, excluding allocator bookkeeping.
    pub fn projection_reserved_bytes(&self) -> usize {
        self.projections
            .as_ref()
            .map_or(0, ProjectionOutbox::reserved_bytes)
    }

    /// Replace undeliverable deltas with a full current-state projection.
    ///
    /// # Errors
    /// Returns a world publication fault without changing the pending outbox.
    pub fn reset_projection_baseline(&mut self) -> Result<(), String> {
        let outbox = self
            .projections
            .as_mut()
            .ok_or("projection output is disabled")?;
        let page = self
            .sim
            .published_page()
            .map_err(|error| format!("world page: {error:?}"))?;
        outbox.reset_full(&page);
        Ok(())
    }

    /// Admit monotonic host time through the shared fixed-step clock. Every due
    /// update commits once before a visual page can be published. A full
    /// presentation ring skips an optional view with a counter, never the world
    /// update. Native worker completion can submit on a zero-update callback;
    /// serial extraction submits only after a due update. Overload is explicit.
    /// # Errors
    /// Returns a clock, simulation or GPU fault; never silently skips an update.
    pub fn drive(
        &mut self,
        now: Duration,
        surface: Option<&SurfaceOutput<'_>>,
    ) -> Result<DriveReport, String> {
        if surface.is_some() && self.presenter.is_none() {
            return Err("host has no surface presenter".to_owned());
        }
        self.advance(now)?.submit(surface)
    }

    /// Admit authoritative time before deciding whether to acquire a drawable.
    /// The returned frame owns the exclusive host borrow until it is submitted
    /// or dropped; a dropped frame skips only optional presentation.
    /// # Errors
    /// Returns a clock or simulation fault without an incomplete world update.
    pub fn advance(&mut self, now: Duration) -> Result<AdvancedFrame<'_>, String> {
        let wall_start = Instant::now();
        self.poll_gpu()?;
        let mut tick_cpu = Duration::ZERO;
        let desired_axes = self.desired_axes;
        let admitted_axes = &mut self.admitted_axes;
        let input_sequence = &mut self.input_sequence;
        let sim = &mut self.sim;
        let projections = &mut self.projections;
        let mut projected_changes = 0;
        let mut skipped_publications = 0_u64;
        let clock = self
            .clock
            .advance(now, 4, || {
                let start = Instant::now();
                let result = (|| {
                    if desired_axes
                        .iter()
                        .zip(admitted_axes.iter())
                        .any(|(desired, admitted)| desired.to_bits() != admitted.to_bits())
                    {
                        let tick = sim.tick().checked_add(1).ok_or("input tick exhausted")?;
                        let frame = scene::control_frame(
                            scene::LOCAL_PEER,
                            desired_axes,
                            tick,
                            *input_sequence,
                        );
                        sim.admit_frame(frame)
                            .map_err(|error| format!("local input admission: {error:?}"))?;
                        *admitted_axes = desired_axes;
                        *input_sequence = input_sequence
                            .checked_add(1)
                            .ok_or("input sequence exhausted")?;
                    }
                    sim.step(&[])
                        .map_err(|error| format!("simulation step: {error:?}"))?;
                    if sim.has_current_page() {
                        let page = sim
                            .published_page()
                            .map_err(|error| format!("world page: {error:?}"))?;
                        if let Some(outbox) = projections.as_mut() {
                            if page.is_baseline() {
                                outbox.reset_full(&page);
                                projected_changes += page.len();
                            } else {
                                projected_changes += outbox.capture(&page);
                            }
                        }
                    } else {
                        skipped_publications = skipped_publications.saturating_add(1);
                    }
                    Ok::<(), String>(())
                })();
                tick_cpu += start.elapsed();
                result
            })
            .map_err(|error| format!("clock advance: {error:?}"))?;
        if let Some(error) = clock.step_error {
            return Err(format!("simulation tick failed: {error:?}"));
        }
        let recovered_publication = if self.sim.has_current_page() {
            false
        } else {
            match self.sim.published_page() {
                Ok(page) => {
                    if let Some(outbox) = self.projections.as_mut() {
                        projected_changes += outbox.capture(&page);
                    }
                    true
                }
                Err(SimError::PublicationBusy) => false,
                Err(error) => return Err(format!("world page: {error:?}")),
            }
        };
        let report = DriveReport {
            tick: self.sim.tick(),
            overload: (clock.state == ClockState::Overloaded).then_some(clock.refused_on_overload),
            completed_ticks: clock.completed,
            queued_ticks: clock.queued,
            projected_changes,
            projection_lost_ticks: self
                .projections
                .as_ref()
                .map_or(0, ProjectionOutbox::lost_ticks),
            submitted: false,
            surface_presented: false,
            dropped_view: skipped_publications != 0,
            wall: Duration::ZERO,
        };
        self.dropped_views = self.dropped_views.saturating_add(skipped_publications);
        Ok(AdvancedFrame {
            host: Some(self),
            report: Some(report),
            tick_cpu,
            wall_start,
            recovered_publication,
        })
    }

    fn finish_advance(
        &mut self,
        mut report: DriveReport,
        tick_cpu: Duration,
        wall_start: Instant,
        recovered_publication: bool,
        surface: Option<&SurfaceOutput<'_>>,
    ) -> Result<DriveReport, String> {
        if surface.is_some() && self.presenter.is_none() {
            return Err("host has no surface presenter".to_owned());
        }
        if report.overload.is_some() {
            if report.completed_ticks > 0 {
                self.dropped_views = self.dropped_views.saturating_add(1);
                report.dropped_view = true;
            }
            report.wall = wall_start.elapsed();
            return Ok(report);
        }
        if !self.sim.has_current_page() {
            report.wall = wall_start.elapsed();
            return Ok(report);
        }
        #[cfg(not(target_arch = "wasm32"))]
        if self.worker.is_some() {
            self.drive_native_view(&mut report, tick_cpu, wall_start, surface)?;
            report.wall = wall_start.elapsed();
            return Ok(report);
        }
        if report.completed_ticks == 0 && !recovered_publication && surface.is_none() {
            report.wall = wall_start.elapsed();
            return Ok(report);
        }
        if self.presenter.is_some()
            && surface.is_none_or(|surface| {
                self.presenter
                    .as_ref()
                    .is_none_or(|presenter| presenter.extent() != surface.extent)
            })
        {
            self.dropped_views += 1;
            self.dropped_surface += 1;
            report.dropped_view = true;
            report.wall = wall_start.elapsed();
            return Ok(report);
        }
        let Some(slot_index) = self
            .slots
            .iter()
            .position(|slot| matches!(slot.state, SlotState::Free))
        else {
            self.dropped_views += 1;
            report.dropped_view = true;
            report.wall = wall_start.elapsed();
            return Ok(report);
        };

        let completed_ticks = report.completed_ticks;
        let queued_ticks = report.queued_ticks;
        let frame = self.extract_serial(tick_cpu, completed_ticks, queued_ticks)?;
        self.submit_extracted(slot_index, &mut report, surface, frame, wall_start)?;
        report.wall = wall_start.elapsed();
        Ok(report)
    }

    fn extract_serial(
        &mut self,
        tick_cpu: Duration,
        completed_ticks: u32,
        queued_ticks: u32,
    ) -> Result<SubmitFrame, String> {
        let extract_start = Instant::now();
        let snapshot = self
            .sim
            .published_page()
            .map_err(|error| format!("world page: {error:?}"))?;
        let extracted = scene::extract(
            self.extractor
                .as_mut()
                .ok_or("serial extractor unavailable")?,
            &snapshot,
        );
        drop(snapshot);
        Ok(SubmitFrame {
            extracted,
            extract_cpu: extract_start.elapsed(),
            worker_queue_cpu: Duration::ZERO,
            worker_ready_cpu: Duration::ZERO,
            tick_cpu,
            completed_ticks,
            queued_ticks,
            frame_tick: self.sim.tick(),
            presentation_alpha: self.clock.presentation_alpha(),
        })
    }

    fn submit_extracted(
        &mut self,
        slot_index: usize,
        report: &mut DriveReport,
        surface: Option<&SurfaceOutput<'_>>,
        frame: SubmitFrame,
        wall_start: Instant,
    ) -> Result<(), String> {
        let Some(prepared) = self.prepare_extracted(slot_index, report, frame)? else {
            return Ok(());
        };
        #[cfg(not(target_arch = "wasm32"))]
        let presentation_alpha = prepared.sample.presentation_alpha;
        self.encode_prepared(slot_index, report, surface, prepared.sample, wall_start)?;
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.last_view_source = Some(prepared.source);
            self.last_render_slot = Some(slot_index);
            self.last_presentation_alpha = Some(presentation_alpha);
        }
        Ok(())
    }

    fn prepare_extracted(
        &mut self,
        slot_index: usize,
        report: &mut DriveReport,
        frame: SubmitFrame,
    ) -> Result<Option<PreparedSample>, String> {
        #[cfg(target_arch = "wasm32")]
        let _ = report;
        #[cfg(not(target_arch = "wasm32"))]
        let frame_source = self
            .extractor
            .as_ref()
            .and_then(|extractor| extractor.frame().source());
        #[cfg(not(target_arch = "wasm32"))]
        let Some(frame_source) = frame_source else {
            self.dropped_stale_views = self.dropped_stale_views.saturating_add(1);
            report.dropped_view = true;
            return Ok(None);
        };
        #[cfg(not(target_arch = "wasm32"))]
        if frame_source.tick != frame.frame_tick
            || frame_source.epoch != self.sim.epoch().0
            || frame_source.timeline != self.sim.timeline()
        {
            self.dropped_stale_views = self.dropped_stale_views.saturating_add(1);
            report.dropped_view = true;
            return Ok(None);
        }
        let upload_start = Instant::now();
        let ApplyReport {
            moving,
            dropped_invalid,
        } = scene::apply_frame_to_scene(
            &mut self.scene,
            self.extractor
                .as_ref()
                .expect("completed extractor belongs to host")
                .frame(),
            &mut self.changed_slots,
        );
        self.scene.view = frame_view(
            frame.frame_tick,
            frame.presentation_alpha,
            self.render_extent,
        )?;
        let lighting = self.frame_lighting(frame.frame_tick, frame.presentation_alpha);
        self.prepared
            .synchronize_history(&self.queue, self.sim.epoch().0, self.sim.timeline());
        let view_update_start = Instant::now();
        self.prepared
            .update_frame(
                &self.queue,
                &mut self.slots[slot_index].target,
                &self.scene,
                lighting,
                frame.presentation_alpha,
            )
            .map_err(|error| format!("live view update: {error:?}"))?;
        let view_update_cpu = view_update_start.elapsed();
        let uploaded = self
            .prepared
            .upload_instances(&self.queue, &self.scene.instances, &self.changed_slots)
            .map_err(|error| format!("instance update: {error:?}"))?;
        let upload_cpu = upload_start.elapsed();
        #[cfg(not(target_arch = "wasm32"))]
        let visual_revision = self.current_visual_revision();
        #[cfg(target_arch = "wasm32")]
        let visual_revision = None;
        Ok(Some(PreparedSample {
            sample: FrameSample {
                tick: frame.frame_tick,
                presentation_alpha: frame.presentation_alpha,
                submission_order: 0,
                completed_ticks: frame.completed_ticks,
                queued_ticks: frame.queued_ticks,
                tick_cpu: frame.tick_cpu,
                extract_cpu: frame.extract_cpu,
                worker_queue_ns: duration_ns_u32(frame.worker_queue_cpu),
                worker_ready_ns: duration_ns_u32(frame.worker_ready_cpu),
                visual_age_ticks: u32::try_from(self.sim.tick().saturating_sub(frame.frame_tick))
                    .unwrap_or(u32::MAX),
                reused_visual: false,
                upload_cpu,
                view_update_ns: duration_ns_u32(view_update_cpu),
                encode_cpu: Duration::ZERO,
                wall: Duration::ZERO,
                gpu_pass_ns: None,
                gpu_present_ns: None,
                surface_presented: false,
                timing_fault: false,
                uploaded_rows: u32::try_from(uploaded.changed_rows).unwrap_or(u32::MAX),
                uploaded_bytes: uploaded.bytes,
                moving: u32::try_from(moving).unwrap_or(u32::MAX),
                dropped_presentation: u32::try_from(
                    frame.extracted.dropped_capacity
                        + frame.extracted.dropped_duplicate
                        + frame.extracted.dropped_invalid
                        + dropped_invalid
                        + uploaded.dropped_invalid
                        + self.prepared.dropped_light_sources(),
                )
                .unwrap_or(u32::MAX),
                dropped_light_references: self.prepared.dropped_light_references(),
                clamped_light_channels: u32::try_from(self.prepared.presentation_light_clamps())
                    .unwrap_or(u32::MAX),
                clamped_shadow_casters: u32::try_from(uploaded.clamped_shadow_casters)
                    .unwrap_or(u32::MAX),
                visual_revision,
            },
            #[cfg(not(target_arch = "wasm32"))]
            source: frame_source,
        }))
    }

    #[cfg_attr(
        not(target_arch = "wasm32"),
        expect(
            clippy::large_types_passed_by_value,
            reason = "ownership transfers the sample into an in-flight GPU slot; borrowing would require the same copy"
        )
    )]
    fn encode_prepared(
        &mut self,
        slot_index: usize,
        report: &mut DriveReport,
        surface: Option<&SurfaceOutput<'_>>,
        sample: FrameSample,
        wall_start: Instant,
    ) -> Result<(), String> {
        let (mut encoder, scene_encode_cpu) = self.encode_scene_only(slot_index)?;
        let present_start = Instant::now();
        let slot = &mut self.slots[slot_index];
        let surface_presented = if let (Some(presenter), Some(binding), Some(surface)) =
            (self.presenter.as_ref(), slot.presentation.as_ref(), surface)
        {
            if presenter
                .encode(
                    &mut encoder,
                    binding,
                    surface.view,
                    surface.extent,
                    Some(&mut slot.timings),
                )
                .is_ok()
            {
                true
            } else {
                self.dropped_surface += 1;
                false
            }
        } else {
            false
        };
        self.finish_encoded(
            slot_index,
            report,
            encoder,
            sample,
            wall_start,
            scene_encode_cpu + present_start.elapsed(),
            surface_presented,
        )?;
        self.prepared.commit_frame();
        Ok(())
    }

    fn encode_scene_only(
        &mut self,
        slot_index: usize,
    ) -> Result<(wgpu::CommandEncoder, Duration), String> {
        let start = Instant::now();
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let slot = &mut self.slots[slot_index];
        if let Err(error) = self.prepared.encode(
            &mut encoder,
            GENERATION,
            &slot.target,
            Some(&mut slot.timings),
        ) {
            drop(encoder);
            let discarded = slot.timings.discard_unsubmitted(GENERATION);
            self.queue.submit([]);
            discarded.map_err(|timing| format!("render timing discard: {timing:?}"))?;
            return Err(format!("render encode: {error:?}"));
        }
        Ok((encoder, start.elapsed()))
    }

    fn finish_encoded(
        &mut self,
        slot_index: usize,
        report: &mut DriveReport,
        encoder: wgpu::CommandEncoder,
        mut sample: FrameSample,
        wall_start: Instant,
        encode_cpu_so_far: Duration,
        surface_presented: bool,
    ) -> Result<(), String> {
        let finish_start = Instant::now();
        let slot = &mut self.slots[slot_index];
        if let Err(error) = slot.timings.finish_render(GENERATION, &encoder) {
            drop(encoder);
            let discarded = slot.timings.discard_unsubmitted(GENERATION);
            self.queue.submit([]);
            discarded
                .map_err(|timing| format!("failed timing discard after {error:?}: {timing:?}"))?;
            return Err(format!("finish timestamps: {error:?}"));
        }
        slot.render_done.store(false, Ordering::Release);
        self.queue.submit([encoder.finish()]);
        let done = Arc::clone(&slot.render_done);
        self.queue
            .on_submitted_work_done(move || done.store(true, Ordering::Release));
        sample.encode_cpu = encode_cpu_so_far + finish_start.elapsed();
        sample.wall = wall_start.elapsed();
        sample.surface_presented = surface_presented;
        sample.submission_order = self.next_submission_order;
        self.next_submission_order = self.next_submission_order.wrapping_add(1);
        slot.state = SlotState::Rendering(sample);
        report.submitted = true;
        report.surface_presented = surface_presented;
        report.wall = wall_start.elapsed();
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn submit_reused_view(
        &mut self,
        slot_index: usize,
        report: &mut DriveReport,
        surface: &SurfaceOutput<'_>,
        source: SourceTick,
        tick_cpu: Duration,
        wall_start: Instant,
    ) -> Result<(), String> {
        let encode_start = Instant::now();
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let slot = &mut self.slots[slot_index];
        self.presenter
            .as_ref()
            .ok_or("reused view needs a presenter")?
            .encode(
                &mut encoder,
                slot.presentation
                    .as_ref()
                    .ok_or("reused view source binding unavailable")?,
                surface.view,
                surface.extent,
                Some(&mut slot.timings),
            )
            .map_err(|error| format!("reused surface encode: {error:?}"))?;
        self.finish_encoded(
            slot_index,
            report,
            encoder,
            FrameSample {
                tick: source.tick,
                presentation_alpha: self.last_presentation_alpha.unwrap_or(1.0),
                submission_order: 0,
                completed_ticks: report.completed_ticks,
                queued_ticks: report.queued_ticks,
                tick_cpu,
                extract_cpu: Duration::ZERO,
                worker_queue_ns: 0,
                worker_ready_ns: 0,
                visual_age_ticks: u32::try_from(self.sim.tick().saturating_sub(source.tick))
                    .unwrap_or(u32::MAX),
                reused_visual: true,
                upload_cpu: Duration::ZERO,
                view_update_ns: 0,
                encode_cpu: Duration::ZERO,
                wall: Duration::ZERO,
                gpu_pass_ns: None,
                gpu_present_ns: None,
                surface_presented: false,
                timing_fault: false,
                uploaded_rows: 0,
                uploaded_bytes: 0,
                moving: 0,
                dropped_presentation: 0,
                dropped_light_references: self.prepared.dropped_light_references(),
                clamped_light_channels: u32::try_from(self.prepared.presentation_light_clamps())
                    .unwrap_or(u32::MAX),
                clamped_shadow_casters: 0,
                visual_revision: self.current_visual_revision(),
            },
            wall_start,
            encode_start.elapsed(),
            true,
        )?;
        self.reused_views = self.reused_views.saturating_add(1);
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn drive_native_view(
        &mut self,
        report: &mut DriveReport,
        tick_cpu: Duration,
        wall_start: Instant,
        surface: Option<&SurfaceOutput<'_>>,
    ) -> Result<(), String> {
        if let Some(completed) = self
            .worker
            .as_mut()
            .expect("native worker enabled")
            .try_complete()?
        {
            debug_assert!(self.worker_ready.is_none());
            self.worker_ready = Some(completed);
        }
        if self.worker_ready.is_none() {
            if self
                .worker
                .as_ref()
                .expect("native worker enabled")
                .is_busy()
            {
                if report.completed_ticks > 0 {
                    self.dropped_views = self.dropped_views.saturating_add(1);
                    report.dropped_view = true;
                }
                self.try_submit_reused_view(report, tick_cpu, wall_start, surface)?;
            } else {
                self.dispatch_latest_native(report, tick_cpu)?;
            }
            return Ok(());
        }
        if self.presenter.is_some()
            && surface.is_none_or(|surface| {
                self.presenter
                    .as_ref()
                    .is_none_or(|presenter| presenter.extent() != surface.extent)
            })
        {
            if report.completed_ticks > 0 {
                self.dropped_views = self.dropped_views.saturating_add(1);
                report.dropped_view = true;
            }
            return Ok(());
        }
        let Some(slot_index) = self
            .slots
            .iter()
            .position(|slot| matches!(slot.state, SlotState::Free))
        else {
            if report.completed_ticks > 0 {
                self.dropped_views = self.dropped_views.saturating_add(1);
                report.dropped_view = true;
            }
            return Ok(());
        };
        let completed = self.worker_ready.take().expect("checked ready view");
        let stale = completed.epoch != self.sim.epoch().0
            || completed.timeline != self.sim.timeline()
            || completed.generation != self.visual_generation
            || self.sim.tick().saturating_sub(completed.tick) > 2;
        self.extractor = Some(completed.extractor);
        let frame = if stale {
            self.dropped_stale_views = self.dropped_stale_views.saturating_add(1);
            let extractor = self.extractor.as_mut().expect("returned extractor");
            extractor.invalidate_history();
            let start = Instant::now();
            let snapshot = self
                .sim
                .published_page()
                .map_err(|error| format!("world page: {error:?}"))?;
            let extracted = scene::extract(extractor, &snapshot);
            drop(snapshot);
            SubmitFrame {
                extracted,
                extract_cpu: start.elapsed(),
                worker_queue_cpu: completed.queue_wait,
                worker_ready_cpu: completed.completed_at.elapsed(),
                tick_cpu,
                completed_ticks: report.completed_ticks,
                queued_ticks: report.queued_ticks,
                frame_tick: self.sim.tick(),
                presentation_alpha: self.clock.presentation_alpha(),
            }
        } else {
            SubmitFrame {
                extracted: completed.report,
                extract_cpu: completed.publication_cpu + completed.cpu,
                worker_queue_cpu: completed.queue_wait,
                worker_ready_cpu: completed.completed_at.elapsed(),
                tick_cpu: completed.tick_cpu,
                completed_ticks: completed.completed_ticks,
                queued_ticks: completed.queued_ticks,
                frame_tick: completed.tick,
                presentation_alpha: if completed.tick == self.sim.tick() {
                    self.clock.presentation_alpha()
                } else {
                    1.0
                },
            }
        };
        self.submit_extracted(slot_index, report, surface, frame, wall_start)?;
        self.dispatch_latest_native(report, tick_cpu)?;
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn dispatch_latest_native(
        &mut self,
        report: &DriveReport,
        tick_cpu: Duration,
    ) -> Result<(), String> {
        if !self.sim.has_current_page()
            || self
                .worker
                .as_ref()
                .expect("native worker enabled")
                .is_busy()
            || self.extractor.is_none()
            || self.last_view_source.is_some_and(|source| {
                source.epoch == self.sim.epoch().0
                    && source.timeline == self.sim.timeline()
                    && source.tick == self.sim.tick()
            })
        {
            return Ok(());
        }
        let publication_start = Instant::now();
        let snapshot = self
            .sim
            .published_page()
            .map_err(|error| format!("world page: {error:?}"))?;
        let publication_cpu = publication_start.elapsed();
        let extractor = self
            .extractor
            .take()
            .expect("idle worker returns extractor");
        self.worker
            .as_mut()
            .expect("native worker enabled")
            .dispatch(
                extractor,
                snapshot,
                ExtractRequest {
                    generation: self.visual_generation,
                    publication_cpu,
                    tick_cpu,
                    completed_ticks: report.completed_ticks,
                    queued_ticks: report.queued_ticks,
                },
            );
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn try_submit_reused_view(
        &mut self,
        report: &mut DriveReport,
        tick_cpu: Duration,
        wall_start: Instant,
        surface: Option<&SurfaceOutput<'_>>,
    ) -> Result<(), String> {
        let Some((surface, source)) = surface.zip(self.last_view_source) else {
            return Ok(());
        };
        if source.epoch != self.sim.epoch().0
            || source.timeline != self.sim.timeline()
            || self
                .presenter
                .as_ref()
                .is_none_or(|presenter| presenter.extent() != surface.extent)
        {
            return Ok(());
        }
        let Some(slot_index) = self.last_render_slot.filter(|&index| {
            self.slots
                .get(index)
                .is_some_and(|slot| matches!(slot.state, SlotState::Free))
        }) else {
            return Ok(());
        };
        self.submit_reused_view(slot_index, report, surface, source, tick_cpu, wall_start)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn current_visual_revision(&self) -> Option<[u8; 32]> {
        self.visual_product
            .as_ref()
            .map(|product| *product.revision().as_bytes())
    }

    /// Render and read back the current committed scene once for visual evidence.
    /// This capture is outside the timed frame path.
    /// # Errors
    /// Returns a GPU encode, completion or readback error.
    pub fn capture_pixels(&mut self) -> Result<Vec<u8>, String> {
        #[cfg(not(target_arch = "wasm32"))]
        self.reclaim_extractor_for_capture()?;
        let needs_extract = self
            .extractor
            .as_ref()
            .ok_or("capture extractor unavailable")?
            .frame()
            .source()
            .is_none_or(|source| {
                source.epoch != self.sim.epoch().0
                    || source.timeline != self.sim.timeline()
                    || source.tick != self.sim.tick()
            });
        let extracted = if needs_extract {
            let snapshot = self
                .sim
                .published_page()
                .map_err(|error| format!("world page: {error:?}"))?;
            let extracted = scene::extract(
                self.extractor
                    .as_mut()
                    .ok_or("capture extractor unavailable")?,
                &snapshot,
            );
            drop(snapshot);
            extracted
        } else {
            ExtractReport::default()
        };
        // A capture describes the current committed endpoint, regardless of
        // the last display sample or a delayed native extraction worker.
        let capture_alpha = 1.0;
        let applied = scene::apply_frame_to_scene(
            &mut self.scene,
            self.extractor
                .as_ref()
                .ok_or("capture extractor unavailable")?
                .frame(),
            &mut self.changed_slots,
        );
        let uploaded = self
            .prepared
            .upload_instances(&self.queue, &self.scene.instances, &self.changed_slots)
            .map_err(|error| format!("capture instance update: {error:?}"))?;
        let dropped = u64::try_from(
            extracted.dropped_capacity
                + extracted.dropped_duplicate
                + extracted.dropped_invalid
                + applied.dropped_invalid
                + uploaded.dropped_invalid,
        )
        .unwrap_or(u64::MAX);
        self.dropped_capture_rows = self.dropped_capture_rows.saturating_add(dropped);
        self.scene.view = frame_view(self.sim.tick(), capture_alpha, self.render_extent)?;
        let lighting = self.frame_lighting(self.sim.tick(), capture_alpha);
        let mut target = OffscreenTarget::new(
            &self.device,
            &self.prepared,
            self.render_extent[0],
            self.render_extent[1],
        )
        .map_err(|error| format!("capture target: {error:?}"))?;
        self.prepared
            .update_frame(
                &self.queue,
                &mut target,
                &self.scene,
                lighting,
                capture_alpha,
            )
            .map_err(|error| format!("capture view update: {error:?}"))?;
        let mut pixels = target.readback(&self.device);
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        self.prepared
            .encode_with_readback(&mut encoder, GENERATION, &target, None, &mut pixels)
            .map_err(|error| format!("capture render/readback: {error:?}"))?;
        let submitted = self.queue.submit([encoder.finish()]);
        self.wait(submitted, "capture")?;
        pixels
            .try_collect(GENERATION)
            .map_err(|error| format!("capture collect: {error:?}"))?
            .ok_or_else(|| "capture readback incomplete".to_owned())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn reclaim_extractor_for_capture(&mut self) -> Result<(), String> {
        if self.extractor.is_some() {
            return Ok(());
        }
        let completed = if let Some(ready) = self.worker_ready.take() {
            ready
        } else {
            self.worker
                .as_mut()
                .expect("worker owns absent extractor")
                .wait_complete(Duration::from_secs(2))?
        };
        let mut extractor = completed.extractor;
        // The returned columns have not been mapped or uploaded to the scene.
        // Force the ordinary capture refresh even when their source tick equals
        // the current world tick.
        extractor.invalidate_history();
        self.extractor = Some(extractor);
        self.dropped_views = self.dropped_views.saturating_add(1);
        Ok(())
    }

    fn wait(&self, submitted: wgpu::SubmissionIndex, operation: &str) -> Result<(), String> {
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submitted),
                timeout: Some(GPU_WAIT),
            })
            .map_err(|error| format!("{operation} poll: {error:?}"))?;
        Ok(())
    }

    fn poll_gpu(&mut self) -> Result<(), String> {
        self.device
            .poll(wgpu::PollType::Poll)
            .map_err(|error| format!("GPU poll: {error:?}"))?;
        let max_passes = self.prepared.scene_pass_count() + u32::from(self.presenter.is_some());
        for slot in &mut self.slots {
            let state = std::mem::replace(&mut slot.state, SlotState::Free);
            match state {
                SlotState::Free => {}
                SlotState::Rendering(mut sample) => {
                    if !slot.render_done.load(Ordering::Acquire) {
                        slot.state = SlotState::Rendering(sample);
                        continue;
                    }
                    if !slot.timings.is_enabled() {
                        retain_sample(&mut self.completed, &mut self.dropped_reports, sample);
                        continue;
                    }
                    let mut encoder = self
                        .device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                    match slot.timings.resolve_if_ready(GENERATION, &mut encoder) {
                        Ok(true) => {
                            self.queue.submit([encoder.finish()]);
                            slot.state = SlotState::Resolving(sample);
                        }
                        Ok(false) => slot.state = SlotState::Rendering(sample),
                        Err(_) => {
                            sample.timing_fault = true;
                            slot.timings =
                                new_timings(self.prepared.capabilities(), &self.queue, max_passes)?;
                            retain_sample(&mut self.completed, &mut self.dropped_reports, sample);
                        }
                    }
                }
                SlotState::Resolving(mut sample) => match slot.timings.try_collect(GENERATION) {
                    Ok(Some(passes)) => {
                        if let Some((scene, present)) =
                            pass_durations(&passes, &sample, &self.prepared)
                        {
                            sample.gpu_pass_ns = Some(scene);
                            sample.gpu_present_ns = present;
                        } else {
                            sample.timing_fault = true;
                        }
                        retain_sample(&mut self.completed, &mut self.dropped_reports, sample);
                    }
                    Ok(None) => slot.state = SlotState::Resolving(sample),
                    Err(_) => {
                        sample.timing_fault = true;
                        slot.timings =
                            new_timings(self.prepared.capabilities(), &self.queue, max_passes)?;
                        retain_sample(&mut self.completed, &mut self.dropped_reports, sample);
                    }
                },
            }
        }
        Ok(())
    }
}

fn new_timings(
    capabilities: &DeviceCapabilities,
    queue: &wgpu::Queue,
    max_passes: u32,
) -> Result<GpuTimings, String> {
    GpuTimings::new(capabilities, queue, max_passes)
        .map_err(|error| format!("GPU timing recovery: {error:?}"))
}

fn pass_durations(
    passes: &[thrum_render::timing::PassTiming],
    sample: &FrameSample,
    prepared: &PreparedSynthetic,
) -> Option<([f64; 6], Option<f64>)> {
    const ALL: [&str; 6] = [
        "synthetic Sun shadow near",
        "synthetic Sun shadow far",
        "synthetic cluster build",
        "synthetic forward opaque",
        "synthetic transform history",
        "synthetic tone encode",
    ];
    const SHADOW_ONLY: [&str; 5] = [ALL[0], ALL[1], ALL[3], ALL[4], ALL[5]];
    const CLUSTER_ONLY: [&str; 4] = [ALL[2], ALL[3], ALL[4], ALL[5]];
    let presented = sample.surface_presented;
    if sample.reused_visual {
        return (passes.len() == 1
            && presented
            && passes[0].name.as_ref() == thrum_render::present::PASS_NAME)
            .then(|| ([0.0; 6], Some(passes[0].nanoseconds)));
    }
    let shadows = prepared.shadowed();
    let clusters = prepared.gpu_clustered();
    let names: &[&str] = match (shadows, clusters) {
        (true, true) => &ALL,
        (true, false) => &SHADOW_ONLY,
        (false, true) => &CLUSTER_ONLY,
        (false, false) => &ALL[3..],
    };
    (passes.len() == names.len() + usize::from(presented)
        && passes
            .iter()
            .zip(names)
            .all(|(pass, name)| pass.name.as_ref() == *name)
        && (!presented || passes[names.len()].name.as_ref() == thrum_render::present::PASS_NAME))
        .then(|| {
            let mut scene = [0.0; 6];
            for pass in passes.iter().take(names.len()) {
                let slot = match pass.name.as_ref() {
                    "synthetic Sun shadow near" => 0,
                    "synthetic Sun shadow far" => 1,
                    "synthetic cluster build" => 2,
                    "synthetic forward opaque" => 3,
                    "synthetic transform history" => 4,
                    _ => 5,
                };
                scene[slot] = pass.nanoseconds;
            }
            (scene, presented.then(|| passes[names.len()].nanoseconds))
        })
}

fn duration_ns_u32(value: Duration) -> u32 {
    u32::try_from(value.as_nanos()).unwrap_or(u32::MAX)
}

#[cfg_attr(
    not(target_arch = "wasm32"),
    expect(
        clippy::large_types_passed_by_value,
        reason = "ownership transfers the completed sample into the report queue without a clone"
    )
)]
fn retain_sample(queue: &mut VecDeque<FrameSample>, dropped: &mut u64, sample: FrameSample) {
    if queue.len() == IN_FLIGHT {
        if queue
            .front()
            .is_some_and(|oldest| sample.submission_order <= oldest.submission_order)
        {
            *dropped += 1;
            return;
        }
        queue.pop_front();
        *dropped += 1;
    }
    let insert = queue
        .iter()
        .position(|prior| prior.submission_order > sample.submission_order)
        .unwrap_or(queue.len());
    queue.insert(insert, sample);
}

fn pop_report_before_pending(
    queue: &mut VecDeque<FrameSample>,
    oldest_pending: Option<u64>,
) -> Option<FrameSample> {
    queue
        .front()
        .is_some_and(|sample| {
            oldest_pending.is_none_or(|pending| sample.submission_order < pending)
        })
        .then(|| queue.pop_front())
        .flatten()
}

#[cfg(test)]
mod tests;
