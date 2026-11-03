# Phase-specific source space receipts — implementation design, 3 November 2026

This note specifies the next narrow F3/T2 diagnostic slice. It follows the
[cross-owner limit policy](HOLISTIC-LIMIT-POLICY-2026-11-03.md), the
[marginal preflight proof](../../evidence/i2a/SOURCE-MARGINAL-SPACE-2026-11-03.md),
the [APFS study](SOURCE-APFS-PHYSICAL-ADMISSION-OPTIONS-2026-11-03.md) and
C11/C12. It changes no source identity, disk threshold, accepted-store format
or publication rule. Its [stage-local portion](../../evidence/i2a/SOURCE-STAGE-SPACE-DETAIL-2026-11-03.md)
and [accepted-store portion](../../evidence/i2a/SOURCE-STORE-SPACE-DETAIL-2026-11-03.md)
are implemented; T2/F4 composition remains open. Neither the design nor
those local details are a physical-space guarantee.

## Name each quantity at its owner

The configurable `SourceSpacePlan` has four operational policies: main-file
target, WAL stop, managed-temporary target and desired observed free headroom.
The existing full-stage and marginal accepted-admission calculations are
*estimates of requested work*, not physical allocation bounds. `fs4` available
space is one non-atomic filesystem observation, not an entitlement or APFS
container-unallocated reading. SQLite `freelist_count × page_size` is reusable
inside SQLite and must not be added to volume availability. Current main/WAL
file lengths and logical page count have distinct meanings.

Use fixed-size, allocation-free typed values. Keep the public diagnostic
surface narrow: `StageError::space_pressure()` returns
`Option<StageSpacePressure>` and `SourceStoreError::space_pressure()` returns
`Option<StoreSpacePressure>` when implemented. The owner-specific types avoid
an unnecessary public all-resource registry; a later UI can wrap them in the
small C12 diagnostic envelope. Existing broad error kinds and exact operation
results remain unchanged. Each value identifies a stage and one of the following
phase-specific payloads. A store-local WAL guard can run during admission,
pin, release or maintenance; its stage must say `StoreWalGuard` unless its
calling operation is carried explicitly. Fields are named by their actual basis, so callers
need not infer one from another or interpret an absent value as zero.

| Refusal | Required values in the payload | Boundary |
| --- | --- | --- |
| Stage existing temporary scan | early-stop lower bound of charged managed-temporary bytes, selected temporary target | The scanner stops on the first excess entry, before a complete current-use total or stage projection exists. File charge rounds `max(allocated_size, file length)` and adds metadata allowance; directory charge is a granularity allowance. |
| Stage temporary preflight | complete rounded current managed-temporary charge, projected rounded complete-stage copy, selected temporary target | After a successful scan, before provisional copy. |
| Stage temporary final scan | early-stop lower bound of charged managed-temporary bytes, selected temporary target | Verification after all stage writes; still not a physical allocation total. |
| Stage copy projection overrun | actual raw bytes written so far, projected rounded complete-stage copy | The per-write counter crossed its capture projection; this is a separate invariant check, not a selected temporary-target crossing. |
| Stage available preflight | projected complete-stage copy, observed filesystem available bytes, selected headroom | Before copy. |
| Stage live available before/after write | next bounded write allowance (zero after a write), observed filesystem available bytes, selected headroom | Use the already performed availability query at the corresponding chunk boundary. |
| Accepted main target | accounted current main bytes (`max(file length, logical pages)`), estimated marginal main growth, selected main target | Before any candidate blob write. |
| Accepted WAL stop at preflight | observed WAL file length, selected stop target | Candidate preflight refuses when the existing file has reached the stop. |
| Store WAL allowance too large | next write allowance, selected stop target; no invented WAL file length | The shared guard refuses before stat when one allowance itself reaches the stop. It may be called outside candidate admission. |
| Store WAL checkpoint blocked | observed WAL file length after checkpoint, next write allowance, selected stop target, checkpoint busy result | The shared guard refuses after its checkpoint attempt. It may be called outside candidate admission. |
| Accepted available preflight | estimated marginal main and WAL growth, current logical-minus-file checkpoint debt, observed filesystem available bytes, selected headroom | Keep the estimate components and policy separate and checked. |
| Store live available | next write allowance, current logical-minus-file debt, observed filesystem available bytes, selected headroom | The shared guard uses the existing live query; do not label every invocation as candidate admission. |

An arithmetic overflow has no trustworthy requested byte count. Report the
existing overflow error with its operation stage; do not saturate into a
misleading receipt. An `ENOSPC` or other OS/SQLite failure is an observed
operation fault, not a failed preflight estimate. Keep its raw error and exact
old-or-new root/outcome recovery status. No preflight result promises that a
later live check or write will succeed.

## Implementation boundaries

1. Give `BudgetError` structured pressure variants or an equivalent small
   internal payload. Populate them at the calculation that actually refuses:
   `StageBudget` preflight, copy and final checks and accepted-store preflight,
   shared WAL guard and available-space checks. The early-stop scanner must
   report `AtLeast` for its subtotal, not manufacture a complete current total.
   Do not reconstruct numbers from display text.
   Keep `SourceStoreErrorKind::SpacePressure` and the existing broad stage
   error classification stable.
2. Reuse the already computed `fs4` sample, rounded stage estimate, SQLite
   page count and candidate hash inventory. Do not add an I/O query on every
   successful chunk or a second candidate scan merely for diagnostics. A
   receipt may be copied into an error, then formatted at the UI edge.
3. Ensure each payload states whether bytes are policy, estimated work,
   accounted current use or observed filesystem availability. Keep the
   accepted preflight's unique-hash reuse and incomplete-blob full-restage
   assumption visible; a future partial-prefix estimator gets a distinct
   revision/basis rather than silently changing the reported number.
4. T2 later composes *cooperating* F3 stage/admission, F4 materialization,
   compiler output, retained old roots and concurrent jobs per volume. An F3
   receipt is a local assessment, not a cross-process allocation grant. A
   successful assessment may be superseded by other writers before the next
   call; the live guard and exact fault recovery stay authoritative.

Do not put disk policy or receipts in `SourceRevision`, schema definitions,
`CookKey` or product bytes. A policy change can alter a refusal for the same
admitted byte tree without changing its exact source identity. The selected
F4 build may still fail for its own space or target support after F3 accepts
the source.

## Project temporary bucket across F3 and F4

The current F3 stage scanner deliberately walks the project `.engine` tree,
excluding only its top-level `source.sqlite` and `source.sqlite-wal` files.
It therefore charges retained F4 materialized input trees placed there,
alongside F3 provisional stages, against F3's selected managed-temporary
target. This is a project-bucket observation at an F3 capture boundary, not
an allocation reservation. Keep that existing scope while proving the next
cross-owner case. Narrowing F3 to `.engine/staging` alone would silently stop
observing known project scratch before T2 has another coordinator.

F4's current `BuildScratchPlan` checks one *new materialized tree*: its own
rounded projection, per-tree target and one observed volume-availability
sample. It does not scan sibling materializations or share F3's project
bucket. A first tree can remain owned while a second tree individually passes
the same 16 MiB target even when their aggregate `.engine` charge exceeds
16 MiB. F3 may then refuse a tiny new edit on its next temporary scan. That
is coherent local behavior but not a composed 16 MiB process or project
guarantee. Private Cargo cache and compiler output outside `.engine` are
absent from that scanner; the filesystem availability probe still sees their
allocation only as a non-atomic free-space change.

The next public-API proof uses an accepted A with an approximately 12 MiB
unchanged master. Close A's provisional stage after acceptance. Hold one A
pin and two distinct `MaterializedSource` owners under `.engine`, each with
an individually sufficient 16 MiB `BuildScratchPlan`. Record each actual
copied byte count and the rounded project charge; both F4 operations should
succeed while their combined charge exceeds 16 MiB. Try a tiny B scene edit
with F3's 16 MiB managed-temporary target: its whole-tree stage projection
remains about 12 MiB, but the existing scan should return a typed
`TemporaryScan` lower bound over the target before new stage publication.
Assert A remains selected and both F4 trees remain intact. Close both F4
owners, retry the same B edit, and verify the source identity is independent
of this operational refusal. A bounded image is unnecessary for this
ownership proof; a separate same-volume fault test is still required for
real copy/compile pressure.

T2 owns the later *cooperating* same-volume schedule and account. F3 supplies
the staged/admitted demand and actual source-store outcome; F4 supplies
build-input, product and retained-candidate requirements and lifetimes. T2's
prepared-environment and compiler-job owners supply their cache/tool/output
observations and lifetimes. T2 must count known retained scratch once,
recognize distinct output and cache locations, and compare projected new work with
fresh availability without subtracting already allocated bytes twice. It
must serialize or defer work when its selected operational policy cannot
support simultaneous jobs, while each owner still handles native write,
sync and cleanup faults. No T2 estimate certifies a physical APFS peak or
prevents an unrelated writer from racing it. The public-API proof above
defines the current asymmetry; it does not select a general T2 allocator or
claim F4 has implemented aggregate admission.
F4 owner retirement during an F3 scan is a separate interaction: the current
scanner tolerates a disappearing F3 staging path, but an independently
removed F4 build-input path can produce an ordinary I/O refusal. The
coordinator must serialize those cooperating lifecycle transitions or prove
safe bounded retry after an independently verified disappearance. F3 must not
delete F4-owned trees to force progress.

## Executable proof before claiming the slice

- At stage preflight, inject independent early scan, post-scan temporary-target
  and available-space refusals; assert lower-bound versus complete charges,
  policy and observed fields, and prove no provisional stage is returned.
  Exercise equality, checked overflow, before/after-write samples and final
  scan refusal without relying on a host's incidental free space.
- At accepted preflight, compare a hash-reused edit with a new blob under the
  same plan. Assert only missing/incomplete bytes enter projected marginal
  growth, and that main, WAL and available-space refusals name their own
  quantities including selected headroom. Cover an oversized allowance before
  WAL stat, a blocked checkpoint after stat, and a pin/maintenance WAL guard
  without falsely labeling those operations admission. Preserve the existing
  blocked-WAL retry and pinned A/B tests.
- Inject a real write/SQLITE_FULL or sync failure after successful preflight.
  Report it as an operation fault; verify selected old root or exact committed
  retry outcome after reopen, with retained old pins. Keep the uncertain final
  COMMIT quarantine rule.
- Record local duration, main/WAL lengths, filesystem availability samples
  and process RSS for small edits, a retained old pin, a large blob, many tiny
  files and blocked checkpoint. These are observations with host/configuration
  metadata, not calibration of an APFS hard bound. Compare the complete
  F3→F4 path separately before a ready-build claim.

The existing 52/2/2/8 GiB defaults remain configurable planning examples.
Their values need qualification against named workloads and storage devices;
the diagnostic contract must not promote them to engine-wide maxima or a
guaranteed amount of physically unallocated space.
