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
#[cfg(feature = "fault-probe")]
mod fault;
mod lease;
#[cfg(feature = "fault-probe")]
pub use fault::FaultProbe;
mod load;
mod material;
mod placement;
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

use thrum_asset::{Ktx2Format, Profile, RemoteGeneration};
use thrum_render::contribution::{
    ContributionFrame, ContributionPasses, SceneContribution, SceneSurface, ShadowBounds,
};

pub use lease::{LeaseRequest, LentProduct, LentSet, ProductLease};
pub use material::{
    MaterialDiagnostic, MaterialHook, MaterialOutcome, MaterialReload, ReloadTiming,
};
pub use placement::{MeshPlacement, PlacementError};
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
    seen: Option<(u64, Option<thrum_asset::Head>)>,
    /// Products of the drawn set, reused by later generations.
    confirmed: load::Confirmed,
    /// Staging of the newest prepared generation, filled as chunks free up.
    filler: Option<staging::Filler>,
}

impl Remote {
    fn prepare_newest(&mut self, mailbox: &load::Mailbox, stop: &AtomicBool) {
        if let Some((serial, generation)) = self.feed.latest()
            && Some((serial, self.confirmed.head)) != self.seen
        {
            self.seen = Some((serial, self.confirmed.head));
            if Some(generation.head()) != self.confirmed.head {
                self.filler =
                    self.loader
                        .prepare_remote(&generation, &self.confirmed, mailbox, stop);
            }
        }
        // No thread here: fill whatever staging is free now, never waiting for more.
        let progress = self.filler.as_mut().map(staging::Filler::fill_available);
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
    /// Opt-in one-shot staging fill fault, absent from ordinary builds.
    #[cfg(feature = "fault-probe")]
    pub fault: Option<FaultProbe>,
    /// Additional sampled products, resident under the same publication and upload budget.
    pub leased: Vec<LeaseRequest>,
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
            #[cfg(feature = "fault-probe")]
            fault: None,
            leased: Vec::new(),
            source: source.into(),
            meshes,
            volume: None,
            upload_budget: DEFAULT_UPLOAD_BUDGET,
            poll_interval: Duration::from_millis(20),
            texture_preference: None,
        }
    }
}

/// One publication that became visible.
#[derive(Clone, Debug)]
pub struct SwapRecord {
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
    /// GPU meshlet cull and indirect drawing.
    Indirect,
    /// One core indexed draw per placement; no meshlet cull resources.
    Direct,
}

impl CookedDrawPath {
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
    /// Device generation on which the drawn content was submitted.
    pub drawn_device_generation: Option<u64>,
    /// Path selected at the latest rebuild.
    pub draw_path: Option<CookedDrawPath>,
    /// Meshlets the GPU cull kept and the meshlets placed, from a recent frame. None on the direct path.
    pub visible_meshlets: Option<(u32, u32)>,
    /// Newest generation the loader has staged (drawn or still uploading).
    pub staged: Option<u64>,
    /// Frames this contribution recorded into, submitted or not.
    pub recorded_frames: u64,
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
    let mut order = Vec::new();
    if features.contains(wgpu::Features::TEXTURE_COMPRESSION_ASTC) {
        order.push(Profile::AppleAstc);
    }
    if features.contains(wgpu::Features::TEXTURE_COMPRESSION_BC) {
        order.push(Profile::DesktopBc);
    }
    order.push(Profile::Portable);
    order
}

/// Volume profiles: BC4 where sliced-3D BC is supported, else R8.
pub fn volume_profiles(features: wgpu::Features) -> Vec<Profile> {
    let mut order = Vec::new();
    if features.contains(
        wgpu::Features::TEXTURE_COMPRESSION_BC | wgpu::Features::TEXTURE_COMPRESSION_BC_SLICED_3D,
    ) {
        order.push(Profile::DesktopBc);
    }
    order.push(Profile::Portable);
    order
}

/// Loader thread and channels for one device generation.
struct Live {
    generation: u64,
    stop: Arc<AtomicBool>,
    /// The loader's shared observations, so retirement can fence its last writes.
    stats: Arc<Mutex<CookedStats>>,
    loader: Option<JoinHandle<()>>,
    /// Remote feed prepared on the render thread, instead of a loader thread.
    remote: Option<Remote>,
    /// Newest staged set not yet taken by the render thread.
    mailbox: Arc<load::Mailbox>,
    resident_tx: Sender<Arc<load::ResidentSet>>,
    pipelines: draw::Pipelines,
    pending: Option<Pending>,
    current: Option<(Arc<load::ResidentSet>, draw::Bound)>,
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
    set: load::PendingSet,
    first_copy: Option<Instant>,
    frames: u32,
    /// Bindings for the frame that records its last copies (kept if that frame is discarded).
    bound: Option<draw::Bound>,
}

/// One frame's recording.
#[derive(Clone, Copy, Default)]
struct Tentative {
    recorded: Option<staging::Recorded>,
    /// The pending set's last copies are in this frame, which draws it.
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
    /// The set this frame draws: the pending one when its last copies are in this frame.
    fn drawn(&self) -> Option<&draw::Bound> {
        if self.tentative.is_some_and(|tentative| tentative.swap) {
            self.pending
                .as_ref()
                .and_then(|pending| pending.bound.as_ref())
        } else {
            self.current.as_ref().map(|(_, bound)| bound)
        }
    }

    /// The material replacement this frame draws instead of the accepted mesh pipeline.
    fn mesh_override(&self) -> Option<&wgpu::RenderPipeline> {
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
    pub fn with_draw_probe(mut self, probe: draw_probe::Probe) -> Result<Self, String> {
        probe.validate(self.config.meshes.len())?;
        if self.live.is_some() {
            return Err("draw probe must precede preparation".into());
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

    fn start(&self, surface: &SceneSurface<'_>, pipelines: draw::Pipelines) -> Live {
        // One newest staged set waits for the render thread; a newer one replaces it and the
        // superseded staging retires at once, even while no frame encodes.
        let mailbox = Arc::new(load::Mailbox::default());
        let posting = Arc::clone(&mailbox);
        let (resident_tx, resident_rx) = channel();
        let stop = Arc::new(AtomicBool::new(false));
        let loader = load::Loader {
            indirect: CookedDrawPath::select(surface.downlevel) == CookedDrawPath::Indirect,
            device: wgpu::Device::clone(surface.device),
            ring: staging::StagingRing::new(
                surface.device,
                self.config.upload_budget,
                Arc::clone(&self.stats),
                #[cfg(feature = "fault-probe")]
                self.config.fault.clone(),
            ),
            config: self.config.clone(),
            texture_profiles: self.config.texture_preference.clone().map_or_else(
                || texture_profiles(surface.features),
                |preferred| {
                    // A preference never selects a format the device cannot sample.
                    let supported = texture_profiles(surface.features);
                    preferred
                        .into_iter()
                        .filter(|profile| supported.contains(profile))
                        .chain([Profile::Portable])
                        .collect()
                },
            ),
            volume_profiles: volume_profiles(surface.features),
            stats: Arc::clone(&self.stats),
        };
        let mut remote = None;
        let handle = match &self.config.source {
            #[cfg(target_arch = "wasm32")]
            CookedSource::Store(_) => {
                drop((posting, resident_rx));
                loader.fail(&stop, "the browser has no store loader thread");
                None
            }
            #[cfg(not(target_arch = "wasm32"))]
            CookedSource::Store(_) => {
                let stopping = Arc::clone(&stop);
                let handle = std::thread::Builder::new()
                    .name("thrum-cooked-loader".into())
                    .spawn(move || loader.run(&posting, &resident_rx, &stopping))
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
                let mut state = Remote {
                    loader,
                    feed: feed.clone(),
                    seen: None,
                    confirmed: load::Confirmed::default(),
                    filler: None,
                };
                state.prepare_newest(&posting, &stop);
                remote = Some(state);
                None
            }
        };
        Live {
            generation: surface.generation,
            stop,
            stats: Arc::clone(&self.stats),
            loader: handle,
            remote,
            mailbox,
            resident_tx,
            pipelines,
            pending: None,
            current: None,
            readback: (CookedDrawPath::select(surface.downlevel) == CookedDrawPath::Indirect)
                .then(|| Readback::new(surface.device)),
            tentative: None,
            material: None,
        }
    }

    /// Rebuild pipelines and bindings for `surface`, or restart on a new device generation.
    fn rebuild_inner(&mut self, surface: &SceneSurface<'_>) {
        if let Ok(mut stats) = self.stats.lock() {
            stats.draw_path = Some(CookedDrawPath::select(surface.downlevel));
            stats.visible_meshlets = None;
        }
        // A material candidate for the previous surface cannot publish. The new pipelines draw
        // the built-in material until the accepted hook validates against this surface.
        self.epoch += 1;
        let target = material::Target::new(surface, self.epoch);
        if let Some(held) = self.live.as_mut().and_then(|live| live.material.take()) {
            self.material.release(&held);
        }
        let pipelines = draw::Pipelines::new(surface, &target);
        #[cfg(feature = "draw-probe")]
        let pipelines = {
            let mut pipelines = pipelines;
            pipelines.probe(self.draw_probe.clone());
            pipelines
        };
        self.material.retarget(&target);
        match self.live.as_mut() {
            // Same device: the scene shader or layouts may have changed; resources stay, history
            // restarts with the new bindings.
            Some(live) if live.generation == surface.generation => {
                live.readback = (CookedDrawPath::select(surface.downlevel)
                    == CookedDrawPath::Indirect)
                    .then(|| Readback::new(surface.device));
                live.pipelines = pipelines;
                live.tentative = None;
                if let Some((set, bound)) = live.current.as_mut() {
                    *bound =
                        live.pipelines
                            .bind(set, &self.config.meshes, self.config.volume.as_ref());
                }
                if let Some(pending) = live.pending.as_mut() {
                    pending.bound = None;
                }
            }
            // New device generation: products are mapped and staged again for it.
            _ => {
                self.lease.clear();
                if let Ok(mut stats) = self.stats.lock() {
                    stats.drawn = None;
                    stats.drawn_device_generation = None;
                    stats.last_frame_history = None;
                }
                self.live = None;
                self.live = Some(self.start(surface, pipelines));
            }
        }
    }

    fn live_for(&mut self, frame: &ContributionFrame) -> Option<&mut Live> {
        self.live
            .as_mut()
            .filter(|live| live.generation == frame.generation)
    }
}

impl SceneContribution for CookedAssets {
    fn passes(&self) -> ContributionPasses {
        ContributionPasses {
            upload: true,
            compute: true,
        }
    }

    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), String> {
        // Pipelines compile against the scene's shader here: a validation error detaches the
        // contribution (the frame host records why) instead of failing later frames.
        let scope = surface
            .device
            .push_error_scope(wgpu::ErrorFilter::Validation);
        self.rebuild_inner(surface);
        let mut pop = std::pin::pin!(scope.pop());
        // Native wgpu validates synchronously, so the scope is ready; a backend that is not
        // ready yet reports through the device's uncaptured-error handler instead.
        match pop
            .as_mut()
            .poll(&mut std::task::Context::from_waker(std::task::Waker::noop()))
        {
            std::task::Poll::Ready(Some(error)) => {
                self.live = None;
                Err(format!("cooked pipelines: {error}"))
            }
            _ => Ok(()),
        }
    }

    fn encode_upload(&mut self, encoder: &mut wgpu::CommandEncoder, frame: &ContributionFrame) {
        let budget = self.config.upload_budget;
        let meshes = &self.config.meshes;
        let volume = self.config.volume.as_ref();
        // A recording that was never submitted must not be committed by a later submission.
        if let Some(live) = self.live.as_mut() {
            live.tentative = None;
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
            // A publication prepared behind a ready switch may reuse an older drawn
            // set. Never record its redundant destinations: the loader replans it
            // when the switch is submitted, on both native and remote feeds.
            live.pending = set
                .ok()
                .filter(|set| set.base_head == live.current.as_ref().map(|(set, _)| set.head))
                .map(|set| Pending {
                    ready: false,
                    set,
                    first_copy: None,
                    frames: 0,
                    bound: None,
                });
        }
        // A set whose staging could not be filled never completes; the loader reported why.
        if live
            .pending
            .as_ref()
            .is_some_and(|pending| pending.set.upload.failed())
        {
            live.pending = None;
        }
        let mut tentative = Tentative {
            material: live.material.is_some(),
            ..Tentative::default()
        };
        if let Some(pending) = live.pending.as_mut() {
            if pending.ready {
                tentative.swap = true;
            } else {
                let recorded = pending.set.upload.encode(encoder, budget);
                tentative.swap =
                    self.config.leased.is_empty() && pending.set.upload.completes(recorded);
                tentative.recorded = Some(recorded);
            }
            if tentative.swap && pending.bound.is_none() {
                pending.bound = Some(live.pipelines.bind(&pending.set.set, meshes, volume));
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
                        capture: false,
                    },
                    history_reset: frame.history_reset || tentative.material,
                    ..*frame
                };
                live.pipelines.history(bound, &frame)
            });
        if let Some(tentative) = live.tentative.as_mut() {
            tentative.meshed = meshed;
            tentative.history = history;
        }
        if let Ok(mut stats) = self.stats.lock() {
            stats.recorded_frames += 1;
        }
    }

    fn computes(&self, frame: &ContributionFrame) -> bool {
        self.live
            .as_ref()
            .filter(|live| live.generation == frame.generation)
            .and_then(Live::drawn)
            .is_some_and(draw::Bound::culls)
    }

    fn encode_compute(&mut self, pass: &mut wgpu::ComputePass<'_>, frame: &ContributionFrame) {
        if let Some(live) = self.live_for(frame)
            && let Some(bound) = live.drawn()
        {
            live.pipelines.cull(bound, pass, frame);
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
            live.pipelines.draw(bound, pass, live.mesh_override());
        }
    }

    fn shadow_bounds(&self) -> Option<ShadowBounds> {
        self.live.as_ref()?.drawn()?.shadow_bounds
    }

    fn draw_shadow(&self, pass: &mut wgpu::RenderPass<'_>) {
        if let Some(live) = &self.live
            && let Some(bound) = live.drawn()
        {
            live.pipelines.draw_shadow(bound, pass);
        }
    }

    fn submitted(&mut self, history_advanced: bool) {
        let Some(live) = self.live.as_mut() else {
            return;
        };
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
        }
        let now = Instant::now();
        if let (Some(recorded), Some(pending)) = (tentative.recorded, live.pending.as_mut()) {
            pending.set.upload.commit(recorded);
            if !self.config.leased.is_empty() && pending.set.upload.completes(recorded) {
                pending.ready = true;
                let products = pending
                    .set
                    .set
                    .leased
                    .iter()
                    .zip(&self.config.leased)
                    .map(|((hash, portable_hash, profile, texture), request)| {
                        LentProduct {
                            name: request.name.clone(),
                            hash: *hash,
                            portable_hash: *portable_hash,
                            profile: *profile,
                            device_generation: live.generation,
                            serial: 0, // Assigned by the lease's lifetime counter at offer.
                            view: texture.view.clone(),
                            texture: texture.texture.clone(),
                        }
                    })
                    .collect();
                self.lease.offer(pending.set.set.generation, products);
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
            && let Some(bound) = pending.bound
        {
            let set = Arc::new(pending.set.set);
            live.resident_tx.send(Arc::clone(&set)).ok();
            if let Some(remote) = live.remote.as_mut() {
                remote.confirmed = load::Loader::confirm(&set);
            }
            if let Ok(mut stats) = self.stats.lock() {
                if stats.drawn.is_none() {
                    stats.failures_before_first_draw = stats.failures;
                }
                #[cfg(feature = "draw-probe")]
                {
                    stats.probe_commands = bound.probe_commands();
                    stats.probe_resources = bound.probe_resources();
                    stats.probe_resources.1 |= live.readback.is_some();
                }
                stats.drawn = Some(set.generation);
                stats.drawn_device_generation = Some(live.generation);
                stats.swaps.push(SwapRecord {
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
            live.current = Some((set, bound));
        }
        // Whatever this frame drew is now in the scene's previous frame, unless the frame was a
        // capture, which leaves view history where it was.
        if history_advanced && let Some((_, bound)) = &live.current {
            bound.continued.set(true);
        }
        // A material is installed only once a submitted frame recorded a resident mesh with it;
        // until then it stays held, ready, and later frame boundaries may still supersede or
        // retire it.
        if tentative.material
            && tentative.meshed
            && let Some(eligible) = live.material.take()
        {
            live.pipelines.install_mesh(eligible.pipeline.clone());
            self.material.promote(eligible);
            // A capture does not become the previous frame: the next frame restarts history.
            if !history_advanced && let Some((_, bound)) = &live.current {
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
