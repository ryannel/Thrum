//! Isolated SQLite transaction proof for the later native source store.
//!
//! Test-only: the fixtures do not bind a fully validated candidate, physical
//! reservation or GC pins, so this is not an accepted-source admission API.

use std::fmt;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use rusqlite::types::ValueRef;
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use sha2::{Digest, Sha256};

use crate::budget::Limits;
use crate::history_json::{
    CarriedHistoryError, HistoryError, compare_carried_histories, parse_registry_history,
};
use crate::manifest_json::{ManifestDecodeError, decode_manifest_jcs};
use crate::registry_stage::{StagedCandidate, StagedRegistryError, validate_staged_candidate};
use crate::stage::stage_declared_tree_with;
use crate::{ManifestEntry, SourcePath, TreeManifest};

const MAX_PAGE_COUNT: i64 = 13_631_488;
const CHUNK_BYTES: usize = 4 * 1024 * 1024;
const FIRST_PROFILE_MANIFEST_BYTES: usize = 64 * 1024 * 1024;
const MAX_HISTORY_BYTES: usize = 16 * 1024 * 1024;
const SMALL_BATCH_FILES: usize = 128;
const SMALL_BATCH_BYTES: usize = 1024 * 1024;
static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "thrum-sqlite-root-proof-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn database(&self) -> PathBuf {
        self.0.join("source.sqlite")
    }

    fn source_root(&self) -> PathBuf {
        self.0.join("source")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[derive(Debug)]
enum ProofError {
    Sql(rusqlite::Error),
    InputIo,
    OutputIo,
    InputLength,
    HashMismatch,
    MissingChunk,
    InvalidChunkLayout,
    MissingCompleteBlob,
    MissingCompleteFile,
    BlobMetadataMismatch,
    StaleBlobWriter,
    StaleGeneration,
    TokenConflict,
    MissingRootHistory,
    AmbiguousRootHistory,
    DifferentProject,
    UnsupportedDependencyRoots,
    CorruptRootBlob,
    Manifest(ManifestDecodeError),
    History(HistoryError),
    Lineage(CarriedHistoryError),
    CandidateStage(StagedRegistryError),
}

impl fmt::Display for ProofError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sql(error) => write!(formatter, "SQLite: {error}"),
            Self::Manifest(error) => write!(formatter, "manifest: {error}"),
            Self::History(error) => write!(formatter, "history: {error:?}"),
            Self::Lineage(error) => write!(formatter, "lineage: {error:?}"),
            Self::CandidateStage(error) => write!(formatter, "candidate stage: {error:?}"),
            other => write!(formatter, "{other:?}"),
        }
    }
}

impl From<rusqlite::Error> for ProofError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sql(value)
    }
}

struct RootTransactionProof {
    conn: Connection,
    scratch: Vec<u8>,
}

fn blob_chunk_count(length: u64) -> Result<i64, ProofError> {
    i64::try_from(length.div_ceil(CHUNK_BYTES as u64)).map_err(|_| ProofError::InputLength)
}

fn verify_supplied_blob(
    input: &mut impl Read,
    length: u64,
    expected_hash: [u8; 32],
    buffer: &mut [u8],
) -> Result<(), ProofError> {
    let mut digest = Sha256::new();
    let mut remaining = length;
    while remaining > 0 {
        let count = remaining.min(buffer.len() as u64) as usize;
        input
            .read_exact(&mut buffer[..count])
            .map_err(|_| ProofError::InputIo)?;
        digest.update(&buffer[..count]);
        remaining -= count as u64;
    }
    let mut extra = [0_u8; 1];
    if input.read(&mut extra).map_err(|_| ProofError::InputIo)? != 0 {
        return Err(ProofError::InputLength);
    }
    if digest.finalize().as_slice() != expected_hash {
        return Err(ProofError::HashMismatch);
    }
    Ok(())
}

fn root_identity(tx: &Transaction<'_>) -> Result<(i64, Option<[u8; 32]>), ProofError> {
    let (generation, revision) = tx.query_row(
        "SELECT generation, revision FROM root WHERE id=1",
        [],
        |row| {
            let revision = match row.get_ref(1)? {
                ValueRef::Null => Some(None),
                ValueRef::Blob(bytes) => bytes.try_into().ok().map(Some),
                _ => None,
            };
            Ok((row.get(0)?, revision))
        },
    )?;
    Ok((generation, revision.ok_or(ProofError::CorruptRootBlob)?))
}

fn read_complete_blob(
    tx: &Transaction<'_>,
    hash: [u8; 32],
    maximum_bytes: usize,
) -> Result<Vec<u8>, ProofError> {
    let (stored_length, stored_chunks, state): (i64, i64, i64) = tx
        .query_row(
            "SELECT length, chunk_count, state FROM blob_header WHERE hash=?1",
            params![hash.as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?
        .ok_or(ProofError::MissingCompleteBlob)?;
    let length = usize::try_from(stored_length).map_err(|_| ProofError::CorruptRootBlob)?;
    if state != 1 || length > maximum_bytes || stored_chunks != blob_chunk_count(length as u64)? {
        return Err(ProofError::CorruptRootBlob);
    }
    let actual_chunks: i64 = tx.query_row(
        "SELECT count(*) FROM blob_chunks WHERE hash=?1",
        params![hash.as_slice()],
        |row| row.get(0),
    )?;
    if actual_chunks != stored_chunks {
        return Err(ProofError::CorruptRootBlob);
    }
    let mut bytes = Vec::with_capacity(length);
    let mut digest = Sha256::new();
    for index in 0..stored_chunks {
        let expected = (length - bytes.len()).min(CHUNK_BYTES);
        let actual: i64 = tx
            .query_row(
                "SELECT length(bytes) FROM blob_chunks WHERE hash=?1 AND chunk_index=?2",
                params![hash.as_slice(), index],
                |row| row.get(0),
            )
            .optional()?
            .ok_or(ProofError::CorruptRootBlob)?;
        if actual != expected as i64 {
            return Err(ProofError::CorruptRootBlob);
        }
        let chunk: Vec<u8> = tx
            .query_row(
                "SELECT bytes FROM blob_chunks WHERE hash=?1 AND chunk_index=?2",
                params![hash.as_slice(), index],
                |row| row.get(0),
            )
            .optional()?
            .ok_or(ProofError::CorruptRootBlob)?;
        digest.update(&chunk);
        bytes.extend_from_slice(&chunk);
    }
    if bytes.len() != length || digest.finalize().as_slice() != hash {
        return Err(ProofError::CorruptRootBlob);
    }
    Ok(bytes)
}

struct HashCountWriter {
    digest: Sha256,
    length: u64,
}

impl Write for HashCountWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.length = self
            .length
            .checked_add(bytes.len() as u64)
            .ok_or_else(|| io::Error::other("manifest length overflow"))?;
        self.digest.update(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct ManifestBlobWriter<'a> {
    store: &'a mut RootTransactionProof,
    buffer: Vec<u8>,
    digest: Sha256,
    length: u64,
    written: u64,
    hash: [u8; 32],
    chunk_count: i64,
    next_chunk: i64,
    failure: Option<ProofError>,
}

impl ManifestBlobWriter<'_> {
    fn append_full_chunk(&mut self) -> io::Result<()> {
        if let Err(error) = self.store.append_blob_chunk(
            self.length,
            self.hash,
            self.chunk_count,
            self.next_chunk,
            &self.buffer,
        ) {
            self.failure = Some(error);
            return Err(io::Error::other("SQLite blob chunk write failed"));
        }
        self.next_chunk += 1;
        self.buffer.clear();
        Ok(())
    }

    fn finish(mut self) -> Result<(), ProofError> {
        if let Some(error) = self.failure.take() {
            return Err(error);
        }
        if self.written != self.length {
            return Err(ProofError::InputLength);
        }
        if !self.buffer.is_empty() {
            self.append_full_chunk()
                .map_err(|_| self.failure.take().unwrap_or(ProofError::OutputIo))?;
        }
        if self.next_chunk != self.chunk_count {
            return Err(ProofError::InvalidChunkLayout);
        }
        if self.digest.finalize().as_slice() != self.hash {
            return Err(ProofError::HashMismatch);
        }
        self.store
            .verify_and_complete_blob(self.length, self.hash, self.chunk_count, || {})
    }
}

impl Write for ManifestBlobWriter<'_> {
    fn write(&mut self, mut bytes: &[u8]) -> io::Result<usize> {
        if self.failure.is_some() {
            return Err(io::Error::other("previous SQLite blob write failed"));
        }
        let count = bytes.len();
        let next = self
            .written
            .checked_add(count as u64)
            .filter(|next| *next <= self.length)
            .ok_or_else(|| io::Error::other("manifest exceeded declared length"))?;
        self.digest.update(bytes);
        while !bytes.is_empty() {
            let take = bytes.len().min(CHUNK_BYTES - self.buffer.len());
            self.buffer.extend_from_slice(&bytes[..take]);
            bytes = &bytes[take..];
            if self.buffer.len() == CHUNK_BYTES {
                self.append_full_chunk()?;
            }
        }
        self.written = next;
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        // Partial chunks must retain their fixed layout until finish.
        Ok(())
    }
}

impl RootTransactionProof {
    fn open(path: &Path) -> Result<Self, ProofError> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA page_size=4096;
             PRAGMA auto_vacuum=INCREMENTAL;
             PRAGMA journal_mode=WAL;
             PRAGMA synchronous=FULL;
             PRAGMA foreign_keys=ON;
             PRAGMA max_page_count=13631488;
             CREATE TABLE IF NOT EXISTS root (
               id INTEGER PRIMARY KEY CHECK (id=1),
               generation INTEGER NOT NULL,
               revision BLOB
             );
             CREATE TABLE IF NOT EXISTS outcome (
               epoch INTEGER NOT NULL,
               key INTEGER NOT NULL,
               request_hash BLOB NOT NULL,
               generation INTEGER NOT NULL,
               revision BLOB NOT NULL,
               PRIMARY KEY (epoch, key)
             );
             CREATE TABLE IF NOT EXISTS blob_header (
               hash BLOB PRIMARY KEY,
               length INTEGER NOT NULL,
               chunk_count INTEGER NOT NULL,
               state INTEGER NOT NULL CHECK (state IN (0, 1))
             );
             CREATE TABLE IF NOT EXISTS blob_chunks (
               hash BLOB NOT NULL,
               chunk_index INTEGER NOT NULL,
               bytes BLOB NOT NULL,
               PRIMARY KEY (hash, chunk_index),
               FOREIGN KEY (hash) REFERENCES blob_header(hash)
             );
             INSERT OR IGNORE INTO root (id, generation, revision) VALUES (1, 0, NULL);",
        )?;
        let page_size: i64 = conn.query_row("PRAGMA page_size", [], |row| row.get(0))?;
        let auto_vacuum: i64 = conn.query_row("PRAGMA auto_vacuum", [], |row| row.get(0))?;
        let journal_mode: String = conn.query_row("PRAGMA journal_mode", [], |row| row.get(0))?;
        let synchronous: i64 = conn.query_row("PRAGMA synchronous", [], |row| row.get(0))?;
        let foreign_keys: i64 = conn.query_row("PRAGMA foreign_keys", [], |row| row.get(0))?;
        let max_page_count: i64 = conn.query_row("PRAGMA max_page_count", [], |row| row.get(0))?;
        assert_eq!(page_size, 4096);
        assert_eq!(auto_vacuum, 2);
        assert_eq!(journal_mode, "wal");
        assert_eq!(synchronous, 2);
        assert_eq!(foreign_keys, 1);
        assert_eq!(max_page_count, MAX_PAGE_COUNT);
        Ok(Self {
            conn,
            scratch: Vec::new(),
        })
    }

    fn root(&self) -> rusqlite::Result<(i64, Option<Vec<u8>>)> {
        self.conn.query_row(
            "SELECT generation, revision FROM root WHERE id=1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
    }

    fn compare_history_with_current_root(
        &mut self,
        expected_generation: i64,
        candidate_project_id: &str,
        candidate_history_bytes: &[u8],
    ) -> Result<(), ProofError> {
        let (revision, manifest_bytes) = {
            let tx = self
                .conn
                .transaction_with_behavior(TransactionBehavior::Deferred)?;
            let (generation, revision) = root_identity(&tx)?;
            if generation != expected_generation {
                return Err(ProofError::StaleGeneration);
            }
            let revision = revision.ok_or(ProofError::MissingRootHistory)?;
            let bytes = read_complete_blob(&tx, revision, FIRST_PROFILE_MANIFEST_BYTES)?;
            tx.commit()?;
            (revision, bytes)
        };
        let manifest = decode_manifest_jcs(&manifest_bytes, FIRST_PROFILE_MANIFEST_BYTES)
            .map_err(ProofError::Manifest)?;
        if manifest.project_id() != candidate_project_id {
            return Err(ProofError::DifferentProject);
        }
        let mut histories = manifest.files().iter().filter_map(|file| {
            file.record_identity()
                .filter(|(_, record_type)| *record_type == "core.registry-history")
                .map(|(id, _)| (id, file.blob_identity()))
        });
        let (history_id, (history_length, history_hash)) =
            histories.next().ok_or(ProofError::MissingRootHistory)?;
        if histories.next().is_some() {
            return Err(ProofError::AmbiguousRootHistory);
        }
        let prior_bytes = {
            let tx = self
                .conn
                .transaction_with_behavior(TransactionBehavior::Deferred)?;
            let (generation, current_revision) = root_identity(&tx)?;
            if generation != expected_generation || current_revision != Some(revision) {
                return Err(ProofError::StaleGeneration);
            }
            let bytes = read_complete_blob(&tx, history_hash, MAX_HISTORY_BYTES)?;
            tx.commit()?;
            bytes
        };
        if prior_bytes.len() as u64 != history_length {
            return Err(ProofError::CorruptRootBlob);
        }
        let prior = parse_registry_history(&prior_bytes).map_err(ProofError::History)?;
        if prior.record_id() != history_id {
            return Err(ProofError::CorruptRootBlob);
        }
        let candidate =
            parse_registry_history(candidate_history_bytes).map_err(ProofError::History)?;
        compare_carried_histories(&prior, &candidate).map_err(ProofError::Lineage)
    }

    fn compare_staged_candidate_with_current_root(
        &mut self,
        expected_generation: i64,
        candidate: &StagedCandidate<'_>,
    ) -> Result<(), ProofError> {
        let bytes = candidate
            .rehashed_history_bytes()
            .map_err(ProofError::CandidateStage)?;
        self.compare_history_with_current_root(
            expected_generation,
            candidate.registry().project().id(),
            &bytes,
        )
    }

    fn stage_blob(
        &mut self,
        input: &mut impl Read,
        length: u64,
        expected_hash: [u8; 32],
    ) -> Result<(), ProofError> {
        self.stage_blob_with(input, length, expected_hash, |_| {})
    }

    fn stage_small_blob_batch<'a>(
        &mut self,
        files: impl IntoIterator<Item = ([u8; 32], &'a [u8])>,
    ) -> Result<(), ProofError> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut count = 0_usize;
        let mut total_bytes = 0_usize;
        for (hash, bytes) in files {
            count += 1;
            total_bytes = total_bytes
                .checked_add(bytes.len())
                .ok_or(ProofError::InputLength)?;
            if count > SMALL_BATCH_FILES || total_bytes > SMALL_BATCH_BYTES {
                return Err(ProofError::InputLength);
            }
            if Sha256::digest(bytes).as_slice() != hash {
                return Err(ProofError::HashMismatch);
            }
            let length = i64::try_from(bytes.len()).map_err(|_| ProofError::InputLength)?;
            let chunk_count = i64::from(!bytes.is_empty());
            let existing: Option<(i64, i64, i64)> = tx
                .query_row(
                    "SELECT length, chunk_count, state FROM blob_header WHERE hash=?1",
                    params![hash.as_slice()],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .optional()?;
            match existing {
                Some((old_length, old_chunks, 1)) => {
                    if old_length != length || old_chunks != chunk_count {
                        return Err(ProofError::BlobMetadataMismatch);
                    }
                    continue;
                }
                Some((_, _, 0)) => {
                    tx.execute(
                        "DELETE FROM blob_chunks WHERE hash=?1",
                        params![hash.as_slice()],
                    )?;
                    tx.execute(
                        "UPDATE blob_header SET length=?2, chunk_count=?3 WHERE hash=?1",
                        params![hash.as_slice(), length, chunk_count],
                    )?;
                }
                None => {
                    tx.execute(
                        "INSERT INTO blob_header (hash, length, chunk_count, state) VALUES (?1, ?2, ?3, 0)",
                        params![hash.as_slice(), length, chunk_count],
                    )?;
                }
                Some(_) => return Err(ProofError::BlobMetadataMismatch),
            }
            if !bytes.is_empty() {
                tx.execute(
                    "INSERT INTO blob_chunks (hash, chunk_index, bytes) VALUES (?1, 0, ?2)",
                    params![hash.as_slice(), bytes],
                )?;
                let stored: Vec<u8> = tx.query_row(
                    "SELECT bytes FROM blob_chunks WHERE hash=?1 AND chunk_index=0",
                    params![hash.as_slice()],
                    |row| row.get(0),
                )?;
                if stored.len() != bytes.len() || Sha256::digest(&stored).as_slice() != hash {
                    return Err(ProofError::HashMismatch);
                }
            }
            tx.execute(
                "UPDATE blob_header SET state=1 WHERE hash=?1 AND state=0",
                params![hash.as_slice()],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    fn stage_manifest_chunked(&mut self, manifest: &TreeManifest) -> Result<[u8; 32], ProofError> {
        if !manifest.dependency_roots().is_empty() {
            return Err(ProofError::UnsupportedDependencyRoots);
        }
        let mut count = HashCountWriter {
            digest: Sha256::new(),
            length: 0,
        };
        manifest
            .write_jcs(&mut count)
            .map_err(|_| ProofError::OutputIo)?;
        let length = count.length;
        if length > FIRST_PROFILE_MANIFEST_BYTES as u64 {
            return Err(ProofError::InputLength);
        }
        let hash: [u8; 32] = count.digest.finalize().into();
        let chunk_count = blob_chunk_count(length)?;
        if self.begin_blob(length, hash, chunk_count)? {
            return Ok(hash);
        }
        let mut writer = ManifestBlobWriter {
            store: self,
            buffer: Vec::with_capacity(CHUNK_BYTES),
            digest: Sha256::new(),
            length,
            written: 0,
            hash,
            chunk_count,
            next_chunk: 0,
            failure: None,
        };
        if manifest.write_jcs(&mut writer).is_err() {
            return Err(writer.failure.take().unwrap_or(ProofError::OutputIo));
        }
        writer.finish()?;
        Ok(hash)
    }

    fn stage_blob_with(
        &mut self,
        input: &mut impl Read,
        length: u64,
        expected_hash: [u8; 32],
        after_chunk: impl FnMut(i64),
    ) -> Result<(), ProofError> {
        let mut scratch = std::mem::take(&mut self.scratch);
        let result =
            self.stage_blob_with_scratch(input, length, expected_hash, &mut scratch, after_chunk);
        self.scratch = scratch;
        result
    }

    fn stage_blob_with_scratch(
        &mut self,
        input: &mut impl Read,
        length: u64,
        expected_hash: [u8; 32],
        scratch: &mut Vec<u8>,
        mut after_chunk: impl FnMut(i64),
    ) -> Result<(), ProofError> {
        let chunk_count = blob_chunk_count(length)?;
        let scratch_len = length.min(CHUNK_BYTES as u64).max(1) as usize;
        scratch.resize(scratch_len, 0);
        if self.begin_blob(length, expected_hash, chunk_count)? {
            verify_supplied_blob(input, length, expected_hash, scratch)?;
            return Ok(());
        }
        let mut digest = Sha256::new();
        let mut remaining = length;
        for index in 0..chunk_count {
            let count = usize::try_from(remaining.min(CHUNK_BYTES as u64))
                .map_err(|_| ProofError::InputLength)?;
            input
                .read_exact(&mut scratch[..count])
                .map_err(|_| ProofError::InputIo)?;
            digest.update(&scratch[..count]);
            self.append_blob_chunk(length, expected_hash, chunk_count, index, &scratch[..count])?;
            remaining -= count as u64;
            after_chunk(index);
        }
        let mut extra = [0_u8; 1];
        if input.read(&mut extra).map_err(|_| ProofError::InputIo)? != 0 {
            return Err(ProofError::InputLength);
        }
        if digest.finalize().as_slice() != expected_hash {
            return Err(ProofError::HashMismatch);
        }

        self.verify_and_complete_blob(length, expected_hash, chunk_count, || {})
    }

    fn begin_blob(
        &mut self,
        length: u64,
        expected_hash: [u8; 32],
        chunk_count: i64,
    ) -> Result<bool, ProofError> {
        let sql_length = i64::try_from(length).map_err(|_| ProofError::InputLength)?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing: Option<(i64, i64, i64)> = tx
            .query_row(
                "SELECT length, chunk_count, state FROM blob_header WHERE hash=?1",
                params![expected_hash.as_slice()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        match existing {
            Some((prior_length, prior_chunks, 1)) => {
                if prior_length != sql_length || prior_chunks != chunk_count {
                    return Err(ProofError::BlobMetadataMismatch);
                }
                // The caller still verifies its candidate stage separately.
                return Ok(true);
            }
            Some((_, _, 0)) => {
                tx.execute(
                    "DELETE FROM blob_chunks WHERE hash=?1",
                    params![expected_hash.as_slice()],
                )?;
                tx.execute(
                    "UPDATE blob_header SET length=?2, chunk_count=?3 WHERE hash=?1",
                    params![expected_hash.as_slice(), sql_length, chunk_count],
                )?;
            }
            None => {
                tx.execute(
                    "INSERT INTO blob_header (hash, length, chunk_count, state) VALUES (?1, ?2, ?3, 0)",
                    params![expected_hash.as_slice(), sql_length, chunk_count],
                )?;
            }
            Some(_) => return Err(ProofError::BlobMetadataMismatch),
        }
        tx.commit()?;
        Ok(false)
    }

    fn append_blob_chunk(
        &mut self,
        length: u64,
        expected_hash: [u8; 32],
        chunk_count: i64,
        index: i64,
        bytes: &[u8],
    ) -> Result<(), ProofError> {
        let sql_length = i64::try_from(length).map_err(|_| ProofError::InputLength)?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let inserted = tx.execute(
            "INSERT INTO blob_chunks (hash, chunk_index, bytes)
                 SELECT ?1, ?2, ?3
                 WHERE EXISTS (
                   SELECT 1 FROM blob_header
                   WHERE hash=?1 AND length=?4 AND chunk_count=?5 AND state=0
                 )",
            params![
                expected_hash.as_slice(),
                index,
                bytes,
                sql_length,
                chunk_count
            ],
        )?;
        if inserted != 1 {
            return Err(ProofError::StaleBlobWriter);
        }
        tx.commit()?;
        Ok(())
    }

    fn verify_and_complete_blob(
        &mut self,
        length: u64,
        expected_hash: [u8; 32],
        chunk_count: i64,
        after_verify: impl FnOnce(),
    ) -> Result<(), ProofError> {
        let sql_length = i64::try_from(length).map_err(|_| ProofError::InputLength)?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let actual_chunks: i64 = tx.query_row(
            "SELECT count(*) FROM blob_chunks WHERE hash=?1",
            params![expected_hash.as_slice()],
            |row| row.get(0),
        )?;
        if actual_chunks != chunk_count {
            return Err(ProofError::InvalidChunkLayout);
        }
        let mut stored = Sha256::new();
        let mut stored_length = 0_u64;
        for index in 0..chunk_count {
            let expected_count = usize::try_from((length - stored_length).min(CHUNK_BYTES as u64))
                .map_err(|_| ProofError::InputLength)?;
            let actual: i64 = tx
                .query_row(
                    "SELECT length(bytes) FROM blob_chunks WHERE hash=?1 AND chunk_index=?2",
                    params![expected_hash.as_slice(), index],
                    |row| row.get(0),
                )
                .optional()?
                .ok_or(ProofError::MissingChunk)?;
            if actual != expected_count as i64 {
                return Err(ProofError::InvalidChunkLayout);
            }
            let bytes: Vec<u8> = tx
                .query_row(
                    "SELECT bytes FROM blob_chunks WHERE hash=?1 AND chunk_index=?2",
                    params![expected_hash.as_slice(), index],
                    |row| row.get(0),
                )
                .optional()?
                .ok_or(ProofError::MissingChunk)?;
            stored_length = stored_length
                .checked_add(bytes.len() as u64)
                .ok_or(ProofError::InputLength)?;
            stored.update(&bytes);
        }
        if stored_length != length || stored.finalize().as_slice() != expected_hash {
            return Err(ProofError::HashMismatch);
        }
        after_verify();
        let updated = tx.execute(
            "UPDATE blob_header SET state=1 WHERE hash=?1 AND length=?2 AND chunk_count=?3 AND state=0",
            params![expected_hash.as_slice(), sql_length, chunk_count],
        )?;
        if updated != 1 {
            return Err(ProofError::BlobMetadataMismatch);
        }
        tx.commit()?;
        Ok(())
    }

    fn commit_probe(
        &mut self,
        expected_generation: i64,
        epoch: i64,
        key: i64,
        request_hash: [u8; 32],
        revision: [u8; 32],
    ) -> Result<i64, ProofError> {
        self.commit_with_closure(
            expected_generation,
            epoch,
            key,
            request_hash,
            revision,
            |_| Ok(()),
        )
    }

    fn commit_manifest_probe(
        &mut self,
        expected_generation: i64,
        epoch: i64,
        key: i64,
        request_hash: [u8; 32],
        manifest: &TreeManifest,
    ) -> Result<i64, ProofError> {
        if !manifest.dependency_roots().is_empty() {
            return Err(ProofError::UnsupportedDependencyRoots);
        }
        let mut count = HashCountWriter {
            digest: Sha256::new(),
            length: 0,
        };
        manifest
            .write_jcs(&mut count)
            .map_err(|_| ProofError::OutputIo)?;
        if count.length > FIRST_PROFILE_MANIFEST_BYTES as u64 {
            return Err(ProofError::InputLength);
        }
        let revision: [u8; 32] = count.digest.finalize().into();
        self.commit_with_closure(
            expected_generation,
            epoch,
            key,
            request_hash,
            revision,
            |tx| {
                let mut lookup =
                    tx.prepare("SELECT length FROM blob_header WHERE hash=?1 AND state=1")?;
                for file in manifest.files() {
                    let (length, hash) = file.blob_identity();
                    let stored_length: Option<i64> = lookup
                        .query_row(params![hash.as_slice()], |row| row.get(0))
                        .optional()?;
                    let length = i64::try_from(length).map_err(|_| ProofError::InputLength)?;
                    if stored_length != Some(length) {
                        return Err(ProofError::MissingCompleteFile);
                    }
                }
                Ok(())
            },
        )
    }

    fn commit_with_closure(
        &mut self,
        expected_generation: i64,
        epoch: i64,
        key: i64,
        request_hash: [u8; 32],
        revision: [u8; 32],
        mut check_closure: impl FnMut(&Transaction<'_>) -> Result<(), ProofError>,
    ) -> Result<i64, ProofError> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing: Option<(Vec<u8>, i64, Vec<u8>)> = tx
            .query_row(
                "SELECT request_hash, generation, revision FROM outcome WHERE epoch=?1 AND key=?2",
                params![epoch, key],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        if let Some((prior_request, generation, prior_revision)) = existing {
            return if prior_request == request_hash && prior_revision == revision {
                Ok(generation)
            } else {
                Err(ProofError::TokenConflict)
            };
        }
        let generation: i64 =
            tx.query_row("SELECT generation FROM root WHERE id=1", [], |row| {
                row.get(0)
            })?;
        if generation != expected_generation {
            return Err(ProofError::StaleGeneration);
        }
        let complete: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM blob_header WHERE hash=?1 AND state=1)",
            params![revision.as_slice()],
            |row| row.get(0),
        )?;
        if !complete {
            return Err(ProofError::MissingCompleteBlob);
        }
        check_closure(&tx)?;
        let next = generation
            .checked_add(1)
            .ok_or(ProofError::StaleGeneration)?;
        tx.execute(
            "UPDATE root SET generation=?1, revision=?2 WHERE id=1",
            params![next, revision.as_slice()],
        )?;
        tx.execute(
            "INSERT INTO outcome (epoch, key, request_hash, generation, revision) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![epoch, key, request_hash.as_slice(), next, revision.as_slice()],
        )?;
        tx.commit()?;
        Ok(next)
    }
}

#[test]
fn root_and_outcome_commit_together_and_retry_survives_reopen() {
    let fixture = Fixture::new();
    let mut store = RootTransactionProof::open(&fixture.database()).unwrap();
    let sqlite_version: String = store
        .conn
        .query_row("SELECT sqlite_version()", [], |row| row.get(0))
        .unwrap();
    eprintln!("bundled_sqlite={sqlite_version}");
    assert_eq!(store.root().unwrap(), (0, None));
    let request = [11_u8; 32];
    let first_manifest = b"manifest A, not yet a source closure";
    let revision: [u8; 32] = Sha256::digest(first_manifest).into();
    assert!(matches!(
        store.commit_probe(0, 1, 1, request, revision),
        Err(ProofError::MissingCompleteBlob)
    ));
    store
        .stage_blob(
            &mut first_manifest.as_slice(),
            first_manifest.len() as u64,
            revision,
        )
        .unwrap();
    assert_eq!(store.commit_probe(0, 1, 1, request, revision).unwrap(), 1);
    assert_eq!(store.root().unwrap(), (1, Some(revision.to_vec())));
    drop(store);

    let mut reopened = RootTransactionProof::open(&fixture.database()).unwrap();
    assert_eq!(reopened.root().unwrap(), (1, Some(revision.to_vec())));

    let second_manifest = b"manifest B, not yet a source closure";
    let revision2: [u8; 32] = Sha256::digest(second_manifest).into();
    reopened
        .stage_blob(
            &mut second_manifest.as_slice(),
            second_manifest.len() as u64,
            revision2,
        )
        .unwrap();
    assert_eq!(
        reopened.commit_probe(0, 1, 1, request, revision).unwrap(),
        1
    );
    assert!(matches!(
        reopened.commit_probe(0, 1, 1, [33; 32], revision),
        Err(ProofError::TokenConflict)
    ));
    assert!(matches!(
        reopened.commit_probe(0, 1, 2, request, revision2),
        Err(ProofError::StaleGeneration)
    ));
    assert_eq!(reopened.root().unwrap(), (1, Some(revision.to_vec())));

    reopened
        .conn
        .execute_batch(
            "CREATE TRIGGER abort_outcome BEFORE INSERT ON outcome
             BEGIN SELECT RAISE(ABORT, 'injected outcome failure'); END;",
        )
        .unwrap();
    match reopened.commit_probe(1, 1, 2, request, revision2) {
        Err(ProofError::Sql(error)) => {
            assert!(error.to_string().contains("injected outcome failure"));
        }
        other => panic!("expected injected SQLite refusal, got {other:?}"),
    }
    assert_eq!(reopened.root().unwrap(), (1, Some(revision.to_vec())));
    reopened
        .conn
        .execute_batch("DROP TRIGGER abort_outcome;")
        .unwrap();
    assert_eq!(
        reopened.commit_probe(1, 1, 2, request, revision2).unwrap(),
        2
    );
    drop(reopened);
    let final_store = RootTransactionProof::open(&fixture.database()).unwrap();
    assert_eq!(final_store.root().unwrap(), (2, Some(revision2.to_vec())));
}

#[test]
fn blob_chunks_verify_before_complete_and_failed_stream_cannot_be_root() {
    let fixture = Fixture::new();
    let mut store = RootTransactionProof::open(&fixture.database()).unwrap();
    let bytes = vec![7_u8; CHUNK_BYTES + 17];
    let hash: [u8; 32] = Sha256::digest(&bytes).into();
    store
        .stage_blob(&mut bytes.as_slice(), bytes.len() as u64, hash)
        .unwrap();
    let (length, count, state): (i64, i64, i64) = store
        .conn
        .query_row(
            "SELECT length, chunk_count, state FROM blob_header WHERE hash=?1",
            params![hash.as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!((length, count, state), ((CHUNK_BYTES + 17) as i64, 2, 1));
    let chunk_lengths: Vec<i64> = (0..2)
        .map(|index| {
            store
                .conn
                .query_row(
                    "SELECT length(bytes) FROM blob_chunks WHERE hash=?1 AND chunk_index=?2",
                    params![hash.as_slice(), index],
                    |row| row.get(0),
                )
                .unwrap()
        })
        .collect();
    assert_eq!(chunk_lengths, [CHUNK_BYTES as i64, 17]);
    assert_eq!(store.commit_probe(0, 1, 1, [1; 32], hash).unwrap(), 1);
    let mut repeated_input = bytes.as_slice();
    store
        .stage_blob(&mut repeated_input, bytes.len() as u64, hash)
        .unwrap();
    assert!(repeated_input.is_empty());
    let mut changed_input = bytes.clone();
    changed_input[0] ^= 1;
    assert!(matches!(
        store.stage_blob(&mut changed_input.as_slice(), bytes.len() as u64, hash),
        Err(ProofError::HashMismatch)
    ));

    let retry_bytes = vec![8_u8; CHUNK_BYTES + 17];
    let retry_hash: [u8; 32] = Sha256::digest(&retry_bytes).into();
    let mut partial = &retry_bytes[..CHUNK_BYTES];
    assert!(matches!(
        store.stage_blob(&mut partial, retry_bytes.len() as u64, retry_hash),
        Err(ProofError::InputIo)
    ));
    assert!(matches!(
        store.commit_probe(1, 1, 2, [2; 32], retry_hash),
        Err(ProofError::MissingCompleteBlob)
    ));
    store
        .stage_blob(
            &mut retry_bytes.as_slice(),
            retry_bytes.len() as u64,
            retry_hash,
        )
        .unwrap();
    let retry_chunk_count: i64 = store
        .conn
        .query_row(
            "SELECT count(*) FROM blob_chunks WHERE hash=?1",
            params![retry_hash.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(retry_chunk_count, 2);

    let bad_hash = [9_u8; 32];
    assert!(matches!(
        store.stage_blob(&mut b"wrong bytes".as_slice(), 11, bad_hash),
        Err(ProofError::HashMismatch)
    ));
    assert!(matches!(
        store.commit_probe(1, 1, 2, [2; 32], bad_hash),
        Err(ProofError::MissingCompleteBlob)
    ));
    let bad_state: i64 = store
        .conn
        .query_row(
            "SELECT state FROM blob_header WHERE hash=?1",
            params![bad_hash.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(bad_state, 0);
    assert_eq!(store.root().unwrap(), (1, Some(hash.to_vec())));
}

#[test]
fn extra_stored_chunk_cannot_become_complete() {
    let fixture = Fixture::new();
    let mut store = RootTransactionProof::open(&fixture.database()).unwrap();
    let bytes = b"one expected chunk";
    let hash: [u8; 32] = Sha256::digest(bytes).into();
    store
        .conn
        .execute(
            "INSERT INTO blob_header (hash, length, chunk_count, state) VALUES (?1, ?2, 1, 0)",
            params![hash.as_slice(), bytes.len() as i64],
        )
        .unwrap();
    for index in 0..2 {
        store
            .conn
            .execute(
                "INSERT INTO blob_chunks (hash, chunk_index, bytes) VALUES (?1, ?2, ?3)",
                params![hash.as_slice(), index, bytes.as_slice()],
            )
            .unwrap();
    }
    assert!(matches!(
        store.verify_and_complete_blob(bytes.len() as u64, hash, 1, || {}),
        Err(ProofError::InvalidChunkLayout)
    ));
    assert!(matches!(
        store.commit_probe(0, 1, 1, [1; 32], hash),
        Err(ProofError::MissingCompleteBlob)
    ));
}

#[test]
fn completion_transaction_excludes_same_hash_retry_reset() {
    let fixture = Fixture::new();
    let mut first = RootTransactionProof::open(&fixture.database()).unwrap();
    let mut second = RootTransactionProof::open(&fixture.database()).unwrap();
    second.conn.busy_timeout(std::time::Duration::ZERO).unwrap();
    let bytes = b"one verified chunk";
    let hash: [u8; 32] = Sha256::digest(bytes).into();
    first
        .conn
        .execute(
            "INSERT INTO blob_header (hash, length, chunk_count, state) VALUES (?1, ?2, 1, 0)",
            params![hash.as_slice(), bytes.len() as i64],
        )
        .unwrap();
    first
        .conn
        .execute(
            "INSERT INTO blob_chunks (hash, chunk_index, bytes) VALUES (?1, 0, ?2)",
            params![hash.as_slice(), bytes.as_slice()],
        )
        .unwrap();

    first
        .verify_and_complete_blob(bytes.len() as u64, hash, 1, || {
            let mut second_input = bytes.as_slice();
            assert!(matches!(
                second.stage_blob(&mut second_input, bytes.len() as u64, hash),
                Err(ProofError::Sql(_))
            ));
        })
        .unwrap();
    let mut second_input = bytes.as_slice();
    second
        .stage_blob(&mut second_input, bytes.len() as u64, hash)
        .unwrap();
    assert!(second_input.is_empty());
    assert_eq!(first.commit_probe(0, 1, 1, [3; 32], hash).unwrap(), 1);
}

#[test]
fn stale_stream_cannot_append_after_same_hash_replacement_completes() {
    let fixture = Fixture::new();
    let mut first = RootTransactionProof::open(&fixture.database()).unwrap();
    let mut second = RootTransactionProof::open(&fixture.database()).unwrap();
    let complete_bytes = vec![5_u8; CHUNK_BYTES];
    let hash: [u8; 32] = Sha256::digest(&complete_bytes).into();
    let stale_bytes = vec![6_u8; CHUNK_BYTES + 17];
    let mut replaced = false;
    assert!(matches!(
        first.stage_blob_with(
            &mut stale_bytes.as_slice(),
            stale_bytes.len() as u64,
            hash,
            |index| {
                if index == 0 {
                    second
                        .stage_blob(
                            &mut complete_bytes.as_slice(),
                            complete_bytes.len() as u64,
                            hash,
                        )
                        .unwrap();
                    replaced = true;
                }
            }
        ),
        Err(ProofError::StaleBlobWriter)
    ));
    assert!(replaced);
    let count: i64 = first
        .conn
        .query_row(
            "SELECT count(*) FROM blob_chunks WHERE hash=?1",
            params![hash.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
    assert_eq!(first.commit_probe(0, 1, 1, [4; 32], hash).unwrap(), 1);
}

#[test]
fn canonical_manifest_bytes_reopen_from_complete_blob() {
    const PROJECT_ID: &str = "2a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    let fixture = Fixture::new();
    let source_bytes = b"small source file";
    let entry = ManifestEntry::new(
        SourcePath::parse("src/game.rs").unwrap(),
        source_bytes.len() as u64,
        Sha256::digest(source_bytes).into(),
        None,
    )
    .unwrap();
    let manifest = TreeManifest::new(PROJECT_ID, Vec::new(), vec![entry]).unwrap();
    let mut canonical = Vec::new();
    manifest.write_jcs(&mut canonical).unwrap();
    let revision = *manifest.revision().as_bytes();
    assert_eq!(Sha256::digest(&canonical).as_slice(), revision);

    let mut store = RootTransactionProof::open(&fixture.database()).unwrap();
    store
        .stage_blob(&mut canonical.as_slice(), canonical.len() as u64, revision)
        .unwrap();
    assert_eq!(store.commit_probe(0, 1, 1, [1; 32], revision).unwrap(), 1);
    drop(store);

    let reopened = RootTransactionProof::open(&fixture.database()).unwrap();
    assert_eq!(reopened.root().unwrap(), (1, Some(revision.to_vec())));
    let stored: Vec<u8> = reopened
        .conn
        .query_row(
            "SELECT bytes FROM blob_chunks WHERE hash=?1 AND chunk_index=0",
            params![revision.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stored, canonical);
}

#[test]
fn first_store_profile_refuses_unresolved_dependency_roots() {
    let fixture = Fixture::new();
    let mut store = RootTransactionProof::open(&fixture.database()).unwrap();
    let manifest = TreeManifest::new(
        "2a5ef644-7225-4770-9f4d-e1d9c0a7d515",
        vec!["external-root".to_owned()],
        Vec::new(),
    )
    .unwrap();
    assert!(matches!(
        store.stage_manifest_chunked(&manifest),
        Err(ProofError::UnsupportedDependencyRoots)
    ));
    assert!(matches!(
        store.commit_manifest_probe(0, 1, 1, [1; 32], &manifest),
        Err(ProofError::UnsupportedDependencyRoots)
    ));
    assert_eq!(store.root().unwrap(), (0, None));
}

#[test]
fn large_manifest_streams_directly_to_verified_sqlite_chunks() {
    const PROJECT_ID: &str = "2a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    let fixture = Fixture::new();
    let files = (0..26_000)
        .map(|index| {
            ManifestEntry::new(
                SourcePath::parse(&format!("src/file-{index:05}.bin")).unwrap(),
                1,
                Sha256::digest([index as u8]).into(),
                None,
            )
            .unwrap()
        })
        .collect();
    let manifest = TreeManifest::new(PROJECT_ID, Vec::new(), files).unwrap();
    let revision = *manifest.revision().as_bytes();
    let mut store = RootTransactionProof::open(&fixture.database()).unwrap();
    assert_eq!(store.stage_manifest_chunked(&manifest).unwrap(), revision);
    let (length, chunks, state): (i64, i64, i64) = store
        .conn
        .query_row(
            "SELECT length, chunk_count, state FROM blob_header WHERE hash=?1",
            params![revision.as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert!(length > CHUNK_BYTES as i64);
    assert_eq!(chunks, 2);
    assert_eq!(state, 1);
    assert_eq!(store.commit_probe(0, 1, 1, [8; 32], revision).unwrap(), 1);
    drop(store);

    let reopened = RootTransactionProof::open(&fixture.database()).unwrap();
    let mut stored = Sha256::new();
    let mut stored_length = 0;
    for index in 0..chunks {
        let bytes: Vec<u8> = reopened
            .conn
            .query_row(
                "SELECT bytes FROM blob_chunks WHERE hash=?1 AND chunk_index=?2",
                params![revision.as_slice(), index],
                |row| row.get(0),
            )
            .unwrap();
        stored_length += bytes.len() as i64;
        stored.update(bytes);
    }
    assert_eq!(stored_length, length);
    assert_eq!(stored.finalize().as_slice(), revision);
    assert_eq!(reopened.root().unwrap(), (1, Some(revision.to_vec())));
}

#[test]
fn manifest_root_requires_each_direct_file_blob_complete() {
    const PROJECT_ID: &str = "2a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    let fixture = Fixture::new();
    let first = b"one source file";
    let second = b"another source file";
    let first_hash: [u8; 32] = Sha256::digest(first).into();
    let second_hash: [u8; 32] = Sha256::digest(second).into();
    let manifest = TreeManifest::new(
        PROJECT_ID,
        Vec::new(),
        vec![
            ManifestEntry::new(
                SourcePath::parse("src/first.rs").unwrap(),
                first.len() as u64,
                first_hash,
                None,
            )
            .unwrap(),
            ManifestEntry::new(
                SourcePath::parse("src/second.rs").unwrap(),
                second.len() as u64,
                second_hash,
                None,
            )
            .unwrap(),
        ],
    )
    .unwrap();
    let mut store = RootTransactionProof::open(&fixture.database()).unwrap();
    let revision = store.stage_manifest_chunked(&manifest).unwrap();
    store
        .stage_blob(&mut first.as_slice(), first.len() as u64, first_hash)
        .unwrap();
    assert!(matches!(
        store.commit_manifest_probe(0, 1, 1, [1; 32], &manifest),
        Err(ProofError::MissingCompleteFile)
    ));
    assert_eq!(store.root().unwrap(), (0, None));
    store
        .stage_blob(&mut second.as_slice(), second.len() as u64, second_hash)
        .unwrap();
    store
        .conn
        .execute(
            "UPDATE blob_header SET length=0 WHERE hash=?1",
            params![second_hash.as_slice()],
        )
        .unwrap();
    assert!(matches!(
        store.commit_manifest_probe(0, 1, 1, [1; 32], &manifest),
        Err(ProofError::MissingCompleteFile)
    ));
    assert_eq!(store.root().unwrap(), (0, None));
    store
        .conn
        .execute(
            "UPDATE blob_header SET length=?2 WHERE hash=?1",
            params![second_hash.as_slice(), second.len() as i64],
        )
        .unwrap();
    assert_eq!(
        store
            .commit_manifest_probe(0, 1, 1, [1; 32], &manifest)
            .unwrap(),
        1
    );
    drop(store);

    let mut reopened = RootTransactionProof::open(&fixture.database()).unwrap();
    assert_eq!(reopened.root().unwrap(), (1, Some(revision.to_vec())));
    assert_eq!(
        reopened
            .commit_manifest_probe(0, 1, 1, [1; 32], &manifest)
            .unwrap(),
        1
    );
    reopened
        .conn
        .execute(
            "UPDATE blob_header SET length=0 WHERE hash=?1",
            params![second_hash.as_slice()],
        )
        .unwrap();
    assert_eq!(
        reopened
            .commit_manifest_probe(0, 1, 1, [1; 32], &manifest)
            .unwrap(),
        1
    );
}

#[test]
fn small_blob_batch_verifies_and_rolls_back_as_one_unit() {
    let fixture = Fixture::new();
    let mut store = RootTransactionProof::open(&fixture.database()).unwrap();
    let first = b"first source";
    let second = b"second source";
    let first_hash: [u8; 32] = Sha256::digest(first).into();
    let second_hash: [u8; 32] = Sha256::digest(second).into();
    assert!(matches!(
        store.stage_small_blob_batch([
            (first_hash, first.as_slice()),
            (second_hash, b"wrong source".as_slice()),
        ]),
        Err(ProofError::HashMismatch)
    ));
    let count: i64 = store
        .conn
        .query_row("SELECT count(*) FROM blob_header", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
    store
        .stage_small_blob_batch([
            (first_hash, first.as_slice()),
            (second_hash, second.as_slice()),
        ])
        .unwrap();
    store
        .stage_small_blob_batch([(first_hash, first.as_slice())])
        .unwrap();
    let complete: i64 = store
        .conn
        .query_row(
            "SELECT count(*) FROM blob_header WHERE state=1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(complete, 2);
    let manifest = TreeManifest::new(
        "2a5ef644-7225-4770-9f4d-e1d9c0a7d515",
        Vec::new(),
        vec![
            ManifestEntry::new(
                SourcePath::parse("src/first.rs").unwrap(),
                first.len() as u64,
                first_hash,
                None,
            )
            .unwrap(),
            ManifestEntry::new(
                SourcePath::parse("src/second.rs").unwrap(),
                second.len() as u64,
                second_hash,
                None,
            )
            .unwrap(),
        ],
    )
    .unwrap();
    let revision = store.stage_manifest_chunked(&manifest).unwrap();
    assert_eq!(
        store
            .commit_manifest_probe(0, 1, 1, [7; 32], &manifest)
            .unwrap(),
        1
    );
    drop(store);
    let reopened = RootTransactionProof::open(&fixture.database()).unwrap();
    assert_eq!(reopened.root().unwrap(), (1, Some(revision.to_vec())));
    let stored: Vec<u8> = reopened
        .conn
        .query_row(
            "SELECT bytes FROM blob_chunks WHERE hash=?1 AND chunk_index=0",
            params![second_hash.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stored, second);
}

const LINEAGE_PROJECT_ID: &str = "2a5ef644-7225-4770-9f4d-e1d9c0a7d515";
const LINEAGE_HISTORY_ID: &str = "82b0ec04-6866-4271-a2cf-1f1576ad604a";

fn lineage_history(extra: bool) -> Vec<u8> {
    let mut fields = vec![serde_json::json!({
        "typeId":"game.marker", "fieldId":"marker.caption",
        "signature":"00".repeat(32), "retired":false
    })];
    let mut versions = vec![serde_json::json!({
        "typeId":"game.marker", "targetVersion":1,
        "definitionHash":"11".repeat(32)
    })];
    if extra {
        fields.push(serde_json::json!({
            "typeId":"game.marker", "fieldId":"marker.extra",
            "signature":"22".repeat(32), "retired":false
        }));
        versions.push(serde_json::json!({
            "typeId":"game.marker", "targetVersion":2,
            "definitionHash":"33".repeat(32)
        }));
    }
    serde_json::to_vec(&serde_json::json!({
        "id":LINEAGE_HISTORY_ID,
        "type":"core.registry-history",
        "schema":1,
        "fields":{"registry.fields":fields,"registry.versions":versions}
    }))
    .unwrap()
}

fn lineage_manifest(history_bytes: &[u8]) -> TreeManifest {
    TreeManifest::new(
        LINEAGE_PROJECT_ID,
        Vec::new(),
        vec![
            ManifestEntry::new(
                SourcePath::parse("schemas/history.registry.json").unwrap(),
                history_bytes.len() as u64,
                Sha256::digest(history_bytes).into(),
                Some(crate::RecordDescriptor::checked_parts(
                    LINEAGE_HISTORY_ID,
                    "core.registry-history",
                )),
            )
            .unwrap(),
        ],
    )
    .unwrap()
}

fn minimal_source_stage(fixture: &Fixture) -> crate::StagedTree {
    const SCENE_ID: &str = "3a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    let root = fixture.source_root();
    fs::create_dir_all(root.join("scenes")).unwrap();
    fs::create_dir_all(root.join("schemas")).unwrap();
    fs::write(
        root.join("project.json"),
        serde_json::to_vec(&serde_json::json!({
            "id":LINEAGE_PROJECT_ID,"type":"core.project","schema":2,
            "fields":{
                "project.format":1,
                "project.records":["scenes/main.scene.json"],
                "project.schemas":[],
                "project.registryHistory":"schemas/history.registry.json",
                "project.entryScenes":[SCENE_ID]
            }
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        root.join("scenes/main.scene.json"),
        serde_json::to_vec(&serde_json::json!({
            "id":SCENE_ID,"type":"core.scene","schema":1,"fields":{}
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        root.join("schemas/history.registry.json"),
        serde_json::to_vec(&serde_json::json!({
            "id":LINEAGE_HISTORY_ID,"type":"core.registry-history","schema":1,
            "fields":{"registry.fields":[],"registry.versions":[]}
        }))
        .unwrap(),
    )
    .unwrap();
    let paths = [
        "project.json",
        "scenes/main.scene.json",
        "schemas/history.registry.json",
    ]
    .map(|path| SourcePath::parse(path).unwrap());
    stage_declared_tree_with(
        &root,
        &paths,
        Limits {
            main: 2 * 1024 * 1024 * 1024,
            wal: 4 * 1024 * 1024,
            temporary: 64 * 1024 * 1024,
            free_floor: 0,
        },
        |_| {},
    )
    .unwrap()
}

#[test]
fn candidate_history_comes_from_rehashed_stage_and_reused_blobs_are_checked() {
    let fixture = Fixture::new();
    let stage = minimal_source_stage(&fixture);
    let candidate = validate_staged_candidate(&stage, &[]).unwrap();
    let manifest = candidate.provisional_manifest().unwrap();
    let mut store = RootTransactionProof::open(&fixture.database()).unwrap();
    for file in stage.files() {
        let mut reader = stage.open(file.path()).unwrap();
        store
            .stage_blob(&mut reader, file.length(), *file.sha256())
            .unwrap();
    }
    store.stage_manifest_chunked(&manifest).unwrap();
    store
        .commit_manifest_probe(0, 1, 1, [1; 32], &manifest)
        .unwrap();
    assert!(
        store
            .compare_staged_candidate_with_current_root(1, &candidate)
            .is_ok()
    );

    let second_stage = minimal_source_stage(&fixture);
    let second_candidate = validate_staged_candidate(&second_stage, &[]).unwrap();
    let history_path = SourcePath::parse("schemas/history.registry.json").unwrap();
    let history_file = second_stage
        .files()
        .iter()
        .find(|file| file.path() == &history_path)
        .unwrap();
    let staged_history_path = second_stage.test_directory().join(history_path.as_str());
    let mut changed = fs::read(&staged_history_path).unwrap();
    changed[0] = b'[';
    fs::write(staged_history_path, changed).unwrap();
    assert!(matches!(
        store.compare_staged_candidate_with_current_root(1, &second_candidate),
        Err(ProofError::CandidateStage(
            StagedRegistryError::StagedDrift(_)
        ))
    ));
    let mut changed_reader = second_stage.open(&history_path).unwrap();
    assert!(matches!(
        store.stage_blob(
            &mut changed_reader,
            history_file.length(),
            *history_file.sha256(),
        ),
        Err(ProofError::HashMismatch)
    ));
    assert_eq!(store.root().unwrap().0, 1);
}

#[test]
fn current_root_history_is_recovered_and_generation_cas_excludes_stale_lineage() {
    let fixture = Fixture::new();
    let mut first = RootTransactionProof::open(&fixture.database()).unwrap();
    let base = lineage_history(false);
    let candidate = lineage_history(true);
    let base_manifest = lineage_manifest(&base);
    let candidate_manifest = lineage_manifest(&candidate);
    first
        .stage_blob(
            &mut base.as_slice(),
            base.len() as u64,
            Sha256::digest(&base).into(),
        )
        .unwrap();
    first.stage_manifest_chunked(&base_manifest).unwrap();
    assert_eq!(
        first
            .commit_manifest_probe(0, 1, 1, [1; 32], &base_manifest)
            .unwrap(),
        1
    );
    drop(first);

    let mut first = RootTransactionProof::open(&fixture.database()).unwrap();
    assert!(
        first
            .compare_history_with_current_root(1, LINEAGE_PROJECT_ID, &candidate)
            .is_ok()
    );
    assert!(matches!(
        first.compare_history_with_current_root(1, LINEAGE_PROJECT_ID, &[]),
        Err(ProofError::History(_))
    ));
    assert!(matches!(
        first.compare_history_with_current_root(1, LINEAGE_PROJECT_ID, &lineage_history(false)),
        Ok(())
    ));
    assert!(matches!(
        first.compare_history_with_current_root(
            1,
            "f2b577f6-d755-4d6b-8df0-609c63914c1b",
            &candidate
        ),
        Err(ProofError::DifferentProject)
    ));
    let mut second = RootTransactionProof::open(&fixture.database()).unwrap();
    second
        .stage_blob(
            &mut candidate.as_slice(),
            candidate.len() as u64,
            Sha256::digest(&candidate).into(),
        )
        .unwrap();
    second.stage_manifest_chunked(&candidate_manifest).unwrap();
    assert_eq!(
        second
            .commit_manifest_probe(1, 1, 2, [2; 32], &candidate_manifest)
            .unwrap(),
        2
    );
    assert!(matches!(
        first.commit_manifest_probe(1, 1, 3, [3; 32], &candidate_manifest),
        Err(ProofError::StaleGeneration)
    ));
    assert!(matches!(
        first.compare_history_with_current_root(1, LINEAGE_PROJECT_ID, &candidate),
        Err(ProofError::StaleGeneration)
    ));
    assert!(matches!(
        first.compare_history_with_current_root(2, LINEAGE_PROJECT_ID, &base),
        Err(ProofError::Lineage(_))
    ));
}

#[test]
fn current_root_history_refuses_corrupt_stored_bytes() {
    let fixture = Fixture::new();
    let mut store = RootTransactionProof::open(&fixture.database()).unwrap();
    let base = lineage_history(false);
    let base_hash: [u8; 32] = Sha256::digest(&base).into();
    let manifest = lineage_manifest(&base);
    store
        .stage_blob(&mut base.as_slice(), base.len() as u64, base_hash)
        .unwrap();
    let revision = store.stage_manifest_chunked(&manifest).unwrap();
    store
        .commit_manifest_probe(0, 1, 1, [1; 32], &manifest)
        .unwrap();
    store
        .conn
        .execute(
            "UPDATE blob_chunks SET bytes=zeroblob(4194305) WHERE hash=?1 AND chunk_index=0",
            params![base_hash.as_slice()],
        )
        .unwrap();
    assert!(matches!(
        store.compare_history_with_current_root(1, LINEAGE_PROJECT_ID, &base),
        Err(ProofError::CorruptRootBlob)
    ));
    store
        .conn
        .execute(
            "UPDATE blob_chunks SET bytes=x'00' WHERE hash=?1 AND chunk_index=0",
            params![revision.as_slice()],
        )
        .unwrap();
    assert!(matches!(
        store.compare_history_with_current_root(1, LINEAGE_PROJECT_ID, &base),
        Err(ProofError::CorruptRootBlob)
    ));
    store
        .conn
        .execute("UPDATE root SET revision=zeroblob(4194305) WHERE id=1", [])
        .unwrap();
    assert!(matches!(
        store.compare_history_with_current_root(1, LINEAGE_PROJECT_ID, &base),
        Err(ProofError::CorruptRootBlob)
    ));
}

#[test]
#[ignore = "local maximum-manifest current-root history resolution cost probe"]
fn measure_maximum_root_history_resolution() {
    let fixture = Fixture::new();
    let mut store = RootTransactionProof::open(&fixture.database()).unwrap();
    let history_bytes = lineage_history(false);
    let history_hash: [u8; 32] = Sha256::digest(&history_bytes).into();
    let ordinary_bytes = [9_u8];
    let ordinary_hash: [u8; 32] = Sha256::digest(ordinary_bytes).into();
    let mut files = Vec::with_capacity(100_000);
    files.push(
        ManifestEntry::new(
            SourcePath::parse("schemas/history.registry.json").unwrap(),
            history_bytes.len() as u64,
            history_hash,
            Some(crate::RecordDescriptor::checked_parts(
                LINEAGE_HISTORY_ID,
                "core.registry-history",
            )),
        )
        .unwrap(),
    );
    files.extend((0_u32..99_999).map(|index| {
        ManifestEntry::new(
            SourcePath::parse(&format!("src/file-{index:06}.bin")).unwrap(),
            1,
            ordinary_hash,
            None,
        )
        .unwrap()
    }));
    let manifest = TreeManifest::new(LINEAGE_PROJECT_ID, Vec::new(), files).unwrap();
    store
        .stage_blob(
            &mut history_bytes.as_slice(),
            history_bytes.len() as u64,
            history_hash,
        )
        .unwrap();
    store
        .stage_blob(&mut ordinary_bytes.as_slice(), 1, ordinary_hash)
        .unwrap();
    store.stage_manifest_chunked(&manifest).unwrap();
    store
        .commit_manifest_probe(0, 1, 1, [1; 32], &manifest)
        .unwrap();
    let start = std::time::Instant::now();
    store
        .compare_history_with_current_root(1, LINEAGE_PROJECT_ID, &history_bytes)
        .unwrap();
    let elapsed = start.elapsed();
    eprintln!(
        "maximum_root_history_resolution files=100000 elapsed_ms={:.3}",
        elapsed.as_secs_f64() * 1000.0
    );
}

#[test]
#[ignore = "local maximum-manifest SQLite streaming cost probe"]
fn measure_maximum_manifest_sqlite_stream() {
    const PROJECT_ID: &str = "2a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    let fixture = Fixture::new();
    let files = (0..100_000)
        .map(|index| {
            ManifestEntry::new(
                SourcePath::parse(&format!("src/file-{index:06}.bin")).unwrap(),
                1,
                Sha256::digest([index as u8]).into(),
                None,
            )
            .unwrap()
        })
        .collect();
    let manifest = TreeManifest::new(PROJECT_ID, Vec::new(), files).unwrap();
    let revision = *manifest.revision().as_bytes();
    let mut store = RootTransactionProof::open(&fixture.database()).unwrap();
    let start = std::time::Instant::now();
    assert_eq!(store.stage_manifest_chunked(&manifest).unwrap(), revision);
    let elapsed = start.elapsed();
    let (length, chunks): (i64, i64) = store
        .conn
        .query_row(
            "SELECT length, chunk_count FROM blob_header WHERE hash=?1 AND state=1",
            params![revision.as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    eprintln!(
        "maximum_manifest_sqlite_stream files=100000 bytes={length} chunks={chunks} elapsed_ms={:.3}",
        elapsed.as_secs_f64() * 1000.0
    );
    assert!(chunks > 1);
}

#[test]
#[ignore = "local small-file SQLite transaction amplification probe"]
fn measure_small_blob_transaction_cost() {
    let fixture = Fixture::new();
    let mut store = RootTransactionProof::open(&fixture.database()).unwrap();
    let start = std::time::Instant::now();
    for index in 0_u64..1_000 {
        let bytes = index.to_le_bytes();
        let hash: [u8; 32] = Sha256::digest(bytes).into();
        store
            .stage_blob(&mut bytes.as_slice(), bytes.len() as u64, hash)
            .unwrap();
    }
    let elapsed = start.elapsed();
    eprintln!(
        "small_blob_transaction_cost files=1000 bytes=8000 elapsed_ms={:.3}",
        elapsed.as_secs_f64() * 1000.0
    );
}

#[test]
#[ignore = "local small-file batched SQLite cost probe"]
fn measure_small_blob_batch_cost() {
    let fixture = Fixture::new();
    let mut store = RootTransactionProof::open(&fixture.database()).unwrap();
    let files: Vec<_> = (0_u64..1_000)
        .map(|index| {
            let bytes = index.to_le_bytes();
            let hash: [u8; 32] = Sha256::digest(bytes).into();
            (hash, bytes)
        })
        .collect();
    let start = std::time::Instant::now();
    for group in files.chunks(SMALL_BATCH_FILES) {
        store
            .stage_small_blob_batch(group.iter().map(|(hash, bytes)| (*hash, bytes.as_slice())))
            .unwrap();
    }
    let elapsed = start.elapsed();
    eprintln!(
        "small_blob_batch_cost files=1000 bytes=8000 elapsed_ms={:.3}",
        elapsed.as_secs_f64() * 1000.0
    );
    let complete: i64 = store
        .conn
        .query_row(
            "SELECT count(*) FROM blob_header WHERE state=1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(complete, 1_000);
}

#[test]
#[ignore = "local verified small-blob reuse cost probe"]
fn measure_small_blob_reuse_cost() {
    let fixture = Fixture::new();
    let mut store = RootTransactionProof::open(&fixture.database()).unwrap();
    let files: Vec<_> = (0_u64..1_000)
        .map(|index| {
            let bytes = index.to_le_bytes();
            let hash: [u8; 32] = Sha256::digest(bytes).into();
            (hash, bytes)
        })
        .collect();
    for group in files.chunks(SMALL_BATCH_FILES) {
        store
            .stage_small_blob_batch(group.iter().map(|(hash, bytes)| (*hash, bytes.as_slice())))
            .unwrap();
    }
    let start = std::time::Instant::now();
    for (hash, bytes) in &files {
        store
            .stage_blob(&mut bytes.as_slice(), bytes.len() as u64, *hash)
            .unwrap();
    }
    let elapsed = start.elapsed();
    eprintln!(
        "small_blob_reuse_cost files=1000 bytes=8000 elapsed_ms={:.3} scratch_capacity={}",
        elapsed.as_secs_f64() * 1000.0,
        store.scratch.capacity()
    );
}
