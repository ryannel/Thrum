# V1–V4 rendering research and environment handoffs — 2 November 2026

## Disposition and evidence boundary

Recommend retaining Thrum's selected WGSL/wgpu/Naga, depth-prepared clustered-forward baseline,
finite cooked pipeline closure, scene-linear photometric transport and optional spatial reconstruction.
The consequential remaining design work is a concrete bounded layer/pass contract, query-complete
water visibility, per-view sample identity and shared environmental visibility ownership. More elaborate
sky/cloud lighting or a faster opaque light loop cannot close those interfaces.

This is supporting research for [RENDER-SPEC](../RENDER-SPEC.md), [the rendering family](../modules/RENDERING.md),
[C07/C08](../CONTRACTS.md#c07-render-scene-views-and-temporal-data--v2-with-p4v3v4),
[BASELINE-SPEC](../BASELINE-SPEC.md) and [WATER-OPTICS](../modules/WATER-OPTICS.md).
Canonical specifications and contracts remain authoritative. Recommendations below are **proposed** unless
already selected there. No renderer implementation, image approval or target measurement is supplied.
The author read the existing Thrum design and reconciliation before this pass; this is an independent
target derivation from requirements and sources, not a blind study. Broadsides code was inspected only
after deriving the ownership, representation and operating model below. No code was copied or modified.

The commission applies the repository engine-module-design research, review, reconciliation and performance
references, with engine-architecture for owner boundaries. This pass is self-challenged; it is not an
independent review of itself. Source access was through official pages and primary PDF text, including
substantive production slides/notes. The follow-up audit below also reads official conference-video
transcripts; no video was watched or sample application run.

## Requirement coverage before selecting mechanisms

The reusable engine needs Mac/Windows native, WebGPU browser and native iOS/Android profiles, perspective
and orthographic cameras, optional environment capabilities and one gameplay authority. The first small
game must remain useful without water, weather, vegetation or temporal reconstruction. The family must
also admit their consequential interactions without separate devices, clocks, Suns or display transforms.

| Capability | Required observable result and basis | Producer → consumer; proposed/selected mechanism | Evidence still needed |
| --- | --- | --- | --- |
| RR01 Execution/readiness | A pinned game produces its first intended input/visible response without undisclosed first-draw compilation; R0/F4/C05/C06 | F4 immutable shader/descriptor closure → V1 local prepared pipelines and one submission owner | Actual target ABI, cold preparation, failure/device loss and first-play trace |
| RR02 Shared scene/multiple views | One committed world, two different cameras, independent cadence and histories; R1/C02/C07 | W1/P4/E1 snapshots → V2 persistent instance tables plus per-view visible lists; V4 histories keyed by camera identity | Two equal-profile cameras, skipped samples, cuts, corrections and leases |
| RR03 Visibility/LOD | Ground, plants, characters, offscreen casters and reflections preserve declared silhouette/coverage/error | F4 geometry/material products → V2 projected-error LOD and conservative spatial hierarchy | Orthographic, shoreline, thin cutouts, caster-only geometry, LOD history |
| RR04 Surface light | Sun/local lights, materials, probes and shadows agree in one colour/unit convention; R0/R1/C08 | W1 light descriptors and E1 drivers → V3 one registry, GGX material and clustered forward | Hemisphere response, masked parity, overlap overflow, light movement |
| RR05 Ordered transport | Water, fog, glass and particles have coherent occlusion/lighting; R3/C08 | E1 coefficients/interfaces → V3 jointly lit segments, bounded transparent events and explicit reflection/refraction policy | Absorber ordering, medium boundaries, unsupported layers, offscreen receivers |
| RR06 Coherent environment | Cloud shadows, ground, water and foliage respond to the same source with visible clouds absent | E1 sampled causes → V3 typed source-ray attenuation; V2 geometry visibility | One-attenuation checks, headless agreement, source validity and feature-off resources |
| RR07 Deformed motion | Plants/water/skins do not borrow rigid motion or another camera's prior sample | P4/E1 evaluators → V2 current/prior material-point correspondence → V4 confidence/reactivity | Ordinary motion, disocclusion, sample gaps, changing wind/content/topology |
| RR08 Display | Stable brightness and output colour across authored exposure/camera changes | V3 pre-exposed scene colour → V4 reconstruction, exposure, tone/gamut mapping and output encoding | SDR cards, moving bright/dark scene, output formats and sustained targets |
| RR09 Optionality/creation | An omitted capability removes avoidable preparation/residency; assets remain reproducible | F2 typed profile → F4 target products → V1 prepared generation; stable source IDs and diagnostics | Missing-provider admission, stop/load, replacement peak and off-state ablation |

R0 has 64 entities, one camera, Sun/four local lights and opaque/masked materials; the concrete first
scene need not exercise every family provider. R1 has 4,096 static instances, 32×64-joint skins, 128
local lights and two cameras. Its secondary camera is half width/height every other frame. R2 reduces
authored presentation population to 1,024 visible static instances/eight skins/32 local lights and one
camera; gameplay compatibility still follows C13. R3 adds prescribed sea, one 128×64 shallow grid,
16 incident components/four floating bodies, fog overlap and cloud attenuation without visible clouds.
No R3 success establishes the wider water provider's full mechanics.

## Evidence account and competing whole approaches

### Sources substantively inspected in this pass

Each row records the useful locator and actual access scope. Statements drawn from each external source
are deliberately compact; the operating model and cost derivations later are Thrum synthesis.

| Source and locator | Inspected material, maturity and consequence |
| --- | --- |
| [WebGPU specification](https://gpuweb.github.io/gpuweb/), texture formats/capabilities, supported limits and asynchronous pipeline creation; [WGSL](https://gpuweb.github.io/gpuweb/wgsl/), memory layout and atomic built-ins; accessed 2 November | Normative evolving API, not a performance result. Query limits/features and validate exact attachment/binding tuples. `depth24plus` can use different underlying representations; format byte estimates cannot be physical measurements. Portable WGSL does not imply native vendor feature access. Pipeline readiness is an explicit state, not source validation. |
| [wgpu 30.0.1 Limits](https://docs.rs/wgpu/30.0.1/wgpu/struct.Limits.html), fields/defaults/check methods | Official version-specific API reference. Keep the existing four-group ABI within queried texture/storage/uniform limits. This is the version cited by RENDER-SPEC, not an instruction to adopt an unreviewed latest dependency. |
| [Epic RDG](https://dev.epicgames.com/documentation/en-us/unreal-engine/render-dependency-graph-in-unreal-engine), external/transient resources, load actions, memory lifetimes, debugging and Insights | Production engine practice. Declared uses support resource lifetime analysis and pass culling. Its transient allocator can alias memory beyond equal-descriptor pooling, but that facility is backend-dependent. Never assume prior contents after reuse. Retain explicit initialization and a graph/resource diagnostic; do not promise WebGPU heap aliasing. |
| [Lempiäinen, Stochastic Tile-Based Lighting, SIGGRAPH 2025 notes](https://advances.realtimerendering.com/s2025/content/s2025_stb_lighting_v1.1_notes.pdf), PDF pages 12–17, 23–33, 59–61 | Primary production notes, not watched video. Two-stage tile sampling shares a 16-light coarse reservoir, then resamples 1–4 lights; separate shadow evaluation improves coherence. Four-point importance estimation can miss lit pixels and bias edges. TAA/noise remains a challenge, and transparency/volume extension is future work in this account. Credible mobile many-light challenger, not a replacement for the complete water/media baseline. |
| [Drobot, Adaptive Voxel-Based OIT, SIGGRAPH 2025](https://advances.realtimerendering.com/s2025/content/AVBOIT_SIG2025_MDROBOT-final.pdf), PDF pages 10–18, 48–69, 94–97 | Primary production slides/text, not watched video. Stores/integrates depth extinction for approximate weighted transparency; adaptive depth occupancy compacts empty slices. Splat/sampling aliasing and self-occlusion remain representation errors. Compelling dense-VFX challenger; it does not itself supply refracted offscreen radiance or exact mixed-media source terms. |
| [Epic MegaLights](https://dev.epicgames.com/documentation/en-us/unreal-engine/megalights-in-unreal-engine), Technique Overview, Lighting Complexity, ray-tracing scene | Current UE 5.8 production documentation. Fixed ray/light sample cost trades increased lighting complexity for noise/denoiser bias and ghosting. Screen rays continue into a separate tracing scene; proxy fidelity matters. Its current media/translucency support is broader than HypeHype's disclosed 2025 path. Hardware/API and geometry preparation remain substantial obligations. |
| [Heitz 2014](https://jcgt.org/published/0003/02/03/paper.pdf), §§2.5, 3.6, 4.4, 6 and weak white-furnace discussion | Primary derivation. Correct masking is linked to projected-area normalization; first-scatter microfacet models omit interreflection. Supports verifying the selected height-correlated Smith closure, rather than calling any roughness/specular formula energy conserving. |
| [Bruneton 2017 author implementation account](https://ebruneton.github.io/precomputed_atmospheric_scattering/index.html), Introduction, Structure and Tests | Author documentation with dimensional checks and CPU spectral-reference strategy. Precomputed atmospheric colour still requires explicit luminance conversion/units. Useful comparator and reference design; this pass did not run or audit its entire source. No claim that it solves local overlapping fog or dynamic cloud structure. |
| [Epic Single Layer Water](https://dev.epicgames.com/documentation/en-us/unreal-engine/single-layer-water-shading-model-in-unreal-engine), Implementation and low-end/mobile alternatives | Production documentation. Lit opaque colour/depth is read for water transmission; reflection tiles combine screen and capture/environment terms. Mobile may omit volume integration/SSR. This demonstrates a practical bounded path and explicit reduced functionality, not optical completeness or a direct forward-renderer port. |
| [Asirvatham/Hoppe, GPU geometry clipmaps](https://developer.nvidia.com/gpugems/gpugems2/part-i-geometric-complexity/chapter-2-terrain-rendering-using-gpu-based-geometry), §§2.1–2.4 | Primary developer chapter. Nested regular height windows, incremental strips and transition morphing give bounded heightfield rendering. The single-valued height representation cannot own caves/overhangs. Its historic demo timings are not estimates for Thrum. |
| [AMD FSR 3.1.4 manual page](https://gpuopen.com/manuals/fidelityfx_sdk/techniques/super-resolution-upscaler/), Input resources, Motion vectors, Reactive mask, Exposure, Memory requirements | Official integration account. Current-to-previous unjittered vectors, exposure and transparency/reactivity are semantic inputs. Internal working set includes persistent and aliasable resources beyond input colour/depth. Version label differs from the newer SDK's analytical 3.1.6 fallback; record exact SDK when integrating. |
| [AMD current FSR Upscaling](https://gpuopen.com/amd-fsr-upscaling/), Requirements and 4.1.2 October 2026 version history | Official successor check. Reports Vulkan support and expanded AMD hardware support alongside DX12/Windows requirements. This still supplies no Metal/WebGPU implementation. Do not freeze a 2025 claim that all FSR4 is RDNA4/DX12-only, or infer a portable Rust/browser integration from a new native API entry. |
| [Epic Nanite](https://dev.epicgames.com/documentation/en-us/unreal-engine/nanite-virtualized-geometry-in-unreal-engine), foliage/material/platform sections; [VSM](https://dev.epicgames.com/documentation/en-us/unreal-engine/virtual-shadow-maps-in-unreal-engine), caching/invalidation | Current engine documentation checked alongside retained 2021 Nanite research. Virtual geometry/shadow products introduce distinct content/residency and invalidation contracts. Wind/LOD/moving lights change cache economics. Do not transfer a foliage claim to arbitrary alpha materials or every browser/mobile device. |

The retained [rendering source ledger](RENDERING-SOURCES.md) and [temporal composition study](TEMPORAL-COMPOSITION.md)
remain useful, especially Olsson's clustered-light derivation, Frostbite's material practice and Hillaire's
atmosphere closure. Access limitations in this pass: the linked Olsson presentation and Hillaire PDF
exceeded the web reader's size limit; Bruneton's official implementation account was accessible. Guerrilla's
[Nubis³ official page](https://www.guerrilla-games.com/read/nubis-cubed) describes compressed-distance marching,
voxel cloud modeling and light sampling, but its 14 MB slides could not be fetched here. Thus this pass
uses that page only to establish a successor to 2015/2017's 2.5D cloud family, not detailed algorithm or
performance evidence. An attempted shell download failed DNS; no local PDF was retained. The initial
2026 Advances URL attempt failed, but the **official www-hosted index was accessible in the follow-up
audit below**, including PSSR/tessellation notes and a retained local Smolder deck. The earlier failed
URL must not stand as a claim that 2026 conference material is unavailable. Remaining access limits
leave richer cloud selection open and do not block authored attenuation.

### Whole-path comparison and selection

| Complete approach | Strength and complete obligations | Thrum disposition |
| --- | --- | --- |
| CPU-visible batched raster, uniform small light list, static ambient | Minimal R0 preparation and diagnostics; still needs masked depth/shadow parity, exact pipeline readiness and display | **Selected first slice within baseline.** Cluster data may collapse to one finite list; no separate public renderer |
| Clustered forward + conventional shadows + probe indirect/reflections + spatial reconstruction | Shared surface closure for opaque/masked/blend; no mandatory external material G-buffer; light assignment, overdraw and shadow production still cost | **Retain selected portable family baseline.** Orthographic cluster depth and transparent-volume membership must be designed explicitly |
| Tile/deferred hybrid | Avoids repeated expensive lighting on overlapping surfaces; tile-local native implementations may reduce external bandwidth; requires full material/transparent/depth/media/velocity mapping | **Qualified challenger** if high light overlap or overdraw defeats forward. No universal tile-memory advantage through WebGPU |
| Visibility-buffer/virtualized geometry + material resolve | Defers heavy shading and can reduce geometry submissions; needs cooked clusters, coverage/deformation, derivative reconstruction, streaming and supported material fallback | **Later provider.** R0/R2 loads alone do not justify its residency/toolchain burden |
| Stochastic direct lighting + denoising | Caps shaded light samples, handles unpredictable clumping; pays reservoir/ray/proxy/history cost and moves error into variance, bias, temporal stability | **Challenger for a wider many-light workload.** Does not turn a 32/8 cluster overflow into permission to silently discard lights |
| Ray-traced/path-traced primary transport | Unified visibility and potentially richer indirect/transparent paths; needs complete geometry/light/material representation, sampling/denoising and acceleration updates | **Reference/high-end optional path.** Not a proven browser/mobile replacement; sample count alone cannot bound image error |
| Sorted/depth-peeled bounded layers | Preserves event order with fixed memory/pass caps; depth ties, refraction coverage, further layers and multiple interfaces need explicit handling | **Recommended initial two-event compositor** below; wider water retains its separate four-event profile |
| Weighted/voxel OIT | Useful dense particles/glass approximation with bounded storage; depth/colour mixing and overlap approximations remain | **Optional declared VFX policy**, not exact replacement for water interfaces/joint participating media |
| Authored sky + joint air/fog + cloud attenuation | Headless source coherence, bounded valid domain and no cloud-volume residency; limits artistic/physical cloud lighting | **Retain selected first air profile.** Visible cloud provider remains independently optional |
| Dynamic LUT atmosphere + 2.5D cloud shell, or voxel cloud field | Greater sky/range/camera freedom with LUT/volume preparation, lighting approximation, history and coverage costs | **Separate provider designs.** Choose only against required camera domain, source ownership and joint-overlap contract |

The decision is based on complete supported combinations and costs, not on one attractive image or one
light-loop timing. The new stochastic lighting evidence changes the challenger queue; it does not revoke
the selected bounded light/material/media model.

## Mathematical and resource model

### Scene state and extraction

V2 consumes leased committed read models, never live mutable physics/environment storage. Maintain
persistent dense GPU records indexed by stable generational instance identity, immutable cooked geometry
and material handles, and a render spatial hierarchy. A change list updates dirty transforms/products;
spawn/despawn rebuild or patch indexes at a declared barrier. A dirty-list overflow takes a bounded full
rebuild/loading path; it cannot lose an update. CPU frustum traversal/batching is the reliable initial
provider. GPU compaction is a candidate once CPU extraction or submissions justify it.

For `N` instances, `D` changed records, `V` active views and `Q_v` tested hierarchy nodes/candidates,
preparation is approximately `O(D + ΣQ_v + batches + shadow candidates)` rather than reconstructing
`N` heap objects for every pass/view. Worst-case `Q_v=O(N)`, so dirty lists are not a claim of sublinear
visibility. Shadow/reflection visible sets have their own conservative support; main-view culling
cannot remove their producers. GPU lists reduce CPU draws but add compaction, indirect buffers,
overflow handling and possible readback latency. Counts used for diagnostics may read back slowly;
gameplay does not depend on asynchronous visibility counts.

Projected geometric error is `e_px≈f_px e_m/z` in perspective and `e_px=s_px/m e_m` in orthographic.
LOD thresholds include silhouette/alpha coverage, normal error and shadow footprint, with hysteresis.
For foliage displacement bound `d_max`, culling bounds expand by `d_max`; use the same support in
colour/depth/shadow. Terrain morphing includes the morph term in previous position and bounds. A
single main-camera LOD may not serve an unrelated close secondary camera or offscreen light caster.
Share the product table; choose per-view or conservative union detail under the budget.

RENDER-SPEC's 192-byte ObjectRecord gives 786,432 B = 0.75 MiB for 4,096 instances; an unchanged full
upload at 60 Hz is 45 MiB/s before driver staging. This is small bandwidth but 245,760 record visits/s
per repeated CPU walk; several walks, allocations, sorts and two views can matter. Incremental storage
removes repeated preparation, while measuring the residual traversal remains necessary. Two current/prior
`mat4` palettes for 32×64 joints occupy 262,144 B = 0.25 MiB if stored as f32 matrices; retained
additional presentation times and CPU evaluator inputs are extra, not an unbounded palette history.

### Light assignment, shadows and one visibility meaning

Let `P_v` be pixels shaded, `O_v` shaded overdraw, `K_v` mean admitted surface lights, `C_v` clusters,
`I_v` light-cluster intersections and `S_l` shadow submissions for light `l`. Forward work is approximately
`Σ_v O_v P_v K_v` BRDF-light evaluations plus assignment `O(N_l+I_v+C_v)` and shadow geometry/fill.
The depth prepass pays vertex/raster/alpha fetch again to reduce costly opaque shading; sparse masked
foliage or tile hardware can change that balance. Compare whole pass traffic and energy before disabling
it or universally forcing it for every material.

Recommend fixed screen tiles and depth bins with an explicit near/far domain: logarithmic perspective
bins `z_i=n(f/n)^(i/Z)`; linear orthographic bins `z_i=n+i(f−n)/Z`. Admit point spheres and bounded
spot cones conservatively. A depth range derived only from opaque hits is insufficient for a transparent
surface in an otherwise empty tile or a fog sample ahead of the opaque hit. Retain full admitted spatial
bins for those consumers, or build separate conservative lists. Surface and media light participation
remain distinct: 32/8 surface lights per cluster versus Sun+4/2 local media lights in the current recipe.

Illustrative R2 720p, 32×32 tiles and 16 bins: `40×23×16=14,720` clusters. At 8 u32 indices plus an
8-byte count/offset per cluster, storage is 588,800 B ≈0.56 MiB; at 32 indices it is 2,001,920 B
≈1.91 MiB. These are proposed layouts/counts, not selected grid tuning. Prefix/compaction scratch and
overflow flags add cost. Every limit needs a diagnosed producer/admission failure or an admitted
alternate, never shader-side truncation. All-light overlap has `I≈N_l C`; a big grid is no cure.

Sun maps, local-light maps, cloud attenuation and ambient occlusion are different products. Conventional
shadow-map projection uses the same masked geometry cutoff and deformation as the material pass.
Local shadow-caster counts/resolutions and Sun cascade recipe still need concrete workload allocation;
128 local lights does not mean 128 shadow maps. V3 combines binary/geometric visibility, source-ray air/fog
and cloud transmittance **once each**. Screen AO modifies the selected indirect term only. A foliage
canopy visibility approximation cannot also multiply a shadow that already included the same crown.
Either it replaces a declared unresolved component or its overlap is partitioned explicitly.

### Ordered colour, media and bounded layers

The existing scene basis remains linear Rec.709/D65 with source photometric RGB; E1 radiometric heat
is separate. For view segment `[a,b]`, `L_a=S_ab+T_ab L_b`,
`T_ab=exp(−∫σ_t ds)`, `S_ab=∫T_as j(s)ds`. Near/far composition is associative:
`(T_n,S_n)∘(T_f,S_f)=(T_nT_f,S_n+T_nS_f)`. It is not commutative. Overlap combines coefficients
before integration and evaluates source lighting through the same total light-path extinction.
Independent atmosphere/fog/cloud image pairs are not exact overlap operands. This is the governing
operator already selected in C08, not a proposed second compositor.

For nonnegative medium terms, coefficient errors satisfy `|δT|≤|δτ|` componentwise; a local segment
colour bound is `|δL|≤|δS|+|δT||L_behind|+T|δL_behind|`. Thus an early-exit `T<ε` has an absolute
error guarantee only if omitted source/background radiance is bounded. A bright Sun/emissive source
can invalidate a transmittance-only stop. Midpoint integration of a sufficiently smooth source has
global `O(h²)` error, but discontinuous shadow/volume boundaries require explicit splitting and
cannot inherit that bound. The specified 32/16 sample caps and 2%/5% non-edge luminance targets remain
predictions to falsify, not a proof that every admitted sharp-light case fits them.

**Proposed first bounded compositor:** preserve nearest opaque/masked depth/colour; peel at most two
transparent interface events per camera ray with deterministic depth/primitive tie handling, and
detect a third event. Depth peeling repeats geometry submission but uses ordinary raster/depth resources
and has a finite pass/storage bound. Keep surface depth separate from opaque/submerged receiver depth.
Shade the far event, integrate its preceding/following medium segments, then shade the near event and
front medium. A premultiplied surface event is `L=E+α L_surface+(1−α)L_behind`, with `E` the separately
premultiplied emission and `L_surface` excluding that emission, for the declared nonrefractive alpha
approximation. Dielectric water instead uses WATER-OPTICS's Fresnel/index/medium operator. Its changed
ray direction means raster-camera depth peeling alone cannot enumerate the next refracted interface:
continue the bounded regional interface/geometry query along that branch and charge its events to the
same optical budget. The raster peel is a candidate/coverage accelerator there, not first-hit proof.
Content admission bounds layers, but runtime diagnostic coverage must still detect a dynamic
violation. Overflow refuses that promised optical mode or selects its explicitly authored fallback.

Do not allocate a universal deep image or silently use whole-object sorting when surfaces intersect.
The implementation must choose whether to store/re-shade bounded event data and report its actual
pipeline/pass closure. Fully transparent no-op fragments and binary cutouts have explicit classification;
cutouts participate in opaque depth/shadows rather than consuming an optical event. Tie handling cannot
merge distinct media boundaries merely because their depths quantize together.

**Water interface:** the selected wider design already supplies a regional rigid triangle/bed BVH,
support masks, explicit CoveredMiss/Incomplete/Uncovered statuses and a shared surface evaluator.
Its Snell/Fresnel ray branch obtains an actual submerged receiver or a declared missing-data fallback.
Screen depth is an accelerator/candidate, not proof of the first refracted hit. Physical contact surface,
visual residual, optical intersection and caustic source share location/time/error products. Single-layer
screen water can be a labelled cheaper provider, but an unknown submerged receiver cannot become black
or a generic sky probe. Reflected and transmitted histories require hit identity/confidence separately
from the moving interface's motion vector. Caustics replace the through-water direct-light partition;
ordinary Sun is not added to focused Sun. The wider four-interface profile remains a separate admission.

The R3 baseline must explicitly choose its supported camera domain, water optical material/visibility
product and two-event behavior before implementation. Water physics being selected does not settle
those choices. The wider WATER-OPTICS design is a concrete candidate; a cheaper R3 mapping must retain
its declared cues and label omitted thickness/caustic/refraction behavior. This is a **core mapping gap**,
not a claim that all wider water optics lacks a mechanism.

### Temporal samples and display

For material coordinate `q`, instance transform `M`, deformation `D`, view `V` and projection `P`,
`x_v(q,t)=pixel(P_v(t)V_v(t)M(t)D(q,t))`. Motion is
`m=x_v(q,t_history)−x_v(q,t_current)` in current render pixels, top-left origin, unjittered.
The history sample is the image accepted by this view/provider, not simply the preceding tick or host
callback. Each view owns its identity, projections, origin, dimensions, sample times, jitter and
pre-exposure. Two cameras with the same quality profile still have different histories.

Retain bounded evaluator inputs/previous pose products until the per-view sample horizon; recompute
analytic wind/waves at the exact old time with the old parameters. Changed wind forcing cannot be
retroactively substituted into old deformation. Missing material-point correspondence, topology/LOD
switch, camera cut, correction, origin convention, product/source/device/profile change or excessive
sample gap invalidates affected effect and display histories. Partial screen history rejection can
avoid whole-view resets only with conservative coverage and unchanged correspondence elsewhere.

V4's accumulation is an estimator such as `H=(1−α)H_old(x+m)+αC`; choose/clamp weights from confidence,
depth, coverage and shading changes. The exposure ratio `p_current/p_history` rescales valid history;
it cannot repair stale light/geometry. Reflection/refraction and transparent material animation may
require reactivity even with correct geometric motion. A spatial path remains valid and affordable when
history is unavailable. With temporal reconstruction off, omit velocity/history attachments and prior
GPU palettes **only if no retained consumer needs them**; correctness of E1/P4 state is unaffected.

Keep one V4 exposure/display authority. Pre-exposure for storage is metadata, not a second camera
adaptation model. UI composes under an explicit display/scene policy, after physical scene transport;
it does not accidentally drive exposure. HDR/P3 output is a separately admitted transform, not a change
of scene units. Vendor frame generation is outside the selected baseline and cannot be counted as new
world updates or used to hide input latency.

## Combined cost account and optionality

All estimates are logical texel/record bytes, not physical residency. Resource padding, tile compression,
driver/browser allocations, Wasm memory and unified-memory sharing require separate measurements.
The single whole-scene planning envelopes remain BASELINE-SPEC's desktop CPU/GPU 5/10 ms, browser
8/16 ms, mobile 7/16 ms and 3 GiB/512 MiB/1 GiB memory. This study does not allocate those budgets twice.

| Illustrative resource at 1280×720 (`P=921,600`) | Logical bytes / MiB | Scope and avoidable multiplier |
| --- | ---: | --- |
| RGBA16Float scene colour | 7,372,800 / 7.03 | Per active view; double buffer/scratch is another allocation unless lifetimes permit reuse |
| Depth32Float opaque depth | 3,686,400 / 3.52 | Per active view; optical layer depth must not destroy it |
| RG16Float geometric motion | 3,686,400 / 3.52 | Optional only with every consumer absent; extra confidence/previous depth may need more channels |
| Two R8 reactive/confidence masks | 1,843,200 / 1.76 | Can pack if format/consumer semantics agree |
| Two RGBA16Float temporal colour histories | 14,745,600 / 14.06 | Per temporal view; persistent, not transient reusable while sampled |
| One history depth R32Float | 3,686,400 / 3.52 | Per temporal view if selected |
| Joint air/fog `(T_rgb,S_rgb)` as two half-resolution RGBA16Float textures | 3,686,400 / 3.52 | Only when media retained; no mandatory medium history in first profile |
| Each full-resolution 16-byte optical event record | 14,745,600 / 14.06 | Two events alone 28.13 MiB; classified tiles/packed events are alternatives with explicit overflow |
| One 2048² depth32 shadow map | 16,777,216 / 16.00 | Every cascade/light/view-specific copy multiplies; sharing needs common support/validity |
| One 256²×128 RGBA8 cloud macro level | 33,554,432 / 32.00 | Every retained level/transition adds; absent from authored attenuation baseline |

The illustrative temporal image set (colour, opaque depth, motion, two masks, two colour histories,
history depth, half-resolution medium pair) totals **36.91 MiB**. Its histories belong to persistent GPU
products; the remaining **19.34 MiB** is image/transient work before optical events, shadows, output,
reconstruction scratch or any VFX. Do not compare all 36.91 MiB only to the browser's transient 51.2 MiB
share. Two full-screen event records would bring that transient subtotal to **47.46 MiB**, leaving
only **3.74 MiB** of that planning share. A compact coverage layout, re-shading, serial reuse or an explicit cheaper mode
is required before claiming the combined browser profile fits. A four-interface wider water path cannot
allocate four unrestricted event images and assume the reserve absorbs them.

At 1080p, image footprints scale by 2.25. R1 secondary half-width/height adds 25% of main-view image
storage while retained, but averages 12.5% main-view pixel shading at every-other-frame cadence.
Both views' histories remain live even on a skipped frame. Geometry extraction, shadow support and
shared LUTs do not automatically scale by 12.5%. Share only scene/light-direction products whose
camera support and source epochs match; independent captures keep their own cost.

An externally materialized G-buffer of `G` bytes/pixel costs at least `2GP` bytes for one write and
read before compression. For illustrative `G=16` at 1080p this is 66.36 MB/frame, 3.98 GB/s at 60 Hz,
before depth, lighting output, overdraw or multisampling. This lower-bound traffic motivates the
portable forward baseline; it is not proof that deferred loses on every target. An extra RGBA16Float
full-screen copy at 1080p similarly transfers at least 33.18 MB/frame read+write. Avoid unconditional
water/refraction copies when no water consumer exists.

The first half-resolution medium image has 518,400 desktop rays at 1080p; 32 samples and Sun+four local
lights imply up to 82,944,000 sample-light evaluations per active main view, plus coefficient/light-ray
intersection and upsample. R2 720p/16 samples/Sun+two locals implies 11,059,200. Boundary splitting,
adaptive sampling and empty support can reduce actual work, but a cap is not a timing. Optical water,
source-shadow preparation, foliage alpha overdraw and the second view must share the same frame envelope.

| Configuration | Retain | Remove/retire after leases and submissions |
| --- | --- | --- |
| Headless | E1/P3 gameplay queries and source facts, required cooked CPU products | Device/surface, all render targets, shader/pipeline preparation and extraction; no hidden GPU dependency |
| Visible clouds off; cloud attenuation needed | Authored or selected E1 tau source, V3 receiver/light-path sampling | View march, cloud colour/depth/history, cloud-only density/noise/lighting volume if the retained source does not need them |
| All cloud effects off | Explicit clear `T_cloud=1` or admitted alternate source | Tau simulation/map builds, cloud GPU textures and associated pipeline closure |
| Water graphics off; buoyancy remains | CPU physical domains/queries, causal gameplay state and ledger | Optical BVH GPU copy if unused, surface draws, caustics/reflection captures, optical histories/residual work |
| Vegetation graphics off | Physical blockers/damage/required plants as game semantics demand | Cosmetic instances, alpha/shadow draws and visual deformation buffers; cannot remove authoritative occluders |
| Sky draw off; illumination remains | Explicit environment/probe/constant light provider and Sun facts | Sky image pass and visible-sky-only resources; illumination does not vanish |
| Temporal reconstruction off | Spatial AA, current geometry and any independent effect history | V4 colour/depth histories and unneeded motion/prior buffers; retain effect-specific history only for declared consumers |

Build exclusion should remove unreachable cooked/pipeline demands. Load selection avoids initial allocation.
A live switch may temporarily overlap old/new resources, then retires at completion fences; hiding a pass
is not zero residency. Source/collision/gameplay changes retain the existing restart/stop-load policy.
Optional diagnostics allocate readback/capture buffers only while invoked. A deferred work queue is not
a resource saving until cancelled or completed and released.

## Complete operating cases and creator path

1. **R0 initial boot and edit.** F3 admits immutable source; F4 cooks one camera/material/mesh/light/UI
   closure; V1 verifies ABI, limits and required finite tuples before view readiness. W1 ticks once,
   V2 extracts owned records, V3 draws depth/shadows/forward and constant environment, V4 spatially
   resolves/displays. A source edit creates a new pinned run after stop/load; required pipeline failure
   preserves the previous ready run or displays diagnosed loading. No environment/skin/media work runs.
2. **R1 normal dual-view frame.** Commit tick state and product generations; P4/E1 provide evaluable
   presentation samples. V2 patches shared scene records, then separately culls the main/secondary
   cameras and required shadow/reflection support. Shared Sun/environment products prepare once if
   valid for both. V3 lights and composes each active view; V4 reads that view's last accepted history.
   On the skipped secondary frame its sample time does not advance. An expired previous pose forces
   reset on resumption; it does not pin the whole old world. Streaming/reimport publishes matching
   products at the C05 barrier and invalidates affected visibility/light/effect/image histories.
3. **R2 low-cost and suspend.** Preload bounded gameplay closure; use fewer visual instances, lower
   shadow/media budgets and qualified spatial AA if temporal working set is too expensive. Source/light
   meaning and replica gameplay remain unchanged. Suspend releases/invalidates device/surface products
   by platform policy; resume reacquires and prepares them from pinned products, resetting histories.
   Measure combined decode/upload scratch and all retained browser buffers during cold first play.
4. **R3 hull/shore/fog/cloud interaction.** E1/P3 commit one physical water query/force result and
   once-only reservoir/body reaction ledger. Terrain bed/collision/render products share source epoch.
   V2 evaluates water interface and foliage at current/prior sample times; V3 queries common Sun/cloud
   attenuation at ground, hull, water and medium samples, with per-path air/fog exactly once. The camera
   ray splits at the true water event; below-water receiver depth is not replaced by interface depth.
   Covered refraction/mean-caustic or wider focused mode uses its own admitted product and light partition.
   Physical mixed-model hull crossing remains blocked even if a visual seam blends. Reversing an absorbing
   fog slab relative to an emitter must change the image; merely fogging each feature's final colour fails.
5. **Top-down cloud-shadow-only scene and removal.** E1 headless source remains while visible cloud
   passes/volumes are absent. The same source revision moves attenuation over terrain, plants and water,
   independent of camera position. Switching to clear removes tau work only if no retained consumer needs
   it; F2 validates a replacement and V3/effect/V4 histories invalidate together. Two camera exposures
   may differ, but source intensity/tau cannot change with exposure.

**Creator journey:** ordinary versioned source identifies meshes, material alpha/shader closure, light
descriptors, cameras, plant placement/response recipes, terrain geometry/bed and optional environmental
source providers. F3 validates source; F4 cooks tangent/LOD/coverage, geometry and shader/pass dependencies
into immutable target products. W2 opens required gameplay products together; V1 prepares GPU products
and reports declared/cooked/delivered/CPU-verified/device-ready separately. T1/T2 inspection must trace a
pixel/pass/resource back to those IDs and show missing/unsupported demands. Proposed author-facing tools
are unbuilt; `./dev` commands are contributor checks, not an engine SDK.

A changed terrain cell rebuilds its affected collider/bed/render/placement products; a presentation-only
texture change can use an admitted visual preview generation without changing contact. A plant wind
recipe change invalidates old deformation unless the old recipe/time inputs remain evaluable. A shader
or provider edit recomputes reachable tuples and overlap before publication; failure leaves old valid
products active. Save reload restores physical/source state, not GPU cache identities or V4 histories.

## Broadsides reconciliation after target derivation

Inspected read-only on 2 November. Historical main checkout was `02dc688556263b34e74d4a468328d8e25e031690`.
The relevant **Rust** candidate is `tmp/worktrees/engine-rebuild` at
`49a6bc78a2385a18f5d57c8fb15a5b4fa7c30cc5`, clean at inspection. Cloud variants include clean
`cloud-core` at `f4a5ef1fa0ed1584286c706e0a85470b0b18fd84`, `clouds-proof` at
`d81f29611d90b42cf6e2d7a618dde7216ebaa16a`, and **dirty** `clouds-slice1` at
`4db503d1e975053c928545a1bbb99ab64e5f7e3c`. Dirty slice1 files below are current working bytes,
not proven contents of that HEAD. Broadsides tests/captures were not rerun and do not qualify Thrum.

Paths in the table are relative to `/Users/ryannel/Workspace/Broadsides/tmp/worktrees/engine-rebuild`
unless a different worktree is named. These are inspected code locators, not build dependencies or
instructions to copy packages. Licensing/provenance requires separate audit before any code reuse.

| Actual producer/preparation/consumer path inspected | Fit with the derived target | Retain/redesign disposition |
| --- | --- | --- |
| `crates/gpu/bw_gpu/src/pass_list.rs`: `PassDecl`, `Stage`, `TargetKind`, `PassList::compile`; `target_pool.rs` | Pure profile compilation checks producers/order, culls output-dead work and separates transient, persistent-per-view and shared resources. This is much closer to C06 than the historical TS stage callbacks | **Retain principles/comparison cases.** Thrum needs typed formats/subresource uses, external side effects and product epochs. Reusing compatible textures is not proof of arbitrary memory aliasing. Hard-coded forest/sea profiles and a fixed cloud/water sequence are not the generic public contract |
| `crates/gpu/bw_gpu/src/lib.rs`: `Renderer`, `BeginRenderer::finish`, `Renderer::encode` (~675), `Renderer::cut`; `pipelines.rs`; `lifetimes.rs` | One renderer records into host encoder; prepared pipeline list, counters, target-identity history invalidation and persistent resources are real code. Host retains device/submission responsibilities | **Retain ownership/lifetime ideas.** Pools are found by `ViewProfile` (~692); two cameras sharing a profile need distinct view identity/sample history. Texture identity alone cannot validate source/content/time. Thrum F4 finite demand/device-generation admission is stronger than a local warm list |
| `crates/host/bw_engine/src/scene_renderer.rs`: start/apply/reconfigure, module construction and replacements | Reuses scene modules, changes geometry/atmosphere/post inputs and cuts history on changes. Real Rust path supersedes old TS per-frame-array inventory | **Redesign around C02/C05 leases and exact source/build generations.** Direct CPU geometry/settings replacement is not atomic admitted world/product activation |
| `crates/render/bw_render_surfaces/src/flora.rs`: persistent vertex/index/material/instance upload, GPU `cs_cull`/`cs_draws`, camera/cascade/probe lists; `shaders/flora.wesl` | Persistent instance storage, indirect draws, stable LOD chains and alpha-tested depth/shadow/color are good cost/coverage structures. Source explicitly says `no wind sway yet, stub(flora-wind)` (~31–33); previous placement is rigid (~304) | **Retain the storage/coverage approach; do not claim two-view deformed wind proof.** Add shared wind evaluator/current-prior time parameters, deformed bounds and camera-ID history under C07. Fixed camera/cascade/probe list slots do not establish arbitrary view capacity |
| `crates/render/bw_render_surfaces/src/floor.rs`, `scene.rs`, `stream.rs` | Layered colour/normal-height texture arrays, linear weights and persistent upload support terrain appearance; `MAX_LAYERS=4` is a local bound | **Retain format/cost examples, replace admission semantics.** Silent excess-layer clipping/default weights cannot stand in for Thrum cook validation; ground material does not establish one terrain/collider/water-bed owner |
| `crates/render/bw_render_water/src/lib.rs`, `block.rs`, `scene_reflection.rs`; water current/prior phase upload (~426) | Water shares scene depth/velocity/HDR and reflection products; reversed-depth writes and previous analytic phases are concrete. It is an ocean-focused surface path | **Useful comparator, different complete problem.** Match Thrum physical/query surface, covered underwater receiver and media segmentation first. A scene-colour copy plus water depth cannot establish complete refraction under offscreen/hidden geometry |
| `crates/render/bw_render_clouds/src/lib.rs`, `module.rs`, `twin.rs`; cloud-core equivalent | Engine candidate uses 96 screen/48 sky steps, six near light taps and temporal fresh weight 0.1; cloud-core introduces profile volume and fitted lighting/two-stream approximations | **Research comparator only.** These are a richer camera/cloud provider than R3 authored attenuation. CPU twin agreement can test packing/operators but does not independently validate missing light physics or accepted visual quality |
| Clean `clouds-proof/crates/render/bw_render_clouds/src/production.rs`: `ProductionCloudModule`, fields/inputs, pass declarations (~649), prepare/encode; `notes/research/clouds-weather/PRODUCTION-PROOF.md`, `PRODUCTION-PERFORMANCE.md`, `PRODUCTION-REVIEW.md` | The 31 October authored production path is a distinct newer provider: one 128³ source group, deterministic 900 s lifecycle, bounded four-term lighting, independent material history and common current extinction for camera/shadow/planar/environment. Recursive angular solver is absent on this ordinary path | **Current bounded running comparator, not Thrum qualification.** Reported default cloud textures are 51.3 MiB logical (56.5 higher lighting), plus shared/staging/driver costs. Latest production notes leave appearance/lifecycle acceptance open and native window presentation unqualified because the session supplied no drawable. Flat mirror proof water has no direct beam/glint/body light, and its far strip exceeds qualified cloud-shadow support. This is not the dirty macro candidate |
| Dirty `clouds-slice1/crates/render/bw_render_clouds/src/macro_volume.rs`, `MacroVolume`/`MacroEncoding`; associated field/screen shaders listed by status | One RGBA8 macro level packs density, density×water×type, density×water and distance; reconstructs type/water after filtering; resource creation/build separated. A 256²×128 level is 32 MiB and filter guard is explicit | **Retain the premultiplied-field/filter-validity idea for study.** Quantized distance guards, scroll validity, replacement and two-view sharing need proof; a cloud-only level must not become mandatory browser residency. Do not call dirty candidate design reports executed evidence |
| `crates/render/bw_render_post/src/taa_plan.rs`, `taa.rs`; `bw_gpu/src/lib.rs` scene velocity definition (~88) | Pure TAA parameters/reset planning, jitter and previous-depth/disocclusion are real. Velocity encodes UV delta plus previous reversed depth/validity in RGBA16F | **Retain input discipline, adapt at boundary.** Thrum C07 uses current-render-pixel motion; explicit conversion required. TAA before `Stage::Transparents` is one policy with different particle behavior, not the complete provider-neutral layer contract |

The [earlier reconciliation](../../evidence/integration/BROADSIDES-MODULE-RECONCILIATION-2026-11-02.md)
remains a record of the historical TS inspection and dated visual reports. Its TS placement-allocation,
duplicate exposure and reported 845–918 MiB findings must not be asserted as current Rust defects or
current Rust memory measurements. Conversely, Rust structs/modules and existing tests do not prove the
assembled clouds/forest/water image, Thrum identity/lifecycle fit or named-device performance. The right
reuse unit is a verified algorithm, fixture or contract, not Broadsides' crate/foundation organization.

The older `clouds-proof/tmp/handoffs/2026-10-30-clouds-proof/PROGRESS.md` rejection at 31 October 09:51
concerns the preceding complete numerical proof and its appearance/lifecycle/cost. The later production
records select authored clouds under relaxed physical requirements and record actual moving output;
they do not grant user lifecycle/beauty acceptance. Keep the **historically rejected numerical proof**,
the **current authored production proof with unaccepted lifecycle and unqualified native presentation**,
and the **dirty macro-volume research candidate** as three different evidence scopes. Nothing in this
read-only inspection newly accepts or rejects the current production appearance.

## Core design gaps versus empirical qualification

| Still a consequential design decision | Owner and completion criterion |
| --- | --- |
| Initial two-event layer representation, depth/primitive tie handling, buffer formats, overflow and exact pass roles | V3/V1/F4: adopt a concrete bounded compositor and enumerate reachable material/pass demands before implementation; proposed peeling above is a candidate |
| R3 baseline optical mapping versus wider WATER-OPTICS | V3/E1/V2: name camera/support/refraction/caustic/layer envelope and complete receiver product; preserve explicit unsupported/fallback states |
| Sun/local shadow workload allocation and visibility overlap | V3/V2/E1/F2: choose local shadow subset/resolution/range/cascade support, shared canopy/cloud/geometry partition and invalidation; no accidental full lighting Cartesian cost |
| Concrete render-feature registration and removal contract | F2/V1/V2/V3: typed contribution descriptor with required input/output, pass/layout/format, colour/media/depth semantics, history owner, finite demand and off-state resource list; no public raw queue/device ownership |
| Terrain/plant geometry and source-time handoff details | W2/F4/E1/V2: source product IDs, coverage/error/previous evaluator support and affected consumers, including bed/contact/shadows; renderer may not invent a second terrain or wind |

These are not solved by a benchmark. In contrast, the selected ABI on real adapters, clustered/light
overlap cost, depth-prepass crossover, medium sample sufficiency, shadow acne/coverage, two-view ghosting,
cloud visual appeal, TAA response and thermal/memory/cold-start performance are **empirical uncertainties**
once the associated operator/domain is fixed. A failure may narrow a recipe or reopen a representation;
it does not permit changing the evaluator or calling fewer required consumers an optimization.

## Dependency-ordered proof queue and integration handoffs

Every proof pins source/build/fixture, target/backend/features, resolution, exposure and timing conditions.
Repository checks establish documentation consistency only. Runtime queues below are unbuilt proposals.

| Order / discriminating proof | Inputs, affected owners and observation | Reject the plausible wrong substitute |
| --- | --- | --- |
| P0 Contract packet before renderer implementation | F2/F4/V1/V2/V3/E1/W2 accept product/source epochs, exact view/sample IDs, light units, medium support and bounded-layer/error statuses; inspect two equal-profile cameras and feature-off dependency closure | Reject provider names without actual producer, private submit/display chains, profile-as-view-ID and a missing receiver treated as a dark miss |
| P1 R0 first complete rendered interaction | Pinned admitted/cooked mesh/material, one camera/light/shadow/UI, source edit/restart; record exact ABI/tuples, first-play feedback, resource graph and device-ready transition | Reject first-draw compilation, masked shadow loss, neutral fallback for a required missing product or a shell-only startup score |
| P2 Common environmental visibility with no cloud draw | Constant/keyframed E1 Sun/wind/tau source; terrain, one cutout plant, water patch and fog sample; two views, independent exposure, clear/source-off transitions | Reject independent clocks, screen-space-only cloud shadow, canopy double shadow and attenuation applied both to source and integrator |
| P3 Bounded mixed transport/visibility | Analytic constant media oracle plus high-sample joint reference; two overlapping fog volumes, emitter/absorber swap, water interface, offscreen submerged blocker/bed, third-event overflow | Reject independent image fog blends, a valid Newton depth hit called first-hit proof, lost layer, or failed photon interpreted as darkness |
| P4 Shared shore product and physical/render mismatch | One source terrain → render/collider/bed/plant placement; edit/restart and stale epoch injection; R3 physical graph/mouth blocked crossing with view seam blend | Reject visual-only matching height functions, swapped floor before collision, rendered displacement used for forces or cosmetic crossing of a forbidden physical seam |
| P5 Two-view deformed geometry/history | One moving skin and wind-driven plant plus water; near/far/orthographic cameras including same profile, one skipped every other frame, cuts/corrections/content changes | Reject rigid-only velocity, adjacent-tick history, main-camera-only bounds or shared exposure/history. Compare ordinary motion and disocclusions, not only stills |
| P6 Assembled R1/R2/R3 cost and feature-off ablation | Full active views/shadows/media, CPU extraction/upload, GPU passes, persistent/transient/staging peaks, cold closure and suspend/device loss; compare retained sources versus fully absent providers | Reject hidden queued work, unused allocated cloud volumes, omitted second-view storage, local wins multiplied into frame speedup or logical bytes called physical RSS |
| P7 Wider challengers only after attribution | Hold semantics/quality; compare forward/deferred for actual high overlap, CPU/GPU visibility, stochastic many-light with shadows and temporal noise, wider water/regional rays | Reject lower light count/resolution or missing transparency/media as evidence that a new complete path wins |

P2/P3 preserve the least-established cross-feature causality before large cloud/vegetation content
production. P1 remains independent of water/weather and can proceed once foundation admission permits.
P4/P5 then qualify the hardest geometry/history consumers under that common contract. Device runs use
BASELINE-SPEC's five-minute warmup/25-minute duration, ten cold launches and lifecycle cases; report
p50/p95/p99, stalls, work debt, physical and logical memory, image/motion review and thermal state separately.

Precise proposed canonical changes for the coordinating owner:

1. **C07:** require stable camera/view instance identity independent of quality profile; add current/prior
   source/product/evaluator parameter revision and material-point validity to producer handoffs. Existing
   sample-time and bounded-retention clauses remain the authority.
2. **C08/RENDER-SPEC:** select a concrete two-event storage/order/overflow policy; distinguish opaque depth,
   interface event and refracted receiver/complete-coverage status. Refer to WATER-OPTICS for the wider
   regional query and light partition; do not weaken its missing-data distinctions.
3. **V2/V3/E1:** add a source-visibility contribution table: geometric shadow, unresolved canopy component,
   cloud tau, air/fog, water and indirect occlusion each have one owner and nonoverlapping role. Keep
   surface versus volume light membership and transparent-empty-tile behavior explicit.
4. **RENDERING:** reconcile stale statements that backend/material closure/numeric budgets remain open with
   RENDER-SPEC/BASELINE-SPEC's selected baseline. Preserve open target qualification and specialized-provider
   choices; do not imply a renderer exists.
5. **F2/F4/V1:** use one typed feature contribution/removal descriptor for pass demands and off-state
   resources; report per-view persistent histories separately from transients, and browser optical event
   storage against the combined peak before advertising R2+R3.
6. **BUILD-PLAN/evidence navigation:** distinguish current Rust Broadsides candidates from dated TS reports,
   keep P1 R0 unblocked by optional environments, and place common source/visibility and R3 optical mapping
   ahead of broad clouds/forest construction. Research completion changes no runtime maturity dimension.
7. **C06:** make resource declarations include view/world scope, device generation, exact format/usage,
   subresource range, first initialization, external import/retirement and observable side effects so
   output-dead pass culling cannot remove a required cache/query product. Persistent histories require
   owner validity beyond texture identity; imported resources cannot be reused until all relevant
   submissions and CPU leases complete. Portable pooling only reuses compatible whole resources; native
   heap aliasing is a separate backend facility with its own hazard proof. Preparation callbacks check
   product/profile/device generations before publishing handles; diagnostics expose live, pending-retire
   and optional-feature allocations separately. These refine existing C05/C06 invariants without giving
   feature modules another device/queue authority.

This file alone is the research output. Integration of those proposals belongs to the canonical owners;
unmeasured quality/performance and the listed core choices remain visible until actually resolved.

## Follow-up source audit — 2024 through 2 November 2026

Added 2 November 2026 after the user requested a broader audit of recent papers, production engineering,
presentations and actual reference code. This extends the earlier selection challenge; it does not
retroactively claim the first pass inspected these materials. The sweep checked the official
[2024](https://advances.realtimerendering.com/s2024/),
[2025](https://advances.realtimerendering.com/s2025/) and
[2026 Advances](https://www.advances.realtimerendering.com/s2026/) programmes, current native-platform
talks and versioned engine paths. Selection remains the portable clustered-forward baseline, with the
cost/quality and completeness qualifications below. Broadsides remains read-only historical/candidate
context; accessible external code is separately identified and is not being adopted as a dependency.

### Recent paper and production-presentation findings

PDF page numbers below are one-based. “Speaker notes” means inspected text embedded in the deck,
not a transcript obtained by watching the recording. Title/abstract-only discoveries are excluded
from algorithm evidence.

| Source, publication/version and exact inspected locator | Maturity, substantive finding and decision consequence |
| --- | --- |
| [Cao, NanoMesh / Seamless Rendering on Mobile](https://advances.realtimerendering.com/s2024/content/Cao-NanoMesh/AdavanceRealtimeRendering_NanoMesh0810.pdf), SIGGRAPH July 2024, deck updated 10 August 2024; PDF pp. 9–15 | Production presentation notes: cooked cluster hierarchy/streaming, 32-bit hardware visibility buffer, separate skinned/masked paths, material-tile shading and conservative bone-space animation envelopes. It deliberately avoids software rasterization/atomic64. **Changes challenger scope:** a mobile virtual-geometry candidate need not reproduce Nanite's 64-bit software rasterizer. Still requires cooked hierarchy, material/pass parity, residency and valid bounds for all admitted deformations; it does not displace R0's lower-overhead raster path. |
| [Jakubowski, LucidRaster v2](https://arxiv.org/pdf/2405.13364), 1 June 2024; §§2–4, PDF pp. 3–9, 13 | Primary author preprint/prototype: primitive binning, subgroup sorting and immediate shading/blending avoid a global per-pixel fragment list. Despite the “exact” title, its bounded depth-filter description explicitly admits ordering errors and a conservative reprocessing possibility. Prototype restrictions include simple shading, resolution limits and about 1,200 MB buffers for its large scenes. **Changes quality gate:** an OIT title is insufficient; adversarial intersecting foliage/order cases, overflow and complete material integration must be inspected. No measured portable advantage follows. |
| [Lempiäinen, HypeHype 2025](https://advances.realtimerendering.com/s2025/content/s2025_stb_lighting_v1.1_notes.pdf), August 2025, v1.1 speaker notes; pp. 12–17, 23–33, 59–61; [Drobot AVBOIT](https://advances.realtimerendering.com/s2025/content/AVBOIT_SIG2025_MDROBOT-final.pdf), August 2025; pp. 48–69, 94–97 | These are the substantive recent many-light and transparency competitors already inspected above, not new proof from this audit. **Disposition retained:** stochastic lighting shifts cost into history/noise/bias and its disclosed 2025 transparent/volume extension is unfinished; adaptive extinction storage does not supply complete refractive visibility or joint lighting. Their production status cannot certify Thrum's supported assembly. |
| [Wang et al., Real-Time Level-of-Detail Rendering with ReSTIR](https://research.nvidia.com/labs/rtr/publication/wang2026levelOfDetail/wang2026levelOfDetail.pdf), SIGGRAPH July 2026; §§4.1–4.4, §5 and §6, pp. 4–6 | Primary paper: persistent UV correspondence, matching for overlapping charts and Jacobian adjustment permit reuse across topology-changing LODs; auxiliary UV BVHs perform inversion. Evaluation uses Falcor/RTX 3090. Consistent UV mappings are required, degeneracy harms correspondence, and the implementation uses one LOD per object/frame. **Changes C07 refinement:** LOD change need not always reset if a provider proves correspondence; without such a mapping, reset remains correct. This is not proof for changing water topology, arbitrary remeshing or different per-ray LODs. |
| [Craig, Upgrading PSSR](https://www.advances.realtimerendering.com/s2026/content/siggraph2026_advances_dcraig_v1_0_publish.pdf), SIGGRAPH July 2026, published deck v1.0 updated 13 August; speaker notes pp. 27, 39–43, 53–68 | Production presentation: replaces direct colour prediction with predicted filter weights, leaving explicit colour/history arithmetic outside the quantized network; uses input-space elliptical filtering and colour differences. Its platform/model is specialized. **Changes V4 challenger criteria:** account quantization, working set, history responsiveness and information loss at high upscale factors; a learned provider is not automatically a better owner of colour/storage/blending. No PSSR integration or training pipeline is proposed for Thrum. |
| [Hable, Adaptive Tessellation and Subdivision](https://www.advances.realtimerendering.com/s2026/content/Adaptive%20Tessellation%20and%20Subdivision_7.pdf), SIGGRAPH July 2026, updated 13 August; pp. 47–52, 77–80 | Production research presentation/slides: compute-generated welded geometry can be stored once and reused across depth/material/motion/shadow passes; finite output capacity constrains tessellation factors. Its timing comparisons are RTX 3070-specific. **Changes cost account:** compare shared preparation plus retained bytes against per-pass reevaluation; include seam welding and current/prior geometry. No automatic import of its reported ratios, distance rules or buffer-overflow quality policy. |
| [Mueller, Smolder](https://www.advances.realtimerendering.com/s2026/content/AlexanderMueller_Smolder_Siggraph26.pdf), SIGGRAPH July 2026, speaker notes updated 3 August; pp. 26, 44–45, 69–71, 76–85 | Production presentation accessed from a retained local PDF. Four-volume segmented overlap and moment shadows amortize repeated work. Crucially, the shipped fog/transparency integration uses one representative integration depth after a fuller mutual-transmittance design proved impractical; it is explicitly approximate. **Changes C08 evidence:** production integration is not exact overlapping transport. Preserve the joint-coefficient baseline; make any such approximation a named provider with edge/dense-overlap/HDR limits. Wave-size specialization and moment cost need target qualification. |

Smolder's retained PDF was read from
`/Users/ryannel/Workspace/Broadsides/tmp/worktrees/clouds-proof/tmp/research/cloud-lifecycle/smolder.pdf`
and its adjacent extracted speaker-note text, read-only; SHA-256
`1db757c81568181f812a279d97f42fb2ce691b2018f5111e407ef1cb72316268`.
The web reader rejected the 30 MB download, so this provenance matters. No retained deck was copied into
Thrum. The 2026 index's “fully integrated” wording is reconciled with the actual shipped approximation
in the notes, rather than treated as a guarantee of exact composition.

### Actual talk transcripts and engineering accounts

| Source/date and inspected interval or passage | Evidence maturity, inference and effect |
| --- | --- |
| [Apple, Discover Metal 4](https://developer.apple.com/videos/play/wwdc2025/205/), WWDC June 2025; official transcript chapters 4:03–13:52, Manage resources/Compile shaders | Actual official video transcript read, video not watched. Residency sets, explicit stage barriers and dedicated compilation contexts reinforce V1 ownership and device-specific preparation. **C06 refinement:** CPU scheduling or a pass label alone cannot imply GPU hazard ordering/residency. These native facilities do not establish corresponding WebGPU exposure; isolate them behind the same semantic contract. |
| [Apple, Go further with Metal 4 games](https://developer.apple.com/videos/play/wwdc2025/211/), WWDC June 2025; official transcript 2:13–7:17 and 19:25–26:08 plus embedded host-code examples | Actual transcript/code snippets read. Current MetalFX supports dynamic input dimensions and optional reactivity hints; it warns against reusing another upscaler's tuned mask. Denoised upscaling needs auxiliary signals beyond ordinary colour/depth. **Changes V4 adaptation:** supply provider-neutral semantic coverage/confidence, then convert to provider-specific masks/units; temporal provider changes invalidate history and reserve their actual products. Dynamic resolution does not waive the quality floor. |
| [Apple, Build real-time neural rendering pipelines](https://developer.apple.com/videos/play/wwdc2026/359/), WWDC June 2026; official transcript 2:16–9:57 and 9:57–13:40 | Actual official transcript read, not a watched demonstration. Redshift Live integration stresses noise-free auxiliary inputs; custom networks share command-buffer scheduling and resources. **Challenger update:** neural reconstruction is an active native option, with input-generation/encoding cost and target restrictions. It supplies no WebGPU/mobile portability proof or reason to place a second display/colour authority inside E1. |
| [JMS55, Realtime Raytracing in Bevy 0.20](https://jms55.github.io/posts/2026-09-18-solari-bevy-0-20/), 18 September 2026; MIS in Solari, Unified ReSTIR and denoising discussion | First-party engine-author engineering post, not a benchmark independently rerun here. Explains why testing old-frame reuse against the current TLAS caused shadow lag and why retaining the previous TLAS costs memory. **Changes history cost gate:** geometry visibility history can be necessary in addition to image/pose history. A cheap reservoir/denoiser switch cannot claim equivalent quality without moving-shadow and missing-shadow review. |
| [Bevy 0.19 release engineering](https://bevy.org/news/bevy-0-19/), 19 June 2026, Render Big Scenes Faster / Render Graph as Systems / Improved Skinned Mesh Culling; [0.20](https://bevy.org/news/bevy-0-20/), 8 October 2026, Solari/WESL/Mesh Shaders | First-party release explanations checked against tagged code below. **Selection unchanged:** a render-resource graph is a semantic requirement, not a mandated object-oriented “graph API.” Incremental work lists, deformed bounds and optional advanced pipelines are relevant; Bevy's reported scene timings or use of Metal are not Thrum qualification. Bevy remains a reference rather than a runtime dependency. |

### Tagged reference/engine code inspection

These are real producer/preparation/consumer reads, not just repository landing pages. The inspected
Bevy source is **v0.20.0**, released 8 October 2026. Filament's current observed release is **v1.79.0**,
observed 2 November with release record 26 October and commit
`a854ad6ac9045d211001ba597831ff878490a1de`. A historical **v1.56.0** graph snapshot is separately
labelled because current graph-file retrieval failed. Tag-specific URLs preserve the actual reviewed
version. No source was built, vendored or executed. Individual-file licence headers are evidence of
those files' licences, not a completed transitive reuse/legal audit.

| Pinned source and inspected code path | Producer/preparation/consumer result and disposition |
| --- | --- |
| [Filament v1.56.0 `filament/src/fg/FrameGraph.cpp`](https://raw.githubusercontent.com/google/filament/v1.56.0/filament/src/fg/FrameGraph.cpp), `Builder::sideEffect`, `compile`, `execute`, `createNewVersion`, subresource handling (lines 47–49, 110–178, 179–250) | Actual implementation: side effects root otherwise output-dead passes; compilation culls and derives first/last uses; execution allocates/destroys at those uses; writes have versions. **Supports C06 refinement** about imported/cache side effects, subresources, initialization and lifetimes. This is a fixed historical snapshot, not a claim that all current Filament allocation semantics were audited. Apache-2.0 header inspected. |
| [Bevy v0.20.0 `bevy_core_pipeline/src/core_3d/mod.rs`](https://raw.githubusercontent.com/bevyengine/bevy/v0.20.0/crates/bevy_core_pipeline/src/core_3d/mod.rs), `Core3dPlugin::build` (lines 93–158), per-view phase resources | Actual code uses render systems/sets and explicit ordering: completed prepass before lighting-ID copy, opaque before transparent, display afterwards. **Selection unchanged:** a general GPU graph package is optional; exact resource/pass semantics are not. Thrum's checked source/readiness and layer order must not be inferred from system dependency order alone. |
| [Bevy v0.20.0 `bevy_pbr/src/meshlet/mod.rs`](https://raw.githubusercontent.com/bevyengine/bevy/v0.20.0/crates/bevy_pbr/src/meshlet/mod.rs), `MeshletPlugin` setup/required features and preprocessing/resource-manager hooks (lines 74–93 and registration below) | Actual provider declares preprocessing, higher base overhead/material limits and `TEXTURE_INT64_ATOMIC`; current documented backends are Vulkan/Metal. **Keeps virtual geometry optional.** Compare NanoMesh's different hardware raster path rather than assuming Bevy meshlets prove a WebGPU implementation. No high-level feature claim is transferred to R2. |
| [Bevy v0.20.0 `bevy_pbr/src/contact_shadows.rs`](https://raw.githubusercontent.com/bevyengine/bevy/v0.20.0/crates/bevy_pbr/src/contact_shadows.rs), settings/required prepass and `prepare_contact_shadows_settings` (lines 21–51, 108–128); [Filament v1.79.0 `ShadowMapManager.cpp`](https://raw.githubusercontent.com/google/filament/v1.79.0/filament/src/ShadowMapManager.cpp), allocation/requirements (lines 1224–1306) | Real shadow consumers: screen contact rays assume finite thickness from 2.5D depth; Filament chooses atlas/array allocation, moments/precision, mip/MSAA and cascade/local-map footprint. **Changes validation:** screen shadows are a refinement with missing-geometry limits; map count alone is not a memory estimate. Preserve offscreen casters and count atlas/mip/filter/replacement resources. |
| [Filament v1.79.0 `shaders/src/surface_shading_lit.fs`](https://raw.githubusercontent.com/google/filament/v1.79.0/shaders/src/surface_shading_lit.fs), `computeDiffuseAlpha`, `computeKernelRoughness`, `normalFiltering`, `getCommonPixelParams` (lines 3–41, 45–132) | Actual material preparation distinguishes masked coverage, normal-derivative roughness filtering, dielectric/metal F0 and solid/thin refraction/absorption parameters. **Material gate refinement:** silhouette alpha and unresolved normal variance need independent checks across passes/resolutions; specular shimmer cannot be solved only by a temporal blend weight. This does not select all Filament material layers or its refraction visibility as Thrum's model. |
| [Bevy v0.20.0 `bevy_core_pipeline/src/oit/mod.rs`](https://raw.githubusercontent.com/bevyengine/bevy/v0.20.0/crates/bevy_core_pipeline/src/oit/mod.rs), settings, `OitFragmentNode`, `OitBuffers`, `prepare_oit_buffers` (lines 40–79, 132–148, 203–286) | Actual two-pass linked-list implementation stores packed colour/depth-alpha/next, separates exact-sort count from allocated average fragments, shares scratch across cameras and releases large buffers with no OIT camera. Its documented capacity policy can discard fragments. **Do not inherit that policy** for required layers. Adopt the distinction between storage capacity, sorting capacity, approximation and removal; neither one average nor one “OIT on” flag proves bounded correctness. |
| [Bevy v0.20.0 `bevy_anti_alias/src/taa/mod.rs`](https://raw.githubusercontent.com/bevyengine/bevy/v0.20.0/crates/bevy_anti_alias/src/taa/mod.rs), extraction/reset, `prepare_taa_history_textures`, `prepare_taa_pipelines` (lines 308–439); [shader](https://raw.githubusercontent.com/bevyengine/bevy/v0.20.0/crates/bevy_anti_alias/src/taa/taa.wesl), bindings, reversible tone-map and clipping/reprojection (lines 8–69 and fragment body) | Real current/history textures are per camera, reset and non-reset pipeline variants are prepared, and the shader reads motion/depth and clamps history. **Supports finite closure and per-view history.** Its global frame-count ping-pong is not evidence for Thrum's independently skipped view cadence. C07's accepted-sample identity remains stronger; test skip/resume with each provider. |
| [Bevy v0.20.0 atmosphere module](https://raw.githubusercontent.com/bevyengine/bevy/v0.20.0/crates/bevy_pbr/src/atmosphere/mod.rs), warning lines 25–32; [LUT/render node](https://raw.githubusercontent.com/bevyengine/bevy/v0.20.0/crates/bevy_pbr/src/atmosphere/node.rs), `atmosphere_luts`/`render_sky`; [fog registration](https://raw.githubusercontent.com/bevyengine/bevy/v0.20.0/crates/bevy_pbr/src/volumetric_fog/mod.rs), lines 8–23, 82–98 | Actual LUT producers and separate screen fog pass exist; atmosphere documentation explicitly leaves their combined physical accuracy untested. **Keeps joint integration gap visible:** running two official engine features is not proof of correct overlap. Missing pipelines simply skip those passes in this code, which is not Thrum's required first-play readiness policy. |
| [Bevy v0.20.0 Solari `realtime/mod.rs`](https://raw.githubusercontent.com/bevyengine/bevy/v0.20.0/crates/bevy_solari/src/realtime/mod.rs), feature/prepass/previous-scene setup; [node](https://raw.githubusercontent.com/bevyengine/bevy/v0.20.0/crates/bevy_solari/src/realtime/node.rs), `solari_lighting` (lines 80–175 and dispatch sequence) | Actual alternative owns deferred inputs, optional reservoirs, previous view/scene bindings, world-cache update and light sampling dispatches; it recommends disabling ordinary shadow maps when replacing them. **Challenger handoff:** replace a declared light partition, account previous acceleration structures/cache updates, and avoid paying both complete shadow paths unintentionally. This is evidence of executable structure, not measured compatibility or whole-renderer completeness. |
| [ReSTIR-LoD `ShiftMapping.slang`](https://raw.githubusercontent.com/elite-sheep/ReSTIR-LoD/master/Source/RenderPasses/ReservoirSplatting/ShiftMapping.slang), current `master` accessed 2 November 2026, `geometricLoDShift` (lines 646 onward), reconnection mapping/visibility (lines 390–470); [author repository](https://github.com/elite-sheep/ReSTIR-LoD) | Actual code chooses current/prior LOD, queries UV correspondence, carries geometric Jacobians and retests visibility. **Supports the paper's required mechanism**, beyond a plausible motion-vector claim. This mutable branch was not pinned to a resolved commit; it is an inspection locator, not a reusable verified artifact. The separate scatter-temporal file failed retrieval, so its entire temporal integration is not code-audited here. |

### Audit consequences, costs and remaining access limits

The newer sources preserve the main architectural choice but make four acceptance conditions sharper:

1. **A render graph is a contract before it is an API.** C06 must see resource versions, writes,
   initialization, subresources and persistent/exported side effects regardless of whether execution is
   expressed as graph nodes or scheduled systems. Feature removal must reach owned storage, not merely
   remove its final display pass. The existing proposed C06 delta is retained.
2. **Topology change is a conditional correspondence decision.** Amend the earlier unconditional LOD
   reset wording to: reset when correspondence is unavailable; an admitted provider can preserve history
   through a proved map with valid source/target geometry and error. A UV label alone is insufficient.
   This does not add ReSTIR or UV BVHs to R0/R2.
3. **Bounded transparency must name two different bounds.** Sorting two events, allocating two events
   and detecting a third are distinct operations. A linked-list alternative at four average 12-byte nodes
   plus one u32 head costs `(4×12+4)P = 45.70 MiB` at 720p before resolve, counters or allocator overhead.
   This is arithmetic from an inspected record layout, not Bevy residency or a proposal to allocate it.
   Sequential views can share scratch only when their use/retirement intervals do not overlap; histories
   cannot be shared that way. The earlier two-event memory warning becomes stronger, not obsolete.
4. **Production mixed-media approximations require explicit gates.** Retain the joint-coefficient C08
   operator; a representative-depth or moment-based policy must declare approximation and transmittance
   conditioning. Removing fog through division by small transmittance amplifies numerical/input error;
   HDR emitters and near-opaque overlap need an explicit fallback. This mathematical inference does not
   claim a new defect in the shipped reference system. Water's refracted rays still require their own
   supported geometry/light-path result; no new screen-space source eliminates that obligation.

Additional gate updates: compare generated-geometry reuse against its full retained/current-prior buffer
peak; test specular/alpha coverage before and after reconstruction; distinguish temporal denoiser masks
from generic coverage; add previous visibility/acceleration products when a challenger actually needs
them. Keep test thresholds and equal-quality resolution fixed before comparing architectures. None of
the audited 2026 native hardware/neural successes establishes the browser profile or a second display
authority. No implementation or maturity promotion follows from this audit.

Access failures are retained rather than converted into invented negative findings:

- **VRRT 2026:** the [official speaker-note PDF](https://www.advances.realtimerendering.com/s2026/content/SIGGRAPH2026%20-%20Micha%C5%82%20Olejnik%20-%20Variable%20Rate%20Ray%20Tracing%20in%20COD%20MW4.pdf)
  exceeded the web reader's 12.6 MB limit. The index describes a fixed-total-ray scheduler and instability
  detection, but only that abstract was accessible; its algorithm/measurements were not inspected and
  cannot close Thrum's fixed-work proof. No suitable retained local deck was found.
- **SLIM/ORCA/Serac:** the official 2026 SLIM 20 MB PDF and ORCA PPTX, and 2024 Serac PPTX, were discovered;
  presentation formats/size prevented substantive inspection here. They remain candidate leads, not
  adopted techniques. Existing WGSL demand closure does not depend on Serac.
- **Reference code:** current Filament graph/post-process file requests returned cache misses, although
  current material/shadow files and the historical graph were readable. LucidRaster shader bodies also
  failed retrieval; the paper and repository licence/structure were inspected, not its full implementation.
  ReSTIR-LoD scatter-temporal shader retrieval failed, as recorded above. No blanket “code audited” label
  covers these missing paths.
- **Recordings:** official Apple talk transcripts were read. ReSTIR-LoD's linked MP4 was unsupported by
  the reader. No conference video was watched, and no unavailable recording is reported as inspected.

The sweep stops here because the remaining consequential baseline questions concern its concrete
layer/source/visibility handoff and actual qualification; adding more inaccessible titles would not
resolve those decisions. The source audit and code reads improve evidence coverage while preserving
the unbuilt status of Thrum's renderer.
