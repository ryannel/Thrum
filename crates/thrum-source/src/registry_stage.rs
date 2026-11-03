//! Staged-byte and declaration proof for the first source-carried registry.
//!
//! Registry validation precedes a bounded scene/game/built-in closure pass.
//! Prior-root lineage, physical peak admission and durable admission remain
//! separate gates.

use std::io::Read;
use std::sync::LazyLock;

use sha2::{Digest, Sha256};

use crate::bootstrap_json::parse_scene;
use crate::environment_json::{self, ENVIRONMENT_DEFINITION, LIGHT_DEFINITION, SCENE_DEFINITION};
#[cfg(test)]
use crate::game_json::parse_game_record;
use crate::game_json::{
    GameRecordError, GameRecordSummary, parse_game_record_with_reference_limit,
};
use crate::history_json::{
    CarriedHistoryError, HistoryError, compare_carried_histories, parse_registry_history,
};
use crate::project_v2_json::{ProjectV2, ProjectV2Error, parse_project_v2};
use crate::registry::{RegistryConsistencyError, validate_active_registry_with};
use crate::schema_json::{SchemaDescriptor, SchemaError, parse_schema_descriptor};
use crate::{
    LimitExceeded, MAX_RECORD_BYTES, ManifestEntry, ManifestError, ObservedQuantity,
    RecordDescriptor, SourceDimension, SourcePath, StageError, StagedFile, StagedTree,
    TreeManifest,
};

pub(crate) const MAX_REQUIRED_REFERENCES: usize = 65_536;

/// A declared first-profile registry does not agree with its staged source.
#[derive(Debug)]
pub enum StagedRegistryError {
    /// This profile has no external dependency-root resolver yet.
    UnsupportedDependencyRoots,
    /// The staged tree contains no project record.
    MissingProject,
    /// A declared record path is absent from the stage.
    MissingRecord(SourcePath),
    /// A supported engine-record path was omitted from its project declaration.
    UnlistedRecord(SourcePath),
    /// A reserved record kind has no schema in this profile.
    UnsupportedRecord(SourcePath),
    /// A staged JSON record exceeds the first-profile 16 MiB bound.
    RecordTooLarge(SourcePath),
    /// Staged bytes changed after the exact-byte capture.
    StagedDrift(SourcePath),
    /// Opening a staged file failed.
    Stage(StageError),
    /// Reading a staged file failed.
    Io {
        /// Source path whose read failed.
        path: SourcePath,
        /// Underlying I/O error.
        source: std::io::Error,
    },
    /// The fixed project declaration refused.
    Project(ProjectV2Error),
    /// A declared descriptor refused.
    Schema {
        /// Declared descriptor path.
        path: SourcePath,
        /// Fixed descriptor error.
        source: SchemaError,
    },
    /// The declared registry-history record refused.
    History(HistoryError),
    /// Active descriptors disagree with their carried history.
    Registry(RegistryConsistencyError),
    /// Two staged engine records claim one stable ID.
    DuplicateRecordId(String),
}

impl StagedRegistryError {
    /// Specific nested source-capacity refusal, when available.
    pub fn limit_exceeded(&self) -> Option<LimitExceeded> {
        match self {
            Self::Stage(error) => error.limit_exceeded(),
            Self::Project(error) => error.limit_exceeded(),
            Self::Schema { source, .. } => source.limit_exceeded(),
            Self::History(error) => error.limit_exceeded(),
            _ => None,
        }
    }
}

/// A registry's declared scene/game/built-in records failed the closure profile.
#[derive(Debug)]
pub enum StagedRecordError {
    /// A game record has no available game descriptor.
    GameSchemaCount(usize),
    /// No checked descriptor owns this game record's source type.
    UnknownGameType {
        /// Declared game record path.
        path: SourcePath,
        /// Type ID found in its exact staged bytes.
        record_type: String,
    },
    /// Stage bytes changed or could not be reread.
    Stage(StagedRegistryError),
    /// A declared `core.scene` record failed its fixed schema.
    Scene {
        /// Source path of the scene.
        path: SourcePath,
        /// Fixed-schema parser error.
        source: serde_json::Error,
    },
    /// A declared game record failed the source-carried descriptor.
    Game {
        /// Source path of the game record.
        path: SourcePath,
        /// Bounded game parser error.
        source: GameRecordError,
    },
    /// An ID collides with another scene, game, built-in or registry record.
    DuplicateRecordId(String),
    /// A project entry ID does not name a staged scene.
    UnresolvedEntryScene(String),
    /// A required game reference does not name a staged scene.
    UnresolvedGameScene {
        /// Source game record ID.
        source_id: String,
        /// Required target scene ID.
        target_id: String,
    },
    /// Required-edge summaries exceed this first profile's 65,536-edge cap.
    TooManyRequiredEdges {
        /// Processed reference count at refusal; more records may remain.
        observed_at_least: u64,
    },
    /// A source envelope could not supply unambiguous type/version dispatch.
    Envelope {
        /// Declared source record path.
        path: SourcePath,
        /// Bounded discriminator parser error.
        source: serde_json::Error,
    },
    /// An engine-owned built-in failed its fixed schema/profile.
    Builtin {
        /// Declared source record path.
        path: SourcePath,
        /// Fixed-schema parser error.
        source: serde_json::Error,
    },
    /// This profile has no built-in for this engine-owned type/version pair.
    UnsupportedBuiltin {
        /// Declared source record path.
        path: SourcePath,
        /// Checked top-level type.
        record_type: String,
        /// Checked top-level version.
        schema: u64,
    },
    /// The first environment closure permits exactly one entry scene.
    EnvironmentEntrySceneCount(usize),
    /// A scene's required environment UUID has no authored-source target.
    UnresolvedEnvironmentSource {
        /// Source scene ID.
        scene_id: String,
        /// Required authored environment target.
        target_id: String,
    },
    /// An authored Sun UUID has no light identity target.
    UnresolvedEnvironmentLight {
        /// Source authored environment ID.
        source_id: String,
        /// Required light identity target.
        target_id: String,
    },
}

impl StagedRecordError {
    /// Specific nested source-capacity refusal, when available.
    pub fn limit_exceeded(&self) -> Option<LimitExceeded> {
        match self {
            Self::TooManyRequiredEdges { observed_at_least } => Some(LimitExceeded {
                dimension: SourceDimension::RequiredReferences,
                allowed: MAX_REQUIRED_REFERENCES as u64,
                observed: ObservedQuantity::AtLeast(*observed_at_least),
            }),
            Self::Stage(error) => error.limit_exceeded(),
            Self::Game { source, .. } => source.limit_exceeded(),
            _ => None,
        }
    }
}

/// A first-profile staged candidate failed before deriving a manifest.
#[derive(Debug)]
pub enum StagedCandidateError {
    /// Project, schema or carried history validation refused.
    Registry(StagedRegistryError),
    /// Scene/game/built-in identity or required reference closure refused.
    Records(StagedRecordError),
}

impl StagedCandidateError {
    /// Specific nested source-capacity refusal, when available.
    pub fn limit_exceeded(&self) -> Option<LimitExceeded> {
        match self {
            Self::Registry(error) => error.limit_exceeded(),
            Self::Records(error) => error.limit_exceeded(),
        }
    }
}

/// Two rehashed staged histories cannot form one monotonic source lineage.
#[derive(Debug)]
pub enum StagedLineageError {
    /// The two project declarations name different durable projects.
    DifferentProject,
    /// One history file could not be reread or its staged hash changed.
    Stage(StagedRegistryError),
    /// One source-carried history record failed its fixed grammar.
    History(HistoryError),
    /// A permanent field meaning or type-version definition was lost or changed.
    Conflict(CarriedHistoryError),
}

impl StagedLineageError {
    /// Specific nested source-capacity refusal, when available.
    pub fn limit_exceeded(&self) -> Option<LimitExceeded> {
        match self {
            Self::Stage(error) => error.limit_exceeded(),
            Self::History(error) => error.limit_exceeded(),
            Self::DifferentProject | Self::Conflict(_) => None,
        }
    }
}

/// Registry and records checked together against one provisional stage.
pub struct StagedCandidate<'a> {
    registry: StagedRegistry<'a>,
    records: StagedRecords,
}

impl StagedCandidate<'_> {
    /// Candidate registry and its source-carried meanings.
    pub fn registry(&self) -> &StagedRegistry<'_> {
        &self.registry
    }

    /// Checked scene/game/built-in identities and local required edges.
    pub fn records(&self) -> &StagedRecords {
        &self.records
    }

    pub(crate) fn rehashed_history_bytes(&self) -> Result<Vec<u8>, StagedRegistryError> {
        let path = fixed_path(self.registry.project.history_path());
        let file = find_file(self.registry.stage, &path).expect("candidate checked history path");
        read_record_bytes(self.registry.stage, file)
    }

    /// Derive exact-byte metadata and its reproducible provisional revision.
    ///
    /// This uses hashes recorded by staging. Durable admission must rehash
    /// every staged file, compare authenticated prior history and commit the
    /// root/outcome transaction before calling this revision accepted.
    pub fn provisional_manifest(&self) -> Result<TreeManifest, ManifestError> {
        let project_path = fixed_path("project.json");
        let history_path = fixed_path(self.registry.project.history_path());
        let mut labels = Vec::with_capacity(
            2 + self.registry.schemas.len()
                + self.records.scenes.len()
                + self.records.games.len()
                + self.records.builtins.len(),
        );
        labels.push((&project_path, self.registry.project.id(), "core.project"));
        labels.push((
            &history_path,
            self.registry.history_id.as_str(),
            "core.registry-history",
        ));
        labels.extend(
            self.registry
                .schemas
                .iter()
                .map(|item| (&item.path, item.descriptor.record_id(), "core.schema")),
        );
        labels.extend(
            self.records
                .scenes
                .iter()
                .map(|item| (&item.path, item.id.as_str(), "core.scene")),
        );
        labels.extend(
            self.records
                .games
                .iter()
                .map(|item| (&item.path, item.summary.id(), item.summary.record_type())),
        );
        labels.extend(
            self.records
                .builtins
                .iter()
                .map(|item| (&item.path, item.id.as_str(), item.definition.record_type())),
        );
        labels.sort_unstable_by(|left, right| left.0.cmp(right.0));

        let mut label_index = 0;
        let mut entries = Vec::with_capacity(self.registry.stage.files().len());
        for file in self.registry.stage.files() {
            let record = if labels
                .get(label_index)
                .is_some_and(|(path, _, _)| *path == file.path())
            {
                let (_, id, record_type) = labels[label_index];
                label_index += 1;
                Some(RecordDescriptor::checked_parts(id, record_type))
            } else {
                None
            };
            entries.push(ManifestEntry::new(
                file.path().clone(),
                file.length(),
                *file.sha256(),
                record,
            )?);
        }
        debug_assert_eq!(label_index, labels.len());
        TreeManifest::new(self.registry.project.id(), Vec::new(), entries)
    }
}

/// One checked scene identity paired with its staged source path.
pub struct StagedScene {
    path: SourcePath,
    id: String,
    schema: u64,
    label: Option<String>,
    environment_source: Option<String>,
    definition: Option<&'static BuiltinSchema>,
}

impl StagedScene {
    /// Declared staged path.
    pub fn path(&self) -> &SourcePath {
        &self.path
    }

    /// Canonical stable scene ID.
    pub fn id(&self) -> &str {
        &self.id
    }
    /// Checked source schema version; legacy scenes retain schema 1.
    pub fn schema(&self) -> u64 {
        self.schema
    }
    /// Optional schema-2 human label; never used for reference resolution.
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }
    /// Optional typed authored-source UUID selected by scene schema 2.
    pub fn environment_source(&self) -> Option<&str> {
        self.environment_source.as_deref()
    }
    /// Immutable built-in meaning for schema 2. Legacy schema 1 continues
    /// through its existing fixed bootstrap parser.
    pub fn definition(&self) -> Option<&'static BuiltinSchema> {
        self.definition
    }
}

/// Immutable SDK-owned meaning descriptor used by a fixed built-in parser.
///
/// Descriptor bytes use the explicit `thrum-builtin-definition-v1` language.
/// SHA-256 hashes `thrum-f3-builtin-schema-v1\0` followed by those exact bytes.
/// These are source schema meanings, not F4 products or admission certificates.
pub struct BuiltinSchema {
    record_type: &'static str,
    schema: u64,
    descriptor: &'static str,
    definition_hash: [u8; 32],
}

impl BuiltinSchema {
    /// Fixed engine-owned source type.
    pub fn record_type(&self) -> &'static str {
        self.record_type
    }
    /// Fixed source schema version.
    pub fn schema(&self) -> u64 {
        self.schema
    }
    /// Exact immutable versioned meaning descriptor bytes.
    pub fn descriptor(&self) -> &'static str {
        self.descriptor
    }
    /// Domain-separated SHA-256 of the exact descriptor bytes.
    pub fn definition_hash(&self) -> &[u8; 32] {
        &self.definition_hash
    }
}

static BUILTIN_SCHEMAS: LazyLock<[BuiltinSchema; 3]> = LazyLock::new(|| {
    [
        ("core.scene", 2, SCENE_DEFINITION),
        ("thrum.environment.authored", 1, ENVIRONMENT_DEFINITION),
        ("thrum.light.identity", 1, LIGHT_DEFINITION),
    ]
    .map(|(record_type, schema, descriptor)| {
        let mut digest = Sha256::new();
        digest.update(b"thrum-f3-builtin-schema-v1\0");
        digest.update(descriptor.as_bytes());
        BuiltinSchema {
            record_type,
            schema,
            descriptor,
            definition_hash: digest.finalize().into(),
        }
    })
});

/// The immutable F3 meaning used for an engine-owned record type and version.
///
/// F4 may use this read-only identity when keying a product from separately
/// authenticated pinned bytes. A lookup does not validate a record or grant
/// source authority.
pub fn builtin_schema(record_type: &str, schema: u64) -> Option<&'static BuiltinSchema> {
    BUILTIN_SCHEMAS
        .iter()
        .find(|definition| definition.record_type() == record_type && definition.schema() == schema)
}

/// A checked fixed built-in identity and its required typed source edge.
pub struct StagedBuiltin {
    path: SourcePath,
    id: String,
    label: Option<String>,
    definition: &'static BuiltinSchema,
    light_ref: Option<String>,
}

impl StagedBuiltin {
    /// Declared staged source path.
    pub fn path(&self) -> &SourcePath {
        &self.path
    }
    /// Canonical stable source record ID.
    pub fn id(&self) -> &str {
        &self.id
    }
    /// Optional human label, never used for reference resolution.
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }
    /// SDK-owned source meaning used to validate this staged record.
    pub fn definition(&self) -> &'static BuiltinSchema {
        self.definition
    }
    /// Required `thrum.light.identity` target of an authored Sun, if this
    /// built-in is an environment source. Light identity records have no edge.
    pub fn required_light(&self) -> Option<&str> {
        self.light_ref.as_deref()
    }
}

/// One checked game record paired with its staged source path.
pub struct StagedGame {
    path: SourcePath,
    summary: GameRecordSummary,
}

impl StagedGame {
    /// Declared staged path.
    pub fn path(&self) -> &SourcePath {
        &self.path
    }

    /// Descriptor-bound identity and required references.
    pub fn summary(&self) -> &GameRecordSummary {
        &self.summary
    }
}

/// Checked scene/game/built-in identities and required local reference closure.
pub struct StagedRecords {
    scenes: Vec<StagedScene>,
    games: Vec<StagedGame>,
    builtins: Vec<StagedBuiltin>,
    required_edges: usize,
}

impl StagedRecords {
    /// Scene records in declared path order.
    pub fn scenes(&self) -> &[StagedScene] {
        &self.scenes
    }

    /// Game records in declared path order.
    pub fn games(&self) -> &[StagedGame] {
        &self.games
    }

    /// Engine-owned environment/light records in declared path order.
    pub fn builtins(&self) -> &[StagedBuiltin] {
        &self.builtins
    }

    /// Number of checked required game-to-scene, scene-to-environment and
    /// environment-to-light references.
    pub fn required_edges(&self) -> usize {
        self.required_edges
    }
}

/// One descriptor paired with its declared and rehashed source path.
pub struct StagedSchema {
    path: SourcePath,
    descriptor: SchemaDescriptor,
}

impl StagedSchema {
    /// Declared source path of the fixed descriptor record.
    pub fn path(&self) -> &SourcePath {
        &self.path
    }

    /// Parsed descriptor, with hashes derived from its checked fields.
    pub fn descriptor(&self) -> &SchemaDescriptor {
        &self.descriptor
    }
}

/// Checked source-carried registry; not a complete semantic candidate.
pub struct StagedRegistry<'a> {
    stage: &'a StagedTree,
    project: ProjectV2,
    schemas: Vec<StagedSchema>,
    history_id: String,
}

impl StagedRegistry<'_> {
    /// Stage whose exact bytes were checked for the registry records.
    pub fn stage(&self) -> &StagedTree {
        self.stage
    }

    /// Checked project declaration, including still-unresolved record paths.
    pub fn project(&self) -> &ProjectV2 {
        &self.project
    }

    /// Checked descriptors in ascending game type ID order.
    pub fn schemas(&self) -> &[StagedSchema] {
        &self.schemas
    }

    /// Stable ID of the source-carried history record.
    pub fn history_id(&self) -> &str {
        &self.history_id
    }
}

/// Rehash declared registry bytes, check reserved path membership, and join
/// active descriptor meanings with the carried history.
///
/// A complete candidate also needs scene/game/built-in parsing and ID/reference
/// closure. The history checked here is only the candidate's own history;
/// comparison to an authenticated predecessor belongs to durable admission.
pub fn validate_staged_registry<'a>(
    stage: &'a StagedTree,
    dependency_roots: &[String],
) -> Result<StagedRegistry<'a>, StagedRegistryError> {
    let registry = prepare_staged_registry(stage, dependency_roots)?;
    let mut ids = Vec::with_capacity(registry.schemas.len() + 2);
    ids.push(registry.project.id());
    ids.push(registry.history_id.as_str());
    ids.extend(
        registry
            .schemas
            .iter()
            .map(|item| item.descriptor.record_id()),
    );
    if let Some(id) = first_duplicate_record_id(&mut ids) {
        return Err(StagedRegistryError::DuplicateRecordId(id.to_owned()));
    }
    Ok(registry)
}

// Prepare registry structure and descriptor meanings without a UUID collision
// pass. Complete candidates check all top-level IDs after declared structure.
fn prepare_staged_registry<'a>(
    stage: &'a StagedTree,
    dependency_roots: &[String],
) -> Result<StagedRegistry<'a>, StagedRegistryError> {
    if !dependency_roots.is_empty() {
        return Err(StagedRegistryError::UnsupportedDependencyRoots);
    }
    let project_path = fixed_path("project.json");
    let project_file =
        find_file(stage, &project_path).ok_or(StagedRegistryError::MissingProject)?;
    let mut record_bytes = read_record_bytes(stage, project_file)?;
    let project = parse_project_v2(&record_bytes).map_err(StagedRegistryError::Project)?;

    let history_path = fixed_path(project.history_path());
    for path in project
        .record_paths()
        .iter()
        .chain(project.schema_paths())
        .chain(std::iter::once(&history_path))
    {
        if find_file(stage, path).is_none() {
            return Err(StagedRegistryError::MissingRecord(path.clone()));
        }
    }
    for file in stage.files() {
        let path = file.path();
        if path == &project_path {
            continue;
        }
        let text = path.as_str();
        let listed = if text.starts_with("scenes/") && text.ends_with(".scene.json")
            || text.starts_with("records/") && text.ends_with(".record.json")
        {
            project.record_paths().binary_search(path).is_ok()
        } else if text.starts_with("schemas/") && text.ends_with(".schema.json") {
            project.schema_paths().binary_search(path).is_ok()
        } else if path == &history_path {
            true
        } else if text.ends_with(".prefab.json") || text.ends_with(".asset.json") {
            return Err(StagedRegistryError::UnsupportedRecord(path.clone()));
        } else {
            continue;
        };
        if !listed {
            return Err(StagedRegistryError::UnlistedRecord(path.clone()));
        }
    }

    let mut schemas = Vec::with_capacity(project.schema_paths().len());
    for path in project.schema_paths() {
        let file = find_file(stage, path).expect("membership checked above");
        read_record_bytes_into(stage, file, &mut record_bytes)?;
        let descriptor = parse_schema_descriptor(&record_bytes).map_err(|source| {
            StagedRegistryError::Schema {
                path: path.clone(),
                source,
            }
        })?;
        schemas.push(StagedSchema {
            path: path.clone(),
            descriptor,
        });
    }
    drop(record_bytes);
    schemas.sort_unstable_by(|left, right| {
        left.descriptor
            .target_type()
            .cmp(right.descriptor.target_type())
    });

    let history_file = find_file(stage, &history_path).expect("membership checked above");
    let history_bytes = read_record_bytes(stage, history_file)?;
    let history = parse_registry_history(&history_bytes).map_err(StagedRegistryError::History)?;
    validate_active_registry_with(&schemas, &history, |item| &item.descriptor)
        .map_err(StagedRegistryError::Registry)?;
    let history_id = history.record_id().to_owned();
    drop(history);
    drop(history_bytes);

    Ok(StagedRegistry {
        stage,
        project,
        schemas,
        history_id,
    })
}

/// Rehash declared record bytes and resolve required local typed references.
///
/// Checked top-level type/version selects a fixed engine-owned parser or a
/// source-carried game descriptor, independent of JSON key order. Schema-1
/// scenes retain the existing bootstrap grammar. This result is not an
/// authenticated lineage comparison or an admitted source root.
pub fn validate_staged_records(
    registry: &StagedRegistry<'_>,
) -> Result<StagedRecords, StagedRecordError> {
    const MAX_REQUIRED_EDGES: usize = MAX_REQUIRED_REFERENCES;

    let paths = registry.project.record_paths();
    let scene_count = paths
        .iter()
        .filter(|path| path.as_str().starts_with("scenes/"))
        .count();
    let mut scenes = Vec::with_capacity(scene_count);
    let mut games = Vec::new();
    let mut builtins = Vec::new();
    let mut required_edges = 0_usize;
    let mut bytes = Vec::new();
    for path in paths {
        let file = find_file(registry.stage, path).expect("registry checked declared membership");
        read_record_bytes_into(registry.stage, file, &mut bytes)
            .map_err(StagedRecordError::Stage)?;
        let header = environment_json::record_header(&bytes).map_err(|source| {
            StagedRecordError::Envelope {
                path: path.clone(),
                source,
            }
        })?;
        let builtin_error = |source| StagedRecordError::Builtin {
            path: path.clone(),
            source,
        };
        if path.as_str().starts_with("scenes/") {
            let scene_error = |source| StagedRecordError::Scene {
                path: path.clone(),
                source,
            };
            let scene = match (header.record_type.as_str(), header.schema) {
                ("core.scene", 1) => {
                    let parsed = parse_scene(&bytes).map_err(scene_error)?;
                    StagedScene {
                        path: path.clone(),
                        id: parsed.id,
                        schema: 1,
                        label: None,
                        environment_source: None,
                        definition: None,
                    }
                }
                ("core.scene", 2) => {
                    let parsed = environment_json::parse_scene(&bytes).map_err(scene_error)?;
                    StagedScene {
                        path: path.clone(),
                        id: parsed.id,
                        schema: 2,
                        label: parsed.label,
                        environment_source: parsed.environment_source,
                        definition: Some(&BUILTIN_SCHEMAS[0]),
                    }
                }
                _ => {
                    return Err(StagedRecordError::UnsupportedBuiltin {
                        path: path.clone(),
                        record_type: header.record_type,
                        schema: header.schema,
                    });
                }
            };
            account_required_edges(
                &mut required_edges,
                usize::from(scene.environment_source.is_some()),
                &mut scenes,
                &mut games,
                &mut builtins,
            );
            if required_edges <= MAX_REQUIRED_EDGES {
                scenes.push(scene);
            }
        } else {
            match (header.record_type.as_str(), header.schema) {
                ("thrum.environment.authored", 1) => {
                    let parsed =
                        environment_json::parse_environment(&bytes).map_err(builtin_error)?;
                    account_required_edges(
                        &mut required_edges,
                        1,
                        &mut scenes,
                        &mut games,
                        &mut builtins,
                    );
                    if required_edges <= MAX_REQUIRED_EDGES {
                        builtins.push(StagedBuiltin {
                            path: path.clone(),
                            id: parsed.id,
                            label: parsed.label,
                            definition: &BUILTIN_SCHEMAS[1],
                            light_ref: Some(parsed.sun.light_ref),
                        });
                    }
                }
                ("thrum.light.identity", 1) => {
                    let parsed = environment_json::parse_light(&bytes).map_err(builtin_error)?;
                    if required_edges <= MAX_REQUIRED_EDGES {
                        builtins.push(StagedBuiltin {
                            path: path.clone(),
                            id: parsed.id,
                            label: parsed.label,
                            definition: &BUILTIN_SCHEMAS[2],
                            light_ref: None,
                        });
                    }
                }
                (record_type, _) if record_type.starts_with("game.") => {
                    if registry.schemas.is_empty() {
                        return Err(StagedRecordError::GameSchemaCount(0));
                    }
                    let descriptor = if registry.schemas.len() == 1 {
                        &registry.schemas[0].descriptor
                    } else {
                        let index = registry
                            .schemas
                            .binary_search_by(|item| item.descriptor.target_type().cmp(record_type))
                            .map_err(|_| StagedRecordError::UnknownGameType {
                                path: path.clone(),
                                record_type: header.record_type.clone(),
                            })?;
                        &registry.schemas[index].descriptor
                    };
                    let remaining = MAX_REQUIRED_EDGES.saturating_sub(required_edges);
                    let (summary, reference_count) =
                        parse_game_record_with_reference_limit(&bytes, descriptor, remaining)
                            .map_err(|source| StagedRecordError::Game {
                                path: path.clone(),
                                source,
                            })?;
                    account_required_edges(
                        &mut required_edges,
                        reference_count,
                        &mut scenes,
                        &mut games,
                        &mut builtins,
                    );
                    if required_edges <= MAX_REQUIRED_EDGES {
                        games.push(StagedGame {
                            path: path.clone(),
                            summary,
                        });
                    }
                }
                _ => {
                    return Err(StagedRecordError::UnsupportedBuiltin {
                        path: path.clone(),
                        record_type: header.record_type,
                        schema: header.schema,
                    });
                }
            }
        }
    }

    if required_edges > MAX_REQUIRED_EDGES {
        return Err(StagedRecordError::TooManyRequiredEdges {
            observed_at_least: (MAX_REQUIRED_EDGES + 1) as u64,
        });
    }

    let mut all_ids = Vec::with_capacity(2 + registry.schemas.len() + paths.len());
    all_ids.push(registry.project.id());
    all_ids.push(registry.history_id.as_str());
    all_ids.extend(
        registry
            .schemas
            .iter()
            .map(|item| item.descriptor.record_id()),
    );
    all_ids.extend(scenes.iter().map(StagedScene::id));
    all_ids.extend(games.iter().map(|item| item.summary.id()));
    all_ids.extend(builtins.iter().map(StagedBuiltin::id));
    if let Some(id) = first_duplicate_record_id(&mut all_ids) {
        return Err(StagedRecordError::DuplicateRecordId(id.to_owned()));
    }

    let mut scene_ids: Vec<_> = scenes.iter().map(StagedScene::id).collect();
    scene_ids.sort_unstable();
    for id in registry.project.entry_scenes() {
        if scene_ids.binary_search(&id.as_str()).is_err() {
            return Err(StagedRecordError::UnresolvedEntryScene(id.clone()));
        }
    }
    for game in &games {
        for reference in game.summary.references() {
            if scene_ids.binary_search(&reference.target_id()).is_err() {
                return Err(StagedRecordError::UnresolvedGameScene {
                    source_id: game.summary.id().to_owned(),
                    target_id: reference.target_id().to_owned(),
                });
            }
        }
    }
    // Only an entry scene selects active forcing. Other declared scenes and
    // sources remain content, with their required references still checked.
    if scenes.iter().any(|scene| {
        scene.environment_source.is_some()
            && registry
                .project
                .entry_scenes()
                .binary_search(&scene.id)
                .is_ok()
    }) && registry.project.entry_scenes().len() != 1
    {
        return Err(StagedRecordError::EnvironmentEntrySceneCount(
            registry.project.entry_scenes().len(),
        ));
    }
    let mut environment_ids: Vec<_> = builtins
        .iter()
        .filter(|item| item.definition.record_type() == "thrum.environment.authored")
        .map(StagedBuiltin::id)
        .collect();
    let mut light_ids: Vec<_> = builtins
        .iter()
        .filter(|item| item.definition.record_type() == "thrum.light.identity")
        .map(StagedBuiltin::id)
        .collect();
    environment_ids.sort_unstable();
    light_ids.sort_unstable();
    for scene in &scenes {
        if let Some(target_id) = &scene.environment_source
            && environment_ids.binary_search(&target_id.as_str()).is_err()
        {
            return Err(StagedRecordError::UnresolvedEnvironmentSource {
                scene_id: scene.id.clone(),
                target_id: target_id.clone(),
            });
        }
    }
    for source in &builtins {
        if let Some(target_id) = &source.light_ref
            && light_ids.binary_search(&target_id.as_str()).is_err()
        {
            return Err(StagedRecordError::UnresolvedEnvironmentLight {
                source_id: source.id.clone(),
                target_id: target_id.clone(),
            });
        }
    }
    Ok(StagedRecords {
        scenes,
        games,
        builtins,
        required_edges,
    })
}

fn first_duplicate_record_id<'a>(ids: &mut [&'a str]) -> Option<&'a str> {
    ids.sort_unstable();
    ids.windows(2)
        .find(|pair| pair[0] == pair[1])
        .map(|pair| pair[0])
}

fn account_required_edges(
    current: &mut usize,
    added: usize,
    scenes: &mut Vec<StagedScene>,
    games: &mut Vec<StagedGame>,
    builtins: &mut Vec<StagedBuiltin>,
) {
    let next = current
        .saturating_add(added)
        .min(MAX_REQUIRED_REFERENCES + 1);
    if *current <= MAX_REQUIRED_REFERENCES && next > MAX_REQUIRED_REFERENCES {
        // The semantic limit is reported only after every declared record's
        // local grammar. No retained projection can be used after overflow.
        *scenes = Vec::new();
        *games = Vec::new();
        *builtins = Vec::new();
    }
    *current = next;
}

/// Check one narrow staged registry and its declared record closure together.
///
/// This does not perform authenticated prior-root lineage, full combined
/// allocation/RSS qualification, editor round-trip or durable source admission.
pub fn validate_staged_candidate<'a>(
    stage: &'a StagedTree,
    dependency_roots: &[String],
) -> Result<StagedCandidate<'a>, StagedCandidateError> {
    let registry =
        prepare_staged_registry(stage, dependency_roots).map_err(StagedCandidateError::Registry)?;
    let records = validate_staged_records(&registry).map_err(StagedCandidateError::Records)?;
    Ok(StagedCandidate { registry, records })
}

/// Compare source-carried histories from two already checked staged registries.
///
/// Both history inputs are reread and rehashed, and their parsed vectors borrow
/// their input buffers through one linear comparison. This proves only the
/// staged-byte lineage relationship. A later durable store must authenticate
/// `prior` as the current admitted root under a generation precondition.
pub fn compare_staged_histories(
    prior: &StagedRegistry<'_>,
    candidate: &StagedRegistry<'_>,
) -> Result<(), StagedLineageError> {
    if prior.project().id() != candidate.project().id() {
        return Err(StagedLineageError::DifferentProject);
    }
    let prior_path = fixed_path(prior.project().history_path());
    let candidate_path = fixed_path(candidate.project().history_path());
    let prior_file = find_file(prior.stage(), &prior_path).expect("checked prior history path");
    let candidate_file =
        find_file(candidate.stage(), &candidate_path).expect("checked candidate history path");
    let prior_bytes =
        read_record_bytes(prior.stage(), prior_file).map_err(StagedLineageError::Stage)?;
    let candidate_bytes =
        read_record_bytes(candidate.stage(), candidate_file).map_err(StagedLineageError::Stage)?;
    let prior_history =
        parse_registry_history(&prior_bytes).map_err(StagedLineageError::History)?;
    let candidate_history =
        parse_registry_history(&candidate_bytes).map_err(StagedLineageError::History)?;
    compare_carried_histories(&prior_history, &candidate_history)
        .map_err(StagedLineageError::Conflict)
}

fn fixed_path(text: &str) -> SourcePath {
    SourcePath::parse(text).expect("fixed portable source path")
}

fn find_file<'a>(stage: &'a StagedTree, path: &SourcePath) -> Option<&'a StagedFile> {
    stage
        .files()
        .binary_search_by(|file| file.path().cmp(path))
        .ok()
        .map(|index| &stage.files()[index])
}

fn read_record_bytes(
    stage: &StagedTree,
    file: &StagedFile,
) -> Result<Vec<u8>, StagedRegistryError> {
    let mut bytes = Vec::new();
    read_record_bytes_into(stage, file, &mut bytes)?;
    Ok(bytes)
}

fn read_record_bytes_into(
    stage: &StagedTree,
    file: &StagedFile,
    bytes: &mut Vec<u8>,
) -> Result<(), StagedRegistryError> {
    let path = file.path();
    if file.length() > MAX_RECORD_BYTES as u64 {
        return Err(StagedRegistryError::RecordTooLarge(path.clone()));
    }
    bytes.clear();
    bytes.reserve(file.length() as usize + 1);
    stage
        .open(path)
        .map_err(StagedRegistryError::Stage)?
        .take(file.length() + 1)
        .read_to_end(bytes)
        .map_err(|source| StagedRegistryError::Io {
            path: path.clone(),
            source,
        })?;
    if bytes.len() as u64 != file.length() || Sha256::digest(&*bytes).as_slice() != file.sha256() {
        return Err(StagedRegistryError::StagedDrift(path.clone()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::{BufWriter, Write};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use serde_json::json;

    use crate::budget::Limits;
    use crate::game_edit::replace_game_text_field;
    use crate::stage::stage_declared_tree_with;

    const PROJECT_ID: &str = "2a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    const SCENE_ID: &str = "3a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    const GAME_ID: &str = "4a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    const SCHEMA_ID: &str = "5a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    const HISTORY_ID: &str = "6a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    const PATHS: &[&str] = &[
        "project.json",
        "records/marker.record.json",
        "scenes/main.scene.json",
        "schemas/history.registry.json",
        "schemas/marker.schema.json",
    ];
    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "thrum-registry-stage-{}-{}",
                std::process::id(),
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn write(&self, path: &str, bytes: &[u8]) {
            let full = self.0.join(path);
            fs::create_dir_all(full.parent().unwrap()).unwrap();
            fs::write(full, bytes).unwrap();
        }

        fn stage(&self, paths: &[&str]) -> StagedTree {
            let declared: Vec<_> = paths
                .iter()
                .map(|path| SourcePath::parse(path).unwrap())
                .collect();
            let limits = Limits {
                main: 2 * 1024 * 1024 * 1024,
                wal: 4 * 1024 * 1024,
                temporary: 64 * 1024 * 1024,
                free_floor: 0,
            };
            stage_declared_tree_with(&self.0, &declared, limits, |_| {}).unwrap()
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    fn hex(bytes: &[u8; 32]) -> String {
        let mut text = String::with_capacity(64);
        for byte in bytes {
            use std::fmt::Write;
            write!(&mut text, "{byte:02x}").unwrap();
        }
        text
    }

    fn basic(fixture: &Fixture) {
        fixture.write(
            "project.json",
            &serde_json::to_vec(&json!({
                "id":PROJECT_ID, "type":"core.project", "schema":2,
                "fields":{
                    "project.format":1,
                    "project.records":["records/marker.record.json", "scenes/main.scene.json"],
                    "project.schemas":["schemas/marker.schema.json"],
                    "project.registryHistory":"schemas/history.registry.json",
                    "project.entryScenes":[SCENE_ID]
                }
            }))
            .unwrap(),
        );
        fixture.write(
            "records/marker.record.json",
            &serde_json::to_vec(&json!({
                "id":GAME_ID, "type":"game.marker", "schema":1,
                "fields":{
                    "marker.caption":"first marker",
                    "marker.scene":SCENE_ID,
                    "x.game.marker.notes":{"nested":[true, null, {"n": 3}]}
                }
            }))
            .unwrap(),
        );
        fixture.write(
            "scenes/main.scene.json",
            &serde_json::to_vec(&json!({
                "id":SCENE_ID, "type":"core.scene", "schema":1, "fields":{}
            }))
            .unwrap(),
        );
        let schema_bytes = serde_json::to_vec(&json!({
            "id":SCHEMA_ID, "type":"core.schema", "schema":1,
            "fields":{
                "schema.targetType":"game.marker", "schema.targetVersion":1,
                "schema.fields":[
                    {"id":"marker.caption", "kind":"text", "presence":"optional", "maxUtf8Bytes":1024},
                    {"id":"marker.scene", "kind":"record-ref", "presence":"required", "targetType":"core.scene", "resolution":"required"}
                ],
                "schema.optionalExtensionPrefix":"x.game.marker."
            }
        }))
        .unwrap();
        fixture.write("schemas/marker.schema.json", &schema_bytes);
        let descriptor = parse_schema_descriptor(&schema_bytes).unwrap();
        fixture.write(
            "schemas/history.registry.json",
            &serde_json::to_vec(&json!({
                "id":HISTORY_ID, "type":"core.registry-history", "schema":1,
                "fields":{
                    "registry.fields":[
                        {"typeId":"game.marker", "fieldId":"marker.caption", "signature":hex(descriptor.fields()[0].signature()), "retired":false},
                        {"typeId":"game.marker", "fieldId":"marker.scene", "signature":hex(descriptor.fields()[1].signature()), "retired":false}
                    ],
                    "registry.versions":[
                        {"typeId":"game.marker", "targetVersion":1, "definitionHash":hex(descriptor.definition_hash())}
                    ]
                }
            }))
            .unwrap(),
        );
    }

    const ENVIRONMENT_ID: &str = "10000000-0000-4000-8000-000000000001";
    const LIGHT_ID: &str = "10000000-0000-4000-8000-000000000002";
    const ENVIRONMENT_PATH: &str = "records/environment.record.json";
    const LIGHT_PATH: &str = "records/light.record.json";
    const BUILTIN_PATHS: &[&str] = &[
        "project.json",
        ENVIRONMENT_PATH,
        LIGHT_PATH,
        "records/marker.record.json",
        "scenes/main.scene.json",
        "schemas/history.registry.json",
        "schemas/marker.schema.json",
    ];

    fn constant_light_bytes() -> Vec<u8> {
        serde_json::to_vec(&json!({
            "type":"thrum.environment.authored", "schema":1, "id":ENVIRONMENT_ID,
            "fields":{
                "label":"fixture light source", "profile":"constant-light-v1",
                "sun":{
                    "light_ref":LIGHT_ID, "toward_source_unit":[0,1,0],
                    "perpendicular_illuminance_lux":100000,
                    "unit_luminance_linear_rec709":[1,1,1], "angular_radius_rad":0.00465,
                    "direction_model":"geometric-straight", "applied_attenuation":[]
                },
                "ambient_irradiance_lux":[1000,1000,1000],
                "background_radiance_cd_m2":[0,0,0],
                "unsupported":["wind","cloud_tau","precipitation","air","thermal","moon","air_quality"]
            }
        })).unwrap()
    }

    fn write_environment_records(fixture: &Fixture) {
        fixture.write(ENVIRONMENT_PATH, &constant_light_bytes());
        fixture.write(
            LIGHT_PATH,
            &serde_json::to_vec(&json!({
                "id":LIGHT_ID, "type":"thrum.light.identity", "schema":1,
                "fields":{"label":"shared Sun", "driver_binding":"incoming-environment-sun"}
            }))
            .unwrap(),
        );
        fixture.write(
            "scenes/main.scene.json",
            &serde_json::to_vec(&json!({
                "id":SCENE_ID, "type":"core.scene", "schema":2,
                "fields":{"scene.label":"entry", "environment.source":ENVIRONMENT_ID}
            }))
            .unwrap(),
        );
    }

    fn builtin_fixture(fixture: &Fixture) {
        basic(fixture);
        write_environment_records(fixture);
        let mut project: serde_json::Value =
            serde_json::from_slice(&fs::read(fixture.0.join("project.json")).unwrap()).unwrap();
        project["fields"]["project.records"] = json!([
            ENVIRONMENT_PATH,
            LIGHT_PATH,
            "records/marker.record.json",
            "scenes/main.scene.json"
        ]);
        fixture.write("project.json", &serde_json::to_vec(&project).unwrap());
    }

    #[test]
    fn staged_constant_light_closes_typed_edges_and_pins_builtin_meaning() {
        let fixture = Fixture::new();
        builtin_fixture(&fixture);
        let stage = fixture.stage(BUILTIN_PATHS);
        let candidate = validate_staged_candidate(&stage, &[]).unwrap();
        assert_eq!(candidate.records().games().len(), 1);
        assert_eq!(candidate.records().builtins().len(), 2);
        assert_eq!(candidate.records().required_edges(), 3);
        let scene = &candidate.records().scenes()[0];
        assert_eq!(scene.schema(), 2);
        assert_eq!(scene.environment_source(), Some(ENVIRONMENT_ID));
        assert_eq!(scene.label(), Some("entry"));
        let source = &candidate.records().builtins()[0];
        assert_eq!(source.id(), ENVIRONMENT_ID);
        assert_eq!(source.required_light(), Some(LIGHT_ID));
        assert_eq!(source.label(), Some("fixture light source"));
        assert_eq!(
            source.definition().record_type(),
            "thrum.environment.authored"
        );
        assert_eq!(source.definition().schema(), 1);
        assert_eq!(source.definition().descriptor(), ENVIRONMENT_DEFINITION);
        assert!(std::ptr::eq(
            source.definition(),
            builtin_schema("thrum.environment.authored", 1).unwrap()
        ));
        assert_eq!(source.path().as_str(), ENVIRONMENT_PATH);
        let light = &candidate.records().builtins()[1];
        assert_eq!(light.required_light(), None);
        assert_eq!(light.definition().descriptor(), LIGHT_DEFINITION);
        assert!(std::ptr::eq(
            light.definition(),
            builtin_schema("thrum.light.identity", 1).unwrap()
        ));
        assert_eq!(scene.definition().unwrap().descriptor(), SCENE_DEFINITION);
        assert!(std::ptr::eq(
            scene.definition().unwrap(),
            builtin_schema("core.scene", 2).unwrap()
        ));
        assert!(builtin_schema("core.scene", 3).is_none());
        assert!(builtin_schema("thrum.light.identity", 2).is_none());
        let identities: Vec<_> = candidate
            .provisional_manifest()
            .unwrap()
            .files()
            .iter()
            .filter_map(|file| {
                file.record_identity()
                    .map(|(id, kind)| (id.to_owned(), kind.to_owned()))
            })
            .collect();
        assert!(identities.contains(&(
            ENVIRONMENT_ID.to_owned(),
            "thrum.environment.authored".to_owned()
        )));
        assert!(identities.contains(&(LIGHT_ID.to_owned(), "thrum.light.identity".to_owned())));
        // Literal digests make meaning changes visible in review, independently
        // of a refactor in descriptor construction or staged-summary plumbing.
        let hashes: Vec<_> = BUILTIN_SCHEMAS
            .iter()
            .map(|definition| hex(definition.definition_hash()))
            .collect();
        assert_eq!(hashes, BUILTIN_HASHES);
    }

    #[test]
    fn staged_builtin_rejections_preserve_prior_capture_and_typed_namespace() {
        let fixture = Fixture::new();
        builtin_fixture(&fixture);
        let prior = fixture.stage(BUILTIN_PATHS);
        let prior_candidate = validate_staged_candidate(&prior, &[]).unwrap();
        let prior_revision = prior_candidate.provisional_manifest().unwrap().revision();
        let source = String::from_utf8(constant_light_bytes()).unwrap();
        let light = fs::read_to_string(fixture.0.join(LIGHT_PATH)).unwrap();
        for id in [
            PROJECT_ID,
            HISTORY_ID,
            SCHEMA_ID,
            SCENE_ID,
            GAME_ID,
            ENVIRONMENT_ID,
        ] {
            fixture.write(LIGHT_PATH, light.replace(LIGHT_ID, id).as_bytes());
            let candidate = fixture.stage(BUILTIN_PATHS);
            assert!(matches!(validate_staged_candidate(&candidate, &[]),
                Err(StagedCandidateError::Records(StagedRecordError::DuplicateRecordId(found))) if found == id));
        }
        fixture.write(LIGHT_PATH, light.as_bytes());
        for invalid in [
            source.replace(LIGHT_ID, SCENE_ID),
            source.replace("constant-light-v1", "authored-vertical-rain-v1"),
            source.replace("thrum.environment.authored", "thrum.environment.future"),
            source.replace(
                "thrum.environment.authored",
                "thrum.fixture.static_surface_receiver",
            ),
            source.replace("\"schema\":1", "\"schema\":2"),
            source.replace("thrum.environment.authored", "game.marker"),
        ] {
            fixture.write(ENVIRONMENT_PATH, invalid.as_bytes());
            let candidate = fixture.stage(BUILTIN_PATHS);
            assert!(validate_staged_candidate(&candidate, &[]).is_err());
            assert_eq!(
                validate_staged_candidate(&prior, &[])
                    .unwrap()
                    .provisional_manifest()
                    .unwrap()
                    .revision(),
                prior_revision
            );
        }
        fixture.write(ENVIRONMENT_PATH, source.as_bytes());
        let scene = fs::read_to_string(fixture.0.join("scenes/main.scene.json")).unwrap();
        fixture.write(
            "scenes/main.scene.json",
            scene.replace(ENVIRONMENT_ID, LIGHT_ID).as_bytes(),
        );
        let candidate = fixture.stage(BUILTIN_PATHS);
        assert!(matches!(validate_staged_candidate(&candidate, &[]),
            Err(StagedCandidateError::Records(StagedRecordError::UnresolvedEnvironmentSource { target_id, .. })) if target_id == LIGHT_ID));
        fixture.write("scenes/main.scene.json", scene.as_bytes());
        let game = fs::read_to_string(fixture.0.join("records/marker.record.json")).unwrap();
        fixture.write(
            "records/marker.record.json",
            game.replace("game.marker", "thrum.environment.authored")
                .as_bytes(),
        );
        let candidate = fixture.stage(BUILTIN_PATHS);
        assert!(matches!(
            validate_staged_candidate(&candidate, &[]),
            Err(StagedCandidateError::Records(
                StagedRecordError::Builtin { .. }
            ))
        ));
        fixture.write("records/marker.record.json", game.as_bytes());
        let schema = fs::read_to_string(fixture.0.join("schemas/marker.schema.json")).unwrap();
        fixture.write(
            "schemas/marker.schema.json",
            schema
                .replace("game.marker", "thrum.environment.authored")
                .as_bytes(),
        );
        let candidate = fixture.stage(BUILTIN_PATHS);
        assert!(matches!(
            validate_staged_candidate(&candidate, &[]),
            Err(StagedCandidateError::Registry(
                StagedRegistryError::Schema { .. }
            ))
        ));
        assert_eq!(
            prior_candidate.provisional_manifest().unwrap().revision(),
            prior_revision
        );
    }

    #[test]
    fn staged_production_dispatch_rejects_deep_and_dense_fields_before_excess_values() {
        let fixture = Fixture::new();
        basic(&fixture);
        let prior = fixture.stage(PATHS);
        let prior_revision = validate_staged_candidate(&prior, &[])
            .unwrap()
            .provisional_manifest()
            .unwrap()
            .revision();
        // The bodies after the forbidden depth/node boundary are deliberately
        // malformed. The header must refuse at the boundary before reading them.
        let deep = format!("{{\"unused\":{}{{\"unclosed\":", "[".repeat(63));
        let dense = format!(
            "{{\"unused\":[{}{{\"unclosed\":",
            "0,".repeat(crate::MAX_RECORD_NODES - 3)
        );
        for record_type in ["game.marker", "thrum.environment.authored"] {
            for (fields, expected_error) in [
                (&deep, "source header JSON nesting limit"),
                (&dense, "source header JSON value count limit"),
            ] {
                // Put fields first so malformed tails cannot hide an unbounded
                // traversal behind already-known discriminator keys.
                let bytes = format!(
                    r#"{{"fields":{fields},"id":"{GAME_ID}","type":"{record_type}","schema":1}}"#
                );
                fixture.write("records/marker.record.json", bytes.as_bytes());
                let changed = fixture.stage(PATHS);
                let error = validate_staged_candidate(&changed, &[]).err().unwrap();
                match error {
                    StagedCandidateError::Records(StagedRecordError::Envelope {
                        source, ..
                    }) => {
                        assert!(source.to_string().starts_with(expected_error), "{source}");
                    }
                    other => panic!("unexpected staged refusal: {other:?}"),
                }
                assert_eq!(
                    validate_staged_candidate(&prior, &[])
                        .unwrap()
                        .provisional_manifest()
                        .unwrap()
                        .revision(),
                    prior_revision
                );
            }
        }
    }

    #[test]
    fn staged_builtin_known_neighbor_edit_keeps_exact_opaque_bytes_and_changes_revision() {
        let fixture = Fixture::new();
        builtin_fixture(&fixture);
        let raw = r#"{ "number": 1.00, "escaped":"\u0061", "array":[true, null, -0] }"#;
        let source = String::from_utf8(constant_light_bytes()).unwrap().replace(
            "\"profile\":",
            &format!("\"x.thrum.environment.note\":{raw},\"profile\":"),
        );
        fixture.write(ENVIRONMENT_PATH, source.as_bytes());
        let prior = fixture.stage(BUILTIN_PATHS);
        let prior_candidate = validate_staged_candidate(&prior, &[]).unwrap();
        let prior_revision = prior_candidate.provisional_manifest().unwrap().revision();
        let edited = source.replace(
            "\"perpendicular_illuminance_lux\":100000",
            "\"perpendicular_illuminance_lux\":75000",
        );
        fixture.write(ENVIRONMENT_PATH, edited.as_bytes());
        let changed = fixture.stage(BUILTIN_PATHS);
        let candidate = validate_staged_candidate(&changed, &[]).unwrap();
        assert_ne!(
            candidate.provisional_manifest().unwrap().revision(),
            prior_revision
        );
        assert_eq!(
            candidate.records().builtins()[0]
                .definition()
                .definition_hash(),
            prior_candidate.records().builtins()[0]
                .definition()
                .definition_hash()
        );
        for (stage, expected) in [(&prior, source.as_bytes()), (&changed, edited.as_bytes())] {
            let bytes = read_record_bytes(
                stage,
                find_file(stage, &fixed_path(ENVIRONMENT_PATH)).unwrap(),
            )
            .unwrap();
            assert_eq!(bytes, expected);
            assert_eq!(
                environment_json::parse_environment(&bytes)
                    .unwrap()
                    .extensions,
                [("x.thrum.environment.note".to_owned(), raw.to_owned())]
            );
        }
        // Formatting alone is also an exact-byte source revision change.
        fixture.write(ENVIRONMENT_PATH, format!("{edited}\n").as_bytes());
        let formatted = fixture.stage(BUILTIN_PATHS);
        assert_ne!(
            validate_staged_candidate(&formatted, &[])
                .unwrap()
                .provisional_manifest()
                .unwrap()
                .revision(),
            candidate.provisional_manifest().unwrap().revision()
        );
    }

    #[test]
    fn builtin_only_project_needs_no_game_descriptor_and_single_entry_is_enforced() {
        let fixture = Fixture::new();
        builtin_fixture(&fixture);
        fs::remove_file(fixture.0.join("records/marker.record.json")).unwrap();
        fs::remove_file(fixture.0.join("schemas/marker.schema.json")).unwrap();
        let mut project: serde_json::Value =
            serde_json::from_slice(&fs::read(fixture.0.join("project.json")).unwrap()).unwrap();
        project["fields"]["project.records"] =
            json!([ENVIRONMENT_PATH, LIGHT_PATH, "scenes/main.scene.json"]);
        project["fields"]["project.schemas"] = json!([]);
        fixture.write("project.json", &serde_json::to_vec(&project).unwrap());
        fixture.write(
            "schemas/history.registry.json",
            &serde_json::to_vec(&json!({
                "id":HISTORY_ID, "type":"core.registry-history", "schema":1,
                "fields":{"registry.fields":[], "registry.versions":[]}
            }))
            .unwrap(),
        );
        let paths = [
            "project.json",
            ENVIRONMENT_PATH,
            LIGHT_PATH,
            "scenes/main.scene.json",
            "schemas/history.registry.json",
        ];
        let stage = fixture.stage(&paths);
        let candidate = validate_staged_candidate(&stage, &[]).unwrap();
        assert!(candidate.records().games().is_empty());
        assert!(candidate.registry().schemas().is_empty());
        assert_eq!(candidate.records().required_edges(), 2);
        // Declared but unselected content is valid and does not acquire an
        // active environment binding merely by appearing in project.records.
        fixture.write(
            "scenes/main.scene.json",
            &serde_json::to_vec(&json!({
                "id":SCENE_ID, "type":"core.scene", "schema":1, "fields":{}
            }))
            .unwrap(),
        );
        let unselected = fixture.stage(&paths);
        let unselected_candidate = validate_staged_candidate(&unselected, &[]).unwrap();
        assert_eq!(unselected_candidate.records().required_edges(), 1);
        assert_eq!(
            unselected_candidate.records().scenes()[0].environment_source(),
            None
        );
        let other_scene = "20000000-0000-4000-8000-000000000001";
        project["fields"]["project.records"] = json!([
            ENVIRONMENT_PATH,
            LIGHT_PATH,
            "scenes/main.scene.json",
            "scenes/second.scene.json"
        ]);
        let mut entries = vec![SCENE_ID, other_scene];
        entries.sort_unstable();
        project["fields"]["project.entryScenes"] = json!(entries);
        fixture.write("project.json", &serde_json::to_vec(&project).unwrap());
        fixture.write(
            "scenes/second.scene.json",
            &serde_json::to_vec(
                &json!({"id":other_scene,"type":"core.scene","schema":1,"fields":{}}),
            )
            .unwrap(),
        );
        let mut multiple_paths = paths.to_vec();
        multiple_paths.push("scenes/second.scene.json");
        let multiple = fixture.stage(&multiple_paths);
        let multiple_candidate = validate_staged_candidate(&multiple, &[]).unwrap();
        assert_eq!(multiple_candidate.records().required_edges(), 1);
        assert!(
            multiple_candidate
                .records()
                .scenes()
                .iter()
                .all(|scene| scene.environment_source().is_none())
        );
        // A non-entry scene's binding is inactive declared content. It still
        // has a required typed edge, but does not select another active Sun.
        let unused_scene = "40000000-0000-4000-8000-000000000001";
        project["fields"]["project.records"]
            .as_array_mut()
            .unwrap()
            .push(json!("scenes/unused.scene.json"));
        fixture.write("project.json", &serde_json::to_vec(&project).unwrap());
        fixture.write(
            "scenes/unused.scene.json",
            &serde_json::to_vec(&json!({
                "id":unused_scene, "type":"core.scene", "schema":2,
                "fields":{"environment.source":ENVIRONMENT_ID}
            }))
            .unwrap(),
        );
        multiple_paths.push("scenes/unused.scene.json");
        let unused = fixture.stage(&multiple_paths);
        assert_eq!(
            validate_staged_candidate(&unused, &[])
                .unwrap()
                .records()
                .required_edges(),
            2
        );
        // Declared sources are checked even when neither entry selects them.
        fixture.write(
            ENVIRONMENT_PATH,
            &String::from_utf8(constant_light_bytes())
                .unwrap()
                .replace(LIGHT_ID, SCENE_ID)
                .into_bytes(),
        );
        let broken = fixture.stage(&multiple_paths);
        assert!(matches!(
            validate_staged_candidate(&broken, &[]),
            Err(StagedCandidateError::Records(
                StagedRecordError::UnresolvedEnvironmentLight { .. }
            ))
        ));
        fixture.write(ENVIRONMENT_PATH, &constant_light_bytes());
        // Selecting an environment on either entry enables the first R0/E1
        // closure and therefore makes this multi-entry project unsupported.
        fixture.write(
            "scenes/main.scene.json",
            &serde_json::to_vec(&json!({
                "id":SCENE_ID, "type":"core.scene", "schema":2,
                "fields":{"environment.source":ENVIRONMENT_ID}
            }))
            .unwrap(),
        );
        let selected = fixture.stage(&multiple_paths);
        assert!(matches!(
            validate_staged_candidate(&selected, &[]),
            Err(StagedCandidateError::Records(
                StagedRecordError::EnvironmentEntrySceneCount(2)
            ))
        ));
    }

    #[test]
    fn builtin_required_edges_share_the_existing_project_edge_cap() {
        let fixture = Fixture::new();
        basic(&fixture);
        fs::remove_file(fixture.0.join("records/marker.record.json")).unwrap();
        let fields: Vec<_> = (0..128)
            .map(|i| {
                json!({
                    "id":format!("marker.r{i:03}"), "kind":"record-ref", "presence":"required",
                    "targetType":"core.scene", "resolution":"required"
                })
            })
            .collect();
        let schema = serde_json::to_vec(&json!({
            "id":SCHEMA_ID, "type":"core.schema", "schema":1,
            "fields":{"schema.targetType":"game.marker", "schema.targetVersion":1,
                "schema.fields":fields, "schema.optionalExtensionPrefix":null}
        }))
        .unwrap();
        fixture.write("schemas/marker.schema.json", &schema);
        let descriptor = parse_schema_descriptor(&schema).unwrap();
        fixture.write(
            "schemas/history.registry.json",
            &serde_json::to_vec(&json!({
                "id":HISTORY_ID, "type":"core.registry-history", "schema":1,
                "fields":{
                    "registry.fields":descriptor.fields().iter().map(|field| json!({
                        "typeId":"game.marker", "fieldId":field.id(),
                        "signature":hex(field.signature()), "retired":false
                    })).collect::<Vec<_>>(),
                    "registry.versions":[{"typeId":"game.marker", "targetVersion":1,
                        "definitionHash":hex(descriptor.definition_hash())}]
                }
            }))
            .unwrap(),
        );
        let game_fields: serde_json::Map<_, _> = (0..128)
            .map(|i| (format!("marker.r{i:03}"), json!(SCENE_ID)))
            .collect();
        let mut records = Vec::new();
        for index in 0..512 {
            let path = format!("records/g{index:04}.record.json");
            fixture.write(
                &path,
                &serde_json::to_vec(&json!({
                    "id":format!("30000000-0000-4000-8000-{index:012x}"),
                    "type":"game.marker", "schema":1, "fields":game_fields
                }))
                .unwrap(),
            );
            records.push(path);
        }
        records.push("scenes/main.scene.json".to_owned());
        let mut project = json!({
            "id":PROJECT_ID, "type":"core.project", "schema":2,
            "fields":{"project.format":1, "project.records":records,
                "project.schemas":["schemas/marker.schema.json"],
                "project.registryHistory":"schemas/history.registry.json", "project.entryScenes":[SCENE_ID]}
        });
        fixture.write("project.json", &serde_json::to_vec(&project).unwrap());
        let mut staged_paths = records.clone();
        staged_paths.extend([
            "project.json".to_owned(),
            "schemas/marker.schema.json".to_owned(),
            "schemas/history.registry.json".to_owned(),
        ]);
        let path_refs: Vec<_> = staged_paths.iter().map(String::as_str).collect();
        let prior = fixture.stage(&path_refs);
        assert_eq!(
            validate_staged_candidate(&prior, &[])
                .unwrap()
                .records()
                .required_edges(),
            65_536
        );
        write_environment_records(&fixture);
        records.extend([ENVIRONMENT_PATH.to_owned(), LIGHT_PATH.to_owned()]);
        records.sort_unstable();
        project["fields"]["project.records"] = json!(records);
        fixture.write("project.json", &serde_json::to_vec(&project).unwrap());
        staged_paths.extend([ENVIRONMENT_PATH.to_owned(), LIGHT_PATH.to_owned()]);
        let path_refs: Vec<_> = staged_paths.iter().map(String::as_str).collect();
        let changed = fixture.stage(&path_refs);
        let Err(error) = validate_staged_candidate(&changed, &[]) else {
            panic!("one more built-in reference must exceed the candidate profile");
        };
        assert_eq!(
            error.limit_exceeded(),
            Some(LimitExceeded {
                dimension: SourceDimension::RequiredReferences,
                allowed: 65_536,
                observed: ObservedQuantity::AtLeast(65_537),
            })
        );
        // The records/ paths exhaust the edge unit before the later scenes/
        // path. Its malformed local grammar must still win over the capacity
        // refusal, even though retained summaries have already been dropped.
        fixture.write(
            "scenes/main.scene.json",
            &serde_json::to_vec(&json!({
                "id":SCENE_ID, "type":"core.scene", "schema":1, "fields":[]
            }))
            .unwrap(),
        );
        let malformed_after_overflow = fixture.stage(&path_refs);
        assert!(matches!(
            validate_staged_candidate(&malformed_after_overflow, &[]),
            Err(StagedCandidateError::Records(StagedRecordError::Scene { path, .. }))
                if path.as_str() == "scenes/main.scene.json"
        ));
        fixture.write(
            "scenes/main.scene.json",
            &serde_json::to_vec(&json!({
                "id":SCENE_ID, "type":"core.scene", "schema":1, "fields":{}
            }))
            .unwrap(),
        );
        for (path, index, fields) in [
            ("records/z001.record.json", 512, json!(game_fields)),
            ("records/z002.record.json", 513, json!([])),
        ] {
            fixture.write(
                path,
                &serde_json::to_vec(&json!({
                    "id":format!("30000000-0000-4000-8000-{index:012x}"),
                    "type":"game.marker", "schema":1, "fields":fields
                }))
                .unwrap(),
            );
            records.push(path.to_owned());
            staged_paths.push(path.to_owned());
        }
        records.sort_unstable();
        project["fields"]["project.records"] = json!(records);
        fixture.write("project.json", &serde_json::to_vec(&project).unwrap());
        let later_path_refs: Vec<_> = staged_paths.iter().map(String::as_str).collect();
        let malformed_game_after_overflow = fixture.stage(&later_path_refs);
        assert!(matches!(
            validate_staged_candidate(&malformed_game_after_overflow, &[]),
            Err(StagedCandidateError::Records(StagedRecordError::Game { path, .. }))
                if path.as_str() == "records/z002.record.json"
        ));
        fixture.write(
            "records/z002.record.json",
            &serde_json::to_vec(&json!({
                "id":"30000000-0000-4000-8000-000000000201",
                "type":"game.marker", "schema":1, "fields":game_fields
            }))
            .unwrap(),
        );
        let valid_but_overflowed = fixture.stage(&later_path_refs);
        assert!(matches!(
            validate_staged_candidate(&valid_but_overflowed, &[]),
            Err(StagedCandidateError::Records(
                StagedRecordError::TooManyRequiredEdges { .. }
            ))
        ));
        assert_eq!(
            validate_staged_candidate(&prior, &[])
                .unwrap()
                .records()
                .required_edges(),
            65_536
        );
    }

    const BUILTIN_HASHES: [&str; 3] = [
        "75c09153b459c576f89ad3328dc6e3bac82379368cd7e3ac9317f058a2ea8aa7",
        "8f3d060dcd716a40ebfeddcdae58cc7285d281b6bba24fa9a38ebf2000ab5cfb",
        "9376fbe1d51794e5cae447c41ae0d02a71cd3f652830c9b650375144ac97b166",
    ];

    #[test]
    fn two_game_descriptors_dispatch_by_type_and_preserve_staged_rejection() {
        const SIGNAL_ID: &str = "7a5ef644-7225-4770-9f4d-e1d9c0a7d515";
        const SIGNAL_SCHEMA_ID: &str = "8a5ef644-7225-4770-9f4d-e1d9c0a7d515";
        const SIGNAL_PATH: &str = "records/signal.record.json";
        const SIGNAL_SCHEMA_PATH: &str = "schemas/signal.schema.json";
        let fixture = Fixture::new();
        basic(&fixture);
        let mut project: serde_json::Value =
            serde_json::from_slice(&fs::read(fixture.0.join("project.json")).unwrap()).unwrap();
        project["fields"]["project.records"]
            .as_array_mut()
            .unwrap()
            .insert(1, json!(SIGNAL_PATH));
        project["fields"]["project.schemas"]
            .as_array_mut()
            .unwrap()
            .push(json!(SIGNAL_SCHEMA_PATH));
        fixture.write("project.json", &serde_json::to_vec(&project).unwrap());
        let schema_bytes = serde_json::to_vec(&json!({
            "id":SIGNAL_SCHEMA_ID, "type":"core.schema", "schema":1,
            "fields":{
                "schema.targetType":"game.signal", "schema.targetVersion":1,
                "schema.fields":[
                    {"id":"signal.note", "kind":"text", "presence":"optional", "maxUtf8Bytes":64}
                ],
                "schema.optionalExtensionPrefix":null
            }
        }))
        .unwrap();
        fixture.write(SIGNAL_SCHEMA_PATH, &schema_bytes);
        let signal_descriptor = parse_schema_descriptor(&schema_bytes).unwrap();
        let mut history: serde_json::Value = serde_json::from_slice(
            &fs::read(fixture.0.join("schemas/history.registry.json")).unwrap(),
        )
        .unwrap();
        history["fields"]["registry.fields"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "typeId":"game.signal", "fieldId":"signal.note",
                "signature":hex(signal_descriptor.fields()[0].signature()), "retired":false
            }));
        history["fields"]["registry.versions"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "typeId":"game.signal", "targetVersion":1,
                "definitionHash":hex(signal_descriptor.definition_hash())
            }));
        fixture.write(
            "schemas/history.registry.json",
            &serde_json::to_vec(&history).unwrap(),
        );
        let signal = serde_json::to_vec(&json!({
            "fields":{"signal.note":"nearby"},
            "id":SIGNAL_ID, "schema":1, "type":"game.signal"
        }))
        .unwrap();
        fixture.write(SIGNAL_PATH, &signal);
        let paths = [
            "project.json",
            "records/marker.record.json",
            SIGNAL_PATH,
            "scenes/main.scene.json",
            "schemas/history.registry.json",
            "schemas/marker.schema.json",
            SIGNAL_SCHEMA_PATH,
        ];
        let old = fixture.stage(&paths);
        let candidate = validate_staged_candidate(&old, &[]).unwrap();
        assert_eq!(candidate.registry().schemas().len(), 2);
        assert_eq!(candidate.records().required_edges(), 1);
        let types: Vec<_> = candidate
            .records()
            .games()
            .iter()
            .map(|game| game.summary().record_type())
            .collect();
        assert_eq!(types, ["game.marker", "game.signal"]);
        assert_eq!(candidate.provisional_manifest().unwrap().files().len(), 7);

        let wrong_type = String::from_utf8(signal.clone())
            .unwrap()
            .replace("game.signal", "game.unknown");
        fixture.write(SIGNAL_PATH, wrong_type.as_bytes());
        assert_eq!(candidate.records().games().len(), 2);
        let changed = fixture.stage(&paths);
        assert!(matches!(
            validate_staged_candidate(&changed, &[]),
            Err(StagedCandidateError::Records(StagedRecordError::UnknownGameType { record_type, .. }))
                if record_type == "game.unknown"
        ));
        let wrong_version = String::from_utf8(signal)
            .unwrap()
            .replace("\"schema\":1", "\"schema\":2");
        fixture.write(SIGNAL_PATH, wrong_version.as_bytes());
        let changed = fixture.stage(&paths);
        assert!(matches!(
            validate_staged_candidate(&changed, &[]),
            Err(StagedCandidateError::Records(
                StagedRecordError::Game { .. }
            ))
        ));
    }

    #[test]
    fn declared_stage_registry_checks_history_and_keeps_old_capture() {
        let fixture = Fixture::new();
        basic(&fixture);
        let old = fixture.stage(PATHS);
        let profile = validate_staged_registry(&old, &[]).unwrap();
        assert_eq!(profile.project().id(), PROJECT_ID);
        assert_eq!(
            profile.schemas()[0].descriptor().target_type(),
            "game.marker"
        );
        assert_eq!(profile.history_id(), HISTORY_ID);
        assert_eq!(profile.stage().files().len(), PATHS.len());

        let path = "schemas/marker.schema.json";
        let schema = fs::read_to_string(fixture.0.join(path)).unwrap();
        fixture.write(path, schema.replace("1024", "2048").as_bytes());
        assert!(validate_staged_registry(&old, &[]).is_ok());
        let new = fixture.stage(PATHS);
        assert!(matches!(
            validate_staged_registry(&new, &[]),
            Err(StagedRegistryError::Registry(
                RegistryConsistencyError::ChangedFieldMeaning(0)
            ))
        ));
    }

    #[test]
    fn staged_history_lineage_preserves_retired_fields_and_old_versions() {
        let fixture = Fixture::new();
        basic(&fixture);
        let history_path = "schemas/history.registry.json";
        let base_bytes = fs::read(fixture.0.join(history_path)).unwrap();
        let base_stage = fixture.stage(PATHS);
        let base = validate_staged_registry(&base_stage, &[]).unwrap();

        let mut extended: serde_json::Value = serde_json::from_slice(&base_bytes).unwrap();
        extended["fields"]["registry.fields"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "typeId":"game.zzz", "fieldId":"old.value",
                "signature":"00".repeat(32), "retired":true
            }));
        extended["fields"]["registry.versions"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "typeId":"game.zzz", "targetVersion":1,
                "definitionHash":"00".repeat(32)
            }));
        fixture.write(history_path, &serde_json::to_vec(&extended).unwrap());
        let extended_stage = fixture.stage(PATHS);
        let extended_registry = validate_staged_registry(&extended_stage, &[]).unwrap();
        assert!(compare_staged_histories(&base, &extended_registry).is_ok());
        assert!(matches!(
            compare_staged_histories(&extended_registry, &base),
            Err(StagedLineageError::Conflict(CarriedHistoryError::Field(_)))
        ));

        let mut version_only: serde_json::Value = serde_json::from_slice(&base_bytes).unwrap();
        version_only["fields"]["registry.versions"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "typeId":"game.zzz", "targetVersion":1,
                "definitionHash":"00".repeat(32)
            }));
        fixture.write(history_path, &serde_json::to_vec(&version_only).unwrap());
        let version_stage = fixture.stage(PATHS);
        let version_registry = validate_staged_registry(&version_stage, &[]).unwrap();
        assert!(matches!(
            compare_staged_histories(&version_registry, &base),
            Err(StagedLineageError::Conflict(CarriedHistoryError::Version(
                _
            )))
        ));
    }

    #[test]
    fn missing_unlisted_unsupported_and_dependency_paths_refuse() {
        let fixture = Fixture::new();
        basic(&fixture);
        let full = fixture.stage(PATHS);
        assert!(matches!(
            validate_staged_registry(&full, &["external".to_owned()]),
            Err(StagedRegistryError::UnsupportedDependencyRoots)
        ));
        let omitted_path = "schemas/marker.schema.json";
        let omitted_bytes = fs::read(fixture.0.join(omitted_path)).unwrap();
        fs::remove_file(fixture.0.join(omitted_path)).unwrap();
        let omitted = fixture.stage(&PATHS[..PATHS.len() - 1]);
        assert!(matches!(
            validate_staged_registry(&omitted, &[]),
            Err(StagedRegistryError::MissingRecord(_))
        ));
        fixture.write(omitted_path, &omitted_bytes);
        fixture.write("records/extra.record.json", b"{}");
        let mut paths = PATHS.to_vec();
        paths.push("records/extra.record.json");
        let unlisted = fixture.stage(&paths);
        assert!(matches!(
            validate_staged_registry(&unlisted, &[]),
            Err(StagedRegistryError::UnlistedRecord(_))
        ));
        fs::remove_file(fixture.0.join("records/extra.record.json")).unwrap();
        fixture.write("scenes/hidden.prefab.json", b"{}");
        let mut unsupported_paths = PATHS.to_vec();
        unsupported_paths.push("scenes/hidden.prefab.json");
        let unsupported = fixture.stage(&unsupported_paths);
        assert!(matches!(
            validate_staged_registry(&unsupported, &[]),
            Err(StagedRegistryError::UnsupportedRecord(_))
        ));
    }

    #[test]
    fn duplicate_registry_identity_and_target_type_refuse() {
        let fixture = Fixture::new();
        basic(&fixture);
        let history_path = "schemas/history.registry.json";
        let history = fs::read_to_string(fixture.0.join(history_path)).unwrap();
        fixture.write(
            history_path,
            history.replace(HISTORY_ID, PROJECT_ID).as_bytes(),
        );
        let duplicate_id = fixture.stage(PATHS);
        assert!(matches!(
            validate_staged_registry(&duplicate_id, &[]),
            Err(StagedRegistryError::DuplicateRecordId(id)) if id == PROJECT_ID
        ));

        fixture.write(history_path, history.as_bytes());
        let other_path = "schemas/other.schema.json";
        let schema = fs::read_to_string(fixture.0.join("schemas/marker.schema.json")).unwrap();
        fixture.write(other_path, schema.replace(SCHEMA_ID, GAME_ID).as_bytes());
        let project_path = "project.json";
        let project = fs::read_to_string(fixture.0.join(project_path)).unwrap();
        fixture.write(
            project_path,
            project
                .replace(
                    "\"schemas/marker.schema.json\"",
                    "\"schemas/marker.schema.json\",\"schemas/other.schema.json\"",
                )
                .as_bytes(),
        );
        let mut paths = PATHS.to_vec();
        paths.push(other_path);
        let duplicate_type = fixture.stage(&paths);
        assert!(matches!(
            validate_staged_registry(&duplicate_type, &[]),
            Err(StagedRegistryError::Registry(
                RegistryConsistencyError::DescriptorOrder(1)
            ))
        ));
        assert!(matches!(
            validate_staged_candidate(&duplicate_type, &[]),
            Err(StagedCandidateError::Registry(
                StagedRegistryError::Registry(RegistryConsistencyError::DescriptorOrder(1))
            ))
        ));
    }

    #[test]
    fn candidate_registry_identity_collision_follows_declared_scene_structure() {
        let fixture = Fixture::new();
        basic(&fixture);
        let history_path = "schemas/history.registry.json";
        let history = fs::read_to_string(fixture.0.join(history_path)).unwrap();
        fixture.write(
            history_path,
            history.replace(HISTORY_ID, PROJECT_ID).as_bytes(),
        );

        for schema in [1, 2] {
            let scene_path = "scenes/main.scene.json";
            let mut scene = json!({
                "id":SCENE_ID, "type":"core.scene", "schema":schema, "fields":[]
            });
            fixture.write(scene_path, &serde_json::to_vec(&scene).unwrap());
            let malformed = fixture.stage(PATHS);
            assert!(matches!(
                validate_staged_candidate(&malformed, &[]),
                Err(StagedCandidateError::Records(StagedRecordError::Scene { path, .. }))
                    if path.as_str() == scene_path
            ));
            assert!(matches!(
                validate_staged_registry(&malformed, &[]),
                Err(StagedRegistryError::DuplicateRecordId(id)) if id == PROJECT_ID
            ));

            scene["fields"] = json!({});
            fixture.write(scene_path, &serde_json::to_vec(&scene).unwrap());
            let valid_structure = fixture.stage(PATHS);
            assert!(matches!(
                validate_staged_candidate(&valid_structure, &[]),
                Err(StagedCandidateError::Records(StagedRecordError::DuplicateRecordId(id)))
                    if id == PROJECT_ID
            ));
            assert!(matches!(
                validate_staged_registry(&valid_structure, &[]),
                Err(StagedRegistryError::DuplicateRecordId(id)) if id == PROJECT_ID
            ));
        }
    }

    #[test]
    fn registry_and_candidate_collision_passes_share_canonical_choice() {
        for mut ids in [["b", "a", "b", "a"], ["a", "b", "a", "b"]] {
            assert_eq!(first_duplicate_record_id(&mut ids), Some("a"));
        }
    }

    #[test]
    fn staged_scene_game_and_required_reference_close_over_captured_bytes() {
        let fixture = Fixture::new();
        basic(&fixture);
        let old = fixture.stage(PATHS);
        let registry = validate_staged_registry(&old, &[]).unwrap();
        let records = validate_staged_records(&registry).unwrap();
        assert_eq!(records.scenes().len(), 1);
        assert_eq!(records.scenes()[0].id(), SCENE_ID);
        assert_eq!(records.games().len(), 1);
        assert_eq!(records.games()[0].summary().id(), GAME_ID);
        assert_eq!(records.games()[0].summary().record_type(), "game.marker");
        assert_eq!(
            records.games()[0].summary().definition_hash(),
            registry.schemas()[0].descriptor().definition_hash()
        );
        assert_eq!(records.required_edges(), 1);

        let path = "records/marker.record.json";
        let game = fs::read_to_string(fixture.0.join(path)).unwrap();
        fixture.write(path, game.replace(SCENE_ID, PROJECT_ID).as_bytes());
        // A captured candidate still reads its own exact bytes after an edit.
        assert!(validate_staged_records(&registry).is_ok());
        let new = fixture.stage(PATHS);
        let registry = validate_staged_registry(&new, &[]).unwrap();
        assert!(matches!(
            validate_staged_records(&registry),
            Err(StagedRecordError::UnresolvedGameScene { target_id, .. }) if target_id == PROJECT_ID
        ));
    }

    #[test]
    fn staged_record_identity_and_schema_errors_refuse() {
        let fixture = Fixture::new();
        basic(&fixture);
        let path = "records/marker.record.json";
        let game = fs::read_to_string(fixture.0.join(path)).unwrap();
        fixture.write(path, game.replace(GAME_ID, SCENE_ID).as_bytes());
        let stage = fixture.stage(PATHS);
        let registry = validate_staged_registry(&stage, &[]).unwrap();
        assert!(matches!(
            validate_staged_records(&registry),
            Err(StagedRecordError::DuplicateRecordId(id)) if id == SCENE_ID
        ));

        fixture.write(
            path,
            game.replace("\"game.marker\"", "\"game.other\"").as_bytes(),
        );
        let stage = fixture.stage(PATHS);
        let registry = validate_staged_registry(&stage, &[]).unwrap();
        assert!(matches!(
            validate_staged_records(&registry),
            Err(StagedRecordError::Game { .. })
        ));

        fixture.write(path, game.as_bytes());
        let project_path = "project.json";
        let project = fs::read_to_string(fixture.0.join(project_path)).unwrap();
        fixture.write(project_path, project.replace(SCENE_ID, GAME_ID).as_bytes());
        let stage = fixture.stage(PATHS);
        let registry = validate_staged_registry(&stage, &[]).unwrap();
        assert!(matches!(
            validate_staged_records(&registry),
            Err(StagedRecordError::UnresolvedEntryScene(id)) if id == GAME_ID
        ));
    }

    #[test]
    fn checked_candidate_revision_matches_independent_manifest_and_changes_on_edit() {
        let fixture = Fixture::new();
        basic(&fixture);
        fixture.write("assets/note.txt", b"candidate note");
        let mut paths = vec!["assets/note.txt"];
        paths.extend_from_slice(PATHS);
        let old = fixture.stage(&paths);
        let candidate = validate_staged_candidate(&old, &[]).unwrap();
        let revision = candidate.provisional_manifest().unwrap().revision();
        let files: Vec<_> = old
            .files()
            .iter()
            .map(|file| {
                let (id, record_type) = match file.path().as_str() {
                    "project.json" => (Some(PROJECT_ID), Some("core.project")),
                    "records/marker.record.json" => (Some(GAME_ID), Some("game.marker")),
                    "scenes/main.scene.json" => (Some(SCENE_ID), Some("core.scene")),
                    "schemas/history.registry.json" => {
                        (Some(HISTORY_ID), Some("core.registry-history"))
                    }
                    "schemas/marker.schema.json" => (Some(SCHEMA_ID), Some("core.schema")),
                    _ => (None, None),
                };
                json!({
                    "hash":hex(file.sha256()), "kind":"file", "length":file.length(),
                    "path":file.path().as_str(), "recordId":id, "recordType":record_type
                })
            })
            .collect();
        let oracle = json!({
            "dependencyRoots":[], "files":files, "format":1, "projectId":PROJECT_ID
        });
        let oracle_bytes = serde_json_canonicalizer::to_vec(&oracle).unwrap();
        assert_eq!(
            revision.as_bytes().as_slice(),
            Sha256::digest(oracle_bytes).as_slice()
        );

        let path = "records/marker.record.json";
        let game = fs::read_to_string(fixture.0.join(path)).unwrap();
        fixture.write(
            path,
            game.replace("first marker", "second marker").as_bytes(),
        );
        assert_eq!(
            candidate.provisional_manifest().unwrap().revision(),
            revision
        );
        let new = fixture.stage(&paths);
        let candidate = validate_staged_candidate(&new, &[]).unwrap();
        assert_ne!(
            candidate.provisional_manifest().unwrap().revision(),
            revision
        );
    }

    #[test]
    fn text_edit_changes_candidate_revision_without_rewriting_opaque_value() {
        let fixture = Fixture::new();
        basic(&fixture);
        let path = "records/marker.record.json";
        let source = format!(
            "{{\"id\":\"{GAME_ID}\",\"type\":\"game.marker\",\"schema\":1,\"fields\":{{\"marker.caption\":\"before\",\"marker.scene\":\"{SCENE_ID}\",\"x.game.marker.notes\":{{\"escaped\":\"\\u0061\", \"list\":[1,  2]}}}}}}"
        );
        fixture.write(path, source.as_bytes());
        let old = fixture.stage(PATHS);
        let old_candidate = validate_staged_candidate(&old, &[]).unwrap();
        let old_revision = old_candidate.provisional_manifest().unwrap().revision();
        let edited = replace_game_text_field(
            source.as_bytes(),
            old_candidate.registry().schemas()[0].descriptor(),
            "marker.caption",
            "after",
        )
        .unwrap();
        fixture.write(path, &edited);
        let new = fixture.stage(PATHS);
        let new_candidate = validate_staged_candidate(&new, &[]).unwrap();
        assert_ne!(
            new_candidate.provisional_manifest().unwrap().revision(),
            old_revision
        );
        assert_eq!(
            old_candidate.provisional_manifest().unwrap().revision(),
            old_revision
        );
        let edited_text = std::str::from_utf8(&edited).unwrap();
        assert!(edited_text.contains(r#"{"escaped":"\u0061", "list":[1,  2]}"#));
        assert_eq!(new_candidate.records().games()[0].summary().id(), GAME_ID);
        assert_eq!(new_candidate.records().required_edges(), 1);
    }

    #[test]
    #[ignore = "local small staged-registry validation cost probe"]
    fn measure_small_staged_registry() {
        let fixture = Fixture::new();
        basic(&fixture);
        let stage = fixture.stage(PATHS);
        let start = std::time::Instant::now();
        for _ in 0..1_000 {
            std::hint::black_box(validate_staged_registry(&stage, &[]).unwrap());
        }
        eprintln!("iterations=1000 elapsed={:?}", start.elapsed());
    }

    #[test]
    #[ignore = "local small staged-record closure cost probe"]
    fn measure_small_staged_record_closure() {
        let fixture = Fixture::new();
        basic(&fixture);
        let stage = fixture.stage(PATHS);
        let registry = validate_staged_registry(&stage, &[]).unwrap();
        let start = std::time::Instant::now();
        for _ in 0..1_000 {
            std::hint::black_box(validate_staged_records(&registry).unwrap());
        }
        eprintln!("iterations=1000 elapsed={:?}", start.elapsed());
    }

    #[test]
    #[ignore = "local small provisional candidate revision cost probe"]
    fn measure_small_candidate_revision() {
        let fixture = Fixture::new();
        basic(&fixture);
        let stage = fixture.stage(PATHS);
        let start = std::time::Instant::now();
        for _ in 0..1_000 {
            let candidate = validate_staged_candidate(&stage, &[]).unwrap();
            std::hint::black_box(candidate.provisional_manifest().unwrap().revision());
        }
        eprintln!("iterations=1000 elapsed={:?}", start.elapsed());
    }

    #[test]
    #[ignore = "local maximum declared-record candidate cost probe"]
    fn measure_4096_record_candidate_revision() {
        let fixture = Fixture::new();
        basic(&fixture);
        let mut record_paths = vec![
            "records/marker.record.json".to_owned(),
            "scenes/main.scene.json".to_owned(),
        ];
        let mut staged_paths: Vec<String> = PATHS.iter().map(|path| (*path).to_owned()).collect();
        for index in 1..4_095_u32 {
            let path = format!("scenes/scene-{index:04}.scene.json");
            let id = format!("7a5ef644-7225-4770-9f4d-{index:012x}");
            fixture.write(
                &path,
                &serde_json::to_vec(&json!({
                    "id":id, "type":"core.scene", "schema":1, "fields":{}
                }))
                .unwrap(),
            );
            record_paths.push(path.clone());
            staged_paths.push(path);
        }
        record_paths.sort_unstable();
        fixture.write(
            "project.json",
            &serde_json::to_vec(&json!({
                "id":PROJECT_ID, "type":"core.project", "schema":2,
                "fields":{
                    "project.format":1,
                    "project.records":record_paths,
                    "project.schemas":["schemas/marker.schema.json"],
                    "project.registryHistory":"schemas/history.registry.json",
                    "project.entryScenes":[SCENE_ID]
                }
            }))
            .unwrap(),
        );
        let staged_refs: Vec<_> = staged_paths.iter().map(String::as_str).collect();
        let stage = fixture.stage(&staged_refs);
        let start = std::time::Instant::now();
        let candidate = validate_staged_candidate(&stage, &[]).unwrap();
        std::hint::black_box(candidate.provisional_manifest().unwrap().revision());
        eprintln!("records=4096 elapsed={:?}", start.elapsed());
    }

    #[derive(Clone, Copy, Eq, PartialEq)]
    enum DenseShape {
        OneGame,
        MaxEdges,
        ExcessEdges,
    }

    fn write_dense_reference_game(
        fixture: &Fixture,
        path: &str,
        id: &str,
        record_type: &str,
        field_prefix: &str,
        seed: usize,
    ) {
        let mut game = BufWriter::new(fs::File::create(fixture.0.join(path)).unwrap());
        write!(
            game,
            "{{\"id\":\"{id}\",\"type\":\"{record_type}\",\"schema\":1,\"fields\":{{"
        )
        .unwrap();
        for field in 0..128 {
            if field != 0 {
                game.write_all(b",").unwrap();
            }
            let scene_index = (seed * 128 + field) % 3_583;
            let target = if scene_index == 0 {
                SCENE_ID.to_owned()
            } else {
                format!("7a5ef644-7225-4770-9f4d-{scene_index:012x}")
            };
            write!(game, "\"{field_prefix}.r{field:03}\":\"{target}\"").unwrap();
        }
        game.write_all(b"}}").unwrap();
        game.flush().unwrap();
    }

    fn dense_fixture(schema_count: usize, shape: DenseShape) -> (Fixture, StagedTree) {
        assert!((1..=256).contains(&schema_count));
        assert!(shape == DenseShape::OneGame || schema_count == 256);
        let fixture = Fixture::new();
        basic(&fixture);

        let schema_path = "schemas/marker.schema.json";
        let mut schema: serde_json::Value =
            serde_json::from_slice(&fs::read(fixture.0.join(schema_path)).unwrap()).unwrap();
        let fields = schema["fields"]["schema.fields"].as_array_mut().unwrap();
        if shape == DenseShape::OneGame {
            for index in 0..126 {
                fields.push(json!({
                    "id":format!("marker.extra{index:03}"), "kind":"text",
                    "presence":"optional", "maxUtf8Bytes":64
                }));
            }
        } else {
            fields.clear();
            fields.extend((0..128).map(|field| {
                json!({
                    "id":format!("marker.r{field:03}"), "kind":"record-ref",
                    "presence":"required", "targetType":"core.scene", "resolution":"required"
                })
            }));
        }
        fields.sort_unstable_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
        let schema_bytes = serde_json::to_vec(&schema).unwrap();
        let descriptor = parse_schema_descriptor(&schema_bytes).unwrap();
        fixture.write(schema_path, &schema_bytes);
        let mut descriptors = vec![descriptor];
        let mut schema_paths = vec![schema_path.to_owned()];
        for index in 1..schema_count {
            let path = format!("schemas/type{index:03}.schema.json");
            let type_id = format!("game.type{index:03}");
            let record_id = format!("8a5ef644-7225-4770-9f4d-{index:012x}");
            let fields: Vec<_> = (0..128)
                .map(|field| match shape {
                    DenseShape::OneGame => json!({
                        "id":format!("value.f{field:03}"), "kind":"text",
                        "presence":"optional", "maxUtf8Bytes":64
                    }),
                    DenseShape::MaxEdges | DenseShape::ExcessEdges => json!({
                        "id":format!("value.r{field:03}"), "kind":"record-ref",
                        "presence":"required", "targetType":"core.scene", "resolution":"required"
                    }),
                })
                .collect();
            let bytes = serde_json::to_vec(&json!({
                "id":record_id, "type":"core.schema", "schema":1,
                "fields":{
                    "schema.targetType":type_id, "schema.targetVersion":1,
                    "schema.fields":fields, "schema.optionalExtensionPrefix":null
                }
            }))
            .unwrap();
            descriptors.push(parse_schema_descriptor(&bytes).unwrap());
            fixture.write(&path, &bytes);
            schema_paths.push(path);
        }

        let history_path = fixture.0.join("schemas/history.registry.json");
        let mut history = BufWriter::new(fs::File::create(history_path).unwrap());
        write!(history, "{{\"id\":\"{HISTORY_ID}\",\"type\":\"core.registry-history\",\"schema\":1,\"fields\":{{\"registry.fields\":[").unwrap();
        let mut first = true;
        let mut active_fields = 0;
        for descriptor in &descriptors {
            for field in descriptor.fields() {
                if !first {
                    history.write_all(b",").unwrap();
                }
                first = false;
                active_fields += 1;
                write!(history, "{{\"typeId\":\"{}\",\"fieldId\":\"{}\",\"signature\":\"{}\",\"retired\":false}}", descriptor.target_type(), field.id(), hex(field.signature())).unwrap();
            }
        }
        for index in 0..(100_000 - active_fields) {
            write!(history, ",{{\"typeId\":\"game.zzz\",\"fieldId\":\"old.f{index:05}\",\"signature\":\"{}\",\"retired\":true}}", "00".repeat(32)).unwrap();
        }
        history.write_all(b"],\"registry.versions\":[").unwrap();
        for (index, descriptor) in descriptors.iter().enumerate() {
            if index != 0 {
                history.write_all(b",").unwrap();
            }
            write!(
                history,
                "{{\"typeId\":\"{}\",\"targetVersion\":1,\"definitionHash\":\"{}\"}}",
                descriptor.target_type(),
                hex(descriptor.definition_hash())
            )
            .unwrap();
        }
        for version in 1..=(1024 - descriptors.len()) {
            write!(history, ",{{\"typeId\":\"game.zzz\",\"targetVersion\":{version},\"definitionHash\":\"{}\"}}", "00".repeat(32)).unwrap();
        }
        history.write_all(b"]}}").unwrap();
        history.flush().unwrap();
        drop(history);

        let mut record_paths = vec![
            "records/marker.record.json".to_owned(),
            "scenes/main.scene.json".to_owned(),
        ];
        let mut staged_paths: Vec<String> = PATHS.iter().map(|path| (*path).to_owned()).collect();
        staged_paths.extend(schema_paths.iter().skip(1).cloned());
        let game_count = if shape == DenseShape::OneGame {
            let game_path = fixture.0.join("records/marker.record.json");
            let mut game = BufWriter::new(fs::File::create(game_path).unwrap());
            write!(game, "{{\"id\":\"{GAME_ID}\",\"type\":\"game.marker\",\"schema\":1,\"fields\":{{\"marker.scene\":\"{SCENE_ID}\"").unwrap();
            let large_value = "v".repeat(65_000);
            for index in 0..128 {
                write!(game, ",\"x.game.marker.note{index:03}\":\"{large_value}\"").unwrap();
            }
            game.write_all(b"}}").unwrap();
            game.flush().unwrap();
            1
        } else {
            write_dense_reference_game(
                &fixture,
                "records/marker.record.json",
                GAME_ID,
                "game.marker",
                "marker",
                0,
            );
            for index in 0..schema_count {
                let record_type = if index == 0 {
                    "game.marker".to_owned()
                } else {
                    format!("game.type{index:03}")
                };
                let field_prefix = if index == 0 { "marker" } else { "value" };
                for copy in 0..2 {
                    if index == 0 && copy == 0 {
                        continue;
                    }
                    let path = if index == 0 {
                        format!("records/marker-{copy}.record.json")
                    } else {
                        format!("records/type{index:03}-{copy}.record.json")
                    };
                    let id = format!("9a5ef644-7225-4770-9f4d-{:012x}", index * 2 + copy);
                    write_dense_reference_game(
                        &fixture,
                        &path,
                        &id,
                        &record_type,
                        field_prefix,
                        index * 2 + copy,
                    );
                    record_paths.push(path.clone());
                    staged_paths.push(path);
                }
            }
            if shape == DenseShape::ExcessEdges {
                let path = "records/marker-2.record.json";
                let id = "9a5ef644-7225-4770-9f4d-000000000200";
                write_dense_reference_game(&fixture, path, id, "game.marker", "marker", 512);
                record_paths.push(path.to_owned());
                staged_paths.push(path.to_owned());
            }
            schema_count * 2 + usize::from(shape == DenseShape::ExcessEdges)
        };
        for index in 1..(4_096 - game_count) {
            let path = format!("scenes/scene-{index:04}.scene.json");
            let id = format!("7a5ef644-7225-4770-9f4d-{index:012x}");
            fixture.write(
                &path,
                &serde_json::to_vec(&json!({
                    "id":id, "type":"core.scene", "schema":1, "fields":{}
                }))
                .unwrap(),
            );
            record_paths.push(path.clone());
            staged_paths.push(path);
        }
        record_paths.sort_unstable();
        fixture.write(
            "project.json",
            &serde_json::to_vec(&json!({
                "id":PROJECT_ID, "type":"core.project", "schema":2,
                "fields":{
                    "project.format":1,
                    "project.records":record_paths,
                    "project.schemas":schema_paths,
                    "project.registryHistory":"schemas/history.registry.json",
                    "project.entryScenes":[SCENE_ID]
                }
            }))
            .unwrap(),
        );
        let staged_refs: Vec<_> = staged_paths.iter().map(String::as_str).collect();
        drop(descriptors);
        let declared: Vec<_> = staged_refs
            .iter()
            .map(|path| SourcePath::parse(path).unwrap())
            .collect();
        let stage = stage_declared_tree_with(
            &fixture.0,
            &declared,
            Limits {
                main: 2 * 1024 * 1024 * 1024,
                wal: 4 * 1024 * 1024,
                temporary: 256 * 1024 * 1024,
                free_floor: 0,
            },
            |_| {},
        )
        .unwrap();
        (fixture, stage)
    }

    #[test]
    #[ignore = "local one-schema dense candidate cost probe"]
    fn measure_dense_combined_candidate() {
        let (_fixture, stage) = dense_fixture(1, DenseShape::OneGame);
        let start = std::time::Instant::now();
        let candidate = validate_staged_candidate(&stage, &[]).unwrap();
        std::hint::black_box(candidate.provisional_manifest().unwrap().revision());
        eprintln!(
            "files={} copied_bytes={} elapsed={:?}",
            stage.files().len(),
            stage.copied_bytes(),
            start.elapsed()
        );
    }

    #[test]
    #[ignore = "local 256-schema dense registry and game parse cost probe"]
    fn measure_dense_registry_and_game_parse() {
        let (_fixture, stage) = dense_fixture(256, DenseShape::OneGame);
        let start = std::time::Instant::now();
        let registry = validate_staged_registry(&stage, &[]).unwrap();
        let game_path = fixed_path("records/marker.record.json");
        let game_file = find_file(&stage, &game_path).unwrap();
        let bytes = read_record_bytes(&stage, game_file).unwrap();
        std::hint::black_box(
            parse_game_record(&bytes, registry.schemas()[0].descriptor()).unwrap(),
        );
        eprintln!(
            "files={} copied_bytes={} elapsed={:?}",
            stage.files().len(),
            stage.copied_bytes(),
            start.elapsed()
        );
    }

    #[test]
    #[ignore = "local 256-descriptor full provisional candidate cost probe"]
    fn measure_dense_256_descriptor_candidate() {
        let (_fixture, stage) = dense_fixture(256, DenseShape::OneGame);
        let start = std::time::Instant::now();
        let candidate = validate_staged_candidate(&stage, &[]).unwrap();
        assert_eq!(candidate.registry().schemas().len(), 256);
        assert_eq!(candidate.records().games().len(), 1);
        assert_eq!(candidate.records().required_edges(), 1);
        let revision = candidate.provisional_manifest().unwrap().revision();
        std::hint::black_box(revision);
        eprintln!(
            "dense_256_candidate files={} copied_bytes={} elapsed={:?}",
            stage.files().len(),
            stage.copied_bytes(),
            start.elapsed()
        );
    }

    #[test]
    #[ignore = "local all-type and maximum required-edge candidate cost probe"]
    fn measure_dense_all_type_max_edges_candidate() {
        let (_fixture, stage) = dense_fixture(256, DenseShape::MaxEdges);
        let start = std::time::Instant::now();
        let candidate = validate_staged_candidate(&stage, &[]).unwrap();
        let revision = candidate.provisional_manifest().unwrap().revision();
        std::hint::black_box(revision);
        let elapsed = start.elapsed();
        assert_eq!(candidate.registry().schemas().len(), 256);
        assert_eq!(candidate.records().games().len(), 512);
        assert_eq!(candidate.records().scenes().len(), 3_584);
        assert_eq!(candidate.records().required_edges(), 65_536);
        let mut types = [0_u8; 256];
        for game in candidate.records().games() {
            let record_type = game.summary().record_type();
            let index = if record_type == "game.marker" {
                0
            } else {
                record_type
                    .strip_prefix("game.type")
                    .unwrap()
                    .parse::<usize>()
                    .unwrap()
            };
            types[index] += 1;
        }
        assert!(types.iter().all(|count| *count == 2));
        eprintln!(
            "all_type_max_edges files={} copied_bytes={} elapsed={elapsed:?}",
            stage.files().len(),
            stage.copied_bytes(),
        );
    }

    #[test]
    #[ignore = "local one-over-limit required-edge refusal probe"]
    fn dense_all_type_excess_edges_refuse() {
        let (_fixture, stage) = dense_fixture(256, DenseShape::ExcessEdges);
        let start = std::time::Instant::now();
        assert!(matches!(
            validate_staged_candidate(&stage, &[]),
            Err(StagedCandidateError::Records(
                StagedRecordError::TooManyRequiredEdges { .. }
            ))
        ));
        eprintln!("all_type_excess_edges elapsed={:?}", start.elapsed());
    }

    #[test]
    #[ignore = "local two-candidate all-type maximum-edge overlap cost probe"]
    fn measure_dense_all_type_candidate_overlap() {
        let (fixture, prior_stage) = dense_fixture(256, DenseShape::MaxEdges);
        let prior = validate_staged_candidate(&prior_stage, &[]).unwrap();
        let prior_revision = prior.provisional_manifest().unwrap().revision();
        let declared: Vec<_> = prior_stage
            .files()
            .iter()
            .map(|file| file.path().clone())
            .collect();
        let restage_start = std::time::Instant::now();
        let candidate_stage = stage_declared_tree_with(
            &fixture.0,
            &declared,
            Limits {
                main: 2 * 1024 * 1024 * 1024,
                wal: 4 * 1024 * 1024,
                temporary: 256 * 1024 * 1024,
                free_floor: 0,
            },
            |_| {},
        )
        .unwrap();
        let restage_elapsed = restage_start.elapsed();
        let validate_start = std::time::Instant::now();
        let candidate = validate_staged_candidate(&candidate_stage, &[]).unwrap();
        compare_staged_histories(prior.registry(), candidate.registry()).unwrap();
        let candidate_revision = candidate.provisional_manifest().unwrap().revision();
        let validate_elapsed = validate_start.elapsed();
        assert_eq!(prior_revision, candidate_revision);
        assert_eq!(candidate.registry().schemas().len(), 256);
        assert_eq!(candidate.records().games().len(), 512);
        assert_eq!(candidate.records().required_edges(), 65_536);
        eprintln!(
            "all_type_candidate_overlap files_per_stage={} bytes_per_stage={} restage={restage_elapsed:?} validate_and_lineage={validate_elapsed:?}",
            candidate_stage.files().len(),
            candidate_stage.copied_bytes(),
        );
    }

    #[test]
    #[ignore = "local two-buffer maximum history lineage cost probe"]
    fn measure_dense_staged_history_lineage() {
        let (fixture, prior_stage) = dense_fixture(1, DenseShape::OneGame);
        let declared: Vec<_> = prior_stage
            .files()
            .iter()
            .map(|file| file.path().clone())
            .collect();
        let candidate_stage = stage_declared_tree_with(
            &fixture.0,
            &declared,
            Limits {
                main: 2 * 1024 * 1024 * 1024,
                wal: 4 * 1024 * 1024,
                temporary: 256 * 1024 * 1024,
                free_floor: 0,
            },
            |_| {},
        )
        .unwrap();
        let prior = validate_staged_registry(&prior_stage, &[]).unwrap();
        let candidate = validate_staged_registry(&candidate_stage, &[]).unwrap();
        let start = std::time::Instant::now();
        compare_staged_histories(&prior, &candidate).unwrap();
        eprintln!(
            "files_per_stage={} bytes_per_stage={} elapsed={:?}",
            prior_stage.files().len(),
            prior_stage.copied_bytes(),
            start.elapsed()
        );
    }
}
