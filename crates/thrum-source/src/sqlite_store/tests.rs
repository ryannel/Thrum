//! Native source-store integration and fault tests.

use super::*;
const LINEAGE_PROJECT_ID: &str = "2a5ef644-7225-4770-9f4d-e1d9c0a7d515";
const LINEAGE_HISTORY_ID: &str = "82b0ec04-6866-4271-a2cf-1f1576ad604a";
use crate::provisional_store_vfs_test as checkpoint_vfs;
use crate::registry_stage::validate_staged_candidate;
use crate::stage::stage_declared_tree_with;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

#[cfg(test)]
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

struct ClosureFault<A, B, C> {
    after_chunk: A,
    before_commit: Option<B>,
    after_sql: Option<C>,
}

impl<F: FnMut()> AdmissionFault for F {
    fn after_committed_chunk(&mut self) {
        self();
    }
}

impl<A: FnMut(), B: FnOnce(), C: FnOnce()> AdmissionFault for ClosureFault<A, B, C> {
    fn after_committed_chunk(&mut self) {
        (self.after_chunk)();
    }

    fn before_final_commit(&mut self) {
        if let Some(before) = self.before_commit.take() {
            before();
        }
    }

    fn after_final_sql(&mut self) {
        if let Some(after) = self.after_sql.take() {
            after();
        }
    }
}

impl NativeSourceStore {
    fn admit_with_fault_hooks(
        &self,
        expected_generation: i64,
        epoch: i64,
        key: i64,
        request_hash: [u8; 32],
        candidate: &StagedCandidate<'_>,
        after_chunk: impl FnMut(),
        before_commit: impl FnOnce(),
    ) -> Result<AcceptedSourceRoot, SourceStoreError> {
        self.admit_with_fault(
            expected_generation,
            epoch,
            key,
            request_hash,
            candidate,
            &mut ClosureFault {
                after_chunk,
                before_commit: Some(before_commit),
                after_sql: None::<fn()>,
            },
        )
    }
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
        assert_eq!(copied, u64::try_from(bytes.len()).unwrap());
        let mut staged_bytes = Vec::new();
        stage
            .open(&source_path)
            .unwrap()
            .read_to_end(&mut staged_bytes)
            .unwrap();
        assert_eq!(bytes, staged_bytes);
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
fn native_admission_refuses_dropped_and_corrupt_predecessor_history() {
    let fixture = Fixture::new();
    let history_path = fixture.source_root().join("schemas/history.registry.json");
    minimal_source_stage(&fixture, "prior history");
    fs::write(&history_path, lineage_history_with_field()).unwrap();
    let prior_stage = stage_existing_source(&fixture);
    let prior = validate_staged_candidate(&prior_stage, &[]).unwrap();
    let store = NativeSourceStore::open(&fixture.database(), SourceSpacePlan::DEFAULT).unwrap();
    let root = store.admit(0, 1, 1, [1; 32], &prior).unwrap();

    let next_stage = minimal_source_stage(&fixture, "history dropped");
    let next = validate_staged_candidate(&next_stage, &[]).unwrap();
    assert_eq!(
        store
            .admit(root.generation, 1, 2, [2; 32], &next)
            .unwrap_err()
            .kind(),
        SourceStoreErrorKind::Candidate
    );
    assert_eq!(store.current_root().unwrap(), Some(root));
    assert_eq!(store.outcome(1, 2, [2; 32]).unwrap(), None);

    let prior_history = prior_stage
        .files()
        .iter()
        .find(|file| file.path().as_str() == "schemas/history.registry.json")
        .unwrap();
    store
        .owner
        .lock()
        .unwrap()
        .conn
        .execute(
            "UPDATE blob_chunks SET bytes=?1 WHERE hash=?2 AND chunk_index=0",
            params![b"corrupt".as_slice(), prior_history.sha256().as_slice()],
        )
        .unwrap();
    assert_eq!(
        store
            .admit(root.generation, 1, 2, [2; 32], &next)
            .unwrap_err()
            .kind(),
        SourceStoreErrorKind::CorruptStore
    );
    assert_eq!(store.current_root().unwrap(), Some(root));
    assert_eq!(store.outcome(1, 2, [2; 32]).unwrap(), None);
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
    let store_path = store.store_path().to_owned();
    assert_eq!(pinned.store_path(), store_path);
    let path = SourcePath::parse("project.json").unwrap();
    let expected = fs::read(fixture.source_root().join(path.as_str())).unwrap();
    drop(store);
    assert_eq!(pinned.store_path(), store_path);
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
fn identical_accepted_bytes_from_distinct_stores_have_distinct_pin_bindings() {
    let left = Fixture::new();
    let right = Fixture::new();
    let plan = SourceSpacePlan {
        free_headroom_bytes: 0,
        ..SourceSpacePlan::DEFAULT
    };
    let left_stage = minimal_source_stage(&left, "same source");
    let right_stage = minimal_source_stage(&right, "same source");
    let left_candidate = validate_staged_candidate(&left_stage, &[]).unwrap();
    let right_candidate = validate_staged_candidate(&right_stage, &[]).unwrap();
    let left_store = NativeSourceStore::open(&left.database(), plan).unwrap();
    let right_store = NativeSourceStore::open(&right.database(), plan).unwrap();
    let left_root = left_store.admit(0, 1, 1, [1; 32], &left_candidate).unwrap();
    let right_root = right_store
        .admit(0, 1, 1, [1; 32], &right_candidate)
        .unwrap();
    assert_eq!(left_root, right_root);
    let left_pin = left_store.pin_current(1).unwrap();
    let right_pin = right_store.pin_current(1).unwrap();
    assert_eq!(left_pin.store_path(), left_store.store_path());
    assert_eq!(right_pin.store_path(), right_store.store_path());
    assert_ne!(left_pin.store_path(), right_pin.store_path());
    left_pin.close().unwrap();
    right_pin.close().unwrap();
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
        .arg("sqlite_store::tests::native_owner_lock_child_holds_until_killed")
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
    let _retained_fixture = std::mem::ManuallyDrop::new(fixture);
    let candidate = validate_staged_candidate(&stage, &[]).unwrap();
    let plan = SourceSpacePlan {
        free_headroom_bytes: 0,
        ..SourceSpacePlan::DEFAULT
    };
    let store = NativeSourceStore::open(&database, plan).unwrap();
    store
        .admit_with_fault_hooks(
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
        .arg("sqlite_store::tests::accepted_kill_after_verified_stage_child")
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
    let admission = store.owner.admission().unwrap();
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
    drop(admission);
    a_pin.close().unwrap();
}

#[test]
fn accepted_kill_mid_blob_child() {
    let Some(directory) = std::env::var_os("THRUM_TEST_ACCEPTED_MID_BLOB_KILL_DIR") else {
        return;
    };
    let marker =
        PathBuf::from(std::env::var_os("THRUM_TEST_ACCEPTED_MID_BLOB_KILL_MARKER").unwrap());
    let fixture = std::mem::ManuallyDrop::new(Fixture(PathBuf::from(directory)));
    let database = fixture.database();
    let plan = SourceSpacePlan {
        wal_stop_bytes: 40 * MIB,
        free_headroom_bytes: 0,
        ..SourceSpacePlan::DEFAULT
    };
    let first_stage = minimal_source_stage(&fixture, "A retained during B retry");
    let stage_b = stage_n_mib_binary(&fixture, &first_stage, plan, 68, 31);
    let candidate_b = validate_staged_candidate(&stage_b, &[]).unwrap();
    let store = NativeSourceStore::open(&database, plan).unwrap();
    let mut chunks = 0;
    store
        .admit_with_fault_hooks(
            1,
            1,
            2,
            [2; 32],
            &candidate_b,
            || {
                chunks += 1;
                if chunks == 16 {
                    signal_and_wait_for_kill(&marker);
                }
            },
            || {},
        )
        .unwrap();
    panic!("child passed the mid-blob kill point");
}

#[test]
fn accepted_owner_kill_mid_blob_reopens_a_pin_and_retries_b() {
    let fixture = Fixture::new();
    let database = fixture.database();
    let plan = SourceSpacePlan {
        wal_stop_bytes: 40 * MIB,
        free_headroom_bytes: 0,
        ..SourceSpacePlan::DEFAULT
    };
    let store = NativeSourceStore::open(&database, plan).unwrap();
    let stage_a = minimal_source_stage(&fixture, "A retained during B retry");
    let candidate_a = validate_staged_candidate(&stage_a, &[]).unwrap();
    let a = store.admit(0, 1, 1, [1; 32], &candidate_a).unwrap();
    let scene = SourcePath::parse("scenes/main.scene.json").unwrap();
    let old_scene = fs::read(fixture.source_root().join(scene.as_str())).unwrap();
    drop(store);

    let marker = fixture.0.join("accepted-mid-blob.ready");
    let child = Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("sqlite_store::tests::accepted_kill_mid_blob_child")
        .arg("--nocapture")
        .env("THRUM_TEST_ACCEPTED_MID_BLOB_KILL_DIR", &fixture.0)
        .env("THRUM_TEST_ACCEPTED_MID_BLOB_KILL_MARKER", &marker)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    kill_after_marker_with_deadline(child, &marker, Duration::from_secs(30), || {
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
    let mut a_pin = store.pin_current(a.generation).unwrap();
    let mut copied = Vec::new();
    a_pin.copy_file_to(&scene, &mut copied).unwrap();
    assert_eq!(copied, old_scene);
    let stage_b = stage_n_mib_binary(&fixture, &stage_a, plan, 68, 31);
    let master = SourcePath::parse("master.bin").unwrap();
    let hash = *stage_b
        .files()
        .iter()
        .find(|file| file.path() == &master)
        .unwrap()
        .sha256();
    let (state, retained): (i64, i64) = store
        .owner
        .lock()
        .unwrap()
        .conn
        .query_row(
            "SELECT state, (SELECT count(*) FROM blob_chunks WHERE hash=?1)
             FROM blob_header WHERE hash=?1",
            params![hash.as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!((state, retained), (0, 16));
    store
        .owner
        .lock()
        .unwrap()
        .conn
        .execute_batch(
            "CREATE TRIGGER no_partial_delete BEFORE DELETE ON blob_chunks
             BEGIN SELECT RAISE(ABORT, 'killed prefix must be reused'); END;
             CREATE TRIGGER no_partial_update BEFORE UPDATE ON blob_chunks
             BEGIN SELECT RAISE(ABORT, 'killed prefix must not be rewritten'); END;",
        )
        .unwrap();
    let candidate_b = validate_staged_candidate(&stage_b, &[]).unwrap();
    let b = store
        .admit_with_fault_hooks(
            a.generation,
            1,
            2,
            [2; 32],
            &candidate_b,
            || {
                copied.clear();
                a_pin.copy_file_to(&scene, &mut copied).unwrap();
                assert_eq!(copied, old_scene);
            },
            || {},
        )
        .unwrap();
    assert_eq!(b.generation, a.generation + 1);
    assert_eq!(store.current_root().unwrap(), Some(b));
    assert_eq!(store.outcome(1, 2, [2; 32]).unwrap(), Some(b));
    assert_eq!(
        store
            .admit(a.generation, 1, 2, [2; 32], &candidate_b)
            .unwrap(),
        b
    );
    let mut b_pin = store.pin_current(b.generation).unwrap();
    assert_eq!(
        b_pin.copy_file_to(&master, &mut io::sink()).unwrap(),
        68 * MIB
    );
    b_pin.close().unwrap();
    a_pin.close().unwrap();
    drop(store);
    let reopened = NativeSourceStore::open(&database, plan).unwrap();
    assert_eq!(reopened.current_root().unwrap(), Some(b));
    assert_eq!(reopened.outcome(1, 2, [2; 32]).unwrap(), Some(b));
}

#[test]
fn accepted_partial_retry_defers_behind_reader_blocked_wal() {
    let fixture = Fixture::new();
    let database = fixture.database();
    let plan = SourceSpacePlan {
        wal_stop_bytes: 40 * MIB,
        free_headroom_bytes: 0,
        ..SourceSpacePlan::DEFAULT
    };
    let mut store = NativeSourceStore::open(&database, plan).unwrap();
    let stage_a = minimal_source_stage(&fixture, "A retained during blocked B retry");
    let candidate_a = validate_staged_candidate(&stage_a, &[]).unwrap();
    let a = store.admit(0, 1, 1, [1; 32], &candidate_a).unwrap();
    let mut a_pin = store.pin_current(a.generation).unwrap();
    let scene = SourcePath::parse("scenes/main.scene.json").unwrap();
    let old_scene = fs::read(fixture.source_root().join(scene.as_str())).unwrap();
    let stage_b = stage_n_mib_binary(&fixture, &stage_a, plan, 68, 41);
    let master = SourcePath::parse("master.bin").unwrap();
    let master_file = stage_b
        .files()
        .iter()
        .find(|file| file.path() == &master)
        .unwrap();
    let candidate_b = validate_staged_candidate(&stage_b, &[]).unwrap();
    let mut scratch = Vec::new();
    let admission = store.owner.admission().unwrap();
    let mut short_input = stage_b.open(&master).unwrap().take(16 * CHUNK_BYTES as u64);
    assert!(matches!(
        store.stage_blob_phased(
            &mut short_input,
            master_file.length(),
            *master_file.sha256(),
            &mut scratch,
            &mut || {},
        ),
        Err(StoreError::InputIo)
    ));
    drop(admission);
    let wal_before = fs::metadata(wal_path(&database)).unwrap().len();
    assert!(wal_before > 0 && wal_before < plan.wal_stop_bytes);
    // Select the operational stop after seeding the retained prefix.
    // It exactly reaches the next four-MiB chunk allowance, so that the
    // blocked checkpoint is exercised regardless of prior checkpoint timing.
    let retry_stop = wal_before + 9 * MIB;
    store.plan.wal_stop_bytes = retry_stop;
    store.owner.lock().unwrap().wal_guard = WalGuard::for_plan(store.plan).unwrap();

    let reader = Connection::open(&database).unwrap();
    reader.execute_batch("BEGIN").unwrap();
    assert_eq!(
        reader
            .query_row("SELECT generation FROM root WHERE id=1", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        a.generation
    );
    let mut old_reads = 0;
    let blocked = store
        .admit_with_fault_hooks(
            a.generation,
            1,
            2,
            [2; 32],
            &candidate_b,
            || {
                let mut copied = Vec::new();
                a_pin.copy_file_to(&scene, &mut copied).unwrap();
                assert_eq!(copied, old_scene);
                old_reads += 1;
            },
            || {},
        )
        .unwrap_err();
    assert_eq!(old_reads, 16);
    let Some(StoreSpacePressure::WalCheckpoint {
        observed_wal_bytes,
        incoming_allowance_bytes,
        stop_bytes,
        checkpoint_busy: true,
    }) = blocked.space_pressure()
    else {
        panic!("expected reader-blocked WAL checkpoint: {blocked}");
    };
    assert_eq!(stop_bytes, retry_stop);
    assert_eq!(incoming_allowance_bytes, 9 * MIB);
    assert!(observed_wal_bytes + incoming_allowance_bytes >= stop_bytes);
    assert_eq!(store.current_root().unwrap(), Some(a));
    assert_eq!(store.outcome(1, 2, [2; 32]).unwrap(), None);
    let (state, retained): (i64, i64) = store
        .owner
        .lock()
        .unwrap()
        .conn
        .query_row(
            "SELECT state, (SELECT count(*) FROM blob_chunks WHERE hash=?1)
             FROM blob_header WHERE hash=?1",
            params![master_file.sha256().as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!((state, retained), (0, 16));
    reader.execute_batch("COMMIT").unwrap();
    let b = store
        .admit(a.generation, 1, 2, [2; 32], &candidate_b)
        .unwrap();
    assert_eq!(b.generation, a.generation + 1);
    assert_eq!(store.current_root().unwrap(), Some(b));
    assert_eq!(store.outcome(1, 2, [2; 32]).unwrap(), Some(b));
    let mut copied = Vec::new();
    a_pin.copy_file_to(&scene, &mut copied).unwrap();
    assert_eq!(copied, old_scene);
    a_pin.close().unwrap();
}

#[test]
fn accepted_malformed_partial_cleanup_resumes_after_sql_failure() {
    let fixture = Fixture::new();
    let database = fixture.database();
    let plan = SourceSpacePlan {
        wal_stop_bytes: 40 * MIB,
        free_headroom_bytes: 0,
        ..SourceSpacePlan::DEFAULT
    };
    let store = NativeSourceStore::open(&database, plan).unwrap();
    let stage_a = minimal_source_stage(&fixture, "A retained during B cleanup");
    let candidate_a = validate_staged_candidate(&stage_a, &[]).unwrap();
    let a = store.admit(0, 1, 1, [1; 32], &candidate_a).unwrap();
    let scene = SourcePath::parse("scenes/main.scene.json").unwrap();
    let old_scene = fs::read(fixture.source_root().join(scene.as_str())).unwrap();
    let mut a_pin = store.pin_current(a.generation).unwrap();
    let stage_b = stage_n_mib_binary(&fixture, &stage_a, plan, 68, 42);
    let master = SourcePath::parse("master.bin").unwrap();
    let master_file = stage_b
        .files()
        .iter()
        .find(|file| file.path() == &master)
        .unwrap();
    let hash = *master_file.sha256();
    let candidate_b = validate_staged_candidate(&stage_b, &[]).unwrap();
    let mut scratch = Vec::new();
    let admission = store.owner.admission().unwrap();
    let mut short_input = stage_b.open(&master).unwrap().take(16 * CHUNK_BYTES as u64);
    assert!(matches!(
        store.stage_blob_phased(
            &mut short_input,
            master_file.length(),
            hash,
            &mut scratch,
            &mut || {},
        ),
        Err(StoreError::InputIo)
    ));
    drop(admission);
    {
        let inner = store.owner.lock().unwrap();
        inner
            .conn
            .execute(
                "DELETE FROM blob_chunks WHERE hash=?1 AND chunk_index=0",
                params![hash.as_slice()],
            )
            .unwrap();
        inner
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
    }
    let failed = store
        .admit(a.generation, 1, 2, [2; 32], &candidate_b)
        .unwrap_err();
    assert_eq!(failed.kind(), SourceStoreErrorKind::Storage);
    assert_eq!(store.current_root().unwrap(), Some(a));
    assert_eq!(store.outcome(1, 2, [2; 32]).unwrap(), None);
    let (state, remaining): (i64, i64) = store
        .owner
        .lock()
        .unwrap()
        .conn
        .query_row(
            "SELECT state, (SELECT count(*) FROM blob_chunks WHERE hash=?1)
             FROM blob_header WHERE hash=?1",
            params![hash.as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!((state, remaining), (0, 14));
    let mut copied = Vec::new();
    a_pin.copy_file_to(&scene, &mut copied).unwrap();
    assert_eq!(copied, old_scene);
    a_pin.close().unwrap();
    drop(store);

    let reopened = NativeSourceStore::open(&database, plan).unwrap();
    assert_eq!(reopened.current_root().unwrap(), Some(a));
    assert_eq!(reopened.outcome(1, 2, [2; 32]).unwrap(), None);
    let mut a_pin = reopened.pin_current(a.generation).unwrap();
    reopened
        .owner
        .lock()
        .unwrap()
        .conn
        .execute_batch("DROP TRIGGER fail_second_cleanup")
        .unwrap();
    let b = reopened
        .admit_with_fault_hooks(
            a.generation,
            1,
            2,
            [2; 32],
            &candidate_b,
            || {
                copied.clear();
                a_pin.copy_file_to(&scene, &mut copied).unwrap();
                assert_eq!(copied, old_scene);
            },
            || {},
        )
        .unwrap();
    assert_eq!(b.generation, a.generation + 1);
    assert_eq!(reopened.current_root().unwrap(), Some(b));
    assert_eq!(reopened.outcome(1, 2, [2; 32]).unwrap(), Some(b));
    let mut b_pin = reopened.pin_current(b.generation).unwrap();
    assert_eq!(
        b_pin.copy_file_to(&master, &mut io::sink()).unwrap(),
        68 * MIB
    );
    b_pin.close().unwrap();
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
            .admit_with_fault_hooks(
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
        .admit_with_fault_hooks(
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
    let _retained_fixture = std::mem::ManuallyDrop::new(fixture);
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
        .arg("sqlite_store::tests::accepted_kill_after_public_admit_child")
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
    let fault = store
        .admit_with_fault_hooks(
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
        let commit = store.admit_with_fault(
            a.generation,
            1,
            2,
            [2; 32],
            &candidate_b,
            &mut ClosureFault {
                after_chunk: || {},
                before_commit: None::<fn()>,
                after_sql: Some(|| {
                    observer.arm_wal_fault(checkpoint_vfs::WalFault::AfterCommitSync);
                    go_tx.send(()).unwrap();
                    started_rx.recv_timeout(Duration::from_secs(2)).unwrap();
                    std::thread::sleep(Duration::from_millis(10));
                }),
            },
        );
        (commit, reader.join().unwrap())
    });
    assert_eq!(
        commit.unwrap_err().kind(),
        SourceStoreErrorKind::RecoveryRequired
    );
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
    fs::write(
        root.join("master.bin"),
        vec![7_u8; usize::try_from(MIB).unwrap()],
    )
    .unwrap();
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
    assert_eq!(
        store
            .admit_with_fault_hooks(
                1,
                1,
                2,
                [2; 32],
                &second,
                || {},
                || {
                    let inner = store.owner.lock().unwrap();
                    let current_pages: i64 = inner
                        .conn
                        .query_row("PRAGMA page_count", [], |row| row.get(0))
                        .unwrap();
                    inner
                        .conn
                        .pragma_update(None, "max_page_count", current_pages)
                        .unwrap();
                }
            )
            .unwrap_err()
            .kind(),
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
            // A killed owner can leave one full provisional copy to be
            // charged until project cleanup; allow a fresh retry copy.
            temporary: (size_mib * 3).max(64) * MIB,
            free_floor: 0,
            ..plan.into()
        },
        |_| {},
    )
    .unwrap()
}

fn lineage_history_with_field() -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "id":LINEAGE_HISTORY_ID,
        "type":"core.registry-history",
        "schema":1,
        "fields":{
            "registry.fields":[{
                "typeId":"game.marker",
                "fieldId":"marker.caption",
                "signature":"00".repeat(32),
                "retired":true
            }],
            "registry.versions":[]
        }
    }))
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
    stage_existing_source(fixture)
}

fn stage_existing_source(fixture: &Fixture) -> crate::StagedTree {
    let root = fixture.source_root();
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
struct KillChildOnDrop(Child);

impl Drop for KillChildOnDrop {
    fn drop(&mut self) {
        drop(self.0.kill());
        drop(self.0.wait());
    }
}

fn signal_and_wait_for_kill(marker: &Path) -> ! {
    fs::write(marker, b"SQLite writes reached kill point").unwrap();
    loop {
        std::thread::sleep(Duration::from_secs(1));
    }
}

#[cfg(test)]
fn kill_after_marker(child: Child, marker: &Path, before_kill: impl FnOnce()) {
    kill_after_marker_with_deadline(child, marker, Duration::from_secs(5), before_kill);
}

#[cfg(test)]
fn kill_after_marker_with_deadline(
    child: Child,
    marker: &Path,
    timeout: Duration,
    before_kill: impl FnOnce(),
) {
    let mut child = KillChildOnDrop(child);
    let deadline = Instant::now() + timeout;
    while !marker.exists() {
        assert!(Instant::now() < deadline, "child did not reach kill point");
        assert!(child.0.try_wait().unwrap().is_none(), "child exited early");
        std::thread::sleep(Duration::from_millis(10));
    }
    before_kill();
    child.0.kill().unwrap();
    assert!(!child.0.wait().unwrap().success());
}
