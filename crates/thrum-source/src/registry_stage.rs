//! Staged-byte and declaration proof for the first source-carried registry.
//!
//! Registry validation precedes a bounded scene/game-record closure pass.
//! Prior-root lineage, physical peak admission and durable admission remain
//! separate gates.

use std::collections::HashSet;
use std::io::Read;

use sha2::{Digest, Sha256};

use crate::bootstrap_json::parse_scene;
use crate::game_json::{GameRecordError, GameRecordSummary, parse_game_record};
use crate::history_json::{
    CarriedHistoryError, HistoryError, compare_carried_histories, parse_registry_history,
};
use crate::project_v2_json::{ProjectV2, ProjectV2Error, parse_project_v2};
use crate::registry::{RegistryConsistencyError, validate_active_registry_with};
use crate::schema_json::{SchemaDescriptor, SchemaError, parse_schema_descriptor};
use crate::{
    MAX_RECORD_BYTES, ManifestEntry, ManifestError, RecordDescriptor, SourcePath, StageError,
    StagedFile, StagedTree, TreeManifest,
};

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

/// A registry's declared scene/game records failed the first closure profile.
#[derive(Debug)]
pub enum StagedRecordError {
    /// The game-record profile needs one unambiguous active descriptor.
    GameSchemaCount(usize),
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
    /// An ID collides with another scene, game or registry record.
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
    TooManyRequiredEdges,
}

/// A first-profile staged candidate failed before deriving a manifest.
#[derive(Debug)]
pub enum StagedCandidateError {
    /// Project, schema or carried history validation refused.
    Registry(StagedRegistryError),
    /// Scene/game identity or required reference closure refused.
    Records(StagedRecordError),
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

    /// Checked scene/game identities and local required edges.
    pub fn records(&self) -> &StagedRecords {
        &self.records
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
            2 + self.registry.schemas.len() + self.records.scenes.len() + self.records.games.len(),
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

/// Checked scene/game identities and required local reference closure.
pub struct StagedRecords {
    scenes: Vec<StagedScene>,
    games: Vec<StagedGame>,
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

    /// Number of checked required game-to-scene references.
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
/// A complete candidate also needs scene/game-record parsing and ID/reference
/// closure. The history checked here is only the candidate's own history;
/// comparison to an authenticated predecessor belongs to durable admission.
pub fn validate_staged_registry<'a>(
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

    let mut ids = HashSet::with_capacity(schemas.len() + 2);
    for id in std::iter::once(project.id())
        .chain(std::iter::once(history_id.as_str()))
        .chain(schemas.iter().map(|item| item.descriptor.record_id()))
    {
        if !ids.insert(id) {
            return Err(StagedRegistryError::DuplicateRecordId(id.to_owned()));
        }
    }
    Ok(StagedRegistry {
        stage,
        project,
        schemas,
        history_id,
    })
}

/// Rehash declared scene/game bytes and resolve required local scene IDs.
///
/// The first closure profile accepts one active game descriptor when game
/// records exist. This avoids a second full parse or an order-dependent JSON
/// envelope probe on the source-edit path. A later multi-schema profile must
/// select its dispatch and physical peak separately. This result is not a
/// manifest, an authenticated lineage comparison or an admitted source root.
pub fn validate_staged_records(
    registry: &StagedRegistry<'_>,
) -> Result<StagedRecords, StagedRecordError> {
    const MAX_REQUIRED_EDGES: usize = 65_536;

    let paths = registry.project.record_paths();
    let scene_count = paths
        .iter()
        .filter(|path| path.as_str().starts_with("scenes/"))
        .count();
    let game_count = paths.len() - scene_count;
    if game_count != 0 && registry.schemas.len() != 1 {
        return Err(StagedRecordError::GameSchemaCount(registry.schemas.len()));
    }
    let mut scenes = Vec::with_capacity(scene_count);
    let mut games = Vec::with_capacity(game_count);
    let mut required_edges = 0_usize;
    let mut bytes = Vec::new();
    for path in paths {
        let file = find_file(registry.stage, path).expect("registry checked declared membership");
        read_record_bytes_into(registry.stage, file, &mut bytes)
            .map_err(StagedRecordError::Stage)?;
        if path.as_str().starts_with("scenes/") {
            let scene = parse_scene(&bytes).map_err(|source| StagedRecordError::Scene {
                path: path.clone(),
                source,
            })?;
            scenes.push(StagedScene {
                path: path.clone(),
                id: scene.id,
            });
        } else {
            let descriptor = &registry.schemas[0].descriptor;
            let summary = parse_game_record(&bytes, descriptor).map_err(|source| {
                StagedRecordError::Game {
                    path: path.clone(),
                    source,
                }
            })?;
            required_edges = required_edges
                .checked_add(summary.references().len())
                .filter(|edges| *edges <= MAX_REQUIRED_EDGES)
                .ok_or(StagedRecordError::TooManyRequiredEdges)?;
            games.push(StagedGame {
                path: path.clone(),
                summary,
            });
        }
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
    all_ids.sort_unstable();
    if let Some(pair) = all_ids.windows(2).find(|pair| pair[0] == pair[1]) {
        return Err(StagedRecordError::DuplicateRecordId(pair[0].to_owned()));
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
    Ok(StagedRecords {
        scenes,
        games,
        required_edges,
    })
}

/// Check one narrow staged registry and its declared record closure together.
///
/// This does not perform authenticated prior-root lineage, maximum physical
/// parser/graph reservation, editor round-trip or durable source admission.
pub fn validate_staged_candidate<'a>(
    stage: &'a StagedTree,
    dependency_roots: &[String],
) -> Result<StagedCandidate<'a>, StagedCandidateError> {
    let registry = validate_staged_registry(stage, dependency_roots)
        .map_err(StagedCandidateError::Registry)?;
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

    fn dense_fixture(schema_count: usize) -> (Fixture, StagedTree) {
        assert!((1..=256).contains(&schema_count));
        let fixture = Fixture::new();
        basic(&fixture);

        let schema_path = "schemas/marker.schema.json";
        let mut schema: serde_json::Value =
            serde_json::from_slice(&fs::read(fixture.0.join(schema_path)).unwrap()).unwrap();
        let fields = schema["fields"]["schema.fields"].as_array_mut().unwrap();
        for index in 0..126 {
            fields.push(json!({
                "id":format!("marker.extra{index:03}"), "kind":"text",
                "presence":"optional", "maxUtf8Bytes":64
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
                .map(|field| {
                    json!({
                        "id":format!("value.f{field:03}"), "kind":"text",
                        "presence":"optional", "maxUtf8Bytes":64
                    })
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

        let game_path = fixture.0.join("records/marker.record.json");
        let mut game = BufWriter::new(fs::File::create(game_path).unwrap());
        write!(game, "{{\"id\":\"{GAME_ID}\",\"type\":\"game.marker\",\"schema\":1,\"fields\":{{\"marker.scene\":\"{SCENE_ID}\"").unwrap();
        let large_value = "v".repeat(65_000);
        for index in 0..128 {
            write!(game, ",\"x.game.marker.note{index:03}\":\"{large_value}\"").unwrap();
        }
        game.write_all(b"}}").unwrap();
        game.flush().unwrap();
        drop(game);

        let mut record_paths = vec![
            "records/marker.record.json".to_owned(),
            "scenes/main.scene.json".to_owned(),
        ];
        let mut staged_paths: Vec<String> = PATHS.iter().map(|path| (*path).to_owned()).collect();
        staged_paths.extend(schema_paths.iter().skip(1).cloned());
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
        let (_fixture, stage) = dense_fixture(1);
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
        let (_fixture, stage) = dense_fixture(256);
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
    #[ignore = "local two-buffer maximum history lineage cost probe"]
    fn measure_dense_staged_history_lineage() {
        let (fixture, prior_stage) = dense_fixture(1);
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
