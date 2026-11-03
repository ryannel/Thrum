# Provisional source COMMIT I/O recovery — 3 November 2026

Actual final SQLite WAL I/O failures exposed a correctness defect beyond the
earlier [publication interruption proof](SOURCE-PROVISIONAL-PUBLICATION-RECOVERY-2026-11-02.md)
and [checkpoint proof](SOURCE-EXPLICIT-CHECKPOINT-2026-11-03.md). A successful
native WAL commit sync followed by an injected `SQLITE_IOERR_FSYNC` left B
recoverable on a fresh open. However, ordinary `drop(store); drop(lease_a)`
released A's pin through the old connection. That `DELETE` could overwrite
the uncertain B commit frame, leaving A selected and B pending after reopen.
The new lease-drop regression failed with that exact result before the fix.

The provisional store (historical source: `git show 1a6273a:crates/thrum-source/src/provisional_store.rs`)
now quarantines its owner on an actual final `COMMIT` error. The original
SQLite error is returned; every subsequent connection acquisition returns
`StoreError::RecoveryRequired`, including selection, exact status, retry,
new leases, old lease reads and pin cleanup. Drop all store/lease owners,
then reopen to resolve the exact keyed result. A failed pin release leaves
its durable row retained. Immediately before attempting the final commit,
the transaction disables rusqlite's automatic rollback-on-drop, preventing
hidden SQL after the error. Earlier precommit failures retain normal rollback.

The test-only [native VFS](../../../crates/thrum-source/src/provisional_store_vfs_test.rs)
observes successful commit-frame writes and native syncs under the store's
verified 4096-byte page profile. Tests persist B's pending intent and complete
blobs first, so its identical retry's next WAL transaction is the final
selection/outcome transaction. One-shot faults exercise these boundaries:

| Actual callback failure | Fresh native open |
| --- | --- |
| Before the first final WAL `xWrite` | A selected; exact B key is `PendingNeedsRetry` |
| Before `xSync` of a written commit frame | B selected; exact B key is `Committed(B)` in this orderly-reopen fixture |
| Successful native commit-frame `xSync`, then report failure | B selected; exact B key is `Committed(B)` |

Both explicit A-lease close and ordinary A-lease drop after the final sync
error preserve B, retain A's pin and bytes, and produce no additional observed
WAL/main writes or main truncates. Fresh open reads B's exact bytes and
same-key retry returns B. The prewrite case can retry forward to B. Existing
normal publication, acknowledgement-loss, checkpoint and process-kill tests
remain passing. The observer is absent from production builds; the production
guard adds one relaxed atomic load after acquiring the existing owner mutex.

Reproduce the focused cases with:

```sh
RUSTC_WRAPPER= rustup run 1.99.1 cargo test --locked --offline -p thrum-source final_commit_ -- --nocapture
```

The two focused tests passed. The direct pinned source-library gate and
`RUSTC_WRAPPER= ./dev check` each reported **149 passed, 35 ignored** source
tests (184 total), including both new cases; repository guards, rustfmt,
Clippy and workspace tests passed. Both gates visibly rebuilt `thrum-source`;
no stale-cache workaround was needed. An independent full-gate replay exposed
an existing test-marker race: the parent observed the empty file created by
`fs::write` before its boundary byte was written. Repeated focused runs
reproduced it on iteration 83 at `IntentRecorded`, before the new final-COMMIT
handling is reached. The test-only child now writes a sibling pending marker
and renames the complete file to its ready name. All 100 subsequent focused
iterations passed (400 boundaries), and the full gate passed again with
149 passed and 35 ignored source tests. Host: Darwin 25.6.0 arm64, native
`unix` VFS, bundled SQLite 3.53.2. Source identity is HEAD
`05184410ccdee3494c9558822f98306c4081f277` plus the current uncommitted files:

- `provisional_store.rs` SHA-256: `f9355227dc36ce39b5b6ae6e56d14115fbc3ab41dc12e234f6cff8b06430cd44`
- `provisional_store_vfs_test.rs` SHA-256: `a74e2df3a2accd86be7ae58eb81cbc3fcfdafb57c336f1e012ed620cd1451eb5`

This is deterministic callback-failure and orderly-reopen evidence. The
before-sync case does not establish power-loss durability. Hardware honesty,
real constrained-volume faults, other native VFS/platform runs, physical peak
reservation, admitted-source authority and abandoned-pin recovery remain open.
