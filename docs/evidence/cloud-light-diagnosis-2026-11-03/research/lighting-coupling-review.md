# Covered-Cb lighting/source coupling review

3 November 2026. Bounded analytical and CPU study. **No source-art, lifecycle, GPU precision, timing or full-sky acceptance. No new cache implementation.** The next decision is the unchanged-density low/high-cache image comparison under explicitly aligned scattering profiles and isolated cloud terms.

## Exact input and reproduction

- Frozen source: `cb-covered-256.density.r16f`, 33,554,432 bytes, SHA256 `377e8f9def4ff30ac38c02266b8666ae5c592d59f9a36820c62731fb52d89eef` (377e8f9d is the scalar hash prefix, not the browser-build ID).
- Observed original browser build: `f06bd77e72c5d79d`; actual384-ray receipt: `tmp/cloud-cb-covered-proof/captures/master-cb-covered-256-oblique-mode0-f06bd77e72c5d79d-e14775f5-2.json`. Its96-ray sibling has suffix1.
- Source is one static liquid optical proxy, R16F little-endian, X fastest / NumPy[Z,Y,X], no additional transform. Bounds `[-6000,0,-6000]..[6000,14000,6000]` metres; native pitch `[46.875,54.6875,46.875]` metres. Extinction uses kappa=.0045/m (the unused ice calibration is.0018/m); no source/noise remapping, density gain or lighting bake.
- Actual held camera: yaw.3rad, pitch−.05rad, distance25,000m, target[0,6500,0], native1280×720. Sun elevation30deg, azimuth−.7rad; unit vector is reconstructed in the script. The numerical sample is NOT weighted by this view; camera affects the separate phase calculation only.
- Source preparation/metrics inspected: `tmp/cloud-cb-covered-anatomy/cook.py`, `cb-covered-256.json`, `coverage.json`. The field has220m smallest displacement element, approximately200.16m narrowest cap-fibre FWHM and330m final distance-to-density transition. Its2,137lobe coverage result checks8,133points on the original scaffold, not the final outer-surface normals. Full footprint coverage does not prove convincing cauliflower geometry.

Run from the repository root:

```sh
/opt/homebrew/bin/python3.14 -B tmp/cloud-core-proof/lighting-coupling-reproduce.py --output /tmp/lighting-coupling-reproduced.json
```

`--density PATH` accepts a relocated exact scalar file and rejects different bytes. Original runtime: Python3.14.10, NumPy2.4.6, SciPy1.17.1. The retained `lighting-coupling-results.json` records versions, exact native point indices, all per-point transmittances, summaries, phase profiles and grid arithmetic. Script and results are owned scratch, not canonical evidence or a new renderer.

Point selection is exactly: `np.where((q>=.015)&(q<=.12))` on decoded native float32 density, C-order; `np.random.default_rng(731).choice(eligibleCount,2048,replace=False)`; use selected texel centres. There is no pixel/visibility/phase weighting. Each ray uses the same finite source-box intersection and uniform midpoint quadrature. Source sampling is SciPy order1 / prefilterFalse float32 interpolation; source collar and outside are zero. This is dense-equivalent CPU sampling, not a model of all WGSL rounding behavior.

The emulated cache evaluates32-step point-to-boundary Sun rays at32×24×24cell centres, computes exp(−tau), rounds to half and trilinearly samples that T field. At each test point the local segment uses two100m cells at50/150m, with cached tail queried at the200m endpoint. Outside-cache fallback uses direct32-step rays. Reference whole/tail rays use512steps;256steps provide the sampled convergence comparison.512steps is not exact integration and this does not test ambient/view radiance.

## Numerical result

All values below are absolute scalar Sun-transmittance errors on the2,048selected points:

| Comparison | Mean | p95 | Maximum |
| --- | ---: | ---: | ---: |
| Direct32-step tail vs512-step tail | .0024330732 | .0081934113 | .0308473525 |
| Current interpolated cache vs512-step tail | .0197005211 | .0582594270 | .1834873967 |
| Current2×100m local + cached tail vs512-step whole ray | .0184897574 | .0552195280 | .1804030075 |
| Direct256-step tail vs512-step tail | .0000366092 | .0001228716 | .0004624944 |

This sample indicates spatial cache loss is materially larger than32-step tail quadrature error. It does not establish how much of the beauty failure is lighting, quantify every source point, prove portable GPU precision, or accept the geometry. Positive and negative errors can occur; do not describe every sample as a light leak. Interpolating T also differs from exponentiating interpolated tau (Jensen convexity); changing that filter is a separate approximation.

## Actual shader/profile mismatch

The f06receipt pins these unchanged files:

| File | SHA256 |
| --- | --- |
| src/scene.js |22520144e1ced1916aa77bfa54370332aa6799e219a69c79d03a760782795629|
| shaders/light.wgsl |65419f0ecb026794a43659fdc269ce7454fe8e9ff608b2e64d9ea6e2bf389c05|
| shaders/optics.wgsl |ebc27137ec69bfc9e5dc3f674c9f71d47ddd65e4d19be0614ef3410cb5fbd970|
| shaders/view.wgsl |248eb6b09861c997bf04ecc871c892d057358bbbe906bba678661f09bf92f406|

The selected equations in `docs/design/modules/CLOUD-PRODUCTION-BASELINE.md` (Cloud source / Voxel lighting) specify extra-octave amplitude a=.5, optical-depth factor b=.5 and phase-eccentricity factor c=.5. The actual held shader uniforms are **legacy a=1,b=.25,c=0**: Scene writes sun.w=1, domain.y=.25, domain.z=0; view uses these respectively for the extra contribution, T attenuation and HG eccentricity. This discrepancy must be exposed as named profiles, not attributed to a density change.

At the held focus ray, mu=.36181429187, liquid first-scattering phase=.03281619201sr^−1, while the legacy isotropic extra term is.07957747155sr^−1: ratio2.42494533 at zero optical depth. Its ratio to first scattering then grows by exp(.75tau). The selected profile has half amplitude, half eccentricities and exp(−.5tau); its exact zero-depth ratio is recorded separately by the script. These are source approximations, not energy-exact scattering orders. The fixed ambient20percent floor remains another fill contribution.

Old viewmode2 removes ambient but retains BOTH the first and extra scattering terms; it is not a first-scattering-only diagnostic. Unchanged-source comparisons should expose first-only, extra-only, ambient-only and combined terms, and record profile/cache/sample settings. No parameter is being selected or visually accepted by this study. Root has commissioned explicit legacy/selected profiles and those term diagnostics before judging low/high-cache images.

## Spatial and temporal mismatch

The held light texture has18,432nodes at375×583.333×500m pitch. Its cell diagonal is about855m, larger than the220m/200m source cues and330m boundary transition. Fixed32Sun and20ambient quadrature cells can also cross several source features. Two100m local Sun samples cover only the initial200m; they cannot reconstruct narrow remote shadows already lost by the tail cache. Four upper directions are averaged for ambient on the same coarse spatial grid. View96vs384rays does not refine either cache.

Scene rebuilds this cache and receiver map on each rendered frame, including held source/camera frames. That repeats work yet leaves the underlying light bandwidth coarse. A production schedule should key work by compatible source/Sun/geometry/control revisions and admitted temporal error, not equate display frames with source changes. A source interpolated between fixed states is itself changing; reuse still needs a stated error/staleness policy.

## Primary-deck interpretation

Re-read the retained primary Nubis³ PDF: SHA256 `8b084d192c72cac4c69e29de40ecf633ac396678a0c5fb83178ad7fee193700f`, official [2023 deck](https://advances.realtimerendering.com/s2023/Nubis%20Cubed%20%28Advances%202023%29.pdf). Printed slide numbers are PDF-page numbers minus3: slides125–129 correspond to physical pages128–132;147to150;154to157.

Slides126–127 describe two local samples, followed by remaining accumulated density stored in a256×256×32voxel grid outside view marching. They describe occupied-voxel-to-grid-boundary ray marching and8-frame amortization.147applies accumulated sky-direction density to ambient accessibility;154summarizes the decoupling. The grid has about114times the nodes of our held cache, but metric extents/precision are not supplied there. It is not source-density resolution and the described producer is not a Sun-space cumulative scan. Their console performance/amortization does not predict our workload, browser timing or fast1x/16xsource latency. No private studio lifecycle is inferred.

## Scoped production challenger, conditional on the image comparison

A Sun-aligned cumulative optical-depth scan is a practical alternative producer for a genuine point-to-Sun tail:

`tau(u,v,s) = integral_s^sunwardBoundary sigma_t(u,v,s') ds'`.

Each column evaluates the joint source at cells once and accumulates toward the Sun-away side. Construction is linear in grid cells times local source/quadrature cost, rather than a complete long ray per output voxel. Dense-equivalent pointmax occurs within a formation, independent formations/media add, then integration occurs. A fixed small per-cell integration quadrature can be needed for thin features; one midpoint per large cell is not automatically adequate.

Keep the first proof's output/filter semantics unchanged: derive T0/T1 at scan nodes and linearly filter those, rather than silently substituting exp(−interpolated tau). R32tau arithmetic below is a useful storage screen; a directly writable/filterable RGBA16F T0/T1/A/valid layout costs twice as much. Portable R16/RG16 storage writes cannot be presumed; optional texture-format features or buffer-copy/packing need real capability, staging and bandwidth accounting ([WebGPU format capabilities](https://gpuweb.github.io/gpuweb/#plain-color-formats)).

At the actualSun orientation, illustrative grids rounded to8cells (BEFORE interpolation guards, padding, transient overlap and other products) are:

| Domain | Transverse / longitudinal pitch | Dimensions | R32tau MiB | RGBA16 MiB |
| --- | --- | --- | ---: | ---: |
| Full12×14×12km box |128/128m|136×168×176|15.34|30.68|
| Tight native positive-support AABB |128/128m|88×120×120|4.83|9.67|
| Tight native positive-support AABB |64/128m|168×232×120|17.84|35.68|

Native positive-support bounds came from the exact frozen field, not a future moving/swept-source admission.128m may still lose200m cues;64m may still fail cost. The proposed8MiBlight/shadow reservation cannot contain the fine layout unchanged, and not even the tight coarse RGBA16layout before extra products. Double generations/staging can enlarge these numbers. A4.68Mcell scan versus32long samples at each of4.68Mpoints removes a factor32in Sun coefficient queries at equivalent chosen nodes, but this is not an equal-quality GPU speedup claim; prefix/scheduling/memory and all other work remain.

A bounded implementation comparison would use a coarse regional tail plus fine demanded Sun-space coverage, conservative projected/swept caster bounds and direct-ray fallback. Fine columns start from the coarse upstream-boundary tail, then accumulate only their own segment: no duplicated coarse/fine path. A below-source receiver can use the full-column value; an arbitrary interior cloud point must use its own depth, never a total-column shadow. Main/mirror/environment/receiver demands and upstream offscreen casters all participate.

Opaque geometry visibility remains separate from weakened cloud-MS attenuation. Fog's RGB extinction is not automatically multiplied by the cloud octave factor. Clear-air Sun attenuation remains separately applied once. Ambient remains an independent problem: directional cumulative scans can remove its long-ray multiplier too, but four directions have their own resolution/construction/memory cost and require an ambient reference. A Sun-only scan is not complete lighting.

Changed casters invalidate downstream column influence, not merely their occupied bricks. Sun direction or coordinate rebasing can invalidate whole products. Publish compatible source/light/geometry/time generations atomically, or declare and bound reuse error with current fallback. Do not transplant8-frame reuse into rapid source growth or16xwithout that check. Preserve first/extra attenuation accuracy when choosing tau saturation or early termination: b=.25makes an otherwise opaque first-scattering path relevant to the extra term much longer.

## Current disposition

No cache implementation or canonical/evidence change is commissioned here. Root first compares unchanged-density images under aligned profiles, isolated terms and low/high light caches. Source-normal and source-only diagnostics remain necessary. Even a decisive lighting improvement would not pass source anatomy, Cb/ice/lifecycle, motion, full-sky consumers, cache budgets or device performance.

Reproduction verification: a second script run compared equal to the complete retained JSON, including all2,048point indices and per-point arrays. Script SHA256 `7eccff2b257c9845dfc134f59149cb079ed9d001067f9958ae1257fdbeff3edf`; results SHA256 `fb467e782aa17a8f6bb0efa5955a5a8f5480a9847f6f9a067c861446b81baac0`.
