//! Test-only bounded decoder for the stored canonical source manifest.
//!
//! The durable store must authenticate the blob hash before decoding and
//! constrain the byte slice with its selected physical source profile.

use std::fmt;
use std::io::{self, Write};

use serde::de::{DeserializeSeed, Error as _, MapAccess, SeqAccess, Visitor};

use crate::manifest::MAX_ENTRIES;
use crate::record::canonical_uuid;
use crate::{ManifestEntry, ManifestError, RecordDescriptor, SourcePath, TreeManifest};

#[derive(Debug)]
pub(crate) enum ManifestDecodeError {
    TooLarge,
    Json(serde_json::Error),
    Invalid(ManifestError),
    NonCanonical,
}

impl fmt::Display for ManifestDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge => formatter.write_str("stored manifest exceeds selected byte profile"),
            Self::Json(error) => write!(formatter, "stored manifest JSON refused: {error}"),
            Self::Invalid(error) => write!(formatter, "stored manifest invalid: {error:?}"),
            Self::NonCanonical => formatter.write_str("stored manifest is not canonical"),
        }
    }
}

/// Decode only a bounded, authenticated manifest blob from a selected store profile.
/// The final byte comparison rejects alternate JSON spelling, order and whitespace.
pub(crate) fn decode_manifest_jcs(
    bytes: &[u8],
    maximum_bytes: usize,
) -> Result<TreeManifest, ManifestDecodeError> {
    if bytes.len() > maximum_bytes {
        return Err(ManifestDecodeError::TooLarge);
    }
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    let raw = ManifestSeed
        .deserialize(&mut decoder)
        .map_err(ManifestDecodeError::Json)?;
    decoder.end().map_err(ManifestDecodeError::Json)?;
    let manifest = TreeManifest::new(&raw.project_id, raw.dependency_roots, raw.files)
        .map_err(ManifestDecodeError::Invalid)?;
    let mut compare = CompareWriter { bytes, offset: 0 };
    manifest
        .write_jcs(&mut compare)
        .map_err(|_| ManifestDecodeError::NonCanonical)?;
    if compare.offset != bytes.len() {
        return Err(ManifestDecodeError::NonCanonical);
    }
    Ok(manifest)
}

struct CompareWriter<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl Write for CompareWriter<'_> {
    fn write(&mut self, chunk: &[u8]) -> io::Result<usize> {
        let end = self
            .offset
            .checked_add(chunk.len())
            .ok_or_else(|| io::Error::other("manifest comparison overflow"))?;
        if self.bytes.get(self.offset..end) != Some(chunk) {
            return Err(io::Error::other("noncanonical manifest bytes"));
        }
        self.offset = end;
        Ok(chunk.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct RawManifest {
    project_id: String,
    dependency_roots: Vec<String>,
    files: Vec<ManifestEntry>,
}

struct ManifestSeed;

impl<'de> DeserializeSeed<'de> for ManifestSeed {
    type Value = RawManifest;

    fn deserialize<D: serde::Deserializer<'de>>(self, decoder: D) -> Result<Self::Value, D::Error> {
        decoder.deserialize_map(ManifestVisitor)
    }
}

struct ManifestVisitor;

impl<'de> Visitor<'de> for ManifestVisitor {
    type Value = RawManifest;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a canonical v1 source manifest")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut project_id = None;
        let mut dependency_roots = None;
        let mut files = None;
        let mut format = None;
        while let Some(key) = map.next_key::<&str>()? {
            match key {
                "projectId" if project_id.is_none() => {
                    project_id = Some(map.next_value::<String>()?)
                }
                "dependencyRoots" if dependency_roots.is_none() => {
                    dependency_roots = Some(map.next_value_seed(EmptyRootsSeed)?)
                }
                "files" if files.is_none() => files = Some(map.next_value_seed(FilesSeed)?),
                "format" if format.is_none() => format = Some(map.next_value::<u64>()?),
                _ => return Err(A::Error::custom("duplicate or unknown manifest key")),
            }
        }
        if format != Some(1) {
            return Err(A::Error::custom("unsupported manifest format"));
        }
        Ok(RawManifest {
            project_id: project_id.ok_or_else(|| A::Error::custom("missing projectId"))?,
            dependency_roots: dependency_roots
                .ok_or_else(|| A::Error::custom("missing dependencyRoots"))?,
            files: files.ok_or_else(|| A::Error::custom("missing files"))?,
        })
    }
}

struct FilesSeed;

impl<'de> DeserializeSeed<'de> for FilesSeed {
    type Value = Vec<ManifestEntry>;

    fn deserialize<D: serde::Deserializer<'de>>(self, decoder: D) -> Result<Self::Value, D::Error> {
        decoder.deserialize_seq(FilesVisitor)
    }
}

struct FilesVisitor;

impl<'de> Visitor<'de> for FilesVisitor {
    type Value = Vec<ManifestEntry>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("at most 100000 source files")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut files = Vec::with_capacity(seq.size_hint().unwrap_or(0).min(MAX_ENTRIES));
        while files.len() < MAX_ENTRIES {
            let Some(file) = seq.next_element_seed(FileSeed)? else {
                return Ok(files);
            };
            files.push(file);
        }
        let _: Option<()> = seq.next_element_seed(RejectExtra("too many manifest files"))?;
        Ok(files)
    }
}

struct EmptyRootsSeed;

impl<'de> DeserializeSeed<'de> for EmptyRootsSeed {
    type Value = Vec<String>;

    fn deserialize<D: serde::Deserializer<'de>>(self, decoder: D) -> Result<Self::Value, D::Error> {
        decoder.deserialize_seq(EmptyRootsVisitor)
    }
}

struct EmptyRootsVisitor;

impl<'de> Visitor<'de> for EmptyRootsVisitor {
    type Value = Vec<String>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("no dependency roots in the first source profile")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let _: Option<()> = seq.next_element_seed(RejectExtra(
            "first source profile does not support dependency roots",
        ))?;
        Ok(Vec::new())
    }
}

struct RejectExtra(&'static str);

impl<'de> DeserializeSeed<'de> for RejectExtra {
    type Value = ();

    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        _decoder: D,
    ) -> Result<Self::Value, D::Error> {
        Err(D::Error::custom(self.0))
    }
}

struct FileSeed;

impl<'de> DeserializeSeed<'de> for FileSeed {
    type Value = ManifestEntry;

    fn deserialize<D: serde::Deserializer<'de>>(self, decoder: D) -> Result<Self::Value, D::Error> {
        decoder.deserialize_map(FileVisitor)
    }
}

struct FileVisitor;

impl<'de> Visitor<'de> for FileVisitor {
    type Value = ManifestEntry;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a canonical source file entry")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut hash = None;
        let mut kind = None;
        let mut length = None;
        let mut path = None;
        let mut record_id = None;
        let mut record_type = None;
        while let Some(key) = map.next_key::<&str>()? {
            match key {
                "hash" if hash.is_none() => {
                    let value = map.next_value::<&str>()?;
                    hash =
                        Some(parse_lower_hex(value).ok_or_else(|| {
                            A::Error::custom("file hash must be lowercase SHA-256")
                        })?);
                }
                "kind" if kind.is_none() => kind = Some(map.next_value::<&str>()?),
                "length" if length.is_none() => length = Some(map.next_value::<u64>()?),
                "path" if path.is_none() => path = Some(map.next_value::<&str>()?),
                "recordId" if record_id.is_none() => {
                    record_id = Some(map.next_value::<Option<String>>()?)
                }
                "recordType" if record_type.is_none() => {
                    record_type = Some(map.next_value::<Option<String>>()?)
                }
                _ => return Err(A::Error::custom("duplicate or unknown file entry key")),
            }
        }
        if kind != Some("file") {
            return Err(A::Error::custom("unsupported source entry kind"));
        }
        let pair_present = record_id.is_some() && record_type.is_some();
        let record = match (record_id.flatten(), record_type.flatten()) {
            (None, None) if pair_present => None,
            (Some(id), Some(record_type))
                if canonical_uuid(&id) && !record_type.is_empty() && record_type.is_ascii() =>
            {
                Some(RecordDescriptor::checked_owned_parts(id, record_type))
            }
            _ => return Err(A::Error::custom("invalid record identity pair")),
        };
        let path = SourcePath::parse(path.ok_or_else(|| A::Error::custom("missing path"))?)
            .map_err(|error| A::Error::custom(format!("invalid file path: {error:?}")))?;
        ManifestEntry::new(
            path,
            length.ok_or_else(|| A::Error::custom("missing length"))?,
            hash.ok_or_else(|| A::Error::custom("missing hash"))?,
            record,
        )
        .map_err(|error| A::Error::custom(format!("invalid file entry: {error:?}")))
    }
}

fn parse_lower_hex(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64 {
        return None;
    }
    let mut hash = [0_u8; 32];
    for (index, pair) in value.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        let high = hex_digit(pair[0])?;
        let low = hex_digit(pair[1])?;
        hash[index] = (high << 4) | low;
    }
    Some(hash)
}

fn hex_digit(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::Digest;

    const PROJECT_ID: &str = "2a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    const RECORD_ID: &str = "82b0ec04-6866-4271-a2cf-1f1576ad604a";

    fn fixture() -> TreeManifest {
        TreeManifest::new(
            PROJECT_ID,
            Vec::new(),
            vec![
                ManifestEntry::new(
                    SourcePath::parse("src/a.rs").unwrap(),
                    7,
                    [0xab; 32],
                    Some(RecordDescriptor::checked_parts(RECORD_ID, "core.scene")),
                )
                .unwrap(),
                ManifestEntry::new(SourcePath::parse("src/b.rs").unwrap(), 9, [0xcd; 32], None)
                    .unwrap(),
            ],
        )
        .unwrap()
    }

    #[test]
    fn canonical_manifest_decodes_without_a_generic_json_tree() {
        let original = fixture();
        let mut bytes = Vec::new();
        original.write_jcs(&mut bytes).unwrap();
        let decoded = decode_manifest_jcs(&bytes, bytes.len()).unwrap();
        assert_eq!(decoded.revision(), original.revision());
        assert_eq!(decoded.files().len(), 2);
        assert_eq!(decoded.files()[0].blob_identity(), (7, [0xab; 32]));
        assert!(matches!(
            decode_manifest_jcs(&bytes, bytes.len() - 1),
            Err(ManifestDecodeError::TooLarge)
        ));
    }

    #[test]
    fn alternate_spelling_and_invalid_entries_refuse() {
        let mut bytes = Vec::new();
        fixture().write_jcs(&mut bytes).unwrap();
        let canonical = String::from_utf8(bytes).unwrap();
        for changed in [
            format!(" {canonical}"),
            canonical.replacen("\"dependencyRoots\":[]", "\"dependencyRoots\":[\"x\"]", 1),
            canonical.replace("src/a.rs", "src/z.rs"),
            canonical.replace("abababab", "ABABABAB"),
            canonical.replacen("\"format\":1", "\"format\":1,\"format\":1", 1),
            canonical.replacen("\"kind\":\"file\"", "\"kind\":\"directory\"", 1),
            canonical.replacen("\"recordType\":\"core.scene\"", "\"recordType\":null", 1),
        ] {
            assert!(
                decode_manifest_jcs(changed.as_bytes(), changed.len()).is_err(),
                "accepted {changed}"
            );
        }
    }

    #[test]
    fn emitted_escaped_ascii_record_type_round_trips() {
        let manifest = TreeManifest::new(
            PROJECT_ID,
            Vec::new(),
            vec![
                ManifestEntry::new(
                    SourcePath::parse("src/escaped.rs").unwrap(),
                    1,
                    [5; 32],
                    Some(RecordDescriptor::checked_parts(RECORD_ID, "game.\"quoted")),
                )
                .unwrap(),
            ],
        )
        .unwrap();
        let mut bytes = Vec::new();
        manifest.write_jcs(&mut bytes).unwrap();
        let decoded = decode_manifest_jcs(&bytes, bytes.len()).unwrap();
        assert_eq!(decoded.revision(), manifest.revision());
    }

    #[test]
    #[ignore = "local maximum-manifest decode cost and memory probe"]
    fn measure_maximum_stored_manifest_decode() {
        let files = (0_u32..100_000)
            .map(|index| {
                ManifestEntry::new(
                    SourcePath::parse(&format!("src/file-{index:06}.bin")).unwrap(),
                    1,
                    sha2::Sha256::digest(index.to_le_bytes()).into(),
                    None,
                )
                .unwrap()
            })
            .collect();
        let manifest = TreeManifest::new(PROJECT_ID, Vec::new(), files).unwrap();
        let mut bytes = Vec::new();
        manifest.write_jcs(&mut bytes).unwrap();
        let expected = manifest.revision();
        let start = std::time::Instant::now();
        let decoded = decode_manifest_jcs(&bytes, 64 * 1024 * 1024).unwrap();
        let elapsed = start.elapsed();
        assert_eq!(decoded.revision(), expected);
        eprintln!(
            "maximum_stored_manifest_decode entries=100000 bytes={} elapsed_ms={:.3}",
            bytes.len(),
            elapsed.as_secs_f64() * 1000.0
        );
    }
}
