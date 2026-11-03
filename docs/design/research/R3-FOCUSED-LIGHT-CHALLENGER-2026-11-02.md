# R3 focused-light challenger — 2 November 2026

Status: independent analytical challenge; no implementation, image acceptance or device timing.
Owner scope: this research note only. Canonical selection belongs to the R3 coordinator.

## Independent target, before inspecting R3's optical design

Commission: finite-Sun direct light through moving water must illuminate submerged and wet
above-mean-water receivers, including wave crests, overturning folds and falls, with real
occluders, scattering media, two views and bounded regional proxy coverage. A caustic texture
on a nominal plane does not satisfy that commission. This section was written after reading
repository instructions and the design skill, before WATER-OPTICS, RENDER-SPEC, PLAY-ENV,
CONTRACTS or the render dogfood. The task brief is prior exposure; blind independence is
limited to derivation of the operator, not discovery of those requirements.

The independent physical construction is an incoming source-plane measure, not a receiver
projection. For a bounded world domain, place a plane outside all admitted transport geometry
orthogonal to the Sun centre direction. Integrate over plane coordinate u (m²) and the actual
solar disk direction ω (sr), with incident packet flux
`dΦλ = Lsun,λ(ω) |ω·np| du dω`. The disk radiance integrates to the E1 irradiance; treating that
irradiance as disk radiance would multiply energy by the disk solid angle incorrectly.

A source element follows first intersections in the complete admitted optical scene. At a
water boundary apply Fresnel reflection/transmission and Snell direction with the actual
oriented interface; at each homogeneous/heterogeneous segment split incoming energy into
transmitted, absorbed and scattered fractions using optical depth. An opaque first hit ends
the direct path and deposits its surviving flux. “Below mean sea” is never an intersection
predicate: an above-mean wet crest can be the first receiver, and a falling sheet can provide
two or more interfaces. A water-to-air crossing can totally internally reflect. Finite path
count is a model restriction if further reflected/transmitted paths retain energy.

For a smooth, single-path source cell C, map its four-dimensional source coordinates into
receiver position. A footprint is the pushforward of flux, with positive normalized kernel K
on the actual receiver surface: `E(x) = Σ ΦC KC(x)`, where `∫receiver KC dA = 1`. At a fold the
Jacobian can change sign or vanish; dividing by a signed determinant is wrong. Mapping only
cell vertices can bridge a shadow or miss an interior focused branch. A caustic may be sharp
at finite solar extent, but the solar disk alone gives no geometry-independent error bound.

The analytical closure is conditional. Deterministic adaptive source cells can cover all of
a finite source support, account for all admitted energy and bound unresolved energy by
retaining conservative path/visibility intervals and charge for each unresolved cell. They
cannot guarantee arbitrary image accuracy at a fixed work budget for arbitrary moving
geometry. A tiny aperture or thin refracting sliver lying strictly between all sampled rays
is a counterexample to endpoint/centre/differential-only refinement. It can redirect a finite
unmeasured source region to one receiver pixel while producing no variation at any sample.
Conservation of the packets actually traced does not bound that missing source region.

A viable selection therefore requires either certified conservative source-cell intersection
and mapping bounds, or an explicit numerical quality limit with unresolved flux reported.
Photon maps and light-space traces are useful realizations/comparators; screen-space-only
coverage cannot establish source completeness. Coarse mean-plane refraction cannot represent
the required folds or above-mean receiver ordering. A finite proxy domain must conservatively
include all source paths which can reach admitted receivers, not only a camera frustum.

The useful error norm is receiver-bin integrated flux (and radiance after the selected BSDF),
not unbounded pointwise caustic irradiance. Each cell has `[Φlow, Φhigh]`, conservative reachable
receiver-bin support and a path/model status. Summing cell intervals gives per-bin bounds.
A global unresolved-flux upper bound certifies an L1 flux error only when the treatment of
resolved cells is itself bounded; it does not certify local error or beauty. Bounds containing
most of the Sun's input energy are honest but useless for quality. Fixed budget exhaustion
must return incomplete/unsupported rather than declaring this operator complete.

## Selection after the bounded-envelope challenge

**Recommend retaining a concrete high-quality challenger, `FocusedInterval`, with refusal before
presentation when it cannot certify its admitted error. Do not promote it to the practical 60 Hz
R3 tier yet.** The earlier counterexample rejects sampled-error refinement, not every bounded
algorithm. A finite conservative algorithm can be specified for a deliberately restricted scene.
The following is an authored mathematical construction, not a method disclosed by any cited engine.

A sound conditional algorithm and a useful real-time admission envelope are different achievements.
This note closes the former for finite receiver-bin flux under the restrictions below. It does not
supply an implemented interval arithmetic library, a performance proof, an accepted caustic image,
or the complete filtered image-space error proof for focused single scattering. Those remain gates.

### Concrete scene and source producer

F4 cooks an optical world inside one convex finite box B with a complete static/rigid triangle BVH,
actual oriented finite-thickness water shells, declared material sides and binary cutout boundaries.
Binary masks must become finite triangle geometry for this challenger, or carry conservative UV
opacity range queries that return opaque/clear/mixed; a mip-average alpha texture cannot certify an
aperture. F4 can merge exactly coplanar adjacent triangles into convex planar faces to avoid needless
seam uncertainty. Camera, receiver and participating-media support are finite and pinned. Unsupported
deforming/transparent geometry rejects admission; it is never omitted. A later version may use
conservative swept pose and displacement intervals, but their producers are a separate extension.

The optical shell is **the selected numerical geometry**, shared by raster/optical consumers. Its
approximation to the physical/evaluated water is model/geometry error with separate admitted bounds.
This initial challenger does not pretend to interval-evaluate an arbitrary analytic wave normal.
A bounded wave-curvature tessellation producer can cook/refit that shell, but a position-only error
cannot bound Snell direction. Need normal error, topology preservation, thickness/order margins and
minimum retained optical feature. Specular shading normals which disagree with the shell require a
separate microfacet transport model; substituting them silently breaks this closure.

Place the incoming source plane outside B, facing the disk centre. Project all eight corners of B
back to that plane for every direction in the finite solar disk; interval direction extrema give a
conservative finite 2D support rectangle. Rays outside that rectangle cannot enter B. This deliberately
covers the whole admitted box, avoiding an unspecified inverse caustic footprint from one camera.
A cheaper support clip is allowed only with its own conservative reachability proof. All geometry,
interfaces and media along rays in B are pinned; external atmosphere/cloud attenuation supplies one
boundary incident radiance under E1, and must not be charged again inside B. Alternatively place those
fields inside B and integrate them there, with a declared clear boundary source. The choice is pinned.

Parameterize the solar disk by `μ=cos θ∈[cos α,1]`, `φ∈[0,2π)` around its centre; `dω=dμ dφ`.
Source cells are rectangles in `(u1,u2,μ,φ)` with exact parameter measure. Direction comes from the
orthonormal disk frame and `sqrt(1−μ²), sin φ, cos φ`; interval evaluation encloses it. Constant disk
radiance is the first admitted source model. E1 normalizes its disk integral to the stated irradiance.
The polar singular coordinate at θ=0 is harmless to area but may weaken interval direction bounds;
split or use a local Cartesian disk chart with conservative clipped area there. Positive RGB flux
bounds use source radiance, plane area, solid angle and interval projected cosine. Solar limb-darkening
can be added only with bounded radiance extrema, not an unrelated noise texture.

### Actual interval path and occluder producer

Use outward-rounded interval arithmetic for every bound, including the chosen triangle predicates,
trigonometry, square root, Fresnel and exponential. Native f32/WGSL arithmetic alone is not such a
producer: the implementation must use validated conservative rounding/error expansions or a CPU
reference path. Overflow, zero-containing divisors, NaN, uncertain ordering and underflow affecting
the budget produce Unknown, never a convenient tight interval.

1. A source cell supplies interval origin O and direction D. Traverse B's complete BVH using interval
   ray-box tests. A node is excluded only if every ray in the cell misses it. Otherwise visit its
   children; uncertain boxes remain candidates. Candidate-list/node/triangle caps retain the cell's
   entire remaining flux upper bound as unresolved. No triangle enumeration completion means no miss.
2. For each planar candidate, interval-evaluate ray-plane intersection `t`, then barycentric edge
   tests (or interval Möller–Trumbore). If the direction-plane denominator includes zero, keep an
   uncertain candidate. A hit is mandatory only when all interval points are forward, within the
   finite path segment and within the face's half-open interior, with conservative predicate margins.
   A single mandatory hit is the unique first hit only when its largest t is smaller than every
   other possible hit's smallest t. Otherwise split the source cell. An exact shared-edge predicate
   may resolve coplanar ties; conflicting media/noncoplanar coincidence rejects the shell.
3. Propagate the hit position interval and oriented side. Compute `k=1−η²(1−cos² i)` for Snell.
   Entirely negative k certifies TIR. Entirely positive k permits interval transmitted direction;
   a k interval spanning zero subdivides or remains unresolved. Interval Fresnel factors partition
   the packet into reflected/transmitted flux; retain both branches where their power is nonzero.
   Do not apply the radiance index-squared factor to energy flux. A path-branch cap applies as well
   as the interface-event cap: four interfaces can create more than four live branches.
4. On a segment, intersect all admitted homogeneous air/water/fog volumes, order their entry/exits
   using the same interval rules and sum compatible coefficients. Beer factors use interval segment
   lengths and coefficient extrema. Absorbed and out-scattered power have separate ledgers. A
   heterogeneous field needs an actual extrema hierarchy per cell/segment; the first challenger
   instead admits bounded homogeneous volumes. Unknown medium membership leaves power unresolved.
5. At a first opaque receiver, stop the direct branch. Its interval position plus face ID yields
   possible receiver bins. A cell wholly inside one bin gives a mandatory-bin lower bound as well
   as an upper bound. A bin boundary crossing subdivides. A branch leaving the convex B into a
   declared terminal background records escaped power. Do not classify it as receiver darkness.

For each terminal family retain a lower/upper flux, origin cell ID, receiver/material/medium identity,
endpoint epoch and numerical status. Refine the source dimension contributing the widest projected
uncertainty, using a fixed tie order; pure depth/variance heuristics can prioritize work but never
certify a leaf. At cap retain its whole conservative upper power. All pending cells remain charged.

For each receiver bin b, sum mandatory-throughput lower bounds `Lb` and possible upper bounds `Ub`.
A nonnegative estimated deposition `Φhat_b` is accepted only if its interval error is below the
admitted absolute-plus-relative bin tolerance. A separate summed flux-error tolerance prevents many
individually small errors exhausting the scene's energy. Before division by bin area, preserve these
flux bounds. Very dark bins need an absolute threshold; relative-only bounds fail at black/penumbra.
If unresolved source packets are deposited arbitrarily with normalized kernels, their contribution to
a global L1 flux error can be as large as **twice** their upper input flux; dropping them gives a
one-sided missing-flux bound but is not a valid completed-light presentation. Add bounded errors from
resolved throughput/source integration; input-energy bookkeeping by itself is insufficient.

The reconstruction may splat a cell as positive flux over its certified receiver footprint, clipped to
the actual receiver face and normalized **after** clipping. Never renormalize across an occluder or
disjoint receiver identity. Split folds/multiple hit branches before splatting. Bounds are evaluated
against the source-to-bin transport integral, not merely against the splat's own normalization.
The raster kernel is a reconstruction choice; finite solar extent is integrated in the source domain,
so enlarging the splat again by the entire solar disk can blur it twice. At unresolved footprints,
retain bounds and reject rather than clamp bright values. Filtered normal covariance is a different
model from deterministic resolved shell directions and requires a separately normalized distribution.

### Finite work, adversaries and useful acceptance

The recipe pins root count C0, binary subdivision depth D, total cell-evaluation cap M, live-cell cap Q,
per-cell node/triangle visits, per-path length, interface count K and live branch count J. Full expansion
would visit at most `C0(2^(D+1)−1)` cells; execution stops at the smaller M. Every individual operation is
bounded. A full queue, visit cap, event cap, unresolved flux or unsupported scene never produces Hit/Miss
or Ready. It produces `IncompleteFocusedTransport {source_partition, reason, upper_flux, endpoint}`.
If bounds fail the requested tolerance at those caps, F2 refuses this focused capability or chooses a
separately admitted whole-partition model. It does not reinterpret the refusal as a failed experiment
proving focused transport impossible.

Minimum feature size, bounded curvature, finite disk and K **alone** do not ensure a small M suffices.
Two well-sized surfaces may approach arbitrarily closely, making their order uncertain at tiny source
scales. Water-to-air rays can approach the critical angle with derivative amplification at k≈0. A
large smooth source footprint can lie almost on a receiver-bin boundary; geometry-wide displacement
error does not bound the distribution between its neighbouring bins. Coplanar positive-measure exact
ties need explicit ownership or rejection. These conditions are why certification/refusal is part of
the selected mechanism, rather than a claim of universal error after a fixed sample count.

Two small adversaries distinguish the operators:

- **Aperture:** one 0.5 m clear opening in an opaque roof over a shallow pool, a refracting patch under
  it, and a wet ledge above mean sea. Translate the opening so every coarse ray hits the roof but the
  transmitted interval tube overlaps the opening. Sample variance is zero; interval candidates remain
  mixed and refine or refuse. A correct packet ledger with no transmitted packets fails this test.
- **Critical angle/order:** an oriented curved exit shell whose refracted k ranges over zero in one
  source cell, plus a 0.5 m slab only δ above the receiver. Shrink δ without shrinking either feature.
  Fixed derivative thresholds can claim stability while changing the reached receiver/TIR branch.
  The interval method retains both possibilities until it can certify order and k or reaches a cap.

A **predicted easy admission fixture**, not measured evidence, is a 16×16 m calm coast region, 3 m
clear water, planar broad bed, one wet ledge above mean sea, Sun elevation 45°, no nearly tangent hits,
no transparent solids and well-separated broad rigid blockers. A shell extending beyond the source/
receiver interest by the maximum finite-disk shift avoids source-edge ambiguity. Use 4 m receiver bins
for the first numerical closure check, then refine to the actual pixel/receiver footprint for beauty.
On the blocker-free parallel-plane interior, exact planar mapping is a translation per solar direction;
polygon-overlap area gives an independent flux reference. Such an interior must admit with arbitrarily
small tolerance as source/angular cells refine, apart from interval roundoff limits. Broad blocker
boundaries should consume a narrow refinement strip; this is a falsifiable prediction, not a count
proof. If the cap cannot admit even this case, tighten bounds/merge coplanar faces before increasing
budget. Passing coarse bins does not qualify sharp caustic detail, a folded breaker, or the larger coast.

An illustrative initial resource envelope: 64² plane roots × 8 angular roots = 32,768 cells; at most
131,072 cell evaluations, Q=65,536 pending records and 160 B/record = **10 MiB** pending storage, plus
10 MiB replacement/frontier space. At two path segments and an assumed 24 BVH node visits per evaluation,
node traffic alone is about **201 MB per update** (`131072×2×24×32 B`), before interval arithmetic,
triangle candidates, branching, outputs and receiver-bin work. A strict 256-node cap allows **2.15 GB**
node traffic at that M/two-segment envelope. Four segments double these numbers. Those are logical byte
counts, not DRAM traffic or timings. Interval records/scratch and dual-view buffers are additional.
Small source regions and branch-aware reuse matter more than saving one scalar instruction.
This is plausibly an offline/reference or powerful-desktop challenger; it is not a demonstrated
browser/mobile/60 Hz selection. Two views can reuse the source-cell tree/transport only for identical
endpoints and common support; receiver binning, phase evaluation, camera visibility and temporal state
remain per view. Widely separated views can enlarge B and source work substantially.

## Media, two views and the remaining full-scope gap

Direct extinction removes `σ_t=σ_a+σ_s` power along the source path; only the σ_a share is absorption.
The σ_s share is the source for scattered transport. Surface irradiance alone loses incoming direction
and cannot supply anisotropic scattering. The correct single-scattering camera source is
`j(x,v)=σ_s(x) ∫ p(ω,v) L_direct(x,ω) dω`; integrate it with camera-path transmittance in the same
media operator. Retain each certified beam's direction/throughput/segment identity, not one scalar
caustic multiplier per receiver. Both views read the same world incident field and apply their own
phase/visibility and ordered camera interfaces at their actual retained times. Never multiply
already attenuated incident beam power by the same source Beer/cloud factor again.

There is a consequential remaining limit: the surface-bin interval construction above does **not**
automatically provide a bounded filtered volumetric image. Accumulating energy into voxels and then
marching their average is a new spatial/directional reconstruction, capable of leaking across a thin
occluder. Conservative beam bounding boxes are support bounds, not beam interior radiance bounds.
Published beam-volume slicing is a useful candidate; its vertex interpolation/volume overlap must
also be bounded, particularly when a beam folds or a camera path refracts. A coherent staged admission
is absorption-only focused surfaces first, then homogeneous single scattering after an explicit
beam-to-camera footprint integral/error construction. Until that construction is selected, the
commission's **complete focused surface + media operator remains unfinished**. Reporting scattered
energy as removed does not qualify the missing visible shaft. Multiple scattering, arbitrary
heterogeneous media and transparent solids are separate model extensions.

Finite-Sun quadrature error, interval looseness, BVH predicate error, footprint/receiver binning and
camera reconstruction are numerical errors. Mean-interface direction, unresolved surface covariance,
geometry approximation, finite interface/branch truncation and omitted multiple scattering are model
errors. An untraced tail can instead be a quantified numerical truncation only if its remaining
energy/radiance influence is bounded and within the admitted tolerance; an event count alone is not
such a bound. The surface-only candidate can rigorously refuse on a branch cap without pretending to
solve every path. More samples cannot repair an omitted interface, material, media source or domain.

## Alternatives and inspected primary evidence

Research was checked on 2 November 2026. The table distinguishes substantive inspection from access
limits. No source code was adopted or copied, no demo was executed and no video was watched.

| Candidate/evidence | What was inspected; consequence for this decision |
| --- | --- |
| Deterministic adaptive source tessellation / interval footprints | The construction above is Thrum-authored. It provides conservative source support and receiver-bin bounds for its finite triangle model, with a complete refusal route. Its performance, portable conservative arithmetic and full scattered-image extension remain unqualified. Prefer it as a high-quality challenger/reference, not a claimed production tier. |
| [Gruen, Ray Tracing Gems chapter 14](https://link.springer.com/content/pdf/10.1007/978-1-4842-4427-2_14.pdf), 2019 | Full chapter text inspected, especially §§14.2–14.3 and 14.6, pp. 186–196, 200. Actual producer: light-view water map → traced receiver map → adaptive beam volumes → phase-weighted slicing. Twisted beams, thin-feature leakage and resolution dependence matter. It supports a practical candidate, not certified source-cell bounds or portable hardware-tessellation availability. |
| [NVIDIA water-caustics implementation account](https://developer.nvidia.com/blog/generating-ray-traced-caustic-effects-in-unreal-engine-4-part-2/), 2020 | Inspected workflow and PDS/PCM/cascade sections, figures 4–8. Adjacent-ray differences determine footprints; connected receiver triangles provide a sharper alternative. Resolution blur and triangles spanning discontinuities are disclosed. Cascades address scale distribution; neither method certifies unsampled path energy. Reported RTX timings are not Thrum timings. |
| [NVIDIA AAPS mesh-caustics account](https://developer.nvidia.com/blog/generating-ray-traced-caustic-effects-in-unreal-engine-4-part-1), 2020 | Inspected basic workflow. Photon differentials and temporal feedback allocate/splat photons. Statistical efficiency and stable ellipses are useful, but feedback on observed photons cannot discover every narrow aperture deterministically. Camera-directed allocation needs a separate source coverage obligation for two views. |
| [Mayer et al., JCGT 2026](https://jcgt.org/published/0015/01/03/paper-lowres.pdf) | Full paper text inspected, especially §§2.2, 2.4, 3.4, 4 (pp. 47–49, 52–53). Fast Newton intersection and light-space mesh deposition are credible accelerators. Discarded failed triangles/fragments and missing capture geometry prevent an energy-complete fallback. A light G-buffer extends camera coverage but still hides geometry behind its first layer. Use candidates only after regional first-hit validation. |
| [Hedstrom et al., ReSTIR BDPT](https://research.nvidia.com/labs/rtr/publication/hedstrom2025restir/), 2025 | Author project summary inspected; official [implementation README](https://github.com/Shmaug/ReSTIR-BDPT) inspected for platform and known rough-glass shift bug. Bidirectional sampling/resampling is a credible broad-path comparator, not a deterministic error certificate. Its reported test cost and DXR dependencies do not establish a portable R3 tier. Full paper/code paths not audited here. |
| [Guided ReSTIR FG+ official implementation](https://github.com/TU-Clausthal-Rendering/Guided-ReSTIR-FG-Plus), EGSR 2026 | Author README sections on demo, new scenes and prerequisites inspected. It explicitly excludes directional/environment photon emission, currently requires several sources and warns of darkening on photon-buffer overflow. Thus this current implementation does not directly supply the Sun case. Refactored public code is not the exact measured paper build. No full code audit or runtime claim. |
| [Cascaded Image-Space Photon Tracing](https://diglib.eg.org/items/2fa1b831-8b43-4a18-83c3-66223e3543f9), 2022 | Primary repository entry/abstract inspected: camera and light captures, one specular bounce, denoising. PDF fetch returned 403, so no algorithm-detail claim beyond that entry. It remains a comparison candidate; combining two captures does not itself establish complete regional geometry. |
| [Liktor and Dachsbacher adaptive beam tracing](https://cg.ivd.kit.edu/downloads/VolumeCaustics_Preprint.pdf), 2011 | Indexed primary abstract/first-page material inspected; full preprint fetch timed out. It supports the lineage of adaptive beams for single scattering, not the detailed correctness of this new interval proposal. Gruen's inspected chapter provides the substantive beam operator used in this comparison. |

The [GDC 2019 talk entry](https://www.gdcvault.com/play/1026161/Ray-Traced-Water-Caustics-with)
and NVIDIA's [slide link](https://developer.nvidia.com/rtcaustics-unrolled-gdc19) were checked. The latter
was inaccessible in this session; neither a recording nor those slides was inspected. The full
same-author open chapter is the substantive substitute, not evidence of having watched the talk.

Available implementation inspection was deliberately scoped. In Evan Wallace's MIT
[`renderer.js`](https://raw.githubusercontent.com/evanw/webgl-water/master/renderer.js), inspected
`causticsShader`, `updateCaustics`, `getWallColor` and `getSphereColor` paths: graph-water vertices
project through cube/plane receivers; fragment derivative area ratios build a 1024² caustic texture;
receiver queries use a mean direction and special sphere/rim shadow terms. It is an elegant bounded
pool demonstration. It supplies neither general regional first-hit geometry nor folds/falls,
finite-disk integration or conservative missed-energy bounds. Only these specific producer/consumer
paths were inspected. No reusable subsystem was selected from it.

**Revision limitation:** GitHub pages/raw sources were readable, but commit/history/API fetches did
not expose a verifiable revision, and shell network resolution was unavailable. These code readings
are date-scoped observations of the displayed branches, **not pinned reuse evidence**. Before adopting
code, F4 must pin the exact revision, license and producer/consumer/configuration closure and repeat
inspection. This access limit does not affect the analytical counterexample or select uninspected code.

## Reconciliation with current Thrum authority

After the independent section, inspected [WATER-OPTICS](../modules/WATER-OPTICS.md),
[RENDER-SPEC](../RENDER-SPEC.md) §5, [PLAY-ENVIRONMENT-SPEC](../PLAY-ENVIRONMENT-SPEC.md),
[CONTRACTS](../CONTRACTS.md) C07–C09 and the
[render implementor dogfood](RENDER-IMPLEMENTOR-DOGFOOD-2026-11-02.md) R3 proposal.
The existing first-water-interface emission, actual rigid/cutout regional coverage, shared surface
evaluator, exclusive ordinary/focused source partition and typed incomplete hits are the right
invariants. The missing focused mechanism was the producer of **complete source coverage and
bounded unresolved flux**, not the normalization equation. `FocusedInterval` supplies a restricted
candidate with explicit prerequisites/refusal; it does not make those prerequisites implemented.

The dogfood's optical-A intentionally excludes focused light, folds/falls and scattering. That is a
legitimate small proof but cannot close the wider commission. A mean plane through the actual mean
sea can lie below a wet crest receiver: the reverse water ray then reaches it behind the receiver.
The regional owner's subsequently proposed **virtual cover plane above every admitted wet receiver**
fixes this geometric query-domain defect and may be a coherent lower model. It deliberately changes
actual entry position, angle, Fresnel, attenuation lengths, focusing and shadow relationships. Its
source completeness is completeness **within that virtual model**, not conservation/equivalence of
the actual folded surface. Raising a virtual plane does not recover actual-surface caustics.

For the coordinator:

- Keep one immutable E1/Sun/surface endpoint and shared regional geometry coverage across source,
  camera and both views. Water above mean sea and straddling solids are selected by actual events,
  never by a wet/dry geometry mask alone. Render-only folds do not grant physical water occupancy.
- Keep `FocusedInterval` a conditional high-quality challenger, with absorption-only first admission
  and explicit remaining single-scatter image closure. Virtual-cover transport is separately named
  and selected where its model limits are accepted. A failed focused tile may switch only its complete
  source partition to a ready admitted lower model; it cannot mix successful focused samples and full
  lower-model energy or label missing source flux as shadow.
- Select whole-partition/tile identity in **incoming source coordinates**, not arbitrary receiver tiles.
  Camera-local blending can double a common packet or omit a packet whose receiver moves between views.
  Any crossfade weights sum to one for the same source flux; both whole solutions must be valid.
- Keep media incident-direction/source ownership, first-hit coverage, event/branch caps, unresolved
  energy and model/numerical error visible in C08/C09. F2 quality admission and C12 measurements should
  not confuse valid lower-model transport with accepted full focused optics.

## Discriminating proof queue and status

1. **Arithmetic/coverage:** independent exact/high-precision planar and triangle scenes, finite disk,
   opaque roof aperture, hidden/offscreen bed and blocker. Compare receiver-bin reference flux against
   certified intervals; deliberately exhaust every queue/visit/event/branch cap. Any missed reference
   flux outside bounds or Ready after unreported cap rejects the implementation.
2. **Actual water:** one smooth crest above mean, one finite fall into a pool and one oriented fold.
   Enter/exit/Fresnel/TIR and first-source emission must agree with an independent high-sample light
   tracer. Move the wet ledge and blocker without changing views, then move each view independently.
   Do not let both evaluators share the same missing geometry/cutout product.
3. **Spatial quality:** refine bins/footprints to the intended image scale, native motion, crest and
   roof translation. Compare focused, virtual-cover and high-sample reference at equal exposure and
   source flux. Reject shadow bridging, lost wet-crest illumination, pumping, blur of defining caustic
   concentration, temporal ghosting and finite-disk double blur. Numerical conservation alone passes
   a spatially uniform wrong substitute; these observations must reject it.
4. **Media extension:** specify and verify beam-to-camera filtered integral before claiming focused
   shafts; move the same blocker through clear and scattering water and compare two view phases.
   Merely reporting scattered-away flux does not satisfy this stage.
5. **Integrated cost:** one admitted exact build/content/device identity, all render/environment/terrain
   consumers and two views active. Record source-cell/branch debt, refusal rate, upper unresolved flux,
   peak buffers, uploaded/refit bytes, completion latency and sustained GPU/CPU frame tails. Then reduce
   cell rate under a motion/error bound or select the valid lower model. A stale caustic history is not
   a free completed update. No mobile/browser inference from RTX reference results.

Evidence achieved here: independent derivation, counterexamples, current primary-source comparison,
a restricted conservative surface-flux algorithm with finite refusal, and a falsifiable cost/acceptance
proposal. **Unrun:** arithmetic implementation, useful conservative-bound tightness, focused media
image closure, rendered appearance and all target-device viability. No runtime maturity is promoted.
