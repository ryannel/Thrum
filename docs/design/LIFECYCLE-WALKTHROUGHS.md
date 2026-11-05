# Asset and optimization lifecycle walkthroughs

1 November 2026. Four concrete paper journeys challenge the [storage lifecycle](STORAGE-LIFECYCLE.md).
Two fresh Sol High agents explored [assets](../evidence/handoffs/2026-11-01-engine-lifecycle-walkthroughs/ASSETS.md)
and [campaigns](../evidence/handoffs/2026-11-01-engine-lifecycle-walkthroughs/CAMPAIGNS.md); the coordinator reconciled
simplicity, acceptance and current tool limits. These are proposed interactions, not generated assets, measured
optimizations, executed publications or cleanup. Example names and counts do not describe new real packages.

## L01 — Generate a lantern for a browser game

**Request:** “Make a dock lantern; let me choose a look, then use it in my game.”

The agent checks the actual camera/use, existing reusable props and its generation/storage budget, then produces a
small candidate set in one owned scratch run. It does not open a worktree or durable asset package per generation.
The creator chooses B's shape, retaining A as a paint comparison. The selection records its scope and exact candidate;
choosing B does not imply permission to discard A or final approval of B's runtime appearance.

The agent adopts B's original generated GLB, external textures and editable master into one source package, with
provider/license facts and a rebuild recipe. The original bytes matter: neither a generation prompt nor a later
Blender export substitutes for them. A short current package record points to the source, recipe and review; one
production-index entry makes it discoverable. Trial history stays in the existing run/review note.

One browser export is staged and checked in the actual consumer. Within the requested local integration, the agent
can wire and preview B, record its agent visual review and leave the creator's paint choice open. It must not present
the preview as the creator's chosen art. The real game must load the selected package and dependencies without scratch paths. Keep A for its open choice,
B's required sources and decisive review evidence. Owned rejected candidates and redundant previews can retire once
inspected and unused. No full generation-folder archive is needed.

**Where it breaks:** the paid source download expires; a texture stays only in scratch; “approved” mixes shape and
runtime claims; a novice is asked to maintain many manifests; backup failure blocks all useful work.

**Simplification:** one adopted package, one runtime delivery and one small existing run record. If offline, retain
locally verified canonical inputs and mark remote recovery unverified. Local editing/preview may continue. Verified
duplicate scratch copies may go; the only required original cannot. Remote handoff claims still require a real
restore. The user chooses the look; the agent handles identity, paths and scoped cleanup.

## L02 — Iterate on a rigged asset while the previous game still works

**Request:** “Change this ship's sail shape and rig, but keep the playable version working.”

The agent records the source and delivery it started from, edits the owning master/recipe and identifies the changed
interfaces: bones, sockets, clips, collision or gameplay bindings. A paint-only change should not become a full rig
redesign. Existing evidence remains applicable only where the changed dependency closure leaves its scope intact.

Suppose the close profile succeeds but the required reduced profile fails. The new delivery is incomplete; its staging
failure leaves the old game untouched. Retry the failed cook from the selected source. Do not produce another full
workbench copy or replace half the runtime files. A crash leaves a named incomplete run, not a timestamp-based cleanup
permission. The final delivery must pass its changed consumer/attachment checks, as well as format validation.

Now another artist publishes a newer revision while the export is cooking. Immediately before the switch, compare
both the selected source and expected current delivery. If either changed, reconcile and revalidate. A fully valid
old export must not overwrite the new choice. Historical review remains bound to the exact reviewed revision.

**Where it breaks today:** we do not have a generic revision-checked asset publisher or browser reader protocol.
Renaming a directory does not make several browser requests atomic. Replacing files at stable URLs can mix revisions.

**Current safe route:** leave a running release untouched; test in an isolated staging consumer. For a local dev
replacement, serialize publication with its owner, quiesce affected readers and reload the complete validated package.
If uninterrupted use is required, prepare a separate coherent release with revision-specific asset URLs and retain
the old served closure. External deployment remains a separately authorized operation. Do not claim hot switching
from this procedure. Future F4/T1 publication must prove the expected-revision and reader lifetime guarantees.

**What stays:** selected editable sources, relevant comparison evidence and supported previous releases. Retain an old
release separately from the next build's input tree; do not ship every rollback revision inside the new `public`.
Its retirement follows support/rollback/reader conditions, not merely “a newer export exists.”

## L03 — Reuse one fitting across two games and art styles

**Request:** “Use our cannon in the naval game and a stylized harbour demo, with lighter browser content.”

Reuse one canonical geometry/attachment family. The games own placement, skins and declared target profiles; a skin
or device tier does not create another editable cannon master. Each game pins the exact source/product revisions it
uses, and selects only the profiles it needs. Different compression or binding requirements can legitimately yield
different cooked products. Sharing a filename does not prove two products can share memory or cache entries.

A naval paint edit recooks its affected surfaces. It does not upgrade the harbour game's geometry, physical behavior
or chosen release. A shared socket edit needs checks in both affected consumers before either opts into the revision.
The old demo can keep its exact delivery while the naval game advances.

**Where it breaks:** “canonical factory path” works within this repository, not across arbitrary projects. A universal
registry is excessive, but a path into another developer's mutable checkout is not a distribution contract.

**Simplification:** identify the owning repository/package/store and a pinned distribution/retrieval method when the
second independent project actually appears. Keep consumer selections local. Do not invent a global registry or
replicate the entire asset library now. If source retrieval is unavailable, an existing complete release remains
usable; a missing-input rebuild is reported honestly. Shared cache tooling can follow demonstrated reuse.

## L04 — Finish an AI optimization campaign without retaining every attempt

**Request:** “Improve forest rendering on this Mac, preserve the scene, then clean up the campaign.”

The coordinator pins a real baseline, workload, required quality, evaluator and resource limits. Candidate worker A
changes visibility/layout; B changes shadow-demand reuse. An evaluator verifies correctness and measures complete
workloads outside the candidate writers' control. Contended hardware measurements are serialized. Their storage
budget is joint: two individually affordable workers may exceed the machine's free-space floor together.

Imagine twenty reasoned proposals, eight cheap screens and three finalists. These counts are illustrative, not a
required search itinerary. Each tried idea gets a compact row: hypothesis/parent, source or patch identity, evaluator,
result, disposition and evidence location. Current best means the best that passed the relevant gates; a faster
candidate with missing moving shadows is rejected. Full compiled outputs and repeated traces need not survive that
rejection. An unresolved crash keeps a minimal reproducer and its unique inputs, not necessarily its whole worktree.

The evaluator compares the baseline and viable finalists, including a coherent combined candidate only when their
contracts permit it. The coordinator integrates the selected change and the evaluator reruns that exact assembled
revision. A local win with worse whole-scene latency, memory or startup is not promoted. Before retiring its worker,
verify the adopted code, dirty source deltas, generated inputs and evidence in the destination. A commit label alone
can omit the shader patch or lookup table that actually produced the result.

| At closeout | Retain | May release after readers/writers finish |
| --- | --- | --- |
| Baseline and selected integrated result | Exact source/input closure, evaluator/workload/configuration, dependency/tool identities, decisive raw samples and quality evidence | Rebuildable compilation caches and redundant captures |
| Rejected trial | Compact hypothesis/result/reason; source patch and minimal failure evidence when still useful or claimed | Dominated build, repeated traces and full workspace when no unique retained input remains |
| Unresolved/user-retained alternative | Exact required comparison or reproducer and next decision | Unrelated bulk inside that run only after inspection |
| Handoff | Verified destination, retained artifacts, scoped claims and outstanding work | Owned idle scratch; never unknown or active worktrees |

Keep raw measurements sufficient to recompute claimed results, including adverse samples; a graph alone is not enough.
A rejected trial with expired inputs may remain useful history, but must be labeled non-rebuildable. Do not call every
proposal a permanent finalist. Use the campaign's end-of-support/review policy to revisit finalist retention later.

**Failure case:** another process consumes the disk. Stop admitting work, cancel owned jobs safely, wait for readers,
and inspect only owned disposable outputs. A stale heartbeat may mean sleep, not abandonment. Preserve the last good
baseline and unique incomplete work. If the campaign ends before integrated validation, report that unfinished claim;
it may still discard unrelated proven-disposable bulk. No automated collector is needed for this first manual path.

## What we simplified, and what remains missing

The lifecycle is useful, but five formal stages should not become five user approvals, duplicate records or directory
moves. Creators mostly make three decisions: **what to try, which result to use, and what must survive the handoff**.
Agents perform the bookkeeping within the authorized task. Recovery, technical validity, appearance and integration
remain separate facts, not a single mandatory chain of green lights.

Use existing records: the package describes the current asset, the delivery describes selected runtime bytes, and
the run records temporary work and decisions. A closeout section links them: **selected result/evidence; keep and why;
release and why; unresolved owner/next decision**. Do not repeat complete hashes/status tables in each document.
Small edits reuse records and relevant prior evidence; no new campaign, empty folder tree or cold restore ritual for
every preview. Changed recovery inputs need new evidence before extending a recovery claim.

| Still missing | First useful support | Owner / stage |
| --- | --- | --- |
| Manual hash/path/ignore checks are easy to miss | Read-only package closure and retention report; state unknowns | F4/T2, D1 then I1/I2 |
| Two exporters can race | Expected source/delivery revision check and durable complete publication | F4/T1, D1/I2; A02 remains open |
| Parallel jobs overfill disk | Shared admission plus actual free-space monitoring and controlled cancellation | T2, I1 before bulk automation |
| Selected code depends on forgotten dirty/generated inputs | Exact rebuild inputs and destination verification at closeout | T2/F4, I1/I2 |
| Old releases bloat new downloads | Build selected declared roots; retain rollback releases separately | F4/T2, I2 and target packaging |
| No safe universal cleanup mechanism | Owner-scoped dry run first; automatic GC only after reader/publication/crash proofs | T2/F4, later gated work |

These are required implementation proofs, not problems that more skill prose can enforce away.

## Skill support now and later

We updated two existing references rather than adding a skill for each stage. The current routing is:

| Request | Skill responsibility |
| --- | --- |
| Make or change a ship, plant or other asset | Relevant craft/tool skill owns quality and construction; `repo-structure` routes placement, delivery and retention |
| Select, deliver or tidy asset work | `repo-structure` → storage reference and existing asset contract; no new lifecycle schema |
| Optimize a module within a commissioned design | Relevant `engine-module-design` performance reference → repository storage policy; ordinary local fixes need no architecture redesign |
| Clean up disk | Existing `repo-structure` ownership/recovery guidance; an explicit cleanup task still needs actual path/process/source inspection |

The storage reference now covers offline local work, exact promotion preconditions, compact closeout, retained user
choices and incomplete-source handoffs. The performance reference distinguishes failed-idea history from permanent
bulk retention. Descriptions stay unchanged because these were already valid routes. Future SDK contribution/game
skills inherit the same operations when those tools exist; no fictional command or broad “AI cleanup” skill is installed.

This follows [OpenAI's Astra guidance](https://developers.openai.com/blog/rethinking-skills-and-prompts-for-gpt-6-astra):
precise discovery, conditional detail, useful decision boundaries and completion scope. Our application is to keep
judgment in short task guidance and put hashes, closure checks, publication and storage admission in tested tools.
The creator-facing guidance does not need the engine's collector protocol or the entire architecture programme.

An independent [paper skill trial](../evidence/handoffs/2026-11-01-engine-lifecycle-walkthroughs/SKILL-TRIAL.md) uses raw
operator cases without the synthesis above. Its decisions are reviewed separately; it cannot prove runtime safety,
Astra behavior, restore reliability or tool enforcement. Real validation remains one modest asset delivery and one
campaign closeout after the relevant implementation exists, with no mass migration or cleanup implied here.
