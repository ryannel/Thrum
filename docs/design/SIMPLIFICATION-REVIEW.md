# Fresh architecture challenge and simplification review

1 November 2026. Three fresh **Sol High** agents independently challenged the completed plan from network correctness,
creator simplicity, and runtime/delivery perspectives, then cross-reviewed one another's findings. The coordinator
kept the 42-file baseline unchanged during review. Exact reviewed hashes are retained in `maturity.json`.

**Current disposition — C4:** the user selected the simplifications and requested coherent closure. The source,
multiplayer and delivery owners revised their specifications, then cross-challenged the integrated result. The
initial design now uses file-first admission, restart/stop-load, native authoring and early small-game multiplayer.
The exact action/result, lifecycle/state and compatibility gaps below are resolved for the admitted first profile;
runtime qualification remains unrun. This document's findings describe the frozen pre-reconciliation baseline.

| Finding | Selected resolution and single mechanism owner |
| --- | --- |
| SR01 | Committed/rejected action results, contiguous prefix plus individually resolved higher inputs, exact state/tick correspondence, bounded retention and same-epoch baseline acknowledgement — [NETWORK-SPEC](NETWORK-SPEC.md) |
| SR02 | One atomic StateGroup includes lifecycle and required state/results; optional water pose/sample correspondence uses the same unit — [NETWORK-SPEC](NETWORK-SPEC.md) |
| SR03 | Two exact identities: gameplay semantic manifest and wire/protocol schema; client visuals and tool-only schemas are independent — [NETWORK-SPEC](NETWORK-SPEC.md), C13 |
| SR04 | Proposed patch → coordinated file application → immutable admission; internal native recovery store, no automatic mirror or first-SDK browser editor — [FOUNDATION-SPEC](FOUNDATION-SPEC.md), C10 |
| SR05 | Substantive edits restart; bounded initial worlds preload; stopped player transitions checkpoint, drain/retire, load/publish, and recover the old pinned checkpoint on failure — [FOUNDATION-SPEC](FOUNDATION-SPEC.md) |
| SR06 | I2a minimal source/save path → small I3a plus real I2n; thin mobile shells early, I2b broader content/artist work later — [CONSTRUCTION-PLAN](CONSTRUCTION-PLAN.md) |
| SR07 | R4 full groups selected after concrete encoded actions/state/results/events/baseline and resync arithmetic; playable/degraded timing explicit. Portable peak and browser boot estimates scoped — [NETWORK-SPEC](NETWORK-SPEC.md), [BASELINE-SPEC](BASELINE-SPEC.md) |
| SR08 | C4 owns invariants, five specs own algorithms/adapters, modules own consequences, creator guide owns tasks, ledger owns status; duplicate source protocol removed and newcomer index shortened |

The review recommendations below are retained as reasoning for these selections, not another live proposal queue.
Broader live streaming and richer providers still require their own scoped detailed design; no initial profile
completion automatically qualifies the whole family. [Conversation coverage](ARCHITECTURE-BRIEF.md#conversation-coverage-and-retained-scope)
traces all E01–E20 requirements through the smaller sequence.
The independent reports contain counterexamples, baseline line references and coverage limits:
[network](../evidence/handoffs/2026-11-01-engine-simplification-review/NETWORK.md),
[creator simplicity](../evidence/handoffs/2026-11-01-engine-simplification-review/SIMPLICITY.md), and
[delivery](../evidence/handoffs/2026-11-01-engine-simplification-review/DELIVERY.md).

## Keep the core small

Teach four concepts: **project** (ordinary game code and authored assets), **world** (one authority advancing game
rules), **target** (platform and selected capabilities), and **run** (a pinned build running locally or as server/replica).
Components, actions, systems and providers are registered through ordinary typed code. Revision counters, retry
epochs, store leases and transport queues belong in extension contracts and diagnostics, not every creator task.

Multiplayer fits this model directly. Local and server worlds run the same rules. A client submits an intent; the
authority validates and commits it, then publishes its result and coherent state. Rendering and sound consume that
state. Optional prediction is a bounded speculative branch. The authority owns saves and irreversible effects.
A Mac, browser and mobile client may use different visual products while agreeing on gameplay meaning.

Preserve distinct source, runtime and wire identities; immutable admitted builds; coherent publication; bounded
resources; separate content/CPU/GPU dependency graphs; and explicit provider compatibility. None requires a generic
plugin container, universal rollback, a copied server solver in every client, or a second mutable gameplay authority.

## Findings and minimum corrections — reviewed baseline

### SR01 — Action receipt is not an action result

**Implementation blocker; G08; N1/P6/F1, with P7 for durable operations.**
[NETWORK-SPEC](NETWORK-SPEC.md) permits several independently validated actions in an InputBatch, but its message
acknowledgement does not identify each action's committed/rejected outcome. Prediction and speculative effects need
that identity. A batch containing movement, a rejected purchase and an accepted shot cannot be reconciled from an
envelope acknowledgement alone.

Specify one bounded result path using the existing game-event/outcome ownership: actor/input sequence, disposition,
commit tick or rejection reason, and event origin where relevant. Define the resolved-input watermark and the exact
authoritative controller state it corresponds to before enabling prediction. Admission and committed execution must
remain distinguishable. On reconnect, discard old-epoch pending inputs and take a fresh baseline; do not automatically
retry an uncertain one-shot. A game promising retryable durable transactions supplies its own durable operation ID
and outcome. This does not require a generic RPC service or cross-reconnect exactly-once guarantee.

### SR02 — Ordered delivery does not make lifecycle and state atomic

**Implementation blocker; G08; N1/W1/F3/P6.** A LifecycleBatch can arrive before its separate StateGroup. Publishing
between them exposes an actor without its required pose, attachment or inventory, violating C13 despite reliable
message ordering. The same problem applies to interest re-entry and dependent detach/despawn effects.

The smallest correction is to carry lifecycle changes and required initial/dependent state in **one bounded atomic
StateGroup admission unit**. Reject a replication recipe that cannot fit the chosen profile; no new general-purpose
transaction subsystem is needed. The optional E1 water recipe should similarly publish a body pose and its matching
query sample together, or require the exact retained pose before publication. The existing sample-time invariant is
correct; its concrete realization is missing. Water fields belong to that optional recipe, not every N1 game.

### SR03 — Join equality must cover gameplay, not an entire platform build

**Contract ambiguity; G08; N1/F3/F4 and gameplay providers.** `game hash` and `schema hash` do not precisely identify
their closures. Hashing every local F3 schema or whole project would reject a compatible browser-only material change;
omitting rule/query meanings could admit an incompatible game.

Define the existing semantic manifest as the gameplay compatibility identity, including rules/actions, authority/tick
semantics and required physical/query meanings with role-specific product mappings. Define protocol equality over
the envelope and wire action/state schemas. Remove `game hash` or make it an explicit alias, rather than adding a
third competing manifest. Local visual/tool schemas stay in their own build provenance. Closure requires both a
compatible visual-only change trace and an incompatible rule/collider change trace.

### SR04 — Everyday authoring has two destinations, and browser authoring adds an unrequested product

**Recommended scope/design reduction; G01 remains selected but challenged; T1/F3/F4/P7.** The public guide promises
ordinary source files, while [FOUNDATION-SPEC](FOUNDATION-SPEC.md) accepts an edit into a private immutable store
before checkout materialization. In a shared checkout the editor returns a patch, so play can use accepted changes
that ordinary files and `git diff` do not show. Exact admitted bytes are useful; this default creator experience is
surprising. Full IndexedDB source-authority/editor parity is also more than the confirmed browser-demo requirement.

Prefer **working files → explicit immutable snapshot admission → build/run**. A workbench/AI edit remains a proposed
patch until coordinated file application and admission succeed. Preserve conflict detection, last-good runs, accepted
root/outcome durability, undo and retained sources; do not replace these with an unsafe watcher or assume Git alone
provides crash-safe multi-file editing. Remove automatic checkout mirroring from the first SDK. Native authoring plus
browser play/export is enough initially; browser runtime caches and player saves still need their own persistence
policy. Full browser authoring can follow a named need.

This is a recommended alternative, not a completed replacement protocol. Before selection, reconcile C10, Foundation,
T1, creator/AI/storage guides and interrupted file-application/admission traces together. A prose override in one guide
would recreate the inconsistency we are trying to remove.

### SR05 — Full-world live replacement is too expensive as the ordinary edit path

**Recommended reduction and capacity-proof gap; G02/G09; W1/W2/F1/F4/P3/P5/E1.** The initial algorithm pauses and
duplicates the whole world/provider state for an authored or region transaction. If replaceable state uses the allowed
15% of a browser memory budget, its extra copy already exceeds the separate 10% transition allowance. Admission can
safely reject it, so this is not a memory-safety defect; it means steady-state fit does not prove the live transition.

Default substantive edits to **restart a disposable play run**. A bounded first portable game can preload gameplay
content or use an explicit stop/load transition. Only qualified visual fields get live preview. Keep the coherent
region activation boundary, and design scoped live streaming against a real residency/transfer budget when required.
Do not make every provider implement complete live state transfer before it can serve the first game. The initial
static multiplayer server remains a valid profile; future server streaming must stage without clients pausing the
authority. Client interest and local render residency are not substitutes for authoritative streaming.

### SR06 — Bring the useful game forward and shrink the first network recipe

**Recommended sequencing reduction; construction coordinator and all initial owners.** I3a currently follows a broad
I2 import/cook/region/save/edit path. Pull forward a narrow R0 path: one admitted snapshot, local authority, one action,
one camera/material, inspect/restart, save and one clean export. Keep early thin mobile boot/suspend/surface checks in
parallel; delaying all mobile lifecycle knowledge would trade one rebuild risk for another.

Exercise role separation and actions in an in-memory authority/replica fixture from the start. Keep real **I2n alongside
the first small game**, before broad I3b integration. For I2n, specify action outcomes, atomic lifecycle,
compatibility, join/reconnect and effect identity. Evaluate bounded full groups as the simpler first candidate;
retain the selected delta design unless the R4 encoded-byte/admission comparison supports replacing it.
Prediction and the E1 water projection are optional recipes and need not gate first multiplayer
completion. Their existing contracts stay available. This simplifies the implementation without parking multiplayer.

### SR07 — Counts and caps do not yet describe a feasible simultaneous workload

**Design-proof gap; reopen G09 for R4 and portable transitions; N1/T2 plus affected owners.**
128 entities, 20 publications/s and 256 KiB/s do not establish a bandwidth result without group sizes/cadence.
For example, one independently changing group per entity uses 160 KiB/s in 64-byte headers alone. That grouping is
not required, and four water bodies do not imply 128 water records. The review therefore finds a missing recipe,
not proof that the profile is impossible. Specify actions, encoded projections, group packing, change cadence,
baseline/lifecycle bursts and resync reserve before selecting full groups versus deltas.

At 60 Hz, the default three-tick lead is 50 ms. Symmetric 300 ms RTT delivers a one-shot roughly six ticks after its
intended tick, beyond the two-tick late limit. The plan tests 300 ms but does not promise it is playable, so label
playable and degraded cases. Prefer next-authority-tick admission with bounded age/order and game-state preconditions
for board/inventory actions; intended-tick windows are for actions that need them. Server rewind is unnecessary here.

The browser's 25 MiB/5 s target at 50 Mbit/s leaves about 0.81 s after ideal payload transfer. This was already
acknowledged, not newly discovered. It still needs a boot dependency/byte/prepare/scratch estimate. Keep pipeline counts,
storage limits and timing thresholds as declared reference-profile gates, not general SDK or format ceilings.
Analytical admission precedes construction; device measurement later tests the estimates.

### SR08 — Repeated normative prose makes a small architecture difficult to maintain

**Recommended consolidation; architecture coordinator.** Source acceptance/recovery appears at length in the creator
guide, C10, Foundation, T1 and decisions. Similar duplication lets a closed ledger coexist with missing wire behavior.

Use the creator guide for tasks, CONTRACTS for invariants, the five specifications for selected algorithms/adapters,
modules for ownership and consumer consequences, and the ledger for current status. Replace repeated algorithms with
precise links during the coordinated rewrite; keep historical review evidence outside the newcomer reading path.
Numerical protocol settings are already scoped to a first profile. The issue is maintaining that separation in APIs
and documentation, not evidence that C3 demands universal four-player limits.

## Recommended smaller delivery — adopted through the construction plan

1. **One complete small game:** native file/snapshot authoring, local authority, minimal rendering, save/restart and
   one recipient export; early mobile shell checks alongside it.
2. **The same game with real multiplayer:** server plus native/browser clients, committed action results, coherent
   state, reconnect, content compatibility and bounded resources. Prefer full groups if the R4 budget supports them;
   prediction and water enter when that game's declared needs justify their cost.
3. **Qualify platform variants and grow capabilities:** real browser/mobile boot and lifecycle evidence, then the
   scoped streaming, richer editing and provider combinations demanded by named games.

This is a dependency-order recommendation, not permission to build before the remaining mechanisms are resolved.
The smallest next design pass closes SR01–SR03 and the R4 recipe, compares/adopts SR04–SR06 coherently across owners,
and consolidates their duplicated specifications. It does not reopen ECS choice, invent another framework, or remove
the engine's wider 3D/camera/art-style/platform goals. Changing an implementation later is normal; these boundaries
reduce the risk of having to change who owns game state, actions, saves or content meaning.

## Review-time disposition and evidence — superseded by C4 above

Applied in this review: G08 implementation readiness reopened for SR01–SR03; G09 admission proof reopened for SR05/SR07;
current status links and N1's misleading “no remaining design” claim corrected. G01/G02 remain the selected mechanisms
with explicit simplification challenges. SR04–SR06/SR08 are recommendations, not silently selected replacements.
Earlier reviewed hashes and design decisions remain historical evidence. Multiplayer keeps its early milestone.

All three agents inspected the current whole-engine design and their cross-system consequences; coverage was
deliberately complementary, not three line-by-line re-audits of every historical research paragraph. The network agent
read all eight current owner modules; the other agents focused on creator/delivery seams and checked additional
render/environment/AI/storage consequences. Reports identify limits. No runtime implementation, device qualification,
visual acceptance or measured optimization was performed. Arithmetic and document checks are not runtime proof.
