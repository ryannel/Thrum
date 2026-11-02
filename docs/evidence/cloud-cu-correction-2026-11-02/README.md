# Cu source/lighting correction design

2 November2026. The [R0/first image](../cloud-sky-r0-2026-11-01/README.md) proves finite scalar arithmetic but rejects
the coarse source's actual appearance. The [complete design](../../design/modules/CLOUDS.md) remains the authority.

The source author's [proposal](proposed-cu-source-correction.md) supplies a surface-attached scale hierarchy, physical
metre ramp, explicit local drying and bounded current-pose brick index. A different reader's
[challenge](review-cu-source-correction.md) found two missing choices: the spatial front/gradient and the actual field
build/index. The [closure](closure-cu-source-correction.md) verifies those design corrections; it does not certify
numeric0.01m padding, native geometry, motion or cost. The selected current profile is
[CLOUD-CU-SOURCE](../../design/modules/CLOUD-CU-SOURCE.md). Its first scope is spherical primitives and rigid shared
wind; this does not qualify general sheared world-distance fields.

The light author's [proposal](proposed-cloud-light-correction.md), a different reader's
[challenge](review-cloud-light-correction.md) and [closure](closure-cloud-light-correction.md) select the
[source-only effective-light profile](../../design/modules/CLOUD-LIGHT-PROFILE.md). The review corrected world versus
reference-column semantics under shear, explicit liquid/ice dispatch, portable R32float lookup and evidence scope.
Generated scatter events and escaped power are different: a strict generated-event cap can underlight high-albedo
clouds. The replacement preserves exact primary and adds one isotropic bank fitted to finite-slab escaped flux.
Its reflectance target is authored; arbitrary finite-cloud energy/appearance remains unproven. An empirical lookup
can support a labelled native comparison without pretending to have a uniform numerical energy certificate.

Shared ordinary extinction, committed source time, receiver shadows, independent reflections and display remain
fixed. Source-only shape and held-density light comparisons precede combined acceptance. No new family, weather
sequence, complete sky or performance profile is accepted by this design checkpoint.

## Actual implementation checkpoint

**Result: a useful bounded liquid-lighting correction; source-family and complete coupled-profile acceptance remain open.**
The surface hierarchy was implemented and rendered, then rejected. The retained Q1 detailed source now runs through the
packed field and new fit as a held geometry comparator. It is one previously authored finite clip, with faint birth,
static mature structure and rounded late remnants—not a newly accepted family or replacement lifecycle.

- [Native held passive](images/passive-front.png), [exact primary](images/primary-front.png) and [fitted](images/fit-front.png)
  use the same geometry/time/camera/Sun/exposure. The fit improves neutral body lighting and preserves darker structure.
- The [held local passive](images/passive-coupled.png) and [fitted coupling](images/fit-coupled-final.png) pair shows the
  cloud, receiver and independent mirror together. Selected receiver-shadow pixels are bit-identical while source
  radiance changes. This is a deliberately low-altitude diagnostic, not normal cloud altitude or the SKY-G atmosphere.
- [Main hidden](images/main-hidden.png) retains the reflected cloud and receiver shadow. [All illumination off](images/all-off.png)
  is RGB8 zero everywhere while the [extinction diagnostic](images/all-off-extinction.png) retains the cloud.
- [Low Sun](images/fit-low-sun.png) and [high Sun](images/fit-high-sun.png) retain shading structure and changed direct shadows.
  The fixture keeps Sun RGB and external sky constant; it does not prove atmospheric sunset colour or clouded diffuse ground light.
- [Final hierarchy front](images/hierarchy-final-front.png), [side](images/hierarchy-side.png) and [top](images/hierarchy-top.png)
  reject buttons, bare broad flanks and mechanical joins. The final front/probe uses the corrected header-admission build;
  earlier images are labelled by their own JSON builds. Geometry was not changed by the admission fix.

The independent [source appearance review](review/review-hierarchy-appearance.md),
[implementation closure](review/closure-hierarchy-implementation.md), [fit implementation review](review/review-fit-implementation.md)
and [native/motion review](review/review-fit-appearance-motion.md) retain actual findings and limits. Corrected defects:
failed creation leaked partial GPU resources (including nested fields); malformed same-time index headers could silently
lose contributors; rejected candidate replacement could publish its clock before its resources. Five forced failed-creation
CPU cases and the source/index negative suite now pass. Actual device loss/recovery and full provider retirement remain unrun.

## Numerical checks and approximations

[Summary](summary.json) is rebuilt by `scripts/cloud_sky_proof/analyse_checkpoint.py` from retained inputs.
The [final hierarchy probe](hierarchy-final-probe.json) has388 actual CPU/WGSL/index/selected field-store checks,
max sigma error2.2353e-8/m. The [prepared-source probe](prepared-source-probe.json) checks the pinned decoder in the new
field path; it does not replace Q1's existing full decode/repack evidence. Source calibration is retained, including
Q1's.125/m maximum; this is not automatic admission to the new SKY-G.1/m profile.

The [30-degree](fit-sun30-probe.json), [5-degree](fit-sun5-probe.json) and [75-degree](fit-sun75-probe.json) fit receipts
include104 actual GPU LUT queries each. Max GPU/double-CPU lookup error is2.261e-7, below the declared3e-5 arithmetic
threshold; actual16-column A storage/generation checks also pass. At30 degrees, the sampled canonical-column tau error
is.00057751 and A error1.599e-6. This is bounded sample evidence, not a uniform ray/column certificate.

The empirically cooked256×256 table is pinned in `scripts/cloud_sky_proof/fixtures/scatter-liquid/`; the
[cooker review](review/review-scatter-cooker.md) reproduces its finite fit checks. Its worst sampled off-grid fit error
.02095824 misses the.02 heuristic and remains explicit. GPU lookup parity does not remove that fit error.
Interpolating cached A differs from evaluating A of interpolated body thickness: sampled30-degree A-map error.025743
is separately retained. Its radiance impact, thin edges, fragments-over-parent columns, tower/overcast/ice/layer
witnesses and complete quality admission remain open. No liquid table is promoted for ice.

## Actual sampled motion

All four records are actual native1280×720 canvas videos plus nine sampled PNGs and per-frame committed times:

| Witness | Retained movie/receipt | Frames | Source / observed completion |
| --- | --- | ---: | --- |
| Normal growth | [video](motion/motion-1793579723098-growth.webm), [receipt](motion/motion-1793579723098-growth.json) |829|40→60s /20.0572s|
| Normal breakup | [video](motion/motion-1793579800488-breakup.webm), [receipt](motion/motion-1793579800488-breakup.json) |780|200→220s /20.0428s|
| Complete16× | [video](motion/motion-1793579885974-full16.webm), [receipt](motion/motion-1793579885974-full16.json) |631|0→240s /15.0491s|
| Complete64× | [video](motion/motion-1793579938289-full64.webm), [receipt](motion/motion-1793579938289-full64.json) |158|0→240s /3.7858s|

Root inspected native growth/breakup0/4/8, full16 2/4/7 and full64 2/7; the independent review records its wider
sample selection. These are sampled-motion judgements, not a claim to have continuously watched the WebMs. Growth
and support breakup have matching receiver/reflected evolution; no invalid path/step-cap counters occurred. The retained
clip's faint prefigured birth and static mature arrangement still fail the new defining source ambitions. Recorder/readback
cost is included in these records; they are not the quiet timing dataset. No meteorological time-lapse was watched for
new lifecycle acceptance, so that reference gate remains open.

## Work, memory and budgets

[330-frame cost receipt](cost-330-frames.json): first30 warmup, then300 actual fresh-source/fresh-light frames, no recorder
or PNGs, one frame in flight and no invalid paths. Source time follows wall time. GPU brackets now begin on the actual
first compute pass and end on a compute pass reading the actual rendered canvas; raw timestamps are retained. This fixes
the earlier empty-marker interval's missing render dependency. It is GPU elapsed scope, not present/display latency.
Queue/cadence include query and error-counter readbacks. CPU encode excludes other host/browser work.
The original records' top-level `scene` was captured before the first requested render (cost shows the preceding paused240s scene). Use the explicit start/rate and each actual sample's full state/time; the analyser does so. Future captures now latch the initial scene after rendering it. The old cost filename suffix `full64` came from the unused motion selector; this cost run is rate1, as its metadata/rows record.

| Scope | p50 | p95 | p99 | max |
| --- | ---: | ---: | ---: | ---: |
| GPU bracket ms |11.96032|15.07328|17.89198|19.00544|
| Queue completion/readbacks ms |19.7|28.105|30.4|32.1|
| CPU encode ms |.1|.4|.4|.5|
| Completed-frame cadence ms |24.1|32.9|34.504|41.0|

**FAIL browser cloud5ms; native3ms/M2/mobile unrun.** No complete engine frame was measured; this fixture's p99 already
exceeds the browser16ms whole-frame target. The previous coupled instrument used different caches, air/fog and reconstruction;
these results are not a matched performance improvement claim.

Known logical GPU subtotal is18,194,516B (~17.35MiB), including one visible canvas texture and288B of field/source uniforms
caught in independent audit; a transient16B error readback is additional. Actual encoded source is6,491,440B GPU plus its
CPU recovery copy; CPU manifest73,104B and fit recovery292,941B precede parsed-object/compiler overhead. Swapchain multiplicity,
GPU driver allocations, staging, process residency and the complete512MiB browser budget are unmeasured. Source replacement
has old+new overlap. Lower logical bytes do not qualify full-sky memory or equivalent-experience performance.

The [independent cost/probe audit](review/review-correction-cost.md) recomputed the timing, corrected known bytes, all three finite LUT/body/A summaries and the selected build archive hashes. It did not run GPU work or extend full-engine/source-factor qualification.

## Identity, recovery and next work

The comparison/motion/cost build is `e0425a51c36f46855c9dedfd79465915805c4ad910c0e556679de4cb7c9db748`.
Its [identity](loaded-comparison-build/identity.json) and exact loaded JS/WGSL/HTML text are retained. Numerical probes,
final hierarchy and additional held coupling use `e18bdc2707002525fa7292ac8f59848fc00525005b10ec40809b487007a71355`;
[identity/deltas](loaded-probe-build/identity.json) extend that retained base. Each capture's JSON is authoritative for its
actual build, source/calibration and camera. The pinned Q1 master/rights/source chain remain in the prior source package;
the new fit/cooker/helper source and fixtures are tracked. UI-only closeout edits are not silently substituted into old results.

Next consequential coupling work is the canonical all-layer ordinary light/deep-product path, finite external air/fog
ambient and final clouded surface environment, with independent point/ray/source-factor and mirror composition comparisons.
The current64cube point-tau/8cube five-axis ambient/constant receiver diffuse fixture cannot close that complete gate.
Keep this usable comparison paused while doing that work. In parallel dependency terms, prepared new family/variant masters
and mature structural renewal are the source gate before any failed source enters combinations/weather/full skies.
All five families/weather/full-sky capabilities retain their existing fail/partial/unproven dispositions; no required
behaviour was removed to meet cost.
