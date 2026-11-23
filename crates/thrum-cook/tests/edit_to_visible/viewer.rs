//! Diagnostic host for the shipping cooked contribution. No product loader lives here.
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use thrum_asset::Profile;
use thrum_asset_gpu::{CookedAssets, CookedConfig, CookedStats, MeshPlacement, VolumePlacement};
use thrum_core::{
    WorldEpoch,
    sim::{SnapshotId, WorldAdvance, WorldPublication},
};
use thrum_render::{
    capabilities::DeviceCapabilities,
    extract::{ExtractReport, RenderExtractor, SourceTick},
    runtime::{FrameConfig, FrameError, FrameHost, FrameWorld, SceneFrame, SceneSource},
    synthetic::{FrameLighting, SyntheticLight, SyntheticScene, SyntheticView},
};

pub(crate) struct ViewerConfig {
    pub(crate) store: PathBuf,
    pub(crate) extent: [u32; 2],
    pub(crate) upload_chunk: usize,
}
impl ViewerConfig {
    pub(crate) fn demo(store: impl Into<PathBuf>) -> Self {
        Self {
            store: store.into(),
            extent: [960, 540],
            upload_chunk: 1 << 20,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct FrameRecord {
    pub(crate) index: u64,
    pub(crate) generation: u64,
    pub(crate) cpu: Duration,
    pub(crate) submitted: Instant,
}
#[derive(Debug)]
pub(crate) struct SwapRecord {
    pub(crate) mapped: bool,
    pub(crate) texture_profile: Profile,
    pub(crate) volume_profile: Profile,
    pub(crate) generation: u64,
    pub(crate) head_seen: Instant,
    pub(crate) prepared: Instant,
    pub(crate) swapped: Instant,
    pub(crate) uploaded_bytes: u64,
}
#[derive(Default)]
pub(crate) struct Records {
    pub(crate) frames: Vec<FrameRecord>,
    pub(crate) completions: Vec<(u64, Instant)>,
    pub(crate) swaps: Vec<SwapRecord>,
    pub(crate) errors: Vec<String>,
}
impl Records {
    pub(crate) fn visible(&self, generation: u64) -> Option<Instant> {
        let index = self
            .frames
            .iter()
            .find(|f| f.generation >= generation)?
            .index;
        self.completions
            .iter()
            .find(|(i, _)| *i == index)
            .map(|(_, at)| *at)
    }
}

struct World {
    tick: u64,
}
fn identity(tick: u64) -> SourceTick {
    SourceTick {
        epoch: 1,
        tick,
        timeline: 1,
    }
}
impl FrameWorld for World {
    type Page = SourceTick;
    fn source(&self) -> SourceTick {
        identity(self.tick)
    }
    fn has_current_page(&self) -> bool {
        true
    }
    fn is_threaded(&self) -> bool {
        false
    }
    fn advance(&mut self, _: Duration) -> Result<WorldAdvance, FrameError> {
        self.tick += 1;
        Ok(WorldAdvance {
            tick: self.tick,
            overload: None,
            completed_ticks: 1,
            queued_ticks: 0,
            projected_changes: 0,
            projection_lost_ticks: 0,
            skipped_publications: 0,
            recovered_publication: false,
            tick_cpu: Duration::ZERO,
        })
    }
    fn publication(&mut self) -> Result<WorldPublication<Self::Page>, FrameError> {
        Ok(WorldPublication {
            id: SnapshotId {
                epoch: WorldEpoch(1),
                tick: self.tick,
            },
            timeline: 1,
            page: identity(self.tick),
        })
    }
    fn presentation_alpha_at(&self, _: Duration) -> f32 {
        1.0
    }
    fn display_phase_at(&self, _: Duration) -> f32 {
        1.0
    }
    fn is_interpolating(&self) -> bool {
        false
    }
    fn tick_due_time(&self) -> Option<Duration> {
        None
    }
}
struct Scene {
    time: f32,
}
impl SceneSource<SourceTick> for Scene {
    fn extract(&self, into: &mut RenderExtractor, page: &SourceTick) -> ExtractReport {
        into.extract(*page, [0.0; 3], |sink| {
            sink.push(0, 1, 1, [1000.0; 3], 0);
        })
    }
    fn frame(
        &self,
        _: SourceTick,
        _: f32,
        extent: [u32; 2],
        _: Option<FrameLighting>,
    ) -> Result<SceneFrame, FrameError> {
        let angle = self.time * 0.2;
        Ok(SceneFrame {
            view: SyntheticView::perspective(
                [1.9 * angle.sin(), 1.4, 1.9 * angle.cos()],
                [0.0; 3],
                [0.0, 1.0, 0.0],
                0.9,
                extent[0] as f32 / extent[1] as f32,
                0.05,
                100.0,
            )?,
            lighting: FrameLighting::Diagnostic(SyntheticLight {
                direction_to_light: [0.4, 0.8, 0.3],
                irradiance: [4.0; 3],
            }),
        })
    }
}

pub(crate) struct Viewer {
    host: FrameHost<World, Scene>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    stats: Arc<Mutex<CookedStats>>,
    records: Arc<Mutex<Records>>,
    extent: [u32; 2],
    features: wgpu::Features,
    frame: u64,
}
impl Viewer {
    pub(crate) fn with_test_device(
        config: ViewerConfig,
        adapter: &wgpu::Adapter,
        device: wgpu::Device,
        queue: wgpu::Queue,
    ) -> Self {
        let capabilities = DeviceCapabilities::capture(adapter, &device, 1);
        let mut scene = SyntheticScene::grid(1).expect("scene");
        // The prepared renderer requires a nonempty scene table. Its one diagnostic instance
        // stays outside the view; the visible geometry belongs to CookedAssets.
        scene.instances[0].current[3] = [1000.0, 1000.0, 1000.0, 1.0];
        scene.instances[0].previous = scene.instances[0].current;
        let mut host = pollster::block_on(FrameHost::new(
            &capabilities,
            &queue,
            FrameConfig {
                render_extent: config.extent,
                surface: None,
            },
            World { tick: 0 },
            Scene { time: 0.0 },
            scene,
            1.0,
        ))
        .expect("host");
        let mut cooked = CookedConfig::new(
            config.store,
            vec![MeshPlacement::new("meshes/terrain", "textures/checker")],
        );
        cooked.volume = Some(VolumePlacement {
            name: "cloud".into(),
            transform: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.5, 0.0, 0.0, 1.0],
            ],
            extinction: 0.1,
            steps: 16,
            light_steps: 8,
        });
        cooked.poll_interval = Duration::from_millis(1);
        cooked.upload_budget = config.upload_chunk as u64;
        let assets = CookedAssets::new(cooked).expect("assets");
        let stats = assets.stats();
        host.set_scene_contribution(Box::new(assets))
            .expect("attach");
        let features = device.features();
        Self {
            host,
            device,
            queue,
            stats,
            records: Arc::new(Mutex::new(Records::default())),
            extent: config.extent,
            features,
            frame: 0,
        }
    }
    fn sync(&self) {
        let stats = self.stats.lock().expect("stats");
        let mut records = self.records.lock().expect("records");
        records.errors.clone_from(&stats.errors);
        let seen = records.swaps.len();
        records
            .swaps
            .extend(stats.swaps[seen..].iter().map(|s| SwapRecord {
                mapped: s.mapped,
                generation: s.generation,
                head_seen: s.head_seen,
                prepared: s.staged,
                swapped: s.swapped,
                uploaded_bytes: s.uploaded_bytes,
                texture_profile: s.texture_profile.expect("texture"),
                volume_profile: s.volume_profile.expect("volume"),
            }));
    }
    pub(crate) fn generation(&self) -> u64 {
        self.stats
            .lock()
            .expect("stats")
            .swaps
            .last()
            .map_or(0, |s| s.generation)
    }
    pub(crate) fn frame(&mut self, time: f32) -> FrameRecord {
        let start = Instant::now();
        self.host.continuous_source_mut().time = time;
        self.host
            .drive(Duration::from_secs_f64(self.frame as f64 / 120.0), None)
            .expect("frame");
        self.sync();
        let frame = FrameRecord {
            index: self.frame,
            generation: self.generation(),
            cpu: start.elapsed(),
            submitted: Instant::now(),
        };
        self.frame += 1;
        let records = Arc::clone(&self.records);
        self.queue.on_submitted_work_done(move || {
            records
                .lock()
                .expect("records")
                .completions
                .push((frame.index, Instant::now()));
        });
        self.records.lock().expect("records").frames.push(frame);
        frame
    }
    pub(crate) fn records(&self) -> Arc<Mutex<Records>> {
        self.sync();
        Arc::clone(&self.records)
    }
    pub(crate) fn settle(&self) {
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(10)),
            })
            .expect("GPU completion");
    }
    pub(crate) fn extent(&self) -> [u32; 2] {
        self.extent
    }
    pub(crate) fn texture_profiles(&self) -> Vec<Profile> {
        thrum_asset_gpu::texture_profiles(self.features)
    }
    pub(crate) fn volume_profiles(&self) -> Vec<Profile> {
        thrum_asset_gpu::volume_profiles(self.features)
    }
    pub(crate) fn capture(&mut self, time: f32) -> Vec<u8> {
        self.frame(time);
        self.host.capture_pixels().expect("capture")
    }
}
