//! Append-only native SQLite source store and its transaction proofs.
//!
//! The first profile retains all verified blobs. Collection and physical disk
//! limits require separate qualification; planning targets are advisory.

#[cfg(test)]
use std::collections::HashSet;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::process::{Child, Command, Stdio};
#[cfg(test)]
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, RwLock};
#[cfg(test)]
use std::time::{Duration, Instant};

use fs4::{FileExt, TryLockError};
use rusqlite::types::ValueRef;
use rusqlite::{
    Connection, OpenFlags, OptionalExtension, Transaction, TransactionBehavior, config::DbConfig,
    params,
};
#[cfg(test)]
use rusqlite::{Statement, ToSql};

#[cfg(test)]
use crate::provisional_store_vfs_test as checkpoint_vfs;
use sha2::{Digest, Sha256};

use crate::budget::{BudgetError, Limits, SourceSpacePlan, open_regular_lock};
use crate::history_json::{
    CarriedHistoryError, HistoryError, compare_carried_histories, parse_registry_history,
};
use crate::manifest_json::{ManifestDecodeError, decode_manifest_jcs};
#[cfg(test)]
use crate::registry_stage::validate_staged_candidate;
use crate::registry_stage::{StagedCandidate, StagedRegistryError};
#[cfg(test)]
use crate::schema_json::parse_schema_descriptor;
#[cfg(test)]
use crate::stage::stage_declared_tree_with;
use crate::{
    LimitExceeded, ManifestEntry, ManifestError, SourceDimension, SourcePath, SourceRevision,
    StageError, TreeManifest,
};

const CHUNK_BYTES: usize = 4 * 1024 * 1024;
pub(crate) const FIRST_PROFILE_MANIFEST_BYTES: usize = 64 * 1024 * 1024;
const MAX_HISTORY_BYTES: usize = 16 * 1024 * 1024;
const SMALL_BATCH_FILES: usize = 128;
const SMALL_BATCH_BYTES: usize = 1024 * 1024;
const SMALL_BATCH_FILE_BYTES: u64 = 4 * 1024;
#[cfg(test)]
const CLOSURE_BATCH_FILES: usize = 128;
#[cfg(test)]
const GC_MARK_SCAN_BLOBS: usize = 4096;
#[cfg(test)]
const GC_MARK_CANDIDATES: usize = 128;
const SQLITE_PAGE_BYTES: u64 = 4 * 1024;
const SOURCE_STORE_FORMAT: i64 = 1;
const MIB: u64 = 1024 * 1024;

#[cfg(test)]
type ClosureRef = ([u8; 32], i64);

#[cfg(test)]
fn prepare_manifest_closure(manifest: &TreeManifest) -> Result<Vec<ClosureRef>, StoreError> {
    let mut refs = Vec::with_capacity(manifest.files().len());
    for file in manifest.files() {
        let (length, hash) = file.blob_identity();
        refs.push((
            hash,
            i64::try_from(length).map_err(|_| StoreError::InputLength)?,
        ));
    }
    refs.sort_unstable();
    refs.dedup();
    Ok(refs)
}

#[cfg(test)]
fn closure_batch_sql(count: usize) -> String {
    let mut sql = String::with_capacity(160 + count * 6);
    sql.push_str("WITH expected(hash, length) AS (VALUES ");
    for index in 0..count {
        if index != 0 {
            sql.push(',');
        }
        sql.push_str("(?,?)");
    }
    sql.push_str(
        ") SELECT EXISTS(SELECT 1 FROM expected AS e LEFT JOIN blob_header AS b ON b.hash=e.hash WHERE b.hash IS NULL OR b.length!=e.length OR b.state!=1)",
    );
    sql
}

#[cfg(test)]
fn check_closure_batch<'a>(
    statement: &mut Statement<'_>,
    batch: &'a [ClosureRef],
    params: &mut Vec<&'a dyn ToSql>,
) -> Result<(), StoreError> {
    params.clear();
    for (hash, length) in batch {
        params.push(hash);
        params.push(length);
    }
    let missing: i64 = statement.query_row(params.as_slice(), |row| row.get(0))?;
    if missing != 0 {
        return Err(StoreError::MissingCompleteFile);
    }
    Ok(())
}

#[cfg(test)]
fn check_prepared_manifest_closure(
    tx: &Transaction<'_>,
    refs: &[ClosureRef],
) -> Result<(), StoreError> {
    let mut params = Vec::with_capacity(CLOSURE_BATCH_FILES * 2);
    let (groups, remainder) = refs.as_chunks::<CLOSURE_BATCH_FILES>();
    let mut full = tx.prepare(&closure_batch_sql(CLOSURE_BATCH_FILES))?;
    for batch in groups {
        check_closure_batch(&mut full, batch, &mut params)?;
    }
    drop(full);
    if !remainder.is_empty() {
        let mut last = tx.prepare(&closure_batch_sql(remainder.len()))?;
        check_closure_batch(&mut last, remainder, &mut params)?;
    }
    Ok(())
}
#[cfg(test)]
const GIB: u64 = 1024 * MIB;
#[cfg(test)]
static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

#[cfg(test)]
struct Fixture(PathBuf);

#[cfg(test)]
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

#[cfg(test)]
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

/// One accepted-store policy or filesystem-availability refusal.
///
/// Current accounted bytes, work estimates, policy targets and filesystem
/// observations have distinct meanings. No variant reserves APFS blocks or
/// guarantees a future write will retain headroom.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StoreSpacePressure {
    /// Accounted current main bytes plus marginal candidate growth exceeds policy.
    MainTarget {
        /// Greater of observed main-file length and SQLite logical page bytes.
        accounted_current_bytes: u64,
        /// Estimated main growth for missing or incomplete candidate blobs.
        projected_growth_bytes: u64,
        /// Selected main-file target.
        target_bytes: u64,
    },
    /// Existing WAL file reached the selected stop before candidate writes.
    WalPreflight {
        /// Observed WAL file length.
        observed_wal_bytes: u64,
        /// Selected WAL stop target.
        stop_bytes: u64,
    },
    /// One shared-guard allowance itself reached the WAL stop before file stat.
    WalAllowance {
        /// Next requested write allowance.
        incoming_allowance_bytes: u64,
        /// Selected WAL stop target.
        stop_bytes: u64,
    },
    /// Shared guard could not make WAL room after a checkpoint attempt.
    WalCheckpoint {
        /// WAL file length observed after the checkpoint attempt.
        observed_wal_bytes: u64,
        /// Next requested write allowance.
        incoming_allowance_bytes: u64,
        /// Selected WAL stop target.
        stop_bytes: u64,
        /// SQLite reported a reader or writer still blocking the checkpoint.
        checkpoint_busy: bool,
    },
    /// Candidate preflight estimate exceeded observed filesystem availability.
    AvailablePreflight {
        /// Estimated marginal main growth for missing or incomplete blobs.
        projected_main_bytes: u64,
        /// Estimated marginal WAL growth within the current stop window.
        projected_wal_bytes: u64,
        /// Logical main pages not yet reflected in the main-file length.
        checkpoint_debt_bytes: u64,
        /// Non-atomic filesystem available-space observation.
        observed_available_bytes: u64,
        /// Selected desired free-space headroom.
        headroom_bytes: u64,
    },
    /// Shared live guard refused its next allowance against observed availability.
    AvailableLive {
        /// Next bounded write or checkpoint allowance.
        incoming_allowance_bytes: u64,
        /// Logical main pages not yet reflected in the main-file length.
        checkpoint_debt_bytes: u64,
        /// Non-atomic filesystem available-space observation.
        observed_available_bytes: u64,
        /// Selected desired free-space headroom.
        headroom_bytes: u64,
    },
}

#[derive(Debug)]
enum StoreError {
    Sql(rusqlite::Error),
    UncertainCommit(rusqlite::Error),
    RecoveryRequired,
    OwnerPoisoned,
    OwnerBusy,
    InputIo,
    OutputIo,
    InputLength,
    ManifestTooLarge(u64),
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
    MissingRootPin,
    #[cfg(test)]
    StaleGcEpoch,
    #[cfg(test)]
    GcMarkLimit,
    Manifest(ManifestDecodeError),
    History(HistoryError),
    Lineage(CarriedHistoryError),
    CandidateStage(StagedRegistryError),
    CandidateManifest(ManifestError),
    CandidateFile(StageError),
    Budget(BudgetError),
    SpacePressure(StoreSpacePressure),
    InvalidPlan,
    CorruptRootOutcome,
    MissingFile,
    MissingAcceptedRoot,
    UnsupportedStoreMode,
    ConsumerWrite(io::Error),
}

impl fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sql(error) => write!(formatter, "SQLite: {error}"),
            Self::UncertainCommit(error) => {
                write!(
                    formatter,
                    "SQLite final COMMIT uncertain; reopen to resolve outcome: {error}"
                )
            }
            Self::Manifest(error) => write!(formatter, "manifest: {error}"),
            Self::History(error) => write!(formatter, "history: {error:?}"),
            Self::Lineage(error) => write!(formatter, "lineage: {error:?}"),
            Self::CandidateStage(error) => write!(formatter, "candidate stage: {error:?}"),
            Self::CandidateManifest(error) => write!(formatter, "candidate manifest: {error:?}"),
            Self::ManifestTooLarge(length) => write!(
                formatter,
                "canonical manifest has {length} bytes; first accepted profile allows {FIRST_PROFILE_MANIFEST_BYTES}"
            ),
            Self::CandidateFile(error) => write!(formatter, "candidate file: {error:?}"),
            Self::Budget(error) => write!(formatter, "physical preflight: {error:?}"),
            Self::SpacePressure(detail) => write!(formatter, "store space pressure: {detail:?}"),
            Self::ConsumerWrite(error) => write!(formatter, "consumer output: {error}"),
            Self::OwnerBusy => write!(formatter, "another accepted-source owner holds the store"),
            other => write!(formatter, "{other:?}"),
        }
    }
}

impl From<rusqlite::Error> for StoreError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sql(value)
    }
}

fn physical_io(path: &Path, source: io::Error) -> StoreError {
    StoreError::Budget(BudgetError::Io {
        path: path.to_owned(),
        source,
    })
}

fn wal_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push("-wal");
    PathBuf::from(name)
}

fn next_stored_blob(
    rows: &mut rusqlite::Rows<'_>,
) -> Result<Option<([u8; 32], i64, i64)>, StoreError> {
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    let ValueRef::Blob(bytes) = row.get_ref(0)? else {
        return Err(StoreError::BlobMetadataMismatch);
    };
    let hash: [u8; 32] = bytes
        .try_into()
        .map_err(|_| StoreError::BlobMetadataMismatch)?;
    Ok(Some((hash, row.get(1)?, row.get(2)?)))
}

fn charge_projected_blob(
    projected_main: &mut u64,
    length: u64,
    stored: Option<(i64, i64)>,
) -> Result<(), StoreError> {
    if let Some((stored_length, 1)) = stored {
        if stored_length != i64::try_from(length).map_err(|_| StoreError::InputLength)? {
            return Err(StoreError::BlobMetadataMismatch);
        }
        return Ok(());
    }
    let rounded = length
        .div_ceil(SQLITE_PAGE_BYTES)
        .checked_mul(SQLITE_PAGE_BYTES)
        .ok_or(StoreError::Budget(BudgetError::Overflow))?;
    let chunks = length.div_ceil(CHUNK_BYTES as u64);
    *projected_main = projected_main
        .checked_add(
            rounded
                .checked_mul(2)
                .ok_or(StoreError::Budget(BudgetError::Overflow))?,
        )
        .and_then(|value| value.checked_add(16 * 1024))
        .and_then(|value| value.checked_add(chunks.checked_mul(SQLITE_PAGE_BYTES)?))
        .ok_or(StoreError::Budget(BudgetError::Overflow))?;
    Ok(())
}

fn required_free_bytes(
    projected_main: u64,
    projected_wal: u64,
    checkpoint_debt: u64,
    free_floor: u64,
) -> Result<u64, StoreError> {
    projected_main
        .checked_add(projected_wal)
        .and_then(|value| value.checked_add(checkpoint_debt))
        .and_then(|value| value.checked_add(free_floor))
        .ok_or(StoreError::Budget(BudgetError::Overflow))
}

fn assess_preflight_available(
    projected_main: u64,
    projected_wal: u64,
    checkpoint_debt: u64,
    headroom: u64,
    read_available: impl FnOnce() -> Result<u64, StoreError>,
) -> Result<(), StoreError> {
    let required = required_free_bytes(projected_main, projected_wal, checkpoint_debt, headroom)?;
    let available = read_available()?;
    if available < required {
        return Err(StoreError::SpacePressure(
            StoreSpacePressure::AvailablePreflight {
                projected_main_bytes: projected_main,
                projected_wal_bytes: projected_wal,
                checkpoint_debt_bytes: checkpoint_debt,
                observed_available_bytes: available,
                headroom_bytes: headroom,
            },
        ));
    }
    Ok(())
}

fn assess_live_available(
    incoming_allowance: u64,
    checkpoint_debt: u64,
    headroom: u64,
    read_available: impl FnOnce() -> Result<u64, StoreError>,
) -> Result<(), StoreError> {
    let required = headroom
        .checked_add(incoming_allowance)
        .and_then(|value| value.checked_add(checkpoint_debt))
        .ok_or(StoreError::Budget(BudgetError::Overflow))?;
    let available = read_available()?;
    if available < required {
        return Err(StoreError::SpacePressure(
            StoreSpacePressure::AvailableLive {
                incoming_allowance_bytes: incoming_allowance,
                checkpoint_debt_bytes: checkpoint_debt,
                observed_available_bytes: available,
                headroom_bytes: headroom,
            },
        ));
    }
    Ok(())
}

fn projected_wal_growth(projected_main: u64, remaining_window: u64) -> Result<u64, StoreError> {
    projected_main
        .checked_mul(2)
        .map(|work| work.min(remaining_window))
        .ok_or(StoreError::Budget(BudgetError::Overflow))
}

struct SqliteStoreCore {
    conn: Connection,
    recovery_required: bool,
    #[cfg(test)]
    scratch: Vec<u8>,
    path: PathBuf,
    wal_guard: WalGuard,
    pending_pin_releases: Vec<i64>,
    active_free_floor: Option<u64>,
}

#[derive(Clone, Copy)]
struct WalGuard {
    passive_at: u64,
    next_passive_at: u64,
    stop_at: u64,
}

impl WalGuard {
    fn for_plan(plan: SourceSpacePlan) -> Result<Self, StoreError> {
        if plan.main_target_bytes == 0
            || plan.wal_stop_bytes <= 2 * CHUNK_BYTES as u64 + MIB
            || plan.temporary_target_bytes == 0
        {
            return Err(StoreError::InvalidPlan);
        }
        let passive_at = plan.wal_stop_bytes / 2;
        Ok(Self {
            passive_at,
            next_passive_at: passive_at,
            stop_at: plan.wal_stop_bytes,
        })
    }
}

struct OutcomeRow {
    request_hash: [u8; 32],
    generation: i64,
    revision: [u8; 32],
}

struct RootReadPin {
    id: i64,
    generation: i64,
    revision: [u8; 32],
}

fn blob_chunk_count(length: u64) -> Result<i64, StoreError> {
    i64::try_from(length.div_ceil(CHUNK_BYTES as u64)).map_err(|_| StoreError::InputLength)
}

fn ensure_manifest_size(length: u64) -> Result<(), StoreError> {
    if length > FIRST_PROFILE_MANIFEST_BYTES as u64 {
        Err(StoreError::ManifestTooLarge(length))
    } else {
        Ok(())
    }
}

fn verify_supplied_blob(
    input: &mut impl Read,
    length: u64,
    expected_hash: [u8; 32],
    buffer: &mut [u8],
) -> Result<(), StoreError> {
    let mut digest = Sha256::new();
    let mut remaining = length;
    while remaining > 0 {
        let count = remaining.min(buffer.len() as u64) as usize;
        input
            .read_exact(&mut buffer[..count])
            .map_err(|_| StoreError::InputIo)?;
        digest.update(&buffer[..count]);
        remaining -= count as u64;
    }
    let mut extra = [0_u8; 1];
    if input.read(&mut extra).map_err(|_| StoreError::InputIo)? != 0 {
        return Err(StoreError::InputLength);
    }
    if digest.finalize().as_slice() != expected_hash {
        return Err(StoreError::HashMismatch);
    }
    Ok(())
}

fn root_identity(tx: &Transaction<'_>) -> Result<(i64, Option<[u8; 32]>), StoreError> {
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
    Ok((generation, revision.ok_or(StoreError::CorruptRootBlob)?))
}

fn verify_selected_outcome_head(
    conn: &Connection,
    generation: i64,
    revision: Option<[u8; 32]>,
) -> Result<(), StoreError> {
    // The retained outcome index makes this a top-two lookup, not a scan of
    // every historical publication on each owner open or root observation.
    let mut statement = conn.prepare(
        "SELECT generation, revision FROM outcome INDEXED BY outcome_generation ORDER BY generation DESC LIMIT 2",
    )?;
    let mut rows = statement.query([])?;
    let latest = rows.next()?;
    match (generation, revision, latest) {
        (0, None, None) => Ok(()),
        (1.., Some(selected_revision), Some(row)) => {
            let latest_generation: i64 = row.get(0)?;
            let latest_revision = match row.get_ref(1)? {
                ValueRef::Blob(bytes) => <[u8; 32]>::try_from(bytes).ok(),
                _ => None,
            };
            if latest_generation != generation || latest_revision != Some(selected_revision) {
                return Err(StoreError::CorruptRootOutcome);
            }
            if let Some(row) = rows.next()? {
                let second_generation: i64 = row.get(0)?;
                if second_generation == generation {
                    return Err(StoreError::CorruptRootOutcome);
                }
            }
            Ok(())
        }
        _ => Err(StoreError::CorruptRootOutcome),
    }
}

fn stored_outcome(
    conn: &Connection,
    epoch: i64,
    key: i64,
) -> Result<Option<OutcomeRow>, StoreError> {
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
        Some(_) => Err(StoreError::CorruptOutcome),
    }
}

fn read_complete_blob(
    tx: &Transaction<'_>,
    hash: [u8; 32],
    maximum_bytes: usize,
) -> Result<Vec<u8>, StoreError> {
    let (stored_length, stored_chunks, state): (i64, i64, i64) = tx
        .query_row(
            "SELECT length, chunk_count, state FROM blob_header WHERE hash=?1",
            params![hash.as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?
        .ok_or(StoreError::MissingCompleteBlob)?;
    let length = usize::try_from(stored_length).map_err(|_| StoreError::CorruptRootBlob)?;
    if state != 1 || length > maximum_bytes || stored_chunks != blob_chunk_count(length as u64)? {
        return Err(StoreError::CorruptRootBlob);
    }
    let actual_chunks: i64 = tx.query_row(
        "SELECT count(*) FROM blob_chunks WHERE hash=?1",
        params![hash.as_slice()],
        |row| row.get(0),
    )?;
    if actual_chunks != stored_chunks {
        return Err(StoreError::CorruptRootBlob);
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
            .ok_or(StoreError::CorruptRootBlob)?;
        if actual != expected as i64 {
            return Err(StoreError::CorruptRootBlob);
        }
        let chunk: Vec<u8> = tx
            .query_row(
                "SELECT bytes FROM blob_chunks WHERE hash=?1 AND chunk_index=?2",
                params![hash.as_slice(), index],
                |row| row.get(0),
            )
            .optional()?
            .ok_or(StoreError::CorruptRootBlob)?;
        digest.update(&chunk);
        bytes.extend_from_slice(&chunk);
    }
    if bytes.len() != length || digest.finalize().as_slice() != hash {
        return Err(StoreError::CorruptRootBlob);
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

struct ManifestBlobWriter<F> {
    append: F,
    buffer: Vec<u8>,
    digest: Sha256,
    length: u64,
    written: u64,
    hash: [u8; 32],
    chunk_count: i64,
    next_chunk: i64,
    failure: Option<StoreError>,
}

impl<F: FnMut(i64, &[u8]) -> Result<(), StoreError>> ManifestBlobWriter<F> {
    fn append_full_chunk(&mut self) -> io::Result<()> {
        if let Err(error) = (self.append)(self.next_chunk, &self.buffer) {
            self.failure = Some(error);
            return Err(io::Error::other("SQLite blob chunk write failed"));
        }
        self.next_chunk += 1;
        self.buffer.clear();
        Ok(())
    }

    fn finish(mut self) -> Result<(), StoreError> {
        if let Some(error) = self.failure.take() {
            return Err(error);
        }
        if self.written != self.length {
            return Err(StoreError::InputLength);
        }
        if !self.buffer.is_empty() {
            self.append_full_chunk()
                .map_err(|_| self.failure.take().unwrap_or(StoreError::OutputIo))?;
        }
        if self.next_chunk != self.chunk_count {
            return Err(StoreError::InvalidChunkLayout);
        }
        if self.digest.finalize().as_slice() != self.hash {
            return Err(StoreError::HashMismatch);
        }
        Ok(())
    }
}

impl<F: FnMut(i64, &[u8]) -> Result<(), StoreError>> Write for ManifestBlobWriter<F> {
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

impl SqliteStoreCore {
    fn require_clean(&self) -> Result<(), StoreError> {
        if self.recovery_required {
            Err(StoreError::RecoveryRequired)
        } else {
            Ok(())
        }
    }

    #[cfg(test)]
    fn open(path: &Path) -> Result<Self, StoreError> {
        Self::open_with_plan(path, SourceSpacePlan::DEFAULT)
    }

    #[cfg(test)]
    fn open_with_plan(path: &Path, plan: SourceSpacePlan) -> Result<Self, StoreError> {
        Self::open_with_plan_and_vfs(path, plan, None)
    }

    fn open_with_plan_and_vfs(
        path: &Path,
        plan: SourceSpacePlan,
        vfs: Option<&str>,
    ) -> Result<Self, StoreError> {
        let wal_guard = WalGuard::for_plan(plan)?;
        let mut existing = match fs::symlink_metadata(path) {
            Ok(metadata) if !metadata.file_type().is_file() => {
                return Err(StoreError::Budget(BudgetError::UnsafeStore(
                    path.to_owned(),
                )));
            }
            Ok(metadata) => metadata.len() > 0,
            Err(error) if error.kind() == io::ErrorKind::NotFound => false,
            Err(source) => return Err(physical_io(path, source)),
        };
        let conn = if let Some(vfs) = vfs {
            Connection::open_with_flags_and_vfs(path, OpenFlags::default(), vfs)?
        } else {
            Connection::open(path)?
        };
        // A reported final-COMMIT I/O error may follow a durable WAL frame.
        // Neither this connection's drop nor SQLite's automatic WAL threshold
        // may checkpoint or replace that uncertain frame before fresh open.
        if !conn.set_db_config(DbConfig::SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE, true)?
            || !conn.db_config(DbConfig::SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE)?
        {
            return Err(StoreError::UnsupportedStoreMode);
        }
        conn.execute_batch("PRAGMA wal_autocheckpoint=0;")?;
        let auto_checkpoint: i64 =
            conn.query_row("PRAGMA wal_autocheckpoint", [], |row| row.get(0))?;
        if auto_checkpoint != 0 {
            return Err(StoreError::UnsupportedStoreMode);
        }
        if existing {
            let format: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
            if format == 0 {
                let user_tables: i64 = conn.query_row(
                    "SELECT count(*) FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%'",
                    [],
                    |row| row.get(0),
                )?;
                if user_tables == 0 {
                    let pages: i64 = conn.query_row("PRAGMA page_count", [], |row| row.get(0))?;
                    let page_size: i64 =
                        conn.query_row("PRAGMA page_size", [], |row| row.get(0))?;
                    let auto_vacuum: i64 =
                        conn.query_row("PRAGMA auto_vacuum", [], |row| row.get(0))?;
                    let journal_mode: String =
                        conn.query_row("PRAGMA journal_mode", [], |row| row.get(0))?;
                    if pages == 0
                        || (page_size == SQLITE_PAGE_BYTES as i64
                            && auto_vacuum == 2
                            && journal_mode == "wal")
                    {
                        existing = false;
                    }
                }
            }
        }
        if existing {
            let page_size: i64 = conn.query_row("PRAGMA page_size", [], |row| row.get(0))?;
            let auto_vacuum: i64 = conn.query_row("PRAGMA auto_vacuum", [], |row| row.get(0))?;
            let journal_mode: String =
                conn.query_row("PRAGMA journal_mode", [], |row| row.get(0))?;
            let format: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
            if page_size != SQLITE_PAGE_BYTES as i64
                || auto_vacuum != 2
                || journal_mode != "wal"
                || format != SOURCE_STORE_FORMAT
            {
                return Err(StoreError::UnsupportedStoreMode);
            }
        }
        #[cfg(target_os = "macos")]
        conn.execute_batch("PRAGMA fullfsync=ON; PRAGMA checkpoint_fullfsync=ON;")?;
        conn.execute_batch(
            "PRAGMA page_size=4096;
             PRAGMA auto_vacuum=INCREMENTAL;
             PRAGMA journal_mode=WAL;
             PRAGMA synchronous=FULL;
             PRAGMA foreign_keys=ON;
             PRAGMA busy_timeout=0;",
        )?;
        if !existing {
            conn.execute_batch(
                "BEGIN IMMEDIATE;
             CREATE TABLE root (
               id INTEGER PRIMARY KEY CHECK (id=1),
               generation INTEGER NOT NULL,
               revision BLOB
             );
             CREATE TABLE outcome (
               epoch INTEGER NOT NULL,
               key INTEGER NOT NULL,
               request_hash BLOB NOT NULL,
               generation INTEGER NOT NULL,
               revision BLOB NOT NULL,
               PRIMARY KEY (epoch, key)
             );
             CREATE INDEX outcome_generation ON outcome (generation);
             CREATE TABLE read_pin (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               generation INTEGER NOT NULL,
               revision BLOB NOT NULL
             );
             CREATE TABLE gc_epoch (
               id INTEGER PRIMARY KEY CHECK (id=1),
               generation INTEGER NOT NULL
             );
             CREATE TABLE gc_mark (
               hash BLOB PRIMARY KEY,
               epoch INTEGER NOT NULL
             );
             CREATE TABLE blob_header (
               hash BLOB PRIMARY KEY,
               length INTEGER NOT NULL,
               chunk_count INTEGER NOT NULL,
               state INTEGER NOT NULL CHECK (state IN (0, 1))
             );
             CREATE INDEX complete_blob_hash
               ON blob_header (hash) WHERE state=1;
             CREATE TABLE blob_chunks (
               hash BLOB NOT NULL,
               chunk_index INTEGER NOT NULL,
               bytes BLOB NOT NULL,
               PRIMARY KEY (hash, chunk_index),
               FOREIGN KEY (hash) REFERENCES blob_header(hash)
             );
             INSERT INTO root (id, generation, revision) VALUES (1, 0, NULL);
             INSERT INTO gc_epoch (id, generation) VALUES (1, 0);
             PRAGMA user_version=1;
             COMMIT;",
            )?;
        }
        let page_size: i64 = conn.query_row("PRAGMA page_size", [], |row| row.get(0))?;
        let auto_vacuum: i64 = conn.query_row("PRAGMA auto_vacuum", [], |row| row.get(0))?;
        let journal_mode: String = conn.query_row("PRAGMA journal_mode", [], |row| row.get(0))?;
        let synchronous: i64 = conn.query_row("PRAGMA synchronous", [], |row| row.get(0))?;
        let foreign_keys: i64 = conn.query_row("PRAGMA foreign_keys", [], |row| row.get(0))?;
        let busy_timeout: i64 = conn.query_row("PRAGMA busy_timeout", [], |row| row.get(0))?;
        let format: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if page_size != 4096
            || auto_vacuum != 2
            || journal_mode != "wal"
            || synchronous != 2
            || foreign_keys != 1
            || busy_timeout != 0
            || format != SOURCE_STORE_FORMAT
        {
            return Err(StoreError::UnsupportedStoreMode);
        }
        #[cfg(target_os = "macos")]
        {
            let fullfsync: i64 = conn.query_row("PRAGMA fullfsync", [], |row| row.get(0))?;
            let checkpoint_fullfsync: i64 =
                conn.query_row("PRAGMA checkpoint_fullfsync", [], |row| row.get(0))?;
            if fullfsync != 1 || checkpoint_fullfsync != 1 {
                return Err(StoreError::UnsupportedStoreMode);
            }
        }
        Ok(Self {
            conn,
            recovery_required: false,
            #[cfg(test)]
            scratch: Vec::new(),
            path: path.to_owned(),
            wal_guard,
            pending_pin_releases: Vec::new(),
            active_free_floor: None,
        })
    }

    fn ensure_wal_room(&mut self, incoming_allowance: u64) -> Result<(), StoreError> {
        if incoming_allowance >= self.wal_guard.stop_at {
            return Err(StoreError::SpacePressure(
                StoreSpacePressure::WalAllowance {
                    incoming_allowance_bytes: incoming_allowance,
                    stop_bytes: self.wal_guard.stop_at,
                },
            ));
        }
        // A checkpoint can grow the main file before the next write starts.
        self.check_active_free(incoming_allowance)?;
        let wal_path = wal_path(&self.path);
        let current = match fs::metadata(&wal_path) {
            Ok(metadata) => metadata.len(),
            Err(error) if error.kind() == io::ErrorKind::NotFound => 0,
            Err(source) => return Err(physical_io(&wal_path, source)),
        };
        let checkpointed = current >= self.wal_guard.next_passive_at;
        if checkpointed {
            let _: (i64, i64, i64) =
                self.conn
                    .query_row("PRAGMA wal_checkpoint(PASSIVE)", [], |row| {
                        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                    })?;
            self.wal_guard.next_passive_at = current.saturating_add(64 * MIB);
        }
        if current
            .checked_add(incoming_allowance)
            .ok_or(StoreError::Budget(BudgetError::Overflow))?
            < self.wal_guard.stop_at
        {
            return if checkpointed {
                self.check_active_free(incoming_allowance)
            } else {
                Ok(())
            };
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
                .ok_or(StoreError::Budget(BudgetError::Overflow))?
                >= self.wal_guard.stop_at
        {
            return Err(StoreError::SpacePressure(
                StoreSpacePressure::WalCheckpoint {
                    observed_wal_bytes: after,
                    incoming_allowance_bytes: incoming_allowance,
                    stop_bytes: self.wal_guard.stop_at,
                    checkpoint_busy: busy != 0,
                },
            ));
        }
        self.wal_guard.next_passive_at = self.wal_guard.passive_at;
        self.check_active_free(incoming_allowance)
    }

    fn check_active_free(&self, incoming_allowance: u64) -> Result<(), StoreError> {
        let Some(floor) = self.active_free_floor else {
            return Ok(());
        };
        let pages: i64 = self
            .conn
            .query_row("PRAGMA page_count", [], |row| row.get(0))?;
        let logical_main = u64::try_from(pages)
            .map_err(|_| StoreError::Budget(BudgetError::Overflow))?
            .checked_mul(SQLITE_PAGE_BYTES)
            .ok_or(StoreError::Budget(BudgetError::Overflow))?;
        let physical_main = fs::metadata(&self.path)
            .map_err(|source| physical_io(&self.path, source))?
            .len();
        let checkpoint_debt = logical_main.saturating_sub(physical_main);
        let volume = self.path.parent().ok_or(StoreError::InputIo)?;
        assess_live_available(incoming_allowance, checkpoint_debt, floor, || {
            fs4::available_space(volume).map_err(|source| physical_io(volume, source))
        })
    }

    #[cfg(test)]
    fn root(&self) -> rusqlite::Result<(i64, Option<Vec<u8>>)> {
        self.conn.query_row(
            "SELECT generation, revision FROM root WHERE id=1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
    }

    fn pin_current_root(&mut self, expected_generation: i64) -> Result<RootReadPin, StoreError> {
        self.ensure_wal_room(MIB)?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (generation, revision) = root_identity(&tx)?;
        if generation != expected_generation {
            return Err(StoreError::StaleGeneration);
        }
        let revision = revision.ok_or(StoreError::MissingRootHistory)?;
        tx.execute(
            "INSERT INTO read_pin (generation, revision) VALUES (?1, ?2)",
            params![generation, revision.as_slice()],
        )?;
        tx.execute("UPDATE gc_epoch SET generation=generation+1 WHERE id=1", [])?;
        let id = tx.last_insert_rowid();
        tx.commit()?;
        Ok(RootReadPin {
            id,
            generation,
            revision,
        })
    }

    fn pin_accepted_root(&mut self, root: AcceptedSourceRoot) -> Result<RootReadPin, StoreError> {
        self.ensure_wal_room(MIB)?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let accepted: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM outcome WHERE generation=?1 AND revision=?2)",
            params![root.generation, root.revision.as_bytes().as_slice()],
            |row| row.get(0),
        )?;
        if !accepted {
            return Err(StoreError::MissingAcceptedRoot);
        }
        tx.execute(
            "INSERT INTO read_pin (generation, revision) VALUES (?1, ?2)",
            params![root.generation, root.revision.as_bytes().as_slice()],
        )?;
        tx.execute("UPDATE gc_epoch SET generation=generation+1 WHERE id=1", [])?;
        let id = tx.last_insert_rowid();
        tx.commit()?;
        Ok(RootReadPin {
            id,
            generation: root.generation,
            revision: *root.revision.as_bytes(),
        })
    }

    fn release_root_pin(&mut self, id: i64) -> Result<(), StoreError> {
        let released = self.release_root_pin_now(id);
        match &released {
            Ok(()) | Err(StoreError::MissingRootPin) => {
                self.pending_pin_releases.retain(|pending| *pending != id);
            }
            Err(_) if !self.pending_pin_releases.contains(&id) => {
                self.pending_pin_releases.push(id);
            }
            Err(_) => {}
        }
        released
    }

    fn release_root_pin_now(&mut self, id: i64) -> Result<(), StoreError> {
        self.require_clean()?;
        self.ensure_wal_room(MIB)?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if tx.execute("DELETE FROM read_pin WHERE id=?1", params![id])? != 1 {
            return Err(StoreError::MissingRootPin);
        }
        tx.execute("UPDATE gc_epoch SET generation=generation+1 WHERE id=1", [])?;
        tx.commit()?;
        Ok(())
    }

    fn retry_pending_pin_releases(&mut self) -> Result<(), StoreError> {
        self.require_clean()?;
        while let Some(&id) = self.pending_pin_releases.last() {
            match self.release_root_pin_now(id) {
                Ok(()) | Err(StoreError::MissingRootPin) => {
                    self.pending_pin_releases.pop();
                }
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    #[cfg(test)]
    fn mark_unreachable_probe(&mut self) -> Result<usize, StoreError> {
        self.mark_unreachable_probe_with(|| {})
    }

    #[cfg(test)]
    fn mark_unreachable_probe_with(
        &mut self,
        after_snapshot: impl FnOnce(),
    ) -> Result<usize, StoreError> {
        let (epoch, roots) = {
            let tx = self
                .conn
                .transaction_with_behavior(TransactionBehavior::Deferred)?;
            let epoch: i64 =
                tx.query_row("SELECT generation FROM gc_epoch WHERE id=1", [], |row| {
                    row.get(0)
                })?;
            let mut roots = Vec::new();
            if let (_, Some(revision)) = root_identity(&tx)? {
                roots.push(revision);
            }
            {
                let mut statement = tx.prepare("SELECT revision FROM read_pin")?;
                let mut rows = statement.query([])?;
                while let Some(row) = rows.next()? {
                    let ValueRef::Blob(bytes) = row.get_ref(0)? else {
                        return Err(StoreError::CorruptRootBlob);
                    };
                    roots.push(bytes.try_into().map_err(|_| StoreError::CorruptRootBlob)?);
                }
            }
            tx.commit()?;
            roots.sort_unstable();
            roots.dedup();
            (epoch, roots)
        };
        let mut reachable = HashSet::new();
        for revision in roots {
            reachable.insert(revision);
            let bytes = {
                let tx = self
                    .conn
                    .transaction_with_behavior(TransactionBehavior::Deferred)?;
                let bytes = read_complete_blob(&tx, revision, FIRST_PROFILE_MANIFEST_BYTES)?;
                tx.commit()?;
                bytes
            };
            let manifest = decode_manifest_jcs(&bytes, FIRST_PROFILE_MANIFEST_BYTES)
                .map_err(StoreError::Manifest)?;
            if !manifest.dependency_roots().is_empty() {
                return Err(StoreError::UnsupportedDependencyRoots);
            }
            for file in manifest.files() {
                reachable.insert(file.blob_identity().1);
            }
        }
        // Bound the read-side inventory and the later writer transaction. New
        // complete blobs staged after this snapshot simply wait for another
        // mark; a publication or pin change invalidates it below.
        let candidates = {
            let tx = self
                .conn
                .transaction_with_behavior(TransactionBehavior::Deferred)?;
            let mut candidates = Vec::new();
            {
                let mut scan =
                    tx.prepare("SELECT hash FROM blob_header WHERE state=1 ORDER BY hash")?;
                let mut rows = scan.query([])?;
                let mut scanned = 0;
                while let Some(row) = rows.next()? {
                    scanned += 1;
                    if scanned > GC_MARK_SCAN_BLOBS {
                        return Err(StoreError::GcMarkLimit);
                    }
                    let ValueRef::Blob(bytes) = row.get_ref(0)? else {
                        return Err(StoreError::BlobMetadataMismatch);
                    };
                    let hash: [u8; 32] = bytes
                        .try_into()
                        .map_err(|_| StoreError::BlobMetadataMismatch)?;
                    if !reachable.contains(&hash) {
                        candidates.push(hash);
                        if candidates.len() > GC_MARK_CANDIDATES {
                            return Err(StoreError::GcMarkLimit);
                        }
                    }
                }
            }
            tx.commit()?;
            candidates
        };
        after_snapshot();
        self.ensure_wal_room(8 * MIB)?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current_epoch: i64 =
            tx.query_row("SELECT generation FROM gc_epoch WHERE id=1", [], |row| {
                row.get(0)
            })?;
        if current_epoch != epoch {
            return Err(StoreError::StaleGcEpoch);
        }
        tx.execute("DELETE FROM gc_mark", [])?;
        if !candidates.is_empty() {
            let mut insert = tx.prepare("INSERT INTO gc_mark (hash, epoch) VALUES (?1, ?2)")?;
            for hash in &candidates {
                insert.execute(params![hash.as_slice(), epoch])?;
            }
        }
        tx.commit()?;
        Ok(candidates.len())
    }

    fn compare_history_with_current_root(
        &mut self,
        expected_generation: i64,
        candidate_project_id: &str,
        candidate_history_bytes: &[u8],
    ) -> Result<(), StoreError> {
        self.retry_pending_pin_releases()?;
        let pin = self.pin_current_root(expected_generation)?;
        let compared = self.compare_history_with_pinned_root(
            &pin,
            candidate_project_id,
            candidate_history_bytes,
        );
        self.release_root_pin(pin.id)?;
        compared
    }

    fn compare_history_with_pinned_root(
        &mut self,
        pin: &RootReadPin,
        candidate_project_id: &str,
        candidate_history_bytes: &[u8],
    ) -> Result<(), StoreError> {
        let manifest_bytes = {
            let tx = self
                .conn
                .transaction_with_behavior(TransactionBehavior::Deferred)?;
            let (generation, revision) = root_identity(&tx)?;
            if generation != pin.generation || revision != Some(pin.revision) {
                return Err(StoreError::StaleGeneration);
            }
            let bytes = read_complete_blob(&tx, pin.revision, FIRST_PROFILE_MANIFEST_BYTES)?;
            tx.commit()?;
            bytes
        };
        let manifest = decode_manifest_jcs(&manifest_bytes, FIRST_PROFILE_MANIFEST_BYTES)
            .map_err(StoreError::Manifest)?;
        if manifest.project_id() != candidate_project_id {
            return Err(StoreError::DifferentProject);
        }
        let mut histories = manifest.files().iter().filter_map(|file| {
            file.record_identity()
                .filter(|(_, record_type)| *record_type == "core.registry-history")
                .map(|(id, _)| (id, file.blob_identity()))
        });
        let (history_id, (history_length, history_hash)) =
            histories.next().ok_or(StoreError::MissingRootHistory)?;
        if histories.next().is_some() {
            return Err(StoreError::AmbiguousRootHistory);
        }
        let prior_bytes = {
            let tx = self
                .conn
                .transaction_with_behavior(TransactionBehavior::Deferred)?;
            let (generation, current_revision) = root_identity(&tx)?;
            if generation != pin.generation || current_revision != Some(pin.revision) {
                return Err(StoreError::StaleGeneration);
            }
            let bytes = read_complete_blob(&tx, history_hash, MAX_HISTORY_BYTES)?;
            tx.commit()?;
            bytes
        };
        if prior_bytes.len() as u64 != history_length {
            return Err(StoreError::CorruptRootBlob);
        }
        let prior = parse_registry_history(&prior_bytes).map_err(StoreError::History)?;
        if prior.record_id() != history_id {
            return Err(StoreError::CorruptRootBlob);
        }
        let candidate =
            parse_registry_history(candidate_history_bytes).map_err(StoreError::History)?;
        compare_carried_histories(&prior, &candidate).map_err(StoreError::Lineage)
    }

    fn compare_staged_candidate_with_current_root(
        &mut self,
        expected_generation: i64,
        candidate: &StagedCandidate<'_>,
    ) -> Result<(), StoreError> {
        let bytes = candidate
            .rehashed_history_bytes()
            .map_err(StoreError::CandidateStage)?;
        self.compare_history_with_current_root(
            expected_generation,
            candidate.registry().project().id(),
            &bytes,
        )
    }

    #[cfg(test)]
    fn admit_staged_candidate_probe(
        &mut self,
        expected_generation: i64,
        epoch: i64,
        key: i64,
        request_hash: [u8; 32],
        candidate: &StagedCandidate<'_>,
    ) -> Result<(i64, [u8; 32]), StoreError> {
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
    ) -> Result<Option<i64>, StoreError> {
        self.require_clean()?;
        match stored_outcome(&self.conn, epoch, key)? {
            None => Ok(None),
            Some(outcome)
                if outcome.request_hash == request_hash && outcome.revision == revision =>
            {
                Ok(Some(outcome.generation))
            }
            Some(_) => Err(StoreError::TokenConflict),
        }
    }

    #[cfg(test)]
    fn admit_staged_candidate_probe_with(
        &mut self,
        expected_generation: i64,
        epoch: i64,
        key: i64,
        request_hash: [u8; 32],
        candidate: &StagedCandidate<'_>,
        after_initial_lookup: impl FnOnce(),
    ) -> Result<(i64, [u8; 32]), StoreError> {
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
            |_| {},
            |_| {},
        )
    }

    #[cfg(test)]
    fn admit_staged_candidate_probe_with_limits(
        &mut self,
        expected_generation: i64,
        epoch: i64,
        key: i64,
        request_hash: [u8; 32],
        candidate: &StagedCandidate<'_>,
        limits: Limits,
        after_initial_lookup: impl FnOnce(),
        after_preflight: impl FnOnce(&mut Self),
        after_verified_stage: impl FnOnce(&mut Self),
    ) -> Result<(i64, [u8; 32]), StoreError> {
        let manifest = candidate
            .provisional_manifest()
            .map_err(StoreError::CandidateManifest)?;
        let revision = *manifest.revision().as_bytes();
        if let Some(generation) = self.matching_outcome(epoch, key, request_hash, revision)? {
            return Ok((generation, revision));
        }
        after_initial_lookup();
        let attempted = (|| {
            self.active_free_floor = Some(limits.free_floor);
            // An earlier bounded transaction may have crossed the planning
            // watermark. Drain it before the preflight's WAL-length refusal.
            self.ensure_wal_room(0)?;
            self.preflight_candidate_space(&manifest, revision, limits)?;
            after_preflight(self);
            if expected_generation > 0 {
                self.compare_staged_candidate_with_current_root(expected_generation, candidate)?;
            }
            self.stage_candidate_files(candidate)?;
            let stored_revision = self.stage_manifest_chunked(&manifest)?;
            if stored_revision != revision {
                return Err(StoreError::HashMismatch);
            }
            after_verified_stage(self);
            let generation = self.commit_manifest_probe(
                expected_generation,
                epoch,
                key,
                request_hash,
                &manifest,
            )?;
            Ok((generation, revision))
        })();
        self.active_free_floor = None;
        match attempted {
            Ok(value) => Ok(value),
            Err(error) => {
                if self.recovery_required {
                    return Err(error);
                }
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
    ) -> Result<(), StoreError> {
        let mut count = HashCountWriter {
            digest: Sha256::new(),
            length: 0,
        };
        manifest
            .write_jcs(&mut count)
            .map_err(|_| StoreError::OutputIo)?;
        ensure_manifest_size(count.length)?;
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
            return Err(StoreError::BlobMetadataMismatch);
        }
        blobs.dedup_by_key(|(_, hash)| *hash);
        let count_limit = i64::try_from(blobs.len() + 1)
            .map_err(|_| StoreError::Budget(BudgetError::Overflow))?;
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
            .map_err(|_| StoreError::Budget(BudgetError::Overflow))?
            .checked_mul(SQLITE_PAGE_BYTES)
            .ok_or(StoreError::Budget(BudgetError::Overflow))?;
        let physical_main = fs::metadata(&self.path)
            .map_err(|source| physical_io(&self.path, source))?
            .len();
        let main = physical_main.max(logical_main);
        if main
            .checked_add(projected_main)
            .ok_or(StoreError::Budget(BudgetError::Overflow))?
            > limits.main
        {
            return Err(StoreError::SpacePressure(StoreSpacePressure::MainTarget {
                accounted_current_bytes: main,
                projected_growth_bytes: projected_main,
                target_bytes: limits.main,
            }));
        }
        let wal_path = wal_path(&self.path);
        let wal = match fs::metadata(&wal_path) {
            Ok(metadata) => metadata.len(),
            Err(error) if error.kind() == io::ErrorKind::NotFound => 0,
            Err(source) => return Err(physical_io(&wal_path, source)),
        };
        if wal >= limits.wal {
            return Err(StoreError::SpacePressure(
                StoreSpacePressure::WalPreflight {
                    observed_wal_bytes: wal,
                    stop_bytes: limits.wal,
                },
            ));
        }
        // The WAL stop is backpressure capacity, not work demanded by this
        // candidate. Price only estimated writes for its missing blobs and
        // metadata, capped by the currently reachable WAL window. A later
        // checkpoint/live check still handles actual growth and blocked WAL.
        let projected_wal = projected_wal_growth(projected_main, limits.wal - wal)?;
        let checkpoint_debt = logical_main.saturating_sub(physical_main);
        let volume = self.path.parent().ok_or(StoreError::InputIo)?;
        assess_preflight_available(
            projected_main,
            projected_wal,
            checkpoint_debt,
            limits.free_floor,
            || fs4::available_space(volume).map_err(|source| physical_io(volume, source)),
        )
    }

    #[cfg(test)]
    fn stage_candidate_files(&mut self, candidate: &StagedCandidate<'_>) -> Result<(), StoreError> {
        let stage = candidate.registry().stage();
        let mut entries: Vec<([u8; 32], std::ops::Range<usize>)> = Vec::new();
        let mut bytes = Vec::new();
        for file in stage.files() {
            if file.length() <= SMALL_BATCH_FILE_BYTES {
                let length = usize::try_from(file.length()).map_err(|_| StoreError::InputLength)?;
                if entries.len() == SMALL_BATCH_FILES || bytes.len() + length > SMALL_BATCH_BYTES {
                    self.flush_candidate_batch(&mut entries, &mut bytes)?;
                }
                let mut reader = stage.open(file.path()).map_err(StoreError::CandidateFile)?;
                let start = bytes.len();
                bytes.resize(start + length, 0);
                reader
                    .read_exact(&mut bytes[start..])
                    .map_err(|_| StoreError::InputIo)?;
                let mut extra = [0_u8; 1];
                if reader.read(&mut extra).map_err(|_| StoreError::InputIo)? != 0 {
                    return Err(StoreError::InputLength);
                }
                entries.push((*file.sha256(), start..bytes.len()));
            } else {
                self.flush_candidate_batch(&mut entries, &mut bytes)?;
                let mut reader = stage.open(file.path()).map_err(StoreError::CandidateFile)?;
                self.stage_blob(&mut reader, file.length(), *file.sha256())?;
            }
        }
        self.flush_candidate_batch(&mut entries, &mut bytes)
    }

    fn flush_candidate_batch(
        &mut self,
        entries: &mut Vec<([u8; 32], std::ops::Range<usize>)>,
        bytes: &mut Vec<u8>,
    ) -> Result<(), StoreError> {
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

    #[cfg(test)]
    fn stage_blob(
        &mut self,
        input: &mut impl Read,
        length: u64,
        expected_hash: [u8; 32],
    ) -> Result<(), StoreError> {
        self.stage_blob_with(input, length, expected_hash, |_| {})
    }

    fn stage_small_blob_batch<'a>(
        &mut self,
        files: impl IntoIterator<Item = ([u8; 32], &'a [u8])>,
    ) -> Result<(), StoreError> {
        self.ensure_wal_room(5 * MIB)?;
        let mut malformed = None;
        let result = (|| {
            let tx = self
                .conn
                .transaction_with_behavior(TransactionBehavior::Immediate)?;
            let mut count = 0_usize;
            let mut total_bytes = 0_usize;
            for (hash, bytes) in files {
                count += 1;
                total_bytes = total_bytes
                    .checked_add(bytes.len())
                    .ok_or(StoreError::InputLength)?;
                if count > SMALL_BATCH_FILES
                    || total_bytes > SMALL_BATCH_BYTES
                    || bytes.len() as u64 > SMALL_BATCH_FILE_BYTES
                {
                    return Err(StoreError::InputLength);
                }
                if Sha256::digest(bytes).as_slice() != hash {
                    return Err(StoreError::HashMismatch);
                }
                let length = i64::try_from(bytes.len()).map_err(|_| StoreError::InputLength)?;
                let chunk_count = i64::from(!bytes.is_empty());
                let existing: Option<(i64, i64, i64)> = tx
                    .query_row(
                        "SELECT length, chunk_count, state FROM blob_header WHERE hash=?1",
                        params![hash.as_slice()],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                    )
                    .optional()?;
                let mut insert_chunk = !bytes.is_empty();
                match existing {
                    Some((old_length, old_chunks, 1)) => {
                        if old_length != length || old_chunks != chunk_count {
                            return Err(StoreError::BlobMetadataMismatch);
                        }
                        continue;
                    }
                    Some((old_length, old_chunks, 0)) => {
                        if old_length != length || old_chunks != chunk_count {
                            return Err(StoreError::BlobMetadataMismatch);
                        }
                        let prior = {
                            let mut statement = tx.prepare(
                                "SELECT chunk_index, length(bytes) FROM blob_chunks
                                 WHERE hash=?1 ORDER BY chunk_index LIMIT 2",
                            )?;
                            let mut rows = statement.query(params![hash.as_slice()])?;
                            let first = rows
                                .next()?
                                .map(|row| {
                                    Ok::<_, rusqlite::Error>((
                                        row.get::<_, i64>(0)?,
                                        row.get::<_, i64>(1)?,
                                    ))
                                })
                                .transpose()?;
                            let extra = rows.next()?.is_some();
                            (first, extra)
                        };
                        match prior {
                            (None, false) => {}
                            (Some((0, size)), false) if chunk_count == 1 && size == length => {
                                let matches: bool = tx.query_row(
                                    "SELECT bytes FROM blob_chunks WHERE hash=?1 AND chunk_index=0",
                                    params![hash.as_slice()],
                                    |row| {
                                        let ValueRef::Blob(stored) = row.get_ref(0)? else {
                                            return Ok(false);
                                        };
                                        Ok(stored == bytes)
                                    },
                                )?;
                                if !matches {
                                    tx.execute(
                                        "UPDATE blob_chunks SET bytes=?2 WHERE hash=?1 AND chunk_index=0",
                                        params![hash.as_slice(), bytes],
                                    )?;
                                }
                                insert_chunk = false;
                            }
                            _ => {
                                malformed = Some((hash, length, chunk_count));
                                return Err(StoreError::InvalidChunkLayout);
                            }
                        }
                    }
                    None => {
                        tx.execute(
                            "INSERT INTO blob_header (hash, length, chunk_count, state) VALUES (?1, ?2, ?3, 0)",
                            params![hash.as_slice(), length, chunk_count],
                        )?;
                    }
                    Some(_) => return Err(StoreError::BlobMetadataMismatch),
                }
                if insert_chunk {
                    tx.execute(
                        "INSERT INTO blob_chunks (hash, chunk_index, bytes) VALUES (?1, 0, ?2)",
                        params![hash.as_slice(), bytes],
                    )?;
                }
                if !bytes.is_empty() {
                    let stored: Vec<u8> = tx.query_row(
                        "SELECT bytes FROM blob_chunks WHERE hash=?1 AND chunk_index=0",
                        params![hash.as_slice()],
                        |row| row.get(0),
                    )?;
                    if stored.len() != bytes.len() || Sha256::digest(&stored).as_slice() != hash {
                        return Err(StoreError::HashMismatch);
                    }
                }
                tx.execute(
                    "UPDATE blob_header SET state=1 WHERE hash=?1 AND state=0",
                    params![hash.as_slice()],
                )?;
            }
            tx.commit()?;
            Ok(())
        })();
        if let Some((hash, length, chunk_count)) = malformed {
            // The batch rolled back before touching any selected state. Clear
            // the bad incomplete rows in bounded transactions for a retry.
            self.clear_incomplete_blob_chunks(hash, length, chunk_count)?;
        }
        result
    }

    #[cfg(test)]
    fn stage_manifest_chunked(&mut self, manifest: &TreeManifest) -> Result<[u8; 32], StoreError> {
        if !manifest.dependency_roots().is_empty() {
            return Err(StoreError::UnsupportedDependencyRoots);
        }
        let mut count = HashCountWriter {
            digest: Sha256::new(),
            length: 0,
        };
        manifest
            .write_jcs(&mut count)
            .map_err(|_| StoreError::OutputIo)?;
        let length = count.length;
        ensure_manifest_size(length)?;
        let hash: [u8; 32] = count.digest.finalize().into();
        let chunk_count = blob_chunk_count(length)?;
        if self.begin_blob(length, hash, chunk_count)? {
            return Ok(hash);
        }
        let mut writer = ManifestBlobWriter {
            append: |index: i64, bytes: &[u8]| {
                self.append_blob_chunk(length, hash, chunk_count, index, bytes)
            },
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
            return Err(writer.failure.take().unwrap_or(StoreError::OutputIo));
        }
        writer.finish()?;
        self.verify_and_complete_blob(length, hash, chunk_count, || {})?;
        Ok(hash)
    }

    #[cfg(test)]
    fn stage_blob_with(
        &mut self,
        input: &mut impl Read,
        length: u64,
        expected_hash: [u8; 32],
        after_chunk: impl FnMut(i64),
    ) -> Result<(), StoreError> {
        let mut scratch = std::mem::take(&mut self.scratch);
        let result =
            self.stage_blob_with_scratch(input, length, expected_hash, &mut scratch, after_chunk);
        self.scratch = scratch;
        result
    }

    #[cfg(test)]
    fn stage_blob_with_scratch(
        &mut self,
        input: &mut impl Read,
        length: u64,
        expected_hash: [u8; 32],
        scratch: &mut Vec<u8>,
        mut after_chunk: impl FnMut(i64),
    ) -> Result<(), StoreError> {
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
                .map_err(|_| StoreError::InputLength)?;
            input
                .read_exact(&mut scratch[..count])
                .map_err(|_| StoreError::InputIo)?;
            digest.update(&scratch[..count]);
            self.append_blob_chunk(length, expected_hash, chunk_count, index, &scratch[..count])?;
            remaining -= count as u64;
            after_chunk(index);
        }
        let mut extra = [0_u8; 1];
        if input.read(&mut extra).map_err(|_| StoreError::InputIo)? != 0 {
            return Err(StoreError::InputLength);
        }
        if digest.finalize().as_slice() != expected_hash {
            return Err(StoreError::HashMismatch);
        }

        self.verify_and_complete_blob(length, expected_hash, chunk_count, || {})
    }

    fn begin_blob(
        &mut self,
        length: u64,
        expected_hash: [u8; 32],
        chunk_count: i64,
    ) -> Result<bool, StoreError> {
        let sql_length = i64::try_from(length).map_err(|_| StoreError::InputLength)?;
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
                return Err(StoreError::BlobMetadataMismatch);
            }
            return Ok(true);
        }
        let existing: Option<(i64, i64, i64)> = self
            .conn
            .query_row(
                "SELECT length, chunk_count, state FROM blob_header WHERE hash=?1",
                params![expected_hash.as_slice()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        match existing {
            Some((prior_length, prior_chunks, 1)) => {
                if prior_length != sql_length || prior_chunks != chunk_count {
                    return Err(StoreError::BlobMetadataMismatch);
                }
                // The caller still verifies its candidate stage separately.
                return Ok(true);
            }
            Some((prior_length, prior_chunks, 0)) => {
                if prior_length != sql_length || prior_chunks != chunk_count {
                    return Err(StoreError::BlobMetadataMismatch);
                }
                if !self.partial_blob_has_contiguous_layout(length, expected_hash, chunk_count)? {
                    self.clear_incomplete_blob_chunks(expected_hash, sql_length, chunk_count)?;
                }
                return Ok(false);
            }
            None => {
                self.ensure_wal_room(MIB)?;
                let tx = self
                    .conn
                    .transaction_with_behavior(TransactionBehavior::Immediate)?;
                tx.execute(
                    "INSERT INTO blob_header (hash, length, chunk_count, state) VALUES (?1, ?2, ?3, 0)",
                    params![expected_hash.as_slice(), sql_length, chunk_count],
                )?;
                tx.commit()?;
            }
            Some(_) => return Err(StoreError::BlobMetadataMismatch),
        }
        Ok(false)
    }

    /// A partial blob is never selected. Its persisted chunks can be reused
    /// only as a contiguous prefix with the expected lengths; the caller also
    /// compares every byte with its immutable input before skipping an insert.
    fn partial_blob_has_contiguous_layout(
        &self,
        length: u64,
        hash: [u8; 32],
        chunk_count: i64,
    ) -> Result<bool, StoreError> {
        let mut statement = self.conn.prepare(
            "SELECT chunk_index, length(bytes) FROM blob_chunks WHERE hash=?1 ORDER BY chunk_index",
        )?;
        let mut rows = statement.query(params![hash.as_slice()])?;
        let mut index = 0_i64;
        while let Some(row) = rows.next()? {
            let actual_index: i64 = row.get(0)?;
            let actual_length: i64 = row.get(1)?;
            if actual_index != index || index >= chunk_count {
                return Ok(false);
            }
            let offset = u64::try_from(index)
                .map_err(|_| StoreError::InvalidChunkLayout)?
                .checked_mul(CHUNK_BYTES as u64)
                .ok_or(StoreError::InvalidChunkLayout)?;
            let expected_length = length.saturating_sub(offset).min(CHUNK_BYTES as u64);
            if actual_length != expected_length as i64 {
                return Ok(false);
            }
            index += 1;
        }
        Ok(true)
    }

    /// Delete at most one bounded chunk per transaction. A crash leaves the
    /// header incomplete and the next retry continues this idempotent cleanup.
    fn clear_incomplete_blob_chunks(
        &mut self,
        hash: [u8; 32],
        sql_length: i64,
        chunk_count: i64,
    ) -> Result<(), StoreError> {
        loop {
            let last: Option<(i64, i64)> = self
                .conn
                .query_row(
                    "SELECT chunk_index, length(bytes) FROM blob_chunks WHERE hash=?1 ORDER BY chunk_index DESC LIMIT 1",
                    params![hash.as_slice()],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?;
            let Some((index, stored_length)) = last else {
                return Ok(());
            };
            // A corrupt oversized row cannot be removed under this profile's
            // bounded-transaction claim; leave it unselectable for repair.
            let stored_length =
                u64::try_from(stored_length).map_err(|_| StoreError::InvalidChunkLayout)?;
            if stored_length > CHUNK_BYTES as u64 {
                return Err(StoreError::InvalidChunkLayout);
            }
            let rounded = stored_length
                .div_ceil(SQLITE_PAGE_BYTES)
                .checked_mul(SQLITE_PAGE_BYTES)
                .ok_or(StoreError::Budget(BudgetError::Overflow))?;
            let allowance = rounded
                .checked_mul(2)
                .and_then(|value| value.checked_add(MIB))
                .ok_or(StoreError::Budget(BudgetError::Overflow))?;
            self.ensure_wal_room(allowance)?;
            let tx = self
                .conn
                .transaction_with_behavior(TransactionBehavior::Immediate)?;
            let deleted = tx.execute(
                "DELETE FROM blob_chunks WHERE hash=?1 AND chunk_index=?2
                 AND EXISTS (SELECT 1 FROM blob_header
                   WHERE hash=?1 AND length=?3 AND chunk_count=?4 AND state=0)",
                params![hash.as_slice(), index, sql_length, chunk_count],
            )?;
            if deleted != 1 {
                return Err(StoreError::StaleBlobWriter);
            }
            tx.commit()?;
        }
    }

    fn append_blob_chunk(
        &mut self,
        length: u64,
        expected_hash: [u8; 32],
        chunk_count: i64,
        index: i64,
        bytes: &[u8],
    ) -> Result<(), StoreError> {
        let sql_length = i64::try_from(length).map_err(|_| StoreError::InputLength)?;
        let retained: Option<bool> = self
            .conn
            .query_row(
                "SELECT c.bytes FROM blob_chunks AS c JOIN blob_header AS h ON h.hash=c.hash
                 WHERE c.hash=?1 AND c.chunk_index=?2 AND h.length=?3 AND h.chunk_count=?4 AND h.state=0",
                params![expected_hash.as_slice(), index, sql_length, chunk_count],
                |row| {
                    let ValueRef::Blob(stored) = row.get_ref(0)? else {
                        return Ok(false);
                    };
                    Ok(stored == bytes)
                },
            )
            .optional()?;
        if retained == Some(true) {
            return Ok(());
        }
        let rounded = (bytes.len() as u64)
            .div_ceil(SQLITE_PAGE_BYTES)
            .checked_mul(SQLITE_PAGE_BYTES)
            .ok_or(StoreError::Budget(BudgetError::Overflow))?;
        let allowance = rounded
            .checked_mul(2)
            .and_then(|value| value.checked_add(MIB))
            .ok_or(StoreError::Budget(BudgetError::Overflow))?;
        self.ensure_wal_room(allowance)?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed = if retained == Some(false) {
            // The partial row is not authority. Repair only this bounded
            // chunk from the supplied stage; the full hash is checked again
            // before the complete marker can be published.
            tx.execute(
                "UPDATE blob_chunks SET bytes=?3 WHERE hash=?1 AND chunk_index=?2
                 AND EXISTS (SELECT 1 FROM blob_header
                   WHERE hash=?1 AND length=?4 AND chunk_count=?5 AND state=0)",
                params![
                    expected_hash.as_slice(),
                    index,
                    bytes,
                    sql_length,
                    chunk_count
                ],
            )?
        } else {
            tx.execute(
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
            )?
        };
        if changed != 1 {
            return Err(StoreError::StaleBlobWriter);
        }
        tx.commit()?;
        Ok(())
    }

    #[cfg(test)]
    fn verify_and_complete_blob(
        &mut self,
        length: u64,
        expected_hash: [u8; 32],
        chunk_count: i64,
        after_verify: impl FnOnce(),
    ) -> Result<(), StoreError> {
        self.ensure_wal_room(MIB)?;
        let sql_length = i64::try_from(length).map_err(|_| StoreError::InputLength)?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let actual_chunks: i64 = tx.query_row(
            "SELECT count(*) FROM blob_chunks WHERE hash=?1",
            params![expected_hash.as_slice()],
            |row| row.get(0),
        )?;
        if actual_chunks != chunk_count {
            return Err(StoreError::InvalidChunkLayout);
        }
        let mut stored = Sha256::new();
        let mut stored_length = 0_u64;
        for index in 0..chunk_count {
            let expected_count = usize::try_from((length - stored_length).min(CHUNK_BYTES as u64))
                .map_err(|_| StoreError::InputLength)?;
            let actual: i64 = tx
                .query_row(
                    "SELECT length(bytes) FROM blob_chunks WHERE hash=?1 AND chunk_index=?2",
                    params![expected_hash.as_slice(), index],
                    |row| row.get(0),
                )
                .optional()?
                .ok_or(StoreError::MissingChunk)?;
            if actual != expected_count as i64 {
                return Err(StoreError::InvalidChunkLayout);
            }
            let bytes: Vec<u8> = tx
                .query_row(
                    "SELECT bytes FROM blob_chunks WHERE hash=?1 AND chunk_index=?2",
                    params![expected_hash.as_slice(), index],
                    |row| row.get(0),
                )
                .optional()?
                .ok_or(StoreError::MissingChunk)?;
            stored_length = stored_length
                .checked_add(bytes.len() as u64)
                .ok_or(StoreError::InputLength)?;
            stored.update(&bytes);
        }
        if stored_length != length || stored.finalize().as_slice() != expected_hash {
            return Err(StoreError::HashMismatch);
        }
        after_verify();
        let updated = tx.execute(
            "UPDATE blob_header SET state=1 WHERE hash=?1 AND length=?2 AND chunk_count=?3 AND state=0",
            params![expected_hash.as_slice(), sql_length, chunk_count],
        )?;
        if updated != 1 {
            return Err(StoreError::BlobMetadataMismatch);
        }
        tx.commit()?;
        Ok(())
    }

    #[cfg(test)]
    fn commit_probe(
        &mut self,
        expected_generation: i64,
        epoch: i64,
        key: i64,
        request_hash: [u8; 32],
        revision: [u8; 32],
    ) -> Result<i64, StoreError> {
        self.commit_with_closure(
            expected_generation,
            epoch,
            key,
            request_hash,
            revision,
            |_| Ok(()),
            None,
        )
    }

    #[cfg(test)]
    fn commit_manifest_probe(
        &mut self,
        expected_generation: i64,
        epoch: i64,
        key: i64,
        request_hash: [u8; 32],
        manifest: &TreeManifest,
    ) -> Result<i64, StoreError> {
        if !manifest.dependency_roots().is_empty() {
            return Err(StoreError::UnsupportedDependencyRoots);
        }
        let mut count = HashCountWriter {
            digest: Sha256::new(),
            length: 0,
        };
        manifest
            .write_jcs(&mut count)
            .map_err(|_| StoreError::OutputIo)?;
        ensure_manifest_size(count.length)?;
        let revision: [u8; 32] = count.digest.finalize().into();
        self.commit_verified_manifest_probe(
            expected_generation,
            epoch,
            key,
            request_hash,
            revision,
            manifest,
            None,
        )
    }

    fn commit_verified_manifest_probe(
        &mut self,
        expected_generation: i64,
        epoch: i64,
        key: i64,
        request_hash: [u8; 32],
        revision: [u8; 32],
        manifest: &TreeManifest,
        publication_gate: Option<&RwLock<bool>>,
    ) -> Result<i64, StoreError> {
        if !manifest.dependency_roots().is_empty() {
            return Err(StoreError::UnsupportedDependencyRoots);
        }
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
                    let length = i64::try_from(length).map_err(|_| StoreError::InputLength)?;
                    if stored_length != Some(length) {
                        return Err(StoreError::MissingCompleteFile);
                    }
                }
                Ok(())
            },
            publication_gate,
        )
    }

    #[cfg(test)]
    fn commit_prepared_manifest_probe(
        &mut self,
        expected_generation: i64,
        epoch: i64,
        key: i64,
        request_hash: [u8; 32],
        revision: [u8; 32],
        refs: &[ClosureRef],
        publication_gate: Option<&RwLock<bool>>,
    ) -> Result<i64, StoreError> {
        self.commit_with_closure(
            expected_generation,
            epoch,
            key,
            request_hash,
            revision,
            |tx| check_prepared_manifest_closure(tx, refs),
            publication_gate,
        )
    }

    fn commit_with_closure(
        &mut self,
        expected_generation: i64,
        epoch: i64,
        key: i64,
        request_hash: [u8; 32],
        revision: [u8; 32],
        check_closure: impl FnMut(&Transaction<'_>) -> Result<(), StoreError>,
        publication_gate: Option<&RwLock<bool>>,
    ) -> Result<i64, StoreError> {
        self.commit_with_closure_after_sql(
            expected_generation,
            epoch,
            key,
            request_hash,
            revision,
            check_closure,
            publication_gate,
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
        mut check_closure: impl FnMut(&Transaction<'_>) -> Result<(), StoreError>,
        publication_gate: Option<&RwLock<bool>>,
        after_sql: impl FnOnce(),
    ) -> Result<i64, StoreError> {
        self.require_clean()?;
        if let Some(outcome) = stored_outcome(&self.conn, epoch, key)? {
            return if outcome.request_hash == request_hash && outcome.revision == revision {
                Ok(outcome.generation)
            } else {
                Err(StoreError::TokenConflict)
            };
        }
        self.ensure_wal_room(MIB)?;
        let mut tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing = stored_outcome(&tx, epoch, key)?;
        if let Some(outcome) = existing {
            return if outcome.request_hash == request_hash && outcome.revision == revision {
                Ok(outcome.generation)
            } else {
                Err(StoreError::TokenConflict)
            };
        }
        let generation: i64 =
            tx.query_row("SELECT generation FROM root WHERE id=1", [], |row| {
                row.get(0)
            })?;
        if generation != expected_generation {
            return Err(StoreError::StaleGeneration);
        }
        let complete: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM blob_header WHERE hash=?1 AND state=1)",
            params![revision.as_slice()],
            |row| row.get(0),
        )?;
        if !complete {
            return Err(StoreError::MissingCompleteBlob);
        }
        check_closure(&tx)?;
        // Existing pins may read on the owner's separate read-only connection
        // while closure is checked. Drain those short reads before publishing
        // root/outcome and hold the gate until the COMMIT outcome is known.
        let mut publication = publication_gate
            .map(|gate| gate.write().map_err(|_| StoreError::OwnerPoisoned))
            .transpose()?;
        let next = generation
            .checked_add(1)
            .ok_or(StoreError::StaleGeneration)?;
        tx.execute(
            "UPDATE root SET generation=?1, revision=?2 WHERE id=1",
            params![next, revision.as_slice()],
        )?;
        tx.execute(
            "INSERT INTO outcome (epoch, key, request_hash, generation, revision) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![epoch, key, request_hash.as_slice(), next, revision.as_slice()],
        )?;
        tx.execute("UPDATE gc_epoch SET generation=generation+1 WHERE id=1", [])?;
        after_sql();
        // A final COMMIT can report an I/O error after SQLite has written a
        // recoverable commit frame. Never let Transaction::drop roll it back,
        // then issue no more SQL through this connection until fresh open.
        tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
        if let Err(error) = tx.commit() {
            self.recovery_required = true;
            if let Some(ref mut quarantined) = publication {
                **quarantined = true;
            }
            return Err(StoreError::UncertainCommit(error));
        }
        Ok(next)
    }
}

/// One selected, durable authoring source root.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AcceptedSourceRoot {
    /// Monotone root generation, starting at one for the first accepted tree.
    pub generation: i64,
    /// Digest of the authenticated canonical tree manifest.
    pub revision: SourceRevision,
}

/// One non-atomic observation of the native SQLite store and its volume.
/// Apparent file lengths do not report allocated blocks or a reserved peak.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceStoreUsage {
    /// Current SQLite main-file length on the filesystem.
    pub main_file_bytes: u64,
    /// Current WAL-file length, zero when no WAL file is present.
    pub wal_file_bytes: u64,
    /// Page count times page size, including pages not yet checkpointed to main.
    pub logical_main_bytes: u64,
    /// Reusable pages within SQLite, not bytes returned to the volume.
    pub reusable_freelist_bytes: u64,
    /// Observed available bytes on the containing volume.
    pub volume_available_bytes: u64,
}

/// Broad failure category for source admission and verified reads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceStoreErrorKind {
    /// Owner cannot safely continue; discard it and reopen. A final COMMIT may be uncertain.
    RecoveryRequired,
    /// Another cooperating owner still holds this native source store.
    Busy,
    /// The selected planning target or observed free space deferred work.
    SpacePressure,
    /// Another root won the generation precondition.
    StaleGeneration,
    /// A retry key was already used for different work.
    TokenConflict,
    /// Candidate bytes or source-carried lineage failed validation.
    Candidate,
    /// Stored root, outcome, manifest or blob failed authentication.
    CorruptStore,
    /// The requested source path does not exist in the pinned tree.
    MissingFile,
    /// No durable admission outcome owns this root identity.
    MissingAcceptedRoot,
    /// The consumer's output writer refused bytes; discard its partial output.
    Output,
    /// The selected plan is unusable for bounded writes.
    InvalidPlan,
    /// SQLite or filesystem operation failed; retry only after inspecting it.
    Storage,
}

/// Failure from the native admitted-source store.
#[derive(Debug)]
pub struct SourceStoreError(StoreError);

impl SourceStoreError {
    /// Stable handling category; the display text carries the detail.
    pub fn kind(&self) -> SourceStoreErrorKind {
        match &self.0 {
            StoreError::RecoveryRequired
            | StoreError::UncertainCommit(_)
            | StoreError::OwnerPoisoned => SourceStoreErrorKind::RecoveryRequired,
            StoreError::OwnerBusy => SourceStoreErrorKind::Busy,
            StoreError::SpacePressure(_)
            | StoreError::Budget(BudgetError::StagePressure(_))
            | StoreError::Sql(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error {
                    code: rusqlite::ErrorCode::DiskFull,
                    ..
                },
                _,
            )) => SourceStoreErrorKind::SpacePressure,
            StoreError::StaleGeneration => SourceStoreErrorKind::StaleGeneration,
            StoreError::TokenConflict => SourceStoreErrorKind::TokenConflict,
            StoreError::CandidateStage(_)
            | StoreError::CandidateManifest(_)
            | StoreError::CandidateFile(_)
            | StoreError::Lineage(_)
            | StoreError::History(_)
            | StoreError::DifferentProject
            | StoreError::InputLength
            | StoreError::ManifestTooLarge(_)
            | StoreError::HashMismatch
            | StoreError::InputIo => SourceStoreErrorKind::Candidate,
            StoreError::InvalidPlan => SourceStoreErrorKind::InvalidPlan,
            StoreError::MissingFile => SourceStoreErrorKind::MissingFile,
            StoreError::MissingAcceptedRoot => SourceStoreErrorKind::MissingAcceptedRoot,
            StoreError::ConsumerWrite(_) => SourceStoreErrorKind::Output,
            StoreError::CorruptRootBlob
            | StoreError::CorruptRootOutcome
            | StoreError::UnsupportedStoreMode
            | StoreError::CorruptOutcome
            | StoreError::Manifest(_)
            | StoreError::MissingCompleteBlob
            | StoreError::MissingCompleteFile
            | StoreError::BlobMetadataMismatch
            | StoreError::InvalidChunkLayout
            | StoreError::MissingChunk
            | StoreError::MissingRootHistory
            | StoreError::AmbiguousRootHistory
            | StoreError::UnsupportedDependencyRoots => SourceStoreErrorKind::CorruptStore,
            _ => SourceStoreErrorKind::Storage,
        }
    }

    /// Specific content-capacity refusal, when the accepted store knows one.
    pub fn limit_exceeded(&self) -> Option<LimitExceeded> {
        match &self.0 {
            StoreError::ManifestTooLarge(length) => Some(LimitExceeded::exact(
                SourceDimension::AcceptedManifestBytes,
                FIRST_PROFILE_MANIFEST_BYTES as u64,
                *length,
            )),
            StoreError::CandidateManifest(error) => error.limit_exceeded(),
            StoreError::CandidateFile(error) => error.limit_exceeded(),
            StoreError::CandidateStage(error) => error.limit_exceeded(),
            StoreError::History(error) => error.limit_exceeded(),
            _ => None,
        }
    }

    /// Store-local estimate or filesystem sample that caused space pressure.
    pub fn space_pressure(&self) -> Option<StoreSpacePressure> {
        match &self.0 {
            StoreError::SpacePressure(detail) => Some(*detail),
            _ => None,
        }
    }
}

impl fmt::Display for SourceStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl std::error::Error for SourceStoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.0 {
            StoreError::Sql(error) | StoreError::UncertainCommit(error) => Some(error),
            StoreError::Budget(BudgetError::Io { source, .. })
            | StoreError::ConsumerWrite(source) => Some(source),
            _ => None,
        }
    }
}

impl From<StoreError> for SourceStoreError {
    fn from(error: StoreError) -> Self {
        Self(error)
    }
}

impl From<rusqlite::Error> for SourceStoreError {
    fn from(error: rusqlite::Error) -> Self {
        Self(StoreError::Sql(error))
    }
}

struct NativeSourceOwner {
    // Drop the reader before the writer, with the owner lock held through both.
    read_conn: Mutex<Connection>,
    read_gate: RwLock<bool>,
    inner: Mutex<SqliteStoreCore>,
    admission_scratch: Mutex<Vec<u8>>,
    _lock: File,
}

impl NativeSourceOwner {
    fn lock(&self) -> Result<MutexGuard<'_, SqliteStoreCore>, StoreError> {
        self.inner.lock().map_err(|_| StoreError::OwnerPoisoned)
    }

    fn admission(&self) -> Result<MutexGuard<'_, Vec<u8>>, StoreError> {
        self.admission_scratch
            .lock()
            .map_err(|_| StoreError::OwnerPoisoned)
    }

    fn with_read_conn<T>(
        &self,
        read: impl FnOnce(&Connection) -> Result<T, StoreError>,
    ) -> Result<T, StoreError> {
        let quarantined = self
            .read_gate
            .read()
            .map_err(|_| StoreError::OwnerPoisoned)?;
        if *quarantined {
            return Err(StoreError::RecoveryRequired);
        }
        let conn = self
            .read_conn
            .lock()
            .map_err(|_| StoreError::OwnerPoisoned)?;
        read(&conn)
    }
}

fn canonical_store_path(path: &Path) -> Result<PathBuf, StoreError> {
    if !path.is_absolute() {
        return Err(StoreError::Budget(BudgetError::UnsafeStore(
            path.to_owned(),
        )));
    }
    let parent = path.parent().ok_or(StoreError::InputIo)?;
    let parent = parent
        .canonicalize()
        .map_err(|source| physical_io(parent, source))?;
    if !parent.is_dir() {
        return Err(StoreError::Budget(BudgetError::UnsafeStore(parent)));
    }
    let file_name = path.file_name().ok_or(StoreError::InputIo)?;
    let database = parent.join(file_name);
    match fs::symlink_metadata(&database) {
        Ok(metadata) => {
            if !metadata.file_type().is_file() {
                return Err(StoreError::Budget(BudgetError::UnsafeStore(database)));
            }
            // Distinct names of one SQLite inode would derive distinct WAL and
            // sidecar paths. This profile admits only one-name regular files.
            #[cfg(unix)]
            if std::os::unix::fs::MetadataExt::nlink(&metadata) != 1 {
                return Err(StoreError::Budget(BudgetError::UnsafeStore(database)));
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(source) => return Err(physical_io(&database, source)),
    }
    Ok(database)
}

/// Native append-only source authority, outside world-update callbacks.
///
/// The path must be a trusted app-managed SQLite file. All accepted blobs are
/// retained in this first profile; no collector deletion runs here.
pub struct NativeSourceStore {
    owner: Arc<NativeSourceOwner>,
    plan: SourceSpacePlan,
}

impl NativeSourceStore {
    /// Open or create one native SQLite source store with a planning policy.
    ///
    /// A cooperating owner holds an exclusive sidecar lock until this handle
    /// and every accepted-source lease have closed. Opening the same store from
    /// another process or handle returns `Busy` before SQLite recovery runs.
    pub fn open(path: &Path, plan: SourceSpacePlan) -> Result<Self, SourceStoreError> {
        Self::open_with_vfs(path, plan, None)
    }

    fn open_with_vfs(
        path: &Path,
        plan: SourceSpacePlan,
        vfs: Option<&str>,
    ) -> Result<Self, SourceStoreError> {
        let database = canonical_store_path(path)?;
        let mut lock_name = database
            .file_name()
            .ok_or(StoreError::InputIo)?
            .to_os_string();
        lock_name.push(".owner.lock");
        let lock_path = database.with_file_name(lock_name);
        let lock = open_regular_lock(&lock_path).map_err(|error| match error {
            BudgetError::Busy => StoreError::OwnerBusy,
            other => StoreError::Budget(other),
        })?;
        match FileExt::try_lock(&lock) {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => return Err(StoreError::OwnerBusy.into()),
            Err(TryLockError::Error(source)) => {
                return Err(physical_io(&lock_path, source).into());
            }
        }
        let core = SqliteStoreCore::open_with_plan_and_vfs(&database, plan, vfs)?;
        let read_conn = if let Some(vfs) = vfs {
            Connection::open_with_flags_and_vfs(&database, OpenFlags::SQLITE_OPEN_READ_ONLY, vfs)?
        } else {
            Connection::open_with_flags(&database, OpenFlags::SQLITE_OPEN_READ_ONLY)?
        };
        if !read_conn.set_db_config(DbConfig::SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE, true)?
            || !read_conn.db_config(DbConfig::SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE)?
        {
            return Err(StoreError::UnsupportedStoreMode.into());
        }
        let store = Self {
            owner: Arc::new(NativeSourceOwner {
                read_conn: Mutex::new(read_conn),
                read_gate: RwLock::new(false),
                inner: Mutex::new(core),
                admission_scratch: Mutex::new(Vec::new()),
                _lock: lock,
            }),
            plan,
        };
        store.current_root()?;
        Ok(store)
    }

    /// Observe main/WAL lengths, logical pages and volume headroom.
    /// This optional tooling query samples several changing values, so it is
    /// neither an atomic snapshot nor a hard reservation.
    pub fn space_usage(&self) -> Result<SourceStoreUsage, SourceStoreError> {
        let inner = self.owner.lock()?;
        inner.require_clean()?;
        let pages: i64 = inner
            .conn
            .query_row("PRAGMA page_count", [], |row| row.get(0))?;
        let freelist: i64 = inner
            .conn
            .query_row("PRAGMA freelist_count", [], |row| row.get(0))?;
        let logical_main_bytes = u64::try_from(pages)
            .ok()
            .and_then(|value| value.checked_mul(SQLITE_PAGE_BYTES))
            .ok_or(StoreError::Budget(BudgetError::Overflow))?;
        let reusable_freelist_bytes = u64::try_from(freelist)
            .ok()
            .and_then(|value| value.checked_mul(SQLITE_PAGE_BYTES))
            .ok_or(StoreError::Budget(BudgetError::Overflow))?;
        let main_file_bytes = fs::metadata(&inner.path)
            .map_err(|source| physical_io(&inner.path, source))?
            .len();
        let wal = wal_path(&inner.path);
        let wal_file_bytes = match fs::metadata(&wal) {
            Ok(metadata) => metadata.len(),
            Err(error) if error.kind() == io::ErrorKind::NotFound => 0,
            Err(source) => return Err(physical_io(&wal, source).into()),
        };
        let volume = inner.path.parent().ok_or(StoreError::InputIo)?;
        let volume_available_bytes =
            fs4::available_space(volume).map_err(|source| physical_io(volume, source))?;
        Ok(SourceStoreUsage {
            main_file_bytes,
            wal_file_bytes,
            logical_main_bytes,
            reusable_freelist_bytes,
            volume_available_bytes,
        })
    }

    /// Read the selected root and verify its matching durable outcome.
    pub fn current_root(&self) -> Result<Option<AcceptedSourceRoot>, SourceStoreError> {
        let inner = self.owner.lock()?;
        inner.require_clean()?;
        let tx = inner
            .conn
            .unchecked_transaction()
            .map_err(StoreError::Sql)?;
        let (generation, revision) = root_identity(&tx)?;
        verify_selected_outcome_head(&tx, generation, revision)?;
        let selected = match (generation, revision) {
            (0, None) => None,
            (1.., Some(revision)) => Some(AcceptedSourceRoot {
                generation,
                revision: SourceRevision::from_bytes(revision),
            }),
            _ => return Err(StoreError::CorruptRootOutcome.into()),
        };
        tx.commit()?;
        Ok(selected)
    }

    /// Recover a committed admission after a lost response without reopening
    /// or reconstructing its staged candidate. A reused key with a different
    /// request hash is a conflict; an unknown key has no committed outcome.
    pub fn outcome(
        &self,
        epoch: i64,
        key: i64,
        request_hash: [u8; 32],
    ) -> Result<Option<AcceptedSourceRoot>, SourceStoreError> {
        let inner = self.owner.lock()?;
        inner.require_clean()?;
        let Some(row) = stored_outcome(&inner.conn, epoch, key)? else {
            return Ok(None);
        };
        if row.request_hash != request_hash {
            return Err(StoreError::TokenConflict.into());
        }
        Ok(Some(AcceptedSourceRoot {
            generation: row.generation,
            revision: SourceRevision::from_bytes(row.revision),
        }))
    }

    /// Admit a validated staged candidate under a root CAS and retry token.
    ///
    /// Every staged file is rehashed, prior history is authenticated, all
    /// referenced blobs are complete, and the root/outcome commit is atomic.
    /// Precommit refusal preserves the previous root. A reported final-COMMIT
    /// failure has an uncertain outcome: discard this owner, reopen the store
    /// and query the exact key before retrying or reporting selection.
    pub fn admit(
        &self,
        expected_generation: i64,
        epoch: i64,
        key: i64,
        request_hash: [u8; 32],
        candidate: &StagedCandidate<'_>,
    ) -> Result<AcceptedSourceRoot, SourceStoreError> {
        self.admit_with_probes(
            expected_generation,
            epoch,
            key,
            request_hash,
            candidate,
            || {},
            || {},
        )
    }

    fn admit_with_probes(
        &self,
        expected_generation: i64,
        epoch: i64,
        key: i64,
        request_hash: [u8; 32],
        candidate: &StagedCandidate<'_>,
        mut after_committed_chunk: impl FnMut(),
        before_final_commit: impl FnOnce(),
    ) -> Result<AcceptedSourceRoot, SourceStoreError> {
        let mut scratch = self.owner.admission()?;
        let manifest = candidate
            .provisional_manifest()
            .map_err(StoreError::CandidateManifest)?;
        let revision = *manifest.revision().as_bytes();
        {
            let inner = self.owner.lock()?;
            inner.require_clean()?;
            if let Some(generation) = inner.matching_outcome(epoch, key, request_hash, revision)? {
                return Ok(AcceptedSourceRoot {
                    generation,
                    revision: SourceRevision::from_bytes(revision),
                });
            }
        }
        let limits: Limits = self.plan.into();
        let attempted = (|| {
            {
                let mut inner = self.owner.lock()?;
                inner.require_clean()?;
                inner.active_free_floor = Some(limits.free_floor);
                inner.ensure_wal_room(0)?;
                inner.preflight_candidate_space(&manifest, revision, limits)?;
                if expected_generation > 0 {
                    inner.compare_staged_candidate_with_current_root(
                        expected_generation,
                        candidate,
                    )?;
                }
            }
            self.stage_candidate_files_phased(candidate, &mut scratch, &mut after_committed_chunk)?;
            let stored_revision = self.stage_manifest_phased(&manifest, &mut scratch)?;
            if stored_revision != revision {
                return Err(StoreError::HashMismatch);
            }
            let generation = {
                let mut inner = self.owner.lock()?;
                inner.require_clean()?;
                before_final_commit();
                inner.commit_verified_manifest_probe(
                    expected_generation,
                    epoch,
                    key,
                    request_hash,
                    revision,
                    &manifest,
                    Some(&self.owner.read_gate),
                )?
            };
            Ok(generation)
        })();
        let mut inner = self.owner.lock()?;
        inner.active_free_floor = None;
        let generation = match attempted {
            Ok(generation) => generation,
            Err(error) => {
                if inner.recovery_required {
                    return Err(error.into());
                }
                if let Some(generation) =
                    inner.matching_outcome(epoch, key, request_hash, revision)?
                {
                    generation
                } else {
                    return Err(error.into());
                }
            }
        };
        Ok(AcceptedSourceRoot {
            generation,
            revision: SourceRevision::from_bytes(revision),
        })
    }

    fn stage_candidate_files_phased(
        &self,
        candidate: &StagedCandidate<'_>,
        scratch: &mut Vec<u8>,
        after_committed_chunk: &mut impl FnMut(),
    ) -> Result<(), StoreError> {
        let stage = candidate.registry().stage();
        let mut entries: Vec<([u8; 32], std::ops::Range<usize>)> = Vec::new();
        let mut bytes = Vec::new();
        for file in stage.files() {
            if file.length() <= SMALL_BATCH_FILE_BYTES {
                let length = usize::try_from(file.length()).map_err(|_| StoreError::InputLength)?;
                if entries.len() == SMALL_BATCH_FILES || bytes.len() + length > SMALL_BATCH_BYTES {
                    self.owner
                        .lock()?
                        .flush_candidate_batch(&mut entries, &mut bytes)?;
                }
                let mut reader = stage.open(file.path()).map_err(StoreError::CandidateFile)?;
                let start = bytes.len();
                bytes.resize(start + length, 0);
                reader
                    .read_exact(&mut bytes[start..])
                    .map_err(|_| StoreError::InputIo)?;
                let mut extra = [0_u8; 1];
                if reader.read(&mut extra).map_err(|_| StoreError::InputIo)? != 0 {
                    return Err(StoreError::InputLength);
                }
                entries.push((*file.sha256(), start..bytes.len()));
            } else {
                self.owner
                    .lock()?
                    .flush_candidate_batch(&mut entries, &mut bytes)?;
                let mut reader = stage.open(file.path()).map_err(StoreError::CandidateFile)?;
                self.stage_blob_phased(
                    &mut reader,
                    file.length(),
                    *file.sha256(),
                    scratch,
                    after_committed_chunk,
                )?;
            }
        }
        self.owner
            .lock()?
            .flush_candidate_batch(&mut entries, &mut bytes)
    }

    fn stage_manifest_phased(
        &self,
        manifest: &TreeManifest,
        scratch: &mut Vec<u8>,
    ) -> Result<[u8; 32], StoreError> {
        if !manifest.dependency_roots().is_empty() {
            return Err(StoreError::UnsupportedDependencyRoots);
        }
        let mut count = HashCountWriter {
            digest: Sha256::new(),
            length: 0,
        };
        manifest
            .write_jcs(&mut count)
            .map_err(|_| StoreError::OutputIo)?;
        let length = count.length;
        ensure_manifest_size(length)?;
        let hash: [u8; 32] = count.digest.finalize().into();
        let chunk_count = blob_chunk_count(length)?;
        if self.owner.lock()?.begin_blob(length, hash, chunk_count)? {
            return Ok(hash);
        }
        let mut writer = ManifestBlobWriter {
            append: |index: i64, bytes: &[u8]| {
                self.owner
                    .lock()?
                    .append_blob_chunk(length, hash, chunk_count, index, bytes)
            },
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
            return Err(writer.failure.take().unwrap_or(StoreError::OutputIo));
        }
        writer.finish()?;
        self.verify_and_complete_blob_phased(length, hash, chunk_count, scratch)?;
        Ok(hash)
    }

    fn stage_blob_phased(
        &self,
        input: &mut impl Read,
        length: u64,
        hash: [u8; 32],
        scratch: &mut Vec<u8>,
        after_committed_chunk: &mut impl FnMut(),
    ) -> Result<(), StoreError> {
        let chunk_count = blob_chunk_count(length)?;
        scratch.resize(length.min(CHUNK_BYTES as u64).max(1) as usize, 0);
        if self.owner.lock()?.begin_blob(length, hash, chunk_count)? {
            return verify_supplied_blob(input, length, hash, scratch);
        }
        let mut digest = Sha256::new();
        let mut remaining = length;
        for index in 0..chunk_count {
            let count = usize::try_from(remaining.min(CHUNK_BYTES as u64))
                .map_err(|_| StoreError::InputLength)?;
            input
                .read_exact(&mut scratch[..count])
                .map_err(|_| StoreError::InputIo)?;
            digest.update(&scratch[..count]);
            self.owner.lock()?.append_blob_chunk(
                length,
                hash,
                chunk_count,
                index,
                &scratch[..count],
            )?;
            after_committed_chunk();
            remaining -= count as u64;
        }
        let mut extra = [0_u8; 1];
        if input.read(&mut extra).map_err(|_| StoreError::InputIo)? != 0 {
            return Err(StoreError::InputLength);
        }
        if digest.finalize().as_slice() != hash {
            return Err(StoreError::HashMismatch);
        }
        self.verify_and_complete_blob_phased(length, hash, chunk_count, scratch)
    }

    fn verify_and_complete_blob_phased(
        &self,
        length: u64,
        hash: [u8; 32],
        chunk_count: i64,
        scratch: &mut Vec<u8>,
    ) -> Result<(), StoreError> {
        // The admission permit excludes cooperating writes to incomplete
        // chunks. Copy each committed chunk under the core lock, hash outside
        // it, then publish the complete marker in one final transaction.
        let mut stored = Sha256::new();
        let mut stored_length = 0_u64;
        for index in 0..chunk_count {
            let expected_count = usize::try_from((length - stored_length).min(CHUNK_BYTES as u64))
                .map_err(|_| StoreError::InputLength)?;
            scratch.clear();
            {
                let inner = self.owner.lock()?;
                inner.require_clean()?;
                let mut statement = inner
                    .conn
                    .prepare("SELECT bytes FROM blob_chunks WHERE hash=?1 AND chunk_index=?2")?;
                let mut rows = statement.query(params![hash.as_slice(), index])?;
                let row = rows.next()?.ok_or(StoreError::MissingChunk)?;
                let ValueRef::Blob(bytes) = row.get_ref(0)? else {
                    return Err(StoreError::InvalidChunkLayout);
                };
                if bytes.len() != expected_count {
                    return Err(StoreError::InvalidChunkLayout);
                }
                scratch.extend_from_slice(bytes);
            }
            stored.update(&scratch[..]);
            stored_length += scratch.len() as u64;
        }
        if stored_length != length || stored.finalize().as_slice() != hash {
            return Err(StoreError::CorruptRootBlob);
        }
        let mut inner = self.owner.lock()?;
        inner.require_clean()?;
        inner.ensure_wal_room(MIB)?;
        let tx = inner
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let actual_chunks: i64 = tx.query_row(
            "SELECT count(*) FROM blob_chunks WHERE hash=?1",
            params![hash.as_slice()],
            |row| row.get(0),
        )?;
        if actual_chunks != chunk_count {
            return Err(StoreError::InvalidChunkLayout);
        }
        let updated = tx.execute(
            "UPDATE blob_header SET state=1 WHERE hash=?1 AND length=?2 AND chunk_count=?3 AND state=0",
            params![
                hash.as_slice(),
                i64::try_from(length).map_err(|_| StoreError::InputLength)?,
                chunk_count
            ],
        )?;
        if updated != 1 {
            return Err(StoreError::BlobMetadataMismatch);
        }
        tx.commit()?;
        Ok(())
    }

    /// Pin and authenticate the current manifest for a build or cook read.
    pub fn pin_current(&self, expected_generation: i64) -> Result<PinnedSource, SourceStoreError> {
        let _admission = self.owner.admission()?;
        let pin = {
            let mut inner = self.owner.lock()?;
            inner.require_clean()?;
            inner.retry_pending_pin_releases()?;
            inner.pin_current_root(expected_generation)?
        };
        self.load_pin(pin)
    }

    /// Pin an earlier accepted root for a retained build, match or save.
    /// The supplied generation and revision must have a durable outcome.
    pub fn pin_accepted(&self, root: AcceptedSourceRoot) -> Result<PinnedSource, SourceStoreError> {
        let _admission = self.owner.admission()?;
        let pin = {
            let mut inner = self.owner.lock()?;
            inner.require_clean()?;
            inner.retry_pending_pin_releases()?;
            inner.pin_accepted_root(root)?
        };
        self.load_pin(pin)
    }

    fn load_pin(&self, pin: RootReadPin) -> Result<PinnedSource, SourceStoreError> {
        let loaded = (|| {
            let bytes = {
                let mut inner = self.owner.lock()?;
                inner.require_clean()?;
                let tx = inner
                    .conn
                    .transaction_with_behavior(TransactionBehavior::Deferred)?;
                let bytes = read_complete_blob(&tx, pin.revision, FIRST_PROFILE_MANIFEST_BYTES)?;
                tx.commit()?;
                bytes
            };
            let manifest = decode_manifest_jcs(&bytes, FIRST_PROFILE_MANIFEST_BYTES)
                .map_err(StoreError::Manifest)?;
            if !manifest.dependency_roots().is_empty() {
                return Err(StoreError::UnsupportedDependencyRoots);
            }
            let mut inner = self.owner.lock()?;
            inner.require_clean()?;
            let tx = inner
                .conn
                .transaction_with_behavior(TransactionBehavior::Deferred)?;
            let mut lookup =
                tx.prepare("SELECT length FROM blob_header WHERE hash=?1 AND state=1")?;
            for file in manifest.files() {
                let (length, hash) = file.blob_identity();
                let stored: Option<i64> = lookup
                    .query_row(params![hash.as_slice()], |row| row.get(0))
                    .optional()?;
                if stored != Some(i64::try_from(length).map_err(|_| StoreError::CorruptRootBlob)?) {
                    return Err(StoreError::MissingCompleteFile);
                }
            }
            drop(lookup);
            tx.commit()?;
            Ok(manifest)
        })();
        match loaded {
            Ok(manifest) => Ok(PinnedSource {
                owner: Arc::clone(&self.owner),
                root: AcceptedSourceRoot {
                    generation: pin.generation,
                    revision: SourceRevision::from_bytes(pin.revision),
                },
                manifest,
                pin_id: Some(pin.id),
                scratch: Vec::new(),
            }),
            Err(error) => {
                if let Ok(mut inner) = self.owner.lock() {
                    let _ = inner.release_root_pin(pin.id);
                }
                Err(error.into())
            }
        }
    }
}

/// One authenticated source manifest held by a durable read pin.
pub struct PinnedSource {
    owner: Arc<NativeSourceOwner>,
    root: AcceptedSourceRoot,
    manifest: TreeManifest,
    pin_id: Option<i64>,
    scratch: Vec<u8>,
}

impl PinnedSource {
    /// Pinned root identity; it stays stable when another root is admitted.
    pub fn root(&self) -> AcceptedSourceRoot {
        self.root
    }

    /// Authenticated canonical manifest and its sorted file entries.
    pub fn manifest(&self) -> &TreeManifest {
        &self.manifest
    }

    /// Copy a file through bounded SQLite chunks, verifying length and SHA-256.
    ///
    /// On any error, the caller must discard all bytes already written to the
    /// output. Only a successful return makes that output a verified product.
    pub fn copy_file_to(
        &mut self,
        path: &SourcePath,
        output: &mut impl Write,
    ) -> Result<u64, SourceStoreError> {
        let file = self
            .manifest
            .files()
            .binary_search_by(|entry| entry.path().cmp(path))
            .ok()
            .map(|index| &self.manifest.files()[index])
            .ok_or(StoreError::MissingFile)?;
        let (length, hash) = file.blob_identity();
        let chunks = blob_chunk_count(length)?;
        {
            let (stored_length, stored_chunks, state): (i64, i64, i64) =
                self.owner.with_read_conn(|conn| {
                    let header = conn
                        .query_row(
                            "SELECT length, chunk_count, state FROM blob_header WHERE hash=?1",
                            params![hash.as_slice()],
                            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                        )
                        .optional()?
                        .ok_or(StoreError::MissingCompleteBlob)?;
                    let actual_chunks: i64 = conn.query_row(
                        "SELECT count(*) FROM blob_chunks WHERE hash=?1",
                        params![hash.as_slice()],
                        |row| row.get(0),
                    )?;
                    if actual_chunks != chunks {
                        return Err(StoreError::InvalidChunkLayout);
                    }
                    Ok(header)
                })?;
            if stored_length != length as i64 || stored_chunks != chunks || state != 1 {
                return Err(StoreError::CorruptRootBlob.into());
            }
        }
        let mut digest = Sha256::new();
        let mut copied = 0_u64;
        for index in 0..chunks {
            self.scratch.clear();
            {
                self.owner.with_read_conn(|conn| {
                    let mut statement = conn.prepare(
                        "SELECT bytes FROM blob_chunks WHERE hash=?1 AND chunk_index=?2",
                    )?;
                    let mut rows = statement.query(params![hash.as_slice(), index])?;
                    let row = rows.next()?.ok_or(StoreError::MissingChunk)?;
                    let ValueRef::Blob(bytes) = row.get_ref(0)? else {
                        return Err(StoreError::InvalidChunkLayout);
                    };
                    let expected = (length - copied).min(CHUNK_BYTES as u64) as usize;
                    if bytes.len() != expected {
                        return Err(StoreError::InvalidChunkLayout);
                    }
                    self.scratch.extend_from_slice(bytes);
                    Ok(())
                })?;
            }
            digest.update(&self.scratch);
            output
                .write_all(&self.scratch)
                .map_err(StoreError::ConsumerWrite)?;
            copied += self.scratch.len() as u64;
        }
        if copied != length || digest.finalize().as_slice() != hash {
            return Err(StoreError::CorruptRootBlob.into());
        }
        Ok(copied)
    }

    /// Release the read pin and report a failed SQLite release to the caller.
    pub fn close(mut self) -> Result<(), SourceStoreError> {
        if let Some(id) = self.pin_id.take() {
            let _admission = self.owner.admission()?;
            self.owner.lock()?.release_root_pin(id)?;
        }
        Ok(())
    }
}

impl Drop for PinnedSource {
    fn drop(&mut self) {
        if let Some(id) = self.pin_id.take()
            && let Ok(_admission) = self.owner.admission()
            && let Ok(mut inner) = self.owner.lock()
        {
            let _ = inner.release_root_pin(id);
        }
    }
}

#[cfg(unix)]
#[test]
fn accepted_store_quarantines_after_uncertain_native_final_commit() {
    use std::sync::atomic::Ordering;

    let fixture = Fixture::new();
    let database = fixture.database();
    let observer = checkpoint_vfs::Observer::new("unix");
    let store = NativeSourceStore::open_with_vfs(
        &database,
        SourceSpacePlan::DEFAULT,
        Some(observer.name()),
    )
    .unwrap();
    let a = b"accepted root A";
    let revision_a: [u8; 32] = Sha256::digest(a).into();
    let pin_a = {
        let mut core = store.owner.lock().unwrap();
        core.stage_blob(&mut a.as_slice(), a.len() as u64, revision_a)
            .unwrap();
        assert_eq!(core.commit_probe(0, 1, 1, [1; 32], revision_a).unwrap(), 1);
        core.pin_current_root(1).unwrap()
    };

    let b = b"accepted root B";
    let revision_b: [u8; 32] = Sha256::digest(b).into();
    store
        .owner
        .lock()
        .unwrap()
        .stage_blob(&mut b.as_slice(), b.len() as u64, revision_b)
        .unwrap();
    observer.arm_wal_fault(checkpoint_vfs::WalFault::AfterCommitSync);
    assert!(matches!(
        store
            .owner
            .lock()
            .unwrap()
            .commit_probe(1, 1, 2, [2; 32], revision_b),
        Err(StoreError::UncertainCommit(_))
    ));
    assert_eq!(observer.counts.wal_fault_hits.load(Ordering::Relaxed), 1);
    let wal_writes = observer.counts.wal_writes.load(Ordering::Relaxed);
    let main_writes = observer.counts.main_writes.load(Ordering::Relaxed);
    assert_eq!(
        store.current_root().unwrap_err().kind(),
        SourceStoreErrorKind::RecoveryRequired
    );
    assert_eq!(
        store.outcome(1, 2, [2; 32]).unwrap_err().kind(),
        SourceStoreErrorKind::RecoveryRequired
    );
    assert_eq!(
        store.space_usage().unwrap_err().kind(),
        SourceStoreErrorKind::RecoveryRequired
    );
    assert_eq!(
        store.pin_current(1).err().unwrap().kind(),
        SourceStoreErrorKind::RecoveryRequired
    );
    assert!(matches!(
        store.owner.lock().unwrap().release_root_pin(pin_a.id),
        Err(StoreError::RecoveryRequired)
    ));
    assert_eq!(
        observer.counts.wal_writes.load(Ordering::Relaxed),
        wal_writes
    );
    assert_eq!(
        observer.counts.main_writes.load(Ordering::Relaxed),
        main_writes
    );
    drop(store);
    observer.assert_closed();
    assert_eq!(
        observer.counts.wal_writes.load(Ordering::Relaxed),
        wal_writes
    );
    assert_eq!(
        observer.counts.main_writes.load(Ordering::Relaxed),
        main_writes
    );

    let reopened = NativeSourceStore::open(&database, SourceSpacePlan::DEFAULT).unwrap();
    let selected = reopened.current_root().unwrap().unwrap();
    assert_eq!(
        (selected.generation, selected.revision.as_bytes()),
        (2, &revision_b)
    );
    assert_eq!(reopened.outcome(1, 2, [2; 32]).unwrap(), Some(selected));
    let retained_pin: i64 = reopened
        .owner
        .lock()
        .unwrap()
        .conn
        .query_row(
            "SELECT count(*) FROM read_pin WHERE id=?1",
            params![pin_a.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(retained_pin, 1);
    reopened
        .owner
        .lock()
        .unwrap()
        .release_root_pin(pin_a.id)
        .unwrap();
}

#[test]
fn root_and_outcome_commit_together_and_retry_survives_reopen() {
    let fixture = Fixture::new();
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
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
        Err(StoreError::MissingCompleteBlob)
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

    let mut reopened = SqliteStoreCore::open(&fixture.database()).unwrap();
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
        Err(StoreError::TokenConflict)
    ));
    assert!(matches!(
        reopened.commit_probe(0, 1, 2, request, revision2),
        Err(StoreError::StaleGeneration)
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
        Err(StoreError::Sql(error)) => {
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
    let final_store = SqliteStoreCore::open(&fixture.database()).unwrap();
    assert_eq!(final_store.root().unwrap(), (2, Some(revision2.to_vec())));
}

#[test]
fn native_store_admits_reopens_and_streams_pinned_source() {
    let fixture = Fixture::new();
    let path = fixture.0.join("authoring.db");
    let plan = SourceSpacePlan {
        free_headroom_bytes: 0,
        ..SourceSpacePlan::DEFAULT
    };
    let store = NativeSourceStore::open(&path, plan).unwrap();
    assert_eq!(store.current_root().unwrap(), None);
    let stage = minimal_source_stage(&fixture, "accepted source");
    let candidate = validate_staged_candidate(&stage, &[]).unwrap();
    let accepted = store.admit(0, 3, 7, [7; 32], &candidate).unwrap();
    assert_eq!(accepted.generation, 1);
    assert_eq!(store.current_root().unwrap(), Some(accepted));
    let usage = store.space_usage().unwrap();
    assert!(usage.logical_main_bytes > 0);
    assert!(usage.reusable_freelist_bytes <= usage.logical_main_bytes);
    assert!(usage.volume_available_bytes > 0);
    let source_path = SourcePath::parse("scenes/main.scene.json").unwrap();
    {
        let mut pinned = store.pin_current(1).unwrap();
        assert_eq!(pinned.root(), accepted);
        assert_eq!(pinned.manifest().revision(), accepted.revision);
        let mut bytes = Vec::new();
        let copied = pinned.copy_file_to(&source_path, &mut bytes).unwrap();
        assert_eq!(copied as usize, bytes.len());
        assert_eq!(
            bytes,
            fs::read(stage.test_directory().join(source_path.as_str())).unwrap()
        );
        assert_eq!(
            pinned
                .copy_file_to(&SourcePath::parse("missing.json").unwrap(), &mut Vec::new())
                .unwrap_err()
                .kind(),
            SourceStoreErrorKind::MissingFile
        );
        pinned.close().unwrap();
    }
    drop(store);
    let store = NativeSourceStore::open(&path, plan).unwrap();
    assert_eq!(store.current_root().unwrap(), Some(accepted));
    assert_eq!(store.outcome(3, 7, [7; 32]).unwrap(), Some(accepted));
    assert_eq!(store.outcome(3, 8, [8; 32]).unwrap(), None);
    assert_eq!(
        store.outcome(3, 7, [8; 32]).unwrap_err().kind(),
        SourceStoreErrorKind::TokenConflict
    );
    assert_eq!(store.admit(0, 3, 7, [7; 32], &candidate).unwrap(), accepted);
    assert_eq!(
        store
            .admit(0, 3, 7, [8; 32], &candidate)
            .unwrap_err()
            .kind(),
        SourceStoreErrorKind::TokenConflict
    );
    assert_eq!(
        store
            .admit(0, 3, 8, [8; 32], &candidate)
            .unwrap_err()
            .kind(),
        SourceStoreErrorKind::StaleGeneration
    );
    assert!(wal_path(&path).exists());
}

#[test]
fn native_open_refuses_rolled_back_selection_and_duplicate_latest_outcome() {
    let fixture = Fixture::new();
    let database = fixture.database();
    let plan = SourceSpacePlan {
        free_headroom_bytes: 0,
        ..SourceSpacePlan::DEFAULT
    };
    let store = NativeSourceStore::open(&database, plan).unwrap();
    let stage_a = minimal_source_stage(&fixture, "head A");
    let candidate_a = validate_staged_candidate(&stage_a, &[]).unwrap();
    let a = store.admit(0, 1, 1, [1; 32], &candidate_a).unwrap();
    let stage_b = minimal_source_stage(&fixture, "head B");
    let candidate_b = validate_staged_candidate(&stage_b, &[]).unwrap();
    let b = store
        .admit(a.generation, 1, 2, [2; 32], &candidate_b)
        .unwrap();
    drop(store);

    let conn = Connection::open(&database).unwrap();
    conn.execute(
        "UPDATE root SET generation=?1, revision=?2 WHERE id=1",
        params![a.generation, a.revision.as_bytes().as_slice()],
    )
    .unwrap();
    drop(conn);
    assert_eq!(
        NativeSourceStore::open(&database, plan)
            .err()
            .unwrap()
            .kind(),
        SourceStoreErrorKind::CorruptStore
    );

    let conn = Connection::open(&database).unwrap();
    conn.execute(
        "UPDATE root SET generation=?1, revision=?2 WHERE id=1",
        params![b.generation, b.revision.as_bytes().as_slice()],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO outcome (epoch,key,request_hash,generation,revision) VALUES (1,3,?1,?2,?3)",
        params![
            [3_u8; 32].as_slice(),
            b.generation,
            b.revision.as_bytes().as_slice()
        ],
    )
    .unwrap();
    drop(conn);
    assert_eq!(
        NativeSourceStore::open(&database, plan)
            .err()
            .unwrap()
            .kind(),
        SourceStoreErrorKind::CorruptStore
    );
}

#[test]
fn native_store_failed_final_outcome_preserves_old_root_and_retry() {
    let fixture = Fixture::new();
    let plan = SourceSpacePlan {
        free_headroom_bytes: 0,
        ..SourceSpacePlan::DEFAULT
    };
    let store = NativeSourceStore::open(&fixture.database(), plan).unwrap();
    let first_stage = minimal_source_stage(&fixture, "first accepted scene");
    let first = validate_staged_candidate(&first_stage, &[]).unwrap();
    let old = store.admit(0, 1, 1, [1; 32], &first).unwrap();
    let second_stage = minimal_source_stage(&fixture, "next accepted scene");
    let second = validate_staged_candidate(&second_stage, &[]).unwrap();
    store
        .owner
        .lock()
        .unwrap()
        .conn
        .execute_batch(
            "CREATE TRIGGER abort_native_outcome BEFORE INSERT ON outcome
             BEGIN SELECT RAISE(ABORT, 'injected native outcome failure'); END;",
        )
        .unwrap();
    assert_eq!(
        store.admit(1, 1, 2, [2; 32], &second).unwrap_err().kind(),
        SourceStoreErrorKind::Storage
    );
    assert_eq!(store.current_root().unwrap(), Some(old));
    drop(store);
    let store = NativeSourceStore::open(&fixture.database(), plan).unwrap();
    assert_eq!(store.current_root().unwrap(), Some(old));
    assert!(
        stored_outcome(&store.owner.lock().unwrap().conn, 1, 2)
            .unwrap()
            .is_none()
    );
    store
        .owner
        .lock()
        .unwrap()
        .conn
        .execute_batch("DROP TRIGGER abort_native_outcome")
        .unwrap();
    let next = store.admit(1, 1, 2, [2; 32], &second).unwrap();
    assert_eq!(next.generation, 2);
    drop(store);
    assert_eq!(
        NativeSourceStore::open(&fixture.database(), plan)
            .unwrap()
            .current_root()
            .unwrap(),
        Some(next)
    );
}

#[test]
fn native_store_rejects_incompatible_database_before_schema_writes() {
    let fixture = Fixture::new();
    let path = fixture.database();
    let incompatible = Connection::open(&path).unwrap();
    incompatible
        .execute_batch("CREATE TABLE unrelated (id INTEGER PRIMARY KEY)")
        .unwrap();
    drop(incompatible);
    let refused = NativeSourceStore::open(&path, SourceSpacePlan::DEFAULT);
    assert_eq!(
        refused.err().unwrap().kind(),
        SourceStoreErrorKind::CorruptStore
    );
    let check = Connection::open(&path).unwrap();
    let tables: i64 = check
        .query_row(
            "SELECT count(*) FROM sqlite_schema WHERE type='table' AND name='root'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(tables, 0);

    let empty_fixture = Fixture::new();
    let empty_path = empty_fixture.database();
    let initialized = Connection::open(&empty_path).unwrap();
    initialized
        .execute_batch("CREATE TABLE removed (id INTEGER); DROP TABLE removed;")
        .unwrap();
    drop(initialized);
    let refused = NativeSourceStore::open(&empty_path, SourceSpacePlan::DEFAULT);
    assert_eq!(
        refused.err().unwrap().kind(),
        SourceStoreErrorKind::CorruptStore
    );
    let check = Connection::open(&empty_path).unwrap();
    let format: i64 = check
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    let auto_vacuum: i64 = check
        .query_row("PRAGMA auto_vacuum", [], |row| row.get(0))
        .unwrap();
    assert_eq!((format, auto_vacuum), (0, 0));
}

#[test]
fn native_store_pinned_copy_refuses_corrupt_chunk() {
    let fixture = Fixture::new();
    let plan = SourceSpacePlan {
        free_headroom_bytes: 0,
        ..SourceSpacePlan::DEFAULT
    };
    let store = NativeSourceStore::open(&fixture.database(), plan).unwrap();
    let stage = minimal_source_stage(&fixture, "verified read");
    let candidate = validate_staged_candidate(&stage, &[]).unwrap();
    store.admit(0, 1, 1, [1; 32], &candidate).unwrap();
    let path = SourcePath::parse("scenes/main.scene.json").unwrap();
    let hash = candidate
        .provisional_manifest()
        .unwrap()
        .files()
        .iter()
        .find(|file| file.path() == &path)
        .unwrap()
        .blob_identity()
        .1;
    store
        .owner
        .lock()
        .unwrap()
        .conn
        .execute(
            "UPDATE blob_chunks SET bytes=zeroblob(length(bytes)) WHERE hash=?1 AND chunk_index=0",
            params![hash.as_slice()],
        )
        .unwrap();
    let mut pinned = store.pin_current(1).unwrap();
    let mut output = Vec::new();
    assert_eq!(
        pinned.copy_file_to(&path, &mut output).unwrap_err().kind(),
        SourceStoreErrorKind::CorruptStore
    );
    pinned.close().unwrap();
}

#[test]
fn native_store_pinned_copy_reports_consumer_output_failure() {
    struct RefusingOutput;

    impl Write for RefusingOutput {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("consumer output refused"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    let fixture = Fixture::new();
    let plan = SourceSpacePlan {
        free_headroom_bytes: 0,
        ..SourceSpacePlan::DEFAULT
    };
    let store = NativeSourceStore::open(&fixture.database(), plan).unwrap();
    let stage = minimal_source_stage(&fixture, "output refusal");
    let candidate = validate_staged_candidate(&stage, &[]).unwrap();
    let root = store.admit(0, 1, 1, [1; 32], &candidate).unwrap();
    let mut pinned = store.pin_current(root.generation).unwrap();
    let path = SourcePath::parse("scenes/main.scene.json").unwrap();
    let error = pinned.copy_file_to(&path, &mut RefusingOutput).unwrap_err();
    assert_eq!(error.kind(), SourceStoreErrorKind::Output);
    assert!(std::error::Error::source(&error).is_some());
    pinned.close().unwrap();
    assert_eq!(store.current_root().unwrap(), Some(root));
}

#[test]
fn native_store_drains_wal_overrun_before_candidate_preflight() {
    let fixture = Fixture::new();
    let plan = SourceSpacePlan {
        wal_stop_bytes: 12 * MIB,
        free_headroom_bytes: 0,
        ..SourceSpacePlan::DEFAULT
    };
    let invalid = SourceSpacePlan {
        wal_stop_bytes: 2 * CHUNK_BYTES as u64 + MIB,
        ..plan
    };
    assert_eq!(
        NativeSourceStore::open(&fixture.database(), invalid)
            .err()
            .unwrap()
            .kind(),
        SourceStoreErrorKind::InvalidPlan
    );
    let store = NativeSourceStore::open(&fixture.database(), plan).unwrap();
    store
        .owner
        .lock()
        .unwrap()
        .conn
        .execute_batch(
            "PRAGMA wal_autocheckpoint=0;
             CREATE TABLE injected_wal_load (bytes BLOB);
             INSERT INTO injected_wal_load VALUES (zeroblob(16000000));",
        )
        .unwrap();
    assert!(fs::metadata(wal_path(&fixture.database())).unwrap().len() >= plan.wal_stop_bytes);
    let stage = minimal_source_stage(&fixture, "after wal drain");
    let candidate = validate_staged_candidate(&stage, &[]).unwrap();
    let accepted = store.admit(0, 1, 1, [1; 32], &candidate).unwrap();
    assert_eq!(accepted.generation, 1);
    assert!(fs::metadata(wal_path(&fixture.database())).unwrap().len() < plan.wal_stop_bytes);
}

#[test]
fn native_store_multichunk_pin_survives_same_owner_root_advance_during_copy() {
    struct UpdateOnFirstWrite<F> {
        bytes: Vec<u8>,
        update: Option<F>,
    }

    impl<F: FnMut()> Write for UpdateOnFirstWrite<F> {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if let Some(mut update) = self.update.take() {
                update();
            }
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    let fixture = Fixture::new();
    let plan = SourceSpacePlan {
        free_headroom_bytes: 0,
        ..SourceSpacePlan::DEFAULT
    };
    let initial_stage = minimal_source_stage(&fixture, "before asset removal");
    let root = fixture.source_root();
    let asset = vec![0x5a_u8; CHUNK_BYTES + 17];
    fs::write(root.join("master.bin"), &asset).unwrap();
    let mut declared: Vec<_> = initial_stage
        .files()
        .iter()
        .map(|file| file.path().clone())
        .collect();
    let asset_path = SourcePath::parse("master.bin").unwrap();
    declared.push(asset_path.clone());
    declared.sort_unstable();
    let stage = stage_declared_tree_with(
        &root,
        &declared,
        Limits {
            temporary: 64 * MIB,
            free_floor: 0,
            ..plan.into()
        },
        |_| {},
    )
    .unwrap();
    let candidate = validate_staged_candidate(&stage, &[]).unwrap();
    let store = NativeSourceStore::open(&fixture.database(), plan).unwrap();
    let first = store.admit(0, 1, 1, [1; 32], &candidate).unwrap();
    let mut pinned = store.pin_current(first.generation).unwrap();

    fs::remove_file(root.join("master.bin")).unwrap();
    let next_stage = minimal_source_stage(&fixture, "after asset removal");
    let next_candidate = validate_staged_candidate(&next_stage, &[]).unwrap();
    let mut second = None;
    let mut output = UpdateOnFirstWrite {
        bytes: Vec::new(),
        update: Some(|| {
            second = Some(
                store
                    .admit(first.generation, 1, 2, [2; 32], &next_candidate)
                    .unwrap(),
            );
            let (busy, _, _): (i64, i64, i64) = store
                .owner
                .lock()
                .unwrap()
                .conn
                .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                })
                .unwrap();
            assert_eq!(busy, 0, "pinned output held a WAL read transaction");
        }),
    };
    assert_eq!(
        pinned.copy_file_to(&asset_path, &mut output).unwrap(),
        asset.len() as u64
    );
    assert_eq!(output.bytes, asset);
    drop(output);
    let second = second.unwrap();
    assert_eq!(second.generation, 2);
    pinned.close().unwrap();
    assert_eq!(store.current_root().unwrap(), Some(second));
    let mut historical = store.pin_accepted(first).unwrap();
    let mut output = Vec::new();
    assert_eq!(
        historical.copy_file_to(&asset_path, &mut output).unwrap(),
        asset.len() as u64
    );
    assert_eq!(output, asset);
    historical.close().unwrap();
    assert_eq!(
        store
            .pin_accepted(AcceptedSourceRoot {
                generation: first.generation,
                revision: second.revision,
            })
            .err()
            .unwrap()
            .kind(),
        SourceStoreErrorKind::MissingAcceptedRoot
    );
}

#[test]
fn native_source_pin_keeps_owner_after_store_handle_drops() {
    let fixture = Fixture::new();
    let plan = SourceSpacePlan {
        free_headroom_bytes: 0,
        ..SourceSpacePlan::DEFAULT
    };
    let stage = minimal_source_stage(&fixture, "held source");
    let candidate = validate_staged_candidate(&stage, &[]).unwrap();
    let store = NativeSourceStore::open(&fixture.database(), plan).unwrap();
    let accepted = store.admit(0, 1, 1, [1; 32], &candidate).unwrap();
    let mut pinned = store.pin_current(accepted.generation).unwrap();
    let path = SourcePath::parse("project.json").unwrap();
    let expected = fs::read(fixture.source_root().join(path.as_str())).unwrap();
    drop(store);
    assert_eq!(
        NativeSourceStore::open(&fixture.database(), plan)
            .err()
            .unwrap()
            .kind(),
        SourceStoreErrorKind::Busy
    );
    fs::remove_file(fixture.source_root().join(path.as_str())).unwrap();

    let mut copied = Vec::new();
    assert_eq!(
        pinned.copy_file_to(&path, &mut copied).unwrap(),
        expected.len() as u64
    );
    assert_eq!(copied, expected);
    pinned.close().unwrap();
    let reopened = NativeSourceStore::open(&fixture.database(), plan).unwrap();
    assert_eq!(reopened.current_root().unwrap(), Some(accepted));
    let pins: i64 = reopened
        .owner
        .lock()
        .unwrap()
        .conn
        .query_row("SELECT count(*) FROM read_pin", [], |row| row.get(0))
        .unwrap();
    assert_eq!(pins, 0);
}

#[test]
fn native_owner_lock_canonicalizes_parent_and_refuses_symlink_sidecar() {
    let fixture = Fixture::new();
    let database = fixture.database();
    let plan = SourceSpacePlan::DEFAULT;
    let owner = NativeSourceStore::open(&database, plan).unwrap();
    let alias = fixture.0.join("source").join("..").join("source.sqlite");
    fs::create_dir(fixture.0.join("source")).unwrap();
    assert_eq!(
        NativeSourceStore::open(&alias, plan).err().unwrap().kind(),
        SourceStoreErrorKind::Busy
    );
    drop(owner);
    #[cfg(unix)]
    {
        let lock = fixture.0.join("source.sqlite.owner.lock");
        fs::remove_file(&lock).unwrap();
        std::os::unix::fs::symlink(&database, &lock).unwrap();
        assert!(NativeSourceStore::open(&database, plan).is_err());
    }
}

#[cfg(unix)]
#[test]
fn native_owner_lock_refuses_hard_link_database_alias() {
    let fixture = Fixture::new();
    let database = fixture.database();
    let alias = fixture.0.join("second-name.sqlite");
    let owner = NativeSourceStore::open(&database, SourceSpacePlan::DEFAULT).unwrap();
    fs::hard_link(&database, &alias).unwrap();
    assert!(NativeSourceStore::open(&alias, SourceSpacePlan::DEFAULT).is_err());
    drop(owner);
    assert!(NativeSourceStore::open(&database, SourceSpacePlan::DEFAULT).is_err());
    fs::remove_file(alias).unwrap();
    assert!(NativeSourceStore::open(&database, SourceSpacePlan::DEFAULT).is_ok());
}

#[test]
fn native_owner_lock_child_holds_until_killed() {
    let Some(database) = std::env::var_os("THRUM_TEST_NATIVE_OWNER_DB") else {
        return;
    };
    let marker = PathBuf::from(std::env::var_os("THRUM_TEST_NATIVE_OWNER_MARKER").unwrap());
    let _owner =
        NativeSourceStore::open(&PathBuf::from(database), SourceSpacePlan::DEFAULT).unwrap();
    signal_and_wait_for_kill(&marker);
}

#[test]
fn native_owner_lock_refuses_other_process_and_reopens_after_kill() {
    let fixture = Fixture::new();
    let database = fixture.database();
    let marker = fixture.0.join("owner.ready");
    let child = Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("sqlite_store::native_owner_lock_child_holds_until_killed")
        .arg("--nocapture")
        .env("THRUM_TEST_NATIVE_OWNER_DB", &database)
        .env("THRUM_TEST_NATIVE_OWNER_MARKER", &marker)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    kill_after_marker(child, &marker, || {
        assert_eq!(
            NativeSourceStore::open(&database, SourceSpacePlan::DEFAULT)
                .err()
                .unwrap()
                .kind(),
            SourceStoreErrorKind::Busy
        );
    });
    let reopened = NativeSourceStore::open(&database, SourceSpacePlan::DEFAULT).unwrap();
    assert_eq!(reopened.current_root().unwrap(), None);
}

#[test]
fn accepted_kill_after_verified_stage_child() {
    let Some(directory) = std::env::var_os("THRUM_TEST_ACCEPTED_STAGE_KILL_DIR") else {
        return;
    };
    let marker = PathBuf::from(std::env::var_os("THRUM_TEST_ACCEPTED_STAGE_KILL_MARKER").unwrap());
    let fixture = Fixture(PathBuf::from(directory));
    let database = fixture.database();
    let stage = minimal_source_stage(&fixture, "after verified stage");
    // The parent owns this shared fixture directory and inspects it after the kill.
    std::mem::forget(fixture);
    let candidate = validate_staged_candidate(&stage, &[]).unwrap();
    let plan = SourceSpacePlan {
        free_headroom_bytes: 0,
        ..SourceSpacePlan::DEFAULT
    };
    let store = NativeSourceStore::open(&database, plan).unwrap();
    store
        .admit_with_probes(
            1,
            1,
            2,
            [2; 32],
            &candidate,
            || {},
            || signal_and_wait_for_kill(&marker),
        )
        .unwrap();
}

#[test]
fn accepted_kill_after_verified_stage_retains_a_and_retries_b() {
    let fixture = Fixture::new();
    let database = fixture.database();
    let plan = SourceSpacePlan {
        free_headroom_bytes: 0,
        ..SourceSpacePlan::DEFAULT
    };
    let store = NativeSourceStore::open(&database, plan).unwrap();
    let stage_a = minimal_source_stage(&fixture, "before verified stage kill");
    let candidate_a = validate_staged_candidate(&stage_a, &[]).unwrap();
    let a = store.admit(0, 1, 1, [1; 32], &candidate_a).unwrap();
    let scene = SourcePath::parse("scenes/main.scene.json").unwrap();
    let old_scene = fs::read(fixture.source_root().join(scene.as_str())).unwrap();
    drop(store);

    let marker = fixture.0.join("accepted-stage.ready");
    let child = Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("sqlite_store::accepted_kill_after_verified_stage_child")
        .arg("--nocapture")
        .env("THRUM_TEST_ACCEPTED_STAGE_KILL_DIR", &fixture.0)
        .env("THRUM_TEST_ACCEPTED_STAGE_KILL_MARKER", &marker)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    kill_after_marker(child, &marker, || {
        assert_eq!(
            NativeSourceStore::open(&database, plan)
                .err()
                .unwrap()
                .kind(),
            SourceStoreErrorKind::Busy
        );
    });

    let store = NativeSourceStore::open(&database, plan).unwrap();
    assert_eq!(store.current_root().unwrap(), Some(a));
    assert_eq!(store.outcome(1, 2, [2; 32]).unwrap(), None);
    let mut old_pin = store.pin_current(a.generation).unwrap();
    let stage_b = minimal_source_stage(&fixture, "after verified stage");
    let candidate_b = validate_staged_candidate(&stage_b, &[]).unwrap();
    let b_revision = candidate_b.provisional_manifest().unwrap().revision();
    let retained_complete: i64 = store
        .owner
        .lock()
        .unwrap()
        .conn
        .query_row(
            "SELECT state FROM blob_header WHERE hash=?1",
            params![b_revision.as_bytes().as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(retained_complete, 1);
    let b = store
        .admit(a.generation, 1, 2, [2; 32], &candidate_b)
        .unwrap();
    assert_eq!(b.generation, a.generation + 1);
    let mut copied = Vec::new();
    old_pin.copy_file_to(&scene, &mut copied).unwrap();
    assert_eq!(copied, old_scene);
    old_pin.close().unwrap();
    drop(store);
    let reopened = NativeSourceStore::open(&database, plan).unwrap();
    assert_eq!(reopened.current_root().unwrap(), Some(b));
    assert_eq!(reopened.outcome(1, 2, [2; 32]).unwrap(), Some(b));
}

#[test]
fn accepted_owner_reuses_partial_blob_while_old_pin_reads() {
    let fixture = Fixture::new();
    let plan = SourceSpacePlan {
        free_headroom_bytes: 0,
        ..SourceSpacePlan::DEFAULT
    };
    let store = NativeSourceStore::open(&fixture.database(), plan).unwrap();
    let stage_a = minimal_source_stage(&fixture, "before partial B retry");
    let candidate_a = validate_staged_candidate(&stage_a, &[]).unwrap();
    let a = store.admit(0, 1, 1, [1; 32], &candidate_a).unwrap();
    let scene = SourcePath::parse("scenes/main.scene.json").unwrap();
    let old_scene = fs::read(fixture.source_root().join(scene.as_str())).unwrap();
    let mut a_pin = store.pin_current(a.generation).unwrap();

    let bytes = vec![0x9d_u8; CHUNK_BYTES * 2 + 17];
    let hash: [u8; 32] = Sha256::digest(&bytes).into();
    let mut scratch = Vec::new();
    let _admission = store.owner.admission().unwrap();
    let mut copied = Vec::new();
    let mut visits = 0;
    assert!(matches!(
        store.stage_blob_phased(
            &mut bytes[..CHUNK_BYTES * 2].as_ref(),
            bytes.len() as u64,
            hash,
            &mut scratch,
            &mut || {
                copied.clear();
                a_pin.copy_file_to(&scene, &mut copied).unwrap();
                assert_eq!(copied, old_scene);
                visits += 1;
            },
        ),
        Err(StoreError::InputIo)
    ));
    assert_eq!(visits, 2);
    store
        .owner
        .lock()
        .unwrap()
        .conn
        .execute_batch(
            "CREATE TRIGGER no_partial_delete BEFORE DELETE ON blob_chunks
             BEGIN SELECT RAISE(ABORT, 'matching partial chunks must be reused'); END;",
        )
        .unwrap();
    store
        .stage_blob_phased(
            &mut bytes.as_slice(),
            bytes.len() as u64,
            hash,
            &mut scratch,
            &mut || {
                copied.clear();
                a_pin.copy_file_to(&scene, &mut copied).unwrap();
                assert_eq!(copied, old_scene);
                visits += 1;
            },
        )
        .unwrap();
    assert_eq!(visits, 5);
    assert_eq!(store.current_root().unwrap(), Some(a));
    drop(_admission);
    a_pin.close().unwrap();
}

#[test]
fn accepted_old_pin_reads_between_new_blob_chunks() {
    let fixture = Fixture::new();
    let plan = SourceSpacePlan {
        free_headroom_bytes: 0,
        ..SourceSpacePlan::DEFAULT
    };
    let store = NativeSourceStore::open(&fixture.database(), plan).unwrap();
    let stage_a = minimal_source_stage(&fixture, "old interleaved scene");
    let candidate_a = validate_staged_candidate(&stage_a, &[]).unwrap();
    let a = store.admit(0, 1, 1, [1; 32], &candidate_a).unwrap();
    let old_pin = store.pin_current(a.generation).unwrap();
    let scene = SourcePath::parse("scenes/main.scene.json").unwrap();
    let expected = fs::read(fixture.source_root().join(scene.as_str())).unwrap();
    let stage_b = stage_12_mib_binary(&fixture, &stage_a, plan);
    let candidate_b = validate_staged_candidate(&stage_b, &[]).unwrap();
    let (start_tx, start_rx) = std::sync::mpsc::channel();
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let b = std::thread::scope(|scope| {
        let store_ref = &store;
        let reader = scope.spawn(move || {
            let mut old_pin = old_pin;
            start_rx.recv_timeout(Duration::from_secs(5)).unwrap();
            let mut copied = Vec::new();
            old_pin.copy_file_to(&scene, &mut copied).unwrap();
            assert_eq!(copied, expected);
            assert_eq!(store_ref.current_root().unwrap(), Some(a));
            done_tx.send(()).unwrap();
            old_pin.close().unwrap();
        });
        let mut first_chunk = true;
        let b = store
            .admit_with_probes(
                a.generation,
                1,
                2,
                [2; 32],
                &candidate_b,
                || {
                    if first_chunk {
                        first_chunk = false;
                        start_tx.send(()).unwrap();
                        done_rx.recv_timeout(Duration::from_secs(5)).unwrap();
                    }
                },
                || {},
            )
            .unwrap();
        reader.join().unwrap();
        b
    });
    assert_eq!(b.generation, a.generation + 1);
    assert_eq!(store.current_root().unwrap(), Some(b));
}

#[test]
fn accepted_phased_verification_refuses_corrupt_stored_chunk_and_retries() {
    let fixture = Fixture::new();
    let plan = SourceSpacePlan {
        free_headroom_bytes: 0,
        ..SourceSpacePlan::DEFAULT
    };
    let store = NativeSourceStore::open(&fixture.database(), plan).unwrap();
    let stage_a = minimal_source_stage(&fixture, "before corrupt stored chunk");
    let candidate_a = validate_staged_candidate(&stage_a, &[]).unwrap();
    let a = store.admit(0, 1, 1, [1; 32], &candidate_a).unwrap();
    let stage_b = stage_12_mib_binary(&fixture, &stage_a, plan);
    let hash = *stage_b
        .files()
        .iter()
        .find(|file| file.path().as_str() == "master.bin")
        .unwrap()
        .sha256();
    let candidate_b = validate_staged_candidate(&stage_b, &[]).unwrap();
    let mut committed = 0;
    let failed = store
        .admit_with_probes(
            a.generation,
            1,
            2,
            [2; 32],
            &candidate_b,
            || {
                committed += 1;
                if committed == 3 {
                    store
                        .owner
                        .lock()
                        .unwrap()
                        .conn
                        .execute(
                            "UPDATE blob_chunks SET bytes=?3 WHERE hash=?1 AND chunk_index=?2",
                            params![hash.as_slice(), 0, vec![0_u8; CHUNK_BYTES]],
                        )
                        .unwrap();
                }
            },
            || {},
        )
        .unwrap_err();
    assert_eq!(failed.kind(), SourceStoreErrorKind::CorruptStore);
    assert_eq!(store.current_root().unwrap(), Some(a));
    assert_eq!(store.outcome(1, 2, [2; 32]).unwrap(), None);
    assert_eq!(committed, 3);
    let b = store
        .admit(a.generation, 1, 2, [2; 32], &candidate_b)
        .unwrap();
    assert_eq!(b.generation, a.generation + 1);
    assert_eq!(store.current_root().unwrap(), Some(b));
}

#[test]
fn accepted_kill_after_public_admit_child() {
    let Some(directory) = std::env::var_os("THRUM_TEST_ACCEPTED_COMMITTED_KILL_DIR") else {
        return;
    };
    let marker =
        PathBuf::from(std::env::var_os("THRUM_TEST_ACCEPTED_COMMITTED_KILL_MARKER").unwrap());
    let fixture = Fixture(PathBuf::from(directory));
    let database = fixture.database();
    let stage = minimal_source_stage(&fixture, "committed before response");
    std::mem::forget(fixture);
    let candidate = validate_staged_candidate(&stage, &[]).unwrap();
    let plan = SourceSpacePlan {
        free_headroom_bytes: 0,
        ..SourceSpacePlan::DEFAULT
    };
    let store = NativeSourceStore::open(&database, plan).unwrap();
    let b = store.admit(1, 1, 2, [2; 32], &candidate).unwrap();
    assert_eq!(b.generation, 2);
    signal_and_wait_for_kill(&marker);
}

#[test]
fn accepted_kill_after_public_admit_recovers_exact_outcome() {
    let fixture = Fixture::new();
    let database = fixture.database();
    let plan = SourceSpacePlan {
        free_headroom_bytes: 0,
        ..SourceSpacePlan::DEFAULT
    };
    let store = NativeSourceStore::open(&database, plan).unwrap();
    let stage_a = minimal_source_stage(&fixture, "before lost response");
    let candidate_a = validate_staged_candidate(&stage_a, &[]).unwrap();
    let a = store.admit(0, 1, 1, [1; 32], &candidate_a).unwrap();
    let scene = SourcePath::parse("scenes/main.scene.json").unwrap();
    let old_scene = fs::read(fixture.source_root().join(scene.as_str())).unwrap();
    drop(store);

    let marker = fixture.0.join("accepted-committed.ready");
    let child = Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("sqlite_store::accepted_kill_after_public_admit_child")
        .arg("--nocapture")
        .env("THRUM_TEST_ACCEPTED_COMMITTED_KILL_DIR", &fixture.0)
        .env("THRUM_TEST_ACCEPTED_COMMITTED_KILL_MARKER", &marker)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    kill_after_marker(child, &marker, || {
        assert_eq!(
            NativeSourceStore::open(&database, plan)
                .err()
                .unwrap()
                .kind(),
            SourceStoreErrorKind::Busy
        );
    });

    let store = NativeSourceStore::open(&database, plan).unwrap();
    let b = store.outcome(1, 2, [2; 32]).unwrap().unwrap();
    assert_eq!(b.generation, a.generation + 1);
    assert_eq!(store.current_root().unwrap(), Some(b));
    let stage_b = minimal_source_stage(&fixture, "committed before response");
    let candidate_b = validate_staged_candidate(&stage_b, &[]).unwrap();
    assert_eq!(
        store
            .admit(a.generation, 1, 2, [2; 32], &candidate_b)
            .unwrap(),
        b
    );
    assert_eq!(
        store.outcome(1, 2, [3; 32]).unwrap_err().kind(),
        SourceStoreErrorKind::TokenConflict
    );
    let mut old_pin = store.pin_accepted(a).unwrap();
    let mut copied = Vec::new();
    old_pin.copy_file_to(&scene, &mut copied).unwrap();
    assert_eq!(copied, old_scene);
    old_pin.close().unwrap();
    assert_eq!(store.current_root().unwrap(), Some(b));
}

#[cfg(unix)]
#[test]
fn accepted_live_lease_refuses_sql_after_uncertain_commit() {
    use std::sync::atomic::Ordering;

    let fixture = Fixture::new();
    let database = fixture.database();
    let observer = checkpoint_vfs::Observer::new("unix");
    let plan = SourceSpacePlan {
        free_headroom_bytes: 0,
        ..SourceSpacePlan::DEFAULT
    };
    let store = NativeSourceStore::open_with_vfs(&database, plan, Some(observer.name())).unwrap();
    let stage_a = minimal_source_stage(&fixture, "accepted A");
    let candidate_a = validate_staged_candidate(&stage_a, &[]).unwrap();
    let a = store.admit(0, 1, 1, [1; 32], &candidate_a).unwrap();
    let mut pinned = store.pin_current(a.generation).unwrap();
    let pin_id = pinned.pin_id.unwrap();

    let stage_b = minimal_source_stage(&fixture, "accepted B");
    let candidate_b = validate_staged_candidate(&stage_b, &[]).unwrap();
    let manifest_b = candidate_b.provisional_manifest().unwrap();
    {
        let mut inner = store.owner.lock().unwrap();
        inner.stage_candidate_files(&candidate_b).unwrap();
        inner.stage_manifest_chunked(&manifest_b).unwrap();
    }
    let fault = store
        .admit_with_probes(
            a.generation,
            1,
            2,
            [2; 32],
            &candidate_b,
            || {},
            || observer.arm_wal_fault(checkpoint_vfs::WalFault::AfterCommitSync),
        )
        .unwrap_err();
    assert_eq!(fault.kind(), SourceStoreErrorKind::RecoveryRequired);
    assert_eq!(observer.counts.wal_fault_hits.load(Ordering::Relaxed), 1);
    assert_eq!(
        NativeSourceStore::open(&database, plan)
            .err()
            .unwrap()
            .kind(),
        SourceStoreErrorKind::Busy
    );
    let wal_writes = observer.counts.wal_writes.load(Ordering::Relaxed);
    let main_writes = observer.counts.main_writes.load(Ordering::Relaxed);
    let mut output = Vec::new();
    assert_eq!(
        pinned
            .copy_file_to(&SourcePath::parse("project.json").unwrap(), &mut output)
            .unwrap_err()
            .kind(),
        SourceStoreErrorKind::RecoveryRequired
    );
    assert_eq!(
        pinned.close().unwrap_err().kind(),
        SourceStoreErrorKind::RecoveryRequired
    );
    assert_eq!(
        observer.counts.wal_writes.load(Ordering::Relaxed),
        wal_writes
    );
    assert_eq!(
        observer.counts.main_writes.load(Ordering::Relaxed),
        main_writes
    );
    drop(store);
    observer.assert_closed();
    assert_eq!(
        observer.counts.wal_writes.load(Ordering::Relaxed),
        wal_writes
    );
    assert_eq!(
        observer.counts.main_writes.load(Ordering::Relaxed),
        main_writes
    );

    let reopened = NativeSourceStore::open(&database, plan).unwrap();
    let b = reopened.outcome(1, 2, [2; 32]).unwrap().unwrap();
    assert_eq!(reopened.current_root().unwrap(), Some(b));
    assert_eq!(b.generation, a.generation + 1);
    let retained_pin: i64 = reopened
        .owner
        .lock()
        .unwrap()
        .conn
        .query_row(
            "SELECT count(*) FROM read_pin WHERE id=?1",
            params![pin_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(retained_pin, 1);
    reopened
        .owner
        .lock()
        .unwrap()
        .release_root_pin(pin_id)
        .unwrap();
}

#[test]
fn accepted_old_pin_reads_during_final_closure_transaction() {
    let fixture = Fixture::new();
    let store = NativeSourceStore::open(&fixture.database(), SourceSpacePlan::DEFAULT).unwrap();
    let stage_a = minimal_source_stage(&fixture, "accepted A");
    let candidate_a = validate_staged_candidate(&stage_a, &[]).unwrap();
    let a = store.admit(0, 1, 1, [1; 32], &candidate_a).unwrap();
    let mut old_pin = store.pin_current(a.generation).unwrap();
    let path = SourcePath::parse("project.json").unwrap();
    let expected = fs::read(fixture.source_root().join(path.as_str())).unwrap();

    let stage_b = minimal_source_stage(&fixture, "accepted B");
    let candidate_b = validate_staged_candidate(&stage_b, &[]).unwrap();
    let manifest_b = candidate_b.provisional_manifest().unwrap();
    let revision_b = *manifest_b.revision().as_bytes();
    {
        let mut inner = store.owner.lock().unwrap();
        inner.stage_candidate_files(&candidate_b).unwrap();
        inner.stage_manifest_chunked(&manifest_b).unwrap();
    }
    let refs = prepare_manifest_closure(&manifest_b).unwrap();
    let (go_tx, go_rx) = std::sync::mpsc::channel();
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let (committed, copied) = std::thread::scope(|scope| {
        let old_pin_ref = &mut old_pin;
        let reader = scope.spawn(move || {
            go_rx.recv().unwrap();
            let mut copied = Vec::new();
            let result = old_pin_ref.copy_file_to(&path, &mut copied);
            done_tx.send(()).unwrap();
            (result, copied)
        });
        let mut inner = store.owner.lock().unwrap();
        let committed = inner.commit_with_closure(
            a.generation,
            1,
            2,
            [2; 32],
            revision_b,
            |tx| {
                check_prepared_manifest_closure(tx, &refs)?;
                go_tx.send(()).unwrap();
                done_rx
                    .recv_timeout(Duration::from_secs(2))
                    .map_err(|_| StoreError::OutputIo)?;
                Ok(())
            },
            Some(&store.owner.read_gate),
        );
        drop(inner);
        (committed, reader.join().unwrap())
    });
    assert_eq!(committed.unwrap(), a.generation + 1);
    assert_eq!(copied.0.unwrap(), expected.len() as u64);
    assert_eq!(copied.1, expected);
    assert_eq!(store.current_root().unwrap().unwrap().generation, 2);
    old_pin.close().unwrap();
}

#[cfg(unix)]
#[test]
fn accepted_reader_waiting_at_publication_sees_uncertain_commit_quarantine() {
    let fixture = Fixture::new();
    let observer = checkpoint_vfs::Observer::new("unix");
    let store = NativeSourceStore::open_with_vfs(
        &fixture.database(),
        SourceSpacePlan::DEFAULT,
        Some(observer.name()),
    )
    .unwrap();
    let stage_a = minimal_source_stage(&fixture, "accepted A");
    let candidate_a = validate_staged_candidate(&stage_a, &[]).unwrap();
    let a = store.admit(0, 1, 1, [1; 32], &candidate_a).unwrap();
    let mut old_pin = store.pin_current(a.generation).unwrap();

    let stage_b = minimal_source_stage(&fixture, "accepted B");
    let candidate_b = validate_staged_candidate(&stage_b, &[]).unwrap();
    let manifest_b = candidate_b.provisional_manifest().unwrap();
    let revision_b = *manifest_b.revision().as_bytes();
    {
        let mut inner = store.owner.lock().unwrap();
        inner.stage_candidate_files(&candidate_b).unwrap();
        inner.stage_manifest_chunked(&manifest_b).unwrap();
    }
    let refs = prepare_manifest_closure(&manifest_b).unwrap();
    let (go_tx, go_rx) = std::sync::mpsc::channel();
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let path = SourcePath::parse("project.json").unwrap();
    let (commit, read) = std::thread::scope(|scope| {
        let old_pin_ref = &mut old_pin;
        let reader = scope.spawn(move || {
            go_rx.recv().unwrap();
            started_tx.send(()).unwrap();
            old_pin_ref.copy_file_to(&path, &mut io::sink())
        });
        let mut inner = store.owner.lock().unwrap();
        let commit = inner.commit_with_closure_after_sql(
            a.generation,
            1,
            2,
            [2; 32],
            revision_b,
            |tx| check_prepared_manifest_closure(tx, &refs),
            Some(&store.owner.read_gate),
            || {
                observer.arm_wal_fault(checkpoint_vfs::WalFault::AfterCommitSync);
                go_tx.send(()).unwrap();
                started_rx.recv_timeout(Duration::from_secs(2)).unwrap();
                std::thread::sleep(Duration::from_millis(10));
            },
        );
        drop(inner);
        (commit, reader.join().unwrap())
    });
    assert!(matches!(commit, Err(StoreError::UncertainCommit(_))));
    assert_eq!(
        read.unwrap_err().kind(),
        SourceStoreErrorKind::RecoveryRequired
    );
    assert_eq!(
        old_pin.close().unwrap_err().kind(),
        SourceStoreErrorKind::RecoveryRequired
    );
    let wal_writes = observer.counts.wal_writes.load(Ordering::Relaxed);
    let main_writes = observer.counts.main_writes.load(Ordering::Relaxed);
    drop(store);
    observer.assert_closed();
    assert_eq!(
        observer.counts.wal_writes.load(Ordering::Relaxed),
        wal_writes
    );
    assert_eq!(
        observer.counts.main_writes.load(Ordering::Relaxed),
        main_writes
    );
}

#[test]
fn native_store_sqlite_full_during_blob_preserves_root_and_retries() {
    let fixture = Fixture::new();
    let plan = SourceSpacePlan {
        free_headroom_bytes: 0,
        ..SourceSpacePlan::DEFAULT
    };
    let store = NativeSourceStore::open(&fixture.database(), plan).unwrap();
    let first_stage = minimal_source_stage(&fixture, "before sqlite full");
    let first = validate_staged_candidate(&first_stage, &[]).unwrap();
    let old = store.admit(0, 1, 1, [1; 32], &first).unwrap();
    let root = fixture.source_root();
    fs::write(root.join("master.bin"), vec![7_u8; MIB as usize]).unwrap();
    let mut declared: Vec<_> = first_stage
        .files()
        .iter()
        .map(|file| file.path().clone())
        .collect();
    declared.push(SourcePath::parse("master.bin").unwrap());
    declared.sort_unstable();
    let stage = stage_declared_tree_with(
        &root,
        &declared,
        Limits {
            temporary: 64 * MIB,
            free_floor: 0,
            ..plan.into()
        },
        |_| {},
    )
    .unwrap();
    let candidate = validate_staged_candidate(&stage, &[]).unwrap();
    let current_pages: i64 = store
        .owner
        .lock()
        .unwrap()
        .conn
        .query_row("PRAGMA page_count", [], |row| row.get(0))
        .unwrap();
    store
        .owner
        .lock()
        .unwrap()
        .conn
        .pragma_update(None, "max_page_count", current_pages)
        .unwrap();
    let full = store.admit(1, 1, 2, [2; 32], &candidate).unwrap_err();
    assert_eq!(full.kind(), SourceStoreErrorKind::SpacePressure);
    assert_eq!(full.space_pressure(), None);
    assert_eq!(store.current_root().unwrap(), Some(old));
    assert_eq!(store.outcome(1, 2, [2; 32]).unwrap(), None);
    assert_eq!(Arc::strong_count(&store.owner), 1);
    drop(store);
    let reopened = NativeSourceStore::open(&fixture.database(), plan).unwrap();
    assert_eq!(reopened.current_root().unwrap(), Some(old));
    assert_eq!(reopened.outcome(1, 2, [2; 32]).unwrap(), None);
    let next = reopened.admit(1, 1, 2, [2; 32], &candidate).unwrap();
    assert_eq!(next.generation, 2);
}

#[test]
fn native_store_sqlite_full_in_final_transaction_rolls_back_outcome() {
    let fixture = Fixture::new();
    let plan = SourceSpacePlan {
        free_headroom_bytes: 0,
        ..SourceSpacePlan::DEFAULT
    };
    let store = NativeSourceStore::open(&fixture.database(), plan).unwrap();
    let first_stage = minimal_source_stage(&fixture, "old root");
    let first = validate_staged_candidate(&first_stage, &[]).unwrap();
    let old = store.admit(0, 1, 1, [1; 32], &first).unwrap();
    let second_stage = minimal_source_stage(&fixture, "new root");
    let second = validate_staged_candidate(&second_stage, &[]).unwrap();
    let manifest = second.provisional_manifest().unwrap();
    store
        .owner
        .lock()
        .unwrap()
        .stage_candidate_files(&second)
        .unwrap();
    store
        .owner
        .lock()
        .unwrap()
        .stage_manifest_chunked(&manifest)
        .unwrap();
    store
        .owner
        .lock()
        .unwrap()
        .conn
        .execute_batch(
            "CREATE TABLE injected_outcome_payload (bytes BLOB NOT NULL);
             CREATE TRIGGER fill_on_native_outcome BEFORE INSERT ON outcome
             BEGIN INSERT INTO injected_outcome_payload (bytes) VALUES (zeroblob(1048576)); END;",
        )
        .unwrap();
    let current_pages: i64 = store
        .owner
        .lock()
        .unwrap()
        .conn
        .query_row("PRAGMA page_count", [], |row| row.get(0))
        .unwrap();
    store
        .owner
        .lock()
        .unwrap()
        .conn
        .pragma_update(None, "max_page_count", current_pages)
        .unwrap();
    assert_eq!(
        store.admit(1, 1, 2, [2; 32], &second).unwrap_err().kind(),
        SourceStoreErrorKind::SpacePressure
    );
    assert_eq!(store.current_root().unwrap(), Some(old));
    assert_eq!(store.outcome(1, 2, [2; 32]).unwrap(), None);
    drop(store);
    let reopened = NativeSourceStore::open(&fixture.database(), plan).unwrap();
    assert_eq!(reopened.current_root().unwrap(), Some(old));
    assert_eq!(reopened.outcome(1, 2, [2; 32]).unwrap(), None);
    reopened
        .owner
        .lock()
        .unwrap()
        .conn
        .execute_batch("DROP TRIGGER fill_on_native_outcome")
        .unwrap();
    let next = reopened.admit(1, 1, 2, [2; 32], &second).unwrap();
    assert_eq!(next.generation, 2);
}

#[test]
fn native_store_reader_blocked_wal_defers_and_recovers_admission() {
    let fixture = Fixture::new();
    let plan = SourceSpacePlan {
        wal_stop_bytes: 12 * MIB,
        free_headroom_bytes: 0,
        ..SourceSpacePlan::DEFAULT
    };
    let store = NativeSourceStore::open(&fixture.database(), plan).unwrap();
    let first_stage = minimal_source_stage(&fixture, "before blocked reader");
    let first = validate_staged_candidate(&first_stage, &[]).unwrap();
    let old = store.admit(0, 1, 1, [1; 32], &first).unwrap();
    store
        .owner
        .lock()
        .unwrap()
        .conn
        .execute_batch("PRAGMA wal_autocheckpoint=0")
        .unwrap();
    let checkpoint: (i64, i64, i64) = store
        .owner
        .lock()
        .unwrap()
        .conn
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .unwrap();
    assert_eq!(checkpoint.0, 0);
    let reader = Connection::open(fixture.database()).unwrap();
    reader.execute_batch("BEGIN").unwrap();
    let _: i64 = reader
        .query_row("SELECT generation FROM root WHERE id=1", [], |row| {
            row.get(0)
        })
        .unwrap();

    let root = fixture.source_root();
    fs::write(root.join("master.bin"), vec![0x52_u8; 3 * CHUNK_BYTES]).unwrap();
    let mut declared: Vec<_> = first_stage
        .files()
        .iter()
        .map(|file| file.path().clone())
        .collect();
    declared.push(SourcePath::parse("master.bin").unwrap());
    declared.sort_unstable();
    let stage = stage_declared_tree_with(
        &root,
        &declared,
        Limits {
            temporary: 64 * MIB,
            free_floor: 0,
            ..plan.into()
        },
        |_| {},
    )
    .unwrap();
    let candidate = validate_staged_candidate(&stage, &[]).unwrap();
    let blocked = store.admit(1, 1, 2, [2; 32], &candidate).unwrap_err();
    assert_eq!(blocked.kind(), SourceStoreErrorKind::SpacePressure);
    assert!(blocked.space_pressure().is_some());
    assert_eq!(store.current_root().unwrap(), Some(old));
    assert_eq!(store.outcome(1, 2, [2; 32]).unwrap(), None);
    let blocked_usage = store.space_usage().unwrap();
    assert!(blocked_usage.wal_file_bytes > 0);
    reader.execute_batch("COMMIT").unwrap();
    let next = store.admit(1, 1, 2, [2; 32], &candidate).unwrap();
    assert_eq!(next.generation, 2);
    assert_eq!(store.current_root().unwrap(), Some(next));
    let recovered_usage = store.space_usage().unwrap();
    eprintln!(
        "native_blocked_reader blocked_wal_bytes={} recovered_main_bytes={} recovered_wal_bytes={}",
        blocked_usage.wal_file_bytes,
        recovered_usage.main_file_bytes,
        recovered_usage.wal_file_bytes
    );
}

#[test]
#[ignore = "local sustained public admission under reader-blocked WAL probe"]
fn measure_native_phased_sustained_blocked_wal() {
    #[cfg(unix)]
    fn allocated_bytes(path: &Path) -> u64 {
        use std::os::unix::fs::MetadataExt;
        fs::metadata(path).unwrap().blocks() * 512
    }

    let fixture = Fixture::new();
    let plan = SourceSpacePlan {
        wal_stop_bytes: 40 * MIB,
        free_headroom_bytes: 0,
        ..SourceSpacePlan::DEFAULT
    };
    let store = NativeSourceStore::open(&fixture.database(), plan).unwrap();
    let first_stage = minimal_source_stage(&fixture, "before sustained WAL");
    let first = validate_staged_candidate(&first_stage, &[]).unwrap();
    let a = store.admit(0, 1, 1, [1; 32], &first).unwrap();
    let mut old_pin = store.pin_current(a.generation).unwrap();
    let scene = SourcePath::parse("scenes/main.scene.json").unwrap();
    let checkpoint: (i64, i64, i64) = store
        .owner
        .lock()
        .unwrap()
        .conn
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .unwrap();
    assert_eq!(checkpoint.0, 0);
    let reader = Connection::open(fixture.database()).unwrap();
    reader.execute_batch("BEGIN").unwrap();
    let _: i64 = reader
        .query_row("SELECT generation FROM root WHERE id=1", [], |row| {
            row.get(0)
        })
        .unwrap();
    let started = Instant::now();
    let mut selected = a;
    let mut completed = 0;
    let mut deferred_wal = None;
    #[cfg(unix)]
    let mut max_sampled_allocated = 0;
    #[cfg(unix)]
    let mut blocked_allocated = None;
    #[cfg(unix)]
    let mut recovered_allocated = None;
    #[cfg(unix)]
    let sample_allocated =
        || allocated_bytes(&fixture.database()) + allocated_bytes(&wal_path(&fixture.database()));
    for index in 2_u8..=17 {
        let stage = stage_n_mib_binary(&fixture, &first_stage, plan, 4, index);
        let candidate = validate_staged_candidate(&stage, &[]).unwrap();
        let result = store.admit(
            selected.generation,
            1,
            i64::from(index),
            [index; 32],
            &candidate,
        );
        match result {
            Ok(next) => {
                selected = next;
                completed += 1;
                #[cfg(unix)]
                {
                    max_sampled_allocated = max_sampled_allocated.max(sample_allocated());
                }
            }
            Err(error) if error.kind() == SourceStoreErrorKind::SpacePressure => {
                assert_eq!(store.current_root().unwrap(), Some(selected));
                assert_eq!(
                    store.outcome(1, i64::from(index), [index; 32]).unwrap(),
                    None
                );
                deferred_wal = Some(store.space_usage().unwrap().wal_file_bytes);
                #[cfg(unix)]
                {
                    blocked_allocated = Some(sample_allocated());
                    max_sampled_allocated = max_sampled_allocated.max(blocked_allocated.unwrap());
                }
                reader.execute_batch("COMMIT").unwrap();
                selected = store
                    .admit(
                        selected.generation,
                        1,
                        i64::from(index),
                        [index; 32],
                        &candidate,
                    )
                    .unwrap();
                #[cfg(unix)]
                {
                    recovered_allocated = Some(sample_allocated());
                    max_sampled_allocated = max_sampled_allocated.max(recovered_allocated.unwrap());
                }
                break;
            }
            Err(error) => panic!("unexpected sustained WAL admission error: {error}"),
        }
        old_pin.copy_file_to(&scene, &mut io::sink()).unwrap();
    }
    let blocked_wal = deferred_wal.expect("reader-blocked WAL must defer admission");
    old_pin.copy_file_to(&scene, &mut io::sink()).unwrap();
    old_pin.close().unwrap();
    assert_eq!(store.current_root().unwrap(), Some(selected));
    eprintln!(
        "native_phased_sustained_wal completed={completed} blocked_wal_bytes={blocked_wal} recovered_wal_bytes={} elapsed_ms={:.3}",
        store.space_usage().unwrap().wal_file_bytes,
        started.elapsed().as_secs_f64() * 1000.0
    );
    #[cfg(unix)]
    eprintln!(
        "native_phased_sustained_wal_allocated max_sampled_main_plus_wal_bytes={max_sampled_allocated} blocked_bytes={} after_retry_bytes={}",
        blocked_allocated.unwrap(),
        recovered_allocated.unwrap()
    );
}

#[test]
#[ignore = "local release native admission and pinned-read cost probe"]
fn measure_native_store_small_admission_and_read() {
    let fixture = Fixture::new();
    let plan = SourceSpacePlan {
        free_headroom_bytes: 0,
        ..SourceSpacePlan::DEFAULT
    };
    let store = NativeSourceStore::open(&fixture.database(), plan).unwrap();
    let scene_path = SourcePath::parse("scenes/main.scene.json").unwrap();
    let mut admission_ms = Vec::new();
    let mut pinned_read_ms = Vec::new();
    let mut max_main = 0;
    let mut max_wal = 0;
    let mut max_logical = 0;
    let mut min_available = u64::MAX;
    for index in 0..10 {
        let stage = minimal_source_stage(&fixture, &format!("edit {index}"));
        let candidate = validate_staged_candidate(&stage, &[]).unwrap();
        let start = Instant::now();
        let accepted = store
            .admit(index, 1, index + 1, [index as u8; 32], &candidate)
            .unwrap();
        admission_ms.push(start.elapsed().as_secs_f64() * 1000.0);
        let usage = store.space_usage().unwrap();
        max_main = max_main.max(usage.main_file_bytes);
        max_wal = max_wal.max(usage.wal_file_bytes);
        max_logical = max_logical.max(usage.logical_main_bytes);
        min_available = min_available.min(usage.volume_available_bytes);
        let start = Instant::now();
        let mut pinned = store.pin_current(accepted.generation).unwrap();
        pinned.copy_file_to(&scene_path, &mut io::sink()).unwrap();
        pinned.close().unwrap();
        pinned_read_ms.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    admission_ms.sort_by(f64::total_cmp);
    pinned_read_ms.sort_by(f64::total_cmp);
    let selected = store.current_root().unwrap();
    drop(store);
    let mut reopen_ms = Vec::new();
    for _ in 0..10 {
        let start = Instant::now();
        let reopened = NativeSourceStore::open(&fixture.database(), plan).unwrap();
        reopen_ms.push(start.elapsed().as_secs_f64() * 1000.0);
        assert_eq!(reopened.current_root().unwrap(), selected);
    }
    reopen_ms.sort_by(f64::total_cmp);
    eprintln!(
        "native_small_admission n=10 min_ms={:.3} median_ms={:.3} max_ms={:.3}",
        admission_ms[0], admission_ms[5], admission_ms[9]
    );
    eprintln!(
        "native_small_pin_copy n=10 min_ms={:.3} median_ms={:.3} max_ms={:.3}",
        pinned_read_ms[0], pinned_read_ms[5], pinned_read_ms[9]
    );
    eprintln!(
        "native_small_reopen n=10 min_ms={:.3} median_ms={:.3} max_ms={:.3}",
        reopen_ms[0], reopen_ms[5], reopen_ms[9]
    );
    eprintln!(
        "native_small_space sampled_after_admit=10 max_main_bytes={max_main} max_wal_bytes={max_wal} max_logical_bytes={max_logical} min_available_bytes={min_available}"
    );
}

#[test]
#[ignore = "local release-only 12 MiB accepted admission and pinned-read cost probe"]
fn measure_native_store_large_admission_and_read() {
    let fixture = Fixture::new();
    let plan = SourceSpacePlan {
        free_headroom_bytes: 0,
        ..SourceSpacePlan::DEFAULT
    };
    let store = NativeSourceStore::open(&fixture.database(), plan).unwrap();
    let first_stage = minimal_source_stage(&fixture, "before 12 MiB edit");
    let first = validate_staged_candidate(&first_stage, &[]).unwrap();
    let a = store.admit(0, 1, 1, [1; 32], &first).unwrap();
    let mut old_pin = store.pin_current(a.generation).unwrap();

    let stage = stage_12_mib_binary(&fixture, &first_stage, plan);
    let master_path = SourcePath::parse("master.bin").unwrap();
    let candidate = validate_staged_candidate(&stage, &[]).unwrap();
    let start = Instant::now();
    let b = store
        .admit(a.generation, 1, 2, [2; 32], &candidate)
        .unwrap();
    let admission_ms = start.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(b.generation, a.generation + 1);
    let mut old_scene = Vec::new();
    old_pin
        .copy_file_to(
            &SourcePath::parse("scenes/main.scene.json").unwrap(),
            &mut old_scene,
        )
        .unwrap();
    old_pin.close().unwrap();
    let mut pin_b = store.pin_current(b.generation).unwrap();
    let start = Instant::now();
    assert_eq!(
        pin_b.copy_file_to(&master_path, &mut io::sink()).unwrap(),
        12 * MIB
    );
    let copy_ms = start.elapsed().as_secs_f64() * 1000.0;
    pin_b.close().unwrap();
    let usage = store.space_usage().unwrap();
    eprintln!(
        "native_large_admission bytes={} admission_ms={admission_ms:.3} pinned_copy_ms={copy_ms:.3} main_bytes={} wal_bytes={} logical_bytes={}",
        12 * MIB,
        usage.main_file_bytes,
        usage.wal_file_bytes,
        usage.logical_main_bytes
    );
}

#[cfg(test)]
fn stage_12_mib_binary(
    fixture: &Fixture,
    first_stage: &crate::StagedTree,
    plan: SourceSpacePlan,
) -> crate::StagedTree {
    stage_n_mib_binary(fixture, first_stage, plan, 12, 7)
}

#[cfg(test)]
fn stage_n_mib_binary(
    fixture: &Fixture,
    first_stage: &crate::StagedTree,
    plan: SourceSpacePlan,
    size_mib: u64,
    fill: u8,
) -> crate::StagedTree {
    let root = fixture.source_root();
    let mut master = File::create(root.join("master.bin")).unwrap();
    let block = [fill; 4096];
    for _ in 0..(size_mib * MIB / block.len() as u64) {
        master.write_all(&block).unwrap();
    }
    drop(master);
    let mut declared: Vec<_> = first_stage
        .files()
        .iter()
        .map(|file| file.path().clone())
        .collect();
    declared.push(SourcePath::parse("master.bin").unwrap());
    declared.sort_unstable();
    stage_declared_tree_with(
        &root,
        &declared,
        Limits {
            temporary: (size_mib * 2).max(64) * MIB,
            free_floor: 0,
            ..plan.into()
        },
        |_| {},
    )
    .unwrap()
}

#[test]
#[ignore = "local release-only reader wait behind a 12 MiB accepted admission"]
fn measure_native_reader_wait_behind_large_admission() {
    let fixture = Fixture::new();
    let plan = SourceSpacePlan {
        free_headroom_bytes: 0,
        ..SourceSpacePlan::DEFAULT
    };
    let store = NativeSourceStore::open(&fixture.database(), plan).unwrap();
    let first_stage = minimal_source_stage(&fixture, "before reader wait");
    let first = validate_staged_candidate(&first_stage, &[]).unwrap();
    let a = store.admit(0, 1, 1, [1; 32], &first).unwrap();
    let mut old_pin = store.pin_current(a.generation).unwrap();
    let stage = stage_12_mib_binary(&fixture, &first_stage, plan);
    let candidate = validate_staged_candidate(&stage, &[]).unwrap();
    let scene = SourcePath::parse("scenes/main.scene.json").unwrap();
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let (go_tx, go_rx) = std::sync::mpsc::channel();
    let (reader_ms, writer_remaining_ms) = std::thread::scope(|scope| {
        let reader = scope.spawn(move || {
            ready_rx.recv().unwrap();
            let start = Instant::now();
            go_tx.send(()).unwrap();
            let mut bytes = Vec::new();
            old_pin.copy_file_to(&scene, &mut bytes).unwrap();
            assert!(!bytes.is_empty());
            old_pin.close().unwrap();
            start.elapsed().as_secs_f64() * 1000.0
        });
        let mut writer_start = None;
        let (generation, _) = store
            .owner
            .lock()
            .unwrap()
            .admit_staged_candidate_probe_with_limits(
                a.generation,
                1,
                2,
                [2; 32],
                &candidate,
                plan.into(),
                || {},
                |_| {
                    ready_tx.send(()).unwrap();
                    go_rx.recv().unwrap();
                    writer_start = Some(Instant::now());
                },
                |_| {},
            )
            .unwrap();
        assert_eq!(generation, a.generation + 1);
        let writer_remaining_ms = writer_start.unwrap().elapsed().as_secs_f64() * 1000.0;
        (reader.join().unwrap(), writer_remaining_ms)
    });
    eprintln!(
        "native_reader_behind_large_admission reader_wait_copy_ms={reader_ms:.3} writer_after_preflight_ms={writer_remaining_ms:.3}"
    );
}

#[test]
#[ignore = "local release-only phased writer and pinned-reader contention probe"]
fn measure_native_phased_reader_progress() {
    let fixture = Fixture::new();
    let plan = SourceSpacePlan {
        free_headroom_bytes: 0,
        ..SourceSpacePlan::DEFAULT
    };
    let store = NativeSourceStore::open(&fixture.database(), plan).unwrap();
    let first_stage = minimal_source_stage(&fixture, "before phased wait");
    let first = validate_staged_candidate(&first_stage, &[]).unwrap();
    let a = store.admit(0, 1, 1, [1; 32], &first).unwrap();
    let mut old_pin = store.pin_current(a.generation).unwrap();
    let size_mib = std::env::var("THRUM_TEST_SOURCE_PROBE_MIB")
        .ok()
        .map(|value| value.parse::<u64>().unwrap())
        .unwrap_or(12);
    assert!(matches!(size_mib, 12 | 64));
    let stage = stage_n_mib_binary(&fixture, &first_stage, plan, size_mib, 7);
    let candidate = validate_staged_candidate(&stage, &[]).unwrap();
    let scene = SourcePath::parse("scenes/main.scene.json").unwrap();
    let done = std::sync::atomic::AtomicBool::new(false);
    let (mut samples, writer_ms) = std::thread::scope(|scope| {
        let writer = scope.spawn(|| {
            let start = Instant::now();
            let b = store
                .admit(a.generation, 1, 2, [2; 32], &candidate)
                .unwrap();
            done.store(true, Ordering::Release);
            (b, start.elapsed().as_secs_f64() * 1000.0)
        });
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match store.owner.admission_scratch.try_lock() {
                Err(std::sync::TryLockError::WouldBlock) => break,
                Ok(guard) => drop(guard),
                Err(std::sync::TryLockError::Poisoned(_)) => panic!("admission owner poisoned"),
            }
            assert!(Instant::now() < deadline, "writer did not start admission");
            std::thread::yield_now();
        }
        let mut samples = Vec::new();
        while !done.load(Ordering::Acquire) && samples.len() < 1000 {
            let start = Instant::now();
            old_pin.copy_file_to(&scene, &mut io::sink()).unwrap();
            samples.push(start.elapsed().as_secs_f64() * 1000.0);
            std::thread::sleep(Duration::from_millis(1));
        }
        let (b, writer_ms) = writer.join().unwrap();
        assert_eq!(b.generation, a.generation + 1);
        (samples, writer_ms)
    });
    assert!(!samples.is_empty());
    old_pin.close().unwrap();
    samples.sort_by(f64::total_cmp);
    let p95 = samples[(samples.len() * 95 / 100).min(samples.len() - 1)];
    eprintln!(
        "native_phased_reader size_mib={size_mib} samples={} min_ms={:.3} p95_ms={p95:.3} max_ms={:.3} writer_ms={writer_ms:.3}",
        samples.len(),
        samples[0],
        samples[samples.len() - 1]
    );
}

#[test]
#[ignore = "local release-only continuous multi-pin admission fairness probe"]
fn measure_native_continuous_multi_pin_admission() {
    let fixture = Fixture::new();
    let plan = SourceSpacePlan {
        free_headroom_bytes: 0,
        ..SourceSpacePlan::DEFAULT
    };
    let store = NativeSourceStore::open(&fixture.database(), plan).unwrap();
    let first_stage = minimal_source_stage(&fixture, "before multi-pin wait");
    let first = validate_staged_candidate(&first_stage, &[]).unwrap();
    let a = store.admit(0, 1, 1, [1; 32], &first).unwrap();
    let pins: Vec<_> = (0..4)
        .map(|_| store.pin_current(a.generation).unwrap())
        .collect();
    let stage = stage_n_mib_binary(&fixture, &first_stage, plan, 64, 7);
    let candidate = validate_staged_candidate(&stage, &[]).unwrap();
    let scene = SourcePath::parse("scenes/main.scene.json").unwrap();
    let stop = std::sync::atomic::AtomicBool::new(false);
    let done = std::sync::atomic::AtomicBool::new(false);
    let (starved, writer_ms, samples) = std::thread::scope(|scope| {
        let readers: Vec<_> = pins
            .into_iter()
            .map(|mut pin| {
                let scene = scene.clone();
                let stop = &stop;
                scope.spawn(move || {
                    let mut count = 0_u64;
                    let mut max_ms = 0.0_f64;
                    while !stop.load(Ordering::Acquire) {
                        let start = Instant::now();
                        pin.copy_file_to(&scene, &mut io::sink()).unwrap();
                        max_ms = max_ms.max(start.elapsed().as_secs_f64() * 1000.0);
                        count += 1;
                    }
                    pin.close().unwrap();
                    (count, max_ms)
                })
            })
            .collect();
        let writer = scope.spawn(|| {
            let start = Instant::now();
            let b = store
                .admit(a.generation, 1, 2, [2; 32], &candidate)
                .unwrap();
            done.store(true, Ordering::Release);
            assert_eq!(b.generation, a.generation + 1);
            start.elapsed().as_secs_f64() * 1000.0
        });
        let deadline = Instant::now() + Duration::from_secs(5);
        while !done.load(Ordering::Acquire) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        let starved = !done.load(Ordering::Acquire);
        stop.store(true, Ordering::Release);
        let writer_ms = writer.join().unwrap();
        let samples: Vec<_> = readers
            .into_iter()
            .map(|reader| reader.join().unwrap())
            .collect();
        (starved, writer_ms, samples)
    });
    assert_eq!(store.current_root().unwrap().unwrap().generation, 2);
    assert!(
        !starved,
        "publication waited past the local five-second probe window"
    );
    eprintln!(
        "native_multi_pin_admission readers=4 size_mib=64 writer_ms={writer_ms:.3} copies={samples:?}"
    );
}

#[test]
fn sqlite_full_inside_root_outcome_transaction_rolls_back_both() {
    let fixture = Fixture::new();
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
    let bytes = b"verified manifest blob for a space-fault transaction";
    let revision: [u8; 32] = Sha256::digest(bytes).into();
    store
        .stage_blob(&mut bytes.as_slice(), bytes.len() as u64, revision)
        .unwrap();
    store
        .conn
        .execute_batch(
            "CREATE TABLE fault_payload (bytes BLOB NOT NULL);
             CREATE TRIGGER fill_on_outcome BEFORE INSERT ON outcome
             BEGIN INSERT INTO fault_payload (bytes) VALUES (zeroblob(1048576)); END;",
        )
        .unwrap();
    let current_pages: i64 = store
        .conn
        .query_row("PRAGMA page_count", [], |row| row.get(0))
        .unwrap();
    store
        .conn
        .pragma_update(None, "max_page_count", current_pages)
        .unwrap();
    assert!(matches!(
        store.commit_probe(0, 1, 1, [1; 32], revision),
        Err(StoreError::Sql(rusqlite::Error::SqliteFailure(error, _)))
            if error.code == rusqlite::ErrorCode::DiskFull
    ));
    assert_eq!(store.root().unwrap(), (0, None));
    assert!(stored_outcome(&store.conn, 1, 1).unwrap().is_none());
    drop(store);

    let mut reopened = SqliteStoreCore::open(&fixture.database()).unwrap();
    assert_eq!(reopened.root().unwrap(), (0, None));
    assert!(stored_outcome(&reopened.conn, 1, 1).unwrap().is_none());
    reopened
        .conn
        .execute_batch("DROP TRIGGER fill_on_outcome;")
        .unwrap();
    assert_eq!(
        reopened.commit_probe(0, 1, 1, [1; 32], revision).unwrap(),
        1
    );
    assert_eq!(reopened.root().unwrap(), (1, Some(revision.to_vec())));
}

#[test]
fn blob_chunks_verify_before_complete_and_failed_stream_cannot_be_root() {
    let fixture = Fixture::new();
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
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
        Err(StoreError::HashMismatch)
    ));

    let retry_bytes = vec![8_u8; CHUNK_BYTES + 17];
    let retry_hash: [u8; 32] = Sha256::digest(&retry_bytes).into();
    let mut partial = &retry_bytes[..CHUNK_BYTES];
    assert!(matches!(
        store.stage_blob(&mut partial, retry_bytes.len() as u64, retry_hash),
        Err(StoreError::InputIo)
    ));
    assert!(matches!(
        store.commit_probe(1, 1, 2, [2; 32], retry_hash),
        Err(StoreError::MissingCompleteBlob)
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
        Err(StoreError::HashMismatch)
    ));
    assert!(matches!(
        store.commit_probe(1, 1, 2, [2; 32], bad_hash),
        Err(StoreError::MissingCompleteBlob)
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
fn incomplete_blob_retry_reuses_verified_chunks_without_deleting_them() {
    let fixture = Fixture::new();
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
    let bytes = vec![0x5a_u8; CHUNK_BYTES + 17];
    let hash: [u8; 32] = Sha256::digest(&bytes).into();
    assert!(matches!(
        store.stage_blob(&mut bytes[..CHUNK_BYTES].as_ref(), bytes.len() as u64, hash),
        Err(StoreError::InputIo)
    ));
    store
        .conn
        .execute_batch(
            "CREATE TRIGGER no_partial_delete BEFORE DELETE ON blob_chunks
             BEGIN SELECT RAISE(ABORT, 'matching partial chunks must be reused'); END;",
        )
        .unwrap();
    store
        .stage_blob(&mut bytes.as_slice(), bytes.len() as u64, hash)
        .unwrap();
    let (state, count): (i64, i64) = store
        .conn
        .query_row(
            "SELECT state, (SELECT count(*) FROM blob_chunks WHERE hash=?1)
             FROM blob_header WHERE hash=?1",
            params![hash.as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!((state, count), (1, 2));
}

#[test]
fn mismatched_partial_chunk_is_repaired_in_one_bounded_write() {
    let fixture = Fixture::new();
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
    let bytes = vec![0x6b_u8; CHUNK_BYTES + 17];
    let hash: [u8; 32] = Sha256::digest(&bytes).into();
    assert!(matches!(
        store.stage_blob(&mut bytes[..CHUNK_BYTES].as_ref(), bytes.len() as u64, hash),
        Err(StoreError::InputIo)
    ));
    store
        .conn
        .execute(
            "UPDATE blob_chunks SET bytes=zeroblob(length(bytes)) WHERE hash=?1 AND chunk_index=0",
            params![hash.as_slice()],
        )
        .unwrap();
    store
        .stage_blob(&mut bytes.as_slice(), bytes.len() as u64, hash)
        .unwrap();
    let (state, count): (i64, i64) = store
        .conn
        .query_row(
            "SELECT state, (SELECT count(*) FROM blob_chunks WHERE hash=?1)
             FROM blob_header WHERE hash=?1",
            params![hash.as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!((state, count), (1, 2));
    assert_eq!(store.commit_probe(0, 1, 1, [1; 32], hash).unwrap(), 1);
}

#[test]
fn malformed_partial_cleanup_resumes_after_sql_failure() {
    let fixture = Fixture::new();
    let database = fixture.database();
    let mut store = SqliteStoreCore::open(&database).unwrap();
    let bytes = vec![0x7c_u8; CHUNK_BYTES * 2 + 17];
    let hash: [u8; 32] = Sha256::digest(&bytes).into();
    assert!(matches!(
        store.stage_blob(
            &mut bytes[..CHUNK_BYTES * 2].as_ref(),
            bytes.len() as u64,
            hash
        ),
        Err(StoreError::InputIo)
    ));
    store
        .conn
        .execute(
            "DELETE FROM blob_chunks WHERE hash=?1 AND chunk_index=0",
            params![hash.as_slice()],
        )
        .unwrap();
    store
        .conn
        .execute(
            "INSERT INTO blob_chunks (hash, chunk_index, bytes) VALUES (?1, 3, x'00')",
            params![hash.as_slice()],
        )
        .unwrap();
    store
        .conn
        .execute_batch(
            "CREATE TABLE cleanup_calls (count INTEGER NOT NULL);
             INSERT INTO cleanup_calls VALUES (0);
             CREATE TRIGGER fail_second_cleanup BEFORE DELETE ON blob_chunks
             BEGIN
               UPDATE cleanup_calls SET count=count+1;
               SELECT CASE WHEN (SELECT count FROM cleanup_calls)=2
                 THEN RAISE(ABORT, 'injected cleanup failure') END;
             END;",
        )
        .unwrap();
    assert!(matches!(
        store.stage_blob(&mut bytes.as_slice(), bytes.len() as u64, hash),
        Err(StoreError::Sql(_))
    ));
    let remaining: i64 = store
        .conn
        .query_row(
            "SELECT count(*) FROM blob_chunks WHERE hash=?1",
            params![hash.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(remaining, 1);
    drop(store);

    let mut reopened = SqliteStoreCore::open(&database).unwrap();
    reopened
        .conn
        .execute_batch("DROP TRIGGER fail_second_cleanup;")
        .unwrap();
    reopened
        .stage_blob(&mut bytes.as_slice(), bytes.len() as u64, hash)
        .unwrap();
    assert_eq!(reopened.commit_probe(0, 1, 1, [1; 32], hash).unwrap(), 1);
}

#[test]
fn partial_header_mismatch_refuses_without_deleting_chunks() {
    let fixture = Fixture::new();
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
    let bytes = vec![0x3e_u8; CHUNK_BYTES + 17];
    let hash: [u8; 32] = Sha256::digest(&bytes).into();
    assert!(matches!(
        store.stage_blob(&mut bytes[..CHUNK_BYTES].as_ref(), bytes.len() as u64, hash),
        Err(StoreError::InputIo)
    ));
    store
        .conn
        .execute(
            "UPDATE blob_header SET chunk_count=chunk_count+1 WHERE hash=?1",
            params![hash.as_slice()],
        )
        .unwrap();
    assert!(matches!(
        store.stage_blob(&mut bytes.as_slice(), bytes.len() as u64, hash),
        Err(StoreError::BlobMetadataMismatch)
    ));
    let (state, count): (i64, i64) = store
        .conn
        .query_row(
            "SELECT state, (SELECT count(*) FROM blob_chunks WHERE hash=?1)
             FROM blob_header WHERE hash=?1",
            params![hash.as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!((state, count), (0, 1));
}

#[test]
fn oversized_partial_chunk_refuses_automatic_cleanup() {
    let fixture = Fixture::new();
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
    let bytes = b"small expected blob";
    let hash: [u8; 32] = Sha256::digest(bytes).into();
    store
        .conn
        .execute(
            "INSERT INTO blob_header (hash, length, chunk_count, state) VALUES (?1, ?2, 1, 0)",
            params![hash.as_slice(), bytes.len() as i64],
        )
        .unwrap();
    store
        .conn
        .execute(
            "INSERT INTO blob_chunks (hash, chunk_index, bytes) VALUES (?1, 0, zeroblob(?2))",
            params![hash.as_slice(), CHUNK_BYTES as i64 + 1],
        )
        .unwrap();
    assert!(matches!(
        store.stage_blob(&mut bytes.as_slice(), bytes.len() as u64, hash),
        Err(StoreError::InvalidChunkLayout)
    ));
    let (state, length): (i64, i64) = store
        .conn
        .query_row(
            "SELECT state, (SELECT length(bytes) FROM blob_chunks WHERE hash=?1 AND chunk_index=0)
             FROM blob_header WHERE hash=?1",
            params![hash.as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!((state, length), (0, CHUNK_BYTES as i64 + 1));
}

#[test]
fn extra_stored_chunk_cannot_become_complete() {
    let fixture = Fixture::new();
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
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
        Err(StoreError::InvalidChunkLayout)
    ));
    assert!(matches!(
        store.commit_probe(0, 1, 1, [1; 32], hash),
        Err(StoreError::MissingCompleteBlob)
    ));
}

#[test]
fn completion_transaction_excludes_same_hash_retry_reset() {
    let fixture = Fixture::new();
    let mut first = SqliteStoreCore::open(&fixture.database()).unwrap();
    let mut second = SqliteStoreCore::open(&fixture.database()).unwrap();
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
                Err(StoreError::Sql(_))
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
    let mut first = SqliteStoreCore::open(&fixture.database()).unwrap();
    let mut second = SqliteStoreCore::open(&fixture.database()).unwrap();
    let complete_bytes = vec![5_u8; CHUNK_BYTES + 17];
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
        Err(StoreError::StaleBlobWriter)
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
    assert_eq!(count, 2);
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

    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
    store
        .stage_blob(&mut canonical.as_slice(), canonical.len() as u64, revision)
        .unwrap();
    assert_eq!(store.commit_probe(0, 1, 1, [1; 32], revision).unwrap(), 1);
    drop(store);

    let reopened = SqliteStoreCore::open(&fixture.database()).unwrap();
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
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
    let manifest = TreeManifest::new(
        "2a5ef644-7225-4770-9f4d-e1d9c0a7d515",
        vec!["external-root".to_owned()],
        Vec::new(),
    )
    .unwrap();
    assert!(matches!(
        store.stage_manifest_chunked(&manifest),
        Err(StoreError::UnsupportedDependencyRoots)
    ));
    assert!(matches!(
        store.commit_manifest_probe(0, 1, 1, [1; 32], &manifest),
        Err(StoreError::UnsupportedDependencyRoots)
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
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
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

    let reopened = SqliteStoreCore::open(&fixture.database()).unwrap();
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
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
    let revision = store.stage_manifest_chunked(&manifest).unwrap();
    store
        .stage_blob(&mut first.as_slice(), first.len() as u64, first_hash)
        .unwrap();
    assert!(matches!(
        store.commit_manifest_probe(0, 1, 1, [1; 32], &manifest),
        Err(StoreError::MissingCompleteFile)
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
        Err(StoreError::MissingCompleteFile)
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

    let mut reopened = SqliteStoreCore::open(&fixture.database()).unwrap();
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
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
    let first = b"first source";
    let second = b"second source";
    let first_hash: [u8; 32] = Sha256::digest(first).into();
    let second_hash: [u8; 32] = Sha256::digest(second).into();
    assert!(matches!(
        store.stage_small_blob_batch([
            (first_hash, first.as_slice()),
            (second_hash, b"wrong source".as_slice()),
        ]),
        Err(StoreError::HashMismatch)
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
    let reopened = SqliteStoreCore::open(&fixture.database()).unwrap();
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

#[test]
fn small_blob_batch_reuses_and_repairs_incomplete_chunk() {
    let fixture = Fixture::new();
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
    let bytes = b"reused tiny source";
    let hash: [u8; 32] = Sha256::digest(bytes).into();
    store
        .conn
        .execute(
            "INSERT INTO blob_header (hash, length, chunk_count, state) VALUES (?1, ?2, 1, 0)",
            params![hash.as_slice(), bytes.len() as i64],
        )
        .unwrap();
    store
        .conn
        .execute(
            "INSERT INTO blob_chunks (hash, chunk_index, bytes) VALUES (?1, 0, ?2)",
            params![hash.as_slice(), bytes.as_slice()],
        )
        .unwrap();
    store
        .conn
        .execute_batch(
            "CREATE TRIGGER no_small_retry_delete BEFORE DELETE ON blob_chunks
             BEGIN SELECT RAISE(ABORT, 'small retry must reuse'); END;",
        )
        .unwrap();
    store
        .stage_small_blob_batch([(hash, bytes.as_slice())])
        .unwrap();
    store
        .conn
        .execute(
            "UPDATE blob_header SET state=0 WHERE hash=?1",
            params![hash.as_slice()],
        )
        .unwrap();
    store
        .conn
        .execute(
            "UPDATE blob_chunks SET bytes=zeroblob(length(bytes)) WHERE hash=?1",
            params![hash.as_slice()],
        )
        .unwrap();
    store
        .stage_small_blob_batch([(hash, bytes.as_slice())])
        .unwrap();
    let stored: Vec<u8> = store
        .conn
        .query_row(
            "SELECT bytes FROM blob_chunks WHERE hash=?1 AND chunk_index=0",
            params![hash.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stored, bytes);
}

#[test]
fn small_blob_batch_refuses_partial_header_mismatch_without_overwriting() {
    let fixture = Fixture::new();
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
    let bytes = b"header identity";
    let hash: [u8; 32] = Sha256::digest(bytes).into();
    store
        .conn
        .execute(
            "INSERT INTO blob_header (hash, length, chunk_count, state) VALUES (?1, ?2, 1, 0)",
            params![hash.as_slice(), bytes.len() as i64 + 1],
        )
        .unwrap();
    assert!(matches!(
        store.stage_small_blob_batch([(hash, bytes.as_slice())]),
        Err(StoreError::BlobMetadataMismatch)
    ));
    let (length, state): (i64, i64) = store
        .conn
        .query_row(
            "SELECT length, state FROM blob_header WHERE hash=?1",
            params![hash.as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!((length, state), (bytes.len() as i64 + 1, 0));
}

#[test]
fn small_blob_batch_cleans_wrong_length_partial_before_retry() {
    let fixture = Fixture::new();
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
    let bytes = b"exact tiny source";
    let hash: [u8; 32] = Sha256::digest(bytes).into();
    store
        .conn
        .execute(
            "INSERT INTO blob_header (hash, length, chunk_count, state) VALUES (?1, ?2, 1, 0)",
            params![hash.as_slice(), bytes.len() as i64],
        )
        .unwrap();
    store
        .conn
        .execute(
            "INSERT INTO blob_chunks (hash, chunk_index, bytes) VALUES (?1, 0, x'00')",
            params![hash.as_slice()],
        )
        .unwrap();
    assert!(matches!(
        store.stage_small_blob_batch([(hash, bytes.as_slice())]),
        Err(StoreError::InvalidChunkLayout)
    ));
    let remaining: i64 = store
        .conn
        .query_row(
            "SELECT count(*) FROM blob_chunks WHERE hash=?1",
            params![hash.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(remaining, 0);
    store
        .stage_small_blob_batch([(hash, bytes.as_slice())])
        .unwrap();
}

#[test]
fn small_blob_batch_cleans_malformed_partial_one_row_at_a_time() {
    let fixture = Fixture::new();
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
    let bytes = b"bounded tiny retry";
    let hash: [u8; 32] = Sha256::digest(bytes).into();
    let fresh = b"new batch member";
    let fresh_hash: [u8; 32] = Sha256::digest(fresh).into();
    store
        .conn
        .execute(
            "INSERT INTO blob_header (hash, length, chunk_count, state) VALUES (?1, ?2, 1, 0)",
            params![hash.as_slice(), bytes.len() as i64],
        )
        .unwrap();
    for index in [1_i64, 2] {
        store
            .conn
            .execute(
                "INSERT INTO blob_chunks (hash, chunk_index, bytes) VALUES (?1, ?2, x'00')",
                params![hash.as_slice(), index],
            )
            .unwrap();
    }
    store
        .conn
        .execute_batch(
            "CREATE TABLE tiny_cleanup_calls (count INTEGER NOT NULL);
             INSERT INTO tiny_cleanup_calls VALUES (0);
             CREATE TRIGGER fail_second_tiny_cleanup BEFORE DELETE ON blob_chunks
             BEGIN
               UPDATE tiny_cleanup_calls SET count=count+1;
               SELECT CASE WHEN (SELECT count FROM tiny_cleanup_calls)=2
                 THEN RAISE(ABORT, 'second deletion') END;
             END;",
        )
        .unwrap();
    assert!(matches!(
        store.stage_small_blob_batch([(fresh_hash, fresh.as_slice()), (hash, bytes.as_slice())]),
        Err(StoreError::Sql(_))
    ));
    let fresh_count: i64 = store
        .conn
        .query_row(
            "SELECT count(*) FROM blob_header WHERE hash=?1",
            params![fresh_hash.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(fresh_count, 0);
    let (state, remaining): (i64, i64) = store
        .conn
        .query_row(
            "SELECT state, (SELECT count(*) FROM blob_chunks WHERE hash=?1)
             FROM blob_header WHERE hash=?1",
            params![hash.as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!((state, remaining), (0, 1));
    store
        .conn
        .execute_batch("DROP TRIGGER fail_second_tiny_cleanup;")
        .unwrap();
    assert!(matches!(
        store.stage_small_blob_batch([(fresh_hash, fresh.as_slice()), (hash, bytes.as_slice())]),
        Err(StoreError::InvalidChunkLayout)
    ));
    store
        .stage_small_blob_batch([(fresh_hash, fresh.as_slice()), (hash, bytes.as_slice())])
        .unwrap();
    let completed: i64 = store
        .conn
        .query_row(
            "SELECT count(*) FROM blob_header WHERE state=1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(completed, 2);
}

#[cfg(test)]
const LINEAGE_PROJECT_ID: &str = "2a5ef644-7225-4770-9f4d-e1d9c0a7d515";
#[cfg(test)]
const LINEAGE_HISTORY_ID: &str = "82b0ec04-6866-4271-a2cf-1f1576ad604a";

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
const CONSTANT_LIGHT_SCENE_ID: &str = "3a5ef644-7225-4770-9f4d-e1d9c0a7d515";
#[cfg(test)]
const CONSTANT_LIGHT_SOURCE_ID: &str = "10000000-0000-4000-8000-000000000001";
#[cfg(test)]
const CONSTANT_LIGHT_ID: &str = "10000000-0000-4000-8000-000000000002";
#[cfg(test)]
const CONSTANT_LIGHT_SOURCE_PATH: &str = "records/environment.record.json";
#[cfg(test)]
const CONSTANT_LIGHT_OPAQUE_VALUE: &str =
    r#"{ "decimal":1.00, "escaped":"\u0061", "list":[true, null, -0] }"#;

#[cfg(test)]
fn constant_light_source_stage(
    fixture: &Fixture,
    illuminance_lux: i64,
    selected_source_id: &str,
    sun_light_id: &str,
) -> crate::StagedTree {
    let root = fixture.source_root();
    for directory in ["records", "scenes", "schemas"] {
        fs::create_dir_all(root.join(directory)).unwrap();
    }
    let paths = [
        "project.json",
        CONSTANT_LIGHT_SOURCE_PATH,
        "records/light.record.json",
        "scenes/main.scene.json",
        "schemas/history.registry.json",
    ];
    fs::write(
        root.join("project.json"),
        serde_json::to_vec(&serde_json::json!({
            "id":LINEAGE_PROJECT_ID,"type":"core.project","schema":2,
            "fields":{
                "project.format":1,
                "project.records":[CONSTANT_LIGHT_SOURCE_PATH,"records/light.record.json","scenes/main.scene.json"],
                "project.schemas":[],
                "project.registryHistory":"schemas/history.registry.json",
                "project.entryScenes":[CONSTANT_LIGHT_SCENE_ID]
            }
        })).unwrap(),
    ).unwrap();
    fs::write(
        root.join("scenes/main.scene.json"),
        serde_json::to_vec(&serde_json::json!({
            "id":CONSTANT_LIGHT_SCENE_ID,"type":"core.scene","schema":2,
            "fields":{"scene.label":"constant light entry","environment.source":selected_source_id}
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        root.join(CONSTANT_LIGHT_SOURCE_PATH),
        format!(
            r#"{{
  "id":"{CONSTANT_LIGHT_SOURCE_ID}","type":"thrum.environment.authored","schema":1,
  "fields":{{
    "profile":"constant-light-v1",
    "sun":{{
      "light_ref":"{sun_light_id}","toward_source_unit":[0,1,0],
      "perpendicular_illuminance_lux":{illuminance_lux},
      "unit_luminance_linear_rec709":[1,1,1],"angular_radius_rad":0.00465,
      "direction_model":"geometric-straight","applied_attenuation":[]
    }},
    "ambient_irradiance_lux":[1000,1000,1000],"background_radiance_cd_m2":[0,0,0],
    "unsupported":["wind","cloud_tau","precipitation","air","thermal","moon","air_quality"],
    "x.thrum.environment.note":{CONSTANT_LIGHT_OPAQUE_VALUE}
  }}
}}"#
        ),
    )
    .unwrap();
    fs::write(
        root.join("records/light.record.json"),
        serde_json::to_vec(&serde_json::json!({
            "id":CONSTANT_LIGHT_ID,"type":"thrum.light.identity","schema":1,
            "fields":{"driver_binding":"incoming-environment-sun"}
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
    let declared = paths.map(|path| SourcePath::parse(path).unwrap());
    stage_declared_tree_with(
        &root,
        &declared,
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
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
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
        Err(StoreError::CandidateStage(
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
        Err(StoreError::HashMismatch)
    ));
    assert_eq!(store.root().unwrap().0, 1);
}

#[test]
fn staged_candidate_publication_keeps_prior_root_on_drift_and_recovers_retry() {
    let fixture = Fixture::new();
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
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
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
    assert_eq!(
        store
            .admit_staged_candidate_probe(1, 1, 2, [2; 32], &second)
            .unwrap(),
        (2, second_revision)
    );
    assert!(matches!(
        store.admit_staged_candidate_probe(1, 1, 2, [3; 32], &second),
        Err(StoreError::TokenConflict)
    ));
    assert!(matches!(
        store.admit_staged_candidate_probe(1, 1, 3, [3; 32], &first),
        Err(StoreError::StaleGeneration)
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
        Err(StoreError::HashMismatch)
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
fn current_root_read_pin_survives_root_advance_and_releases_after_validation_error() {
    let fixture = Fixture::new();
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
    let first_stage = minimal_source_stage(&fixture, "old pinned scene");
    let first = validate_staged_candidate(&first_stage, &[]).unwrap();
    let (_, first_revision) = store
        .admit_staged_candidate_probe(0, 1, 1, [1; 32], &first)
        .unwrap();
    let pin = store.pin_current_root(1).unwrap();
    assert_eq!((pin.generation, pin.revision), (1, first_revision));

    let second_stage = minimal_source_stage(&fixture, "new selected scene");
    let second = validate_staged_candidate(&second_stage, &[]).unwrap();
    let (_, second_revision) = store
        .admit_staged_candidate_probe(1, 1, 2, [2; 32], &second)
        .unwrap();
    assert_ne!(first_revision, second_revision);
    drop(store);
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
    assert_eq!(store.root().unwrap(), (2, Some(second_revision.to_vec())));
    let pinned_revision: Vec<u8> = store
        .conn
        .query_row(
            "SELECT revision FROM read_pin WHERE id=?1",
            params![pin.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(pinned_revision, first_revision);
    let old_manifest = {
        let tx = store
            .conn
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .unwrap();
        let bytes = read_complete_blob(&tx, first_revision, FIRST_PROFILE_MANIFEST_BYTES).unwrap();
        tx.commit().unwrap();
        bytes
    };
    assert_eq!(
        *decode_manifest_jcs(&old_manifest, FIRST_PROFILE_MANIFEST_BYTES)
            .unwrap()
            .revision()
            .as_bytes(),
        first_revision
    );
    store.release_root_pin(pin.id).unwrap();
    assert!(matches!(
        store.compare_history_with_current_root(2, "wrong-project", &[]),
        Err(StoreError::DifferentProject)
    ));
    let pin_count: i64 = store
        .conn
        .query_row("SELECT count(*) FROM read_pin", [], |row| row.get(0))
        .unwrap();
    assert_eq!(pin_count, 0);
}

#[test]
fn busy_pin_release_retains_id_until_next_bounded_read() {
    let fixture = Fixture::new();
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
    let stage = minimal_source_stage(&fixture, "pinned scene");
    let candidate = validate_staged_candidate(&stage, &[]).unwrap();
    store
        .admit_staged_candidate_probe(0, 1, 1, [1; 32], &candidate)
        .unwrap();
    let pin = store.pin_current_root(1).unwrap();
    let blocker = Connection::open(fixture.database()).unwrap();
    blocker.execute_batch("BEGIN IMMEDIATE").unwrap();
    assert!(matches!(
        store.release_root_pin(pin.id),
        Err(StoreError::Sql(_))
    ));
    assert_eq!(store.pending_pin_releases, vec![pin.id]);
    blocker.execute_batch("COMMIT").unwrap();

    let history = fs::read(fixture.source_root().join("schemas/history.registry.json")).unwrap();
    store
        .compare_history_with_current_root(1, LINEAGE_PROJECT_ID, &history)
        .unwrap();
    assert!(store.pending_pin_releases.is_empty());
    let pin_count: i64 = store
        .conn
        .query_row("SELECT count(*) FROM read_pin", [], |row| row.get(0))
        .unwrap();
    assert_eq!(pin_count, 0);

    let pin = store.pin_current_root(1).unwrap();
    blocker.execute_batch("BEGIN IMMEDIATE").unwrap();
    assert!(matches!(
        store.release_root_pin(pin.id),
        Err(StoreError::Sql(_))
    ));
    blocker.execute_batch("COMMIT").unwrap();
    store.release_root_pin(pin.id).unwrap();
    assert!(store.pending_pin_releases.is_empty());
    store
        .compare_history_with_current_root(1, LINEAGE_PROJECT_ID, &history)
        .unwrap();
}

#[test]
fn gc_mark_keeps_selected_and_pinned_manifest_closures() {
    let fixture = Fixture::new();
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
    let first_stage = minimal_source_stage(&fixture, "old scene for GC");
    let first = validate_staged_candidate(&first_stage, &[]).unwrap();
    let old_manifest = first.provisional_manifest().unwrap();
    let old_scene = first_stage
        .files()
        .iter()
        .find(|file| file.path().as_str() == "scenes/main.scene.json")
        .unwrap()
        .sha256()
        .to_owned();
    let (_, old_revision) = store
        .admit_staged_candidate_probe(0, 1, 1, [1; 32], &first)
        .unwrap();
    let pin = store.pin_current_root(1).unwrap();
    let second_stage = minimal_source_stage(&fixture, "new scene for GC");
    let second = validate_staged_candidate(&second_stage, &[]).unwrap();
    let new_manifest = second.provisional_manifest().unwrap();
    let (_, new_revision) = store
        .admit_staged_candidate_probe(1, 1, 2, [2; 32], &second)
        .unwrap();
    let orphan = b"unpublished source blob";
    let orphan_hash: [u8; 32] = Sha256::digest(orphan).into();
    store
        .stage_blob(&mut orphan.as_slice(), orphan.len() as u64, orphan_hash)
        .unwrap();

    assert_eq!(store.mark_unreachable_probe().unwrap(), 1);
    let marked: Vec<Vec<u8>> = {
        let mut scan = store
            .conn
            .prepare("SELECT hash FROM gc_mark ORDER BY hash")
            .unwrap();
        scan.query_map([], |row| row.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    };
    assert_eq!(marked, vec![orphan_hash.to_vec()]);

    store.release_root_pin(pin.id).unwrap();
    let marked_count = store.mark_unreachable_probe().unwrap();
    let still_reachable: HashSet<[u8; 32]> = std::iter::once(new_revision)
        .chain(
            new_manifest
                .files()
                .iter()
                .map(|file| file.blob_identity().1),
        )
        .collect();
    let expected: HashSet<[u8; 32]> = std::iter::once(old_revision)
        .chain(
            old_manifest
                .files()
                .iter()
                .map(|file| file.blob_identity().1),
        )
        .chain(std::iter::once(orphan_hash))
        .filter(|hash| !still_reachable.contains(hash))
        .collect();
    let marked: HashSet<[u8; 32]> = {
        let mut scan = store.conn.prepare("SELECT hash FROM gc_mark").unwrap();
        scan.query_map([], |row| row.get::<_, Vec<u8>>(0))
            .unwrap()
            .map(|row| row.unwrap().try_into().unwrap())
            .collect()
    };
    assert!(expected.contains(&old_revision));
    assert!(expected.contains(&old_scene));
    assert_eq!(marked_count, expected.len());
    assert_eq!(marked, expected);
}

#[test]
fn gc_mark_refuses_publication_or_pin_change_after_snapshot() {
    let fixture = Fixture::new();
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
    let first_stage = minimal_source_stage(&fixture, "old selected scene");
    let first = validate_staged_candidate(&first_stage, &[]).unwrap();
    store
        .admit_staged_candidate_probe(0, 1, 1, [1; 32], &first)
        .unwrap();
    let orphan = b"preexisting mark";
    let orphan_hash: [u8; 32] = Sha256::digest(orphan).into();
    store
        .stage_blob(&mut orphan.as_slice(), orphan.len() as u64, orphan_hash)
        .unwrap();
    assert_eq!(store.mark_unreachable_probe().unwrap(), 1);
    let second_stage = minimal_source_stage(&fixture, "new selected scene");
    let second = validate_staged_candidate(&second_stage, &[]).unwrap();
    assert!(matches!(
        store.mark_unreachable_probe_with(|| {
            let mut publisher = SqliteStoreCore::open(&fixture.database()).unwrap();
            publisher
                .admit_staged_candidate_probe(1, 1, 2, [2; 32], &second)
                .unwrap();
        }),
        Err(StoreError::StaleGcEpoch)
    ));
    let marked: Vec<Vec<u8>> = {
        let mut scan = store.conn.prepare("SELECT hash FROM gc_mark").unwrap();
        scan.query_map([], |row| row.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    };
    assert_eq!(marked, vec![orphan_hash.to_vec()]);
    assert!(matches!(
        store.mark_unreachable_probe_with(|| {
            SqliteStoreCore::open(&fixture.database())
                .unwrap()
                .pin_current_root(2)
                .unwrap();
        }),
        Err(StoreError::StaleGcEpoch)
    ));
    assert_eq!(store.mark_unreachable_probe().unwrap(), 3);
}

#[test]
fn gc_mark_limits_writer_batch_and_keeps_previous_marks_on_refusal() {
    let fixture = Fixture::new();
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
    let blobs: Vec<[u8; 2]> = (0..=GC_MARK_CANDIDATES)
        .map(|index| (index as u16).to_le_bytes())
        .collect();
    store
        .stage_small_blob_batch(blobs[..GC_MARK_CANDIDATES].iter().map(|bytes| {
            let hash: [u8; 32] = Sha256::digest(bytes).into();
            (hash, bytes.as_slice())
        }))
        .unwrap();
    assert_eq!(store.mark_unreachable_probe().unwrap(), GC_MARK_CANDIDATES);
    let last = blobs.last().unwrap();
    let last_hash: [u8; 32] = Sha256::digest(last).into();
    store
        .stage_small_blob_batch([(last_hash, last.as_slice())])
        .unwrap();
    assert!(matches!(
        store.mark_unreachable_probe(),
        Err(StoreError::GcMarkLimit)
    ));
    let marks: i64 = store
        .conn
        .query_row("SELECT count(*) FROM gc_mark", [], |row| row.get(0))
        .unwrap();
    assert_eq!(marks, GC_MARK_CANDIDATES as i64);
    let last_marked: bool = store
        .conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM gc_mark WHERE hash=?1)",
            params![last_hash.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    assert!(!last_marked);
}

#[test]
fn gc_mark_uses_complete_only_index_with_many_incomplete_headers() {
    let fixture = Fixture::new();
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
    {
        let tx = store.conn.transaction().unwrap();
        let mut insert = tx
            .prepare(
                "INSERT INTO blob_header (hash, length, chunk_count, state) VALUES (?1, 0, 0, 0)",
            )
            .unwrap();
        for index in 0..=GC_MARK_SCAN_BLOBS {
            let mut hash = [0_u8; 32];
            hash[..8].copy_from_slice(&(index as u64).to_le_bytes());
            insert.execute(params![hash.as_slice()]).unwrap();
        }
        drop(insert);
        tx.commit().unwrap();
    }
    let plan: String = store
        .conn
        .query_row(
            "EXPLAIN QUERY PLAN SELECT hash FROM blob_header WHERE state=1 ORDER BY hash",
            [],
            |row| row.get(3),
        )
        .unwrap();
    assert!(plan.contains("complete_blob_hash"), "{plan}");
    assert_eq!(store.mark_unreachable_probe().unwrap(), 0);
}

#[test]
fn game_schema_candidate_reaches_root_with_exact_optional_bytes() {
    const SCENE_ID: &str = "3a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    const GAME_ID: &str = "4a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    let fixture = Fixture::new();
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
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
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
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
fn constant_light_candidate_reaches_root_with_reopened_exact_source_and_idempotent_retry() {
    let fixture = Fixture::new();
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
    let first_stage = constant_light_source_stage(
        &fixture,
        100000,
        CONSTANT_LIGHT_SOURCE_ID,
        CONSTANT_LIGHT_ID,
    );
    let first = validate_staged_candidate(&first_stage, &[]).unwrap();
    assert_eq!(first.records().required_edges(), 2);
    assert_eq!(first.records().scenes()[0].schema(), 2);
    assert_eq!(
        first.records().scenes()[0].environment_source(),
        Some(CONSTANT_LIGHT_SOURCE_ID)
    );
    assert_eq!(first.records().builtins().len(), 2);
    assert!(first.records().games().is_empty());
    let (generation, first_revision) = store
        .admit_staged_candidate_probe(0, 1, 1, [1; 32], &first)
        .unwrap();
    assert_eq!(generation, 1);
    assert_eq!(
        first_revision,
        *first.provisional_manifest().unwrap().revision().as_bytes()
    );

    let second_stage =
        constant_light_source_stage(&fixture, 75000, CONSTANT_LIGHT_SOURCE_ID, CONSTANT_LIGHT_ID);
    let second = validate_staged_candidate(&second_stage, &[]).unwrap();
    let (generation, second_revision) = store
        .admit_staged_candidate_probe(1, 1, 2, [2; 32], &second)
        .unwrap();
    assert_eq!(generation, 2);
    assert_eq!(
        second_revision,
        *second.provisional_manifest().unwrap().revision().as_bytes()
    );
    assert_ne!(first_revision, second_revision);
    drop(store);

    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
    assert_eq!(store.root().unwrap(), (2, Some(second_revision.to_vec())));
    // Read both manifests and all their complete source blobs through the
    // reopened store. The current checkout has already changed to revision 2.
    for (revision, candidate, stage, expected_illuminance) in [
        (first_revision, &first, &first_stage, 100000.0),
        (second_revision, &second, &second_stage, 75000.0),
    ] {
        let tx = store
            .conn
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .unwrap();
        let manifest_bytes =
            read_complete_blob(&tx, revision, FIRST_PROFILE_MANIFEST_BYTES).unwrap();
        let stored_manifest =
            decode_manifest_jcs(&manifest_bytes, FIRST_PROFILE_MANIFEST_BYTES).unwrap();
        assert_eq!(*stored_manifest.revision().as_bytes(), revision);
        assert_eq!(
            stored_manifest.files(),
            candidate.provisional_manifest().unwrap().files()
        );
        let identities: Vec<_> = stored_manifest
            .files()
            .iter()
            .filter_map(ManifestEntry::record_identity)
            .collect();
        assert_eq!(
            identities,
            [
                (LINEAGE_PROJECT_ID, "core.project"),
                (CONSTANT_LIGHT_SOURCE_ID, "thrum.environment.authored"),
                (CONSTANT_LIGHT_ID, "thrum.light.identity"),
                (CONSTANT_LIGHT_SCENE_ID, "core.scene"),
                (LINEAGE_HISTORY_ID, "core.registry-history"),
            ]
        );
        for file in stored_manifest.files() {
            let (length, hash) = file.blob_identity();
            let complete = read_complete_blob(&tx, hash, crate::MAX_RECORD_BYTES).unwrap();
            assert_eq!(complete.len() as u64, length);
        }
        let environment = stored_manifest
            .files()
            .iter()
            .find(|file| {
                file.record_identity()
                    == Some((CONSTANT_LIGHT_SOURCE_ID, "thrum.environment.authored"))
            })
            .unwrap();
        let environment_bytes =
            read_complete_blob(&tx, environment.blob_identity().1, crate::MAX_RECORD_BYTES)
                .unwrap();
        assert_eq!(
            environment_bytes,
            fs::read(stage.test_directory().join(CONSTANT_LIGHT_SOURCE_PATH)).unwrap()
        );
        let parsed = crate::environment_json::parse_environment(&environment_bytes).unwrap();
        assert_eq!(
            parsed.sun.perpendicular_illuminance_lux,
            expected_illuminance
        );
        assert_eq!(
            parsed.extensions,
            [(
                "x.thrum.environment.note".to_owned(),
                CONSTANT_LIGHT_OPAQUE_VALUE.to_owned()
            )]
        );
        tx.commit().unwrap();
    }
    // A current retry and a historical retry return their original outcomes,
    // without rewinding the current root or incrementing its generation.
    assert_eq!(
        store
            .admit_staged_candidate_probe(1, 1, 2, [2; 32], &second)
            .unwrap(),
        (2, second_revision)
    );
    assert_eq!(
        store
            .admit_staged_candidate_probe(0, 1, 1, [1; 32], &first)
            .unwrap(),
        (1, first_revision)
    );
    assert!(matches!(
        store.admit_staged_candidate_probe(1, 1, 2, [3; 32], &second),
        Err(StoreError::TokenConflict)
    ));
    assert_eq!(store.root().unwrap(), (2, Some(second_revision.to_vec())));

    // F3 refusal occurs before a typed candidate can reach the root probe.
    for (illuminance, source_ref, light_ref, expected_error) in [
        (-1, CONSTANT_LIGHT_SOURCE_ID, CONSTANT_LIGHT_ID, "quantity"),
        (
            100000,
            CONSTANT_LIGHT_SOURCE_ID,
            CONSTANT_LIGHT_SCENE_ID,
            "light",
        ),
        (100000, CONSTANT_LIGHT_ID, CONSTANT_LIGHT_ID, "source"),
    ] {
        let invalid_stage =
            constant_light_source_stage(&fixture, illuminance, source_ref, light_ref);
        let error = validate_staged_candidate(&invalid_stage, &[])
            .err()
            .unwrap();
        match (expected_error, error) {
            (
                "quantity",
                crate::registry_stage::StagedCandidateError::Records(
                    crate::registry_stage::StagedRecordError::Builtin { .. },
                ),
            )
            | (
                "light",
                crate::registry_stage::StagedCandidateError::Records(
                    crate::registry_stage::StagedRecordError::UnresolvedEnvironmentLight { .. },
                ),
            )
            | (
                "source",
                crate::registry_stage::StagedCandidateError::Records(
                    crate::registry_stage::StagedRecordError::UnresolvedEnvironmentSource {
                        ..
                    },
                ),
            ) => {}
            (_, other) => panic!("unexpected constant-light refusal: {other:?}"),
        }
        assert_eq!(store.root().unwrap(), (2, Some(second_revision.to_vec())));
        assert!(stored_outcome(&store.conn, 1, 3).unwrap().is_none());
    }
    let outcomes: i64 = store
        .conn
        .query_row("SELECT count(*) FROM outcome", [], |row| row.get(0))
        .unwrap();
    assert_eq!(outcomes, 2);
    drop(store);
    let reopened = SqliteStoreCore::open(&fixture.database()).unwrap();
    assert_eq!(
        reopened.root().unwrap(),
        (2, Some(second_revision.to_vec()))
    );
}

#[test]
fn constant_light_pinned_source_reads_old_revision_after_root_advance_and_refuses_corruption() {
    let fixture = Fixture::new();
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
    let first_stage = constant_light_source_stage(
        &fixture,
        100000,
        CONSTANT_LIGHT_SOURCE_ID,
        CONSTANT_LIGHT_ID,
    );
    let first = validate_staged_candidate(&first_stage, &[]).unwrap();
    let first_bytes = fs::read(fixture.source_root().join(CONSTANT_LIGHT_SOURCE_PATH)).unwrap();
    let (_, first_revision) = store
        .admit_staged_candidate_probe(0, 1, 1, [1; 32], &first)
        .unwrap();
    // This helper reads the selected root and registers its durable pin in
    // one transaction, before another admission can change the selection.
    let pin = store.pin_current_root(1).unwrap();
    assert_eq!((pin.generation, pin.revision), (1, first_revision));
    drop(first);
    drop(first_stage);

    let second_stage =
        constant_light_source_stage(&fixture, 75000, CONSTANT_LIGHT_SOURCE_ID, CONSTANT_LIGHT_ID);
    let second = validate_staged_candidate(&second_stage, &[]).unwrap();
    let second_bytes = fs::read(fixture.source_root().join(CONSTANT_LIGHT_SOURCE_PATH)).unwrap();
    let (_, second_revision) = store
        .admit_staged_candidate_probe(1, 1, 2, [2; 32], &second)
        .unwrap();
    assert_ne!(first_revision, second_revision);
    assert_ne!(first_bytes, second_bytes);
    drop(second);
    drop(second_stage);
    drop(store);

    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
    assert_eq!(store.root().unwrap(), (2, Some(second_revision.to_vec())));
    assert_eq!(
        fs::read(fixture.source_root().join(CONSTANT_LIGHT_SOURCE_PATH)).unwrap(),
        second_bytes
    );
    let pinned_revision: Vec<u8> = store
        .conn
        .query_row(
            "SELECT revision FROM read_pin WHERE id=?1",
            params![pin.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(pinned_revision, pin.revision);

    // Read A by the pinned revision, without the admission-predecessor checks
    // against the now-current root B. No stage or checkout provides these bytes.
    let manifest_bytes = {
        let tx = store
            .conn
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .unwrap();
        let bytes = read_complete_blob(&tx, pin.revision, FIRST_PROFILE_MANIFEST_BYTES).unwrap();
        tx.commit().unwrap();
        bytes
    };
    let manifest = decode_manifest_jcs(&manifest_bytes, FIRST_PROFILE_MANIFEST_BYTES).unwrap();
    assert_eq!(*manifest.revision().as_bytes(), pin.revision);
    assert_eq!(manifest.project_id(), LINEAGE_PROJECT_ID);
    let environment = manifest
        .files()
        .iter()
        .find(|file| {
            file.record_identity() == Some((CONSTANT_LIGHT_SOURCE_ID, "thrum.environment.authored"))
        })
        .unwrap();
    let (environment_length, environment_hash) = environment.blob_identity();
    let environment_bytes = {
        let tx = store
            .conn
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .unwrap();
        let bytes = read_complete_blob(&tx, environment_hash, crate::MAX_RECORD_BYTES).unwrap();
        tx.commit().unwrap();
        bytes
    };
    assert_eq!(environment_bytes.len() as u64, environment_length);
    assert_eq!(environment_bytes, first_bytes);
    let parsed = crate::environment_json::parse_environment(&environment_bytes).unwrap();
    assert_eq!(parsed.id, CONSTANT_LIGHT_SOURCE_ID);
    assert_eq!(parsed.sun.light_ref, CONSTANT_LIGHT_ID);
    assert_eq!(parsed.sun.perpendicular_illuminance_lux, 100000.0);
    assert_eq!(store.root().unwrap(), (2, Some(second_revision.to_vec())));

    // A same-length stored corruption leaves complete metadata and chunk layout
    // intact. The next verified read must return an error, not usable bytes.
    let corrupt_bytes = String::from_utf8(first_bytes)
        .unwrap()
        .replace(
            "\"perpendicular_illuminance_lux\":100000",
            "\"perpendicular_illuminance_lux\":999999",
        )
        .into_bytes();
    assert_eq!(corrupt_bytes.len() as u64, environment_length);
    assert_ne!(corrupt_bytes, environment_bytes);
    assert_eq!(
        store
            .conn
            .execute(
                "UPDATE blob_chunks SET bytes=?1 WHERE hash=?2 AND chunk_index=0",
                params![corrupt_bytes, environment_hash.as_slice()],
            )
            .unwrap(),
        1
    );
    {
        let tx = store
            .conn
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .unwrap();
        assert!(matches!(
            read_complete_blob(&tx, environment_hash, crate::MAX_RECORD_BYTES),
            Err(StoreError::CorruptRootBlob)
        ));
        tx.commit().unwrap();
    }
    store.release_root_pin(pin.id).unwrap();
    let pin_count: i64 = store
        .conn
        .query_row("SELECT count(*) FROM read_pin", [], |row| row.get(0))
        .unwrap();
    assert_eq!(pin_count, 0);
    assert_eq!(store.root().unwrap(), (2, Some(second_revision.to_vec())));
    let tx = store
        .conn
        .transaction_with_behavior(TransactionBehavior::Deferred)
        .unwrap();
    let second_manifest_bytes =
        read_complete_blob(&tx, second_revision, FIRST_PROFILE_MANIFEST_BYTES).unwrap();
    let second_manifest =
        decode_manifest_jcs(&second_manifest_bytes, FIRST_PROFILE_MANIFEST_BYTES).unwrap();
    let second_environment = second_manifest
        .files()
        .iter()
        .find(|file| file.record_identity() == environment.record_identity())
        .unwrap();
    assert_eq!(
        read_complete_blob(
            &tx,
            second_environment.blob_identity().1,
            crate::MAX_RECORD_BYTES,
        )
        .unwrap(),
        second_bytes
    );
    tx.commit().unwrap();
}

#[test]
#[ignore = "local game-schema candidate edit-to-root cost probe"]
fn measure_game_schema_candidate_edit_to_root() {
    const SCENE_ID: &str = "3a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    let fixture = Fixture::new();
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
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
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
    let limits = Limits {
        free_floor: 0,
        ..Limits::V1
    };
    let pages: i64 = store
        .conn
        .query_row("PRAGMA page_count", [], |row| row.get(0))
        .unwrap();
    let accounted_main = fs::metadata(fixture.database())
        .unwrap()
        .len()
        .max(pages as u64 * SQLITE_PAGE_BYTES);
    let mut manifest_bytes = Vec::new();
    manifest.write_jcs(&mut manifest_bytes).unwrap();
    let mut blobs: Vec<_> = manifest
        .files()
        .iter()
        .map(ManifestEntry::blob_identity)
        .collect();
    blobs.push((manifest_bytes.len() as u64, revision));
    blobs.sort_unstable_by_key(|(_, hash)| *hash);
    blobs.dedup_by_key(|(_, hash)| *hash);
    let mut projected_main = 64 * 1024_u64;
    for (length, _) in blobs {
        charge_projected_blob(&mut projected_main, length, None).unwrap();
    }
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
            |_| {},
            |_| {},
        ),
        Err(StoreError::SpacePressure(StoreSpacePressure::MainTarget {
            accounted_current_bytes,
            projected_growth_bytes,
            target_bytes: 4096,
        })) if accounted_current_bytes == accounted_main
            && projected_growth_bytes == projected_main
    ));
    let observed_wal = fs::metadata(wal_path(&fixture.database())).unwrap().len();
    assert!(matches!(
        store.admit_staged_candidate_probe_with_limits(
            0,
            1,
            1,
            [1; 32],
            &candidate,
            Limits { wal: 1, ..limits },
            || {},
            |_| {},
            |_| {},
        ),
        Err(StoreError::SpacePressure(StoreSpacePressure::WalPreflight {
            observed_wal_bytes,
            stop_bytes: 1,
        })) if observed_wal_bytes == observed_wal
    ));
    let headroom = 1_u64 << 60;
    assert!(matches!(
        store.preflight_candidate_space(
            &manifest,
            revision,
            Limits {
                free_floor: headroom,
                ..limits
            },
        ),
        Err(StoreError::SpacePressure(StoreSpacePressure::AvailablePreflight {
            headroom_bytes,
            ..
        })) if headroom_bytes == headroom
    ));
    assert!(matches!(
        store.admit_staged_candidate_probe_with_limits(
            0,
            1,
            1,
            [1; 32],
            &candidate,
            Limits {
                free_floor: headroom,
                ..limits
            },
            || {},
            |_| {},
            |_| {},
        ),
        Err(StoreError::SpacePressure(StoreSpacePressure::AvailableLive {
            headroom_bytes,
            ..
        })) if headroom_bytes == headroom
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
    let accounted_after_admit = fs::metadata(fixture.database())
        .unwrap()
        .len()
        .max(pages as u64 * SQLITE_PAGE_BYTES);
    assert!(matches!(
        store.preflight_candidate_space(
            &manifest,
            revision,
            Limits {
                main: accounted_after_admit + 64 * 1024 - 1,
                ..limits
            },
        ),
        Err(StoreError::SpacePressure(StoreSpacePressure::MainTarget {
            accounted_current_bytes,
            projected_growth_bytes: 65536,
            ..
        })) if accounted_current_bytes == accounted_after_admit
    ));
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
fn sqlite_full_during_candidate_blob_keeps_prior_root_and_retry_recovers() {
    let fixture = Fixture::new();
    let first_stage = minimal_source_stage(&fixture, "before space fault");
    let first = validate_staged_candidate(&first_stage, &[]).unwrap();
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
    let (_, first_revision) = store
        .admit_staged_candidate_probe(0, 1, 1, [1; 32], &first)
        .unwrap();

    let root = fixture.source_root();
    fs::write(root.join("master.bin"), vec![7_u8; MIB as usize]).unwrap();
    let mut declared: Vec<_> = first_stage
        .files()
        .iter()
        .map(|file| file.path().clone())
        .collect();
    declared.push(SourcePath::parse("master.bin").unwrap());
    declared.sort_unstable();
    let second_stage = stage_declared_tree_with(
        &root,
        &declared,
        Limits {
            main: 2 * GIB,
            wal: 4 * MIB,
            temporary: 64 * MIB,
            free_floor: 0,
        },
        |_| {},
    )
    .unwrap();
    let second = validate_staged_candidate(&second_stage, &[]).unwrap();
    let second_revision = *second.provisional_manifest().unwrap().revision().as_bytes();
    assert_ne!(first_revision, second_revision);

    let current_pages: i64 = store
        .conn
        .query_row("PRAGMA page_count", [], |row| row.get(0))
        .unwrap();
    let max_pages: i64 = store
        .conn
        .pragma_query_value(None, "max_page_count", |row| row.get(0))
        .unwrap();
    assert!(max_pages > current_pages);
    store
        .conn
        .pragma_update(None, "max_page_count", current_pages)
        .unwrap();
    assert!(matches!(
        store.admit_staged_candidate_probe(1, 1, 2, [2; 32], &second),
        Err(StoreError::Sql(rusqlite::Error::SqliteFailure(error, _)))
            if error.code == rusqlite::ErrorCode::DiskFull
    ));
    assert_eq!(store.root().unwrap(), (1, Some(first_revision.to_vec())));
    assert!(stored_outcome(&store.conn, 1, 2).unwrap().is_none());
    drop(store);

    let mut reopened = SqliteStoreCore::open(&fixture.database()).unwrap();
    assert_eq!(reopened.root().unwrap(), (1, Some(first_revision.to_vec())));
    assert!(stored_outcome(&reopened.conn, 1, 2).unwrap().is_none());
    assert_eq!(
        reopened
            .admit_staged_candidate_probe(1, 1, 2, [2; 32], &second)
            .unwrap(),
        (2, second_revision)
    );
    assert_eq!(
        reopened.root().unwrap(),
        (2, Some(second_revision.to_vec()))
    );
}

#[test]
fn candidate_space_preflight_reserves_existing_checkpoint_debt() {
    let fixture = Fixture::new();
    let store = SqliteStoreCore::open(&fixture.database()).unwrap();
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
        Err(StoreError::Budget(BudgetError::Overflow))
    ));
}

#[test]
fn candidate_space_preflight_does_not_charge_unused_wal_stop_as_work() {
    let fixture = Fixture::new();
    let store = SqliteStoreCore::open(&fixture.database()).unwrap();
    let stage = minimal_source_stage(&fixture, "tiny marginal source");
    let candidate = validate_staged_candidate(&stage, &[]).unwrap();
    let manifest = candidate.provisional_manifest().unwrap();
    let revision = *manifest.revision().as_bytes();
    for wal in [64 * MIB, 1_u64 << 40] {
        store
            .preflight_candidate_space(
                &manifest,
                revision,
                Limits {
                    main: 2 * GIB,
                    wal,
                    temporary: 64 * MIB,
                    free_floor: 0,
                },
            )
            .unwrap();
    }
}

#[test]
fn accepted_manifest_limit_reports_exact_capacity_without_writing() {
    assert!(ensure_manifest_size(FIRST_PROFILE_MANIFEST_BYTES as u64).is_ok());
    let error: SourceStoreError = ensure_manifest_size(FIRST_PROFILE_MANIFEST_BYTES as u64 + 1)
        .unwrap_err()
        .into();
    assert_eq!(error.kind(), SourceStoreErrorKind::Candidate);
    assert_eq!(
        error.limit_exceeded(),
        Some(LimitExceeded::exact(
            SourceDimension::AcceptedManifestBytes,
            FIRST_PROFILE_MANIFEST_BYTES as u64,
            FIRST_PROFILE_MANIFEST_BYTES as u64 + 1,
        ))
    );
}

#[test]
fn candidate_wal_projection_prices_work_then_caps_reachable_growth() {
    assert_eq!(
        projected_wal_growth(64 * 1024, 64 * MIB).unwrap(),
        projected_wal_growth(64 * 1024, 1_u64 << 40).unwrap()
    );
    assert_eq!(projected_wal_growth(8 * MIB, 4 * MIB).unwrap(), 4 * MIB);
    assert!(matches!(
        projected_wal_growth(u64::MAX, 4 * MIB),
        Err(StoreError::Budget(BudgetError::Overflow))
    ));
}

#[test]
fn accepted_availability_details_preserve_samples_equality_and_overflow() {
    assert!(assess_preflight_available(10, 20, 30, 40, || Ok(100)).is_ok());
    assert!(matches!(
        assess_preflight_available(10, 20, 30, 40, || Ok(99)),
        Err(StoreError::SpacePressure(
            StoreSpacePressure::AvailablePreflight {
                projected_main_bytes: 10,
                projected_wal_bytes: 20,
                checkpoint_debt_bytes: 30,
                observed_available_bytes: 99,
                headroom_bytes: 40,
            }
        ))
    ));
    assert!(matches!(
        assess_preflight_available(u64::MAX, 1, 0, 0, || {
            panic!("overflow must precede I/O")
        }),
        Err(StoreError::Budget(BudgetError::Overflow))
    ));

    assert!(assess_live_available(10, 20, 30, || Ok(60)).is_ok());
    assert!(matches!(
        assess_live_available(10, 20, 30, || Ok(59)),
        Err(StoreError::SpacePressure(
            StoreSpacePressure::AvailableLive {
                incoming_allowance_bytes: 10,
                checkpoint_debt_bytes: 20,
                observed_available_bytes: 59,
                headroom_bytes: 30,
            }
        ))
    ));
    assert!(matches!(
        assess_live_available(u64::MAX, 0, 1, || panic!("overflow must precede I/O")),
        Err(StoreError::Budget(BudgetError::Overflow))
    ));
}

#[test]
fn incomplete_blob_keeps_full_marginal_charge_until_complete() {
    let length = CHUNK_BYTES as u64 + 1;
    let mut missing = 0;
    let mut incomplete = 0;
    let mut complete = 0;
    charge_projected_blob(&mut missing, length, None).unwrap();
    charge_projected_blob(&mut incomplete, length, Some((length as i64, 0))).unwrap();
    charge_projected_blob(&mut complete, length, Some((length as i64, 1))).unwrap();
    assert_eq!(incomplete, missing);
    assert!(missing > 0);
    assert_eq!(complete, 0);
}

#[test]
fn wal_allowance_refuses_before_file_stat_with_its_policy_target() {
    let fixture = Fixture::new();
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
    store.wal_guard.stop_at = MIB;
    assert!(matches!(
        store.ensure_wal_room(MIB),
        Err(StoreError::SpacePressure(
            StoreSpacePressure::WalAllowance {
                incoming_allowance_bytes: MIB,
                stop_bytes: MIB,
            }
        ))
    ));
}

#[test]
fn active_candidate_free_floor_refuses_before_blob_header_write() {
    let fixture = Fixture::new();
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
    let stage = minimal_source_stage(&fixture, "live free floor");
    let candidate = validate_staged_candidate(&stage, &[]).unwrap();
    assert!(matches!(
        store.admit_staged_candidate_probe_with_limits(
            0,
            1,
            1,
            [1; 32],
            &candidate,
            Limits {
                free_floor: 0,
                ..Limits::V1
            },
            || {},
            |store| {
                store.active_free_floor = Some(1 << 60);
            },
            |_| {},
        ),
        Err(StoreError::SpacePressure(
            StoreSpacePressure::AvailableLive { .. }
        ))
    ));
    let headers: i64 = store
        .conn
        .query_row("SELECT count(*) FROM blob_header", [], |row| row.get(0))
        .unwrap();
    assert_eq!(headers, 0);
    assert_eq!(store.root().unwrap(), (0, None));
    assert!(store.active_free_floor.is_none());
    store
        .admit_staged_candidate_probe(0, 1, 1, [1; 32], &candidate)
        .unwrap();
}

#[test]
fn live_free_floor_refuses_before_checkpoint_can_grow_main_file() {
    let fixture = Fixture::new();
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
    let bytes = b"WAL bytes awaiting checkpoint";
    let hash: [u8; 32] = Sha256::digest(bytes).into();
    store
        .stage_blob(&mut bytes.as_slice(), bytes.len() as u64, hash)
        .unwrap();
    let wal_path = fixture.database().with_extension("sqlite-wal");
    let before = fs::metadata(&wal_path).unwrap().len();
    assert!(before > 0);
    store.wal_guard.stop_at = before + MIB;
    store.wal_guard.next_passive_at = 0;
    store.active_free_floor = Some(1 << 60);
    assert!(matches!(
        store.ensure_wal_room(MIB),
        Err(StoreError::SpacePressure(
            StoreSpacePressure::AvailableLive { .. }
        ))
    ));
    assert_eq!(fs::metadata(&wal_path).unwrap().len(), before);
}

#[test]
fn blocked_reader_keeps_wal_checkpoint_debt_visible() {
    let fixture = Fixture::new();
    let mut writer = SqliteStoreCore::open(&fixture.database()).unwrap();
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
    let refused = writer
        .stage_blob(&mut third.as_slice(), 1, third_hash)
        .unwrap_err();
    let observed_after = fs::metadata(&wal_path).unwrap().len();
    assert!(matches!(
        refused,
        StoreError::SpacePressure(StoreSpacePressure::WalCheckpoint {
            observed_wal_bytes,
            incoming_allowance_bytes: MIB,
            stop_bytes,
            checkpoint_busy: true,
        }) if observed_wal_bytes == observed_after
            && stop_bytes == before + MIB - 1
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
        Err(StoreError::TokenConflict)
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

#[test]
fn sustained_blocked_reader_defers_chunk_staging_and_resumes_after_drain() {
    let fixture = Fixture::new();
    let mut writer = SqliteStoreCore::open(&fixture.database()).unwrap();
    writer
        .conn
        .execute_batch("PRAGMA wal_autocheckpoint=0")
        .unwrap();
    writer
        .conn
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
        })
        .unwrap();
    let reader = Connection::open(fixture.database()).unwrap();
    reader.execute_batch("BEGIN").unwrap();
    let _: i64 = reader
        .query_row("SELECT generation FROM root WHERE id=1", [], |row| {
            row.get(0)
        })
        .unwrap();
    writer.wal_guard = WalGuard {
        passive_at: 8 * MIB,
        next_passive_at: 8 * MIB,
        stop_at: 40 * MIB,
    };
    let started = Instant::now();
    let mut completed = 0;
    let mut deferred = None;
    for index in 1..=16_u8 {
        let bytes = vec![index; CHUNK_BYTES];
        let hash: [u8; 32] = Sha256::digest(&bytes).into();
        match writer.stage_blob(&mut bytes.as_slice(), bytes.len() as u64, hash) {
            Ok(()) => completed += 1,
            Err(StoreError::SpacePressure(StoreSpacePressure::WalCheckpoint { .. })) => {
                deferred = Some((bytes, hash));
                break;
            }
            Err(error) => panic!("unexpected staging error: {error}"),
        }
    }
    assert!(completed >= 2);
    let (deferred_bytes, deferred_hash) = deferred.expect("WAL stop must defer a chunk");
    let wal_bytes = fs::metadata(fixture.database().with_extension("sqlite-wal"))
        .unwrap()
        .len();
    eprintln!(
        "blocked_reader_chunks completed={completed} wal_bytes={wal_bytes} elapsed_ms={:.3}",
        started.elapsed().as_secs_f64() * 1000.0
    );
    assert!(writer.wal_guard.next_passive_at > 40 * MIB);
    let passive: (i64, i64, i64) = writer
        .conn
        .query_row("PRAGMA wal_checkpoint(PASSIVE)", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .unwrap();
    assert!(passive.1 > passive.2);
    assert_eq!(writer.root().unwrap(), (0, None));
    let blocked: (i64, i64, i64) = writer
        .conn
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .unwrap();
    assert_eq!(blocked.0, 1);
    reader.execute_batch("COMMIT").unwrap();
    writer
        .stage_blob(
            &mut deferred_bytes.as_slice(),
            deferred_bytes.len() as u64,
            deferred_hash,
        )
        .unwrap();
    let complete: bool = writer
        .conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM blob_header WHERE hash=?1 AND state=1)",
            params![deferred_hash.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    assert!(complete);
}

#[cfg(test)]
struct KillChildOnDrop(Child);

#[cfg(test)]
impl Drop for KillChildOnDrop {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[cfg(test)]
fn signal_and_wait_for_kill(marker: &Path) -> ! {
    fs::write(marker, b"SQLite writes reached kill point").unwrap();
    loop {
        std::thread::sleep(Duration::from_secs(1));
    }
}

#[cfg(test)]
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
    let mut store = SqliteStoreCore::open(&database).unwrap();
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
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
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
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
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
        Err(StoreError::MissingCompleteBlob)
    ));
    store
        .conn
        .execute_batch(
            "CREATE TRIGGER no_partial_delete BEFORE DELETE ON blob_chunks
             BEGIN SELECT RAISE(ABORT, 'killed partial chunk must be reused'); END;",
        )
        .unwrap();
    store
        .stage_blob(&mut bytes.as_slice(), bytes.len() as u64, hash)
        .unwrap();
    assert_eq!(store.commit_probe(1, 1, 2, [2; 32], hash).unwrap(), 2);
    drop(store);
    let store = SqliteStoreCore::open(&fixture.database()).unwrap();
    assert_eq!(store.root().unwrap(), (2, Some(hash.to_vec())));
}

#[test]
fn process_kill_before_root_commit_child() {
    let Some(database) = std::env::var_os("THRUM_TEST_ROOT_TXN_DB") else {
        return;
    };
    let database = PathBuf::from(database);
    let marker = PathBuf::from(std::env::var_os("THRUM_TEST_ROOT_TXN_MARKER").unwrap());
    let mut store = SqliteStoreCore::open(&database).unwrap();
    let revision: [u8; 32] = Sha256::digest(b"complete new root blob").into();
    store
        .commit_with_closure_after_sql(
            1,
            1,
            2,
            [2; 32],
            revision,
            |_| Ok(()),
            None,
            || {
                signal_and_wait_for_kill(&marker);
            },
        )
        .unwrap();
}

#[test]
fn process_kill_inside_root_transaction_rolls_back_root_and_outcome() {
    let fixture = Fixture::new();
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
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

    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
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
    let store = SqliteStoreCore::open(&fixture.database()).unwrap();
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
    let mut first_store = SqliteStoreCore::open(&fixture.database()).unwrap();
    let base_stage = minimal_source_stage(&fixture, "base scene");
    let base = validate_staged_candidate(&base_stage, &[]).unwrap();
    first_store
        .admit_staged_candidate_probe(0, 1, 1, [1; 32], &base)
        .unwrap();

    let next_stage = minimal_source_stage(&fixture, "next scene");
    let next = validate_staged_candidate(&next_stage, &[]).unwrap();
    let mut winner = SqliteStoreCore::open(&fixture.database()).unwrap();
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
        Err(StoreError::TokenConflict)
    ));
}

#[test]
fn assembled_candidate_refuses_to_drop_current_root_history() {
    let fixture = Fixture::new();
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
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
        Err(StoreError::Lineage(_))
    ));
    assert_eq!(store.root().unwrap(), (1, Some(prior_revision.to_vec())));
}

#[test]
#[ignore = "local small candidate edit-to-root cost probe"]
fn measure_small_candidate_edit_to_root() {
    let fixture = Fixture::new();
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
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
    let mut first = SqliteStoreCore::open(&fixture.database()).unwrap();
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

    let mut first = SqliteStoreCore::open(&fixture.database()).unwrap();
    assert!(
        first
            .compare_history_with_current_root(1, LINEAGE_PROJECT_ID, &candidate)
            .is_ok()
    );
    assert!(matches!(
        first.compare_history_with_current_root(1, LINEAGE_PROJECT_ID, &[]),
        Err(StoreError::History(_))
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
        Err(StoreError::DifferentProject)
    ));
    let mut second = SqliteStoreCore::open(&fixture.database()).unwrap();
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
        Err(StoreError::StaleGeneration)
    ));
    assert!(matches!(
        first.compare_history_with_current_root(1, LINEAGE_PROJECT_ID, &candidate),
        Err(StoreError::StaleGeneration)
    ));
    assert!(matches!(
        first.compare_history_with_current_root(2, LINEAGE_PROJECT_ID, &base),
        Err(StoreError::Lineage(_))
    ));
}

#[test]
fn current_root_history_refuses_corrupt_stored_bytes() {
    let fixture = Fixture::new();
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
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
        Err(StoreError::CorruptRootBlob)
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
        Err(StoreError::CorruptRootBlob)
    ));
    store
        .conn
        .execute("UPDATE root SET revision=zeroblob(4194305) WHERE id=1", [])
        .unwrap();
    assert!(matches!(
        store.compare_history_with_current_root(1, LINEAGE_PROJECT_ID, &base),
        Err(StoreError::CorruptRootBlob)
    ));
}

#[test]
#[ignore = "local maximum-manifest current-root history resolution cost probe"]
fn measure_maximum_root_history_resolution() {
    let fixture = Fixture::new();
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
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
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
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
#[ignore = "local maximum-profile phased manifest and pinned-reader probe"]
fn measure_maximum_phased_manifest_reader_progress() {
    const PROJECT_ID: &str = "2a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    let fixture = Fixture::new();
    let plan = SourceSpacePlan {
        free_headroom_bytes: 0,
        ..SourceSpacePlan::DEFAULT
    };
    let store = NativeSourceStore::open(&fixture.database(), plan).unwrap();
    let stage_a = minimal_source_stage(&fixture, "before maximum manifest");
    let candidate_a = validate_staged_candidate(&stage_a, &[]).unwrap();
    let a = store.admit(0, 1, 1, [1; 32], &candidate_a).unwrap();
    let mut old_pin = store.pin_current(a.generation).unwrap();
    let scene = SourcePath::parse("scenes/main.scene.json").unwrap();
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
    let done = std::sync::atomic::AtomicBool::new(false);
    let (mut samples, writer_ms) = std::thread::scope(|scope| {
        let writer = scope.spawn(|| {
            let mut scratch = store.owner.admission().unwrap();
            let start = Instant::now();
            assert_eq!(
                store
                    .stage_manifest_phased(&manifest, &mut scratch)
                    .unwrap(),
                revision
            );
            done.store(true, Ordering::Release);
            start.elapsed().as_secs_f64() * 1000.0
        });
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match store.owner.admission_scratch.try_lock() {
                Err(std::sync::TryLockError::WouldBlock) => break,
                Ok(guard) => drop(guard),
                Err(std::sync::TryLockError::Poisoned(_)) => panic!("admission owner poisoned"),
            }
            assert!(Instant::now() < deadline, "writer did not start staging");
            std::thread::yield_now();
        }
        let mut samples = Vec::new();
        while !done.load(Ordering::Acquire) && samples.len() < 1000 {
            let start = Instant::now();
            old_pin.copy_file_to(&scene, &mut io::sink()).unwrap();
            samples.push(start.elapsed().as_secs_f64() * 1000.0);
            std::thread::sleep(Duration::from_millis(1));
        }
        (samples, writer.join().unwrap())
    });
    assert!(!samples.is_empty());
    old_pin.close().unwrap();
    assert_eq!(store.current_root().unwrap(), Some(a));
    let complete: i64 = store
        .owner
        .lock()
        .unwrap()
        .conn
        .query_row(
            "SELECT chunk_count FROM blob_header WHERE hash=?1 AND state=1",
            params![revision.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(complete, 4);
    samples.sort_by(f64::total_cmp);
    let p95 = samples[(samples.len() * 95 / 100).min(samples.len() - 1)];
    eprintln!(
        "maximum_phased_manifest_reader files=100000 samples={} min_ms={:.3} p95_ms={p95:.3} max_ms={:.3} writer_ms={writer_ms:.3}",
        samples.len(),
        samples[0],
        samples[samples.len() - 1]
    );
}

#[test]
#[ignore = "local maximum-profile final reference-closure contention probe"]
fn measure_maximum_manifest_closure_reader_wait() {
    const PROJECT_ID: &str = "2a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    let distinct = if std::env::var_os("THRUM_TEST_UNIQUE_MANIFEST_CLOSURE").is_some() {
        100_000
    } else {
        256
    };
    let fixture = Fixture::new();
    let plan = SourceSpacePlan {
        free_headroom_bytes: 0,
        ..SourceSpacePlan::DEFAULT
    };
    let store = NativeSourceStore::open(&fixture.database(), plan).unwrap();
    let stage_a = minimal_source_stage(&fixture, "before maximum closure");
    let candidate_a = validate_staged_candidate(&stage_a, &[]).unwrap();
    let a = store.admit(0, 1, 1, [1; 32], &candidate_a).unwrap();
    let mut old_pin = store.pin_current(a.generation).unwrap();
    let blobs: Vec<_> = (0..distinct)
        .map(|index| {
            let bytes = if distinct == 256 {
                vec![index as u8]
            } else {
                (index as u32).to_le_bytes().to_vec()
            };
            (Sha256::digest(&bytes).into(), bytes)
        })
        .collect();
    for group in blobs.chunks(SMALL_BATCH_FILES) {
        store
            .owner
            .lock()
            .unwrap()
            .stage_small_blob_batch(group.iter().map(|(hash, bytes)| (*hash, bytes.as_slice())))
            .unwrap();
    }
    let files = (0..100_000)
        .map(|index| {
            ManifestEntry::new(
                SourcePath::parse(&format!("src/file-{index:06}.bin")).unwrap(),
                blobs[index % distinct].1.len() as u64,
                blobs[index % distinct].0,
                None,
            )
            .unwrap()
        })
        .collect();
    let manifest = TreeManifest::new(PROJECT_ID, Vec::new(), files).unwrap();
    let revision = *manifest.revision().as_bytes();
    let mut scratch = store.owner.admission().unwrap();
    store
        .stage_manifest_phased(&manifest, &mut scratch)
        .unwrap();
    drop(scratch);
    let batched = std::env::var_os("THRUM_TEST_BATCHED_MANIFEST_CLOSURE").is_some();
    let prepare_start = Instant::now();
    let prepared = batched.then(|| prepare_manifest_closure(&manifest).unwrap());
    let prepare_ms = prepare_start.elapsed().as_secs_f64() * 1000.0;
    let scene = SourcePath::parse("scenes/main.scene.json").unwrap();
    let (go_tx, go_rx) = std::sync::mpsc::channel();
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let (reader_ms, commit_ms, generation) = std::thread::scope(|scope| {
        let old_pin_ref = &mut old_pin;
        let reader = scope.spawn(move || {
            go_rx.recv().unwrap();
            let start = Instant::now();
            started_tx.send(()).unwrap();
            old_pin_ref.copy_file_to(&scene, &mut io::sink()).unwrap();
            start.elapsed().as_secs_f64() * 1000.0
        });
        let mut inner = store.owner.lock().unwrap();
        go_tx.send(()).unwrap();
        started_rx.recv().unwrap();
        let start = Instant::now();
        let generation = match prepared.as_deref() {
            Some(refs) => inner
                .commit_prepared_manifest_probe(
                    a.generation,
                    1,
                    2,
                    [2; 32],
                    revision,
                    refs,
                    Some(&store.owner.read_gate),
                )
                .unwrap(),
            None => inner
                .commit_verified_manifest_probe(
                    a.generation,
                    1,
                    2,
                    [2; 32],
                    revision,
                    &manifest,
                    Some(&store.owner.read_gate),
                )
                .unwrap(),
        };
        let commit_ms = start.elapsed().as_secs_f64() * 1000.0;
        drop(inner);
        (reader.join().unwrap(), commit_ms, generation)
    });
    old_pin.close().unwrap();
    assert_eq!(generation, a.generation + 1);
    assert_eq!(
        store.current_root().unwrap().unwrap().generation,
        generation
    );
    eprintln!(
        "maximum_manifest_closure files=100000 distinct={distinct} batched={batched} prepare_ms={prepare_ms:.3} reader_wait_copy_ms={reader_ms:.3} commit_ms={commit_ms:.3}"
    );
}

#[test]
fn prepared_closure_probe_checks_complete_bytes_across_batch_boundaries() {
    let fixture = Fixture::new();
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
    let blobs: Vec<_> = (0_u32..129)
        .map(|value| {
            let bytes = value.to_le_bytes();
            (Sha256::digest(bytes).into(), bytes)
        })
        .collect();
    for group in blobs.chunks(SMALL_BATCH_FILES) {
        store
            .stage_small_blob_batch(group.iter().map(|(hash, bytes)| (*hash, bytes.as_slice())))
            .unwrap();
    }
    let refs: Vec<ClosureRef> = blobs.iter().map(|(hash, _)| (*hash, 4)).collect();
    {
        let tx = store.conn.unchecked_transaction().unwrap();
        check_prepared_manifest_closure(&tx, &[]).unwrap();
        check_prepared_manifest_closure(&tx, &refs[..128]).unwrap();
        check_prepared_manifest_closure(&tx, &refs).unwrap();
        let mut missing = refs.clone();
        missing[128].0 = [0; 32];
        assert!(matches!(
            check_prepared_manifest_closure(&tx, &missing),
            Err(StoreError::MissingCompleteFile)
        ));
        let mut wrong_length = refs.clone();
        wrong_length[128].1 = 5;
        assert!(matches!(
            check_prepared_manifest_closure(&tx, &wrong_length),
            Err(StoreError::MissingCompleteFile)
        ));
        tx.commit().unwrap();
    }
    store
        .conn
        .execute(
            "UPDATE blob_header SET state=0 WHERE hash=?1",
            params![refs[128].0.as_slice()],
        )
        .unwrap();
    let tx = store.conn.unchecked_transaction().unwrap();
    assert!(matches!(
        check_prepared_manifest_closure(&tx, &refs),
        Err(StoreError::MissingCompleteFile)
    ));
}

#[test]
#[ignore = "local small-file SQLite transaction amplification probe"]
fn measure_small_blob_transaction_cost() {
    let fixture = Fixture::new();
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
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
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
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
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
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
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
    store
        .conn
        .execute_batch("PRAGMA wal_autocheckpoint=0;")
        .unwrap();
    let snapshot = |label: &str, store: &SqliteStoreCore| {
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
    let mut batched = SqliteStoreCore::open(&batch_fixture.database()).unwrap();
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
    let mut individual = SqliteStoreCore::open(&individual_fixture.database()).unwrap();
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
    let mut store = SqliteStoreCore::open(&fixture.database()).unwrap();
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
