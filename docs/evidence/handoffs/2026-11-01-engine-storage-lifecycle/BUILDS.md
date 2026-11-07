# Build, cache and AI-run retention proposal

Status: design challenge for the 1 November 2026 storage review; no cleanup or implementation performed. Read with [repo storage lifecycle](../../../ASSET-LIFECYCLE.md), [asset storage](../../../ASSET-LIFECYCLE.md), [F4](../../../design/modules/WORLD-CONTENT.md), [T2](../../../design/modules/TOOLS-DELIVERY.md), and [AI performance](../../../design/AI-PERFORMANCE.md).

## Decision to make

The existing F4/T2 design gives immutable, dependency-keyed cooked products and exact candidate/build identity. It does not yet specify **disk retention authority**. Use a small storage controller owned by developer tooling, with F4 supplying product dependencies and runtime leases and T2 supplying build, evaluation and release roots. Broadsides' current Vite/TypeScript workflow remains distinct from the future engine cache. Do not move local scratch, Git worktrees, runtime payloads or LFS into one undifferentiated content store.

There are four different byte classes:

| Class | Owner and authority | Reclamation rule |
| --- | --- | --- |
| Authored source and selected masters | Git/LFS or recorded durable artifact store; factory package owns art | Never automatic cache GC; recovery must be verified separately |
| Pinned release/rebuild roots | T2 manifest and source/toolchain closure, with publication state | Explicit release-retention policy; preserve prior known-good and all required rollback roots |
| Reproducible shared cache | F4 cooked products, compiler/dependency cache where exact safe reuse is known | GC only when unreachable from protected roots and idle, under capacity policy |
| Mutable per-run state | Isolated checkout/build staging, intermediate captures/traces, temporary package assembly | Owner releases after result preservation and process exit; unknown ownership stays protected |

The first two are **retained records**, not merely cache entries with long TTL. Build provenance is metadata; a cache hit or matching hash proves identity of present bytes, not that source/toolchain/licenses are remotely recoverable. A same-disk archive is another local copy, not a backup. Git LFS pointer presence alone does not prove its object uploaded or can be restored.

## Reuse and isolation

Use one canonical product key over canonical input bytes, dependency product hashes, cooker/importer/schema versions, target triple/API/profile, toolchain/decoder/compression settings and every environment input that affects output. This follows F4's stated key. A key for a derivative must also include the generating algorithm and relevant options. If a tool is nondeterministic, produces undeclared inputs or cannot be validated, keep its output private to the run until this is resolved; do not advertise a shared hit. Verify product bytes and manifest closure on admission and on use after an unclean exit. Store immutable results by verified content identity and publish through an atomic manifest/root update; concurrent writers may race to compute but only one complete verified object becomes visible. No mutable hard-linked checkout files or shared mutable output directory.

The cache can be shared across **compatible** Broadsides and custom-engine work only where the key and byte contract truly match; merely pointing both at `node_modules`, Vite's local cache or a GPU tool directory is unsafe. Keep one dependency/tool download cache per compatible lockfile/toolchain/platform, but separate its mutable install/build state per worktree or candidate. Vite's `dist/` is disposable output; `game/public/` is selected deployable content and can inflate every build. A candidate must own its source checkout, compiled output and staging namespace. It may read immutable verified F4 products and locked dependency downloads. That avoids full source/library duplication without letting candidate edits contaminate evaluators or neighbors. Cache identity is independent of candidate identity; evaluator identity and actually loaded binary/shader/product IDs remain recorded by T2.

## Roots, leases and accounting

Protect at least: active source checkout with uncommitted or ignored unique inputs; active build/evaluator jobs and their transitive inputs; currently served previews; current selected release, prior rollback release and explicit retained review candidates; required tests/replay/visual evidence; adopted asset masters. An AI campaign's append-only record is a small durable summary and lineage pointer. Bulk failed candidate builds and raw trace/capture sweeps are short-lived unless they reproduce a named failure or are required to decide a live review. Do not keep one full worktree and output set per rejected idea. Keep compact failure code patch or commit, commission/evaluator versions, failure reason, sampled measurements and any minimal reproduction. If a rejected candidate contains unique authored source, first preserve it deliberately or record why the user discarded it.

Each job reserves estimated peak bytes **before** starting: checkout/materialization, parallel compiler/cook outputs, unpacking, old/new staging overlap, captures and package assembly. Account for real allocated disk bytes as well as logical bytes; filesystem cloning, compression, APFS snapshots and hardlinks can make `du` and free-space movement differ. Set a total local cap plus per-campaign/per-job caps and a free-space floor sized for other work. Admission should reduce concurrency or output scope when projected peak breaches the floor; GC cannot promise immediate free bytes if live work still holds files. Use high/low watermarks to avoid repeated cache churn. Report cache hit/miss, reserved/actual peak, protected bytes, reclaimable bytes and current free space without claiming those quantities add perfectly.

Protection uses explicit owner/job IDs and expiring heartbeats for *temporary* leases, plus persistent roots for release and selected work. Lease records include host identity, machine boot identity, PID/start identity when available, path, epoch and deadline. A timeout or reboot moves a job to **suspect**, not directly to deletable: inspect process/consumer state and worktree changes, then require owner acknowledgement or a conservative recovery policy for unknown owners. An active process may be unable to heartbeat during system sleep; clock changes and PID reuse must not make its files collectible. Runtime F4 CPU/GPU leases/fences and local disk leases are separate lifetimes; neither substitutes for the other.

Garbage collection is mark–quarantine/recheck–release. At a consistent root snapshot, mark reachable immutable objects and active mutable namespaces. Select only known owned, idle, unpinned, validated disposable paths; immediately before release, recheck generation, leases, process state, source cleanliness, ignored/untracked inputs and manifest reachability. Reclaimed immutable objects are safe only if no new reader can acquire them without a lease or atomically repopulating them. Crash recovery scans incomplete staging, verifies completed immutable blobs, and leaves ambiguous worktrees/inputs for owner review. Failed deletion, partial files and newly acquired leases must leave a coherent manifest, not a half-published build.

## Promotion evidence and workflow

Before a candidate is promoted, T2 records exact source/dirty state, evaluator revision, target/profile/tool lock, build and actually loaded artifact IDs, gate results and limits. Preserve the minimum reproducible result and compact raw evidence needed to audit the claim; a full trace or all captures are not automatic permanent roots. For a selected release, preserve publication manifest, selected runtime payloads, rollback root, source/recipe/tool closure and licenses. Verify an independent cold restore of required LFS/external masters and a clean rebuild where the claim is source-rebuildable. Reproducible build is a tested claim; nondeterministic outputs can still be retained with exact artifact identity and scoped explanation. Do not infer visual acceptance from format or performance checks.

Current Broadsides work should use the existing storage lifecycle manually: inspect the job's own outputs before generation and at handoff; estimate peak; reuse canonical inputs; keep a representative capture and selected result; record large retained scratch with owner and cleanup trigger. `tmp/` and ignored paths are not automatically disposable, and a clean Git status does not clear them. Preserve active or unknown worktrees, including the documented legacy `.tmp/` checkout. The repo's `clean` script is not a policy engine. Add no automatic project-wide cleanup until roots and ownership can be proved.

## Minimum v1, then only proven extensions

1. Inventory *top-level* output classes and owners (without recursive per-candidate hashing); give every new run a unique owned staging path and short manifest with source/evaluator/config identity, status, byte budget and expiry/review trigger. Existing unknown paths remain protected.
2. Admission check for free-space floor and estimated peak; cap parallel builds/captures. Record actual peak and selected retain set at handoff.
3. Share only immutable verified F4 products and already-safe dependency downloads by full key. Keep mutable build/install output isolated. Pin release/rollback roots and active jobs explicitly.
4. Offer a dry-run GC report with reason and owner for each candidate path. Initially reclaim only the invoking job's known scratch after its owner marks it complete; add cross-job collection after lease and crash tests.
5. Retain compact campaign records; expire failed bulk after a bounded review window, with explicit exceptions for named reproductions and unresolved user review.

Do not start with a global CAS for Git, LFS, `node_modules`, images, traces and cooked payloads. Cross-filesystem deduplication, distributed cache, remote leases and archive tiers are later options only if measured disk use and recovery needs justify them. The v1 controller should first prove: two concurrent writers do not publish a partial product; a crash cannot collect a live/dirty worktree; a release still loads after cache GC; cold restoration retrieves selected masters and toolchain; and disk pressure stops new bulk work before consuming the shared floor.

## Failure modes to challenge in design review

- **False recoverability:** hash exists, LFS pointer exists, or archive exists, but source/licensed inputs/toolchain cannot be fetched and rebuilt.
- **False cache hit:** key omits backend, compiler, profile, decoder, environment or candidate source, yielding stale or mismatched binaries/shaders.
- **Overretention:** every failed AI trial pins full checkout, cooked products, raw traces and captures forever; quota only moves the blowup elsewhere.
- **Unsafe reclamation:** TTL expires during sleep, PID reused, active preview or GPU/CPU consumer exists, ignored master sits in a clean worktree, or a new release gains a reference between mark and deletion.
- **Misleading savings:** same-disk archive/snapshot and logical deduplication do not free physical space; build staging and packaging peak exceeds steady-state size.
- **Measurement contamination:** concurrent cache eviction/build I/O distorts T2 A/B results; measurement lease must cover contended disk resources as AI-PERFORMANCE already requires.
- **Policy collapse:** conflating Codex conversation records with repository worktrees or assuming app chat deletion retires files. They have separate owners, byte costs and lifecycle; repository GC has no authority over app records.
