//! Private render-v1 envelope and borrowed collection handoff.
//!
//! This is the first inactive layer of the shared F3/F4 decoder. It reuses
//! F3's existing whole-record bounded header walk exactly once, then checks
//! the strict render envelope and borrows the six collection values from the
//! original bytes. Collection objects, numeric meaning and global closure
//! are not checked by this layer and cannot receive an admission receipt.

use std::borrow::Cow;
use std::fmt;
use std::marker::PhantomData;

use serde::de::Visitor;
use serde::{Deserialize, Deserializer};
use serde_json::value::RawValue;

use crate::environment_json::{CheckedRecord, checked_record};
use crate::record::canonical_uuid;

#[derive(Debug)]
pub(crate) enum RenderEnvelopeError {
    Header(serde_json::Error),
    Typed(serde_json::Error),
    Meaning,
}

/// The original byte borrow and successful one-pass F3 header result cannot
/// be separated or substituted by a caller.
pub(crate) struct CheckedRenderRecord<'a> {
    checked: CheckedRecord<'a>,
}

pub(crate) fn check_render_record(
    bytes: &[u8],
) -> Result<CheckedRenderRecord<'_>, RenderEnvelopeError> {
    let checked = checked_record(bytes).map_err(RenderEnvelopeError::Header)?;
    checked_render_record(checked)
}

/// F3 dispatch passes the opaque checked record it used to classify the
/// source; no caller can pair an independently supplied header with bytes.
pub(crate) fn checked_render_record(
    checked: CheckedRecord<'_>,
) -> Result<CheckedRenderRecord<'_>, RenderEnvelopeError> {
    let header = checked.header();
    if header.record_type != "thrum.render.scene" || header.schema != 1 {
        return Err(RenderEnvelopeError::Meaning);
    }
    Ok(CheckedRenderRecord { checked })
}

/// Fixed-count raw-slice handoff. Every slice points into the authenticated
/// candidate or accepted record; no collection-wide raw-value vector exists.
/// The caller must finish strict typed decoding before treating any field as
/// valid source meaning.
pub(crate) struct RenderEnvelopeSlices<'a> {
    pub(crate) id: [u8; 16],
    pub(crate) active_camera: [u8; 16],
    pub(crate) cameras: &'a RawValue,
    pub(crate) meshes: &'a RawValue,
    pub(crate) instances: &'a RawValue,
    pub(crate) materials: &'a RawValue,
    pub(crate) textures: &'a RawValue,
    pub(crate) local_lights: &'a RawValue,
}

pub(crate) fn render_envelope_slices<'a>(
    checked: CheckedRenderRecord<'a>,
) -> Result<RenderEnvelopeSlices<'a>, RenderEnvelopeError> {
    let mut deserializer = serde_json::Deserializer::from_slice(checked.checked.bytes());
    let root =
        RenderEnvelope::deserialize(&mut deserializer).map_err(RenderEnvelopeError::Typed)?;
    deserializer.end().map_err(RenderEnvelopeError::Typed)?;
    if root.record_type.0 != "thrum.render.scene" || root.schema != 1 {
        return Err(RenderEnvelopeError::Meaning);
    }
    if let Some(raw) = root.documentation {
        // At most six ASCII escape bytes can encode one decoded UTF-8 byte.
        // Reject a longer raw string before serde can allocate escape scratch.
        let token = raw
            .get()
            .trim_matches(|byte| matches!(byte, ' ' | '\t' | '\r' | '\n'));
        if token.len() > 6 * 4096 + 2 {
            return Err(RenderEnvelopeError::Meaning);
        }
        let text: SourceText<'_> =
            serde_json::from_str(raw.get()).map_err(RenderEnvelopeError::Typed)?;
        if text.0.len() > 4096 {
            return Err(RenderEnvelopeError::Meaning);
        }
    }
    Ok(RenderEnvelopeSlices {
        id: uuid_bytes(&root.id.0).ok_or(RenderEnvelopeError::Meaning)?,
        active_camera: uuid_bytes(&root.fields.active_camera.0)
            .ok_or(RenderEnvelopeError::Meaning)?,
        cameras: root.fields.cameras,
        meshes: root.fields.meshes,
        instances: root.fields.instances,
        materials: root.fields.materials,
        textures: root.fields.textures,
        local_lights: root.fields.local_lights,
    })
}

fn present<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

/// Borrow ordinary JSON strings; only escaped spellings need owned scratch.
struct SourceText<'a>(Cow<'a, str>);

impl<'de: 'a, 'a> Deserialize<'de> for SourceText<'a> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct TextVisitor<'a>(PhantomData<&'a str>);
        impl<'de: 'a, 'a> Visitor<'de> for TextVisitor<'a> {
            type Value = SourceText<'a>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a source string")
            }

            fn visit_borrowed_str<E: serde::de::Error>(
                self,
                text: &'de str,
            ) -> Result<Self::Value, E> {
                Ok(SourceText(Cow::Borrowed(text)))
            }

            fn visit_str<E: serde::de::Error>(self, text: &str) -> Result<Self::Value, E> {
                Ok(SourceText(Cow::Owned(text.to_owned())))
            }

            fn visit_string<E: serde::de::Error>(self, text: String) -> Result<Self::Value, E> {
                Ok(SourceText(Cow::Owned(text)))
            }
        }
        deserializer.deserialize_str(TextVisitor(PhantomData))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RenderEnvelope<'a> {
    #[serde(borrow)]
    id: SourceText<'a>,
    #[serde(borrow, rename = "type")]
    record_type: SourceText<'a>,
    schema: u64,
    #[serde(borrow)]
    fields: RenderFields<'a>,
    #[serde(borrow, default, deserialize_with = "present")]
    documentation: Option<&'a RawValue>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RenderFields<'a> {
    #[serde(borrow)]
    active_camera: SourceText<'a>,
    #[serde(borrow)]
    cameras: &'a RawValue,
    #[serde(borrow)]
    meshes: &'a RawValue,
    #[serde(borrow)]
    instances: &'a RawValue,
    #[serde(borrow)]
    materials: &'a RawValue,
    #[serde(borrow)]
    textures: &'a RawValue,
    #[serde(borrow)]
    local_lights: &'a RawValue,
}

fn uuid_bytes(text: &str) -> Option<[u8; 16]> {
    if !canonical_uuid(text) {
        return None;
    }
    let mut result = [0_u8; 16];
    let mut digits = text.as_bytes().iter().copied().filter(|byte| *byte != b'-');
    for octet in &mut result {
        let hex = |byte| match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            _ => unreachable!("canonical_uuid checked hexadecimal digits"),
        };
        *octet = (hex(digits.next()?) << 4) | hex(digits.next()?);
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOT: &str = r#"{"fields":{"textures":[],"materials":[],"instances":[],"meshes":[],"local_lights":[],"cameras":[],"active_camera":"a6b9c300-0000-4000-8000-00000000000a"},"schema":1,"type":"thrum.render.scene","id":"a6b9c300-0000-4000-8000-000000000006"}"#;

    #[test]
    fn fields_before_identity_and_borrowed_collections() {
        let checked = checked_render_record(checked_record(ROOT.as_bytes()).unwrap()).unwrap();
        let parsed = render_envelope_slices(checked).unwrap();
        assert_eq!(parsed.id[15], 6);
        assert_eq!(parsed.active_camera[15], 10);
        assert_eq!(parsed.cameras.get(), "[]");
        assert_eq!(parsed.textures.get(), "[]");
        let base = ROOT.as_ptr() as usize;
        let end = base + ROOT.len();
        for raw in [
            parsed.cameras,
            parsed.meshes,
            parsed.instances,
            parsed.materials,
            parsed.textures,
            parsed.local_lights,
        ] {
            let slice = raw.get().as_ptr() as usize;
            assert!((base..end).contains(&slice));
        }
        let root: RenderEnvelope<'_> = serde_json::from_str(ROOT).unwrap();
        assert!(matches!(root.id.0, Cow::Borrowed(_)));
        assert!(matches!(root.record_type.0, Cow::Borrowed(_)));
        assert!(matches!(root.fields.active_camera.0, Cow::Borrowed(_)));
    }

    #[test]
    fn decoded_duplicate_unknown_missing_and_null_fields_refuse() {
        let escaped = ROOT.replace("\"active_camera\"", "\"active\\u005fcamera\"");
        let duplicate = escaped.replace(
            "\"active\\u005fcamera\"",
            "\"active\\u005fcamera\":\"a6b9c300-0000-4000-8000-00000000000a\",\"active_camera\"",
        );
        assert!(matches!(
            render_envelope_slices(check_render_record(duplicate.as_bytes()).unwrap()),
            Err(RenderEnvelopeError::Typed(_))
        ));
        let unknown = ROOT.replace("\"cameras\":[]", "\"normal_maps\":[],\"cameras\":[]");
        assert!(render_envelope_slices(check_render_record(unknown.as_bytes()).unwrap()).is_err());
        let missing = ROOT.replace("\"textures\":[],", "");
        assert!(render_envelope_slices(check_render_record(missing.as_bytes()).unwrap()).is_err());
        let null_documentation = ROOT.replacen('{', "{\"documentation\":null,", 1);
        assert!(check_render_record(null_documentation.as_bytes()).is_err());
    }

    #[test]
    fn escaped_equivalent_envelope_and_bounded_documentation() {
        let escaped = ROOT
            .replace("\"type\"", "\"t\\u0079pe\"")
            .replace("thrum.render.scene", "thrum.render.\\u0073cene")
            .replace("\"active_camera\"", "\"active\\u005fcamera\"")
            .replace("\"a6b9c300", "\"\\u00616b9c300");
        let parsed =
            render_envelope_slices(check_render_record(escaped.as_bytes()).unwrap()).unwrap();
        assert_eq!(parsed.id[15], 6);
        assert_eq!(parsed.active_camera[15], 10);
        let escaped_root: RenderEnvelope<'_> = serde_json::from_str(&escaped).unwrap();
        assert!(matches!(escaped_root.id.0, Cow::Owned(_)));

        let documented = ROOT.replacen(
            '{',
            &format!("{{\"documentation\":\"{}\",", "x".repeat(4096)),
            1,
        );
        assert!(
            render_envelope_slices(check_render_record(documented.as_bytes()).unwrap()).is_ok()
        );
        let too_long = ROOT.replacen(
            '{',
            &format!("{{\"documentation\":\"{}\",", "x".repeat(4097)),
            1,
        );
        assert!(matches!(
            render_envelope_slices(check_render_record(too_long.as_bytes()).unwrap()),
            Err(RenderEnvelopeError::Meaning)
        ));
        let escaped_doc = ROOT.replacen(
            '{',
            &format!("{{\"documentation\":\"{}\",", "\\u0061".repeat(4096)),
            1,
        );
        assert!(
            render_envelope_slices(check_render_record(escaped_doc.as_bytes()).unwrap()).is_ok()
        );
    }
}
