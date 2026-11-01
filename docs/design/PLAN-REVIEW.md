# Whole-plan review from three perspectives

**Later disposition, 1 November 2026:** [selected architecture decisions](ARCHITECTURE-DECISIONS.md) and C3 now
integrate these findings. The review below records its original baseline, not the current open-status list. A01–A05
have selected architectural boundaries; J01–J04 have scoped selections; WP01 selects TRS rejection with the exact numerical
limits now supplied in [FOUNDATION-SPEC](FOUNDATION-SPEC.md); WP02–WP06 are adopted in construction, release and maturity gates.
Detailed mechanisms and every runtime/device/visual proof remain unfinished. [Maturity](MATURITY.md) owns current status.


1 November 2026. **The architectural direction is coherent enough for coordinated detailed design. It is not yet a
construction-ready specification for the whole engine.** Keep the core ownership/composition model; close the known
boundaries, correct one newly demonstrated transform contract error, and make the first useful outcomes explicit.
The strongest improvements here clarify decisions and acceptance gates rather than add subsystems.

## Scope and independent perspectives

The user requested three different perspectives, each reading the whole plan start to finish. Three fresh Sol High
agents independently read the same **45 Markdown files plus the maturity ledger**: 26 top-level documents, seven
module designs and 12 research notes. They distinguished the superseded host/Traveller assessments and historical
reviews from the current proposal. After their independent reports, each read and challenged the other two reports.
The coordinator checked the material source passages, independently reproduced the transform counterexample and
merged duplicate findings. The [baseline inventory](../evidence/handoffs/2026-11-01-engine-whole-plan-review/BASELINE.md)
records hashes; all 46 inputs remained unchanged through the independent reading and cross-review.

| Perspective | Independent report | Principal contribution |
| --- | --- | --- |
| Systems architect | [Systems](../evidence/handoffs/2026-11-01-engine-whole-plan-review/SYSTEMS.md) | State/representation correctness, integration boundaries and inherited maturity gaps |
| Game technical director / creator | [Creator](../evidence/handoffs/2026-11-01-engine-whole-plan-review/CREATOR.md) | Small-game path, artist composition and SDK evolution |
| Delivery / platform / maintenance lead | [Delivery](../evidence/handoffs/2026-11-01-engine-whole-plan-review/DELIVERY.md) | Executable milestones, early platform compatibility and support obligations |

This was a document review, not a new external-engine survey or runtime audit. Existing source citations were assessed
at their recorded scope; their external pages were not all reverified. No engine was built, device tested, asset
promoted or optimization campaign run. The reports contain precise file/line citations against the baseline.

## What should survive the review

The reviewers agree on the value of a small creator vocabulary—project, scene, target—and direct typed game setup
with optional reusable packages. Preserve the distinction between a capability, its physical model and its visible
representation. A game can need cloud attenuation without sky drawing, or a simple authored field without a dynamic
simulation. This does not require arbitrary runtime hot swapping or a universal plugin framework.

Keep one owner for world state, solved body motion, source, content products and saves; keep their identities and
lifetimes distinct. Specialized physics/animation/render stores can coexist with the common world authority. Separate
content-build, CPU-schedule and GPU-pass graphs remain justified. The snapshot/lease/recovery work addresses real
failure cases. Reuse commodity libraries deliberately without restoring Bevy as host.

The AI/performance/storage plans correctly distinguish candidate improvements from whole-product qualification,
selected masters from caches, and compact failure history from permanent bulk archives. Preserve that discipline.
No reviewer found a reason to replace it with another framework. None of these strengths establishes measured runtime
quality or portability; the ledger's not-run implementation/quality/platform dimensions remain correct.

## Reconciled findings

### WP01 — Correct preserve-world reparenting before freezing transforms

**New correctness defect; medium; W1/T1/F3, with P3/V2 consumers; D1 before I2 editing.**

[World/content](modules/WORLD-CONTENT.md) stores local transforms as translation × rotation × scale, but promises
preserve-world reparenting using `inverse(new_parent_world) * old_world` whenever the parent is invertible. That result
can require shear. For an old 45-degree world rotation and a new parent scaled by `(2,1)`, the required local matrix
`diag(0.5,1) * R(45°)` has basis-column dot product `0.375`; rotation × diagonal scale requires orthogonal columns.
Thus a valid static prop can change shape/placement if the result is silently decomposed back to TRS.

Choose either affine-capable local storage with explicit consumer rules, or TRS-only storage that rejects an
unrepresentable preserve-world operation using a specified round-trip tolerance. Preserving local transform remains
an explicit alternative. The smaller TRS-only restriction is worth considering first; this review does not select it
or mandate general affine support. Add scaled/rotated ancestry to the transform and editor-operation fixture.

### WP02 — Give the small rendered game its own acceptance milestone

**New sequencing clarification; high delivery value; coordinator/F1/F2/T1/T2 and selected render/input owners.**

The [construction plan](CONSTRUCTION-PLAN.md) already includes an I1 headless game and I3 first interaction/empty-project
paths. It explicitly says the two-region animated reference scene is an integration workload. Therefore the initial
claim that small games *must contain* animation, audio and environment is rejected. The remaining issue is that the
package table gives broad I3/I4 gates without separately naming the small rendered/exported result and its required
subset of dependencies.

Name an early checkpoint: a small 3D board or empty-scene game, direct rule/action/state/visible result, one camera,
basic material, useful inspection/restart, and one target's development export working for a clean recipient.
Derive and implement only its required mechanisms, including applicable source/play/shader contracts. Verify omitted
capabilities do not bring avoidable work. Keep the demanding two-view animated/streamed scene as a later integration
checkpoint. This uses the existing engine and target packaging path; it is not another engine tier or a shortcut
around analytical design.

### WP03 — Align early mobile feasibility with the master sequence

**New scheduling clarification; high portability value; F1/P1/T2 and affected consumers; I1 after relevant D1 design.**

The [platform module](modules/PLATFORM-RUNTIME.md) already asks for early native mobile shell validation. The master
construction table leaves this implicit under “selected platform adapters,” while complete native delivery appears
at I4. Make the existing early gate explicit before substantial I2/I3 code depends on unqualified host assumptions.

After deriving the host contracts, prove a thin shell on named iOS and Android development devices: launch, a minimal
world tick, boot content, surface/lifecycle recovery and basic input. Add audio/IME or other bridge behavior at this
stage only where the chosen shared API depends on it. Full accessibility, sustained thermal performance, visual
qualification, stores and broad device coverage remain later. The initial broader proposal to test all mobile
subsystems before D1 design is narrowed; measurements validate a designed boundary rather than discover it blindly.

### WP04 — State what an SDK release promises to existing games

**New support-policy gap; medium; SDK owner/F2/F3/F4/T2; before external release or promised upgrade support.**

Versioned schemas, saves, package locks and release-matched skills/examples already exist in the proposal. They do not
answer what happens when a game moves from SDK N to N+1 with changed source APIs, custom passes, content formats and
player saves. A pinned-release-only initial policy is valid; automatic upgrades and a stable binary ABI are not required.

State the initial support/availability policy and whether upgrades are offered. Before a breaking upgrade or
multi-release compatibility claim, exercise an ordinary game, a public extension and a save across versions, including
abort/rollback and required recook/manual source changes. Do not silently rewrite game-owned code. This belongs later
than the first single-release starter and must not become another I1 prerequisite.

### WP05 — Prove spatial authoring, not just a good inspector

**New acceptance clarification; medium; T1/W1/F3/F4; before I3 artist-usability claims.**

Placement, transforms, typed operations and prefab editing are intended in the plan. The minimum-workbench checklist
could nevertheless pass with selection and property panels alone. Add a short trace: import a prop; place, rotate,
scale and duplicate/instance it; view it through the actual game camera; undo; reparent; reimport; verify placement and
authored material overrides survive; build the selected target. Include WP01's unrepresentable case with clear rejection
or exact supported behavior. No full modeling suite, node editor or new source database follows from this test.

### WP06 — Make inherited open findings visible in maturity queries

**New status-navigation issue; low, fix before assigning D2 from leaf-only queries; maturity coordinator/E1.**

The E1 parent has A04/A05 open while atmosphere/water child rows say their historical local integration review is
reconciled. An agent filtering leaves could mistake that for complete family readiness. Parent links already exist.
Expose relevant ancestor findings in an effective review view, or explicitly require checking parent scope alongside
local status. Avoid copying every parent issue into every child: A04 does not automatically block unrelated vegetation.
This is metadata interpretation, not another physics defect. The maturity note now clarifies the reading rule; an
automated effective-status query remains unimplemented. Historical reviews are retained.

## Existing gaps remain the controlling design work

[Audit A01–A05](ARCHITECTURE-AUDIT.md) remains open: play-session isolation, coherent external source admission/recovery,
shader demand/readiness, water-domain exchange and overlapping media. They are not new discoveries and were not closed
by these reviews. A01/A02 join the applicable identity/source contracts in D1; A03 precedes the first cooked rendering
path that needs it. A04/A05 gate the chosen water/media combinations, not every simple game. Minimum devices, ordinary
networking scope, exact budgets and library choices remain openly declared decisions, not newly discovered defects.

## Recommended next sequence and disposition

1. D0 chooses contrasting qualification workloads and initial authoring/build/player target configurations, while
   preserving the full reusable-engine/platform ambition. Distinguish estimates from later device qualification.
2. D1/D2 resolve the mechanisms required by the first slice: A01/A02, WP01 and applicable A03, plus the chosen host,
   source, world and rendering contracts. Continue coordinating other families without forcing optional model work
   onto the first slice's critical path.
3. Make the existing I1 headless result and early platform shell checks explicit; then produce the named small
   rendered/one-target-exported game from the relevant I2/I3/I4 mechanisms.
4. Qualify the larger integrated scene and artist spatial-authoring trace, then complete target-specific I4 claims.
   Define SDK support before external release; validate upgrades when that promise actually exists.

These are review recommendations, not an automatically selected implementation backlog. No transform representation,
SDK support window, target minimum or new runtime feature was chosen in this pass. The construction/module contracts
remain unchanged. This review is recorded in maturity and tracking; only the parent-status reading clarification is
applied now. The next useful progress is resolving the named decisions and inspecting the resulting implementation
against them, rather than another broad consistency review of the same prose.
