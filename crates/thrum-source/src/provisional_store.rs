//! Provisional native snapshot storage, **not accepted-source authority**.
//!
//! Exact bytes, keyed outcomes and reader pins survive SQLite commits/reopen.
//! Physical peak reservation, application recovery, outcome retirement and
//! abandoned-owner recovery are not implemented. These types cannot certify a
//! ready build. Destructive collection is disabled; complete blobs and failed
//! pin releases remain retained until a separately designed recovery policy.
//!
//! The deliberately small profile permits 4096 files, 16 MiB/file, 64 MiB/tree,
//! a 4 MiB manifest, 1024 publication intents/outcomes and 1024 pins. SQLite main is capped at
//! 256 MiB; a 64 MiB WAL check is backpressure, not a certified physical bound.
//! One advisory owner and one connection serialize cooperating store access.
//! Automatic and last-close checkpoints are disabled on that connection.
//! At 32 MiB retained WAL, checked PASSIVE/TRUNCATE maintenance runs before
//! further writes. Its distinct-page data allowance is conditional on the
//! pinned SQLite implementation and valid, serialized state; it is not an
//! APFS allocation bound, physical reservation or the Foundation 8 GiB floor.
//! Blocked/failed maintenance retains WAL/debt and refuses the next write.
//! Maintenance never downgrades an already committed keyed publication.
//! A final COMMIT error quarantines the connection, including lease cleanup,
//! until every owner drops and a fresh open resolves the exact keyed result.
//! No SQL read transaction spans consumer work. Leases verify manifest/complete
//! metadata closure on acquisition and hash each record before returning bytes;
//! acquisition does not rehash every master in the tree.
//! Blob sizes are checked from SQLite metadata before selecting payloads; these
//! input bounds are not a whole-process SQLite allocation/RSS guarantee.
//! Source bytes are schema-validated by the supplied F3 candidate; this store
//! does not pin an SDK/schema interpretation or implement the RecoveryRoot
//! protocol. Its selection and retry rows are deliberately provisional.
//! Format 2 records a pending publication intent before staging new blobs and
//! commits selection, keyed outcome and intent completion together. Reopen does
//! not select pending work or apply/repair working files. The recovery-status
//! query distinguishes missing, pending and exactly committed requests. This
//! is not the Foundation RecoveryRoot/journal/save protocol; format 1 refuses.
//! Files at most 4 KiB share bounded transactions (128 files, 1 MiB input).
//! The two tighter limits currently imply at most 512 KiB of packed payload;
//! larger files retain the separate bounded-chunk path.
//! Open audits the retained outcome generations once (at most 1025 rows);
//! live selection verification uses one indexed high-water lookup. This detects
//! a rolled-back selection row while later outcomes remain, not restoration of
//! a coherent older database image. Selection tokens are encoding checksums,
//! not accepted-source certificates or cryptographic authentication.

use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use fs4::{FileExt, TryLockError};
use rusqlite::{
    Connection, OpenFlags, OptionalExtension, TransactionBehavior, config::DbConfig, params,
};
use sha2::{Digest, Sha256};

use crate::history_json::{compare_carried_histories, parse_registry_history};
use crate::manifest_json::decode_manifest_jcs;
use crate::record::canonical_uuid;
use crate::registry_stage::StagedCandidate;
use crate::{MAX_RECORD_BYTES, TreeManifest};

const CHUNK: usize = 4 * 1024 * 1024;
const MANIFEST_LIMIT: usize = 4 * 1024 * 1024;
const TREE_LIMIT: u64 = 64 * 1024 * 1024;
const WAL_LIMIT: u64 = 64 * 1024 * 1024;
const PAGE_BYTES: u64 = 4096;
const MAX_MAIN_PAGES: u64 = 65536;
const CHECKPOINT_SQLITE_SOURCE: &str =
    "2026-06-03 19:12:13 d6e03d8c777cfa2d35e3b60d8ec3e0187f3e9f99d8e2ee9cac695fd6fcdf1a24";
const NATIVE_VFS: &str = if cfg!(windows) { "win32" } else { "unix" };
const MAX_FILES: usize = 4096;
const MAX_OWNERS: i64 = 1024;
const APP_ID: i64 = 0x54485031;
const TINY_FILE_BYTES: usize = 4 * 1024;
const TINY_BATCH_FILES: usize = 128;
const TINY_BATCH_BYTES: usize = 1024 * 1024;
const SELECTION_TOKEN_MAGIC: &[u8] = b"thrum-provisional-selection-token-v2\0";
const SELECTION_TOKEN_PAYLOAD: usize = 16 + 36 + 8 + 1 + 32;
const SELECTION_TOKEN_BYTES: usize = SELECTION_TOKEN_MAGIC.len() + SELECTION_TOKEN_PAYLOAD + 32;
const COMMITTED_HEAD_SQL: &str = "SELECT CASE WHEN typeof(generation)='integer' THEN generation ELSE NULL END FROM outcome INDEXED BY outcome_generation ORDER BY generation DESC LIMIT 1";

#[cfg(test)]
#[path = "provisional_store_vfs_test.rs"]
mod checkpoint_vfs;

/// A serialized, conditional data-write plan. WAL remains retained throughout
/// execution; neither frame progress nor this plan reserves physical space.
#[derive(Debug)]
struct CheckpointPlan {
    wal_bytes: u64,
    main_bytes: u64,
    pages: u64,
    frames: u64,
    backfilled: u64,
    data_bytes: u64,
}

#[derive(Clone, Copy)]
enum CheckpointMode {
    Passive,
    Truncate,
}

#[derive(Debug)]
struct CheckpointProgress {
    busy: bool,
    frames: u64,
    backfilled: u64,
    retained_wal_bytes: u64,
}

fn check_provisional_free(available: u64) -> Result<(), StoreError> {
    if available < TREE_LIMIT {
        return Err(StoreError::Refused(
            "provisional free-space backpressure; no physical reservation",
        ));
    }
    Ok(())
}

fn configure_checkpoint_policy(conn: &Connection) -> Result<(), StoreError> {
    // Configure immediately after open, before format reads or writes. Even a
    // later format refusal must not implicitly checkpoint on Connection drop.
    if !conn.set_db_config(DbConfig::SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE, true)?
        || !conn.db_config(DbConfig::SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE)?
    {
        return Err(StoreError::Refused("checkpoint-on-close unavailable"));
    }
    conn.execute_batch("PRAGMA wal_autocheckpoint=0; PRAGMA mmap_size=0;")?;
    let auto: i64 = conn.query_row("PRAGMA wal_autocheckpoint", [], |r| r.get(0))?;
    let mmap: i64 = conn.query_row("PRAGMA mmap_size", [], |r| r.get(0))?;
    let source: String = conn.query_row("SELECT sqlite_source_id()", [], |r| r.get(0))?;
    if auto != 0 || mmap != 0 || source != CHECKPOINT_SQLITE_SOURCE {
        return Err(StoreError::Refused(
            "unreviewed checkpoint configuration/SQLite",
        ));
    }
    Ok(())
}

/// Failure of provisional storage; none implies accepted-source publication.
#[derive(Debug)]
pub enum StoreError {
    /// SQLite refused an operation.
    Sql(rusqlite::Error),
    /// Filesystem access failed.
    Io(io::Error),
    /// A finite profile, identity or publication precondition refused.
    Refused(&'static str),
    /// Stored bytes or metadata failed verification.
    Corrupt(&'static str),
    /// A cooperating owner or pending SQLite writer holds the store.
    Busy,
    /// An earlier panic poisoned the connection owner.
    Poisoned,
    /// A final COMMIT failed. Drop every store/lease owner and reopen before
    /// querying the exact keyed result; no further SQL runs on this owner.
    RecoveryRequired,
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sql(e) => write!(f, "provisional SQLite: {e}"),
            Self::Io(e) => write!(f, "provisional filesystem: {e}"),
            Self::Refused(e) => write!(f, "provisional source refused: {e}"),
            Self::Corrupt(e) => write!(f, "provisional source corrupt: {e}"),
            Self::Busy => f.write_str("provisional store busy"),
            Self::Poisoned => f.write_str("provisional store owner poisoned"),
            Self::RecoveryRequired => {
                f.write_str("provisional COMMIT outcome uncertain; drop all owners and reopen")
            }
        }
    }
}
impl std::error::Error for StoreError {}
impl From<rusqlite::Error> for StoreError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sql(value)
    }
}
impl From<io::Error> for StoreError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

/// Exact selection in one provisional store. Constructed only by that store.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProvisionalSelection {
    store_id: [u8; 16],
    project_id: String,
    generation: u64,
    revision: Option<[u8; 32]>,
}
/// Recovery state of one exact provisional publication request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PublicationStatus {
    /// No intent or outcome exists for this key/request.
    Missing,
    /// The target is durably recorded but not selected. Retry requires the same
    /// validated candidate and a still-selected base; no automatic rebase occurs.
    PendingNeedsRetry,
    /// The exact target committed. This may be a historical selection and does
    /// not acquire a reader lease or rewind the current selection.
    Committed(ProvisionalSelection),
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum PublicationPoint {
    IntentRecorded = 1,
    BeforeFinal = 2,
    FinalWritten = 3,
    FinalCommitted = 4,
}
impl ProvisionalSelection {
    /// Encode a fixed-size, versioned request-base token for restart recovery.
    /// Persist this before publication together with its key and target revision.
    /// The checksum detects encoding damage; it is not authentication or a lease.
    pub fn to_token_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(SELECTION_TOKEN_BYTES);
        out.extend_from_slice(SELECTION_TOKEN_MAGIC);
        out.extend_from_slice(&self.store_id);
        out.extend_from_slice(self.project_id.as_bytes());
        out.extend_from_slice(&self.generation.to_be_bytes());
        out.push(u8::from(self.revision.is_some()));
        out.extend_from_slice(&self.revision.unwrap_or([0; 32]));
        let checksum = Sha256::digest(&out);
        out.extend_from_slice(&checksum);
        out
    }
    /// Decode only the exact canonical token encoding. The store still checks
    /// store/project identity and exact intent/outcome on a recovery query;
    /// decoding cannot authorize publication or acquire a reader lease.
    pub fn from_token_bytes(bytes: &[u8]) -> Result<Self, StoreError> {
        if bytes.len() != SELECTION_TOKEN_BYTES || !bytes.starts_with(SELECTION_TOKEN_MAGIC) {
            return Err(StoreError::Refused("selection token format"));
        }
        let payload_end = bytes.len() - 32;
        if Sha256::digest(&bytes[..payload_end]).as_slice() != &bytes[payload_end..] {
            return Err(StoreError::Refused("selection token checksum"));
        }
        let bytes = &bytes[SELECTION_TOKEN_MAGIC.len()..payload_end];
        let store_id = bytes[..16]
            .try_into()
            .map_err(|_| StoreError::Refused("selection token format"))?;
        let project_id = std::str::from_utf8(&bytes[16..52])
            .map_err(|_| StoreError::Refused("selection token project"))?;
        if !canonical_uuid(project_id) {
            return Err(StoreError::Refused("selection token project"));
        }
        let generation = u64::from_be_bytes(
            bytes[52..60]
                .try_into()
                .map_err(|_| StoreError::Refused("selection token generation"))?,
        );
        if generation > i64::MAX as u64 {
            return Err(StoreError::Refused("selection token generation"));
        }
        let revision = match bytes[60] {
            0 if bytes[61..] == [0; 32] => None,
            1 => Some(
                bytes[61..]
                    .try_into()
                    .map_err(|_| StoreError::Refused("selection token revision"))?,
            ),
            _ => return Err(StoreError::Refused("selection token revision")),
        };
        if (generation == 0) != revision.is_none() {
            return Err(StoreError::Refused("selection token selection invariant"));
        }
        Ok(Self {
            store_id,
            project_id: project_id.to_owned(),
            generation,
            revision,
        })
    }
    /// Durable project identity.
    pub fn project_id(&self) -> &str {
        &self.project_id
    }
    /// Monotonic provisional publication generation.
    pub fn generation(&self) -> u64 {
        self.generation
    }
    /// Exact canonical manifest digest, absent before first publication.
    pub fn revision(&self) -> Option<&[u8; 32]> {
        self.revision.as_ref()
    }
}

struct Owner {
    conn: Mutex<Connection>,
    recovery_required: std::sync::atomic::AtomicBool,
    _lock: File,
    database: PathBuf,
    store_id: [u8; 16],
    project_id: String,
    #[cfg(test)]
    publication_transactions: std::sync::atomic::AtomicU64,
    #[cfg(test)]
    batch_tiny_files: bool,
    #[cfg(test)]
    publication_fault: std::sync::atomic::AtomicU8,
    #[cfg(test)]
    publication_pause: std::sync::atomic::AtomicU8,
}
impl Owner {
    fn publication_committed(&self) {
        #[cfg(test)]
        self.publication_transactions
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
    fn publication_point(&self, point: PublicationPoint) -> Result<(), StoreError> {
        #[cfg(test)]
        {
            use std::sync::atomic::Ordering;
            if self.publication_pause.load(Ordering::Relaxed) == point as u8 {
                let directory = self
                    .database
                    .parent()
                    .ok_or(StoreError::Refused("store directory"))?;
                let pending = directory.join("publication-ready.pending");
                fs::write(&pending, [point as u8])?;
                // The parent waits for the final name. fs::write creates an
                // empty file before writing, so publish only its complete byte.
                fs::rename(pending, directory.join("publication-ready"))?;
                loop {
                    std::thread::park();
                }
            }
            if self
                .publication_fault
                .compare_exchange(point as u8, 0, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                return Err(StoreError::Refused("injected publication interruption"));
            }
        }
        #[cfg(not(test))]
        let _ = point;
        Ok(())
    }
    fn connection(&self) -> Result<MutexGuard<'_, Connection>, StoreError> {
        let conn = self.conn.lock().map_err(|_| StoreError::Poisoned)?;
        // The latch is set while holding this same mutex. Checking after lock
        // acquisition also excludes a caller that waited for the failed COMMIT.
        if self
            .recovery_required
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            return Err(StoreError::RecoveryRequired);
        }
        Ok(conn)
    }
    fn check_owner(&self, selected: &ProvisionalSelection) -> Result<(), StoreError> {
        if selected.store_id != self.store_id || selected.project_id != self.project_id {
            return Err(StoreError::Refused(
                "selection belongs to another store/project",
            ));
        }
        if (selected.generation == 0) != selected.revision.is_none() {
            return Err(StoreError::Refused("invalid selection"));
        }
        Ok(())
    }
    fn selection(&self, conn: &Connection) -> Result<ProvisionalSelection, StoreError> {
        let (generation, revision, empty): (i64, Option<[u8; 32]>, bool) = conn.query_row(
            "SELECT generation, revision, revision IS NULL FROM selection WHERE id=1",
            [],
            |r| Ok((r.get(0)?, fixed_bytes(r, 1)?, r.get(2)?)),
        )?;
        let generation =
            u64::try_from(generation).map_err(|_| StoreError::Corrupt("generation"))?;
        if !empty && revision.is_none() {
            return Err(StoreError::Corrupt("revision"));
        }
        if (generation == 0) != revision.is_none() {
            return Err(StoreError::Corrupt("empty selection"));
        }
        Ok(ProvisionalSelection {
            store_id: self.store_id,
            project_id: self.project_id.clone(),
            generation,
            revision,
        })
    }
    fn prepare_write(&self, conn: &Connection) -> Result<(), StoreError> {
        let directory = self
            .database
            .parent()
            .ok_or(StoreError::Refused("store directory"))?;
        self.prepare_write_with_available(conn, || fs4::available_space(directory))
    }
    fn prepare_write_with_available(
        &self,
        conn: &Connection,
        available: impl Fn() -> io::Result<u64>,
    ) -> Result<(), StoreError> {
        check_provisional_free(available()?)?;
        if self.wal_bytes()? >= WAL_LIMIT / 2 {
            let (_, passive) =
                self.checkpoint_with_available(conn, CheckpointMode::Passive, &available)?;
            if passive.busy || passive.backfilled != passive.frames {
                return Err(StoreError::Busy);
            }
            let (_, truncated) =
                self.checkpoint_with_available(conn, CheckpointMode::Truncate, &available)?;
            if truncated.busy || truncated.retained_wal_bytes != 0 {
                return Err(StoreError::Busy);
            }
        }
        Ok(())
    }
    fn wal_bytes(&self) -> Result<u64, StoreError> {
        match fs::metadata(
            self.database
                .with_file_name("provisional-source.sqlite-wal"),
        ) {
            Ok(m) => Ok(m.len()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(0),
            Err(e) => Err(e.into()),
        }
    }
    fn checkpoint_plan(&self, conn: &Connection) -> Result<CheckpointPlan, StoreError> {
        if !conn.is_autocommit() {
            return Err(StoreError::Refused("checkpoint inside SQL transaction"));
        }
        let pages: i64 = conn.query_row("PRAGMA page_count", [], |r| r.get(0))?;
        let (busy, frames, backfilled): (i64, i64, i64) =
            conn.query_row("PRAGMA wal_checkpoint(NOOP)", [], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })?;
        if busy != 0 {
            return Err(StoreError::Busy);
        }
        let wal_bytes = self.wal_bytes()?;
        let main_bytes = fs::metadata(&self.database)?.len();
        if pages <= 0
            || pages as u64 > MAX_MAIN_PAGES
            || frames < 0
            || backfilled < 0
            || backfilled > frames
            || main_bytes > MAX_MAIN_PAGES * PAGE_BYTES
            || (frames as u64).saturating_mul(PAGE_BYTES + 24) > wal_bytes.saturating_sub(32)
        {
            return Err(StoreError::Corrupt("checkpoint counters/profile"));
        }
        // Pinned walIteratorNext yields increasing distinct DB pages. Keep the
        // cooperating lock/connection until execution. This bounds requested
        // checkpoint data writes, not VFS/CoW/metadata physical allocation.
        let data_bytes = (pages as u64)
            .min((frames - backfilled) as u64)
            .checked_mul(PAGE_BYTES)
            .ok_or(StoreError::Refused("checkpoint allowance overflow"))?;
        Ok(CheckpointPlan {
            wal_bytes,
            main_bytes,
            pages: pages as u64,
            frames: frames as u64,
            backfilled: backfilled as u64,
            data_bytes,
        })
    }
    fn checkpoint_with_available(
        &self,
        conn: &Connection,
        mode: CheckpointMode,
        available: &impl Fn() -> io::Result<u64>,
    ) -> Result<(CheckpointPlan, CheckpointProgress), StoreError> {
        check_provisional_free(available()?)?;
        let plan = self.checkpoint_plan(conn)?;
        let minimum = TREE_LIMIT
            .checked_add(plan.data_bytes)
            .ok_or(StoreError::Refused("checkpoint allowance overflow"))?;
        // Actual available space already excludes the retained main and WAL.
        // Add the full distinct-page data allowance; do not credit the WAL's
        // expected truncation, or only the main file's length growth.
        if available()? < minimum {
            return Err(StoreError::Refused(
                "checkpoint data allowance exceeds provisional free space",
            ));
        }
        let sql = match mode {
            CheckpointMode::Passive => "PRAGMA wal_checkpoint(PASSIVE)",
            CheckpointMode::Truncate => "PRAGMA wal_checkpoint(TRUNCATE)",
        };
        let attempted: Result<(i64, i64, i64), rusqlite::Error> =
            conn.query_row(sql, [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)));
        // Recheck even after a SQL error: a failed checkpoint can have copied
        // pages. Preserve its original error rather than claiming no work.
        let after_space = available();
        let (busy, frames, backfilled) = attempted?;
        check_provisional_free(after_space?)?;
        if busy != 0 && (frames < 0 || backfilled < 0) {
            return Err(StoreError::Busy);
        }
        if !(0..=1).contains(&busy) || frames < 0 || backfilled < 0 || backfilled > frames {
            return Err(StoreError::Corrupt("checkpoint progress"));
        }
        let retained_wal_bytes = self.wal_bytes()?;
        if fs::metadata(&self.database)?.len() > plan.main_bytes.max(plan.pages * PAGE_BYTES)
            || retained_wal_bytes > plan.wal_bytes
        {
            return Err(StoreError::Corrupt("checkpoint extent changed beyond plan"));
        }
        let truncated = matches!(mode, CheckpointMode::Truncate) && busy == 0;
        if (truncated && (frames != 0 || backfilled != 0 || retained_wal_bytes != 0))
            || (!truncated
                && (frames as u64 != plan.frames || (backfilled as u64) < plan.backfilled))
        {
            return Err(StoreError::Corrupt(
                "checkpoint progress changed beyond plan",
            ));
        }
        Ok((
            plan,
            CheckpointProgress {
                busy: busy != 0,
                frames: frames as u64,
                backfilled: backfilled as u64,
                retained_wal_bytes,
            },
        ))
    }
    fn release_pin(&self, id: i64) -> Result<(), StoreError> {
        let conn = self.connection()?;
        self.prepare_write(&conn)?;
        conn.execute("DELETE FROM pin WHERE id=?1", params![id])?;
        Ok(())
    }
}

/// Persistent provisional snapshots. This is not the foundation admission API.
pub struct ProvisionalSourceStore {
    owner: Arc<Owner>,
}

impl ProvisionalSourceStore {
    /// Open/create a dedicated store in an existing directory for one project.
    /// An exclusive advisory lock remains held through every outstanding lease.
    /// Existing incompatible databases refuse rather than being initialized.
    pub fn open(directory: &Path, project_id: &str) -> Result<Self, StoreError> {
        Self::open_with_vfs(directory, project_id, NATIVE_VFS)
    }
    fn open_with_vfs(directory: &Path, project_id: &str, vfs: &str) -> Result<Self, StoreError> {
        if !canonical_uuid(project_id) {
            return Err(StoreError::Refused("project UUID"));
        }
        if !fs::symlink_metadata(directory)?.file_type().is_dir() {
            return Err(StoreError::Refused("store directory"));
        }
        let directory = directory.canonicalize()?;
        let lock_path = directory.join("provisional-source.lock");
        regular_or_absent(&lock_path)?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)?;
        match FileExt::try_lock(&lock) {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => return Err(StoreError::Busy),
            Err(TryLockError::Error(error)) => return Err(error.into()),
        }
        let database = directory.join("provisional-source.sqlite");
        regular_or_absent(&database)?;
        regular_or_absent(&directory.join("provisional-source.sqlite-wal"))?;
        regular_or_absent(&directory.join("provisional-source.sqlite-shm"))?;
        let fresh = !database.exists() || fs::metadata(&database)?.len() == 0;
        if fresh
            && (directory.join("provisional-source.sqlite-wal").exists()
                || directory.join("provisional-source.sqlite-shm").exists())
        {
            return Err(StoreError::Refused(
                "orphan SQLite sidecar beside an empty store",
            ));
        }
        let conn = Connection::open_with_flags_and_vfs(&database, OpenFlags::default(), vfs)?;
        configure_checkpoint_policy(&conn)?;
        if fresh {
            conn.execute_batch(
                "PRAGMA page_size=4096; PRAGMA auto_vacuum=INCREMENTAL;
                PRAGMA application_id=1414025265; PRAGMA user_version=2;",
            )?;
        }
        let app: i64 = conn.query_row("PRAGMA application_id", [], |r| r.get(0))?;
        let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        let page: i64 = conn.query_row("PRAGMA page_size", [], |r| r.get(0))?;
        let vacuum: i64 = conn.query_row("PRAGMA auto_vacuum", [], |r| r.get(0))?;
        if app != APP_ID || version != 2 || page != 4096 || vacuum != 2 {
            return Err(StoreError::Refused("incompatible provisional store format"));
        }
        conn.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;
            PRAGMA foreign_keys=ON; PRAGMA busy_timeout=0; PRAGMA max_page_count=65536;",
        )?;
        let mode: String = conn.query_row("PRAGMA journal_mode", [], |r| r.get(0))?;
        let sync: i64 = conn.query_row("PRAGMA synchronous", [], |r| r.get(0))?;
        let pages: i64 = conn.query_row("PRAGMA max_page_count", [], |r| r.get(0))?;
        if mode != "wal" || sync != 2 || pages != 65536 {
            return Err(StoreError::Refused("SQLite WAL/FULL unavailable"));
        }
        if fresh {
            conn.execute_batch("BEGIN IMMEDIATE;
                CREATE TABLE identity(id INTEGER PRIMARY KEY CHECK(id=1), store_id BLOB NOT NULL, project_id TEXT NOT NULL);
                CREATE TABLE selection(id INTEGER PRIMARY KEY CHECK(id=1), generation INTEGER NOT NULL, revision BLOB, publication_key BLOB);
                CREATE TABLE outcome(key BLOB PRIMARY KEY, request BLOB NOT NULL, generation INTEGER NOT NULL, revision BLOB NOT NULL);
                CREATE INDEX outcome_generation ON outcome(generation DESC);
                CREATE TABLE publication_intent(key BLOB PRIMARY KEY, request BLOB NOT NULL, base_generation INTEGER NOT NULL, base_revision BLOB, target_generation INTEGER NOT NULL, target_revision BLOB NOT NULL, committed INTEGER NOT NULL CHECK(committed IN(0,1)));
                CREATE TABLE blob(hash BLOB PRIMARY KEY, length INTEGER NOT NULL, chunks INTEGER NOT NULL, complete INTEGER NOT NULL CHECK(complete IN(0,1)));
                CREATE TABLE chunk(hash BLOB NOT NULL REFERENCES blob(hash), ix INTEGER NOT NULL, bytes BLOB NOT NULL, PRIMARY KEY(hash,ix));
                CREATE TABLE pin(id INTEGER PRIMARY KEY AUTOINCREMENT, revision BLOB NOT NULL);
                INSERT INTO selection VALUES(1,0,NULL,NULL);")?;
            conn.execute(
                "INSERT INTO identity VALUES(1,randomblob(16),?1)",
                params![project_id],
            )?;
            conn.execute_batch("COMMIT")?;
        }
        conn.prepare("SELECT key,request,base_generation,base_revision,target_generation,target_revision,committed FROM publication_intent LIMIT 0")?;
        conn.prepare("SELECT publication_key FROM selection LIMIT 0")?;
        conn.prepare(COMMITTED_HEAD_SQL)?;
        let (store_id, same_project, valid_project): (Option<[u8; 16]>, bool, bool) = conn
            .query_row(
                "SELECT store_id,project_id FROM identity WHERE id=1",
                [],
                |r| {
                    let project = r.get_ref(1)?;
                    let project = match project {
                        rusqlite::types::ValueRef::Text(bytes) => std::str::from_utf8(bytes).ok(),
                        _ => None,
                    };
                    Ok((
                        fixed_bytes(r, 0)?,
                        project == Some(project_id),
                        project.is_some_and(canonical_uuid),
                    ))
                },
            )?;
        if !valid_project {
            return Err(StoreError::Corrupt("store project identity"));
        }
        if !same_project {
            return Err(StoreError::Refused("different project"));
        }
        let owner = Arc::new(Owner {
            conn: Mutex::new(conn),
            recovery_required: std::sync::atomic::AtomicBool::new(false),
            _lock: lock,
            database,
            store_id: store_id.ok_or(StoreError::Corrupt("store identity"))?,
            project_id: project_id.to_owned(),
            #[cfg(test)]
            publication_transactions: std::sync::atomic::AtomicU64::new(0),
            #[cfg(test)]
            batch_tiny_files: true,
            #[cfg(test)]
            publication_fault: std::sync::atomic::AtomicU8::new(0),
            #[cfg(test)]
            publication_pause: std::sync::atomic::AtomicU8::new(0),
        });
        let store = Self { owner };
        {
            let conn = store.owner.connection()?;
            verify_committed_chain_at_open(&conn)?;
        }
        store.selection()?;
        Ok(store)
    }

    /// Observe the current provisional selection; this does not acquire a pin.
    pub fn selection(&self) -> Result<ProvisionalSelection, StoreError> {
        let conn = self.owner.connection()?;
        let selected = self.owner.selection(&conn)?;
        verify_selected_publication(&conn, &self.owner, &selected)?;
        Ok(selected)
    }

    /// Recover one exact keyed request without a candidate or checkout read.
    /// Pending work is never automatically selected. Committed results verify
    /// their exact intent/outcome and bounded manifest/complete metadata closure;
    /// consumed records still require hash-checked lease reads.
    pub fn publication_status(
        &self,
        expected: &ProvisionalSelection,
        key: [u8; 16],
        target_revision: [u8; 32],
    ) -> Result<PublicationStatus, StoreError> {
        let (target, request) = publication_request(&self.owner, expected, key, target_revision)?;
        let conn = self.owner.connection()?;
        recover_publication(&conn, &self.owner, expected, key, request, &target)
    }

    /// Publish only an F3-validated candidate after rehashing every staged file.
    /// A stable 128-bit key has one persistent provisional outcome; a changed
    /// request conflicts. Outcomes are retained without expiry, up to 1024.
    /// Success requires a confirmed SQLite FULL commit, not physical-budget-
    /// qualified admission. A COMMIT error is returned even if recovery finds
    /// its exact recorded result; query status after reopen to resolve identity.
    /// That error quarantines all operations and lease cleanup on this owner;
    /// drop every store/lease before reopening. Failed pin releases stay retained.
    /// A matching historical retry verifies its manifest and metadata closure,
    /// then returns the original identifier without selecting or leasing it.
    pub fn publish_candidate(
        &self,
        expected: &ProvisionalSelection,
        key: [u8; 16],
        candidate: &StagedCandidate<'_>,
    ) -> Result<ProvisionalSelection, StoreError> {
        self.owner.check_owner(expected)?;
        let manifest = candidate
            .provisional_manifest()
            .map_err(|_| StoreError::Refused("candidate manifest"))?;
        check_profile(&manifest, &self.owner.project_id)?;
        let mut manifest_bytes = BoundedBytes(Vec::new());
        manifest.write_jcs(&mut manifest_bytes)?;
        let revision = *manifest.revision().as_bytes();
        let (result, request) = publication_request(&self.owner, expected, key, revision)?;
        let generation = result.generation as i64;
        let mut conn = self.owner.connection()?;
        match recover_publication(&conn, &self.owner, expected, key, request, &result)? {
            PublicationStatus::Committed(committed) => return Ok(committed),
            PublicationStatus::Missing | PublicationStatus::PendingNeedsRetry => {}
        }
        if self.owner.selection(&conn)? != *expected {
            return Err(StoreError::Refused("stale selection"));
        }
        let outcomes: i64 = conn.query_row("SELECT count(*) FROM outcome", [], |r| r.get(0))?;
        if outcomes >= MAX_OWNERS {
            return Err(StoreError::Refused(
                "provisional outcome limit; retirement unavailable",
            ));
        }
        self.owner.prepare_write(&conn)?;
        if let Some(prior) = expected.revision {
            let bytes = read_blob(&conn, prior, MANIFEST_LIMIT)?;
            let prior_manifest = decode_manifest_jcs(&bytes, MANIFEST_LIMIT)
                .map_err(|_| StoreError::Corrupt("prior manifest"))?;
            check_profile(&prior_manifest, &self.owner.project_id)?;
            let history = prior_manifest
                .files()
                .iter()
                .find(|f| {
                    f.record_identity()
                        .is_some_and(|(_, t)| t == "core.registry-history")
                })
                .ok_or(StoreError::Corrupt("prior history missing"))?;
            let history_bytes = read_blob(&conn, history.blob_identity().1, MAX_RECORD_BYTES)?;
            let next_history = manifest
                .files()
                .iter()
                .find(|f| f.source_path().as_str() == candidate.registry().project().history_path())
                .ok_or(StoreError::Refused("candidate history missing"))?;
            let next_bytes = read_staged(candidate, next_history)?;
            let old = parse_registry_history(&history_bytes)
                .map_err(|_| StoreError::Corrupt("prior history"))?;
            let new = parse_registry_history(&next_bytes)
                .map_err(|_| StoreError::Refused("candidate history"))?;
            compare_carried_histories(&old, &new)
                .map_err(|_| StoreError::Refused("history lineage"))?;
        }
        record_intent(&mut conn, &self.owner, expected, key, request, &result)?;
        self.owner
            .publication_point(PublicationPoint::IntentRecorded)?;
        store_candidate_files(&mut conn, &self.owner, candidate, &manifest)?;
        store_blob(&mut conn, &self.owner, revision, &manifest_bytes.0)?;
        self.owner
            .publication_point(PublicationPoint::BeforeFinal)?;
        self.owner.prepare_write(&conn)?;
        let mut commit_confirmed = false;
        let finalized = (|| -> Result<(), StoreError> {
            let mut tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
            if self.owner.selection(&tx)? != *expected {
                return Err(StoreError::Refused("stale selection"));
            }
            verify_selected_publication(&tx, &self.owner, expected)?;
            if lookup_intent(&tx, expected, key, request, &result)? != Some(false) {
                return Err(StoreError::Corrupt(
                    "publication intent before final commit",
                ));
            }
            check_closure(&tx, &manifest)?;
            tx.execute(
                "UPDATE selection SET generation=?1,revision=?2,publication_key=?3 WHERE id=1",
                params![generation, revision.as_slice(), key.as_slice()],
            )?;
            tx.execute(
                "INSERT INTO outcome VALUES(?1,?2,?3,?4)",
                params![
                    key.as_slice(),
                    request.as_slice(),
                    generation,
                    revision.as_slice()
                ],
            )?;
            if tx.execute(
                "UPDATE publication_intent SET committed=1 WHERE key=?1 AND committed=0",
                params![key.as_slice()],
            )? != 1
            {
                return Err(StoreError::Corrupt("publication intent completion"));
            }
            self.owner
                .publication_point(PublicationPoint::FinalWritten)?;
            // A failed COMMIT must not cause rusqlite's Transaction::drop to
            // issue another SQL command before we can quarantine the owner.
            // Earlier validation/SQL errors retain ordinary rollback-on-drop.
            tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
            if let Err(error) = tx.commit() {
                // SQLite may have synced a commit frame while reporting I/O
                // failure and still expose its old cached selection. Any later
                // write, even a pin DELETE from Drop, could overwrite that
                // uncertain frame. Only a fresh native open resolves its result.
                self.owner
                    .recovery_required
                    .store(true, std::sync::atomic::Ordering::Relaxed);
                return Err(error.into());
            }
            commit_confirmed = true;
            self.owner.publication_committed();
            self.owner
                .publication_point(PublicationPoint::FinalCommitted)?;
            Ok(())
        })();
        match finalized {
            Ok(()) => Ok(result),
            Err(error) => {
                if self
                    .owner
                    .recovery_required
                    .load(std::sync::atomic::Ordering::Relaxed)
                {
                    return Err(error);
                }
                if !conn.is_autocommit() {
                    conn.execute_batch("ROLLBACK")?;
                }
                match recover_publication(&conn, &self.owner, expected, key, request, &result)? {
                    PublicationStatus::Committed(committed) => {
                        if self.owner.selection(&conn)? != committed {
                            return Err(StoreError::Corrupt("uncertain commit selected result"));
                        }
                        if !commit_confirmed {
                            return Err(error);
                        }
                        Ok(committed)
                    }
                    PublicationStatus::Missing | PublicationStatus::PendingNeedsRetry => Err(error),
                }
            }
        }
    }

    /// Atomically compare and pin the current provisional selection. The lease
    /// remains bound to it when later publications advance the current root.
    pub fn lease_current(
        &self,
        expected: &ProvisionalSelection,
    ) -> Result<VerifiedSourceLease, StoreError> {
        self.owner.check_owner(expected)?;
        let revision = expected
            .revision
            .ok_or(StoreError::Refused("empty selection"))?;
        let mut conn = self.owner.connection()?;
        self.owner.prepare_write(&conn)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if self.owner.selection(&tx)? != *expected {
            return Err(StoreError::Refused("stale selection"));
        }
        verify_selected_publication(&tx, &self.owner, expected)?;
        let pins: i64 = tx.query_row("SELECT count(*) FROM pin", [], |r| r.get(0))?;
        if pins >= MAX_OWNERS {
            return Err(StoreError::Refused(
                "pin limit; abandoned-owner recovery unavailable",
            ));
        }
        tx.execute(
            "INSERT INTO pin(revision) VALUES(?1)",
            params![revision.as_slice()],
        )?;
        let id = tx.last_insert_rowid();
        tx.commit()?;
        drop(conn);
        let pin = Arc::new(LeasePin {
            owner: Arc::clone(&self.owner),
            id,
            selected: expected.clone(),
        });
        let manifest = {
            let conn = self.owner.connection()?;
            load_manifest(&conn, revision, &self.owner.project_id)?
        };
        let mut records: Vec<_> = manifest
            .files()
            .iter()
            .enumerate()
            .filter_map(|(index, file)| {
                file.record_identity().map(|(id, _)| (id.to_owned(), index))
            })
            .collect();
        records.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        Ok(VerifiedSourceLease {
            pin,
            manifest: Arc::new(manifest),
            records: Arc::new(records),
        })
    }
}

fn regular_or_absent(path: &Path) -> Result<(), StoreError> {
    match fs::symlink_metadata(path) {
        Ok(m) if m.file_type().is_file() => Ok(()),
        Ok(_) => Err(StoreError::Refused("unsafe store entry")),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

struct LeasePin {
    owner: Arc<Owner>,
    id: i64,
    selected: ProvisionalSelection,
}
impl Drop for LeasePin {
    fn drop(&mut self) {
        // Failure retains the durable row. No collection can turn that leak
        // into data loss; no automatic abandoned-owner recovery is claimed.
        let _ = self.owner.release_pin(self.id);
    }
}

/// Cloneable exact-byte reader, with one shared final-owner pin. This is not an
/// admitted revision or permission to publish a ready build.
#[derive(Clone)]
pub struct VerifiedSourceLease {
    pin: Arc<LeasePin>,
    manifest: Arc<TreeManifest>,
    records: Arc<Vec<(String, usize)>>,
}
impl VerifiedSourceLease {
    /// Pinned provisional source selection, unaffected by later publications.
    pub fn selection(&self) -> &ProvisionalSelection {
        &self.pin.selected
    }
    /// Read one classified record by stable UUID, checking stored length/hash
    /// before exposing bytes. Unclassified files cannot satisfy this lookup.
    pub fn read_record(&self, id: &str) -> Result<Vec<u8>, StoreError> {
        let index = self
            .records
            .binary_search_by(|(known, _)| known.as_str().cmp(id))
            .map_err(|_| StoreError::Refused("unknown record ID"))?;
        let file = &self.manifest.files()[self.records[index].1];
        let conn = self.pin.owner.connection()?;
        let bytes = read_blob(&conn, file.blob_identity().1, MAX_RECORD_BYTES)?;
        drop(conn);
        if bytes.len() as u64 != file.blob_identity().0 {
            return Err(StoreError::Corrupt("record length"));
        }
        let header = crate::environment_json::record_header(&bytes)
            .map_err(|_| StoreError::Corrupt("record envelope"))?;
        if header.id != id
            || file.record_identity().map(|(_, t)| t) != Some(header.record_type.as_str())
        {
            return Err(StoreError::Corrupt("record identity"));
        }
        Ok(bytes)
    }
    /// Release this reader. If it is the final clone, report pin-release failure;
    /// any failed release remains safely retained in the provisional store.
    pub fn close(self) -> Result<(), StoreError> {
        if Arc::strong_count(&self.pin) == 1 {
            self.pin.owner.release_pin(self.pin.id)?;
        }
        Ok(())
    }
}

struct BoundedBytes(Vec<u8>);
impl io::Write for BoundedBytes {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self
            .0
            .len()
            .checked_add(bytes.len())
            .is_none_or(|n| n > MANIFEST_LIMIT)
        {
            return Err(io::Error::other("provisional manifest limit"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn check_profile(manifest: &TreeManifest, project: &str) -> Result<(), StoreError> {
    if manifest.project_id() != project
        || !manifest.dependency_roots().is_empty()
        || manifest.files().len() > MAX_FILES
    {
        return Err(StoreError::Refused("project/tree profile"));
    }
    let mut total = 0u64;
    for f in manifest.files() {
        let n = f.blob_identity().0;
        total = total
            .checked_add(n)
            .ok_or(StoreError::Refused("tree size overflow"))?;
        if n > MAX_RECORD_BYTES as u64 || total > TREE_LIMIT {
            return Err(StoreError::Refused("provisional source byte profile"));
        }
    }
    Ok(())
}
fn read_staged(
    candidate: &StagedCandidate<'_>,
    file: &crate::ManifestEntry,
) -> Result<Vec<u8>, StoreError> {
    let mut bytes = Vec::new();
    append_staged(candidate, file, &mut bytes)?;
    Ok(bytes)
}
fn append_staged(
    candidate: &StagedCandidate<'_>,
    file: &crate::ManifestEntry,
    bytes: &mut Vec<u8>,
) -> Result<(), StoreError> {
    let (length, hash) = file.blob_identity();
    let mut input = candidate
        .registry()
        .stage()
        .open(file.source_path())
        .map_err(|_| StoreError::Refused("staged file"))?;
    let start = bytes.len();
    input.by_ref().take(length + 1).read_to_end(bytes)?;
    if (bytes.len() - start) as u64 != length
        || <[u8; 32]>::from(Sha256::digest(&bytes[start..])) != hash
    {
        return Err(StoreError::Refused("staged bytes changed"));
    }
    Ok(())
}
fn check_closure(conn: &Connection, manifest: &TreeManifest) -> Result<(), StoreError> {
    let mut query = conn.prepare("SELECT length,complete FROM blob WHERE hash=?1")?;
    for f in manifest.files() {
        let (length, hash) = f.blob_identity();
        let found: Option<(i64, i64)> = query
            .query_row(params![hash.as_slice()], |r| Ok((r.get(0)?, r.get(1)?)))
            .optional()?;
        if found != Some((length as i64, 1)) {
            return Err(StoreError::Corrupt("missing/incomplete file closure"));
        }
    }
    Ok(())
}
fn load_manifest(
    conn: &Connection,
    revision: [u8; 32],
    project: &str,
) -> Result<TreeManifest, StoreError> {
    let bytes = read_blob(conn, revision, MANIFEST_LIMIT)?;
    let manifest =
        decode_manifest_jcs(&bytes, MANIFEST_LIMIT).map_err(|_| StoreError::Corrupt("manifest"))?;
    check_profile(&manifest, project)?;
    if *manifest.revision().as_bytes() != revision {
        return Err(StoreError::Corrupt("manifest revision"));
    }
    check_closure(conn, &manifest)?;
    Ok(manifest)
}
fn publication_request(
    owner: &Owner,
    base: &ProvisionalSelection,
    key: [u8; 16],
    revision: [u8; 32],
) -> Result<(ProvisionalSelection, [u8; 32]), StoreError> {
    owner.check_owner(base)?;
    let generation = base
        .generation
        .checked_add(1)
        .and_then(|g| i64::try_from(g).ok())
        .ok_or(StoreError::Refused("generation overflow"))?;
    let target = ProvisionalSelection {
        generation: generation as u64,
        revision: Some(revision),
        ..base.clone()
    };
    let mut request = Sha256::new();
    request.update(b"thrum-provisional-publication-v2\0");
    request.update(owner.store_id);
    request.update(key);
    request.update(base.generation.to_be_bytes());
    request.update(base.revision.unwrap_or([0; 32]));
    request.update(revision);
    Ok((target, request.finalize().into()))
}
struct RawIntent {
    request: Option<[u8; 32]>,
    base_generation: i64,
    base_revision: Option<[u8; 32]>,
    base_empty: bool,
    target_generation: i64,
    target_revision: Option<[u8; 32]>,
    committed: i64,
}
fn lookup_intent(
    conn: &Connection,
    base: &ProvisionalSelection,
    key: [u8; 16],
    request: [u8; 32],
    target: &ProvisionalSelection,
) -> Result<Option<bool>, StoreError> {
    let found = read_intent(conn, key)?;
    found
        .map(|found| {
            let recorded = found
                .request
                .ok_or(StoreError::Corrupt("publication intent request"))?;
            if recorded != request {
                return Err(StoreError::Refused("provisional retry key conflict"));
            }
            if u64::try_from(found.base_generation).ok() != Some(base.generation)
                || found.base_empty != base.revision.is_none()
                || found.base_revision != base.revision
            {
                return Err(StoreError::Corrupt("publication intent base"));
            }
            if u64::try_from(found.target_generation).ok() != Some(target.generation)
                || found.target_revision != target.revision
            {
                return Err(StoreError::Corrupt("publication intent target"));
            }
            match found.committed {
                0 => Ok(false),
                1 => Ok(true),
                _ => Err(StoreError::Corrupt("publication intent state")),
            }
        })
        .transpose()
}
fn read_intent(conn: &Connection, key: [u8; 16]) -> Result<Option<RawIntent>, StoreError> {
    let found:Option<RawIntent>=conn.query_row(
        "SELECT request,base_generation,base_revision,base_revision IS NULL,target_generation,target_revision,committed FROM publication_intent WHERE key=?1",
        params![key.as_slice()],|r|Ok(RawIntent{request:fixed_bytes(r,0)?,base_generation:r.get(1)?,base_revision:fixed_bytes(r,2)?,base_empty:r.get(3)?,target_generation:r.get(4)?,target_revision:fixed_bytes(r,5)?,committed:r.get(6)?}),
    ).optional()?;
    Ok(found)
}
fn verify_selected_publication(
    conn: &Connection,
    owner: &Owner,
    selected: &ProvisionalSelection,
) -> Result<(), StoreError> {
    // One indexed head lookup per verification; the complete bounded chain is
    // checked once at open. Complete outcomes are retained by this profile.
    let high_water: Option<Option<i64>> = conn
        .query_row(COMMITTED_HEAD_SQL, [], |r| r.get(0))
        .optional()?;
    let high_water = match high_water {
        None => 0,
        Some(Some(generation)) if generation > 0 => generation as u64,
        _ => return Err(StoreError::Corrupt("committed generation high-water")),
    };
    if selected.generation != high_water {
        return Err(StoreError::Corrupt(
            "selected generation is not committed high-water",
        ));
    }
    let (key, empty): (Option<[u8; 16]>, bool) = conn.query_row(
        "SELECT publication_key,publication_key IS NULL FROM selection WHERE id=1",
        [],
        |r| Ok((fixed_bytes(r, 0)?, r.get(1)?)),
    )?;
    if selected.generation == 0 {
        if !empty {
            return Err(StoreError::Corrupt("empty selection publication key"));
        }
        let committed: bool =
            conn.query_row("SELECT EXISTS(SELECT 1 FROM outcome)", [], |r| r.get(0))?;
        if committed {
            return Err(StoreError::Corrupt("empty selection has committed outcome"));
        }
        return Ok(());
    }
    let key = key.ok_or(StoreError::Corrupt("selected publication key"))?;
    let found = read_intent(conn, key)?
        .ok_or(StoreError::Corrupt("selected publication intent missing"))?;
    if found.base_empty != found.base_revision.is_none() {
        return Err(StoreError::Corrupt("selected publication base"));
    }
    let base = ProvisionalSelection {
        store_id: owner.store_id,
        project_id: owner.project_id.clone(),
        generation: u64::try_from(found.base_generation)
            .map_err(|_| StoreError::Corrupt("selected publication base"))?,
        revision: found.base_revision,
    };
    if (base.generation == 0) != base.revision.is_none() {
        return Err(StoreError::Corrupt("selected publication base"));
    }
    if base.generation.checked_add(1) != Some(selected.generation) {
        return Err(StoreError::Corrupt("selected publication target"));
    }
    let revision = selected
        .revision
        .ok_or(StoreError::Corrupt("selected publication revision"))?;
    let (target, request) = publication_request(owner, &base, key, revision)?;
    if target != *selected {
        return Err(StoreError::Corrupt("selected publication target"));
    }
    if lookup_intent(conn, &base, key, request, &target)? != Some(true)
        || lookup_outcome(conn, key, request, &target)?.as_ref() != Some(selected)
    {
        return Err(StoreError::Corrupt("selected publication incomplete"));
    }
    Ok(())
}
fn verify_committed_chain_at_open(conn: &Connection) -> Result<(), StoreError> {
    let (count,distinct,minimum,maximum):(i64,i64,Option<i64>,Option<i64>)=conn.query_row(
        "SELECT count(*),count(DISTINCT safe_generation),min(safe_generation),max(safe_generation) FROM (SELECT CASE WHEN typeof(generation)='integer' THEN generation ELSE NULL END AS safe_generation FROM outcome LIMIT ?1)",
        params![MAX_OWNERS+1],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)),
    )?;
    if count == 0 {
        return Ok(());
    }
    if count > MAX_OWNERS || count != distinct || minimum != Some(1) || maximum != Some(count) {
        return Err(StoreError::Corrupt("committed generation chain"));
    }
    Ok(())
}
fn recover_publication(
    conn: &Connection,
    owner: &Owner,
    base: &ProvisionalSelection,
    key: [u8; 16],
    request: [u8; 32],
    target: &ProvisionalSelection,
) -> Result<PublicationStatus, StoreError> {
    let current = owner.selection(conn)?;
    verify_selected_publication(conn, owner, &current)?;
    let outcome = lookup_outcome(conn, key, request, target)?;
    let intent = lookup_intent(conn, base, key, request, target)?;
    match (intent, outcome) {
        (None, None) => Ok(PublicationStatus::Missing),
        (Some(false), None) => Ok(PublicationStatus::PendingNeedsRetry),
        (Some(true), Some(committed)) => {
            let current = owner.selection(conn)?;
            if current.generation < committed.generation
                || (current.generation == committed.generation && current != committed)
            {
                return Err(StoreError::Corrupt("committed selected result"));
            }
            if current.generation == committed.generation {
                let selected_key: bool = conn.query_row(
                    "SELECT publication_key=?1 FROM selection WHERE id=1",
                    params![key.as_slice()],
                    |r| r.get(0),
                )?;
                if !selected_key {
                    return Err(StoreError::Corrupt("committed selected publication key"));
                }
            }
            let revision = committed
                .revision
                .ok_or(StoreError::Corrupt("outcome revision"))?;
            load_manifest(conn, revision, &owner.project_id)?;
            Ok(PublicationStatus::Committed(committed))
        }
        _ => Err(StoreError::Corrupt("publication intent/outcome mismatch")),
    }
}
fn record_intent(
    conn: &mut Connection,
    owner: &Owner,
    base: &ProvisionalSelection,
    key: [u8; 16],
    request: [u8; 32],
    target: &ProvisionalSelection,
) -> Result<(), StoreError> {
    match lookup_intent(conn, base, key, request, target)? {
        Some(false) => return Ok(()),
        Some(true) => return Err(StoreError::Corrupt("publication intent already committed")),
        None => {}
    }
    let count: i64 = conn.query_row("SELECT count(*) FROM publication_intent", [], |r| r.get(0))?;
    if count >= MAX_OWNERS {
        return Err(StoreError::Refused(
            "provisional publication intent limit; retirement unavailable",
        ));
    }
    owner.prepare_write(conn)?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    if owner.selection(&tx)? != *base {
        return Err(StoreError::Refused("stale selection"));
    }
    verify_selected_publication(&tx, owner, base)?;
    let revision = target
        .revision
        .ok_or(StoreError::Corrupt("publication target revision"))?;
    tx.execute(
        "INSERT INTO publication_intent VALUES(?1,?2,?3,?4,?5,?6,0)",
        params![
            key.as_slice(),
            request.as_slice(),
            base.generation as i64,
            base.revision.as_ref().map(|v| v.as_slice()),
            target.generation as i64,
            revision.as_slice()
        ],
    )?;
    tx.commit()?;
    owner.publication_committed();
    Ok(())
}
struct RawOutcome {
    request: Option<[u8; 32]>,
    generation: i64,
    revision: Option<[u8; 32]>,
}
fn lookup_outcome(
    conn: &Connection,
    key: [u8; 16],
    request: [u8; 32],
    expected_result: &ProvisionalSelection,
) -> Result<Option<ProvisionalSelection>, StoreError> {
    let found: Option<RawOutcome> = conn
        .query_row(
            "SELECT request,generation,revision FROM outcome WHERE key=?1",
            params![key.as_slice()],
            |r| {
                Ok(RawOutcome {
                    request: fixed_bytes(r, 0)?,
                    generation: r.get(1)?,
                    revision: fixed_bytes(r, 2)?,
                })
            },
        )
        .optional()?;
    found
        .map(
            |RawOutcome {
                 request: stored,
                 generation,
                 revision,
             }| {
                let stored = stored.ok_or(StoreError::Corrupt("outcome request"))?;
                if stored != request {
                    return Err(StoreError::Refused("provisional retry key conflict"));
                }
                if u64::try_from(generation).ok() != Some(expected_result.generation) {
                    return Err(StoreError::Corrupt("outcome generation"));
                }
                let revision = revision.ok_or(StoreError::Corrupt("outcome revision"))?;
                if Some(revision) != expected_result.revision {
                    return Err(StoreError::Corrupt("outcome revision"));
                }
                Ok(expected_result.clone())
            },
        )
        .transpose()
}
fn fixed_bytes<const N: usize>(
    row: &rusqlite::Row<'_>,
    column: usize,
) -> rusqlite::Result<Option<[u8; N]>> {
    Ok(match row.get_ref(column)? {
        rusqlite::types::ValueRef::Blob(bytes) => bytes.try_into().ok(),
        _ => None,
    })
}
fn read_blob(conn: &Connection, hash: [u8; 32], maximum: usize) -> Result<Vec<u8>, StoreError> {
    read_blob_with_state(conn, hash, maximum, 1)
}
fn read_blob_with_state(
    conn: &Connection,
    hash: [u8; 32],
    maximum: usize,
    expected_complete: i64,
) -> Result<Vec<u8>, StoreError> {
    let header: Option<(i64, i64, i64)> = conn
        .query_row(
            "SELECT length,chunks,complete FROM blob WHERE hash=?1",
            params![hash.as_slice()],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let (length, chunks, complete) = header.ok_or(StoreError::Corrupt("missing blob"))?;
    let length = usize::try_from(length).map_err(|_| StoreError::Corrupt("blob length"))?;
    if length > maximum || chunks != length.div_ceil(CHUNK) as i64 || complete != expected_complete
    {
        return Err(StoreError::Corrupt("blob header"));
    }
    let count: i64 = conn.query_row(
        "SELECT count(*) FROM (SELECT 1 FROM chunk WHERE hash=?1 LIMIT ?2)",
        params![hash.as_slice(), chunks + 1],
        |r| r.get(0),
    )?;
    if count != chunks {
        return Err(StoreError::Corrupt("chunk count"));
    }
    let mut out = Vec::with_capacity(length);
    let mut digest = Sha256::new();
    for ix in 0..chunks {
        // Each query closes its statement before the next chunk/read/consumer.
        // Check metadata before selecting the payload: a borrowed ValueRef
        // alone bounds Rust's copy but not SQLite's payload materialization.
        // The private connection/advisory owner excludes cooperating writes
        // between these queries; arbitrary direct database mutation is outside
        // this provisional ownership protocol.
        let (kind, actual): (String, i64) = conn
            .query_row(
            "SELECT typeof(bytes),CASE WHEN typeof(bytes)='blob' THEN length(bytes) ELSE -1 END FROM chunk WHERE hash=?1 AND ix=?2",
                params![hash.as_slice(), ix],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?
            .ok_or(StoreError::Corrupt("missing chunk"))?;
        if kind != "blob" || actual != (length - out.len()).min(CHUNK) as i64 {
            return Err(StoreError::Corrupt("chunk length"));
        }
        let valid: bool = conn
            .query_row(
                "SELECT bytes FROM chunk WHERE hash=?1 AND ix=?2",
                params![hash.as_slice(), ix],
                |r| {
                    let rusqlite::types::ValueRef::Blob(bytes) = r.get_ref(0)? else {
                        return Ok(false);
                    };
                    if bytes.len() != (length - out.len()).min(CHUNK) {
                        return Ok(false);
                    }
                    digest.update(bytes);
                    out.extend_from_slice(bytes);
                    Ok(true)
                },
            )
            .optional()?
            .ok_or(StoreError::Corrupt("missing chunk"))?;
        if !valid {
            return Err(StoreError::Corrupt("chunk length"));
        }
    }
    if <[u8; 32]>::from(digest.finalize()) != hash {
        return Err(StoreError::Corrupt("blob hash"));
    }
    Ok(out)
}
#[derive(Clone, Copy, Default)]
struct TinyBudget {
    files: usize,
    bytes: usize,
}
impl TinyBudget {
    fn add(&mut self, length: usize) -> Result<(), StoreError> {
        let files = self
            .files
            .checked_add(1)
            .ok_or(StoreError::Refused("tiny batch limits"))?;
        let bytes = self
            .bytes
            .checked_add(length)
            .ok_or(StoreError::Refused("tiny batch limits"))?;
        if length > TINY_FILE_BYTES || files > TINY_BATCH_FILES || bytes > TINY_BATCH_BYTES {
            return Err(StoreError::Refused("tiny batch limits"));
        }
        *self = Self { files, bytes };
        Ok(())
    }
}
#[derive(Default)]
struct TinyBuffer {
    budget: TinyBudget,
    entries: Vec<([u8; 32], usize, usize)>,
    bytes: Vec<u8>,
}
impl TinyBuffer {
    fn flush(&mut self, conn: &mut Connection, owner: &Owner) -> Result<(), StoreError> {
        if self.entries.is_empty() {
            return Ok(());
        }
        store_tiny_batch(
            conn,
            owner,
            self.entries
                .iter()
                .map(|&(hash, start, length)| (hash, &self.bytes[start..start + length])),
        )?;
        self.entries.clear();
        self.bytes.clear();
        self.budget = TinyBudget::default();
        Ok(())
    }
}
fn store_candidate_files(
    conn: &mut Connection,
    owner: &Owner,
    candidate: &StagedCandidate<'_>,
    manifest: &TreeManifest,
) -> Result<(), StoreError> {
    #[cfg(test)]
    if !owner.batch_tiny_files {
        for file in manifest.files() {
            let bytes = read_staged(candidate, file)?;
            store_blob(conn, owner, file.blob_identity().1, &bytes)?;
        }
        return Ok(());
    }
    let mut pending = TinyBuffer::default();
    let mut reused_scratch = Vec::new();
    for file in manifest.files() {
        let (length, hash) = file.blob_identity();
        if length > TINY_FILE_BYTES as u64 {
            pending.flush(conn, owner)?;
            let bytes = read_staged(candidate, file)?;
            store_blob(conn, owner, hash, &bytes)?;
        } else {
            let length = length as usize;
            // An unchanged tiny file still needs its staged hash checked, but
            // does not need a batch entry, packed copy or second input hash.
            if complete_blob_matches(conn, hash, length)? {
                reused_scratch.clear();
                append_staged(candidate, file, &mut reused_scratch)?;
                continue;
            }
            let mut next = pending.budget;
            if next.add(length).is_err() {
                pending.flush(conn, owner)?;
                next = pending.budget;
                next.add(length)?;
            }
            let start = pending.bytes.len();
            append_staged(candidate, file, &mut pending.bytes)?;
            pending.entries.push((hash, start, length));
            pending.budget = next;
        }
    }
    pending.flush(conn, owner)
}
fn complete_blob_matches(
    conn: &Connection,
    hash: [u8; 32],
    length: usize,
) -> Result<bool, StoreError> {
    let stored: Option<(i64, i64, i64)> = conn
        .query_row(
            "SELECT length,chunks,complete FROM blob WHERE hash=?1",
            params![hash.as_slice()],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    match stored {
        Some((stored_length, chunks, 1)) => {
            if stored_length != length as i64 || chunks != length.div_ceil(CHUNK) as i64 {
                return Err(StoreError::Corrupt("reused blob"));
            }
            Ok(true)
        }
        Some((_, _, 0)) | None => Ok(false),
        Some(_) => Err(StoreError::Corrupt("blob state")),
    }
}
fn begin_blob(conn: &Connection, hash: [u8; 32], length: usize) -> Result<(), StoreError> {
    conn.execute("DELETE FROM chunk WHERE hash=?1", params![hash.as_slice()])?;
    conn.execute(
        "DELETE FROM blob WHERE hash=?1 AND complete=0",
        params![hash.as_slice()],
    )?;
    conn.execute(
        "INSERT INTO blob VALUES(?1,?2,?3,0)",
        params![
            hash.as_slice(),
            length as i64,
            length.div_ceil(CHUNK) as i64
        ],
    )?;
    Ok(())
}
fn finish_blob(conn: &Connection, hash: [u8; 32]) -> Result<(), StoreError> {
    if conn.execute(
        "UPDATE blob SET complete=1 WHERE hash=?1 AND complete=0",
        params![hash.as_slice()],
    )? != 1
    {
        return Err(StoreError::Corrupt("blob completion"));
    }
    Ok(())
}
fn store_tiny_batch<'a>(
    conn: &mut Connection,
    owner: &Owner,
    files: impl IntoIterator<Item = ([u8; 32], &'a [u8])>,
) -> Result<(), StoreError> {
    let mut budget = TinyBudget::default();
    let mut fresh = Vec::new();
    for (hash, bytes) in files {
        budget.add(bytes.len())?;
        if <[u8; 32]>::from(Sha256::digest(bytes)) != hash {
            return Err(StoreError::Refused("tiny source hash"));
        }
        if !complete_blob_matches(conn, hash, bytes.len())? {
            fresh.push((hash, bytes));
        }
    }
    // Reused complete metadata is immutable under this connection/owner. An
    // unchanged group needs no SQL write transaction or fresh payload sweep.
    if fresh.is_empty() {
        return Ok(());
    }
    owner.prepare_write(conn)?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    for (hash, bytes) in fresh {
        // Repeated hashes within this batch may already be complete here.
        if complete_blob_matches(&tx, hash, bytes.len())? {
            continue;
        }
        begin_blob(&tx, hash, bytes.len())?;
        if !bytes.is_empty() {
            tx.execute(
                "INSERT INTO chunk VALUES(?1,0,?2)",
                params![hash.as_slice(), bytes],
            )?;
        }
        if read_blob_with_state(&tx, hash, TINY_FILE_BYTES, 0)? != bytes {
            return Err(StoreError::Corrupt("stored blob"));
        }
        finish_blob(&tx, hash)?;
    }
    tx.commit()?;
    owner.publication_committed();
    Ok(())
}
fn store_blob(
    conn: &mut Connection,
    owner: &Owner,
    hash: [u8; 32],
    bytes: &[u8],
) -> Result<(), StoreError> {
    if complete_blob_matches(conn, hash, bytes.len())? {
        // Complete bytes are immutable within this store. Reuse checks their
        // metadata, avoiding an extra stored-tree byte sweep for every edit.
        // Each consumer record read still checks actual chunk lengths/hash.
        return Ok(());
    }
    owner.prepare_write(conn)?;
    {
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        begin_blob(&tx, hash, bytes.len())?;
        tx.commit()?;
        owner.publication_committed();
    }
    for (ix, chunk) in bytes.chunks(CHUNK).enumerate() {
        owner.prepare_write(conn)?;
        conn.execute(
            "INSERT INTO chunk VALUES(?1,?2,?3)",
            params![hash.as_slice(), ix as i64, chunk],
        )?;
        owner.publication_committed();
    }
    // Verify exact stored bytes before marking an immutable blob complete.
    if read_blob_with_state(conn, hash, MAX_RECORD_BYTES, 0)? != bytes {
        return Err(StoreError::Corrupt("stored blob"));
    }
    owner.prepare_write(conn)?;
    finish_blob(conn, hash)?;
    owner.publication_committed();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::budget::Limits;
    use crate::registry_stage::validate_staged_candidate;
    use crate::stage::stage_declared_tree_with;
    use crate::{SourcePath, StagedTree};
    use std::sync::atomic::{AtomicU64, Ordering};

    const PROJECT: &str = "2a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    const SCENE: &str = "3a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    const HISTORY: &str = "4a5ef644-7225-4770-9f4d-e1d9c0a7d515";
    const SOURCE: &str = "10000000-0000-4000-8000-000000000001";
    const LIGHT: &str = "10000000-0000-4000-8000-000000000002";
    const SOURCE_PATH: &str = "records/environment.record.json";
    static NEXT: AtomicU64 = AtomicU64::new(0);

    struct Fixture {
        root: PathBuf,
        cleanup: bool,
    }
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "thrum-provisional-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(root.join("project")).unwrap();
            fs::create_dir_all(root.join("store")).unwrap();
            Self {
                root,
                cleanup: true,
            }
        }
        fn directory(&self) -> PathBuf {
            self.root.join("store")
        }
        fn source(&self) -> PathBuf {
            self.root.join("project")
        }
        fn stage(&self, lux: u64) -> StagedTree {
            use serde_json::json;
            let root = self.source();
            for d in ["records", "scenes", "schemas"] {
                fs::create_dir_all(root.join(d)).unwrap();
            }
            let records = [
                SOURCE_PATH,
                "records/light.record.json",
                "scenes/main.scene.json",
            ];
            let files = [
                (
                    "project.json",
                    json!({"id":PROJECT,"type":"core.project","schema":2,"fields":{"project.format":1,"project.records":records,"project.schemas":[],"project.registryHistory":"schemas/history.registry.json","project.entryScenes":[SCENE]}}),
                ),
                (
                    SOURCE_PATH,
                    json!({"id":SOURCE,"type":"thrum.environment.authored","schema":1,"fields":{"profile":"constant-light-v1","sun":{"light_ref":LIGHT,"toward_source_unit":[0,1,0],"perpendicular_illuminance_lux":lux,"unit_luminance_linear_rec709":[1,1,1],"angular_radius_rad":0.00465,"direction_model":"geometric-straight","applied_attenuation":[]},"ambient_irradiance_lux":[1000,1000,1000],"background_radiance_cd_m2":[0,0,0],"unsupported":["wind","cloud_tau","precipitation","air","thermal","moon","air_quality"],"x.thrum.environment.note":{"retained":true}}}),
                ),
                (
                    "records/light.record.json",
                    json!({"id":LIGHT,"type":"thrum.light.identity","schema":1,"fields":{"driver_binding":"incoming-environment-sun"}}),
                ),
                (
                    "scenes/main.scene.json",
                    json!({"id":SCENE,"type":"core.scene","schema":2,"fields":{"environment.source":SOURCE}}),
                ),
                (
                    "schemas/history.registry.json",
                    json!({"id":HISTORY,"type":"core.registry-history","schema":1,"fields":{"registry.fields":[],"registry.versions":[]}}),
                ),
            ];
            for (p, value) in &files {
                fs::write(root.join(p), serde_json::to_vec(value).unwrap()).unwrap();
            }
            let paths: Vec<_> = files
                .iter()
                .map(|(p, _)| SourcePath::parse(p).unwrap())
                .collect();
            self.capture(&paths)
        }
        fn capture(&self, paths: &[SourcePath]) -> StagedTree {
            stage_declared_tree_with(
                &self.source(),
                paths,
                Limits {
                    main: 256 * 1024 * 1024,
                    wal: 64 * 1024 * 1024,
                    temporary: 64 * 1024 * 1024,
                    free_floor: 0,
                },
                |_| {},
            )
            .unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            if self.cleanup {
                fs::remove_dir_all(&self.root).unwrap();
            }
        }
    }
    fn reopen(fixture: &Fixture) -> ProvisionalSourceStore {
        open_released(fixture, PROJECT)
            .unwrap_or_else(|error| panic!("reopen after final owner release: {error}"))
    }
    fn open_released(
        fixture: &Fixture,
        project: &str,
    ) -> Result<ProvisionalSourceStore, StoreError> {
        let began = std::time::Instant::now();
        loop {
            match ProvisionalSourceStore::open(&fixture.directory(), project) {
                // A concurrent test's spawn can briefly inherit a lock's file
                // descriptor before close-on-exec, after its Rust owner drops.
                Err(StoreError::Busy) if began.elapsed() < std::time::Duration::from_secs(2) => {
                    std::thread::sleep(std::time::Duration::from_millis(1))
                }
                result => return result,
            }
        }
    }
    struct PublicationChild(std::process::Child);
    impl Drop for PublicationChild {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    fn pause_publication_child(f: &Fixture, point: PublicationPoint) -> PublicationChild {
        use std::process::{Command, Stdio};
        use std::time::{Duration, Instant};
        let mut child = PublicationChild(
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "provisional_store::tests::publication_process_interrupt_child",
                    "--nocapture",
                ])
                .env("THRUM_PROVISIONAL_PUBLICATION_CHILD_ROOT", &f.root)
                .env(
                    "THRUM_PROVISIONAL_PUBLICATION_CHILD_POINT",
                    format!("{}", point as u8),
                )
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        let marker = f.directory().join("publication-ready");
        let began = Instant::now();
        while !marker.exists() {
            if let Some(status) = child.0.try_wait().unwrap() {
                panic!("publication child exited before boundary: {status}");
            }
            if began.elapsed() > Duration::from_secs(10) {
                panic!("publication child did not reach boundary");
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(fs::read(marker).unwrap(), [point as u8]);
        child
    }

    fn pin_count(store: &ProvisionalSourceStore) -> i64 {
        store
            .owner
            .connection()
            .unwrap()
            .query_row("SELECT count(*) FROM pin", [], |r| r.get(0))
            .unwrap()
    }
    fn publication_transactions(store: &ProvisionalSourceStore) -> u64 {
        store.owner.publication_transactions.load(Ordering::Relaxed)
    }
    fn hash(bytes: &[u8]) -> [u8; 32] {
        Sha256::digest(bytes).into()
    }
    fn blob_count(store: &ProvisionalSourceStore) -> i64 {
        store
            .owner
            .connection()
            .unwrap()
            .query_row("SELECT count(*) FROM blob", [], |r| r.get(0))
            .unwrap()
    }

    fn add_checkpoint_debt(conn: &Connection) {
        conn.execute_batch("CREATE TABLE physical_probe(bytes BLOB)")
            .unwrap();
        for _ in 0..8 {
            conn.execute(
                "INSERT INTO physical_probe VALUES(zeroblob(?1))",
                [CHUNK as i64],
            )
            .unwrap();
        }
    }

    fn with_master(f: &Fixture, lux: u64) -> StagedTree {
        let initial = f.stage(lux);
        let mut paths: Vec<_> = initial
            .files()
            .iter()
            .map(|file| file.path().clone())
            .collect();
        fs::write(f.source().join("master.bin"), vec![0x39; CHUNK]).unwrap();
        paths.push(SourcePath::parse("master.bin").unwrap());
        f.capture(&paths)
    }

    #[test]
    fn checked_policy_has_no_automatic_noop_or_last_close_main_writes() {
        let f = Fixture::new();
        let observer = checkpoint_vfs::Observer::new(NATIVE_VFS);
        let store = ProvisionalSourceStore::open_with_vfs(&f.directory(), PROJECT, observer.name())
            .unwrap();
        {
            let conn = store.owner.connection().unwrap();
            assert_eq!(
                conn.query_row("PRAGMA wal_autocheckpoint", [], |r| r.get::<_, i64>(0))
                    .unwrap(),
                0
            );
            assert!(
                conn.db_config(DbConfig::SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE)
                    .unwrap()
            );
        }
        let main_writes = observer.counts.main_writes.load(Ordering::Relaxed);
        let truncates = observer.counts.main_truncates.load(Ordering::Relaxed);
        let stage = with_master(&f, 100000);
        let candidate = validate_staged_candidate(&stage, &[]).unwrap();
        let selected = store
            .publish_candidate(&store.selection().unwrap(), [1; 16], &candidate)
            .unwrap();
        assert!(store.owner.wal_bytes().unwrap() > 4 * 1024 * 1024);
        assert!(store.owner.wal_bytes().unwrap() < WAL_LIMIT / 2);
        assert_eq!(
            observer.counts.main_writes.load(Ordering::Relaxed),
            main_writes
        );
        {
            let conn = store.owner.connection().unwrap();
            let plan = store.owner.checkpoint_plan(&conn).unwrap();
            assert!(plan.data_bytes > 0);
            assert_eq!(
                observer.counts.main_writes.load(Ordering::Relaxed),
                main_writes
            );
            assert_eq!(
                observer.counts.main_truncates.load(Ordering::Relaxed),
                truncates
            );
        }
        let main = fs::read(&store.owner.database).unwrap();
        let wal = store.owner.wal_bytes().unwrap();
        drop(store);
        observer.assert_closed();
        assert_eq!(
            observer.counts.main_writes.load(Ordering::Relaxed),
            main_writes
        );
        assert_eq!(
            observer.counts.main_truncates.load(Ordering::Relaxed),
            truncates
        );
        let reopened =
            ProvisionalSourceStore::open_with_vfs(&f.directory(), PROJECT, observer.name())
                .unwrap();
        assert_eq!(reopened.selection().unwrap(), selected);
        assert_eq!(fs::read(&reopened.owner.database).unwrap(), main);
        assert_eq!(reopened.owner.wal_bytes().unwrap(), wal);
        drop(reopened);
        observer.assert_closed();
        assert_eq!(
            observer.counts.main_writes.load(Ordering::Relaxed),
            main_writes
        );
        assert_eq!(
            observer.counts.main_truncates.load(Ordering::Relaxed),
            truncates
        );
    }

    #[test]
    fn checked_partial_and_busy_truncate_retain_wal_until_reader_drain() {
        let f = Fixture::new();
        let observer = checkpoint_vfs::Observer::new(NATIVE_VFS);
        let store = ProvisionalSourceStore::open_with_vfs(&f.directory(), PROJECT, observer.name())
            .unwrap();
        let a_stage = f.stage(100000);
        let a = validate_staged_candidate(&a_stage, &[]).unwrap();
        let sa = store
            .publish_candidate(&store.selection().unwrap(), [1; 16], &a)
            .unwrap();
        {
            let conn = store.owner.connection().unwrap();
            store
                .owner
                .checkpoint_with_available(&conn, CheckpointMode::Truncate, &|| Ok(u64::MAX))
                .unwrap();
        }
        let b_stage = with_master(&f, 75000);
        let b = validate_staged_candidate(&b_stage, &[]).unwrap();
        let sb = store.publish_candidate(&sa, [2; 16], &b).unwrap();
        let reader = Connection::open(&store.owner.database).unwrap();
        configure_checkpoint_policy(&reader).unwrap();
        reader.execute_batch("BEGIN").unwrap();
        assert_eq!(
            reader
                .query_row("SELECT generation FROM selection", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            2
        );
        fs::remove_file(f.source().join("master.bin")).unwrap();
        let c_stage = f.stage(74000);
        let c = validate_staged_candidate(&c_stage, &[]).unwrap();
        let sc = store.publish_candidate(&sb, [3; 16], &c).unwrap();
        let conn = store.owner.connection().unwrap();
        add_checkpoint_debt(&conn);
        let wal = store.owner.wal_bytes().unwrap();
        assert!(wal >= WAL_LIMIT / 2);
        let before = observer.counts.main_bytes.load(Ordering::Relaxed);
        let (plan, progress) = store
            .owner
            .checkpoint_with_available(&conn, CheckpointMode::Passive, &|| Ok(u64::MAX))
            .unwrap();
        assert!(progress.backfilled > 0 && progress.backfilled < progress.frames);
        assert_eq!(progress.retained_wal_bytes, wal);
        assert!(observer.counts.main_bytes.load(Ordering::Relaxed) - before <= plan.data_bytes);
        assert!(matches!(
            store
                .owner
                .prepare_write_with_available(&conn, || Ok(u64::MAX)),
            Err(StoreError::Busy)
        ));
        assert_eq!(store.owner.selection(&conn).unwrap(), sc);
        reader.execute_batch("COMMIT").unwrap();
        drop(reader);
        let latest = Connection::open(&store.owner.database).unwrap();
        configure_checkpoint_policy(&latest).unwrap();
        latest.execute_batch("BEGIN").unwrap();
        assert_eq!(
            latest
                .query_row("SELECT generation FROM selection", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            3
        );
        let (_, complete) = store
            .owner
            .checkpoint_with_available(&conn, CheckpointMode::Passive, &|| Ok(u64::MAX))
            .unwrap();
        assert_eq!(complete.frames, complete.backfilled);
        let (_, blocked) = store
            .owner
            .checkpoint_with_available(&conn, CheckpointMode::Truncate, &|| Ok(u64::MAX))
            .unwrap();
        assert!(blocked.busy);
        assert_eq!(blocked.retained_wal_bytes, wal);
        assert!(matches!(
            store
                .owner
                .prepare_write_with_available(&conn, || Ok(u64::MAX)),
            Err(StoreError::Busy)
        ));
        latest.execute_batch("COMMIT").unwrap();
        drop(latest);
        store
            .owner
            .prepare_write_with_available(&conn, || Ok(u64::MAX))
            .unwrap();
        assert_eq!(store.owner.wal_bytes().unwrap(), 0);
        assert_eq!(store.owner.selection(&conn).unwrap(), sc);
        drop(conn);
        drop(store);
        observer.assert_closed();
    }

    #[test]
    fn final_commit_wal_fault_recovers_exact_key_after_fresh_open() {
        for fault in [
            checkpoint_vfs::WalFault::BeforeWrite,
            checkpoint_vfs::WalFault::BeforeCommitSync,
            checkpoint_vfs::WalFault::AfterCommitSync,
        ] {
            let f = Fixture::new();
            let observer = checkpoint_vfs::Observer::new(NATIVE_VFS);
            let store =
                ProvisionalSourceStore::open_with_vfs(&f.directory(), PROJECT, observer.name())
                    .unwrap();
            let a_stage = f.stage(100000);
            let a = validate_staged_candidate(&a_stage, &[]).unwrap();
            let sa = store
                .publish_candidate(&store.selection().unwrap(), [1; 16], &a)
                .unwrap();
            let lease_a = store.lease_current(&sa).unwrap();
            let bytes_a = lease_a.read_record(SOURCE).unwrap();
            lease_a.close().unwrap();
            let b_stage = f.stage(75000);
            let b = validate_staged_candidate(&b_stage, &[]).unwrap();
            let revision = *b.provisional_manifest().unwrap().revision().as_bytes();
            let sb = ProvisionalSelection {
                generation: sa.generation + 1,
                revision: Some(revision),
                ..sa.clone()
            };

            // Persist B's intent and complete blobs before arming the VFS.
            // Their same-key retry is read-only until the final transaction:
            // this targets actual COMMIT I/O, without a production fault hook.
            store
                .owner
                .publication_fault
                .store(PublicationPoint::BeforeFinal as u8, Ordering::Relaxed);
            assert!(matches!(
                store.publish_candidate(&sa, [2; 16], &b),
                Err(StoreError::Refused("injected publication interruption"))
            ));
            assert_eq!(
                store.publication_status(&sa, [2; 16], revision).unwrap(),
                PublicationStatus::PendingNeedsRetry
            );
            let transactions = publication_transactions(&store);
            let writes = observer.counts.wal_writes.load(Ordering::Relaxed);
            let frames = observer.counts.wal_commit_frames.load(Ordering::Relaxed);
            let syncs = observer.counts.wal_commit_syncs.load(Ordering::Relaxed);
            observer.arm_wal_fault(fault);
            let error = store.publish_candidate(&sa, [2; 16], &b).unwrap_err();
            let StoreError::Sql(rusqlite::Error::SqliteFailure(error, _)) = error else {
                panic!("{fault:?}: final COMMIT did not return a SQLite I/O error");
            };
            assert_eq!(
                error.extended_code,
                if matches!(fault, checkpoint_vfs::WalFault::BeforeWrite) {
                    rusqlite::ffi::SQLITE_IOERR_WRITE
                } else {
                    rusqlite::ffi::SQLITE_IOERR_FSYNC
                },
                "{fault:?}"
            );
            assert_eq!(observer.counts.wal_fault_hits.load(Ordering::Relaxed), 1);
            assert!(observer.counts.wal_writes.load(Ordering::Relaxed) > writes);
            assert_eq!(publication_transactions(&store), transactions);
            if matches!(fault, checkpoint_vfs::WalFault::BeforeWrite) {
                assert_eq!(
                    observer.counts.wal_commit_frames.load(Ordering::Relaxed),
                    frames
                );
                assert_eq!(
                    observer.counts.wal_commit_syncs.load(Ordering::Relaxed),
                    syncs
                );
            } else {
                assert!(observer.counts.wal_commit_frames.load(Ordering::Relaxed) > frames);
            }
            if matches!(fault, checkpoint_vfs::WalFault::BeforeCommitSync) {
                assert_eq!(
                    observer.counts.wal_commit_syncs.load(Ordering::Relaxed),
                    syncs
                );
            }
            if matches!(fault, checkpoint_vfs::WalFault::AfterCommitSync) {
                assert!(observer.counts.wal_commit_syncs.load(Ordering::Relaxed) > syncs);
            }
            assert!(matches!(
                store.selection(),
                Err(StoreError::RecoveryRequired)
            ));
            assert!(matches!(
                store.publication_status(&sa, [2; 16], revision),
                Err(StoreError::RecoveryRequired)
            ));
            // Perform no further writes on the connection reporting an
            // uncertain COMMIT; resolve its outcome through a fresh open.
            drop(store);
            observer.assert_closed();

            // Use the ordinary native VFS for recovery: no observer state or
            // same-connection cached selection can determine this answer.
            let reopened = reopen(&f);
            let committed = !matches!(fault, checkpoint_vfs::WalFault::BeforeWrite);
            assert_eq!(
                reopened.publication_status(&sa, [2; 16], revision).unwrap(),
                if committed {
                    PublicationStatus::Committed(sb.clone())
                } else {
                    PublicationStatus::PendingNeedsRetry
                },
                "{fault:?}"
            );
            let selected = if committed { &sb } else { &sa };
            assert_eq!(&reopened.selection().unwrap(), selected, "{fault:?}");
            let lease = reopened.lease_current(selected).unwrap();
            assert_eq!(
                lease.read_record(SOURCE).unwrap(),
                if committed {
                    fs::read(f.source().join(SOURCE_PATH)).unwrap()
                } else {
                    bytes_a
                }
            );
            lease.close().unwrap();
            assert_eq!(reopened.publish_candidate(&sa, [2; 16], &b).unwrap(), sb);
            assert_eq!(
                reopened.publication_status(&sa, [2; 16], revision).unwrap(),
                PublicationStatus::Committed(sb)
            );
        }
    }

    #[test]
    fn final_commit_sync_error_survives_prior_lease_drop_before_reopen() {
        for explicit_close in [false, true] {
            let f = Fixture::new();
            let observer = checkpoint_vfs::Observer::new(NATIVE_VFS);
            let store =
                ProvisionalSourceStore::open_with_vfs(&f.directory(), PROJECT, observer.name())
                    .unwrap();
            let a_stage = f.stage(100000);
            let a = validate_staged_candidate(&a_stage, &[]).unwrap();
            let sa = store
                .publish_candidate(&store.selection().unwrap(), [1; 16], &a)
                .unwrap();
            let lease_a = store.lease_current(&sa).unwrap();
            let bytes_a = lease_a.read_record(SOURCE).unwrap();
            let b_stage = f.stage(75000);
            let b = validate_staged_candidate(&b_stage, &[]).unwrap();
            let revision = *b.provisional_manifest().unwrap().revision().as_bytes();
            store
                .owner
                .publication_fault
                .store(PublicationPoint::BeforeFinal as u8, Ordering::Relaxed);
            assert!(store.publish_candidate(&sa, [2; 16], &b).is_err());
            let syncs = observer.counts.wal_commit_syncs.load(Ordering::Relaxed);
            observer.arm_wal_fault(checkpoint_vfs::WalFault::AfterCommitSync);
            assert!(matches!(
                store.publish_candidate(&sa, [2; 16], &b),
                Err(StoreError::Sql(_))
            ));
            assert!(observer.counts.wal_commit_syncs.load(Ordering::Relaxed) > syncs);
            let io_counts = || {
                (
                    observer.counts.main_writes.load(Ordering::Relaxed),
                    observer.counts.main_truncates.load(Ordering::Relaxed),
                    observer.counts.wal_writes.load(Ordering::Relaxed),
                    observer.counts.wal_commit_syncs.load(Ordering::Relaxed),
                )
            };
            let after_fault = io_counts();
            assert!(matches!(
                store.selection(),
                Err(StoreError::RecoveryRequired)
            ));
            assert!(matches!(
                store.publication_status(&sa, [2; 16], revision),
                Err(StoreError::RecoveryRequired)
            ));
            assert!(matches!(
                store.publish_candidate(&sa, [2; 16], &b),
                Err(StoreError::RecoveryRequired)
            ));
            assert!(matches!(
                store.lease_current(&sa),
                Err(StoreError::RecoveryRequired)
            ));
            assert!(matches!(
                lease_a.read_record(SOURCE),
                Err(StoreError::RecoveryRequired)
            ));
            // Ordinary public-API cleanup must preserve an uncertain durable B.
            drop(store);
            if explicit_close {
                assert!(matches!(lease_a.close(), Err(StoreError::RecoveryRequired)));
            } else {
                drop(lease_a);
            }
            observer.assert_closed();
            assert_eq!(io_counts(), after_fault);
            let reopened = reopen(&f);
            assert_eq!(
                reopened.publication_status(&sa, [2; 16], revision).unwrap(),
                PublicationStatus::Committed(ProvisionalSelection {
                    generation: sa.generation + 1,
                    revision: Some(revision),
                    ..sa.clone()
                })
            );
            assert_eq!(pin_count(&reopened), 1); // Failed cleanup retains A's pin.
            assert_eq!(
                read_blob(
                    &reopened.owner.connection().unwrap(),
                    hash(&bytes_a),
                    MAX_RECORD_BYTES
                )
                .unwrap(),
                bytes_a
            );
            let sb = reopened.selection().unwrap();
            assert_eq!(reopened.publish_candidate(&sa, [2; 16], &b).unwrap(), sb);
            let lease_b = reopened.lease_current(&sb).unwrap();
            assert_eq!(
                lease_b.read_record(SOURCE).unwrap(),
                fs::read(f.source().join(SOURCE_PATH)).unwrap()
            );
            lease_b.close().unwrap();
            assert_eq!(pin_count(&reopened), 1);
        }
    }

    #[test]
    fn checkpoint_io_failure_preserves_exact_committed_retry_and_prior_lease() {
        let f = Fixture::new();
        let observer = checkpoint_vfs::Observer::new(NATIVE_VFS);
        let store = ProvisionalSourceStore::open_with_vfs(&f.directory(), PROJECT, observer.name())
            .unwrap();
        let a_stage = f.stage(100000);
        let a = validate_staged_candidate(&a_stage, &[]).unwrap();
        let sa = store
            .publish_candidate(&store.selection().unwrap(), [1; 16], &a)
            .unwrap();
        let lease = store.lease_current(&sa).unwrap();
        let old = lease.read_record(SOURCE).unwrap();
        let b_stage = f.stage(75000);
        let b = validate_staged_candidate(&b_stage, &[]).unwrap();
        let sb = store.publish_candidate(&sa, [2; 16], &b).unwrap();
        {
            let conn = store.owner.connection().unwrap();
            add_checkpoint_debt(&conn);
            observer
                .counts
                .fail_main_write
                .store(true, Ordering::Relaxed);
            assert!(matches!(
                store.owner.prepare_write(&conn),
                Err(StoreError::Sql(_))
            ));
        }
        assert_eq!(store.selection().unwrap(), sb);
        assert_eq!(
            store
                .publication_status(&sa, [2; 16], *sb.revision().unwrap())
                .unwrap(),
            PublicationStatus::Committed(sb.clone())
        );
        let attempts = observer.counts.main_writes.load(Ordering::Relaxed);
        assert_eq!(store.publish_candidate(&sa, [2; 16], &b).unwrap(), sb);
        assert_eq!(
            observer.counts.main_writes.load(Ordering::Relaxed),
            attempts
        );
        assert_eq!(lease.read_record(SOURCE).unwrap(), old);
        let c_stage = f.stage(74000);
        let c = validate_staged_candidate(&c_stage, &[]).unwrap();
        assert!(matches!(
            store.publish_candidate(&sb, [3; 16], &c),
            Err(StoreError::Sql(_))
        ));
        assert_eq!(store.selection().unwrap(), sb);
        observer
            .counts
            .fail_main_write
            .store(false, Ordering::Relaxed);
        lease.close().unwrap();
        drop(store);
        observer.assert_closed();
        let reopened =
            ProvisionalSourceStore::open_with_vfs(&f.directory(), PROJECT, observer.name())
                .unwrap();
        assert_eq!(reopened.publish_candidate(&sa, [2; 16], &b).unwrap(), sb);
        drop(reopened);
        observer.assert_closed();
    }

    #[test]
    fn low_space_refuses_before_checkpoint_and_rechecks_after_checkpoint() {
        use std::cell::Cell;

        let f = Fixture::new();
        let store = ProvisionalSourceStore::open(&f.directory(), PROJECT).unwrap();
        let stage = f.stage(100000);
        let candidate = validate_staged_candidate(&stage, &[]).unwrap();
        let selected = store
            .publish_candidate(&store.selection().unwrap(), [1; 16], &candidate)
            .unwrap();
        let conn = store.owner.connection().unwrap();
        store
            .owner
            .checkpoint_with_available(&conn, CheckpointMode::Truncate, &|| Ok(u64::MAX))
            .unwrap();
        // An unreferenced test table creates checkpoint debt without changing
        // the selected source closure or depending on blob publication policy.
        conn.execute_batch("CREATE TABLE physical_probe(bytes BLOB)")
            .unwrap();
        for _ in 0..8 {
            conn.execute(
                "INSERT INTO physical_probe VALUES(zeroblob(?1))",
                [CHUNK as i64],
            )
            .unwrap();
        }
        let wal = f.directory().join("provisional-source.sqlite-wal");
        let main_before = fs::metadata(&store.owner.database).unwrap().len();
        let wal_before = fs::metadata(&wal).unwrap().len();
        assert!(wal_before >= WAL_LIMIT / 2);
        assert!(matches!(
            store.owner.prepare_write_with_available(&conn, || Ok(0)),
            Err(StoreError::Refused(
                "provisional free-space backpressure; no physical reservation"
            ))
        ));
        assert_eq!(
            fs::metadata(&store.owner.database).unwrap().len(),
            main_before
        );
        assert_eq!(fs::metadata(&wal).unwrap().len(), wal_before);
        assert_eq!(store.owner.selection(&conn).unwrap(), selected);

        let plan = store.owner.checkpoint_plan(&conn).unwrap();
        assert!(plan.data_bytes > 0);
        assert!(matches!(
            store
                .owner
                .checkpoint_with_available(&conn, CheckpointMode::Truncate, &|| Ok(TREE_LIMIT
                    + plan.data_bytes
                    - 1)),
            Err(StoreError::Refused(
                "checkpoint data allowance exceeds provisional free space"
            ))
        ));
        assert_eq!(
            fs::metadata(&store.owner.database).unwrap().len(),
            main_before
        );
        assert_eq!(fs::metadata(&wal).unwrap().len(), wal_before);
        let checks = Cell::new(0);
        assert!(matches!(
            store.owner.prepare_write_with_available(&conn, || {
                checks.set(checks.get() + 1);
                Ok(if checks.get() < 4 { u64::MAX } else { 0 })
            }),
            Err(StoreError::Refused(
                "provisional free-space backpressure; no physical reservation"
            ))
        ));
        assert_eq!(checks.get(), 4);
        assert!(fs::metadata(&store.owner.database).unwrap().len() > main_before);
        // PASSIVE can copy data without releasing any retained WAL.
        assert_eq!(fs::metadata(&wal).unwrap().len(), wal_before);
        assert_eq!(store.owner.selection(&conn).unwrap(), selected);
        store
            .owner
            .prepare_write_with_available(&conn, || Ok(u64::MAX))
            .unwrap();
        assert_eq!(fs::metadata(wal).unwrap().len(), 0);
        drop(conn);
        let lease = store.lease_current(&selected).unwrap();
        assert_eq!(
            lease.read_record(SOURCE).unwrap(),
            fs::read(f.source().join(SOURCE_PATH)).unwrap()
        );
        lease.close().unwrap();
    }

    #[test]
    fn tiny_batch_mixes_fresh_reused_empty_and_duplicate_inputs_without_rewriting_complete_bytes() {
        let f = Fixture::new();
        let store = ProvisionalSourceStore::open(&f.directory(), PROJECT).unwrap();
        let reused = b"immutable existing bytes";
        let fresh = vec![0x5a; TINY_FILE_BYTES];
        let empty = [];
        let mut conn = store.owner.connection().unwrap();
        store_blob(&mut conn, &store.owner, hash(reused), reused).unwrap();
        let before = publication_transactions(&store);
        store_tiny_batch(
            &mut conn,
            &store.owner,
            [
                (hash(reused), reused.as_slice()),
                (hash(&fresh), fresh.as_slice()),
                (hash(&empty), empty.as_slice()),
                (hash(&fresh), fresh.as_slice()),
            ],
        )
        .unwrap();
        assert_eq!(publication_transactions(&store) - before, 1);
        for bytes in [reused.as_slice(), fresh.as_slice(), empty.as_slice()] {
            assert_eq!(
                read_blob(&conn, hash(bytes), TINY_FILE_BYTES).unwrap(),
                bytes
            );
        }
        let before = publication_transactions(&store);
        store_tiny_batch(
            &mut conn,
            &store.owner,
            [
                (hash(reused), reused.as_slice()),
                (hash(&fresh), fresh.as_slice()),
            ],
        )
        .unwrap();
        assert_eq!(publication_transactions(&store), before);
        let failed = b"new bytes before refusal";
        assert!(matches!(
            store_tiny_batch(
                &mut conn,
                &store.owner,
                [
                    (hash(failed), failed.as_slice()),
                    ([0; 32], reused.as_slice())
                ]
            ),
            Err(StoreError::Refused("tiny source hash"))
        ));
        assert!(matches!(
            read_blob(&conn, hash(failed), TINY_FILE_BYTES),
            Err(StoreError::Corrupt("missing blob"))
        ));
        assert_eq!(
            read_blob(&conn, hash(reused), TINY_FILE_BYTES).unwrap(),
            reused
        );
    }

    #[test]
    fn tiny_batch_128_and_129_file_limits_and_independent_byte_guard_are_checked() {
        let f = Fixture::new();
        let store = ProvisionalSourceStore::open(&f.directory(), PROJECT).unwrap();
        let files: Vec<_> = (0u64..TINY_BATCH_FILES as u64)
            .map(|n| n.to_be_bytes())
            .collect();
        {
            let mut conn = store.owner.connection().unwrap();
            store_tiny_batch(
                &mut conn,
                &store.owner,
                files.iter().map(|b| (hash(b), b.as_slice())),
            )
            .unwrap();
        }
        assert_eq!(blob_count(&store), 128);
        assert_eq!(publication_transactions(&store), 1);
        let excess: Vec<_> = (1000u64..1000 + TINY_BATCH_FILES as u64 + 1)
            .map(|n| n.to_be_bytes())
            .collect();
        {
            let mut conn = store.owner.connection().unwrap();
            assert!(matches!(
                store_tiny_batch(
                    &mut conn,
                    &store.owner,
                    excess.iter().map(|b| (hash(b), b.as_slice()))
                ),
                Err(StoreError::Refused("tiny batch limits"))
            ));
            let oversized = vec![7; TINY_FILE_BYTES + 1];
            assert!(matches!(
                store_tiny_batch(
                    &mut conn,
                    &store.owner,
                    [(hash(&oversized), oversized.as_slice())]
                ),
                Err(StoreError::Refused("tiny batch limits"))
            ));
        }
        assert_eq!(blob_count(&store), 128);
        assert_eq!(publication_transactions(&store), 1);
        // With 128 files <=4KiB the aggregate cannot reach 1MiB. Exercise the
        // independent arithmetic guard with synthetic accumulated byte counts.
        let mut budget = TinyBudget {
            files: 0,
            bytes: TINY_BATCH_BYTES - TINY_FILE_BYTES,
        };
        budget.add(TINY_FILE_BYTES).unwrap();
        assert_eq!(budget.bytes, TINY_BATCH_BYTES);
        assert!(matches!(
            budget.add(1),
            Err(StoreError::Refused("tiny batch limits"))
        ));
        assert_eq!((budget.files, budget.bytes), (1, TINY_BATCH_BYTES));
        let mut overflow = TinyBudget {
            files: 0,
            bytes: usize::MAX,
        };
        assert!(overflow.add(1).is_err());
    }

    #[test]
    fn failed_tiny_group_rolls_back_fresh_blobs_and_preserves_selection_outcome_and_pin_until_retry()
     {
        let f = Fixture::new();
        let store = ProvisionalSourceStore::open(&f.directory(), PROJECT).unwrap();
        let empty = store.selection().unwrap();
        let a_stage = f.stage(100000);
        let a = validate_staged_candidate(&a_stage, &[]).unwrap();
        let selected_a = store.publish_candidate(&empty, [1; 16], &a).unwrap();
        let lease = store.lease_current(&selected_a).unwrap();
        let old_source = lease.read_record(SOURCE).unwrap();
        let initial_b = f.stage(75000);
        let paths: Vec<_> = initial_b
            .files()
            .iter()
            .map(|file| file.path().clone())
            .collect();
        let light_path = f.source().join("records/light.record.json");
        let mut light: serde_json::Value =
            serde_json::from_slice(&fs::read(&light_path).unwrap()).unwrap();
        light["fields"]["label"] = serde_json::json!("changed light label");
        fs::write(light_path, serde_json::to_vec(&light).unwrap()).unwrap();
        let b_stage = f.capture(&paths);
        let b = validate_staged_candidate(&b_stage, &[]).unwrap();
        let manifest = b.provisional_manifest().unwrap();
        let source_hash = manifest
            .files()
            .iter()
            .find(|file| file.record_identity().is_some_and(|(id, _)| id == SOURCE))
            .unwrap()
            .blob_identity()
            .1;
        let light_hash = manifest
            .files()
            .iter()
            .find(|file| file.record_identity().is_some_and(|(id, _)| id == LIGHT))
            .unwrap()
            .blob_identity()
            .1;
        let hex: String = light_hash.iter().map(|b| format!("{b:02x}")).collect();
        store.owner.connection().unwrap().execute_batch(&format!("CREATE TEMP TRIGGER corrupt_tiny AFTER INSERT ON chunk WHEN NEW.hash=X'{hex}' BEGIN UPDATE chunk SET bytes=zeroblob(length(NEW.bytes)) WHERE hash=NEW.hash AND ix=NEW.ix; END")).unwrap();
        let blobs_before = blob_count(&store);
        let commits_before = publication_transactions(&store);
        assert!(matches!(
            store.publish_candidate(&selected_a, [2; 16], &b),
            Err(StoreError::Corrupt("blob hash"))
        ));
        assert_eq!(blob_count(&store), blobs_before);
        assert_eq!(publication_transactions(&store), commits_before + 1); // durable pending intent
        assert_eq!(store.selection().unwrap(), selected_a);
        assert_eq!(pin_count(&store), 1);
        assert_eq!(lease.read_record(SOURCE).unwrap(), old_source);
        {
            let conn = store.owner.connection().unwrap();
            for h in [source_hash, light_hash] {
                assert!(matches!(
                    read_blob(&conn, h, TINY_FILE_BYTES),
                    Err(StoreError::Corrupt("missing blob"))
                ));
            }
            let outcomes: i64 = conn
                .query_row("SELECT count(*) FROM outcome", [], |r| r.get(0))
                .unwrap();
            assert_eq!(outcomes, 1);
            conn.execute_batch("DROP TRIGGER corrupt_tiny").unwrap();
        }
        let selected_b = store.publish_candidate(&selected_a, [2; 16], &b).unwrap();
        assert_eq!(selected_b.generation(), 2);
        assert_eq!(
            store.publish_candidate(&selected_a, [2; 16], &b).unwrap(),
            selected_b
        );
        assert_eq!(lease.read_record(SOURCE).unwrap(), old_source);
        lease.close().unwrap();
        assert_eq!(pin_count(&store), 0);
    }

    #[test]
    fn reused_tiny_files_still_refuse_staged_drift_before_publication() {
        let f = Fixture::new();
        let store = ProvisionalSourceStore::open(&f.directory(), PROJECT).unwrap();
        let a_stage = f.stage(100000);
        let a = validate_staged_candidate(&a_stage, &[]).unwrap();
        let selected_a = store
            .publish_candidate(&store.selection().unwrap(), [1; 16], &a)
            .unwrap();
        let lease = store.lease_current(&selected_a).unwrap();
        let source_a = lease.read_record(SOURCE).unwrap();
        let b_stage = f.stage(75000);
        let b = validate_staged_candidate(&b_stage, &[]).unwrap();
        let light = b_stage.test_directory().join("records/light.record.json");
        let original = fs::read(&light).unwrap();
        let mut drifted = original.clone();
        drifted[0] = b' ';
        fs::write(&light, drifted).unwrap();
        let commits = publication_transactions(&store);
        let blobs = blob_count(&store);
        assert!(matches!(
            store.publish_candidate(&selected_a, [2; 16], &b),
            Err(StoreError::Refused("staged bytes changed"))
        ));
        assert_eq!(store.selection().unwrap(), selected_a);
        assert_eq!(publication_transactions(&store), commits + 1); // durable pending intent
        assert_eq!(blob_count(&store), blobs);
        assert_eq!(pin_count(&store), 1);
        assert_eq!(lease.read_record(SOURCE).unwrap(), source_a);
        fs::write(light, original).unwrap();
        let selected_b = store.publish_candidate(&selected_a, [2; 16], &b).unwrap();
        assert_eq!(selected_b.generation(), 2);
        lease.close().unwrap();
    }

    #[test]
    fn larger_files_keep_the_bounded_chunk_path() {
        let f = Fixture::new();
        let store = ProvisionalSourceStore::open(&f.directory(), PROJECT).unwrap();
        let bytes = vec![0x39; CHUNK + 1];
        let mut conn = store.owner.connection().unwrap();
        store_blob(&mut conn, &store.owner, hash(&bytes), &bytes).unwrap();
        assert_eq!(publication_transactions(&store), 4); // header + two chunks + complete
        assert_eq!(
            read_blob(&conn, hash(&bytes), MAX_RECORD_BYTES).unwrap(),
            bytes
        );
    }

    #[test]
    #[ignore = "isolated release probe of provisional tiny-file publication transactions, WAL length and edit latency"]
    fn measure_tiny_file_batches_against_individual_transactions() {
        use std::time::Instant;
        const TINY_FILES: usize = 1000;
        // Counters advance only after successful publication COMMIT/autocommit
        // calls. Initialization, pin writes and checkpoint I/O are excluded.
        // WAL length is the observed file length after publication under the
        // explicit checkpoint policy, not allocated bytes or an RSS peak.
        for batching in [false, true] {
            let f = Fixture::new();
            let initial = f.stage(100000);
            let mut paths: Vec<_> = initial
                .files()
                .iter()
                .map(|file| file.path().clone())
                .collect();
            fs::create_dir_all(f.source().join("notes")).unwrap();
            for n in 0..TINY_FILES {
                let path = format!("notes/{n:04}.txt");
                let mut bytes = vec![b'x'; TINY_FILE_BYTES];
                bytes[..8].copy_from_slice(&(n as u64).to_le_bytes());
                fs::write(f.source().join(&path), bytes).unwrap();
                paths.push(SourcePath::parse(&path).unwrap());
            }
            let a_stage = f.capture(&paths);
            let a = validate_staged_candidate(&a_stage, &[]).unwrap();
            let mut store = ProvisionalSourceStore::open(&f.directory(), PROJECT).unwrap();
            Arc::get_mut(&mut store.owner).unwrap().batch_tiny_files = batching;
            let empty = store.selection().unwrap();
            let began = Instant::now();
            let selected_a = store.publish_candidate(&empty, [1; 16], &a).unwrap();
            let fresh_elapsed = began.elapsed();
            let fresh_transactions = publication_transactions(&store);
            let wal = f.directory().join("provisional-source.sqlite-wal");
            let fresh_wal = fs::metadata(&wal).unwrap().len();
            let lease = store.lease_current(&selected_a).unwrap();
            let old_source = lease.read_record(SOURCE).unwrap();
            let source_path = f.source().join(SOURCE_PATH);
            let mut source: serde_json::Value =
                serde_json::from_slice(&fs::read(&source_path).unwrap()).unwrap();
            source["fields"]["sun"]["perpendicular_illuminance_lux"] = serde_json::json!(75000);
            fs::write(source_path, serde_json::to_vec(&source).unwrap()).unwrap();
            let b_stage = f.capture(&paths);
            let b = validate_staged_candidate(&b_stage, &[]).unwrap();
            let began = Instant::now();
            let selected_b = store.publish_candidate(&selected_a, [2; 16], &b).unwrap();
            let edit_elapsed = began.elapsed();
            let edit_transactions = publication_transactions(&store) - fresh_transactions;
            let edit_wal = fs::metadata(&wal).unwrap().len();
            assert_eq!(store.selection().unwrap(), selected_b);
            assert_eq!(lease.read_record(SOURCE).unwrap(), old_source);
            lease.close().unwrap();
            eprintln!(
                "provisional_tiny_files batching={batching} files={} tiny_files={TINY_FILES} tiny_bytes={} fresh_transactions={fresh_transactions} fresh_elapsed={fresh_elapsed:?} fresh_wal_length={fresh_wal} edit_transactions={edit_transactions} edit_elapsed={edit_elapsed:?} edit_wal_length={edit_wal}",
                paths.len(),
                TINY_FILES * TINY_FILE_BYTES
            );
        }
    }

    #[test]
    #[ignore = "local instrumented 1000 tiny/12 MiB master publication and checked-checkpoint data cost"]
    fn measure_checked_checkpoint_fresh_and_record_edit() {
        use std::time::Instant;
        let f = Fixture::new();
        let initial = f.stage(100000);
        let mut paths: Vec<_> = initial
            .files()
            .iter()
            .map(|file| file.path().clone())
            .collect();
        fs::create_dir_all(f.source().join("notes")).unwrap();
        for n in 0..1000u64 {
            let path = format!("notes/{n:04}.txt");
            let mut bytes = vec![b'x'; TINY_FILE_BYTES];
            bytes[..8].copy_from_slice(&n.to_le_bytes());
            fs::write(f.source().join(&path), bytes).unwrap();
            paths.push(SourcePath::parse(&path).unwrap());
        }
        fs::write(f.source().join("master.bin"), vec![0x42; 12 * 1024 * 1024]).unwrap();
        paths.push(SourcePath::parse("master.bin").unwrap());
        let observer = checkpoint_vfs::Observer::new(NATIVE_VFS);
        let store = ProvisionalSourceStore::open_with_vfs(&f.directory(), PROJECT, observer.name())
            .unwrap();
        let mut selected = store.selection().unwrap();
        let mut old_lease = None;
        for (n, label) in [(1, "fresh"), (2, "one-record-edit")] {
            if n == 2 {
                let path = f.source().join(SOURCE_PATH);
                let mut source: serde_json::Value =
                    serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
                source["fields"]["sun"]["perpendicular_illuminance_lux"] = serde_json::json!(75000);
                fs::write(path, serde_json::to_vec(&source).unwrap()).unwrap();
            }
            let stage = f.capture(&paths);
            let candidate = validate_staged_candidate(&stage, &[]).unwrap();
            let start = Instant::now();
            selected = store
                .publish_candidate(&selected, [n; 16], &candidate)
                .unwrap();
            let publication = start.elapsed();
            let conn = store.owner.connection().unwrap();
            let before = observer.counts.main_bytes.load(Ordering::Relaxed);
            let start = Instant::now();
            let (plan, passive) = store
                .owner
                .checkpoint_with_available(&conn, CheckpointMode::Passive, &|| Ok(u64::MAX))
                .unwrap();
            let before_truncate = observer.counts.main_bytes.load(Ordering::Relaxed);
            let (truncate_plan, truncated) = store
                .owner
                .checkpoint_with_available(&conn, CheckpointMode::Truncate, &|| Ok(u64::MAX))
                .unwrap();
            assert!(
                observer.counts.main_bytes.load(Ordering::Relaxed) - before_truncate
                    <= truncate_plan.data_bytes
            );
            let maintenance = start.elapsed();
            let main_writes = observer.counts.main_bytes.load(Ordering::Relaxed) - before;
            assert!(main_writes <= plan.data_bytes);
            assert!(!truncated.busy && truncated.retained_wal_bytes == 0);
            eprintln!(
                "checked_checkpoint workload={label} files={} publication={publication:?} maintenance={maintenance:?} main_before={} wal_before={} logical_pages={} frames={} backfilled_before={} data_allowance={} main_data_written={main_writes} passive_backfilled={} wal_after={}",
                paths.len(),
                plan.main_bytes,
                plan.wal_bytes,
                plan.pages,
                plan.frames,
                plan.backfilled,
                plan.data_bytes,
                passive.backfilled,
                truncated.retained_wal_bytes
            );
            drop(conn);
            if n == 1 {
                let lease = store.lease_current(&selected).unwrap();
                let bytes = lease.read_record(SOURCE).unwrap();
                old_lease = Some((lease, bytes));
            } else {
                let (lease, bytes) = old_lease.take().unwrap();
                assert_eq!(lease.read_record(SOURCE).unwrap(), bytes);
                lease.close().unwrap();
            }
        }
        drop(store);
        observer.assert_closed();
        // VFS registry observation adds synchronization per main write. These
        // single release samples exclude capture/validation and are not an
        // uninstrumented edit-latency, RSS or physical-allocation qualification.
    }

    #[test]
    fn provisional_ab_publication_retry_and_shared_final_owner_survive_reopen() {
        let f = Fixture::new();
        let store = ProvisionalSourceStore::open(&f.directory(), PROJECT).unwrap();
        let empty = store.selection().unwrap();
        assert!(matches!(
            store.lease_current(&empty),
            Err(StoreError::Refused("empty selection"))
        ));
        let a_stage = f.stage(100000);
        let a = validate_staged_candidate(&a_stage, &[]).unwrap();
        let a_bytes = fs::read(f.source().join(SOURCE_PATH)).unwrap();
        let selected_a = store.publish_candidate(&empty, [1; 16], &a).unwrap();
        let lease_a = store.lease_current(&selected_a).unwrap();
        let clone_a = lease_a.clone();
        assert_eq!(pin_count(&store), 1);
        assert_eq!(
            store.publish_candidate(&empty, [1; 16], &a).unwrap(),
            selected_a
        );
        let b_stage = f.stage(75000);
        let b = validate_staged_candidate(&b_stage, &[]).unwrap();
        let b_bytes = fs::read(f.source().join(SOURCE_PATH)).unwrap();
        assert!(matches!(
            store.publish_candidate(&selected_a, [1; 16], &b),
            Err(StoreError::Refused("provisional retry key conflict"))
        ));
        let selected_b = store.publish_candidate(&selected_a, [2; 16], &b).unwrap();
        assert_ne!(selected_a.revision(), selected_b.revision());
        assert_eq!(store.selection().unwrap(), selected_b);
        assert_eq!(lease_a.read_record(SOURCE).unwrap(), a_bytes);
        assert_eq!(clone_a.selection(), &selected_a);
        assert!(matches!(
            store.lease_current(&selected_a),
            Err(StoreError::Refused("stale selection"))
        ));
        assert!(matches!(
            store.publish_candidate(&empty, [3; 16], &a),
            Err(StoreError::Refused("stale selection"))
        ));
        assert!(matches!(
            lease_a.read_record("unclassified"),
            Err(StoreError::Refused("unknown record ID"))
        ));
        let lease_b = store.lease_current(&selected_b).unwrap();
        assert_eq!(lease_b.read_record(SOURCE).unwrap(), b_bytes);
        assert_eq!(pin_count(&store), 2);
        lease_a.close().unwrap();
        assert_eq!(pin_count(&store), 2);
        clone_a.close().unwrap();
        assert_eq!(pin_count(&store), 1);
        drop(store);
        assert!(matches!(
            ProvisionalSourceStore::open(&f.directory(), PROJECT),
            Err(StoreError::Busy)
        ));
        assert_eq!(lease_b.read_record(SOURCE).unwrap(), b_bytes);
        lease_b.close().unwrap();
        let reopened = reopen(&f);
        assert_eq!(pin_count(&reopened), 0);
        assert_eq!(reopened.selection().unwrap(), selected_b);
        assert_eq!(
            reopened
                .publish_candidate(&selected_a, [2; 16], &b)
                .unwrap(),
            selected_b
        );
        assert_eq!(
            reopened.publish_candidate(&empty, [1; 16], &a).unwrap(),
            selected_a
        );
        assert_eq!(reopened.selection().unwrap(), selected_b);
        drop(reopened);
        assert!(matches!(
            open_released(&f, SCENE),
            Err(StoreError::Refused("different project"))
        ));
    }

    #[test]
    fn retry_outcome_cannot_redirect_revision_or_change_publication_generation() {
        let f = Fixture::new();
        let store = ProvisionalSourceStore::open(&f.directory(), PROJECT).unwrap();
        let empty = store.selection().unwrap();
        let a_stage = f.stage(100000);
        let a = validate_staged_candidate(&a_stage, &[]).unwrap();
        let selected_a = store.publish_candidate(&empty, [1; 16], &a).unwrap();
        let b_stage = f.stage(75000);
        let b = validate_staged_candidate(&b_stage, &[]).unwrap();
        let selected_b = store.publish_candidate(&selected_a, [2; 16], &b).unwrap();
        let a_revision = selected_a.revision.unwrap();
        let b_revision = selected_b.revision.unwrap();
        // Both revisions are valid stored closures. Corrupt only A's outcome,
        // preserving its original request digest, so content verification alone
        // cannot detect redirection to B or a fabricated generation.
        for (generation, revision, expected_error) in [
            (1, b_revision, "outcome revision"),
            (2, a_revision, "outcome generation"),
            (2, b_revision, "outcome generation"),
            (0, a_revision, "outcome generation"),
            (-1, a_revision, "outcome generation"),
            (
                3,
                a_revision,
                "selected generation is not committed high-water",
            ),
        ] {
            store
                .owner
                .connection()
                .unwrap()
                .execute(
                    "UPDATE outcome SET generation=?1,revision=?2 WHERE key=?3",
                    params![generation, revision.as_slice(), [1u8; 16].as_slice()],
                )
                .unwrap();
            match store.publish_candidate(&empty, [1; 16], &a) {
                Err(StoreError::Corrupt(reason)) => assert_eq!(reason, expected_error),
                other => panic!("corrupted retry returned {other:?}"),
            }
            {
                let conn = store.owner.connection().unwrap();
                assert_eq!(store.owner.selection(&conn).unwrap(), selected_b);
                conn.execute(
                    "UPDATE outcome SET generation=1,revision=?1 WHERE key=?2",
                    params![a_revision.as_slice(), [1u8; 16].as_slice()],
                )
                .unwrap();
            }
            assert_eq!(store.selection().unwrap(), selected_b);
            assert_eq!(
                store.publish_candidate(&selected_a, [2; 16], &b).unwrap(),
                selected_b
            );
        }
        store
            .owner
            .connection()
            .unwrap()
            .execute(
                "UPDATE outcome SET generation=1,revision=?1 WHERE key=?2",
                params![a_revision.as_slice(), [1u8; 16].as_slice()],
            )
            .unwrap();
        assert_eq!(
            store.publish_candidate(&empty, [1; 16], &a).unwrap(),
            selected_a
        );
        assert_eq!(store.selection().unwrap(), selected_b);
        let invalid = ProvisionalSelection {
            generation: 0,
            revision: Some(a_revision),
            ..empty.clone()
        };
        assert!(matches!(
            store.publish_candidate(&invalid, [3; 16], &a),
            Err(StoreError::Refused("invalid selection"))
        ));
    }

    #[test]
    fn provisional_reads_refuse_corruption_and_failed_acquisition_releases_its_pin() {
        let f = Fixture::new();
        let store = ProvisionalSourceStore::open(&f.directory(), PROJECT).unwrap();
        let stage = f.stage(100000);
        let candidate = validate_staged_candidate(&stage, &[]).unwrap();
        let empty = store.selection().unwrap();
        let selected = store
            .publish_candidate(&empty, [1; 16], &candidate)
            .unwrap();
        let lease = store.lease_current(&selected).unwrap();
        let index = lease
            .records
            .binary_search_by(|(id, _)| id.as_str().cmp(SOURCE))
            .unwrap();
        let file = &lease.manifest.files()[lease.records[index].1];
        let hash = file.blob_identity().1;
        let original = lease.read_record(SOURCE).unwrap();
        let mut corrupt = original.clone();
        corrupt[0] = b' ';
        store
            .owner
            .connection()
            .unwrap()
            .execute(
                "UPDATE chunk SET bytes=?1 WHERE hash=?2 AND ix=0",
                params![corrupt, hash.as_slice()],
            )
            .unwrap();
        assert!(matches!(
            lease.read_record(SOURCE),
            Err(StoreError::Corrupt("blob hash"))
        ));
        for invalid_length in [CHUNK + 1, CHUNK + 2] {
            store
                .owner
                .connection()
                .unwrap()
                .execute(
                    "UPDATE chunk SET bytes=zeroblob(?1) WHERE hash=?2 AND ix=0",
                    params![invalid_length as i64, hash.as_slice()],
                )
                .unwrap();
            assert!(matches!(
                lease.read_record(SOURCE),
                Err(StoreError::Corrupt("chunk length"))
            ));
        }
        store
            .owner
            .connection()
            .unwrap()
            .execute(
                "UPDATE chunk SET bytes=CAST(zeroblob(?1) AS TEXT) WHERE hash=?2 AND ix=0",
                params![(CHUNK + 2) as i64, hash.as_slice()],
            )
            .unwrap();
        assert!(matches!(
            lease.read_record(SOURCE),
            Err(StoreError::Corrupt("chunk length"))
        ));
        store
            .owner
            .connection()
            .unwrap()
            .execute(
                "UPDATE chunk SET bytes=?1 WHERE hash=?2 AND ix=0",
                params![original, hash.as_slice()],
            )
            .unwrap();
        store
            .owner
            .connection()
            .unwrap()
            .execute(
                "UPDATE blob SET complete=0 WHERE hash=?1",
                params![hash.as_slice()],
            )
            .unwrap();
        assert!(matches!(
            store.lease_current(&selected),
            Err(StoreError::Corrupt("missing/incomplete file closure"))
        ));
        assert!(matches!(
            store.publish_candidate(&empty, [1; 16], &candidate),
            Err(StoreError::Corrupt("missing/incomplete file closure"))
        ));
        assert_eq!(pin_count(&store), 1);
        store
            .owner
            .connection()
            .unwrap()
            .execute(
                "UPDATE blob SET complete=1 WHERE hash=?1",
                params![hash.as_slice()],
            )
            .unwrap();
        store
            .owner
            .connection()
            .unwrap()
            .execute("DELETE FROM chunk WHERE hash=?1", params![hash.as_slice()])
            .unwrap();
        assert!(matches!(
            lease.read_record(SOURCE),
            Err(StoreError::Corrupt("chunk count"))
        ));
        lease.close().unwrap();
        assert_eq!(pin_count(&store), 0);
        assert_eq!(store.selection().unwrap(), selected);
    }

    #[test]
    fn provisional_store_identity_drift_and_abandoned_pin_limit_fail_closed() {
        let f = Fixture::new();
        let store = ProvisionalSourceStore::open(&f.directory(), PROJECT).unwrap();
        let empty = store.selection().unwrap();
        let stage = f.stage(100000);
        let candidate = validate_staged_candidate(&stage, &[]).unwrap();
        let source = stage.test_directory().join(SOURCE_PATH);
        let original = fs::read(&source).unwrap();
        fs::write(&source, b"changed after F3 validation").unwrap();
        assert!(matches!(
            store.publish_candidate(&empty, [1; 16], &candidate),
            Err(StoreError::Refused("staged bytes changed"))
        ));
        assert_eq!(store.selection().unwrap(), empty);
        fs::write(&source, &original).unwrap();
        let selected = store
            .publish_candidate(&empty, [1; 16], &candidate)
            .unwrap();
        let other = Fixture::new();
        let other_store = ProvisionalSourceStore::open(&other.directory(), PROJECT).unwrap();
        assert!(matches!(
            other_store.lease_current(&selected),
            Err(StoreError::Refused(
                "selection belongs to another store/project"
            ))
        ));
        // Simulate durable rows whose process owners have been lost. Reopen
        // retains them; this slice refuses capacity rather than guessing expiry.
        {
            let mut conn = store.owner.connection().unwrap();
            let tx = conn.transaction().unwrap();
            for _ in 0..MAX_OWNERS {
                tx.execute(
                    "INSERT INTO pin(revision) VALUES(?1)",
                    params![selected.revision.unwrap().as_slice()],
                )
                .unwrap();
            }
            tx.commit().unwrap();
        }
        drop(store);
        let reopened = reopen(&f);
        assert_eq!(pin_count(&reopened), MAX_OWNERS);
        assert!(matches!(
            reopened.lease_current(&selected),
            Err(StoreError::Refused(
                "pin limit; abandoned-owner recovery unavailable"
            ))
        ));
        assert_eq!(reopened.selection().unwrap(), selected);
    }

    #[test]
    fn publication_intent_survives_interruption_and_recovers_exact_status_on_reopen() {
        for point in [
            PublicationPoint::IntentRecorded,
            PublicationPoint::BeforeFinal,
            PublicationPoint::FinalWritten,
            PublicationPoint::FinalCommitted,
        ] {
            let f = Fixture::new();
            let store = ProvisionalSourceStore::open(&f.directory(), PROJECT).unwrap();
            let empty = store.selection().unwrap();
            let a_stage = f.stage(100000);
            let a = validate_staged_candidate(&a_stage, &[]).unwrap();
            let selected_a = store.publish_candidate(&empty, [1; 16], &a).unwrap();
            let b_stage = f.stage(75000);
            let b = validate_staged_candidate(&b_stage, &[]).unwrap();
            let revision = *b.provisional_manifest().unwrap().revision().as_bytes();
            assert_eq!(
                store
                    .publication_status(&selected_a, [2; 16], revision)
                    .unwrap(),
                PublicationStatus::Missing
            );
            store
                .owner
                .publication_fault
                .store(point as u8, Ordering::Relaxed);
            let result = store.publish_candidate(&selected_a, [2; 16], &b);
            let expected_target = ProvisionalSelection {
                generation: 2,
                revision: Some(revision),
                ..selected_a.clone()
            };
            if point == PublicationPoint::FinalCommitted {
                // Simulate a final acknowledgement error after SQLite committed.
                // Exact keyed recovery returns only the actual selected target.
                assert_eq!(result.unwrap(), expected_target);
                assert_eq!(
                    store
                        .publication_status(&selected_a, [2; 16], revision)
                        .unwrap(),
                    PublicationStatus::Committed(expected_target.clone())
                );
            } else {
                assert!(matches!(
                    result,
                    Err(StoreError::Refused("injected publication interruption"))
                ));
                assert_eq!(store.selection().unwrap(), selected_a);
                assert_eq!(
                    store
                        .publication_status(&selected_a, [2; 16], revision)
                        .unwrap(),
                    PublicationStatus::PendingNeedsRetry
                );
                assert_eq!(
                    store
                        .owner
                        .connection()
                        .unwrap()
                        .query_row("SELECT count(*) FROM outcome", [], |r| r.get::<_, i64>(0))
                        .unwrap(),
                    1
                );
            }
            drop(store);
            let reopened = reopen(&f);
            assert_eq!(
                reopened
                    .publication_status(&selected_a, [2; 16], revision)
                    .unwrap(),
                if point == PublicationPoint::FinalCommitted {
                    PublicationStatus::Committed(expected_target.clone())
                } else {
                    PublicationStatus::PendingNeedsRetry
                }
            );
            assert_eq!(
                reopened
                    .publish_candidate(&selected_a, [2; 16], &b)
                    .unwrap(),
                expected_target
            );
            assert_eq!(reopened.selection().unwrap(), expected_target);
            assert_eq!(
                reopened
                    .publish_candidate(&selected_a, [2; 16], &b)
                    .unwrap(),
                expected_target
            );
            assert_eq!(
                reopened
                    .owner
                    .connection()
                    .unwrap()
                    .query_row("SELECT count(*) FROM outcome", [], |r| r.get::<_, i64>(0))
                    .unwrap(),
                2
            );
        }
    }

    #[test]
    fn matching_selected_bytes_do_not_commit_another_pending_key_and_corrupt_intents_refuse() {
        let f = Fixture::new();
        let store = ProvisionalSourceStore::open(&f.directory(), PROJECT).unwrap();
        let a_stage = f.stage(100000);
        let a = validate_staged_candidate(&a_stage, &[]).unwrap();
        let selected_a = store
            .publish_candidate(&store.selection().unwrap(), [1; 16], &a)
            .unwrap();
        let b_stage = f.stage(75000);
        let b = validate_staged_candidate(&b_stage, &[]).unwrap();
        let revision = *b.provisional_manifest().unwrap().revision().as_bytes();
        store
            .owner
            .publication_fault
            .store(PublicationPoint::IntentRecorded as u8, Ordering::Relaxed);
        assert!(store.publish_candidate(&selected_a, [2; 16], &b).is_err());
        let selected_b = store.publish_candidate(&selected_a, [3; 16], &b).unwrap();
        assert_eq!(
            store
                .publication_status(&selected_a, [2; 16], revision)
                .unwrap(),
            PublicationStatus::PendingNeedsRetry
        );
        assert!(matches!(
            store.publish_candidate(&selected_a, [2; 16], &b),
            Err(StoreError::Refused("stale selection"))
        ));
        assert_eq!(
            store
                .publication_status(&selected_a, [3; 16], revision)
                .unwrap(),
            PublicationStatus::Committed(selected_b.clone())
        );
        assert!(matches!(
            store.publication_status(&selected_a, [3; 16], [0; 32]),
            Err(StoreError::Refused("provisional retry key conflict"))
        ));
        store
            .owner
            .connection()
            .unwrap()
            .execute(
                "UPDATE publication_intent SET base_generation=0 WHERE key=?1",
                params![[3u8; 16].as_slice()],
            )
            .unwrap();
        assert!(matches!(
            store.publication_status(&selected_a, [3; 16], revision),
            Err(StoreError::Corrupt("selected publication base"))
        ));
        store
            .owner
            .connection()
            .unwrap()
            .execute(
                "UPDATE publication_intent SET base_generation=1,committed=0 WHERE key=?1",
                params![[3u8; 16].as_slice()],
            )
            .unwrap();
        assert!(matches!(
            store.publication_status(&selected_a, [3; 16], revision),
            Err(StoreError::Corrupt("selected publication incomplete"))
        ));
        assert!(matches!(
            store.selection(),
            Err(StoreError::Corrupt("selected publication incomplete"))
        ));
        store
            .owner
            .connection()
            .unwrap()
            .execute(
                "UPDATE publication_intent SET committed=1 WHERE key=?1",
                params![[3u8; 16].as_slice()],
            )
            .unwrap();
        assert_eq!(store.selection().unwrap(), selected_b);
    }

    #[test]
    fn pending_intent_capacity_refuses_new_keys_but_preserves_exact_retry() {
        let f = Fixture::new();
        let store = ProvisionalSourceStore::open(&f.directory(), PROJECT).unwrap();
        let empty = store.selection().unwrap();
        let stage = f.stage(100000);
        let candidate = validate_staged_candidate(&stage, &[]).unwrap();
        let revision = *candidate
            .provisional_manifest()
            .unwrap()
            .revision()
            .as_bytes();
        store
            .owner
            .publication_fault
            .store(PublicationPoint::IntentRecorded as u8, Ordering::Relaxed);
        assert!(
            store
                .publish_candidate(&empty, [0; 16], &candidate)
                .is_err()
        );
        {
            let mut conn = store.owner.connection().unwrap();
            let tx = conn.transaction().unwrap();
            for index in 1..MAX_OWNERS {
                let key = (index as u128).to_be_bytes();
                let (target, request) =
                    publication_request(&store.owner, &empty, key, revision).unwrap();
                tx.execute(
                    "INSERT INTO publication_intent VALUES(?1,?2,0,NULL,?3,?4,0)",
                    params![
                        key.as_slice(),
                        request.as_slice(),
                        target.generation as i64,
                        revision.as_slice()
                    ],
                )
                .unwrap();
            }
            tx.commit().unwrap();
        }
        let new_key = (MAX_OWNERS as u128).to_be_bytes();
        assert!(matches!(
            store.publish_candidate(&empty, new_key, &candidate),
            Err(StoreError::Refused(
                "provisional publication intent limit; retirement unavailable"
            ))
        ));
        assert_eq!(
            store.publication_status(&empty, new_key, revision).unwrap(),
            PublicationStatus::Missing
        );
        assert_eq!(
            store.publication_status(&empty, [0; 16], revision).unwrap(),
            PublicationStatus::PendingNeedsRetry
        );
        assert_eq!(store.selection().unwrap(), empty);
        assert_eq!(blob_count(&store), 0);
        let selected = store
            .publish_candidate(&empty, [0; 16], &candidate)
            .unwrap();
        assert_eq!(selected.generation(), 1);
        assert_eq!(
            store.publication_status(&empty, [0; 16], revision).unwrap(),
            PublicationStatus::Committed(selected)
        );
    }

    #[test]
    fn selected_publication_key_and_v2_schema_are_verified_on_reopen_and_lease() {
        let f = Fixture::new();
        let store = ProvisionalSourceStore::open(&f.directory(), PROJECT).unwrap();
        let empty = store.selection().unwrap();
        let stage = f.stage(100000);
        let candidate = validate_staged_candidate(&stage, &[]).unwrap();
        let selected = store
            .publish_candidate(&empty, [1; 16], &candidate)
            .unwrap();
        let revision = selected.revision.unwrap();
        store
            .owner
            .connection()
            .unwrap()
            .execute(
                "UPDATE selection SET publication_key=?1 WHERE id=1",
                params![[9u8; 16].as_slice()],
            )
            .unwrap();
        assert!(matches!(
            store.selection(),
            Err(StoreError::Corrupt("selected publication intent missing"))
        ));
        assert!(matches!(
            store.lease_current(&selected),
            Err(StoreError::Corrupt("selected publication intent missing"))
        ));
        assert_eq!(pin_count(&store), 0);
        drop(store);
        assert!(matches!(
            open_released(&f, PROJECT),
            Err(StoreError::Corrupt("selected publication intent missing"))
        ));
        let conn = Connection::open(f.directory().join("provisional-source.sqlite")).unwrap();
        conn.execute(
            "UPDATE selection SET publication_key=?1 WHERE id=1",
            params![[1u8; 16].as_slice()],
        )
        .unwrap();
        drop(conn);
        let store = reopen(&f);
        assert_eq!(
            store.publication_status(&empty, [1; 16], revision).unwrap(),
            PublicationStatus::Committed(selected)
        );
        store
            .owner
            .connection()
            .unwrap()
            .execute_batch("DROP TABLE publication_intent")
            .unwrap();
        drop(store);
        assert!(matches!(
            open_released(&f, PROJECT),
            Err(StoreError::Sql(_))
        ));
    }

    #[test]
    fn version_one_store_refuses_without_migration_or_selection_change() {
        let f = Fixture::new();
        let store = ProvisionalSourceStore::open(&f.directory(), PROJECT).unwrap();
        let stage = f.stage(100000);
        let candidate = validate_staged_candidate(&stage, &[]).unwrap();
        let selected = store
            .publish_candidate(&store.selection().unwrap(), [1; 16], &candidate)
            .unwrap();
        store
            .owner
            .connection()
            .unwrap()
            .execute_batch("PRAGMA user_version=1")
            .unwrap();
        drop(store);
        assert!(matches!(
            open_released(&f, PROJECT),
            Err(StoreError::Refused("incompatible provisional store format"))
        ));
        let conn = Connection::open(f.directory().join("provisional-source.sqlite")).unwrap();
        assert_eq!(
            conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        let (generation, revision): (i64, Vec<u8>) = conn
            .query_row(
                "SELECT generation,revision FROM selection WHERE id=1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(generation, selected.generation as i64);
        assert_eq!(revision, selected.revision.unwrap());
    }

    #[test]
    fn publication_process_interrupt_child() {
        let Ok(root) = std::env::var("THRUM_PROVISIONAL_PUBLICATION_CHILD_ROOT") else {
            return;
        };
        let point: u8 = std::env::var("THRUM_PROVISIONAL_PUBLICATION_CHILD_POINT")
            .unwrap()
            .parse()
            .unwrap();
        let f = Fixture {
            root: PathBuf::from(root),
            cleanup: false,
        };
        let store = ProvisionalSourceStore::open(&f.directory(), PROJECT).unwrap();
        let base = store.selection().unwrap();
        let stage = f.stage(75000);
        let candidate = validate_staged_candidate(&stage, &[]).unwrap();
        store
            .owner
            .publication_pause
            .store(point, Ordering::Relaxed);
        store.publish_candidate(&base, [2; 16], &candidate).unwrap();
        panic!("child did not pause at selected publication boundary");
    }

    #[test]
    fn process_kill_before_inside_and_after_final_publication_recovers_by_exact_key() {
        for point in [
            PublicationPoint::IntentRecorded,
            PublicationPoint::BeforeFinal,
            PublicationPoint::FinalWritten,
            PublicationPoint::FinalCommitted,
        ] {
            let f = Fixture::new();
            let store = ProvisionalSourceStore::open(&f.directory(), PROJECT).unwrap();
            let a_stage = f.stage(100000);
            let a = validate_staged_candidate(&a_stage, &[]).unwrap();
            let selected_a = store
                .publish_candidate(&store.selection().unwrap(), [1; 16], &a)
                .unwrap();
            let b_stage = f.stage(75000);
            let b = validate_staged_candidate(&b_stage, &[]).unwrap();
            let revision = *b.provisional_manifest().unwrap().revision().as_bytes();
            let expected_target = ProvisionalSelection {
                generation: 2,
                revision: Some(revision),
                ..selected_a.clone()
            };
            drop(store);
            let mut child = pause_publication_child(&f, point);
            // The uncommitted final SQL writes remain invisible to another
            // connection even though the child has executed all three writes.
            if point == PublicationPoint::FinalWritten {
                let observer =
                    Connection::open(f.directory().join("provisional-source.sqlite")).unwrap();
                assert_eq!(
                    observer
                        .query_row("SELECT generation FROM selection WHERE id=1", [], |r| r
                            .get::<_, i64>(0))
                        .unwrap(),
                    1
                );
                assert_eq!(
                    observer
                        .query_row(
                            "SELECT count(*) FROM outcome WHERE key=?1",
                            params![[2u8; 16].as_slice()],
                            |r| r.get::<_, i64>(0)
                        )
                        .unwrap(),
                    0
                );
            }
            child.0.kill().unwrap();
            child.0.wait().unwrap();
            let reopened = reopen(&f);
            if point == PublicationPoint::FinalCommitted {
                assert_eq!(reopened.selection().unwrap(), expected_target);
                assert_eq!(
                    reopened
                        .publication_status(&selected_a, [2; 16], revision)
                        .unwrap(),
                    PublicationStatus::Committed(expected_target.clone())
                );
            } else {
                assert_eq!(reopened.selection().unwrap(), selected_a);
                assert_eq!(
                    reopened
                        .publication_status(&selected_a, [2; 16], revision)
                        .unwrap(),
                    PublicationStatus::PendingNeedsRetry
                );
            }
            assert_eq!(
                reopened
                    .publish_candidate(&selected_a, [2; 16], &b)
                    .unwrap(),
                expected_target
            );
            assert_eq!(reopened.selection().unwrap(), expected_target);
        }
    }

    #[test]
    fn public_token_recovery_process_child() {
        let Ok(root) = std::env::var("THRUM_PROVISIONAL_RECOVERY_CHILD_ROOT") else {
            return;
        };
        let root = PathBuf::from(root);
        // Only public source-store APIs and persisted bytes are used here.
        // No parent selection value, private constructor or candidate survives.
        let base =
            ProvisionalSelection::from_token_bytes(&fs::read(root.join("base.token")).unwrap())
                .unwrap();
        let key: [u8; 16] = fs::read(root.join("request.key"))
            .unwrap()
            .try_into()
            .unwrap();
        let target: [u8; 32] = fs::read(root.join("target.revision"))
            .unwrap()
            .try_into()
            .unwrap();
        let store = ProvisionalSourceStore::open(&root.join("store"), base.project_id()).unwrap();
        let PublicationStatus::Committed(selected) =
            store.publication_status(&base, key, target).unwrap()
        else {
            panic!("restart did not recover committed request");
        };
        assert_eq!(selected.generation(), base.generation() + 1);
        assert_eq!(selected.revision(), Some(&target));
        assert_eq!(store.selection().unwrap(), selected);
        let lease = store.lease_current(&selected).unwrap();
        let source: serde_json::Value =
            serde_json::from_slice(&lease.read_record(SOURCE).unwrap()).unwrap();
        assert_eq!(source["id"], SOURCE);
        assert_eq!(
            source["fields"]["sun"]["perpendicular_illuminance_lux"],
            75000
        );
        lease.close().unwrap();
        fs::write(root.join("recovered.token"), selected.to_token_bytes()).unwrap();
    }

    #[test]
    fn fresh_application_process_recovers_lost_ack_only_from_persisted_public_token() {
        use std::process::{Command, Stdio};
        let f = Fixture::new();
        {
            let store = ProvisionalSourceStore::open(&f.directory(), PROJECT).unwrap();
            let a_stage = f.stage(100000);
            let a = validate_staged_candidate(&a_stage, &[]).unwrap();
            let base = store
                .publish_candidate(&store.selection().unwrap(), [1; 16], &a)
                .unwrap();
            let b_stage = f.stage(75000);
            let b = validate_staged_candidate(&b_stage, &[]).unwrap();
            // The exact request token is persisted before the child publishes.
            for (name, bytes) in [
                ("base.token", base.to_token_bytes()),
                ("request.key", vec![2; 16]),
                (
                    "target.revision",
                    b.provisional_manifest()
                        .unwrap()
                        .revision()
                        .as_bytes()
                        .to_vec(),
                ),
            ] {
                let path = f.root.join(name);
                fs::write(&path, bytes).unwrap();
                OpenOptions::new()
                    .write(true)
                    .open(path)
                    .unwrap()
                    .sync_all()
                    .unwrap();
            }
        } // The original store, candidate and base selection are all dropped.
        let mut publisher = pause_publication_child(&f, PublicationPoint::FinalCommitted);
        publisher.0.kill().unwrap();
        publisher.0.wait().unwrap();
        fs::write(f.source().join(SOURCE_PATH), b"dirty checkout after commit").unwrap();
        let status = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "provisional_store::tests::public_token_recovery_process_child",
                "--nocapture",
            ])
            .env("THRUM_PROVISIONAL_RECOVERY_CHILD_ROOT", &f.root)
            .stdout(Stdio::null())
            .status()
            .unwrap();
        assert!(
            status.success(),
            "fresh recovery application failed: {status}"
        );
        let recovered = ProvisionalSelection::from_token_bytes(
            &fs::read(f.root.join("recovered.token")).unwrap(),
        )
        .unwrap();
        assert_eq!(recovered.generation(), 2);
        assert_eq!(
            recovered.revision().unwrap().as_slice(),
            fs::read(f.root.join("target.revision")).unwrap()
        );
    }

    #[test]
    fn valid_old_selected_row_cannot_rewind_committed_head_or_fork_publication() {
        let f = Fixture::new();
        let store = ProvisionalSourceStore::open(&f.directory(), PROJECT).unwrap();
        let empty = store.selection().unwrap();
        let a_stage = f.stage(100000);
        let a = validate_staged_candidate(&a_stage, &[]).unwrap();
        let selected_a = store.publish_candidate(&empty, [1; 16], &a).unwrap();
        let lease_a = store.lease_current(&selected_a).unwrap();
        let bytes_a = lease_a.read_record(SOURCE).unwrap();
        let b_stage = f.stage(75000);
        let b = validate_staged_candidate(&b_stage, &[]).unwrap();
        let selected_b = store.publish_candidate(&selected_a, [2; 16], &b).unwrap();
        let c_stage = f.stage(50000);
        let c = validate_staged_candidate(&c_stage, &[]).unwrap();
        let blobs = blob_count(&store);
        let commits = publication_transactions(&store);
        {
            let conn = store.owner.connection().unwrap();
            let plan: String = conn
                .query_row(
                    &format!("EXPLAIN QUERY PLAN {COMMITTED_HEAD_SQL}"),
                    [],
                    |r| r.get(3),
                )
                .unwrap();
            assert!(
                plan.contains("outcome_generation") && !plan.contains("TEMP B-TREE"),
                "{plan}"
            );
            conn.execute(
                "UPDATE selection SET generation=1,revision=?1,publication_key=?2 WHERE id=1",
                params![
                    selected_a.revision.unwrap().as_slice(),
                    [1u8; 16].as_slice()
                ],
            )
            .unwrap();
        }
        assert!(matches!(
            store.selection(),
            Err(StoreError::Corrupt(
                "selected generation is not committed high-water"
            ))
        ));
        assert!(matches!(
            store.lease_current(&selected_a),
            Err(StoreError::Corrupt(
                "selected generation is not committed high-water"
            ))
        ));
        assert!(matches!(
            store.publication_status(&empty, [1; 16], selected_a.revision.unwrap()),
            Err(StoreError::Corrupt(
                "selected generation is not committed high-water"
            ))
        ));
        assert!(matches!(
            store.publish_candidate(&selected_a, [3; 16], &c),
            Err(StoreError::Corrupt(
                "selected generation is not committed high-water"
            ))
        ));
        assert_eq!(blob_count(&store), blobs);
        assert_eq!(publication_transactions(&store), commits);
        assert_eq!(pin_count(&store), 1);
        assert_eq!(lease_a.read_record(SOURCE).unwrap(), bytes_a);
        assert_eq!(
            store
                .owner
                .connection()
                .unwrap()
                .query_row("SELECT count(*) FROM publication_intent", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            2
        );
        lease_a.close().unwrap();
        drop(store);
        assert!(matches!(
            open_released(&f, PROJECT),
            Err(StoreError::Corrupt(
                "selected generation is not committed high-water"
            ))
        ));
        let conn = Connection::open(f.directory().join("provisional-source.sqlite")).unwrap();
        assert_eq!(
            conn.query_row("SELECT generation FROM selection", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        conn.execute(
            "UPDATE selection SET generation=2,revision=?1,publication_key=?2 WHERE id=1",
            params![
                selected_b.revision.unwrap().as_slice(),
                [2u8; 16].as_slice()
            ],
        )
        .unwrap();
        drop(conn);
        let reopened = reopen(&f);
        let selected_c = reopened
            .publish_candidate(&selected_b, [4; 16], &c)
            .unwrap();
        assert_eq!(selected_c.generation(), 3);
        assert_eq!(
            reopened
                .owner
                .connection()
                .unwrap()
                .query_row("SELECT count(*) FROM outcome WHERE generation=2", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }

    #[test]
    fn open_checks_retained_generation_chain_once_and_refuses_gaps_or_duplicates() {
        let f = Fixture::new();
        let store = ProvisionalSourceStore::open(&f.directory(), PROJECT).unwrap();
        let a_stage = f.stage(100000);
        let a = validate_staged_candidate(&a_stage, &[]).unwrap();
        let selected_a = store
            .publish_candidate(&store.selection().unwrap(), [1; 16], &a)
            .unwrap();
        let b_stage = f.stage(75000);
        let b = validate_staged_candidate(&b_stage, &[]).unwrap();
        let selected_b = store.publish_candidate(&selected_a, [2; 16], &b).unwrap();
        store
            .owner
            .connection()
            .unwrap()
            .execute(
                "UPDATE outcome SET generation=2 WHERE key=?1",
                params![[1u8; 16].as_slice()],
            )
            .unwrap();
        // The indexed live-head check is intentionally not a full historical
        // audit; reopen performs the bounded contiguous-history validation.
        assert_eq!(store.selection().unwrap(), selected_b);
        drop(store);
        assert!(matches!(
            open_released(&f, PROJECT),
            Err(StoreError::Corrupt("committed generation chain"))
        ));
    }

    #[test]
    fn selection_tokens_are_fixed_canonical_checked_and_still_bound_to_the_store() {
        let f = Fixture::new();
        let store = ProvisionalSourceStore::open(&f.directory(), PROJECT).unwrap();
        let empty = store.selection().unwrap();
        let token = empty.to_token_bytes();
        assert_eq!(token.len(), SELECTION_TOKEN_BYTES);
        assert_eq!(
            ProvisionalSelection::from_token_bytes(&token).unwrap(),
            empty
        );
        for bytes in [&token[..token.len() - 1], &token[..0]] {
            assert!(matches!(
                ProvisionalSelection::from_token_bytes(bytes),
                Err(StoreError::Refused("selection token format"))
            ));
        }
        let mut extended = token.clone();
        extended.push(0);
        assert!(ProvisionalSelection::from_token_bytes(&extended).is_err());
        let mut damaged = token.clone();
        damaged[SELECTION_TOKEN_MAGIC.len()] ^= 1;
        assert!(matches!(
            ProvisionalSelection::from_token_bytes(&damaged),
            Err(StoreError::Refused("selection token checksum"))
        ));
        let repair_checksum = |bytes: &mut Vec<u8>| {
            let end = bytes.len() - 32;
            let checksum = Sha256::digest(&bytes[..end]);
            bytes[end..].copy_from_slice(&checksum);
        };
        let mut padded = token.clone();
        padded[SELECTION_TOKEN_MAGIC.len() + 61] = 1;
        repair_checksum(&mut padded);
        assert!(matches!(
            ProvisionalSelection::from_token_bytes(&padded),
            Err(StoreError::Refused("selection token revision"))
        ));
        let mut bad_flag = token.clone();
        bad_flag[SELECTION_TOKEN_MAGIC.len() + 60] = 2;
        repair_checksum(&mut bad_flag);
        assert!(matches!(
            ProvisionalSelection::from_token_bytes(&bad_flag),
            Err(StoreError::Refused("selection token revision"))
        ));
        let mut inconsistent = token.clone();
        inconsistent[SELECTION_TOKEN_MAGIC.len() + 60] = 1;
        repair_checksum(&mut inconsistent);
        assert!(matches!(
            ProvisionalSelection::from_token_bytes(&inconsistent),
            Err(StoreError::Refused("selection token selection invariant"))
        ));
        repair_checksum(&mut damaged);
        let decoded = ProvisionalSelection::from_token_bytes(&damaged).unwrap();
        assert!(matches!(
            store.publication_status(&decoded, [1; 16], [1; 32]),
            Err(StoreError::Refused(
                "selection belongs to another store/project"
            ))
        ));
        let other = Fixture::new();
        let other_store = ProvisionalSourceStore::open(&other.directory(), PROJECT).unwrap();
        assert!(matches!(
            other_store.publication_status(
                &ProvisionalSelection::from_token_bytes(&token).unwrap(),
                [1; 16],
                [1; 32]
            ),
            Err(StoreError::Refused(
                "selection belongs to another store/project"
            ))
        ));
    }

    #[test]
    fn foreign_database_and_orphan_sidecars_are_not_initialized() {
        let f = Fixture::new();
        let path = f.directory().join("provisional-source.sqlite");
        let foreign = Connection::open(&path).unwrap();
        foreign
            .execute_batch("CREATE TABLE unrelated(value TEXT)")
            .unwrap();
        drop(foreign);
        assert!(matches!(
            ProvisionalSourceStore::open(&f.directory(), PROJECT),
            Err(StoreError::Refused("incompatible provisional store format"))
        ));
        let other = Fixture::new();
        let wal = other.directory().join("provisional-source.sqlite-wal");
        fs::write(&wal, b"unrelated sidecar").unwrap();
        assert!(matches!(
            ProvisionalSourceStore::open(&other.directory(), PROJECT),
            Err(StoreError::Refused(
                "orphan SQLite sidecar beside an empty store"
            ))
        ));
        assert_eq!(fs::read(wal).unwrap(), b"unrelated sidecar");
        assert!(!other.directory().join("provisional-source.sqlite").exists());
        let file = other.root.join("not-a-directory");
        fs::write(&file, b"ordinary file").unwrap();
        assert!(matches!(
            ProvisionalSourceStore::open(&file, PROJECT),
            Err(StoreError::Refused("store directory"))
        ));
        #[cfg(unix)]
        {
            let alias = other.root.join("store-alias");
            std::os::unix::fs::symlink(other.directory(), &alias).unwrap();
            assert!(matches!(
                ProvisionalSourceStore::open(&alias, PROJECT),
                Err(StoreError::Refused("store directory"))
            ));
        }
    }
}
