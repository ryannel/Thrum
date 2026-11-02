//! Isolated SQLite transaction proof for the later native source store.
//!
//! Test-only: these hashes have no verified blob closure, so this module must
//! not be used as an accepted-source admission API.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

const MAX_PAGE_COUNT: i64 = 13_631_488;
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
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[derive(Debug)]
enum ProofError {
    Sql(rusqlite::Error),
    StaleGeneration,
    TokenConflict,
}

impl From<rusqlite::Error> for ProofError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sql(value)
    }
}

struct RootTransactionProof {
    conn: Connection,
}

impl RootTransactionProof {
    fn open(path: &Path) -> Result<Self, ProofError> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA page_size=4096;
             PRAGMA auto_vacuum=INCREMENTAL;
             PRAGMA journal_mode=WAL;
             PRAGMA synchronous=FULL;
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
             INSERT OR IGNORE INTO root (id, generation, revision) VALUES (1, 0, NULL);",
        )?;
        let page_size: i64 = conn.query_row("PRAGMA page_size", [], |row| row.get(0))?;
        let auto_vacuum: i64 = conn.query_row("PRAGMA auto_vacuum", [], |row| row.get(0))?;
        let journal_mode: String = conn.query_row("PRAGMA journal_mode", [], |row| row.get(0))?;
        let synchronous: i64 = conn.query_row("PRAGMA synchronous", [], |row| row.get(0))?;
        let max_page_count: i64 = conn.query_row("PRAGMA max_page_count", [], |row| row.get(0))?;
        assert_eq!(page_size, 4096);
        assert_eq!(auto_vacuum, 2);
        assert_eq!(journal_mode, "wal");
        assert_eq!(synchronous, 2);
        assert_eq!(max_page_count, MAX_PAGE_COUNT);
        Ok(Self { conn })
    }

    fn root(&self) -> rusqlite::Result<(i64, Option<Vec<u8>>)> {
        self.conn.query_row(
            "SELECT generation, revision FROM root WHERE id=1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
    }

    fn commit_probe(
        &mut self,
        expected_generation: i64,
        epoch: i64,
        key: i64,
        request_hash: [u8; 32],
        revision: [u8; 32],
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
    let revision = [22_u8; 32];
    assert_eq!(store.commit_probe(0, 1, 1, request, revision).unwrap(), 1);
    assert_eq!(store.root().unwrap(), (1, Some(revision.to_vec())));
    drop(store);

    let mut reopened = RootTransactionProof::open(&fixture.database()).unwrap();
    assert_eq!(reopened.root().unwrap(), (1, Some(revision.to_vec())));
    assert_eq!(
        reopened.commit_probe(0, 1, 1, request, revision).unwrap(),
        1
    );
    assert!(matches!(
        reopened.commit_probe(0, 1, 1, [33; 32], revision),
        Err(ProofError::TokenConflict)
    ));
    assert!(matches!(
        reopened.commit_probe(0, 1, 2, request, [44; 32]),
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
    match reopened.commit_probe(1, 1, 2, request, [44; 32]) {
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
        reopened.commit_probe(1, 1, 2, request, [44; 32]).unwrap(),
        2
    );
    drop(reopened);
    let final_store = RootTransactionProof::open(&fixture.database()).unwrap();
    assert_eq!(final_store.root().unwrap(), (2, Some(vec![44; 32])));
}
