use std::time::Duration;
use thrum_core::{
    WorldEpoch,
    sim::{SnapshotId, WorldAdvance, WorldPublication},
};
use thrum_render::{
    atmosphere::{AtmosphereFrame, SunOpticalDepth},
    capabilities::DeviceCapabilities,
    extract::{ExtractReport, RenderExtractor, SourceTick},
    runtime::{FrameConfig, FrameError, FrameHost, FrameWorld, SceneFrame, SceneSource},
    synthetic::{FrameLighting, SyntheticScene, SyntheticView},
};

pub(crate) struct World(u64);
fn tick(tick: u64) -> SourceTick {
    SourceTick {
        epoch: 1,
        tick,
        timeline: 1,
    }
}
impl FrameWorld for World {
    type Page = SourceTick;
    fn source(&self) -> SourceTick {
        tick(self.0)
    }
    fn has_current_page(&self) -> bool {
        true
    }
    fn is_threaded(&self) -> bool {
        false
    }
    fn advance(&mut self, _: Duration) -> Result<WorldAdvance, FrameError> {
        Ok(WorldAdvance {
            tick: self.0,
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
    fn publication(&mut self) -> Result<WorldPublication<Self::Page>, FrameError> {
        Ok(WorldPublication {
            id: SnapshotId {
                epoch: WorldEpoch(1),
                tick: self.0,
            },
            timeline: 1,
            page: tick(self.0),
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
pub(crate) struct Source {
    pub(crate) view: SyntheticView,
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
    ) -> Result<SceneFrame, FrameError> {
        Ok(SceneFrame {
            view: self.view,
            lighting: light.unwrap_or(self.light),
        })
    }
}
pub(crate) async fn prepare_view(
    caps: &DeviceCapabilities,
    queue: &wgpu::Queue,
    ground: Option<u32>,
    config: FrameConfig,
    sun: [f32; 3],
    source_tick: u64,
    view: &SyntheticView,
    view_volume: bool,
) -> FrameHost<World, Source> {
    prepare_view_with_overlay(
        caps,
        queue,
        ground,
        config,
        sun,
        source_tick,
        view,
        (view_volume, false),
    )
    .await
}

pub(crate) async fn prepare_view_with_overlay(
    caps: &DeviceCapabilities,
    queue: &wgpu::Queue,
    ground: Option<u32>,
    config: FrameConfig,
    sun: [f32; 3],
    source_tick: u64,
    view: &SyntheticView,
    (view_volume, overlay): (bool, bool),
) -> FrameHost<World, Source> {
    let mut scene = SyntheticScene::grid(1).expect("fixture scene");
    scene.view = *view;
    scene.sky_overlay = overlay;
    let light = if let Some(ground) = ground {
        scene.sun_optical_depth =
            Some(SunOpticalDepth::gpu_target([ground; 2], 1).expect("ground target"));
        let light =
            AtmosphereFrame::new(sun, 100_000.0, [1.0; 3], 40.0, 1).expect("physical light");
        FrameLighting::Atmosphere(if view_volume {
            light.with_view_volume()
        } else {
            light
        })
    } else {
        FrameLighting::Diagnostic(thrum_render::synthetic::SyntheticLight::default())
    };
    let source = Source { view: *view, light };
    Box::pin(FrameHost::new(
        caps,
        queue,
        config,
        World(source_tick),
        source,
        scene,
        1.0,
    ))
    .await
    .expect("public frame host")
}
