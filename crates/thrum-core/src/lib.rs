//! Portable, headless authority foundations for Thrum.
//!
//! The first executable slice owns a fixed-tick world, checked identities,
//! game-owned typed rules, immutable publication and an in-memory replica.
//! It has no renderer, transport or persistent source adapter.

mod authority;
mod clock;
mod host;
mod publication_pool;
mod replica;
mod world;

pub use authority::{
    ActionRejection, ActionResult, Admission, AdmissionError, Authority, CommittedEvent, GameRules,
    GameStateGroup, GameTick, Input, InputSchedule, Origin, PeerId, PreparedCommit,
    ResultDisposition, SnapshotWriteBounds, StepError, TickPublication,
};
pub use clock::{ClockConfig, ClockError, ClockReport, ClockState, TickClock};
pub use host::{
    GameHostCallback, HeadlessHost, HostAdmissionError, HostBudget, HostCallback, HostError,
    HostPublication, InitialOutcomes,
};
pub use publication_pool::{
    ClosedPublicationValue, ClosedValueError, ClosedValueLimits, PublicationBytes,
    PublicationCapacity, PublicationCharge, PublicationPool, PublicationRecord,
    PublicationRecordError, PublicationReservation,
};
pub use replica::{AdmittedGroup, Replica, ReplicaError, StateGroup};
pub use world::{
    ActorId, AuthorityId, EntityChange, EntityRecord, Handle, MatchId, PreparedEdits, SessionId,
    Snapshot, StableId, World, WorldEdit, WorldEpoch, WorldError, WorldPublication,
    WorldPublicationError, WorldWork,
};
