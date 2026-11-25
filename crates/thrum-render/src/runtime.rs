//! Bounded committed-world presentation using the prepared V1 renderer.

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use std::time::Instant;
use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
#[cfg(target_arch = "wasm32")]
pub(crate) use web_time::Instant;

use crate::contribution::{ContributionPasses, SceneContribution};
use crate::{
    capabilities::DeviceCapabilities,
    error::RenderError,
    extract::{ExtractReport, RenderExtractor, SourceTick},
    present::{PresentError, PresentationBinding, SurfacePresenter, SurfaceProfile},
    synthetic::{
        CullMode, CullingCounts, FrameLighting, OffscreenTarget, PreparedSynthetic,
        SceneApplyReport, SceneTimingLayout, SyntheticScene, SyntheticView,
    },
    timing::{FrameCounters, GpuTimings, TimingError, TimingFrame, TimingRecords, TimingStats},
};
use preparation::{FrameTarget, PreparedRenderer};
use thrum_core::sim::{WorldAdvance, WorldPublication};

#[cfg(feature = "creation-counters")]
mod creation;
#[cfg(not(feature = "creation-counters"))]
mod creation_off;
#[cfg(not(feature = "creation-counters"))]
use creation_off as creation;
mod device;
mod extent;
#[cfg(not(target_arch = "wasm32"))]
mod extent_transition;
mod frame_timing;
mod governor;
pub use governor::GovernorFrame;
mod native_provider;
mod preparation;
mod presentation;
mod sprites;
mod submitted_capture;
mod surface_switch;
mod tick;
mod tokens;
#[cfg(not(target_arch = "wasm32"))]
mod visual;
mod world;
#[cfg(not(feature = "creation-counters"))]
use creation::CreationPhase;
#[cfg(feature = "creation-counters")]
pub use creation::{
    CreationCounts, CreationPhase, CreationReport, CreationRow, CreationViolation, Instrumentation,
    NativeSites, PipelineKind, PipelineRow,
};
pub use creation::{CreationRegion, DeviceSite};
pub use device::{DevicePrepareInputs, PreparedDevice};
pub use extent::FrameExtent;
#[cfg(not(target_arch = "wasm32"))]
pub use extent::{ExtentPrepareInputs, PreparedExtent};
#[cfg(not(target_arch = "wasm32"))]
pub use extent_transition::{ExtentRequest, ExtentTransition};
pub use submitted_capture::{CaptureLimits, SubmittedImage};
pub use surface_switch::{SurfaceSwitch, SurfaceSwitchOutcome, SurfaceSwitchRecord};
pub use tick::{TickSubmissionPermission, TickSubmissionStats};
pub use tokens::{AdvancedFrame, EncodedFrame};
#[cfg(not(target_arch = "wasm32"))]
pub use visual::{PreparedVisual, VisualPrepareInputs};
pub use world::FrameWorld;

pub(crate) const GPU_WAIT: Duration = Duration::from_secs(10);
const IN_FLIGHT: usize = 2;
// A native governor can have one detached timing report per render slot.
const COMPLETED_REPORTS: usize = if cfg!(target_arch = "wasm32") {
    IN_FLIGHT
} else {
    2 * IN_FLIGHT
};

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
    ) -> Result<SceneFrame, FrameError>;
    /// Map the committed publication to world sprite rows.
    fn extract_sprites(&self, _into: &mut crate::sprite::SpriteSink<'_>, _page: &P) {}

    /// Sample the game's unjittered 2D view and frame rows on the physical pixel grid.
    /// # Errors
    /// Refuses a missing or invalid game presentation policy.
    fn sprites(
        &self,
        _source: SourceTick,
        _alpha: f32,
        _extent: [u32; 2],
    ) -> Result<crate::sprite::SpriteFrame<'_>, FrameError> {
        Err(FrameError::Render(RenderError::MissingSpriteSource))
    }
    /// Sample positions/intensities into the prepared local-light slice using
    /// the same committed endpoint and alpha as the view. Keep the prepared
    /// capacity and valid photometric values; the source remains their owner.
    fn sample_local_lights(
        &self,
        _source: SourceTick,
        _alpha: f32,
        _lights: &mut [crate::synthetic::SyntheticPointLight],
    ) {
    }
}

/// Load-time selection of the prepared scene half.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SceneKind {
    /// The default camera, lighting and opaque renderer.
    #[default]
    ThreeD,
    /// Sprites at the output pixel extent, with fixed exposure.
    TwoD,
}

/// Requests shared launch controls may offer when selecting a 2D scene.
#[derive(Clone, Copy, Debug, Default)]
pub struct SpriteSceneRequests {
    /// A request for temporal sampling.
    pub temporal_sampling: bool,
    /// An explicit camera culling selection.
    pub culling: Option<CullMode>,
    /// A requested reconstruction provider or explicitly disabled reconstruction.
    pub reconstruction: crate::synthetic::Reconstruction,
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
    /// Diagnostic from the world port during clock, publication or resident world work.
    World(String),
    /// Diagnostic from a scene or contribution callback.
    Scene(String),
    /// GPU preparation, recording, polling or readback failure.
    Gpu {
        /// Frame operation that failed.
        operation: &'static str,
        /// Underlying failure.
        error: crate::error::FrameGpuError,
    },
    /// Device generation was retired by its owner.
    DeviceLost(String),
    /// An external contribution reported an unavailable device capability.
    ContributionDeviceLimit(String),
    /// The effect graph declaration could not be scheduled.
    EffectGraph(crate::error::EffectGraphError),
    /// A secondary view declaration is invalid.
    SecondaryView(crate::contribution::SecondaryViewError),
    /// 2D scenes do not support 3D camera culling.
    SpriteCulling(CullMode),
    /// 2D scenes do not support a 3D reconstructor.
    SpriteReconstruction(crate::synthetic::Reconstruction),
    /// A scene or sprite input was refused by the renderer.
    Render(RenderError),
    /// Timestamp completion failed, and discarding the failed batch also failed.
    TimingDiscard {
        /// Completion failure.
        finish: TimingError,
        /// Cleanup failure.
        discard: TimingError,
    },
    /// Surface encoding failed permanently for this prepared frame.
    SurfaceEncode {
        /// Drive outcome, including the kind of surface fault.
        report: Box<DriveReport>,
        /// Original failure at the renderer boundary.
        error: RenderError,
    },
    /// A dedicated native command buffer failed in this platform generation.
    NativeCommandBuffer {
        /// Platform generation whose queue must no longer be used.
        generation: u64,
    },
    /// The requested operation cannot use the current prepared state.
    InvalidState(&'static str),
}
impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::World(message) => write!(f, "frame world: {message}"),
            Self::Scene(message) => write!(f, "frame scene: {message}"),
            Self::Gpu { operation, error } => write!(f, "frame GPU: {operation}: {error}"),
            Self::DeviceLost(message) | Self::ContributionDeviceLimit(message) => {
                write!(f, "frame GPU: {message}")
            }
            Self::EffectGraph(error) => write!(f, "frame scene: {error}"),
            Self::SecondaryView(error) => write!(f, "frame scene: {error}"),
            Self::SpriteCulling(mode) => write!(
                f,
                "frame scene: 2D scene refuses camera culling mode {mode:?}"
            ),
            Self::SpriteReconstruction(mode) => {
                write!(f, "frame scene: 2D scene refuses reconstructor {mode:?}")
            }
            Self::Render(error) => write!(f, "frame scene: {error}"),
            Self::TimingDiscard { finish, discard } => {
                write!(
                    f,
                    "frame GPU: could not discard timestamps after {finish}: {discard}"
                )
            }
            Self::SurfaceEncode { error, .. } => write!(f, "frame surface encode: {error}"),
            Self::NativeCommandBuffer { generation } => write!(
                f,
                "native command buffer failed in device generation {generation}"
            ),
            Self::InvalidState(message) => f.write_str(message),
        }
    }
}
impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Gpu { error, .. } => Some(error),
            Self::SurfaceEncode { error, .. } | Self::Render(error) => Some(error),
            Self::EffectGraph(error) => Some(error),
            Self::SecondaryView(error) => Some(error),
            Self::TimingDiscard { finish, .. } => Some(finish),
            _ => None,
        }
    }
}

impl FrameError {
    /// A GPU-facing operation failed with a typed source.
    pub fn gpu(operation: &'static str, error: impl Into<crate::error::FrameGpuError>) -> Self {
        Self::Gpu {
            operation,
            error: error.into(),
        }
    }

    fn contribution_diagnostic(&self) -> String {
        match self {
            Self::Scene(message) | Self::ContributionDeviceLimit(message) => message.clone(),
            Self::Gpu { operation, error } => format!("{operation}: {error}"),
            _ => self.to_string(),
        }
    }
}

impl From<RenderError> for FrameError {
    fn from(error: RenderError) -> Self {
        Self::Render(error)
    }
}

/// For a caller whose errors are still strings, such as a game shell or a capture tool,
/// so it can use `?` on frame host calls.
impl From<FrameError> for String {
    fn from(error: FrameError) -> Self {
        error.to_string()
    }
}

/// Why the surface pass could not be encoded for a frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SurfaceFault {
    /// The slot has no presentation binding for the current presenter.
    MissingBinding,
    /// The bound target belongs to a different preparation.
    StaleBinding,
    /// The acquired surface size differs from the current presenter size.
    ExtentMismatch,
    /// The surface pass could not reserve its timing queries.
    Timing(TimingError),
    /// The prepared graph has no matching surface recorder or attachments.
    Graph,
    /// Another renderer failure prevented the surface pass.
    Render,
}

impl std::fmt::Display for SurfaceFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingBinding => f.write_str("Frame slot has no presentation binding"),
            Self::StaleBinding => f.write_str("Surface binding belongs to another preparation"),
            Self::ExtentMismatch => {
                f.write_str("Acquired surface extent differs from the presenter")
            }
            Self::Timing(error) => write!(f, "Surface pass timing: {error}"),
            Self::Graph => {
                f.write_str("Surface graph has no matching pass recorder or attachments")
            }
            Self::Render => f.write_str("Renderer could not encode the surface pass"),
        }
    }
}

impl std::error::Error for SurfaceFault {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Timing(error) => Some(error),
            _ => None,
        }
    }
}

impl SurfaceFault {
    fn from_encode(error: &RenderError) -> Self {
        match error {
            RenderError::StaleTarget | RenderError::Present(PresentError::StaleTarget) => {
                Self::StaleBinding
            }
            RenderError::Present(PresentError::Extent) => Self::ExtentMismatch,
            RenderError::Timing(error) | RenderError::Present(PresentError::Timing(error)) => {
                Self::Timing(*error)
            }
            RenderError::GraphPassMismatch
            | RenderError::Graph(_)
            | RenderError::Present(PresentError::GraphPassMismatch) => Self::Graph,
            _ => Self::Render,
        }
    }

    fn is_transient(self) -> bool {
        matches!(self, Self::StaleBinding | Self::ExtentMismatch)
    }
}

fn surface_encode_outcome(
    report: &DriveReport,
    error: Option<RenderError>,
) -> Result<(), FrameError> {
    match error {
        Some(error) => Err(FrameError::SurfaceEncode {
            report: Box::new(*report),
            error,
        }),
        None => Ok(()),
    }
}

/// One completed simulation-to-GPU frame. GPU pass values are absent when the
/// adapter lacks timestamp queries or a timing readback fails. Callback time
/// excludes deferred GPU completion and must not be added to GPU pass times.
#[derive(Clone, Debug)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "completed telemetry records independent clock, presentation and timing outcomes rather than a mutable state machine"
)]
pub struct FrameSample {
    /// Last complete world tick used by the draw.
    pub tick: u64,
    /// GPU submission order; independent of the visual source tick.
    pub submission_order: u64,
    /// Candidate and predecessor serials packed from the submitted transform state.
    pub history_identity: [u64; 2],
    /// Halton phase packed for this submission, if the view was jittered.
    pub jitter_phase: Option<u32>,
    /// Pixel extent of the drawn scene subrectangle.
    pub render_extent: [u32; 2],
    /// Dynamic quality committed for this frame, if enabled.
    pub governor: Option<GovernorFrame>,
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
    /// Presentation phase at observation plus lead; may exceed one.
    pub display_phase: f32,
    /// Host-clock timestamp supplied to this drive.
    pub observation: Duration,
    /// Lead consumed by this frame's clock sample.
    pub presentation_lead: Duration,
    /// False for endpoint-only clock states.
    pub clock_interpolating: bool,
    /// Host due time of this frame's tick, present only while interpolating.
    pub tick_due_time: Option<Duration>,
    /// Runner's requested target in the host-clock origin, when supplied.
    pub platform_target: Option<Duration>,
    /// Status measured by the host from the same observation as the lead.
    pub lead_status: Option<crate::frame::TargetLeadStatus>,
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
    /// Actual GPU durations keyed by executed graph pass name. `None` means
    /// unsupported or unusable counters; optional absent passes have no entry.
    pub gpu_pass_ns: Option<crate::timing::SceneGpuTimings>,
    /// Dedicated Metal segment times, separate from wgpu query clock counters.
    pub native_segment: Option<crate::synthetic::NativeSegment>,
    /// GPU time for the optional surface blit, when it was submitted and timed.
    pub gpu_present_ns: Option<f64>,
    /// GPU time for a scene contribution's compute pass alone (already in forward), when one is
    /// attached and timed.
    pub gpu_contribution_ns: Option<f64>,
    /// This frame also wrote an acquired platform surface.
    pub surface_presented: bool,
    /// Surface encode refusal, if the scene completed without presentation.
    pub surface_fault: Option<SurfaceFault>,
    /// The GPU frame completed but its optional timing batch was unusable.
    pub timing_fault: bool,
    /// Whether this frame requested GPU timestamps when it was encoded.
    pub timing_requested: bool,
    /// The camera culling mode the prepared renderer ran this frame with.
    pub culling_mode: CullMode,
    /// Preparation-time request and capability reason for this frame's generation.
    pub culling_selection: crate::synthetic::CullingSelection,
    /// Requested and resolved direct-light shadow methods.
    pub shadow_selection: crate::synthetic::ShadowSelection,
    /// Requested and admitted traced ambient occlusion.
    pub ambient_occlusion: crate::capabilities::AmbientOcclusionSelection,
    /// Instances camera culling drew, read back on completion without
    /// timestamps. `None` for Direct, which lists nothing, and when the
    /// readback failed.
    pub camera_culling: Option<CullingCounts>,
    /// Records whose changed ABI bytes were enqueued for upload.
    pub uploaded_rows: u32,
    /// Sprite pass measurements for a 2D scene.
    pub sprites: Option<crate::sprite::SpriteFrameStats>,
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

impl FrameSample {
    /// Move GPU timing data into storage owned by this consumer.
    ///
    /// Call this before retaining a long history of frame samples. The host
    /// publishes from a finite pool so short-lived samples need no allocation.
    /// Detaching may allocate on the consumer's thread.
    pub fn detach_gpu_timing(&mut self) {
        if let Some(timing) = &mut self.gpu_pass_ns {
            timing.detach();
        }
    }
}

/// Clock of a frame accepted for GPU submission, available before completion.
#[derive(Clone, Copy, Debug)]
pub struct SubmittedFrameClock {
    /// Complete world tick used by the submitted view.
    pub tick: u64,
    /// Fraction between the previous and current committed ticks.
    pub presentation_alpha: f32,
    /// GPU submission order.
    pub submission_order: u64,
    /// Monotonic display time consumed when evaluating the world clock.
    pub display_time: Duration,
    /// Host observation that admitted the committed endpoint.
    pub observation: Duration,
    /// Host due time of the committed endpoint while interpolating.
    pub tick_due_time: Option<Duration>,
    /// Whether the submission wrote to the platform surface.
    pub surface_presented: bool,
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
    /// Surface encode refusal for this drive, if any.
    pub surface_fault: Option<SurfaceFault>,
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

impl FrameConfig {
    /// Render without a surface at the caller's explicit pixel extent.
    pub const fn offscreen(render_extent: [u32; 2]) -> Self {
        Self {
            render_extent,
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
    target: FrameTarget,
    presentation: Option<PresentationBinding>,
    timing: FrameTimingBatch,
    deferred_timing: Option<DeferredFrameTiming>,
    timing_records: TimingRecords,
    render_done: Arc<AtomicBool>,
    state: SlotState,
}

struct FrameTimingBatch {
    queries: GpuTimings,
    layout: Option<SceneTimingLayout>,
    readback: bool,
    counter_floor: u64,
}

struct DeferredFrameTiming {
    batch: FrameTimingBatch,
    sample: Option<FrameSample>,
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
    display_phase: f32,
    clock_interpolating: bool,
    tick_due_time: Option<Duration>,
}

struct PreparedSample {
    sample: FrameSample,
    source: SourceTick,
    view_history_epoch: u64,
}

struct RecordedScene {
    segments: Vec<wgpu::CommandBuffer>,
    native: Option<crate::synthetic::NativeFrame>,
    slot_index: usize,
    capture_slot: Option<usize>,
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
    submitted_capture: Option<submitted_capture::Tap>,
    retired_capture: VecDeque<submitted_capture::Tap>,
    presentation_lead: Duration,
    presentation_lead_limit: Duration,
    observation: Duration,
    platform_target: Option<Duration>,
    lead_status: Option<crate::frame::TargetLeadStatus>,
    presenter_generation: u64,
    display_headroom: f32,
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
    render_scale: crate::synthetic::RenderScale,
    governor: Option<governor::GovernorHost>,
    governor_display_tick: Option<governor::DisplayTick>,
    governor_missed_ticks: u64,
    changed_slots: Vec<u32>,
    prepared: PreparedRenderer,
    presenter: Option<SurfacePresenter>,
    slots: Vec<FrameSlot>,
    retired_slots: Vec<FrameSlot>,
    completed: VecDeque<FrameSample>,
    next_submission_order: u64,
    timing_stats: TimingStats,
    frame_counters: FrameCounters,
    submitted_clock: Option<SubmittedFrameClock>,
    timing_requested: bool,
    ticks: tick::TickStage,
    native_failed_generation: Option<u64>,
    dropped_views: u64,
    dropped_stale_views: u64,
    dropped_reports: u64,
    dropped_capture_rows: u64,
    endpoint_readbacks: u64,
    dropped_surface: u64,
    visual_lighting: Option<FrameLighting>,
    visual_revision: Option<[u8; 32]>,
    visual_owner: Arc<()>,
    contribution: Option<Box<dyn SceneContribution>>,
    contribution_error: Option<String>,
    creations: creation::Ledger,
    #[cfg(feature = "creation-counters")]
    retired_creations: Option<CreationReport>,
}

impl<W: FrameWorld, V: SceneSource<W::Page>> FrameHost<W, V> {
    /// Read the preparation observer or install one for subsequent mesh builds.
    /// Install before attaching a contribution whose builds should be observed.
    #[cfg(all(feature = "creation-counters", target_os = "macos"))]
    #[must_use]
    pub fn observe_ray_mesh_builds(&mut self) -> Option<crate::synthetic::RayBuildProbe> {
        match &mut self.prepared {
            PreparedRenderer::ThreeD(scene) => scene.observe_ray_mesh_builds(),
            PreparedRenderer::TwoD(_) => None,
        }
    }

    /// Inspect the actual scene structure from the latest recording.
    #[cfg(all(feature = "creation-counters", target_os = "macos"))]
    #[must_use]
    pub fn ray_structure_instances(&self) -> Option<Vec<crate::synthetic::RayStructureInstance>> {
        self.prepared.three_d()?.ray_structure_instances()
    }

    /// Bind the latest scene structure for a diagnostic query on the host's queue.
    /// Reacquire after a recording, which can increase the structure's capacity.
    #[cfg(all(feature = "creation-counters", target_os = "macos"))]
    #[must_use]
    pub fn ray_structure_probe_group(
        &self,
        layout: &wgpu::BindGroupLayout,
    ) -> Option<wgpu::BindGroup> {
        self.prepared.three_d()?.ray_structure_probe_group(layout)
    }

    /// Inspect the Sun pass volumes determined by the latest camera and scene bounds.
    #[cfg(all(feature = "creation-counters", target_os = "macos"))]
    #[must_use]
    pub fn ray_inspection_shadow_volumes(
        &self,
    ) -> Vec<crate::contribution::SecondaryShadowCascade> {
        let Some(prepared) = self.prepared.three_d() else {
            return Vec::new();
        };
        prepared.ray_inspection_shadow_volumes(
            self.contribution
                .as_ref()
                .and_then(|entry| entry.shadow_bounds()),
        )
    }

    /// Optional entity-row reader retained with the scene through reconstruction.
    pub fn mesh_rows(&self) -> Option<crate::synthetic::mesh_rows::MeshRows> {
        self.scene
            .meshes
            .as_ref()
            .and_then(|selection| selection.updates.clone())
    }
    /// Maximum admitted byte size of one entity arena buffer on this device.
    pub fn mesh_row_binding_limit(&self) -> u64 {
        self.device.limits().max_storage_buffer_binding_size
    }
    /// Attach an engine module that draws into the scene's forward pass with the scene's
    /// lighting. It is rebuilt on every visual or device republication.
    ///
    /// # Errors
    /// Refuses an in-flight scene, a contribution that cannot draw, or a graph preparation
    /// fault. The current attachment is retained.
    pub fn set_scene_contribution(
        &mut self,
        contribution: Box<dyn SceneContribution>,
    ) -> Result<(), FrameError> {
        self.attach_scene_contribution(contribution)
    }

    /// Attach a contribution with classified preparation failures.
    /// # Errors
    /// Refuses an in-flight scene, a contribution that cannot draw, or a graph preparation
    /// fault. The current attachment is retained.
    pub fn attach_scene_contribution(
        &mut self,
        mut contribution: Box<dyn SceneContribution>,
    ) -> Result<(), FrameError> {
        if self.slots.iter().any(FrameSlot::is_pending) {
            return Err(FrameError::InvalidState(
                "attach a scene contribution with no frames in flight",
            ));
        }
        let _region = self
            .creations
            .region(CreationPhase::Transition, "contribution");
        if self.scene_kind() == SceneKind::TwoD {
            self.attach_sprite_contribution(contribution.as_mut())?;
            self.contribution = Some(contribution);
            if let Some(governor) = &mut self.governor {
                governor.refresh();
            }
            self.contribution_error = None;
            return Ok(());
        }
        #[cfg(not(target_arch = "wasm32"))]
        if self.scene.ray_admission(self.prepared.capabilities())
            == crate::capabilities::RayStructureSelection::Enabled
        {
            let capabilities = self.prepared.capabilities().clone();
            let selection = preparation::contribution_admission(
                self.scene.ray_admission(&capabilities),
                contribution.supports_candidate_preparation(),
            );
            let extent = self.render_extent;
            let frame = self.replacement_frame(extent, self.visual_lighting)?;
            let queue = self.queue.clone();
            let presenter = self.presenter.take();
            let staged = self.stage_3d_transition(
                selection,
                None,
                preparation::TransitionInputs {
                    capabilities: &capabilities,
                    queue: &queue,
                    extent,
                    presenter: presenter.as_ref(),
                    frame: &frame,
                    exposure: self.prepared.fallback_exposure(),
                },
                Some(contribution.as_ref()),
            );
            self.presenter = presenter;
            let mut staged = staged?;
            if staged.contribution.is_none() {
                // Direct Maps admission keeps the original attachment hook and error policy.
                if let Some(slots) = preparation::rebuild_3d_contribution(
                    &mut staged.scene.prepared,
                    contribution.as_mut(),
                    &queue,
                    &self.scene,
                    extent,
                    self.presenter.as_ref(),
                    true,
                )? {
                    staged.scene.slots = slots;
                }
            }
            self.contribution = Some(contribution);
            self.commit_3d_transition(staged, || true);
            self.visual_owner = Arc::new(());
            self.reset_replacement_history();
            if let Some(governor) = &mut self.governor {
                governor.refresh();
            }
            self.contribution_error = None;
            return Ok(());
        }
        let slots = preparation::rebuild_3d_contribution(
            &mut self.prepared,
            contribution.as_mut(),
            &self.queue,
            &self.scene,
            self.render_extent,
            self.presenter.as_ref(),
            true,
        );
        if let Some(slots) = slots? {
            self.slots = slots;
            self.visual_owner = Arc::new(());
            self.reset_replacement_history();
        }
        self.contribution = Some(contribution);
        if let Some(governor) = &mut self.governor {
            governor.refresh();
        }
        self.contribution_error = None;
        Ok(())
    }

    /// Why the last rebuild detached the scene contribution, if it did.
    pub fn contribution_error(&self) -> Option<&str> {
        self.contribution_error.as_deref()
    }

    /// Rebuild members before accepting their new generation's declarations. Optional
    /// content can refuse reconstruction; the base scene remains usable when detached.
    fn rebuild_contribution(&mut self) {
        if self.scene_kind() == SceneKind::TwoD {
            if let Some(mut contribution) = self.contribution.take() {
                match self.attach_sprite_contribution(contribution.as_mut()) {
                    Ok(()) => self.contribution = Some(contribution),
                    Err(error) => {
                        self.record_contribution_error(&error);
                    }
                }
            }
            return;
        }
        let Some(mut contribution) = self.contribution.take() else {
            return;
        };
        let result = preparation::rebuild_3d_contribution(
            &mut self.prepared,
            contribution.as_mut(),
            &self.queue,
            &self.scene,
            self.render_extent,
            self.presenter.as_ref(),
            false,
        );
        match result {
            Ok(Some(slots)) => {
                self.slots = slots;
                self.visual_owner = Arc::new(());
                self.reset_replacement_history();
                self.contribution = Some(contribution);
                if let Some(governor) = &mut self.governor {
                    governor.refresh();
                }
            }
            Ok(None) => self.contribution = Some(contribution),
            Err(error) => {
                self.record_contribution_error(&error);
            }
        }
    }

    fn record_contribution_error(&mut self, error: &FrameError) {
        self.contribution_error = Some(error.contribution_diagnostic());
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
    /// The reconstruction provider prepared for the live scene generation.
    pub fn reconstruction(&self) -> crate::synthetic::Reconstruction {
        match &self.prepared {
            PreparedRenderer::ThreeD(scene) => {
                if self.native_provider_report().is_some_and(|report| {
                    report.pass == crate::apple_tile::metalfx::PASS && report.twin.is_none()
                }) {
                    crate::synthetic::Reconstruction::MetalFx
                } else {
                    scene.reconstruction()
                }
            }
            PreparedRenderer::TwoD(_) => crate::synthetic::Reconstruction::Off,
        }
    }

    /// Set the drawn portion of the prepared output for the next 3D frame.
    ///
    /// # Errors
    /// Reduced scale requires portable temporal reconstruction.
    pub fn set_render_scale(
        &mut self,
        scale: crate::synthetic::RenderScale,
    ) -> Result<(), FrameError> {
        if scale != crate::synthetic::RenderScale::Native
            && !self
                .prepared
                .three_d()
                .is_some_and(PreparedSynthetic::supports_render_scale)
        {
            return Err(FrameError::InvalidState(
                "reduced render scale requires TAAU",
            ));
        }
        self.render_scale = scale;
        Ok(())
    }

    /// The scene half selected at preparation.
    pub fn scene_kind(&self) -> SceneKind {
        match &self.prepared {
            PreparedRenderer::ThreeD(_) => SceneKind::ThreeD,
            PreparedRenderer::TwoD(_) => SceneKind::TwoD,
        }
    }

    /// Capability record for the active device generation.
    pub fn device_capabilities(&self) -> &DeviceCapabilities {
        self.prepared.capabilities()
    }

    /// The temporal reconstructor resolved for the prepared scene, when active.
    pub fn resolved_reconstructor(&self) -> Option<crate::capabilities::Reconstructor> {
        use crate::{capabilities::Reconstructor, synthetic::Reconstruction};
        match self.reconstruction() {
            Reconstruction::MetalFx => Some(Reconstructor::MetalFx),
            Reconstruction::Taau => Some(Reconstructor::Taau),
            Reconstruction::Off | Reconstruction::Reference => None,
        }
    }
    /// Change presentation policy between frames. Invalidate retained extraction
    /// and GPU correspondence because the mapping itself may have changed.
    pub fn source_mut(&mut self) -> &mut V {
        if let PreparedRenderer::TwoD(scene) = &mut self.prepared {
            scene.clear_world_history();
        }
        self.extractor.invalidate_history();
        self.last_view_source = None;
        self.history_reset_pending = true;
        &mut self.source
    }
    /// Edit a continuous presentation parameter, such as camera orientation,
    /// without changing entity mapping or retiring the submitted predecessor.
    /// Cuts and mapping changes must use [`Self::source_mut`] instead; a source
    /// may also express a cut with the view's history epoch/reset flag.
    pub fn continuous_source_mut(&mut self) -> &mut V {
        &mut self.source
    }
    /// Reset correspondence on the next successful scene submission.
    pub fn invalidate_history(&mut self) {
        self.history_reset_pending = true;
    }
    fn visual_lighting(&self) -> Option<FrameLighting> {
        self.visual_lighting
    }

    /// Set the lead to the platform's next display target. Fixed-clock worlds
    /// still advance only to the callback's observed monotonic time.
    pub fn set_presentation_lead(&mut self, lead: Duration) {
        self.presentation_lead = lead.min(self.presentation_lead_limit);
        self.platform_target = None;
        self.lead_status = None;
    }

    /// Bound the display lead consumed by this host. A zero limit samples at
    /// the observation, as needed by a driver that admits pairs at that time.
    pub fn set_presentation_lead_limit(&mut self, limit: Duration) {
        self.presentation_lead_limit = limit;
        self.set_presentation_lead(self.presentation_lead);
    }

    /// Record a runner's platform target and the status of its consumed lead.
    /// Use after setting the lead; timestamps share the world's monotonic origin.
    pub fn set_presentation_target(
        &mut self,
        target: Duration,
        status: crate::frame::TargetLeadStatus,
    ) {
        self.platform_target = Some(target);
        self.lead_status = Some(status);
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
        let [width, height] = config.render_extent;
        if width == 0 || height == 0 {
            return Err(FrameError::InvalidState("render extent must be nonzero"));
        }
        let capacity = scene.instances.len();
        let mut extractor = RenderExtractor::new(capacity, capacity);
        let initial = world.publication()?;
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
        let creations = capabilities.device().ledger();
        let build = creations.region(CreationPhase::Build, "host");
        let mut changed_slots = Vec::with_capacity(capacity);
        let initial_scene_apply = scene.apply_render_columns(extractor.frame(), &mut changed_slots);
        let initial_frame = source.frame(world.source(), 0.0, config.render_extent, None)?;
        scene.view = initial_frame.view;
        let (gpu, presenter) = preparation::prepare_3d_transition(
            scene.ray_admission(capabilities),
            async |selection| {
                preparation::validate_attempt(capabilities, selection, async || {
                    let mut gpu = preparation::prepare_scene(
                        capabilities,
                        queue,
                        &scene,
                        initial_frame.lighting,
                        exposure,
                        config.render_extent,
                        config.surface.is_some(),
                        ContributionPasses::default(),
                        crate::contribution::EffectGraph::default(),
                        None,
                        selection,
                    )
                    .await?;
                    let presenter = match config.surface {
                        Some(profile) => Some(
                            SurfacePresenter::prepare(capabilities, config.render_extent, profile)
                                .await
                                .map_err(|error| match error {
                                    PresentError::Extent => {
                                        FrameError::InvalidState("invalid surface extent")
                                    }
                                    PresentError::UnsupportedFormat => {
                                        FrameError::InvalidState("unsupported surface format")
                                    }
                                    error => FrameError::gpu("surface prepare", error),
                                })?,
                        ),
                        None => None,
                    };
                    gpu.bind_presenter(presenter.as_ref())?;
                    Ok((gpu, presenter))
                })
                .await
            },
        )
        .await?;
        drop(build);
        creations.ready();
        Ok(Self::from_prepared(
            capabilities,
            queue,
            config.render_extent,
            world,
            source,
            scene,
            extractor,
            initial_source,
            initial_extracted,
            initial_scene_apply,
            changed_slots,
            gpu,
            presenter,
        ))
    }

    /// Prepare a sprite scene on the exact output grid. The game supplies the 2D
    /// view and frame rows through its existing scene source.
    ///
    /// # Errors
    /// Refuses named 3D-only requests and invalid initial world, sheet or surface inputs.
    pub async fn new_2d(
        capabilities: &DeviceCapabilities,
        queue: &wgpu::Queue,
        config: FrameConfig,
        mut world: W,
        source: V,
        sprites: crate::sprite::SpriteScene,
        requests: SpriteSceneRequests,
    ) -> Result<Self, FrameError> {
        if requests.temporal_sampling {
            return Err(FrameError::InvalidState(
                "2D scene refuses temporal sampling",
            ));
        }
        if let Some(mode) = requests.culling {
            return Err(FrameError::SpriteCulling(mode));
        }
        DeviceCapabilities::no_reconstructor(requests.reconstruction)?;
        let extent = config
            .surface
            .map_or(config.render_extent, |profile| profile.extent);
        let initial = world.publication()?;
        let initial_source = publication_source(&initial);
        if initial_source != world.source() {
            return Err(FrameError::InvalidState(
                "initial publication is not current",
            ));
        }
        source
            .sprites(initial_source, 0.0, extent)?
            .view
            .clip_scale(extent)
            .map_err(FrameError::Render)?;
        let creations = capabilities.site().ledger();
        let build = creations.region(CreationPhase::Build, "host");
        let mut gpu =
            preparation::prepare_sprites(capabilities, queue, sprites, extent, config.surface)
                .await?;
        let presenter = match config.surface {
            Some(profile) => Some(
                SurfacePresenter::prepare(capabilities, extent, profile)
                    .await
                    .map_err(|error| match error {
                        PresentError::Extent => FrameError::InvalidState("invalid surface extent"),
                        PresentError::UnsupportedFormat => {
                            FrameError::InvalidState("unsupported surface format")
                        }
                        error => FrameError::gpu("sprite surface prepare", error),
                    })?,
            ),
            None => None,
        };
        gpu.bind_presenter(presenter.as_ref())?;
        if let PreparedRenderer::TwoD(prepared) = &mut gpu.prepared {
            prepared.extract(initial_source, |sink| {
                source.extract_sprites(sink, &initial.page);
            });
        }
        drop(initial);
        drop(build);
        creations.ready();
        Ok(Self::from_prepared(
            capabilities,
            queue,
            extent,
            world,
            source,
            SyntheticScene::empty(),
            RenderExtractor::new(0, 0),
            initial_source,
            ExtractReport::default(),
            SceneApplyReport {
                moving: 0,
                dropped_invalid: 0,
            },
            Vec::new(),
            gpu,
            presenter,
        ))
    }

    fn from_prepared(
        capabilities: &DeviceCapabilities,
        queue: &wgpu::Queue,
        render_extent: [u32; 2],
        world: W,
        source: V,
        scene: SyntheticScene,
        extractor: RenderExtractor,
        initial_source: SourceTick,
        initial_extracted: ExtractReport,
        initial_scene_apply: SceneApplyReport,
        changed_slots: Vec<u32>,
        gpu: preparation::PreparedScene,
        presenter: Option<SurfacePresenter>,
    ) -> Self {
        Self {
            device: wgpu::Device::clone(capabilities.device()),
            queue: queue.clone(),
            capture_owner: Arc::new(()),
            submitted_capture: None,
            retired_capture: VecDeque::new(),
            presentation_lead: Duration::ZERO,
            presentation_lead_limit: Duration::MAX,
            observation: Duration::ZERO,
            platform_target: None,
            lead_status: None,
            presenter_generation: u64::from(presenter.is_some()),
            display_headroom: 1.0,
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
            render_extent,
            render_scale: crate::synthetic::RenderScale::Native,
            governor: None,
            governor_display_tick: None,
            governor_missed_ticks: 0,
            changed_slots,
            prepared: gpu.prepared,
            presenter,
            slots: gpu.slots,
            retired_slots: Vec::with_capacity(IN_FLIGHT),
            completed: VecDeque::with_capacity(COMPLETED_REPORTS),
            next_submission_order: 0,
            timing_stats: TimingStats::default(),
            frame_counters: FrameCounters::default(),
            submitted_clock: None,
            timing_requested: true,
            ticks: tick::TickStage::default(),
            native_failed_generation: None,
            dropped_views: 0,
            dropped_stale_views: 0,
            dropped_reports: 0,
            dropped_capture_rows: 0,
            endpoint_readbacks: 0,
            dropped_surface: 0,
            visual_lighting: None,
            visual_revision: None,
            visual_owner: Arc::new(()),
            contribution: None,
            contribution_error: None,
            creations: capabilities.site().ledger(),
            #[cfg(feature = "creation-counters")]
            retired_creations: None,
        }
    }

    /// Copy presentation inputs once for an asynchronous visual preparation.
    /// This copies the finite scene only on accepted edits, never on ticks.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn visual_prepare_inputs(&self) -> VisualPrepareInputs {
        let prepared = self.prepared.three_d();
        VisualPrepareInputs {
            queue: self.queue.clone(),
            capabilities: self.prepared.capabilities().clone(),
            scene: self.scene.clone(),
            retained_ray_meshes: prepared.and_then(PreparedSynthetic::retained_ray_meshes),
            surface: self.presenter.is_some(),
            contribution_passes: prepared.map_or_else(
                ContributionPasses::default,
                PreparedSynthetic::contribution_passes,
            ),
            effect_graph: prepared
                .map_or_else(crate::contribution::EffectGraph::default, |scene| {
                    scene.effect_graph().clone()
                }),
            render_extent: self.render_extent,
            owner: Arc::clone(&self.visual_owner),
            creations: self.creations.detach(),
            kind: self.scene_kind(),
            supports_candidates: preparation::supports_candidates(self.contribution.as_deref()),
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
    /// Returns a binding, upload or failed Maps retry without replacing the active view.
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
        let creations = self.creations.clone();
        let _region = creations.region(CreationPhase::Transition, "publish-visual");
        let selection = candidate
            .scene
            .as_ref()
            .ok_or(FrameError::InvalidState(
                "visual candidate already published",
            ))?
            .prepared
            .ray_structure_selection();
        let queue = self.queue.clone();
        let extent = candidate
            .extent
            .as_ref()
            .map_or(self.render_extent, |(extent, _)| *extent);
        let frame = self.replacement_frame(extent, Some(candidate.lighting))?;
        let publication = if preparation::is_candidate_transition(selection) {
            let detached = candidate.scene.take().ok_or(FrameError::InvalidState(
                "visual candidate already published",
            ))?;
            let capabilities = detached.prepared.capabilities().clone();
            let exposure = detached.prepared.fallback_exposure();
            let presenter = self.presenter.take();
            let replacement = candidate.extent.as_ref().map(|(_, presenter)| presenter);
            let staged = self.stage_attached_3d_transition(
                selection,
                Some(detached),
                preparation::TransitionInputs {
                    capabilities: &capabilities,
                    queue: &queue,
                    extent,
                    presenter: replacement.or(presenter.as_ref()),
                    frame: &frame,
                    exposure,
                },
            );
            self.presenter = presenter;
            preparation::ScenePublication::Candidate(staged?)
        } else {
            let scene = candidate.scene.as_mut().ok_or(FrameError::InvalidState(
                "visual candidate already published",
            ))?;
            if candidate.extent.is_none() {
                scene.bind_presenter(self.presenter.as_ref())?;
            }
            let (view, inherit_rows) = self.refresh_replacement(scene, &queue, &frame)?;
            if inherit_rows {
                scene.prepared.commit_mesh_rows(&mut self.prepared);
            }
            preparation::ScenePublication::Legacy(
                candidate.scene.take().ok_or(FrameError::InvalidState(
                    "visual candidate already published",
                ))?,
                Some(view),
            )
        };
        let contribution_prepared = self.install_scene_publication(publication);
        self.visual_lighting = Some(candidate.lighting);
        self.visual_revision = candidate.revision;
        if let Some((extent, presenter)) = candidate.extent.take() {
            self.render_extent = extent;
            self.presenter = Some(presenter);
            self.presenter_generation += 1;
        }
        self.reset_replacement_history();
        self.visual_owner = Arc::new(());
        if !contribution_prepared {
            self.rebuild_contribution();
        }
        Ok(true)
    }

    fn replacement_frame(
        &self,
        extent: [u32; 2],
        lighting: Option<FrameLighting>,
    ) -> Result<SceneFrame, FrameError> {
        self.source.frame(
            self.world.source(),
            self.presentation_alpha(),
            extent,
            lighting,
        )
    }

    fn refresh_replacement(
        &mut self,
        replacement: &mut preparation::PreparedScene,
        queue: &wgpu::Queue,
        frame: &SceneFrame,
    ) -> Result<(SyntheticView, bool), FrameError> {
        let authority = self.world.source();
        let alpha = self.presentation_alpha();
        let view = frame.view.with_history_reset(true);
        let previous = std::mem::replace(&mut self.scene.view, view);
        replacement.prepared.inherit_submitted_view(&self.prepared);
        let refreshed = (|| {
            let inherit_rows = replacement
                .prepared
                .stage_mesh_rows(&self.prepared, queue)
                .map_err(|error| FrameError::gpu("replacement mesh rows", error))?;
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
                .map_err(|error| FrameError::gpu("replacement frame", error))?;
            self.changed_slots.clear();
            let count = u32::try_from(self.scene.instances.len())
                .map_err(|_| FrameError::InvalidState("scene instance slots exceed u32"))?;
            self.changed_slots.extend(0..count);
            replacement
                .prepared
                .upload_instances(queue, &self.scene.instances, &self.changed_slots)
                .map_err(|error| FrameError::gpu("replacement full upload", error))?;
            Ok((view, inherit_rows))
        })();
        self.scene.view = previous;
        refreshed
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
        self.prepared
            .three_d()
            .map_or(CullMode::Direct, PreparedSynthetic::camera_culling)
    }
    /// Capability decision belonging to the currently published generation.
    pub fn culling_selection(&self) -> crate::synthetic::CullingSelection {
        self.prepared.three_d().map_or(
            crate::synthetic::CullingSelection {
                requested: CullMode::Direct,
                resolved: CullMode::Direct,
                reason: crate::synthetic::CullingReason::NoCameraCulling,
            },
            PreparedSynthetic::culling_selection,
        )
    }

    /// Bytes of Sun visibility history shared by this host's active frame slots.
    #[must_use]
    pub fn sun_penumbra_history_bytes(&self) -> u64 {
        self.slots
            .first()
            .and_then(|slot| slot.target.three_d())
            .map_or(0, OffscreenTarget::sun_penumbra_history_bytes)
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

    /// Inspect prepared ordinary slot texture storage without allocating GPU objects.
    #[must_use]
    pub fn storage_report(&self) -> String {
        let slots = self
            .slots
            .iter()
            .enumerate()
            .map(|(index, slot)| format!("slot={index}\n{}", slot.target.storage_report()))
            .collect::<Vec<_>>()
            .join("\n");
        match &self.prepared {
            PreparedRenderer::ThreeD(_) => slots,
            PreparedRenderer::TwoD(scene) => format!("{}\n{slots}", scene.storage_report()),
        }
    }

    /// Resident view-volume bytes, when declared by the prepared 3D scene.
    #[must_use]
    pub fn view_volume_storage(&self) -> Option<crate::atmosphere::ViewVolumeStorage> {
        self.prepared.three_d()?.view_volume_storage()
    }

    /// Prepared main 3D forward shader, available for diagnostics.
    #[must_use]
    pub fn scene_shader_source(&self) -> Option<&str> {
        Some(self.prepared.three_d()?.scene_shader_source())
    }
    /// Slots in the latest extracted 2D world's retained back-to-front order.
    /// Frame rows are excluded. A 3D scene has no sprite order.
    #[must_use]
    pub fn sprite_world_order(&self) -> Option<&[u32]> {
        match &self.prepared {
            PreparedRenderer::TwoD(scene) => Some(scene.world_order()),
            PreparedRenderer::ThreeD(_) => None,
        }
    }

    /// Inspect the prepared slot's graph without encoding or allocating GPU work.
    /// # Errors
    /// Refuses a stale target during an incomplete scene transition.
    pub fn scene_graph(&self) -> Result<crate::graph::CompiledGraph, FrameError> {
        self.scene_graph_variant(false)
    }

    /// Inspect the prepared graph with its optional submitted-image copy.
    /// # Errors
    /// Refuses a stale target during a scene transition.
    pub fn scene_graph_variant(
        &self,
        capture: bool,
    ) -> Result<crate::graph::CompiledGraph, FrameError> {
        if let FrameTarget::TwoD(target) = &self.slots[0].target {
            return Ok(target
                .inspection_plan(capture, self.sprite_sdr_capture_enabled())
                .clone());
        }
        // A slot that has not submitted since a cooked visual publication
        // can still carry the previous frame revision.
        let mut slots = self.slots.iter();
        slots
            .find_map(|slot| self.prepared.graph(&slot.target, capture).ok())
            .ok_or_else(|| FrameError::gpu("graph inspection", RenderError::StaleTarget))
    }
    /// Read-only reconstruction witness of the latest submitted frame, before report retirement.
    /// This texture is queue ordered after that frame. With reconstruction enabled, select
    /// [`crate::synthetic::DisplayOverlay::TemporalInputs`] to declare the diagnostic reader.
    #[must_use]
    pub fn submitted_temporal_witness(&self) -> Option<&wgpu::Texture> {
        self.latest_submitted_target()?.temporal_witness()
    }

    /// Queue-ordered depth layers of the latest submitted diagnostic frame.
    #[cfg(feature = "creation-counters")]
    #[must_use]
    pub fn submitted_shadow_witness(&self) -> Option<&wgpu::Texture> {
        self.latest_submitted_target()?.shadow_witness()
    }

    /// Raw Sun disc ray results from the newest in-flight submitted frame.
    #[cfg(feature = "creation-counters")]
    #[must_use]
    pub fn submitted_sun_disc_witness(&self) -> Option<&wgpu::Texture> {
        self.latest_submitted_target()?.sun_disc_witness()
    }

    fn latest_submitted_target(&self) -> Option<&OffscreenTarget> {
        self.slots
            .iter()
            .filter_map(|slot| {
                let sample = slot.active_sample().or_else(|| slot.deferred_sample())?;
                Some((sample.submission_order, &slot.target))
            })
            .max_by_key(|(order, _)| *order)
            .and_then(|(_, target)| target.three_d())
    }

    /// Pipeline creation events in this generation, when creation instrumentation is enabled.
    /// A runtime cadence change must leave this count unchanged.
    #[cfg_attr(
        feature = "creation-counters",
        expect(
            clippy::unnecessary_wraps,
            reason = "the same instrumentation API returns None when counters are disabled"
        )
    )]
    #[cfg_attr(
        not(feature = "creation-counters"),
        expect(
            clippy::unused_self,
            reason = "the counters-disabled build preserves the generation instrumentation API without gathering data"
        )
    )]
    pub fn pipeline_creation_count(&self) -> Option<u64> {
        #[cfg(feature = "creation-counters")]
        {
            Some(
                self.creations
                    .report()
                    .pipelines
                    .iter()
                    .map(|row| row.render + row.compute + row.mesh)
                    .sum(),
            )
        }
        #[cfg(not(feature = "creation-counters"))]
        {
            None
        }
    }

    /// GPU object creations on the current device generation by thread and
    /// phase, with any pipeline built in play after readiness. Native-object
    /// sites are reported absent; an uninstrumented backend reports so.
    #[cfg(feature = "creation-counters")]
    pub fn creation_report(&self) -> CreationReport {
        self.creations.report()
    }

    /// The final creation report of the device generation the last device
    /// replacement retired, if any.
    #[cfg(feature = "creation-counters")]
    pub fn retired_creation_report(&self) -> Option<&CreationReport> {
        self.retired_creations.as_ref()
    }

    /// Recompile the surface extent declaration after the platform configures
    /// a nonzero replacement; in-flight old textures remain GPU-owned.
    /// # Errors
    /// Refuses an invalid extent or a host without a surface presenter.
    pub fn resize_surface(&mut self, extent: [u32; 2]) -> Result<(), FrameError> {
        self.check_native_completion()?;
        if self.scene_kind() == SceneKind::TwoD {
            return if self.resize_scene_surface(extent)? {
                Ok(())
            } else {
                Err(FrameError::InvalidState(
                    "2D surface resize requires a nonzero extent and retired frames",
                ))
            };
        }
        let _region = self
            .creations
            .region(CreationPhase::Transition, "resize-surface");
        self.presenter
            .as_mut()
            .ok_or(FrameError::InvalidState("host has no surface presenter"))?
            .resize(&self.device, extent)
            .map_err(|error| match error {
                PresentError::Extent => FrameError::InvalidState("invalid surface extent"),
                error => FrameError::gpu("surface resize", error),
            })?;
        preparation::bind_presenter(&self.prepared, &mut self.slots, self.presenter.as_ref())?;
        self.resize_drawable_capture();
        self.presenter_generation += 1;
        if self.native_provider_report().is_some() {
            self.invalidate_history();
        }
        Ok(())
    }

    /// Engine-owned identity of the last successful queue submission.
    pub fn last_submission(&self) -> Option<u64> {
        self.next_submission_order.checked_sub(1)
    }

    /// Latest submitted view's clock on the current device, even while GPU work
    /// is pending. Empty until this device submits its first view.
    pub fn last_submitted_clock(&self) -> Option<SubmittedFrameClock> {
        self.submitted_clock
    }

    /// Number of submitted render packets awaiting GPU completion or readback.
    pub fn pending_frames(&self) -> usize {
        self.slots
            .iter()
            .chain(&self.retired_slots)
            .flat_map(FrameSlot::samples)
            .count()
    }

    /// Whether a frame slot was free at the last poll, so the next encode can take one.
    pub fn has_free_slot(&self) -> bool {
        self.slots
            .iter()
            .any(|slot| matches!(slot.state, SlotState::Free))
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
            .chain(&self.retired_slots)
            .flat_map(FrameSlot::samples)
            .map(|sample| sample.submission_order)
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
    /// Returns a clock, simulation or GPU fault. A surface encode error can
    /// return after the scene completed recording and was submitted.
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
        self.observation = now;
        let wall_start = Instant::now();
        self.governor_boundary(now);
        self.poll_gpu()?;
        self.commit_governor(now)?;
        let advanced = self.world.advance(now)?;
        let report = self.record_world_advance(&advanced);
        self.ask_tick_stage_for_view()?;
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
        let advanced = self.world.advance(now)?;
        Ok((self.record_world_advance(&advanced), advanced))
    }

    fn record_world_advance(&mut self, advanced: &WorldAdvance) -> DriveReport {
        let report = DriveReport {
            tick: advanced.tick,
            overload: advanced.overload,
            completed_ticks: advanced.completed_ticks,
            queued_ticks: advanced.queued_ticks,
            projected_changes: advanced.projected_changes,
            projection_lost_ticks: advanced.projection_lost_ticks,
            submitted: false,
            surface_presented: false,
            surface_fault: None,
            dropped_view: advanced.skipped_publications != 0,
            wall: Duration::ZERO,
        };
        self.dropped_views = self
            .dropped_views
            .saturating_add(advanced.skipped_publications);
        report
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
        if self.presenter.is_some() && surface.is_none() {
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
        let snapshot = self.world.publication()?;
        let source = publication_source(&snapshot);
        let extractor = &mut self.extractor;
        let reused_source =
            extractor.frame().source() == Some(source) && self.last_view_source == Some(source);
        let mut extract_cpu = None;
        let extracted = if let PreparedRenderer::TwoD(prepared) = &mut self.prepared {
            extract_cpu = Some(prepared.extract(source, |sink| {
                self.source.extract_sprites(sink, &snapshot.page);
            }));
            ExtractReport::default()
        } else if reused_source {
            self.last_extracted
        } else {
            self.source.extract(extractor, &snapshot.page)
        };
        drop(snapshot);
        Ok(SubmitFrame {
            extracted,
            reused_source,
            extract_cpu: extract_cpu.unwrap_or_else(|| extract_start.elapsed()),
            tick_cpu,
            completed_ticks,
            queued_ticks,
            source,
            presentation_alpha: self.presentation_alpha(),
            display_phase: self.world.display_phase_at(self.presentation_lead),
            clock_interpolating: self.world.is_interpolating(),
            tick_due_time: self.world.tick_due_time(),
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
        if self.scene_kind() == SceneKind::TwoD {
            return self.prepare_sprite_frame(slot_index, report, frame);
        }
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
        let frame_scene = self.source.frame(
            frame.source,
            frame.presentation_alpha,
            self.render_extent,
            self.visual_lighting(),
        )?;
        self.source.sample_local_lights(
            frame.source,
            frame.presentation_alpha,
            &mut self.scene.local_lights,
        );
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
        self.history_reset_pending |= self.prepared.commit_lamp_quality(&self.queue);
        if let Some(scene) = self.prepared.three_d_mut() {
            scene.commit_aerial_quality();
            scene.commit_sun_penumbra_quality();
            scene.commit_ambient_quality();
        }
        if self.contribution.as_mut().is_some_and(|entry| {
            entry.commit_quality_knobs() == crate::contribution::QualityInvalidation::ViewHistory
        }) {
            self.history_reset_pending = true;
        }
        self.record_committed_governor_knobs();
        self.history_reset_pending |= self
            .prepared
            .record_mesh_rows(&self.queue)
            .map_err(|error| FrameError::gpu("mesh row update", error))?;
        let scale = if self
            .prepared
            .three_d()
            .is_some_and(PreparedSynthetic::supports_render_scale)
        {
            self.render_scale
        } else {
            crate::synthetic::RenderScale::Native
        };
        self.scene.view = self
            .view_with_history_reset(&frame_scene.view)
            .with_render_scale(scale);
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
            return Err(FrameError::gpu("live view update", error));
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
                return Err(FrameError::gpu("instance update", error));
            }
        };
        self.upload_reset_pending = false;
        let upload_cpu = upload_start.elapsed();
        let visual_revision = self.visual_revision;
        self.prepared
            .set_effect_time(crate::contribution::EffectTime {
                epoch: frame.source.epoch,
                timeline: frame.source.timeline,
                tick: frame.source.tick,
                tick_duration: self.world.tick_duration(),
                alpha: frame.presentation_alpha,
                display_phase: frame.display_phase,
                capture: false,
            });
        Ok(Some(PreparedSample {
            sample: FrameSample {
                tick: frame.source.tick,
                presentation_alpha: frame.presentation_alpha,
                display_phase: frame.display_phase,
                observation: self.observation,
                presentation_lead: self.presentation_lead,
                clock_interpolating: frame.clock_interpolating,
                tick_due_time: frame.tick_due_time,
                platform_target: self.platform_target,
                lead_status: self.lead_status,
                submission_order: 0,
                history_identity: self.prepared.history_identity(),
                jitter_phase: self
                    .prepared
                    .three_d()
                    .and_then(PreparedSynthetic::pending_jitter_phase),
                render_extent: scale.extent(self.render_extent),
                governor: self.governor_frame(),
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
                surface_fault: None,
                timing_fault: false,
                native_segment: None,
                timing_requested: self.timing_requested || self.governor.is_some(),
                culling_mode: self.prepared.camera_culling(),
                culling_selection: self.prepared.culling_selection(),
                shadow_selection: self.prepared.shadow_selection(),
                ambient_occlusion: self.prepared.ambient_occlusion_selection(),
                camera_culling: None,
                sprites: None,
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

    fn view_with_history_reset(&self, view: &SyntheticView) -> SyntheticView {
        view.with_history_reset(
            view.history_reset()
                || self.history_reset_pending
                || view.history_epoch() != self.submitted_view_history_epoch,
        )
        .with_output_extent(
            self.presenter
                .as_ref()
                .map_or(self.render_extent, SurfacePresenter::extent),
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
        mut prepared: PreparedSample,
    ) -> Result<RecordedScene, FrameError> {
        self.refresh_capture_capacity();
        let start = Instant::now();
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let slot = &mut self.slots[slot_index];
        let native = slot
            .target
            .three_d()
            .map(|target| {
                let view = self
                    .prepared
                    .three_d()
                    .and_then(PreparedSynthetic::pending_view)
                    .unwrap_or(&self.scene.view);
                target.reserve_native(view)
            })
            .transpose()
            .map_err(FrameError::Render)?
            .flatten();
        let mut segments = Vec::new();
        if let FrameTarget::ThreeD(target) = &mut slot.target {
            target.arm_culling_stats();
        }
        let timing_requested = prepared.sample.timing_requested;
        let capture = submitted_capture::reserve(
            &mut self.submitted_capture,
            self.generation,
            slot.target.capture_extent(),
            slot.target.capture_source(),
            timing_requested,
        );
        let capture_slot = capture.as_ref().map(|(index, _)| *index);
        let timings = timing_requested.then_some(&mut slot.timing.queries);
        let readback = capture.map(|(_, readback)| readback);
        let contribution = self.contribution.as_deref_mut();
        let encoded = match (&self.prepared, &slot.target) {
            (PreparedRenderer::ThreeD(scene), FrameTarget::ThreeD(target)) => scene
                .encode_contributed(
                    &mut encoder,
                    self.generation,
                    target,
                    timings,
                    readback,
                    contribution,
                    native.as_ref(),
                    &mut segments,
                ),
            (PreparedRenderer::TwoD(scene), FrameTarget::TwoD(target)) => scene.record(
                &mut encoder,
                target,
                timings,
                readback,
                contribution,
                prepared.sample.sprites.as_mut(),
                crate::contribution::EffectTime {
                    epoch: prepared.source.epoch,
                    timeline: prepared.source.timeline,
                    tick: prepared.source.tick,
                    tick_duration: self.world.tick_duration(),
                    alpha: prepared.sample.presentation_alpha,
                    display_phase: prepared.sample.display_phase,
                    capture: false,
                },
            ),
            _ => Err(RenderError::StaleTarget),
        };
        if let Err(error) = encoded {
            drop(encoder);
            if let Some(index) = capture_slot
                && let Some(tap) = self.submitted_capture.as_mut()
            {
                tap.abandon(index);
            }
            let discarded = slot.discard_timing(self.generation, &mut self.timing_stats);
            self.flush_staged_uploads()?;
            discarded.map_err(|timing| FrameError::gpu("render timing discard", timing))?;
            return Err(FrameError::gpu("render encode", error));
        }
        Ok(RecordedScene {
            native,
            segments,
            slot_index,
            capture_slot,
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
        self.finish_encoded_with(recorded, report, surface, wall_start, || Ok(()))
    }

    // A platform/queue admission fault must be observed before committing history.
    // The ordinary queue API is infallible; explicit refusal and device-loss
    // adapters can reject here without manufacturing a submitted sample.
    fn finish_encoded_with(
        &mut self,
        recorded: RecordedScene,
        report: &mut DriveReport,
        surface: Option<&SurfaceOutput<'_>>,
        wall_start: Instant,
        admit: impl FnOnce() -> Result<(), FrameError>,
    ) -> Result<(), FrameError> {
        let RecordedScene {
            native,
            segments,
            slot_index,
            mut capture_slot,
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
        let admission = self.check_native_completion().and_then(|()| admit());
        let slot = &mut self.slots[slot_index];
        if let Err(error) = admission {
            drop(encoder);
            if let Some(index) = capture_slot
                && let Some(tap) = self.submitted_capture.as_mut()
            {
                tap.abandon(index);
            }
            slot.discard_timing(self.generation, &mut self.timing_stats)
                .map_err(|e| FrameError::gpu("submission discard", e))?;
            self.history_reset_pending = true;
            return Err(error);
        }

        let edr_capture = self
            .submitted_capture
            .as_ref()
            .is_some_and(|tap| tap.source == crate::synthetic::CaptureSource::Edr);
        let (surface_presented, surface_error) = presentation::encode_surface_pass(
            &self.prepared,
            self.presenter.as_ref(),
            slot,
            surface,
            &mut encoder,
            sample.timing_requested,
            capture_slot.is_some(),
            report,
        );
        if self.presenter.is_some() && !surface_presented {
            self.dropped_surface = self.dropped_surface.saturating_add(1);
            report.dropped_view = true;
        }
        if self.presenter.is_some()
            && !surface_presented
            && let Some(native) = native
        {
            drop(encoder);
            drop(segments);
            self.discard_native_presentation(slot_index, capture_slot, native, surface.is_none())?;
            report.wall = wall_start.elapsed();
            return surface_encode_outcome(report, surface_error);
        }
        if edr_capture {
            match submitted_capture::copy_drawable(
                self.submitted_capture.as_mut(),
                self.generation,
                surface.filter(|_| surface_presented),
                &self.prepared,
                &mut encoder,
                sample.timing_requested,
            ) {
                Ok(index) => capture_slot = index,
                Err(error) => {
                    drop(encoder);
                    slot.discard_timing(self.generation, &mut self.timing_stats)
                        .map_err(|discard| FrameError::gpu("drawable copy discard", discard))?;
                    self.flush_staged_uploads()?;
                    return Err(error);
                }
            }
        }
        if let Some(index) = capture_slot
            && let Some(tap) = self.submitted_capture.as_mut()
            && let Err(error) = tap.finish_timing(index, &encoder)
        {
            drop(encoder);
            tap.abandon(index);
            slot.discard_timing(self.generation, &mut self.timing_stats)
                .map_err(|discard| FrameError::gpu("capture submission discard", discard))?;
            self.flush_staged_uploads()?;
            return Err(error);
        }
        sample.submission_order = self.next_submission_order;
        if sample.timing_requested {
            let frame = TimingFrame {
                device_generation: self.generation,
                submission_order: sample.submission_order,
            };
            match slot.timing.queries.finish_frame(frame, &encoder) {
                Ok(gathered) => self.timing_stats.gathered += u64::from(gathered),
                Err(error) => {
                    drop(encoder);
                    if let Some(index) = capture_slot
                        && let Some(tap) = self.submitted_capture.as_mut()
                    {
                        tap.abandon(index);
                    }
                    let discarded = slot.discard_timing(self.generation, &mut self.timing_stats);
                    self.flush_staged_uploads()?;
                    discarded.map_err(|discard| FrameError::TimingDiscard {
                        finish: error,
                        discard,
                    })?;
                    return Err(FrameError::gpu("finish timestamps", error));
                }
            }
        }
        if let Some(frame) = slot.timing.queries.pending_frame() {
            self.track_frame_counters(slot_index, frame.submission_order);
        }
        let slot = &mut self.slots[slot_index];
        slot.render_done.store(false, Ordering::Release);
        self.queue.submit(
            segments
                .into_iter()
                .chain(std::iter::once(encoder.finish())),
        );
        let native = native.map(|native| {
            let (completion, segment) = native.submitted(
                self.generation,
                self.next_submission_order,
                sample.timing_requested,
            );
            sample.native_segment = Some(segment);
            completion
        });
        if let PreparedRenderer::ThreeD(scene) = &mut self.prepared {
            scene.submit_mesh_rows();
        }
        if let Some(contribution) = self.contribution.as_mut() {
            contribution.submitted(true);
        }
        let done = Arc::clone(&slot.render_done);
        self.queue.on_submitted_work_done(move || {
            if let Some(native) = native {
                native.completed();
            }
            done.store(true, Ordering::Release);
        });
        self.creations.frame();
        if let (PreparedRenderer::ThreeD(scene), FrameTarget::ThreeD(target)) =
            (&self.prepared, &mut slot.target)
        {
            target.map_culling_stats(scene.camera_culling(), scene.active_instance_count());
            slot.timing.layout = Some(SceneTimingLayout::from((&**scene, &**target)));
        }
        sample.encode_cpu = encode_cpu + finish_start.elapsed();
        sample.wall = wall_start.elapsed();
        sample.render_cpu = if self.world.is_threaded() {
            sample.wall
        } else {
            sample.wall.saturating_sub(sample.tick_cpu)
        };
        sample.surface_presented = surface_presented;
        sample.surface_fault = report.surface_fault;
        self.submitted_clock = Some(SubmittedFrameClock {
            tick: sample.tick,
            presentation_alpha: sample.presentation_alpha,
            submission_order: sample.submission_order,
            display_time: sample.observation.saturating_add(sample.presentation_lead),
            observation: sample.observation,
            tick_due_time: sample.tick_due_time,
            surface_presented,
        });
        if let Some(index) = capture_slot
            && let Some(tap) = self.submitted_capture.as_mut()
        {
            tap.submitted(
                index,
                &submitted_capture::Stamp {
                    sample: sample.clone(),
                    source,
                    history_epoch: view_history_epoch,
                    submitted_at: Instant::now(),
                    device_generation: self.generation,
                    presenter_generation: self.presenter_generation,
                    presentation_lead: self.presentation_lead,
                },
            );
        }
        self.next_submission_order = self.next_submission_order.wrapping_add(1);
        slot.timing.readback = capture_slot.is_some() && !edr_capture;
        slot.state = SlotState::Rendering(sample);
        report.submitted = true;
        report.surface_presented = surface_presented;
        report.wall = wall_start.elapsed();
        self.commit_submitted_view(source, view_history_epoch);
        surface_encode_outcome(report, surface_error)
    }

    fn commit_submitted_view(&mut self, source: SourceTick, view_history_epoch: u64) {
        if let PreparedRenderer::ThreeD(scene) = &mut self.prepared {
            scene.commit_frame();
        }
        if let Some(governor) = &mut self.governor {
            governor.submitted();
        }
        self.history_reset_pending = false;
        self.submitted_view_history_epoch = view_history_epoch;
        self.last_view_source = Some(source);
    }

    /// Render and read back the current committed scene once for visual evidence.
    /// This capture is outside the timed frame path. Native waits here; a
    /// browser uses [`Self::begin_capture`] and [`Self::try_finish_capture`].
    /// # Errors
    /// Returns a GPU encode, completion or readback error.
    pub fn capture_pixels(&mut self) -> Result<Vec<u8>, FrameError> {
        let mut pending = self.begin_capture()?;
        self.wait(pending.submitted.clone(), "capture poll")?;
        // The submission being complete does not mean the readback's map callback has run here:
        // another thread's poll of the same device can collect the callback and fire it a moment
        // later. Keep polling until this capture's pixels arrive, within the same GPU_WAIT bound.
        let deadline = Instant::now() + GPU_WAIT;
        loop {
            if let Some(pixels) = self.try_finish_capture(&mut pending)? {
                return Ok(pixels);
            }
            if Instant::now() >= deadline {
                return Err(FrameError::InvalidState("capture readback incomplete"));
            }
            std::thread::yield_now();
        }
    }

    /// Record and submit a capture of the current committed endpoint without
    /// waiting. The caller owns the returned token and finishes it with
    /// [`Self::try_finish_capture`]. Capture never advances authority or
    /// commits display history.
    /// # Errors
    /// Returns a GPU encode or submission error.
    pub fn begin_capture(&mut self) -> Result<PendingCapture, FrameError> {
        let (pixels, submitted) = self.record_endpoint(true)?;
        self.endpoint_readbacks += 1;
        Ok(PendingCapture {
            pixels: pixels.ok_or(FrameError::InvalidState("capture recorded no readback"))?,
            submitted,
            owner: Arc::clone(&self.capture_owner),
            generation: self.generation,
            tick: self.world.source().tick,
            extent: self.render_extent,
        })
    }

    /// Endpoint captures this host has recorded with a readback, through
    /// [`Self::begin_capture`] or [`Self::capture_pixels`].
    #[must_use]
    pub fn endpoint_readbacks(&self) -> u64 {
        self.endpoint_readbacks
    }

    /// Draw and submit the current committed endpoint once, as
    /// [`Self::begin_capture`] records it, with no readback and no wait. The
    /// scene contribution records its staged work into the frame, so a caller
    /// waiting for content to be drawn advances it this way. Like a capture,
    /// this never advances authority or commits display history.
    /// # Errors
    /// Returns a GPU encode or submission error.
    pub fn render_once(&mut self) -> Result<wgpu::SubmissionIndex, FrameError> {
        self.record_endpoint(false).map(|(_, submitted)| submitted)
    }

    fn record_endpoint(
        &mut self,
        readback: bool,
    ) -> Result<
        (
            Option<crate::synthetic::PixelReadback>,
            wgpu::SubmissionIndex,
        ),
        FrameError,
    > {
        if self.native_provider_report().is_some() {
            return Err(FrameError::InvalidState(
                "native providers use the submitted-image tap",
            ));
        }
        let creations = self.creations.clone();
        let _region = creations.region(CreationPhase::Transition, "capture");
        if self.scene_kind() == SceneKind::TwoD {
            return self.record_sprite_endpoint(readback);
        }
        let needs_extract = self.extractor.frame().source().is_none_or(|source| {
            source.epoch != self.world.source().epoch
                || source.timeline != self.world.source().timeline
                || source.tick != self.world.source().tick
        });
        let extracted = if needs_extract {
            let snapshot = self.world.publication()?;
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
        self.source.sample_local_lights(
            self.world.source(),
            capture_alpha,
            &mut self.scene.local_lights,
        );
        let frame_scene = self.source.frame(
            self.world.source(),
            capture_alpha,
            self.render_extent,
            self.visual_lighting(),
        )?;
        let lighting = frame_scene.lighting;
        self.history_reset_pending |= self
            .prepared
            .record_mesh_rows(&self.queue)
            .map_err(|error| FrameError::gpu("capture mesh row update", error))?;
        self.scene.view = self
            .view_with_history_reset(&frame_scene.view)
            .without_jitter();
        if self.prepared.reconstruction() != crate::synthetic::Reconstruction::Off {
            // This detached endpoint oracle owns no submitted temporal sample.
            // The real T2 tap instead captures the ordinary reconstructed frame.
            self.scene.view = self
                .scene
                .view
                .with_reconstruction(false)
                .with_history_reset(true);
        }
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
                return Err(FrameError::gpu("capture instance update", error));
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
        .map_err(|error| FrameError::gpu("capture target", error))?;
        if let Err(error) = self.prepared.update_frame(
            &self.queue,
            &mut target,
            &self.scene,
            lighting,
            capture_alpha,
        ) {
            self.retry_upload();
            return Err(FrameError::gpu("capture view update", error));
        }
        let source = self.world.source();
        self.prepared
            .set_effect_time(crate::contribution::EffectTime {
                epoch: source.epoch,
                timeline: source.timeline,
                tick: source.tick,
                tick_duration: self.world.tick_duration(),
                alpha: capture_alpha,
                display_phase: 1.0,
                capture: true,
            });
        let mut pixels = readback.then(|| target.readback(&self.device));
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        self.prepared
            .encode_contributed(
                &mut encoder,
                self.generation,
                &target,
                None,
                pixels.as_mut(),
                self.contribution.as_deref_mut(),
                None,
                &mut Vec::new(),
            )
            .map_err(|error| FrameError::gpu("capture render/readback", error))?;
        let submitted = self.queue.submit([encoder.finish()]);
        self.prepared.submit_mesh_rows();
        if let Some(contribution) = self.contribution.as_mut() {
            contribution.submitted(false);
        }
        Ok((pixels, submitted))
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
            .map_err(|error| FrameError::gpu("capture poll", error))?;
        pending
            .pixels
            .try_collect(self.generation)
            .map_err(|error| FrameError::gpu("capture collect", error))
    }

    fn wait(
        &self,
        submitted: wgpu::SubmissionIndex,
        operation: &'static str,
    ) -> Result<(), FrameError> {
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submitted),
                timeout: Some(GPU_WAIT),
            })
            .map_err(|error| FrameError::gpu(operation, error))?;
        Ok(())
    }

    fn flush_staged_uploads(&mut self) -> Result<(), FrameError> {
        self.check_native_completion()?;
        tokens::staging_flush::flush(&self.queue);
        Ok(())
    }

    fn poll_gpu(&mut self) -> Result<(), FrameError> {
        self.check_native_completion()?;
        self.device
            .poll(wgpu::PollType::Poll)
            .map_err(|error| FrameError::gpu("GPU poll", error))?;
        // Inspect every completed native segment before resolving any slot's
        // queries: a later failed slot must also forbid earlier readback work.
        self.check_native_completion()?;
        self.collect_frame_timings();
        for slot in self.slots.iter_mut().chain(&mut self.retired_slots) {
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
                        .or_else(|| slot.target.take_culling_counts_if_3d());
                    if slot.timing.queries.pending_frame().is_none() {
                        if let Some(governor) = &mut self.governor {
                            governor.observe(&sample);
                        }
                        retain_sample(&mut self.completed, &mut self.dropped_reports, sample);
                        continue;
                    }
                    let mut encoder = self
                        .device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                    match slot
                        .timing
                        .queries
                        .resolve_if_ready(self.generation, &mut encoder)
                    {
                        Ok(true) => {
                            self.queue.submit([encoder.finish()]);
                            // The elapsed offscreen owner has no numbered
                            // display-link observer. Let it reuse the target
                            // while timing maps; keep drawable admission's
                            // existing bound on submitted reports.
                            if self.governor.is_some() && self.governor_display_tick.is_none() {
                                slot.defer_timing(sample);
                            } else {
                                slot.state = SlotState::Resolving(sample);
                            }
                        }
                        Ok(false) => slot.state = SlotState::Rendering(sample),
                        Err(error) => {
                            slot.timing
                                .refuse_timing(&mut sample, &mut self.timing_stats, error);
                            slot.timing.queries.recover_readback(&self.device);
                            if let Some(governor) = &mut self.governor {
                                governor.observe(&sample);
                            }
                            retain_sample(&mut self.completed, &mut self.dropped_reports, sample);
                        }
                    }
                }
                SlotState::Resolving(_) => slot.state = state,
            }
        }
        self.retired_slots.retain(FrameSlot::is_pending);
        self.sync_sprite_capture_stage();
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

fn duration_ns_u32(value: Duration) -> u32 {
    u32::try_from(value.as_nanos()).unwrap_or(u32::MAX)
}

fn retain_sample(queue: &mut VecDeque<FrameSample>, dropped: &mut u64, sample: FrameSample) {
    if queue.len() == COMPLETED_REPORTS {
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
