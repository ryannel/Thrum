//! Static scene fixture on the public frame host, with caller-authored weather.
use std::time::Duration;
use thrum_clouds::{Shade, WeatherInput};
use thrum_core::{
    WorldEpoch,
    sim::{SnapshotId, WorldAdvance, WorldPublication},
};
use thrum_render::{
    atmosphere::{AtmosphereFrame, SunOpticalDepth},
    capabilities::DeviceCapabilities,
    extract::{ExtractReport, RenderExtractor, SourceTick},
    runtime::{FrameConfig, FrameHost, FrameWorld, SceneFrame, SceneSource},
    synthetic::{FrameLighting, SyntheticScene, SyntheticView},
};

pub(super) struct World;
fn tick() -> SourceTick {
    SourceTick {
        epoch: 1,
        tick: 0,
        timeline: 1,
    }
}
impl FrameWorld for World {
    type Page = SourceTick;
    fn source(&self) -> SourceTick {
        tick()
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
            page: tick(),
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
pub(super) struct Source {
    view: SyntheticView,
    light: FrameLighting,
}
impl SceneSource<SourceTick> for Source {
    fn extract(&self, into: &mut RenderExtractor, page: &SourceTick) -> ExtractReport {
        into.extract(*page, [0.0; 3], |sink| {
            sink.push(0, 1, 1, [0.0; 3], 0);
        })
    }
    fn frame(
        &self,
        _: SourceTick,
        _: f32,
        _: [u32; 2],
        light: Option<FrameLighting>,
    ) -> Result<SceneFrame, String> {
        Ok(SceneFrame {
            view: self.view,
            lighting: light.unwrap_or(self.light),
        })
    }
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

pub(super) async fn prepare(
    caps: &DeviceCapabilities,
    queue: &wgpu::Queue,
    ground: u32,
    config: FrameConfig,
) -> FrameHost<World, Source> {
    let mut scene = SyntheticScene::grid(1).expect("fixture scene");
    scene.view = SyntheticView::perspective(
        [0.0, 40.0, 0.0],
        [0.0, 100.0, -300.0],
        [0.0, 1.0, 0.0],
        1.0,
        1.0,
        0.1,
        200_000.0,
    )
    .expect("camera");
    scene.sun_optical_depth =
        Some(SunOpticalDepth::gpu_target([ground, ground], 1).expect("ground target"));
    let source = Source {
        view: scene.view,
        light: FrameLighting::Atmosphere(
            AtmosphereFrame::new([0.0, 0.866_025_4, 0.5], 100_000.0, [1.0; 3], 40.0, 1)
                .expect("physical light"),
        ),
    };
    Box::pin(FrameHost::new(
        caps, queue, config, World, source, scene, 1.0,
    ))
    .await
    .expect("public frame host")
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
