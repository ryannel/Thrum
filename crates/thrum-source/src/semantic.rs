//! Narrow F3 bootstrap proof over provisional staged bytes.
//!
//! This admits only core.project/core.scene schema 1 with empty scenes. It is
//! not the general registry, a candidate manifest, or durable source admission.

use std::io::Read;

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::record::canonical_uuid;
use crate::{
    JsonError, MAX_RECORD_BYTES, RecordError, SourcePath, StageError, StagedFile, StagedTree,
    parse_record_json, validate_record_envelope,
};

/// A provisional source tree failed the deliberately narrow core schema proof.
#[derive(Debug)]
pub enum BootstrapError {
    /// This proof has no cross-project dependency-root resolver.
    UnsupportedDependencyRoots,
    /// The staged tree has no `project.json` record.
    MissingProject,
    /// A required scene path is absent from the staged tree.
    MissingRecord(SourcePath),
    /// A reserved engine-record path was omitted from `project.records`.
    UnlistedRecord(SourcePath),
    /// A record is too large for v1 bounded JSON syntax.
    RecordTooLarge(SourcePath),
    /// Staged bytes no longer match their captured length and hash.
    StagedDrift(SourcePath),
    /// Opening a staged file failed.
    Stage(StageError),
    /// Reading staged bytes failed.
    Io {
        /// Path of the file that could not be read.
        path: SourcePath,
        /// Underlying I/O error.
        source: std::io::Error,
    },
    /// Strict JSON syntax failed.
    Json {
        /// Path of the malformed record.
        path: SourcePath,
        /// Bounded parser error.
        source: JsonError,
    },
    /// The common stable identity envelope failed.
    Envelope {
        /// Path of the record.
        path: SourcePath,
        /// Envelope error.
        source: RecordError,
    },
    /// A fixed v1 field, type, version or ordering rule failed.
    Schema {
        /// Path of the record.
        path: SourcePath,
        /// Stable narrow reason for the rejection.
        reason: &'static str,
    },
    /// Two records claim one durable ID.
    DuplicateRecordId(String),
    /// An entry scene ID does not resolve to a staged core scene.
    UnresolvedEntryScene(String),
}

/// Identity of one schema-checked `core.scene` source record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BootstrapScene {
    path: SourcePath,
    id: String,
}

impl BootstrapScene {
    /// Portable source path of this scene record.
    pub fn path(&self) -> &SourcePath {
        &self.path
    }

    /// Stable authored scene ID.
    pub fn id(&self) -> &str {
        &self.id
    }
}

/// Narrow project/empty-scene proof; not a validated candidate or root.
pub struct BootstrapProfile {
    project_id: String,
    scenes: Vec<BootstrapScene>,
    entry_scenes: Vec<String>,
}

impl BootstrapProfile {
    /// Stable ID read from `project.json` exact staged bytes.
    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    /// Schema-checked scene records in path order.
    pub fn scenes(&self) -> &[BootstrapScene] {
        &self.scenes
    }

    /// Required entry-scene IDs in canonical sorted order.
    pub fn entry_scenes(&self) -> &[String] {
        &self.entry_scenes
    }
}

/// Check the selected first `core.project`/empty-`core.scene` interaction.
///
/// Every parsed byte is read from the provisional stage and rehashed. This
/// proof refuses nonempty dependency roots because their closure cannot yet
/// be resolved. It does not construct a manifest or authorize admission.
pub fn validate_bootstrap_profile(
    stage: &StagedTree,
    dependency_roots: &[String],
) -> Result<BootstrapProfile, BootstrapError> {
    if !dependency_roots.is_empty() {
        return Err(BootstrapError::UnsupportedDependencyRoots);
    }
    let project_path = SourcePath::parse("project.json").expect("fixed portable project path");
    let project_file = stage
        .files()
        .binary_search_by(|file| file.path().cmp(&project_path))
        .ok()
        .map(|index| &stage.files()[index])
        .ok_or(BootstrapError::MissingProject)?;
    let project = read_record(stage, project_file)?;
    let project_id = check_envelope(&project_path, &project, "core.project")?;
    check_top_level(&project_path, &project)?;
    let fields = project
        .get("fields")
        .and_then(Value::as_object)
        .expect("validated envelope");
    if fields.len() != 3 || fields.get("project.format").and_then(Value::as_u64) != Some(1) {
        return Err(schema(
            &project_path,
            "unsupported project fields or format",
        ));
    }
    let paths = fields
        .get("project.records")
        .and_then(Value::as_array)
        .ok_or_else(|| schema(&project_path, "project.records must be an array"))?;
    if paths.len() > stage.files().len().saturating_sub(1) {
        return Err(schema(
            &project_path,
            "project.records exceeds staged file membership",
        ));
    }
    let mut record_paths = Vec::with_capacity(paths.len());
    for value in paths {
        let text = value
            .as_str()
            .ok_or_else(|| schema(&project_path, "project.records path must be a string"))?;
        let path = SourcePath::parse(text)
            .map_err(|_| schema(&project_path, "project.records path is not portable"))?;
        if !is_scene_path(&path) {
            return Err(schema(&project_path, "unsupported engine record path"));
        }
        if record_paths
            .last()
            .is_some_and(|previous| previous >= &path)
        {
            return Err(schema(
                &project_path,
                "project.records is not strictly sorted",
            ));
        }
        record_paths.push(path);
    }
    let entry_values = fields
        .get("project.entryScenes")
        .and_then(Value::as_array)
        .filter(|values| !values.is_empty())
        .ok_or_else(|| schema(&project_path, "project.entryScenes must be nonempty"))?;
    if entry_values.len() > record_paths.len() {
        return Err(schema(
            &project_path,
            "entry-scene count exceeds declared scene records",
        ));
    }
    let mut entry_scenes: Vec<String> = Vec::with_capacity(entry_values.len());
    for value in entry_values {
        let id = value
            .as_str()
            .filter(|id| canonical_uuid(id))
            .ok_or_else(|| schema(&project_path, "invalid entry-scene ID"))?;
        if entry_scenes
            .last()
            .is_some_and(|previous| previous.as_str() >= id)
        {
            return Err(schema(
                &project_path,
                "project.entryScenes is not strictly sorted",
            ));
        }
        entry_scenes.push(id.to_owned());
    }
    drop(project);

    for file in stage.files() {
        if is_reserved_record_path(file.path()) && record_paths.binary_search(file.path()).is_err()
        {
            return Err(BootstrapError::UnlistedRecord(file.path().clone()));
        }
    }

    let mut scenes = Vec::with_capacity(record_paths.len());
    for path in record_paths {
        let file = stage
            .files()
            .binary_search_by(|file| file.path().cmp(&path))
            .ok()
            .map(|index| &stage.files()[index])
            .ok_or_else(|| BootstrapError::MissingRecord(path.clone()))?;
        let value = read_record(stage, file)?;
        let id = check_envelope(&path, &value, "core.scene")?;
        check_top_level(&path, &value)?;
        let fields = value
            .get("fields")
            .and_then(Value::as_object)
            .expect("validated envelope");
        if fields.len() > 1
            || fields
                .get("scene.label")
                .is_some_and(|label| !label.is_string())
            || fields.keys().any(|field| field != "scene.label")
        {
            return Err(schema(&path, "unsupported scene fields"));
        }
        scenes.push(BootstrapScene { path, id });
    }

    let mut scene_ids: Vec<&str> = scenes.iter().map(|scene| scene.id.as_str()).collect();
    scene_ids.sort_unstable();
    if let Some(pair) = scene_ids.windows(2).find(|pair| pair[0] == pair[1]) {
        return Err(BootstrapError::DuplicateRecordId(pair[0].to_owned()));
    }
    if scene_ids.binary_search(&project_id.as_str()).is_ok() {
        return Err(BootstrapError::DuplicateRecordId(project_id));
    }
    for id in &entry_scenes {
        if scene_ids.binary_search(&id.as_str()).is_err() {
            return Err(BootstrapError::UnresolvedEntryScene(id.clone()));
        }
    }
    Ok(BootstrapProfile {
        project_id,
        scenes,
        entry_scenes,
    })
}

fn read_record(stage: &StagedTree, file: &StagedFile) -> Result<Value, BootstrapError> {
    let path = file.path();
    if file.length() > MAX_RECORD_BYTES as u64 {
        return Err(BootstrapError::RecordTooLarge(path.clone()));
    }
    let mut bytes = Vec::with_capacity(file.length() as usize + 1);
    stage
        .open(path)
        .map_err(BootstrapError::Stage)?
        .take(file.length() + 1)
        .read_to_end(&mut bytes)
        .map_err(|source| BootstrapError::Io {
            path: path.clone(),
            source,
        })?;
    if bytes.len() as u64 != file.length() || Sha256::digest(&bytes).as_slice() != file.sha256() {
        return Err(BootstrapError::StagedDrift(path.clone()));
    }
    parse_record_json(&bytes).map_err(|source| BootstrapError::Json {
        path: path.clone(),
        source,
    })
}

fn check_envelope(
    path: &SourcePath,
    value: &Value,
    expected_type: &str,
) -> Result<String, BootstrapError> {
    let envelope = validate_record_envelope(value).map_err(|source| BootstrapError::Envelope {
        path: path.clone(),
        source,
    })?;
    if envelope.record_type() != expected_type || envelope.schema() != 1 {
        return Err(schema(path, "unsupported record type or schema"));
    }
    Ok(envelope.id().to_owned())
}

fn check_top_level(path: &SourcePath, value: &Value) -> Result<(), BootstrapError> {
    let object = value.as_object().expect("validated envelope");
    if object.keys().any(|key| {
        !matches!(
            key.as_str(),
            "id" | "type" | "schema" | "fields" | "documentation"
        )
    }) || object
        .get("documentation")
        .is_some_and(|documentation| !documentation.is_string())
    {
        return Err(schema(path, "unsupported top-level record field"));
    }
    Ok(())
}

fn schema(path: &SourcePath, reason: &'static str) -> BootstrapError {
    BootstrapError::Schema {
        path: path.clone(),
        reason,
    }
}

fn is_scene_path(path: &SourcePath) -> bool {
    let text = path.as_str();
    text.strip_prefix("scenes/")
        .is_some_and(|name| !name.contains('/') && name.ends_with(".scene.json"))
}

fn is_reserved_record_path(path: &SourcePath) -> bool {
    let text = path.as_str();
    text.starts_with("scenes/") && text.ends_with(".scene.json")
        || text.ends_with(".prefab.json")
        || text.ends_with(".asset.json")
        || text.starts_with("schemas/") && text.ends_with(".schema.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use crate::budget::Limits;
    use crate::stage::stage_declared_tree_with;

    const PROJECT_ID: &str = "2a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    const SCENE_ID: &str = "3a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "thrum-bootstrap-{}-{}",
                std::process::id(),
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&root).unwrap();
            Self(root)
        }

        fn write(&self, path: &str, bytes: &str) {
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

    fn project(id: &str, scene_path: &str, entry_id: &str) -> String {
        format!(
            "{{\"id\":\"{id}\",\"type\":\"core.project\",\"schema\":1,\"fields\":{{\"project.format\":1,\"project.records\":[\"{scene_path}\"],\"project.entryScenes\":[\"{entry_id}\"]}}}}"
        )
    }

    fn scene(id: &str, label: &str) -> String {
        format!(
            "{{\"id\":\"{id}\",\"type\":\"core.scene\",\"schema\":1,\"fields\":{{\"scene.label\":\"{label}\"}}}}"
        )
    }

    fn basic(fixture: &Fixture) {
        fixture.write(
            "project.json",
            &project(PROJECT_ID, "scenes/main.scene.json", SCENE_ID),
        );
        fixture.write("scenes/main.scene.json", &scene(SCENE_ID, "first"));
        fixture.write("src/game.rs", "fn game() {}");
    }

    #[test]
    fn project_to_scene_closure_keeps_stable_id_across_exact_byte_edit() {
        let fixture = Fixture::new();
        basic(&fixture);
        let paths = ["project.json", "scenes/main.scene.json", "src/game.rs"];
        let first = fixture.stage(&paths);
        let first_profile = validate_bootstrap_profile(&first, &[]).unwrap();
        assert_eq!(first_profile.project_id(), PROJECT_ID);
        assert_eq!(first_profile.scenes()[0].id(), SCENE_ID);
        assert_eq!(first_profile.entry_scenes(), &[SCENE_ID]);

        fixture.write("scenes/main.scene.json", &scene(SCENE_ID, "second"));
        let second = fixture.stage(&paths);
        let second_profile = validate_bootstrap_profile(&second, &[]).unwrap();
        assert_eq!(second_profile.scenes()[0].id(), SCENE_ID);
        let scene_path = SourcePath::parse("scenes/main.scene.json").unwrap();
        let first_file = first
            .files()
            .iter()
            .find(|file| file.path() == &scene_path)
            .unwrap();
        let second_file = second
            .files()
            .iter()
            .find(|file| file.path() == &scene_path)
            .unwrap();
        assert_ne!(first_file.sha256(), second_file.sha256());
    }

    #[test]
    fn missing_duplicate_or_unsupported_semantics_refuse_without_stage_mutation() {
        let fixture = Fixture::new();
        basic(&fixture);
        let paths = ["project.json", "scenes/main.scene.json", "src/game.rs"];
        let old = fixture.stage(&paths);
        assert!(matches!(
            validate_bootstrap_profile(&old, &["external".to_owned()]),
            Err(BootstrapError::UnsupportedDependencyRoots)
        ));

        fixture.write("scenes/main.scene.json", &scene(PROJECT_ID, "duplicate"));
        let duplicate = fixture.stage(&paths);
        assert!(matches!(
            validate_bootstrap_profile(&duplicate, &[]),
            Err(BootstrapError::DuplicateRecordId(_))
        ));
        drop(duplicate);

        fixture.write("scenes/main.scene.json", &scene(SCENE_ID, "first"));
        fixture.write(
            "project.json",
            &project(PROJECT_ID, "scenes/missing.scene.json", SCENE_ID),
        );
        fs::remove_file(fixture.0.join("scenes/main.scene.json")).unwrap();
        let missing = fixture.stage(&["project.json", "src/game.rs"]);
        assert!(matches!(
            validate_bootstrap_profile(&missing, &[]),
            Err(BootstrapError::MissingRecord(_))
        ));
        drop(missing);

        basic(&fixture);
        fixture.write(
            "scenes/main.scene.json",
            &scene(SCENE_ID, "first").replace("\"scene.label\"", "\"scene.objects\""),
        );
        let objects = fixture.stage(&paths);
        assert!(matches!(
            validate_bootstrap_profile(&objects, &[]),
            Err(BootstrapError::Schema { .. })
        ));
        let scene_path = SourcePath::parse("scenes/main.scene.json").unwrap();
        let mut old_bytes = Vec::new();
        old.open(&scene_path)
            .unwrap()
            .read_to_end(&mut old_bytes)
            .unwrap();
        assert_eq!(old_bytes, scene(SCENE_ID, "first").as_bytes());
    }

    #[test]
    fn unlisted_reserved_record_and_unresolved_entry_refuse() {
        let fixture = Fixture::new();
        basic(&fixture);
        fixture.write("scenes/extra.scene.json", &scene(SCENE_ID, "extra"));
        let extra = fixture.stage(&[
            "project.json",
            "scenes/main.scene.json",
            "scenes/extra.scene.json",
            "src/game.rs",
        ]);
        assert!(matches!(
            validate_bootstrap_profile(&extra, &[]),
            Err(BootstrapError::UnlistedRecord(_))
        ));
        drop(extra);
        fs::remove_file(fixture.0.join("scenes/extra.scene.json")).unwrap();

        fixture.write(
            "project.json",
            &project(PROJECT_ID, "scenes/main.scene.json", PROJECT_ID),
        );
        let wrong_entry = fixture.stage(&["project.json", "scenes/main.scene.json", "src/game.rs"]);
        assert!(matches!(
            validate_bootstrap_profile(&wrong_entry, &[]),
            Err(BootstrapError::UnresolvedEntryScene(_))
        ));
    }

    #[test]
    fn impossible_declaration_counts_refuse_before_summary_growth() {
        let fixture = Fixture::new();
        basic(&fixture);
        let two_paths = project(PROJECT_ID, "scenes/main.scene.json", SCENE_ID).replace(
            "[\"scenes/main.scene.json\"]",
            "[\"scenes/main.scene.json\",\"scenes/other.scene.json\"]",
        );
        fixture.write("project.json", &two_paths);
        fs::remove_file(fixture.0.join("scenes/main.scene.json")).unwrap();
        let too_many_paths = fixture.stage(&["project.json", "src/game.rs"]);
        assert!(matches!(
            validate_bootstrap_profile(&too_many_paths, &[]),
            Err(BootstrapError::Schema { .. })
        ));
        drop(too_many_paths);

        basic(&fixture);
        let two_entries = project(PROJECT_ID, "scenes/main.scene.json", SCENE_ID).replace(
            &format!("[\"{SCENE_ID}\"]"),
            &format!("[\"{PROJECT_ID}\",\"{SCENE_ID}\"]"),
        );
        fixture.write("project.json", &two_entries);
        let too_many_entries =
            fixture.stage(&["project.json", "scenes/main.scene.json", "src/game.rs"]);
        assert!(matches!(
            validate_bootstrap_profile(&too_many_entries, &[]),
            Err(BootstrapError::Schema { .. })
        ));
    }

    #[test]
    fn changed_staged_bytes_refuse_before_semantic_parse() {
        let fixture = Fixture::new();
        basic(&fixture);
        let stage = fixture.stage(&["project.json", "scenes/main.scene.json", "src/game.rs"]);
        let stage_parent = fixture.0.join(".engine/staging");
        let stage_dir = fs::read_dir(stage_parent)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let staged_scene = stage_dir.join("scenes/main.scene.json");
        let bytes = fs::read(&staged_scene).unwrap();
        let changed = String::from_utf8(bytes).unwrap().replace("first", "other");
        fs::write(staged_scene, changed).unwrap();
        assert!(matches!(
            validate_bootstrap_profile(&stage, &[]),
            Err(BootstrapError::StagedDrift(_))
        ));
    }

    #[test]
    #[ignore = "local 1,000-scene staged semantic validation cost probe"]
    fn measure_bootstrap_validation_1000_scenes() {
        let fixture = Fixture::new();
        let mut record_paths = Vec::with_capacity(1000);
        let mut scene_ids = Vec::with_capacity(1000);
        let mut declared = Vec::with_capacity(1001);
        declared.push(SourcePath::parse("project.json").unwrap());
        for index in 0..1000_u64 {
            let path = format!("scenes/{index:04}.scene.json");
            let id = format!("00000000-0000-4000-8000-{index:012x}");
            fixture.write(&path, &scene(&id, "small scene"));
            declared.push(SourcePath::parse(&path).unwrap());
            record_paths.push(path);
            scene_ids.push(id);
        }
        fixture.write(
            "project.json",
            &format!(
                "{{\"id\":\"{PROJECT_ID}\",\"type\":\"core.project\",\"schema\":1,\"fields\":{{\"project.format\":1,\"project.records\":{},\"project.entryScenes\":{}}}}}",
                serde_json::to_string(&record_paths).unwrap(),
                serde_json::to_string(&scene_ids).unwrap()
            ),
        );
        let limits = Limits {
            main: 2 * 1024 * 1024 * 1024,
            wal: 4 * 1024 * 1024,
            temporary: 32 * 1024 * 1024,
            free_floor: 0,
        };
        let stage = stage_declared_tree_with(&fixture.0, &declared, limits, |_| {}).unwrap();
        let start = std::time::Instant::now();
        let proof = validate_bootstrap_profile(&stage, &[]).unwrap();
        let elapsed = start.elapsed();
        assert_eq!(proof.scenes().len(), 1000);
        eprintln!("staged_scene_records=1000 validation_elapsed={elapsed:?}");
        std::hint::black_box(proof);
    }
}
