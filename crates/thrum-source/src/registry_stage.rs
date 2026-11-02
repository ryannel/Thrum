//! Staged-byte and declaration proof for the first source-carried registry.
//!
//! This validates project, descriptor and history records only. Game records,
//! scene IDs, prior-root lineage and durable admission remain separate gates.

use std::collections::HashSet;
use std::io::Read;

use sha2::{Digest, Sha256};

use crate::history_json::{HistoryError, parse_registry_history};
use crate::project_v2_json::{ProjectV2, ProjectV2Error, parse_project_v2};
use crate::registry::{RegistryConsistencyError, validate_active_registry_with};
use crate::schema_json::{SchemaDescriptor, SchemaError, parse_schema_descriptor};
use crate::{MAX_RECORD_BYTES, SourcePath, StageError, StagedFile, StagedTree};

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
    let project_bytes = read_record_bytes(stage, project_file)?;
    let project = parse_project_v2(&project_bytes).map_err(StagedRegistryError::Project)?;
    drop(project_bytes);

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
        let bytes = read_record_bytes(stage, file)?;
        let descriptor =
            parse_schema_descriptor(&bytes).map_err(|source| StagedRegistryError::Schema {
                path: path.clone(),
                source,
            })?;
        schemas.push(StagedSchema {
            path: path.clone(),
            descriptor,
        });
    }
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
    let path = file.path();
    if file.length() > MAX_RECORD_BYTES as u64 {
        return Err(StagedRegistryError::RecordTooLarge(path.clone()));
    }
    let mut bytes = Vec::with_capacity(file.length() as usize + 1);
    stage
        .open(path)
        .map_err(StagedRegistryError::Stage)?
        .take(file.length() + 1)
        .read_to_end(&mut bytes)
        .map_err(|source| StagedRegistryError::Io {
            path: path.clone(),
            source,
        })?;
    if bytes.len() as u64 != file.length() || Sha256::digest(&bytes).as_slice() != file.sha256() {
        return Err(StagedRegistryError::StagedDrift(path.clone()));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use serde_json::json;

    use crate::budget::Limits;
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
                temporary: 16 * 1024 * 1024,
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
        fixture.write("records/marker.record.json", b"{}");
        fixture.write("scenes/main.scene.json", b"{}");
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
}
