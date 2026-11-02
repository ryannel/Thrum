//! Exact-byte candidate capture for native authoring source.
//!
//! Candidate bytes have no accepted-source authority until a separate durable
//! root transaction commits them. This crate never reads a running world.

mod bootstrap_json;
mod budget;
mod inventory;
mod json;
mod manifest;
mod meaning;
mod path;
mod record;
mod semantic;
mod stage;

pub use inventory::{Inventory, InventoryError, InventoryFile, scan_declared_tree};
pub use json::{JsonError, MAX_RECORD_BYTES, MAX_RECORD_NODES, parse_record_json};
pub use manifest::{ManifestEntry, ManifestError, RecordDescriptor, SourceRevision, TreeManifest};
pub use meaning::{MeaningEntry, MeaningHistoryError, compare_meaning_histories};
pub use path::{PathError, SourcePath, validate_paths};
pub use record::{RecordEnvelope, RecordError, validate_record_envelope};
pub use semantic::{BootstrapError, BootstrapProfile, BootstrapScene, validate_bootstrap_profile};
pub use stage::{StageError, StagedFile, StagedLease, StagedTree, stage_declared_tree};
