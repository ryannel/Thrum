//! Authored valley preparation and submitted frame helpers for the interface proof.

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use thrum_cook::{CookConfig, Cooker, Durability, Quality};
use thrum_render::{
    capabilities::DeviceCapabilities,
    present::SurfaceProfile,
    runtime::{CaptureLimits, FrameConfig, FrameHost, FrameSample, SubmittedImage, SurfaceOutput},
    synthetic::{CullMode, Reconstruction, RenderScale},
};
use v1_game::{GameHost, GameScene, GameSetup, GameWorld};

pub(crate) type Host = FrameHost<GameWorld, GameScene>;

pub(crate) fn cook(root: &Path) -> PathBuf {
    let store = root.join("store");
    let mut cooker = Cooker::open(CookConfig {
        source: Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/showcase"),
        store: store.clone(),
        profiles: vec![thrum_asset::Profile::Portable],
        quality: Quality::Fast,
        durability: Durability::Process,
    })
    .expect("open the authored valley cooker");
    let report = cooker.cook_all().expect("cook the authored valley");
    assert!(report.failed.is_empty(), "{:?}", report.failed);
    store
}

pub(crate) fn host(
    device: &wgpu::Device,
    caps: &DeviceCapabilities,
    queue: &wgpu::Queue,
    surface: SurfaceProfile,
    store: &Path,
    interface: bool,
) -> Host {
    let setup = GameSetup {
        interface_scale: interface.then_some(1),
        reconstruction: Reconstruction::Taau,
        camera_culling: CullMode::Frustum,
        ..GameSetup::default()
    };
    let mut host = pollster::block_on(setup.prepare(
        device,
        queue,
        caps,
        FrameConfig {
            render_extent: surface.extent,
            surface: Some(surface),
        },
    ))
    .expect("prepare the V1 host");
    let (camera, assets) = v1_game::prepare_cooked_valley(store.to_owned().into())
        .expect("load authored valley placements");
    let stats = assets.stats();
    host.set_game_contribution(Box::new(assets))
        .expect("attach authored valley");
    host.set_scene_camera(camera)
        .expect("install the authored valley camera");
    host.world_mut()
        .pause()
        .expect("pause during content residency");
    thrum_asset_gpu::await_drawn(&mut host, &stats, thrum_asset_gpu::Drawn::Generation(1))
        .expect("valley placements drawn");
    host.world_mut()
        .resume(Duration::ZERO)
        .expect("resume deterministic game time");
    assert!(
        host.resolved_reconstructor().is_some(),
        "fixture must resolve TAAU"
    );
    host.set_render_scale(RenderScale::Half)
        .expect("prepare reduced render extent");
    host.set_display_headroom(8.0);
    host.enable_submitted_capture_with_limits(
        1,
        CaptureLimits {
            staging_slots: 1,
            cpu_buffers: 1,
            byte_allowance: 2 * 1024 * 1024,
        },
    )
    .expect("capture the actual submitted display encoding");
    host
}

pub(crate) fn drawable(device: &wgpu::Device, surface: SurfaceProfile) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("interface valley drawable"),
            size: wgpu::Extent3d {
                width: surface.extent[0],
                height: surface.extent[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: surface.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

pub(crate) fn frame(
    host: &mut Host,
    surface: &SurfaceOutput<'_>,
    time: Duration,
) -> (FrameSample, SubmittedImage) {
    let report = host
        .drive(time, Some(surface))
        .expect("submit valley scene");
    assert!(report.submitted && report.surface_presented);
    let mut sample = None;
    let image =
        thrum_test_support::wait::until("valley interface tap", Duration::from_secs(5), || {
            if let Some(completed) = host.poll_completed().expect("complete valley frame") {
                sample = Some(completed);
            }
            host.try_submitted_image().expect("read valley output")
        });
    let sample = sample.unwrap_or_else(|| {
        thrum_test_support::wait::until("valley interface sample", Duration::from_secs(5), || {
            host.poll_completed().expect("complete valley frame")
        })
    });
    (sample, image)
}
