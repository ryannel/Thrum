//! Direct first-profile game-record validation under a parsed source descriptor.
//!
//! Unknown values are allowed only under the descriptor's optional prefix and
//! receive strict bounded syntax checking. Exact staged bytes remain the
//! round-trip authority; this parser does not edit or reserialize them.

use std::cell::Cell;
use std::collections::HashSet;
use std::fmt;

use serde::de::{DeserializeSeed, Error as _, MapAccess, SeqAccess, Visitor};

use crate::MAX_RECORD_BYTES;
use crate::definition::{Presence, valid_symbol};
use crate::record::canonical_uuid;
use crate::schema_json::{SchemaDescriptor, SchemaFieldKind};

const MAX_OPAQUE_FIELDS: usize = 128;
const MAX_RECORD_FIELDS: usize = 256;
const MAX_OPAQUE_NODES: usize = 65_536;
const MAX_OPAQUE_DEPTH: u8 = 64;
const OPAQUE_START_DEPTH: u8 = 2;
const MAX_OPAQUE_OBJECT_MEMBERS: usize = 1_024;
const MAX_OPAQUE_KEY_BYTES: usize = 128;
const MAX_OPAQUE_STRING_BYTES: usize = 65_536;

/// A game record failed the first-profile descriptor or bounded JSON rules.
#[derive(Debug)]
pub enum GameRecordError {
    /// Exact JSON bytes exceed the 16 MiB record bound.
    TooLarge,
    /// Syntax, known-field semantics, opaque limits or envelope refused.
    Invalid(serde_json::Error),
}

/// One required typed reference emitted by a parsed game record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GameReference {
    field_index: u8,
    target_id: String,
}

impl GameReference {
    /// Index into the pinned descriptor's sorted field list.
    pub fn field_index(&self) -> u8 {
        self.field_index
    }

    /// Canonical stable ID of the required target record.
    pub fn target_id(&self) -> &str {
        &self.target_id
    }
}

/// Bounded identity and reference summary of one checked game record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GameRecordSummary {
    id: String,
    record_type: String,
    schema: u64,
    definition_hash: [u8; 32],
    references: Vec<GameReference>,
}

impl GameRecordSummary {
    /// Stable record ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Descriptor type ID used for validation.
    pub fn record_type(&self) -> &str {
        &self.record_type
    }

    /// Descriptor target version used for validation.
    pub fn schema(&self) -> u64 {
        self.schema
    }

    /// Hash of the exact descriptor definition used for this validation.
    pub fn definition_hash(&self) -> &[u8; 32] {
        &self.definition_hash
    }

    /// Required typed references, with indices into the same descriptor.
    pub fn references(&self) -> &[GameReference] {
        &self.references
    }
}

/// Validate one supplied game record against a source-carried descriptor.
///
/// This checks one record only. A staged-tree pass must rehash these bytes,
/// verify declaration/classification, and resolve reference IDs and types.
pub fn parse_game_record(
    bytes: &[u8],
    descriptor: &SchemaDescriptor,
) -> Result<GameRecordSummary, GameRecordError> {
    if bytes.len() > MAX_RECORD_BYTES {
        return Err(GameRecordError::TooLarge);
    }
    let nodes = Cell::new(0);
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let record = GameRecordSeed {
        descriptor,
        nodes: &nodes,
    }
    .deserialize(&mut deserializer)
    .map_err(GameRecordError::Invalid)?;
    deserializer.end().map_err(GameRecordError::Invalid)?;
    Ok(record)
}

/// Read only the top-level type discriminator before descriptor-bound parsing.
///
/// This does not validate game semantics; the selected descriptor must parse
/// the entire record afterward. Non-type values are traversed without a JSON
/// tree, so field order cannot change which descriptor is selected.
#[cfg(test)]
pub(crate) fn game_record_type(bytes: &[u8]) -> Result<String, GameRecordError> {
    if bytes.len() > MAX_RECORD_BYTES {
        return Err(GameRecordError::TooLarge);
    }
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let nodes = Cell::new(0);
    let record_type = RecordTypeSeed { nodes: &nodes }
        .deserialize(&mut deserializer)
        .map_err(GameRecordError::Invalid)?;
    deserializer.end().map_err(GameRecordError::Invalid)?;
    Ok(record_type)
}

#[cfg(test)]
struct RecordTypeSeed<'a> {
    nodes: &'a Cell<usize>,
}

#[cfg(test)]
impl<'de> DeserializeSeed<'de> for RecordTypeSeed<'_> {
    type Value = String;

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(RecordTypeVisitor { nodes: self.nodes })
    }
}

#[cfg(test)]
struct RecordTypeVisitor<'a> {
    nodes: &'a Cell<usize>,
}

#[cfg(test)]
impl<'de> Visitor<'de> for RecordTypeVisitor<'_> {
    type Value = String;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a game record with one type discriminator")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut record_type = None;
        let mut id_seen = false;
        let mut schema_seen = false;
        let mut fields_seen = false;
        let mut documentation_seen = false;
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "type" if record_type.is_none() => {
                    record_type = Some(map.next_value::<String>()?);
                }
                "id" if !id_seen => {
                    let _: String = map.next_value()?;
                    id_seen = true;
                }
                "schema" if !schema_seen => {
                    let _: u64 = map.next_value()?;
                    schema_seen = true;
                }
                "fields" if !fields_seen => {
                    map.next_value_seed(OpaqueSeed {
                        depth: 1,
                        nodes: self.nodes,
                        node_limit: MAX_OPAQUE_NODES + MAX_RECORD_FIELDS + 2,
                    })?;
                    fields_seen = true;
                }
                "documentation" if !documentation_seen => {
                    let _: String = map.next_value()?;
                    documentation_seen = true;
                }
                _ => return Err(A::Error::custom("duplicate or unsupported game record key")),
            }
        }
        record_type.ok_or_else(|| A::Error::custom("missing game record type"))
    }
}

struct GameRecordSeed<'a> {
    descriptor: &'a SchemaDescriptor,
    nodes: &'a Cell<usize>,
}

impl<'de> DeserializeSeed<'de> for GameRecordSeed<'_> {
    type Value = GameRecordSummary;

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(GameRecordVisitor {
            descriptor: self.descriptor,
            nodes: self.nodes,
        })
    }
}

struct GameRecordVisitor<'a> {
    descriptor: &'a SchemaDescriptor,
    nodes: &'a Cell<usize>,
}

impl<'de> Visitor<'de> for GameRecordVisitor<'_> {
    type Value = GameRecordSummary;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a first-profile game record")
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
                "fields" if fields.is_none() => {
                    fields = Some(map.next_value_seed(GameFieldsSeed {
                        descriptor: self.descriptor,
                        nodes: self.nodes,
                    })?);
                }
                "documentation" if !documentation_seen => {
                    let _: String = map.next_value()?;
                    documentation_seen = true;
                }
                _ => return Err(A::Error::custom("duplicate or unsupported game record key")),
            }
        }
        let id: String = id.ok_or_else(|| A::Error::custom("missing game record ID"))?;
        if !canonical_uuid(&id)
            || record_type.as_deref() != Some(self.descriptor.target_type())
            || schema != Some(self.descriptor.target_version())
        {
            return Err(A::Error::custom(
                "invalid game record identity, type or schema",
            ));
        }
        let references = fields.ok_or_else(|| A::Error::custom("missing game record fields"))?;
        Ok(GameRecordSummary {
            id,
            record_type: self.descriptor.target_type().to_owned(),
            schema: self.descriptor.target_version(),
            definition_hash: *self.descriptor.definition_hash(),
            references,
        })
    }
}

struct GameFieldsSeed<'a> {
    descriptor: &'a SchemaDescriptor,
    nodes: &'a Cell<usize>,
}

impl<'de> DeserializeSeed<'de> for GameFieldsSeed<'_> {
    type Value = Vec<GameReference>;

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(GameFieldsVisitor {
            descriptor: self.descriptor,
            nodes: self.nodes,
        })
    }
}

struct GameFieldsVisitor<'a> {
    descriptor: &'a SchemaDescriptor,
    nodes: &'a Cell<usize>,
}

impl<'de> Visitor<'de> for GameFieldsVisitor<'_> {
    type Value = Vec<GameReference>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("bounded known fields and declared optional opaque fields")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut seen = [0_u64; 2];
        let mut field_count = 0;
        let mut opaque_count = 0;
        let mut opaque_names: Option<HashSet<String>> = None;
        let mut references = Vec::new();
        loop {
            if field_count >= MAX_RECORD_FIELDS {
                map.next_key_seed(RejectExtraKeySeed)?;
                break;
            }
            let Some(key) = map.next_key::<String>()? else {
                break;
            };
            field_count += 1;
            match self
                .descriptor
                .fields()
                .binary_search_by(|field| field.id().cmp(&key))
            {
                Ok(index) => {
                    let word = index / 64;
                    let mask = 1_u64 << (index % 64);
                    if seen[word] & mask != 0 {
                        return Err(A::Error::custom("duplicate known game field"));
                    }
                    seen[word] |= mask;
                    match self.descriptor.fields()[index].kind() {
                        SchemaFieldKind::Text { max_utf8_bytes } => {
                            map.next_value_seed(TextSeed {
                                limit: usize::from(*max_utf8_bytes),
                            })?;
                        }
                        SchemaFieldKind::RecordRef { .. } => {
                            let target_id = map.next_value_seed(ReferenceSeed)?;
                            references.push(GameReference {
                                field_index: u8::try_from(index)
                                    .expect("descriptor has at most 128 fields"),
                                target_id,
                            });
                        }
                    }
                }
                Err(_) => {
                    let allowed = self
                        .descriptor
                        .optional_extension_prefix()
                        .is_some_and(|prefix| key.starts_with(prefix));
                    if !allowed || !valid_symbol(&key) {
                        return Err(A::Error::custom("unknown required game field"));
                    }
                    if opaque_count >= MAX_OPAQUE_FIELDS {
                        return Err(A::Error::custom("optional field count limit"));
                    }
                    let names = opaque_names.get_or_insert_with(HashSet::new);
                    if !names.insert(key) {
                        return Err(A::Error::custom("duplicate optional game field"));
                    }
                    opaque_count += 1;
                    map.next_value_seed(OpaqueSeed {
                        depth: OPAQUE_START_DEPTH,
                        nodes: self.nodes,
                        node_limit: MAX_OPAQUE_NODES,
                    })?;
                }
            }
        }
        for (index, field) in self.descriptor.fields().iter().enumerate() {
            if field.presence() == Presence::Required
                && seen[index / 64] & (1_u64 << (index % 64)) == 0
            {
                return Err(A::Error::custom("missing required game field"));
            }
        }
        Ok(references)
    }
}

struct RejectExtraKeySeed;

impl<'de> DeserializeSeed<'de> for RejectExtraKeySeed {
    type Value = String;

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        _deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        Err(D::Error::custom("game field count limit"))
    }
}

struct TextSeed {
    limit: usize,
}

impl<'de> DeserializeSeed<'de> for TextSeed {
    type Value = ();

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_str(TextVisitor { limit: self.limit })
    }
}

struct TextVisitor {
    limit: usize,
}

impl Visitor<'_> for TextVisitor {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("bounded UTF-8 text")
    }

    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
        if value.len() > self.limit {
            return Err(E::custom("text byte limit"));
        }
        Ok(())
    }

    fn visit_string<E: serde::de::Error>(self, value: String) -> Result<Self::Value, E> {
        self.visit_str(&value)
    }
}

struct ReferenceSeed;

impl<'de> DeserializeSeed<'de> for ReferenceSeed {
    type Value = String;

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_str(ReferenceVisitor)
    }
}

struct ReferenceVisitor;

impl Visitor<'_> for ReferenceVisitor {
    type Value = String;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a canonical target record UUID")
    }

    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
        if !canonical_uuid(value) {
            return Err(E::custom("invalid target record UUID"));
        }
        Ok(value.to_owned())
    }

    fn visit_string<E: serde::de::Error>(self, value: String) -> Result<Self::Value, E> {
        if !canonical_uuid(&value) {
            return Err(E::custom("invalid target record UUID"));
        }
        Ok(value)
    }
}

struct OpaqueSeed<'a> {
    depth: u8,
    nodes: &'a Cell<usize>,
    node_limit: usize,
}

impl<'de> DeserializeSeed<'de> for OpaqueSeed<'_> {
    type Value = ();

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        let count = self.nodes.get();
        if count >= self.node_limit {
            return Err(D::Error::custom("opaque node count limit"));
        }
        self.nodes.set(count + 1);
        deserializer.deserialize_any(OpaqueVisitor {
            depth: self.depth,
            nodes: self.nodes,
            node_limit: self.node_limit,
        })
    }
}

struct OpaqueVisitor<'a> {
    depth: u8,
    nodes: &'a Cell<usize>,
    node_limit: usize,
}

impl<'de> Visitor<'de> for OpaqueVisitor<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a bounded strict optional JSON value")
    }

    fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_bool<E: serde::de::Error>(self, _value: bool) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_i64<E: serde::de::Error>(self, _value: i64) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_u64<E: serde::de::Error>(self, _value: u64) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Self::Value, E> {
        if !value.is_finite() {
            return Err(E::custom("nonfinite optional number"));
        }
        Ok(())
    }

    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
        if value.len() > MAX_OPAQUE_STRING_BYTES {
            return Err(E::custom("opaque string byte limit"));
        }
        Ok(())
    }

    fn visit_string<E: serde::de::Error>(self, value: String) -> Result<Self::Value, E> {
        self.visit_str(&value)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
        if self.depth >= MAX_OPAQUE_DEPTH {
            return Err(A::Error::custom("opaque depth limit"));
        }
        while sequence
            .next_element_seed(OpaqueSeed {
                depth: self.depth + 1,
                nodes: self.nodes,
                node_limit: self.node_limit,
            })?
            .is_some()
        {}
        Ok(())
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        if self.depth >= MAX_OPAQUE_DEPTH {
            return Err(A::Error::custom("opaque depth limit"));
        }
        let mut keys = HashSet::new();
        loop {
            if keys.len() >= MAX_OPAQUE_OBJECT_MEMBERS {
                map.next_key_seed(RejectOpaqueKeySeed)?;
                break;
            }
            let Some(key) = map.next_key::<String>()? else {
                break;
            };
            if key.len() > MAX_OPAQUE_KEY_BYTES {
                return Err(A::Error::custom("opaque key byte limit"));
            }
            if !keys.insert(key) {
                return Err(A::Error::custom("duplicate opaque JSON key"));
            }
            map.next_value_seed(OpaqueSeed {
                depth: self.depth + 1,
                nodes: self.nodes,
                node_limit: self.node_limit,
            })?;
        }
        Ok(())
    }
}

struct RejectOpaqueKeySeed;

impl<'de> DeserializeSeed<'de> for RejectOpaqueKeySeed {
    type Value = String;

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        _deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        Err(D::Error::custom("opaque object member limit"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema_json::parse_schema_descriptor;
    use serde_json::json;

    const SCHEMA_ID: &str = "2a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    const RECORD_ID: &str = "3a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    const SCENE_ID: &str = "4a5ef644-7225-4770-9f4d-e1d9c0a7d515";

    fn schema() -> SchemaDescriptor {
        let bytes = serde_json::to_vec(&json!({
            "id":SCHEMA_ID, "type":"core.schema", "schema":1,
            "fields":{
                "schema.targetType":"game.marker", "schema.targetVersion":1,
                "schema.optionalExtensionPrefix":"x.game.marker.",
                "schema.fields":[
                    {"id":"marker.caption", "kind":"text", "presence":"optional", "maxUtf8Bytes":64},
                    {"id":"marker.scene", "kind":"record-ref", "presence":"required", "targetType":"core.scene", "resolution":"required"}
                ]
            }
        }))
        .unwrap();
        parse_schema_descriptor(&bytes).unwrap()
    }

    fn record() -> String {
        format!(
            "{{\"id\":\"{RECORD_ID}\",\"type\":\"game.marker\",\"schema\":1,\"fields\":{{\"marker.caption\":\"hello\",\"marker.scene\":\"{SCENE_ID}\",\"x.game.marker.note\":{{\"a\":[1,true,null]}}}}}}"
        )
    }

    #[test]
    fn type_discriminator_uses_only_top_level_key_and_rejects_duplicates() {
        let bytes =
            br#"{"fields":{"nested":{"type":"game.fake"}},"schema":1,"type":"game.marker"}"#;
        assert_eq!(game_record_type(bytes).unwrap(), "game.marker");
        assert!(game_record_type(br#"{"type":"game.marker","type":"game.fake"}"#).is_err());
        assert!(game_record_type(br#"{"fields":{"type":"game.fake"}}"#).is_err());
        assert!(game_record_type(br#"{"type":"game.marker","fields":[]"#).is_err());
        let unknown = game_record_type(br#"{"unknown":[null],"type":"game.marker"}"#).unwrap_err();
        assert!(format!("{unknown:?}").contains("unsupported game record key"));
    }

    #[test]
    fn type_discriminator_bounds_deep_and_dense_fields_before_dispatch() {
        let deep = format!(
            "{{\"fields\":{}0{},\"type\":\"game.marker\"}}",
            "[".repeat(65),
            "]".repeat(65)
        );
        let error = game_record_type(deep.as_bytes()).unwrap_err();
        assert!(format!("{error:?}").contains("opaque depth limit"));
        let dense = format!(
            "{{\"fields\":[{}],\"type\":\"game.marker\"}}",
            "null,".repeat(MAX_OPAQUE_NODES + MAX_RECORD_FIELDS + 2)
        );
        let error = game_record_type(dense.as_bytes()).unwrap_err();
        assert!(format!("{error:?}").contains("opaque node count limit"));
    }

    #[test]
    fn known_fields_and_strict_opaque_value_emit_only_typed_reference() {
        let schema = schema();
        let bytes = record();
        let parsed = parse_game_record(bytes.as_bytes(), &schema).unwrap();
        assert_eq!(parsed.id(), RECORD_ID);
        assert_eq!(parsed.record_type(), "game.marker");
        assert_eq!(parsed.schema(), 1);
        assert_eq!(parsed.definition_hash(), schema.definition_hash());
        assert_eq!(parsed.references().len(), 1);
        assert_eq!(parsed.references()[0].field_index(), 1);
        assert_eq!(parsed.references()[0].target_id(), SCENE_ID);
        let changed = bytes.replace("hello", "world");
        assert_eq!(
            &bytes[bytes.find("\"x.game.marker.note\"").unwrap()..],
            &changed[changed.find("\"x.game.marker.note\"").unwrap()..]
        );
        assert_eq!(
            parse_game_record(changed.as_bytes(), &schema)
                .unwrap()
                .references(),
            parsed.references()
        );
    }

    #[test]
    fn required_and_unknown_fields_and_nested_duplicates_refuse() {
        let schema = schema();
        let valid = record();
        let without_caption = valid.replace("\"marker.caption\":\"hello\",", "");
        assert!(parse_game_record(without_caption.as_bytes(), &schema).is_ok());
        let unresolved_here = valid.replace(SCENE_ID, RECORD_ID);
        assert!(parse_game_record(unresolved_here.as_bytes(), &schema).is_ok());
        for changed in [
            valid.replace(&format!("\"marker.scene\":\"{SCENE_ID}\","), ""),
            valid.replace("\"hello\"", "123"),
            valid.replace("\"hello\"", &format!("\"{}\"", "é".repeat(33))),
            valid.replace(SCENE_ID, "not-a-uuid"),
            valid.replace("\"a\":[1,true,null]", "\"a\":0,\"a\":1"),
            valid.replace("x.game.marker.note", "other.note"),
            valid.replace("\"schema\":1", "\"schema\":2"),
            format!("{valid} trailing"),
        ] {
            assert!(
                parse_game_record(changed.as_bytes(), &schema).is_err(),
                "{changed}"
            );
        }
    }

    #[test]
    fn opaque_node_and_string_limits_refuse() {
        let schema = schema();
        let many = format!(
            "{{\"id\":\"{RECORD_ID}\",\"type\":\"game.marker\",\"schema\":1,\"fields\":{{\"marker.scene\":\"{SCENE_ID}\",\"x.game.marker.note\":[{}]}}}}",
            vec!["0"; MAX_OPAQUE_NODES].join(",")
        );
        let error = parse_game_record(many.as_bytes(), &schema).err().unwrap();
        assert!(format!("{error:?}").contains("opaque node count limit"));
        let huge = record().replace(
            "[1,true,null]",
            &format!("\"{}\"", "a".repeat(MAX_OPAQUE_STRING_BYTES + 1)),
        );
        assert!(parse_game_record(huge.as_bytes(), &schema).is_err());

        let object = (0..MAX_OPAQUE_OBJECT_MEMBERS)
            .map(|index| format!("\"k{index}\":0"))
            .collect::<Vec<_>>()
            .join(",");
        let extra = record().replace(
            "\"a\":[1,true,null]",
            &format!("{object},\"extra\":[[[0]]]"),
        );
        let error = parse_game_record(extra.as_bytes(), &schema).err().unwrap();
        assert!(format!("{error:?}").contains("opaque object member limit"));

        let opaque_fields = (0..=MAX_OPAQUE_FIELDS)
            .map(|index| format!("\"x.game.marker.f{index:03}\":0"))
            .collect::<Vec<_>>()
            .join(",");
        let extra = record().replace(
            "\"x.game.marker.note\":{\"a\":[1,true,null]}",
            &opaque_fields,
        );
        let error = parse_game_record(extra.as_bytes(), &schema).err().unwrap();
        assert!(format!("{error:?}").contains("optional field count limit"));

        let allowed = record().replace(
            "{\"a\":[1,true,null]}",
            &format!("{}0{}", "[".repeat(62), "]".repeat(62)),
        );
        assert!(parse_game_record(allowed.as_bytes(), &schema).is_ok());
        let too_deep = record().replace(
            "{\"a\":[1,true,null]}",
            &format!("{}0{}", "[".repeat(63), "]".repeat(63)),
        );
        let error = parse_game_record(too_deep.as_bytes(), &schema)
            .err()
            .unwrap();
        assert!(format!("{error:?}").contains("opaque depth limit"));
    }

    #[test]
    #[ignore = "local dense optional JSON streaming cost probe"]
    fn measure_dense_optional_record() {
        let schema = schema();
        let values = vec!["0"; MAX_OPAQUE_NODES - 2].join(",");
        let bytes = record().replace("[1,true,null]", &format!("[{values}]"));
        let start = std::time::Instant::now();
        let summary = parse_game_record(bytes.as_bytes(), &schema).unwrap();
        eprintln!(
            "game_record_bytes={} opaque_nodes={} elapsed={:?}",
            bytes.len(),
            MAX_OPAQUE_NODES,
            start.elapsed()
        );
        std::hint::black_box(summary);
    }

    #[test]
    #[ignore = "local deep and wide opaque key-set peak probe"]
    fn measure_deep_wide_optional_record() {
        let schema = schema();
        let long_tail = "x".repeat(123);
        let siblings = (0..MAX_OPAQUE_OBJECT_MEMBERS - 1)
            .map(|index| format!("\"k{index:04}{long_tail}\":0"))
            .collect::<Vec<_>>()
            .join(",");
        let opaque_maps = usize::from(MAX_OPAQUE_DEPTH - OPAQUE_START_DEPTH);
        let mut nested = String::with_capacity(opaque_maps * (siblings.len() + 12) + 64);
        for _ in 0..opaque_maps {
            nested.push('{');
            nested.push_str(&siblings);
            nested.push_str(",\"z.next\":");
        }
        nested.push('0');
        for _ in 0..opaque_maps {
            nested.push('}');
        }
        let bytes = record().replace("{\"a\":[1,true,null]}", &nested);
        assert!(bytes.len() <= MAX_RECORD_BYTES);
        let start = std::time::Instant::now();
        let summary = parse_game_record(bytes.as_bytes(), &schema).unwrap();
        eprintln!(
            "deep_wide_record_bytes={} maps={} members_per_map=1024 elapsed={:?}",
            bytes.len(),
            opaque_maps,
            start.elapsed()
        );
        std::hint::black_box(summary);
    }
}
