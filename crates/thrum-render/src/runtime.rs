//! Bounded committed-world presentation using the prepared V1 renderer.

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

use crate::contribution::{ContributionPasses, SceneContribution};
use crate::{
    capabilities::DeviceCapabilities,
    extract::{ExtractReport, RenderExtractor, SourceTick},
    present::{PresentationBinding, SurfacePresenter, SurfaceProfile},
    synthetic::{
        CullMode, CullingCounts, FrameLighting, OffscreenTarget, PreparedSynthetic,
        SceneApplyReport, SceneTimingLayout, SyntheticScene, SyntheticView, pass_durations,
    },
    timing::{GpuTimings, TimingError},
};
use thrum_core::sim::{WorldAdvance, WorldPublication};

mod device;
mod preparation;
mod tokens;
#[cfg(not(target_arch = "wasm32"))]
mod visual;
pub use device::{DevicePrepareInputs, PreparedDevice};
pub use tokens::{AdvancedFrame, EncodedFrame};
#[cfg(not(target_arch = "wasm32"))]
pub use visual::{PreparedVisual, VisualPrepareInputs};

const DEFAULT_RENDER_EXTENT: [u32; 2] = [640, 360];
const GPU_WAIT: Duration = Duration::from_secs(10);
const IN_FLIGHT: usize = 2;

/// A submitted capture of one committed endpoint, owned by the caller until
/// [`FrameHost::try_finish_capture`] returns its pixels. It retains its own
/// readback and belongs to the recording host's device generation.
#[must_use = "a pending capture is read with try_finish_capture"]
pub struct PendingCapture {
    pixels: crate::synthetic::PixelReadback,
    submitted: wgpu::SubmissionIndex,
    owner: Arc<()>,
    generation: u64,
    tick: u64,
    extent: [u32; 2],
}

impl PendingCapture {
    /// Device generation that recorded the capture.
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }
    /// Committed world tick the capture shows.
    #[must_use]
    pub fn tick(&self) -> u64 {
        self.tick
    }
    /// Pixel extent of the capture, `[width, height]`.
    #[must_use]
    pub fn extent(&self) -> [u32; 2] {
        self.extent
    }
}

/// A world observer backed by one authoritative clock or its resident lane.
pub trait FrameWorld {
    /// Immutable committed page lease.
    type Page;
    /// Latest committed authority identity, possibly newer than its publication.
    fn source(&self) -> SourceTick;
    /// Whether the current committed endpoint has an available publication.
    fn has_current_page(&self) -> bool;
    /// Whether authoritative work runs independently of the render caller.
    fn is_threaded(&self) -> bool;
    /// Advance inline authority or observe resident authority at monotonic time.
    /// # Errors
    /// Returns an authoritative clock, game or lane failure.
    fn advance(&mut self, now: Duration) -> Result<WorldAdvance, String>;
    /// Lease a page with its actual committed identity.
    /// # Errors
    /// Returns a publication failure without changing authority.
    fn publication(&mut self) -> Result<WorldPublication<Self::Page>, String>;
    /// Sample a display target without advancing authority.
    fn presentation_alpha_at(&self, lead: Duration) -> f32;
}

/// Game-owned mapping from committed schema to the prepared renderer.
pub trait SceneSource<P> {
    /// Extract immutable game values into retained renderer columns.
    fn extract(&self, into: &mut RenderExtractor, page: &P) -> ExtractReport;
    /// Sample game camera and lighting policy for a presentation endpoint.
    /// # Errors
    /// Refuses invalid presentation values without changing world authority.
    fn frame(
        &self,
        source: SourceTick,
        alpha: f32,
        extent: [u32; 2],
        lighting: Option<FrameLighting>,
    ) -> Result<SceneFrame, String>;
}

/// One sampled camera/lighting input; resource updates belong to the executor.
pub struct SceneFrame {
    /// Checked camera and display exposure selected by the game.
    pub view: SyntheticView,
    /// Checked renderer lighting handoff.
    pub lighting: FrameLighting,
}

/// A frame operation failed; authoritative ticks already committed remain valid.
#[derive(Debug)]
pub enum FrameError {
    /// Clock, publication or resident world failure.
    World(String),
    /// Game presentation mapping failure.
    Scene(String),
    /// GPU preparation, recording, polling or readback failure.
    Gpu(String),
    /// The requested operation cannot use the current prepared state.
    InvalidState(&'static str),
}
impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::World(message) => write!(f, "frame world: {message}"),
            Self::Scene(message) => write!(f, "frame scene: {message}"),
            Self::Gpu(message) => write!(f, "frame GPU: {message}"),
            Self::InvalidState(message) => f.write_str(message),
        }
    }
}
impl std::error::Error for FrameError {}
impl From<String> for FrameError {
    fn from(message: String) -> Self {
        Self::Gpu(message)
    }
}
impl From<&'static str> for FrameError {
    fn from(message: &'static str) -> Self {
        Self::InvalidState(message)
    }
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
    /// Committed ticks newer than this view at submission.
    pub visual_age_ticks: u32,
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
    /// CPU wall attributable to the render owner. Inline hosts exclude their
    /// tick callback; threaded hosts keep the full render callback wall.
    pub render_cpu: Duration,
    /// Actual GPU pass durations: near Sun shadow, far Sun shadow plus the horizon height map and the air shadow heights, local clusters,
    /// forward, history, tone, then atmosphere transmittance, multiple/ground
    /// irradiance preparation, sky view/specular preparation, diffuse irradiance and aerial perspective. The sky draw and an attached
    /// scene contribution's compute pass are included in forward. Disabled or cached compute passes report zero.
    pub gpu_pass_ns: Option<[f64; 11]>,
    /// GPU time for the optional surface blit, when it was submitted and timed.
    pub gpu_present_ns: Option<f64>,
    /// GPU time for a scene contribution's compute pass alone (already in forward), when one is
    /// attached and timed.
    pub gpu_contribution_ns: Option<f64>,
    /// This frame also wrote an acquired platform surface.
    pub surface_presented: bool,
    /// The GPU frame completed but its optional timing batch was unusable.
    pub timing_fault: bool,
    /// The camera culling mode the prepared renderer ran this frame with.
    pub culling_mode: CullMode,
    /// Instances camera culling drew, read back on completion without
    /// timestamps. `None` for Direct, which lists nothing, and when the
    /// readback failed.
    pub camera_culling: Option<CullingCounts>,
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
    /// First refused tick count for the current stopped overload episode.
    /// Repeats until successful resume; this is state, not an increment.
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
    timing_layout: SceneTimingLayout,
    render_done: Arc<AtomicBool>,
    state: SlotState,
}

#[derive(Clone, Copy)]
struct SubmitFrame {
    extracted: ExtractReport,
    reused_source: bool,
    extract_cpu: Duration,
    tick_cpu: Duration,
    completed_ticks: u32,
    queued_ticks: u32,
    source: SourceTick,
    presentation_alpha: f32,
}

struct PreparedSample {
    sample: FrameSample,
    source: SourceTick,
    view_history_epoch: u64,
}

struct RecordedScene {
    slot_index: usize,
    encoder: wgpu::CommandEncoder,
    prepared: PreparedSample,
    encode_cpu: Duration,
}

/// One fixed-clock simulation and prepared GPU scene. The device and queue
/// belong to the caller's executor; this host creates no independent GPU owner.
/// The prepared target ring bounds in-flight work and defers GPU timing readback.
/// A platform adapter may also supply an acquired surface view for the
/// prepared presentation pass; the host never owns the surface lifecycle.
pub struct FrameHost<W: FrameWorld, V: SceneSource<W::Page>> {
    device: wgpu::Device,
    queue: wgpu::Queue,
    /// Stable across visual changes. Backend device IDs can repeat across instances.
    capture_owner: Arc<()>,
    presentation_lead: Duration,
    generation: u64,
    source: V,
    history_reset_pending: bool,
    submitted_view_history_epoch: u64,
    upload_reset_pending: bool,
    world: W,
    extractor: RenderExtractor,
    last_view_source: Option<SourceTick>,
    scene: SyntheticScene,
    last_extracted: ExtractReport,
    last_scene_apply: SceneApplyReport,
    render_extent: [u32; 2],
    changed_slots: Vec<u32>,
    prepared: PreparedSynthetic,
    presenter: Option<SurfacePresenter>,
    slots: Vec<FrameSlot>,
    completed: VecDeque<FrameSample>,
    next_submission_order: u64,
    dropped_views: u64,
    dropped_stale_views: u64,
    dropped_reports: u64,
    dropped_capture_rows: u64,
    dropped_surface: u64,
    visual_lighting: Option<FrameLighting>,
    visual_revision: Option<[u8; 32]>,
    visual_owner: Arc<()>,
    contribution: Option<Box<dyn SceneContribution>>,
    contribution_error: Option<String>,
}

impl<W: FrameWorld, V: SceneSource<W::Page>> FrameHost<W, V> {
    /// Attach an engine module that draws into the scene's forward pass with the scene's
    /// lighting. It is rebuilt on every visual or device republication.
    ///
    /// # Errors
    /// Refuses an in-flight scene, a contribution that cannot draw, or a graph preparation
    /// fault. The current attachment is retained.
    pub fn set_scene_contribution(
        &mut self,
        mut contribution: Box<dyn SceneContribution>,
    ) -> Result<(), String> {
        if self
            .slots
            .iter()
            .any(|slot| !matches!(slot.state, SlotState::Free))
        {
            return Err("attach a scene contribution with no frames in flight".to_owned());
        }
        contribution.rebuild(&self.prepared.scene_surface(&self.queue))?;
        let passes = contribution.passes();
        self.declare_contribution_passes(passes)
            .map_err(|error| format!("contribution passes: {error}"))?;
        self.contribution = Some(contribution);
        self.contribution_error = None;
        Ok(())
    }

    /// Why the last rebuild detached the scene contribution, if it did.
    pub fn contribution_error(&self) -> Option<&str> {
        self.contribution_error.as_deref()
    }

    /// Optional content can refuse a new generation. Keep the already prepared
    /// graph and targets: contribution passes record no work when detached.
    fn rebuild_contribution(&mut self) {
        if let Some(contribution) = self.contribution.as_mut()
            && let Err(error) = contribution.rebuild(&self.prepared.scene_surface(&self.queue))
        {
            self.contribution = None;
            self.contribution_error = Some(error);
        }
    }

    /// Prepare a complete graph/target set before changing the active slots.
    fn declare_contribution_passes(
        &mut self,
        passes: ContributionPasses,
    ) -> Result<(), FrameError> {
        let previous = self.prepared.contribution_passes();
        if previous == passes {
            return Ok(());
        }
        self.prepared.set_contribution_passes(passes);
        let slots = preparation::prepare_slots(
            &self.prepared,
            &self.queue,
            self.render_extent,
            self.presenter.is_some(),
        )
        .and_then(|mut slots| {
            preparation::bind_presenter(&self.prepared, &mut slots, self.presenter.as_ref())?;
            Ok(slots)
        });
        match slots {
            Ok(slots) => {
                self.slots = slots;
                // Detached candidates carry the previous contribution pass set.
                self.visual_owner = Arc::new(());
                self.reset_replacement_history();
                Ok(())
            }
            Err(error) => {
                self.prepared.set_contribution_passes(previous);
                Err(error)
            }
        }
    }
    /// Maximum submitted scene frames awaiting GPU completion.
    pub const MAX_IN_FLIGHT: usize = IN_FLIGHT;

    /// Inspect the world adapter between frame operations.
    pub fn world(&self) -> &W {
        &self.world
    }
    /// Exclusively inspect or control inline game state between frame operations.
    pub fn world_mut(&mut self) -> &mut W {
        &mut self.world
    }
    /// Inspect the game-owned presentation policy.
    pub fn source(&self) -> &V {
        &self.source
    }
    /// Change presentation policy between frames. Invalidate retained extraction
    /// and GPU correspondence because the mapping itself may have changed.
    pub fn source_mut(&mut self) -> &mut V {
        self.extractor.invalidate_history();
        self.last_view_source = None;
        self.history_reset_pending = true;
        &mut self.source
    }
    /// Reset correspondence on the next successful scene submission.
    pub fn invalidate_history(&mut self) {
        self.history_reset_pending = true;
    }
    fn visual_lighting(&self) -> Option<FrameLighting> {
        self.visual_lighting
    }

    /// Set the lead to the platform's next display target. Simulation still
    /// advances only to the callback's observed monotonic time.
    pub fn set_presentation_lead(&mut self, lead: Duration) {
        self.presentation_lead = lead;
    }

    fn presentation_alpha(&self) -> f32 {
        self.world.presentation_alpha_at(self.presentation_lead)
    }

    /// Prepare a bounded renderer using the caller's selected device generation.
    /// `queue` must be the queue returned with `capabilities`' device; the
    /// platform owns this pairing and the renderer does not privately probe it.
    /// # Errors
    /// Returns invalid extent, initial publication or GPU preparation failure.
    pub async fn new(
        capabilities: &DeviceCapabilities,
        queue: &wgpu::Queue,
        config: FrameConfig,
        mut world: W,
        source: V,
        mut scene: SyntheticScene,
        exposure: f32,
    ) -> Result<Self, FrameError> {
        let device = capabilities.device();
        let [width, height] = config.render_extent;
        if width == 0 || height == 0 {
            return Err(FrameError::InvalidState("render extent must be nonzero"));
        }
        let capacity = scene.instances.len();
        let mut extractor = RenderExtractor::new(capacity, capacity);
        let initial = world.publication().map_err(FrameError::World)?;
        let initial_source = publication_source(&initial);
        if initial_source != world.source() {
            return Err(FrameError::InvalidState(
                "initial publication is not current",
            ));
        }
        let initial_extracted = source.extract(&mut extractor, &initial.page);
        drop(initial);
        if extractor.frame().source() != Some(initial_source) {
            return Err(FrameError::InvalidState(
                "initial extraction source does not match its publication",
            ));
        }
        let mut changed_slots = Vec::with_capacity(capacity);
        let initial_scene_apply = scene.apply_render_columns(extractor.frame(), &mut changed_slots);
        let initial_frame = source
            .frame(world.source(), 0.0, config.render_extent, None)
            .map_err(FrameError::Scene)?;
        scene.view = initial_frame.view;
        let mut gpu = preparation::prepare_scene(
            capabilities,
            queue,
            &scene,
            initial_frame.lighting,
            exposure,
            config.render_extent,
            config.surface.is_some(),
            ContributionPasses::default(),
        )
        .await?;
        let presenter = match config.surface {
            Some(profile) => Some(
                SurfacePresenter::prepare(capabilities, config.render_extent, profile)
                    .await
                    .map_err(|error| format!("surface prepare: {error:?}"))?,
            ),
            None => None,
        };
        gpu.bind_presenter(presenter.as_ref())?;
        Ok(Self {
            device: device.clone(),
            queue: queue.clone(),
            capture_owner: Arc::new(()),
            presentation_lead: Duration::ZERO,
            generation: capabilities.generation(),
            source,
            history_reset_pending: false,
            submitted_view_history_epoch: 0,
            upload_reset_pending: false,
            world,
            extractor,
            last_view_source: Some(initial_source),
            scene,
            last_extracted: initial_extracted,
            last_scene_apply: initial_scene_apply,
            render_extent: config.render_extent,
            changed_slots,
            prepared: gpu.prepared,
            presenter,
            slots: gpu.slots,
            completed: VecDeque::with_capacity(IN_FLIGHT),
            next_submission_order: 0,
            dropped_views: 0,
            dropped_stale_views: 0,
            dropped_reports: 0,
            dropped_capture_rows: 0,
            dropped_surface: 0,
            visual_lighting: None,
            visual_revision: None,
            visual_owner: Arc::new(()),
            contribution: None,
            contribution_error: None,
        })
    }

    /// Copy presentation inputs once for an asynchronous visual preparation.
    /// This copies the finite scene only on accepted edits, never on ticks.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn visual_prepare_inputs(&self) -> VisualPrepareInputs {
        VisualPrepareInputs {
            queue: self.queue.clone(),
            capabilities: self.prepared.capabilities().clone(),
            scene: self.scene.clone(),
            surface: self.presenter.is_some(),
            contribution_passes: self.prepared.contribution_passes(),
            render_extent: self.render_extent,
            owner: Arc::clone(&self.visual_owner),
        }
    }

    /// The native render lane extracts the current publication itself, without
    /// waiting for a world update.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn has_serial_view(&self) -> bool {
        self.presenter.is_some()
            && self.world.has_current_page()
            && self
                .slots
                .iter()
                .any(|slot| matches!(slot.state, SlotState::Free))
    }

    /// A retained tick pair can be sampled again at a new display target
    /// without waiting for another simulation update or extraction.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn has_current_scene_for_display(&self) -> bool {
        self.last_view_source == Some(self.world.source())
            && self
                .slots
                .iter()
                .any(|slot| matches!(slot.state, SlotState::Free))
    }

    /// Select a fully prepared cooked view at a GPU-safe extraction barrier.
    /// `false` retains the candidate for retry while older packets are live.
    ///
    /// # Errors
    ///
    /// Returns a binding or full-table upload fault without replacing the active view.
    ///
    #[cfg(not(target_arch = "wasm32"))]
    pub fn publish_visual(&mut self, candidate: &mut PreparedVisual) -> Result<bool, FrameError> {
        if !Arc::ptr_eq(&candidate.owner, &self.visual_owner) {
            return Err(FrameError::InvalidState(
                "stale or foreign visual candidate",
            ));
        }
        self.poll_gpu()?;
        if self.pending_frames() != 0 {
            return Ok(false);
        }
        let scene = candidate.scene.as_mut().ok_or(FrameError::InvalidState(
            "visual candidate already published",
        ))?;
        scene.bind_presenter(self.presenter.as_ref())?;
        let queue = self.queue.clone();
        let view = self.refresh_replacement(scene, &queue, Some(candidate.lighting))?;
        let scene = candidate.scene.take().ok_or(FrameError::InvalidState(
            "visual candidate already published",
        ))?;
        self.prepared = scene.prepared;
        self.slots = scene.slots;
        self.scene.view = view;
        self.visual_lighting = Some(candidate.lighting);
        self.visual_revision = candidate.revision;
        self.reset_replacement_history();
        self.visual_owner = Arc::new(());
        self.rebuild_contribution();
        Ok(true)
    }

    fn refresh_replacement(
        &mut self,
        replacement: &mut preparation::PreparedScene,
        queue: &wgpu::Queue,
        lighting: Option<FrameLighting>,
    ) -> Result<SyntheticView, FrameError> {
        let authority = self.world.source();
        let alpha = self.presentation_alpha();
        let frame = self
            .source
            .frame(authority, alpha, self.render_extent, lighting)
            .map_err(FrameError::Scene)?;
        let view = frame.view.with_history_reset(true);
        let previous = std::mem::replace(&mut self.scene.view, view);
        let refreshed: Result<(), FrameError> = (|| {
            replacement
                .prepared
                .synchronize_history(queue, authority.epoch, authority.timeline);
            // Fit current view/cascades before instance packing uses caster bounds.
            replacement
                .prepared
                .update_frame(
                    queue,
                    &mut replacement.slots[0].target,
                    &self.scene,
                    frame.lighting,
                    alpha,
                )
                .map_err(|error| FrameError::Gpu(format!("replacement frame: {error:?}")))?;
            self.changed_slots.clear();
            let count = u32::try_from(self.scene.instances.len())
                .map_err(|_| FrameError::InvalidState("scene instance slots exceed u32"))?;
            self.changed_slots.extend(0..count);
            replacement
                .prepared
                .upload_instances(queue, &self.scene.instances, &self.changed_slots)
                .map_err(|error| FrameError::Gpu(format!("replacement full upload: {error:?}")))?;
            Ok(())
        })();
        self.scene.view = previous;
        refreshed?;
        Ok(view)
    }

    fn reset_replacement_history(&mut self) {
        self.history_reset_pending = true;
        self.upload_reset_pending = true;
        self.last_view_source = None;
        self.extractor.invalidate_history();
    }

    /// The camera culling mode the prepared renderer runs; it survives
    /// surface resize and device replacement.
    pub fn camera_culling(&self) -> CullMode {
        self.prepared.camera_culling()
    }

    /// Number of optional views skipped because all GPU packet slots were held.
    pub fn dropped_views(&self) -> u64 {
        self.dropped_views
    }

    /// Completed native views discarded after their source or visual generation expired.
    pub fn dropped_stale_views(&self) -> u64 {
        self.dropped_stale_views
    }

    /// Profiler reports discarded by the bounded queue or device replacement.
    /// Replacement includes abandoned pending reports, without claiming GPU
    /// completion. This never changes authoritative ticks.
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
    pub fn resize_surface(&mut self, extent: [u32; 2]) -> Result<(), FrameError> {
        self.presenter
            .as_mut()
            .ok_or_else(|| "host has no surface presenter".to_owned())?
            .resize(&self.device, extent)
            .map_err(|error| FrameError::Gpu(format!("surface resize: {error:?}")))
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
    pub fn poll_completed(&mut self) -> Result<Option<FrameSample>, FrameError> {
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

    /// Admit monotonic host time through the shared fixed-step clock. Every due
    /// update commits once before a visual page can be published. A full
    /// presentation ring skips an optional view with a counter, never the world
    /// update. A display callback may resample a complete publication with a
    /// new interpolation fraction. Overload is explicit.
    /// # Errors
    /// Returns a clock, simulation or GPU fault; never silently skips an update.
    pub fn drive(
        &mut self,
        now: Duration,
        surface: Option<&SurfaceOutput<'_>>,
    ) -> Result<DriveReport, FrameError> {
        if surface.is_some() && self.presenter.is_none() {
            return Err(FrameError::InvalidState("host has no surface presenter"));
        }
        self.advance(now)?.submit(surface)
    }

    /// Admit authoritative time before deciding whether to acquire a drawable.
    /// The returned frame owns the exclusive host borrow until it is submitted
    /// or dropped; a dropped frame skips only optional presentation.
    /// # Errors
    /// Returns a clock or simulation fault without an incomplete world update.
    pub fn advance(&mut self, now: Duration) -> Result<AdvancedFrame<'_, W, V>, FrameError> {
        let wall_start = Instant::now();
        self.poll_gpu()?;
        let (report, advanced) = self.admit_world(now)?;
        Ok(AdvancedFrame {
            host: Some(self),
            report: Some(report),
            tick_cpu: advanced.tick_cpu,
            wall_start,
            recovered_publication: advanced.recovered_publication,
        })
    }

    /// Advance or observe authority while explicitly omitting optional rendering.
    /// This never polls the device, collects GPU completions, samples scene
    /// policy, extracts rows, uploads, encodes or submits. Pending GPU packets
    /// and submitted history remain unchanged, including after device loss.
    /// Skip counters follow dropped admission-token semantics: count one omitted
    /// view for completed updates or recovered publication with an available
    /// page, or for completed updates alongside overload. This does not pause
    /// the world.
    /// # Errors
    /// Returns the world's clock, game or publication failure. Already committed
    /// ticks retain the world's existing failure/retry semantics.
    pub fn advance_without_rendering(&mut self, now: Duration) -> Result<DriveReport, FrameError> {
        let wall_start = Instant::now();
        let (mut report, advanced) = self.admit_world(now)?;
        self.skip_advance(&mut report, advanced.recovered_publication);
        report.wall = wall_start.elapsed();
        Ok(report)
    }

    fn admit_world(&mut self, now: Duration) -> Result<(DriveReport, WorldAdvance), FrameError> {
        let advanced = self.world.advance(now).map_err(FrameError::World)?;
        let report = DriveReport {
            tick: advanced.tick,
            overload: advanced.overload,
            completed_ticks: advanced.completed_ticks,
            queued_ticks: advanced.queued_ticks,
            projected_changes: advanced.projected_changes,
            projection_lost_ticks: advanced.projection_lost_ticks,
            submitted: false,
            surface_presented: false,
            dropped_view: advanced.skipped_publications != 0,
            wall: Duration::ZERO,
        };
        self.dropped_views = self
            .dropped_views
            .saturating_add(advanced.skipped_publications);
        Ok((report, advanced))
    }

    fn skip_advance(&mut self, report: &mut DriveReport, recovered_publication: bool) {
        if report.overload.is_some() {
            if report.completed_ticks > 0 {
                self.dropped_views = self.dropped_views.saturating_add(1);
                report.dropped_view = true;
            }
        } else if self.world.has_current_page()
            && (report.completed_ticks > 0 || recovered_publication)
        {
            self.dropped_views = self.dropped_views.saturating_add(1);
            if self.presenter.is_some() {
                self.dropped_surface = self.dropped_surface.saturating_add(1);
            }
            report.dropped_view = true;
        }
    }

    fn finish_advance(
        &mut self,
        mut report: DriveReport,
        tick_cpu: Duration,
        wall_start: Instant,
        recovered_publication: bool,
        surface: Option<&SurfaceOutput<'_>>,
    ) -> Result<DriveReport, FrameError> {
        if surface.is_some() && self.presenter.is_none() {
            return Err(FrameError::InvalidState("host has no surface presenter"));
        }
        if report.overload.is_some() {
            self.skip_advance(&mut report, recovered_publication);
            report.wall = wall_start.elapsed();
            return Ok(report);
        }
        if !self.world.has_current_page() {
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
    ) -> Result<SubmitFrame, FrameError> {
        let extract_start = Instant::now();
        let snapshot = self.world.publication().map_err(FrameError::World)?;
        let source = publication_source(&snapshot);
        let extractor = &mut self.extractor;
        let reused_source =
            extractor.frame().source() == Some(source) && self.last_view_source == Some(source);
        let extracted = if reused_source {
            self.last_extracted
        } else {
            self.source.extract(extractor, &snapshot.page)
        };
        drop(snapshot);
        Ok(SubmitFrame {
            extracted,
            reused_source,
            extract_cpu: extract_start.elapsed(),
            tick_cpu,
            completed_ticks,
            queued_ticks,
            source,
            presentation_alpha: self.presentation_alpha(),
        })
    }

    fn submit_extracted(
        &mut self,
        slot_index: usize,
        report: &mut DriveReport,
        surface: Option<&SurfaceOutput<'_>>,
        frame: SubmitFrame,
        wall_start: Instant,
    ) -> Result<(), FrameError> {
        let Some(prepared) = self.prepare_extracted(slot_index, report, frame)? else {
            return Ok(());
        };
        let recorded = self.encode_scene_only(slot_index, prepared)?;
        self.finish_encoded(recorded, report, surface, wall_start)
    }

    fn prepare_extracted(
        &mut self,
        slot_index: usize,
        report: &mut DriveReport,
        frame: SubmitFrame,
    ) -> Result<Option<PreparedSample>, FrameError> {
        let authority = self.world.source();
        let frame_source = self.extractor.frame().source();
        let Some(frame_source) = frame_source else {
            self.dropped_stale_views = self.dropped_stale_views.saturating_add(1);
            report.dropped_view = true;
            return Ok(None);
        };
        if frame_source != frame.source || frame.source != authority {
            self.dropped_stale_views = self.dropped_stale_views.saturating_add(1);
            report.dropped_view = true;
            return Ok(None);
        }
        // Policy refusal must leave the CPU/GPU instance delta pending. In
        // particular, applying columns first could make a same-source retry
        // mistake the new CPU scene for already uploaded data.
        let upload_start = Instant::now();
        let frame_scene = self
            .source
            .frame(
                frame.source,
                frame.presentation_alpha,
                self.render_extent,
                self.visual_lighting(),
            )
            .map_err(FrameError::Scene)?;
        let SceneApplyReport {
            moving,
            dropped_invalid,
        } = if frame.reused_source {
            self.changed_slots.clear();
            self.last_scene_apply
        } else {
            let applied = self
                .scene
                .apply_render_columns(self.extractor.frame(), &mut self.changed_slots);
            self.last_scene_apply = applied;
            applied
        };
        self.restore_upload_slots()?;
        self.last_extracted = frame.extracted;
        let lighting = frame_scene.lighting;
        self.scene.view = self.view_with_history_reset(frame_scene.view);
        self.prepared
            .synchronize_history(&self.queue, authority.epoch, authority.timeline);
        let view_update_start = Instant::now();
        if let Err(error) = self.prepared.update_frame(
            &self.queue,
            &mut self.slots[slot_index].target,
            &self.scene,
            lighting,
            frame.presentation_alpha,
        ) {
            self.retry_upload();
            return Err(FrameError::Gpu(format!("live view update: {error:?}")));
        }
        let view_update_cpu = view_update_start.elapsed();
        let uploaded = match self.prepared.upload_instances(
            &self.queue,
            &self.scene.instances,
            &self.changed_slots,
        ) {
            Ok(uploaded) => uploaded,
            Err(error) => {
                self.retry_upload();
                return Err(FrameError::Gpu(format!("instance update: {error:?}")));
            }
        };
        self.upload_reset_pending = false;
        let upload_cpu = upload_start.elapsed();
        let visual_revision = self.visual_revision;
        Ok(Some(PreparedSample {
            sample: FrameSample {
                tick: frame.source.tick,
                presentation_alpha: frame.presentation_alpha,
                submission_order: 0,
                completed_ticks: frame.completed_ticks,
                queued_ticks: frame.queued_ticks,
                tick_cpu: frame.tick_cpu,
                extract_cpu: frame.extract_cpu,
                visual_age_ticks: u32::try_from(authority.tick.saturating_sub(frame.source.tick))
                    .unwrap_or(u32::MAX),
                upload_cpu,
                view_update_ns: duration_ns_u32(view_update_cpu),
                encode_cpu: Duration::ZERO,
                wall: Duration::ZERO,
                render_cpu: Duration::ZERO,
                gpu_pass_ns: None,
                gpu_present_ns: None,
                gpu_contribution_ns: None,
                surface_presented: false,
                timing_fault: false,
                culling_mode: self.prepared.camera_culling(),
                camera_culling: None,
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
            source: frame_source,
            view_history_epoch: self.scene.view.history_epoch(),
        }))
    }

    fn view_with_history_reset(&self, view: SyntheticView) -> SyntheticView {
        view.with_history_reset(
            view.history_reset()
                || self.history_reset_pending
                || view.history_epoch() != self.submitted_view_history_epoch,
        )
    }

    fn restore_upload_slots(&mut self) -> Result<(), FrameError> {
        if self.upload_reset_pending {
            self.changed_slots.clear();
            let count = u32::try_from(self.scene.instances.len())
                .map_err(|_| FrameError::InvalidState("scene instance slots exceed u32"))?;
            self.changed_slots.extend(0..count);
        }
        Ok(())
    }

    fn retry_upload(&mut self) {
        self.upload_reset_pending = true;
        self.history_reset_pending = true;
        self.last_view_source = None;
    }

    fn encode_scene_only(
        &mut self,
        slot_index: usize,
        prepared: PreparedSample,
    ) -> Result<RecordedScene, FrameError> {
        let start = Instant::now();
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let slot = &mut self.slots[slot_index];
        slot.target.arm_culling_stats();
        if let Err(error) = self.prepared.encode_contributed(
            &mut encoder,
            self.generation,
            &slot.target,
            Some(&mut slot.timings),
            None,
            self.contribution.as_deref_mut(),
        ) {
            drop(encoder);
            let discarded = slot.timings.discard_unsubmitted(self.generation);
            self.queue.submit([]);
            discarded.map_err(|timing| format!("render timing discard: {timing:?}"))?;
            return Err(FrameError::Gpu(format!("render encode: {error:?}")));
        }
        Ok(RecordedScene {
            slot_index,
            encoder,
            prepared,
            encode_cpu: start.elapsed(),
        })
    }

    fn finish_encoded(
        &mut self,
        recorded: RecordedScene,
        report: &mut DriveReport,
        surface: Option<&SurfaceOutput<'_>>,
        wall_start: Instant,
    ) -> Result<(), FrameError> {
        let RecordedScene {
            slot_index,
            mut encoder,
            prepared,
            encode_cpu,
        } = recorded;
        let PreparedSample {
            mut sample,
            source,
            view_history_epoch,
        } = prepared;
        let finish_start = Instant::now();
        let slot = &mut self.slots[slot_index];
        let surface_presented = self
            .presenter
            .as_ref()
            .zip(slot.presentation.as_ref())
            .zip(surface)
            .is_some_and(|((presenter, binding), surface)| {
                presenter
                    .encode(
                        &mut encoder,
                        binding,
                        surface.view,
                        surface.extent,
                        Some(&mut slot.timings),
                    )
                    .is_ok()
            });
        if self.presenter.is_some() && !surface_presented {
            self.dropped_surface = self.dropped_surface.saturating_add(1);
            report.dropped_view = true;
        }
        if let Err(error) = slot.timings.finish_render(self.generation, &encoder) {
            drop(encoder);
            let discarded = slot.timings.discard_unsubmitted(self.generation);
            self.queue.submit([]);
            discarded
                .map_err(|timing| format!("failed timing discard after {error:?}: {timing:?}"))?;
            return Err(FrameError::Gpu(format!("finish timestamps: {error:?}")));
        }
        slot.render_done.store(false, Ordering::Release);
        self.queue.submit([encoder.finish()]);
        if let Some(contribution) = self.contribution.as_mut() {
            contribution.submitted(true);
        }
        let done = Arc::clone(&slot.render_done);
        self.queue
            .on_submitted_work_done(move || done.store(true, Ordering::Release));
        slot.target.map_culling_stats(
            self.prepared.camera_culling(),
            self.prepared.active_instance_count(),
        );
        sample.encode_cpu = encode_cpu + finish_start.elapsed();
        sample.wall = wall_start.elapsed();
        sample.render_cpu = if self.world.is_threaded() {
            sample.wall
        } else {
            sample.wall.saturating_sub(sample.tick_cpu)
        };
        sample.surface_presented = surface_presented;
        sample.submission_order = self.next_submission_order;
        self.next_submission_order = self.next_submission_order.wrapping_add(1);
        slot.timing_layout = SceneTimingLayout::from((&self.prepared, &slot.target));
        slot.state = SlotState::Rendering(sample);
        report.submitted = true;
        report.surface_presented = surface_presented;
        report.wall = wall_start.elapsed();
        self.prepared.commit_frame();
        self.history_reset_pending = false;
        self.submitted_view_history_epoch = view_history_epoch;
        self.last_view_source = Some(source);
        Ok(())
    }

    /// Render and read back the current committed scene once for visual evidence.
    /// This capture is outside the timed frame path. Native waits here; a
    /// browser uses [`Self::begin_capture`] and [`Self::try_finish_capture`].
    /// # Errors
    /// Returns a GPU encode, completion or readback error.
    pub fn capture_pixels(&mut self) -> Result<Vec<u8>, FrameError> {
        let mut pending = self.begin_capture()?;
        self.wait(pending.submitted.clone(), "capture")?;
        self.try_finish_capture(&mut pending)?
            .ok_or(FrameError::InvalidState("capture readback incomplete"))
    }

    /// Record and submit a capture of the current committed endpoint without
    /// waiting. The caller owns the returned token and finishes it with
    /// [`Self::try_finish_capture`]. Capture never advances authority or
    /// commits display history.
    /// # Errors
    /// Returns a GPU encode or submission error.
    pub fn begin_capture(&mut self) -> Result<PendingCapture, FrameError> {
        let needs_extract = self.extractor.frame().source().is_none_or(|source| {
            source.epoch != self.world.source().epoch
                || source.timeline != self.world.source().timeline
                || source.tick != self.world.source().tick
        });
        let extracted = if needs_extract {
            let snapshot = self.world.publication().map_err(FrameError::World)?;
            let source = publication_source(&snapshot);
            if source != self.world.source() {
                return Err(FrameError::InvalidState(
                    "capture publication is not current",
                ));
            }
            let extracted = self.source.extract(&mut self.extractor, &snapshot.page);
            drop(snapshot);
            if self.extractor.frame().source() != Some(source) {
                return Err(FrameError::InvalidState(
                    "capture extraction source does not match its publication",
                ));
            }
            extracted
        } else {
            ExtractReport::default()
        };
        // A capture describes the current committed endpoint, regardless of
        // the last display sample.
        let capture_alpha = 1.0;
        let frame_scene = self
            .source
            .frame(
                self.world.source(),
                capture_alpha,
                self.render_extent,
                self.visual_lighting(),
            )
            .map_err(FrameError::Scene)?;
        let lighting = frame_scene.lighting;
        self.scene.view = self.view_with_history_reset(frame_scene.view);
        let applied = self
            .scene
            .apply_render_columns(self.extractor.frame(), &mut self.changed_slots);
        self.restore_upload_slots()?;
        let uploaded = match self.prepared.upload_instances(
            &self.queue,
            &self.scene.instances,
            &self.changed_slots,
        ) {
            Ok(uploaded) => uploaded,
            Err(error) => {
                self.retry_upload();
                return Err(FrameError::Gpu(format!(
                    "capture instance update: {error:?}"
                )));
            }
        };
        self.upload_reset_pending = false;
        let dropped = u64::try_from(
            extracted.dropped_capacity
                + extracted.dropped_duplicate
                + extracted.dropped_invalid
                + applied.dropped_invalid
                + uploaded.dropped_invalid,
        )
        .unwrap_or(u64::MAX);
        self.dropped_capture_rows = self.dropped_capture_rows.saturating_add(dropped);
        let mut target = OffscreenTarget::new(
            &self.device,
            &self.prepared,
            self.render_extent[0],
            self.render_extent[1],
        )
        .map_err(|error| format!("capture target: {error:?}"))?;
        if let Err(error) = self.prepared.update_frame(
            &self.queue,
            &mut target,
            &self.scene,
            lighting,
            capture_alpha,
        ) {
            self.retry_upload();
            return Err(FrameError::Gpu(format!("capture view update: {error:?}")));
        }
        let mut pixels = target.readback(&self.device);
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        self.prepared
            .encode_contributed(
                &mut encoder,
                self.generation,
                &target,
                None,
                Some(&mut pixels),
                self.contribution.as_deref_mut(),
            )
            .map_err(|error| format!("capture render/readback: {error:?}"))?;
        let submitted = self.queue.submit([encoder.finish()]);
        if let Some(contribution) = self.contribution.as_mut() {
            contribution.submitted(false);
        }
        Ok(PendingCapture {
            pixels,
            submitted,
            owner: Arc::clone(&self.capture_owner),
            generation: self.generation,
            tick: self.world.source().tick,
            extent: self.render_extent,
        })
    }

    /// Nonblocking check of a pending capture: `Ok(None)` until its readback
    /// completes. A capture recorded by another host, or by a retired device
    /// generation, fails instead and is never satisfied by a later capture's
    /// completion. A platform with its own device-loss signal checks it before
    /// calling; the browser game does.
    /// # Errors
    /// Returns a foreign host, retired generation, GPU poll or readback failure.
    pub fn try_finish_capture(
        &mut self,
        pending: &mut PendingCapture,
    ) -> Result<Option<Vec<u8>>, FrameError> {
        if !Arc::ptr_eq(&pending.owner, &self.capture_owner) {
            return Err(FrameError::InvalidState(
                "capture was recorded by another frame host",
            ));
        }
        if pending.generation != self.generation {
            return Err(FrameError::InvalidState(
                "capture belongs to a retired device generation",
            ));
        }
        self.device
            .poll(wgpu::PollType::Poll)
            .map_err(|error| format!("capture poll: {error:?}"))?;
        Ok(pending
            .pixels
            .try_collect(self.generation)
            .map_err(|error| format!("capture collect: {error:?}"))?)
    }

    fn wait(&self, submitted: wgpu::SubmissionIndex, operation: &str) -> Result<(), FrameError> {
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submitted),
                timeout: Some(GPU_WAIT),
            })
            .map_err(|error| format!("{operation} poll: {error:?}"))?;
        Ok(())
    }

    fn poll_gpu(&mut self) -> Result<(), FrameError> {
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
                    // Counts never hold the slot: take them if their mapping
                    // has settled, keeping any an earlier pass took.
                    sample.camera_culling = sample
                        .camera_culling
                        .or_else(|| slot.target.take_culling_counts());
                    if !slot.timings.is_enabled() {
                        retain_sample(&mut self.completed, &mut self.dropped_reports, sample);
                        continue;
                    }
                    let mut encoder = self
                        .device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                    match slot.timings.resolve_if_ready(self.generation, &mut encoder) {
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
                SlotState::Resolving(mut sample) => {
                    sample.camera_culling = sample
                        .camera_culling
                        .or_else(|| slot.target.take_culling_counts());
                    match slot.timings.try_collect(self.generation) {
                        Ok(Some(passes)) => {
                            if let Some((scene, present, contribution)) =
                                pass_durations(passes, sample.surface_presented, slot.timing_layout)
                            {
                                sample.gpu_pass_ns = Some(scene);
                                sample.gpu_present_ns = present;
                                sample.gpu_contribution_ns = contribution;
                            } else {
                                sample.timing_fault = true;
                            }
                            retain_sample(&mut self.completed, &mut self.dropped_reports, sample);
                        }
                        Ok(None) => slot.state = SlotState::Resolving(sample),
                        Err(error) => {
                            sample.timing_fault = true;
                            // Collection unmaps and resets this batch even when its counter values
                            // are unusable. Keep the resources for the next submission.
                            if !matches!(error, TimingError::InvalidTimestamp { .. }) {
                                slot.timings = new_timings(
                                    self.prepared.capabilities(),
                                    &self.queue,
                                    max_passes,
                                )?;
                            }
                            retain_sample(&mut self.completed, &mut self.dropped_reports, sample);
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

fn publication_source<P>(publication: &WorldPublication<P>) -> SourceTick {
    SourceTick {
        epoch: publication.id.epoch.0,
        tick: publication.id.tick,
        timeline: publication.timeline,
    }
}

fn new_timings(
    capabilities: &DeviceCapabilities,
    queue: &wgpu::Queue,
    max_passes: u32,
) -> Result<GpuTimings, FrameError> {
    GpuTimings::new(capabilities, queue, max_passes)
        .map_err(|error| FrameError::Gpu(format!("GPU timing recovery: {error:?}")))
}

fn duration_ns_u32(value: Duration) -> u32 {
    u32::try_from(value.as_nanos()).unwrap_or(u32::MAX)
}

#[expect(
    clippy::large_types_passed_by_value,
    reason = "ownership transfers the completed sample into the report queue without a clone"
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
