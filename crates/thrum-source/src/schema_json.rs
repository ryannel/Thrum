//! Bounded direct decoding of one first-profile `core.schema` source record.
//!
//! This parses supplied bytes only. A source-capture validator must verify
//! staged bytes, declared path, active history and graph closure separately.

use std::fmt;

use serde::de::{DeserializeSeed, Error as _, MapAccess, SeqAccess, Visitor};

use crate::MAX_RECORD_BYTES;
use crate::definition::{
    DefinitionError, FieldDefinition, FieldKind, MAX_FIELDS, Presence, VersionField,
    field_meaning_signature, type_version_definition_hash,
};
use crate::record::canonical_uuid;

/// A source schema record failed the fixed first-profile grammar.
#[derive(Debug)]
pub enum SchemaError {
    /// Exact JSON bytes exceed the 16 MiB source-record bound.
    TooLarge,
    /// JSON syntax, duplicate keys, field count or fixed record shape refused.
    Invalid(serde_json::Error),
    /// Field or type-version semantics cannot receive canonical hashes.
    Definition(DefinitionError),
}

/// Value kind retained from a checked source-carried field descriptor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SchemaFieldKind {
    /// UTF-8 text bounded by encoded byte length.
    Text {
        /// Maximum encoded bytes from 1 through 4,096.
        max_utf8_bytes: u16,
    },
    /// A canonical record UUID with required typed resolution.
    RecordRef {
        /// The first profile permits only `core.scene`.
        target_type: String,
    },
}

/// One checked field and its canonical meaning signature.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaField {
    id: String,
    presence: Presence,
    kind: SchemaFieldKind,
    signature: [u8; 32],
}

impl SchemaField {
    /// Permanent field ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Required or optional presence in a record.
    pub fn presence(&self) -> Presence {
        self.presence
    }

    /// Bounded first-profile value kind.
    pub fn kind(&self) -> &SchemaFieldKind {
        &self.kind
    }

    /// Canonical field meaning signature.
    pub fn signature(&self) -> &[u8; 32] {
        &self.signature
    }
}

/// One fixed-grammar source schema, before history or stage validation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaDescriptor {
    record_id: String,
    target_type: String,
    target_version: u64,
    fields: Vec<SchemaField>,
    optional_extension_prefix: Option<String>,
    definition_hash: [u8; 32],
}

impl SchemaDescriptor {
    /// Stable ID of the `core.schema` source record.
    pub fn record_id(&self) -> &str {
        &self.record_id
    }

    /// Game-owned type described by this record.
    pub fn target_type(&self) -> &str {
        &self.target_type
    }

    /// Positive target schema version.
    pub fn target_version(&self) -> u64 {
        self.target_version
    }

    /// Strictly sorted active field definitions.
    pub fn fields(&self) -> &[SchemaField] {
        &self.fields
    }

    /// Explicit prefix under which unknown fields remain optional and opaque.
    pub fn optional_extension_prefix(&self) -> Option<&str> {
        self.optional_extension_prefix.as_deref()
    }

    /// Canonical hash of this immutable type-version definition.
    pub fn definition_hash(&self) -> &[u8; 32] {
        &self.definition_hash
    }
}

/// Decode one `core.schema` schema 1 record without a generic JSON tree.
///
/// This checks fixed keys, duplicate keys, at most 128 sorted fields and the
/// canonical first-profile hash inputs. It does not validate a project tree.
pub fn parse_schema_descriptor(bytes: &[u8]) -> Result<SchemaDescriptor, SchemaError> {
    if bytes.len() > MAX_RECORD_BYTES {
        return Err(SchemaError::TooLarge);
    }
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let raw = SchemaSeed
        .deserialize(&mut deserializer)
        .map_err(SchemaError::Invalid)?;
    deserializer.end().map_err(SchemaError::Invalid)?;
    if !canonical_uuid(&raw.record_id) || raw.record_type != "core.schema" || raw.schema != 1 {
        return Err(SchemaError::Invalid(serde::de::Error::custom(
            "invalid core.schema envelope",
        )));
    }

    let mut fields = raw.fields.fields;
    let mut version_fields = Vec::with_capacity(fields.len());
    for field in &mut fields {
        let kind = match &field.kind {
            SchemaFieldKind::Text { max_utf8_bytes } => FieldKind::Text {
                max_utf8_bytes: *max_utf8_bytes,
            },
            SchemaFieldKind::RecordRef { target_type } => FieldKind::RecordRef { target_type },
        };
        field.signature = field_meaning_signature(FieldDefinition {
            type_id: &raw.fields.target_type,
            field_id: &field.id,
            presence: field.presence,
            kind,
        })
        .map_err(SchemaError::Definition)?;
        version_fields.push(VersionField {
            field_id: &field.id,
            signature: field.signature,
        });
    }
    let definition_hash = type_version_definition_hash(
        &raw.fields.target_type,
        raw.fields.target_version,
        &version_fields,
        raw.fields.optional_extension_prefix.as_deref(),
    )
    .map_err(SchemaError::Definition)?;
    Ok(SchemaDescriptor {
        record_id: raw.record_id,
        target_type: raw.fields.target_type,
        target_version: raw.fields.target_version,
        fields,
        optional_extension_prefix: raw.fields.optional_extension_prefix,
        definition_hash,
    })
}

struct RawSchema {
    record_id: String,
    record_type: String,
    schema: u64,
    fields: RawSchemaFields,
}

struct RawSchemaFields {
    target_type: String,
    target_version: u64,
    fields: Vec<SchemaField>,
    optional_extension_prefix: Option<String>,
}

struct SchemaSeed;

impl<'de> DeserializeSeed<'de> for SchemaSeed {
    type Value = RawSchema;

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(SchemaVisitor)
    }
}

struct SchemaVisitor;

impl<'de> Visitor<'de> for SchemaVisitor {
    type Value = RawSchema;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a core.schema schema 1 record")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut id = None;
        let mut record_type = None;
        let mut schema = None;
        let mut fields = None;
        let mut documentation_seen = false;
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "id" if id.is_none() => id = Some(map.next_value()?),
                "type" if record_type.is_none() => record_type = Some(map.next_value()?),
                "schema" if schema.is_none() => schema = Some(map.next_value()?),
                "fields" if fields.is_none() => {
                    fields = Some(map.next_value_seed(SchemaFieldsSeed)?)
                }
                "documentation" if !documentation_seen => {
                    let _: String = map.next_value()?;
                    documentation_seen = true;
                }
                _ => {
                    return Err(A::Error::custom(
                        "duplicate or unsupported schema record key",
                    ));
                }
            }
        }
        Ok(RawSchema {
            record_id: id.ok_or_else(|| A::Error::custom("missing schema record ID"))?,
            record_type: record_type
                .ok_or_else(|| A::Error::custom("missing schema record type"))?,
            schema: schema.ok_or_else(|| A::Error::custom("missing schema version"))?,
            fields: fields.ok_or_else(|| A::Error::custom("missing schema fields"))?,
        })
    }
}

struct SchemaFieldsSeed;

impl<'de> DeserializeSeed<'de> for SchemaFieldsSeed {
    type Value = RawSchemaFields;

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(SchemaFieldsVisitor)
    }
}

struct SchemaFieldsVisitor;

impl<'de> Visitor<'de> for SchemaFieldsVisitor {
    type Value = RawSchemaFields;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("fixed core.schema fields")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut target_type = None;
        let mut target_version = None;
        let mut fields = None;
        let mut optional_extension_prefix = None;
        let mut prefix_seen = false;
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "schema.targetType" if target_type.is_none() => {
                    target_type = Some(map.next_value()?);
                }
                "schema.targetVersion" if target_version.is_none() => {
                    target_version = Some(map.next_value()?);
                }
                "schema.fields" if fields.is_none() => {
                    fields = Some(map.next_value_seed(FieldArraySeed)?);
                }
                "schema.optionalExtensionPrefix" if !prefix_seen => {
                    optional_extension_prefix = map.next_value()?;
                    prefix_seen = true;
                }
                _ => return Err(A::Error::custom("duplicate or unsupported schema field")),
            }
        }
        Ok(RawSchemaFields {
            target_type: target_type.ok_or_else(|| A::Error::custom("missing target type"))?,
            target_version: target_version
                .ok_or_else(|| A::Error::custom("missing target version"))?,
            fields: fields.ok_or_else(|| A::Error::custom("missing field descriptors"))?,
            optional_extension_prefix,
        })
    }
}

struct FieldArraySeed;

impl<'de> DeserializeSeed<'de> for FieldArraySeed {
    type Value = Vec<SchemaField>;

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_seq(FieldArrayVisitor)
    }
}

struct FieldArrayVisitor;

impl<'de> Visitor<'de> for FieldArrayVisitor {
    type Value = Vec<SchemaField>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("at most 128 strictly sorted field descriptors")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
        let mut fields: Vec<SchemaField> = Vec::new();
        loop {
            if fields.len() >= MAX_FIELDS {
                sequence.next_element_seed(RejectExtraFieldSeed)?;
                break;
            }
            let Some(field) = sequence.next_element_seed(FieldSeed)? else {
                break;
            };
            if fields
                .last()
                .is_some_and(|previous| previous.id >= field.id)
            {
                return Err(A::Error::custom("schema.fields not strictly sorted"));
            }
            fields.push(field);
        }
        Ok(fields)
    }
}

struct RejectExtraFieldSeed;

impl<'de> DeserializeSeed<'de> for RejectExtraFieldSeed {
    type Value = ();

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        _deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        Err(D::Error::custom("schema.fields exceeds 128 descriptors"))
    }
}

struct FieldSeed;

impl<'de> DeserializeSeed<'de> for FieldSeed {
    type Value = SchemaField;

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(FieldVisitor)
    }
}

struct FieldVisitor;

impl<'de> Visitor<'de> for FieldVisitor {
    type Value = SchemaField;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a text or required record-ref field descriptor")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut id = None;
        let mut kind = None;
        let mut presence = None;
        let mut max_utf8_bytes = None;
        let mut target_type = None;
        let mut resolution: Option<String> = None;
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "id" if id.is_none() => id = Some(map.next_value()?),
                "kind" if kind.is_none() => kind = Some(map.next_value::<String>()?),
                "presence" if presence.is_none() => {
                    presence = Some(map.next_value::<String>()?);
                }
                "maxUtf8Bytes" if max_utf8_bytes.is_none() => {
                    max_utf8_bytes = Some(map.next_value()?);
                }
                "targetType" if target_type.is_none() => target_type = Some(map.next_value()?),
                "resolution" if resolution.is_none() => resolution = Some(map.next_value()?),
                _ => return Err(A::Error::custom("duplicate or unsupported field key")),
            }
        }
        let presence = match presence.as_deref() {
            Some("required") => Presence::Required,
            Some("optional") => Presence::Optional,
            _ => return Err(A::Error::custom("invalid field presence")),
        };
        let kind = match kind.as_deref() {
            Some("text") if target_type.is_none() && resolution.is_none() => {
                SchemaFieldKind::Text {
                    max_utf8_bytes: max_utf8_bytes
                        .ok_or_else(|| A::Error::custom("missing maxUtf8Bytes"))?,
                }
            }
            Some("record-ref")
                if max_utf8_bytes.is_none() && resolution.as_deref() == Some("required") =>
            {
                SchemaFieldKind::RecordRef {
                    target_type: target_type
                        .ok_or_else(|| A::Error::custom("missing targetType"))?,
                }
            }
            _ => return Err(A::Error::custom("invalid field kind attributes")),
        };
        Ok(SchemaField {
            id: id.ok_or_else(|| A::Error::custom("missing field ID"))?,
            presence,
            kind,
            signature: [0; 32],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCHEMA_ID: &str = "2a5ef644-7225-4770-9f4d-e1d9c0a7d515";

    fn marker_schema() -> String {
        format!(
            "{{\"id\":\"{SCHEMA_ID}\",\"type\":\"core.schema\",\"schema\":1,\"fields\":{{\"schema.targetType\":\"game.marker\",\"schema.targetVersion\":1,\"schema.fields\":[{{\"id\":\"marker.caption\",\"kind\":\"text\",\"presence\":\"optional\",\"maxUtf8Bytes\":1024}},{{\"id\":\"marker.scene\",\"kind\":\"record-ref\",\"presence\":\"required\",\"targetType\":\"core.scene\",\"resolution\":\"required\"}}],\"schema.optionalExtensionPrefix\":\"x.game.marker.\"}}}}"
        )
    }

    #[test]
    fn direct_schema_parse_derives_canonical_hashes() {
        let bytes = marker_schema();
        let descriptor = parse_schema_descriptor(bytes.as_bytes()).unwrap();
        assert_eq!(descriptor.record_id(), SCHEMA_ID);
        assert_eq!(descriptor.target_type(), "game.marker");
        assert_eq!(descriptor.target_version(), 1);
        assert_eq!(
            descriptor.optional_extension_prefix(),
            Some("x.game.marker.")
        );
        assert_eq!(descriptor.fields().len(), 2);
        assert_ne!(descriptor.fields()[0].signature(), &[0; 32]);
        let changed = bytes.replace("1024", "2048");
        let changed = parse_schema_descriptor(changed.as_bytes()).unwrap();
        assert_ne!(descriptor.definition_hash(), changed.definition_hash());
    }

    #[test]
    fn duplicate_unknown_wrong_kind_and_version_refuse() {
        let valid = marker_schema();
        for changed in [
            valid.replace("\"schema\":1", "\"schema\":1,\"schema\":1"),
            valid.replace(
                "\"marker.caption\"",
                "\"marker.caption\",\"id\":\"marker.caption\"",
            ),
            valid.replace(
                "\"maxUtf8Bytes\":1024",
                "\"maxUtf8Bytes\":1024,\"unknown\":[[[0]]]",
            ),
            valid.replace("\"resolution\":\"required\"", "\"resolution\":\"soft\""),
            valid.replace(
                "\"schema.targetVersion\":1",
                "\"schema.targetVersion\":4294967296",
            ),
            valid.replace("\"marker.scene\"", "\"marker.caption\""),
            valid.replace("\"marker.scene\"", "\"marker.a\""),
            format!("{valid} trailing"),
        ] {
            assert!(
                parse_schema_descriptor(changed.as_bytes()).is_err(),
                "{changed}"
            );
        }
    }

    #[test]
    fn field_count_refuses_before_parsing_extra_value() {
        let fields = (0..128)
            .map(|index| format!("{{\"id\":\"marker.f{index:03}\",\"kind\":\"text\",\"presence\":\"optional\",\"maxUtf8Bytes\":1}}"))
            .collect::<Vec<_>>()
            .join(",");
        let bytes = marker_schema().replace(
            "{\"id\":\"marker.caption\",\"kind\":\"text\",\"presence\":\"optional\",\"maxUtf8Bytes\":1024},{\"id\":\"marker.scene\",\"kind\":\"record-ref\",\"presence\":\"required\",\"targetType\":\"core.scene\",\"resolution\":\"required\"}",
            &format!("{fields},[[[[[[0]]]]]]"),
        );
        let error = parse_schema_descriptor(bytes.as_bytes()).err().unwrap();
        assert!(format!("{error:?}").contains("exceeds 128"));
    }

    #[test]
    #[ignore = "local maximum first-profile descriptor parse cost probe"]
    fn measure_256_maximum_schema_records() {
        let fields = (0..128)
            .map(|index| {
                format!(
                    "{{\"id\":\"field.f{index:03}\",\"kind\":\"text\",\"presence\":\"optional\",\"maxUtf8Bytes\":4096}}"
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let bytes = format!(
            "{{\"id\":\"{SCHEMA_ID}\",\"type\":\"core.schema\",\"schema\":1,\"fields\":{{\"schema.targetType\":\"game.large\",\"schema.targetVersion\":1,\"schema.fields\":[{fields}]}}}}"
        );
        let start = std::time::Instant::now();
        let mut checksum = 0_u8;
        for _ in 0..256 {
            let descriptor = parse_schema_descriptor(bytes.as_bytes()).unwrap();
            checksum ^= descriptor.definition_hash()[0];
            std::hint::black_box(descriptor);
        }
        std::hint::black_box(checksum);
        eprintln!(
            "schema_records=256 fields_per_record=128 elapsed={:?}",
            start.elapsed()
        );
    }
}
