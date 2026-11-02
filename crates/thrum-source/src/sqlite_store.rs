//! Isolated SQLite transaction proof for the later native source store.
//!
//! Test-only: the complete blob in this fixture is not a parsed manifest or a
//! full source closure, so this is not an accepted-source admission API.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use sha2::{Digest, Sha256};

use crate::{ManifestEntry, SourcePath, TreeManifest};

const MAX_PAGE_COUNT: i64 = 13_631_488;
const CHUNK_BYTES: usize = 4 * 1024 * 1024;
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
    InputIo,
    InputLength,
    HashMismatch,
    MissingChunk,
    InvalidChunkLayout,
    MissingCompleteBlob,
    BlobMetadataMismatch,
    StaleBlobWriter,
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
        Ok(Self { conn })
    }

    fn root(&self) -> rusqlite::Result<(i64, Option<Vec<u8>>)> {
        self.conn.query_row(
            "SELECT generation, revision FROM root WHERE id=1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
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

    fn stage_blob_with(
        &mut self,
        input: &mut impl Read,
        length: u64,
        expected_hash: [u8; 32],
        mut after_chunk: impl FnMut(i64),
    ) -> Result<(), ProofError> {
        let chunk_count = length.div_ceil(CHUNK_BYTES as u64);
        let chunk_count = i64::try_from(chunk_count).map_err(|_| ProofError::InputLength)?;
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
                return Ok(());
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
        let mut buffer = vec![0_u8; CHUNK_BYTES];
        let mut digest = Sha256::new();
        let mut remaining = length;
        for index in 0..chunk_count {
            let count = usize::try_from(remaining.min(CHUNK_BYTES as u64))
                .map_err(|_| ProofError::InputLength)?;
            input
                .read_exact(&mut buffer[..count])
                .map_err(|_| ProofError::InputIo)?;
            digest.update(&buffer[..count]);
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
                    &buffer[..count],
                    sql_length,
                    chunk_count
                ],
            )?;
            if inserted != 1 {
                return Err(ProofError::StaleBlobWriter);
            }
            tx.commit()?;
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
        let mut stored = Sha256::new();
        let mut stored_length = 0_u64;
        for index in 0..chunk_count {
            let bytes: Vec<u8> = tx
                .query_row(
                    "SELECT bytes FROM blob_chunks WHERE hash=?1 AND chunk_index=?2",
                    params![expected_hash.as_slice(), index],
                    |row| row.get(0),
                )
                .optional()?
                .ok_or(ProofError::MissingChunk)?;
            let expected_count = usize::try_from((length - stored_length).min(CHUNK_BYTES as u64))
                .map_err(|_| ProofError::InputLength)?;
            if bytes.len() != expected_count {
                return Err(ProofError::InvalidChunkLayout);
            }
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
    assert_eq!(repeated_input.len(), bytes.len());

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
    assert_eq!(second_input.len(), bytes.len());
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
