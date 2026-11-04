//! Provisional A1 simulation seam. This is a bounded component/schedule fixture,
//! not the selected ECS, general game API, physics solver or replacement I1 host.
//!
//! One [`Simulation`] owns its component columns. It stages a whole fixed tick,
//! validates it, then commits once; publication copies never alias those columns.
//! The storage implementation stays private for the A0 hecs/bevy/custom comparison.

mod input;
mod publication;
mod schedule;
mod storage;
mod world;

pub use input::{FrameError, InputFrame, SimAction};
pub use publication::{VisualSnapshotLease, VisualSnapshotPool};
pub use schedule::{
    Access, AccessKey, AccessMode, ComponentKind, Schedule, ScheduleError, Stage, SystemId,
    SystemSpec,
};
pub use world::{
    ChangeTicks, Command, ComponentEntity, ComponentView, Ownership, Position, SimConfig, SimError,
    Simulation, SimulationSnapshot, TickOutcome, Velocity,
};
