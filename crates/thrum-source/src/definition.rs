//! Canonical hashes for the deliberately small first game-schema profile.
//!
//! A later descriptor parser must supply checked, source-carried definitions.
//! These pure routines do not parse staged bytes or authenticate history.

use sha2::{Digest, Sha256};

use crate::manifest::{write_decimal, write_hex, write_jcs_string};

const FIELD_DOMAIN: &[u8] = b"thrum-f3-field-meaning-v1\0";
const TYPE_VERSION_DOMAIN: &[u8] = b"thrum-f3-type-version-v1\0";
const MAX_SYMBOL_BYTES: usize = 128;
pub(crate) const MAX_FIELDS: usize = 128;

/// Required or optional presence of a known field in the first schema profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Presence {
    /// This field must appear in the record.
    Required,
    /// This field may be absent from the record.
    Optional,
}

impl Presence {
    fn as_str(self) -> &'static str {
        match self {
            Self::Required => "required",
            Self::Optional => "optional",
        }
    }
}

/// Supported field kinds in the first source-carried schema profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FieldKind<'a> {
    /// UTF-8 text bounded by encoded byte length.
    Text {
        /// Maximum number of UTF-8 bytes, from 1 through 4,096.
        max_utf8_bytes: u16,
    },
    /// A canonical record UUID with a required typed resolution.
    RecordRef {
        /// First profile permits only `core.scene`.
        target_type: &'a str,
    },
}

/// Interpretation-bearing attributes of one game-owned field.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FieldDefinition<'a> {
    /// Stable game-owned record type ID.
    pub type_id: &'a str,
    /// Permanent field ID within the type.
    pub field_id: &'a str,
    /// Whether a record must carry the field.
    pub presence: Presence,
    /// First-profile value kind.
    pub kind: FieldKind<'a>,
}

/// One active field in a type-version definition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VersionField<'a> {
    /// Permanent field ID.
    pub field_id: &'a str,
    /// Canonical signature of the active field definition.
    pub signature: [u8; 32],
}

/// A first-profile definition cannot receive a canonical meaning hash.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DefinitionError {
    /// Type ID is outside the first game-owned grammar.
    TypeId,
    /// Field ID is outside the first permanent-ID grammar.
    FieldId,
    /// Reference target is unsupported in the first profile.
    TargetType,
    /// Text byte bound is outside 1 through 4,096.
    TextLimit,
    /// Target schema version must be within 1 through u32::MAX.
    Version,
    /// A type version has more than 128 active fields.
    TooManyFields,
    /// Fields are not strictly sorted or contain a duplicate at this index.
    FieldOrder(usize),
    /// Opaque extension prefix is outside the first profile grammar.
    ExtensionPrefix,
    /// Active field at this index overlaps the opaque extension prefix.
    ExtensionOverlap(usize),
}

/// Hash one field's interpretation-bearing attributes in fixed JCS form.
pub fn field_meaning_signature(field: FieldDefinition<'_>) -> Result<[u8; 32], DefinitionError> {
    if !field.type_id.starts_with("game.") || !valid_symbol(field.type_id) {
        return Err(DefinitionError::TypeId);
    }
    if !valid_symbol(field.field_id) {
        return Err(DefinitionError::FieldId);
    }

    let mut digest = Sha256::new();
    digest.update(FIELD_DOMAIN);
    digest.update(b"{\"fieldId\":");
    write_jcs_string(&mut digest, field.field_id);
    match field.kind {
        FieldKind::Text { max_utf8_bytes } => {
            if !(1..=4096).contains(&max_utf8_bytes) {
                return Err(DefinitionError::TextLimit);
            }
            digest.update(b",\"kind\":\"text\",\"maxUtf8Bytes\":");
            write_decimal(&mut digest, u64::from(max_utf8_bytes));
        }
        FieldKind::RecordRef { target_type } => {
            if target_type != "core.scene" {
                return Err(DefinitionError::TargetType);
            }
            digest.update(b",\"kind\":\"record-ref\"");
        }
    }
    digest.update(b",\"presence\":");
    write_jcs_string(&mut digest, field.presence.as_str());
    if let FieldKind::RecordRef { target_type } = field.kind {
        digest.update(b",\"resolution\":\"required\",\"targetType\":");
        write_jcs_string(&mut digest, target_type);
    }
    digest.update(b",\"typeId\":");
    write_jcs_string(&mut digest, field.type_id);
    digest.update(b"}");
    Ok(digest.finalize().into())
}

/// Hash one immutable type/version definition, including its optional prefix.
///
/// Field signatures must already have been derived from the active descriptor.
pub fn type_version_definition_hash(
    type_id: &str,
    version: u64,
    fields: &[VersionField<'_>],
    optional_extension_prefix: Option<&str>,
) -> Result<[u8; 32], DefinitionError> {
    if !type_id.starts_with("game.") || !valid_symbol(type_id) {
        return Err(DefinitionError::TypeId);
    }
    if version == 0 || version > u64::from(u32::MAX) {
        return Err(DefinitionError::Version);
    }
    if fields.len() > MAX_FIELDS {
        return Err(DefinitionError::TooManyFields);
    }
    for (index, field) in fields.iter().enumerate() {
        if !valid_symbol(field.field_id) {
            return Err(DefinitionError::FieldId);
        }
        if index > 0 && fields[index - 1].field_id >= field.field_id {
            return Err(DefinitionError::FieldOrder(index));
        }
    }
    if let Some(prefix) = optional_extension_prefix {
        let Some(stem) = prefix.strip_suffix('.') else {
            return Err(DefinitionError::ExtensionPrefix);
        };
        if !stem.starts_with("x.") || !valid_symbol(stem) {
            return Err(DefinitionError::ExtensionPrefix);
        }
        if let Some(index) = fields
            .iter()
            .position(|field| field.field_id.starts_with(prefix))
        {
            return Err(DefinitionError::ExtensionOverlap(index));
        }
    }

    let mut digest = Sha256::new();
    digest.update(TYPE_VERSION_DOMAIN);
    digest.update(b"{\"fields\":[");
    for (index, field) in fields.iter().enumerate() {
        if index != 0 {
            digest.update(b",");
        }
        digest.update(b"{\"fieldId\":");
        write_jcs_string(&mut digest, field.field_id);
        digest.update(b",\"signature\":\"");
        write_hex(&mut digest, &field.signature);
        digest.update(b"\"}");
    }
    digest.update(b"],\"optionalExtensionPrefix\":");
    if let Some(prefix) = optional_extension_prefix {
        write_jcs_string(&mut digest, prefix);
    } else {
        digest.update(b"null");
    }
    digest.update(b",\"targetVersion\":");
    write_decimal(&mut digest, version);
    digest.update(b",\"typeId\":");
    write_jcs_string(&mut digest, type_id);
    digest.update(b"}");
    Ok(digest.finalize().into())
}

fn valid_symbol(symbol: &str) -> bool {
    if symbol.len() > MAX_SYMBOL_BYTES || !symbol.is_ascii() {
        return false;
    }
    let mut segments = symbol.split('.');
    let Some(first) = segments.next() else {
        return false;
    };
    if !valid_segment(first) {
        return false;
    }
    let mut has_second = false;
    for segment in segments {
        if !valid_segment(segment) {
            return false;
        }
        has_second = true;
    }
    has_second
}

fn valid_segment(segment: &str) -> bool {
    let mut bytes = segment.bytes();
    matches!(bytes.next(), Some(b'a'..=b'z'))
        && bytes.all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'-'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn field_hashes_match_an_independent_jcs_oracle() {
        let text = FieldDefinition {
            type_id: "game.marker",
            field_id: "marker.caption",
            presence: Presence::Optional,
            kind: FieldKind::Text {
                max_utf8_bytes: 4096,
            },
        };
        let reference = FieldDefinition {
            type_id: "game.marker",
            field_id: "marker.scene",
            presence: Presence::Required,
            kind: FieldKind::RecordRef {
                target_type: "core.scene",
            },
        };
        let text_json = json!({"typeId":"game.marker", "fieldId":"marker.caption", "kind":"text", "presence":"optional", "maxUtf8Bytes":4096});
        let reference_json = json!({"typeId":"game.marker", "fieldId":"marker.scene", "kind":"record-ref", "presence":"required", "targetType":"core.scene", "resolution":"required"});
        for (field, oracle) in [(text, text_json), (reference, reference_json)] {
            let mut digest = Sha256::new();
            digest.update(FIELD_DOMAIN);
            digest.update(serde_json_canonicalizer::to_vec(&oracle).unwrap());
            let expected: [u8; 32] = digest.finalize().into();
            assert_eq!(field_meaning_signature(field).unwrap(), expected);
        }
        assert_ne!(
            field_meaning_signature(text).unwrap(),
            field_meaning_signature(FieldDefinition {
                kind: FieldKind::Text {
                    max_utf8_bytes: 1024
                },
                ..text
            })
            .unwrap()
        );
    }

    #[test]
    fn type_version_hash_covers_field_set_and_optional_policy() {
        let fields = [
            VersionField {
                field_id: "marker.caption",
                signature: [1; 32],
            },
            VersionField {
                field_id: "marker.scene",
                signature: [2; 32],
            },
        ];
        let mut digest = Sha256::new();
        digest.update(TYPE_VERSION_DOMAIN);
        digest.update(
            serde_json_canonicalizer::to_vec(&json!({
                "typeId":"game.marker", "targetVersion":1,
                "fields":[
                    {"fieldId":"marker.caption", "signature":"01".repeat(32)},
                    {"fieldId":"marker.scene", "signature":"02".repeat(32)}
                ],
                "optionalExtensionPrefix":"x.game.marker."
            }))
            .unwrap(),
        );
        let current =
            type_version_definition_hash("game.marker", 1, &fields, Some("x.game.marker."))
                .unwrap();
        let expected: [u8; 32] = digest.finalize().into();
        assert_eq!(current, expected);
        assert_ne!(
            current,
            type_version_definition_hash("game.marker", 1, &fields, None).unwrap()
        );
        assert_ne!(
            current,
            type_version_definition_hash("game.marker", 1, &fields[..1], Some("x.game.marker."))
                .unwrap()
        );
        assert_ne!(
            current,
            type_version_definition_hash("game.marker", 2, &fields, Some("x.game.marker."))
                .unwrap()
        );

        let mut boundary = Sha256::new();
        boundary.update(TYPE_VERSION_DOMAIN);
        boundary.update(
            serde_json_canonicalizer::to_vec(&json!({
                "typeId":"game.marker", "targetVersion":u32::MAX,
                "fields":[], "optionalExtensionPrefix":null
            }))
            .unwrap(),
        );
        let expected_boundary: [u8; 32] = boundary.finalize().into();
        assert_eq!(
            type_version_definition_hash("game.marker", u64::from(u32::MAX), &[], None).unwrap(),
            expected_boundary
        );
    }

    #[test]
    fn invalid_semantic_symbols_and_bounds_refuse_before_hashing() {
        let text = FieldDefinition {
            type_id: "game.marker",
            field_id: "marker.caption",
            presence: Presence::Optional,
            kind: FieldKind::Text { max_utf8_bytes: 1 },
        };
        assert_eq!(
            field_meaning_signature(FieldDefinition {
                field_id: "marker..caption",
                ..text
            }),
            Err(DefinitionError::FieldId)
        );
        assert_eq!(
            field_meaning_signature(FieldDefinition {
                type_id: "core.marker",
                ..text
            }),
            Err(DefinitionError::TypeId)
        );
        assert_eq!(
            field_meaning_signature(FieldDefinition {
                kind: FieldKind::Text { max_utf8_bytes: 0 },
                ..text
            }),
            Err(DefinitionError::TextLimit)
        );
        assert_eq!(
            field_meaning_signature(FieldDefinition {
                kind: FieldKind::RecordRef {
                    target_type: "game.other"
                },
                ..text
            }),
            Err(DefinitionError::TargetType)
        );
        let field = VersionField {
            field_id: "marker.caption",
            signature: [1; 32],
        };
        assert_eq!(
            type_version_definition_hash("game.marker", 0, &[field], None),
            Err(DefinitionError::Version)
        );
        assert_eq!(
            type_version_definition_hash("game.marker", u64::MAX, &[field], None),
            Err(DefinitionError::Version)
        );
        assert_eq!(
            type_version_definition_hash("game.marker", 1, &[field, field], None),
            Err(DefinitionError::FieldOrder(1))
        );
        assert_eq!(
            type_version_definition_hash("game.marker", 1, &[field], Some("marker.")),
            Err(DefinitionError::ExtensionPrefix)
        );
        let extension_field = VersionField {
            field_id: "x.game.marker.value",
            signature: [1; 32],
        };
        assert_eq!(
            type_version_definition_hash(
                "game.marker",
                1,
                &[extension_field],
                Some("x.game.marker.")
            ),
            Err(DefinitionError::ExtensionOverlap(0))
        );
    }

    #[test]
    #[ignore = "local first-profile descriptor hash cost probe"]
    fn measure_definition_hashes_at_first_profile_bound() {
        let types: Vec<_> = (0..256)
            .map(|index| format!("game.type{index:03}"))
            .collect();
        let field_ids: Vec<_> = (0..128).map(|index| format!("field.f{index:03}")).collect();
        let mut version_fields = Vec::with_capacity(field_ids.len());
        let start = std::time::Instant::now();
        let mut checksum = 0_u8;
        for type_id in &types {
            version_fields.clear();
            for field_id in &field_ids {
                let signature = field_meaning_signature(FieldDefinition {
                    type_id,
                    field_id,
                    presence: Presence::Optional,
                    kind: FieldKind::Text {
                        max_utf8_bytes: 4096,
                    },
                })
                .unwrap();
                version_fields.push(VersionField {
                    field_id,
                    signature,
                });
            }
            checksum ^= type_version_definition_hash(type_id, 1, &version_fields, None).unwrap()[0];
        }
        std::hint::black_box(checksum);
        eprintln!(
            "definition_fields=32768 versions=256 elapsed={:?}",
            start.elapsed()
        );
    }
}
