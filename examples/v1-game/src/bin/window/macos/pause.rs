//! Local P-key intent and the last acknowledgement, never a simulation clock.

use thrum_core::sim::WorldLifecycle;

#[derive(Default)]
pub(super) struct PauseIntent {
    desired_paused: bool,
    confirmed_paused: bool,
    pending: Option<WorldLifecycle>,
    overload: Option<Overload>,
    overload_count: u64,
    refused_ticks: u64,
}

#[derive(Clone, Copy)]
enum Overload {
    Draining,
    Drained,
}

impl PauseIntent {
    pub(super) fn overloaded(&mut self, refused_ticks: u64) {
        // Health retains the first refusal until Resume. Repeated health and
        // frame notifications describe the same episode, not new lost time.
        // A health read can race a not-yet-consumed Resume reply. Defer that
        // observation until the reply is consumed, then read health again;
        // otherwise its acknowledgement could clear a newer overload episode.
        if self.overload.is_none() && self.pending != Some(WorldLifecycle::Resume) {
            self.overload = Some(Overload::Draining);
            self.overload_count = self.overload_count.saturating_add(1);
            self.refused_ticks = self.refused_ticks.saturating_add(refused_ticks);
        }
    }

    pub(super) fn overload_count(&self) -> u64 {
        self.overload_count
    }

    pub(super) fn refused_ticks(&self) -> u64 {
        self.refused_ticks
    }

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
        if self.pending.is_some() {
            return None;
        }
        // Pause is acknowledged only after all admitted ticks drain. Recovery
        // never resumes ahead of that barrier, including after a Busy retry.
        if matches!(self.overload, Some(Overload::Draining)) {
            return Some(WorldLifecycle::Pause);
        }
        if self.desired_paused == self.confirmed_paused {
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
        self.overload = match (self.overload, request) {
            (Some(_), WorldLifecycle::Pause) => Some(Overload::Drained),
            (_, WorldLifecycle::Resume) | (None, WorldLifecycle::Pause) => None,
        };
        Ok(())
    }

    pub(super) fn label(&self) -> &'static str {
        if let Some(overload) = self.overload {
            return match overload {
                Overload::Draining => "Overloaded: draining (P)",
                Overload::Drained if self.desired_paused => "Overloaded: paused (P)",
                Overload::Drained => "Overloaded: resuming (P)",
            };
        }
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
