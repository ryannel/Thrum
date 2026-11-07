# Private source requested-I/O fence

3 November 2026. Design and bounded executable evidence following [the options study](SOURCE-PHYSICAL-RESERVATION-OPTIONS-2026-11-03.md) and [implemented provisional checkpoint policy](../../evidence/i2a/SOURCE-EXPLICIT-CHECKPOINT-2026-11-03.md). The experiment did not change production code, public API, schema or contracts. The selected [proof archive](../../evidence/ARCHIVE-ACCESS.md#archive-inventory) is **267,354 bytes**, SHA-256 `62adf54a7b1f7af518f4bb53d3121c50b5e46b18216f9b3b7987fd79a3494c08`, with 58 byte-verified members. It retains the complete recipe, raw logs, workload manifests, oracle/mutants, frozen source inputs and tool identities. Independent replay found no actionable defect within this stated scratch scope.

**Decision:** B is worth retaining as a private **request quota**, using the contract below. Reject requested-byte credits as the answer to the Foundation physical-space gate. Do not add production unsafe code directly from the scratch adapter: first implement/review the safe ledger and owner/close barrier, then prove the complete native adapter. APFS allocation, durable reservation and accepted-source authority remain unproved.
The later [isolated safe-ledger proof](../../evidence/i2a/SOURCE-REQUEST-CREDIT-LEDGER-2026-11-03.md)
closes that state-machine prerequisite within its mock-witness scope, before
production owner integration or native callback work.
The separate [APFS admission-options study](SOURCE-APFS-PHYSICAL-ADMISSION-OPTIONS-2026-11-03.md)
shows why shared-volume guarding, reserve/quota and dedicated containers still
do not close the exact physical-floor requirement.

## Boundary and units

Keep the current source store's single cooperating process owner, cross-process lock, validated-candidate publication and logical leases. No SDK connection, VFS pointer, grant-reset function, arbitrary SQL, ATTACH, virtual-table extension, alternate writer or long consumer SQL read transaction is exposed. F4 still reads through its source lease. The fence does not interpret source records, authorize a root or release pins; those remain source/SQLite authorities.

The first adapter profile is the **audited SQLite 3.53.2 native Unix VFS on macOS arm64/APFS**, 4 KiB database pages and measured 16 KiB OS pages. Pin source ID, relevant compile options, native VFS implementation, OS mapping granularity and policy digest. Other VFS/ABI/platform combinations refuse until reviewed. Retain Option A's disabled/read-back automatic and last-close checkpoints, zero data mmap/chunk rounding and checked explicit maintenance. This fence adds no physical guarantee to that policy.

Use an atomic vector of independent credits; never label their sum physical bytes:

| Credit | Exact meaning |
| --- | --- |
| `W` | Sum of requested `xWrite` lengths, including repeated offsets and failed/partially completed native writes. |
| `E` | Additional logical extent reserved above each file object's retained high-water mark. Sparse gaps count. Main, WAL and journal reservations coexist. |
| `S` | Additional granted SHM mapping capacity, rounded to the native mapping group. This covers a mapped channel's capacity, not the number of later memory stores. |
| `M` | Operation/envelope tokens for open, prepaid close, truncate, sync, delete and SHM setup/teardown. These count API work, not filesystem metadata bytes. |

`reserve(W,E,S,M)` checks every component and checked-arithmetic/file cap **before changing any balance or forwarding**. An insufficient vector leaves balances and native dispatch unchanged. Once reserved, burn the whole vector before dispatch. Native failure never refunds it. A successful shrink/unlink does not immediately return extent credit.

The first finite role caps are main **256 MiB**, WAL **64 MiB**, main journal **16 MiB**, SHM **128 KiB**; at most **8 authorized file handles** and **8 active/quarantined store-owner contexts** per host process. These are proposed request-profile limits, not a new accepted-source budget. An initial candidate-operation ceiling may grant at most **256 MiB W, 336 MiB E, 128 KiB S and 65,536 M**; the actor may grant less. These limits bound requests and may refuse a valid candidate: completion across all source-profile states is unqualified. Bootstrap/recovery grants have separate smaller privileges below. No grant automatically expands or replenishes with time.

## Per-method contract

The native VFS is delegated to, not reimplemented. Keep native locks, sync semantics and device guarantees; clear optional atomic/batch-write capabilities unless their separate control paths are audited and charged. All failures remain visible.

| Entry point | Before forwarding / required behavior |
| --- | --- |
| `xOpen` | Bind canonical expected main path and role flags. Permit only main, its WAL and main journal. Reject unknown, null-name, TEMP, TRANSIENT, SUBJOURNAL, SUPER_JOURNAL and DELETEONCLOSE roles in this first profile. Reserve **2 M** for the open attempt and eventual native close, plus a fixed handle slot; validate existing extent against its role cap. A partial native open with non-null methods still receives a valid wrapper and prepaid cleanup. |
| `xWrite(file,offset,n)` | Reject invalid/overflowing ranges. Reserve `W=n`, `E=max(0,end−object_hwm)`; check the role's maximum end. Advance reserved HWM before native dispatch, even if it later fails after writing only part. Rewrites spend W again. |
| `xTruncate(file,n)` | Reserve **1 M** and any HWM growth before forwarding. Keep the previous reservation on shrink or failure. A metadata operation that can extend a sparse file cannot bypass E. |
| `xSync` | Reserve **1 M** before native sync. Refusal/error may make a final commit uncertain; it does not prove no bytes were written or flushed. |
| `xShmMap` | Validate region/index and compute the native group-rounded end. Reserve new **S** even when `extend=0`: native SHM opening/mapping can occur below this callback. Reserve a conservative **4 M** envelope per attempt, including possible hidden SHM open and prepaid unmap/close. No SHM extension is delegated without S. |
| `SIZE_HINT` | Always suppress; return NOTFOUND without calling native allocation. Do not rely on an error from a forwarded hint to abort SQLite. |
| chunk/mmap/fetch controls | Keep chunk rounding and data mmap zero. Reject nonzero enabling controls; `xFetch` returns OK with a null result so SQLite uses ordinary reads, never a writable data mapping. |
| `xDelete` | Never delete main. Check owned auxiliary path/generation and operation phase; reserve **1 M** before native delete. An unlinked object with live handles/maps remains reserved. SQLite owns WAL safety; the fence does not duplicate its checkpoint algorithm. |
| `xShmUnmap`, `xClose` | Use cleanup already prepaid by open/map; never require replenishing the exhausted write grant to release a handle. Set/read native PERSIST_WAL=1. An unexpected request to unlink SHM is not silently reported successful: unmap without deletion, return an error and retain/quarantine state. Ordinary auxiliary deletion requires an explicit charged phase. |
| other controls | Forward only the audited list below. Unknown controls return NOTFOUND without native dispatch. FILE_POINTER/JOURNAL_POINTER/VFS_POINTER must identify wrapped objects; never return underlying writable methods. Disable dynamic extension/system-call replacement escapes. |

SQLite's VFS distinguishes these file roles, permits partial-open cleanup and specifies filename lifetime through close. [VFS interface](https://sqlite.org/c3ref/vfs.html), [open flags](https://sqlite.org/c3ref/c_open_autoproxy.html). Its I/O interface separately exposes mapping and file controls. [I/O methods](https://sqlite.org/c3ref/io_methods.html). SIZE_HINT is advisory and may preallocate; the pinned Unix implementation can perform internal writes from it, bypassing an outer `xWrite` wrapper. [File controls](https://sqlite.org/c3ref/c_fcntl_begin_atomic_write.html).

The initial forwarding allowlist is LOCKSTATE, LAST_ERRNO, read-only POWERSAFE_OVERWRITE query, VFSNAME, native Unix PRAGMA fallback, HAS_MOVED, SYNC, COMMIT_PHASETWO, CKPT_START/DONE, DATA_VERSION, zero CHUNK_SIZE/MMAP_SIZE, and set/query PERSIST_WAL=1. Mask unimplemented atomic capabilities and reject enabling controls. Record optional unknown codes during proof; do not expand the list merely to make a failing test pass. This is a trusted private SQL boundary, not a sandbox against arbitrary native code in the same process.

For the pinned Unix SHM code, the mapping group is `max(32768, OS page size)` for the qualified page sizes, and the requested end rounds upward to that group. A 64 KiB-page platform can map two regions for a request for region zero; charging just 32 KiB undercounts. Native SHM setup writes/materializes storage internally, and later mapped updates never call outer `xWrite`. Thus S must remain a separate capacity allowance. At a strictly enforced 64 MiB WAL extent, the pinned WAL-index layout fits four 32 KiB regions; reject a larger request rather than guessing. Other mapping granularities require a new policy.

## Representation and concurrency

Use a fixed table of role/object generations and handle slots, with per-object retained HWM, SHM capacity, live handle/map references and unlink state. Main/WAL/journal are distinct reservations; never subtract expected WAL truncation while main grows. File identity cannot be keyed by pathname alone: unlink plus reopening that path can create another object while old handles still retain the previous one. Closed/unlinked objects retire only at the actor barrier below.

No interval tree or per-page set is needed for W: every repeated request spends its full length. HWM makes E constant-time. A set that deduplicates repeated offsets would undercount write work; reducing W by coalescing requests would require its own proven completion contract. The safe four-slot oracle occupies **152 B**; a real eight-slot owner adds binding, synchronization and lifetime state and needs a C12 allocation charge. It is not a measured production-memory budget.

The connection mutex and per-store cooperating owner lock span the logical operation; the owner lock remains with leases/quarantine. A separate ledger mutex makes each vector reservation atomic across authorized handles; release it **before native callbacks**. Do not call SQLite, acquire the connection mutex or perform grant journaling from a VFS callback. Bind every grant to directory/volume/store identity, owner epoch, policy digest, operation/key/base/target and monotonic grant sequence. Raw selection-token bytes cannot mint credit. No next operation/regrant is allowed while a native callback is in flight or the SQL operation has not reached its explicit recovery barrier.

The cost adapter uses serial static counters and therefore excludes production ledger-mutex, grant-journaling and quarantine overhead. Its measured cost cannot qualify that future representation.

## Registration, close and panic lifetime

Use a unique **non-default** private VFS registration per owner. Its object, immutable method tables, name and context address stay stable until unregister completes. Preallocate handle slots and wrapper/native storage with the reviewed native alignment and `szOsFile`; no unbounded maps or allocations in callbacks. Every delegated method receives the real native file, not the wrapper header. The original native VFS storage must outlive all delegation.

Before native `xOpen`, set outer methods null and establish context/prepaid cleanup. If native returns non-null methods even on error, install valid wrapped cleanup before returning. A callback must not unwind through C: use panic-free checked operations plus an outer panic barrier that poisons allocating work. Reads/unlock/prepaid close remain available for cleanup. Never refund from a reservation guard's Drop.

**The last file callback must not unregister/free its VFS.** SQLite can still use the VFS after that callback returns while closing the connection. A strong registration root and owner lock survive the entire outer `Connection::close` call. Finalize statements, close, require successful return, no active callbacks and no live handles/maps, then unregister and free. If close/panic cannot establish that barrier, move the connection, lock and registration into a **pre-reserved quarantine slot**; keep them protected until explicit successful close or process exit. Do not release the lock merely because an Arc count or timeout reached zero. Fail new owner creation if the finite registry/quarantine slots are exhausted.

SQLite documents registry lifetime/thread-safety, but the close barrier and quarantine are this design's requirements. The safe oracle checks the early-free predicate; it is not an FFI use-after-free proof. Native partial-open, nested cleanup, failed close and panic conformance are mandatory before production unsafe code.

## Bootstrap, durable recovery and source ownership

Acquire the **per-store owner lock first**, before registration/open, and retain it through source leases/quarantine. Before any potentially allocating SQLite operation, acquire the common **volume reservation/writer permit**; release that short permit after the operation/barrier, never hold it through CPU decode or a returned lease. All cooperating Thrum stores on one volume must use the same broker/lock namespace or serialize to one active allocating store. Independent per-project free-space checks can otherwise spend the same headroom twice. No path may wait for another store-owner lock while holding the volume permit. The requested-only prototype has no multi-store physical broker; this remains part of the physical gate.

Validate the engine directory and file types; use native no-follow behavior and keep the cooperating namespace stable. Path checks alone do not exclude a noncooperating directory replacement. That assumption, like unrelated volume consumers, must be diagnosed explicitly.

Use these private phases:

1. **Existing-store bootstrap:** a grant permitting bounded opens/reads, at most 128 KiB SHM capacity and 256 M, with **zero main/WAL/journal W/E**. Validate finite existing file extents before opening. Configure/read back Option A immediately after open. Authenticate store/project and exact selected/intent state before ordinary grants. If recovery demands data writes, refuse this phase; do not silently upgrade it.
2. **Creation/explicit recovery:** a separately authorized finite request envelope, never a free bypass. A possible creation envelope is 4 MiB W, 1 MiB main extent, 4 MiB WAL and 256 M; it may refuse partially initialized storage. Proving deterministic creation recovery remains required. Existing hot-journal or unauthenticated recovery writes need a specific recovery plan; do not initialize over them.
3. **Prepared operation:** compute unique old/new retained closure and managed staging/temporary obligations in the source actor. After the physical policy's fresh-volume check, record a grant intent in a short transaction funded by an already reserved metadata/recovery tranche. That record, its WAL and any outcome are charged too. Bootstrap cannot be funded by recursively inserting an unrecorded grant into the same database.
4. **Execute/reconcile:** debit requests, retain incomplete blobs and old roots/pins, then use the exact publication key/base/target to resolve the SQLite outcome. Replace/release an operation grant only with SQL autocommit established, no in-flight callbacks, checked native extents, and known unlink/close results. A logical E/S release is not a claim of returned physical space.
5. **New process:** acquire the lock, read/verify bounded recovery state, and regard an unresolved grant as wholly unavailable until reconciliation. Never restore its original balance because RAM was lost. The new owner can distinguish dead ephemeral pin ownership only through the lock/owner protocol; persistent build/save/release/intent pins retain their own lifetimes. Keep destructive GC disabled until C10's independent two-epoch gate is satisfied.

A failed data/cap grant returns FULL before native write/truncate; invalid requests return the matching I/O error. Open refusal uses CANTOPEN; delete/sync refusal uses IOERR_DELETE/FSYNC. Record a bounded private refusal diagnostic so the actor can distinguish quota from native ENOSPC/I/O error; do not collapse either into contention. Partial native failure burns all reserved credit and blocks allocating work pending the actor's recovery barrier.

Budget failure during final sync can leave an uncertain commit. Do not promise A if exact status says B committed, or report B accepted from an in-memory counter. Preserve current exact keyed publication/status semantics. A lease remains pinned to A after B; release failure retains protection. The fence neither authenticates schema meaning nor creates the canonical RecoveryRoot envelope.

## Why this does not close physical admission

The physical policy must still count working-file staging, old/new unique closure, SQLite/index/cache spill, retained WAL, checkpoint copies, managed temporary work and recovery slack, leaving **8 GiB actually free after projected peak**. W/E/S/M are not a conversion to that physical peak. SQLite freelists, successful truncation and SHM unmapping do not certify space returned to the volume.

APFS snapshots, shared space and CoW metadata can retain blocks after overwrite/unlink. Public allocation hints/preallocation results do not prove all later rewrite, metadata or snapshot costs. The application can serialize and limit its own requests, check available space and preserve exact roots; it cannot exclude unrelated volume consumption between checks or dishonest device flushes. [Apple File System Reference](https://developer.apple.com/support/downloads/Apple-File-System-Reference.pdf), [Apple fcntl/preallocation manual](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/fcntl.2.html).

Accordingly reject B alone as a production physical reservation. The durable grant broker, bootstrap reconciliation, qualified APFS growth model and safe owner-pin lifecycle must be resolved before admitted-source authority.

## Executable evidence and limits

A safe Rust dispatch oracle runs the reference and **14 seeded negative mutants**: skipped write debit; repeated-offset dedup; refund after partial failure; sparse-gap growth bypass; truncate bypass; SHM read-map bypass; 64 KiB mapping-rounding undercharge; forwarded SIZE_HINT; allowed temp open; delete without token; refund while unlinked object remains open; ignored recovery block; early registration free; partial vector debit. Every reference passes and every mutant is killed. Write/truncate cases assume an already validated file-slot capability; handle binding/generation validation is a separate gate. This is a bounded dispatch/lifecycle oracle, not an exhaustive native VFS or filesystem proof.

A serial C wrapper exercises actual public F3 capture/validation and current provisional source publication/lease/status. It meters selected methods, enforces finite role caps, suppresses hints/temp roles, and preserves native delegation. In its isolated process it unregisters the native `unix` entry and registers a forwarding entry under that same name, retaining the native object beneath it; this ensures public store operations use the observed path without duplicate names. Production would instead use the private non-default registration through its internal open helper.

Bootstrap uses an unlimited **test** grant. Normal cost samples use unlimited W to avoid changing the workload, with finite 64 MiB E, 128 KiB S and 65,536 M; the fault worker activates finite W. This does not prove the proposed finite bootstrap/candidate grant policy.

One matched workload: 1,000 distinct 4 KiB notes, a 12 MiB master and five source records; one neighboring environment-record edit; A lease remains exact after B. Native/fence A and B **canonical source manifests are byte-equal** and retained, with every input length/hash. The archive freezes all 24 source-crate Rust files, original Cargo metadata and a derived isolated workspace binding, alongside the fixture generator and exact dependencies. The final run used that frozen source copy; native/fence manifests also match the earlier live-workspace inputs. This permits regeneration without relying on mutable HEAD. Bulky SQLite databases/stages are removed. The fault workload uses a 4 MiB master.

| Single final sample | Native | Request adapter |
| --- | ---: | ---: |
| Fresh publication | 94.141 ms | 103.742 ms |
| Fresh explicit checkpoint call | 24.266 ms | 18.431 ms |
| Record edit publication | 32.028 ms | 35.530 ms |
| Edit checkpoint call | 0.529 ms | 0.670 ms |
| Whole-worker peak RSS | 48,349,184 B | 48,775,168 B |

Observed publication deltas are +9.602 ms fresh and +3.501 ms edit; these are single diagnostic samples, **not qualified overhead or speedup**. Capture/validation and manifest recording precede publication timing. Checkpoint timing excludes its plan/open preamble; the harness uses an additional serial diagnostic connection. RSS includes fixture, capture, manifest recording, validation and SQLite, not just the ledger. Native counter fields are null because that worker has no observer.

The adapter measured fresh total W **35,974,544 B**, E growth **35,951,040 B**, new S **32,768 B**, M **38**; the fresh WAL and main reservations coexist. Edit total W was **480,696 B**, E **167,936 B**, M **18**. These include explicit checkpoint activity; they are request accounting, not physical allocation. A 65,536-B W grant refused at **61,824 B** before the next payload, preserving selected/readable A. Reopened status was pending; an explicitly fresh unlimited *test* grant retried to B. Durable-credit recovery was not implemented.

The adapter is not the complete production design: it has serial static state, no durable grant/broker/quarantine, no complete slot-generation/escaped-control conformance, and no actual partial-kernel-write, process-kill or constrained-volume ENOSPC matrix. Those are missing gates, not passed evidence.

## Implementation/proof queue

1. Freeze safe vector ledger, fixed object/handle table and actor phase/binding types; integrate the oracle and all mutants. No production unsafe in this step.
2. Prove wrapper ABI/registration lifetime and prepaid cleanup against the pinned native VFS: partial-open cleanup, failed write without refund, failed close/quarantine, reentrancy/panic, SHM rounding/materialization, controls/atomic/mmap escapes and every denied file role. Require no native dispatch on refused requests. Then review production unsafe scope.
3. Integrate grant intents/bootstrap/restart reconciliation with exact source outcomes and pins. Kill before/during reservation, staging, final sync/commit and close; ambiguous outcome resolves by key. Verify corrupted recovery records cannot restore credit or expose usable bytes.
4. Qualify real-volume physical policy separately, including actual ENOSPC, checkpoint overlap/blocked readers, CoW/snapshots, recovery work, managed temp and the 8 GiB floor. Repeat meaningful edit/RSS/overhead measurements with the actual synchronized/durable implementation before promotion.

Reproduce the retained scratch with pinned offline release Cargo, then `python3 tmp/source-requested-io-fence/run.py`. `raw/` contains exact stdout/stderr, runtime compile options, workload manifests and per-worker commands/RSS. `build.rs` identifies the locally cached bundled header; another location/toolchain requires a new identity.

Runtime: SQLite **3.53.2**, source ID `2026-06-03 19:12:13 d6e03d8c777cfa2d35e3b60d8ec3e0187f3e9f99d8e2ee9cac695fd6fcdf1a24`; macOS 26.6.2 arm64/APFS, 16 KiB OS pages, Rust 1.99.1. Production store source is unchanged at SHA256 `053e11ca26bbcd7870b5501c0861311b36595e8474b76ee06e45b45e65db28eb`.

- Final native probe: `9e985cb2eb19df34ba3c5ab32ad808c4ac6b0b287cc3cc8d051a951d30f9cf8b`.
- Safe oracle executable: `3f45999460a6ef37c229c7477b8558937bb5876114506a26698a5b169b45695e`.
- C adapter source: `6cd51e830263dc1e6b784c9690f00166a84523974f5687dafa3083111cc5f7ca`.
- Safe oracle source: `e158ab8adfe9587f44787250cc039d58e55c27b88a580bf000e2f94e02430e38`.

Passed scoped release build, public API assertions, all mutant checks, scratch all-target Clippy/fmt, documentation checks and diff checks; no admitted durability or physical-space claim follows.
