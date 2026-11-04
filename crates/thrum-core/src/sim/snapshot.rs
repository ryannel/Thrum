//! Identity and structural journal for one complete published endpoint.

use crate::{Handle, WorldEpoch};

/// Identity of one complete committed endpoint.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnapshotId {
    /// Constructed world generation.
    pub epoch: WorldEpoch,
    /// Committed fixed tick.
    pub tick: u64,
}

/// Structural events retain order even when a despawn and spawn reuse a slot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StructuralChange {
    /// Retired checked handle.
    Despawn(Handle),
    /// Newly live checked handle.
    Spawn(Handle),
}
