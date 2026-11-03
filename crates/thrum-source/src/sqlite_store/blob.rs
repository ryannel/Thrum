//! Blob responsibilities for the native source store.

use super::{
    BudgetError, CHUNK_BYTES, Digest, MIB, OptionalExtension, SMALL_BATCH_BYTES,
    SMALL_BATCH_FILE_BYTES, SMALL_BATCH_FILES, SQLITE_PAGE_BYTES, Sha256, SqliteStoreCore,
    StoreError, TransactionBehavior, ValueRef, params,
};

impl SqliteStoreCore {
    pub(super) fn flush_candidate_batch(
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

    pub(super) fn stage_small_blob_batch<'a>(
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

    pub(super) fn begin_blob(
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

    pub(super) fn partial_blob_has_contiguous_layout(
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
            if actual_length
                != i64::try_from(expected_length).map_err(|_| StoreError::InvalidChunkLayout)?
            {
                return Ok(false);
            }
            index += 1;
        }
        Ok(true)
    }

    pub(super) fn clear_incomplete_blob_chunks(
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

    pub(super) fn append_blob_chunk(
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
}
