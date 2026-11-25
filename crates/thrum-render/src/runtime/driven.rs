use std::time::Duration;

use thrum_core::sim::{DrivenWorld, WorldAdvance, WorldDriver, WorldPublication};

use crate::extract::SourceTick;

use super::{FrameError, FrameWorld};

impl<D: WorldDriver> FrameWorld for DrivenWorld<D> {
    type Page = D::Page;

    fn source(&self) -> SourceTick {
        let (id, timeline) = self.endpoint();
        SourceTick {
            epoch: id.epoch.0,
            timeline,
            tick: id.tick,
        }
    }

    fn has_current_page(&self) -> bool {
        self.has_current_page()
    }

    fn is_threaded(&self) -> bool {
        false
    }

    fn advance(&mut self, now: Duration) -> Result<WorldAdvance, FrameError> {
        self.advance(now)
            .map_err(|error| FrameError::World(error.to_string()))
    }

    fn publication(&mut self) -> Result<WorldPublication<Self::Page>, FrameError> {
        self.publication()
            .map_err(|error| FrameError::World(error.to_string()))?
            .ok_or(FrameError::InvalidState("missing driven world publication"))
    }

    fn presentation_alpha_at(&self, lead: Duration) -> f32 {
        self.presentation_alpha_at(lead)
    }

    fn display_phase_at(&self, lead: Duration) -> f32 {
        self.presentation_alpha_at(lead)
    }

    fn is_interpolating(&self) -> bool {
        true
    }

    fn tick_due_time(&self) -> Option<Duration> {
        None
    }
}
