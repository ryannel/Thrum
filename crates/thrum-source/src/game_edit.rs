//! One bounded source edit that carries untouched JSON values verbatim.
//!
//! This is an offline proposal over supplied bytes. It does not write working
//! files, stage a candidate, admit a root, or change a running world.

use std::collections::BTreeMap;

use serde::Serialize;
use serde::Serializer as _;
use serde::ser::SerializeMap;
use serde_json::Serializer;
use serde_json::value::RawValue;

use crate::MAX_RECORD_BYTES;
use crate::game_json::{GameRecordError, parse_game_record};
use crate::schema_json::{SchemaDescriptor, SchemaFieldKind};

/// A one-field source edit could not preserve a valid bounded game record.
#[derive(Debug)]
pub enum GameTextEditError {
    /// The input is not a record under the supplied descriptor.
    InvalidOriginal(GameRecordError),
    /// The selected field is absent from the descriptor.
    UnknownField,
    /// The selected descriptor field is not text.
    NotText,
    /// The replacement exceeds this field's UTF-8 byte bound.
    TextTooLong,
    /// Raw-value borrowing or JSON output failed.
    Json(serde_json::Error),
    /// The resulting exact record exceeds the first-profile byte bound.
    OutputTooLarge,
    /// The output failed the same descriptor after serialization.
    InvalidEdited(GameRecordError),
    /// An edit unexpectedly changed record identity or required references.
    ChangedIdentity,
}

/// Replace one known text value while preserving all other JSON value bytes.
///
/// Both records are checked against the same source-carried descriptor. The
/// output may normalize object key order and whitespace between members, but
/// untouched optional extension values keep their exact JSON spelling. The
/// caller must still apply, capture and admit the resulting file through the
/// source transaction; these returned bytes have no accepted-source authority.
pub fn replace_game_text_field(
    bytes: &[u8],
    descriptor: &SchemaDescriptor,
    field_id: &str,
    replacement: &str,
) -> Result<Vec<u8>, GameTextEditError> {
    let before =
        parse_game_record(bytes, descriptor).map_err(GameTextEditError::InvalidOriginal)?;
    let field = descriptor
        .fields()
        .binary_search_by(|field| field.id().cmp(field_id))
        .ok()
        .map(|index| &descriptor.fields()[index])
        .ok_or(GameTextEditError::UnknownField)?;
    let SchemaFieldKind::Text { max_utf8_bytes } = field.kind() else {
        return Err(GameTextEditError::NotText);
    };
    if replacement.len() > usize::from(*max_utf8_bytes) {
        return Err(GameTextEditError::TextTooLong);
    }

    let top: BTreeMap<String, &RawValue> =
        serde_json::from_slice(bytes).map_err(GameTextEditError::Json)?;
    let fields_raw = top.get("fields").expect("validated game record has fields");
    let fields: BTreeMap<String, &RawValue> =
        serde_json::from_str(fields_raw.get()).map_err(GameTextEditError::Json)?;
    let replacement_raw =
        RawValue::from_string(serde_json::to_string(replacement).map_err(GameTextEditError::Json)?)
            .map_err(GameTextEditError::Json)?;

    let mut edited = Vec::with_capacity(bytes.len() + replacement.len());
    {
        let mut serializer = Serializer::new(&mut edited);
        let mut map = serializer
            .serialize_map(Some(top.len()))
            .map_err(GameTextEditError::Json)?;
        for (key, raw) in &top {
            if key == "fields" {
                map.serialize_entry(
                    key,
                    &EditedFields {
                        fields: &fields,
                        field_id,
                        replacement: &replacement_raw,
                    },
                )
                .map_err(GameTextEditError::Json)?;
            } else {
                map.serialize_entry(key, raw)
                    .map_err(GameTextEditError::Json)?;
            }
        }
        map.end().map_err(GameTextEditError::Json)?;
    }
    if edited.len() > MAX_RECORD_BYTES {
        return Err(GameTextEditError::OutputTooLarge);
    }
    let after = parse_game_record(&edited, descriptor).map_err(GameTextEditError::InvalidEdited)?;
    let mut reference_targets = [None; 128];
    for reference in before.references() {
        reference_targets[usize::from(reference.field_index())] = Some(reference.target_id());
    }
    if before.id() != after.id()
        || before.references().len() != after.references().len()
        || after.references().iter().any(|reference| {
            reference_targets[usize::from(reference.field_index())] != Some(reference.target_id())
        })
    {
        return Err(GameTextEditError::ChangedIdentity);
    }
    Ok(edited)
}

struct EditedFields<'a> {
    fields: &'a BTreeMap<String, &'a RawValue>,
    field_id: &'a str,
    replacement: &'a RawValue,
}

impl Serialize for EditedFields<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let extra = usize::from(!self.fields.contains_key(self.field_id));
        let mut map = serializer.serialize_map(Some(self.fields.len() + extra))?;
        let mut inserted = false;
        for (key, raw) in self.fields {
            if !inserted && key.as_str() >= self.field_id {
                map.serialize_entry(self.field_id, self.replacement)?;
                inserted = true;
            }
            if key != self.field_id {
                map.serialize_entry(key, raw)?;
            }
        }
        if !inserted {
            map.serialize_entry(self.field_id, self.replacement)?;
        }
        map.end()
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

    #[test]
    fn known_text_edit_preserves_opaque_value_spelling_and_reference() {
        let source = format!(
            "{{\"id\":\"{RECORD_ID}\",\"type\":\"game.marker\",\"schema\":1,\"fields\":{{\"marker.caption\":\"before\",\"marker.scene\":\"{SCENE_ID}\",\"x.game.marker.note\":{{\"a\" : [1,  2, {{\"s\":\"\\u0061\"}}]}}}}}}"
        );
        let opaque = r#"{"a" : [1,  2, {"s":"\u0061"}]}"#;
        let edited =
            replace_game_text_field(source.as_bytes(), &schema(), "marker.caption", "after")
                .unwrap();
        let output = std::str::from_utf8(&edited).unwrap();
        assert!(output.contains(opaque));
        assert!(output.contains("\"marker.caption\":\"after\""));
        assert_eq!(
            parse_game_record(&edited, &schema()).unwrap().references()[0].target_id(),
            SCENE_ID
        );
        assert_eq!(
            parse_game_record(&edited, &schema()).unwrap().id(),
            RECORD_ID
        );
    }

    #[test]
    fn optional_text_can_be_added_and_bad_edits_refuse() {
        let source = format!(
            "{{\"id\":\"{RECORD_ID}\",\"type\":\"game.marker\",\"schema\":1,\"fields\":{{\"marker.scene\":\"{SCENE_ID}\"}}}}"
        );
        let descriptor = schema();
        let edited =
            replace_game_text_field(source.as_bytes(), &descriptor, "marker.caption", "added")
                .unwrap();
        assert!(
            std::str::from_utf8(&edited)
                .unwrap()
                .contains("\"marker.caption\":\"added\"")
        );
        assert!(matches!(
            replace_game_text_field(source.as_bytes(), &descriptor, "marker.scene", "x"),
            Err(GameTextEditError::NotText)
        ));
        assert!(matches!(
            replace_game_text_field(source.as_bytes(), &descriptor, "missing", "x"),
            Err(GameTextEditError::UnknownField)
        ));
        assert!(matches!(
            replace_game_text_field(
                source.as_bytes(),
                &descriptor,
                "marker.caption",
                &"é".repeat(33)
            ),
            Err(GameTextEditError::TextTooLong)
        ));
        let malformed = source.replace(
            &format!("\"marker.scene\":\"{SCENE_ID}\""),
            &format!("\"marker.scene\":\"{SCENE_ID}\",\"marker.scene\":\"{SCENE_ID}\""),
        );
        assert!(matches!(
            replace_game_text_field(malformed.as_bytes(), &descriptor, "marker.caption", "after"),
            Err(GameTextEditError::InvalidOriginal(_))
        ));
    }

    #[test]
    fn reference_order_does_not_change_identity() {
        const OTHER_SCENE_ID: &str = "5a5ef644-7225-4770-9f4d-e1d9c0a7d515";
        let bytes = serde_json::to_vec(&json!({
            "id":SCHEMA_ID, "type":"core.schema", "schema":1,
            "fields":{
                "schema.targetType":"game.marker", "schema.targetVersion":1,
                "schema.optionalExtensionPrefix":"x.game.marker.",
                "schema.fields":[
                    {"id":"marker.alt-scene", "kind":"record-ref", "presence":"required", "targetType":"core.scene", "resolution":"required"},
                    {"id":"marker.caption", "kind":"text", "presence":"optional", "maxUtf8Bytes":64},
                    {"id":"marker.scene", "kind":"record-ref", "presence":"required", "targetType":"core.scene", "resolution":"required"}
                ]
            }
        }))
        .unwrap();
        let descriptor = parse_schema_descriptor(&bytes).unwrap();
        let source = format!(
            "{{\"id\":\"{RECORD_ID}\",\"type\":\"game.marker\",\"schema\":1,\"fields\":{{\"marker.scene\":\"{SCENE_ID}\",\"marker.caption\":\"before\",\"marker.alt-scene\":\"{OTHER_SCENE_ID}\"}}}}"
        );
        let edited =
            replace_game_text_field(source.as_bytes(), &descriptor, "marker.caption", "after")
                .unwrap();
        let text = std::str::from_utf8(&edited).unwrap();
        assert!(text.contains(&format!("\"marker.alt-scene\":\"{OTHER_SCENE_ID}\"")));
        assert!(text.contains(&format!("\"marker.scene\":\"{SCENE_ID}\"")));
    }

    #[test]
    #[ignore = "local large optional-value edit cost probe"]
    fn measure_large_opaque_text_edit() {
        let mut source = format!(
            "{{\"id\":\"{RECORD_ID}\",\"type\":\"game.marker\",\"schema\":1,\"fields\":{{\"marker.caption\":\"before\",\"marker.scene\":\"{SCENE_ID}\""
        );
        let large_value = "v".repeat(32 * 1024);
        for index in 0..128 {
            source.push_str(&format!(
                ",\"x.game.marker.note{index:03}\":\"{large_value}\""
            ));
        }
        source.push_str("}}");
        let descriptor = schema();
        let start = std::time::Instant::now();
        let edited =
            replace_game_text_field(source.as_bytes(), &descriptor, "marker.caption", "after")
                .unwrap();
        eprintln!(
            "input_bytes={} output_bytes={} elapsed={:?}",
            source.len(),
            edited.len(),
            start.elapsed()
        );
    }
}
