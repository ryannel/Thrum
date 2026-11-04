//! Provisional simulation seam. This is a bounded component/schedule fixture,
//! not the selected generic game API or physics solver.
//!
//! One [`Simulation`] owns its component columns. It stages a whole fixed tick,
//! validates it, then commits once; publication copies never alias those columns.
//! The storage implementation stays private for the A0 hecs/bevy/custom comparison.

mod controller;
mod input;
mod published;
mod registered_world;
mod runtime;
mod schedule;
mod snapshot_ring;
mod storage;
mod world;

pub use controller::{ControllerInput, ControllerRule, ControllerStepError};
pub use input::{FrameError, InputFrame, SimAction};
pub use published::{DirtySlots, PublishedPage, SnapshotId, StructuralChange};
pub use registered_world::{EntityWriter, RegisteredWorldError};
pub use runtime::{
    GameRuntime, Read2, ReadPermit, RuntimeError, StructuralPermit, TickError, TickGame,
    WorldCommit, WorldRead, Write1, WritePermit,
};
pub use schedule::{
    Access, AccessKey, AccessMode, ComponentKey, ComponentRegistry, ComponentSchemaId,
    RegistrationError, Schedule, ScheduleError, Stage, SystemId, SystemSpec,
};
pub use snapshot_ring::{PublicationBusy, SnapshotRing};
pub use world::{
    ChangeTicks, Command, ComponentEntity, ComponentView, ControllerState, Ownership, Position,
    SimConfig, SimError, Simulation, TickOutcome, Velocity,
};
