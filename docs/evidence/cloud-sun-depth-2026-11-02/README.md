# Cloud Sun-depth and coupled-light checkpoint

2 November2026. [Canonical profile](../../design/modules/CLOUD-SUN-PROFILE.md), owned by
[the complete cloud design](../../design/modules/CLOUDS.md). **Partial standalone proof; lateral point-transmittance
quality and the5ms cloud GPU allocation fail. Full SKY-G atmosphere/fog, surface environment, new source families,
full skies and target-device qualification remain open.** The earlier [usable proof](../cloud-cu-correction-2026-11-02/README.md)
stays available. The current local comparator is intentionally a vacuum/constant-sky diagnostic with a plane receiver
and independent planar mirror; its plane boundary is not a finished horizon.

## What changed and what was actually proved

The first K48 streaming minimum-removal compressor fails a gradual tau witness. The independently challenged
[one-pass tau tube](review-deep-tube.md) fixes that representation defect without changing source coefficients.
[CPU reference evidence](deep-light-reference-note.md), [GPU implementation review](review-deep-implementation.md),
and the [actual GPU validation note](deep-gpu-validation-note.md) separate its real-arithmetic bound, numerical reserve,
source quadrature, lateral filtering and experienced appearance. The proposed2e-6 reserve remains an unqualified
backend allowance. A many-thin-step capacity failure is explicitly unavailable, never a dropped blocker.

Actual64²/66²physical/K48/512sample GPU curves fit all4356 columns for the held source. The cropped128 capture has
at most31knots;16selected full input streams have maximum transformed PL compression errors.004990709/.004973868,
below.005. Manual GPU-vs-CPU four-tap arithmetic differs by at most1.05e-6/.68e-6. These are finite witnesses,
not uniform source/shape/lighting qualification. [Raw buffers and exact input streams](probes/deep-probe-1793588276463.json)
and [independent results](deep-validation-cropped128-30.json) are retained.

**The lateral point-query gate fails.** Fine four-column visibility vs fine point visibility reaches.121507/.091745
against.008; actual GPU-vs-fine-point reaches.121184/.093170 against.02. At the same pinned material point the
old full-field total error.308117/.120521 falls to.115078/.044694 after tightening the allocation, still failing.
Zero fallback counters certify no fallback was invoked, not that this footprint is accurate at a point. A production
filtered-visibility closure and an exact point-tau service must not be conflated or retroactively relabel this as a pass.

## Source allocation correction

The [all-frame support scan](prepared-support-note.md) validates all154unique encoded/decoded chunks and162knots.
The adopted correction crops the **field allocation**, not just the private Sun projection. Canonical master coordinates,
world scale, camera targets and source sampling stay fixed. Bounds align to4logical cells so all3mip phases remain.
For128 the logical allocation changes128³ to64×48×76; storage with guards/mips falls5,316,544B to653,376B.
For256 it becomes128×88×156. A producer-only128 original-centre shortcut avoids redundant trilinear R8 decoding;
arbitrary canonical queries remain trilinear. Neither operation authors new cloud shape or lifecycle.

Actual GPU receipts at [128growth](prepared-field-1793586435485.json), [128maturity](prepared-field-1793586430251.json),
[128breakup](prepared-field-1793586440202.json) and [256maturity](prepared-field-1793586510291.json) show every logical
mip codeword equal in the retained subregion, exterior and padding zero, and no nonfinite values. Full generic vs128
specialized also matches. Validation/OOM/device-loss scopes are part of admission, so a failed all-zero dispatch cannot
become a false pass. [Review](review-prepared-crop.md) and [closure](closure-prepared-crop.md) retain both found/fixed
issues, including the independent fit oracle's old512m bounds. [Actual fit probe](fit-probe-1793588376474.json) passes
LUT/storage arithmetic; sampled bank interpolation and body quality remain separate unqualified approximations.
[Repeated R0 GPU packing proof](packing-current.json) passes1273samples after the shared physical-pitch change.

This does **not** imply bit-identical final images: tighter ray intervals, ambient and light grids change quadrature
phase. The source remains the old detailed static master with authored lifecycle and historical.125/m calibration;
it is not admitted to the proposed SKY-G.1/m profile and has no new variants or renewing mature structure.

## Native images and motion

The root and [independent reviewer](review-coupled-light-checkpoint.md) inspected actual1280×720images and sampled
native sequence PNGs. All selected pairs hold source age, camera, Sun, fit and loaded build fixed within the pair.
Only the direct-reference-patch switch changes. Those slow32²patches integrate512source samples per queried point;
they are bounded reference regions, not a full-frame reference or benchmark.

| Witness | Actual images | Result |
| --- | --- | --- |
| Cropped localSun30 | [cached](images/cu-1793588264329.png), [direct patches](images/cu-1793588285170.png) | Main/mirror patch max2RGB8codes;1163changedpixels, none>8. |
| Cropped backlitSun5 | [cached](images/cu-1793588295707.png), [direct patch](images/cu-1793588301387.png) | Remaining local gradient error:794changedpixels,max21codes,38pixels>8. |
| Earlier full-field backlitSun5 | [cached](images/cu-1793583985129.png), [direct patch](images/cu-1793583992042.png) |953changed,max26codes,167pixels>8. Cross-build improvement alone does not isolate crop/phase effects. |
| Main hidden | [actual](images/cu-1793588503926.png) | Selected shadow and mirror rectangles exactly unchanged while visible main cloud disappears. |
| Illumination off | [actual](images/cu-1793588508085.png) | All image RGB8exactly zero. |
| HighSun75 | [actual](images/cu-1793588513817.png) | Brighter cap, body/mirror response and shorter/nearer receiver shadow are coherent. |

Four actual-canvas videos and native snapshots are retained: [normal growth](motion/motion-1793588403807-growth.webm)
40–60s,799frames/20.046s; [normal breakup](motion/motion-1793588435351-breakup.webm)200–220s,989frames/20.019s;
[full16×](motion/motion-1793588380273-full16.webm)654frames/15.0095s; [full64×](motion/motion-1793588475511-full64.webm)
163frames/3.7798s. Counters record no view-cap/fallback/unsupported failure. Cloud, shadow and reflection strengthen,
split and fade together in inspected snapshots; terminal states are clear. Faint prefigured birth, static maturity and
rounded breakup remnants persist. Only sampled frames were visually reviewed; do not claim continuous WebM inspection
or certify the absence of between-frame popping. Recorder/readback overhead is included and is not quiet timing.

## Cost and identity

Both quiet sequential runs use native1280×720, one frame in flight, retained prepared128, fitted liquid, deepSun30,
local main+mirror+receiver,30warmup+300measured frames, actual first compute through a compute dependency on the
rendered canvas, no present latency. They rebuild source/light every frame as time changes, even during the source's
static mature interval. They exercise fresh-work cost, not a cache-reuse workload. There were no concurrent GPU probes
or recording jobs; temperature/OS scheduling and exact source-time samples differ, so this is a finite paired comparison.

| Workload | GPU p50/p95/p99/max,ms | Completed callback cadence p50/p95/p99/max,ms |
| --- | --- | --- |
| Full generic field |19.005/23.003/26.018/27.001 |31.0/38.91/41.002/44.0 |
| Cropped + specialized producer |12.976/17.075/19.988/21.037 |24.3/32.8/35.018/37.8 |

**Both fail5ms.** This comparison changes crop and producer together and does not attribute the individual runtime
saving. CPU encode is only a partial scope (cropped p99.401ms); whole-frame CPU/16ms engine and512MiB actual process/
GPU residency remain unqualified. The allocated deep product is1,812,096B; the old unselected prefix resource is still
allocated. No memory promotion follows from a declared-resource subtotal. Component timestamps are the next cost gate.

[Compressed raw receipt index](compressed-receipts.json) pins original and deterministic-gzip hashes/bytes. Decompression
restores the exact original JSON, including every raw timing/sample row. The [loaded crop/full experiment identity](crop-profile/identity.json)
is82909fdc44fa6191112b602bf079bc155d81d14df159a4d0ccdfd1ecd4a6c56d; exact loaded executable texts are retained there.
The older probe had a host/component identity property collision; its raw record is preserved. The [same-build full-field
replay](probes/deep-probe-1793588536488.json) now retains both identities and reproduces its node,metadata,field and
path binary hashes exactly. Historical [loaded sources](loaded-64-profile/operator-identity.json) remain separate.

Next: choose the production visibility footprint explicitly, retain the failing point-query evidence, use component
measurements to select the query representation, then prove the complete nonrecursive air/fog/ambient/surface graph.
No isolated family or whole-engine maturity is promoted by this checkpoint.

## Component timestamp qualification failure

A subsequent instrumented builda8f7c1874b5c9554468816f0ef6070dabc9c26a51b35d0e4d866757ffe3d64e2 separates cache
compute passes and timestamps source/mips, fit, Sun, ambient and views. Its exact coordinator and fullidentity are
retained in[component-profile](component-profile/identity.json). The [330-frame raw run](cost/cost-1793589037436-fresh-source-1x.json.gz)
retains all raw timestamps. **None of300fresh measured rows has an admitted additive component sum**: actual render-pass
begin timestamps precede the first compute begin, and intervals overlap. The decoder detects nonoverlap/containment
failures and returns null sums/gaps. Pass durations can include backend scheduling/waits; they are not isolated kernel
costs and cannot prove a dominant shader. Whole first-compute-to-rendered-consumer bracket remains separate
(p50/p95/p99/max16.056/21.067/23.003/26.018ms). Split passes perturb scheduling, so this does not replace the earlier
matched comparison. The [cached-frame receipt](component-profile/cu-1793589021657.json) validates explicit reused stages
with null inactive timestamps. True attribution requires a separately labelled dependency-serialized diagnostic;
source/render representation selection must not rely on a fabricated component sum.
