# Asset promotion, build storage and retention

Recommended design, 1 November 2026. This extends the [construction plan](CONSTRUCTION-PLAN.md),
[AI performance campaigns](AI-PERFORMANCE.md) and existing [asset contract](../ASSET-LIFECYCLE.md).
It adds no runtime module. No cleanup, remote restore, asset promotion or storage tooling was executed in this review.

The [four lifecycle walkthroughs](LIFECYCLE-WALKTHROUGHS.md) refine this design with offline work, stale exports,
multiple game selections and campaign closeout. Detailed GC mechanisms below are tooling obligations, not creator
paperwork. Reuse the current package, delivery and run records; a small edit needs no new lifecycle scaffold.

## What failed, and what the inventory establishes

Our rules already distinguish sources, runtime products and scratch. They have not made ownership, selection,
recoverability and expiry visible enough, and one factory instruction explicitly archived every losing iteration.
That instruction is corrected. More folders or a universal asset database would not solve the underlying problem.

A read-only sample found about **25 GiB available**, **325 GiB in repository scratch**, of which **315 GiB was
worktrees**, **74 GiB in `.git`** (about 71 GiB LFS objects), and **37 GiB across active/archived Codex session
folders**. Sample worktrees contained large nested scratch, build outputs and materialized source libraries.
Current generation candidates were a much smaller share. These are directory footprints, not verified reclaimable
space; APFS sharing, snapshots and concurrent writes prevent summing them into a deletion estimate.
See [inventory and limitations](../evidence/handoffs/2026-11-01-engine-storage-lifecycle/INVENTORY.md).

The policy is **retain the selected result and its complete recovery chain; bound the working set that produced it**.
Explicit user-retention choices and unique feedback/playtests remain protected unless deliberately released.
Keep compact decisions and evidence across iterations. Do not retain a full workspace for every decision.

## One lifecycle, distinct storage classes

| Class | Authority and location in Broadsides | Retention rule |
| --- | --- | --- |
| Experiment/candidate | Owner's `tmp/generations`, captures or existing workbench | A bounded run and named unresolved decision; selected alternatives only at checkpoints |
| Adopted source | Current `factory/<category>/<asset-id>/` package | Required original/generated inputs, editable master, recipes, licenses and exact dependencies; recoverable durable storage |
| Published products | Exact selected runtime delivery and release manifest | Protected by active releases, rollback/support policy and declared consumers |
| Reproducible shared cache | Tool-owned immutable verified cook/download products | Evictable under policy after readers release; keyed by all relevant inputs |
| Mutable run/build state | Private worker build and staging roots | Active owner/lease and bounded budget; reuse compatible working roots, retire after handoff |
| Evidence | Compact owning note/package and selected decisive artifacts | Retain decisions, provenance and sufficient evidence for the claims made; expire bulky redundant traces |
| Conversations | App-owned session records, separate from repository assets | App-specific retention, never swept by repository cleanup |

A path named `tmp`, an old timestamp, a merged commit or clean Git status is not evidence of disposability.
Ignored files can be unique masters. Required source inputs never become an evictable cache merely because a recipe exists.

## Asset promotion as a deliberate operation

**Experiment → review candidate → adopted source → staged cooked delivery → active release.**

1. Start with an owner, question, canonical inputs, consumer, peak-space estimate and selection checkpoint. Sample
   before producing a large sweep. Keep comparison candidates only while their decision remains open.
2. Select exact bytes, not “latest.” Adopt the original/generated inputs, editable master and required ancestry into
   the existing source package. A generation prompt or service task ID cannot recreate the same generated result.
   Keep a small current `asset.json`; link compact provenance and review records instead of embedding every trial.
3. Verify retention: correct tracking/LFS or durable-store location, complete dependencies, and an actual restore
   before claiming recovery. A local archive, LFS pointer, hash or upload receipt alone is not a recovery test.
   Local adoption, preview and authorized integration can proceed with remote recovery explicitly unverified; retain
   canonical required bytes. Locally verified duplicate scratch can retire without pretending it was remotely backed up.
4. Cook the selected target profiles into private staging. Validate manifests, hashes, formats, dependency closure,
   licenses and the intended consumer. No runtime dependency may point into scratch or another worker's checkout.
5. Recheck the expected selected source and current delivery; reconcile stale exports rather than overwriting a
   newer choice. Protect the new release closure durably, then publish the coherent manifest selection. A failure leaves the prior delivery
   active. Content revisions keep older manifests paired with their bytes. Retain the previous known-good closure
   until its explicit rollback/support condition ends; source adoption alone does not retire it.
6. Close the run: retain selected recovery inputs, unresolved alternatives and decisive evidence; remove only the
   owner's inspected disposable outputs. Record any retained bulk and its next decision. Do not move every rejected
   result into `tmp/retired`; temporary recovery material needs an owner, reason and review trigger.

Source retained, exported, consumer tested, visually accepted, game integrated and target qualified are separate
claims. A technically valid export does not replace an artist's selected candidate or silently approve appearance.
The existing [asset contract](../ASSET-LIFECYCLE.md) remains authoritative for package fields and paths;
this proposal does not require migrating all packages or changing their schema version. A changed source invalidates
only relevant evidence/consumers; visual acceptance is tied to the exact reviewed scope and bytes, not a mutable name.

Current Broadsides has no generic atomic multi-file publisher or reader lease tool. Serialize local promotion with
owners, recheck identities, quiesce affected dev readers and reload a complete package. When uninterrupted old readers
are required, preserve their served closure and stage a separate whole release with revision-specific URLs; external
deployment needs its own authorization. Keep rollback releases outside the next build's input/public tree. Across
independent games, name a source owner and pinned distribution method; do not rely on another project's mutable path.

## Efficient builds without duplicated mutable workspaces

F4 owns the source/dependency graph and immutable cooked products. Keys include source/dependency hashes, importer,
cooker/schema version, target/backend/profile, toolchain/compression settings and relevant environment. Unrecorded
inputs or incomplete keys invalidate reuse. T2 reports cache hits/misses, inclusion reasons and bytes by product.
Dynamic asset roots, locales and supported save/release dependencies must be declared; static import scanning alone
cannot establish that a product is unused.

Share verified immutable products/downloads using the tool's supported concurrency model. Keep mutable compiler
outputs, incremental state and staging private to compatible workers. Do not indiscriminately share `target/`,
`node_modules` or hard-linked mutable files. Use a bounded pool of reusable worker roots instead of a new full
checkout/build directory per proposal. Code-only worktrees should selectively materialize LFS inputs; a worker that
needs art fetches the exact pinned closure, not another branch's changing library. Before reassigning a worker root,
verify prior processes/readers ended, preserve inspected unique ignored/untracked source, clear or invalidate private
staging/incremental state as required, and restore exact candidate/evaluator identities. Reuse must preserve isolation.

Before admission, reserve the anticipated **peak**: inputs, unpacking, staging, old/new overlap, compiler temporaries,
captures and concurrent jobs. A machine free-space floor and high/low cache watermarks prevent exhausting the disk
or repeatedly purging/rebuilding the same useful cache. Target-specific values remain an explicit deployment choice,
not invented limits here. Unexpected growth pauses or fails the owned job safely before publication; it must not
trigger deletion of unknown work. One local reservation coordinator is sufficient initially. Estimates govern admission, not an enforceable filesystem
quota: monitor actual free space and owned growth, including competing external writers, with a stop threshold and
controlled cancellation/partial-output policy that preserves source and the last published release.

## Minimal records and safe reclamation

Use a small per-run manifest or existing handoff section: run/owner ID, checkout and dirty-source identity, owned
paths, input/product identities, storage class, maximum/reserved bytes, status, selected outputs, retention reason
and next cleanup trigger. Record artifact groups, not every temporary file. T2 eventually makes this machine-readable;
the current design is not an already implemented CLI or garbage collector.

The proposed collector marks roots from protected source/recovery sets, supported releases/rollback, saved-content
compatibility requirements, named evidence/campaign finalists, explicit user-retention choices, unique feedback/playtests and live readers/writers. Runtime GPU fences and
filesystem leases solve different lifetimes; completion of one does not prove the other has ended. Active leases
identify host/boot, process start or job epoch and owner. Expired heartbeats after sleep/crash are **suspect**, not
permission to delete. Reconcile with the owner/process and preserve unknown legacy directories by default.

The collector first reports a dry run with reasons. Publication, root registration and lease acquisition must
coordinate with collection: mark, quarantine/recheck, then release only if no new root/reader can acquire the object
between the last check and deletion. Crash recovery preserves staged/protected source and the last good release.
Readers acquire a lease before opening; atomically retiring objects reject new leases and wait for existing ones.
Release roots become durable before manifest publication. Quarantine is metadata or a same-volume rename, not a
second bulk copy under disk pressure. Exact locking/persistence mechanisms belong in D1 before an automatic collector is enabled.

Git worktrees and Git/LFS storage use their supported lifecycle, not generic directory aging. Git worktree pruning
cleans stale administrative records for missing worktrees; it is not deletion of all existing checkouts. LFS pruning
has specific protection/remote-verification rules, including separate treatment of unreachable objects and shared
custom stores. Never infer remote recovery or rewrite history to make a local storage report look better.
See [Git worktrees](https://git-scm.com/docs/git-worktree) and
[Git LFS pruning](https://github.com/git-lfs/git-lfs/blob/main/docs/man/git-lfs-prune.adoc).

## AI campaigns and conversation history

Each performance commission has a storage budget alongside compute/time budgets. Preserve baseline, current best,
finalists and representative failure evidence; release dominated bulk at reviewed checkpoints. Keep a compact record
of every hypothesis, patch/source identity, evaluator/configuration, result and rejection reason. Retain enough raw
measurements and unique inputs to substantiate the promoted claim and reproduce the final. A hash of deleted data
is provenance, not available evidence; mark expired artifacts and narrow reproducibility claims accordingly.
Fresh evaluation and whole-engine quality/performance gates still apply after cleanup. Before retiring a winner's
worker, verify adopted dirty deltas, generated inputs and evidence at the destination. Distinguish retained full
rebuild/evidence sets from summary-only rejected trials. Cleanup completion means the inspected owned disposable set
is cleared and retained/deferred groups are accounted for; a legitimate retained comparison or open failure does not
by itself prevent closing the campaign. Do not claim final performance qualification while its rerun is unfinished.

Durable decisions, current source/output paths, open questions and selected evidence belong in the owning repo notes
and packages, linked from a short handoff. Avoid repeating huge stdout, captures or encoded media in successive agent
messages; use bounded summaries and artifact references. Required assets must not exist only as chat attachments.
This reduces future duplication without discarding historical decisions.

Conversation retention is a separate app lifecycle. A chat's archive state does not establish that its transcript,
attachments or worktree were removed. Use supported app operations and explicit retention choices when that cleanup
is commissioned; do not delete app session files from repository tooling. This review inspected session directory
sizes only, not conversation contents. No app retention mechanism or automatic conversation deletion is implemented.

## Ownership, rollout and proof

| Stage | Deliverable and owner | Acceptance |
| --- | --- | --- |
| Now | Correct contradictory retention guidance; keep measured inventory and this plan | No blanket retirement archive; no unsupported recovery or reclamation claims |
| D1 | F4 source/product/root identities; T1 promotion/selection status; T2 reservations, leases and retention reports | Interrupted promotion, new-reader/GC races and recovery have explicit protocols; C4 selects file-first changes and immutable snapshot admission, with interrupted-application recovery; adapter durability remains unqualified |
| I1 | T2 local run manifest, inventory, budget admission and owner-scoped dry-run reporting | Unknown/active work survives; concurrent reservations cannot overcommit; no autonomous bulk campaign before admission works |
| I2 | F4/T1/T2 one package through promotion, cold restore, two cooked profiles and release | Partial publication rejected; selected closure loads with cache absent; only proven disposable candidates retire |
| Later | Shared immutable caching and automatic cache collection where justified | Two writers publish coherently, reader/collector race is safe, crash/sleep recovery holds, cache loss affects speed only |

The first implementation should expose inventory, provenance and explicit decisions before automatic deletion.
It does not need a distributed artifact service, global per-file registry or another runtime subsystem.

Dogfood six cases: (1) two generated candidates with an irreplaceable selected original; (2) interrupted publication
while the previous game stays usable; (3) concurrent builds/collector and sleeping worker; (4) cold source restoration
with empty caches; (5) an optimization campaign that discards dominated traces but reproduces its final integrated
result; (6) a browser demo whose declared dynamic/locale roots ship while old trial exports do not.
Vite copies `public` as-is, so putting experiments there expands deployment independently of code imports
([official guidance](https://vite.dev/guide/assets.html#the-public-directory)).

For existing disk pressure, the next operational pass should classify the largest worktrees with their owners,
then inspect nested scratch/builds and selected-source recovery. LFS recovery and app session retention are separate
passes. The inventory alone authorizes no deletion and cannot predict recovered space. Measure both retained bytes
and actual free space after any later cleanup; do not mistake moving files on this disk for freeing it.

Design inputs: fresh Sol High [asset lifecycle](../evidence/handoffs/2026-11-01-engine-storage-lifecycle/ASSETS.md) and
[build retention](../evidence/handoffs/2026-11-01-engine-storage-lifecycle/BUILDS.md) challenges, reconciled by the coordinator.

## Exact admitted-source handoff

C4 source admission can select dirty/generated inputs not represented by Git HEAD. A reproducible delivery therefore
exports the selected immutable SourceRevision's exact files/masters and required toolchain, or verifies a VCS tree
plus resolved LFS/master closure against that same manifest. The current checkout is not a substitute when it has
changed since admission. Export to a new staged package, never overwrite working files to make them appear clean.
Exclude internal database/cache/losing trials; verify the recipient source closure before retiring the local pin.
This is the existing selected-source/recovery requirement applied to file-first authoring, not another asset registry.
