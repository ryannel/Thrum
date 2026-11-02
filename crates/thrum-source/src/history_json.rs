//! Direct bounded decoding of one first-profile source-carried registry history.
//!
//! This validates supplied bytes and local order. A later staged-tree adapter
//! must compare active descriptors and authenticate the prior admitted root.

use std::borrow::Cow;
use std::fmt;

use serde::de::{DeserializeSeed, Error as _, MapAccess, SeqAccess, Visitor};

use crate::MAX_RECORD_BYTES;
use crate::definition::valid_symbol;
use crate::meaning::{
    FieldHistoryView, MeaningHistoryError, TypeVersionHistoryError, TypeVersionHistoryView,
    compare_field_history_views, compare_type_version_views,
};
use crate::record::canonical_uuid;

const MAX_FIELD_HISTORY: usize = 100_000;
const MAX_VERSION_HISTORY: usize = 1_024;

/// A fixed registry-history source record failed its local grammar.
#[derive(Debug)]
pub enum HistoryError {
    /// Exact JSON bytes exceed the 16 MiB record limit.
    TooLarge,
    /// Syntax, fixed shape, key order or count refused.
    Invalid(serde_json::Error),
}

/// One permanent field meaning carried in the registry-history record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryField<'a> {
    type_id: Cow<'a, str>,
    field_id: Cow<'a, str>,
    signature: [u8; 32],
    retired: bool,
}

impl HistoryField<'_> {
    /// Stable game-owned type ID.
    pub fn type_id(&self) -> &str {
        &self.type_id
    }

    /// Permanent field ID within the type.
    pub fn field_id(&self) -> &str {
        &self.field_id
    }

    /// Canonical mechanical-meaning signature supplied by source.
    pub fn signature(&self) -> &[u8; 32] {
        &self.signature
    }

    /// Whether the ID is permanently retired.
    pub fn retired(&self) -> bool {
        self.retired
    }
}

impl FieldHistoryView for HistoryField<'_> {
    fn history_key(&self) -> (&str, &str) {
        (self.type_id(), self.field_id())
    }

    fn history_signature(&self) -> &[u8; 32] {
        self.signature()
    }

    fn is_retired(&self) -> bool {
        self.retired()
    }
}

/// One immutable type-version definition carried in registry history.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryVersion<'a> {
    type_id: Cow<'a, str>,
    version: u64,
    definition_hash: [u8; 32],
}

impl HistoryVersion<'_> {
    /// Stable game-owned type ID.
    pub fn type_id(&self) -> &str {
        &self.type_id
    }

    /// Target schema version from 1 through `u32::MAX`.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Immutable definition hash supplied by source.
    pub fn definition_hash(&self) -> &[u8; 32] {
        &self.definition_hash
    }
}

impl TypeVersionHistoryView for HistoryVersion<'_> {
    fn history_key(&self) -> (&str, u64) {
        (self.type_id(), self.version())
    }

    fn history_definition_hash(&self) -> &[u8; 32] {
        self.definition_hash()
    }
}

/// Locally well-formed, sorted history from one supplied source record.
pub struct RegistryHistory<'a> {
    record_id: Cow<'a, str>,
    fields: Vec<HistoryField<'a>>,
    versions: Vec<HistoryVersion<'a>>,
}

impl RegistryHistory<'_> {
    /// Stable ID of the `core.registry-history` record.
    pub fn record_id(&self) -> &str {
        &self.record_id
    }

    /// Permanent fields sorted by `(type ID, field ID)`.
    pub fn fields(&self) -> &[HistoryField<'_>] {
        &self.fields
    }

    /// Immutable definitions sorted by `(type ID, target version)`.
    pub fn versions(&self) -> &[HistoryVersion<'_>] {
        &self.versions
    }
}

/// Parsed candidate history cannot monotonically extend a supplied predecessor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CarriedHistoryError {
    /// A permanent field ID was removed, changed or revived.
    Field(MeaningHistoryError),
    /// A permanent type-version definition was removed or changed.
    Version(TypeVersionHistoryError),
}

/// Compare two parsed source-carried histories without a second entry vector.
///
/// This uses the same monotonic rules as the pure slice comparators. Both JSON
/// input buffers remain live through the comparison. The caller must obtain
/// `prior` from the current admitted root under its generation precondition;
/// this function cannot authenticate either source.
pub fn compare_carried_histories(
    prior: &RegistryHistory<'_>,
    candidate: &RegistryHistory<'_>,
) -> Result<(), CarriedHistoryError> {
    compare_field_history_views(prior.fields(), candidate.fields())
        .map_err(CarriedHistoryError::Field)?;
    compare_type_version_views(prior.versions(), candidate.versions())
        .map_err(CarriedHistoryError::Version)?;
    Ok(())
}

/// Parse one `core.registry-history` schema 1 record without a JSON value tree.
///
/// The returned string values borrow unescaped input where serde_json permits;
/// escaped spelling may allocate. Keep `bytes` alive while using the result.
pub fn parse_registry_history(bytes: &[u8]) -> Result<RegistryHistory<'_>, HistoryError> {
    if bytes.len() > MAX_RECORD_BYTES {
        return Err(HistoryError::TooLarge);
    }
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let raw = HistorySeed
        .deserialize(&mut deserializer)
        .map_err(HistoryError::Invalid)?;
    deserializer.end().map_err(HistoryError::Invalid)?;
    Ok(raw)
}

struct HistorySeed;

impl<'de> DeserializeSeed<'de> for HistorySeed {
    type Value = RegistryHistory<'de>;

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(HistoryVisitor)
    }
}

struct HistoryVisitor;

impl<'de> Visitor<'de> for HistoryVisitor {
    type Value = RegistryHistory<'de>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a core.registry-history schema 1 record")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut id = None;
        let mut record_type = None;
        let mut schema = None;
        let mut fields = None;
        let mut documentation_seen = false;
        while let Some(key) = map.next_key_seed(CowStringSeed)? {
            match key.as_ref() {
                "id" if id.is_none() => id = Some(map.next_value_seed(CowStringSeed)?),
                "type" if record_type.is_none() => {
                    record_type = Some(map.next_value_seed(CowStringSeed)?);
                }
                "schema" if schema.is_none() => schema = Some(map.next_value::<u64>()?),
                "fields" if fields.is_none() => {
                    fields = Some(map.next_value_seed(HistoryFieldsSeed)?)
                }
                "documentation" if !documentation_seen => {
                    let _: String = map.next_value()?;
                    documentation_seen = true;
                }
                _ => {
                    return Err(A::Error::custom(
                        "duplicate or unsupported history record key",
                    ));
                }
            }
        }
        let record_id: Cow<'de, str> = id.ok_or_else(|| A::Error::custom("missing history ID"))?;
        if !canonical_uuid(&record_id)
            || record_type.as_deref() != Some("core.registry-history")
            || schema != Some(1)
        {
            return Err(A::Error::custom("invalid registry history envelope"));
        }
        let (fields, versions) =
            fields.ok_or_else(|| A::Error::custom("missing history fields"))?;
        Ok(RegistryHistory {
            record_id,
            fields,
            versions,
        })
    }
}

struct HistoryFieldsSeed;

impl<'de> DeserializeSeed<'de> for HistoryFieldsSeed {
    type Value = (Vec<HistoryField<'de>>, Vec<HistoryVersion<'de>>);

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(HistoryFieldsVisitor)
    }
}

struct HistoryFieldsVisitor;

impl<'de> Visitor<'de> for HistoryFieldsVisitor {
    type Value = (Vec<HistoryField<'de>>, Vec<HistoryVersion<'de>>);

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("fixed registry.fields and registry.versions arrays")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut fields = None;
        let mut versions = None;
        while let Some(key) = map.next_key_seed(CowStringSeed)? {
            match key.as_ref() {
                "registry.fields" if fields.is_none() => {
                    fields = Some(map.next_value_seed(FieldHistoryArraySeed)?);
                }
                "registry.versions" if versions.is_none() => {
                    versions = Some(map.next_value_seed(VersionHistoryArraySeed)?);
                }
                _ => return Err(A::Error::custom("duplicate or unsupported history field")),
            }
        }
        Ok((
            fields.ok_or_else(|| A::Error::custom("missing registry.fields"))?,
            versions.ok_or_else(|| A::Error::custom("missing registry.versions"))?,
        ))
    }
}

struct FieldHistoryArraySeed;

impl<'de> DeserializeSeed<'de> for FieldHistoryArraySeed {
    type Value = Vec<HistoryField<'de>>;

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_seq(FieldHistoryArrayVisitor)
    }
}

struct FieldHistoryArrayVisitor;

impl<'de> Visitor<'de> for FieldHistoryArrayVisitor {
    type Value = Vec<HistoryField<'de>>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("at most 100,000 sorted field-history entries")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
        let mut entries: Vec<HistoryField<'de>> = Vec::new();
        loop {
            if entries.len() >= MAX_FIELD_HISTORY {
                sequence.next_element_seed(RejectExtraSeed)?;
                break;
            }
            let Some(entry) = sequence.next_element_seed(FieldHistoryEntrySeed)? else {
                break;
            };
            if entries.last().is_some_and(|previous| {
                (previous.type_id.as_ref(), previous.field_id.as_ref())
                    >= (entry.type_id.as_ref(), entry.field_id.as_ref())
            }) {
                return Err(A::Error::custom("registry.fields not strictly sorted"));
            }
            entries.push(entry);
        }
        Ok(entries)
    }
}

struct VersionHistoryArraySeed;

impl<'de> DeserializeSeed<'de> for VersionHistoryArraySeed {
    type Value = Vec<HistoryVersion<'de>>;

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_seq(VersionHistoryArrayVisitor)
    }
}

struct VersionHistoryArrayVisitor;

impl<'de> Visitor<'de> for VersionHistoryArrayVisitor {
    type Value = Vec<HistoryVersion<'de>>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("at most 1,024 sorted type-version entries")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
        let mut entries: Vec<HistoryVersion<'de>> = Vec::new();
        loop {
            if entries.len() >= MAX_VERSION_HISTORY {
                sequence.next_element_seed(RejectExtraSeed)?;
                break;
            }
            let Some(entry) = sequence.next_element_seed(VersionHistoryEntrySeed)? else {
                break;
            };
            if entries.last().is_some_and(|previous| {
                (previous.type_id.as_ref(), previous.version)
                    >= (entry.type_id.as_ref(), entry.version)
            }) {
                return Err(A::Error::custom("registry.versions not strictly sorted"));
            }
            entries.push(entry);
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
        Err(D::Error::custom("registry history count limit"))
    }
}

struct FieldHistoryEntrySeed;

impl<'de> DeserializeSeed<'de> for FieldHistoryEntrySeed {
    type Value = HistoryField<'de>;

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(FieldHistoryEntryVisitor)
    }
}

struct FieldHistoryEntryVisitor;

impl<'de> Visitor<'de> for FieldHistoryEntryVisitor {
    type Value = HistoryField<'de>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a permanent field meaning entry")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut type_id = None;
        let mut field_id = None;
        let mut signature = None;
        let mut retired = None;
        while let Some(key) = map.next_key_seed(CowStringSeed)? {
            match key.as_ref() {
                "typeId" if type_id.is_none() => {
                    type_id = Some(map.next_value_seed(CowStringSeed)?);
                }
                "fieldId" if field_id.is_none() => {
                    field_id = Some(map.next_value_seed(CowStringSeed)?);
                }
                "signature" if signature.is_none() => {
                    let value = map.next_value_seed(CowStringSeed)?;
                    signature = Some(
                        parse_hash(&value)
                            .ok_or_else(|| A::Error::custom("invalid field signature"))?,
                    );
                }
                "retired" if retired.is_none() => retired = Some(map.next_value::<bool>()?),
                _ => {
                    return Err(A::Error::custom(
                        "duplicate or unsupported field-history key",
                    ));
                }
            }
        }
        let type_id = type_id.ok_or_else(|| A::Error::custom("missing field type ID"))?;
        let field_id = field_id.ok_or_else(|| A::Error::custom("missing history field ID"))?;
        if !valid_game_type(&type_id) || !valid_symbol(&field_id) {
            return Err(A::Error::custom("invalid history field identity"));
        }
        Ok(HistoryField {
            type_id,
            field_id,
            signature: signature.ok_or_else(|| A::Error::custom("missing field signature"))?,
            retired: retired.ok_or_else(|| A::Error::custom("missing retired flag"))?,
        })
    }
}

struct VersionHistoryEntrySeed;

impl<'de> DeserializeSeed<'de> for VersionHistoryEntrySeed {
    type Value = HistoryVersion<'de>;

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(VersionHistoryEntryVisitor)
    }
}

struct VersionHistoryEntryVisitor;

impl<'de> Visitor<'de> for VersionHistoryEntryVisitor {
    type Value = HistoryVersion<'de>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an immutable type-version history entry")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut type_id = None;
        let mut version = None;
        let mut definition_hash = None;
        while let Some(key) = map.next_key_seed(CowStringSeed)? {
            match key.as_ref() {
                "typeId" if type_id.is_none() => {
                    type_id = Some(map.next_value_seed(CowStringSeed)?);
                }
                "targetVersion" if version.is_none() => version = Some(map.next_value::<u64>()?),
                "definitionHash" if definition_hash.is_none() => {
                    let value = map.next_value_seed(CowStringSeed)?;
                    definition_hash = Some(
                        parse_hash(&value)
                            .ok_or_else(|| A::Error::custom("invalid definition hash"))?,
                    );
                }
                _ => {
                    return Err(A::Error::custom(
                        "duplicate or unsupported version-history key",
                    ));
                }
            }
        }
        let type_id = type_id.ok_or_else(|| A::Error::custom("missing version type ID"))?;
        let version = version.ok_or_else(|| A::Error::custom("missing target version"))?;
        if !valid_game_type(&type_id) || version == 0 || version > u64::from(u32::MAX) {
            return Err(A::Error::custom("invalid type version identity"));
        }
        Ok(HistoryVersion {
            type_id,
            version,
            definition_hash: definition_hash
                .ok_or_else(|| A::Error::custom("missing definition hash"))?,
        })
    }
}

fn valid_game_type(value: &str) -> bool {
    value.starts_with("game.") && valid_symbol(value)
}

struct CowStringSeed;

impl<'de> DeserializeSeed<'de> for CowStringSeed {
    type Value = Cow<'de, str>;

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_str(CowStringVisitor)
    }
}

struct CowStringVisitor;

impl<'de> Visitor<'de> for CowStringVisitor {
    type Value = Cow<'de, str>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a UTF-8 string")
    }

    fn visit_borrowed_str<E: serde::de::Error>(self, value: &'de str) -> Result<Self::Value, E> {
        Ok(Cow::Borrowed(value))
    }

    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
        Ok(Cow::Owned(value.to_owned()))
    }

    fn visit_string<E: serde::de::Error>(self, value: String) -> Result<Self::Value, E> {
        Ok(Cow::Owned(value))
    }
}

fn parse_hash(value: &str) -> Option<[u8; 32]> {
    let bytes = value.as_bytes();
    if bytes.len() != 64 {
        return None;
    }
    let mut decoded = [0_u8; 32];
    let (pairs, remainder) = bytes.as_chunks::<2>();
    debug_assert!(remainder.is_empty());
    for (index, pair) in pairs.iter().enumerate() {
        decoded[index] = (hex_nibble(pair[0])? << 4) | hex_nibble(pair[1])?;
    }
    Some(decoded)
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const HISTORY_ID: &str = "2a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn small_history() -> String {
        format!(
            "{{\"id\":\"{HISTORY_ID}\",\"type\":\"core.registry-history\",\"schema\":1,\"fields\":{{\"registry.fields\":[{{\"typeId\":\"game.marker\",\"fieldId\":\"marker.caption\",\"signature\":\"{HASH}\",\"retired\":false}}],\"registry.versions\":[{{\"typeId\":\"game.marker\",\"targetVersion\":1,\"definitionHash\":\"{HASH}\"}}]}}}}"
        )
    }

    #[test]
    fn history_decodes_sorted_and_borrows_plain_ids() {
        let bytes = small_history();
        let history = parse_registry_history(bytes.as_bytes()).unwrap();
        assert_eq!(history.record_id(), HISTORY_ID);
        assert_eq!(history.fields().len(), 1);
        assert_eq!(history.versions().len(), 1);
        assert_eq!(history.fields()[0].signature()[0], 0x01);
        assert!(!history.fields()[0].retired());
        assert_eq!(history.versions()[0].version(), 1);
        assert!(matches!(history.fields()[0].type_id, Cow::Borrowed(_)));
        assert!(matches!(history.fields()[0].field_id, Cow::Borrowed(_)));

        let escaped = bytes.replace("game.marker", "game.\\u006darker");
        let escaped_history = parse_registry_history(escaped.as_bytes()).unwrap();
        assert_eq!(escaped_history.fields()[0].type_id(), "game.marker");
        assert!(matches!(escaped_history.fields()[0].type_id, Cow::Owned(_)));

        let escaped_key = bytes.replace("registry.fields", "registry.\\u0066ields");
        assert_eq!(
            parse_registry_history(escaped_key.as_bytes())
                .unwrap()
                .fields()
                .len(),
            1
        );
    }

    #[test]
    fn duplicate_unknown_unsorted_and_bad_hash_refuse() {
        let valid = small_history();
        for changed in [
            valid.replace("\"schema\":1", "\"schema\":1,\"schema\":1"),
            valid.replace("\"retired\":false", "\"retired\":false,\"unknown\":[[[0]]]"),
            valid.replace(HASH, &HASH.to_uppercase()),
            valid.replace("\"targetVersion\":1", "\"targetVersion\":4294967296"),
            valid.replace(
                "\"registry.versions\":[",
                &format!("\"registry.versions\":[{{\"typeId\":\"game.marker\",\"targetVersion\":1,\"definitionHash\":\"{HASH}\"}},"),
            ),
            valid.replace("\"registry.fields\":[", "\"registry.fields\":[{\"typeId\":\"game.marker\",\"fieldId\":\"marker.caption\",\"signature\":\"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\",\"retired\":false},"),
            format!("{valid} trailing"),
        ] {
            assert!(parse_registry_history(changed.as_bytes()).is_err(), "{changed}");
        }
    }

    #[test]
    fn version_count_refuses_before_parsing_extra_value() {
        let versions = (1..=1_024)
            .map(|version| {
                format!(
                    "{{\"typeId\":\"game.marker\",\"targetVersion\":{version},\"definitionHash\":\"{HASH}\"}}"
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let bytes = format!(
            "{{\"id\":\"{HISTORY_ID}\",\"type\":\"core.registry-history\",\"schema\":1,\"fields\":{{\"registry.fields\":[],\"registry.versions\":[{versions},[[[0]]] ]}}}}"
        );
        let error = parse_registry_history(bytes.as_bytes()).err().unwrap();
        assert!(format!("{error:?}").contains("count limit"));
    }

    #[test]
    fn parsed_candidate_preserves_both_prior_histories() {
        let prior_bytes = small_history();
        let prior = parse_registry_history(prior_bytes.as_bytes()).unwrap();
        let mut value: serde_json::Value = serde_json::from_slice(prior_bytes.as_bytes()).unwrap();
        value["fields"]["registry.fields"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "typeId":"game.marker", "fieldId":"marker.tint", "signature":HASH, "retired":false
            }));
        value["fields"]["registry.versions"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "typeId":"game.marker", "targetVersion":2, "definitionHash":HASH
            }));
        let candidate_bytes = serde_json::to_vec(&value).unwrap();
        let candidate = parse_registry_history(&candidate_bytes).unwrap();
        assert_eq!(compare_carried_histories(&prior, &candidate), Ok(()));

        value["fields"]["registry.fields"] = json!([]);
        let bytes = serde_json::to_vec(&value).unwrap();
        let missing = parse_registry_history(&bytes).unwrap();
        assert_eq!(
            compare_carried_histories(&prior, &missing),
            Err(CarriedHistoryError::Field(
                MeaningHistoryError::MissingPrior(0)
            ))
        );

        value = serde_json::from_slice(&candidate_bytes).unwrap();
        value["fields"]["registry.fields"][0]["signature"] = json!("00".repeat(32));
        let bytes = serde_json::to_vec(&value).unwrap();
        let changed = parse_registry_history(&bytes).unwrap();
        assert_eq!(
            compare_carried_histories(&prior, &changed),
            Err(CarriedHistoryError::Field(
                MeaningHistoryError::ChangedMeaning(0)
            ))
        );

        value = serde_json::from_slice(&candidate_bytes).unwrap();
        value["fields"]["registry.versions"][0]["definitionHash"] = json!("00".repeat(32));
        let bytes = serde_json::to_vec(&value).unwrap();
        let changed = parse_registry_history(&bytes).unwrap();
        assert_eq!(
            compare_carried_histories(&prior, &changed),
            Err(CarriedHistoryError::Version(
                TypeVersionHistoryError::ChangedDefinition(0)
            ))
        );

        value["fields"]["registry.versions"] = json!([]);
        let bytes = serde_json::to_vec(&value).unwrap();
        let missing = parse_registry_history(&bytes).unwrap();
        assert_eq!(
            compare_carried_histories(&prior, &missing),
            Err(CarriedHistoryError::Version(
                TypeVersionHistoryError::MissingPrior(0)
            ))
        );

        let mut retired_value: serde_json::Value =
            serde_json::from_slice(prior_bytes.as_bytes()).unwrap();
        retired_value["fields"]["registry.fields"][0]["retired"] = json!(true);
        let retired_bytes = serde_json::to_vec(&retired_value).unwrap();
        let retired_prior = parse_registry_history(&retired_bytes).unwrap();
        assert_eq!(
            compare_carried_histories(&retired_prior, &prior),
            Err(CarriedHistoryError::Field(
                MeaningHistoryError::RevivedRetired(0)
            ))
        );
    }

    #[test]
    #[ignore = "local maximum field-history parse and memory probe"]
    fn measure_maximum_field_history() {
        let mut bytes = format!("{{\"id\":\"{HISTORY_ID}\",\"type\":\"core.registry-history\",\"schema\":1,\"fields\":{{\"registry.fields\":[").into_bytes();
        for index in 0..100_000 {
            if index != 0 {
                bytes.push(b',');
            }
            bytes.extend_from_slice(format!("{{\"typeId\":\"game.large\",\"fieldId\":\"field.f{index:06}\",\"signature\":\"{HASH}\",\"retired\":false}}").as_bytes());
        }
        bytes.extend_from_slice(format!("],\"registry.versions\":[{{\"typeId\":\"game.large\",\"targetVersion\":1,\"definitionHash\":\"{HASH}\"}}]}}}}").as_bytes());
        assert!(
            bytes.len() <= MAX_RECORD_BYTES,
            "fixture exceeds record bound"
        );
        let start = std::time::Instant::now();
        let history = parse_registry_history(&bytes).unwrap();
        assert_eq!(history.fields().len(), 100_000);
        eprintln!(
            "history_bytes={} fields=100000 elapsed={:?}",
            bytes.len(),
            start.elapsed()
        );
        let candidate_bytes = bytes.clone();
        let candidate = parse_registry_history(&candidate_bytes).unwrap();
        let compare_start = std::time::Instant::now();
        assert_eq!(compare_carried_histories(&history, &candidate), Ok(()));
        eprintln!(
            "two_history_fields=200000 compare_elapsed={:?}",
            compare_start.elapsed()
        );
        std::hint::black_box(candidate);
        std::hint::black_box(history);
        drop(candidate_bytes);

        let end = bytes
            .windows(b"],\"registry.versions\"".len())
            .position(|window| window == b"],\"registry.versions\"")
            .unwrap();
        bytes.splice(end..end, b",[[[0]]]".iter().copied());
        let error = parse_registry_history(&bytes).err().unwrap();
        assert!(format!("{error:?}").contains("count limit"));
    }
}
