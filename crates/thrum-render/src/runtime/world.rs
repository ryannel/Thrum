use std::time::Duration;

use thrum_core::sim::{DrivenWorld, WorldAdvance, WorldDriver, WorldPublication};

use crate::extract::SourceTick;

use super::FrameError;

/// A world observer backed by one authoritative clock or its resident lane.
pub trait FrameWorld {
    /// Immutable committed page lease.
    type Page;
    /// Latest committed authority identity, possibly newer than its publication.
    fn source(&self) -> SourceTick;
    /// Authoritative fixed timestep; override for a non-default world recipe.
    fn tick_duration(&self) -> Duration {
        Duration::from_secs_f64(1.0 / 60.0)
    }
    /// Whether the current committed endpoint has an available publication.
    fn has_current_page(&self) -> bool;
    /// Whether authoritative work runs independently of the render caller.
    fn is_threaded(&self) -> bool;
    /// Advance inline authority or observe resident authority at monotonic time.
    /// # Errors
    /// Returns an authoritative clock, game or lane failure.
    fn advance(&mut self, now: Duration) -> Result<WorldAdvance, FrameError>;
    /// Admit work for a frame. Inline driven worlds may consume the display lead.
    /// # Errors
    /// Returns an authoritative clock, game or lane failure.
    fn advance_for_display(
        &mut self,
        now: Duration,
        _lead: Duration,
    ) -> Result<WorldAdvance, FrameError> {
        self.advance(now)
    }
    /// Lease a page with its actual committed identity.
    /// # Errors
    /// Returns a publication failure without changing authority.
    fn publication(&mut self) -> Result<WorldPublication<Self::Page>, FrameError>;
    /// Sample a display target without advancing authority.
    fn presentation_alpha_at(&self, lead: Duration) -> f32;
    /// Phase at the same consumed target, taken directly from the world's clock.
    fn display_phase_at(&self, lead: Duration) -> f32;
    /// Whether this world's retained clock sample consumes a target.
    fn is_interpolating(&self) -> bool;
    /// Host due time of the retained tick; `None` for endpoint-only clocks.
    fn tick_due_time(&self) -> Option<Duration>;
}

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

    fn advance_for_display(
        &mut self,
        now: Duration,
        lead: Duration,
    ) -> Result<WorldAdvance, FrameError> {
        self.advance_for_display(now, lead)
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
