//! Schema responsibilities for the native source store.

use super::{
    BudgetError, Connection, DbConfig, OpenFlags, Path, SOURCE_STORE_FORMAT, SQLITE_PAGE_BYTES,
    SourceSpacePlan, SqliteStoreCore, StoreError, WalGuard, fs, io, physical_io,
};

impl SqliteStoreCore {
    pub(super) fn open_with_plan_and_vfs(
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
                        || (page_size
                            == i64::try_from(SQLITE_PAGE_BYTES).expect("fixed page size fits i64")
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
            if page_size != i64::try_from(SQLITE_PAGE_BYTES).expect("fixed page size fits i64")
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
            path: path.to_owned(),
            wal_guard,
            pending_pin_releases: Vec::new(),
            active_free_floor: None,
        })
    }
}
