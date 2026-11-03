//! Space responsibilities for the native source store.

use super::{
    BudgetError, Digest, HashCountWriter, Limits, MIB, ManifestEntry, OptionalExtension,
    SQLITE_PAGE_BYTES, Sha256, SqliteStoreCore, StoreError, StoreSpacePressure, TreeManifest,
    assess_live_available, assess_preflight_available, charge_projected_blob, ensure_manifest_size,
    fs, io, next_stored_blob, params, physical_io, projected_wal_growth, wal_path,
};

impl SqliteStoreCore {
    pub(super) fn ensure_wal_room(&mut self, incoming_allowance: u64) -> Result<(), StoreError> {
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

    pub(super) fn check_active_free(&self, incoming_allowance: u64) -> Result<(), StoreError> {
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

    pub(super) fn preflight_candidate_space(
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
}
