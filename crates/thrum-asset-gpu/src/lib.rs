//! GPU residency for cooked G5 products, drawn inside the prepared scene.
//!
//! [`CookedAssets`] is a [`SceneContribution`]: a loader thread follows the store's `HEAD`, maps
//! and validates the products a placement table names, creates their destinations and fills one
//! staging buffer per publication. The frame host's render thread then records that set's copies
//! under a per-frame byte budget in its own encoder, swaps the set in the frame its last copy is
//! recorded, culls each mesh's meshlets on the GPU and draws meshes with the scene's own fragment
//! shader (so they receive the scene's selected lighting) and the volume as a lit raymarch.
//!
//! Content identity survives device reconstruction: on a new device generation the loader is
//! restarted against the same store and products are mapped and staged again.
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
mod load;
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

pub use placement::{MeshPlacement, PlacementError};
pub use staging::COPY_UNIT;

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
    /// Feed serial last prepared.
    seen: u64,
    /// Products of the drawn set, reused by later generations.
    confirmed: load::Confirmed,
}

impl Remote {
    fn prepare_newest(&mut self, mailbox: &load::Mailbox, stop: &AtomicBool) {
        if let Some((serial, generation)) = self.feed.latest()
            && serial != self.seen
        {
            self.seen = serial;
            self.loader
                .prepare_remote(&generation, &self.confirmed, mailbox, stop);
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
    /// When its staging buffer was filled.
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

/// Shared observations, readable while the contribution belongs to the frame host.
#[derive(Debug, Default)]
pub struct CookedStats {
    /// Swaps in order.
    pub swaps: Vec<SwapRecord>,
    /// Loader failures (the previous set stays drawn).
    pub errors: Vec<String>,
    /// Largest staged-copy bytes recorded in one frame.
    pub max_frame_copy_bytes: u64,
    /// Staged-copy bytes recorded in the most recent frame.
    pub last_frame_copy_bytes: u64,
    /// Generation currently drawn.
    pub drawn: Option<u64>,
    /// Meshlets the GPU cull kept and the meshlets placed, from a recent frame.
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
    readback: Readback,
    /// What this frame's encoder recorded, committed only by `submitted`.
    tentative: Option<Tentative>,
}

/// A publication being uploaded. Its committed copy progress lives in `set.upload`.
struct Pending {
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
        encoder.copy_buffer_to_buffer(&bound.visible, 0, &self.buffer, 0, 4);
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
}

/// Cooked products drawn in the scene. Attach with
/// `FrameHost::set_scene_contribution(Box::new(assets))`.
pub struct CookedAssets {
    config: CookedConfig,
    stats: Arc<Mutex<CookedStats>>,
    live: Option<Live>,
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
            config,
            stats: Arc::new(Mutex::new(CookedStats::default())),
            live: None,
        })
    }

    /// Shared observations.
    pub fn stats(&self) -> Arc<Mutex<CookedStats>> {
        Arc::clone(&self.stats)
    }

    fn start(&self, surface: &SceneSurface<'_>) -> Live {
        // One newest staged set waits for the render thread; a newer one replaces it and the
        // superseded staging retires at once, even while no frame encodes.
        let mailbox = Arc::new(load::Mailbox::default());
        let posting = Arc::clone(&mailbox);
        let (resident_tx, resident_rx) = channel();
        let stop = Arc::new(AtomicBool::new(false));
        let loader = load::Loader {
            device: surface.device.clone(),
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
                    seen: u64::MAX,
                    confirmed: load::Confirmed::default(),
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
            pipelines: draw::Pipelines::new(surface),
            pending: None,
            current: None,
            readback: Readback::new(surface.device),
            tentative: None,
        }
    }

    /// Rebuild pipelines and bindings for `surface`, or restart on a new device generation.
    fn rebuild_inner(&mut self, surface: &SceneSurface<'_>) {
        match self.live.as_mut() {
            // Same device: the scene shader or layouts may have changed; resources stay, history
            // restarts with the new bindings.
            Some(live) if live.generation == surface.generation => {
                live.pipelines = draw::Pipelines::new(surface);
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
                self.live = None;
                self.live = Some(self.start(surface));
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
        // Newest publication wins; a superseded pending set is dropped with its staging.
        if let Some(set) = live.mailbox.take() {
            live.pending = Some(Pending {
                set,
                first_copy: None,
                frames: 0,
                bound: None,
            });
        }
        let mut tentative = Tentative::default();
        if let Some(pending) = live.pending.as_mut() {
            let recorded = pending.set.upload.encode(encoder, budget);
            tentative.swap = pending.set.upload.completes(recorded);
            tentative.recorded = Some(recorded);
            if tentative.swap && pending.bound.is_none() {
                pending.bound = Some(live.pipelines.bind(&pending.set.set, meshes, volume));
            }
        }
        live.tentative = Some(tentative);
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
            let history = live.pipelines.cull(bound, pass, frame);
            if let Some(tentative) = live.tentative.as_mut() {
                tentative.history = Some(history);
            }
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
        let counted = live.readback.record(encoder, bound, &stats);
        // The next frame's cull counts from zero.
        encoder.clear_buffer(&bound.visible, 0, None);
        if let Some(tentative) = live.tentative.as_mut() {
            tentative.counted = counted;
        }
    }

    fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        if let Some(live) = &self.live
            && let Some(bound) = live.drawn()
        {
            live.pipelines.draw(bound, pass);
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
        if let Some(total) = tentative.counted {
            live.readback.submitted(total);
        }
        if let Ok(mut stats) = self.stats.lock() {
            stats.submitted_frames += 1;
            stats.last_frame_history = tentative.history;
        }
        let now = Instant::now();
        if let (Some(recorded), Some(pending)) = (tentative.recorded, live.pending.as_mut()) {
            pending.set.upload.commit(recorded);
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
                stats.drawn = Some(set.generation);
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
    }
}
