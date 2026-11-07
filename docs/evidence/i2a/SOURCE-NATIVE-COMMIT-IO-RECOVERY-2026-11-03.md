# Native accepted-store COMMIT I/O recovery — 3 November 2026

The format-2 provisional store's [native WAL fault proof](SOURCE-COMMIT-IO-RECOVERY-2026-11-03.md)
revealed a missing barrier in the older `NativeSourceStore` path. Its final
root/outcome transaction used rusqlite's default rollback-on-drop, and later
calls, including read-pin release, could issue SQL after a final `COMMIT` error.
An error after a WAL commit-frame sync can leave the newer root recoverable on
fresh open; same-connection cleanup can erase that uncertain result.

The accepted-store core now disables transaction rollback-on-drop immediately
before final `COMMIT`. On a reported error it returns a `RecoveryRequired`
category carrying the original SQLite failure and quarantines the owner.
Every public read, admission, usage and pin
operation refuses with `RecoveryRequired`; pin release and pending-release
retry also refuse without SQL. The caller must drop the old owner and reopen
the database to resolve the exact keyed outcome. Automatic WAL threshold and
last-close checkpoints are disabled and checked at connection open, so close
cannot rewrite an uncertain commit frame. Explicit bounded checkpoint work
remains owned by the store's WAL guard.

The test-only native SQLite VFS injected an I/O error **after** the final WAL
commit-frame sync. The first regression run recovered A instead of B after
drop/reopen, exposing the last-close checkpoint gap. With the checkpoint
policy and quarantine in place, the focused test recovers B at generation 2
and its matching key, retains A's read pin, sees no additional old-connection
WAL or main writes after the error, and releases the pin only on a fresh
connection. The test reaches the same final commit helper used by public
admission, after preparing complete root blobs. The repository gate and the
sandboxed admitted A/B build pass with the combined source changes.

Reproduce the focused case with:

```sh
RUSTC_WRAPPER= rustup run 1.99.1 cargo test --locked --offline -p thrum-source accepted_store_quarantines_after_uncertain_native_final_commit -- --nocapture
```

This is a **single-owner** injected callback and orderly-reopen proof on the
local native VFS. The accepted store currently permits other connections and
reader pins to coexist; the per-connection quarantine cannot stop one of
those owners from writing after another owner's uncertain commit. A
cooperating writer lock and shared recovery state, or a reconciled exclusive
owner, remain required before claiming this guarantee for concurrent use. The
test does not establish power-loss durability, actual full-volume fault
behavior, cross-platform VFS behavior, a physical-space floor or canonical
application recovery. No ready build is published by this test.
