# Q2/Q3 coupled cloud proof

**Result:** shared lighting/shadow/reflection coupling is demonstrated in this bounded instrument; production admission is blocked by cost. [Review and motion](REVIEW.md) · [budget disposition](PERFORMANCE.md) · [independent final review](review/final-review.md).

Commissioned1 November2026. The [governing cloud design](../../design/modules/CLOUDS.md) and
[capabilities](../../design/modules/CLOUD-CAPABILITIES.md) govern this next bounded interaction.
The [Q1 source](../cloud-source-2026-11-01/REVIEW.md) remains pinned; this is not a new lifecycle bake.
No renderer exists in this Thrum checkout. This standalone instrument proves an interaction/model,
not public engine bindings or a bypass of I1/I2/I3a.

## Fixture and prediction

One128³ prepared cloud field drives visible volume, matte-receiver attenuation and a planar perfect-mirror view.
A single source age, analytic preview wind trajectory, Sun and two fog states latch before the graph runs.
Cloud, exponential air and overlapping bounded fog sum coefficients before a shared transport integration.
Every directional light path composes cloud/air/fog attenuation and analytic geometry visibility once.
The mirror trace starts at the interface; camera-to-interface media applies once in the main view.
This mirror is a reflection boundary fixture, not water simulation or a production ocean.

Sun-facing partial optical depth uses a128³ light-aligned prefix grid with guards and density mips0/1/2.
Primary extinction/shadows use ordinary density; cloud-only bounded higher orders alter only the source.
Six-axis external cloud ambient uses the declared authored horizon mask. Air/fog ambient remains zero.
Main720p media starts half-linear640x360; reflected view320x180. Depth/path-aware reconstruction retains
opaque/interface boundaries and a separate full-resolution fallback. No temporal history is admitted here.
One display stage owns fixed scene-linear exposure/tone/output.

## Rejection cases and evidence

- Compare actual GPU light/path queries with an independent fine CPU optical oracle, including an interior
  point, receiver below cloud, outside light projection, changed Sun direction and moved source.
- Slab/clear/absorber and overlapping-media fixtures reject double attenuation, separately lit overlays,
  wrong phase/sign, zero-channel division and source-off failures.
- Native images plus1x/16x/64x sampled sequences must show body, changing receiver shadows and reflected
  structure agreeing in source time/world position. Inspect cloud/geometry/horizon boundaries and a thin occluder.
- Sun off retains explicit sky ambient; all illumination/emission zero produces black radiance while the
  coefficient/path queries still retain extinction. Omit main clouds while retaining reflection as an explicit mask.
- Measure the assembled source/cache/main/reflection/resolve/display graph with loaded-code/source hashes,
  actual1280x720 output, per-stage and complete timing, cadence/tails, source progression and a resource byte ledger.
  Compare incremental targets0.5ms CPU/5ms browser GPU/3ms native GPU and full8/16ms browser CPU/GPU,
  proposed88MiB service and512MiB complete browser. M3 observations cannot certify M2/mobile.

Independent Sol High oracle/review commissions challenge numeric and experienced coupling. Runtime, source/content,
visual, measured local prototype and platform maturity remain distinct. Results will be recorded here after execution.

## Running result — transport and still-image checkpoint

The standalone instrument now runs at `http://localhost:8142/scripts/cloud_coupled_proof/` via
`python3 scripts/cloud_coupled_proof/serve.py`. Its source is pinned by the certified density SHA;
content-specific conservative bounds reject other sources rather than silently clipping them.
The shared decoder writes portable RGBA16F storage directly, with explicit RN-half conversion;
the default readback density and both average-density mips are bit-identical to the Q1 half values. The full fresh
source/mip/max/prefix/ambient build uses one compute pass before both views. No temporal reuse.

[Native rejection cases](images/still-contact.png) show main/reflected cloud, changed Sun shadows,
main-cloud omission with reflected/shadow consumers retained, foreground fog, upper geometry,
sky-only lighting and exactly black zero illumination. The [normal image](images/body.png) and
[full-resolution direct trace](images/full-resolution.png) differ by mean0.0236/255 display-channel
levels;0.0321% pixels differ by more than8 levels. This is a still/reconstruction comparison,
not a whole-motion perceptual or optical accuracy guarantee.

The independent oracle has13 passing tests. Actual GPU trace/query readbacks agree with its1m
solve using the actual caches within0.620% composed radiance on the bounded selected rays;
mirror near/far composition differs at most0.0753%. Illumination-off cases retain coefficients but
have zero radiance. The128³ interpolated light cache still has sampled primary T error0.02859,
including a45.1% relative incident-light error at one opaque interior query; it is an authored
production approximation, not a physically accurate point-light-depth qualification. Delivered
puff-ray source differences from direct fine Sun depth were below0.4% in the selected comparisons.
Finer ambient integration reduces sampled node error below1% within its required cloud query
domain. Ambient is intentionally unavailable/zero outside that conservative domain.

The specialization preserves all selected query radiance exactly and the complete source/mips;
full prefix arrays differ from the un-specialized build by up to0.00115 in T. Do not describe
floating cache accumulation as bit-identical. See the [independent reports](oracle/) and
[implementation challenge](review/follow-up.md). Actual histories, native/M2/mobile and engine
bindings remain unqualified.

The first complete timing sweeps rejected the initial packed-copy path (roughly80ms observed
queue completion). Direct storage, fused compute and correct missed-path guides substantially
reduce it, but the final complete GPU bracket is12.845ms median/18.809ms p95
on this M3 Max. It does not pass the desired5ms cloud allocation or whole-frame16ms tails.
Render beginning timestamps are invalid on this Metal path; raw receipts retain them for
reproduction, and complete brackets use compute-to-compute sentinels. The [final600-sample run](PERFORMANCE.md), [clean motion witnesses](REVIEW.md#motion-actually-inspected) and [rejection matrix](oracle/rejection-case-validation.md) are now retained; no promotion is claimed.

[Storage closeout and exact retained files](STORAGE.md).
