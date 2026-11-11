# Water — commission and wider-provider design

1 November 2026. **D2 alignment plus reviewed bounded-provider design; full-family B/C remains open.**
This refines `E1.water` within [Environment](ENVIRONMENT.md), under [C4](../CONTRACTS.md).
The [initial operators](../PLAY-ENVIRONMENT-SPEC.md#e1-analytic-sea-and-shallow-river) remain selected for their
admitted scope. This document carries the broader water brief into Thrum and names the work needed before it can
be implemented. The [sea and cloud port decision](../decisions/SEA-AND-CLOUD-PORT.md) adds the spectral sea profile
ported from the Broadsides Rust sea. The [water decision](../decisions/E1-GPU-WATER.md) owns where water runs,
body forces, replay and multiplayer: all water simulation runs on the GPU, wakes are physical interactive state,
bodies read water through an asynchronous batched query, multiplayer shares causes rather than water state, and
water needs no exact replay. The wider bounded selection is linked below; numerical algorithms still unselected remain decisions
to resolve analytically, not promises that a future experiment will supply a missing mechanism. No runtime is commissioned here.
The [2 November gap closure](../research/RENDER-ENVIRONMENT-TERRAIN-GAP-CLOSURE-2026-11-02.md)
adds a moving-shore rain operating model and provisional smooth finite shore
solver. It narrows R3 regional optics to an explicit virtual-source profile;
actual-interface focused caustics and focused volume light remain open.

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
| Interactive water | Body/impact disturbances propagate in state sampled by subsequent bodies and displayed surfaces | Selected bounded GH-based proposal below; authored hull/source adaptations and CPU cost remain unqualified |
| Breaking / detached / thin water extensions | Coherent breaking fronts, finite-volume falls/landings, puddle filling/overflow/runoff | Bounded detached transfer, dilute populations, noncontact curls and preconnected thin storage selected below; general dry-slope runoff and contact breaking remain open |

These are capability combinations, not five new plugins or a quality dropdown. A lake may be prescribed, shallow
or interactive according to its needs. A visual tier never silently substitutes one physical profile for another.
For the original cohesive-water experience, interactive water and the appropriate connected/breaking extensions
are the design target; the prescribed profile and R3 are useful smaller products, not substitutes for that target.
The current R3 fixture's cosmetic wake, decorative falls and blocked mixed-model hull crossing cannot qualify the
interactive or connected-transfer requirements. Conversely, those wider requirements do not invalidate R3's stated
small baseline. Preserve its exact bounds and analytical arguments; richer providers inherit none automatically.

## Current bounded selection

These are reviewed engineering recommendations for a wider profile, with published methods, derived adaptations and
authored parameters distinguished. They do not replace the selected initial operators or certify CPU/device capacity.

| Owning detailed design | Actual mechanism and limit |
| --- | --- |
| [Dynamics](WATER-DYNAMICS.md) | Complete generalized-heightfield bulk/dispersion/transport, geometry-derived pressure with independent reference and explicit nonreciprocal/transient/nonlocal error, same P3 buoyancy/drag, total-flux sources and finite fall/first-hit arrival. Thin storage is restricted to filling/preconnected wet edges |
| [Appearance](WATER-APPEARANCE.md) | Causal crest/contact/impact/ventilated-wash sources, separate gas/foam-area/liquid populations, defined flow/rise/surface transfer and source-off tails. Prepared breaking charts are noncontact; predictive entrainment is not fabricated |
| [Optics](WATER-OPTICS.md) | Common surface/attachment, bounded regional visibility with screen-space acceleration, one virtual-source light partition, oriented media/shells, bounded ordered layers and coverage-aware fallback under V3. Actual-interface focused caustics remain a separate open profile |
| [Integration and proof](WATER-INTEGRATION.md) | Common body/fluid candidates and event recomputation, atomic publication/leases, content/save/replica/off-state products, explicit operation/byte account and least-established coupling first |

[Independent cross-review](../../evidence/water/DESIGN-REVIEW-2026-11-01.md) found and corrected material coupling
errors before any build. It also found a genuine missing dry-slope transport mechanism: exact thin-cell volume/energy
can still create detached downhill wet patches. WAT-06 remains open beyond the bounded selection. Traversable mixed
domains, overlapping hull pressure supports, thin-wall exclusion, richer spatial sea spectra/open boundaries, moving
domains, contact-qualified breaking and physical breaker-loss closure likewise remain unselected wider requirements.
Numerical phase/stability/shore leakage, surrogate error, ordinary-motion quality and assembled target cost are unrun
qualification, a different category from those missing mechanisms. The full-family B/C step is not checked off.

## Coverage and decisions to close

| ID | Required outcome | Existing basis / missing detailed mechanism | Main owners |
| --- | --- | --- | --- |
| WAT-01 | Sea state with coherent phase, derivatives and orbital flow | Analytic components specified initially; richer spectra, spatial variation, filtering and prepared content require selection | E1, F4, V2/V3 |
| WAT-02 | Depth-dependent wave transformation, crest breaking, foam and shoreline response | Selected GH depth response plus tracked-front/noncontact presentation; physical breaker-loss/contact and broader coast remain open | E1, W2, V2/V5 |
| WAT-03 | Rivers/streams flow around bed and obstacles | Initial shallow baseline retained; wider GH current/pressure uses resolved bed/thick obstacles. Thin walls and unresolved/narrow geometry remain open | E1, W2, P3 |
| WAT-04 | A fall carries discharge into a finite receiving pool and creates a local response | Selected finite release intervals, moving first-hit event recomputation, volume/momentum/mean traction and source-off continuation in WATER-DYNAMICS; unimplemented | E1, P3, V2/V5, A1 |
| WAT-05 | Lakes/pools receive sources, waves and floaters | Selected bounded shared GH graph with finite inventory, run on the GPU, asynchronous batched queries and same P3 body path; shore stability and surrogate accuracy unqualified | E1, P3, W2 |
| WAT-06 | Rain fills depressions, overflows and drains without detached downhill blobs or area popping | Selected projected triangle storage and preconnected wet-edge exchange only. Advancing dry-slope film transport remains structurally open | E1 surfaces/water, W2, V3 |
| WAT-07 | Hulls and floaters receive and generate meaningful disturbances | Selected reference-immersion pressure surrogate plus same P3 geometry/drag, explicit transient/nonlocal errors and work ledger; no rigid exclusion guarantee | E1, P3, F4 |
| WAT-08 | Foam/bubbles form, travel, disperse and survive source-off | Selected causal area/gas/liquid populations and defined carriers, rise, transfer and source-off in WATER-APPEARANCE; calibration/dense flow unqualified or open | E1 appearance, V5, V3, A1 |
| WAT-09 | Reflection, refraction, water material, underwater light and caustics agree | Selected common evaluator, bounded regional visibility/ordered media, single light partition and temporal/error rules in WATER-OPTICS; device and image proof unrun | V3, V1, E1, V2/V4 |
| WAT-10 | Players and views share the water that matters (big waves, wake bumps, currents); exact replay not required | Shared causes, force providers, conformance and divergence tests in the water decision; save/readiness semantics in WATER-INTEGRATION; executable bindings and qualification absent | E1, F1/F3/F4, W1/W2, P7, N1 |

A stationary obstacle in current and a moving hull share relative-flow/contact facts, not automatically the same
numerical solver. A heightfield with no current advection or pressure/contact operator cannot generate the missing
standing disturbance by adding foam. Resolve required producers before selecting a visually attractive technique.
Layered water, swimming, submerged passages and traversable heterogeneous edges require explicit requested
capabilities; none follows merely from drawing transparent water. Bounded combinations are specified in WATER-INTEGRATION; wider combinations remain open.

## Fit the existing engine owners

| Boundary | Required exchange and authority | Detailed work before dependent code |
| --- | --- | --- |
| F1/W1 → E1/P3 | World/epoch, committed tick, units and admitted physical profile | Batch queries at X[n]; stage body/fluid/source changes and publish one complete next world endpoint; tighter coupling needs a joint substep schedule |
| W2/F4 → E1/P3 | Shared versioned bed/solid/hull geometry, domain topology, source and physical products | Domain support is independent of stream cells; pin the full required physical closure; derive proxy error and reject missing support |
| E1 ↔ P3 | Supported surface/flow/occupancy queries; body-relative contact, force/impulse and disturbance IDs | P3 alone integrates bodies. Define per-mode reaction destinations, approximation/work ledger and self-field treatment; never apply a reaction twice |
| E1 → V2/V3/V5 | Committed interfaces/derivatives, material populations, causal events and prior-state evaluation | Distinguish physical surface from bounded render residual; share attachment/location/time and invalidate affected histories on source/correction/topology changes |
| V3/V1 ↔ E1 | Shared scene lights/media, visibility and graph resources | V3 owns optical transport, V1 execution; E1 does not acquire a private Sun, display transform, renderer or GPU submission authority |
| E1 → P7/N1 | Durable state/forcing and separately bounded confirmed query/interaction projections | Saves, rendering and network products are different products. The network carries water causes, never grids; a player's own vessel may be predicted against local water under the water decision |
| E1 → surfaces/A1/P5 | Actual deposit/impact/contact facts, local flow/validity and physical obstacles | Wetness and audio read causal sources, not culled particle counts; navigation/occupancy cannot be inferred from a visible water mesh |

Use ordinary typed registration and named capabilities under Thrum's composition model. No water-specific service
locator, generic solver-coupling bus or mandatory crate split is commissioned. Separate dynamics, queries, appearance
and transport responsibilities without duplicating their shared state. Physical queries read the GPU state through an asynchronous batched query; an
authority with no GPU uses the declared CPU conformance model.

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
| Queries × bodies × wet-volume elements × ticks | Batch by committed snapshot/domain; prepare invariant hull geometry; compare analytic evaluation and maintained CPU query products. Selected: one batched GPU query per tick with asynchronous readback, one to two ticks late and latency-compensated; synchronous readback is forbidden |
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
| D2-water B — coupled dynamics selection | Bounded mechanisms in WATER-DYNAMICS/ APPEARANCE selected and cross-reviewed; full-family gaps above remain | Numerical proof is next for that bounded profile; required dry-slope/mixed-domain/contact-breaking mechanisms remain design work, not hidden future tuning |
| D2-water C — render/authoring/lifecycle closure | Bounded optics and integration products/order selected and cross-reviewed; executable bindings absent | Prove leases, complete common candidates, save/headless/replica products and optical coverage; wider C09 changes return to engine coordination |
| First implementation, when commissioned | Small complete interaction chosen from B's least-established consequential coupling; unit/reference checks and instrumentation alongside it | Preserve the defining interaction, inspect ordinary motion, reject plausible cosmetic substitutes; no need to build a whole lake first |
| Integration and target qualification | Relevant I1/I2a/D1/D2 dependencies, I3 presentation where needed, R3 baseline separately, then wider I3b/I4 workloads | Save/headless/two-view/source-off and whole-scene performance on named targets; no inheritance from historical demos |

[The central build plan](../../BUILD-PLAN.md) puts water after V1, where a water sandbox is a candidate website
demo. Water planning may proceed alongside V1; R0 does not acquire a water dependency. Do not create an
empty water crate, port the old TypeScript lab, add Rapier/wgpu dependencies early or promote experimental operators into C4.
The Broadsides Rust sea ports after G7 under the port decision's module-by-module policy.

## Evidence, reconciliation and maturity

[Source intake](../research/WATER-SOURCES.md) records fresh 2025/2026 checks and older lineage with actual inspection
scope. Its B/C refresh records decision-level inspections and comparisons; it is not exhaustive literature coverage or a completed selection for every row above.
Each consequential decision in B/C must refresh relevant successors across dynamics, appearance, optics and cost,
use substantive conference material, and record why an alternative changes the choice. Newness alone is insufficient.

[Alignment evidence](../../evidence/water/ALIGNMENT-2026-11-01.md) records the independent capability challenge and
historical lessons. Prior Broadsides code/research is context, not a runtime/build dependency or transferred proof; lifted Rust/WESL
modules re-derive their goldens in Thrum under the port decision.
Thrum's current `E1.water` selection and initial specification status remain intact. Wider detailed design,
implementation, correctness, visual acceptance and platform performance remain separately unqualified. The reviewed
bounded B/C proposal and its open family gaps are recorded in the newer cross-review linked above.
