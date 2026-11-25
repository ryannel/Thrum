//! Seam for engine modules that simulate and draw inside the prepared scene.
//!
//! A contribution (for example cooked meshes and volumes from `thrum-asset-gpu`) never owns a
//! device, queue, submission, lighting mode or frame clock. It records into the scene's own frame
//! graph and may declare tick passes that answer the game independently of a view:
//!
//! 1. [`SceneContribution::rebuild`] whenever a scene is prepared or replaced (visual
//!    republication or device reconstruction), with a read-only [`SceneSurface`]. Generation-bound
//!    resources restart on a new generation. Effects retain owned state on a same-generation
//!    rebuild unless their descriptors changed.
//! 2. `contribution upload`, a graph Copy pass before forward, calls
//!    [`SceneContribution::encode_upload`] for bounded staged copies in the frame's encoder.
//! 3. `contribution compute`, a timed graph Compute pass after the upload and before forward,
//!    calls [`SceneContribution::encode_compute`] (for example GPU culling).
//! 4. The forward pass calls [`SceneContribution::draw`] after the scene's own geometry, with the
//!    scene's view (group 0), light (group 1) and instance (group 3) bind groups still set.
//! 5. Declared [`AfterOpaquePass`] callbacks draw after sky and stars, before temporal inputs
//!    and tone. Depth and the shared refraction source are independent requests.
//! 6. [`SceneContribution::submitted`] after the frame's encoder is submitted.
//! 7. Declared [`TickPass`] callbacks record separately, before any view of the advance.
//!    They read only committed frame state and write only their own query resources.
//!    [`SceneContribution::tick_submitted`] starts publication after the host submits.
//!
//! **Recording is not publication.** A frame may be encoded and then discarded (no drawable, a
//! timing or encode failure). Everything recorded in steps 2–5 is tentative: a contribution must
//! commit upload progress, residency, history validity and readback state only in
//! [`SceneContribution::submitted`], and restart from its last committed state at the next
//! upload. Hosts call `submitted` exactly once per submitted encoder and never for a discarded one.
//!
//! Lighting has one authority. A contribution that shades opaque surfaces compiles the scene's own
//! shader source with its own vertex entry appended and uses the scene's fragment entry, so it
//! receives exactly the scene's Sun, sky, local lights, shadows, reflections and aerial perspective.
//! The source and layouts are an internal, versioned module seam, not a stable WGSL ABI.
//!
//! Opaque casters can contribute bounds and draws to the scene’s existing Sun cascades.
//!
//! [`SceneContributions`] composes members in game-selected order without changing the legacy
//! upload, compute, shadow or opaque callbacks. An effect additionally returns an [`EffectGraph`]
//! of real resource descriptors and named simulate passes. Resource owner identities are local to
//! each member. The graph validates the declarations against the prepared generation's capability
//! record; physical resources and both sides' bind groups are created by the member in `rebuild`.
//! Each named simulate pass calls [`SceneContribution::encode_effect`] before the Sun and opaque
//! passes, under its own executor timing name. A runtime skip creates no graph or pipeline.
//!
//! [`EffectState`] implements the tentative protocol for an effect's clock, anchor and independent
//! history sides. Start each upload with `begin`; a reset is a full write of every producer and
//! time-sliced output. Mark a pair `produced` only after recording its whole-output producer.
//! Consumers resolve [`HistoryBinding::Result`] after the producer, including on skipped frames.
//! Call `submitted` only from the contribution's submission callback. A discarded encoder must
//! leave this bookkeeping alone; the next `begin` replaces its tentative recording. Captures use
//! [`EffectTime::capture`] and run no state producer. Physical ping-pong addressing remains the
//! effect's shader responsibility; the diagnostic uses a world-addressed periodic integer field.

mod after_opaque;
mod graph;
mod list;
mod quality;
mod state;
mod views;
pub use after_opaque::{AfterOpaqueKind, AfterOpaquePass};
pub(crate) use graph::EffectResources;
pub use graph::{
    EffectAccess, EffectCadence, EffectGraph, EffectPass, SunPathResources, TickPass,
    ViewVolumeResources,
};
pub use list::SceneContributions;
pub use quality::{QualityError, QualityInvalidation, QualityKnob, QualitySettings};
pub use state::{EffectState, EffectStep, EffectTime, HistoryBinding};
pub(crate) use views::ViewRequestState;
pub use views::{
    SecondarySceneSurface, SecondaryView, SecondaryViewError, SecondaryViewFrame,
    SecondaryViewRecord, SecondaryViewRequest, SecondaryViewSlot, ViewAir, ViewClipPlane,
};

/// Worker-side binding of replacement secondary view attachments, in slot order.
#[cfg(not(target_arch = "wasm32"))]
pub type SecondaryResizeJob =
    Box<dyn FnOnce(&[SecondaryView]) -> Result<SecondaryResize, crate::runtime::FrameError> + Send>;

#[cfg(not(target_arch = "wasm32"))]
type PublishSecondary = Box<dyn FnOnce(&mut dyn std::any::Any) + Send>;

/// Prepared contribution bindings and the graph they will record into.
#[cfg(not(target_arch = "wasm32"))]
pub struct SecondaryResize {
    pub(crate) graph: EffectGraph,
    publish: PublishSecondary,
}

#[cfg(not(target_arch = "wasm32"))]
impl SecondaryResize {
    /// Pair the replacement graph with the pointer swaps that install its
    /// bindings in the contribution. Publication must not create GPU resources.
    ///
    /// # Panics
    /// Publication panics if the job returned a replacement for a different
    /// contribution type than the owner that created the job.
    pub fn new<T: SceneContribution>(
        graph: EffectGraph,
        publish: impl FnOnce(&mut T) + Send + 'static,
    ) -> Self {
        Self {
            graph,
            publish: Box::new(move |owner| {
                publish(owner.downcast_mut::<T>().expect("secondary view owner"));
            }),
        }
    }

    pub(crate) fn publish(self, owner: &mut dyn SceneContribution) -> EffectGraph {
        (self.publish)(owner);
        self.graph
    }
}

/// Prepared 3D contribution resources and the declarations they will install.
#[cfg(not(target_arch = "wasm32"))]
pub struct ContributionCandidate {
    pub(crate) passes: ContributionPasses,
    pub(crate) graph: EffectGraph,
    commit: PublishSecondary,
    pub(crate) initialization: Vec<wgpu::CommandBuffer>,
}

#[cfg(not(target_arch = "wasm32"))]
impl ContributionCandidate {
    /// Stage declarations and an infallible installation on the member that prepared them.
    /// The installation must not create GPU resources.
    ///
    /// # Panics
    /// Installation panics if given a member of a different type.
    pub fn new<T: SceneContribution>(
        passes: ContributionPasses,
        graph: EffectGraph,
        commit: impl FnOnce(&mut T) + Send + 'static,
    ) -> Self {
        Self {
            passes,
            graph,
            initialization: Vec::new(),
            commit: Box::new(move |owner| {
                commit(
                    owner
                        .downcast_mut::<T>()
                        .expect("contribution candidate owner"),
                );
            }),
        }
    }

    /// Add one-time initialization that writes only this candidate's own resources.
    /// The host submits this work inside the preparation attempt's GPU error scopes.
    #[must_use]
    pub fn with_initialization(mut self, commands: Vec<wgpu::CommandBuffer>) -> Self {
        self.initialization.extend(commands);
        self
    }

    /// Forward a wrapped member's candidate through an adapter.
    #[must_use]
    pub fn map_owner<T: SceneContribution, M: SceneContribution>(
        self,
        member: impl FnOnce(&mut T) -> &mut M + Send + 'static,
    ) -> Self {
        let Self {
            passes,
            graph,
            commit,
            initialization,
        } = self;
        Self::new(passes, graph, move |owner| commit(member(owner)))
            .with_initialization(initialization)
    }

    pub(crate) fn commit(self, owner: &mut dyn SceneContribution) {
        (self.commit)(owner);
    }
}

/// The scene forward shader's material hook point: the fragment's one read of its material
/// record, inside the scene's `shade(input: VertexOutput, ..)` body before any texture sample.
/// A wrapper `fn(MaterialRecord, VertexOutput) -> MaterialRecord` may replace it. The renderer
/// keeps these names stable for it: `MaterialRecord.base`, `.emissive_cutoff` and `.surface`
/// (`vec4<f32>`; roughness in `surface.y`, metalness in `surface.z`) and `VertexOutput.uv`,
/// `.world_position` and `.normal`. Every composed forward variant holds it exactly once.
const MATERIAL_READ: &str = "materials[input.material]";
const MATERIAL_HOOK_NAME: &str = "scene material read";

/// Why [`material_hook_point`] found no single hook point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HookPointError {
    /// The source has no material hook point.
    Missing,
    /// The source has this many material hook points.
    Duplicate(usize),
}

impl std::fmt::Display for HookPointError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing => write!(
                f,
                "the scene shader has no `{MATERIAL_HOOK_NAME}` hook point"
            ),
            Self::Duplicate(count) => write!(
                f,
                "the scene shader has {count} `{MATERIAL_HOOK_NAME}` hook points where exactly one is required"
            ),
        }
    }
}

impl std::error::Error for HookPointError {}

/// The byte range of the material hook point in `source`, normally
/// [`SceneSurface::shader_source`]: the expression a material wrapper replaces.
///
/// # Errors
///
/// [`HookPointError`] when the source has no hook point or more than one.
pub fn material_hook_point(source: &str) -> Result<std::ops::Range<usize>, HookPointError> {
    let mut found = source.match_indices(MATERIAL_READ).map(|(at, _)| at);
    match (found.next(), found.count()) {
        (None, _) => Err(HookPointError::Missing),
        (Some(at), 0) => Ok(at..at + MATERIAL_READ.len()),
        (Some(_), more) => Err(HookPointError::Duplicate(more + 1)),
    }
}

/// Read-only description of a prepared scene's forward pass.
#[derive(Clone)]
pub struct SceneSurface<'a> {
    /// Whether the prepared scene uses ray queries.
    pub ray_demand: bool,
    /// Optional mesh-build observation for a contended diagnostic capture.
    #[cfg(all(feature = "creation-counters", target_os = "macos"))]
    pub ray_build_probe: Option<crate::synthetic::RayBuildProbe>,
    /// Prepared pipeline surfaces for all slots, lent only to a contribution
    /// that opts in through [`SceneContribution::secondary_views_enabled`].
    pub secondary_surfaces: Vec<SecondarySceneSurface<'a>>,
    /// Only this contribution's declared slots, in declaration order. Clone a handle
    /// in `rebuild`, ask it in `encode_upload`, and bind its textures and record in
    /// the same frame's own passes. A handle from a replaced surface is obsolete.
    pub secondary_views: &'a [SecondaryView],
    /// The prepared light group and shader include the shared `cloud_factor`
    /// operation. Volume contributions apply it once per lit march sample.
    pub sun_optical_depth: bool,
    pub(crate) cloud: Option<&'a crate::atmosphere::CloudPrepared>,
    pub(crate) environment: Option<&'a crate::atmosphere::AtmospherePrepared>,
    pub(crate) directional: &'a wgpu::Buffer,
    pub(crate) environment_light: &'a wgpu::Buffer,
    pub(crate) exposure: &'a [wgpu::Buffer; 2],
    /// The device every scene resource belongs to.
    pub device: &'a crate::runtime::DeviceSite,
    /// That device's queue. Contributions may create resources; they never submit.
    pub queue: &'a wgpu::Queue,
    /// Device generation; resources from another generation must not be used.
    pub generation: u64,
    /// Device features and limits of this generation.
    pub features: wgpu::Features,
    /// Adapter execution flags; unknown on an adapter-less record.
    pub downlevel: Option<wgpu::DownlevelFlags>,
    /// Device limits of this generation.
    pub limits: wgpu::Limits,
    /// The generation's resolved recipe.
    pub recipe: crate::capabilities::Recipe,
    /// The composed WGSL of the scene's forward shader (lighting, materials and entry points).
    /// It holds [`material_hook_point`] exactly once.
    pub shader_source: &'a str,
    /// The forward pipeline layout: view, light, material and instance groups.
    pub pipeline_layout: &'a wgpu::PipelineLayout,
    /// Group 0 (view) layout.
    pub view_layout: &'a wgpu::BindGroupLayout,
    /// The main view's existing packed uniform at offset zero. Mesh-stage
    /// bindings apply the recording's `ContributionFrame::view_uniform_offset`.
    pub mesh_view_uniform: wgpu::BufferBinding<'a>,
    /// Group 1 (light) layout.
    pub light_layout: &'a wgpu::BindGroupLayout,
    /// Group 2 (material) layout: material records, four texture arrays and a sampler.
    pub material_layout: &'a wgpu::BindGroupLayout,
    /// The fragment entry selected for the prepared opaque surface.
    pub fragment_entry: &'static str,
    /// Colour attachments of the forward pass: scene radiance, motion and reactive.
    pub color_formats: [wgpu::TextureFormat; 3],
    /// Optional separate ambient receiver pass for opaque geometry.
    pub ambient_receiver: Option<AmbientReceiverSurface>,
    /// Reverse-Z depth attachment (compare `Greater`, cleared to zero).
    pub depth_format: wgpu::TextureFormat,
    /// The existing Sun depth pass, when shadows are enabled for the scene.
    pub shadow: Option<SceneShadowSurface<'a>>,
}

impl SceneSurface<'_> {
    /// Shared GPU Sun-path target declared by this scene's provider, with its
    /// [`crate::atmosphere::GpuSunMapping`]. Absent for a CPU-fed diagnostic or no provider. A
    /// contribution creates its bindings here at rebuild and declares the
    /// target's actual writes, opaque reads and resource indices in its effect graph.
    pub fn sun_path_target(&self) -> Option<(wgpu::Texture, wgpu::Buffer)> {
        self.cloud
            .and_then(crate::atmosphere::CloudPrepared::gpu_target)
    }
    /// Borrow the scene-allocated ground band target and its mapping uniform.
    /// Available only when the provider declares GPU-filled optical depth.
    pub fn ground_band_target(&self) -> Option<(wgpu::Texture, wgpu::Buffer)> {
        self.cloud?.band.as_ref()?.target()
    }

    /// Scene-owned storage and camera records for this frame's medium coefficients.
    /// A contribution fills this target before the view volume integrates it.
    pub fn view_volume_medium_target(&self) -> Option<crate::atmosphere::ViewVolumeMediumTarget> {
        self.environment
            .and_then(crate::atmosphere::AtmospherePrepared::medium_target)
    }

    /// Borrow the atmosphere's physical lighting operations for an independent
    /// contribution pipeline. Creates read-only bindings on request, sharing
    /// the existing products. Scenes with authored or diagnostic light return
    /// `None`. Rebuild the result when this surface is replaced.
    #[must_use]
    pub fn environment_products(&self) -> Option<crate::atmosphere::EnvironmentProducts> {
        self.environment.map(|environment| {
            environment.products(
                self.device,
                self.directional,
                self.environment_light,
                self.exposure,
                self.cloud.is_some(),
            )
        })
    }
}

/// The receiver pass replays opaque geometry against final forward depth.
#[derive(Clone, Copy)]
pub struct AmbientReceiverSurface {
    /// Fragment entry in the same shader source as the forward pass.
    pub fragment_entry: &'static str,
    /// Its only colour target stores a world position and validity.
    pub color_format: wgpu::TextureFormat,
    /// The forward depth format. Use Equal and disable depth writes.
    pub depth_format: wgpu::TextureFormat,
}

/// Pipeline inputs for the scene's depth-only Sun cascades.
#[derive(Clone, Copy)]
pub struct SceneShadowSurface<'a> {
    /// Group 0 contains the cascade's column-major world-to-clip matrix at binding 0.
    pub view_layout: &'a wgpu::BindGroupLayout,
    /// Reverse-Z depth format with the `Greater` comparison.
    pub depth_format: wgpu::TextureFormat,
}

/// Finite world-space bounds for the contribution's selected opaque casters.
#[derive(Clone, Copy)]
pub struct ShadowBounds {
    min: [f32; 3],
    max: [f32; 3],
}

impl ShadowBounds {
    /// Check a world-space axis-aligned box. Flat bounds are valid.
    #[must_use]
    pub fn new(min: [f32; 3], max: [f32; 3]) -> Option<Self> {
        (0..3)
            .all(|axis| min[axis].is_finite() && max[axis].is_finite() && min[axis] <= max[axis])
            .then_some(Self { min, max })
    }

    /// Smallest box containing both inputs.
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        Self {
            min: std::array::from_fn(|axis| self.min[axis].min(other.min[axis])),
            max: std::array::from_fn(|axis| self.max[axis].max(other.max[axis])),
        }
    }

    pub(crate) fn depth_interval(self, axis: [f32; 3]) -> (f32, f32) {
        let mut min = 0.0;
        let mut max = 0.0;
        for (i, weight) in axis.into_iter().enumerate() {
            let a = weight * self.min[i];
            let b = weight * self.max[i];
            min += a.min(b);
            max += a.max(b);
        }
        (min, max)
    }
}

/// The fitted light volume of one secondary Sun pass.
#[derive(Clone, Copy)]
pub struct SecondaryShadowCascade {
    planes: [glam::Vec4; 6],
}

impl SecondaryShadowCascade {
    pub(crate) fn new(matrix: glam::Mat4) -> Self {
        let [x, y, z, w] = std::array::from_fn(|row| matrix.row(row));
        Self {
            planes: [w + x, w - x, w + y, w - y, z, w - z],
        }
    }

    /// Conservatively test world bounds against the cascade's clip volume.
    /// Casters outside the camera frustum can still overlap this volume.
    #[must_use]
    pub fn overlaps(&self, bounds: ShadowBounds) -> bool {
        let magnitude = glam::Vec3::from_array(bounds.min)
            .abs()
            .max(glam::Vec3::from_array(bounds.max).abs());
        !self.planes.iter().any(|plane| {
            let (_, high) = bounds.depth_interval(plane.truncate().to_array());
            // Keep boxes at the clip boundary despite CPU/GPU rounding.
            let guard =
                16.0 * f32::EPSILON * (magnitude.dot(plane.truncate().abs()) + plane.w.abs() + 1.0);
            high + plane.w < -guard
        })
    }
}

/// Per-frame camera facts a contribution may need on the CPU (for example to cull).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContributionFrame {
    /// Source clock of this recording; captures are explicitly read-only.
    pub time: EffectTime,
    /// Resolved exposure side used by this frame's forward pass, also used to
    /// select [`crate::atmosphere::EnvironmentProducts::group`].
    pub exposure_side: usize,
    /// The atmosphere's sole sampled environment, absent for diagnostic and
    /// authored photometric lighting. Every callback receives the same sample.
    pub environment: Option<crate::atmosphere::EnvironmentFrame>,
    /// Column-major world-to-clip matrix of the frame being encoded. In 3D it
    /// uses jittered reverse-Z and the forward raster frustum, while motion uses
    /// the separately bound unjittered matrices. In 2D it is unjittered and maps
    /// world X and Y to the sprite view's output pixels.
    pub view_proj: [[f32; 4]; 4],
    /// Pixel extent rasterized into the top-left of the scene attachments.
    pub render_extent: [u32; 2],
    /// Dynamic byte offset of the packed main view selected for this recording.
    pub view_uniform_offset: u32,
    /// Declared world-space camera position, without matrix inversion. In 2D
    /// this is the view centre in X and Y, with Z zero. In 3D a secondary camera
    /// must copy coordinates 0 and 2 exactly to stand on this camera's vertical.
    pub camera: [f32; 3],
    /// Device generation of the encoder.
    pub generation: u64,
    /// The scene restarts view history this frame (camera cut, replacement or device change):
    /// no geometry may claim valid motion history.
    pub history_reset: bool,
}

/// Graph passes a contribution needs, fixed for its lifetime.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ContributionPasses {
    /// Declare the `contribution upload` Copy pass.
    pub upload: bool,
    /// Declare the timed `contribution compute` pass.
    pub compute: bool,
}

/// A contribution's legacy drawing requirement, before its graph is prepared.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SceneDrawStage {
    /// Draw into the Sun's shadow cascades.
    Shadow,
    /// Draw into the forward opaque pass.
    Opaque,
}

/// Preparation context for an upload/simulate contribution in a 2D scene.
pub struct SpriteContributionSurface<'a> {
    /// Device site, capabilities, limits and generation for this scene.
    pub capabilities: &'a crate::capabilities::DeviceCapabilities,
    /// The scene's queue. Contributions may stage writes and never submit.
    pub queue: &'a wgpu::Queue,
}

/// An engine module drawing into the prepared scene. See the module documentation.
///
/// [`Self::passes`], [`Self::rebuild`] and [`Self::encode_upload`] are required. The other hooks
/// have default bodies that do nothing, so an implementation writes only the hooks it uses. Two
/// rules follow from those defaults:
///
/// - A wrapper that stands in front of other contributions, such as [`SceneContributions`], must
///   forward every hook, the defaulted ones included: a hook it leaves to its default never
///   reaches the contributions behind it, and nothing at compile time says so.
/// - [`Self::encode_compute`] is called only in a frame where [`Self::computes`] returned true,
///   and [`Self::submitted`] is where the work [`Self::encode_upload`] staged is committed.
pub trait SceneContribution: wgpu::WasmNotSend + std::any::Any {
    /// Whether this member can prepare independently of its accepted state.
    #[cfg(not(target_arch = "wasm32"))]
    fn supports_candidate_preparation(&self) -> bool {
        false
    }

    /// Create a traced or Maps candidate using the declared secondary view slots.
    /// Until installation, writes must be confined to resources owned by the candidate.
    ///
    /// # Errors
    /// Returns a preparation fault or the default refusal for a legacy member.
    #[cfg(not(target_arch = "wasm32"))]
    fn prepare_candidate(
        &self,
        _surface: &SceneSurface<'_>,
    ) -> Result<ContributionCandidate, crate::runtime::FrameError> {
        Err(crate::runtime::FrameError::InvalidState(
            "contribution does not support candidate preparation",
        ))
    }

    /// Legacy drawing stages this contribution needs. Pure upload/simulate
    /// contributions declare an empty list and implement [Self::rebuild_2d].
    fn draw_stages(&self) -> Vec<SceneDrawStage> {
        vec![SceneDrawStage::Opaque]
    }

    /// Name a configured input that the contribution must refuse in a 2D scene.
    fn sprite_stage_refusal(&self) -> Option<String> {
        None
    }

    /// Prepare generation-bound upload/simulate resources for a sprite scene.
    /// # Errors
    /// Refuses a contribution that cannot prepare with the supplied capabilities.
    fn rebuild_2d(
        &mut self,
        _surface: &SpriteContributionSurface<'_>,
    ) -> Result<(), crate::runtime::FrameError> {
        Err(crate::runtime::FrameError::Render(
            crate::error::RenderError::UnsupportedSpriteContribution,
        ))
    }

    /// Offer every opaque placement this contribution holds for this recording,
    /// including off-camera placements and geometry outside all shadow passes.
    /// Called only with scene ray demand, after staged uploads and before the
    /// scene structure is built. Offer the same mesh and transform the frame
    /// draws, including a tentative publication switch, in stable placement order.
    /// The mesh must belong to this device generation and have a build recorded
    /// by this point. The default offers no instances.
    fn ray_instances(&self, _instances: &mut RayInstances<'_>) {}

    /// Opt opaque geometry into secondary views. The default receives no
    /// pipeline surfaces and records no extra upload, caster or opaque draw.
    fn secondary_views_enabled(&self) -> bool {
        false
    }
    /// Camera inputs after all ordinary uploads have asked for their slots.
    /// Stage per-view culling here; run it in the existing contribution compute
    /// callback. The list is empty when every slot is idle.
    fn encode_secondary_upload(
        &mut self,
        _encoder: &mut wgpu::CommandEncoder,
        _frame: &ContributionFrame,
        _views: &[SecondaryViewFrame],
    ) {
    }
    /// Draw opaque geometry with the view's groups 0 and 1 already bound.
    /// Build pipelines from the corresponding prepared secondary surface.
    fn draw_secondary(&self, _index: usize, _pass: &mut wgpu::RenderPass<'_>) {}
    /// Draw this view's Sun casters with its caster view at group 0 and the
    /// scene's instance/history group 1 bound. Only opted-in producers run.
    fn draw_secondary_shadow(
        &self,
        _index: usize,
        _cascade: SecondaryShadowCascade,
        _pass: &mut wgpu::RenderPass<'_>,
    ) {
    }
    /// Fixed slot declarations for this contribution. These are admitted and
    /// prepared before `rebuild`; their handles are on `SceneSurface`.
    /// Returning no slots retains the original shaders, resources and graph.
    fn secondary_view_slots(&self) -> Vec<SecondaryViewSlot> {
        Vec::new()
    }

    /// Snapshot the inputs for rebinding owned secondary views on a preparation
    /// thread. Only slot owners need a job. The old bindings stay live until
    /// the host publishes the candidate on the render owner.
    /// # Errors
    /// Refuses a contribution that owns slots but cannot prepare replacements.
    #[cfg(not(target_arch = "wasm32"))]
    fn secondary_resize_job(
        &self,
    ) -> Result<Option<SecondaryResizeJob>, crate::runtime::FrameError> {
        if self.secondary_view_slots().is_empty() {
            Ok(None)
        } else {
            Err(crate::runtime::FrameError::InvalidState(
                "contribution has no detached secondary view preparation",
            ))
        }
    }
    /// Whether a declared late compute pass runs, asked after upload. Render composition
    /// always runs because it loads the scene attachments.
    fn after_opaque_runs(&self, _index: usize, _frame: &ContributionFrame) -> bool {
        true
    }
    /// Record a declared late compute pass. The renderer binds the view at group 0 and
    /// requested scene inputs at group 1. Owned outputs and history sides are prebound by
    /// the effect; mark a history produced only after recording its whole-output writer.
    fn encode_after_opaque(
        &mut self,
        _index: usize,
        _pass: &mut wgpu::ComputePass<'_>,
        _frame: &ContributionFrame,
    ) {
    }
    /// Runtime controls available to a caller without effect-specific knowledge.
    fn quality_knobs(&self) -> Vec<QualityKnob> {
        Vec::new()
    }
    /// Stage a named control for the next frame boundary.
    /// # Errors
    /// Refuses unknown names and values outside the declared range.
    fn set_quality_knob(&mut self, name: &str, _value: u32) -> Result<(), QualityError> {
        Err(QualityError::UnknownKnob { name: name.into() })
    }
    /// Apply staged values before frame preparation. Effects reset their own state here.
    /// A view-history result also asks the host to invalidate reconstruction.
    fn commit_quality_knobs(&mut self) -> QualityInvalidation {
        QualityInvalidation::None
    }
    /// Draw a declared late pass. The renderer binds the view at group 0 and requested
    /// inputs at group 1. Write scene-linear colour, unjittered pixel motion (previous
    /// minus current), and reactive coverage to the three scene targets. Invalid motion
    /// is `(65504, 65504)`; honour `frame.history_reset` and the effect's own reset.
    fn draw_after_opaque(
        &self,
        _index: usize,
        _pass: &mut wgpu::RenderPass<'_>,
        _frame: &ContributionFrame,
    ) {
    }
    /// Real resources and named simulate passes, prepared before the first frame.
    fn effect_graph(&self) -> EffectGraph {
        EffectGraph::default()
    }

    /// Whether this tick pass holds unexpired posts and has capacity to record them.
    fn tick_has_work(&self, _index: usize) -> bool {
        false
    }

    /// Stage the posts for this recording only. Leave them queued until submission.
    fn encode_tick_upload(
        &mut self,
        _index: usize,
        _encoder: &mut wgpu::CommandEncoder,
        _tick_duration: std::time::Duration,
    ) {
    }

    /// Record compute for the staged posts, reading only committed frame state.
    fn encode_tick(&mut self, _index: usize, _pass: &mut wgpu::ComputePass<'_>) {}

    /// Record the staged posts' result copies into the effect's own readbacks.
    fn encode_tick_readback(&mut self, _index: usize, _encoder: &mut wgpu::CommandEncoder) {}

    /// Commit the listed passes' posts and start their asynchronous maps.
    /// Rebuild must invalidate pending results from a replaced device generation.
    fn tick_submitted(&mut self, _passes: &[usize]) {}

    /// Whether a named simulate pass records work, asked after upload.
    fn effect_runs(&self, _index: usize, _frame: &ContributionFrame) -> bool {
        true
    }

    /// Record a named simulate pass. Pipelines and both history bindings already exist.
    fn encode_effect(
        &mut self,
        _index: usize,
        _pass: &mut wgpu::ComputePass<'_>,
        _frame: &ContributionFrame,
    ) {
    }

    /// Record copies of a named effect's results into its own readback buffers.
    /// Called only for a recorded pass, on the same tentative encoder.
    fn encode_effect_readback(&mut self, _index: usize, _encoder: &mut wgpu::CommandEncoder) {}

    /// The graph passes this contribution records into; constant for its lifetime.
    fn passes(&self) -> ContributionPasses;

    /// (Re)create generation-bound pipelines and resources for `surface`.
    /// Effects keep their epoch and state on a same-generation rebuild when their own descriptors
    /// are unchanged; device replacement starts a fresh epoch.
    ///
    /// # Errors
    /// A description of why this contribution cannot draw into the surface; the host then
    /// detaches it and leaves the scene unchanged.
    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), crate::runtime::FrameError>;

    /// Prepare resources during explicit host attachment, retaining a classified failure
    /// when the contribution can distinguish scene input from GPU preparation.
    /// # Errors
    /// Refuses a contribution that cannot draw into the current scene.
    fn rebuild_for_attachment(
        &mut self,
        surface: &SceneSurface<'_>,
    ) -> Result<(), crate::runtime::FrameError> {
        self.rebuild(surface)
    }

    /// Record tentative bounded copies in the `contribution upload` pass.
    fn encode_upload(&mut self, encoder: &mut wgpu::CommandEncoder, frame: &ContributionFrame);

    /// Whether [`Self::encode_compute`] records any dispatch this frame (asked after
    /// [`Self::encode_upload`]). When false the timed pass is not begun, so the frame reports no
    /// contribution time instead of an empty pass some backends leave without timestamps.
    /// The default records no dispatch.
    fn computes(&self, _frame: &ContributionFrame) -> bool {
        false
    }

    /// Record work in the timed `contribution compute` pass, after this frame's upload. Called
    /// only when [`Self::computes`] returned true this frame. The default records nothing.
    fn encode_compute(&mut self, _pass: &mut wgpu::ComputePass<'_>, _frame: &ContributionFrame) {}

    /// Record copies that must follow this frame's compute (for example a counter readback);
    /// still before forward and still tentative. The default records nothing.
    fn encode_after_compute(&mut self, _encoder: &mut wgpu::CommandEncoder) {}

    /// Draw at the end of the forward pass. Groups 0, 1 and 3 are the scene's. The default
    /// draws nothing.
    fn draw(&self, _pass: &mut wgpu::RenderPass<'_>) {}

    /// Replay the same opaque draws in the optional ambient receiver pass.
    /// Forward-only or translucent contributions may leave this empty.
    fn draw_ambient_receivers(&self, _pass: &mut wgpu::RenderPass<'_>) {}

    /// Bounds of the opaque casters selected by this frame's upload recording.
    /// The selection must stay fixed through both cascade draws and forward.
    fn shadow_bounds(&self) -> Option<ShadowBounds> {
        None
    }

    /// Append the selected casters to a cascade pass with its view bound at group 0.
    fn draw_shadow(&self, _pass: &mut wgpu::RenderPass<'_>) {}

    /// The encoder holding this frame's recording was submitted: commit it. `history_advanced`
    /// is false for a capture, which renders without making itself the scene's previous frame.
    /// This is where a contribution commits what [`Self::encode_upload`] staged; the default
    /// has nothing to commit.
    fn submitted(&mut self, _history_advanced: bool) {}
}

/// Receives opaque placements for the current scene structure.
pub struct RayInstances<'a> {
    pub(crate) entries: &'a mut Vec<Option<wgpu::TlasInstance>>,
    pub(crate) next_identity: u32,
    pub(crate) limit: usize,
    pub(crate) overflowed: bool,
}

impl RayInstances<'_> {
    /// Offer one placement of `mesh` using its row-major affine world transform.
    /// Bit 0 enables shadow casting and bit 1 enables ambient light occlusion.
    /// Set both for ordinary opaque geometry, regardless of enabled light effects.
    pub fn offer(&mut self, mesh: &wgpu::Blas, transform: [f32; 12], mask: u8) {
        if self.entries.len() == self.limit {
            self.overflowed = true;
            return;
        }
        self.entries.push(Some(wgpu::TlasInstance::new(
            mesh,
            transform,
            self.next_identity,
            mask & 0b11,
        )));
        self.next_identity += 1;
    }
}

/// Pieces of a prepared scene that a [`SceneSurface`] lends out.
pub(crate) struct SurfaceParts {
    pub(crate) shader_source: String,
    pub(crate) pipeline_layout: wgpu::PipelineLayout,
    pub(crate) light_layout: wgpu::BindGroupLayout,
    pub(crate) material_layout: wgpu::BindGroupLayout,
    pub(crate) fragment_entry: &'static str,
}
