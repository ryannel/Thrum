# Water integration and proof plan

**V1 reset (3 November 2026): the regional-optics portion below is parked with R3 until M9; water dynamics remain later M8 work.**

1 November 2026, revised 2 November. **D2-water B/C specification for bounded
dynamics and regional optics; focused actual-interface light remains an open
family capability. No implementation or qualification.**
The [commission](WATER.md) remains the family inventory. [Dynamics](WATER-DYNAMICS.md),
[whitewater](WATER-APPEARANCE.md) and [optics](WATER-OPTICS.md) select mechanisms, approximations and real limits.
This document connects them under existing C4 ownership; it does not revise the smaller initial specification,
create a second foundation queue or commission a water runtime ahead of the engine.

## State and product boundaries

| Owner / product | Required content and validity | Consumer / failure rule |
| --- | --- | --- |
| F4/W2 physical package | Model/version, fixed domain and edge topology, bed/solid/hull geometry, units, pressure supports, wet masks, symbols/filter plans, source profiles and exact dependency closure | E1/P3 admission pins the complete swept interaction support independent of render/stream/peer interest. Missing support is unavailable, never calm water |
| E1 dynamics endpoint | Physical h/bed, total flux and private integrator phase; applied per-hull pressure identity, independent reference datum, inventory/boundary ledgers, actual source/transit state and event cursors | Batched CPU/headless queries and next E1/P3 step. A renderer's filtered normal is not a force/query input |
| P3 body endpoint | Accepted pose/velocity and body properties, cooked volume/drag geometry, force/torque records and source identity | P3 alone integrates. Same admitted geometric algorithm serves floaters, hulls and fixed-body surrogates; object labels do not select private wake emitters |
| E1 coarse appearance | Front ancestry, area/gas/size populations, actual airborne liquid versus optical tracers, birth/consumption identities and declared proxy/actual work | Save and optional confirmed projections. V5's fine samples are replaceable and cannot create physical deposits |
| E1/V2 surface products | Endpoint pair/time, domain/branch/chart revision, P/J/P_t, distinct fluid/attachment carrier, support/status/error and modelled residual | V2 geometry, V3 intersections/caustics and V5 attachment use the same evaluator and coverage |
| F4/V2 optical support | Rigid regional BVH, exact material/cutout dependencies, accepted poses, masks, geometric error, complete extent and bounded traversal | V3 first-hit/incident-light visibility. A visit cap or absent receiver is not a shadow or a CoveredMiss |
| V3/V4 view products | Through-water light partition, ordered media events, confidence/motion/reactivity, capture versions and actual prior image time | Independent view histories; bounded leases/fences. No per-view physics or second Sun |

All payloads carry world/epoch, profile/model and dependency revisions, tick/substep/source interval where relevant,
plus readiness/error status. Volumes are m³, discharge m³/s or flux m²/s as named, momentum N s, energy J, pressure Pa,
foam area m² and gas volume m³. Work-kind tags distinguish incident, actually dissipated, external exchange and
appearance proxy. The concrete Rust layout is private; these quantities/semantics precede any new public API.

Use Thrum's ordinary optional provider registration and typed query/component contributions. Admit dependencies
before enabling a capability, reject incompatible combinations, and remove consumers before a required provider.
No water-specific service locator, generic coupling framework or speculative crate is needed.

## A complete committed tick

The baseline remains X[n] queries followed by the next committed state. The wider interactive profile explicitly
selects joint bounded E1/P3 substeps beneath that public clock:

1. Reserve candidate working state and publication bytes/counts before mutation, including source/transit growth,
   coarse appearance and exported provider products. Pin physical support. Use F1/C02 admission and leases; do not
   hide allocations in an uncharged provider cache. Count unique shared allocations once and each real copy once.
2. Begin from one common candidate endpoint. Obtain physical queries, independent reference pressure, its previous
   applied identity and P3 loads. Prepare provisional P3 and fluid trajectories. Contact geometry uses the same
   accepted source package and candidate body motion; old poses cannot qualify a moving first hit.
3. Search actual transit intervals for the earliest solid/water hit. Shorten and recompute **both** fluid and body
   candidates to that event, batch ties stably, partition transfer/momentum/energy, then integrate the remainder.
   Root iteration, candidate repeats, due substeps and contact events have admitted caps; their worst case is costed.
4. Release/deposit water once; derive pressure and whitewater from that same consumed transfer. Continuous arrivals
   integrate release-time intervals, not one ring per solver tick. Propulsor thrust/reaction are one agreed source.
   Apply no cosmetic wave impulse on top. Apply body force/torque once through P3.
5. Advance retained coarse appearance with the actual defined post-deposit flow, each newborn only over its lived
   birth-to-end interval. Validate positivity, support,
   model regime, volume/momentum/work ledgers, source consumption and complete product readiness. Only then publish
   one F1 world endpoint and release irreversible audio/wetness/game events from its committed event queue.

Provisional means private and discardable: the chosen P3 binding must offer candidate/checkpoint/reconstruct state
within its budget, including solver caches that affect continuation. A provider-local texture swap is insufficient.
On recoverable failure, retain the complete previous world endpoint/time and report bounded backpressure/fault;
do not advance bodies while fluid remains old. If rollback cannot be supplied, reject this coupling binding before
admission or stop the run on failure. No generic engine-wide rollback or automatic retry forever is promised.
Optional optical work may be canceled; a physical transfer cannot be dropped to keep the frame rate.

## Forces and energy without duplicate causes

Body pressure is derived from independent reference immersion; buoyancy samples the interaction surface with that
body's last-applied equilibrium particular solution removed. Its transient self-load bias and the global reference's
nonlocal source/load error remain explicit bounded approximations, not propagated physical waves. Changing either
source identity invalidates the common endpoint.
Nonreciprocal pressure/buoyancy is an explicit production approximation with an external exchange ledger, not a
hidden guarantee of rigid exclusion or energy conservation. Finite storage never receives a second hull-volume debit.

For an actual arrival, cache incoming momentum, outgoing retained-spray momentum, receiver carrier and credited liquid
once. The residual normal excess supplies mean traction or an isolated pulse according to the source's interval
semantics; it is not both. Its remaining energy is partitioned among physical wave work, physical spray/interface
work, air creation and unresolved loss. Optional fluctuations spend only arrived energy and have a validated mean-only
fallback. Air/foam proxy work never becomes a second physical energy account. Upward spray can increase normal reaction.

Motor disk throughput is existing water. Equal-opposite thrust changes stored flow momentum, which carries bubbles;
ideal wake kinetic power is not automatically dissipated foam power. The initial externally driven closure records
signed motor/body/fluid mismatch. A power-limited motor must jointly bound thrust and reaction; independent fluid
clamping is invalid. Exact rotor turbulence/cavitation and dynamic hull pressure correction remain outside this profile.

Physical contact uses the graph. The selected larger breaking curl is noncontact presentation with a separate error
envelope. Its whole lifetime deformation must avoid the **reachable** hull/query/camera support under the scene's
admitted motion bounds, or use an authored exclusion zone. Current-frame distance alone is insufficient. If that
cannot be proved, do not admit the curl; no last-second fade can repair the contract. A future contact-qualified curl
needs another physical representation. Closed optical volume is separately required for camera medium traversal.

## Cook, edits, saves and confirmed replicas

Cook physical geometry/topology and render optical support separately from the same versioned source. Validate
minimum wave/pressure/bed scale, complete edge/receiver closure, contact error, thin-film regime and visibility masks.
Expose rejected support and its reason. Prepare products before replacing an admitted run; failed cook leaves the
old run intact. Initial physical/profile/source edits restart from exact pins. Optical-only replacement may swap
atomically after reserving old/new/upload peaks and invalidating affected histories; it cannot change physical support.

P7 authority save retains all continuation state: physical fields/integrator phase, reference/applied pressure state,
actual source/transit intervals and ledgers, required coarse populations, consumed-event cursors, RNG/time and exact
source/model pins. Restart reproduces the admitted headless physical continuation; fine GPU particles/history are
derived and rebuilt. Saving only current surface height loses velocity, flight and source-off tails.

N1's confirmed projection is a different bounded product, not an authority save or whole-grid-per-peer copy.
Body pose and required water query state use one atomic compatible StateGroup, endpoint and content readiness under
C13. A client cannot query a new hull against stale incompatible water. Initial water prediction is off; replicas
interpolate confirmed compatible state and may reconstruct optional fine visuals within the declared surface-error
envelope. Reliable consumed-event cursors prevent duplicate births after join/reconnect. A game needing authoritative
foam visibility/navigation must retain its coarse facts in the physical compatibility/projection contract explicitly.

## Supported combinations and remaining family gaps

| Combination | Selection / boundary |
| --- | --- |
| Analytic sea or initial HLL shallow | Existing initial operators unchanged; do not inherit interactive or detached-transfer claims |
| One bounded interactive graph with resolved bed/shore, admitted hulls/floaters and propulsor reaction | Selected GH-based wider profile with explicit pressure approximation; queryable waves/current, wholly wet source footprints and no overlapping hull pressure supports initially |
| Same-provider finite stream/fall/pool | Selected actual discharge/transit/first-hit transfer and continuous mean traction, with bounded contact/event resolution |
| Thin calm rain/puddle graph | Selected basin filling/preconnected wet-edge exchange on an admitted orthogonal gentle-slope mesh; no advancing dry-slope runoff, floating body or inertial flood |
| Interactive graph plus thin film / analytic exterior | Named reservoir or actual released drop may connect through its finite source ledger; a general traversable heterogeneous seam is **not selected** |
| Prepared coast curl / dense appearance | Noncontact curl plus dilute causal populations only. Dense plume, 3D turbulence, arbitrarily intersecting curls and contact/occupancy are not supplied |
| Layered regional optics | Exact `BoundedAlpha2` beside explicitly approximate sorted alpha; bounded actual camera occupancy/covered regional geometry, up to four desktop interface events per camera branch, virtual-cover-plane Sun approximation and explicit overflow. Portable absorption is a smaller authored profile; focused actual-interface caustics and clear fall illumination remain open |

Richer spectra/spatially varying seas, exact open boundaries, automatic moving patches, thin-wall exclusion, inertial
thin flooding, advancing dry-slope runoff, traversable heterogeneous edges, overlapping hull pressure supports and physically traversable breakers
remain **unclosed family capabilities**. The family B/C checkbox cannot be completed by narrowing them away.
The selected bounded profile is nevertheless concrete enough for a dependency-gated analytical/numerical proof.

## Performance and disabled states

Use the [whole-engine allocation](../BASELINE-SPEC.md#budget-allocation-and-failure-rules), not a private water budget.
At 30 Hz presentation two 60 Hz ticks may be due. CPU/GPU overlap does not erase latency, working set or upload cost.

| Work | Cost to admit and compare before building |
| --- | --- |
| GH authority | `substeps × (filter pass count × cells + depth-bin transforms × cells log cells + transport + pressure footprint + hull elements)`; include event repeats, reference solve and optional fluctuation response/trials. Benchmark actual corrected frozen-stage path against the initial local operator for their different supported tasks |
| Physical publication | Live fields plus candidate, integrator half-state, applied pressure support, query export and every leased version. Export compact sampled/derived query products when sufficient; never clone complete fields per view/peer. Test slow readers and replacement peaks |
| Transit and populations | Actual in-flight liquid has its own reserved capacity/backpressure. Proposed16,384 coarse appearance packets need about 1–1.25 MiB before candidates/leases; 250k fine samples add 8–12 MiB. Count queries, transport, births, sorting and upload, not only simulation arithmetic |
| Optics | Proposed 50k-triangle regional proxy roughly 4 MB plus old/new/upload copies. At 720p, 25% coverage and half-width/height tracing, 57,600 rays ×32 average BVH visits ≈59MB node traffic/view/frame before caches; captures, materials and lights add work |
| Interfaces/caustics | Four 16-byte full-res 720p layers ≈59MB; nearest full-res plus three half-res ≈25.8 MB. One 256² caustic tile is 65,536 beams. Tile/light/view counts, coverage, overdraw and histories are explicit multipliers |

These are operation/byte estimates, not frame-time measurements or certified profile counts. Derive actual storage
from selected structs and simultaneous ownership. Compare layout/SIMD/jobs/preparation strategies and hybrid versus
direct BVH tracing on equivalent admitted content. Cache only reusable work with complete version keys.

For the same physical profile, reduce duplicated prep/uploads/captures first, then unresolved/distant detail and fine
sample counts, classified ray/caustic rate/resolution, complete mean light instead of focused light, supported probes,
eligible thinner optical layers and internal output resolution. Record the lost cue at each step: concentration,
parallax, thickness, contact, glints or overlap. Recheck motion/history at every optical reduction. Do not silently
change physics resolution, timestep, source strength, pressure support or coarse totals as a graphics tier.

No water consumer means no water state/preparation/residency. Drawing off releases optical/fine work after leases and
fences while retained physical and promised coarse continuation evolve. Source-off stops release/birth, not transit,
waves or surviving populations. Fully removing coarse appearance is a declared lifecycle change with dependent
consumers checked; restoring it cannot promise the missing historical tail. Headless physics never waits on a GPU query.

## Dependency-ordered proof, before a showcase lake

| Proof | Prediction and rejection condition |
| --- | --- |
| Corrected GH and pressure rest | Verify dispersion/phase against the analytical constant-depth symbol, volume and pressure-balanced rest; independently detect wrong corrected sign, missing static offset, inconsistent frozen filter, dry/mask drift and pressure-source feedback. Repartition tests exercise coefficient changes |
| Least-established coupling: one geometric hull, then a second floater | Solve calm inventory/buoyancy, launch/turn/stop under declared pressure approximation, observe stored bow/wake and independent second-body force response. Current/fixed-body constant-depth comparison, wavelength/shape-volume error and body pressure identity must match the admitted model. Nonzero pixels or scheduled bobbing cannot pass |
| One continuous donor/fall/finite receiver | Measure donor+transit+receiver liquid, first moving water/solid hit, arrival interval integration, pressure/momentum/spray partition, source-off delay and remaining wave/appearance tails. Repeat with different tick/substep schedules: a tick-frequency ring train or event-count-dependent force fails |
| Thin fill/sill/drain | Rain on resolved basin/sill, nearly dry cells and preconnected wet-edge reversals; exact volume, continuous wet area and pairwise energy decay. Reject barrier leaks, whole-cell popping, dry-path jumps, head-order overshoot and unqualified smoothing. Compare wet-flow times with lubrication scaling inside its admitted regime; do not pass general advancing runoff |
| Bounded regional optical path and populations | Moving foam and rising bubbles, no duplicate population; dry hull over submerged bed, offscreen receiver/blocker, masked material, finite **virtual** source path and actual camera underwater ray; source-off, branch/layer/visit overflow and histories in two views. Reject missing-data shadows, doubled Sun/attenuation and static world-locked foam. Compare visible virtual-source loss against actual-interface reference separately |
| Focused-light challenger | Source-complete finite-Sun first hits and receiver-bin flux bounds in a restricted region, then focused single-scattering image reconstruction and cost. Refusal or attractive still frames do not qualify the wider focused/fall profile |
| Save/off/fault/replica | Restore actual phased state and event cursor; slow leases, failed candidate/cook, graphics-off, device loss, source changes, join/reconnect and incompatible content. Reject lost physical water, replay births, stale geometry or advanced bodies with old fluid |
| Sustained performance and experience | Name hardware/build/content/profile, warmup and long time series; collect CPU/GPU critical path, median/p95/p99/max, allocations, bytes, event/packet/capture counts and phase correlation. Watch ordinary motion and cue readability separately. Test low tiers, no renderer, two views, 30/60/high-refresh presentation and 30 Hz catch-up |

Prove the corrected operator and pressure coupling first in a small complete shared domain; then connect its actual
fall and appearance/optical consumers. Keep the same governing design and representative interactions as extent and
body count grow. The initial numerical proof can be headless; visual acceptance waits for I3's actual renderer and
target qualification. This task adds no water runtime, backend dependency, empty crate or transplanted lab.
