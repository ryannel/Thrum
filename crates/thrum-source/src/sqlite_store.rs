//! Append-only native SQLite source store and its transaction proofs.
//!
//! The first profile retains all verified blobs. Collection and physical disk
//! limits require separate qualification; planning targets are advisory.

use std::fmt;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, RwLock};

use fs4::{FileExt, TryLockError};
use rusqlite::types::ValueRef;
use rusqlite::{
    Connection, OpenFlags, OptionalExtension, Transaction, TransactionBehavior, config::DbConfig,
    params,
};

use sha2::{Digest, Sha256};

use crate::budget::{BudgetError, Limits, SourceSpacePlan, open_regular_lock};
use crate::history_json::{
    CarriedHistoryError, HistoryError, compare_carried_histories, parse_registry_history,
};
use crate::manifest_json::{ManifestDecodeError, decode_manifest_jcs};
use crate::registry_stage::{StagedCandidate, StagedRegistryError};
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

const SQLITE_PAGE_BYTES: u64 = 4 * 1024;
const SOURCE_STORE_FORMAT: i64 = 1;
const MIB: u64 = 1024 * 1024;

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

trait AdmissionFault {
    fn after_committed_chunk(&mut self) {}
    fn before_final_commit(&mut self) {}
    fn after_final_sql(&mut self) {}
}

struct NoAdmissionFault;
impl AdmissionFault for NoAdmissionFault {}

struct SqliteStoreCore {
    conn: Connection,
    recovery_required: bool,
    path: PathBuf,
    wal_guard: WalGuard,
    pending_pin_releases: Vec<i64>,
    active_free_floor: Option<u64>,
}

#[expect(
    clippy::struct_field_names,
    reason = "all thresholds are positions in the same WAL byte stream"
)]
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
        let count = usize::try_from(remaining)
            .unwrap_or(buffer.len())
            .min(buffer.len());
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
        if actual != i64::try_from(expected).map_err(|_| StoreError::CorruptRootBlob)? {
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

mod blob;
mod root_pin;
mod schema;
mod space;

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
#[expect(
    clippy::struct_field_names,
    reason = "all public observation fields include byte units to prevent unit confusion"
)]
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
    // Immutable operational identity shared with every pin from this owner.
    store_path: PathBuf,
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
    /// Canonical database path selected when this source owner opened.
    ///
    /// This is an operational store binding for a cooperating host, not part
    /// of the accepted source revision. It does not revalidate the path after
    /// an external rename or replacement.
    pub fn store_path(&self) -> &Path {
        &self.owner.store_path
    }

    /// Open or create one native SQLite source store with a planning policy.
    ///
    /// A cooperating owner holds an exclusive sidecar lock until this handle
    /// and every accepted-source lease have closed. Opening the same store from
    /// another process or handle returns `Busy` before SQLite recovery runs.
    ///
    /// # Errors
    ///
    /// The path, owner lock, SQLite schema, or selected space plan cannot be admitted.
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
                store_path: database,
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
    ///
    /// # Errors
    ///
    /// The owner is unavailable, SQLite observation fails, or size arithmetic overflows.
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
    ///
    /// # Errors
    ///
    /// The selected root or its durable outcome is missing or corrupt.
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
    ///
    /// # Errors
    ///
    /// The owner is unavailable, the token conflicts, or SQLite cannot read its stored outcome.
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
    ///
    /// # Errors
    ///
    /// A staged input, space preflight, blob verification, lineage check, or atomic root transaction fails.
    pub fn admit(
        &self,
        expected_generation: i64,
        epoch: i64,
        key: i64,
        request_hash: [u8; 32],
        candidate: &StagedCandidate<'_>,
    ) -> Result<AcceptedSourceRoot, SourceStoreError> {
        self.admit_with_fault(
            expected_generation,
            epoch,
            key,
            request_hash,
            candidate,
            &mut NoAdmissionFault,
        )
    }

    fn admit_with_fault(
        &self,
        expected_generation: i64,
        epoch: i64,
        key: i64,
        request_hash: [u8; 32],
        candidate: &StagedCandidate<'_>,
        fault: &mut impl AdmissionFault,
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
            self.stage_candidate_files_phased(candidate, &mut scratch, fault)?;
            let stored_revision = self.stage_manifest_phased(&manifest, &mut scratch)?;
            if stored_revision != revision {
                return Err(StoreError::HashMismatch);
            }
            fault.before_final_commit();
            let generation = {
                let mut inner = self.owner.lock()?;
                inner.require_clean()?;
                inner.commit_verified_manifest(
                    expected_generation,
                    epoch,
                    key,
                    request_hash,
                    revision,
                    &manifest,
                    Some(&self.owner.read_gate),
                    fault,
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
        fault: &mut impl AdmissionFault,
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
                self.stage_blob_phased(&mut reader, file.length(), *file.sha256(), scratch, fault)?;
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
        fault: &mut impl AdmissionFault,
    ) -> Result<(), StoreError> {
        let chunk_count = blob_chunk_count(length)?;
        scratch.resize(
            usize::try_from(length.min(CHUNK_BYTES as u64))
                .map_err(|_| StoreError::InputLength)?
                .max(1),
            0,
        );
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
            fault.after_committed_chunk();
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
    ///
    /// # Errors
    ///
    /// The expected generation is stale, the root cannot be authenticated, or a read pin cannot be acquired.
    pub fn pin_current(&self, expected_generation: i64) -> Result<PinnedSource, SourceStoreError> {
        let _admission = self.owner.admission()?;
        let pin = {
            let mut inner = self.owner.lock()?;
            inner.require_clean()?;
            inner.retry_pending_pin_releases()?;
            inner.pin_current_root(expected_generation)?
        };
        self.load_pin(&pin)
    }

    /// Pin an earlier accepted root for a retained build, match or save.
    /// The supplied generation and revision must have a durable outcome.
    ///
    /// # Errors
    ///
    /// The accepted root has no matching durable outcome or a read pin cannot be acquired.
    pub fn pin_accepted(&self, root: AcceptedSourceRoot) -> Result<PinnedSource, SourceStoreError> {
        let _admission = self.owner.admission()?;
        let pin = {
            let mut inner = self.owner.lock()?;
            inner.require_clean()?;
            inner.retry_pending_pin_releases()?;
            inner.pin_accepted_root(root)?
        };
        self.load_pin(&pin)
    }

    fn load_pin(&self, pin: &RootReadPin) -> Result<PinnedSource, SourceStoreError> {
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
                    drop(inner.release_root_pin(pin.id));
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
    /// Canonical database path of the owner that issued this pin.
    ///
    /// The path remains available while the pin keeps that owner alive. A
    /// cooperating host can compare it with its selected project store before
    /// creating build inputs; source bytes and revision do not encode it.
    pub fn store_path(&self) -> &Path {
        &self.owner.store_path
    }

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
    ///
    /// # Errors
    ///
    /// The requested file is absent, stored chunks fail verification, or the output writer fails.
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
            if stored_length != i64::try_from(length).map_err(|_| StoreError::CorruptRootBlob)?
                || stored_chunks != chunks
                || state != 1
            {
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
                    let expected = usize::try_from((length - copied).min(CHUNK_BYTES as u64))
                        .map_err(|_| StoreError::InvalidChunkLayout)?;
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
    ///
    /// # Errors
    ///
    /// The read-pin release cannot be committed.
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
            drop(inner.release_root_pin(id));
        }
    }
}

#[cfg(test)]
mod tests;
