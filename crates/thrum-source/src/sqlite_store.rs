//! Isolated SQLite transaction proof for the later native source store.
//!
//! Test-only: the fixtures do not bind a fully validated candidate, physical
//! reservation or GC pins, so this is not an accepted-source admission API.

use std::fmt;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use rusqlite::types::ValueRef;
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use sha2::{Digest, Sha256};

use crate::budget::{BudgetError, Limits};
use crate::history_json::{
    CarriedHistoryError, HistoryError, compare_carried_histories, parse_registry_history,
};
use crate::manifest_json::{ManifestDecodeError, decode_manifest_jcs};
use crate::registry_stage::{StagedCandidate, StagedRegistryError, validate_staged_candidate};
use crate::schema_json::parse_schema_descriptor;
use crate::stage::stage_declared_tree_with;
use crate::{ManifestEntry, ManifestError, SourcePath, StageError, TreeManifest};

const MAX_PAGE_COUNT: i64 = 13_631_488;
const CHUNK_BYTES: usize = 4 * 1024 * 1024;
const FIRST_PROFILE_MANIFEST_BYTES: usize = 64 * 1024 * 1024;
const MAX_HISTORY_BYTES: usize = 16 * 1024 * 1024;
const SMALL_BATCH_FILES: usize = 128;
const SMALL_BATCH_BYTES: usize = 1024 * 1024;
const SMALL_BATCH_FILE_BYTES: u64 = 4 * 1024;
const SQLITE_PAGE_BYTES: u64 = 4 * 1024;
const MIB: u64 = 1024 * 1024;
const GIB: u64 = 1024 * MIB;
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
    CorruptOutcome,
    Manifest(ManifestDecodeError),
    History(HistoryError),
    Lineage(CarriedHistoryError),
    CandidateStage(StagedRegistryError),
    CandidateManifest(ManifestError),
    CandidateFile(StageError),
    Budget(BudgetError),
    WalBackpressure,
}

impl fmt::Display for ProofError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sql(error) => write!(formatter, "SQLite: {error}"),
            Self::Manifest(error) => write!(formatter, "manifest: {error}"),
            Self::History(error) => write!(formatter, "history: {error:?}"),
            Self::Lineage(error) => write!(formatter, "lineage: {error:?}"),
            Self::CandidateStage(error) => write!(formatter, "candidate stage: {error:?}"),
            Self::CandidateManifest(error) => write!(formatter, "candidate manifest: {error:?}"),
            Self::CandidateFile(error) => write!(formatter, "candidate file: {error:?}"),
            Self::Budget(error) => write!(formatter, "physical preflight: {error:?}"),
            other => write!(formatter, "{other:?}"),
        }
    }
}

impl From<rusqlite::Error> for ProofError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sql(value)
    }
}

fn physical_io(path: &Path, source: io::Error) -> ProofError {
    ProofError::Budget(BudgetError::Io {
        path: path.to_owned(),
        source,
    })
}

fn next_stored_blob(
    rows: &mut rusqlite::Rows<'_>,
) -> Result<Option<([u8; 32], i64, i64)>, ProofError> {
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    let ValueRef::Blob(bytes) = row.get_ref(0)? else {
        return Err(ProofError::BlobMetadataMismatch);
    };
    let hash: [u8; 32] = bytes
        .try_into()
        .map_err(|_| ProofError::BlobMetadataMismatch)?;
    Ok(Some((hash, row.get(1)?, row.get(2)?)))
}

fn charge_projected_blob(
    projected_main: &mut u64,
    length: u64,
    stored: Option<(i64, i64)>,
) -> Result<(), ProofError> {
    if let Some((stored_length, 1)) = stored {
        if stored_length != i64::try_from(length).map_err(|_| ProofError::InputLength)? {
            return Err(ProofError::BlobMetadataMismatch);
        }
        return Ok(());
    }
    let rounded = length
        .div_ceil(SQLITE_PAGE_BYTES)
        .checked_mul(SQLITE_PAGE_BYTES)
        .ok_or(ProofError::Budget(BudgetError::Overflow))?;
    let chunks = length.div_ceil(CHUNK_BYTES as u64);
    *projected_main = projected_main
        .checked_add(
            rounded
                .checked_mul(2)
                .ok_or(ProofError::Budget(BudgetError::Overflow))?,
        )
        .and_then(|value| value.checked_add(16 * 1024))
        .and_then(|value| value.checked_add(chunks.checked_mul(SQLITE_PAGE_BYTES)?))
        .ok_or(ProofError::Budget(BudgetError::Overflow))?;
    Ok(())
}

fn required_free_bytes(
    projected_main: u64,
    remaining_wal: u64,
    checkpoint_debt: u64,
    free_floor: u64,
) -> Result<u64, ProofError> {
    projected_main
        .checked_add(remaining_wal)
        .and_then(|value| value.checked_add(checkpoint_debt))
        .and_then(|value| value.checked_add(free_floor))
        .ok_or(ProofError::Budget(BudgetError::Overflow))
}

struct RootTransactionProof {
    conn: Connection,
    scratch: Vec<u8>,
    path: PathBuf,
    wal_guard: WalGuard,
}

#[derive(Clone, Copy)]
struct WalGuard {
    passive_at: u64,
    next_passive_at: u64,
    stop_at: u64,
}

impl WalGuard {
    const V1: Self = Self {
        passive_at: GIB,
        next_passive_at: GIB,
        stop_at: 2 * GIB,
    };
}

struct OutcomeRow {
    request_hash: [u8; 32],
    generation: i64,
    revision: [u8; 32],
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

fn stored_outcome(
    conn: &Connection,
    epoch: i64,
    key: i64,
) -> Result<Option<OutcomeRow>, ProofError> {
    let raw = conn
        .query_row(
            "SELECT request_hash, generation, revision FROM outcome WHERE epoch=?1 AND key=?2",
            params![epoch, key],
            |row| {
                let request = match row.get_ref(0)? {
                    ValueRef::Blob(bytes) => bytes.try_into().ok(),
                    _ => None,
                };
                let revision = match row.get_ref(2)? {
                    ValueRef::Blob(bytes) => bytes.try_into().ok(),
                    _ => None,
                };
                Ok((request, row.get(1)?, revision))
            },
        )
        .optional()?;
    match raw {
        None => Ok(None),
        Some((Some(request_hash), generation, Some(revision))) => Ok(Some(OutcomeRow {
            request_hash,
            generation,
            revision,
        })),
        Some(_) => Err(ProofError::CorruptOutcome),
    }
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
             PRAGMA busy_timeout=0;
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
        let busy_timeout: i64 = conn.query_row("PRAGMA busy_timeout", [], |row| row.get(0))?;
        let max_page_count: i64 = conn.query_row("PRAGMA max_page_count", [], |row| row.get(0))?;
        assert_eq!(page_size, 4096);
        assert_eq!(auto_vacuum, 2);
        assert_eq!(journal_mode, "wal");
        assert_eq!(synchronous, 2);
        assert_eq!(foreign_keys, 1);
        assert_eq!(busy_timeout, 0);
        assert_eq!(max_page_count, MAX_PAGE_COUNT);
        Ok(Self {
            conn,
            scratch: Vec::new(),
            path: path.to_owned(),
            wal_guard: WalGuard::V1,
        })
    }

    fn ensure_wal_room(&mut self, incoming_allowance: u64) -> Result<(), ProofError> {
        if incoming_allowance >= self.wal_guard.stop_at {
            return Err(ProofError::WalBackpressure);
        }
        let wal_path = self.path.with_extension("sqlite-wal");
        let current = match fs::metadata(&wal_path) {
            Ok(metadata) => metadata.len(),
            Err(error) if error.kind() == io::ErrorKind::NotFound => 0,
            Err(source) => return Err(physical_io(&wal_path, source)),
        };
        if current >= self.wal_guard.next_passive_at {
            let _: (i64, i64, i64) =
                self.conn
                    .query_row("PRAGMA wal_checkpoint(PASSIVE)", [], |row| {
                        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                    })?;
            self.wal_guard.next_passive_at = current.saturating_add(64 * MIB);
        }
        if current
            .checked_add(incoming_allowance)
            .ok_or(ProofError::Budget(BudgetError::Overflow))?
            < self.wal_guard.stop_at
        {
            return Ok(());
        }
        let (busy, _, _): (i64, i64, i64) =
            self.conn
                .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                })?;
        let after = fs::metadata(&wal_path)
            .map_err(|source| physical_io(&wal_path, source))?
            .len();
        if busy != 0
            || after
                .checked_add(incoming_allowance)
                .ok_or(ProofError::Budget(BudgetError::Overflow))?
                >= self.wal_guard.stop_at
        {
            return Err(ProofError::WalBackpressure);
        }
        self.wal_guard.next_passive_at = self.wal_guard.passive_at;
        Ok(())
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

    fn admit_staged_candidate_probe(
        &mut self,
        expected_generation: i64,
        epoch: i64,
        key: i64,
        request_hash: [u8; 32],
        candidate: &StagedCandidate<'_>,
    ) -> Result<(i64, [u8; 32]), ProofError> {
        self.admit_staged_candidate_probe_with(
            expected_generation,
            epoch,
            key,
            request_hash,
            candidate,
            || {},
        )
    }

    fn matching_outcome(
        &self,
        epoch: i64,
        key: i64,
        request_hash: [u8; 32],
        revision: [u8; 32],
    ) -> Result<Option<i64>, ProofError> {
        match stored_outcome(&self.conn, epoch, key)? {
            None => Ok(None),
            Some(outcome)
                if outcome.request_hash == request_hash && outcome.revision == revision =>
            {
                Ok(Some(outcome.generation))
            }
            Some(_) => Err(ProofError::TokenConflict),
        }
    }

    fn admit_staged_candidate_probe_with(
        &mut self,
        expected_generation: i64,
        epoch: i64,
        key: i64,
        request_hash: [u8; 32],
        candidate: &StagedCandidate<'_>,
        after_initial_lookup: impl FnOnce(),
    ) -> Result<(i64, [u8; 32]), ProofError> {
        // Test fixtures use the selected main/WAL ceilings without assuming
        // CI has the production 8 GiB free-floor headroom.
        let limits = Limits {
            free_floor: 0,
            ..Limits::V1
        };
        self.admit_staged_candidate_probe_with_limits(
            expected_generation,
            epoch,
            key,
            request_hash,
            candidate,
            limits,
            after_initial_lookup,
        )
    }

    fn admit_staged_candidate_probe_with_limits(
        &mut self,
        expected_generation: i64,
        epoch: i64,
        key: i64,
        request_hash: [u8; 32],
        candidate: &StagedCandidate<'_>,
        limits: Limits,
        after_initial_lookup: impl FnOnce(),
    ) -> Result<(i64, [u8; 32]), ProofError> {
        let manifest = candidate
            .provisional_manifest()
            .map_err(ProofError::CandidateManifest)?;
        let revision = *manifest.revision().as_bytes();
        if let Some(generation) = self.matching_outcome(epoch, key, request_hash, revision)? {
            return Ok((generation, revision));
        }
        after_initial_lookup();
        let attempted = (|| {
            self.preflight_candidate_space(&manifest, revision, limits)?;
            if expected_generation > 0 {
                self.compare_staged_candidate_with_current_root(expected_generation, candidate)?;
            }
            self.stage_candidate_files(candidate)?;
            let stored_revision = self.stage_manifest_chunked(&manifest)?;
            if stored_revision != revision {
                return Err(ProofError::HashMismatch);
            }
            let generation = self.commit_manifest_probe(
                expected_generation,
                epoch,
                key,
                request_hash,
                &manifest,
            )?;
            Ok((generation, revision))
        })();
        match attempted {
            Ok(value) => Ok(value),
            Err(error) => {
                if let Some(generation) =
                    self.matching_outcome(epoch, key, request_hash, revision)?
                {
                    Ok((generation, revision))
                } else {
                    Err(error)
                }
            }
        }
    }

    fn preflight_candidate_space(
        &self,
        manifest: &TreeManifest,
        revision: [u8; 32],
        limits: Limits,
    ) -> Result<(), ProofError> {
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
        let mut blobs: Vec<_> = manifest
            .files()
            .iter()
            .map(ManifestEntry::blob_identity)
            .collect();
        blobs.push((count.length, revision));
        blobs.sort_unstable_by_key(|(_, hash)| *hash);
        if blobs
            .windows(2)
            .any(|pair| pair[0].1 == pair[1].1 && pair[0].0 != pair[1].0)
        {
            return Err(ProofError::BlobMetadataMismatch);
        }
        blobs.dedup_by_key(|(_, hash)| *hash);
        let count_limit = i64::try_from(blobs.len() + 1)
            .map_err(|_| ProofError::Budget(BudgetError::Overflow))?;
        let stored_count: i64 = self.conn.query_row(
            "SELECT count(*) FROM (SELECT 1 FROM blob_header LIMIT ?1)",
            params![count_limit],
            |row| row.get(0),
        )?;
        let mut projected_main = 64 * 1024_u64;
        if stored_count == 0 {
            for (length, _) in blobs {
                charge_projected_blob(&mut projected_main, length, None)?;
            }
        } else if usize::try_from(stored_count).is_ok_and(|count| count <= blobs.len()) {
            let mut statement = self
                .conn
                .prepare("SELECT hash, length, state FROM blob_header ORDER BY hash")?;
            let mut rows = statement.query([])?;
            let mut current = next_stored_blob(&mut rows)?;
            for (length, hash) in blobs {
                while matches!(current, Some((stored_hash, _, _)) if stored_hash < hash) {
                    current = next_stored_blob(&mut rows)?;
                }
                let stored = match current {
                    Some((stored_hash, stored_length, state)) if stored_hash == hash => {
                        Some((stored_length, state))
                    }
                    _ => None,
                };
                charge_projected_blob(&mut projected_main, length, stored)?;
            }
        } else {
            let mut statement = self
                .conn
                .prepare("SELECT length, state FROM blob_header WHERE hash=?1")?;
            for (length, hash) in blobs {
                let stored = statement
                    .query_row(params![hash.as_slice()], |row| {
                        Ok((row.get(0)?, row.get(1)?))
                    })
                    .optional()?;
                charge_projected_blob(&mut projected_main, length, stored)?;
            }
        }
        let pages: i64 = self
            .conn
            .query_row("PRAGMA page_count", [], |row| row.get(0))?;
        let logical_main = u64::try_from(pages)
            .map_err(|_| ProofError::Budget(BudgetError::Overflow))?
            .checked_mul(SQLITE_PAGE_BYTES)
            .ok_or(ProofError::Budget(BudgetError::Overflow))?;
        let physical_main = fs::metadata(&self.path)
            .map_err(|source| physical_io(&self.path, source))?
            .len();
        let main = physical_main.max(logical_main);
        if main
            .checked_add(projected_main)
            .ok_or(ProofError::Budget(BudgetError::Overflow))?
            > limits.main
        {
            return Err(ProofError::Budget(BudgetError::MainLimit));
        }
        let wal_path = self.path.with_extension("sqlite-wal");
        let wal = match fs::metadata(&wal_path) {
            Ok(metadata) => metadata.len(),
            Err(error) if error.kind() == io::ErrorKind::NotFound => 0,
            Err(source) => return Err(physical_io(&wal_path, source)),
        };
        if wal >= limits.wal {
            return Err(ProofError::Budget(BudgetError::WalLimit));
        }
        let required = required_free_bytes(
            projected_main,
            limits.wal - wal,
            logical_main.saturating_sub(physical_main),
            limits.free_floor,
        )?;
        let volume = self.path.parent().ok_or(ProofError::InputIo)?;
        let available =
            fs4::available_space(volume).map_err(|source| physical_io(volume, source))?;
        if available < required {
            return Err(ProofError::Budget(BudgetError::FreeSpace));
        }
        Ok(())
    }

    fn stage_candidate_files(&mut self, candidate: &StagedCandidate<'_>) -> Result<(), ProofError> {
        let stage = candidate.registry().stage();
        let mut entries: Vec<([u8; 32], std::ops::Range<usize>)> = Vec::new();
        let mut bytes = Vec::new();
        for file in stage.files() {
            if file.length() <= SMALL_BATCH_FILE_BYTES {
                let length = usize::try_from(file.length()).map_err(|_| ProofError::InputLength)?;
                if entries.len() == SMALL_BATCH_FILES || bytes.len() + length > SMALL_BATCH_BYTES {
                    self.flush_candidate_batch(&mut entries, &mut bytes)?;
                }
                let mut reader = stage.open(file.path()).map_err(ProofError::CandidateFile)?;
                let start = bytes.len();
                bytes.resize(start + length, 0);
                reader
                    .read_exact(&mut bytes[start..])
                    .map_err(|_| ProofError::InputIo)?;
                let mut extra = [0_u8; 1];
                if reader.read(&mut extra).map_err(|_| ProofError::InputIo)? != 0 {
                    return Err(ProofError::InputLength);
                }
                entries.push((*file.sha256(), start..bytes.len()));
            } else {
                self.flush_candidate_batch(&mut entries, &mut bytes)?;
                let mut reader = stage.open(file.path()).map_err(ProofError::CandidateFile)?;
                self.stage_blob(&mut reader, file.length(), *file.sha256())?;
            }
        }
        self.flush_candidate_batch(&mut entries, &mut bytes)
    }

    fn flush_candidate_batch(
        &mut self,
        entries: &mut Vec<([u8; 32], std::ops::Range<usize>)>,
        bytes: &mut Vec<u8>,
    ) -> Result<(), ProofError> {
        if !entries.is_empty() {
            self.stage_small_blob_batch(
                entries
                    .iter()
                    .map(|(hash, range)| (*hash, &bytes[range.clone()])),
            )?;
            entries.clear();
            bytes.clear();
        }
        Ok(())
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
        self.ensure_wal_room(5 * MIB)?;
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
        let complete: Option<(i64, i64)> = self
            .conn
            .query_row(
                "SELECT length, chunk_count FROM blob_header WHERE hash=?1 AND state=1",
                params![expected_hash.as_slice()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        if let Some((prior_length, prior_chunks)) = complete {
            if prior_length != sql_length || prior_chunks != chunk_count {
                return Err(ProofError::BlobMetadataMismatch);
            }
            return Ok(true);
        }
        self.ensure_wal_room(MIB)?;
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
        let rounded = (bytes.len() as u64)
            .div_ceil(SQLITE_PAGE_BYTES)
            .checked_mul(SQLITE_PAGE_BYTES)
            .ok_or(ProofError::Budget(BudgetError::Overflow))?;
        let allowance = rounded
            .checked_mul(2)
            .and_then(|value| value.checked_add(MIB))
            .ok_or(ProofError::Budget(BudgetError::Overflow))?;
        self.ensure_wal_room(allowance)?;
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
        self.ensure_wal_room(MIB)?;
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
        check_closure: impl FnMut(&Transaction<'_>) -> Result<(), ProofError>,
    ) -> Result<i64, ProofError> {
        self.commit_with_closure_after_sql(
            expected_generation,
            epoch,
            key,
            request_hash,
            revision,
            check_closure,
            || {},
        )
    }

    fn commit_with_closure_after_sql(
        &mut self,
        expected_generation: i64,
        epoch: i64,
        key: i64,
        request_hash: [u8; 32],
        revision: [u8; 32],
        mut check_closure: impl FnMut(&Transaction<'_>) -> Result<(), ProofError>,
        after_sql: impl FnOnce(),
    ) -> Result<i64, ProofError> {
        if let Some(outcome) = stored_outcome(&self.conn, epoch, key)? {
            return if outcome.request_hash == request_hash && outcome.revision == revision {
                Ok(outcome.generation)
            } else {
                Err(ProofError::TokenConflict)
            };
        }
        self.ensure_wal_room(MIB)?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing = stored_outcome(&tx, epoch, key)?;
        if let Some(outcome) = existing {
            return if outcome.request_hash == request_hash && outcome.revision == revision {
                Ok(outcome.generation)
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
        after_sql();
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

fn minimal_source_stage(fixture: &Fixture, label: &str) -> crate::StagedTree {
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
            "id":SCENE_ID,"type":"core.scene","schema":1,
            "fields":{"scene.label":label}
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

fn game_source_stage(fixture: &Fixture, caption: &str, game_scene_id: &str) -> crate::StagedTree {
    const SCENE_ID: &str = "3a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    const GAME_ID: &str = "4a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    const SCHEMA_ID: &str = "5a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    const HISTORY_ID: &str = "6a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    let root = fixture.source_root();
    for directory in ["records", "scenes", "schemas"] {
        fs::create_dir_all(root.join(directory)).unwrap();
    }
    fs::write(
        root.join("project.json"),
        serde_json::to_vec(&serde_json::json!({
            "id":LINEAGE_PROJECT_ID,"type":"core.project","schema":2,
            "fields":{
                "project.format":1,
                "project.records":["records/marker.record.json","scenes/main.scene.json"],
                "project.schemas":["schemas/marker.schema.json"],
                "project.registryHistory":"schemas/history.registry.json",
                "project.entryScenes":[SCENE_ID]
            }
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        root.join("records/marker.record.json"),
        format!(
            "{{\"id\":\"{GAME_ID}\",\"type\":\"game.marker\",\"schema\":1,\"fields\":{{\"marker.caption\":{},\"marker.scene\":\"{game_scene_id}\",\"x.game.marker.notes\":{{\"escaped\":\"\\u0061\", \"list\":[1,  2]}}}}}}",
            serde_json::to_string(caption).unwrap()
        ),
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
    let schema = serde_json::to_vec(&serde_json::json!({
        "id":SCHEMA_ID,"type":"core.schema","schema":1,
        "fields":{
            "schema.targetType":"game.marker","schema.targetVersion":1,
            "schema.fields":[
                {"id":"marker.caption","kind":"text","presence":"optional","maxUtf8Bytes":1024},
                {"id":"marker.scene","kind":"record-ref","presence":"required","targetType":"core.scene","resolution":"required"}
            ],
            "schema.optionalExtensionPrefix":"x.game.marker."
        }
    }))
    .unwrap();
    fs::write(root.join("schemas/marker.schema.json"), &schema).unwrap();
    let descriptor = parse_schema_descriptor(&schema).unwrap();
    let hex = |hash: &[u8; 32]| {
        let mut text = String::with_capacity(64);
        for byte in hash {
            use std::fmt::Write as _;
            write!(&mut text, "{byte:02x}").unwrap();
        }
        text
    };
    fs::write(
        root.join("schemas/history.registry.json"),
        serde_json::to_vec(&serde_json::json!({
            "id":HISTORY_ID,"type":"core.registry-history","schema":1,
            "fields":{
                "registry.fields":[
                    {"typeId":"game.marker","fieldId":"marker.caption","signature":hex(descriptor.fields()[0].signature()),"retired":false},
                    {"typeId":"game.marker","fieldId":"marker.scene","signature":hex(descriptor.fields()[1].signature()),"retired":false}
                ],
                "registry.versions":[
                    {"typeId":"game.marker","targetVersion":1,"definitionHash":hex(descriptor.definition_hash())}
                ]
            }
        }))
        .unwrap(),
    )
    .unwrap();
    let paths = [
        "project.json",
        "records/marker.record.json",
        "scenes/main.scene.json",
        "schemas/history.registry.json",
        "schemas/marker.schema.json",
    ]
    .map(|path| SourcePath::parse(path).unwrap());
    stage_declared_tree_with(
        &root,
        &paths,
        Limits {
            main: 2 * GIB,
            wal: 4 * MIB,
            temporary: 64 * MIB,
            free_floor: 0,
        },
        |_| {},
    )
    .unwrap()
}

#[test]
fn candidate_history_comes_from_rehashed_stage_and_reused_blobs_are_checked() {
    let fixture = Fixture::new();
    let stage = minimal_source_stage(&fixture, "first");
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

    let second_stage = minimal_source_stage(&fixture, "first");
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
fn staged_candidate_publication_keeps_prior_root_on_drift_and_recovers_retry() {
    let fixture = Fixture::new();
    let mut store = RootTransactionProof::open(&fixture.database()).unwrap();
    let first_stage = minimal_source_stage(&fixture, "first scene");
    let first = validate_staged_candidate(&first_stage, &[]).unwrap();
    let (first_generation, first_revision) = store
        .admit_staged_candidate_probe(0, 1, 1, [1; 32], &first)
        .unwrap();
    assert_eq!(first_generation, 1);

    let second_stage = minimal_source_stage(&fixture, "second scene");
    let second = validate_staged_candidate(&second_stage, &[]).unwrap();
    let (second_generation, second_revision) = store
        .admit_staged_candidate_probe(1, 1, 2, [2; 32], &second)
        .unwrap();
    assert_eq!(second_generation, 2);
    assert_ne!(first_revision, second_revision);
    assert_eq!(store.root().unwrap(), (2, Some(second_revision.to_vec())));
    drop(store);
    let mut store = RootTransactionProof::open(&fixture.database()).unwrap();
    assert_eq!(
        store
            .admit_staged_candidate_probe(1, 1, 2, [2; 32], &second)
            .unwrap(),
        (2, second_revision)
    );
    assert!(matches!(
        store.admit_staged_candidate_probe(1, 1, 2, [3; 32], &second),
        Err(ProofError::TokenConflict)
    ));
    assert!(matches!(
        store.admit_staged_candidate_probe(1, 1, 3, [3; 32], &first),
        Err(ProofError::StaleGeneration)
    ));

    let third_stage = minimal_source_stage(&fixture, "third scene!");
    let third = validate_staged_candidate(&third_stage, &[]).unwrap();
    let scene_path = third_stage.test_directory().join("scenes/main.scene.json");
    let mut changed = fs::read(&scene_path).unwrap();
    let last = changed.len() - 1;
    changed[last] = b' ';
    fs::write(scene_path, changed).unwrap();
    assert!(matches!(
        store.admit_staged_candidate_probe(2, 1, 4, [4; 32], &third),
        Err(ProofError::HashMismatch)
    ));
    assert_eq!(store.root().unwrap(), (2, Some(second_revision.to_vec())));
    let first_complete: bool = store
        .conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM blob_header WHERE hash=?1 AND state=1)",
            params![first_revision.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    assert!(first_complete);
}

#[test]
fn game_schema_candidate_reaches_root_with_exact_optional_bytes() {
    const SCENE_ID: &str = "3a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    const GAME_ID: &str = "4a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    let fixture = Fixture::new();
    let mut store = RootTransactionProof::open(&fixture.database()).unwrap();
    let first_stage = game_source_stage(&fixture, "first marker", SCENE_ID);
    let first = validate_staged_candidate(&first_stage, &[]).unwrap();
    assert_eq!(first.records().required_edges(), 1);
    assert_eq!(first.records().games()[0].summary().id(), GAME_ID);
    let (_, first_revision) = store
        .admit_staged_candidate_probe(0, 1, 1, [1; 32], &first)
        .unwrap();

    let second_stage = game_source_stage(&fixture, "second marker", SCENE_ID);
    let second = validate_staged_candidate(&second_stage, &[]).unwrap();
    let manifest = second.provisional_manifest().unwrap();
    let game = manifest
        .files()
        .iter()
        .find(|file| file.record_identity() == Some((GAME_ID, "game.marker")))
        .unwrap();
    assert_eq!(game.record_identity(), Some((GAME_ID, "game.marker")));
    let (generation, second_revision) = store
        .admit_staged_candidate_probe(1, 1, 2, [2; 32], &second)
        .unwrap();
    assert_eq!(generation, 2);
    assert_ne!(first_revision, second_revision);
    drop(store);
    let mut store = RootTransactionProof::open(&fixture.database()).unwrap();
    assert_eq!(store.root().unwrap(), (2, Some(second_revision.to_vec())));
    let game_bytes = {
        let tx = store
            .conn
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .unwrap();
        let bytes = read_complete_blob(&tx, game.blob_identity().1, MAX_HISTORY_BYTES).unwrap();
        tx.commit().unwrap();
        bytes
    };
    assert_eq!(
        game_bytes,
        fs::read(
            second_stage
                .test_directory()
                .join("records/marker.record.json")
        )
        .unwrap()
    );
    assert!(game_bytes.windows(6).any(|window| window == b"\\u0061"));
    assert_eq!(
        store
            .admit_staged_candidate_probe(1, 1, 2, [2; 32], &second)
            .unwrap(),
        (2, second_revision)
    );

    let invalid_stage = game_source_stage(&fixture, "invalid target", LINEAGE_PROJECT_ID);
    assert!(matches!(
        validate_staged_candidate(&invalid_stage, &[]),
        Err(crate::registry_stage::StagedCandidateError::Records(
            crate::registry_stage::StagedRecordError::UnresolvedGameScene { .. }
        ))
    ));
    assert_eq!(store.root().unwrap(), (2, Some(second_revision.to_vec())));
}

#[test]
#[ignore = "local game-schema candidate edit-to-root cost probe"]
fn measure_game_schema_candidate_edit_to_root() {
    const SCENE_ID: &str = "3a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    let fixture = Fixture::new();
    let mut store = RootTransactionProof::open(&fixture.database()).unwrap();
    let first_stage = game_source_stage(&fixture, "first marker", SCENE_ID);
    let first = validate_staged_candidate(&first_stage, &[]).unwrap();
    store
        .admit_staged_candidate_probe(0, 1, 1, [1; 32], &first)
        .unwrap();

    let start = Instant::now();
    let second_stage = game_source_stage(&fixture, "second marker", SCENE_ID);
    let second = validate_staged_candidate(&second_stage, &[]).unwrap();
    let (generation, _) = store
        .admit_staged_candidate_probe(1, 1, 2, [2; 32], &second)
        .unwrap();
    assert_eq!(generation, 2);
    eprintln!(
        "game_schema_candidate_edit_to_root files=5 elapsed_ms={:.3}",
        start.elapsed().as_secs_f64() * 1000.0
    );
}

#[test]
fn candidate_space_preflight_refuses_before_blob_write_and_counts_reuse_once() {
    let fixture = Fixture::new();
    let stage = minimal_source_stage(&fixture, "space check");
    let candidate = validate_staged_candidate(&stage, &[]).unwrap();
    let manifest = candidate.provisional_manifest().unwrap();
    let revision = *manifest.revision().as_bytes();
    let mut store = RootTransactionProof::open(&fixture.database()).unwrap();
    let limits = Limits {
        free_floor: 0,
        ..Limits::V1
    };
    assert!(matches!(
        store.admit_staged_candidate_probe_with_limits(
            0,
            1,
            1,
            [1; 32],
            &candidate,
            Limits {
                main: 4096,
                ..limits
            },
            || {},
        ),
        Err(ProofError::Budget(BudgetError::MainLimit))
    ));
    assert!(matches!(
        store.admit_staged_candidate_probe_with_limits(
            0,
            1,
            1,
            [1; 32],
            &candidate,
            Limits { wal: 1, ..limits },
            || {},
        ),
        Err(ProofError::Budget(BudgetError::WalLimit))
    ));
    let available = fs4::available_space(&fixture.0).unwrap();
    assert!(matches!(
        store.admit_staged_candidate_probe_with_limits(
            0,
            1,
            1,
            [1; 32],
            &candidate,
            Limits {
                free_floor: available,
                ..limits
            },
            || {},
        ),
        Err(ProofError::Budget(BudgetError::FreeSpace))
    ));
    let rows: i64 = store
        .conn
        .query_row("SELECT count(*) FROM blob_header", [], |row| row.get(0))
        .unwrap();
    assert_eq!(rows, 0);
    assert_eq!(store.root().unwrap(), (0, None));

    store
        .admit_staged_candidate_probe(0, 1, 1, [1; 32], &candidate)
        .unwrap();
    let pages: i64 = store
        .conn
        .query_row("PRAGMA page_count", [], |row| row.get(0))
        .unwrap();
    store
        .preflight_candidate_space(
            &manifest,
            revision,
            Limits {
                main: pages as u64 * SQLITE_PAGE_BYTES + 64 * 1024,
                ..limits
            },
        )
        .unwrap();
    let unrelated = [0x77_u8];
    let unrelated_hash: [u8; 32] = Sha256::digest(unrelated).into();
    store
        .stage_blob(&mut unrelated.as_slice(), 1, unrelated_hash)
        .unwrap();
    let pages: i64 = store
        .conn
        .query_row("PRAGMA page_count", [], |row| row.get(0))
        .unwrap();
    store
        .preflight_candidate_space(
            &manifest,
            revision,
            Limits {
                main: pages as u64 * SQLITE_PAGE_BYTES + 64 * 1024,
                ..limits
            },
        )
        .unwrap();
}

#[test]
fn candidate_space_preflight_reserves_existing_checkpoint_debt() {
    let fixture = Fixture::new();
    let store = RootTransactionProof::open(&fixture.database()).unwrap();
    let logical_pages: i64 = store
        .conn
        .query_row("PRAGMA page_count", [], |row| row.get(0))
        .unwrap();
    let physical_main = fs::metadata(fixture.database()).unwrap().len();
    let debt = (logical_pages as u64 * SQLITE_PAGE_BYTES).saturating_sub(physical_main);
    assert!(debt > 0);
    let without_debt = required_free_bytes(100_000, 1, 0, 8_000_000_000).unwrap();
    let with_debt = required_free_bytes(100_000, 1, debt, 8_000_000_000).unwrap();
    assert_eq!(with_debt - without_debt, debt);
    assert!(matches!(
        required_free_bytes(u64::MAX, 1, debt, 0),
        Err(ProofError::Budget(BudgetError::Overflow))
    ));
}

#[test]
fn blocked_reader_keeps_wal_checkpoint_debt_visible() {
    let fixture = Fixture::new();
    let mut writer = RootTransactionProof::open(&fixture.database()).unwrap();
    writer
        .conn
        .execute_batch("PRAGMA wal_autocheckpoint=0; PRAGMA busy_timeout=0;")
        .unwrap();
    let first = [0x41_u8];
    let first_hash: [u8; 32] = Sha256::digest(first).into();
    writer
        .stage_blob(&mut first.as_slice(), 1, first_hash)
        .unwrap();
    writer.commit_probe(0, 1, 1, [1; 32], first_hash).unwrap();
    let baseline: (i64, i64, i64) = writer
        .conn
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .unwrap();
    assert_eq!(baseline, (0, 0, 0));
    let reader = Connection::open(fixture.database()).unwrap();
    reader.execute_batch("BEGIN").unwrap();
    let _: i64 = reader
        .query_row("SELECT generation FROM root WHERE id=1", [], |row| {
            row.get(0)
        })
        .unwrap();

    let second = [0x42_u8];
    let second_hash: [u8; 32] = Sha256::digest(second).into();
    writer
        .stage_blob(&mut second.as_slice(), 1, second_hash)
        .unwrap();
    let wal_path = fixture.database().with_extension("sqlite-wal");
    let before = fs::metadata(&wal_path).unwrap().len();
    assert!(before > 0);
    let passive: (i64, i64, i64) = writer
        .conn
        .query_row("PRAGMA wal_checkpoint(PASSIVE)", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .unwrap();
    assert!(passive.1 > passive.2);
    writer.wal_guard = WalGuard {
        passive_at: 0,
        next_passive_at: 0,
        stop_at: before + MIB - 1,
    };
    let third = [0x43_u8];
    let third_hash: [u8; 32] = Sha256::digest(third).into();
    assert!(matches!(
        writer.stage_blob(&mut third.as_slice(), 1, third_hash),
        Err(ProofError::WalBackpressure)
    ));
    assert!(writer.wal_guard.next_passive_at > before);
    let staged_third: bool = writer
        .conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM blob_header WHERE hash=?1)",
            params![third_hash.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    assert!(!staged_third);
    writer
        .stage_blob(&mut second.as_slice(), 1, second_hash)
        .unwrap();
    assert_eq!(
        writer.commit_probe(0, 1, 1, [1; 32], first_hash).unwrap(),
        1
    );
    assert!(matches!(
        writer.commit_probe(0, 1, 1, [2; 32], first_hash),
        Err(ProofError::TokenConflict)
    ));
    assert_eq!(writer.root().unwrap().0, 1);
    let blocked: (i64, i64, i64) = writer
        .conn
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .unwrap();
    assert_eq!(blocked.0, 1);
    assert_eq!(fs::metadata(&wal_path).unwrap().len(), before);
    reader.execute_batch("COMMIT").unwrap();
    let drained: (i64, i64, i64) = writer
        .conn
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .unwrap();
    assert_eq!(drained, (0, 0, 0));
    assert_eq!(fs::metadata(&wal_path).unwrap().len(), 0);
    writer
        .stage_blob(&mut third.as_slice(), 1, third_hash)
        .unwrap();
}

struct KillChildOnDrop(Child);

impl Drop for KillChildOnDrop {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn signal_and_wait_for_kill(marker: &Path) -> ! {
    fs::write(marker, b"SQLite writes reached kill point").unwrap();
    loop {
        std::thread::sleep(Duration::from_secs(1));
    }
}

fn kill_after_marker(child: Child, marker: &Path, before_kill: impl FnOnce()) {
    let mut child = KillChildOnDrop(child);
    let deadline = Instant::now() + Duration::from_secs(5);
    while !marker.exists() {
        assert!(Instant::now() < deadline, "child did not reach kill point");
        assert!(child.0.try_wait().unwrap().is_none(), "child exited early");
        std::thread::sleep(Duration::from_millis(10));
    }
    before_kill();
    child.0.kill().unwrap();
    assert!(!child.0.wait().unwrap().success());
}

#[test]
fn process_kill_partial_blob_child() {
    let Some(database) = std::env::var_os("THRUM_TEST_PARTIAL_BLOB_DB") else {
        return;
    };
    let database = PathBuf::from(database);
    let marker = PathBuf::from(std::env::var_os("THRUM_TEST_PARTIAL_BLOB_MARKER").unwrap());
    let mut store = RootTransactionProof::open(&database).unwrap();
    let bytes = vec![0x42_u8; CHUNK_BYTES * 2 + 1];
    let hash: [u8; 32] = Sha256::digest(&bytes).into();
    store
        .stage_blob_with(&mut bytes.as_slice(), bytes.len() as u64, hash, |index| {
            if index == 0 {
                signal_and_wait_for_kill(&marker);
            }
        })
        .unwrap();
}

#[test]
fn process_kill_after_chunk_recovers_old_root_and_restages() {
    let fixture = Fixture::new();
    let mut store = RootTransactionProof::open(&fixture.database()).unwrap();
    let old = [0x41_u8];
    let old_hash: [u8; 32] = Sha256::digest(old).into();
    store.stage_blob(&mut old.as_slice(), 1, old_hash).unwrap();
    store.commit_probe(0, 1, 1, [1; 32], old_hash).unwrap();
    drop(store);

    let marker = fixture.0.join("first-chunk.ready");
    let child = Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("sqlite_store::process_kill_partial_blob_child")
        .arg("--nocapture")
        .env("THRUM_TEST_PARTIAL_BLOB_DB", fixture.database())
        .env("THRUM_TEST_PARTIAL_BLOB_MARKER", &marker)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    kill_after_marker(child, &marker, || {});

    let bytes = vec![0x42_u8; CHUNK_BYTES * 2 + 1];
    let hash: [u8; 32] = Sha256::digest(&bytes).into();
    let mut store = RootTransactionProof::open(&fixture.database()).unwrap();
    assert_eq!(store.root().unwrap(), (1, Some(old_hash.to_vec())));
    let (state, chunks): (i64, i64) = store
        .conn
        .query_row(
            "SELECT state, (SELECT count(*) FROM blob_chunks WHERE hash=?1) FROM blob_header WHERE hash=?1",
            params![hash.as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!((state, chunks), (0, 1));
    assert!(matches!(
        store.commit_probe(1, 1, 2, [2; 32], hash),
        Err(ProofError::MissingCompleteBlob)
    ));
    store
        .stage_blob(&mut bytes.as_slice(), bytes.len() as u64, hash)
        .unwrap();
    assert_eq!(store.commit_probe(1, 1, 2, [2; 32], hash).unwrap(), 2);
    drop(store);
    let store = RootTransactionProof::open(&fixture.database()).unwrap();
    assert_eq!(store.root().unwrap(), (2, Some(hash.to_vec())));
}

#[test]
fn process_kill_before_root_commit_child() {
    let Some(database) = std::env::var_os("THRUM_TEST_ROOT_TXN_DB") else {
        return;
    };
    let database = PathBuf::from(database);
    let marker = PathBuf::from(std::env::var_os("THRUM_TEST_ROOT_TXN_MARKER").unwrap());
    let mut store = RootTransactionProof::open(&database).unwrap();
    let revision: [u8; 32] = Sha256::digest(b"complete new root blob").into();
    store
        .commit_with_closure_after_sql(
            1,
            1,
            2,
            [2; 32],
            revision,
            |_| Ok(()),
            || {
                signal_and_wait_for_kill(&marker);
            },
        )
        .unwrap();
}

#[test]
fn process_kill_inside_root_transaction_rolls_back_root_and_outcome() {
    let fixture = Fixture::new();
    let mut store = RootTransactionProof::open(&fixture.database()).unwrap();
    let old = b"complete old root blob";
    let old_hash: [u8; 32] = Sha256::digest(old).into();
    store
        .stage_blob(&mut old.as_slice(), old.len() as u64, old_hash)
        .unwrap();
    store.commit_probe(0, 1, 1, [1; 32], old_hash).unwrap();
    let new = b"complete new root blob";
    let new_hash: [u8; 32] = Sha256::digest(new).into();
    store
        .stage_blob(&mut new.as_slice(), new.len() as u64, new_hash)
        .unwrap();
    drop(store);

    let marker = fixture.0.join("before-root-commit.ready");
    let child = Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("sqlite_store::process_kill_before_root_commit_child")
        .arg("--nocapture")
        .env("THRUM_TEST_ROOT_TXN_DB", fixture.database())
        .env("THRUM_TEST_ROOT_TXN_MARKER", &marker)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    kill_after_marker(child, &marker, || {
        let reader = Connection::open(fixture.database()).unwrap();
        let generation: i64 = reader
            .query_row("SELECT generation FROM root WHERE id=1", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(generation, 1);
        let outcome: bool = reader
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM outcome WHERE epoch=1 AND key=2)",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!outcome);
    });

    let mut store = RootTransactionProof::open(&fixture.database()).unwrap();
    assert_eq!(store.root().unwrap(), (1, Some(old_hash.to_vec())));
    let outcome: bool = store
        .conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM outcome WHERE epoch=1 AND key=2)",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(!outcome);
    assert_eq!(store.commit_probe(1, 1, 2, [2; 32], new_hash).unwrap(), 2);
    drop(store);
    let store = RootTransactionProof::open(&fixture.database()).unwrap();
    assert_eq!(store.root().unwrap(), (2, Some(new_hash.to_vec())));
    assert_eq!(
        stored_outcome(&store.conn, 1, 2)
            .unwrap()
            .unwrap()
            .generation,
        2
    );
}

#[test]
fn concurrent_same_token_winner_is_recovered_after_stale_preflight() {
    let fixture = Fixture::new();
    let mut first_store = RootTransactionProof::open(&fixture.database()).unwrap();
    let base_stage = minimal_source_stage(&fixture, "base scene");
    let base = validate_staged_candidate(&base_stage, &[]).unwrap();
    first_store
        .admit_staged_candidate_probe(0, 1, 1, [1; 32], &base)
        .unwrap();

    let next_stage = minimal_source_stage(&fixture, "next scene");
    let next = validate_staged_candidate(&next_stage, &[]).unwrap();
    let mut winner = RootTransactionProof::open(&fixture.database()).unwrap();
    let recovered = first_store
        .admit_staged_candidate_probe_with(1, 1, 2, [2; 32], &next, || {
            assert_eq!(
                winner
                    .admit_staged_candidate_probe(1, 1, 2, [2; 32], &next)
                    .unwrap()
                    .0,
                2
            );
        })
        .unwrap();
    assert_eq!(recovered.0, 2);
    assert_eq!(first_store.root().unwrap(), (2, Some(recovered.1.to_vec())));
    assert!(matches!(
        first_store.admit_staged_candidate_probe(1, 1, 2, [3; 32], &next),
        Err(ProofError::TokenConflict)
    ));
}

#[test]
fn assembled_candidate_refuses_to_drop_current_root_history() {
    let fixture = Fixture::new();
    let mut store = RootTransactionProof::open(&fixture.database()).unwrap();
    let prior_bytes = lineage_history(false);
    let prior_hash: [u8; 32] = Sha256::digest(&prior_bytes).into();
    let prior_manifest = lineage_manifest(&prior_bytes);
    store
        .stage_blob(
            &mut prior_bytes.as_slice(),
            prior_bytes.len() as u64,
            prior_hash,
        )
        .unwrap();
    let prior_revision = store.stage_manifest_chunked(&prior_manifest).unwrap();
    store
        .commit_manifest_probe(0, 1, 1, [1; 32], &prior_manifest)
        .unwrap();

    let candidate_stage = minimal_source_stage(&fixture, "empty history candidate");
    let candidate = validate_staged_candidate(&candidate_stage, &[]).unwrap();
    assert!(matches!(
        store.admit_staged_candidate_probe(1, 1, 2, [2; 32], &candidate),
        Err(ProofError::Lineage(_))
    ));
    assert_eq!(store.root().unwrap(), (1, Some(prior_revision.to_vec())));
}

#[test]
#[ignore = "local small candidate edit-to-root cost probe"]
fn measure_small_candidate_edit_to_root() {
    let fixture = Fixture::new();
    let mut store = RootTransactionProof::open(&fixture.database()).unwrap();
    let first_stage = minimal_source_stage(&fixture, "first scene");
    let first = validate_staged_candidate(&first_stage, &[]).unwrap();
    store
        .admit_staged_candidate_probe(0, 1, 1, [1; 32], &first)
        .unwrap();

    let start = std::time::Instant::now();
    let second_stage = minimal_source_stage(&fixture, "second scene");
    let second = validate_staged_candidate(&second_stage, &[]).unwrap();
    let (generation, revision) = store
        .admit_staged_candidate_probe(1, 1, 2, [2; 32], &second)
        .unwrap();
    let elapsed = start.elapsed();
    assert_eq!(generation, 2);
    assert_eq!(store.root().unwrap(), (2, Some(revision.to_vec())));
    eprintln!(
        "small_candidate_edit_to_root files=3 elapsed_ms={:.3}",
        elapsed.as_secs_f64() * 1000.0
    );
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

#[test]
#[ignore = "local SQLite main/WAL growth probe"]
fn measure_sqlite_physical_growth() {
    let fixture = Fixture::new();
    let mut store = RootTransactionProof::open(&fixture.database()).unwrap();
    store
        .conn
        .execute_batch("PRAGMA wal_autocheckpoint=0;")
        .unwrap();
    let snapshot = |label: &str, store: &RootTransactionProof| {
        let main = fs::metadata(fixture.database()).unwrap().len();
        let wal = fs::metadata(fixture.database().with_extension("sqlite-wal"))
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        let pages: i64 = store
            .conn
            .query_row("PRAGMA page_count", [], |row| row.get(0))
            .unwrap();
        let free: i64 = store
            .conn
            .query_row("PRAGMA freelist_count", [], |row| row.get(0))
            .unwrap();
        eprintln!("physical_growth {label} main={main} wal={wal} pages={pages} free={free}");
    };
    snapshot("schema", &store);
    let mut large = vec![0_u8; CHUNK_BYTES];
    for (index, byte) in large.iter_mut().enumerate() {
        *byte = (index as u64)
            .wrapping_mul(0x9e3779b97f4a7c15)
            .to_le_bytes()[3];
    }
    let hash: [u8; 32] = Sha256::digest(&large).into();
    store
        .stage_blob(&mut large.as_slice(), large.len() as u64, hash)
        .unwrap();
    snapshot("one_4mib_blob", &store);
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
    snapshot("plus_1000_tiny_blobs", &store);
    store
        .stage_blob(&mut large.as_slice(), large.len() as u64, hash)
        .unwrap();
    snapshot("after_complete_reuse", &store);
    let checkpoint: (i64, i64, i64) = store
        .conn
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .unwrap();
    eprintln!("physical_growth checkpoint={checkpoint:?}");
    snapshot("after_truncate", &store);

    let batch_fixture = Fixture::new();
    let mut batched = RootTransactionProof::open(&batch_fixture.database()).unwrap();
    batched
        .conn
        .execute_batch("PRAGMA wal_autocheckpoint=0;")
        .unwrap();
    for group in files.chunks(SMALL_BATCH_FILES) {
        batched
            .stage_small_blob_batch(group.iter().map(|(hash, bytes)| (*hash, bytes.as_slice())))
            .unwrap();
    }
    let batched_wal = fs::metadata(batch_fixture.database().with_extension("sqlite-wal"))
        .unwrap()
        .len();
    eprintln!("physical_growth batched_1000_tiny_wal={batched_wal}");

    let individual_fixture = Fixture::new();
    let mut individual = RootTransactionProof::open(&individual_fixture.database()).unwrap();
    individual
        .conn
        .execute_batch("PRAGMA wal_autocheckpoint=0;")
        .unwrap();
    for (hash, bytes) in &files {
        individual
            .stage_blob(&mut bytes.as_slice(), bytes.len() as u64, *hash)
            .unwrap();
    }
    let individual_wal = fs::metadata(individual_fixture.database().with_extension("sqlite-wal"))
        .unwrap()
        .len();
    eprintln!("physical_growth individual_1000_tiny_wal={individual_wal}");
}

#[test]
#[ignore = "local maximum-candidate physical preflight cost probe"]
fn measure_maximum_candidate_space_preflight() {
    const PROJECT_ID: &str = "2a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    let fixture = Fixture::new();
    let mut store = RootTransactionProof::open(&fixture.database()).unwrap();
    let files = (0_u64..100_000)
        .map(|index| {
            ManifestEntry::new(
                SourcePath::parse(&format!("src/file-{index:06}.bin")).unwrap(),
                1,
                Sha256::digest(index.to_le_bytes()).into(),
                None,
            )
            .unwrap()
        })
        .collect();
    let manifest = TreeManifest::new(PROJECT_ID, Vec::new(), files).unwrap();
    let revision = *manifest.revision().as_bytes();
    let start = std::time::Instant::now();
    store
        .preflight_candidate_space(
            &manifest,
            revision,
            Limits {
                free_floor: 0,
                ..Limits::V1
            },
        )
        .unwrap();
    eprintln!(
        "maximum_candidate_space_preflight fresh_files=100000 elapsed_ms={:.3}",
        start.elapsed().as_secs_f64() * 1000.0
    );
    {
        // Metadata-only rows isolate lookup cost; this is not a valid blob
        // closure or an admission benchmark.
        let tx = store
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        {
            let mut insert = tx
                .prepare(
                    "INSERT INTO blob_header (hash, length, chunk_count, state) VALUES (?1, ?2, 1, 1)",
                )
                .unwrap();
            for file in manifest.files() {
                let (length, hash) = file.blob_identity();
                insert
                    .execute(params![hash.as_slice(), length as i64])
                    .unwrap();
            }
        }
        tx.commit().unwrap();
    }
    let start = std::time::Instant::now();
    store
        .preflight_candidate_space(
            &manifest,
            revision,
            Limits {
                free_floor: 0,
                ..Limits::V1
            },
        )
        .unwrap();
    eprintln!(
        "maximum_candidate_space_preflight populated_files=100000 elapsed_ms={:.3}",
        start.elapsed().as_secs_f64() * 1000.0
    );
    let small = TreeManifest::new(
        PROJECT_ID,
        Vec::new(),
        vec![
            ManifestEntry::new(
                SourcePath::parse("src/file-000000.bin").unwrap(),
                1,
                Sha256::digest(0_u64.to_le_bytes()).into(),
                None,
            )
            .unwrap(),
        ],
    )
    .unwrap();
    let small_revision = *small.revision().as_bytes();
    let start = std::time::Instant::now();
    store
        .preflight_candidate_space(
            &small,
            small_revision,
            Limits {
                free_floor: 0,
                ..Limits::V1
            },
        )
        .unwrap();
    eprintln!(
        "maximum_candidate_space_preflight small_files=1 stored_rows=100000 elapsed_ms={:.3}",
        start.elapsed().as_secs_f64() * 1000.0
    );
}
