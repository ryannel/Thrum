//! Direct bounded decoding for the deliberately narrow core bootstrap schema.
//!
//! This avoids building a generic JSON value tree for fixed project/scene
//! records. Unknown keys fail before their values are retained.

use std::fmt;

use serde::de::{DeserializeSeed, Error as _, MapAccess, SeqAccess, Visitor};

use crate::SourcePath;
use crate::record::canonical_uuid;

pub(crate) struct ProjectRecord {
    pub(crate) id: String,
    pub(crate) record_paths: Vec<SourcePath>,
    pub(crate) entry_scenes: Vec<String>,
}

pub(crate) struct SceneRecord {
    pub(crate) id: String,
}

pub(crate) fn parse_project(
    bytes: &[u8],
    max_records: usize,
) -> Result<ProjectRecord, serde_json::Error> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let record = ProjectSeed { max_records }.deserialize(&mut deserializer)?;
    deserializer.end()?;
    Ok(record)
}

pub(crate) fn parse_scene(bytes: &[u8]) -> Result<SceneRecord, serde_json::Error> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let record = SceneSeed.deserialize(&mut deserializer)?;
    deserializer.end()?;
    Ok(record)
}

struct ProjectSeed {
    max_records: usize,
}

impl<'de> DeserializeSeed<'de> for ProjectSeed {
    type Value = ProjectRecord;

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(ProjectVisitor {
            max_records: self.max_records,
        })
    }
}

struct ProjectVisitor {
    max_records: usize,
}

impl<'de> Visitor<'de> for ProjectVisitor {
    type Value = ProjectRecord;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a core.project schema 1 record")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut id: Option<String> = None;
        let mut record_type: Option<String> = None;
        let mut schema: Option<u64> = None;
        let mut fields: Option<ProjectFields> = None;
        let mut documentation_seen = false;
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "id" if id.is_none() => id = Some(map.next_value()?),
                "type" if record_type.is_none() => record_type = Some(map.next_value()?),
                "schema" if schema.is_none() => schema = Some(map.next_value()?),
                "fields" if fields.is_none() => {
                    fields = Some(map.next_value_seed(ProjectFieldsSeed {
                        max_records: self.max_records,
                    })?);
                }
                "documentation" if !documentation_seen => {
                    let _: String = map.next_value()?;
                    documentation_seen = true;
                }
                _ => {
                    return Err(A::Error::custom(
                        "duplicate or unsupported project record key",
                    ));
                }
            }
        }
        let id = id.ok_or_else(|| A::Error::custom("missing project ID"))?;
        if !canonical_uuid(&id)
            || record_type.as_deref() != Some("core.project")
            || schema != Some(1)
        {
            return Err(A::Error::custom("invalid project identity, type or schema"));
        }
        let fields = fields.ok_or_else(|| A::Error::custom("missing project fields"))?;
        Ok(ProjectRecord {
            id,
            record_paths: fields.record_paths,
            entry_scenes: fields.entry_scenes,
        })
    }
}

struct ProjectFields {
    record_paths: Vec<SourcePath>,
    entry_scenes: Vec<String>,
}

struct ProjectFieldsSeed {
    max_records: usize,
}

impl<'de> DeserializeSeed<'de> for ProjectFieldsSeed {
    type Value = ProjectFields;

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(ProjectFieldsVisitor {
            max_records: self.max_records,
        })
    }
}

struct ProjectFieldsVisitor {
    max_records: usize,
}

impl<'de> Visitor<'de> for ProjectFieldsVisitor {
    type Value = ProjectFields;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("fixed core.project fields")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut format: Option<u64> = None;
        let mut paths: Option<Vec<SourcePath>> = None;
        let mut entries: Option<Vec<String>> = None;
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "project.format" if format.is_none() => format = Some(map.next_value()?),
                "project.records" if paths.is_none() => {
                    paths = Some(map.next_value_seed(PathArraySeed {
                        limit: self.max_records,
                    })?);
                }
                "project.entryScenes" if entries.is_none() => {
                    entries = Some(map.next_value_seed(EntryArraySeed {
                        limit: self.max_records,
                    })?);
                }
                _ => return Err(A::Error::custom("duplicate or unsupported project field")),
            }
        }
        if format != Some(1) {
            return Err(A::Error::custom("unsupported project format"));
        }
        let record_paths = paths.ok_or_else(|| A::Error::custom("missing project.records"))?;
        let entry_scenes =
            entries.ok_or_else(|| A::Error::custom("missing project.entryScenes"))?;
        if entry_scenes.is_empty() || entry_scenes.len() > record_paths.len() {
            return Err(A::Error::custom(
                "entry-scene count exceeds declared records",
            ));
        }
        Ok(ProjectFields {
            record_paths,
            entry_scenes,
        })
    }
}

struct PathArraySeed {
    limit: usize,
}

impl<'de> DeserializeSeed<'de> for PathArraySeed {
    type Value = Vec<SourcePath>;

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_seq(PathArrayVisitor { limit: self.limit })
    }
}

struct PathArrayVisitor {
    limit: usize,
}

impl<'de> Visitor<'de> for PathArrayVisitor {
    type Value = Vec<SourcePath>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("sorted portable scene paths")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
        let mut paths = Vec::new();
        loop {
            if paths.len() >= self.limit {
                sequence.next_element_seed(RejectExtraSeed)?;
                break;
            }
            let Some(text) = sequence.next_element::<String>()? else {
                break;
            };
            let path = SourcePath::parse(&text)
                .map_err(|_| A::Error::custom("project.records path is not portable"))?;
            if !is_scene_path(&path) {
                return Err(A::Error::custom("unsupported engine record path"));
            }
            if paths.last().is_some_and(|previous| previous >= &path) {
                return Err(A::Error::custom("project.records is not strictly sorted"));
            }
            paths.push(path);
        }
        Ok(paths)
    }
}

struct EntryArraySeed {
    limit: usize,
}

impl<'de> DeserializeSeed<'de> for EntryArraySeed {
    type Value = Vec<String>;

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_seq(EntryArrayVisitor { limit: self.limit })
    }
}

struct EntryArrayVisitor {
    limit: usize,
}

impl<'de> Visitor<'de> for EntryArrayVisitor {
    type Value = Vec<String>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("sorted stable entry-scene IDs")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
        let mut entries: Vec<String> = Vec::new();
        loop {
            if entries.len() >= self.limit {
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
                return Err(A::Error::custom(
                    "project.entryScenes is not strictly sorted",
                ));
            }
            entries.push(id);
        }
        Ok(entries)
    }
}

struct SceneSeed;

struct RejectExtraSeed;

impl<'de> DeserializeSeed<'de> for RejectExtraSeed {
    type Value = ();

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        _deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        Err(D::Error::custom("record array exceeds staged membership"))
    }
}

impl<'de> DeserializeSeed<'de> for SceneSeed {
    type Value = SceneRecord;

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(SceneVisitor)
    }
}

struct SceneVisitor;

impl<'de> Visitor<'de> for SceneVisitor {
    type Value = SceneRecord;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a core.scene schema 1 record")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut id: Option<String> = None;
        let mut record_type: Option<String> = None;
        let mut schema: Option<u64> = None;
        let mut fields_seen = false;
        let mut documentation_seen = false;
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "id" if id.is_none() => id = Some(map.next_value()?),
                "type" if record_type.is_none() => record_type = Some(map.next_value()?),
                "schema" if schema.is_none() => schema = Some(map.next_value()?),
                "fields" if !fields_seen => {
                    map.next_value_seed(SceneFieldsSeed)?;
                    fields_seen = true;
                }
                "documentation" if !documentation_seen => {
                    let _: String = map.next_value()?;
                    documentation_seen = true;
                }
                _ => {
                    return Err(A::Error::custom(
                        "duplicate or unsupported scene record key",
                    ));
                }
            }
        }
        let id = id.ok_or_else(|| A::Error::custom("missing scene ID"))?;
        if !canonical_uuid(&id)
            || record_type.as_deref() != Some("core.scene")
            || schema != Some(1)
            || !fields_seen
        {
            return Err(A::Error::custom("invalid scene identity, type or schema"));
        }
        Ok(SceneRecord { id })
    }
}

struct SceneFieldsSeed;

impl<'de> DeserializeSeed<'de> for SceneFieldsSeed {
    type Value = ();

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(SceneFieldsVisitor)
    }
}

struct SceneFieldsVisitor;

impl<'de> Visitor<'de> for SceneFieldsVisitor {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("empty scene fields or an optional scene.label string")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut label_seen = false;
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "scene.label" if !label_seen => {
                    let _: String = map.next_value()?;
                    label_seen = true;
                }
                _ => return Err(A::Error::custom("duplicate or unsupported scene field")),
            }
        }
        Ok(())
    }
}

fn is_scene_path(path: &SourcePath) -> bool {
    let text = path.as_str();
    text.strip_prefix("scenes/")
        .is_some_and(|name| !name.contains('/') && name.ends_with(".scene.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROJECT_ID: &str = "2a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    const SCENE_ID: &str = "3a5ef644-7225-4770-9f4d-e1d9c0a7d515";

    #[test]
    fn fixed_project_and_scene_decode_without_generic_value_tree() {
        let project = format!(
            "{{\"fields\":{{\"project.entryScenes\":[\"{SCENE_ID}\"],\"project.records\":[\"scenes/main.scene.json\"],\"project.format\":1}},\"documentation\":\"notes\",\"schema\":1,\"type\":\"core.project\",\"id\":\"{PROJECT_ID}\"}}"
        );
        let parsed = parse_project(project.as_bytes(), 1).unwrap();
        assert_eq!(parsed.id, PROJECT_ID);
        assert_eq!(parsed.record_paths[0].as_str(), "scenes/main.scene.json");
        assert_eq!(parsed.entry_scenes, [SCENE_ID]);

        let scene = format!(
            "{{\"fields\":{{\"scene.label\":\"hello\"}},\"type\":\"core.scene\",\"id\":\"{SCENE_ID}\",\"schema\":1}}"
        );
        assert_eq!(parse_scene(scene.as_bytes()).unwrap().id, SCENE_ID);
    }

    #[test]
    fn duplicate_unknown_and_wrong_type_refuse_before_retaining_payload() {
        let valid = format!(
            "{{\"id\":\"{SCENE_ID}\",\"type\":\"core.scene\",\"schema\":1,\"fields\":{{}}}}"
        );
        assert!(
            parse_scene(
                valid
                    .replace("\"schema\":1", "\"schema\":1,\"schema\":1")
                    .as_bytes()
            )
            .is_err()
        );
        assert!(
            parse_scene(
                valid
                    .replace(
                        "\"fields\":{}",
                        "\"fields\":{\"scene.label\":\"a\",\"scene.label\":\"b\"}"
                    )
                    .as_bytes()
            )
            .is_err()
        );
        assert!(
            parse_scene(
                valid
                    .replace("\"fields\":{}", "\"fields\":{\"scene.objects\":[[[[0]]]]}")
                    .as_bytes()
            )
            .is_err()
        );
        assert!(parse_scene(format!("{valid} trailing").as_bytes()).is_err());
    }

    #[test]
    fn array_limit_refuses_extra_before_deserializing_its_value() {
        let project = format!(
            "{{\"id\":\"{PROJECT_ID}\",\"type\":\"core.project\",\"schema\":1,\"fields\":{{\"project.format\":1,\"project.records\":[\"scenes/a.scene.json\",[[[[[[[0]]]]]]]],\"project.entryScenes\":[\"{SCENE_ID}\"]}}}}"
        );
        let error = parse_project(project.as_bytes(), 1).err().unwrap();
        assert!(error.to_string().contains("exceeds staged membership"));
    }

    #[test]
    #[ignore = "local dense unsupported scene record refusal cost probe"]
    fn measure_dense_unsupported_scene_refusal() {
        let mut bytes =
            format!("{{\"id\":\"{SCENE_ID}\",\"type\":\"core.scene\",\"schema\":1,\"fields\":{{")
                .into_bytes();
        for index in 0..900_000 {
            if index != 0 {
                bytes.push(b',');
            }
            bytes.push(b'"');
            bytes.push(b'k');
            bytes.extend_from_slice(index.to_string().as_bytes());
            bytes.extend_from_slice(b"\":0");
        }
        bytes.extend_from_slice(b"}}");
        assert!(bytes.len() <= crate::MAX_RECORD_BYTES);
        let start = std::time::Instant::now();
        let rejected = parse_scene(&bytes);
        let elapsed = start.elapsed();
        assert!(rejected.is_err());
        eprintln!(
            "unsupported_scene_input_bytes={} refusal_elapsed={elapsed:?}",
            bytes.len()
        );
    }
}
