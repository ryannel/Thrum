# Architectural decision audit

**Later disposition, 1 November 2026:** [selected architecture decisions](ARCHITECTURE-DECISIONS.md) and C3 now
integrate these findings. The review below records its original baseline, not the current open-status list. A01–A05
have selected architectural boundaries; J01–J04 have scoped selections; WP01 selects TRS rejection with the exact numerical
limits now supplied in [FOUNDATION-SPEC](FOUNDATION-SPEC.md); WP02–WP06 are adopted in construction, release and maturity gates.
Detailed mechanisms and every runtime/device/visual proof remain unfinished. [Maturity](MATURITY.md) owns current status.


1 November 2026. **Result: five substantive gaps remain open, alongside known choices that need stronger comparison
and scoped detailed-design obligations.** This audit challenges the selection and completeness of the architecture;
it does not certify an implementation or select a universal plugin model. It qualifies the earlier high-level
handoff: its ownership/lifetime reviews remain useful evidence, but do not establish that every consequential
architectural choice was identified and justified. Do not freeze affected interfaces until the findings below close.

## Method and limits

Three fresh Sol High agents received original engine intent, without our proposed architecture or expected verdict.
Each first recorded an independent decision checklist from primary engine sources, then inspected the design.
They subsequently challenged one another's findings; the coordinator read the reports and relevant source/design
passages, tested counterexamples, and narrowed or rejected overstated findings. The coordinator had prior design
exposure; the agents' initial derivations were separate from that exposure.

| Audit | Preserved initial derivation | Detailed comparison |
| --- | --- | --- |
| Core, composition and execution | [Core baseline](../evidence/handoffs/2026-11-01-engine-audit/core-independent.md) | [Core report](research/AUDIT-CORE.md) |
| Alternative physical/presentation models | [Model baseline](../evidence/handoffs/2026-11-01-engine-audit/models-independent.md) | [Model report](research/AUDIT-MODELS.md) |
| Creation and delivery | [Delivery baseline](../evidence/handoffs/2026-11-01-engine-audit/delivery-independent.md) | [Delivery report](research/AUDIT-CREATION-DELIVERY.md) |

Comparisons sampled official Bevy, Unreal, Unity, Godot, O3DE and PlayCanvas documentation, plus primary atmosphere
and water research where model validity changes architecture. Sources support the named mechanisms, not comparative
performance. No engine was built or benchmarked; no target images were assessed; no exhaustive survey is claimed.
The detailed reports include findings subsequently weakened by cross-review. **The dispositions here govern priority**;
the original reports retain their reasoning rather than being rewritten to manufacture unanimous agreement.

The review question was not whether we match every feature in those engines. It was whether a consequential choice
has a credible competing approach, a reason to choose it, and a complete boundary for the required use cases.

## Substantive gaps retained after challenge

| ID | Gap and consequence | Alternatives / closure condition | Affected owners and gate |
| --- | --- | --- | --- |
| A01 | **Edit/play session isolation.** We describe editable source, running state and preview, but not which world is used during play, what stopping discards, or how simulation changes are intentionally applied back. One authority per world must not accidentally mean one global world per process. | Compare a separate play world, checkpoint/restore in one world, and a separate player process. Define source pin, session/world IDs, save isolation, side effects, stop and apply-back. Trace edit → play → mutate → stop → selected apply-back, including crash. | F1/F3/W1/P7/T1, P3/P6, F4/T2. Before identity/session and authoring APIs freeze in D1. |
| A02 | **External source changes versus the durable authoring journal.** T1 handles individual and batch operations, but an external multi-file checkout can arrive as unordered writes while an accepted edit is pending. Restart must not restore a journal over another branch or cook/activate a partial checkout. | Compare source-files-first, database/export-first, or a versioned manifest selecting source records. Choose the canonical project source and coherent external change-set boundary; define conflict/restart/rename policy. File format and merge UI may follow. | T1/F3/F4/P7/W1/W2/T2. D1 source/recovery contract. |
| A03 | **Shader demand and availability across supported content/profiles.** Generic required-product admission exists, but does not enumerate which material/pass/geometry/provider variants a build and its allowed live transitions demand. A valid profile can request a stripped or unprepared shader. | Compare finite cooked closure, bounded runtime specialization with explicit fallback/loading, and dynamic branches for selected dimensions. Define demand declaration, stripping, cache/product identity, readiness and failure semantics. This extends F4; it does not create another asset system. | F2/F4/V1/V3/T1/T2. Shared D1–D2 contract before the cooked render path. |
| A04 | **Physical coupling between different water domains.** We named spectral, shallow, static and local models, but not the exchange between them. A visually seamless river–sea junction does not establish shared water level, flux, wave behavior or body queries. Solver domains and streaming cells are different partitions. | Declare each adjacency as prescribed/open, one-way, conservative coupled, closed or visual-only as appropriate. Specify transferred quantities, time/substeps, transition query ownership and any source/sink. Compare heterogeneous domains with a common solver where that actually fits. No requirement to implement all regimes now. | E1/P3/F2/F3/F4/W2/P7, V2/V3/V5, P5/T1. Before a water-family API is frozen. |
| A05 | **Allowed atmosphere inputs cannot always meet the overlap promise.** E1/V3 permit an independently preintegrated transmittance/radiance pair, while C08 requires overlapping media to combine coefficients before integration. Those aggregate pairs generally lose the information needed for that operation. | Admit ordered disjoint segments, coefficient/source access to a joint integrator, or a provider that jointly preintegrates the overlapping media. Explicit approximate layering is another declared policy, not exact overlap. Choose representation/compatibility rules before connecting alternative atmosphere/fog models. | E1/V3/F2/V1, V4 consumer. C08 and environment/render interface correction before provider integration. |

A01 is exposed by production choices such as Unreal's duplicated play level and explicit apply-back; duplication is
a precedent, not our selection. [Unreal Play/Simulate](https://dev.epicgames.com/documentation/unreal-engine/ineditor-testing-play-and-simulate-in-unreal-engine).
A03 is illustrated by Unity's documented risk of stripping runtime-required variants and O3DE's shader asset build
pipeline. [Unity variant stripping](https://docs.unity.com/en-us/engine/6000.5/manual/materials-and-shaders/shaders/shader-troubleshooting/shader-reducing/shader-variants/shader-variant-stripping),
[O3DE shader build](https://www.docs.o3de.org/docs/atom-guide/dev-guide/shaders/shader-build-pipeline/).
A04 is sharpened by Unreal's different river/ocean/lake behavior and transition materials: that is an authoring/render
solution, not evidence of conservative flow exchange. [Unreal water bodies](https://dev.epicgames.com/documentation/unreal-engine/water-body-actors-in-unreal-engine).

A05 has a direct counterexample independent of engine precedent. Both providers describe the same ray interval.
Medium B emits near its center, with standalone pair `(1, b)`. Absorbing medium A has pair `(exp(-tau), 0)`;
move A's extinction concentration from the front half to the back half while keeping total optical depth `tau`
unchanged. A can retain a small positive extinction throughout, so the supports overlap in both cases. B's emission
is attenuated differently, yet each provider's standalone whole-interval pair is unchanged. Therefore those pairs
cannot uniquely determine the combined result over the overlap. The existing ordered segment
algebra remains useful; its domain must be respected. This is a concrete contract inconsistency, not merely a request
for more atmosphere research.

## Choices needing better justification, not newly missing capabilities

| ID | Disposition after cross-review | Required follow-up |
| --- | --- | --- |
| J01 Composition mechanism | The plugin proposal is already explicitly reopened. The old comparison against scattered booleans is weak; a typed static recipe with validation is a credible simpler alternative. The audit does not mandate plugins. | D0 compares static recipes, optional feature packages and a richer registration framework. Select build inclusion, public extension and live transitions separately. Make downstream I1 wording conditional on that choice. |
| J02 Renderer replacement level | No requirement for arbitrary full replacement was established. Optional sky, a headless runtime, graphics backend substitution, an extensible render pipeline and a wholly different renderer are different promises. Existing style/material extensions mean a new style does not automatically require a new renderer. | D0 states the supported extension level; D2 compares materially different pipelines only where a chosen game/style or platform case warrants them. Keep the reasoned clustered-forward baseline provisional within its workload assumptions. |
| J03 Runtime library reuse | The world design already permits owned or standalone storage; there is no missing reuse policy. However, a platform note incorrectly treats any application-framework scheduler as excluded by the user's no-Bevy-host choice. | Correct that inference. D1 can compare a standalone ECS confined to W1 and scheduler under F1 against owned mechanisms; no decision to readopt Bevy's host is implied. Public authority and internal implementation are separate. |
| J04 Code iteration | Process-lifetime compiled plugins already allow rebuild/restart. What is absent is the concrete developer loop and handoff for unsaved source, stopped play, schema change and resumed inspection. This is narrower than requiring scripting or binary hot reload. | Choose a minimum supported code-change/restart workflow alongside A01/A02; record what survives and how failure is reported. Rebuild time and responsiveness remain empirical qualification. |

J03's misleading user-scope sentence in PLATFORM-RUNTIME was corrected during this audit. The library selection itself
remains open in D1; no runtime dependency was changed. Other findings remain design work, not silently accepted fixes.

The earlier broad conclusion that all high-level blockers were closed should not substitute for these comparisons.
Conversely, this audit does not justify throwing out the existing source/cook split, identity model, fixed-tick
authority, bounded snapshots, platform lifecycle, body/animation ownership or observability design.

## Where multiple water and atmosphere models belong

The architectural distinction is **capability, physical model, spatial domain, representation and delivery profile**.
They are related but are not one quality slider or one mandatory plugin boundary.

| Selection | Water example | Atmosphere example |
| --- | --- | --- |
| Per game / required behavior | A still canal with authored flow versus interactive flooding | An HDRI background with authored illumination versus time-varying physical atmosphere |
| Per domain in one world | A prescribed sea, shallow river and local falls/pool can have different state and boundary conditions | Global atmosphere, local fog and cloud density can have distinct support and update models |
| Per consumer | Hull contact, swimming occupancy, water rendering and audio may require different quantities/fidelity | Background pixels, ambient/reflection light, Sun transmittance, aerial perspective and cloud shadow are distinct outputs |
| Per target / visual quality | Lower mesh/detail/foam cost while retaining the required gameplay model | Baked/analytical/dynamic visible representation where retained lighting/media semantics are supplied |
| At a model boundary or replacement | Physical flux exchange, prescribed forcing or a visual seam must be declared; state is not automatically convertible | Disjoint segments, joint coefficient integration or explicitly approximate layering; cache/history compatibility must be declared |

These are illustrative cases for analytical comparison, not a new requirement to build a fluid simulator, flight game,
swimming system or every sky algorithm. A game/domain author chooses the intended model and required quantities;
validation rejects incompatible combinations. An automatic cost optimizer must not silently turn a physical flood
model into decorative flowing water. Alternative implementations can share a useful contract **only over behavior
and validity they genuinely share**.

For water, a height query is an efficient supported interface for a graph-like surface. It is not sufficient for every
volume or multiple vertical intersections. Existing support/validity and unsupported-result rules are sound: preserve
them and specialize the consumer capability when such a model is commissioned. Do not impose a universal volume API
on the simple-water path. A04 is the more immediate architectural omission for the already-proposed regional models.

For atmosphere, a sky image can provide background/illumination without supplying aerial extinction; a dynamic
scattering model may supply all three. Keep requested outputs explicit. Unity's separate visual and lighting skies
are a concrete precedent. [Unity ambient-light configuration](https://docs.unity.cn/Packages/com.unity.render-pipelines.high-definition%4017.1/manual/ambient-lighting-configure.html).
The current engine design already recognizes most of this separation; A05 exposes the missing representation rule.

CPU/GPU placement is a per-provider decision to make under the existing headless/authoritative-world contract, not a
new license for rendered pixels to determine physics. Specify direct CPU evaluation, mirrored/coarse CPU queries,
GPU visual residuals or explicitly tolerated readback latency where appropriate. Unity documents CPU interaction
work alongside its GPU water simulation. [Unity water architecture](https://unity.com/blog/engine-platform/new-hdrp-water-system-in-2022-lts-and-2023-1).
Any future GPU-authoritative physical model would require an explicit architecture decision about headless execution,
query age and recovery; it cannot be added merely as a visual-quality provider. The audit does not select it now.

## Cross-review dispositions and legitimate later work

| Original finding | Coordinator disposition after independent challenge |
| --- | --- |
| Core F1 | Retain J01; known open selection and stronger comparator, not a new missing feature |
| Core F2 | Reject broad missing-library-choice claim; retain J03's incorrect scope inference and D1 comparison criterion |
| Core F3 | Weaken to existing D1 obligation: truthful access to specialized stores, cadence and stale-result rules. Existing semantic phases/body order already provide a foundation; no new scheduler framework is demanded |
| Core F4 | Reject missing full-renderer-replacement requirement; retain J02 to prevent ambiguous claims |
| Models 1 | Retain A04 physical domain-edge decision |
| Models 2 | Weaken to conditional query/capability refinement when a commissioned model exceeds heightfield support; no universal fluid API or compulsory swimmer/falls feature |
| Models 3 and 5 | Keep model-selection matrices and atmosphere output compatibility as detailed commissioning obligations under existing F2/E1 principles; no claim that alternative models were absent |
| Models 4 | Reject missing shared environmental authority; keep provider-specific data residence, synchronization and cost work. Existing CPU/headless semantics constrain the baseline |
| Models 6 | Legitimate detailed source/cook requirements; depends on actual selected models, not a new generic asset architecture |
| Models 7 | Retain A05 aggregate-medium overlap inconsistency |
| Delivery CD1 | Retain A01; source versus runtime products already exist, but play-session semantics do not follow from that separation |
| Delivery CD2 | Weaken to J04 baseline developer workflow/state-handoff decision |
| Delivery CD3 | Retain narrow A03 demand/readiness/fallback closure inside the existing F4 system |
| Delivery CD4 | Retain narrow A02 external multi-file change-set/source-authority boundary; ordinary T1 batch edits are already handled |

Exact storage library, solver coefficients/discretizations, shader compiler/cache format, scheduler implementation and
minimum hardware remain detailed choices or product inputs. Networking remains undecided; pure 2D/MMO remain out of
scope. Script VMs, binary hot reload, remote collaboration and an integrated GUI are options, not audit requirements.
Device performance and visual quality remain unmeasured; analytical comparisons do not qualify them.

## Why the earlier review missed these choices

The earlier research mapped engine features to responsibilities and often cited one plausible precedent. That is
useful orientation, but it is weaker than comparing competing ownership, representation and workflow choices.
The review scenarios then concentrated on tick/frame consistency, streaming, camera changes, loss/recovery and
resource lifetimes. They caught real defects there. They did not sufficiently exercise creating a game, changing its
code/source, entering and leaving play, cooking alternative render combinations, or connecting heterogeneous models.

Some comparisons rejected easy extremes—scattered flags or putting every subsystem in one ECS—without weighing the
stronger alternatives. The coordinator's broad completion wording overstated what those scoped reviews established.
The correction is not more reviewers checking the same architecture: selection/coverage challenge must precede
internal consistency review, with independent alternatives and explicit disposition of disagreement.

## Ordered follow-up and completion boundary

**Subsequent scenario pass:** [ten walkthroughs](SCENARIO-WALKTHROUGHS.md) and the resulting
[developer experience](ENGINE-EXPERIENCE.md) provide concrete recommended directions for J01/J02/J04: static setup with
optional packages, scoped rendering extensions, and isolated play plus rebuild/restart. They also propose source-first
sync and finite shader demand. A01–A05 remain open until the affected detailed contracts/mechanisms are reconciled;
the paper walkthroughs do not establish working tools, device costs or human usability.

1. **D0: declare flexibility and iteration promises.** Resolve J01/J02/J04 using concrete game/developer cases; retain
   J03's corrected no-Bevy-host interpretation. Keep source/runtime authority invariant while comparing implementations.
2. **D1: close creation/session/source boundaries.** Resolve A01/A02 together, then select storage/scheduling/reuse
   behind those contracts. A branch change, crash or play restart must not silently overwrite another source/session.
3. **Joint D1–D2: close shader demand.** Resolve A03 with F2/F4/V1/V3/T1/T2 before profile switching and cooked material
   admission depend on it. Do not leave the renderer and asset teams to invent incompatible answers independently.
4. **Joint environment/render/play design: close model edges.** Resolve A04/A05 and author the model/output/domain
   compatibility matrix. Research individual water/atmosphere implementations within those justified boundaries.
5. **Re-review the affected full paths.** Compare credible alternatives, trace normal and failure behavior, then use
   bounded measurements only for remaining numerical, visual or hardware uncertainty. A proposed fix is not closure.

This **audit is complete for its recorded scope; the findings remain open**. It produces a decision queue, not a
replacement fully designed engine. [Maturity](MATURITY.md) separates the prior coherence reviews from these open
architectural obligations. Runtime construction has not started as part of this audit.
