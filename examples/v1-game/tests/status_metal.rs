use std::time::Duration;

use thrum_render::runtime::{FrameHost, FrameWorld, SceneSource, SubmittedImage, SurfaceOutput};
use thrum_test_support::{Opened, Profile, Restriction};

pub(crate) const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;

pub(crate) fn open() -> Opened {
    let mut gpu = thrum_test_support::open(
        "status display",
        Some(wgpu::Backend::Metal),
        Profile::Native,
        &Restriction::default(),
    )
    .expect("Mac Metal device for this check");
    assert_eq!(gpu.capabilities.adapter().backend, wgpu::Backend::Metal);
    gpu.capabilities = gpu
        .capabilities
        .with_recipe_override("portable")
        .expect("portable Metal recipe");
    gpu
}

pub(crate) fn drawable(gpu: &Opened, [width, height]: [u32; 2]) -> wgpu::TextureView {
    gpu.device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("status test drawable"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

pub(crate) fn submit<W: FrameWorld, V: SceneSource<W::Page>>(
    host: &mut FrameHost<W, V>,
    drawable: &wgpu::TextureView,
    extent: [u32; 2],
    clock: Duration,
) -> SubmittedImage {
    let report = host
        .advance(clock)
        .expect("frame")
        .encode_scene()
        .expect("encode")
        .submit(Some(&SurfaceOutput {
            view: drawable,
            extent,
        }))
        .expect("submit");
    assert!(report.submitted);
    thrum_test_support::wait::until(
        "read submitted status display",
        Duration::from_secs(8),
        || {
            while host.poll_completed().expect("poll").is_some() {}
            host.try_submitted_image().expect("status image")
        },
    )
}
