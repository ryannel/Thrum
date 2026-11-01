# Water — Thrum alignment and detailed-design commission

1 November 2026. **D2 planning, not a replacement baseline or a build-ready solver design.**
This refines `E1.water` within [Environment](ENVIRONMENT.md), under [C4](../CONTRACTS.md).
The [initial operators](../PLAY-ENVIRONMENT-SPEC.md#e1-analytic-sea-and-shallow-river) remain selected for their
admitted scope. This document carries the broader water brief into Thrum and names the work needed before it can
be implemented. Numerical algorithms not selected below remain decisions to resolve analytically, not promises
that a future experiment will supply a missing mechanism.

## Product scope and baseline fit

Water is optional for a game; its implementation must support useful combinations rather than impose all features.
The retained water brief covers sea/waves/coast, rivers/streams/falls, lakes/pools and rain/puddles; floating bodies,
interaction-generated waves, causal foam/bubbles, coherent optics and scalable cost. These are required design
coverage for the water family, not prerequisites for R0 or mandatory capabilities of every provider.
Predictive turbulence/air entrainment remains an optional research direction. The preferred production approach
uses published game-engine methods with explicit calibrated closures where full flow information is unavailable.

| Profile / role | What can be promised | Current selection and limit |
| --- | --- | --- |
| No water | No water state, queries, preparation, draws or resident products when no consumer requires them | Retain F2 omission; an invisible but required physical provider is a different combination |
| Prescribed water | Authored stage/flow or analytic sea, bounded buoyancy and optional decorative water effects | Selected initial set; finite storage, physical wakes and falling-water transfer are not implied |
| Finite shallow water | Stored volume, depth-averaged flow, sources/sinks, wet/dry and prescribed-reservoir exchange | Selected bounded HLL/hydrostatic-reconstruction set; not dispersive short waves or a waterfall solver |
| Interactive water | Body/impact disturbances propagate in state sampled by subsequent bodies and displayed surfaces | Required next design coverage; operator, reaction approximation and CPU cost are not yet selected |
| Breaking / detached / thin water extensions | Coherent breaking fronts, finite-volume falls/landings, puddle filling/overflow/runoff | Required wider coverage; admit only individually complete mechanisms and regime connections |

These are capability combinations, not five new plugins or a quality dropdown. A lake may be prescribed, shallow
or interactive according to its needs. A visual tier never silently substitutes one physical profile for another.
For the original cohesive-water experience, interactive water and the appropriate connected/breaking extensions
are the design target; the prescribed profile and R3 are useful smaller products, not substitutes for that target.
The current R3 fixture's cosmetic wake, decorative falls and blocked mixed-model hull crossing cannot qualify the
interactive or connected-transfer requirements. Conversely, those wider requirements do not invalidate R3's stated
small baseline. Preserve its exact bounds and analytical arguments; richer providers inherit none automatically.

## Coverage and decisions to close

| ID | Required outcome | Existing basis / missing detailed mechanism | Main owners |
| --- | --- | --- | --- |
| WAT-01 | Sea state with coherent phase, derivatives and orbital flow | Analytic components specified initially; richer spectra, spatial variation, filtering and prepared content require selection | E1, F4, V2/V3 |
| WAT-02 | Depth-dependent wave transformation, crest breaking, foam and shoreline response | Shallow storage is not dispersive coastal transformation or overturning geometry; select compatible representations and their actual transfers | E1, W2, V2/V5 |
| WAT-03 | Rivers/streams flow around bed and obstacles | Initial shallow operator is a bounded candidate; fine geometry, steady-current disturbance, narrow channels and contact closures remain scoped work | E1, W2, P3 |
| WAT-04 | A fall carries discharge into a finite receiving pool and creates a local response | Derive donor debit, flight/first receiver interception, pool credit, momentum/traction and delayed source-off behavior; decorative seams do not satisfy this | E1, P3, V2/V5, A1 |
| WAT-05 | Lakes/pools receive sources, waves and floaters | Distinguish prescribed level from finite storage and dispersive disturbance; bed/boundary/flow/query support must match | E1, P3, W2 |
| WAT-06 | Rain fills depressions, overflows and drains without detached downhill blobs or area popping | Surface-state runoff must feed actual receiving storage; select terrain/storage reconstruction, wet/dry and thin-flow regime | E1 surfaces/water, W2, V3 |
| WAT-07 | Hulls and floaters receive and generate meaningful disturbances | Retain one P3 body integrator; select E1/P3 source/reaction law and a queryable wave response, including contact pressure support and self-field policy | E1, P3, F4 |
| WAT-08 | Foam/bubbles form, travel, disperse and survive source-off | Separate crest/break, contact/impact and propulsor causes; define population units, birth, advection, rise/surface transfer, loss and optical reconstruction | E1 appearance, V5, V3, A1 |
| WAT-09 | Reflection, refraction, water material, underwater light and caustics agree | C08 owns transport; select interface/receiver visibility, optical properties, energy partitions, missing-data fallback and temporal behavior | V3, V1, E1, V2/V4 |
| WAT-10 | Same physical outcome across views, headless play, saves and optional replicas | C01/C02/C09/C10/C13 already govern; detailed products, codecs, query projections and readiness must implement them | E1, F1/F3/F4, W1/W2, P7, N1 |

A stationary obstacle in current and a moving hull share relative-flow/contact facts, not automatically the same
numerical solver. A heightfield with no current advection or pressure/contact operator cannot generate the missing
standing disturbance by adding foam. Resolve required producers before selecting a visually attractive technique.
Layered water, swimming, submerged passages and traversable heterogeneous edges require explicit requested
capabilities; none follows merely from drawing transparent water. Exact supported combinations remain to be selected.

## Fit the existing engine owners

| Boundary | Required exchange and authority | Detailed work before dependent code |
| --- | --- | --- |
| F1/W1 → E1/P3 | World/epoch, committed tick, units and admitted physical profile | Batch queries at X[n]; stage body/fluid/source changes and publish one complete next world endpoint; tighter coupling needs a joint substep schedule |
| W2/F4 → E1/P3 | Shared versioned bed/solid/hull geometry, domain topology, source and physical products | Domain support is independent of stream cells; pin the full required physical closure; derive proxy error and reject missing support |
| E1 ↔ P3 | Supported surface/flow/occupancy queries; body-relative contact, force/impulse and disturbance IDs | P3 alone integrates bodies. Define per-mode reaction destinations, approximation/work ledger and self-field treatment; never apply a reaction twice |
| E1 → V2/V3/V5 | Committed interfaces/derivatives, material populations, causal events and prior-state evaluation | Distinguish physical surface from bounded render residual; share attachment/location/time and invalidate affected histories on source/correction/topology changes |
| V3/V1 ↔ E1 | Shared scene lights/media, visibility and graph resources | V3 owns optical transport, V1 execution; E1 does not acquire a private Sun, display transform, renderer or GPU submission authority |
| E1 → P7/N1 | Durable state/forcing and separately bounded confirmed query/interaction projections | Saves, rendering and network products are different products; water prediction remains off without complete replay support |
| E1 → surfaces/A1/P5 | Actual deposit/impact/contact facts, local flow/validity and physical obstacles | Wetness and audio read causal sources, not culled particle counts; navigation/occupancy cannot be inferred from a visible water mesh |

Use ordinary typed registration and named capabilities under Thrum's composition model. No water-specific service
locator, generic solver-coupling bus or mandatory crate split is commissioned. Separate dynamics, queries, appearance
and transport responsibilities without duplicating their shared state. Baseline physical queries remain CPU-owned or
analytically reproducible headless; a different backend must separately meet authority, latency and target contracts.

## Analytical obligations and complete operating cases

The governing finite-domain balance is `V[next]-V[now] = delivered sources - delivered sinks - outward boundary volume`.
Detached transit adds a named airborne store; a finite receiver cannot receive volume before the donor releases it.
A conservative edge uses one integrated flux with opposite signs, while pressure/momentum and stability need their
own law. An imposed sea is an external reservoir, not a second conserved ocean volume. Physical wave phase/group
speed and material velocity are distinct quantities. A source must state its dimensions, sign, geometry, spectrum
or support, update interval and work/reaction policy; correct propagation cannot repair arbitrary forcing.

Detailed design must work through the following cases before a dependent runtime is commissioned:

1. **Two floating bodies:** one accelerates, turns, stops and leaves a wake; another crosses older wave state and
   responds through the same P3/E1 query path. Derive load/support from hull geometry. Do not use scheduled bobbing,
   a V-shaped brightness mask or an independent visual wake as the interactive capability. Include zero-input rest,
   source-off tail and same relative-flow facts at a fixed obstacle. Full reciprocal CFD is not required by this case;
   an external calibrated pressure/source approximation needs explicit state, self-field and work accounting.
2. **Stream → fall → finite pool:** derive donor discharge, airborne water, first moving-surface/solid contact,
   receiver volume/momentum, outward disturbances and foam/bubble births. One transfer controls geometry, sound and
   wetness. Turn the source off; flight delay and old waves/populations survive. Resolve multiple hits and stale receiver data.
3. **Coastal front and small wet area:** trace depth transformation/breaking/run-up, then separately rain filling a
   depression and crossing a sill. Do not claim the same grid/model handles both without a compatible scale/closure.
   Visible shoreline area must come from admitted surface/storage reconstruction, not whole-cell flashing or volume blobs.
4. **Shared world, two views and no renderer:** trace force queries, save/restart, replica content readiness and
   independent view histories. A missing query is unavailable, never calm water. A provider failure cannot publish
   advanced bodies with old fluid or invent a flux. Restore exact physical support/forcing before making queries usable.
5. **Creator change and reduced target:** author sources/bed/hull data, cook/validate, edit, restart from exact source,
   and inspect a failed cook without changing the previous admitted run. Turn graphics down or water drawing off;
   preserve retained rules, queries, waves and causal history, and retire only avoidable work after fences.

Each case needs a stated mechanism before implementation, and separate numerical, ordinary-motion, lifecycle and
device-cost proof. A numerical ripple or an attractive still cannot establish the whole interaction.

## Performance design before representation selection

Use [Thrum's performance policy](../AI-PERFORMANCE.md#performance-investment-from-the-first-design) and the sole
[whole-engine budgets](../BASELINE-SPEC.md#budget-allocation-and-failure-rules). R3's cell/wave/body counts are a
rejection workload, not a water allocation or demonstrated capacity. At 30 Hz presentation two 60 Hz ticks may be due.

| Work / residency multiplier | Design comparison and removable work |
| --- | --- |
| Active physical cells × substeps × stencil/pass cost | Compare supported CPU layouts/SIMD/jobs, prepared static topology and justified sparse/refined support. A cheap local operator still has CFL, wet/dry and resolution limits; an iterative/global operator must include its iterations/transforms |
| Queries × bodies × wet-volume elements × ticks | Batch by committed snapshot/domain; prepare invariant hull geometry; compare analytic evaluation and maintained CPU query products. Synchronous GPU readback on the force path is a latency risk, not a default |
| Population state × transport/birth/rise steps | Shared cause records, compact active support and explicit bounded visual population/LOD; rendering culling must not cancel required persistent state |
| Surface samples, refractive pixels/rays, lights and views × passes | Share view-independent geometry/field preparation; use filtered detail, classified work, history validity and declared scene/probe fallbacks; count receiver/occluder capture and bandwidth |
| Solver state + staged next state + leased snapshots + saves + queries | Account peak simultaneous owners/versions. Avoid cloning full grids per view/peer/tick; lease/backpressure must fit F1's still-active memory foundation work |
| Cook, cold start, replacement and source changes | Prepare exact dependency closure and target products; include pipeline/binary variants and old/new peaks. Physical model changes initially restart, rather than silently migrate |

Initial reduction ordering is a hypothesis to verify per bottleneck: remove duplicate preparation/uploads/captures;
share invariant work; reduce distant/unresolved render detail and visual population counts; then reduce optional
reflection/caustic sampling/rate/resolution with valid fallback; then use cheaper declared optical representations
and output resolution where worthwhile. Measure lost cues, bandwidth, residency and tail latency at each step.
Source-off still evolves tails. Changing physical resolution, timestep, boundary law or body feedback is a distinct
physical profile and compatibility decision, not a graphics optimization. No speedup or device qualification is claimed.

## Dependency-ordered work, without a second foundation queue

| Step | Deliverable / decision | Gate and rejection condition |
| --- | --- | --- |
| D2-water A — this alignment | Preserve baseline, full capability coverage, consumers, research intake and open decisions | Coverage/contract review; no runtime or wider solver selection claimed |
| D2-water B — coupled dynamics selection | Compare complete production mechanisms for WAT-02–08, derived scales, forcing, support, CPU queries, state/work ledgers and cost | An implementer can trace the cases without inventing a core operator; reject a method lacking the necessary producers or headless/phase support |
| D2-water C — render/authoring/lifecycle closure | Joint E1/P3/V2/V3/V5/F4/W2/P7/N1 schemas, optics, prior state, products, off-states and admitted combinations | Both sides of each consequential boundary accept the same quantities/time/validity; wider C09 changes return to engine coordination |
| First implementation, when commissioned | Small complete interaction chosen from B's least-established consequential coupling; unit/reference checks and instrumentation alongside it | Preserve the defining interaction, inspect ordinary motion, reject plausible cosmetic substitutes; no need to build a whole lake first |
| Integration and target qualification | Relevant I1/I2a/D1/D2 dependencies, I3 presentation where needed, R3 baseline separately, then wider I3b/I4 workloads | Save/headless/two-view/source-off and whole-scene performance on named targets; no inheritance from historical demos |

[The central build plan](../../BUILD-PLAN.md) retains I1 → source/save → the small rendered game and near-term
network path. Water planning may proceed alongside it; R0 does not acquire a water dependency. Do not create an
empty water crate, port the old lab, add Rapier/wgpu dependencies early or promote experimental operators into C4.

## Evidence, reconciliation and maturity

[Source intake](../research/WATER-SOURCES.md) records fresh 2025/2026 checks and older lineage with actual inspection
scope. It is not a new claim of exhaustive literature coverage or a completed selection for every row above.
Each consequential decision in B/C must refresh relevant successors across dynamics, appearance, optics and cost,
use substantive conference material, and record why an alternative changes the choice. Newness alone is insufficient.

[Alignment evidence](../../evidence/water/ALIGNMENT-2026-11-01.md) records the independent capability challenge and
historical lessons. Prior Broadsides code/research is context, not a runtime/build dependency or transferred proof.
Thrum's current `E1.water` selection and initial specification status remain intact. Wider detailed design,
implementation, correctness, visual acceptance and platform performance remain separately unqualified.
