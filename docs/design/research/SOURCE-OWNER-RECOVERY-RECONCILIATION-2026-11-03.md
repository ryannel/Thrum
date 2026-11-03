# Accepted-source owner and recovery reconciliation

3 November 2026. Supporting design for the [Foundation source contract](../FOUNDATION-SPEC.md#4-working-files-crash-recovery-and-gc). The accepted-source writer-lock and store-reconciliation gates remain **unchecked**. This selects a first-profile owner shape; it does not claim a migrated store or portable recovery proof.

## Required interaction

The previous accepted root and a registered read pin must survive an edit that commits a newer root. A failed or interrupted final commit must leave a recoverable exact keyed outcome, while no cooperating owner, pin cleanup or checkpoint rewrites the uncertain WAL before a fresh open resolves it. Reader CPU work must not hold a SQL transaction. The lock coordinates Thrum-owned processes; it does not exclude a noncooperating process that opens the SQLite file directly.

The current format-1 `NativeSourceStore` has a mutable borrowed `PinnedSource` and permits another open connection to advance the root while that pin lives. Its injected post-sync final-COMMIT fault quarantines only the failing connection. The format-2 `ProvisionalSourceStore` instead holds an exclusive `fs4` advisory lock from before SQLite open until its `Arc` owner and all leases close. It serializes SQLite access under one mutex and quarantines that owner after an uncertain final commit. Its source format, selection and publication semantics are provisional, so the lock cannot simply be copied into format 1: the accepted test that opens a second store to advance the root while the first is pinned would lose its only writer.

## Selected first-profile shape

Reconcile the accepted-source API onto **one exclusive cooperating process owner with shareable leases**. Acquire one sidecar lock before opening SQLite and retain it until the connection and last lease are gone. The owner serializes short SQLite operations; each lease holds a stable accepted identity and pin, releases the connection mutex between bounded reads and consumer work, and can outlive a later source-root advance on the same owner. Admission, pin registration/release and explicit checkpoint work use that owner. A final-COMMIT error latches recovery-required before any further owner SQL; pin drops retain their durable pin rows. Drop the owner and all leases, release the lock, then let a fresh opener verify root/outcome and resolve the exact request key before reporting selection or retry. Disable automatic and last-close checkpoint on every connection. A process crash releases its advisory lock; the new owner still has to verify stored identity and outcomes before writes.

This chooses serialization for the first native authoring profile, not a permanent ban on multi-process readers. If later tools require concurrent independent database connections, add a shared recovery gate and writer lock with the same old-pin and exact-outcome invariants before exposing them. Do not run two independent accepted-source authorities merely to preserve the current borrowed API.

## Reconciliation and proof order

1. Map format-1 accepted root, keyed outcome, verified blobs and pins onto one selected schema and the format-2 owner lifecycle. Preserve distinct source identity and complete authenticated reads. Reject incompatible existing formats until an explicit migration is proved; do not silently initialize or overwrite either database.
2. Give an accepted read lease a shareable owner. With lease A alive, admit B through that same owner, read A's exact bytes, release it, and reopen B and its keyed outcome. Check stale generation and changed-key refusal.
3. Hold the sidecar lock through leases and quarantine. A second cooperating process must refuse open while any lease remains. Inject before-write, before-sync and after-sync final-COMMIT faults; the failed owner performs no later SQL or checkpoint, a fresh opener resolves the exact key, and the prior pin remains usable or retained according to the outcome. Kill the owner before/after commit and repeat the recovery check.
4. Exercise a blocked reader checkpoint, sustained small-file admission and observed main/WAL/free-space peaks under the selected configurable plan. This is part of the existing native planning gate, not evidence of a hard physical cap.

The owner lock is paid at open, not per game tick. SQLite mutex contention occurs only on authoring/build reads and writes; bounded chunk copies release it between operations. Retain bytes, allocation count, edit latency, reader wait and WAL high-water in the assembled source-to-build measurement. The provisional 64 MiB tree and 256 MiB SQLite limits are test-profile constraints and must not silently become the accepted native policy.
