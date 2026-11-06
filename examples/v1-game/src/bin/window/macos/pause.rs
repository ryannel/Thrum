//! Local P-key intent and the last acknowledgement, never a simulation clock.

use thrum_core::sim::WorldLifecycle;

#[derive(Default)]
pub(super) struct PauseIntent {
    desired_paused: bool,
    confirmed_paused: bool,
    pending: Option<WorldLifecycle>,
}

impl PauseIntent {
    pub(super) fn key(&mut self, pressed: bool, repeat: bool) -> bool {
        if !pressed || repeat {
            return false;
        }
        self.desired_paused = !self.desired_paused;
        true
    }

    pub(super) fn needs_reply(&self) -> bool {
        self.pending.is_some()
    }

    pub(super) fn next_request(&self) -> Option<WorldLifecycle> {
        if self.pending.is_some() || self.desired_paused == self.confirmed_paused {
            return None;
        }
        Some(if self.desired_paused {
            WorldLifecycle::Pause
        } else {
            WorldLifecycle::Resume
        })
    }

    pub(super) fn accepted(&mut self, request: WorldLifecycle) {
        debug_assert_eq!(self.next_request(), Some(request));
        self.pending = Some(request);
    }

    pub(super) fn acknowledge(&mut self, request: WorldLifecycle) -> Result<(), String> {
        if self.pending != Some(request) {
            return Err("unexpected world lifecycle acknowledgement".to_owned());
        }
        self.confirmed_paused = request == WorldLifecycle::Pause;
        self.pending = None;
        Ok(())
    }

    pub(super) fn label(&self) -> &'static str {
        match self.pending.or_else(|| self.next_request()) {
            Some(WorldLifecycle::Pause) => "Pausing (P)",
            Some(WorldLifecycle::Resume) => "Resuming (P)",
            None if self.confirmed_paused => "Paused (P)",
            None => "Running (P)",
        }
    }
}

#[cfg(test)]
#[path = "pause_tests.rs"]
mod tests;
