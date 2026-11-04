//! Bounded N1 timing owner for future W1 input frames. Staging is not gameplay admission.

use std::num::NonZeroUsize;

use thrum_core::{
    PeerId,
    sim::{FrameError, InputFrame, Ownership},
};

/// Refusal before a future frame enters the timing window.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FutureInputError {
    /// The same peer was registered twice at construction.
    DuplicatePeer,
    /// The named peer has no timing window.
    UnknownPeer,
    /// The connection incarnation is not the current binding.
    WrongConnection,
    /// The actor does not match the current binding.
    WrongActor,
    /// An axis is nonfinite or outside [-1, 1].
    InvalidAxis,
    /// The game-owned action validator refused before slot reservation.
    InvalidPayload(FrameError),
    /// The target tick has already committed.
    Late,
    /// The target tick exceeds the configured future horizon.
    TooEarly,
    /// Another frame already owns this peer and target tick.
    ConflictingFrame,
    /// A still-retained frame occupies the same physical slot.
    OccupiedSlot,
    /// A committed tick was advanced out of order.
    WrongCommit,
    /// A due frame has not been handed to the W1 admission owner.
    Undrained,
    /// A reconnect did not advance the connection incarnation.
    StaleConnection,
}

/// Idempotent result of staging an owned frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StageResult {
    /// The frame now owns its future tick slot.
    Queued,
    /// Repeated decoded frame already queued for this peer and tick.
    Duplicate,
}

struct PeerSlots<A> {
    binding: Ownership,
    frames: Box<[Option<InputFrame<A>>]>,
    last_handoff_tick: u64,
}

/// Preallocated future frames for registered peers. This owner never changes W1 state.
/// Frames are moved to `Simulation::admit_frame` only at their exact pre-tick barrier.
pub struct FutureInputWindow<A> {
    peers: Vec<PeerSlots<A>>,
    horizon: u64,
    committed_tick: u64,
}

fn same_frame<A: PartialEq>(left: &InputFrame<A>, right: &InputFrame<A>) -> bool {
    left.origin == right.origin
        && left.actor == right.actor
        && left.tick == right.tick
        && left.actions == right.actions
        && left
            .axes
            .iter()
            .zip(right.axes)
            .all(|(left, right)| left.to_bits() == right.to_bits())
}

impl<A: PartialEq> FutureInputWindow<A> {
    /// Reserve one slot per peer and future tick. Construction may allocate;
    /// staging, due handoff and tick advance do not.
    ///
    /// # Errors
    /// Returns `DuplicatePeer` for repeated peer identities.
    pub fn new(
        bindings: &[Ownership],
        future_ticks: NonZeroUsize,
        committed_tick: u64,
    ) -> Result<Self, FutureInputError> {
        let horizon = u64::try_from(future_ticks.get()).map_err(|_| FutureInputError::TooEarly)?;
        let mut peers = Vec::with_capacity(bindings.len());
        for (index, &binding) in bindings.iter().enumerate() {
            if bindings[..index]
                .iter()
                .any(|prior| prior.peer == binding.peer)
            {
                return Err(FutureInputError::DuplicatePeer);
            }
            let frames = std::iter::repeat_with(|| None)
                .take(future_ticks.get())
                .collect::<Vec<_>>()
                .into_boxed_slice();
            peers.push(PeerSlots {
                binding,
                frames,
                last_handoff_tick: committed_tick,
            });
        }
        Ok(Self {
            peers,
            horizon,
            committed_tick,
        })
    }

    /// Stage one valid future control frame. Repeated decoded frames are harmless;
    /// another value for the same peer/tick refuses without replacing the first.
    /// The caller validates game actions before reservation; W1 rechecks them
    /// and current world ownership when the frame becomes due.
    ///
    /// # Errors
    /// Returns an identity, axis, timing, conflict or retained-slot refusal.
    pub fn stage(
        &mut self,
        frame: InputFrame<A>,
        validate_actions: impl FnOnce(&InputFrame<A>) -> Result<(), FrameError>,
    ) -> Result<StageResult, FutureInputError> {
        frame
            .validate_axes()
            .map_err(|_| FutureInputError::InvalidAxis)?;
        let ahead = frame
            .tick
            .checked_sub(self.committed_tick)
            .ok_or(FutureInputError::Late)?;
        if ahead == 0 {
            return Err(FutureInputError::Late);
        }
        if ahead > self.horizon {
            return Err(FutureInputError::TooEarly);
        }
        let peer = self
            .peers
            .iter_mut()
            .find(|peer| peer.binding.peer == frame.origin.peer)
            .ok_or(FutureInputError::UnknownPeer)?;
        if frame.origin.connection_epoch != peer.binding.connection_epoch {
            return Err(FutureInputError::WrongConnection);
        }
        if frame.actor != peer.binding.actor {
            return Err(FutureInputError::WrongActor);
        }
        if frame.tick <= peer.last_handoff_tick {
            return Err(FutureInputError::Late);
        }
        validate_actions(&frame).map_err(FutureInputError::InvalidPayload)?;
        let index =
            usize::try_from(frame.tick % self.horizon).map_err(|_| FutureInputError::TooEarly)?;
        match &mut peer.frames[index] {
            slot @ None => {
                *slot = Some(frame);
                Ok(StageResult::Queued)
            }
            Some(existing) if same_frame(existing, &frame) => Ok(StageResult::Duplicate),
            Some(existing) if existing.tick == frame.tick => {
                Err(FutureInputError::ConflictingFrame)
            }
            Some(_) => Err(FutureInputError::OccupiedSlot),
        }
    }

    /// Move this peer's next-tick frame to the authoritative admission owner.
    /// An absent frame means W1 will hold its previous axes.
    /// Once the handoff is observed, another frame for that peer/tick is late,
    /// even if no frame was present. This closes the pre-tick receive boundary.
    ///
    /// # Errors
    /// Returns `UnknownPeer` for an unregistered peer.
    pub fn take_due(&mut self, peer: PeerId) -> Result<Option<InputFrame<A>>, FutureInputError> {
        let due = self
            .committed_tick
            .checked_add(1)
            .ok_or(FutureInputError::WrongCommit)?;
        let slot = self
            .peers
            .iter_mut()
            .find(|owner| owner.binding.peer == peer)
            .ok_or(FutureInputError::UnknownPeer)?;
        slot.last_handoff_tick = due;
        let index =
            usize::try_from(due % self.horizon).map_err(|_| FutureInputError::WrongCommit)?;
        if slot.frames[index]
            .as_ref()
            .is_some_and(|frame| frame.tick == due)
        {
            Ok(slot.frames[index].take())
        } else {
            Ok(None)
        }
    }

    /// Advance after W1 commits exactly one tick and every due slot has been
    /// handed off or explicitly rejected. A failed W1 tick leaves this cursor.
    ///
    /// # Errors
    /// Returns `WrongCommit` for a gap/repeat and `Undrained` for a due slot.
    pub fn commit_tick(&mut self, tick: u64) -> Result<(), FutureInputError> {
        if self.committed_tick.checked_add(1) != Some(tick) {
            return Err(FutureInputError::WrongCommit);
        }
        let index =
            usize::try_from(tick % self.horizon).map_err(|_| FutureInputError::WrongCommit)?;
        if self.peers.iter().any(|peer| {
            peer.frames[index]
                .as_ref()
                .is_some_and(|frame| frame.tick == tick)
        }) {
            return Err(FutureInputError::Undrained);
        }
        self.committed_tick = tick;
        Ok(())
    }

    /// Flush old-epoch future input before a newly registered connection begins.
    /// The caller separately updates the W1 authority binding at its barrier.
    ///
    /// # Errors
    /// Returns `UnknownPeer` or `StaleConnection` without changing the window.
    pub fn reconnect(&mut self, binding: Ownership) -> Result<(), FutureInputError> {
        let peer = self
            .peers
            .iter_mut()
            .find(|owner| owner.binding.peer == binding.peer)
            .ok_or(FutureInputError::UnknownPeer)?;
        if binding.connection_epoch <= peer.binding.connection_epoch {
            return Err(FutureInputError::StaleConnection);
        }
        peer.frames.fill_with(|| None);
        peer.binding = binding;
        Ok(())
    }

    /// Last world tick this timing owner has observed as committed.
    #[must_use]
    pub fn committed_tick(&self) -> u64 {
        self.committed_tick
    }
}
