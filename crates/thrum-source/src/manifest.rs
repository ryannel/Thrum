//! Deterministic digest of an exact-byte source-tree description.
//!
//! This pure representation is not an accepted source root. F3 semantic
//! validation and the native durable root transaction are separate gates.

use std::collections::HashSet;
use std::convert::Infallible;
use std::io::{self, Write};

use sha2::{Digest, Sha256};

use crate::path::{PathError, SourcePath, validate_path_refs};
use crate::record::{RecordEnvelope, canonical_uuid};

pub(crate) const MAX_ENTRIES: usize = 100_000;
const MAX_BLOB_BYTES: u64 = 16 * 1024 * 1024 * 1024;
const HEX: &[u8; 16] = b"0123456789abcdef";

/// SHA-256 of the RFC 8785 JCS form of one v1 tree manifest.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceRevision([u8; 32]);

impl SourceRevision {
    pub(crate) const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
    /// Raw digest bytes for typed storage or transport adapters.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Root record identity supplied after common envelope validation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordDescriptor {
    id: String,
    record_type: String,
}

impl From<RecordEnvelope<'_>> for RecordDescriptor {
    fn from(value: RecordEnvelope<'_>) -> Self {
        Self {
            id: value.id().to_owned(),
            record_type: value.record_type().to_owned(),
        }
    }
}

impl RecordDescriptor {
    pub(crate) fn checked_parts(id: &str, record_type: &str) -> Self {
        debug_assert!(canonical_uuid(id));
        Self {
            id: id.to_owned(),
            record_type: record_type.to_owned(),
        }
    }

    pub(crate) fn checked_owned_parts(id: String, record_type: String) -> Self {
        debug_assert!(canonical_uuid(&id));
        Self { id, record_type }
    }
}

/// One file's exact-byte metadata; a capture adapter must verify the bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManifestEntry {
    path: SourcePath,
    length: u64,
    sha256: [u8; 32],
    record: Option<RecordDescriptor>,
}

impl ManifestEntry {
    /// Portable path in the admitted tree.
    pub fn path(&self) -> &SourcePath {
        &self.path
    }

    /// Exact byte length and SHA-256 of the admitted file.
    pub fn content_identity(&self) -> (u64, [u8; 32]) {
        (self.length, self.sha256)
    }

    /// Describe one file whose bytes and optional F3 envelope were checked.
    pub fn new(
        path: SourcePath,
        length: u64,
        sha256: [u8; 32],
        record: Option<RecordDescriptor>,
    ) -> Result<Self, ManifestError> {
        if length > MAX_BLOB_BYTES {
            return Err(ManifestError::BlobTooLarge);
        }
        Ok(Self {
            path,
            length,
            sha256,
            record,
        })
    }

    pub(crate) fn blob_identity(&self) -> (u64, [u8; 32]) {
        (self.length, self.sha256)
    }

    /// Validated stable record ID and type, when this file is a record.
    pub fn record_identity(&self) -> Option<(&str, &str)> {
        self.record
            .as_ref()
            .map(|record| (record.id.as_str(), record.record_type.as_str()))
    }

    #[cfg(test)]
    pub(crate) fn source_path(&self) -> &SourcePath {
        &self.path
    }
}

/// A tree description cannot have a portable v1 identity.
#[derive(Debug)]
pub enum ManifestError {
    /// Project ID is not a canonical lowercase UUID.
    ProjectId,
    /// Too many file entries for the v1 source profile.
    TooManyEntries,
    /// One file exceeds the v1 immutable blob bound.
    BlobTooLarge,
    /// File paths are not portable and unambiguous.
    Path(PathError),
    /// Two root records share a durable ID.
    DuplicateRecordId,
    /// Two declared dependency roots are identical.
    DuplicateDependencyRoot,
}

/// V1 source metadata sorted by UTF-8 byte order, before durable admission.
pub struct TreeManifest {
    project_id: String,
    dependency_roots: Vec<String>,
    files: Vec<ManifestEntry>,
}

impl TreeManifest {
    /// Files in portable path order.
    pub fn files(&self) -> &[ManifestEntry] {
        &self.files
    }

    /// Stable project identity carried by this revision.
    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    /// Declared dependency roots (empty in the first native store profile).
    pub fn dependency_roots(&self) -> &[String] {
        &self.dependency_roots
    }

    /// Validate identity and sort paths and roots by UTF-8 bytes without
    /// reading mutable files. The byte-order rule is part of v1 revision
    /// identity and is independent of a target's locale or string collation.
    ///
    /// F3 must validate the meaning and closure of `dependency_roots` before
    /// this manifest can describe a candidate for durable admission.
    pub fn new(
        project_id: &str,
        mut dependency_roots: Vec<String>,
        mut files: Vec<ManifestEntry>,
    ) -> Result<Self, ManifestError> {
        if !canonical_uuid(project_id) {
            return Err(ManifestError::ProjectId);
        }
        if files.len() > MAX_ENTRIES {
            return Err(ManifestError::TooManyEntries);
        }
        validate_path_refs(files.iter().map(|entry| &entry.path)).map_err(ManifestError::Path)?;
        let mut ids = HashSet::new();
        if files
            .iter()
            .filter_map(|entry| entry.record.as_ref())
            .any(|record| !ids.insert(record.id.as_str()))
        {
            return Err(ManifestError::DuplicateRecordId);
        }
        dependency_roots.sort_unstable();
        if dependency_roots.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(ManifestError::DuplicateDependencyRoot);
        }
        files.sort_unstable_by(|left, right| left.path.cmp(&right.path));
        Ok(Self {
            project_id: project_id.to_owned(),
            dependency_roots,
            files,
        })
    }

    /// Hash the exact v1 manifest representation in JCS key order.
    ///
    /// The known object keys and already-sorted arrays stream directly to
    /// SHA-256. A separate JCS library checks the emitted form in tests.
    pub fn revision(&self) -> SourceRevision {
        let mut digest = Sha256::new();
        self.emit_jcs(&mut digest)
            .expect("SHA-256 sink is infallible");
        SourceRevision(digest.finalize().into())
    }

    /// Stream the same canonical manifest bytes used by `revision` to a writer.
    ///
    /// This avoids constructing a second whole-manifest byte vector when a
    /// native store writes the manifest in bounded chunks. A failed writer
    /// returns its I/O error; it does not create an admitted source root.
    pub fn write_jcs(&self, writer: &mut impl Write) -> io::Result<()> {
        self.emit_jcs(&mut IoSink(writer))
    }

    fn emit_jcs<S: ManifestSink>(&self, sink: &mut S) -> Result<(), S::Error> {
        sink.emit(b"{\"dependencyRoots\":[")?;
        for (index, root) in self.dependency_roots.iter().enumerate() {
            if index != 0 {
                sink.emit(b",")?;
            }
            emit_jcs_string(sink, root)?;
        }
        sink.emit(b"],\"files\":[")?;
        for (index, file) in self.files.iter().enumerate() {
            if index != 0 {
                sink.emit(b",")?;
            }
            sink.emit(b"{\"hash\":\"")?;
            emit_hex(sink, &file.sha256)?;
            sink.emit(b"\",\"kind\":\"file\",\"length\":")?;
            emit_decimal(sink, file.length)?;
            sink.emit(b",\"path\":")?;
            emit_jcs_string(sink, file.path.as_str())?;
            sink.emit(b",\"recordId\":")?;
            if let Some(record) = &file.record {
                emit_jcs_string(sink, &record.id)?;
            } else {
                sink.emit(b"null")?;
            }
            sink.emit(b",\"recordType\":")?;
            if let Some(record) = &file.record {
                emit_jcs_string(sink, &record.record_type)?;
            } else {
                sink.emit(b"null")?;
            }
            sink.emit(b"}")?;
        }
        sink.emit(b"],\"format\":1,\"projectId\":")?;
        emit_jcs_string(sink, &self.project_id)?;
        sink.emit(b"}")
    }
}

trait ManifestSink {
    type Error;

    fn emit(&mut self, bytes: &[u8]) -> Result<(), Self::Error>;
}

impl ManifestSink for Sha256 {
    type Error = Infallible;

    fn emit(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        self.update(bytes);
        Ok(())
    }
}

struct IoSink<'a, W: Write>(&'a mut W);

impl<W: Write> ManifestSink for IoSink<'_, W> {
    type Error = io::Error;

    fn emit(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        self.0.write_all(bytes)
    }
}

pub(crate) fn write_jcs_string(digest: &mut Sha256, value: &str) {
    emit_jcs_string(digest, value).expect("SHA-256 sink is infallible");
}

fn emit_jcs_string<S: ManifestSink>(sink: &mut S, value: &str) -> Result<(), S::Error> {
    sink.emit(b"\"")?;
    let bytes = value.as_bytes();
    let mut segment_start = 0;
    for (index, byte) in bytes.iter().copied().enumerate() {
        let escape: &[u8] = match byte {
            b'"' => b"\\\"",
            b'\\' => b"\\\\",
            b'\x08' => b"\\b",
            b'\t' => b"\\t",
            b'\n' => b"\\n",
            b'\x0c' => b"\\f",
            b'\r' => b"\\r",
            0..=0x1f => b"",
            _ => continue,
        };
        sink.emit(&bytes[segment_start..index])?;
        if escape.is_empty() {
            sink.emit(b"\\u00")?;
            sink.emit(&[HEX[usize::from(byte >> 4)], HEX[usize::from(byte & 0x0f)]])?;
        } else {
            sink.emit(escape)?;
        }
        segment_start = index + 1;
    }
    sink.emit(&bytes[segment_start..])?;
    sink.emit(b"\"")
}

pub(crate) fn write_decimal(digest: &mut Sha256, value: u64) {
    emit_decimal(digest, value).expect("SHA-256 sink is infallible");
}

fn emit_decimal<S: ManifestSink>(sink: &mut S, mut value: u64) -> Result<(), S::Error> {
    let mut digits = [0_u8; 20];
    let mut start = digits.len();
    loop {
        start -= 1;
        digits[start] = b'0' + (value % 10) as u8;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    sink.emit(&digits[start..])
}

pub(crate) fn write_hex(digest: &mut Sha256, bytes: &[u8; 32]) {
    emit_hex(digest, bytes).expect("SHA-256 sink is infallible");
}

fn emit_hex<S: ManifestSink>(sink: &mut S, bytes: &[u8; 32]) -> Result<(), S::Error> {
    let mut encoded = [0_u8; 64];
    for (index, byte) in bytes.iter().copied().enumerate() {
        encoded[index * 2] = HEX[usize::from(byte >> 4)];
        encoded[index * 2 + 1] = HEX[usize::from(byte & 0x0f)];
    }
    sink.emit(&encoded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const PROJECT_ID: &str = "2a5ef644-7225-4770-9f4d-e1d9c0a7d515";

    fn file(path: &str, bytes: &[u8]) -> ManifestEntry {
        ManifestEntry::new(
            SourcePath::parse(path).unwrap(),
            bytes.len() as u64,
            Sha256::digest(bytes).into(),
            None,
        )
        .unwrap()
    }

    fn hex(bytes: &[u8]) -> String {
        use std::fmt::Write;

        let mut result = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            write!(&mut result, "{byte:02x}").unwrap();
        }
        result
    }

    fn digest_hex(bytes: &[u8]) -> String {
        hex(&Sha256::digest(bytes))
    }

    #[test]
    fn exact_bytes_and_declared_roots_have_one_order_independent_revision() {
        let files = vec![
            file("src/game.rs", b"fn game() {}"),
            file("scene.json", b"{}"),
        ];
        let manifest = TreeManifest::new(
            PROJECT_ID,
            vec!["z".to_owned(), "a".to_owned()],
            files.clone(),
        )
        .unwrap();
        let reversed = TreeManifest::new(
            PROJECT_ID,
            vec!["a".to_owned(), "z".to_owned()],
            files.into_iter().rev().collect(),
        )
        .unwrap();
        assert_eq!(manifest.revision(), reversed.revision());
        let changed = TreeManifest::new(
            PROJECT_ID,
            vec!["a".to_owned(), "z".to_owned()],
            vec![
                file("scene.json", b"{ }"),
                file("src/game.rs", b"fn game() {}"),
            ],
        )
        .unwrap();
        assert_ne!(manifest.revision(), changed.revision());
        let changed_roots = TreeManifest::new(
            PROJECT_ID,
            vec!["a".to_owned()],
            vec![
                file("scene.json", b"{}"),
                file("src/game.rs", b"fn game() {}"),
            ],
        )
        .unwrap();
        assert_ne!(manifest.revision(), changed_roots.revision());

        let oracle = json!({
            "dependencyRoots": ["a", "z"],
            "files": [
                {"hash": digest_hex(b"{}"), "kind":"file", "length":2, "path":"scene.json", "recordId":null, "recordType":null},
                {"hash": digest_hex(b"fn game() {}"), "kind":"file", "length":12, "path":"src/game.rs", "recordId":null, "recordType":null}
            ],
            "format": 1,
            "projectId": PROJECT_ID
        });
        let oracle_bytes = serde_json_canonicalizer::to_vec(&oracle).unwrap();
        let mut written = Vec::new();
        manifest.write_jcs(&mut written).unwrap();
        assert_eq!(written, oracle_bytes);
        assert_eq!(
            manifest.revision().0.as_slice(),
            Sha256::digest(&written).as_slice()
        );
        assert_eq!(
            hex(manifest.revision().as_bytes()),
            "3c0f7abb9265d184f2a9fa1f2f5565d8a4b43acd10b0ea002386f9f31653d1ff"
        );
    }

    #[test]
    fn canonical_manifest_writer_propagates_output_failure() {
        struct FailingWriter {
            remaining: usize,
        }

        impl Write for FailingWriter {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                if self.remaining == 0 {
                    return Err(io::Error::new(io::ErrorKind::OutOfMemory, "writer refused"));
                }
                let count = bytes.len().min(self.remaining);
                self.remaining -= count;
                Ok(count)
            }

            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let manifest = TreeManifest::new(PROJECT_ID, Vec::new(), vec![file("a", b"x")]).unwrap();
        let mut writer = FailingWriter { remaining: 17 };
        assert_eq!(
            manifest.write_jcs(&mut writer).unwrap_err().kind(),
            io::ErrorKind::OutOfMemory
        );
    }

    #[test]
    fn duplicate_identity_and_path_alias_refuse_manifest() {
        let duplicate_path = TreeManifest::new(
            PROJECT_ID,
            vec![],
            vec![file("Assets/a", b"1"), file("assets/a", b"2")],
        );
        assert!(matches!(duplicate_path, Err(ManifestError::Path(_))));
        assert!(matches!(
            TreeManifest::new(PROJECT_ID, vec!["same".into(), "same".into()], vec![]),
            Err(ManifestError::DuplicateDependencyRoot)
        ));
        let mut first = file("a.scene.json", b"{}");
        first.record = Some(RecordDescriptor {
            id: PROJECT_ID.to_owned(),
            record_type: "core.scene".to_owned(),
        });
        let mut second = file("b.scene.json", b"{}");
        second.record = first.record.clone();
        assert!(matches!(
            TreeManifest::new(PROJECT_ID, vec![], vec![first, second]),
            Err(ManifestError::DuplicateRecordId)
        ));
    }

    #[test]
    fn streamed_jcs_primitives_match_independent_serializer() {
        let mut cases = vec![
            String::new(),
            "quote\" slash\\ line\n".to_owned(),
            "café/音/😀/\u{2028}/\u{2029}".to_owned(),
        ];
        for scalar in 0..=0x1f {
            cases.push(format!("left{}right", char::from_u32(scalar).unwrap()));
        }
        for value in cases {
            let mut digest = Sha256::new();
            write_jcs_string(&mut digest, &value);
            let oracle = serde_json_canonicalizer::to_vec(&value).unwrap();
            assert_eq!(
                digest.finalize().as_slice(),
                Sha256::digest(oracle).as_slice()
            );
        }
        for value in [0, 1, 9, 10, 4_294_967_295, MAX_BLOB_BYTES] {
            let mut digest = Sha256::new();
            write_decimal(&mut digest, value);
            let oracle = serde_json_canonicalizer::to_vec(&value).unwrap();
            assert_eq!(
                digest.finalize().as_slice(),
                Sha256::digest(oracle).as_slice()
            );
        }
    }

    #[test]
    #[ignore = "local 100k-entry metadata and revision cost probe"]
    fn measure_maximum_manifest_metadata_cost() {
        let start = std::time::Instant::now();
        let files = (0..MAX_ENTRIES)
            .map(|index| file(&format!("scenes/{index:06}.scene.json"), b"{}"))
            .collect();
        let generated = start.elapsed();
        let manifest = TreeManifest::new(PROJECT_ID, vec!["game-code".to_owned()], files).unwrap();
        let validated = start.elapsed();
        std::hint::black_box(manifest.revision());
        let hashed = start.elapsed();
        eprintln!(
            "entries={MAX_ENTRIES} generation={generated:?} validate_sort={:?} jcs_sha256={:?} entry_struct_bytes={}",
            validated - generated,
            hashed - validated,
            std::mem::size_of::<ManifestEntry>() * MAX_ENTRIES
        );
    }

    #[test]
    #[ignore = "local 100k-entry canonical manifest writer cost probe"]
    fn measure_maximum_manifest_writer() {
        struct HashingWriter {
            digest: Sha256,
            length: usize,
        }

        impl Write for HashingWriter {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                self.digest.update(bytes);
                self.length += bytes.len();
                Ok(bytes.len())
            }

            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let files = (0..MAX_ENTRIES)
            .map(|index| file(&format!("scenes/{index:06}.scene.json"), b"{}"))
            .collect();
        let manifest = TreeManifest::new(PROJECT_ID, vec!["game-code".to_owned()], files).unwrap();
        let revision = manifest.revision();
        let mut writer = HashingWriter {
            digest: Sha256::new(),
            length: 0,
        };
        let start = std::time::Instant::now();
        manifest.write_jcs(&mut writer).unwrap();
        let elapsed = start.elapsed();
        assert_eq!(writer.digest.finalize().as_slice(), revision.as_bytes());
        eprintln!(
            "entries={MAX_ENTRIES} jcs_bytes={} write_elapsed={elapsed:?}",
            writer.length
        );
    }
}
