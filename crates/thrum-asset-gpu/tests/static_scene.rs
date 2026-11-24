//! A fixed world and camera for cooked-product tests.

use std::time::Duration;

use thrum_core::{
    WorldEpoch,
    sim::{SnapshotId, WorldAdvance, WorldPublication},
};
use thrum_render::{
    extract::{ExtractReport, RenderExtractor, SourceTick},
    runtime::{FrameError, FrameWorld, SceneFrame, SceneSource},
    synthetic::{FrameLighting, PhotometricEnvironment, SyntheticView},
};

const PAGE: SourceTick = SourceTick {
    epoch: 1,
    tick: 0,
    timeline: 1,
};

pub(super) struct World;
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
    fn advance(&mut self, _: Duration) -> Result<WorldAdvance, FrameError> {
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
    fn publication(&mut self) -> Result<WorldPublication<Self::Page>, FrameError> {
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

pub(super) struct Source(pub(super) SyntheticView);
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
    ) -> Result<SceneFrame, FrameError> {
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
