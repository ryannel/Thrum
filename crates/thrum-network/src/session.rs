//! Caller-clocked deterministic matches over the message link.

mod client;
mod clock;
mod host;
mod prediction;
mod wire;

pub use client::{Client, ClientState, Controls};
pub use clock::{Playout, Skip, Stall};
pub use host::{Host, HostState, PeerDisconnect, PeerStream};
pub use prediction::{
    ControllerInput, ControllerView, Correction, CorrectionCause, NoPrediction, PREDICTION_TICKS,
    PredictedController, PredictionEvent,
};

use thrum_core::{
    ActorId, Origin, PeerId,
    profile::StateBytes,
    sim::{ControlBinding, InputFrame, RuleSetIdentity},
};

/// Number of player slots in the first match profile.
pub const SLOTS: usize = 4;
/// Application protocol negotiated inside the stable transport envelope.
pub const PROTOCOL: u16 = 3;

/// Bounds and pacing sent by the authority in its welcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    /// Tick period in microseconds.
    pub tick_us: u32,
    /// Digest cadence in committed ticks.
    pub digest_interval: u16,
    /// Maximum authoring horizon in ticks.
    pub staging: u16,
    /// Maximum retained sets, on either side of the stream.
    pub backlog: u16,
    /// Backlog above which a playing client skips.
    pub behind: u16,
    /// Maximum ticks committed in one advance.
    pub catch_up: u8,
    /// Catch-up playout rate in thousandths of normal speed.
    pub playout_catch_up: u16,
    /// Slow-down playout rate in thousandths of normal speed.
    pub playout_slow_down: u16,
    /// Number of recent timed frames to repeat.
    pub repeats: u8,
    /// Maximum baseline image bytes.
    pub image_limit: u32,
    /// Negotiation and terminal response deadline in milliseconds.
    pub response_ms: u32,
    /// Baseline and catch-up deadline in milliseconds.
    pub join_ms: u32,
    /// Host ticks allowed above the playing backlog threshold.
    pub catch_up_deadline: u16,
    /// Ticks without a fresh timed frame before axes become neutral.
    pub held_expiry: u16,
    /// Ticks without a fresh timed frame before the link is closed.
    pub absence: u16,
    /// Ticks an away slot remains reserved for its token.
    pub grace: u16,
    /// Maximum repairs on one connection.
    pub repairs: u8,
    /// Maximum admitted, uncommitted commands per peer.
    pub command_cap: u16,
    /// Maximum simultaneous transport connections, including pending handshakes.
    pub link_cap: u8,
    /// Number of digest endpoints retained for diagnosis.
    pub digest_history: u16,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            tick_us: 16_667,
            digest_interval: 30,
            staging: 120,
            backlog: 3_600,
            behind: 30,
            catch_up: 8,
            playout_catch_up: 1_100,
            playout_slow_down: 850,
            repeats: 2,
            image_limit: 4 * 1024 * 1024,
            response_ms: 4_000,
            join_ms: 60_000,
            catch_up_deadline: 600,
            held_expiry: 30,
            absence: 600,
            grace: 1_800,
            repairs: 3,
            command_cap: 256,
            link_cap: 16,
            digest_history: 128,
        }
    }
}

impl Settings {
    fn input_burst(self) -> u64 {
        u64::from(self.repeats) * 2 + u64::from(self.behind)
    }

    fn valid(self) -> bool {
        (1_000..=100_000).contains(&self.tick_us)
            && (1..=600).contains(&self.digest_interval)
            && (5..=600).contains(&self.staging)
            && self.backlog > self.behind
            && self.backlog <= 7_200
            && self.behind >= 4
            && (2..=32).contains(&self.catch_up)
            && self.playout_catch_up >= 1_000
            && (1..=1_000).contains(&self.playout_slow_down)
            && (1..=8).contains(&self.repeats)
            && (1..=32 * 1024 * 1024).contains(&self.image_limit)
            && (1..=60_000).contains(&self.response_ms)
            && (self.response_ms..=120_000).contains(&self.join_ms)
            && self.catch_up_deadline > 0
            && self.held_expiry > 0
            && self.absence > self.held_expiry
            && self.grace > 0
            && self.repairs > 0
            && (1..=1024).contains(&self.command_cap)
            && (SLOTS..=32).contains(&usize::from(self.link_cap))
            && (1..=512).contains(&self.digest_history)
    }
}

/// Rejoin credential issued by the authority. An absent token uses the zero wire value.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PlayerToken(pub(crate) [u8; 16]);

impl PlayerToken {
    /// Restore a previously saved rejoin credential. All zeroes represent no token.
    pub const fn from_bytes(bytes: [u8; 16]) -> Option<Self> {
        if u128::from_le_bytes(bytes) == 0 {
            None
        } else {
            Some(Self(bytes))
        }
    }

    /// Save this rejoin credential for use by a future connection.
    pub const fn to_bytes(self) -> [u8; 16] {
        self.0
    }
}

/// Roster transitions committed together with the slot frames at one tick.
/// Each mask uses one bit per player slot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RosterEvents {
    /// A new player enters the match.
    pub joined: u8,
    /// A player's link was lost and its controls become neutral.
    pub away: u8,
    /// The same player returns to its slot.
    pub back: u8,
    /// A slot's character leaves the match.
    pub left: u8,
}

/// Current committed or baseline roster, by slot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Roster {
    /// Slots with a character in the world.
    pub occupied: u8,
    /// Occupied slots whose players are away.
    pub away: u8,
}

impl Roster {
    fn apply(&mut self, events: RosterEvents) {
        self.occupied = (self.occupied | events.joined) & !events.left;
        self.away = (self.away | events.away) & !(events.back | events.left);
    }
}

/// Component or resource diagnostic at a committed endpoint.
pub type DigestPart = (u128, u64);

/// A replica's report and the authority comparison retained for diagnosis.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Divergence {
    /// Transport link identifier.
    pub link: u64,
    /// Player slot on that link.
    pub slot: usize,
    /// Endpoint the client refused or found unequal.
    pub tick: u64,
    /// Authority digest, when still retained.
    pub expected: Option<u64>,
    /// Replica digest. Absent when the replica could not step.
    pub actual: Option<u64>,
    /// Component or resource identifiers with different partial digests.
    pub parts: Vec<u128>,
}

/// A lifecycle close initiated by the session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Close {
    /// No baseline or reliable delivery progress by its deadline.
    Synchronization = 4,
    /// Playing backlog exceeded the catch-up deadline.
    UnableToKeepUp = 5,
    /// No new authored timed input by the absence limit.
    Absent = 6,
    /// The connection exhausted its repair allowance.
    RepairLimit = 7,
}

/// Rule-owned world. All ticks come through the same host and replica boundary.
pub trait World {
    /// An action from the game's registered wire schema.
    type Action: Clone + PartialEq;
    /// Local authority or replica failure.
    type Error;

    /// Tick at the committed endpoint.
    fn tick(&self) -> u64;
    /// Configured deterministic rule-set identity.
    fn identity(&self) -> RuleSetIdentity;
    /// Commit one tick, atomically on error. Slot frames carry held axes even
    /// when the player supplied no timed frame. Events apply at this same tick.
    ///
    /// # Errors
    /// The world refused the candidate without committing it.
    fn step(
        &mut self,
        frames: &[InputFrame<Self::Action>; SLOTS],
        events: RosterEvents,
    ) -> Result<(), Self::Error>;
    /// Current world digest.
    fn digest(&self) -> u64;
    /// Sample the digest and optional partials together at this endpoint.
    fn digest_with_parts(&self, _partial: &mut dyn FnMut(DigestPart)) -> u64 {
        self.digest()
    }
    /// Write the current committed world image into the match's bounded storage.
    ///
    /// # Errors
    /// The authority world was not encodable.
    fn image(&self, destination: &mut StateBytes) -> Result<(), Self::Error>;
    /// Install an image atomically, checking the expected tick before publication.
    ///
    /// # Errors
    /// The image was incompatible, invalid, or belonged to a different tick.
    fn restore(&mut self, tick: u64, image: &[u8]) -> Result<(), Self::Error>;
    /// Observe the authority's pre-tick barrier for instrumentation.
    fn barrier(&mut self, _tick: u64) {}
    /// Observe the last send attempt of a tick whose set entered a link.
    fn set_offered(&mut self, _tick: u64) {}
}

/// Final authority reason sent after all earlier sets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum EndReason {
    /// The runtime refused the authority step.
    AuthorityStep = 1,
    /// The host deliberately ended the match.
    Stopped = 2,
}

/// Negotiation refusal suitable for presentation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Refusal {
    /// Protocol differs.
    Protocol = 1,
    /// Rule-set identity differs.
    RuleSet = 2,
    /// All available slots are reserved.
    Full = 3,
}

/// Identity of an authored input, scoped to a connection and lane.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    /// Timed frame sequence.
    Frame(u64),
    /// Ordered command sequence.
    Command(u64),
}

/// Result learned from a committed set or a terminal message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Disposition {
    /// Included in this committed authority tick.
    Committed(u64),
    /// The authority held this frame's target tick or committed another frame.
    Refused,
    /// The authority ended the match before including this input.
    NotCommitted,
    /// The link closed or resynchronized before the result was observed.
    Uncertain,
}

/// One resolved local input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResultEvent {
    /// Authored input.
    pub input: Input,
    /// Authority disposition.
    pub disposition: Disposition,
}

/// Invalid session settings or local input backpressure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionError {
    /// Match bounds are invalid.
    Settings,
    /// Connection, pending input, or result storage is full.
    Full,
    /// Input cannot be encoded in its declared schema and lane.
    Input,
    /// The session no longer accepts input or connections.
    Closed,
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for SessionError {}

fn binding(slot: usize, epoch: u64) -> ControlBinding {
    let slot = slot as u64;
    ControlBinding {
        peer: PeerId(slot),
        actor: ActorId(slot),
        connection_epoch: epoch,
    }
}

fn frame<A>(slot: usize, epoch: u64, seq: u64, tick: u64, axes: [f32; 3]) -> InputFrame<A> {
    let owner = binding(slot, epoch);
    InputFrame::new(
        Origin {
            peer: owner.peer,
            connection_epoch: epoch,
            sequence: seq,
        },
        owner.actor,
        tick,
        axes,
    )
}

const MAX_PENDING: usize = 8_192;
const CHUNK: usize = 32_768;
const MATCH_EPOCH: u64 = 1;

const MAX_DIGEST_PARTS: usize = 256;
