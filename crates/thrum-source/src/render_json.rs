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

use serde::de::value::MapAccessDeserializer;
use serde::de::{DeserializeSeed, Error as _, MapAccess, SeqAccess, Visitor};
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

/// Walk one borrowed collection without retaining an array-wide item index.
/// The callback may keep an object only by copying it into its own charged
/// sink. A callback refusal is returned without hiding it as JSON syntax.
#[derive(Debug)]
pub(crate) enum RawArrayError<E> {
    Json(serde_json::Error),
    Visit(E),
}

pub(crate) fn visit_raw_array<'a, E>(
    raw: &'a RawValue,
    mut visit: impl FnMut(&'a RawValue) -> Result<(), E>,
) -> Result<(), RawArrayError<E>> {
    let mut callback_error = None;
    let mut deserializer = serde_json::Deserializer::from_str(raw.get());
    let decoded = RawArraySeed {
        visit: &mut visit,
        callback_error: &mut callback_error,
    }
    .deserialize(&mut deserializer);
    match decoded {
        Ok(()) => deserializer.end().map_err(RawArrayError::Json),
        Err(error) => match callback_error {
            Some(cause) => Err(RawArrayError::Visit(cause)),
            None => Err(RawArrayError::Json(error)),
        },
    }
}

struct RawArraySeed<'v, F, E> {
    visit: &'v mut F,
    callback_error: &'v mut Option<E>,
}

impl<'de, F, E> DeserializeSeed<'de> for RawArraySeed<'_, F, E>
where
    F: FnMut(&'de RawValue) -> Result<(), E>,
{
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        struct ArrayVisitor<'v, F, E> {
            visit: &'v mut F,
            callback_error: &'v mut Option<E>,
        }

        impl<'de, F, E> Visitor<'de> for ArrayVisitor<'_, F, E>
        where
            F: FnMut(&'de RawValue) -> Result<(), E>,
        {
            type Value = ();

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a render object collection array")
            }

            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut sequence: A,
            ) -> Result<Self::Value, A::Error> {
                while let Some(item) = sequence.next_element::<&'de RawValue>()? {
                    if let Err(cause) = (self.visit)(item) {
                        *self.callback_error = Some(cause);
                        return Err(A::Error::custom("render object visitor refused"));
                    }
                }
                Ok(())
            }
        }

        deserializer.deserialize_seq(ArrayVisitor {
            visit: self.visit,
            callback_error: self.callback_error,
        })
    }
}

pub(crate) fn present<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

/// Serde's derived struct visitor also accepts positional arrays. Source
/// object grammar requires maps, so feed the derived field visitor only a map.
pub(crate) fn map_only<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct MapOnly<T>(PhantomData<T>);
    impl<'de, T: Deserialize<'de>> Visitor<'de> for MapOnly<T> {
        type Value = T;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a render source object")
        }

        fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
            T::deserialize(MapAccessDeserializer::new(map))
        }
    }
    deserializer.deserialize_map(MapOnly(PhantomData))
}

/// Borrow one object's known values before interpreting field types. This
/// keeps decoding order fixed by the schema even when JSON keys are reordered.
/// Only the lexicographically smallest unknown decoded key is retained.
pub(crate) struct RawObject<'a, const N: usize> {
    pub(crate) values: [Option<&'a RawValue>; N],
    pub(crate) duplicate: [bool; N],
    pub(crate) unknown: Option<SourceText<'a>>,
}

pub(crate) struct IdentifiedObject<'a, const N: usize> {
    pub(crate) id: [u8; 16],
    pub(crate) fields: RawObject<'a, N>,
}

#[derive(Debug)]
pub(crate) enum IdentifiedObjectError {
    Json(serde_json::Error),
    MissingId,
    InvalidId,
    Duplicate { index: usize, id: Option<[u8; 16]> },
}

/// All embedded render objects start with the same canonical ID field.
/// Duplicate recognized decoded keys precede field semantics. Unknown keys
/// are one final Unknown refusal even if repeated. A repeated ID has no
/// unique diagnostic owner, regardless of which value appears last.
pub(crate) fn parse_identified_object<'a, const N: usize>(
    raw: &'a RawValue,
    names: &'static [&'static str; N],
) -> Result<IdentifiedObject<'a, N>, IdentifiedObjectError> {
    let fields = parse_raw_object(raw.get(), names).map_err(IdentifiedObjectError::Json)?;
    if fields.duplicate[0] {
        return Err(IdentifiedObjectError::Duplicate { index: 0, id: None });
    }
    let decoded_id: Option<Result<SourceText<'_>, _>> =
        fields.values[0].map(|value| serde_json::from_str(value.get()));
    let possible_id = decoded_id
        .as_ref()
        .and_then(|value| value.as_ref().ok())
        .and_then(|text| uuid_bytes(&text.0));
    if let Some(index) = fields.duplicate.iter().position(|&duplicate| duplicate) {
        return Err(IdentifiedObjectError::Duplicate {
            index,
            id: possible_id,
        });
    }
    let source_id = decoded_id
        .ok_or(IdentifiedObjectError::MissingId)?
        .map_err(IdentifiedObjectError::Json)?;
    let id = uuid_bytes(&source_id.0).ok_or(IdentifiedObjectError::InvalidId)?;
    Ok(IdentifiedObject { id, fields })
}

pub(crate) fn parse_raw_object<'a, const N: usize>(
    text: &'a str,
    names: &'static [&'static str; N],
) -> Result<RawObject<'a, N>, serde_json::Error> {
    struct RawObjectSeed<const N: usize> {
        names: &'static [&'static str; N],
    }
    impl<'de, const N: usize> DeserializeSeed<'de> for RawObjectSeed<N> {
        type Value = RawObject<'de, N>;

        fn deserialize<D: Deserializer<'de>>(
            self,
            deserializer: D,
        ) -> Result<Self::Value, D::Error> {
            struct ObjectVisitor<const N: usize> {
                names: &'static [&'static str; N],
            }
            impl<'de, const N: usize> Visitor<'de> for ObjectVisitor<N> {
                type Value = RawObject<'de, N>;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("a render source object map")
                }

                fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                    let mut result = RawObject {
                        values: [None; N],
                        duplicate: [false; N],
                        unknown: None,
                    };
                    while let Some(key) = map.next_key::<SourceText<'de>>()? {
                        let value = map.next_value::<&'de RawValue>()?;
                        if let Some(index) = self.names.iter().position(|name| *name == key.0) {
                            if result.values[index].replace(value).is_some() {
                                result.duplicate[index] = true;
                            }
                        } else if result
                            .unknown
                            .as_ref()
                            .is_none_or(|previous| key.0 < previous.0)
                        {
                            result.unknown = Some(key);
                        }
                    }
                    Ok(result)
                }
            }
            deserializer.deserialize_map(ObjectVisitor { names: self.names })
        }
    }

    let mut deserializer = serde_json::Deserializer::from_str(text);
    let result = RawObjectSeed { names }.deserialize(&mut deserializer)?;
    deserializer.end()?;
    Ok(result)
}

/// Borrow ordinary JSON strings; only escaped spellings need owned scratch.
pub(crate) struct SourceText<'a>(pub(crate) Cow<'a, str>);

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
    #[serde(borrow, deserialize_with = "map_only")]
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

pub(crate) fn uuid_bytes(text: &str) -> Option<[u8; 16]> {
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
        let positional_fields = ROOT.replace(
            "{\"textures\":[],\"materials\":[],\"instances\":[],\"meshes\":[],\"local_lights\":[],\"cameras\":[],\"active_camera\":\"a6b9c300-0000-4000-8000-00000000000a\"}",
            "[\"a6b9c300-0000-4000-8000-00000000000a\",[],[],[],[],[],[]]",
        );
        assert!(
            render_envelope_slices(check_render_record(positional_fields.as_bytes()).unwrap())
                .is_err()
        );
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

    #[test]
    fn streaming_array_keeps_each_object_borrowed_and_propagates_callback_refusal() {
        let source = r#"[ {"id":"one"}, {"id":"two"} ]"#;
        let raw: &RawValue = serde_json::from_str(source).unwrap();
        let mut seen = 0;
        visit_raw_array(raw, |item| -> Result<(), ()> {
            let begin = source.as_ptr() as usize;
            assert!((begin..begin + source.len()).contains(&(item.get().as_ptr() as usize)));
            seen += 1;
            Ok(())
        })
        .unwrap();
        assert_eq!(seen, 2);

        let mut seen = 0;
        let refusal = visit_raw_array(raw, |_| {
            seen += 1;
            Err("projection pressure")
        });
        assert!(matches!(
            refusal,
            Err(RawArrayError::Visit("projection pressure"))
        ));
        assert_eq!(seen, 1);
        let wrong: &RawValue = serde_json::from_str("null").unwrap();
        assert!(matches!(
            visit_raw_array(wrong, |_| -> Result<(), ()> { Ok(()) }),
            Err(RawArrayError::Json(_))
        ));
    }
}
