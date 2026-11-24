use std::time::Duration;
use thrum_core::{
    WorldEpoch,
    sim::{SnapshotId, WorldAdvance, WorldPublication},
};
use thrum_render::{
    atmosphere::AtmosphereFrame,
    capabilities::DeviceCapabilities,
    extract::{ExtractReport, RenderExtractor, SourceTick},
    present::SurfaceProfile,
    runtime::{FrameConfig, FrameError, FrameHost, FrameWorld, SceneFrame, SceneSource},
    synthetic::{FrameLighting, OUTPUT_FORMAT, SyntheticScene, SyntheticView},
};

pub(crate) struct Clock {
    tick: u64,
    interpolate: bool,
    now: Duration,
}

impl Clock {
    pub(crate) fn new(interpolate: bool) -> Self {
        Self {
            tick: 0,
            interpolate,
            now: Duration::ZERO,
        }
    }
}

impl FrameWorld for Clock {
    type Page = SourceTick;
    fn source(&self) -> SourceTick {
        SourceTick {
            epoch: 1,
            tick: self.tick,
            timeline: 1,
        }
    }
    fn has_current_page(&self) -> bool {
        true
    }
    fn is_threaded(&self) -> bool {
        false
    }
    fn advance(&mut self, now: Duration) -> Result<WorldAdvance, FrameError> {
        self.now = now;
        let next =
            u64::try_from(now.as_nanos() / self.tick_duration().as_nanos()).expect("short test");
        let completed_ticks = u32::try_from(next - self.tick).expect("short test");
        self.tick = next;
        Ok(WorldAdvance {
            tick: next,
            overload: None,
            completed_ticks,
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
            page: self.source(),
        })
    }
    fn presentation_alpha_at(&self, lead: Duration) -> f32 {
        if self.interpolate {
            let step = self.tick_duration().as_nanos();
            let part = self.now.as_nanos() % step + lead.as_nanos();
            part.min(step) as f32 / step as f32
        } else {
            1.0
        }
    }
    fn display_phase_at(&self, lead: Duration) -> f32 {
        self.presentation_alpha_at(lead)
    }
    fn is_interpolating(&self) -> bool {
        self.interpolate
    }
    fn tick_due_time(&self) -> Option<Duration> {
        None
    }
}

pub(crate) struct Source {
    pub(crate) view: SyntheticView,
    pub(crate) lighting: FrameLighting,
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
        _: Option<FrameLighting>,
    ) -> Result<SceneFrame, FrameError> {
        Ok(SceneFrame {
            view: self.view,
            lighting: self.lighting,
        })
    }
}

pub(crate) const QUERY_EXTENT: [u32; 2] = [96, 64];

pub(crate) fn query_host(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    caps: &DeviceCapabilities,
    clock: Clock,
    extent: [u32; 2],
) -> (FrameHost<Clock, Source>, wgpu::TextureView) {
    let target = device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("query scene output"),
            size: wgpu::Extent3d {
                width: extent[0],
                height: extent[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: OUTPUT_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default());
    let scene = SyntheticScene::grid(1).expect("fixture");
    let view = SyntheticView::perspective(
        [-8.0, 4.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        55_f32.to_radians(),
        1.5,
        0.1,
        500.0,
    )
    .expect("camera");
    let lighting = FrameLighting::Atmosphere(
        AtmosphereFrame::new([0.3, 0.93, 0.2], 100_000.0, [1.0; 3], 55.0, 1).expect("noon"),
    );
    let host = pollster::block_on(FrameHost::new(
        caps,
        queue,
        FrameConfig {
            render_extent: extent,
            surface: Some(SurfaceProfile {
                format: OUTPUT_FORMAT,
                extent,
            }),
        },
        clock,
        Source { view, lighting },
        scene,
        1.0,
    ))
    .expect("frame host");
    (host, target)
}
