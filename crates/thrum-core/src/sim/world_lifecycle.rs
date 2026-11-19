//! Host intent, drain acknowledgement and overload accounting beside the world clock.

use std::{error::Error, fmt};

/// Explicit authority policy, independent of surface availability or occlusion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorldLifecycle {
    /// Stop admission and drain already accepted ticks before acknowledgement.
    Pause,
    /// Restart from the observed time, excluding the paused interval.
    Resume,
}

/// The host's current lifecycle status.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorldLifecycleStatus {
    /// Admitting time.
    Running,
    /// Stopping and draining admitted ticks.
    Pausing,
    /// Holding the drained endpoint.
    Paused,
    /// Restarting world time.
    Resuming,
    /// Draining after the clock refused time.
    OverloadDraining,
    /// An overload drained and the host still wants to hold.
    OverloadPaused,
    /// Restarting after an overload drained.
    OverloadResuming,
}

/// A reply arrived for a request other than the one in flight.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UnexpectedLifecycleReply;

impl fmt::Display for UnexpectedLifecycleReply {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("unexpected world lifecycle acknowledgement")
    }
}

impl Error for UnexpectedLifecycleReply {}

/// Single-owner pause and overload policy for a hosted world.
///
/// A threaded caller polls the outstanding reply before observing current world
/// health, then submits the next request. Busy submissions remain unaccepted.
/// Inline callers use the same state machine through [`Self::drive_inline`].
#[derive(Default)]
pub struct WorldLifecyclePolicy {
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

impl WorldLifecyclePolicy {
    /// Observe current clock health. Repeated observations count once per episode.
    pub fn overloaded(&mut self, refused_ticks: u64) {
        // A health read can race a not-yet-consumed Resume reply. Observe health
        // again after consuming the reply so it cannot clear a newer episode.
        if self.overload.is_none() && self.pending != Some(WorldLifecycle::Resume) {
            self.overload = Some(Overload::Draining);
            self.overload_count = self.overload_count.saturating_add(1);
            self.refused_ticks = self.refused_ticks.saturating_add(refused_ticks);
        }
    }

    /// Number of distinct overload episodes observed.
    pub fn overload_count(&self) -> u64 {
        self.overload_count
    }

    /// Total ticks refused across those episodes.
    pub fn refused_ticks(&self) -> u64 {
        self.refused_ticks
    }

    /// Set the host's pause intent and report whether it changed.
    pub fn request(&mut self, paused: bool) -> bool {
        let changed = self.desired_paused != paused;
        self.desired_paused = paused;
        changed
    }

    /// Whether the host wants the world to stay paused.
    pub fn desired(&self) -> bool {
        self.desired_paused
    }

    /// Whether the last acknowledged transition was a pause.
    pub fn confirmed_paused(&self) -> bool {
        self.confirmed_paused
    }

    /// Whether an accepted request still needs an acknowledgement.
    pub fn needs_reply(&self) -> bool {
        self.pending.is_some()
    }

    /// The next request to submit after any in-flight request is acknowledged.
    pub fn next_request(&self) -> Option<WorldLifecycle> {
        if self.pending.is_some() {
            return None;
        }
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

    /// Record successful submission of the request returned by `next_request`.
    pub fn accepted(&mut self, request: WorldLifecycle) {
        debug_assert_eq!(self.next_request(), Some(request));
        self.pending = Some(request);
    }

    /// Complete the pending request. A pause reply certifies that admitted ticks drained.
    /// # Errors
    /// Refuses a reply that does not match the pending request.
    pub fn acknowledge(&mut self, request: WorldLifecycle) -> Result<(), UnexpectedLifecycleReply> {
        if self.pending != Some(request) {
            return Err(UnexpectedLifecycleReply);
        }
        self.complete(request);
        Ok(())
    }

    fn complete(&mut self, request: WorldLifecycle) {
        self.confirmed_paused = request == WorldLifecycle::Pause;
        self.pending = None;
        self.overload = match (self.overload, request) {
            (Some(_), WorldLifecycle::Pause) => Some(Overload::Drained),
            (_, WorldLifecycle::Resume) | (None, WorldLifecycle::Pause) => None,
        };
    }

    /// Reconcile an inline world's clock with the host's intent.
    ///
    /// Pass the current admitted queue length. Apply pauses immediately to stop
    /// admission, then keep driving the world until the queue empties. Resume
    /// is applied at the caller's current observation, after that drain.
    /// # Errors
    /// Returns a failure applying the requested clock transition.
    pub fn drive_inline<E>(
        &mut self,
        queued_ticks: u32,
        mut apply: impl FnMut(WorldLifecycle) -> Result<(), E>,
    ) -> Result<(), E> {
        loop {
            if self.pending == Some(WorldLifecycle::Pause) && queued_ticks == 0 {
                self.complete(WorldLifecycle::Pause);
            }
            let Some(request) = self.next_request() else {
                return Ok(());
            };
            apply(request)?;
            self.accepted(request);
            if request == WorldLifecycle::Resume {
                self.complete(request);
            }
        }
    }

    /// Current progress for a host's status display.
    pub fn status(&self) -> WorldLifecycleStatus {
        use WorldLifecycleStatus as Status;
        if let Some(overload) = self.overload {
            return match overload {
                Overload::Draining => Status::OverloadDraining,
                Overload::Drained if self.desired_paused => Status::OverloadPaused,
                Overload::Drained => Status::OverloadResuming,
            };
        }
        match self.pending.or_else(|| self.next_request()) {
            Some(WorldLifecycle::Pause) => Status::Pausing,
            Some(WorldLifecycle::Resume) => Status::Resuming,
            None if self.confirmed_paused => Status::Paused,
            None => Status::Running,
        }
    }
}
