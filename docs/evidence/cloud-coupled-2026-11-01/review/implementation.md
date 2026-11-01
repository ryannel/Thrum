# Independent Q2/Q3 implementation audit

2026-11-01. Read complete `common.wgsl`, `caches.wgsl`, `render.wgsl`, `viewer.js`, `source.js`, and relevant independent oracle/query code. Read-only source audit: no GPU benchmark, code edit, commit, loaded build, image/motion inspection or device qualification by this reviewer. Source was changing during the audit; hashes below identify the second source read, and findings remain open until independently checked against the resolution.

```
common.wgsl 2d1bc0b02f7366177c178bac023429a5c709b58749b1658736f60cb750d248d5
caches.wgsl 120bcece090bb1e65fad2e8ee819d61b5badfe6b063925cf55d47f4fb9ee3f57
render.wgsl 0504c1096abfab22476d753a1313c706fbfa5e399e908709f1f3014a0dcde368
viewer.js e09d041ebfe976425b899aa03d6cdf644c044f98fd9a7932c0a338e772f18df2
source.js 2f5a8c029c44c028d2b50d271a0392ec526477baa5dd3f942b924ffec50728e2
```

## Findings requiring resolution/evidence

### F1 — reflected reconstruction lacks cloud interval validity (medium)

`render.wgsl:13–18`, especially coarse-tap selection at line16, validates opaque kind/distance and total fog/air optical path but does not validate the reflected cloud entry/exit interval at all. The coarse reflected view is320x180 for1280x720 output. Two sky-terminal rays have the same kind0, distance4000 and nearly identical air/fog even when only one enters cloud support. Thus all taps can be accepted and bypass the full-resolution fallback despite the path mismatch. Main media already carries/checks cloud entry/exit, so the reflected consumer is materially weaker.

Resolve by carrying reflected cloud-relative interval/support validity into tap selection and rejecting/falling back where it differs outside the declared tolerance. This remains a spatial approximation: intervals alone do not certify interior fine-density reconstruction. Retain native reflected silhouette/thin-feature crops or a fine/full-resolution comparison to quantify that residual rather than describing the guide as an exact error bound.

### F2 — reflected full-resolution fallback ignores failed transport (medium)

`render.wgsl:18` calls `transport` then returns its partial S/T without inspecting `value.valid`. At the300-step guard, transport returns `valid=false` with the prefix accumulated so far. Coarse reflection and main full-resolution fallback explicitly show failure, but this branch can display plausible truncated radiance. This can turn workload overflow into a silent visual shortcut precisely when coarse reconstruction rejects many pixels.

Resolve by propagating invalid/failure output consistently; an error counter alone cannot make a plausible truncated image valid. Current bounded scene may never reach the cap, but the branch must not accept failure.

### F3 — benchmark error/fallback counters cover only final graph (medium evidence defect)

`viewer.js:encodeGraph` clears errorBuffer each graph; `benchmark` reads it only after all150 graphs and reports `last_frame_counters`. This label is honest, but those counters cannot support an all-run statement that no ray caps occurred, nor provide fallback workload distribution or worst fallback frame. Earlier expensive/error frames disappear.

Resolve by copy/read/accumulate per-frame counters outside measured passes, or a separate persistent accumulator plus per-frame fallback maxima. Avoid adding readback to GPU timestamp intervals. Retain the distribution or maxima with the graph report. Do not extrapolate final-frame fallback cost to the sweep.

### F4 — actual consumer readbacks are incomplete (high evidence gap, not proven shader bug)

Initial export contained density/prefix/ambient only. During this audit the author added main S/T/path guide/HDR exports, observed at second read in `viewer.js:84`; this addresses the main-ray part. Still absent in that inspected revision: coarse reflected output/guide, selected arbitrary GPU transport paths, and actual GPU surface/direct-light diagnostic terms. Those omissions prevent a CPU optical oracle from independently checking the mirror far leg, foreground composition, surface shadow, fog-between-light-and-cloud effect, and sphere occlusion of in-cloud scattering on their actual GPU consumers.

Resolve with selected GPU query/readback records plus terminal/path/source identity, or equivalent direct comparison against exported actual consumer buffers. A successful CPU reference's own tests do not verify an unqueried GPU consumer. Surface/volume/reflection probes should include the upper-occluder on/off fixture and the all-illumination-off case. Source reconstruction and prefix readbacks alone qualify products, not their downstream use.

## Author-discovered critical defect, not claimed as independent finding

The author reported finding and correcting `sceneHit` accepting the missed-sphere sentinel `(1,-1)` as a positive near hit. The first complete source read contained this defect; the second `common.wgsl` hash above requires `a.y>=a.x` and `b.y>=b.x` and therefore does correct the reported miss case. This is an important loaded-build provenance boundary: captures before that fix cannot establish cloud appearance/opaque depth validity and must not be silently relabelled. This source inspection verifies the added condition, not the corrected running image.

## Mechanisms checked and found coherent in this bounded recipe

- One current density reconstruction/mip chain is scheduled before maximum/prefix/ambient and both camera consumers. All consumers use one latched scene uniform/source age; no per-view clock exists.
- Density coefficients and actual 128^3/R8 prepared source remain shared. The hardcoded tight cloud traversal box is conservative for this pinned master: nonzero index extrema XYZ[33..94,15..56,28..101] give trilinear support approximately[130..382,58..230,110..410]m, inside traversal[128..384,48..240,96..416]. Authored threshold erosion only removes density. This is content-specific evidence, not a generic source bound; pin or derive it when changing source.
- Maximum reduction includes the trilinear halo (`4*cell+[-1..4]`), so a declared-zero current coarse cell can skip cloud work while air/fog still integrates. Empty-cell traversal advances to a cell boundary; current medium is still evaluated.
- Air + fogA + fogB + cloud extinction are added once; source terms use common light attenuated by both fog volumes, air, geometry and cloud. The primary cloud term appears once and higher terms use only cloud scaled depths with the common noncloud attenuation retained. Surface direct shadows use ordinary mip0 cloud depth.
- HG phase sign is correct for sample-to-light and camera-to-sample ray: `dot(ray,toLight)`. Reflection recomputes phase from reflected ray. Ambient uses the declared five unblocked axes divided by six, with -Y horizon zero; it is not arbitrary geometry/environment multiple scattering.
- Prefix is a source-facing cumulative field, not a receiver-only projected texture. Its midpoint samples, leading zero and trailing total are consistent with the texture coordinate mapping. Boundaries have a16m projected margin. Numerical/cache interpolation error still needs Q2 readback comparison; algebra alone supplies no accuracy gate.
- The mirror far leg starts at the actual ground interface and traces the world scene independently. `finalColor` composes main `S + T*reflectedRadiance` once. Main geometry clips foreground before mirror and reflected geometry clips the far leg. The main off switch leaves reflected cloud and light attenuation intact by explicit independent-consumer policy.
- Main spatial resolution validates opaque/cloud-support/fog/air guides, normalizes suitable taps and requests a separate full-resolution fallback when none qualifies. No history is allocated or claimed. Guide tolerances are heuristic suitability thresholds, not demonstrated radiance error bounds; actual sphere/horizon/foreground-fog image comparisons remain necessary.
- The current Sun choices both have positive altitude, so omitted ground intersection in direct-source visibility is safe for the admitted fixture. Arbitrary below-horizon directions would require ground/horizon validity; this audit does not qualify them. No Moon path is implemented or qualified.

## Cost and qualification risks

No measured cost conclusion. The implementation has the intended half-linear main and quarter-linear reflected view, then an actual1280x720 resolve/fallback/display graph. It is therefore stronger cost evidence than a source kernel alone, but not Thrum renderer integration or complete target qualification.

The source field, mips, conservative maximum, prefix and ambient rebuild every graph even at held age. This is valid conservative measured work; it does not demonstrate cache-key reuse, source-off retirement or no-consumer zero cost. Maximum construction alone performs32^3*216 (~7.08M) texture loads. Ambient performs32^3*5*16 (~2.62M) cloud samples plus air/fog evaluations. Prefix performs64^3 times its admitted light-cell substeps and three filtered mip reads, before230400 main media and57600 reflected pixels, with per-step incident path sphere/fog/air queries. Full-resolution fallback can sharply increase ray work; measure its actual counts/tails.

GPU timestamps explicitly exclude source copies/mips and interpass overhead; summing them is not complete GPU frame time. Queue-completion wall time includes driver/synchronization/observer cost and cannot be relabelled GPU execution. The serialized wait-per-graph sweep is useful assembled-workload evidence, not ordinary display cadence; ordinary motion records still need source-time and callback cadence verification. It is not a sustained thermal/device-loss/startup study. The logical resource ledger excludes process/driver/swapchain overhead, so it cannot certify512MiB browser peak or complete88MiB cloud service. Unavailable native/mobile/M2 evidence remains open.

## Disposition

The bounded joint transport/camera/reflection assembly is structurally coherent. Resolve F1/F2, retain proper whole-run counter evidence for F3, and supply actual consumer readbacks for F4 before claiming Q2/Q3 coupled correctness. Appearance, ordinary/accelerated motion, loaded build identity and integrated device budget remain independently unreviewed here. This audit does not confer user acceptance.

## Follow-up: first graph attribution challenge

The author requested a source-only investigation after observing an approximately81.5ms queue median and render-pass timestamps that appeared cumulative (first reflected pass roughly57.6ms, later main/resolve/fallback/display roughly59–60.6ms). No benchmark was run by this reviewer and these reported numbers are not independently certified.

The reflection implementation has no recursion, nested shadow ray march or reflection-only unbounded loop. It calls the same bounded transport as main; incident Sun visibility is analytic sphere/fog/air plus cached cloud tau. Reflected sky rays intentionally march the full4km (roughly63 air steps at64m), with8m fog and4m occupied-cloud intervals. This is substantial work but does not, by inspection, explain a first reflected pass dominating a four-times-larger main target by tens of milliseconds. Reflection also traces mirror-plane pixels without checking whether main opaque geometry hides that plane; that is avoidable work, but modest for the currently placed spheres.

Before optimizing reflection based on those numbers, isolate/omit render passes in otherwise matched graphs and verify whole-graph timing changes. It is the first render pass: backend timestamp placement, deferred attachment/render work or shared render-encoder boundaries are credible attribution questions. Summing apparently cumulative stage numbers would overstate GPU cost. Do not remove required reflected air/fog transport simply because the first-pass timestamp is large. An optimization of pure-air intervals must retain illumination variation caused by cloud/fog/geometry on incident-light paths; camera-ray disjointness alone does not make analytic unshadowed air integration equivalent.
