//! Registered game simulation: checked identity, schedule access, one committed
//! tick and immutable endpoint leases. Concrete fields and rules belong to games.

mod clocked_world;
mod column_store;
mod dense_directory;
mod driven_world;
mod history;
mod input;
mod publication;
mod registered_world;
mod runtime;
mod schedule;
mod snapshot;
mod snapshot_ring;
#[cfg(test)]
mod tests;
mod transaction;
mod undo;
#[cfg(not(target_arch = "wasm32"))]
mod world_lane;
mod world_lifecycle;

pub use clocked_world::{
    ClockedWorld, ClockedWorldError, WorldAdvance, WorldGame, WorldPublication, WorldStep,
    WorldUpdate,
};
pub use driven_world::{DrivenAdvance, DrivenWorld, DrivenWorldError, WorldDriver};
pub use history::{HistoryError, HistoryGame, RuntimeHistory};
pub use input::{ControlBinding, InputFrame, InputFrameError};
pub use publication::{PublicationChanges, PublicationPage};
pub use registered_world::{ChunkedVisit, EntityWriter, RegisteredWorldError};
pub use runtime::{
    ColumnRead, ExecutionMode, Executor, GameRuntime, LiveSlot, PublicationMode, Read2, ReadPermit,
    RegisteredSystem, RuleSetIdentity, RuntimeCapture, RuntimeEndpoint, RuntimeError, RuntimeUndo,
    StructuralPermit, TickError, TickGame, WorldImageError, WorldRead, WorldTxn, WritePermit,
    WriteRead2,
};
pub use schedule::{
    Access, AccessKey, AccessMode, ComponentKey, ComponentRegistry, ComponentSchemaId,
    RegistrationError, Schedule, ScheduleError, Stage, SystemId, SystemSpec,
};
pub use snapshot::{SnapshotId, StructuralChange};
pub use snapshot_ring::{DirtyPage, PublicationBusy, SnapshotRing};
pub use transaction::{Edit, EditError, EditWriter};
#[cfg(not(target_arch = "wasm32"))]
pub use world_lane::{
    WorldLane, WorldLaneControls, WorldLaneFault, WorldLifecycleReply, WorldTask,
};
pub use world_lifecycle::{
    UnexpectedLifecycleReply, WorldLifecycle, WorldLifecyclePolicy, WorldLifecycleStatus,
};
