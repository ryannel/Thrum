//! Shared identities across simulation, publication and network boundaries.

/// A stable game/content object identity, distinct from a runtime handle.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StableId(pub u128);

/// The identity of a controlled actor within an admitted game.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ActorId(pub u64);

/// A peer in the current authority.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PeerId(pub u64);

/// The local identity of one constructed world, supplied fresh by the host.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct WorldEpoch(pub u128);

/// Checked transient runtime reference. A stale generation never resolves.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Handle {
    /// Constructed world identity.
    pub epoch: WorldEpoch,
    /// Slot within this world.
    pub slot: u32,
    /// Generation of that slot.
    pub generation: u64,
}

/// Exact origin of an admitted input.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Origin {
    /// Controlled connection identity in this authority.
    pub peer: PeerId,
    /// Connection generation; reconnect creates a new value.
    pub connection_epoch: u64,
    /// Monotonic input sequence within the connection epoch.
    pub sequence: u64,
}
