//! Static scene fixture on the public frame host, with caller-authored weather.
use crate::scene_fixture::{self, Source, World};
use std::time::Duration;
use thrum_clouds::{Shade, WeatherInput};
use thrum_render::{
    capabilities::DeviceCapabilities,
    runtime::{FrameConfig, FrameHost},
};

pub(super) async fn prepare(
    caps: &DeviceCapabilities,
    queue: &wgpu::Queue,
    ground: u32,
    config: FrameConfig,
) -> FrameHost<World, Source> {
    let view = thrum_render::synthetic::SyntheticView::perspective(
        [0.0, 40.0, 0.0],
        [0.0, 100.0, -300.0],
        [0.0, 1.0, 0.0],
        1.0,
        1.0,
        0.1,
        200_000.0,
    )
    .expect("shadow scene camera");
    scene_fixture::prepare_view(
        caps,
        queue,
        Some(ground),
        config,
        [0.0, 0.866_025_4, 0.5],
        0,
        &view,
    )
    .await
}

pub(super) async fn host(
    device: &wgpu::Device,
    caps: &DeviceCapabilities,
    queue: &wgpu::Queue,
    shade: Shade,
    ground: u32,
) -> FrameHost<World, Source> {
    let format = wgpu::TextureFormat::Bgra8Unorm;
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("shadow capability presentation"),
        size: wgpu::Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let target = target.create_view(&wgpu::TextureViewDescriptor::default());
    let mut host = prepare(
        caps,
        queue,
        ground,
        FrameConfig {
            render_extent: [64, 64],
            surface: Some(thrum_render::present::SurfaceProfile {
                format,
                extent: [64, 64],
            }),
        },
    )
    .await;
    host.set_scene_contribution(Box::new(shade))
        .expect("attach clouds");
    assert!(
        host.drive(
            Duration::ZERO,
            Some(&thrum_render::runtime::SurfaceOutput {
                view: &target,
                extent: [64, 64]
            })
        )
        .expect("submit complete cloud scene")
        .submitted
    );
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("complete fixture GPU work");
    while host.poll_completed().expect("completed fixture").is_some() {}
    host
}

pub(super) fn ground(device: &wgpu::Device, queue: &wgpu::Queue, input: &WeatherInput) -> Vec<u8> {
    let (texture, _) = input.ground_product().expect("ground product");
    thrum_test_support::readback::texture(
        device,
        queue,
        &texture,
        thrum_test_support::readback::Mip::level(0),
        ..,
    )
}
