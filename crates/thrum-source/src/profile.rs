//! Read-only first-profile source capacities, backed by enforcement owners.
//!
//! These dimensions describe content and accepted-store encoding capacity.
//! They are not disk reservations or F4 target-readiness promises. A candidate
//! must satisfy every applicable bound together; equal numbers with different
//! scopes remain separate dimensions.

use crate::{
    definition, environment_json, game_json, history_json, inventory, json, manifest,
    project_v2_json, registry_stage, sqlite_store,
};

/// The unit of a source-capacity dimension.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceCapacityUnit {
    /// Exact or decoded UTF-8 bytes, as specified by the dimension.
    Bytes,
    /// Files, entries, fields, nodes, references or other discrete items.
    Items,
    /// Nested JSON container levels.
    Levels,
}

/// One independently enforced content or accepted-store capacity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceDimension {
    /// All nonexcluded files and directories, including empty directories.
    InventoryEntries,
    /// Files in a canonical tree manifest; directories are absent.
    ManifestFiles,
    /// Exact bytes in one regular source file.
    FileBytes,
    /// Encoded canonical manifest bytes accepted by the native store.
    AcceptedManifestBytes,
    /// Exact bytes in one editable JSON record.
    RecordBytes,
    /// Values in the generic strict JSON record parser.
    GenericJsonNodes,
    /// Container nesting in the generic strict JSON record parser.
    GenericJsonDepth,
    /// Values traversed while extracting any staged record's type/ID header.
    RecordHeaderNodes,
    /// Container nesting while extracting any staged record's type/ID header.
    RecordHeaderDepth,
    /// Listed scene, game and supported built-in record paths in `core.project` schema 2.
    ProjectRecords,
    /// Listed game-schema paths in `core.project` schema 2.
    ProjectSchemas,
    /// Listed entry-scene IDs in `core.project` schema 2, also bounded by records.
    ProjectEntryScenes,
    /// Emitted required-reference summaries across the staged candidate.
    RequiredReferences,
    /// Fields in one source-carried schema descriptor.
    SchemaFields,
    /// UTF-8 bytes in one type, field or extension symbol in a definition.
    SchemaSymbolBytes,
    /// Largest selectable UTF-8 text bound in one schema field definition.
    SchemaTextLimitBytes,
    /// Permanent field-history entries in the carried registry history.
    HistoryFields,
    /// Type-version entries in the carried registry history.
    HistoryVersions,
    /// Total known and optional fields in one game record.
    GameRecordFields,
    /// Optional opaque fields in one game record.
    GameOpaqueFields,
    /// Values across one game record's opaque fields.
    GameOpaqueNodes,
    /// Whole-record container nesting for game opaque values.
    GameOpaqueDepth,
    /// Members in one nested game opaque object.
    GameOpaqueObjectMembers,
    /// Decoded UTF-8 bytes in one game opaque object key.
    GameOpaqueKeyBytes,
    /// Decoded UTF-8 bytes in one game opaque string value.
    GameOpaqueStringBytes,
    /// Decoded UTF-8 bytes in one authored environment string.
    EnvironmentStringBytes,
    /// Optional authored environment extension keys.
    EnvironmentExtensions,
    /// Exact encoded bytes in one authored environment extension.
    EnvironmentExtensionBytes,
    /// Total encoded extension bytes in one authored environment record.
    EnvironmentTotalExtensionBytes,
    /// Container nesting in one authored environment extension.
    EnvironmentExtensionDepth,
    /// Values across all authored environment extensions in one record.
    EnvironmentExtensionNodes,
}

/// Maximum for one first-profile dimension, in its declared unit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceCapacity {
    /// Scope and meaning of the count.
    pub dimension: SourceDimension,
    /// Unit of `maximum`.
    pub unit: SourceCapacityUnit,
    /// Inclusive maximum in `unit`.
    pub maximum: u64,
}

/// What an owner knew when it refused a capacity bound.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObservedQuantity {
    /// The exact checked value was available without more input work.
    Exact(u64),
    /// Validation stopped at this minimum to keep work bounded.
    AtLeast(u64),
}

/// A content-capacity refusal, independent of source identity and disk policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LimitExceeded {
    /// The specific source-profile dimension.
    pub dimension: SourceDimension,
    /// Inclusive supported maximum in the catalogue's unit.
    pub allowed: u64,
    /// Observed value or a lower bound when validation stopped early.
    pub observed: ObservedQuantity,
}

impl LimitExceeded {
    pub(crate) const fn exact(dimension: SourceDimension, allowed: u64, observed: u64) -> Self {
        Self {
            dimension,
            allowed,
            observed: ObservedQuantity::Exact(observed),
        }
    }
}

const fn cap(dimension: SourceDimension, unit: SourceCapacityUnit, maximum: u64) -> SourceCapacity {
    SourceCapacity {
        dimension,
        unit,
        maximum,
    }
}

/// Current first-profile limits without allocation, mutation or hash input.
///
/// This reports enforced dimensions, not proof that their simultaneous maxima
/// fit a device or that an F4 consumer supports every accepted source shape.
pub fn first_source_profile() -> &'static [SourceCapacity] {
    use SourceCapacityUnit::{Bytes, Items, Levels};
    use SourceDimension::*;
    const CAPACITIES: &[SourceCapacity] = &[
        cap(InventoryEntries, Items, inventory::MAX_ENTRIES as u64),
        cap(ManifestFiles, Items, manifest::MAX_ENTRIES as u64),
        cap(FileBytes, Bytes, manifest::MAX_BLOB_BYTES),
        cap(
            AcceptedManifestBytes,
            Bytes,
            sqlite_store::FIRST_PROFILE_MANIFEST_BYTES as u64,
        ),
        cap(RecordBytes, Bytes, json::MAX_RECORD_BYTES as u64),
        cap(GenericJsonNodes, Items, json::MAX_RECORD_NODES as u64),
        cap(GenericJsonDepth, Levels, json::MAX_CONTAINER_DEPTH as u64),
        cap(RecordHeaderNodes, Items, json::MAX_RECORD_NODES as u64),
        cap(
            RecordHeaderDepth,
            Levels,
            environment_json::MAX_HEADER_CONTAINER_DEPTH as u64,
        ),
        cap(ProjectRecords, Items, project_v2_json::MAX_RECORDS as u64),
        cap(ProjectSchemas, Items, project_v2_json::MAX_SCHEMAS as u64),
        cap(
            ProjectEntryScenes,
            Items,
            project_v2_json::MAX_RECORDS as u64,
        ),
        cap(
            RequiredReferences,
            Items,
            registry_stage::MAX_REQUIRED_REFERENCES as u64,
        ),
        cap(SchemaFields, Items, definition::MAX_FIELDS as u64),
        cap(
            SchemaSymbolBytes,
            Bytes,
            definition::MAX_SYMBOL_BYTES as u64,
        ),
        cap(
            SchemaTextLimitBytes,
            Bytes,
            definition::MAX_TEXT_BYTES as u64,
        ),
        cap(HistoryFields, Items, history_json::MAX_FIELD_HISTORY as u64),
        cap(
            HistoryVersions,
            Items,
            history_json::MAX_VERSION_HISTORY as u64,
        ),
        cap(GameRecordFields, Items, game_json::MAX_RECORD_FIELDS as u64),
        cap(GameOpaqueFields, Items, game_json::MAX_OPAQUE_FIELDS as u64),
        cap(GameOpaqueNodes, Items, game_json::MAX_OPAQUE_NODES as u64),
        cap(GameOpaqueDepth, Levels, game_json::MAX_OPAQUE_DEPTH as u64),
        cap(
            GameOpaqueObjectMembers,
            Items,
            game_json::MAX_OPAQUE_OBJECT_MEMBERS as u64,
        ),
        cap(
            GameOpaqueKeyBytes,
            Bytes,
            game_json::MAX_OPAQUE_KEY_BYTES as u64,
        ),
        cap(
            GameOpaqueStringBytes,
            Bytes,
            game_json::MAX_OPAQUE_STRING_BYTES as u64,
        ),
        cap(
            EnvironmentStringBytes,
            Bytes,
            environment_json::MAX_STRING_BYTES as u64,
        ),
        cap(
            EnvironmentExtensions,
            Items,
            environment_json::MAX_EXTENSIONS as u64,
        ),
        cap(
            EnvironmentExtensionBytes,
            Bytes,
            environment_json::MAX_EXTENSION_BYTES as u64,
        ),
        cap(
            EnvironmentTotalExtensionBytes,
            Bytes,
            environment_json::MAX_TOTAL_EXTENSION_BYTES as u64,
        ),
        cap(
            EnvironmentExtensionDepth,
            Levels,
            environment_json::MAX_EXTENSION_DEPTH as u64,
        ),
        cap(
            EnvironmentExtensionNodes,
            Items,
            environment_json::MAX_EXTENSION_NODES as u64,
        ),
    ];
    CAPACITIES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_profile_distinguishes_equal_values_with_different_scopes() {
        let capacities = first_source_profile();
        let find = |dimension| {
            capacities
                .iter()
                .find(|capacity| capacity.dimension == dimension)
                .copied()
                .unwrap()
        };
        assert_eq!(find(SourceDimension::InventoryEntries).maximum, 100_000);
        assert_eq!(find(SourceDimension::ManifestFiles).maximum, 100_000);
        assert_eq!(find(SourceDimension::ProjectRecords).maximum, 4_096);
        assert_eq!(find(SourceDimension::ProjectEntryScenes).maximum, 4_096);
        assert_eq!(find(SourceDimension::ProjectSchemas).maximum, 256);
        assert_eq!(
            find(SourceDimension::AcceptedManifestBytes).maximum,
            64 << 20
        );
        assert_eq!(find(SourceDimension::RequiredReferences).maximum, 65_536);
        assert!(capacities.iter().all(|capacity| capacity.maximum > 0));
        for (index, capacity) in capacities.iter().enumerate() {
            assert!(
                !capacities[..index]
                    .iter()
                    .any(|prior| prior.dimension == capacity.dimension)
            );
        }
    }
}
