//! Exact-byte candidate capture for native authoring source.
//!
//! Candidate bytes have no accepted-source authority until a separate durable
//! root transaction commits them. This crate never reads a running world.

mod json;
mod manifest;
mod path;
mod record;

pub use json::{JsonError, MAX_RECORD_BYTES, parse_record_json};
pub use manifest::{ManifestEntry, ManifestError, RecordDescriptor, SourceRevision, TreeManifest};
pub use path::{PathError, SourcePath, validate_paths};
pub use record::{RecordEnvelope, RecordError, validate_record_envelope};
