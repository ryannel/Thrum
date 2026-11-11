//! Seam for engine modules that draw into the prepared scene's forward pass.
//!
//! A contribution (for example cooked meshes and volumes from `thrum-asset-gpu`) never owns a
//! device, queue, submission, lighting mode or frame clock. It runs inside the scene's own frame
//! graph, in passes the graph declares, orders and times like any other:
//!
//! 1. [`SceneContribution::rebuild`] whenever a scene is prepared or replaced (visual
//!    republication or device reconstruction), with a read-only [`SceneSurface`]. Generation-bound
//!    resources and history restart here.
//! 2. `contribution upload`, a graph Copy pass before forward, calls
//!    [`SceneContribution::encode_upload`] for bounded staged copies in the frame's encoder.
//! 3. `contribution compute`, a timed graph Compute pass after the upload and before forward,
//!    calls [`SceneContribution::encode_compute`] (for example GPU culling).
//! 4. The forward pass calls [`SceneContribution::draw`] after the scene's own geometry, with the
//!    scene's view (group 0), light (group 1) and instance (group 3) bind groups still set.
//! 5. [`SceneContribution::submitted`] after the frame's encoder is submitted.
//!
//! **Recording is not publication.** A frame may be encoded and then discarded (no drawable, a
//! timing or encode failure). Everything recorded in steps 2–4 is tentative: a contribution must
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
pub struct SceneSurface<'a> {
    /// The prepared light group and shader include the shared `cloud_factor`
    /// operation. Volume contributions apply it once per lit march sample.
    pub sun_optical_depth: bool,
    pub(crate) environment: Option<&'a crate::atmosphere::AtmospherePrepared>,
    pub(crate) directional: &'a wgpu::Buffer,
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
    /// The composed WGSL of the scene's forward shader (lighting, materials and entry points).
    /// It holds [`material_hook_point`] exactly once.
    pub shader_source: &'a str,
    /// The forward pipeline layout: view, light, material and instance groups.
    pub pipeline_layout: &'a wgpu::PipelineLayout,
    /// Group 0 (view) layout.
    pub view_layout: &'a wgpu::BindGroupLayout,
    /// Group 1 (light) layout.
    pub light_layout: &'a wgpu::BindGroupLayout,
    /// Group 2 (material) layout: material records, four texture arrays and a sampler.
    pub material_layout: &'a wgpu::BindGroupLayout,
    /// The fragment entry the scene uses (`fragment` or `shadowed_fragment`).
    pub fragment_entry: &'static str,
    /// Colour attachments of the forward pass: scene radiance and motion.
    pub color_formats: [wgpu::TextureFormat; 2],
    /// Reverse-Z depth attachment (compare `Greater`, cleared to zero).
    pub depth_format: wgpu::TextureFormat,
    /// The existing Sun depth pass, when shadows are enabled for the scene.
    pub shadow: Option<SceneShadowSurface<'a>>,
}

impl SceneSurface<'_> {
    /// Borrow the atmosphere's physical lighting operations for an independent
    /// contribution pipeline. Creates read-only bindings on request, sharing
    /// the existing products. Scenes with authored or diagnostic light return
    /// `None`. Rebuild the result when this surface is replaced.
    #[must_use]
    pub fn environment_products(&self) -> Option<crate::atmosphere::EnvironmentProducts> {
        self.environment
            .map(|environment| environment.products(self.device, self.directional, self.exposure))
    }
}

/// Pipeline inputs for the scene's depth-only Sun cascades.
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

/// Per-frame camera facts a contribution may need on the CPU (for example to cull).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContributionFrame {
    /// Resolved exposure side used by this frame's forward pass, also used to
    /// select [`crate::atmosphere::EnvironmentProducts::group`].
    pub exposure_side: usize,
    /// The atmosphere's sole sampled environment, absent for diagnostic and
    /// authored photometric lighting. Every callback receives the same sample.
    pub environment: Option<crate::atmosphere::EnvironmentFrame>,
    /// Column-major jittered reverse-Z view-projection of the frame being encoded.
    /// Culling uses the same raster frustum as the forward vertices; motion uses
    /// the separately bound unjittered matrices.
    pub view_proj: [[f32; 4]; 4],
    /// World-space camera position.
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

/// An engine module drawing into the prepared scene. See the module documentation.
pub trait SceneContribution: wgpu::WasmNotSend {
    /// The graph passes this contribution records into; constant for its lifetime.
    fn passes(&self) -> ContributionPasses;

    /// (Re)create generation-bound pipelines and resources for `surface`, and restart history.
    ///
    /// # Errors
    /// A description of why this contribution cannot draw into the surface; the host then
    /// detaches it and leaves the scene unchanged.
    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), String>;

    /// Record tentative bounded copies in the `contribution upload` pass.
    fn encode_upload(&mut self, encoder: &mut wgpu::CommandEncoder, frame: &ContributionFrame);

    /// Whether [`Self::encode_compute`] records any dispatch this frame (asked after
    /// [`Self::encode_upload`]). When false the timed pass is not begun, so the frame reports no
    /// contribution time instead of an empty pass some backends leave without timestamps.
    fn computes(&self, frame: &ContributionFrame) -> bool;

    /// Record work in the timed `contribution compute` pass, after this frame's upload.
    fn encode_compute(&mut self, pass: &mut wgpu::ComputePass<'_>, frame: &ContributionFrame);

    /// Record copies that must follow this frame's compute (for example a counter readback);
    /// still before forward and still tentative.
    fn encode_after_compute(&mut self, encoder: &mut wgpu::CommandEncoder);

    /// Draw at the end of the forward pass. Groups 0, 1 and 3 are the scene's.
    fn draw(&self, pass: &mut wgpu::RenderPass<'_>);

    /// Bounds of the opaque casters selected by this frame's upload recording.
    /// The selection must stay fixed through both cascade draws and forward.
    fn shadow_bounds(&self) -> Option<ShadowBounds> {
        None
    }

    /// Append the selected casters to a cascade pass with its view bound at group 0.
    fn draw_shadow(&self, _pass: &mut wgpu::RenderPass<'_>) {}

    /// The encoder holding this frame's recording was submitted: commit it. `history_advanced`
    /// is false for a capture, which renders without making itself the scene's previous frame.
    fn submitted(&mut self, history_advanced: bool);
}

/// Pieces of a prepared scene that a [`SceneSurface`] lends out.
pub(crate) struct SurfaceParts {
    pub(crate) shader_source: String,
    pub(crate) pipeline_layout: wgpu::PipelineLayout,
    pub(crate) light_layout: wgpu::BindGroupLayout,
    pub(crate) material_layout: wgpu::BindGroupLayout,
    pub(crate) fragment_entry: &'static str,
}
