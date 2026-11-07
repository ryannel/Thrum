//! Game adapter for the engine-owned autonomous world lane.

use std::time::{Duration, Instant};

use thrum_core::sim::{
    ClockedWorld, WorldLane, WorldLaneControls, WorldLaneFault, WorldLifecycle, WorldLifecycleReply,
};

use crate::simulation::{PublishedPage, SimError};

use super::world::{GameWorld, WorldAdvance, WorldDriver};

/// Validated local controls for the resident world. Discrete game actions use
/// the simulation's admitted-input path instead of this latest-state mailbox.
#[derive(Clone)]
pub struct WorldControls(WorldLaneControls<ClockedWorld<GameWorld>>);

impl WorldControls {
    /// Request a native world transition without waiting for its completion.
    /// One designated caller must consume replies before another request enters.
    /// # Errors
    /// Returns `Busy` while a request or unconsumed reply occupies the slot,
    /// or the world's retained fault after its owner stops.
    pub fn request_lifecycle(&self, request: WorldLifecycle) -> Result<(), WorldLaneFault> {
        self.0.request_lifecycle(request)
    }

    /// Consume the requested transition's immutable authority acknowledgement.
    /// `None` means it is still pending; enqueueing alone is not completion.
    /// # Errors
    /// Reports a refused transition or a failed/stopped world owner.
    pub fn poll_lifecycle_reply(&self) -> Result<Option<WorldLifecycleReply>, WorldLaneFault> {
        self.0.poll_lifecycle_reply()
    }

    /// Replace continuous local axes for the next world update.
    /// # Errors
    /// Refuses invalid axes or a stopped simulation owner.
    pub fn set_controls(&self, axes: [f32; 3]) -> Result<(), String> {
        self.0.set(axes).map_err(|error| error.to_string())
    }

    /// Observe world failure without scheduling a render callback.
    /// # Errors
    /// Reports a failed owner, fixed-clock overload or synchronization fault.
    pub fn check_health(&self) -> Result<u64, WorldLaneFault> {
        self.0.check_health()
    }
}

pub(super) struct WorldClient(WorldLane<ClockedWorld<GameWorld>>);

impl WorldClient {
    pub(super) fn start(
        driver: WorldDriver,
        origin: Instant,
        wake: impl Fn() + Send + 'static,
    ) -> Result<(Self, WorldControls), String> {
        let initial_controls = driver.held_controls();
        let (lane, controls) = WorldLane::start(driver.world, initial_controls, origin, wake)
            .map_err(|error| error.to_string())?;
        Ok((Self(lane), WorldControls(controls)))
    }

    pub(super) fn poll(&mut self, now: Duration) -> Result<WorldAdvance, String> {
        self.0.poll(now).map_err(|error| error.to_string())
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
