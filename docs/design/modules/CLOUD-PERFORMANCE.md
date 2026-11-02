# Cloud performance execution contract

2 November2026. Supports [the active build queue](CLOUD-MIGRATION.md); this is not a second queue or a benchmark result.
The [governing design](CLOUDS.md#cost-and-profile-admission) owns proposed allocations. All new migration measurements
are unrun. Historical local M3 timings are adverse scoped evidence, not a coefficient for predicting M2 full skies.

## Cost ownership and first optimization priorities

Measure all retained consumers together. Count source/index evaluation, density fetches, local and cached light,
Sun/ambient/environment paths, reconstruction, CPU preparation and transfers. A request cap is not a useful forecast.
The old234.9M Sun/402.7M ambient endpoint ceilings are historical candidate limits, not the production schedule.

| Focus / first stage | Concrete work and comparison | Rejection / evidence |
| --- | --- | --- |
| CPU data/layout, P0/P3a | Stable numeric IDs/revisions, packed event/region arrays, reusable typed scratch and bounded dirty sets. Tiny direct scan is the oracle; compare packed cell-to-event ranges/adjacency at regional size. | Count actual node/candidate/index visits, allocations/bytes, GC and p50/p95/p99 CPU preparation. An index loses if total critical path/peak gets worse despite a faster GPU helper. |
| Submission/observation, P0 | Helpers encode into the owner graph; bounded asynchronous timestamp/counter ring. No per-frame GPU completion/map/readback wait in input/world/update path. Capture/probe mode is separate from quiet timing. | Old `cu-preview.js::renderFrame` maps/awaits completion/reads counters each frame; do not copy this cadence. Compare observer-off/on overhead; unavailable timings are not queue-time substitutes. |
| Source sharing, P1a/P3a | Compare shared immutable endpoint fetches with a blended current field across **all** actual readers, same optical time/quality. Include cache locality, two-fetch loads, LOD/extra channels and decode misses. | No per-age rebuild by default; no presumed compression/sharing gain. Reject content that only fits by holding phase or silently evicting a retained source. |
| Index/dirty propagation, P3a | Compare at least two meaningful tile/column granularities, direct scan versus packed index, old/new swept support and low-Sun influence. | Measure dependency edges, dirty unique regions, exact reused regions, metadata/hash bytes and CPU discovery + GPU rebuild work. If influence is global, charge the global rebuild. |
| Light/sample economy, P3b/P4b | Attribute density vs lighting. Compare mixed-resolution whole rays against per-pixel integration with selectively reduced light rate when lighting dominates. | Preserve per-pixel depth/fog/opacity and fresh silhouette cues; subgroup/workgroup paths need feature/occupancy qualification. Smolder console numbers and async-overlap claims do not certify WebGPU. |
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

The migration's184MiB reservation is a **single simultaneous-lifetime proposal**: encoded64MiB counts CPU+GPU encoded
copies **combined**; source48MiB counts physical decoded endpoint/current atlas, guards and mips; bounds/index8MiB;
light/shadow8MiB; views/history32MiB; staging16MiB counts concurrent CPU/GPU upload/readback slots; air/env8MiB.
Any decoded CPU mirror must be charged explicitly to a named pool; there is no free recovery duplicate. Required
recoverable masters can remain in admitted durable source storage rather than an unbounded live CPU cache.

Inventory each allocation: owner, format/physical dimensions, bytes, multiplicity, retained CPU/GPU copies, created/
last-reader/reclaim events and old/new overlap. Include source revisions, flow/phase fields if used, endpoint cache,
index metadata, compiled pipelines where visible, binding/descriptor objects, browser JS/Wasm heaps and driver scope.
Track cold load, seek/cache thrash, quality switch, failed allocation and device recovery, not merely warm logical voxels.
The328MiB arithmetic remainder is not available process headroom. P7 measures the fixture browser process peak and names
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
shared/independent ages, sparse/full coverage, low Sun, offscreen blockers and1x/16x/64x. Record distributions per region
and whole graph, including the worst region. Compare16/64-subray far transfer to direct resolved demand with correlated
gaps/terminals; include the joint depth storage/traversal cost needed for valid averaging.

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
