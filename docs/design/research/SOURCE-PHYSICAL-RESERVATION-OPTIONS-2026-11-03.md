# Native source physical reservation options

3 November 2026. Bounded design/experiment for the [native physical-space contract](../FOUNDATION-SPEC.md#4-working-files-crash-recovery-and-gc), following the [checkpoint overlap counterexample](../../evidence/i2a/SOURCE-PHYSICAL-ENVELOPE-2026-11-03.md) and [performance policy](../AI-PERFORMANCE.md#performance-investment-from-the-first-design). The selected 22-file scratch source/config/result set is retained in a [byte-verified evidence archive](../../evidence/ARCHIVE-ACCESS.md#archive-inventory), 22,682 bytes, SHA-256 `39a478ab6615bf47e4fb0215e9e1f1fd9d2e95aa8b6e89933a7957b217bcc8a5`. No production code, public API, store schema, Build Plan or capability status changes were made by this experiment. Neither option below establishes an admitted-source lease or a physical reservation on APFS.

The subsequent [provisional Option A implementation](../../evidence/i2a/SOURCE-EXPLICIT-CHECKPOINT-2026-11-03.md)
completed this study's narrow next gate. The option measurements below remain
historical design evidence, not the implementation's performance results.
The later [Option B request-fence design](SOURCE-REQUESTED-IO-FENCE-2026-11-03.md)
adds a frozen-source replay, a safe credit oracle and explicit native-lifetime
blockers without treating request credits as physical reservation.

## Recommendation

Implement **A: explicit checkpoint ownership and a checked checkpoint plan** as the next bounded **provisional** gate. It removes hidden maintenance from ordinary publication calls and supplies a conditional bound on checkpoint data writes. Preserve the existing publication/status semantics: a maintenance failure after a successful final commit cannot be reported as an uncommitted publication.

A does not bound arbitrary SQL staging growth. Follow it with a separately reviewed **B: private requested-I/O credit fence**, if promotion still requires enforcement independent of B-tree/index/cache-spill behavior. B is useful enforcement of application requests, but cannot turn requested bytes into an APFS physical guarantee. The filesystem allocation model and durable reservation/recovery remain explicit promotion blockers. Do not replace the current guard with a multiplier inferred from this fixture.

## Shared accounting and filesystem assumptions

One process-shared cooperating writer/reservation lock must span planning, grant and writes. Count unique old/pinned and new blob closures once; separately count working-file staging, manifests/outcomes/intents/indexes, managed temporaries, retained main/WAL bytes, checkpoint duplication and a recovery tranche. Previously admitted/pinned bytes cannot become free credit merely because a new root replaces them. A SQLite freelist is internal reuse, not volume space returned.

Before beginning admitted work, the proposed physical peak must fit its project buckets and leave **8 GiB actually unallocated**. Recheck availability before each bounded allocating operation/checkpoint. Do not count expected GC, a blocked checkpoint, sparse extent length, purgeable capacity or a future successful truncate as reclaimed capacity. The provisional store's 64 MiB free-space backpressure is not this contract.

APFS has shared-container free space, snapshots and copy-on-write objects. Rewriting existing file ranges can require new allocation while old versions remain retained. The public on-disk format describes these mechanisms but supplies no application-call bound for all metadata/checkpoint allocation. Thus a data-write bound does not prove a physical-space bound. [Apple File System Reference, pp. 7–10 and 116](https://developer.apple.com/support/downloads/Apple-File-System-Reference.pdf).

`F_PREALLOCATE` provides an allocated-byte result and an all-or-none flag, but the cited API does not promise reserved capacity for later CoW rewrites, filesystem metadata or snapshots. A sacrificial reserve file cannot transfer its blocks atomically to SQLite; deleting it only creates a race. Sparse files and allocation hints likewise are not reservations. No APFS allocation/preallocation experiment or dedicated-volume qualification was performed here. [Apple fcntl manual](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/fcntl.2.html).

The application can serialize its participants, enforce its credits, preserve roots/pins and recover an exact outcome. It cannot stop unrelated volume consumers between checks or guarantee honest hardware flushes. Those are external-volume/device assumptions, not failures that credits can prevent.

## A — own checkpoints explicitly

Configure and verify the **same private writer connection after open**, before any ordinary write:

- `sqlite3_wal_autocheckpoint(db, 0)`, with `PRAGMA wal_autocheckpoint` read back as zero.
- `SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE=1`, read back as one. Last-close checkpointing is a separate mechanism; disabling automatic checkpoints alone is insufficient.
- Retain WAL/FULL, verified 4 KiB pages/main cap, busy timeout zero, no attached databases, no consumer SQL read transactions. Verify `mmap_size=0` and zero VFS chunk-size for the audited native path; otherwise hints/truncation may allocate beyond the simple model. Read-only/maintenance handles also need the appropriate close policy.

SQLite documents the per-connection automatic threshold and its disabling value, and separately documents checkpoint-on-close. [Auto-checkpoint API](https://sqlite.org/c3ref/wal_autocheckpoint.html), [NO_CKPT_ON_CLOSE](https://sqlite.org/c3ref/c_dbconfig_defensive.html#sqlitedbconfignockptonclose).

Under the uninterrupted writer lock, obtain valid `N=page_count`, `L=total WAL frames`, `C=backfilled frames` using a non-writing `wal_checkpoint(NOOP)`. Reject errors, negative/invalid counters or mismatched verified limits. A conservative candidate **data-write** allowance is:

```text
P = 4096
D = min(N, L - C)
checkpoint main-data write allowance = D * P
main logical extent after a full checkpoint <= N * P
```

This is an inference from the **pinned SQLite 3.53.2 implementation**: `walIteratorNext` yields increasing distinct database pages; `walCheckpoint` writes a selected latest frame once for each eligible page no greater than `hdr.nPage`. It is conditional on a valid, stable WAL/database state, the reviewed native VFS configuration and no other writer between planning and execution. The probe checked the allowance in one fresh/edit fixture; it is not a public SQLite promise or an exhaustive proof over corruption, interruption, busy readers, resets and future versions.

Charge the retained WAL throughout. An incomplete checkpoint can still write main pages; `BUSY` does not imply zero allocation. Only successful observed truncation and a fresh availability check allow reconciliation. SQLite reports frame progress and documents the partial-progress behavior of checkpoint modes. [Checkpoint API](https://sqlite.org/c3ref/wal_checkpoint_v2.html).

`D*P` bounds requested checkpoint data writes under those conditions, not APFS physical bytes, sync/metadata work or ordinary staging SQL. Even `max(0,N*P-main_length)` misses CoW overwrites. Keep a separate physical allowance whose filesystem assumptions are qualified. Do not disable cache spill merely to obtain a simpler model: that can move the cost into a larger transaction/page-cache memory peak. Index splits, pointer maps, reset/deletion of incomplete blobs and every DML transaction still need an envelope or B's fence. [SQLite cache controls](https://sqlite.org/pragma.html#pragma_cache_spill).

A production scheduler must replace **all** existing ad hoc checkpoint calls, including the provisional guard's threshold checkpoint, and cover initialization/error cleanup/reopen before configuration. At the selected production WAL thresholds it should request PASSIVE progress, seek TRUNCATE when readers drain, and stop further staging if the charged stop cannot be cleared. The current probe stays below the provisional 32 MiB threshold; it does not exercise that replacement.

## B — private requested-I/O credits

A private, store-bound VFS can debit a grant **before** forwarding each allocating request. This avoids guessing SQL-to-page amplification: index splits and repeated page flushes consume actual requested work. The scratch prototype conservatively charges touched 4 KiB ranges per `xWrite`, including overwrites, charges growing truncation and SHM mapping, never refunds a failed write, and suppresses SIZE_HINT. Aggregate debit covers main and WAL together, so it cannot pretend checkpoint overlap is free. Its 64 KiB grant refused a tentative publication before the next write, preserving selected/readable A.

This is a serial, volatile **requested-page-work prototype**, not a production VFS or physical allocator. Full enforcement would require:

| Surface | Required production decision/proof |
| --- | --- |
| `xWrite` | Checked arithmetic; charge before forward; retain charge after partial/failed writes; enforce file and aggregate ceilings. Charge overwrites if CoW is possible. |
| `xTruncate`, hints and chunk rounding | Charge growth; no optimistic refund on shrink. Suppress advisory SIZE_HINT rather than trusting its failure to abort SQLite; set chunk rounding to zero or include it explicitly. |
| SHM/mmap | Reserve mapping extent before extension; account later dirty mapped pages. Disable writable data mappings and close bypasses. |
| open/delete/temp/controls | Bootstrap and directory/metadata allowance; managed temporary root; all file kinds; no unapproved allocating file controls or raw underlying-file escape. |
| identity/concurrency | Same store/volume/owner generation and process-shared lock for all handles. No SDK SQLite connection, alternate writer/VFS, callback reentrancy or foreign store path. |

SQLite exposes separate write, truncate, file-control and shared-memory methods. SIZE_HINT may cause allocation and is expressly only a hint; the pinned native implementation can perform internal allocation from it, outside an outer `xWrite` hook. Therefore a wrapper that checks only main/WAL length growth is incomplete. [I/O methods](https://sqlite.org/c3ref/io_methods.html), [file-control semantics](https://sqlite.org/c3ref/c_fcntl_begin_atomic_write.html).

The scratch fence does **not** solve filesystem metadata, sync allocation, managed temporaries, open/delete accounting, all control bypasses, per-file ceilings, concurrent callers or durable grants. Its conservative rounding charged 74,461,184 B for 35,380,752 B of fresh writes; neither number is a physical peak. Reducing that overcharge needs a separately verified extent/epoch representation, not automatic credit refunds for repeated offsets.

## Durable grants and recovery required by either option

Proposed ordering, not implemented schema/API:

1. Acquire the common lock and establish bounded **bootstrap/recovery credit before SQLite open** can create/rebuild SHM or perform cleanup. Validate file/store identity and volume, then open/configure the private connection. Read-only application intent is not proof that the VFS performs no writes.
2. In a short transaction funded by an already reserved metadata tranche, record a store-bound operation/grant with exact base/target request, retained closures, byte ceilings and recovery policy **before tentative staging**. The record itself and its WAL must be charged; recursively funding the reservation record from an unrecorded grant is invalid.
3. Spend the in-memory ledger without per-page durable writes. Final publication atomically commits exact selection/outcome/intent status. A real uncertain commit is resolved by the exact key; do not infer failure from a later checkpoint or success from an in-memory debit.
4. After process death, acquire the lock as a new owner. Treat the entire unresolved grant as unavailable until bounded reconciliation of main/WAL/temp and exact request status. Do not restore its full volatile balance. Retain incomplete data and old pins; no age-based owner inference or destructive GC. New recovery work needs its own reserved tranche and free-space check.

A grant cannot be persisted for free in the same database it is funding. The reservation bootstrap, crash reconciliation, qualified physical allowance and safe owner-pin lifecycle must be designed together before accepted-source authority. The current canonical RecoveryRoot envelope is still separate from provisional publication intent.

## Narrow local measurements and limitations

One matched fresh/edit per option on macOS 26.6.2 arm64/APFS, Rust 1.99.1, actual public v2 APIs: 1,000 distinct 4 KiB notes, a 12 MiB master and five project records. The edit changes one environment record; A's logical lease remains readable after B. Capture/validation precede the timed publication. Whole-worker RSS includes fixture/capture/validation. These are **single diagnostic samples**, no percentile, speedup or qualified overhead claim.

| Path | Fresh publication | Explicit checkpoint | Edit publication | Explicit checkpoint | Whole-worker peak RSS |
| --- | ---: | ---: | ---: | ---: | ---: |
| Current native default | 114.903 ms | automatic work included | 33.012 ms | automatic work included | 48,054,272 B |
| A, native forwarding shim, explicit scheduling | 103.415 ms | 24.506 ms | 33.822 ms | 0.557 ms | 48,726,016 B |
| B, requested-page fence, default checkpoint policy | 117.591 ms | automatic work included | 31.468 ms | automatic work included | 48,037,888 B |

A's publication had **zero main-file writes**. Its fresh checkpoint wrote exactly 17,596,416 B under that data allowance; edit wrote 229,376 B under 258,048 B. It accumulated 4,471 valid WAL frames before fresh maintenance; moving checkpoint work can increase retained WAL/overlap. A's total fresh measured path is 127.921 ms. B's observed delta versus native was +2.688 ms fresh and −1.544 ms edit; this does not isolate ledger cost from noise. A includes forwarding-shim overhead. Neither path includes a qualified physical allocator, durable credit journaling or all required availability checks.

An initial A attempt configured auto-checkpoint from `sqlite3_auto_extension`; SQLite's later open sequence reset it to the default. That run is retained as a rejected proxy. The corrected scratch hook tracks actual live SQLite handles, matches the store's exact filename, requires autocommit/no concurrent operation, and configures/readbacks the **private writer after public open**. It never substitutes a separate connection. This serial test hook is not a proposed public API.

The single quota fault exhausted exactly 65,536 requested-page credits; SQLite returned FULL, old selection/lease bytes survived, and reopened status was PendingNeedsRetry. A deliberately fresh unlimited test grant allowed retry to generation 2. **Durable-credit recovery is unimplemented**; no new SIGKILL or constrained-volume ENOSPC qualification is claimed.

## Next gate and reproducibility

Next bounded production-code commission should implement **A's private connection configuration, centralized checkpoint planner and progress/refusal semantics while keeping provisional authority**. Require readback, zero hidden checkpoint writes, checked data allowances across blocked-reader/partial/retry cases, and a failure before/within checkpoint preserving exact root/status. Include opening/closing/error cleanup. This is useful even if later B is selected, and avoids committing a new unsafe VFS boundary prematurely.

Before admission promotion, separately require complete B/SQL-envelope enforcement, durable bootstrap/grant recovery, actual constrained-volume ENOSPC and process-kill proof, APFS allocation/preallocation qualification including snapshots/CoW, managed temp limits and the 8 GiB floor. The small local timing sample cannot waive those gates.

Runtime SQLite: **3.53.2**, source ID `2026-06-03 19:12:13 d6e03d8c777cfa2d35e3b60d8ec3e0187f3e9f99d8e2ee9cac695fd6fcdf1a24`.
Reviewed bundled amalgamation and full compile options/tool hashes are retained with the scratch recipe in `tmp/source-vfs-credit-proof/`; no production source was changed.

- Source store SHA256: `b6da4d3f9b335d3f4037c8cdb494d5dd6bf34eef7b47f8807842cac0b551d0ec`.
- Executed final probe SHA256: `de0e210eeb5a9af65618e609de52d8043b9eff582f7cc8b63af5395050114bac`.
- VFS prototype source SHA256: `4cc1badbd76a9f07ee7b59b4dfee18b98ae1d9ad93cd24265701b6e53063cbb2`.

Reproduce with the scratch Cargo recipe (`--locked --offline --release`) and `python3 tmp/source-vfs-credit-proof/run.py`. The build script pins the locally cached 3.53.2 header path; another machine must record its replacement path/tool identity. Raw option JSONL, runtime compile options, worker RSS and rejected initial proxy are selected local evidence. Owned databases/build cache are removed after archive verification. Scoped scratch build/Clippy/fmt and repository documentation checks are required; no production/runtime readiness follows.
