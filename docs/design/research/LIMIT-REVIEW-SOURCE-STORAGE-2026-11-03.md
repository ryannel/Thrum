# Source/build/storage limit review — 3 November 2026

## Recommendation and scope

Retain the selected first-profile policy: one SQLite authority for exact admitted bytes/root/outcome, immutable source leases, conservative append-only retention, configurable disk planning and recovery from real storage faults. Requiring a custom APFS VFS or a guaranteed physical 8 GiB floor would add a different, currently unsupported promise. The present code is not yet a qualified whole source-to-ready-build implementation. No P0/P1 corruption defect was established in this bounded review; the concrete corrections below are P2.

Inspected working HEAD `e3ce277492e3e43e1b48e214d26d703a4d4a421c`, with unrelated changes to CI and CONTRIBUTING present and untouched. Read AGENTS/README/build-plan I2a, both local engine skills and their review/performance references, C10/C12, Foundation sections 1–4, the two commissioned limit notes, and the production source capture/budget/manifest/store and F4 scene/materialization/candidate consumers. This is independent challenge after exposure to the selected policy, not a blind architecture derivation. Static review and current primary-source research only; existing tests below were inspected, not rerun. No platform durability, peak or throughput measurements were performed.

## Findings

### [P2] Incomplete-blob retry escapes the bounded-transaction mechanism

**Location:** `crates/thrum-source/src/sqlite_store.rs:1703`, `:1722` (`begin_blob`); compare Foundation `:156`.

After a process dies with many committed chunks of a large `state=0` blob, retry calls `ensure_wal_room(1 MiB)` and executes `DELETE FROM blob_chunks WHERE hash=?1` for the entire partial blob in one IMMEDIATE transaction. The later four-MiB append loop does not bound this cleanup. A legal large binary can therefore force one transaction to traverse/free its whole partial representation while the admission/core lock is held. Changed-page volume depends on SQLite layout and secure-delete configuration; do not assume it equals blob payload size, but neither is it bounded by the one-MiB allowance. The WAL cannot reset midway through this transaction. This is a code correction, not an argument for a hard physical quota or an established loss of the old root.

**Correction:** Resume validated matching chunks, replacing/refilling at most a bounded chunk per transaction before final full-blob verification; or introduce a recoverable bounded cleanup cursor and finish cleanup before restaging. Retain state=0/nonselectability throughout. Account cleanup work in its own estimate. A new tombstone framework is unnecessary if same-hash deterministic resumption suffices.

**Discriminating proof (integrator clarification after closure review):** In one live owner, admit A, hold its read pin and interrupt/cancel B after 1, 16 and hundreds of four-MiB chunks; retry B under the same owner with a low configurable WAL stop. Separately kill the owner mid-B, reopen, verify A root/outcome/bytes, reacquire an A pin, then retry B. Repeat interruption during cleanup/resume. Record maximum SQL transaction duration, touched pages, WAL delta and live pin-read progress. Require A to remain selected until B's single final root/outcome commit, no selectable partial blob, stable retry identity, and work per cleanup/resume slice independent of total partial blob length. A live lease cannot survive killing its owner under the selected exclusive-owner profile. SQLite explicitly describes large transactions as a source of large WALs and cannot checkpoint them partway through. [SQLite WAL, section 6](https://www.sqlite.org/wal.html).

### [P2] The two admission estimates can reject a cheap reused edit for unrelated maximum allowances

**Location:** `crates/thrum-source/src/budget.rs:217` and `:225`; `crates/thrum-source/src/sqlite_store.rs:1434`. Already acknowledged, without closure, in the holistic policy `:153`.

Capture charges `2 × entire source byte total + 64 MiB + metadata` as additional SQLite main growth before consulting any blob reuse. An approximately 51-GiB retained main plus a one-GiB current tree is refused against the default 52-GiB main target even when the next edit adds only a few bytes and the one-GiB staging copy fits the temporary plan. The accepted-store estimate does deduplicate complete hashes, but then charges `wal_stop - current_wal` in full. Raising a WAL stop allowance can consequently make an unchanged tiny operation require far more free space and fail. Source bytes and actual proposed work did not increase.

These are conservative early refusals, not atomicity failures. They nevertheless obstruct the intended iteration workflow and conflate a configurable backpressure ceiling with predicted work. Capturing the full dirty tree may remain necessary; that does not require pricing a second full tree as new SQLite data.

**Correction:** Give provisional capture its actual staging demand and a clearly labelled fallback estimate when the store is unavailable. At accepted admission, expose missing unique blob bytes, metadata allowance, likely WAL work and existing checkpoint debt separately; use a calibrated marginal WAL estimate bounded by the selected work policy. Keep live checks and exact storage-fault recovery. Do not subtract existing freelist pages as guaranteed returned volume space.

**Proof:** Seed a retained store close to the main target; compare a no-op capture, one-byte edit, changed large master and many tiny files. A larger unused WAL allowance should not alone increase the same marginal-work estimate. Check identical hashes counted once and unchanged SourceRevision under pure operational-policy changes. Capture and admission must report the different estimate classes rather than imply that either physically reserves space.

### [P2] Known simultaneous build/scratch claims are still uncoordinated

**Location:** `crates/thrum-cook/src/native_worktree.rs:150`, `:175`, `:218`; `crates/thrum-source/src/sqlite_store.rs:2302`; holistic policy `:57` and `:128`.

Materialization probes available space once before copying and writes through `PinnedSource::copy_file_to` without a free-space check in its output writer. Its scratch plan describes only that materialized input tree. The accepted store's exclusive sidecar lock coordinates one store, while capture has a different project lock; neither represents claims from other stores, materialized compiler inputs, compiler outputs or retained build candidates on the same volume. For example, two cooperating two-GiB copies can both pass against 11 GiB free with an eight-GiB headroom target, then leave seven GiB. This is a known implementation/composition gap, not a new requirement to defend against arbitrary external writers.

**Correction:** Implement the narrow T2 per-volume claim coordinator promised by the design, using owner-scoped lifetimes and known input/output/scratch estimates. Serialize heavy work when it cannot be bounded credibly. Pass the claim/live-probe guard into the streaming output writer and compiler supervisor; include compiler output and candidate retention before launching a build. A small coordinator shared by actual tool consumers is sufficient; no public generic limits crate is justified.

**Proof:** Start two same-volume materializations/builds with individually affordable plans whose sum is unaffordable. One must defer before allocating its large output, and cancelling/failing it must release only its own claim. Consume free space externally after preflight and inject destination ENOSPC mid-copy; the partial materialization is unusable/cleaned or explicitly reported, the source pin and old ready build survive. Measure total simultaneous peak separately from each job's estimate.

### [P2] Required admission diagnostics cannot yet be obtained from the public result

**Location:** `crates/thrum-source/src/sqlite_store.rs:2071`, `:2081`, `:2096`; Foundation `:146`, `:154`, `:156`; C12 diagnostics contract.

`AcceptedSourceRoot` returns only generation/revision. `SourceStoreUsage` returns non-atomic current lengths/free space, while `SpacePressure` and display strings omit requested/allowed values, the selected plan/profile, estimate breakdown, peak/overrun and a stated adapter commit guarantee. SQLite flags are set and read back internally, which is useful, but callers cannot expose the promised durability qualification or distinguish a marginal shortage from a WAL stop/blocked checkpoint with quantitative next action. The policy correctly calls projected-versus-observed reporting incomplete; this is a concrete API/implementation gate before claiming that diagnostic behavior.

**Correction:** Add a small store-owned admission/diagnostic receipt, not another resource authority. Include estimate class/dimensions and observed peak collection scope, checkpoint result/debt, selected plan, and VFS/filesystem/durability qualification. Keep exact SourceRevision and product hashes independent of this operational receipt. Do not infer a power-loss guarantee merely from a successful PRAGMA readback.

**Proof:** Exercise main-target, temporary-target, live-free-space, blocked-checkpoint and actual SQLITE_FULL refusals and check structured quantities and supported retry advice. Verify the root/outcome remains exact after an uncertain COMMIT and that a later diagnostic/checkpoint failure cannot turn committed acceptance into rejection.

## What is sound and should remain

- Root generation CAS, manifest completeness and exact keyed outcome share the final transaction (`sqlite_store.rs:1988`); unsuccessful staging does not select source. The final-COMMIT quarantine disables subsequent SQL/read publication and close-time/autocheckpoint side effects until fresh open. Existing tests cover both committed and absent resolutions. SQLite documents that FULL/I/O errors may leave different transaction states, supporting explicit outcome recovery rather than guessing from the error. [SQLite transactions](https://www.sqlite.org/lang_transaction.html).
- Accepted pins bind exact manifests, copy in bounded chunks and require a successful hash/length completion barrier before consumers use output. CPU parsing/output writes occur outside individual SQL chunk reads. Current append-only complete blobs make those separate reads coherent; enabling deletion requires the designed publisher/pin/deleting-state protocol first.
- Keeping all verified blobs/outcomes is a valid simpler initial retention policy under finite refusal. Do not activate mark-only deletion to recover space. Undo-only expiry cannot evict source/build/save/release pins. The complete durable ready-build root and its stale/failure protocol remain open; the candidate report explicitly says `build_ready=false`, so it is not evidence of completed publication.
- Identity scopes are coherent: exact source bytes/manifest, typed cook meaning/dependencies and actual product byte hashes answer different questions. A hashed definition that contains a capacity will change when that capacity changes; splitting external processing capacity from semantic definition is an optional future cache-policy improvement, not permission to omit dependencies now.
- Logical freelist reuse, apparent file length, allocated blocks and volume space remain separate quantities. The rejected hard APFS floor must stay rejected. Apple's current capacity API distinguishes important from opportunistic storage, but neither API specifies an allocation reservation or bound on SQLite/APFS peak. [Apple capacity guidance](https://developer.apple.com/documentation/foundation/checking-volume-storage-capacity), [Apple APFS overview](https://developer.apple.com/documentation/foundation/about-apple-file-system). The older archived APFS guide explicitly redirects current users; its atomic safe-save description is not proof of arbitrary multi-file dirty capture.

## Credible alternatives and selection evidence

| Alternative | When it is simpler/useful | Consequence for Thrum |
| --- | --- | --- |
| Selected single SQLite chunk store, destructive GC disabled | One durable authority and one transactional root/outcome boundary; no separate file/database flush race. | Retain for the first authoring profile. Qualify copy/log amplification and retry work, not an invented physical cap. |
| SQLite rollback journal plus short serialized transactions | Fewer WAL/checkpoint policies for a very small single-owner tool. | Credible comparator if concurrent pinned reads have little value; readers/writers contend differently and it does not eliminate retained-copy cost. No measured reason to replace the current working WAL path. |
| External content-addressed immutable files plus SQLite metadata | Potentially attractive for large master I/O and avoiding SQLite overflow-page/log traffic. | Adds file/directory sync, publication ordering and GC recovery across two storage authorities. Compare only with the complete crash protocol, not a bare rename benchmark. |
| Verified immutable Git tree plus retained resolved LFS closure | Lean clean-CI/handoff boundary where all required bytes and their retention are already guaranteed. | Does not solve dirty authoring or unavailable LFS; Git HEAD is not the complete admitted working set. Useful adapter, not a general replacement. |

SQLite's official internal/external BLOB comparison finds different tradeoffs for small and large values, but its large-BLOB experiment used 2011 SQLite/ext4 and a 100-MB database; it does **not** select four-MiB SQLite chunks on current APFS. Measure representative Thrum reads/writes before optimizing away its useful transactional simplicity. [SQLite BLOB comparison](https://www.sqlite.org/intern-v-extern-blob.html).

The OSDI 2014 ALICE/BOB paper and talk show that application crash protocols depend on filesystem persistence properties. This supports explicit flush/order/fault testing and scoped qualification; its Linux results do not certify or invalidate current APFS. Rust also documents ignored close errors and that an open `File` does not prevent external mutation, supporting completion checks and the completed-dirty-set declaration. [OSDI paper/talk](https://www.usenix.org/conference/osdi14/technical-sessions/presentation/pillai), [Rust File](https://doc.rust-lang.org/std/fs/struct.File.html).

## Executable proof queue

Existing focused regression commands (unrun in this review):

```sh
cargo test --offline -p thrum-source accepted_store_quarantines_after_uncertain_native_final_commit
cargo test --offline -p thrum-source native_store_sqlite_full
cargo test --offline -p thrum-source accepted_kill_after
cargo test --offline -p thrum-source blocked_reader_keeps_wal_checkpoint_debt_visible
cargo test --offline -p thrum-source candidate_space_preflight_refuses_before_blob_write_and_counts_reuse_once
cargo test --offline -p thrum-cook admitted_pins_cook_exact_scene_and_failed_replacement_keeps_prior_product
```

Next add the four discriminating cases above to the actual accepted-store/materialization paths, then inject failures by ordinal at each write/sync/checkpoint/final-commit boundary, both one-shot and persistent. Reopen and assert the selected root plus matching outcome together, all pin-required bytes, exact retry and no partial ready publication. Include crashes during recovery itself. SQLite's own anomaly testing uses ordinal and persistent I/O fault sweeps; adopting that method does not inherit SQLite's test results for Thrum. [SQLite testing](https://www.sqlite.org/testing.html).

Also run one combined profile with many tiny files, dense bounded JSON, a large binary, an old pinned revision, a replacement candidate and a compiler build. Report end-to-end elapsed tails, logical/allocator memory, main/WAL/temp peaks and source-to-product identity. The 100,000-entry metadata/preflight microprobes and the 1,000-small-BLOB helper benchmark are useful isolated evidence, not this combined qualification.

Current dependency check: locked `libsqlite3-sys 0.38.2` bundles SQLite **3.53.2** (local amalgamation version macro). The official 2026 WAL-reset warning concerns versions through 3.51.2 and is fixed from 3.51.3, so it is **not a finding** against this pinned bundle. Preserve that qualification when changing the dependency/VFS. [SQLite WAL-reset notice](https://www.sqlite.org/wal.html#walreset).
