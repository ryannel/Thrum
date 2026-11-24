//! Waiting for cooked content renders without reading frames back, with one deadline and
//! error policy.

use std::path::Path;

use super::static_scene::{Source, World};
use thrum_asset_gpu::{
    AwaitDrawnError, CookedAssets, CookedConfig, Drawn, MeshPlacement, await_drawn,
};
use thrum_render::{
    capabilities::DeviceCapabilities,
    runtime::{FrameConfig, FrameHost},
    synthetic::{SyntheticScene, SyntheticView},
};

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
    let Some(thrum_test_support::Opened {
        gpu, device, queue, ..
    }) = thrum_test_support::open(
        "await drawn",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    )
    else {
        return;
    };
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
    let Some(thrum_test_support::Opened {
        gpu, device, queue, ..
    }) = thrum_test_support::open(
        "await drawn failure",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    )
    else {
        return;
    };
    let caps = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
    // A HEAD whose manifest is missing is a loader failure.
    let store = thrum_test_support::Scratch::new("await-drawn");
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/HEAD"),
        store.join("HEAD"),
    )
    .expect("HEAD");
    let (mut host, stats) = host(&caps, &queue, store.path());
    match await_drawn(&mut host, &stats, Drawn::Any) {
        Err(AwaitDrawnError::Content(errors)) => assert_ne!(errors, Vec::<String>::new()),
        other => panic!("expected a content failure, got {other:?}"),
    }
    assert_eq!(host.endpoint_readbacks(), 0);
    drop(host);
}
