//! Registered game simulation: checked identity, schedule access, one committed
//! tick and immutable endpoint leases. Concrete fields and rules belong to games.

mod column_store;
mod input;
mod registered_world;
mod runtime;
mod schedule;
mod snapshot;
mod snapshot_ring;
mod transaction;

pub use input::{ControlBinding, FrameError, InputFrame};
pub use registered_world::{EntityWriter, RegisteredWorldError};
pub use runtime::{
    ColumnRead, ExecutionMode, Executor, GameRuntime, LiveSlot, Read2, ReadPermit,
    RegisteredSystem, RuntimeCapture, RuntimeDelta, RuntimeError, StructuralPermit, TickError,
    TickGame, WorldRead, WritePermit,
};
pub use schedule::{
    Access, AccessKey, AccessMode, ComponentKey, ComponentRegistry, ComponentSchemaId,
    RegistrationError, Schedule, ScheduleError, Stage, SystemId, SystemSpec,
};
pub use snapshot::{SnapshotId, StructuralChange};
pub use snapshot_ring::{PublicationBusy, SnapshotRing};
pub use transaction::{Edit, EditError};
