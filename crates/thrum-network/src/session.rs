//! Caller-clocked deterministic matches over the message link.

mod client;
mod clock;
mod host;
mod wire;

pub use client::{Client, ClientState, Controls};
pub use clock::{Playout, Skip, Stall};
pub use host::{Host, HostState, PeerStream};

use thrum_core::{
    ActorId, Origin, PeerId,
    sim::{ControlBinding, InputFrame, RuleSetIdentity},
};

/// Number of player slots in the first match profile.
pub const SLOTS: usize = 4;
/// Application protocol negotiated inside the stable transport envelope.
pub const PROTOCOL: u16 = 1;

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
    /// Number of recent timed frames to repeat.
    pub repeats: u8,
    /// Maximum baseline image bytes.
    pub image_limit: u32,
    /// Negotiation and terminal response deadline in milliseconds.
    pub response_ms: u32,
    /// Baseline and catch-up deadline in milliseconds.
    pub join_ms: u32,
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
            repeats: 2,
            image_limit: 4 * 1024 * 1024,
            response_ms: 4_000,
            join_ms: 60_000,
        }
    }
}

impl Settings {
    fn valid(self) -> bool {
        (1_000..=100_000).contains(&self.tick_us)
            && (1..=600).contains(&self.digest_interval)
            && (5..=600).contains(&self.staging)
            && self.backlog > self.behind
            && self.backlog <= 7_200
            && self.behind >= 4
            && (2..=32).contains(&self.catch_up)
            && (1..=8).contains(&self.repeats)
            && (1..=32 * 1024 * 1024).contains(&self.image_limit)
            && (1..=60_000).contains(&self.response_ms)
            && (self.response_ms..=120_000).contains(&self.join_ms)
    }
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
    /// when the player supplied no timed frame. Joined is a bit per slot.
    ///
    /// # Errors
    /// The world refused the candidate without committing it.
    fn step(
        &mut self,
        frames: &[InputFrame<Self::Action>; SLOTS],
        joined: u8,
    ) -> Result<(), Self::Error>;
    /// Current world digest.
    fn digest(&self) -> u64;
    /// Write the current committed world image into reusable storage.
    ///
    /// # Errors
    /// The authority world was not encodable.
    fn image(&self, destination: &mut Vec<u8>) -> Result<(), Self::Error>;
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
    /// The link closed without a final authority endpoint.
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

const MAX_LINKS: usize = 16;
const MAX_PENDING: usize = 8_192;
const MAX_COMMANDS: usize = 256;
const CHUNK: usize = 32_768;
const MATCH_EPOCH: u64 = 1;
