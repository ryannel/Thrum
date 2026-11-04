use crate::{ActorId, Origin, PeerId};
use smallvec::SmallVec;

/// One connection's continuous controls and discrete actions for one fixed tick.
/// `AXES` is game-selected; axes are finite normalized values in [-1, 1].
/// A new sequence is consumed only after the complete frame passes admission.
#[derive(Clone, Debug, PartialEq)]
pub struct InputFrame<A, const AXES: usize = 3> {
    /// Connection, generation and monotonic sequence.
    pub origin: Origin,
    /// Controlled actor; authority registration must agree.
    pub actor: ActorId,
    /// Exact intended authority tick. The A1 fixture accepts only its next tick.
    pub tick: u64,
    /// Held controls. Missing subsequent frames retain axes, never actions.
    pub axes: [f32; AXES],
    /// One-shot game actions; order within the frame is significant.
    pub actions: SmallVec<[A; 8]>,
}

impl<A, const AXES: usize> InputFrame<A, AXES> {
    /// Construct a frame with an empty inline action buffer.
    pub fn new(origin: Origin, actor: ActorId, tick: u64, axes: [f32; AXES]) -> Self {
        Self {
            origin,
            actor,
            tick,
            axes,
            actions: SmallVec::new(),
        }
    }

    /// Check normalized axes without copying or allocating.
    ///
    /// # Errors
    ///
    /// A movement axis is nonfinite or outside the normalized range [-1, 1].
    pub fn validate_axes(&self) -> Result<(), FrameError> {
        if self
            .axes
            .iter()
            .any(|axis| !axis.is_finite() || axis.abs() > 1.0)
        {
            Err(FrameError::InvalidAxis)
        } else {
            Ok(())
        }
    }
}

/// Authenticated peer-to-actor binding, independent of a game's component type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ControlBinding {
    /// Controlling peer identity.
    pub peer: PeerId,
    /// Peer connection generation.
    pub connection_epoch: u64,
    /// Controlled actor identity.
    pub actor: ActorId,
}

/// Input admission failure. Refusal changes neither sequence nor held controls.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameError {
    /// A normalized axis is nonfinite or outside [-1, 1].
    InvalidAxis,
    /// An action carries nonfinite numeric input.
    InvalidAction,
    /// The peer is not registered for this authority.
    UnknownPeer,
    /// Connection generation does not match registration.
    WrongConnection,
    /// Controlled actor does not match registration.
    WrongActor,
    /// The frame is late or farther ahead than the next fixed tick.
    WrongTick,
    /// Sequence is not strictly greater than the last admitted sequence.
    StaleSequence,
    /// This peer already has a complete admitted frame for the next tick.
    PendingFrame,
    /// Every publication page is leased, so another due tick cannot commit.
    PublicationBusy,
    /// The configured per-frame action count was exceeded.
    TooManyActions,
    /// The registered controlled entity is stale or its ownership changed.
    InvalidControl,
}
