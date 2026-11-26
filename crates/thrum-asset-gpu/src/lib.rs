//! GPU residency for cooked G5 products, drawn inside the prepared scene.
//!
//! [`CookedAssets`] is a [`SceneContribution`]: a loader thread follows the store's `HEAD`, maps
//! and validates configured products, creates their destinations and fills a bounded staging
//! ring. The frame host records copies under one per-frame byte budget in its own encoder.
//! With placements alone it swaps the set in the frame that records its last copy. Configured
//! [`ProductLease`] readers defer that shared switch until the next frame after those copies
//! are submitted, independent of contribution order. The host culls meshlets and draws meshes with the scene's own fragment
//! shader (so they receive the scene's selected lighting) and the volume as a lit raymarch.
//!
//! Content identity survives device reconstruction: on a new device generation the loader is
//! restarted against the same store and products are mapped and staged again.
//!
//! A game may replace the meshes' material factors with one WGSL hook, reloaded explicitly through
//! [`CookedAssets::material`]; see [`MaterialReload`]. Without a hook nothing changes.
//!
//! A [`CookedSource::Remote`] feed of generations fetched by `thrum_asset::DevClient` (the
//! browser has no store to map and no loader thread) is prepared on the render thread: when the
//! contribution starts, and at the next upload after each [`RemoteFeed::publish`]. Products the
//! drawn set already holds are reused; the rest go through the same staging and swap.

// WebGPU handles are not Send on wasm32, where these Arcs never cross a thread: the browser
// prepares on its one thread. The same code shares them with the native loader thread.
#![cfg_attr(
    target_arch = "wasm32",
    allow(
        clippy::arc_with_non_send_sync,
        reason = "WebGPU handles are not Send; the browser prepares on one thread"
    )
)]

mod draw;
#[cfg(not(target_arch = "wasm32"))]
mod drawn;
#[cfg(feature = "fault-probe")]
mod fault;
mod lease;
#[cfg(feature = "fault-probe")]
pub use fault::FaultProbe;
mod load;
mod material;
mod mesh_stages;
pub mod mesh_upload;
mod placement;
mod ray_mesh;
pub mod records;
pub mod rows;
mod staging;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

use thrum_asset::{ContentHash, Ktx2Format, Profile, RemoteGeneration};
use thrum_render::contribution::{
    ContributionFrame, ContributionPasses, SceneContribution, SceneSurface, SecondaryShadowCascade,
    ShadowBounds,
};

#[cfg(not(target_arch = "wasm32"))]
pub use drawn::{
    AwaitDrawnError, DRAWN_DEADLINE, Drawn, Progress, await_content, await_drawn,
    await_drawn_on_surface, cooked_failure_baseline, cooked_progress, draw_endpoint,
};
pub use lease::{FontCandidate, FontScale, LeaseRequest, LentProduct, LentSet, ProductLease};
pub use material::{
    MaterialDiagnostic, MaterialHook, MaterialOutcome, MaterialReadError, MaterialReload,
    ReloadTiming,
};
pub use placement::{MeshPlacement, PlacementError, PlacementFault};
pub use staging::{COPY_UNIT, STAGING_CHUNKS};

/// ABI revision 2 metallic-roughness factors of one mesh placement. The default is a matte
/// white dielectric: the cooked texture's own colour, roughness 0.55, metalness 0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CookedMaterial {
    /// Linear base-colour factor (multiplies the cooked texture) and alpha, each 0–1.
    pub base_colour: [f32; 4],
    /// Perceptual roughness, 0 (mirror) to 1 (chalk).
    pub roughness: f32,
    /// Metalness, 0 (stone, wood, paint) to 1 (bare metal).
    pub metalness: f32,
}

impl Default for CookedMaterial {
    fn default() -> Self {
        Self {
            base_colour: [1.0; 4],
            roughness: 0.55,
            metalness: 0.0,
        }
    }
}

/// A cooked density volume filling the unit cube of `transform`.
#[derive(Clone, Debug)]
pub struct VolumePlacement {
    /// Manifest name of the volume product.
    pub name: String,
    /// Column-major object-to-world transform of the unit cube.
    pub transform: [[f32; 4]; 4],
    /// Extinction per unit density per unit of object-space length.
    pub extinction: f32,
    /// View-ray samples.
    pub steps: u32,
    /// Samples toward the Sun per view sample.
    pub light_steps: u32,
}

/// Where cooked products come from.
#[derive(Clone, Debug)]
pub enum CookedSource {
    /// Store root written by the cooker, followed by a loader thread.
    Store(PathBuf),
    /// Generations already fetched from a dev server, prepared without a thread.
    Remote(RemoteFeed),
}

/// Newest fetched generation for a [`CookedSource::Remote`]; clones share it.
#[derive(Clone, Debug)]
pub struct RemoteFeed(Arc<Mutex<(u64, Arc<RemoteGeneration>)>>);

impl RemoteFeed {
    /// Newest verified generation, for initial scene row preparation.
    pub fn generation(&self) -> Option<Arc<RemoteGeneration>> {
        self.latest().map(|(_, generation)| generation)
    }
    /// A feed starting at `generation`.
    pub fn new(generation: RemoteGeneration) -> Self {
        Self(Arc::new(Mutex::new((0, Arc::new(generation)))))
    }

    /// Replace the newest generation; the contribution stages it at its next upload.
    pub fn publish(&self, generation: RemoteGeneration) {
        if let Ok(mut latest) = self.0.lock() {
            *latest = (latest.0 + 1, Arc::new(generation));
        }
    }

    fn latest(&self) -> Option<(u64, Arc<RemoteGeneration>)> {
        self.0
            .lock()
            .ok()
            .map(|latest| (latest.0, Arc::clone(&latest.1)))
    }
}

/// Render-thread preparation state of a remote feed.
struct Remote {
    loader: load::Loader,
    feed: RemoteFeed,
    /// Feed serial and drawn generation last considered for preparation.
    seen: Option<(u64, Option<thrum_asset::Head>, u32, u32)>,
    /// Products of the drawn set, reused by later generations.
    confirmed: load::Confirmed,
    /// Staging of the newest prepared generation, filled as chunks free up.
    filler: Option<staging::Filler>,
}

impl Remote {
    fn prepare_newest(&mut self, mailbox: &load::Mailbox, stop: &AtomicBool) {
        let scale = self.loader.config.font_scale();
        if let Some((serial, generation)) = self.feed.latest()
            && Some((
                serial,
                self.confirmed.head,
                self.confirmed.font_scale,
                scale,
            )) != self.seen
        {
            self.seen = Some((
                serial,
                self.confirmed.head,
                self.confirmed.font_scale,
                scale,
            ));
            if Some(generation.head()) != self.confirmed.head || self.confirmed.font_scale != scale
            {
                self.filler =
                    self.loader
                        .prepare_remote(&generation, scale, &self.confirmed, mailbox, stop);
            }
        }
        // Keep the browser's sprite loop moving while a large sheet is staged.
        // The legacy 3D path still fills every free chunk in one call.
        let limit = if self.loader.config.sheet_leases.is_empty()
            && self.loader.config.font_leases.is_empty()
        {
            usize::MAX
        } else {
            1
        };
        let progress = self
            .filler
            .as_mut()
            .map(|filler| filler.fill_available(limit));
        match progress {
            None | Some(staging::Progress::Waiting) => {}
            Some(staging::Progress::Failed(reason)) => {
                self.loader.fail(stop, &reason);
                self.filler = None;
            }
            Some(staging::Progress::Done | staging::Progress::Stopped) => self.filler = None,
        }
    }
}

impl From<PathBuf> for CookedSource {
    fn from(store: PathBuf) -> Self {
        Self::Store(store)
    }
}

/// What to load and how fast to upload it. [`CookedConfig::new`] fills in the defaults.
#[derive(Clone, Debug)]
pub struct CookedConfig {
    /// Forward path requested for the main view.
    pub draw_path: CookedDrawPath,
    /// Diagnostic switch to draw every meshlet through the mesh stages.
    pub mesh_stage_cull: bool,
    /// Force validation failure during mesh pipeline preparation in a diagnostic build.
    #[cfg(feature = "draw-probe")]
    pub fail_mesh_pipeline: bool,
    /// Optional diagnostic counter buffer written by the object stage.
    #[cfg(feature = "draw-probe")]
    pub mesh_stage_rejections: Option<Arc<Mutex<Option<wgpu::Buffer>>>>,
    /// Optional aggregate object-stage counters for the Sun's maps.
    #[cfg(feature = "draw-probe")]
    pub mesh_stage_shadow_rejections: Option<Arc<Mutex<Option<wgpu::Buffer>>>>,
    /// Negative control for mesh-stage Sun caster parity.
    #[cfg(feature = "draw-probe")]
    pub mesh_stage_shadow_fault: Option<draw_probe::ShadowFault>,
    /// Entity mesh rows participating in the same publication as every other reader.
    pub rows: Option<rows::CookedRows>,
    /// Opt-in one-shot staging fill fault, absent from ordinary builds.
    #[cfg(feature = "fault-probe")]
    pub fault: Option<FaultProbe>,
    /// Additional sampled products, resident under the same publication and upload budget.
    pub leased: Vec<LeaseRequest>,
    sheet_leases: std::collections::BTreeSet<String>,
    font_leases: std::collections::BTreeSet<String>,
    font_scale: FontScale,
    /// Where the cooked products come from: a store folder written by `thrum-cook` (a
    /// `PathBuf` converts), or a fetched remote feed.
    pub source: CookedSource,
    /// Meshes to draw, each with its texture, transform and material.
    pub meshes: Vec<MeshPlacement>,
    /// A density volume to draw, if any. Default: none.
    pub volume: Option<VolumePlacement>,
    /// Bytes of staged copies recorded per frame, which spreads a large publication over
    /// several frames (at least one copy unit always progresses). Default:
    /// [`DEFAULT_UPLOAD_BUDGET`].
    pub upload_budget: u64,
    /// How often the loader thread checks the store's `HEAD` for a newly cooked generation.
    /// Default: 20 ms.
    pub poll_interval: Duration,
    /// Texture profiles to prefer, best first; formats the device cannot sample are skipped.
    /// Default: `None`, the device's best ([`texture_profiles`]).
    pub texture_preference: Option<Vec<Profile>>,
}

/// Default [`CookedConfig::upload_budget`]: 8 MiB of copies per frame, a small fraction of a
/// frame on current GPUs.
pub const DEFAULT_UPLOAD_BUDGET: u64 = 8 << 20;

impl CookedConfig {
    /// Draw `meshes` from `source`, with no volume and the default upload budget, poll
    /// interval and texture profiles.
    pub fn new(source: impl Into<CookedSource>, meshes: Vec<MeshPlacement>) -> Self {
        Self {
            draw_path: CookedDrawPath::Auto,
            mesh_stage_cull: true,
            #[cfg(feature = "draw-probe")]
            fail_mesh_pipeline: false,
            #[cfg(feature = "draw-probe")]
            mesh_stage_rejections: None,
            #[cfg(feature = "draw-probe")]
            mesh_stage_shadow_rejections: None,
            #[cfg(feature = "draw-probe")]
            mesh_stage_shadow_fault: None,
            rows: None,
            #[cfg(feature = "fault-probe")]
            fault: None,
            leased: Vec::new(),
            sheet_leases: std::collections::BTreeSet::new(),
            font_leases: std::collections::BTreeSet::new(),
            font_scale: FontScale::default(),
            source: source.into(),
            meshes,
            volume: None,
            upload_budget: DEFAULT_UPLOAD_BUDGET,
            poll_interval: Duration::from_millis(20),
            texture_preference: None,
        }
    }

    /// Request the page array and frame table of one cooked sprite sheet.
    pub fn lease_sheet(&mut self, name: impl Into<String>) {
        let name = name.into();
        if self.sheet_leases.insert(name.clone()) {
            self.leased.push(LeaseRequest {
                name,
                dimension: wgpu::TextureDimension::D2,
            });
        }
    }

    /// Lease the named font's pixel sheet at the interface's requested scale.
    pub fn lease_font(&mut self, name: impl Into<String>) {
        let name = name.into();
        if self.font_leases.insert(name.clone()) {
            self.leased.push(LeaseRequest {
                name,
                dimension: wgpu::TextureDimension::D2,
            });
        }
    }

    /// Shared scale request for this scene's font reader.
    pub fn font_scale_request(&self) -> FontScale {
        self.font_scale.clone()
    }

    fn font_scale(&self) -> u32 {
        if self.font_leases.is_empty() {
            0
        } else {
            self.font_scale.get()
        }
    }
}

/// One publication that became visible.
#[derive(Clone, Debug)]
pub struct SwapRecord {
    /// Every resident mesh, texture, volume and lease originated in a file mapping.
    /// Reused products retain the provenance observed when they were first staged.
    pub mapped: bool,
    /// Published generation.
    pub generation: u64,
    /// When the loader saw its `HEAD`.
    pub head_seen: Instant,
    /// When the loader posted it; its staging fills in bounded chunks from then on.
    pub staged: Instant,
    /// When the render thread recorded its first copy.
    pub first_copy: Instant,
    /// When the render thread recorded its last copy and swapped it in.
    pub swapped: Instant,
    /// Bytes copied from staging (row padding included).
    pub uploaded_bytes: u64,
    /// Frames its copies spanned.
    pub upload_frames: u32,
    /// Texture profile drawn.
    pub texture_profile: Option<Profile>,
    /// Volume profile drawn.
    pub volume_profile: Option<Profile>,
}

/// Selected cooked-mesh recording mechanism for this device generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CookedDrawPath {
    /// Select the existing indexed path for the device.
    Auto,
    /// GPU meshlet cull and indirect drawing.
    Indirect,
    /// One core indexed draw per placement; no meshlet cull resources.
    Direct,
    /// GPU object-stage culling and mesh-stage drawing.
    MeshStages,
}

impl CookedDrawPath {
    fn resolve(self, surface: &SceneSurface<'_>) -> (Self, Option<String>) {
        let indexed = Self::select(surface.downlevel);
        let why = match self {
            Self::Auto => return (indexed, None),
            Self::Direct => return (Self::Direct, None),
            Self::Indirect if indexed == Self::Direct => Some("device lacks indirect execution"),
            Self::MeshStages
                if !surface
                    .features
                    .contains(wgpu::Features::EXPERIMENTAL_MESH_SHADER) =>
            {
                Some("device did not enable mesh-stages")
            }
            Self::MeshStages if surface.recipe != thrum_render::capabilities::Recipe::AppleTile => {
                Some("recipe is portable")
            }
            Self::Indirect | Self::MeshStages => None,
        };
        (indexed, why.map(str::to_owned))
    }

    fn select(flags: Option<wgpu::DownlevelFlags>) -> Self {
        if flags.is_some_and(|flags| flags.contains(wgpu::DownlevelFlags::INDIRECT_EXECUTION)) {
            Self::Indirect
        } else {
            Self::Direct
        }
    }
}

/// Shared observations, readable while the contribution belongs to the frame host.
#[derive(Debug, Default)]
pub struct CookedStats {
    /// Number of individual cooked mesh structures encoded in submitted frames on this surface.
    #[cfg(feature = "draw-probe")]
    pub submitted_ray_builds: usize,
    /// Distinct drawn meshes and their position, attribute and index usage, and BLAS presence.
    #[cfg(feature = "draw-probe")]
    pub ray_mesh_storage: Vec<(String, [wgpu::BufferUsages; 3], bool)>,
    /// Actual bound draw commands observed at the last resident swap in a probe build.
    #[cfg(feature = "draw-probe")]
    pub probe_commands: Vec<String>,
    /// Actual INDIRECT-usage buffer count and presence of cull/count/readback resources.
    #[cfg(feature = "draw-probe")]
    pub probe_resources: (usize, bool),
    /// Swaps in order.
    pub swaps: Vec<SwapRecord>,
    /// The newest loader failures, at most 64 (the previous set stays drawn).
    pub errors: Vec<String>,
    /// Loader failures recorded since the contribution started, including any no longer in
    /// [`Self::errors`]; a reader that remembers it can tell which entries are new.
    pub failures: u64,
    /// [`Self::failures`] when the first set swapped in: later ones were refused while a set drew.
    pub failures_before_first_draw: u64,
    /// Largest staged-copy bytes recorded in one frame.
    pub max_frame_copy_bytes: u64,
    /// Staged-copy bytes recorded in the most recent frame.
    pub last_frame_copy_bytes: u64,
    /// Staging bytes the current device generation holds (its chunks, filled or not).
    pub staging_bytes: u64,
    /// Most staging bytes held at once, never more than [`STAGING_CHUNKS`] chunks of
    /// [`CookedConfig::upload_budget`] (at least [`COPY_UNIT`]).
    pub staging_peak_bytes: u64,
    /// Generation currently drawn.
    pub drawn: Option<u64>,
    /// Manifest hash from the same publication as [`Self::drawn`], committed on submission.
    pub drawn_manifest_hash: Option<ContentHash>,
    /// Bytes of the committed font sheet textures, including their mips.
    pub resident_font_bytes: u64,
    /// Device generation on which the drawn content was submitted.
    pub drawn_device_generation: Option<u64>,
    /// Path selected at the latest rebuild.
    pub draw_path: Option<CookedDrawPath>,
    /// The forward path requested for the main view.
    pub requested_draw_path: Option<CookedDrawPath>,
    /// Recipe used to prepare the current contribution.
    pub recipe: Option<thrum_render::capabilities::Recipe>,
    /// Why an explicit forward path could not be selected.
    pub draw_fallback: Option<String>,
    /// Mesh-stage draws in the latest submitted frame.
    pub mesh_stage_draws: usize,
    /// Indexed indirect draws in the latest submitted main forward pass.
    pub indexed_indirect_draws: usize,
    /// Indexed direct draws in the latest submitted main forward pass.
    pub indexed_direct_draws: usize,
    /// Whether the submitted frame dispatched the cooked compute cull.
    pub meshlet_compute_cull: bool,
    /// Placed mesh products excluded by device limits, with the limit that excluded them.
    pub mesh_stage_exclusions: Vec<(String, &'static str)>,
    /// Buffer residency of distinct drawn mesh products, including any meshlet storage.
    pub mesh_buffer_bytes: u64,
    /// Bytes of additional meshlet storage in distinct drawn mesh products.
    pub mesh_stage_bytes: u64,
    /// Logical bytes of the cooked draw resources in the submitted resident set.
    #[cfg(feature = "draw-probe")]
    pub draw_residency: draw_probe::DrawResidency,
    /// Cooked draws issued by the main Sun caster contribution in the latest submission.
    #[cfg(feature = "draw-probe")]
    pub sun_caster_draws: Vec<usize>,
    /// Resident position and attribute buffers with storage usage.
    pub mesh_stage_vertex_streams: usize,
    /// Candidate recorded by the latest encoding, including an encoding that was discarded.
    #[cfg(feature = "draw-probe")]
    pub recorded_material: Option<u64>,
    /// Slot index and resolved cooked draw path for each view with meshes in the
    /// most recent submitted frame. A slot can use Direct while the main uses Indirect.
    pub secondary_draw_paths: Vec<(usize, CookedDrawPath)>,
    /// Meshlets the GPU cull kept and the meshlets placed, from a recent frame. None on the direct path.
    pub visible_meshlets: Option<(u32, u32)>,
    /// Newest generation the loader has staged (drawn or still uploading).
    pub staged: Option<u64>,
    /// Frames this contribution recorded into, submitted or not.
    pub recorded_frames: u64,
    /// Publication whose last copy this frame recorded, before submission or discard.
    pub recorded_last_copy: Option<u64>,
    /// Recorded frames whose encoder was submitted.
    pub submitted_frames: u64,
    /// Whether the most recent submitted frame drew its meshes with valid motion history
    /// (`false` on a new set's first frame, after a rebuild or a view history reset).
    pub last_frame_history: Option<bool>,
}

/// Device texture format of a KTX2 product format.
pub fn wgpu_format(format: Ktx2Format) -> wgpu::TextureFormat {
    use wgpu::{AstcBlock, AstcChannel, TextureFormat};
    match format {
        Ktx2Format::R8Unorm => TextureFormat::R8Unorm,
        Ktx2Format::Rgba8Unorm => TextureFormat::Rgba8Unorm,
        Ktx2Format::Rgba8Srgb => TextureFormat::Rgba8UnormSrgb,
        Ktx2Format::Bc4Unorm => TextureFormat::Bc4RUnorm,
        Ktx2Format::Bc7Unorm => TextureFormat::Bc7RgbaUnorm,
        Ktx2Format::Bc7Srgb => TextureFormat::Bc7RgbaUnormSrgb,
        Ktx2Format::Astc4x4Unorm => TextureFormat::Astc {
            block: AstcBlock::B4x4,
            channel: AstcChannel::Unorm,
        },
        Ktx2Format::Astc4x4Srgb => TextureFormat::Astc {
            block: AstcBlock::B4x4,
            channel: AstcChannel::UnormSrgb,
        },
    }
}

/// Texture profiles in preference order: ASTC on Apple GPUs, BC7 elsewhere, portable last.
pub fn texture_profiles(features: wgpu::Features) -> Vec<Profile> {
    product_profiles(thrum_asset::ProductKind::Texture, features)
}

/// Volume profiles: BC4 where sliced-3D BC is supported, else R8.
pub fn volume_profiles(features: wgpu::Features) -> Vec<Profile> {
    product_profiles(thrum_asset::ProductKind::Volume, features)
}

fn product_profiles(kind: thrum_asset::ProductKind, features: wgpu::Features) -> Vec<Profile> {
    thrum_asset::preferred_profiles(
        kind,
        features.contains(wgpu::Features::TEXTURE_COMPRESSION_ASTC),
        features.contains(wgpu::Features::TEXTURE_COMPRESSION_BC),
        features.contains(wgpu::Features::TEXTURE_COMPRESSION_BC_SLICED_3D),
    )
}

/// Loader thread and channels for one device generation.
struct Live {
    #[cfg(feature = "draw-probe")]
    sun_caster_draws: std::cell::RefCell<Vec<usize>>,
    generation: u64,
    ray_queue: Option<wgpu::Queue>,
    stop: Arc<AtomicBool>,
    /// The loader's shared observations, so retirement can fence its last writes.
    stats: Arc<Mutex<CookedStats>>,
    loader: Option<JoinHandle<()>>,
    /// Remote feed prepared on the render thread, instead of a loader thread.
    remote: Option<Remote>,
    /// Newest staged set not yet taken by the render thread.
    mailbox: Arc<load::Mailbox>,
    resident_tx: Sender<Arc<load::ResidentSet>>,
    pipelines: Option<draw::Pipelines>,
    secondary_views: Vec<thrum_render::contribution::SecondaryViewFrame>,
    pending: Option<Pending>,
    current: Option<(Arc<load::ResidentSet>, Option<draw::Bound>)>,
    readback: Option<Readback>,
    /// What this frame's encoder recorded, committed only by `submitted`.
    tentative: Option<Tentative>,
    /// A validated material replacement, drawn from the next frame and promoted by the first
    /// submission whose frame recorded a resident cooked mesh with it; held as ready until then.
    material: Option<material::Eligible>,
}

/// A publication being uploaded. Its committed copy progress lives in `set.upload`.
struct Pending {
    /// Last copy committed in an earlier submission. Immutable until the switch submits.
    ready: bool,
    /// Completion of the initial upload, owned by this pending set alone.
    initial_ray_completion: Option<Arc<AtomicBool>>,
    set: load::PendingSet,
    first_copy: Option<Instant>,
    frames: u32,
    /// Bindings for the frame that records its last copies (kept if that frame is discarded).
    bound: Option<draw::Bound>,
}

impl Pending {
    fn initial_ray_upload_completed(&mut self, queue: Option<&wgpu::Queue>) -> bool {
        let Some(queue) = queue.filter(|_| self.set.upload.has_ray_meshes()) else {
            return true;
        };
        self.initial_ray_completion
            .get_or_insert_with(|| {
                let completed = Arc::new(AtomicBool::new(false));
                let signal = Arc::clone(&completed);
                queue.on_submitted_work_done(move || signal.store(true, Ordering::Release));
                completed
            })
            .load(Ordering::Acquire)
    }
}

/// One frame's recording.
#[derive(Clone, Copy, Default)]
struct Tentative {
    view_offset: u32,
    recorded: Option<staging::Recorded>,
    /// This frame draws the pending set and commits it if submitted.
    swap: bool,
    /// The cull count was copied out in this frame, of this many meshlets.
    counted: Option<u32>,
    /// The meshes' history flag this frame.
    history: Option<bool>,
    /// This frame draws the held material replacement.
    material: bool,
    /// This frame records draws of at least one resident cooked mesh (whose cull may still leave
    /// every primitive undrawn).
    meshed: bool,
}

/// Non-blocking copy-out of the cull's visible count: copy in a submitted frame, map after that
/// submission, read whenever the map completes. Never waits.
struct Readback {
    buffer: wgpu::Buffer,
    state: Arc<AtomicU8>,
    /// Meshlets culled in the submitted frame whose count is mapping, so a count read after a
    /// swap is paired with the set it counted.
    total: u32,
}

const IDLE: u8 = 0;
const MAPPING: u8 = 1;
const MAPPED: u8 = 2;

impl Readback {
    fn new(device: &wgpu::Device) -> Self {
        Self {
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("cooked visible readback"),
                size: 4,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }),
            state: Arc::new(AtomicU8::new(IDLE)),
            total: 0,
        }
    }

    /// Read a completed map, then copy this frame's count when the buffer is free.
    fn record(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        bound: &draw::Bound,
        stats: &Mutex<CookedStats>,
    ) -> Option<u32> {
        if self.state.load(Ordering::Acquire) == MAPPED {
            let visible = self
                .buffer
                .get_mapped_range(..)
                .ok()
                .and_then(|bytes| bytes.get(..4).and_then(|word| word.try_into().ok()))
                .map(u32::from_le_bytes);
            self.buffer.unmap();
            if let (Some(visible), Ok(mut stats)) = (visible, stats.lock()) {
                stats.visible_meshlets = Some((visible, self.total));
            }
            self.state.store(IDLE, Ordering::Release);
        }
        if self.state.load(Ordering::Acquire) != IDLE {
            return None;
        }
        encoder.copy_buffer_to_buffer(bound.visible.as_ref()?, 0, &self.buffer, 0, 4);
        Some(bound.meshlets())
    }

    /// The frame holding the copy was submitted: map after it completes.
    fn submitted(&mut self, total: u32) {
        self.total = total;
        self.state.store(MAPPING, Ordering::Release);
        let state = Arc::clone(&self.state);
        self.buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                state.store(
                    if result.is_ok() { MAPPED } else { IDLE },
                    Ordering::Release,
                );
            });
    }
}

impl Drop for Live {
    fn drop(&mut self) {
        // Never block the render thread on the loader: wake it and let it exit on its own after
        // whatever product it is preparing; it owns nothing the next generation needs.
        load::retire(&self.stats, &self.stop);
        if let Some(loader) = self.loader.take() {
            loader.thread().unpark();
        }
    }
}

impl Live {
    fn draw_pipelines(&self) -> &draw::Pipelines {
        self.pipelines.as_ref().expect("bound 3D draw pipelines")
    }

    /// The set this frame draws, including a pending publication at its switch.
    fn drawn(&self) -> Option<&draw::Bound> {
        if self.tentative.is_some_and(|tentative| tentative.swap) {
            self.pending
                .as_ref()
                .and_then(|pending| pending.bound.as_ref())
        } else {
            self.current.as_ref().and_then(|(_, bound)| bound.as_ref())
        }
    }

    fn culls(&self) -> bool {
        self.drawn()
            .is_some_and(|bound| bound.culls(&self.secondary_views))
    }

    /// The material replacement this frame draws instead of the accepted mesh pipeline.
    fn mesh_override(&self) -> Option<&material::MeshPipelines> {
        self.tentative
            .filter(|tentative| tentative.material)
            .and(self.material.as_ref())
            .map(|eligible| &eligible.pipeline)
    }
}

/// Explicitly enabled command controls for the cooked-draw qualification binary.
/// This module is absent from ordinary builds.
#[cfg(feature = "draw-probe")]
pub mod draw_probe;

struct PreparedCooked {
    target: material::Target,
    pipelines: draw::Pipelines,
    readback: Option<Readback>,
    current: Option<(Arc<load::ResidentSet>, draw::Bound)>,
    loader: Option<load::Loader>,
    reuse_live: bool,
    retiring_rays: bool,
    indexed_path: CookedDrawPath,
    fallback: Option<String>,
    recipe: thrum_render::capabilities::Recipe,
    generation: u64,
    secondary_count: usize,
    ray_queue: Option<wgpu::Queue>,
}

/// Cooked products drawn in the scene. Attach with
/// `FrameHost::set_scene_contribution(Box::new(assets))`.
pub struct CookedAssets {
    lease: ProductLease,
    #[cfg(feature = "draw-probe")]
    draw_probe: Option<draw_probe::Probe>,
    config: CookedConfig,
    stats: Arc<Mutex<CookedStats>>,
    live: Option<Live>,
    /// The accepted material hook and every reload request.
    material: MaterialReload,
    /// Scene surfaces rebuilt for; a material candidate is bound to one.
    epoch: u64,
}

impl CookedAssets {
    /// Connect cooked entity rows before attaching the contribution to a host.
    /// # Errors
    /// Refuses a row reader attached after GPU preparation began.
    pub fn with_rows(mut self, rows: rows::CookedRows) -> Result<Self, rows::CookedRowError> {
        if self.live.is_some() {
            return Err(rows::CookedRowError::Missing(
                "rows must precede preparation".into(),
            ));
        }
        self.config.rows = Some(rows);
        Ok(self)
    }

    /// Prepare; nothing runs until the frame host first rebuilds it for a scene.
    ///
    /// # Errors
    /// Refuses a mesh placement whose transform is not rigid with one uniform positive scale,
    /// naming the placement, the cause and the fix.
    pub fn new(config: CookedConfig) -> Result<Self, PlacementError> {
        for (index, placement) in config.meshes.iter().enumerate() {
            placement.check(index)?;
        }
        Ok(Self {
            lease: ProductLease::default(),
            config,
            #[cfg(feature = "draw-probe")]
            draw_probe: None,
            stats: Arc::new(Mutex::new(CookedStats::default())),
            live: None,
            material: MaterialReload::new(),
            epoch: 0,
        })
    }

    /// Attach probe-only commands before the contribution is prepared.
    /// # Errors
    /// Refuses mismatched placement coverage or a control attached after preparation.
    #[cfg(feature = "draw-probe")]
    pub fn with_draw_probe(
        mut self,
        probe: draw_probe::Probe,
    ) -> Result<Self, draw_probe::DrawProbeError> {
        probe.validate(self.config.meshes.len())?;
        if self.live.is_some() {
            return Err(draw_probe::DrawProbeError::AlreadyPrepared);
        }
        self.draw_probe = Some(probe);
        Ok(self)
    }

    /// Explicit reloads of the meshes' material hook. Without a request the meshes draw the
    /// built-in material, exactly as they would without this handle.
    pub fn material(&self) -> MaterialReload {
        self.material.clone()
    }

    /// Shared observations.
    pub fn stats(&self) -> Arc<Mutex<CookedStats>> {
        Arc::clone(&self.stats)
    }

    /// Read-only products named by `CookedConfig::leased`, for other contributions.
    pub fn lease(&self) -> ProductLease {
        self.lease.clone()
    }

    fn prepare_loader(
        &self,
        surface: &SceneSurface<'_>,
        pipelines: &draw::Pipelines,
    ) -> load::Loader {
        let mut loader = self.texture_loader(surface.device);
        loader.ray_demand = surface.ray_demand;
        #[cfg(all(feature = "draw-probe", target_os = "macos"))]
        {
            loader.ray_build_probe.clone_from(&surface.ray_build_probe);
        }
        loader.indirect = pipelines.indexed_path == CookedDrawPath::Indirect
            || (!surface.secondary_surfaces.is_empty()
                && CookedDrawPath::select(surface.downlevel) == CookedDrawPath::Indirect);
        loader.mesh_stage_limits = pipelines.uses_mesh_stages().then(|| surface.limits.clone());
        loader
    }

    fn texture_loader(&self, device: &wgpu::Device) -> load::Loader {
        let features = device.features();
        load::Loader {
            ray_demand: false,
            #[cfg(all(feature = "draw-probe", target_os = "macos"))]
            ray_build_probe: None,
            indirect: false,
            mesh_stage_limits: None,
            device: device.clone(),
            ring: staging::StagingRing::new(
                device,
                self.config.upload_budget,
                Arc::clone(&self.stats),
                #[cfg(feature = "fault-probe")]
                self.config.fault.clone(),
            ),
            config: self.config.clone(),
            texture_profiles: self.config.texture_preference.clone().map_or_else(
                || texture_profiles(features),
                |preferred| {
                    let supported = texture_profiles(features);
                    preferred
                        .into_iter()
                        .filter(|profile| supported.contains(profile))
                        .chain([Profile::Portable])
                        .collect()
                },
            ),
            volume_profiles: volume_profiles(features),
            stats: Arc::clone(&self.stats),
        }
    }

    fn start(&self, loader: load::Loader, generation: u64, draw: Option<PreparedCooked>) -> Live {
        let (ray_queue, pipelines, secondary_count, current, readback) =
            draw.map_or((None, None, 0, None, None), |draw| {
                (
                    draw.ray_queue,
                    Some(draw.pipelines),
                    draw.secondary_count,
                    draw.current.map(|(set, bound)| (set, Some(bound))),
                    draw.readback,
                )
            });
        // Only accepted preparations may start consuming loader or upload progress.
        let mailbox = Arc::new(load::Mailbox::default());
        let posting = Arc::clone(&mailbox);
        let (resident_tx, resident_rx) = channel();
        let stop = Arc::new(AtomicBool::new(false));
        let confirmed = current
            .as_ref()
            .map_or_else(load::Confirmed::default, |(set, _)| {
                load::Loader::confirm(set)
            });
        let mut remote = None;
        let handle = match &self.config.source {
            #[cfg(target_arch = "wasm32")]
            CookedSource::Store(_) => {
                drop((posting, resident_rx, confirmed));
                loader.fail(&stop, "the browser has no store loader thread");
                None
            }
            #[cfg(not(target_arch = "wasm32"))]
            CookedSource::Store(_) => {
                let stopping = Arc::clone(&stop);
                let handle = std::thread::Builder::new()
                    .name("thrum-cooked-loader".into())
                    .spawn(move || loader.run(&posting, &resident_rx, &stopping, confirmed))
                    .ok();
                if handle.is_none()
                    && let Ok(mut stats) = self.stats.lock()
                {
                    stats
                        .errors
                        .push("cooked loader thread failed to start".into());
                }
                handle
            }
            CookedSource::Remote(feed) => {
                let state = Remote {
                    loader,
                    feed: feed.clone(),
                    seen: None,
                    confirmed,
                    filler: None,
                };
                remote = Some(state);
                None
            }
        };
        Live {
            #[cfg(feature = "draw-probe")]
            sun_caster_draws: std::cell::RefCell::default(),
            generation,
            ray_queue,
            stop,
            stats: Arc::clone(&self.stats),
            loader: handle,
            remote,
            mailbox,
            resident_tx,
            pipelines,
            secondary_views: Vec::with_capacity(secondary_count),
            pending: None,
            current,
            readback,
            tentative: None,
            material: None,
        }
    }

    fn stage_rebuild(&self, surface: &SceneSurface<'_>) -> PreparedCooked {
        let (indexed_path, mut fallback) = self.config.draw_path.resolve(surface);
        let mut target = material::Target::new(surface, self.epoch + 1);
        let pipelines = draw::Pipelines::new(
            surface,
            &mut target,
            indexed_path,
            &self.config,
            &mut fallback,
        );
        #[cfg(feature = "draw-probe")]
        let pipelines = {
            let mut pipelines = pipelines;
            pipelines.probe(self.draw_probe.clone());
            pipelines
        };
        let stages = pipelines.uses_mesh_stages();
        let reuse = self.live.as_ref().filter(|live| {
            live.generation == surface.generation
                && live
                    .pipelines
                    .as_ref()
                    .is_some_and(|pipelines| pipelines.uses_mesh_stages() == stages)
                && (!surface.ray_demand || live.ray_queue.is_some())
        });
        let retiring_rays =
            reuse.is_some_and(|live| live.ray_queue.is_some() && !surface.ray_demand);
        let current = reuse
            .and_then(|live| live.current.as_ref())
            .map(|(set, _)| {
                let set = if retiring_rays {
                    Arc::new(set.without_ray_meshes())
                } else {
                    Arc::clone(set)
                };
                let bound = pipelines.bind(&set, &self.config.meshes, self.config.volume.as_ref());
                (set, bound)
            });
        let readback =
            (indexed_path == CookedDrawPath::Indirect).then(|| Readback::new(surface.device));
        let reuse_live = reuse.is_some() && !retiring_rays;
        let loader = (!reuse_live).then(|| self.prepare_loader(surface, &pipelines));
        PreparedCooked {
            target,
            pipelines,
            readback,
            current,
            loader,
            reuse_live,
            retiring_rays,
            indexed_path,
            fallback,
            recipe: surface.recipe,
            generation: surface.generation,
            secondary_count: surface.secondary_surfaces.len(),
            ray_queue: surface.ray_demand.then(|| surface.queue.clone()),
        }
    }

    fn rebuild_inner(
        &mut self,
        surface: &SceneSurface<'_>,
    ) -> Result<(), thrum_render::runtime::FrameError> {
        let internal = surface.device.push_error_scope(wgpu::ErrorFilter::Internal);
        let memory = surface
            .device
            .push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        let validation = surface
            .device
            .push_error_scope(wgpu::ErrorFilter::Validation);
        let prepared = self.stage_rebuild(surface);
        // Native wgpu validates synchronously. A backend whose scope is still pending
        // reports any uncaptured error through the device handler, as on first attach.
        let errors = [validation, memory, internal].map(|scope| {
            let mut pop = std::pin::pin!(scope.pop());
            match pop
                .as_mut()
                .poll(&mut std::task::Context::from_waker(std::task::Waker::noop()))
            {
                std::task::Poll::Ready(error) => error,
                std::task::Poll::Pending => None,
            }
        });
        if let Some(error) = errors.into_iter().flatten().next() {
            return Err(thrum_render::runtime::FrameError::gpu(
                "cooked pipelines",
                error,
            ));
        }
        let restart = !prepared.reuse_live;
        self.install_rebuild(prepared);
        if restart
            && let Some(live) = &mut self.live
            && let Some(remote) = &mut live.remote
        {
            remote.prepare_newest(&live.mailbox, &live.stop);
        }
        Ok(())
    }

    fn install_rebuild(&mut self, mut prepared: PreparedCooked) {
        let PreparedCooked {
            reuse_live,
            retiring_rays,
            indexed_path,
            generation,
            secondary_count,
            ..
        } = prepared;
        // A material candidate for the previous surface cannot publish. The new pipelines draw
        // the built-in material until the accepted hook validates against this surface.
        self.epoch += 1;
        if let Some(held) = self.live.as_mut().and_then(|live| live.material.take()) {
            self.material.release(&held);
        }
        self.material.retarget(&prepared.target);
        if let Ok(mut stats) = self.stats.lock() {
            stats.draw_path = Some(if prepared.pipelines.uses_mesh_stages() {
                CookedDrawPath::MeshStages
            } else {
                indexed_path
            });
            stats.requested_draw_path = Some(self.config.draw_path);
            stats.recipe = Some(prepared.recipe);
            stats.draw_fallback = prepared.fallback.take();
            stats.visible_meshlets = None;
            stats.secondary_draw_paths.clear();
        }
        if let Some(live) = self.live.as_mut().filter(|_| reuse_live) {
            // Same device: the scene shader or layouts may have changed; resources stay, history
            // restarts with the new bindings.
            live.readback = prepared.readback;
            live.pipelines = Some(prepared.pipelines);
            live.current = prepared.current.map(|(set, bound)| (set, Some(bound)));
            live.secondary_views = Vec::with_capacity(secondary_count);
            live.tentative = None;
            #[cfg(feature = "draw-probe")]
            live.sun_caster_draws.get_mut().clear();
            if let Some(pending) = live.pending.as_mut() {
                pending.bound = None;
            }
        } else {
            let old = self.live.take();
            if retiring_rays {
                // The new Maps arena uses committed rows. Replan an unsent
                // cooked publication against the raster set that last drew.
                if let Some(pending) = old.as_ref().and_then(|live| live.pending.as_ref()) {
                    if let Some(rows) = &self.config.rows {
                        rows.updates.invalidate_device();
                    }
                    if pending.ready
                        && (self.config.rows.is_some() || !self.config.leased.is_empty())
                    {
                        if let Some((set, _)) = &prepared.current {
                            offer_leases(&self.lease, &self.config.leased, set, generation);
                        } else {
                            self.lease.clear();
                        }
                    }
                }
            } else {
                // An incompatible device or draw stream needs new residency.
                self.lease.clear();
                if let Some(rows) = &self.config.rows {
                    rows.updates.invalidate_device();
                }
                if let Ok(mut stats) = self.stats.lock() {
                    (stats.drawn, stats.drawn_manifest_hash) = (None, None);
                    stats.resident_font_bytes = 0;
                    stats.drawn_device_generation = None;
                    stats.last_frame_history = None;
                    stats.mesh_stage_exclusions.clear();
                    stats.mesh_buffer_bytes = 0;
                    stats.mesh_stage_bytes = 0;
                    #[cfg(feature = "draw-probe")]
                    {
                        stats.submitted_ray_builds = 0;
                        stats.ray_mesh_storage.clear();
                        stats.draw_residency = draw_probe::DrawResidency::default();
                        stats.sun_caster_draws.clear();
                    }
                }
            }
            drop(old);
            self.live = Some(self.start(
                prepared.loader.take().expect("staged cooked loader"),
                generation,
                Some(prepared),
            ));
        }
        #[cfg(feature = "draw-probe")]
        if let Some(live) = &self.live
            && let Some((set, Some(bound))) = &live.current
            && let Ok(mut stats) = self.stats.lock()
        {
            stats.draw_residency = live.draw_pipelines().draw_residency(
                bound,
                set,
                live.readback.as_ref().map(|readback| &readback.buffer),
            );
            if retiring_rays {
                set.report_mesh_storage(&self.config.meshes, &mut stats);
            }
        }
    }

    fn live_for(&mut self, frame: &ContributionFrame) -> Option<&mut Live> {
        self.live
            .as_mut()
            .filter(|live| live.generation == frame.generation)
    }
}

fn font_stamp(set: &load::ResidentSet) -> Option<thrum_render::sprite::PaintStamp> {
    (set.font_scale != 0).then_some(thrum_render::sprite::PaintStamp {
        head: set.head,
        scale: set.font_scale,
    })
}

fn font_candidate(requests: &[LeaseRequest], set: &load::ResidentSet) -> Option<FontCandidate> {
    Some(FontCandidate {
        stamp: font_stamp(set)?,
        sheets: requests
            .iter()
            .zip(&set.leased)
            .filter_map(|(request, (_, _, _, texture))| {
                if texture.font.is_some() {
                    return None;
                }
                Some((request.name.clone(), texture.sheet.clone()?))
            })
            .collect(),
        fonts: requests
            .iter()
            .zip(&set.leased)
            .filter_map(|(request, (_, _, _, texture))| {
                Some((request.name.clone(), texture.font.clone()?))
            })
            .collect(),
    })
}

fn offer_leases(
    lease: &ProductLease,
    requests: &[LeaseRequest],
    set: &load::ResidentSet,
    device_generation: u64,
) {
    let products = set
        .leased
        .iter()
        .zip(requests)
        .map(
            |((hash, portable_hash, profile, texture), request)| LentProduct {
                name: request.name.clone(),
                hash: *hash,
                portable_hash: *portable_hash,
                profile: *profile,
                device_generation,
                serial: 0, // Assigned by the lease's lifetime counter at offer.
                view: texture.view.clone(),
                texture: texture.texture.clone(),
                sheet: texture.sheet.clone(),
            },
        )
        .collect();
    lease.offer(set.generation, products, font_stamp(set));
}

impl SceneContribution for CookedAssets {
    #[cfg(not(target_arch = "wasm32"))]
    fn supports_candidate_preparation(&self) -> bool {
        true
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn prepare_candidate(
        &self,
        surface: &SceneSurface<'_>,
    ) -> Result<thrum_render::contribution::ContributionCandidate, thrum_render::runtime::FrameError>
    {
        let prepared = self.stage_rebuild(surface);
        Ok(thrum_render::contribution::ContributionCandidate::new(
            self.passes(),
            self.effect_graph(),
            move |owner: &mut Self| owner.install_rebuild(prepared),
        ))
    }

    fn ray_instances(&self, instances: &mut thrum_render::contribution::RayInstances<'_>) {
        if let Some(bound) = self.live.as_ref().and_then(Live::drawn) {
            bound.ray_instances(instances);
        }
    }

    fn secondary_views_enabled(&self) -> bool {
        !self.config.meshes.is_empty()
    }

    fn encode_secondary_upload(
        &mut self,
        _encoder: &mut wgpu::CommandEncoder,
        frame: &ContributionFrame,
        views: &[thrum_render::contribution::SecondaryViewFrame],
    ) {
        if let Some(live) = self.live_for(frame) {
            live.secondary_views.extend_from_slice(views);
            if let Some(pipelines) = &live.pipelines {
                pipelines.stage_views(views);
            }
        }
    }

    fn draw_secondary(&self, index: usize, pass: &mut wgpu::RenderPass<'_>) {
        if let Some(live) = &self.live
            && let Some(bound) = live.drawn()
        {
            live.draw_pipelines()
                .draw_view(bound, pass, index, live.mesh_override());
        }
    }

    fn draw_secondary_shadow(
        &self,
        index: usize,
        cascade: SecondaryShadowCascade,
        pass: &mut wgpu::RenderPass<'_>,
    ) {
        if let Some(live) = &self.live
            && let Some(bound) = live.drawn()
        {
            live.draw_pipelines()
                .draw_view_shadow(bound, pass, index, cascade);
        }
    }

    fn draw_stages(&self) -> Vec<thrum_render::contribution::SceneDrawStage> {
        if self.config.meshes.is_empty()
            && self.config.volume.is_none()
            && self.config.rows.is_none()
        {
            return Vec::new();
        }
        vec![thrum_render::contribution::SceneDrawStage::Opaque]
    }

    fn sprite_stage_refusal(&self) -> Option<String> {
        if let Some(placement) = self.config.meshes.first() {
            return Some(format!(
                "2D scene refuses mesh placement '{}'",
                placement.mesh
            ));
        }
        if let Some(placement) = &self.config.volume {
            return Some(format!(
                "2D scene refuses volume placement '{}'",
                placement.name
            ));
        }
        self.config
            .rows
            .as_ref()
            .and_then(|rows| rows.names.first())
            .map(|name| format!("2D scene refuses cooked mesh row '{name}'"))
    }

    fn rebuild_2d(
        &mut self,
        surface: &thrum_render::contribution::SpriteContributionSurface<'_>,
    ) -> Result<(), thrum_render::runtime::FrameError> {
        if let Some(reason) = self.sprite_stage_refusal() {
            return Err(thrum_render::runtime::FrameError::Scene(reason));
        }
        let generation = surface.capabilities.generation();
        if self
            .live
            .as_ref()
            .is_some_and(|live| live.generation == generation)
        {
            return Ok(());
        }
        let device = surface.capabilities.site();
        let loader = self.texture_loader(device);
        self.lease.clear();
        self.live = None;
        if let Ok(mut stats) = self.stats.lock() {
            (stats.drawn, stats.drawn_manifest_hash) = (None, None);
            stats.resident_font_bytes = 0;
            stats.drawn_device_generation = None;
            stats.draw_path = None;
        }
        self.live = Some(self.start(loader, generation, None));
        Ok(())
    }

    fn passes(&self) -> ContributionPasses {
        ContributionPasses {
            upload: true,
            compute: self
                .live
                .as_ref()
                .is_none_or(|live| live.pipelines.is_some()),
        }
    }

    fn rebuild(
        &mut self,
        surface: &SceneSurface<'_>,
    ) -> Result<(), thrum_render::runtime::FrameError> {
        self.rebuild_inner(surface)
    }

    fn encode_upload(&mut self, encoder: &mut wgpu::CommandEncoder, frame: &ContributionFrame) {
        let budget = self.config.upload_budget;
        let meshes = &self.config.meshes;
        let volume = self.config.volume.as_ref();
        // A recording that was never submitted must not be committed by a later submission.
        if let Some(live) = self.live.as_mut() {
            live.tentative = None;
            live.secondary_views.clear();
        }
        let Some(live) = self
            .live
            .as_mut()
            .filter(|live| live.generation == frame.generation)
        else {
            return;
        };
        // A newer fetched generation is prepared here, on the render thread, before the take.
        if let Some(remote) = live.remote.as_mut() {
            remote.prepare_newest(&live.mailbox, &live.stop);
        }
        // A frame boundary: the newest validated material replacement is drawn from this frame.
        self.material
            .frame_boundary(&mut live.material, (live.generation, self.epoch));
        // Newest publication wins; a superseded pending set is dropped with its staging.
        if !live.pending.as_ref().is_some_and(|pending| pending.ready)
            && let Some(set) = live.mailbox.take()
        {
            if let (Some(rows), Some(pending)) = (&self.config.rows, &live.pending) {
                rows.updates.cancel(pending.set.set.generation);
            }
            // A publication prepared behind a ready switch may reuse an older drawn
            // set. Never record its redundant destinations: the loader replans it
            // when the switch is submitted, on both native and remote feeds.
            live.pending = set
                .ok()
                .filter(|set| {
                    set.base_head == live.current.as_ref().map(|(set, _)| set.head)
                        && set.base_font_scale
                            == live.current.as_ref().map_or(0, |(set, _)| set.font_scale)
                        && set.set.font_scale == self.config.font_scale()
                })
                .map(|set| Pending {
                    ready: false,
                    initial_ray_completion: None,
                    set,
                    first_copy: None,
                    frames: 0,
                    bound: None,
                });
            if !self.config.font_leases.is_empty() {
                self.lease.prepare_font(
                    live.pending
                        .as_ref()
                        .and_then(|pending| font_candidate(&self.config.leased, &pending.set.set)),
                );
            }
            if let (Some(rows), Some(pending)) = (&self.config.rows, &live.pending) {
                // The entity arena already holds these rows, even before
                // the first set of placements becomes resident on this device.
                let shown = live
                    .ray_queue
                    .is_some()
                    .then(|| rows.shown.lock().ok())
                    .flatten();
                let current = shown.as_deref().map_or_else(
                    || {
                        live.current
                            .as_ref()
                            .map_or(&[][..], |(set, _)| set.rows.as_slice())
                    },
                    Vec::as_slice,
                );
                let changed = pending
                    .set
                    .set
                    .rows
                    .iter()
                    .enumerate()
                    .filter(|(index, (hash, _))| {
                        current
                            .get(*index)
                            .is_none_or(|(previous, _)| hash != previous)
                    })
                    .map(|(index, (_, geometry))| (index, geometry.clone()))
                    .collect();
                drop(shown);
                if let Err(error) = rows.updates.prepare(pending.set.set.generation, changed) {
                    if let Ok(mut stats) = self.stats.lock() {
                        stats.errors.push(format!("mesh rows: {error}"));
                        stats.failures += 1;
                    }
                    live.pending = None;
                }
            }
        }
        if let (Some(rows), Some(pending)) = (&self.config.rows, &live.pending)
            && let Some(thrum_render::synthetic::mesh_rows::MeshRowStatus::Refused { row, reason }) =
                rows.updates.status(pending.set.set.generation)
        {
            if let Ok(mut stats) = self.stats.lock() {
                stats.errors.push(format!(
                    "mesh row {} generation {}: {reason}",
                    rows.names.get(row).map_or("unknown", String::as_str),
                    pending.set.set.generation
                ));
                stats.failures += 1;
            }
            rows.updates.cancel(pending.set.set.generation);
            live.pending = None;
        }
        if let Some(pending) = live.pending.as_ref()
            && !self.config.font_leases.is_empty()
        {
            let stamp = font_stamp(&pending.set.set);
            let refusal = self.lease.font_refusal(stamp);
            if refusal.is_some()
                || (!pending.ready && pending.set.set.font_scale != self.config.font_scale())
            {
                if let Some(reason) = refusal
                    && let Ok(mut stats) = self.stats.lock()
                {
                    stats.errors.push(reason);
                    stats.failures += 1;
                }
                if let Some(rows) = &self.config.rows {
                    rows.updates.cancel(pending.set.set.generation);
                }
                live.pending = None;
                self.lease.prepare_font(None);
            }
        }
        // A set whose staging could not be filled never completes; the loader reported why.
        if live
            .pending
            .as_ref()
            .is_some_and(|pending| pending.set.upload.failed())
        {
            if let (Some(rows), Some(pending)) = (&self.config.rows, &live.pending) {
                rows.updates.cancel(pending.set.set.generation);
            }
            live.pending = None;
            self.lease.prepare_font(None);
        }
        #[cfg(feature = "draw-probe")]
        live.sun_caster_draws.get_mut().clear();
        let mut tentative = Tentative {
            view_offset: frame.view_uniform_offset,
            material: live.material.is_some(),
            ..Tentative::default()
        };
        if let Some(pending) = live.pending.as_mut() {
            if pending.ready {
                tentative.swap = true;
            } else {
                let recorded = pending.set.upload.encode(encoder, budget);
                tentative.swap = self.config.leased.is_empty()
                    && self.config.rows.is_none()
                    // Initial residency waits for its build submission to finish.
                    && !(live.ray_queue.is_some()
                        && live.current.is_none()
                        && pending.set.upload.has_ray_meshes())
                    && pending.set.upload.completes(recorded);
                tentative.recorded = Some(recorded);
            }
            if tentative.swap
                && pending.bound.is_none()
                && let Some(pipelines) = &live.pipelines
            {
                pending.bound = Some(pipelines.bind(&pending.set.set, meshes, volume));
            }
        }
        live.tentative = Some(tentative);
        let meshed = live.drawn().is_some_and(draw::Bound::has_meshes);
        let history = live
            .drawn()
            .filter(|bound| bound.has_meshes())
            .map(|bound| {
                let frame = ContributionFrame {
                    time: thrum_render::contribution::EffectTime {
                        epoch: 1,
                        timeline: 1,
                        tick: 0,
                        tick_duration: Duration::from_secs_f64(1.0 / 60.0),
                        alpha: 1.0,
                        display_phase: 1.0,
                        capture: false,
                    },
                    history_reset: frame.history_reset || tentative.material,
                    ..*frame
                };
                live.draw_pipelines().history(bound, &frame)
            });
        if let Some(tentative) = live.tentative.as_mut() {
            tentative.meshed = meshed;
            tentative.history = history;
        }
        if let Ok(mut stats) = self.stats.lock() {
            stats.recorded_frames += 1;
            stats.recorded_last_copy = tentative.recorded.and_then(|recorded| {
                let pending = live.pending.as_ref()?;
                (recorded.bytes > 0 && pending.set.upload.completes(recorded))
                    .then_some(pending.set.set.generation)
            });
            #[cfg(feature = "draw-probe")]
            {
                stats.recorded_material = live
                    .tentative
                    .filter(|tentative| tentative.material && tentative.meshed)
                    .and(live.material.as_ref())
                    .map(|held| held.serial);
            }
        }
    }

    fn computes(&self, frame: &ContributionFrame) -> bool {
        self.live
            .as_ref()
            .filter(|live| live.generation == frame.generation)
            .is_some_and(Live::culls)
    }

    fn encode_compute(&mut self, pass: &mut wgpu::ComputePass<'_>, frame: &ContributionFrame) {
        if let Some(live) = self.live_for(frame)
            && let Some(bound) = live.drawn()
        {
            live.draw_pipelines().cull(bound, pass, frame);
            live.draw_pipelines()
                .cull_views(bound, pass, &live.secondary_views);
        }
    }

    fn encode_after_compute(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let stats = Arc::clone(&self.stats);
        let Some(live) = self.live.as_mut() else {
            return;
        };
        let Some(bound) = live.drawn() else {
            return;
        };
        let Some(readback) = &live.readback else {
            return;
        };
        let counted = readback.record(encoder, bound, &stats);
        // The next frame's cull counts from zero.
        if let Some(visible) = &bound.visible {
            encoder.clear_buffer(visible, 0, None);
        }
        if let Some(tentative) = live.tentative.as_mut() {
            tentative.counted = counted;
        }
    }

    fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        if let Some(live) = &self.live
            && let Some(bound) = live.drawn()
        {
            let view_offset = live.tentative.map_or(0, |record| record.view_offset);
            live.draw_pipelines()
                .draw(bound, pass, live.mesh_override(), view_offset);
        }
    }

    fn draw_ambient_receivers(&self, pass: &mut wgpu::RenderPass<'_>) {
        if let Some(live) = &self.live
            && let Some(bound) = live.drawn()
        {
            let view_offset = live.tentative.map_or(0, |record| record.view_offset);
            live.draw_pipelines().draw_ambient_receivers(
                bound,
                pass,
                live.mesh_override(),
                view_offset,
            );
        }
    }

    fn shadow_bounds(&self) -> Option<ShadowBounds> {
        self.live.as_ref()?.drawn()?.shadow_bounds
    }

    fn draw_shadow(&self, pass: &mut wgpu::RenderPass<'_>) {
        if let Some(live) = &self.live
            && let Some(bound) = live.drawn()
        {
            let draws = live.draw_pipelines().draw_shadow(bound, pass);
            #[cfg(not(feature = "draw-probe"))]
            let _ = draws;
            #[cfg(feature = "draw-probe")]
            live.sun_caster_draws.borrow_mut().push(draws);
        }
    }

    fn submitted(&mut self, history_advanced: bool) {
        let Some(live) = self.live.as_mut() else {
            return;
        };
        let view_path = if live.secondary_views.is_empty() {
            None
        } else {
            live.drawn()
                .filter(|bound| bound.has_meshes())
                .map(draw::Bound::view_path)
        };
        let (stages, indirect, direct) = live.drawn().map_or((0, 0, 0), draw::Bound::draw_counts);
        let compute = live.culls();
        let Some(tentative) = live.tentative.take() else {
            return;
        };
        if let Some(total) = tentative.counted
            && let Some(readback) = &mut live.readback
        {
            readback.submitted(total);
        }
        if let Ok(mut stats) = self.stats.lock() {
            stats.submitted_frames += 1;
            stats.last_frame_history = tentative.history;
            stats.mesh_stage_draws = stages;
            stats.indexed_indirect_draws = indirect;
            stats.indexed_direct_draws = direct;
            stats.meshlet_compute_cull = compute;
            #[cfg(feature = "draw-probe")]
            stats
                .sun_caster_draws
                .clone_from(live.sun_caster_draws.get_mut());
            stats.secondary_draw_paths.clear();
            if let Some(path) = view_path {
                stats
                    .secondary_draw_paths
                    .extend(live.secondary_views.iter().map(|view| (view.index, path)));
            }
        }
        let now = Instant::now();
        let initial_ray_queue = live.ray_queue.as_ref().filter(|_| live.current.is_none());
        if let (Some(recorded), Some(pending)) = (tentative.recorded, live.pending.as_mut()) {
            #[cfg(feature = "draw-probe")]
            if let Ok(mut stats) = self.stats.lock() {
                stats.submitted_ray_builds += pending.set.upload.ray_build_count(recorded);
            }
            pending.set.upload.commit(recorded);
            if !tentative.swap
                && pending.set.upload.completes(recorded)
                && self.lease.font_ready(font_stamp(&pending.set.set))
                && pending.initial_ray_upload_completed(initial_ray_queue)
                && self
                    .config
                    .rows
                    .as_ref()
                    .is_none_or(|rows| rows.updates.offer(pending.set.set.generation))
            {
                pending.ready = true;
                if self.config.rows.is_some() || !self.config.leased.is_empty() {
                    offer_leases(
                        &self.lease,
                        &self.config.leased,
                        &pending.set.set,
                        live.generation,
                    );
                }
            }
            pending.first_copy.get_or_insert(now);
            pending.frames += 1;
            if let Ok(mut stats) = self.stats.lock() {
                stats.max_frame_copy_bytes = stats.max_frame_copy_bytes.max(recorded.bytes);
                stats.last_frame_copy_bytes = recorded.bytes;
            }
        } else if let Ok(mut stats) = self.stats.lock() {
            stats.last_frame_copy_bytes = 0;
        }
        if tentative.swap
            && let Some(pending) = live.pending.take()
        {
            let set = Arc::new(pending.set.set);
            self.lease.commit_font(font_stamp(&set));
            live.resident_tx.send(Arc::clone(&set)).ok();
            if let Some(remote) = live.remote.as_mut() {
                remote.confirmed = load::Loader::confirm(&set);
            }
            if let Ok(mut stats) = self.stats.lock() {
                if stats.drawn.is_none() {
                    stats.failures_before_first_draw = stats.failures;
                }
                #[cfg(feature = "draw-probe")]
                if let Some(bound) = &pending.bound {
                    stats.probe_commands = bound.probe_commands();
                    stats.probe_resources = bound.probe_resources();
                    stats.probe_resources.1 |= live.readback.is_some();
                    stats.draw_residency = live.draw_pipelines().draw_residency(
                        bound,
                        &set,
                        live.readback.as_ref().map(|readback| &readback.buffer),
                    );
                }
                (stats.drawn, stats.drawn_manifest_hash) =
                    (Some(set.head.generation), Some(set.head.manifest));
                stats.resident_font_bytes = set
                    .leased
                    .iter()
                    .filter(|(_, _, _, texture)| texture.font.is_some())
                    .map(|(_, _, _, texture)| texture.bytes)
                    .sum();
                set.report_mesh_storage(&self.config.meshes, &mut stats);
                stats.drawn_device_generation = Some(live.generation);
                stats.swaps.push(SwapRecord {
                    mapped: set
                        .meshes
                        .iter()
                        .all(|(mesh, texture)| mesh.mapped && texture.mapped)
                        && set.volume.as_ref().is_none_or(|volume| volume.mapped)
                        && set.leased.iter().all(|(_, _, _, texture)| texture.mapped),
                    generation: set.generation,
                    head_seen: pending.set.head_seen,
                    staged: pending.set.staged,
                    first_copy: pending.first_copy.unwrap_or(now),
                    swapped: now,
                    uploaded_bytes: pending.set.upload.total(),
                    upload_frames: pending.frames,
                    texture_profile: set.texture_profile,
                    volume_profile: set.volume_profile,
                });
            }
            if let Some(rows) = &self.config.rows
                && let Ok(mut shown) = rows.shown.lock()
            {
                shown.clone_from(&set.rows);
            }
            live.current = Some((set, pending.bound));
        }
        // Whatever this frame drew is now in the scene's previous frame, unless the frame was a
        // capture, which leaves view history where it was.
        if history_advanced && let Some((_, Some(bound))) = &live.current {
            bound.continued.set(true);
        }
        // A material is installed only once a submitted frame recorded a resident mesh with it;
        // until then it stays held, ready, and later frame boundaries may still supersede or
        // retire it.
        if tentative.material
            && tentative.meshed
            && let Some(eligible) = live.material.take()
        {
            if let Some(pipelines) = &mut live.pipelines {
                pipelines.install_mesh(eligible.pipeline.clone());
            }
            self.material.promote(eligible);
            // A capture does not become the previous frame: the next frame restarts history.
            if !history_advanced && let Some((_, Some(bound))) = &live.current {
                bound.continued.set(false);
            }
        }
    }
}

#[cfg(test)]
mod draw_path_tests {
    use super::CookedDrawPath;

    #[test]
    fn only_known_indirect_support_selects_indirect() {
        assert_eq!(CookedDrawPath::select(None), CookedDrawPath::Direct);
        assert_eq!(
            CookedDrawPath::select(Some(wgpu::DownlevelFlags::empty())),
            CookedDrawPath::Direct
        );
        assert_eq!(
            CookedDrawPath::select(Some(wgpu::DownlevelFlags::INDIRECT_EXECUTION)),
            CookedDrawPath::Indirect
        );
    }
}
