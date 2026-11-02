//! Direct bounded decoding of the first source-registry project declaration.
//!
//! This checks one supplied `core.project` schema 2 record. The staged-tree
//! validator must still check every declared path and record identity.

use std::fmt;

use serde::de::{DeserializeSeed, Error as _, MapAccess, SeqAccess, Visitor};

use crate::record::canonical_uuid;
use crate::{MAX_RECORD_BYTES, SourcePath};

const MAX_RECORDS: usize = 4_096;
const MAX_SCHEMAS: usize = 256;
const HISTORY_PATH: &str = "schemas/history.registry.json";

/// A first-profile project declaration failed fixed syntax or bounds.
#[derive(Debug)]
pub enum ProjectV2Error {
    /// Exact JSON bytes exceed the 16 MiB source record limit.
    TooLarge,
    /// Syntax, identity, duplicate key, path shape, order or count refused.
    Invalid(serde_json::Error),
}

/// Parsed `core.project` schema 2 declaration; not yet a validated tree.
pub struct ProjectV2 {
    id: String,
    record_paths: Vec<SourcePath>,
    schema_paths: Vec<SourcePath>,
    entry_scenes: Vec<String>,
}

impl ProjectV2 {
    /// Stable project ID from the record envelope.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Sorted declared scene and game-record paths.
    pub fn record_paths(&self) -> &[SourcePath] {
        &self.record_paths
    }

    /// Sorted declared game-schema paths.
    pub fn schema_paths(&self) -> &[SourcePath] {
        &self.schema_paths
    }

    /// Sorted stable IDs of entry scenes.
    pub fn entry_scenes(&self) -> &[String] {
        &self.entry_scenes
    }

    /// Fixed first-profile path of the declared registry history record.
    pub fn history_path(&self) -> &'static str {
        HISTORY_PATH
    }
}

/// Decode one `core.project` schema 2 record without a generic JSON tree.
///
/// Array limits are checked before deserializing an extra element. This does
/// not check path membership in a staged tree or resolve entry-scene IDs.
pub fn parse_project_v2(bytes: &[u8]) -> Result<ProjectV2, ProjectV2Error> {
    if bytes.len() > MAX_RECORD_BYTES {
        return Err(ProjectV2Error::TooLarge);
    }
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let project = ProjectSeed
        .deserialize(&mut deserializer)
        .map_err(ProjectV2Error::Invalid)?;
    deserializer.end().map_err(ProjectV2Error::Invalid)?;
    Ok(project)
}

struct ProjectSeed;

impl<'de> DeserializeSeed<'de> for ProjectSeed {
    type Value = ProjectV2;

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(ProjectVisitor)
    }
}

struct ProjectVisitor;

impl<'de> Visitor<'de> for ProjectVisitor {
    type Value = ProjectV2;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a core.project schema 2 record")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut id = None;
        let mut record_type = None;
        let mut schema = None;
        let mut fields = None;
        let mut documentation_seen = false;
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "id" if id.is_none() => id = Some(map.next_value::<String>()?),
                "type" if record_type.is_none() => {
                    record_type = Some(map.next_value::<String>()?);
                }
                "schema" if schema.is_none() => schema = Some(map.next_value::<u64>()?),
                "fields" if fields.is_none() => fields = Some(map.next_value_seed(FieldsSeed)?),
                "documentation" if !documentation_seen => {
                    let _: String = map.next_value()?;
                    documentation_seen = true;
                }
                _ => return Err(A::Error::custom("duplicate or unsupported project key")),
            }
        }
        let id: String = id.ok_or_else(|| A::Error::custom("missing project ID"))?;
        if !canonical_uuid(&id)
            || record_type.as_deref() != Some("core.project")
            || schema != Some(2)
        {
            return Err(A::Error::custom("invalid project identity, type or schema"));
        }
        let fields = fields.ok_or_else(|| A::Error::custom("missing project fields"))?;
        Ok(ProjectV2 {
            id,
            record_paths: fields.record_paths,
            schema_paths: fields.schema_paths,
            entry_scenes: fields.entry_scenes,
        })
    }
}

struct ProjectFields {
    record_paths: Vec<SourcePath>,
    schema_paths: Vec<SourcePath>,
    entry_scenes: Vec<String>,
}

struct FieldsSeed;

impl<'de> DeserializeSeed<'de> for FieldsSeed {
    type Value = ProjectFields;

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(FieldsVisitor)
    }
}

struct FieldsVisitor;

impl<'de> Visitor<'de> for FieldsVisitor {
    type Value = ProjectFields;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("fixed core.project schema 2 fields")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut format = None;
        let mut record_paths = None;
        let mut schema_paths = None;
        let mut history_path = None;
        let mut entry_scenes = None;
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "project.format" if format.is_none() => format = Some(map.next_value::<u64>()?),
                "project.records" if record_paths.is_none() => {
                    record_paths = Some(map.next_value_seed(PathArraySeed {
                        limit: MAX_RECORDS,
                        kind: PathKind::Record,
                    })?);
                }
                "project.schemas" if schema_paths.is_none() => {
                    schema_paths = Some(map.next_value_seed(PathArraySeed {
                        limit: MAX_SCHEMAS,
                        kind: PathKind::Schema,
                    })?);
                }
                "project.registryHistory" if history_path.is_none() => {
                    history_path = Some(map.next_value::<String>()?);
                }
                "project.entryScenes" if entry_scenes.is_none() => {
                    entry_scenes = Some(map.next_value_seed(EntryArraySeed)?);
                }
                _ => return Err(A::Error::custom("duplicate or unsupported project field")),
            }
        }
        if format != Some(1) || history_path.as_deref() != Some(HISTORY_PATH) {
            return Err(A::Error::custom(
                "unsupported project format or history path",
            ));
        }
        let record_paths =
            record_paths.ok_or_else(|| A::Error::custom("missing project.records"))?;
        let schema_paths =
            schema_paths.ok_or_else(|| A::Error::custom("missing project.schemas"))?;
        let entry_scenes =
            entry_scenes.ok_or_else(|| A::Error::custom("missing project.entryScenes"))?;
        if entry_scenes.is_empty() || entry_scenes.len() > record_paths.len() {
            return Err(A::Error::custom("invalid entry-scene count"));
        }
        Ok(ProjectFields {
            record_paths,
            schema_paths,
            entry_scenes,
        })
    }
}

#[derive(Clone, Copy)]
enum PathKind {
    Record,
    Schema,
}

struct PathArraySeed {
    limit: usize,
    kind: PathKind,
}

impl<'de> DeserializeSeed<'de> for PathArraySeed {
    type Value = Vec<SourcePath>;

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_seq(PathArrayVisitor {
            limit: self.limit,
            kind: self.kind,
        })
    }
}

struct PathArrayVisitor {
    limit: usize,
    kind: PathKind,
}

impl<'de> Visitor<'de> for PathArrayVisitor {
    type Value = Vec<SourcePath>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("strictly sorted portable record or schema paths")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
        let mut paths: Vec<SourcePath> = Vec::new();
        loop {
            if paths.len() >= self.limit {
                sequence.next_element_seed(RejectExtraSeed)?;
                break;
            }
            let Some(text) = sequence.next_element::<String>()? else {
                break;
            };
            let path = SourcePath::parse(&text)
                .map_err(|_| A::Error::custom("project path is not portable"))?;
            let supported = match self.kind {
                PathKind::Record => is_scene_path(&path) || is_game_record_path(&path),
                PathKind::Schema => is_schema_path(&path),
            };
            if !supported {
                return Err(A::Error::custom("unsupported declared path role"));
            }
            if paths.last().is_some_and(|previous| previous >= &path) {
                return Err(A::Error::custom("project paths not strictly sorted"));
            }
            paths.push(path);
        }
        Ok(paths)
    }
}

struct EntryArraySeed;

impl<'de> DeserializeSeed<'de> for EntryArraySeed {
    type Value = Vec<String>;

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_seq(EntryArrayVisitor)
    }
}

struct EntryArrayVisitor;

impl<'de> Visitor<'de> for EntryArrayVisitor {
    type Value = Vec<String>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("at most 4,096 sorted canonical entry-scene IDs")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
        let mut entries: Vec<String> = Vec::new();
        loop {
            if entries.len() >= MAX_RECORDS {
                sequence.next_element_seed(RejectExtraSeed)?;
                break;
            }
            let Some(id) = sequence.next_element::<String>()? else {
                break;
            };
            if !canonical_uuid(&id) {
                return Err(A::Error::custom("invalid entry-scene ID"));
            }
            if entries.last().is_some_and(|previous| previous >= &id) {
                return Err(A::Error::custom("entry scenes not strictly sorted"));
            }
            entries.push(id);
        }
        Ok(entries)
    }
}

struct RejectExtraSeed;

impl<'de> DeserializeSeed<'de> for RejectExtraSeed {
    type Value = ();

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        _deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        Err(D::Error::custom("project declaration count limit"))
    }
}

fn one_level_path(path: &SourcePath, prefix: &str, suffix: &str) -> bool {
    path.as_str().strip_prefix(prefix).is_some_and(|name| {
        !name.is_empty()
            && !name.contains('/')
            && name.ends_with(suffix)
            && name.len() > suffix.len()
    })
}

fn is_scene_path(path: &SourcePath) -> bool {
    one_level_path(path, "scenes/", ".scene.json")
}

fn is_game_record_path(path: &SourcePath) -> bool {
    one_level_path(path, "records/", ".record.json")
}

fn is_schema_path(path: &SourcePath) -> bool {
    path.as_str() != HISTORY_PATH && one_level_path(path, "schemas/", ".schema.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROJECT_ID: &str = "2a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    const SCENE_ID: &str = "3a5ef644-7225-4770-9f4d-e1d9c0a7d515";

    fn project() -> String {
        format!(
            "{{\"id\":\"{PROJECT_ID}\",\"type\":\"core.project\",\"schema\":2,\"fields\":{{\"project.format\":1,\"project.records\":[\"records/marker.record.json\",\"scenes/main.scene.json\"],\"project.schemas\":[\"schemas/marker.schema.json\"],\"project.registryHistory\":\"{HISTORY_PATH}\",\"project.entryScenes\":[\"{SCENE_ID}\"]}}}}"
        )
    }

    #[test]
    fn direct_v2_project_declaration_is_bounded_and_sorted() {
        let parsed = parse_project_v2(project().as_bytes()).unwrap();
        assert_eq!(parsed.id(), PROJECT_ID);
        assert_eq!(parsed.record_paths().len(), 2);
        assert_eq!(
            parsed.schema_paths()[0].as_str(),
            "schemas/marker.schema.json"
        );
        assert_eq!(parsed.entry_scenes(), [SCENE_ID]);
        assert_eq!(parsed.history_path(), HISTORY_PATH);
    }

    #[test]
    fn wrong_role_duplicate_unknown_and_bad_version_refuse() {
        let valid = project();
        for changed in [
            valid.replace("\"schema\":2", "\"schema\":1"),
            valid.replace("\"schema\":2", "\"schema\":2,\"schema\":2"),
            valid.replace("schemas/marker.schema.json", HISTORY_PATH),
            valid.replace(
                "records/marker.record.json",
                "records/nested/marker.record.json",
            ),
            valid.replace(
                "\"project.format\":1",
                "\"project.format\":1,\"unknown\":[[[0]]]",
            ),
            valid.replace(
                "\"project.records\":[\"records/marker.record.json\",\"scenes/main.scene.json\"]",
                "\"project.records\":[\"scenes/main.scene.json\",\"records/marker.record.json\"]",
            ),
            format!("{valid} trailing"),
        ] {
            assert!(parse_project_v2(changed.as_bytes()).is_err(), "{changed}");
        }
    }

    #[test]
    fn record_limit_refuses_before_parsing_extra_value() {
        let paths = (0..MAX_RECORDS)
            .map(|index| format!("\"records/r{index:04}.record.json\""))
            .collect::<Vec<_>>()
            .join(",");
        let bytes = project().replace(
            "\"records/marker.record.json\",\"scenes/main.scene.json\"",
            &format!("{paths},[[[0]]]"),
        );
        let error = parse_project_v2(bytes.as_bytes()).err().unwrap();
        assert!(format!("{error:?}").contains("count limit"));
    }

    #[test]
    #[ignore = "local maximum first-profile project declaration cost probe"]
    fn measure_maximum_project_declaration() {
        let mut records = (0..MAX_RECORDS - 1)
            .map(|index| format!("\"records/r{index:04}.record.json\""))
            .collect::<Vec<_>>();
        records.push("\"scenes/main.scene.json\"".to_owned());
        let schemas = (0..MAX_SCHEMAS)
            .map(|index| format!("\"schemas/s{index:03}.schema.json\""))
            .collect::<Vec<_>>()
            .join(",");
        let bytes = project()
            .replace(
                "\"records/marker.record.json\",\"scenes/main.scene.json\"",
                &records.join(","),
            )
            .replace("\"schemas/marker.schema.json\"", &schemas);
        let start = std::time::Instant::now();
        let parsed = parse_project_v2(bytes.as_bytes()).unwrap();
        eprintln!(
            "project_bytes={} records={} schemas={} elapsed={:?}",
            bytes.len(),
            parsed.record_paths().len(),
            parsed.schema_paths().len(),
            start.elapsed()
        );
        std::hint::black_box(parsed);
    }
}
