# Accepted-source partial-blob retry — 3 November 2026

This checkpoint changes the production `SqliteStoreCore` path consumed by
`NativeSourceStore`, and its test-only core adapter. It addresses the
[cross-owner review](../../design/research/LIMIT-REVIEW-SOURCE-STORAGE-2026-11-03.md)
finding that `begin_blob` deleted every incomplete chunk in one IMMEDIATE
transaction after only a 1 MiB WAL allowance. It does not change the selected
root/outcome transaction, accepted-source pin, source identity or planning
policy.

## Mechanism and bounded work

An incomplete `state=0` header is never a selectable blob. A retry requires
the same hash, length and chunk count, then scans only `(index, length)`
metadata to check for a contiguous prefix. The streaming writer compares each
retained chunk byte-for-byte with the immutable staged input before reusing
it. A same-length byte mismatch is replaced in one bounded chunk transaction;
the entire stored blob is rehashed before its complete marker can commit.
Only newly missing or repaired chunks enter the WAL on an ordinary retry.
The batched path for files at most four KiB applies the same header identity,
retained-byte comparison and full stored-hash check. A malformed partial rolls
back the whole batch, clears at most one bounded row per cleanup transaction,
and reports failure so the caller can retry after cleanup. Ordinary new-file
batches still use one transaction.

Holes, extra indices and wrong bounded lengths trigger cleanup of at most one
chunk per transaction. That progress is durable and restartable while the
header stays incomplete. A header identity mismatch or a corrupt row larger
than the four-MiB chunk profile refuses automatic repair; a complete-blob
inconsistency still refuses as corruption. Each cleanup/repair write calls
the selected WAL-pressure check before its own transaction. The configured
watermark remains an estimate/backpressure control, not a physical allocation
bound.

For `k` retained chunks, the ordinary retry adds `O(k)` metadata inspection
and at most `k × 4 MiB` stored-byte comparison, while avoiding re-insertion
of the matching prefix. Final full-blob verification still rereads the stored
bytes. The stream uses one four-MiB scratch buffer plus SQLite internals;
this is a logical work account, not an RSS or elapsed-time measurement. Invalid
layout cleanup uses `O(k)` short transactions and can be slow for a very large
damaged blob; no claim of a short total recovery time follows.

## Executable observations

- A no-delete SQL trigger proves a matching interrupted prefix is reused by
  both the core path and the production accepted owner; a live A pin reads
  exact scene bytes between B's committed/reused chunks.
- A process killed after its first B chunk reopens with A selected and B
  incomplete; a no-delete trigger then proves B retries without resetting
  that chunk before the keyed root/outcome commit.
- Same-length stored-byte corruption is repaired in one chunk write and
  reverified; header metadata mismatch refuses without deleting data.
- A hole plus an extra index triggers bounded cleanup. An injected SQL error
  after the first committed deletion leaves one incomplete row; reopening and
  retrying completes cleanup and the blob/root. An oversized corrupt row
  refuses automatic cleanup.
- The existing stale-writer and post-verification corruption tests still pass;
  their source/selection outcomes are unchanged.
- Tiny-file batch tests reuse a partial row under a no-delete trigger, repair
  a same-length mismatching row, refuse a changed header, clear a wrong-length
  row before retry and prove that malformed cleanup rolls back other new batch
  members while deleting one row per transaction.

The focused tests and full `thrum-source` suite run under the pinned Rust
toolchain. `RUSTC_WRAPPER= ./dev check` passed: repository/document checks,
107 Python tests, workspace Clippy and Rust tests, including 189 passing
`thrum-source` unit tests (46 opt-in tests ignored). The tests
inject process kill and SQL failure, but they do not measure hundreds of
chunks, transaction-page high-water, host-volume `ENOSPC`, power loss, or a
complete F3→F4 edit-to-ready build. Those remain open in the build plan.
