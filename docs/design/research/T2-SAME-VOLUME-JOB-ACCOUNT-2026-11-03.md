# T2 cooperating same-volume job account — provisional design

3 November 2026. This resolves the next F3/F4/T2 coordination boundary in the
[cross-owner policy](HOLISTIC-LIMIT-POLICY-2026-11-03.md), [source receipt
design](SOURCE-SPACE-RECEIPT-DESIGN-2026-11-03.md), and C05/C10/C12. The
[two-materialization proof](../../evidence/integration/T2-SAME-VOLUME-MATERIALIZATION-2026-11-03.md)
demonstrates the current asymmetric local limits. This is an implementer design,
not a physical APFS allocation guarantee or an implemented coordinator. F3
source identity and root/outcome remain independent of these operational values.

## One host and one filesystem domain first

The first profile has one T2 host process coordinating its cooperating F3 and F4
jobs, with one exclusive host owner and a fixed, discoverable private T2
job-root layout outside the entire source project tree but on the selected
filesystem domain. Register an immutable job-incarnation token, root/allocation
identity, source/build generation, resource class, intended child process group
and state in a small durable record before
any copy or compiler launch. Create the owned root first, then atomically write
and sync its record and parent; a crash between those steps leaves a
discoverable unpaired root. After verified tree cleanup, sync the parent
before removing the record. `MaterializedSource::drop` currently ignores
removal failure; the T2 owner must use an explicit checked close and retain a
failed-cleanup record. At startup, enumerate all owned job roots and
records before admitting work. An unpaired root, incomplete registration or
unresolved compiler family is quarantined and retains its conservative claim;
the host refuses overlapping new work until inspection/recovery resolves it.
Reconstruct claims by keyed replacement from durable phase records and a
complete inventory, never by adding a root scan to an existing charge. For a
known unresolved root, keep at least its durable full phase claim and any
inventoried overrun. An orphan with no trustworthy phase bound has an unknown
amount, which blocks the domain or overlapping work until recovery; do not
invent a finite zero or default claim. Repeated inventory is idempotent.
PID reuse is not evidence that an earlier child is gone. This makes restart
inventory possible for the first host, without claiming transitive containment.
T2 identifies the destination filesystem domain from opened roots, not
path spelling. It refuses an unaccounted destination, cross-volume move, alias,
or changed root identity before work. The account covers selected store, stage,
materialization, private cache, compiler output, temporary files and retained
candidate roots on that domain. A second host process is **not** coordinated by
an in-memory lock; cross-process use requires a shared owner registry, atomic
registration, recovery and reconciliation before it can claim this policy.
Unrelated OS writers can always consume space between observation and write.
Direct F3/F4 calls that bypass this first host are outside its cooperating-job
promise, even if they write to the same volume.
At host open, derive the expected canonical source-store path from the selected
live F3 store and validate its placement under the selected project. Before
charging, registering or copying a materialization root, compare the F3 pin's
read-only source-store path with that binding. Do not infer a fixed database
basename from the project directory: F3's native store API does not require
one. Two projects can have identical accepted root bytes and generations;
neither proves store ownership. This path is an operational binding held by the
live F3 owner, not a source-content identity or protection against external
path replacement. Preserve the F3 pin while F4 reads or copies its bytes.

T2 owns scheduling and the account, not source or build authority. F3 retains
exact admission, pin, WAL/checkpoint and recovery decisions. F4 retains verified
input/product/build ownership, compiler lifecycle and ready publication. Each
job supplies a checked plan for *remaining incremental retained growth* in its
own phase, the paths it owns, and an idempotent release event keyed to the exact
job-incarnation token. Source/build generation is not an owner identity: two jobs
can use the same generation, and a delayed release cannot match a reused job ID.
A rewrite's raw I/O bytes do not equal new retained growth. T2 records a claim
by job incarnation, source/build generation, filesystem domain, normalized
root/allocation identity with one exclusive owner, resource class (managed
temporary, main/WAL, compiler/output or other), phase generation, owner-report
version and estimate basis. A complete per-class job plan is one atomic
compare-and-register operation; refusal exposes no component claim. Claim and
release receipts name the exact token and subclaims; duplicate release is
idempotent. Claims are operational estimates;
owners still guard writes and syncs and preserve the prior complete state on
failure. A closed/failed job cannot release another job's claim.

| Owner | Current use and remaining work to expose | Release boundary |
| --- | --- | --- |
| F3 stage/admission | Stage copy, missing accepted blobs, main/WAL growth and checkpoint debt; account for existing allocated main/WAL once. | After durable outcome or exact failed/recovered state and owned scratch cleanup. A live accepted pin retains required blobs. |
| F4 materialization | Distinct retained input trees and incremental copies, product files, candidate build roots and receipts. | Each lease/owner closes and cleanup is verified; a ready root retains its required closure. |
| T2 compiler environment/job | Private cache and retained tools already present, remaining compiler output, TMPDIR and intermediates, child/supervisor writes, plus candidate output. | Compiler family exits, receipt and output are resolved, and owned scratch cleanup is verified. Unknown descendants remain unresolved. |

The project managed-temporary target compares complete retained-root charges
and **one charge per active managed-temporary job** against the selected target.
Main/WAL and compiler output do not enter that bucket. F3 must expose a
disjoint per-root charge or an equivalent registered-root exclusion before T2
can replace its opaque `.engine` scan; simply adding a claim to that scan can
double-count a partly built F3 stage. Until that integration, the scanner
remains F3's local guard and T2 reports any conservative aggregate deferral.

At a stable owner-reported handoff, an instrumented job charge is observed
current root use plus its revised remaining growth estimate. During an
in-flight/unreported write, retain at least the previously admitted full
phase charge; do not decrement it from raw bytes written or a racing scan.
For the un-interposed compiler, retain the full phase claim until the family
has stopped and output has been inventoried; any observed overrun triggers
supervisor cancellation/quarantine. An active root is **not** also summed as a
completed root. A conservative charge may exceed actual simultaneous use and
defer otherwise feasible work; the receipt must say so. Checked overflow
refuses before work. The target is configurable scheduling policy, not a
physical quota.

For observed available space, already allocated bytes are reflected in the
filesystem sample. Compare it with other jobs' reported remaining growth and
the proposed incremental growth plus selected headroom. A full un-interposed
compiler claim may include bytes already written, making this comparison
conservatively high until completion; never call that a physical reservation
or an exact remaining-byte count. F3's main/WAL policies remain separate;
SQLite freelist is not free volume space. Scan registered roots outside the
account mutex, pair the result with an owner-reported/versioned claim snapshot
and a fresh availability sample, and retry a changed cooperating epoch. An
unreported write or unrelated writer can still race both observations; retain
the uncertain claim and owner-local live checks. Record each input quantity,
estimate class, sample time, filesystem domain and conservative-overcount
reason in a typed receipt.
Keep class charge, retained allocated bytes, incremental-growth estimate, full
uncertain compiler claim, observation age and unknown quantity as distinct
fields. Policy deferral is distinct from native I/O failure or a physical-space
guarantee.

## Transaction and lifecycle

1. Snapshot registered roots and their owner-reported charges outside the
   account mutex, then acquire T2's short per-domain mutex. Pair that
   snapshot with
   the current claim version and cooperating domain epoch, take a fresh
   free-space observation within that serialized comparison, and compare
   the complete simultaneous plan. Retry if the version changed. Reserve a
   claim or return a typed deferral/refusal with
   requested/allowed/observed quantities. Release the lock before copying or
   compiling. Do not hold it for a whole build.
2. Owner handoffs name the exact job incarnation, phase generation,
   monotonically increasing owner-report version and domain epoch. Reject
   delayed reports or releases from an old phase or incarnation before
   replacing a claim. Separate cancel request, compiler-family stop, output
   inventory, verified cleanup and retained-root transfer. A stop or cancel
   never refunds; verified cleanup also requires no live reader or required
   retained root. Failed cleanup keeps a durable unresolved claim. F3 and F4
   perform owner-local preflight, before/after-write live checks and
   fault handling. Their instrumented stage/store/materialization copy paths
   reconcile completed chunks at short lock boundaries. An overrun requests
   more claim before the next instrumented chunk or defers/stops at an
   owner-safe boundary; it never silently borrows another owner's allowance.
   Compiler output is not chunk-interposed: admit a phase estimate before
   launch, sample actual growth during supervision and cancel on observed
   overrun. Output can exceed the estimate before detection. An unknown demand
   defers or refuses rather than pretending the estimate is a hard write cap.
3. Keep F3's current writer window and `.engine` scans intact. The T2 host
   requires every temporary F4 materialization destination, compiler output
   and TMPDIR to be under a registered private T2 job root outside the
   **entire source project tree**, on the same verified filesystem domain.
   For the first macOS compiler sandbox, put `TMPDIR` beneath that job's
   compiler `OUTPUT` child: the write grant covers only `OUTPUT`, not the
   whole job root or adjacent registrations. The [external-output A/B
   fixture](../../evidence/integration/T2-EXTERNAL-COMPILER-OUTPUT-2026-11-03.md)
   exercises this placement and its unready guard-loss path, without a T2
   account or durable registration.
   This prevents F3's declared-tree inventory and managed-temporary scanner
   from seeing a partial F4 copy, and removes F4 retirement from F3's scan
   lock order. Register the input subdirectory before copy; hold its F4
   materialization owner through compiler reads and verified cleanup. Keep
   the durable job registration outside any subtree
   `MaterializedSource::drop` can delete. T2 counts those external roots'
   current charge and remaining claims once alongside the F3 `.engine`
   charge. A later ready-product/cache root placed inside `.engine` needs its
   own lifecycle and account design; temporary-root disjointness grants no
   exemption to it.
4. Never hold T2's account mutex across an owner callback, file copy, compiler
   wait, F3 source-window/store lock, F4 lease/owner lock or a full filesystem
   scan. Owners prepare plans without the account mutex, then use a versioned
   nonblocking claim attempt; if an owner state or observation epoch changed,
   release and retry. F3 retains its writer window across the complete
   capture, including scans. T2 never waits for that window while holding its
   mutex. F4's external temporary-root retirement needs no F3 lock.
5. One background compiler writer is the first scheduling policy, not a
   semantic limit. Foreground source work may defer or cancel speculative
   compilation when space becomes tight. Cancellation never drops a source pin
   or a ready build. If a compiler family, write, sync, cleanup or receipt state
   is uncertain, quarantine its claim and roots until inspection/recovery;
   do not refund an estimate on process exit alone. An observed detached child
   remains a release fault under the selected compiler-family trust boundary.
6. Publish only after owner-local completeness and the required durability
   barrier. Release claims at verified cleanup or transfer them to retained
   ownership. Receipt failure after publication must report the committed
   outcome, not recast it as a failed source/build operation.

The coordinator can reduce avoidable simultaneous work, but cannot promise an
APFS physical high-water, protect against external writes, or make a compiler
family transitive OS containment. Source/store and F4 retain native ENOSPC,
I/O, uncertain-COMMIT and crash recovery obligations.

## Cost and observation path

The fast path must not repeat a complete project or compiler-output tree walk
for every copied chunk. F3 already scans `.engine` at stage preflight and final
verification; T2 reuses its owner-reported result and must not add another
full scan to each F3 copy. F4's manifest preflight already visits every input;
T2 reuses its checked projection. An instrumented output writer can report
completed chunk growth during the existing copy stream without copying the
payload again. The account mutation and same-domain claim comparison touch
only active jobs and their fixed resource-class totals. One background
compiler writer keeps that first-profile set small; the policy still records
the actual count, lock wait and work rather than assuming an O(1) wall time.

Registration and phase/cleanup state transitions are durable. Per-chunk
progress need not fsync a job record: after host restart, the initial full
phase claim and discovered roots conservatively reconstruct outstanding work.
Full root inventory is required at startup, after uncertain cleanup and before
final release; active compiler output sampling runs off the source-admission
critical path and is a diagnostic/cancellation trigger, not exact metering.
Choose sample cadence from measured output growth and cancellation latency;
no fixed polling interval makes a hard cap. A recent owner-reported sample may
avoid a redundant probe only while its filesystem domain and scope match and
its age is reported. Native before/after-write guards remain at each owner.

Measure projected versus observed bytes by class, conservative overcount and
its resulting false deferrals, writer/claim lock hold and wait tails, per-file
and per-chunk CPU cost, preflight/final-scan time, compiler-output sampling
cost, cold restart inventory, and source edit-to-ready latency. Run both a
large master and many tiny files with a retained A build and B replacement.
If the account becomes the edit critical path, first remove duplicate scans
and hash work; do not relax exact source/ready publication to improve latency.

## Rejection-oriented proof queue

1. Two individually affordable F4 copies whose *combined remaining* plan is
   unaffordable: second defers before output creation; cancelling either frees
   only its own claim. Reopen after restart and detect any retained roots before
   accepting fresh work. Repeat with F3 B admission while compiler A sleeps.
2. Race F4 external-root construction, close and retirement against F3's
   preflight/final scans. Assert F3's declared inventory and `.engine` scan
   never see the F4 partial tree, while T2 counts it exactly once. Show A pin
   and old ready build survive B refusal. Include same-project-path refusal,
   aliases/cross-volume, unknown-output-root refusal, and concurrent capture,
   pin release and retirement without deadlock.
3. Consume free space externally after preflight, then inject native ENOSPC
   during materialization and compiler output, plus sync, cleanup and receipt
   faults. Require typed operation result, no partial ready publication,
   exact source outcome, retained required inputs, and no premature claim
   refund. Fault a child that outlives its supervisor.
4. Measure one combined many-small-file/dense-JSON/large-master, old-pin,
   replacement and compile workload. Report estimated versus observed
   main/WAL/temp/output peaks, free-space samples, stage/build elapsed tails,
   and end-to-end identity. A policy default is revisited with those data;
   isolated microprobes and logical byte accounts do not certify it.

This design stays provisional until F3/F4/T2 owners reconcile root placement,
receipt fields and the first host's job lifecycle, then implement and run the
fault cases. No cross-process or device qualification is inferred.

## Implemented first F4 input slice

The Unix `thrum-jobs` host now owns one synchronous F4 input phase per call.
It receives a live selected F3 store owner and canonical project root, checks
the store path is inside that project, and compares every pin's issuing store
path before charging or creating a job root. The binding is operational; it
does not authenticate a same-user path replacement. The private external job
parent is the deterministic sibling `<project-name>.thrum-jobs` of the
canonical selected project, mode `0700`, owned by the project user and on the
same `st_dev`. A different parent for the same project refuses before creating
a second account; an exclusive lock excludes another cooperating host at that
parent. Any existing entry other than the lock quarantines the parent at open.

The current managed-temporary target charges two filesystem allocation
granules for parent/lock control, then the selected F4 scratch target plus two
granules per active job for its root and registration. Checked aggregate
arithmetic runs before any root creation. One filesystem free-space sample
compares the **proposed** full phase claim plus desired headroom: completed
active input trees and control files are already reflected in the sample, so
adding their full claims again would double-count them. The aggregate target
still counts all active claims. This is a conservative scheduling model, not
a reservation or APFS physical allocation bound. The host adds no per-chunk
account scan to F4's existing verified copy.

The host durably records a separate random host incarnation, sequence, source
generation/revision, phase and claim before F4 copies. It holds parent, job
and input directory descriptors to check path identity before exposing input
or deleting it. Successful explicit F4 close, an empty-root check and directory
syncs precede refund. An uncertain close or identity fault quarantines the
host with the full claim. Host destruction leaves active registered inputs
for restart inspection; restart currently refuses every retained or unknown
root rather than reconstructing claims. A process kill after registration has
been exercised. The [focused proof](../../evidence/integration/T2-F4-JOB-HOST-2026-11-03.md)
records the exact cases and limitations.

This slice does not yet account F3 main/WAL/project temporary work, compiler
output, cache or retained ready roots; it does not perform F3 scan races,
native ENOSPC, sync-fault injection, recovery of retained jobs, or readiness
publication. The complete design and proof queue above remain open.
