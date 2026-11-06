//! Game adapter for the engine-owned autonomous world lane.

use std::time::{Duration, Instant};

use thrum_core::sim::{WorldLane, WorldLaneControls};

use crate::simulation::{PublishedPage, SimError};

use super::world::{WorldAdvance, WorldDriver};

/// Validated local controls for the resident world. Discrete game actions use
/// the simulation's admitted-input path instead of this latest-state mailbox.
#[derive(Clone)]
pub struct WorldControls(WorldLaneControls<WorldDriver>);

impl WorldControls {
    /// Replace continuous local axes for the next world update.
    /// # Errors
    /// Refuses invalid axes or a stopped simulation owner.
    pub fn set_controls(&self, axes: [f32; 3]) -> Result<(), String> {
        self.0.set(axes).map_err(|error| error.to_string())
    }

    /// Observe world failure without scheduling a render callback.
    /// # Errors
    /// Reports a failed owner, fixed-clock overload or synchronization fault.
    pub fn check_health(&self) -> Result<u64, String> {
        self.0.check_health().map_err(|error| error.to_string())
    }
}

pub(super) struct WorldClient(WorldLane<WorldDriver>);

impl WorldClient {
    pub(super) fn start(
        driver: WorldDriver,
        origin: Instant,
        wake: impl Fn() + Send + 'static,
    ) -> Result<(Self, WorldControls), String> {
        let initial_controls = driver.held_controls();
        let (lane, controls) = WorldLane::start(driver, initial_controls, origin, wake)
            .map_err(|error| error.to_string())?;
        Ok((Self(lane), WorldControls(controls)))
    }

    pub(super) fn poll(&mut self, now: Duration) -> Result<WorldAdvance, String> {
        let report = self.0.poll(now).map_err(|error| error.to_string())?;
        Ok(WorldAdvance {
            tick: report.tick,
            overload: report.overload,
            completed_ticks: report.completed_ticks,
            queued_ticks: report.queued_ticks,
            projected_changes: report.projected_changes,
            projection_lost_ticks: report.projection_lost_ticks,
            skipped_publications: report.skipped_publications,
            recovered_publication: report.recovered_publication,
            tick_cpu: report.tick_cpu,
        })
    }

    pub(super) fn tick(&self) -> u64 {
        self.0.tick()
    }

    pub(super) fn epoch(&self) -> u128 {
        self.0.epoch().0
    }

    pub(super) fn timeline(&self) -> u64 {
        self.0.timeline()
    }

    pub(super) fn has_current_page(&self) -> bool {
        self.0.has_current_page()
    }

    pub(super) fn published_page(&self) -> Result<PublishedPage, SimError> {
        self.0.current_page().ok_or(SimError::PublicationBusy)
    }

    pub(super) fn presentation_alpha_at(&self, lead: Duration) -> f32 {
        self.0.presentation_alpha_at(lead)
    }

    pub(super) fn set_controls(&self, axes: [f32; 3]) -> Result<(), String> {
        self.0.set_controls(axes).map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests;
