//! V1 game schema and authority errors. Engine crates own none of these fields
//! or actions after the G2 migration closes.

use std::num::NonZeroU32;

use thrum_core::{ActorId, Handle, PeerId, StableId, sim::ControlBinding};

/// Game-owned component binding a controlled actor to a peer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ownership {
    /// Controlling peer identity.
    pub peer: PeerId,
    /// Peer connection generation.
    pub connection_epoch: u64,
    /// Controlled actor identity.
    pub actor: ActorId,
}

impl From<Ownership> for ControlBinding {
    fn from(value: Ownership) -> Self {
        Self {
            peer: value.peer,
            connection_epoch: value.connection_epoch,
            actor: value.actor,
        }
    }
}

/// V1 world position in metres, right-handed and Y-up.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Position(pub [f64; 3]);

/// V1 movement in metres per fixed-world second.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Velocity(pub [f64; 3]);

/// Game-authored visual selector; rendering resolves it to a material handle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Visual(pub u64);

/// Game-authored saturating health value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Health(pub u16);

/// The V1 test game's discrete action schema.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SimAction {
    /// Add metres/second for this tick after applying held movement controls.
    Impulse([f64; 3]),
    /// Subtract health with saturation at zero.
    Damage(u16),
}

/// Complete V1 entity authored for seed or structural spawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ComponentEntity {
    /// Stable identity, distinct from physical slot and backend identity.
    pub id: StableId,
    /// Committed game position.
    pub position: Position,
    /// Mover component; static entities omit it when this is zero and unowned.
    pub velocity: Velocity,
    /// Health is present on every V1 entity.
    pub health: u16,
    /// Optional peer control binding.
    pub ownership: Option<Ownership>,
    /// Optional visual selector.
    pub visual: Option<u64>,
}

/// V1 resolve edits; the whole batch validates before applying any member.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command {
    /// Spawn a registered bundle after the current simulation step.
    Spawn(ComponentEntity),
    /// Retire a checked handle after the current simulation step.
    Despawn(Handle),
    /// Override the simulated position at resolve.
    SetPosition(Handle, Position),
    /// Change velocity for subsequent simulation.
    SetVelocity(Handle, Velocity),
    /// Change health at resolve.
    SetHealth(Handle, u16),
}

/// V1 game workload limits and rule constants, not an engine-wide policy.
#[derive(Clone, Copy, Debug)]
pub struct SimConfig {
    /// Fixed game tick rate; the engine clock may run at other rates.
    pub tick_hz: NonZeroU32,
    /// Reserved physical entity slots.
    pub entity_capacity: usize,
    /// Distinct peer identities retained by this game world.
    pub peer_capacity: usize,
    /// Maximum actions in one admitted frame.
    pub actions_per_frame: usize,
    /// Metres per second from a unit held movement axis.
    pub movement_speed: f64,
}

/// V1 game authority refusal. Expected errors change no committed state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SimError {
    /// Invalid V1 game configuration.
    InvalidConfig,
    /// Entity or peer capacity exhausted.
    Capacity,
    /// Stable identity already active or repeated in a command batch.
    DuplicateId,
    /// A checked handle is stale, retired or from another epoch.
    InvalidHandle,
    /// Authored or derived number is nonfinite.
    NonFinite,
    /// Two resolve edits target the same field or overlap destruction.
    ConflictingCommands,
    /// Peer binding is absent, stale or mismatched.
    InvalidOwnership,
    /// Initial seeding is closed.
    AlreadyStarted,
    /// The fixed tick cannot advance without wrapping.
    TickExhausted,
    /// Every page in the engine publication ring is leased.
    PublicationBusy,
    /// Replay capture belongs to a different authority or schema.
    InvalidSnapshot,
}

/// Per-slot V1 change stamps. Engine publication leases own their resulting
/// endpoint; these stamps are candidate bookkeeping during G2 migration.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct ChangeTicks {
    pub(crate) position: u64,
    pub(crate) velocity: u64,
    pub(crate) health: u64,
    pub(crate) structural: u64,
}

/// Copied V1 game fields behind one checked handle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ComponentView {
    /// Runtime checked handle.
    pub handle: Handle,
    /// Stable identity.
    pub id: StableId,
    /// Committed position.
    pub position: Position,
    /// Velocity; zero for static entities without the component.
    pub velocity: Velocity,
    /// Saturating health value.
    pub health: u16,
    /// Optional control binding.
    pub ownership: Option<Ownership>,
    /// Optional visual selector.
    pub visual: Option<u64>,
}

/// One complete V1 game tick, including accepted input origins.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TickOutcome<'a> {
    /// Committed tick, advanced exactly once.
    pub tick: u64,
    /// Number of live entities visited by the simulation stage.
    pub simulated_entities: usize,
    /// Changed positions to pre-existing entities.
    pub changed_positions: usize,
    /// Input frames consumed by this complete tick.
    pub committed_inputs: &'a [thrum_core::Origin],
}
