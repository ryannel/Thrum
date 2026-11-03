# Two-cell coupled sentinel: executable integration boundary

3November2026. Read-only source-owner readiness audit of actual build17fca3a6a1012a52, retained below as a construction brief. [CLOUDS](../../design/modules/CLOUDS.md) governs and [migration P4a](../../design/modules/CLOUD-MIGRATION.md#p4a--early-spatial-coupled-sentinel) remains the only checked queue. **No multi-cell implementation, runtime measurement or P4a pass is claimed here.** The [CPU-only integration](../cloud-source-integration-2026-11-03/README.md) is still a single-cell coupling reference; source anatomy remains unfinished.

Do not expand the library or implement the complete P3b renderer first. After pinning a source ingredient, build two independently evolving liquid formations through the retained optics, one shared clock, clear air/fog, receiver/mirror and environment. Their interaction must expose the current structural gaps, then supply the early cost-risk outcome before larger expansion.

## Concrete changes to the actual proof

`src/source/model.js` fixes one cell/root; `held-oracle.js` and `component-source.wgsl`/assembled `optics.wgsl` fix eight global slots and one global MAX. One box controls traversal, voxel-light coordinates and receiver coverage. Those exact files are recoverable from the17f archive. Running two independent copies does not close the integration contract.

Source/weather owner: introduce two persistent cell IDs/initial world roots and qualified `(cellID,cohortID)` identities. Advance both from one immutable quarter-second weather interval and command barrier. Publish one epoch/tick/time/alpha, formation ranges, revisions, controls and conservative support bounds. Keep birth programme, fresh strain, leases and irreversible exposure. No camera-created cells, per-reader stepping or duplicate weather advection.

Query/index owner: keep eight slots per formation and a bounded16-part table with explicit control offsets. Pointwise MAX constructs one formation; independent constituents ADD coefficients:

`qA=max(part amounts in A); qB=max(part amounts in B); sigmaCloud=.019*(qA+qB)`.

Global MAX loses independent material; summing BODY/crown proxies double-counts one formation. Fog adds its own RGB coefficients before the joint Beer integral. Keep complete direct two-group traversal as the oracle. Bounds enclose strict positive native/trilinear support, independently of the thresholded `.005` admission BVH. A small packed-bin comparison at two granularities can follow; overflow takes complete traversal or reports unavailable, never missing material. No general spatial database is needed.

Lighting/transport owner: separate source/index support, light-cache domain and retained receiver/view/environment demand. Every Sun/ambient ray queries all intersected groups to its actual terminal, including an offscreen group outside the cache. Integrate the occupied interval union; extending one global box with fixed steps can miss the remote cloud or erase native detail.

The current seams needing explicit work are:

- `cacheT/ambientA` in optics: Sun falls back outside the cache, while ambient currently returns clear accessibility there. Keep an admitted complete current-source fallback instead of fabricated clear.
- `rayAir` in view: outside-cloud clear-air source lacks cloud visibility; current conditional visibility affects fog only. Cloud shadow moving through clear air is not yet proved.
- Fog coefficients and Sun attenuation currently live in view code. Expose one shared optical path so receiver, mirror and environment apply them once; no completed fog overlay.
- Ground ambient is constant `skyMean()`. Add the required six-face32² environment capture and SH9 surface consumption. Faces share one source/Sun/air/fog generation; exclude the direct Sun disk and environment feedback.

P4a can use a labelled small cache/direct path and existing complete-ray references; it need not build all production cascades/reconstruction first. It must preserve the selected production route in the cost worksheet.

## Fixed initial-condition cases

Overlap control: roots separated16m, with a certified point positive in both at a healthy mature time. This proves coefficient addition against the MAX and proxy-sum negatives, not just disjoint clouds that hide the error.

Remote blocker: at Sun5°, azimuth.5rad and source120s, place B's certified BODY root on the Sun ray from receiver `(0,2,0)`. Under initial controls, A's BODY X displacement is417.408m. B's initial XZ is

`((1239.2−2)*cot(5°))*(cos(.5),sin(.5)) − (417.408,0) ≈ (11992.72,6779.68)m`.

Certify the actual positive chord before execution. B must remain outside the main view and the16km cache centred near A, while still shadowing the retained receiver. Change shared wind/shear after development, then activate a spatial front with main hidden. Keep actual camera/interval/terminal witnesses; a large box or a disabled visibility flag is not evidence that B was queried.

## Cost recorder and initial ledger

Current bench draw deep-clones full metadata/events and records traces every frame. Add an observer-off ordinary path, throttled HUD and on-demand immutable capture records. Bound debug history globally; multiplying large event rings by every virtual cell is not the production model.

Current `lightMs` spans receiver-begin through light-end across separate passes. Use individual within-pass timestamp pairs and a bounded asynchronous readback ring keyed to the immutable submitted sequence/source/quality/domains. Store completed, versioned receipts, not mutable `timings.latest`. Reject zero/nonpositive pairs; reordered/overlapping cross-pass queries leave whole-graph time null. Do not sum those spans or substitute queue-fence latency.

Current resolved media is1280×720. A matched640×360 accuracy/cost comparator is useful after source inspection, but does not implement the production far320×180/near640×360/independent reflection320×180 path.

| Account | Two-cell analytical bound, not measured throughput |
|---|---:|
| CPU control banks |1,886,592B|
| GPU prior/current controls |628,864B|
| GPU poses |1,536B|
| Shared template domains |64B|
| Shared native GPU inputs |16,187,392B, plus retained CPU inputs|
| Cell steps/s at1x /16x |8 /128|
| Worst full-grid weather-node updates/s |314,432 /5,030,912|
| Full control upload rate at1x /16x |2.515 /40.247MB/s|
| Pose uploads at60Hz |92,160B/s|

These exclude heap, indices, fenced products, observers and driver allocations. Existing light dispatch ceilings add up to2,064,384 source samples and receiver rays up to1,048,576 before views. Environment32²×6 adds6,144 rays plus lighting/filter work. Count actual occupied intervals/candidates/fetches; these arithmetic ceilings cannot predict milliseconds.

## Allocation decision before full skies

At512 cells, the prototype's dense banks alone imply482,967,552 CPU B (460.594MiB) and160,989,184 GPU B (153.531MiB). Shared native inputs already exceed the proposed8MiB static pool. These are representation warnings, not measurements of a running sky. The world design already distinguishes virtual events from resident optical state; the prototype does not implement that distinction. Two-cell success cannot promote this allocation topology. The retained17f Apple-adapter receipt reports requested-device `maxStorageBufferBindingSize=134217728B`; a single160989184B control binding would also exceed that observed limit. This is the current device configuration, not proof that every adapter or a deliberate higher-limit request has the same bound.

An exact follow-up is shared immutable zero F64/F32 fields and lazy/COW pages for genuinely nonzero exposure histories, with separate age exposure and prior/current/fenced readers. One fully allocated chart currently costs117,912CPU B across its banks. Preserve raw F64 truthiness, sub-F32 positives, slot generations and irreversible exposure. Measure nonzero-history counts and all-old-positive bursts: exposure persists after the front passes, so a thin instantaneous front band is not a memory bound. Pool exhaustion must report overload, never erase remote weather. Retain the explicit GPU/source residency and recoverable-cause contract; this brief does not select a new approximation or certify512-cell capacity.

## Construction and evidence order

First freeze source/fixture IDs, coefficient composition, domains, tolerances and ledger. Prove overlap addition, conservative membership, shared-clock causality, reset/slots and direct/index agreement. Then inspect fixed-camera native1x/accelerated coupling with main hidden: remote blocker, fog/air shadow, receiver, mirror and environment must move together. Retain omitted-B and stale-generation negatives. Finally measure observer-off steady/birth/dry/wind/cut cases, observer overhead and cold work; report CPU, uploads, stage GPU intervals, backlog and memory separately.

Reject missing blockers, globalMAX, proxy double-counting, stale light/environment, separate fog overlays, lost source intervals or silent capacity failure. P4a's outcome is quantified plausible route, structural rework, or missing measurements; it is not5ms/full-sky/M2 admission. No source/cook/browser/GPU/benchmark execution occurred in this audit.
