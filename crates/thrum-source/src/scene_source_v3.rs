//! Inactive strict `core.scene@3` source facts for the shared F3/F4 path.
//!
//! Local decoding checks source grammar only. These UUIDs are declarations,
//! not resolved edges, selected roots, admission receipts or cooked products.
//! Existing scene versions and the built-in registry remain unchanged.

use serde::de::Error as _;
use serde_json::value::RawValue;

use crate::environment_json::{CheckedRecord, MAX_STRING_BYTES, checked_record};
use crate::render_json::{SourceText, parse_raw_object, uuid_bytes};

/// Exact immutable descriptor bytes under the existing built-in hash domain.
/// Kept separate from scene@2 and deliberately absent from the registry.
pub(crate) const SCENE3_DEFINITION: &str = concat!(
    "thrum-builtin-definition-v1\ncore.scene@3\n",
    "envelope=id:canonical-rfc4122-uuid,type:core.scene,schema:3,fields:object,documentation?:utf8<=4096\n",
    "scene.label?:utf8<=4096\nenvironment.source?:required-ref<thrum.environment.authored>\n",
    "render.source:required-ref<thrum.render.scene@1>\n",
    "selection=environment.source-only-environment;render.source-only-render;unselected-required-refs-close\n",
    "field-diagnostics=recognized-decoded-key-duplicates-in-schema-order-first;values=scene.label,environment.source,render.source;unknown-last\n",
    "optional=absent-or-typed;null=refuse\n",
    "unknown=refuse;duplicate=refuse;json=strict-finite-i-json;record-bytes<=16777216\n",
);

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Scene3Facts {
    pub(crate) id: [u8; 16],
    pub(crate) label: Option<String>,
    pub(crate) environment_source: Option<[u8; 16]>,
    pub(crate) render_source: [u8; 16],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Scene3Field {
    Label,
    EnvironmentSource,
    RenderSource,
    Unknown,
}

#[derive(Debug)]
pub(crate) enum Scene3Problem {
    Json(serde_json::Error),
    Missing,
    Duplicate,
    InvalidUuid,
    StringByteLimit,
    Unknown(String),
}

#[derive(Debug)]
pub(crate) enum Scene3DecodeError {
    Header(serde_json::Error),
    WrongDefinition,
    Envelope(serde_json::Error),
    Documentation(Scene3Problem),
    Field {
        field: Scene3Field,
        problem: Scene3Problem,
    },
}

const ENVELOPE_FIELDS: [&str; 5] = ["id", "type", "schema", "fields", "documentation"];
const SCENE_FIELDS: [&str; 3] = ["scene.label", "environment.source", "render.source"];
const FIELD_ORDER: [Scene3Field; 3] = [
    Scene3Field::Label,
    Scene3Field::EnvironmentSource,
    Scene3Field::RenderSource,
];

/// Perform the common bounded whole-record walk exactly once, then decode.
pub(crate) fn decode_scene_v3(bytes: &[u8]) -> Result<Scene3Facts, Scene3DecodeError> {
    let checked = checked_record(bytes).map_err(Scene3DecodeError::Header)?;
    decode_checked_scene_v3(checked)
}

/// Consume the opaque byte-bound dispatch token without repeating its walk.
/// Definition checking precedes typed decoding, even for malformed fields.
pub(crate) fn decode_checked_scene_v3(
    checked: CheckedRecord<'_>,
) -> Result<Scene3Facts, Scene3DecodeError> {
    if checked.header().record_type != "core.scene" || checked.header().schema != 3 {
        return Err(Scene3DecodeError::WrongDefinition);
    }
    let text = std::str::from_utf8(checked.bytes())
        .map_err(|error| Scene3DecodeError::Envelope(serde_json::Error::custom(error)))?;
    // Fixed-count borrowed slots retain no JSON value tree. The checked header
    // already guarantees these envelope keys and the canonical record ID.
    let envelope = parse_raw_object(text, &ENVELOPE_FIELDS).map_err(Scene3DecodeError::Envelope)?;
    let required_envelope = |index: usize| {
        envelope.values[index].ok_or_else(|| {
            Scene3DecodeError::Envelope(serde_json::Error::custom("missing envelope field"))
        })
    };
    let id = decode_uuid(required_envelope(0)?).map_err(|problem| {
        Scene3DecodeError::Envelope(serde_json::Error::custom(format!(
            "invalid scene ID: {problem:?}"
        )))
    })?;
    if let Some(documentation) = envelope.values[4] {
        decode_text(documentation).map_err(Scene3DecodeError::Documentation)?;
    }
    let fields = parse_raw_object(required_envelope(3)?.get(), &SCENE_FIELDS)
        .map_err(Scene3DecodeError::Envelope)?;
    if let Some(index) = fields.duplicate.iter().position(|&duplicate| duplicate) {
        return Err(field_error(FIELD_ORDER[index], Scene3Problem::Duplicate));
    }

    // Every recognized duplicate precedes field meaning. Meanings follow
    // schema order, independent of JSON key order; unknown keys come last.
    let label = fields.values[0]
        .map(decode_text)
        .transpose()
        .map_err(|problem| field_error(Scene3Field::Label, problem))?
        .map(|text| text.0.into_owned());
    let environment_source = fields.values[1]
        .map(decode_uuid)
        .transpose()
        .map_err(|problem| field_error(Scene3Field::EnvironmentSource, problem))?;
    let render_source = decode_uuid(
        fields.values[2]
            .ok_or_else(|| field_error(Scene3Field::RenderSource, Scene3Problem::Missing))?,
    )
    .map_err(|problem| field_error(Scene3Field::RenderSource, problem))?;
    if let Some(unknown) = fields.unknown {
        return Err(field_error(
            Scene3Field::Unknown,
            Scene3Problem::Unknown(unknown.0.into_owned()),
        ));
    }
    Ok(Scene3Facts {
        id,
        label,
        environment_source,
        render_source,
    })
}

fn field_error(field: Scene3Field, problem: Scene3Problem) -> Scene3DecodeError {
    Scene3DecodeError::Field { field, problem }
}

fn decode_text<'a>(raw: &'a RawValue) -> Result<SourceText<'a>, Scene3Problem> {
    // Six ASCII escape bytes can encode one decoded UTF-8 byte. This bound
    // avoids allocating record-sized escape scratch for text already certain
    // to exceed the decoded byte limit. It never excludes valid bounded text.
    if raw.get().len() > 6 * MAX_STRING_BYTES + 2 {
        return Err(Scene3Problem::StringByteLimit);
    }
    let text: SourceText<'_> = serde_json::from_str(raw.get()).map_err(Scene3Problem::Json)?;
    if text.0.len() > MAX_STRING_BYTES {
        return Err(Scene3Problem::StringByteLimit);
    }
    Ok(text)
}

fn decode_uuid(raw: &RawValue) -> Result<[u8; 16], Scene3Problem> {
    // A canonical UUID is exactly 36 ASCII bytes after JSON unescaping.
    if raw.get().len() > 6 * 36 + 2 {
        return Err(Scene3Problem::InvalidUuid);
    }
    let text = decode_text(raw)?;
    uuid_bytes(&text.0).ok_or(Scene3Problem::InvalidUuid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    const ID: &str = "a6b9c300-0000-4000-8000-000000000003";
    const ENV: &str = "a6b9c300-0000-4000-8000-000000000004";
    const RENDER: &str = "a6b9c300-0000-4000-8000-000000000006";

    fn source(fields: &str) -> String {
        format!(r#"{{"fields":{fields},"id":"{ID}","schema":3,"type":"core.scene"}}"#)
    }

    fn decode(fields: &str) -> Result<Scene3Facts, Scene3DecodeError> {
        decode_scene_v3(source(fields).as_bytes())
    }

    fn assert_field(fields: &str, expected: Scene3Field, duplicate: bool) {
        let Scene3DecodeError::Field { field, problem } = decode(fields).unwrap_err() else {
            panic!("expected a field error");
        };
        assert_eq!(field, expected);
        assert_eq!(matches!(problem, Scene3Problem::Duplicate), duplicate);
    }

    #[test]
    fn authored_shape_optional_fields_and_binary_edges() {
        let facts = decode(&format!(
            r#"{{"environment.source":"{ENV}","render.source":"{RENDER}","scene.label":"R0 candidate receiver, fence and offscreen caster"}}"#
        ))
        .unwrap();
        assert_eq!(facts.id, uuid_bytes(ID).unwrap());
        assert_eq!(facts.environment_source, Some(uuid_bytes(ENV).unwrap()));
        assert_eq!(facts.render_source, uuid_bytes(RENDER).unwrap());
        assert_eq!(
            facts.label.as_deref(),
            Some("R0 candidate receiver, fence and offscreen caster")
        );
        let minimal = decode(&format!(r#"{{"render.source":"{RENDER}"}}"#)).unwrap();
        assert_eq!(minimal.label, None);
        assert_eq!(minimal.environment_source, None);

        let archived = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/evidence/render/R0-AUTHORED-SCENE3-RECORD-2026-11-03.json"
        ));
        assert_eq!(decode_scene_v3(archived).unwrap(), facts);
    }

    #[test]
    fn escaped_spellings_and_checked_token_are_equivalent() {
        let plain = source(&format!(
            r#"{{"scene.label":"café","environment.source":"{ENV}","render.source":"{RENDER}"}}"#
        ));
        let escaped = plain
            .replace("render.source", r"render\u002esource")
            .replace("environment.source", r"environment\u002esource")
            .replace("scene.label", r"scene\u002elabel")
            .replace("café", r"caf\u00e9")
            .replace("a6b9c300", r"\u00616b9c300")
            .replace("core.scene", r"core\u002escene");
        let expected = decode_scene_v3(plain.as_bytes()).unwrap();
        assert_eq!(decode_scene_v3(escaped.as_bytes()).unwrap(), expected);
        assert_eq!(
            decode_checked_scene_v3(checked_record(escaped.as_bytes()).unwrap()).unwrap(),
            expected
        );
        let fully_escaped_uuid = RENDER
            .bytes()
            .map(|byte| format!("\\u{byte:04x}"))
            .collect::<String>();
        assert_eq!(fully_escaped_uuid.len() + 2, 218);
        assert_eq!(
            decode(&format!(r#"{{"render.source":"{fully_escaped_uuid}"}}"#))
                .unwrap()
                .render_source,
            expected.render_source
        );
    }

    #[test]
    fn decoded_duplicates_precede_meaning_and_use_schema_order() {
        for fields in [
            r#"{"scene.label":null,"render.source":null,"render\u002esource":false}"#,
            r#"{"render\u002esource":false,"render.source":null,"scene.label":null}"#,
        ] {
            assert_field(fields, Scene3Field::RenderSource, true);
        }
        assert_field(
            r#"{"render.source":null,"render.source":false,"environment.source":null,"environment\u002esource":false}"#,
            Scene3Field::EnvironmentSource,
            true,
        );
        assert_field(
            r#"{"environment.source":null,"environment.source":false,"scene.label":0,"scene\u002elabel":false}"#,
            Scene3Field::Label,
            true,
        );
    }

    #[test]
    fn field_semantics_and_unknown_keys_have_fixed_order() {
        for fields in [
            r#"{"z":0,"render.source":null,"environment.source":null,"scene.label":null}"#,
            r#"{"scene.label":null,"environment.source":null,"render.source":null,"z":0}"#,
        ] {
            assert_field(fields, Scene3Field::Label, false);
        }
        assert_field(
            r#"{"z":0,"render.source":null,"environment.source":null}"#,
            Scene3Field::EnvironmentSource,
            false,
        );
        assert_field(r#"{"z":0}"#, Scene3Field::RenderSource, false);
        let Scene3DecodeError::Field {
            field: Scene3Field::Unknown,
            problem: Scene3Problem::Unknown(key),
        } = decode(&format!(
            r#"{{"z":0,"a":null,"a":false,"render.source":"{RENDER}"}}"#
        ))
        .unwrap_err()
        else {
            panic!("expected unknown key");
        };
        assert_eq!(key, "a");
    }

    #[test]
    fn missing_wrong_uuid_and_explicit_null_refuse() {
        assert_field("{}", Scene3Field::RenderSource, false);
        for value in [
            "null",
            "42",
            "[]",
            "{}",
            r#""not-a-uuid""#,
            r#""a6b9c300-0000-0000-8000-000000000006""#,
            r#""A6b9c300-0000-4000-8000-000000000006""#,
            r#""a6b9c300-0000-4000-c000-000000000006""#,
        ] {
            assert_field(
                &format!(r#"{{"render.source":{value}}}"#),
                Scene3Field::RenderSource,
                false,
            );
            assert_field(
                &format!(r#"{{"environment.source":{value},"render.source":"{RENDER}"}}"#),
                Scene3Field::EnvironmentSource,
                false,
            );
        }
        assert!(matches!(
            decode_scene_v3(
                source(&format!(r#"{{"render.source":"{RENDER}"}}"#))
                    .replace(ID, "bad-id")
                    .as_bytes()
            ),
            Err(Scene3DecodeError::Header(_))
        ));
        let oversized = format!(r#"{{"render.source":"{}"}}"#, r"\u0061".repeat(37));
        assert!(matches!(
            decode(&oversized),
            Err(Scene3DecodeError::Field {
                field: Scene3Field::RenderSource,
                problem: Scene3Problem::InvalidUuid,
            })
        ));
    }

    #[test]
    fn map_only_envelopes_fields_and_definition_gate() {
        for root in ["[]", "null", "42", r#"["id","type",3,{}]"#] {
            assert!(matches!(
                decode_scene_v3(root.as_bytes()),
                Err(Scene3DecodeError::Header(_))
            ));
        }
        for fields in ["[]", "null", "42", r#"[null,null,null]"#] {
            assert!(matches!(
                decode(fields),
                Err(Scene3DecodeError::Envelope(_))
            ));
        }
        for invalid in [
            source("{}").replace(&format!(r#""id":"{ID}","#), ""),
            source("{}").replacen('{', r#"{"unknown":null,"#, 1),
            source("{}").replacen('{', r#"{"ty\u0070e":"core.scene","#, 1),
        ] {
            assert!(matches!(
                decode_scene_v3(invalid.as_bytes()),
                Err(Scene3DecodeError::Header(_))
            ));
        }
        for (from, to) in [
            (r#""schema":3"#, r#""schema":2"#),
            ("core.scene", "game.scene"),
        ] {
            let wrong = source("null").replace(from, to);
            assert!(matches!(
                decode_scene_v3(wrong.as_bytes()),
                Err(Scene3DecodeError::WrongDefinition)
            ));
            assert!(matches!(
                decode_checked_scene_v3(checked_record(wrong.as_bytes()).unwrap()),
                Err(Scene3DecodeError::WrongDefinition)
            ));
        }
        assert!(crate::registry_stage::builtin_schema("core.scene", 3).is_none());
    }

    #[test]
    fn decoded_text_byte_bound_applies_to_label_and_documentation() {
        for text in ["a".repeat(4096), "é".repeat(2048), r"\u0061".repeat(4096)] {
            let valid = source(&format!(
                r#"{{"render.source":"{RENDER}","scene.label":"{text}"}}"#
            ));
            assert!(decode_scene_v3(valid.as_bytes()).is_ok());
            let valid = valid.replacen('{', &format!(r#"{{"documentation":"{text}","#), 1);
            assert!(decode_scene_v3(valid.as_bytes()).is_ok());
        }
        for text in ["a".repeat(4097), "é".repeat(2049), r"\u0061".repeat(4097)] {
            assert_field(
                &format!(r#"{{"render.source":"{RENDER}","scene.label":"{text}"}}"#),
                Scene3Field::Label,
                false,
            );
            let invalid = source(&format!(r#"{{"render.source":"{RENDER}"}}"#)).replacen(
                '{',
                &format!(r#"{{"documentation":"{text}","#),
                1,
            );
            assert!(matches!(
                decode_scene_v3(invalid.as_bytes()),
                Err(Scene3DecodeError::Documentation(
                    Scene3Problem::StringByteLimit
                ))
            ));
        }
        let null_documentation = source(&format!(r#"{{"render.source":"{RENDER}"}}"#)).replacen(
            '{',
            r#"{"documentation":null,"#,
            1,
        );
        assert!(matches!(
            decode_scene_v3(null_documentation.as_bytes()),
            Err(Scene3DecodeError::Header(_))
        ));
    }

    #[test]
    fn bounded_header_refuses_bytes_nodes_and_depth_before_fields() {
        let mut bytes = source(&format!(r#"{{"render.source":"{RENDER}"}}"#)).into_bytes();
        bytes.resize(crate::MAX_RECORD_BYTES, b' ');
        assert!(decode_scene_v3(&bytes).is_ok());
        bytes.push(b' ');
        assert!(matches!(
            decode_scene_v3(&bytes),
            Err(Scene3DecodeError::Header(_))
        ));
        // Envelope, fields, unknown array, ID, schema and type are six nodes.
        let maximum_nodes = format!(
            "{{\"unknown\":[{}0]}}",
            "0,".repeat(crate::MAX_RECORD_NODES - 7)
        );
        assert_field(&maximum_nodes, Scene3Field::RenderSource, false);
        let too_many_nodes = format!(
            "{{\"unknown\":[{}0]}}",
            "0,".repeat(crate::MAX_RECORD_NODES - 6)
        );
        assert!(matches!(
            decode(&too_many_nodes),
            Err(Scene3DecodeError::Header(_))
        ));
        // Fields occupies depth 1; 62 arrays occupy permitted depths 2..63.
        let maximum_depth = format!("{{\"unknown\":{}0{}}}", "[".repeat(62), "]".repeat(62));
        assert_field(&maximum_depth, Scene3Field::RenderSource, false);
        let too_deep = format!("{{\"unknown\":{}0{}}}", "[".repeat(63), "]".repeat(63));
        assert!(matches!(
            decode(&too_deep),
            Err(Scene3DecodeError::Header(_))
        ));
    }

    #[test]
    fn descriptor_hash_is_frozen_and_scene3_remains_unregistered() {
        let mut digest = Sha256::new();
        digest.update(b"thrum-f3-builtin-schema-v1\0");
        digest.update(SCENE3_DEFINITION.as_bytes());
        let actual: [u8; 32] = digest.finalize().into();
        assert_eq!(
            actual,
            [
                118, 113, 178, 22, 248, 9, 14, 102, 43, 223, 111, 133, 193, 0, 38, 200, 53, 24, 79,
                150, 181, 41, 8, 35, 89, 183, 183, 242, 157, 210, 34, 234
            ]
        );
        assert!(crate::registry_stage::builtin_schema("core.scene", 3).is_none());
        assert_ne!(SCENE3_DEFINITION, crate::environment_json::SCENE_DEFINITION);
    }
}
