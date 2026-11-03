//! Root pin responsibilities for the native source store.

use super::{
    AcceptedSourceRoot, AdmissionFault, FIRST_PROFILE_MANIFEST_BYTES, MAX_HISTORY_BYTES, MIB,
    OptionalExtension, RootReadPin, RwLock, SqliteStoreCore, StagedCandidate, StoreError,
    Transaction, TransactionBehavior, TreeManifest, compare_carried_histories, decode_manifest_jcs,
    params, parse_registry_history, read_complete_blob, root_identity, stored_outcome,
};

impl SqliteStoreCore {
    pub(super) fn require_clean(&self) -> Result<(), StoreError> {
        if self.recovery_required {
            Err(StoreError::RecoveryRequired)
        } else {
            Ok(())
        }
    }

    pub(super) fn pin_current_root(
        &mut self,
        expected_generation: i64,
    ) -> Result<RootReadPin, StoreError> {
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

    pub(super) fn pin_accepted_root(
        &mut self,
        root: AcceptedSourceRoot,
    ) -> Result<RootReadPin, StoreError> {
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

    pub(super) fn release_root_pin(&mut self, id: i64) -> Result<(), StoreError> {
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

    pub(super) fn release_root_pin_now(&mut self, id: i64) -> Result<(), StoreError> {
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

    pub(super) fn retry_pending_pin_releases(&mut self) -> Result<(), StoreError> {
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

    pub(super) fn compare_history_with_current_root(
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

    pub(super) fn compare_history_with_pinned_root(
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

    pub(super) fn compare_staged_candidate_with_current_root(
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

    pub(super) fn matching_outcome(
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

    pub(super) fn commit_verified_manifest(
        &mut self,
        expected_generation: i64,
        epoch: i64,
        key: i64,
        request_hash: [u8; 32],
        revision: [u8; 32],
        manifest: &TreeManifest,
        publication_gate: Option<&RwLock<bool>>,
        fault: &mut impl AdmissionFault,
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
            fault,
        )
    }

    fn commit_with_closure(
        &mut self,
        expected_generation: i64,
        epoch: i64,
        key: i64,
        request_hash: [u8; 32],
        revision: [u8; 32],
        mut check_closure: impl FnMut(&Transaction<'_>) -> Result<(), StoreError>,
        publication_gate: Option<&RwLock<bool>>,
        fault: &mut impl AdmissionFault,
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
        fault.after_final_sql();
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
