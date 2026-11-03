//! Inactive bounded render-texture source validation and chunk replay.
//!
//! F3 can validate and discard decoded chunks. F4 may replay the same checked
//! token from a verified pin into a provisional product sink. Neither path
//! retains a whole decoded texture in this module.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use serde_json::value::RawValue;

use crate::MAX_RECORD_BYTES;
use crate::render_json::{IdentifiedObjectError, SourceText, parse_identified_object};

const ENCODED_CHUNK: usize = 16 * 1024;
const DECODED_CHUNK: usize = ENCODED_CHUNK * 3 / 4;
const TEXTURE_FIELDS: [&str; 5] = ["id", "width", "height", "format", "rgba8"];
const TEXTURE_FIELD_ORDER: [TextureField; 5] = [
    TextureField::Id,
    TextureField::Width,
    TextureField::Height,
    TextureField::Format,
    TextureField::Rgba8,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TextureFormat {
    Rgba8Srgb,
    Rgba8Linear,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TextureFacts {
    pub(crate) id: [u8; 16],
    pub(crate) width: u64,
    pub(crate) height: u64,
    pub(crate) format: TextureFormat,
    pub(crate) decoded_len: usize,
}

pub(crate) struct CheckedTexture<'a> {
    pub(crate) facts: TextureFacts,
    token: &'a RawValue,
    encoded_len: usize,
    escaped: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum TextureField {
    Id,
    Width,
    Height,
    Format,
    Rgba8,
    Unknown,
}

#[derive(Debug)]
pub(crate) enum TextureProblem {
    Json(serde_json::Error),
    Missing,
    Duplicate,
    Unknown(String),
    InvalidId,
    Dimension,
    SizeOverflow,
    InvalidFormat,
    WrongTokenType,
    NonAscii,
    InvalidEscape,
    EncodedLength { expected: usize, actual: usize },
    Base64(base64::DecodeSliceError),
    DecodedLength { expected: usize, actual: usize },
}

#[derive(Debug)]
pub(crate) struct TextureSourceError {
    pub(crate) id: Option<[u8; 16]>,
    pub(crate) field: TextureField,
    pub(crate) problem: TextureProblem,
}

#[derive(Debug)]
pub(crate) enum TextureEmitError<E> {
    Source(TextureSourceError),
    Projection(E),
}

fn source_error(
    id: Option<[u8; 16]>,
    field: TextureField,
    problem: TextureProblem,
) -> TextureSourceError {
    TextureSourceError { id, field, problem }
}

/// Performs complete local texture source validation without a texel Vec.
pub(crate) fn check_texture_v1(raw: &RawValue) -> Result<CheckedTexture<'_>, TextureSourceError> {
    let object = parse_identified_object(raw, &TEXTURE_FIELDS).map_err(|error| match error {
        IdentifiedObjectError::Json(source) => {
            source_error(None, TextureField::Id, TextureProblem::Json(source))
        }
        IdentifiedObjectError::MissingId => {
            source_error(None, TextureField::Id, TextureProblem::Missing)
        }
        IdentifiedObjectError::InvalidId => {
            source_error(None, TextureField::Id, TextureProblem::InvalidId)
        }
        IdentifiedObjectError::Duplicate { index, id } => {
            source_error(id, TEXTURE_FIELD_ORDER[index], TextureProblem::Duplicate)
        }
    })?;
    let id = object.id;
    let fields = object.fields;
    let read = |index: usize, field| {
        fields.values[index].ok_or_else(|| source_error(Some(id), field, TextureProblem::Missing))
    };
    let width: u64 =
        serde_json::from_str(read(1, TextureField::Width)?.get()).map_err(|source| {
            source_error(Some(id), TextureField::Width, TextureProblem::Json(source))
        })?;
    if width == 0 {
        return Err(source_error(
            Some(id),
            TextureField::Width,
            TextureProblem::Dimension,
        ));
    }
    let height: u64 =
        serde_json::from_str(read(2, TextureField::Height)?.get()).map_err(|source| {
            source_error(Some(id), TextureField::Height, TextureProblem::Json(source))
        })?;
    if height == 0 {
        return Err(source_error(
            Some(id),
            TextureField::Height,
            TextureProblem::Dimension,
        ));
    }
    let format_text: SourceText<'_> = serde_json::from_str(read(3, TextureField::Format)?.get())
        .map_err(|source| {
            source_error(Some(id), TextureField::Format, TextureProblem::Json(source))
        })?;
    let format = match format_text.0.as_ref() {
        "rgba8-srgb" => TextureFormat::Rgba8Srgb,
        "rgba8-linear" => TextureFormat::Rgba8Linear,
        _ => {
            return Err(source_error(
                Some(id),
                TextureField::Format,
                TextureProblem::InvalidFormat,
            ));
        }
    };
    let decoded_u64 = width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| source_error(Some(id), TextureField::Rgba8, TextureProblem::SizeOverflow))?;
    let decoded_len = usize::try_from(decoded_u64)
        .map_err(|_| source_error(Some(id), TextureField::Rgba8, TextureProblem::SizeOverflow))?;
    let encoded_len = decoded_len
        .checked_add(2)
        .map(|n| n / 3)
        .and_then(|groups| groups.checked_mul(4))
        .ok_or_else(|| source_error(Some(id), TextureField::Rgba8, TextureProblem::SizeOverflow))?;
    if encoded_len
        .checked_add(2)
        .is_none_or(|n| n > MAX_RECORD_BYTES)
    {
        return Err(source_error(
            Some(id),
            TextureField::Rgba8,
            TextureProblem::SizeOverflow,
        ));
    }
    let token = read(4, TextureField::Rgba8)?;
    let content = string_content(token)
        .map_err(|problem| source_error(Some(id), TextureField::Rgba8, problem))?;
    let (actual, escaped) = semantic_len(content)
        .map_err(|problem| source_error(Some(id), TextureField::Rgba8, problem))?;
    if actual != encoded_len {
        return Err(source_error(
            Some(id),
            TextureField::Rgba8,
            TextureProblem::EncodedLength {
                expected: encoded_len,
                actual,
            },
        ));
    }
    stream_chunks(content, encoded_len, decoded_len, escaped, |_| {
        Ok::<_, ()>(())
    })
    .map_err(|error| match error {
        StreamError::Source(problem) => source_error(Some(id), TextureField::Rgba8, problem),
        StreamError::Projection(()) => unreachable!(),
    })?;
    if let Some(unknown) = fields.unknown {
        return Err(source_error(
            Some(id),
            TextureField::Unknown,
            TextureProblem::Unknown(unknown.0.into_owned()),
        ));
    }
    Ok(CheckedTexture {
        facts: TextureFacts {
            id,
            width,
            height,
            format,
            decoded_len,
        },
        token,
        encoded_len,
        escaped,
    })
}

/// Replays already locally validated bytes into one provisional sink.
pub(crate) fn emit_texture_chunks<E>(
    checked: &CheckedTexture<'_>,
    emit: impl FnMut(&[u8]) -> Result<(), E>,
) -> Result<(), TextureEmitError<E>> {
    let content = string_content(checked.token).map_err(|problem| {
        TextureEmitError::Source(source_error(
            Some(checked.facts.id),
            TextureField::Rgba8,
            problem,
        ))
    })?;
    stream_chunks(
        content,
        checked.encoded_len,
        checked.facts.decoded_len,
        checked.escaped,
        emit,
    )
    .map_err(|error| match error {
        StreamError::Source(problem) => TextureEmitError::Source(source_error(
            Some(checked.facts.id),
            TextureField::Rgba8,
            problem,
        )),
        StreamError::Projection(cause) => TextureEmitError::Projection(cause),
    })
}

fn string_content(raw: &RawValue) -> Result<&[u8], TextureProblem> {
    let token = raw
        .get()
        .trim_matches(|c| matches!(c, ' ' | '\t' | '\r' | '\n'))
        .as_bytes();
    if token.len() < 2 || token[0] != b'"' || token[token.len() - 1] != b'"' {
        return Err(TextureProblem::WrongTokenType);
    }
    Ok(&token[1..token.len() - 1])
}

fn semantic_len(content: &[u8]) -> Result<(usize, bool), TextureProblem> {
    if !content.contains(&b'\\') {
        return Ok((content.len(), false));
    }
    let mut input = JsonAscii { content, at: 0 };
    let mut len = 0usize;
    loop {
        let remaining = &content[input.at..];
        let Some(plain) = remaining.iter().position(|&byte| byte == b'\\') else {
            len += remaining.len();
            break;
        };
        len += plain + 1;
        input.at += plain;
        input.next_escape()?;
    }
    Ok((len, true))
}

struct JsonAscii<'a> {
    content: &'a [u8],
    at: usize,
}

impl JsonAscii<'_> {
    fn next_escape(&mut self) -> Result<u8, TextureProblem> {
        if self.content.get(self.at) != Some(&b'\\') {
            return Err(TextureProblem::InvalidEscape);
        }
        self.at += 1;
        let escaped = *self
            .content
            .get(self.at)
            .ok_or(TextureProblem::InvalidEscape)?;
        self.at += 1;
        let decoded = match escaped {
            b'"' | b'\\' | b'/' => escaped,
            b'b' => 8,
            b'f' => 12,
            b'n' => b'\n',
            b'r' => b'\r',
            b't' => b'\t',
            b'u' => {
                let digits = self
                    .content
                    .get(self.at..self.at + 4)
                    .ok_or(TextureProblem::InvalidEscape)?;
                self.at += 4;
                let mut value = 0u16;
                for digit in digits {
                    value = (value << 4) | hex(*digit).ok_or(TextureProblem::InvalidEscape)?;
                }
                u8::try_from(value).map_err(|_| TextureProblem::NonAscii)?
            }
            _ => return Err(TextureProblem::InvalidEscape),
        };
        Ok(decoded)
    }

    fn fill(&mut self, out: &mut [u8]) -> Result<(), TextureProblem> {
        let mut filled = 0;
        while filled < out.len() {
            let remaining = &self.content[self.at..];
            let limit = remaining.len().min(out.len() - filled);
            let plain = remaining[..limit]
                .iter()
                .position(|&byte| byte == b'\\')
                .unwrap_or(limit);
            out[filled..filled + plain].copy_from_slice(&remaining[..plain]);
            filled += plain;
            self.at += plain;
            if filled < out.len() {
                out[filled] = self.next_escape()?;
                filled += 1;
            }
        }
        Ok(())
    }
}

fn hex(byte: u8) -> Option<u16> {
    match byte {
        b'0'..=b'9' => Some(u16::from(byte - b'0')),
        b'a'..=b'f' => Some(u16::from(byte - b'a') + 10),
        b'A'..=b'F' => Some(u16::from(byte - b'A') + 10),
        _ => None,
    }
}

enum StreamError<E> {
    Source(TextureProblem),
    Projection(E),
}

fn stream_chunks<E>(
    content: &[u8],
    encoded_len: usize,
    decoded_len: usize,
    escaped: bool,
    mut emit: impl FnMut(&[u8]) -> Result<(), E>,
) -> Result<(), StreamError<E>> {
    let mut unescape = JsonAscii { content, at: 0 };
    let mut encoded_scratch = [0u8; ENCODED_CHUNK];
    let mut decoded_scratch = [0u8; DECODED_CHUNK];
    let mut encoded_at = 0usize;
    let mut decoded_at = 0usize;
    while encoded_at < encoded_len {
        let take = (encoded_len - encoded_at).min(ENCODED_CHUNK);
        let encoded = if escaped {
            let plain = content.get(unescape.at..unescape.at + take);
            if let Some(plain) = plain.filter(|bytes| !bytes.contains(&b'\\')) {
                unescape.at += take;
                plain
            } else {
                unescape
                    .fill(&mut encoded_scratch[..take])
                    .map_err(StreamError::Source)?;
                &encoded_scratch[..take]
            }
        } else {
            &content[encoded_at..encoded_at + take]
        };
        let written = STANDARD
            .decode_slice(encoded, &mut decoded_scratch)
            .map_err(|source| StreamError::Source(TextureProblem::Base64(source)))?;
        let nonfinal = encoded_at + take < encoded_len;
        if nonfinal && written != take * 3 / 4 {
            return Err(StreamError::Source(TextureProblem::DecodedLength {
                expected: take * 3 / 4,
                actual: written,
            }));
        }
        decoded_at = decoded_at
            .checked_add(written)
            .ok_or(StreamError::Source(TextureProblem::SizeOverflow))?;
        if decoded_at > decoded_len {
            return Err(StreamError::Source(TextureProblem::DecodedLength {
                expected: decoded_len,
                actual: decoded_at,
            }));
        }
        emit(&decoded_scratch[..written]).map_err(StreamError::Projection)?;
        encoded_at += take;
    }
    if decoded_at != decoded_len {
        return Err(StreamError::Source(TextureProblem::DecodedLength {
            expected: decoded_len,
            actual: decoded_at,
        }));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    const ID: &str = "a6b9c300-0000-4000-8000-000000000017";

    fn raw(text: &str) -> &RawValue {
        serde_json::from_str(text).unwrap()
    }

    fn texture(width: u64, height: u64, encoded_json: &str) -> String {
        format!(
            r#"{{"rgba8":{encoded_json},"format":"rgba8-srgb","height":{height},"width":{width},"id":"{ID}"}}"#
        )
    }

    fn digest_hex(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    #[test]
    fn canonical_padding_cases_and_borrowed_chunk_replay() {
        for (width, encoded, expected) in [
            (1, r#""AAECAw==""#, &b"\x00\x01\x02\x03"[..]),
            (
                2,
                r#""AAECAwQFBgc=""#,
                &b"\x00\x01\x02\x03\x04\x05\x06\x07"[..],
            ),
            (
                3,
                r#""AAECAwQFBgcICQoL""#,
                &b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b"[..],
            ),
        ] {
            let object = texture(width, 1, encoded);
            let checked = check_texture_v1(raw(&object)).unwrap();
            assert_eq!(checked.facts.width, width);
            assert_eq!(checked.facts.decoded_len, expected.len());
            let mut emitted = Vec::new();
            emit_texture_chunks(&checked, |chunk| {
                emitted.extend_from_slice(chunk);
                Ok::<_, ()>(())
            })
            .unwrap();
            assert_eq!(emitted, expected);
        }
    }

    #[test]
    fn escaped_equivalent_and_callback_refusal() {
        let plain = texture(1, 1, r#""/////w==""#);
        let escaped = texture(1, 1, r#""\/\u002f\/\/\/w==""#);
        let mut outputs = Vec::new();
        for object in [&plain, &escaped] {
            let checked = check_texture_v1(raw(object)).unwrap();
            let mut bytes = Vec::new();
            emit_texture_chunks(&checked, |chunk| {
                bytes.extend_from_slice(chunk);
                Ok::<_, ()>(())
            })
            .unwrap();
            outputs.push(bytes);
            assert!(matches!(
                emit_texture_chunks(&checked, |_| Err("sink pressure")),
                Err(TextureEmitError::Projection("sink pressure"))
            ));
        }
        assert_eq!(outputs[0], outputs[1]);
        assert_eq!(outputs[0], [255, 255, 255, 255]);
    }

    #[test]
    fn invalid_padding_alphabet_and_length_refuse_before_projection() {
        for encoded in [
            r#""AAECAx==""#, // same decoded octets only if nonzero pad bits are ignored
            r#""AAECAw=""#,
            r#""AA==AAAw""#,
            r#""AAECAw===""#,
            r#""AAECAw-=""#,
            r#""AAEC Aw==""#,
        ] {
            let object = texture(1, 1, encoded);
            assert!(check_texture_v1(raw(&object)).is_err(), "{encoded}");
        }
        let wrong_type = texture(1, 1, "null");
        assert!(matches!(
            check_texture_v1(raw(&wrong_type)),
            Err(TextureSourceError {
                field: TextureField::Rgba8,
                problem: TextureProblem::WrongTokenType,
                ..
            })
        ));
    }

    #[test]
    fn dimension_overflow_and_field_priority_refuse_without_allocating_texels() {
        let zero = texture(0, 1, r#""AAECAw==""#);
        assert!(matches!(
            check_texture_v1(raw(&zero)),
            Err(TextureSourceError {
                field: TextureField::Width,
                problem: TextureProblem::Dimension,
                ..
            })
        ));
        let huge = texture(u64::MAX, u64::MAX, r#""AAECAw==""#);
        assert!(matches!(
            check_texture_v1(raw(&huge)),
            Err(TextureSourceError {
                field: TextureField::Rgba8,
                problem: TextureProblem::SizeOverflow,
                ..
            })
        ));
        let invalid_format_and_bytes =
            texture(1, 1, r#""not base64""#).replace("rgba8-srgb", "wrong");
        assert!(matches!(
            check_texture_v1(raw(&invalid_format_and_bytes)),
            Err(TextureSourceError {
                field: TextureField::Format,
                problem: TextureProblem::InvalidFormat,
                ..
            })
        ));
        let duplicate =
            texture(1, 1, r#""AAECAw==""#).replace(r#""width":1"#, r#""width":1,"w\u0069dth":1"#);
        assert!(matches!(
            check_texture_v1(raw(&duplicate)),
            Err(TextureSourceError {
                field: TextureField::Width,
                problem: TextureProblem::Duplicate,
                ..
            })
        ));
    }

    #[test]
    fn chunk_boundary_matches_full_bytes_and_rejects_early_padding() {
        let expected: Vec<u8> = (0..4097 * 4).map(|index| index as u8).collect();
        let mut encoded_bytes = vec![0u8; expected.len().div_ceil(3) * 4];
        let written = STANDARD
            .encode_slice(&expected, &mut encoded_bytes)
            .unwrap();
        assert_eq!(written, encoded_bytes.len());
        let encoded = String::from_utf8(encoded_bytes).unwrap();
        // Independent Python base64/hashlib oracle for this deterministic
        // 16,388-byte image, not a self-round-trip assertion.
        assert_eq!(
            digest_hex(encoded.as_bytes()),
            "e0774e7d579c66defc97cfe2ad6b4845c15e03861a2e17fe9dabe371d99bb7e4"
        );
        assert!(encoded.len() > ENCODED_CHUNK);
        let source = texture(4097, 1, &format!(r#""{encoded}""#));
        let checked = check_texture_v1(raw(&source)).unwrap();
        let mut chunks = Vec::new();
        let mut actual = Vec::new();
        emit_texture_chunks(&checked, |chunk| {
            chunks.push(chunk.len());
            actual.extend_from_slice(chunk);
            Ok::<_, ()>(())
        })
        .unwrap();
        assert_eq!(chunks, [DECODED_CHUNK, expected.len() - DECODED_CHUNK]);
        assert_eq!(actual, expected);
        assert_eq!(
            digest_hex(&actual),
            "fed3630f74730e5f219ba6b28455778fd450c6eb62409242d23406ee026e9c2c"
        );

        for at in [0, ENCODED_CHUNK - 1, ENCODED_CHUNK, ENCODED_CHUNK + 1] {
            let mut escaped = encoded.clone();
            let replacement = format!(r"\u00{:02x}", encoded.as_bytes()[at]);
            escaped.replace_range(at..at + 1, &replacement);
            let escaped_source = texture(4097, 1, &format!(r#""{escaped}""#));
            let checked_escaped = check_texture_v1(raw(&escaped_source)).unwrap();
            let mut escaped_bytes = Vec::new();
            emit_texture_chunks(&checked_escaped, |chunk| {
                escaped_bytes.extend_from_slice(chunk);
                Ok::<_, ()>(())
            })
            .unwrap();
            assert_eq!(escaped_bytes, expected, "escape at semantic byte {at}");
        }

        let mut calls = 0;
        assert!(matches!(
            emit_texture_chunks(&checked, |_| {
                calls += 1;
                Err("provisional sink refused")
            }),
            Err(TextureEmitError::Projection("provisional sink refused"))
        ));
        assert_eq!(calls, 1);

        let mut early_padding = encoded;
        early_padding.replace_range(ENCODED_CHUNK - 1..ENCODED_CHUNK, "=");
        let invalid = texture(4097, 1, &format!(r#""{early_padding}""#));
        assert!(check_texture_v1(raw(&invalid)).is_err());
    }

    #[test]
    #[ignore = "local release-mode texture candidate cost probe"]
    fn measure_texture_candidate() {
        use std::hint::black_box;
        use std::time::Instant;

        let small = texture(1, 1, r#""AAECAw==""#);
        let small_raw = raw(&small);
        let mut small_samples = Vec::new();
        for _ in 0..5 {
            let start = Instant::now();
            for _ in 0..10_000 {
                black_box(check_texture_v1(small_raw).unwrap().facts);
            }
            small_samples.push(start.elapsed().as_nanos() / 10_000);
        }
        small_samples.sort_unstable();

        let pixels = vec![0x7fu8; 1280 * 1280 * 4];
        let mut encoded = vec![0u8; pixels.len().div_ceil(3) * 4];
        STANDARD.encode_slice(&pixels, &mut encoded).unwrap();
        let encoded = String::from_utf8(encoded).unwrap();
        let large = texture(1280, 1280, &format!(r#""{encoded}""#));
        let escaped_large = texture(
            1280,
            1280,
            &format!(r#""{}""#, encoded.replacen('f', r"\u0066", 1)),
        );
        let mut large_samples = Vec::new();
        for source in [&large, &escaped_large] {
            let source_raw = raw(source);
            let mut checks = Vec::new();
            let mut emits = Vec::new();
            for _ in 0..5 {
                let start = Instant::now();
                let checked = black_box(check_texture_v1(source_raw).unwrap());
                checks.push(start.elapsed().as_micros());
                let start = Instant::now();
                let mut total = 0usize;
                emit_texture_chunks(&checked, |chunk| {
                    total += chunk.len();
                    Ok::<_, ()>(())
                })
                .unwrap();
                black_box(total);
                emits.push(start.elapsed().as_micros());
            }
            checks.sort_unstable();
            emits.sort_unstable();
            large_samples.push((checks[2], emits[2]));
        }
        eprintln!(
            "texture candidate: small check median {} ns; 1280² plain check/emit {} / {} us; escaped check/emit {} / {} us",
            small_samples[2],
            large_samples[0].0,
            large_samples[0].1,
            large_samples[1].0,
            large_samples[1].1
        );
    }
}
