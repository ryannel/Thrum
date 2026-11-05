//! Registered game simulation: checked identity, schedule access, one committed
//! tick and immutable endpoint leases. Concrete fields and rules belong to games.

mod column_store;
mod history;
mod input;
mod publication;
mod registered_world;
mod runtime;
mod schedule;
mod snapshot;
mod snapshot_ring;
mod transaction;
mod undo;

pub use history::{HistoryError, HistoryGame, RuntimeHistory};
pub use input::{ControlBinding, FrameError, InputFrame};
pub use publication::{
    PublicationChanges, PublicationColumn, PublicationColumns, PublicationDiff, PublicationPage,
};
pub use registered_world::{EntityWriter, RegisteredWorldError};
pub use runtime::{
    ColumnRead, ExecutionMode, Executor, GameRuntime, LiveSlot, PublicationMode, Read2, ReadPermit,
    RegisteredSystem, RuntimeCapture, RuntimeDelta, RuntimeEndpoint, RuntimeError, RuntimeUndo,
    StructuralPermit, TickError, TickGame, WorldRead, WorldTxn, WritePermit, WriteRead2,
};
pub use schedule::{
    Access, AccessKey, AccessMode, ComponentKey, ComponentRegistry, ComponentSchemaId,
    RegistrationError, Schedule, ScheduleError, Stage, SystemId, SystemSpec,
};
pub use snapshot::{SnapshotId, StructuralChange};
pub use snapshot_ring::{DirtyPage, PublicationBusy, SnapshotRing};
pub use transaction::{Edit, EditError};
