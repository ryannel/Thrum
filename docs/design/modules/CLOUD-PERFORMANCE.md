# Cloud performance execution contract

2 November2026. Supports [the active build queue](CLOUD-MIGRATION.md); this is not a second queue or a benchmark result.
The [governing design](CLOUDS.md#cost-and-profile-admission) owns proposed allocations. Scoped [standalone pilot](../../evidence/cloud-runtime-pilot-2026-11-02/README.md) and
[source-work checkpoint](../../evidence/cloud-runtime-work-2026-11-02/README.md) receipts exist; integrated/target-device
measurements remain unrun. Historical local M3 timings are adverse scoped evidence, not a coefficient for predicting M2 full skies.

## Cost ownership and first optimization priorities

Measure all retained consumers together. Count runtime weather/source updates, source/index evaluation, density fetches, local and cached light,
Sun/ambient/environment paths, reconstruction, CPU preparation and transfers. A request cap is not a useful forecast.
The old234.9M Sun/402.7M ambient endpoint ceilings are historical candidate limits, not the production schedule.

| Focus / first stage | Concrete work and comparison | Rejection / evidence |
| --- | --- | --- |
| CPU data/layout, P0/P3a | Stable numeric IDs/revisions, packed event/region arrays, reusable typed scratch and bounded dirty sets. Tiny direct scan is the oracle; compare packed cell-to-event ranges/adjacency at regional size. | Count actual node/candidate/index visits, allocations/bytes, GC and p50/p95/p99 CPU preparation. An index loses if total critical path/peak gets worse despite a faster GPU helper. |
| Submission/observation, P0 | Helpers encode into the owner graph; bounded asynchronous timestamp/counter ring. No per-frame GPU completion/map/readback wait in input/world/update path. Capture/probe mode is separate from quiet timing. | Old `cu-preview.js::renderFrame` maps/awaits completion/reads counters each frame; do not copy this cadence. Compare observer-off/on overhead; unavailable timings are not queue-time substitutes. |
| Source sharing, P1a/P3a | Measure current field preparation versus repeated source evaluation across **all** readers at equal time/quality. Include MacCormack passes/scratch, halo exchange, source growth/mips, cache locality and static detail fetches. | No per-view source simulation; no presumed compression/sharing gain. Reject content that only fits by holding phase or silently evicting a retained source. |
| Index/dirty propagation, P3a | Compare at least two meaningful tile/column granularities, direct scan versus packed index, old/new swept support and low-Sun influence. | Measure dependency edges, dirty unique regions, exact reused regions, metadata/hash bytes and CPU discovery + GPU rebuild work. If influence is global, charge the global rebuild. |
| Light/sample economy, P3b/P4b | Attribute density vs lighting. Measure selected disjoint mixed-resolution rays, current point-voxel Sun/ambient cache,2local Sun samples, direct-ray fallback, BSM receivers and one MS octave. Selective lighting-rate sharing is optional after a measured deficit. | Preserve per-pixel depth/fog/opacity and fresh silhouette cues; subgroup/workgroup paths need feature/occupancy qualification. Smolder console numbers and async-overlap claims do not certify WebGPU. |
| Bandwidth/format, P3b | Portable packing, anisotropic fields, guards/mips and conservative empty skipping; format/filter capability queries, physical layout and decoded precision. | Count actual texture reads/bytes/occupancy. Quantized occupancy must not skip thin cloud; lower bytes that increase traversal/cook cost need total-graph comparison. |
| Pipelines/cold work, P0/P2b/P3b | Finite shader-demand manifest by admitted optical layout/quality/view, not event/time. Bounded required prewarm; record first useful frame and pipeline creation. | First new genus/storm, quality/resize/device recovery cannot hide compile stalls behind a warm GPU score. Never compile per event or weather tick. |
| Per-view work, P4b/P7 | Independent main/reflection/environment ray budget, histories and fresh masks; measured net reconstruction/cadence. | No main image reused as reflected world; no fog/surface shift or stale birth accepted for fewer ray pixels. Proxy versus real rough-water consumers stay separate. |

Only proof-scale structures are built at P0. Do not implement an arbitrary spatial database, allocator or shader-variant
framework in advance of conformance scenes. The competing simple layout stays executable for correctness and cost.

## Bounded scheduling and actual clocks

P0 chooses a small explicit in-flight capacity (initial candidate: up to3 submitted frame graphs, one latest unsubmitted
preparation request per active epoch). Include every retained frame/lease in the peak inventory. Coalesce unsubmitted
work to the newest requested source time; cancel/discard superseded CPU/decode requests on seek/epoch change. Submitted
GPU work is not magically cancelled: mark its results obsolete, retire by fence and count wasted work/bytes. Never
overwrite an in-flight page. Input/world authority continues without awaiting GPU completion.

Record requested source time, candidate-ready time, submitted graph/time, completed sequence and observed canvas
frame/time separately. A requestAnimationFrame callback is not physical presentation; queue completion is not displayed
FPS. Use browser presentation/trace signals only where available and name their scope. Capture a paused frame with
matching render metadata; DOM HUD alone cannot certify which source generated its pixels. Old images keep their old
time/status; no held phase, relabelled generation or unbounded backlog can count as a timely new frame.

For steady, birth, low-Sun sweep, cut/seek, quality swap and64x: report enqueue-to-publication latency, pending/completed/
cancelled work and bytes, oldest outstanding obligation, queue high-water, deadline misses and input responsiveness.
If current light/environment repeatedly misses the profile's declared freshness deadline, revise the representation
or qualify a named age/error approximation against a current reference. A smaller kernel time with stale display fails.

## Residency and allocation ledger

The baseline's200MiB reservation is a **single simultaneous-lifetime proposal**: dynamic source96MiB (all retained
state/advection scratch/guards/mips/occupancy); reusable static/noise8MiB; CPU weather/cause/control8MiB; bounds/index8MiB;
light/shadow8MiB; views/history48MiB; staging/checkpoint16MiB combined CPU/GPU; air/environment8MiB. This replaces the
baked animation encoded/source pools, not an observed saving. CPU mirrors and old/new/fenced products are charged.
Static inputs can regenerate from admitted recipes; visual state needs a bounded recoverable checkpoint/forcing policy.

Inventory each allocation: owner, format/physical dimensions, bytes, multiplicity, retained CPU/GPU copies, created/
last-reader/reclaim events and old/new overlap. Include source revisions, update/activation/phase/rest-coordinate fields if used, source snapshot cache,
index metadata, compiled pipelines where visible, binding/descriptor objects, browser JS/Wasm heaps and driver scope.
Track cold load, seek/cache thrash, quality switch, failed allocation and device recovery, not merely warm logical voxels.
The312MiB arithmetic remainder is not available process headroom. P7 measures the fixture browser process peak and names
unobservable driver components; P8 repeats with actual engine scene/consumers. Unknown memory scope stays unqualified.

Steady hot paths reuse storage; record and justify remaining required per-frame objects such as swapchain views.
Batch uploads and stable bindings where lifetimes allow; avoid stringified cache keys, temporary per-event maps and
new typed buffers per mip/frame. Allocation-free authority logic does not imply the browser driver allocates nothing.
Pool capacity, cache admission/miss and failure counters must be inspectable; no silent lost events or unlimited growth.
Bulk cooking is offline. Admission/decode that would violate the UI/authority deadline runs in a bounded worker/job
with explicit ownership/transfer bytes and cancellation; moving it off-thread does not remove its work/peak from the ledger.

## Early scaling ledger and gates

P3a counts `index_visits + candidate_tests + coefficient_fetches + local_light_steps + cache_fetches + Sun/ambient/env
endpoints + dispatches + upload/readback bytes`, not only pixels*steps. Run1/2/16/64 and regional unique source groups,
shared static ingredients/independently evolving weather cells, sparse/full coverage, low Sun, offscreen blockers and1x/16x/64x. Record distributions per region
and whole graph, including the worst region. Compare selected ordinary footprint mips to direct resolved demand with correlated gaps/terminals. No special
far-transfer summary is a prerequisite. Report actual interior Sun-ray work, cloud-shadowed air/fog and native fresh
work in the5ms cloud account; unshadowed clear air/composition is separate.

Live source update is O(active physical voxels*fixed source ticks + emitter/control work). Count forward/reverse/
correction advection, reaction/conversion, growth halos, occupancy/mips, checkpoint/restore and readers.2M useful
voxels*4B*5 state/scratch copies is40MiB before guards/mips/extra leases; it is an example, not full-sky admission.
At16x/64x the fixed-tick multiplier is real. Bound per-frame catch-up jobs and report preview backlog/deadlines rather
than skip state or block input. The5ms target compares normal1x; accelerated preview throughput/control fidelity is
separate. Current light domains32³+16³+16³=40,960 cells,*(32Sun+4*32ambient)=6,553,600 coarse source steps before skips,
plus local view Sun rays and maps. Count all current light-cache work, changed sources and complete-support fallbacks;
no8-frame amortization or offline all-cloud AO is presumed valid. Future slower cadence requires age/error proof.

Before P3b chooses expensive products, fill FAIR/BROKEN/OVERCAST/STORM steady/burst worksheets with measured count-to-time
ranges and byte/lifetime terms. Explicitly multiply actual endpoint fetch/decode, index visits, light/ray/view counts
and invalidation work. Use lower-bound unavoidable work and an observed candidate reduction, not a cap or guessed
constant throughput. State which figures are direct measurements, extrapolations and missing terms.

P4a may pass optical/visual scope on an available host while M2 is absent. Its cost-risk outcome is separately:
**quantified plausible route**, **structural failure/rework**, or **unqualified missing measurements**. To justify continued
content work, the worksheet must show bounded retained-consumer work and named risks; absence of M2 alone is neither
an observed budget failure nor a cost pass. If no quantified route exists or exact fallback is common at required
coverage, revise before broad variants. Do not reduce caps until blockers/quality disappear from the test.

Before freezing production choices, P3b/P4b also budgets a production-like rough-reflection/depth/transparency proxy:
independent reflected directions/roughness levels, sample/filter/history demand, interface depth ordering and extra
ray/byte/latency envelope. It does not implement the water solver or qualify water appearance. The mirror alone cannot
bound those consumers. P8 supplies actual rough water and engine scene, and requalifies the frame/memory targets.

## Measurement and reduction decisions

For P7: same accepted quality and frozen source/build/view/DPR/scenario; separate warm steady, cold first-use and
transition/cut/birth/recovery bursts. Initial admission protocol is at least60s warm steady per FS scene plus each named
burst, after recorded warmup; report p50/p95/p99/max and missed frames. Proposed target comparison uses p95 cloudGPU5ms,
fixture totalGPU16ms, desired cloudCPU0.5ms/fullCPU8ms and measured fixture-browser peak512MiB. Record recurring freshness/
input deadline misses as failure regardless of median. This is **fixture admission**, never complete Thrum admission.
P8 reruns on actual engine browser/native profiles (native3/10ms), actual consumers and matching quality. No unseen
counter/device or summed overlapping timestamps can be marked passed. Longer runs follow when leaks/repetition need it.

A/A checks establish observer/noise floor; alternate matched A/B blocks and retain adverse results. No capture, Python
cook or concurrent GPU benchmark during quiet timing. Attribute CPU preparation, submit, GPU execution, waits/backlog,
presentation scope and observer work separately. An unavailable GPU timestamp is labelled unavailable, not inferred.

Keep three decision ledgers: exact work removal; qualified representation substitution; perceptual reduction. Each row
states hypothesis, changed work/bytes, extra preparation/residency, cue risks, validity/lifecycle and measured whole-graph
result. Rank by observed bottleneck (CPU/GC, bandwidth/residency, light, fill, reconstruction), not a universal ordering.
Re-accept moving quality for any approximation. A microbenchmark alone never establishes integrated gain.

## Retained velocity evaluation options

These are retained execution options for the earlier flow provider, whose morphology is now rejected. The [finite-material candidate](CLOUD-AUTHORED-SOURCE.md) has no per-voxel flow consumer and allocates no unused velocity cache. Preserve the code, equivalence oracles and full-sky comparison option; a change of source technique is not evidence that caching loses. Compare equivalent outputs within a relevant admitted provider, and record when an old option is inapplicable rather than claiming an unmatched speedup.

The native source retains two private execution paths: evaluate prescribed velocity in each q pass, or prepare a
same-time float32 vector field once per source tick and load it in those passes. This is GPU arithmetic reuse;
no CPU speedup is presumed. At the dense pilot's614,400 cells, a vec4f field is9.375MiB. It is scratch for the
ordered source work, not an allocation per view or three retained histories. Changed queue/lease concurrency must
charge any additional copies.

For N allocated cells, A active cells and effective bandwidth B, the first cost comparison is direct3*C_u*A versus
cached C_u*A plus dispatch/barrier work and roughly(16*N+48*A)/B bytes of field writes/three reads per tick.
The full-domain write is charged even where a mask stores zero. Actual cache reuse/pressure, source masks and
half-resolution carrier work alter this simple account; measure the assembled graph. More elaborate motion can
increase C_u and reverse a small-cell result, while a large field can exceed cache capacity and make reads dearer.

**Keep both implementations and their whole-field differential oracle.** The single-cloud pilot provides no final
selection for a full sky. The owning P7 checkbox in [the build queue](CLOUD-MIGRATION.md#p7--full-standalone-skies-and-fixture-admission)
requires matched full-sky A/B measurements across family/layer mixtures and storm transitions, with identical source,
accepted images/motion and lighting. Report CPU, source/full-graph GPU time, physical peak/residency, effective
bandwidth, preparation and fence overlap; no memory-for-compute win follows from mathematical equivalence alone.
Retain1x and accelerated results separately. Default selection may remain direct while this wider test is pending;
that is not removal of the cached path.

## Bounded storm source cost checkpoint

The [Cb source pilot](../../evidence/cloud-storm-source-2026-11-02/README.md) uses128×80×128 over24km horizontally to
contain a growing anvil without discarding the prior source pitch. Its53.1MiB logical ledger includes40MiB q
textures,2.5MiB carriers and149,796extra native-noise mip bytes; optional float32 velocity storage would add20MiB.
Six new backtraced carrier-neighbour evaluations add983,040 velocity/rest queries each per source tick, shared by all
optical readers rather than repeated per ray. Account for filtered-noise bandwidth too. The10s available-host screens
deliver1x/16x queued source progress, but whole-GPU timestamps are rejected and target devices/full skies/process peaks
remain unqualified. A follow-up phase-age/source attribution correction has fresh1x/16x screens with the same limits;
older birth/age telemetry is invalid for mixed Cb phases, while density/image observations remain historical.
The wider chart again reaches its edge on the1060s trace; no indefinite-source or region-exchange
admission follows. Organised source morphology must be accepted before optimizing an unacceptable visual workload.

## Finite Cu CPU source cost and work removal

The [isolated source cost checkpoint](../../evidence/cloud-source-cost-2026-11-03/README.md) separates ticks,
first exposure publication and same-tick presentation for1/2/16/64 independent copies, with retained adverse memory
stops. Source copies reuse IDs and are a kernel workload, not an admitted multi-cloud sky. Dry exposure publication
is the largest measured CPU source kernel; hoisting slot-invariant selection and bulk F64→F32 writes improves its
16-copy first-publication kernel8.4–10.0× in matched runs. An immutable birth-Q dual cache removes repeated affine
allocations while preserving the eight-corner arithmetic. Further wet presentation improves1.3–1.4× against the
publication-only candidate; dry accelerated totals show no consistent additional win. The two matched windows
must not be compounded into a frame-speedup claim.

These are independently reviewed private execution changes, with exact finite source/pose/bounds differentials.
The [CPU-only integrated17f build](../../evidence/cloud-source-integration-2026-11-03/README.md) now adopts them for
the bounded prototype after actual matched-pixel, drying-query, lifecycle and retained-consumer checks; they do not
change weather, shrink optical support or accept GPU/device/full-sky budgets. The original and failed
cache attempt remain recoverable. Matched integrated CPU cadence, whole-graph GPU and process
memory remain unqualified; the observed integration images/motion have only the bounded scope recorded above. Keep the optional full-sky velocity-cache comparison in P7.


The [GPU query work-removal screen](../../evidence/cloud-query-screen-2026-11-03/README.md) keeps two rejected
strict-bit candidates, exact running instruments and actual drying receipts. Fewer theoretical texture/exposure
loads did not pass the predeclared same-device bit gate, despite small observed differences and the older CPU
oracle passing. Neither is adopted or timed for a gain. Preserve the original GPU path; any later whole-pipeline
zero-exposure dispatch or independently justified precision contract is a separate decision and proof. Avoid making
this optional query study a blocker for source anatomy or the required complete renderer path.


The [two-cell sentinel readiness brief](../../evidence/cloud-sentinel-readiness-2026-11-03/README.md) derives actual
prototype scaling terms and missing cost-recorder/consumer work. Dense control banks copied to512 cells would require
460.594MiB CPU plus153.531MiB GPU before shared inputs, views, heap and driver costs. This rules out promoting the
prototype's per-virtual-cell allocation topology. Shared zeros/lazy exposure pages are a candidate exact reduction,
not a proven bound on retained nonzero histories. Two-cell integration can proceed with an explicit small ledger,
while P7 still requires a measured resident/virtual/cause recovery strategy and all-positive burst coverage.


The [two-cell CPU producer](../../evidence/cloud-two-cell-source-2026-11-03/README.md) now exercises the fixed pair
without concatenating exposure grids: borrowed publications expire on commit/fault, and probes request owned copies.
Twelve focused and six native checks plus existing source/render checks pass in their recorded functional scope.
An accidentally concurrent render-test launch exceeded the256MiB guard before termination; that adverse receipt remains.
The identical serial test run passes under the unchanged guard. Neither this scheduling recovery nor CPU functional
RSS establishes two-cell renderer cost, residency, throughput or the production sky budget. Actual multi-cell optical
passes, retained consumers and valid per-pass GPU timing remain on P4a's critical path.


The [first two-cloud optical proof](../../evidence/cloud-two-cell-optics-2026-11-03/README.md) passes finite source/direct/fallback probes but fails sustained playback with two observed Metal device losses, including a recorder-free GPU Hang on the available M3 Max. FixedA-only cache coverage exposes a147–291 versus3 coefficient-query multiplier at off-cache occupied samples; this is a cost hypothesis requiring correction/qualification, not a measured GPU speedup or isolated hang attribution. The conservative nonoverlap attribution gate nulls overlapping cross-pass durations; this does not diagnose invalid counters or premature resource consumption. Full fallback correctness cannot substitute for a viable retained-view workload; current cache coverage and failed-record retention must be corrected before expansion. M2/5ms and complete fixture residency remain unqualified.


The [covered-light correction](../../evidence/cloud-two-cell-covered-light-2026-11-03/README.md) adds one147,456B light texture and768B bounded uniforms, rebuilding both complete-world charts each frame. Three actual recordings complete (normal90s,16x40s,low-Sun drying90s); this is stability evidence on M3 Max, not a measured speedup or target-device pass. The analytical chart/receiver ceiling rises to10,354,688 coefficient calls/frame while full-weight covered view samples avoid nested fallback. Cached transmittance errors around.06 remain unqualified; complete off-all-chart queries retain nonclear positive/negative checks. Whole-GPU time/residency remain unmeasured.

The [first isolated observed-policy cost run](../../evidence/cloud-observed-policy-cpu-2026-11-03/README.md) fails its predeclared4ms whole synchronous step+publication maximum: single-wet4.766750ms, single-dry5.234000ms, two-wet8.634250ms, all at420s. Retained GC/tails and lower percentiles do not change that outcome. Exact redundant work removal is the next bounded candidate; no runtime enablement, altered timing policy or threshold relaxation follows from functional success.


The [subsequent exact-work checkpoints](../../evidence/cloud-observed-policy-work-2026-11-03/README.md) preserve f57 source decisions, states, counters and publications, but still fail the same4ms whole-step maximum. Latest12777301 observations are single-wet1.832583ms, single-dry4.203417ms, two-wet3.029750ms; the separately pinned two-dry900 supplement reaches7.425500ms at420s (matched legacy3.378667ms). The wet-only two-cell screen did not cover this consequential active-front work. Both protocols are required; neither lower typical cost nor a passing wet case overrides the dry failures. All14 raw arrays/GC/adverse records and functional-only restoration results remain retained. No exact GC attribution or integrated/browser speedup follows.

Next exact-work hypothesis: a conservative complete-domain bound in coordinates moving with the held dry front can prove an entire forecast stays outside the dry band, avoiding repeated midpoint/weather queries. Keep legacy fallback for an unproved or unqualified case. The invariant180-row healthy-response profile can also be shared once lazily instead of rebuilt for each opportunity; charge preparation and residency, and keep legacy-only disabled cost. These are unmeasured implementation candidates, not a changed physical/source rule or passed gate.
