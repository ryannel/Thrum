//! Local single-player foreground policy for the world lane's lifecycle slot.
//!
//! Background pauses world time; foreground resumes it. A fixed-clock overload
//! pauses, drains and resumes from a fresh origin. The shell is the one
//! designated caller: it sends at most one request and consumes its reply
//! before the next. This mirrors the macOS shell's pause intent.

use thrum_core::sim::WorldLifecycle;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Overload {
    Draining,
    Drained,
}

#[derive(Default)]
pub(crate) struct WorldPolicy {
    background: bool,
    confirmed_paused: bool,
    pending: Option<WorldLifecycle>,
    overload: Option<Overload>,
    overloads: u64,
    refused_ticks: u64,
}

impl WorldPolicy {
    pub(crate) fn set_background(&mut self, background: bool) {
        self.background = background;
    }

    pub(crate) fn confirmed_paused(&self) -> bool {
        self.confirmed_paused
    }

    pub(crate) fn needs_reply(&self) -> bool {
        self.pending.is_some()
    }

    pub(crate) fn overloads(&self) -> u64 {
        self.overloads
    }

    pub(crate) fn refused_ticks(&self) -> u64 {
        self.refused_ticks
    }

    /// Health retains one refusal until Resume; count each episode once. A
    /// health read racing an unconsumed Resume reply is read again later.
    pub(crate) fn overloaded(&mut self, refused_ticks: u64) {
        if self.overload.is_none() && self.pending != Some(WorldLifecycle::Resume) {
            self.overload = Some(Overload::Draining);
            self.overloads += 1;
            self.refused_ticks = self.refused_ticks.saturating_add(refused_ticks);
        }
    }

    pub(crate) fn next_request(&self) -> Option<WorldLifecycle> {
        if self.pending.is_some() {
            return None;
        }
        if self.overload == Some(Overload::Draining) {
            return Some(WorldLifecycle::Pause);
        }
        if self.background == self.confirmed_paused {
            return None;
        }
        Some(if self.background {
            WorldLifecycle::Pause
        } else {
            WorldLifecycle::Resume
        })
    }

    pub(crate) fn accepted(&mut self, request: WorldLifecycle) {
        debug_assert_eq!(self.next_request(), Some(request));
        self.pending = Some(request);
    }

    /// # Errors
    /// Refuses a reply that does not match the one pending request.
    pub(crate) fn acknowledge(&mut self, request: WorldLifecycle) -> Result<(), String> {
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
}

#[cfg(test)]
mod tests {
    use thrum_core::sim::WorldLifecycle::{Pause, Resume};

    use super::WorldPolicy;

    fn exchange(policy: &mut WorldPolicy) -> Option<thrum_core::sim::WorldLifecycle> {
        let request = policy.next_request()?;
        assert!(!policy.needs_reply());
        policy.accepted(request);
        assert!(policy.needs_reply(), "the caller polls only while a reply is due");
        policy.acknowledge(request).expect("matching reply");
        assert!(!policy.needs_reply());
        Some(request)
    }

    #[test]
    fn background_pauses_and_foreground_resumes_in_order() {
        let mut policy = WorldPolicy::default();
        assert_eq!(policy.next_request(), None);
        policy.set_background(true);
        assert_eq!(policy.next_request(), Some(Pause));
        policy.accepted(Pause);
        // Foreground before the pause reply waits for it; no overlapping request.
        policy.set_background(false);
        assert_eq!(policy.next_request(), None);
        policy.acknowledge(Pause).expect("pause reply");
        assert!(policy.confirmed_paused());
        assert_eq!(exchange(&mut policy), Some(Resume));
        assert!(!policy.confirmed_paused());
        assert_eq!(policy.next_request(), None);
    }

    #[test]
    fn a_mismatched_reply_is_refused() {
        let mut policy = WorldPolicy::default();
        policy.set_background(true);
        policy.accepted(Pause);
        assert!(policy.acknowledge(Resume).is_err());
    }

    #[test]
    fn overload_drains_then_resumes_once_per_episode() {
        let mut policy = WorldPolicy::default();
        policy.overloaded(9);
        policy.overloaded(9);
        assert_eq!((policy.overloads(), policy.refused_ticks()), (1, 9));
        assert_eq!(exchange(&mut policy), Some(Pause));
        assert_eq!(exchange(&mut policy), Some(Resume));
        assert_eq!(policy.next_request(), None);
    }

    #[test]
    fn overload_while_backgrounded_stays_paused() {
        let mut policy = WorldPolicy::default();
        policy.set_background(true);
        policy.overloaded(3);
        assert_eq!(exchange(&mut policy), Some(Pause));
        assert_eq!(policy.next_request(), None);
        policy.set_background(false);
        assert_eq!(exchange(&mut policy), Some(Resume));
    }
}
