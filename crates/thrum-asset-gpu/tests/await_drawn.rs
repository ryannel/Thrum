//! Waiting for cooked content renders without reading frames back, with one deadline and
//! error policy.

use std::{path::Path, time::Duration};

use thrum_asset_gpu::{
    AwaitDrawnError, CookedAssets, CookedConfig, Drawn, MeshPlacement, await_drawn,
};
use thrum_core::{
    WorldEpoch,
    sim::{SnapshotId, WorldAdvance, WorldPublication},
};
use thrum_render::{
    capabilities::DeviceCapabilities,
    extract::{ExtractReport, RenderExtractor, SourceTick},
    runtime::{FrameConfig, FrameHost, FrameWorld, SceneFrame, SceneSource},
    synthetic::{FrameLighting, PhotometricEnvironment, SyntheticScene, SyntheticView},
};

const PAGE: SourceTick = SourceTick {
    epoch: 1,
    tick: 0,
    timeline: 1,
};

struct World;
impl FrameWorld for World {
    type Page = SourceTick;
    fn source(&self) -> SourceTick {
        PAGE
    }
    fn has_current_page(&self) -> bool {
        true
    }
    fn is_threaded(&self) -> bool {
        false
    }
    fn advance(&mut self, _: Duration) -> Result<WorldAdvance, String> {
        Ok(WorldAdvance {
            tick: 0,
            overload: None,
            completed_ticks: 0,
            queued_ticks: 0,
            projected_changes: 0,
            projection_lost_ticks: 0,
            skipped_publications: 0,
            recovered_publication: false,
            tick_cpu: Duration::ZERO,
        })
    }
    fn publication(&mut self) -> Result<WorldPublication<Self::Page>, String> {
        Ok(WorldPublication {
            id: SnapshotId {
                epoch: WorldEpoch(1),
                tick: 0,
            },
            timeline: 1,
            page: PAGE,
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

struct Source(SyntheticView);
impl SceneSource<SourceTick> for Source {
    fn extract(&self, into: &mut RenderExtractor, page: &SourceTick) -> ExtractReport {
        into.extract(*page, [0.0; 3], |_| {})
    }
    fn frame(
        &self,
        _: SourceTick,
        _: f32,
        _: [u32; 2],
        _: Option<FrameLighting>,
    ) -> Result<SceneFrame, String> {
        Ok(SceneFrame {
            view: self.0,
            lighting: FrameLighting::Photometric(PhotometricEnvironment {
                toward_sun: [0.0, 0.8, 0.6],
                sun_lux: 50_000.0,
                sun_colour: [1.0; 3],
                ambient_lux: [1500.0; 3],
                background_radiance: [0.1; 3],
            }),
        })
    }
}

fn host(
    caps: &DeviceCapabilities,
    queue: &wgpu::Queue,
    store: &Path,
) -> (
    FrameHost<World, Source>,
    std::sync::Arc<std::sync::Mutex<thrum_asset_gpu::CookedStats>>,
) {
    let view = SyntheticView::perspective(
        [0.4, 1.7, 5.0],
        [0.0, 0.75, 0.0],
        [0.0, 1.0, 0.0],
        0.92,
        1.0,
        0.1,
        4000.0,
    )
    .expect("camera");
    let mut scene = SyntheticScene::grid(1).expect("scene");
    scene.view = view;
    let mut host = pollster::block_on(FrameHost::new(
        caps,
        queue,
        FrameConfig {
            render_extent: [64; 2],
            surface: None,
        },
        World,
        Source(view),
        scene,
        1.0,
    ))
    .expect("host");
    let placement = MeshPlacement::new("meshes/rock", "textures/white").on_ground();
    let cooked =
        CookedAssets::new(CookedConfig::new(store.to_path_buf(), vec![placement])).expect("cooked");
    let stats = cooked.stats();
    host.set_scene_contribution(Box::new(cooked))
        .expect("attach");
    (host, stats)
}

#[test]
fn waiting_draws_frames_until_content_is_drawn_and_reads_nothing_back() {
    let Some(gpu) = thrum_test_support::gpu("await drawn", Some(wgpu::Backend::Metal)) else {
        return;
    };
    let (device, queue) = gpu.device(
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    );
    let caps = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
    let (mut host, stats) = host(
        &caps,
        &queue,
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data"),
    );
    let generation = await_drawn(&mut host, &stats, Drawn::Any).expect("drawn");
    let recorded = {
        let stats = stats.lock().expect("stats");
        assert_eq!(stats.drawn, Some(generation));
        assert!(stats.submitted_frames > 0, "the wait submitted no frame");
        stats.recorded_frames
    };
    assert_eq!(host.endpoint_readbacks(), 0, "waiting read a frame back");

    // Content already drawn returns at once, drawing nothing more.
    assert_eq!(
        await_drawn(&mut host, &stats, Drawn::Generation(generation)).expect("drawn again"),
        generation
    );
    assert_eq!(stats.lock().expect("stats").recorded_frames, recorded);

    // The one capture a caller wants afterwards is its only readback.
    let pixels = host.capture_pixels().expect("capture");
    assert_eq!(pixels.len(), 64 * 64 * 4);
    assert_eq!(host.endpoint_readbacks(), 1);
}

#[test]
fn a_loader_failure_ends_the_wait() {
    let Some(gpu) = thrum_test_support::gpu("await drawn failure", Some(wgpu::Backend::Metal))
    else {
        return;
    };
    let (device, queue) = gpu.device(
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    );
    let caps = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
    // A HEAD whose manifest is missing is a loader failure.
    let store = std::env::temp_dir().join(format!("thrum-await-drawn-{}", std::process::id()));
    std::fs::create_dir_all(&store).expect("store");
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/HEAD"),
        store.join("HEAD"),
    )
    .expect("HEAD");
    let (mut host, stats) = host(&caps, &queue, &store);
    match await_drawn(&mut host, &stats, Drawn::Any) {
        Err(AwaitDrawnError::Content(errors)) => assert_ne!(errors, Vec::<String>::new()),
        other => panic!("expected a content failure, got {other:?}"),
    }
    assert_eq!(host.endpoint_readbacks(), 0);
    drop(host);
    std::fs::remove_dir_all(&store).ok();
}
