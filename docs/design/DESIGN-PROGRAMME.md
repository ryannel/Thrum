# Coordinated engine design programme

> **Superseded for process (8 November 2026).** How work runs is now in
> [AI development: how work runs](AI-DEVELOPMENT.md#how-work-runs) and the level-named skills. This document is kept as
> the record of the 31 October design commission; its process instructions (model and effort, delegation, coordination) no longer apply.

Commissioned 31 October 2026. Owner: coordinating architect. Research workers use Sol, high reasoning, as requested.
The [brief](ARCHITECTURE-BRIEF.md) defines the product; [DESIGN](../provenance/CONTEXT.md#source-779e25e8668e) records user decisions.

## Design commission — historical scope

Develop a cohesive, analytically justified architecture and structured implementation plan for a custom
3D game engine targeting Mac, PC, browser and mobile app stores, with exceptional visual quality,
full use of each device's hardware, configurable capabilities and AI-oriented development. Use Sol agents at high reasoning for
bounded recursive research. Derive the system from mathematical/domain knowledge, published evidence, explicit
representations, governing relationships, approximations and cost/error models before implementation; use experiments
only to validate stated predictions and genuine empirical uncertainties, never as a substitute for missing design.
Actively coordinate throughout: the root understands and owns the assembled engine, gives agents whole-system context,
maintains one shared contract and ownership map, routes changes to affected producers and consumers, and resolves
duplication, conflicts and complete interaction paths before accepting module designs. Revise the skills using official
OpenAI Astra guidance. Deliver integrated architecture/module designs, evidence and decision records, platform/reuse
strategy, accurate multidimensional maturity tracking and a dependency-ordered construction/qualification plan. Do not
claim implementation, performance or visual acceptance from design work, and do not begin a runtime rewrite in this
design commission.

The design commission is recorded here for rationale and future coordinated work. Current construction scope is in [the build plan](../BUILD-PLAN.md); repository setup does not establish runtime maturity.

## Outcome and scope

Produce a cohesive high-level architecture and implementation programme for a custom 3D engine: selected ownership
and integration principles, justified provisional mechanisms, full responsibility coverage, source-backed module
designs, explicit unknowns, a maturity/evidence system and risk-ordered proofs. Do not present an overnight literature
study as an implemented or performance-qualified engine. Runtime construction follows these commissions later.

Support contrasting games and cameras on Mac, PC, browser and native mobile stores. Initial working interpretations
are Windows PC and Apple-silicon Mac, iOS and Android mobile, and a WebGPU browser demo profile; Linux, Intel Mac,
WebGL2 and exact device/OS floors need explicit qualification or later selection. These interpretations are proposals,
not silent exclusions from the user's broad platform goal. Pure 2D and massive multiplayer remain excluded; ordinary
networking is first-class, with the initial bounded N1 path selected in [NETWORK-SPEC](NETWORK-SPEC.md). No current game, water solver or demo selects the new architecture by existing.

## Orchestration

The root owns requirement coverage, shared contracts, total budgets, the decision register and the final synthesis.
The [shared contracts](CONTRACTS.md) are the actively maintained integration baseline, not an end-of-study appendix.
Every worker receives the whole-engine brief, this baseline, its neighboring responsibilities and the complete
scenarios it affects. The root reviews handoffs while work is underway and routes changes to both sides immediately.
Domain leads own cohesive designs, research, child commissions and responses to reviews. Leaves resolve a consequential
question; they do not invent new independent engine architectures. The concurrency limit is three workers alongside
the coordinator, so recursive commissions run in waves. Finishing a commission frees a slot; it does not terminate
its unresolved dependencies or erase a later review obligation. A domain report is not integrated until its inputs
have real producers, its consumers accept the outputs, and the coordinator has traced its role in the assembled engine.

Every commission contains its goal, parent requirements, scope and file ownership, inputs and consumers, consequential
questions, expected artifact/evidence, stopping condition and escalation rule. Changes to a shared boundary come back
to the coordinator and both sides of the boundary. Workers may recommend a new responsibility but must explain what
required behaviour it owns and why an existing owner cannot supply it. Unbounded delegation is not an outcome.

## Decomposition and stopping

1. Derive responsibility families and cross-system scenarios from the product, not existing folders.
2. Domain researchers compare complete representations and decompose their internal responsibilities.
3. Delegate leaf questions only when competing answers could change a representation, public contract, platform
   feasibility, quality ceiling or substantial cost. Trace promising sources and successors deeply enough to decide.
4. Synthesize each answer analytically: derive the mechanism, assumptions, invariants, coupled update relationships
   and cost/error model; then accept with conditions, reject with reasons, or record a precise remaining design gap.
   A child's report alone does not change the engine's selected contract.
5. Independently challenge the integrated system against original requirements and adverse transitions. Resolve
   actionable contradictions; keep unmeasured feasibility visibly open.

Stop a branch when it has a supported recommendation and clear failure conditions, when further sources would not
change its decision, or when only empirical qualification of an explicit prediction remains. Record that qualification
rather than fabricating certainty. An unresolved core representation, producer or coupling remains a design gap;
it cannot be promoted to a completed design simply by scheduling an experiment.
No fixed recursion depth or paper count implies quality. Reopen a branch when a sibling's evidence invalidates its
assumption. Do not keep researching a polished domain while leaving required capabilities unowned.

## Research standard

Use current official engine/platform sources, primary papers and supplements, author implementations, and substantive
production presentations. Record dates/versions, inspected sections/pages, evidence maturity and consequences. Research
materials support claims only to their actual scope. A slide deck is not a watched talk; an abstract is not an
implementation; a shipping feature on one platform is not cross-platform qualification. Keep source paraphrases
compact and respect source limits. Record inaccessible consequential evidence and how that changes confidence.

Classify decisions as selected by the user, recommended/provisional, open, rejected or superseded. Separate production
practice, research candidate and original hypothesis. Proven techniques are the baseline; experimental providers
compete at defined seams. Borrowing code requires exact source revision, license/notice retention and a maintenance
plan. Do not silently vendor a large framework or adopt another engine as host through indirect dependencies.

## Required synthesis

**Analytical design first (user correction):** do not discover the architecture by experimenting to see what works.
Use deep mathematical and domain knowledge to get the design as far as possible before implementation. State the
governing relationships, actual operators and data representations, boundary conditions, approximations, invariants,
coupled normal/transition behaviour, and complexity/error/cost estimates. Derive recommendations and explicitly rule
out unsuitable alternatives. Measurements subsequently qualify specific predictions about hardware, numerical or
perceptual behaviour. They do not replace the reasoning that selects the cohesive system.

The integrated design must explain a normal tick/frame, asynchronous content activation, two simultaneous views,
animated/physical movement, an environmental interaction, editing/save reload, quality changes, device loss and mobile
suspend/resume. Each important state/resource has one owner and an explicit lifetime. CPU jobs, GPU passes and asset
builds have separate dependency systems connected by explicit handoffs.

Maintain a navigable module map plus shared contracts for identity/coordinates/time, state authority, assets and
residency, rendering/material/environment exchange, tool operations and persistence. No universal data model, service
locator or event bus is selected merely to connect boxes. Design useful seams, not a wrapper around every library.

Allocate combined frame, memory and startup budgets per representative profile. Estimates and proposed budgets remain
labelled until measured on named builds/devices/content. Include sustained mobile operation, p95/p99 timing, loading
hitches, visual stability, disabled-feature costs and retained gameplay semantics. Separate full engine ambition from
initial qualification scenes and shipping profiles; the initial slice must preserve the interaction under test.

## Maturity and evidence

Track each responsibility independently across design, implementation, functional evidence, visual acceptance and
platform/performance qualification. Each change names artifact revision, evidence and remaining limitation. No single
completion percentage hides a weak required subsystem. Existing branch capabilities are unassessed candidates until
reconciled against the target; they do not inherit qualification from a different host or assembly.

An AI-oriented workflow requires inspectable schemas and diagnostics, explicit command/query contracts, deterministic
or reproducible headless scenarios where possible, source-to-runtime provenance, bounded context entry points and
small owned changes. Visual quality still needs rendered evidence, gated by automated checks and agent review with
asynchronous owner steer ([steering](AI-DEVELOPMENT.md#visual-review-and-human-steering)). No language-model service is
required at game runtime by this development requirement.

## Completion for this commission

Complete when required module families have coherent decompositions, source-backed choices and open questions; shared
contracts and simultaneous budgets are reconciled; build/reuse and migration decisions are explicit; an independent
review's consequential findings are dispositioned; skills are validated and forward-tested; and the maturity ledger
and ordered proof plan accurately describe the result. Qualification that needs hardware or implementation remains
planned and cannot be marked passed. Research is complete for this high-level stage, not for every future algorithm.

## High-level handoff, 1 November 2026

**Subsequent qualification:** the [architectural decision audit](ARCHITECTURE-AUDIT.md), commissioned after the user's
challenge about major-engine lessons, found five open boundaries and narrower decision follow-ups. The handoff below
records the earlier pass; its completion does not close those newly identified findings. Use the audit's ordered
follow-up before freezing affected detailed interfaces. Source-backed alternatives and developer/model-change paths
now supplement the earlier emphasis on internal consistency.

The commissioned high-level design pass is complete: seven family designs, 29 tracked responsibilities, C01–C12,
source studies, recursive temporal/content research, independent whole-engine and skill reviews, and a detailed
design/construction plan. Review found and closed actual cross-system gaps; see [dispositions](INTEGRATION-REVIEW.md).
The [integrated design](SYSTEM-DESIGN.md) is ready for review and detailed module commissions. The
[maturity ledger](maturity.json) records reconciled high-level design and unrun implementation/quality/platform evidence.
The plan still requires exact device/workload choices, detailed algorithms/APIs and real construction/qualification;
those were not silently marked delivered by this research pass.

Documentation validation passed on 1 November: both skill validators; relative links and heading anchors; Markdown
whitespace; and maturity-ledger ID/state/parent/contract references, artifact existence and content hashes. Independent
skill forward artifacts are retained in the project handoff. No runtime test or device measurement was run in this
documentation/design commission.
