//! Exact-byte candidate capture for native authoring source.
//!
//! Candidate bytes have no accepted-source authority until a separate durable
//! root transaction commits them. This crate never reads a running world.

mod bootstrap_json;
mod budget;
pub mod definition;
pub(crate) mod environment_json;
pub mod game_edit;
pub mod game_json;
pub mod history_json;
mod inventory;
mod json;
mod manifest;
mod manifest_json;
mod meaning;
mod path;
mod profile;
pub mod project_v2_json;
#[cfg(test)]
mod provisional_store;
#[cfg(test)]
mod provisional_store_vfs_test;
mod record;
pub mod registry;
pub mod registry_stage;
// The exact predicate is compiled privately ahead of the shared render decoder.
#[allow(dead_code)]
mod render_bound;
// Exact source geometry is compiled privately ahead of the same decoder.
#[allow(dead_code)]
mod render_geometry_exact;
pub mod schema_json;
mod semantic;
mod sqlite_store;
mod stage;

pub use bootstrap_json::{SceneRecord, parse_scene};
pub use budget::{SourceSpacePlan, StageScanPhase, StageSpacePressure};
pub use environment_json::{
    AuthoredEnvironment, AuthoredSun, EnvironmentLightIdentity, EnvironmentScene,
    parse_authored_environment, parse_environment_light_identity, parse_environment_scene,
};
pub use inventory::{Inventory, InventoryError, InventoryFile, scan_declared_tree};
pub use json::{JsonError, MAX_RECORD_BYTES, MAX_RECORD_NODES, parse_record_json};
pub use manifest::{ManifestEntry, ManifestError, RecordDescriptor, SourceRevision, TreeManifest};
pub use meaning::{
    MeaningEntry, MeaningHistoryError, TypeVersionEntry, TypeVersionHistoryError,
    compare_meaning_histories, compare_type_version_histories,
};
pub use path::{PathError, SourcePath, validate_paths};
pub use profile::{
    LimitExceeded, ObservedQuantity, SourceCapacity, SourceCapacityUnit, SourceDimension,
    first_source_profile,
};
pub use record::{RecordEnvelope, RecordError, validate_record_envelope};
pub use semantic::{BootstrapError, BootstrapProfile, BootstrapScene, validate_bootstrap_profile};
pub use sqlite_store::{
    AcceptedSourceRoot, NativeSourceStore, PinnedSource, SourceStoreError, SourceStoreErrorKind,
    SourceStoreUsage, StoreSpacePressure,
};
pub use stage::{
    StageError, StagedFile, StagedLease, StagedTree, stage_declared_tree,
    stage_declared_tree_with_plan,
};
