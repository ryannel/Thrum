# R3 optical transport and bounded transparency

2 November 2026. **Design recommendation for canonical integration; no renderer, optical
fixture, device measurement or visual acceptance exists.** This study closes a wider,
explicitly approximate regional profile beyond optical-A. It does **not** close the
actual-interface focused-light requirement for arbitrary water, occluders and falls.
That distinction is essential: complete geometry and finite event storage cannot prove
complete illumination, and conserving the photons that happened to be emitted cannot
bound unsampled caustic energy.

The commission is the remaining R3 gap in the
[build plan](../../plans/BRIEF.md), [render implementor walk](RENDER-IMPLEMENTOR-DOGFOOD-2026-11-02.md)
and [joint readiness report](RENDER-ENVIRONMENT-TERRAIN-IMPLEMENTOR-READINESS-2026-11-02.md).
Owning designs remain [RENDER-SPEC](../RENDER-SPEC.md),
[water optics](../modules/WATER-OPTICS.md), [play/environment](../PLAY-ENVIRONMENT-SPEC.md)
and [C4](../CONTRACTS.md). This file proposes changes; it does not silently supersede them.
The author read those plans before this proposal, so this is an independent derivation
of mechanisms and alternatives, not a blind architecture study. The coordinating owner
commissioned a separate focused-light challenge. Its reported thin-aperture counterexample
is reproduced below; this author's paper walkthrough is not independent validation.

## 1. Decision and capability inventory

Keep `SortedAlpha` as an explicitly approximate optional R0/R1 material policy. Select
`BoundedAlpha2` for the exact nonrefractive two-event promise. Select `R3-RegionalVirtual`
for the wider regional camera/medium capability described here. These are separate
capability decisions, not quality settings that silently change one promise.

| Profile | Selected supported outcome | Limits and status |
| --- | --- | --- |
| `SortedAlpha` | Existing grouped, stable sorted premultiplied alpha | Intersections/order and refraction remain approximate; cannot satisfy R3 exact ordering |
| `BoundedAlpha2` | Two ordered nonrefractive coverage surfaces before opaque depth, deterministic ties, detection of a third | One raster gather plus sort/resolve; qualified adapter and capacity proof still unrun |
| `R3-RegionalVirtual-D` | Above/below-water camera, actual occupied-volume interfaces, reflection/refraction/TIR, rigid/cutout/deformed opaque and coverage-alpha geometry, joint air/fog/water single scattering, one virtual-interface Sun source model | Four water interfaces and four alpha events per camera-tree path, at most two alpha events on the initial raster segment; smooth optical interfaces; finite regional geometry; source model omits wave focusing and actual-interface source redistribution |
| `R3-RegionalAbsorption-P` | Proposed portable optical floor: actual above/below-water camera, one graph/bed volume, reflection/refraction/TIR, complete rigid/cutout/deforming/coverage-alpha queries and water absorption | At most one water interface along each camera path, two alpha events, one view and spatial reconstruction; no water-scattering source evaluation/shafts/haze; no glass/folds/falls. Device support unqualified |
| `R3-RegionalVirtual-P` | Provisional portable scattering expansion, with the same query/material/medium meanings and certified source-work removal | Two water interfaces and two alpha events per path; one view, spatial reconstruction, finite source-query cap and required certified-empty coverage below. This is not an established 16 ms recipe |
| `R3-FocusedActual` | Actual-interface focused surface and volumetric caustics, source transport through thick/curved falls and folded shells | **Unselected full-family core design gap.** The separate interval challenger now specifies a conditional restricted surface-bin producer; focused volumetric image closure and practical admission remain open |
| `ParallelSheet` | Absorption-only, locally parallel finite sheet optical collapse | Conditional material specialization with explicit thickness/parallax error; does not grant underwater occupancy, arbitrary fall illumination or a focused-light solution |

The D/P suffixes identify proposed admission recipes, not measured desktop/portable support.
Neither can be required by the first small game. R3-RegionalVirtual is a complete finite
operator for its declared **source approximation**; it is not the physically complete
water-light system requested by the wider water family. If focused caustics or clear
nonparallel falls are indispensable, that family remains unfinished.

| ID / requirement basis | Observable outcome and selected producer | Proof obligation / actual status |
| --- | --- | --- |
| O1 / baseline two-layer promise | Intersecting panes order per pixel; extra events fail before display; V3 bounded gather | CPU sorted reference and overflow injection; unrun |
| O2 / water medium implication | Camera sees the correct air/water side, entry/exit, underwater Snell window and TIR; E1/F4 occupancy plus V3 ordered traversal | Closed-volume analytic fixture and boundary motion; unrun |
| O3 / complete regional visibility | Offscreen bed, dry hull, moving character, cutout and alpha pane participate in the same query; F4/P4/V2 regional product | Brute-force triangle/coverage reference; unrun |
| O4 / coupled medium requirement | Air/fog overlap jointly; water replaces air; water-compatible bubbles add coefficients once; V3 interval operator | Analytic segment and high-sample joint comparison; unrun |
| O5 / one source requirement | Cloud/air/fog/alpha shadows paid once; submerged direct light has one named owner; E1 source facts, V3 virtual-source query | Source ledger/constant-plane/crest counterexample; unrun |
| O6 / wider water goal | Actual source-interface focusing, finite-Sun caustics and nonparallel falls | No selected complete operator; not discharged by O1–O5 |
| O7 / creator and lifecycle requirement | Source edit, restart, graphics-off, two views and device restoration preserve authority and remove unused work | Product closure and lifecycle fixtures; unrun |
| O8 / performance policy | Finite whole-scene CPU/GPU/bytes, exact work removal before fidelity loss | Cost derivation below; all device constants unmeasured |

## 2. State ownership, units and geometry correspondence

W1/E1 retain physical water, source identity/time and authoritative wet support. P3 retains
contact/occupancy semantics for gameplay. F4 owns cooked geometry, material conversion,
topology certification and dependency pins. P4 supplies accepted deformation. V2 owns
derived current/prior geometry, bounds and optical spatial indexes. V3 owns optical
occupancy interpretation, materials, paths, source approximation and composition. V1
owns allocation, hazards and submission; V4 owns reconstruction/display. No optical shell,
GPU depth image or source cache writes E1 volume, contact height, P3 swimming state or W1.

All positions are metres, source quantities retain C08's photometric Rec.709/D65 basis,
sigma values are m^-1 and phase functions are sr^-1. Local GPU coordinates have one
declared origin epoch; f64 world-to-local conversion precedes f32 geometry preparation.
Interface normals point from water to air. Every query names world/endpoint pair,
presentation time, origin, topology/deformation/material/texture/profile/device revisions,
purpose, finite segment, footprint and caller error budget. A local BLAS address is never
a durable object identity. Hit identity is stable instance/part/primitive plus generation;
packet-local dense IDs are only ordering keys.

### 2.1 Physical shoreline first

The selected shallow model has **full-cell wet/dry support** and a bed/storage
reconstruction. Its physical provider must publish that exact support and reconstruction;
rendering must not infer a shoreline from interpolated alpha or a separately triangulated
terrain height. A wet cell's optical water occupies its declared vertical interval above
the provider's bed and below its accepted surface. Dry cells contain no water. A cell that
becomes wet publishes its entire selected support at the committed endpoint; interpolating
a wet polygon into a physically dry cell invents water.

V2 prepares a watertight piecewise shell from this support: top, exposed wet/dry side
faces and bed closure. Bed/side contact faces coincident with an opaque bed are topological
closures, not extra visible dielectric interfaces. Opaque bed wins that termination; no
ray refracts through the bed. Adjacent cells of the same optical material cancel internal
faces, even across stream/product boundaries. An actual height discontinuity retains its
exposed side; smoothing it needs a separately admitted mapping, not a free normal fix.
The initial regional source profile rejects clear exposed waterfall sides; camera-only
shell construction does not solve their illumination.

At dry-to-wet onset, disappearance and cell connectivity change, reset affected optical
history. Use the committed endpoint's support for boundary classification; interpolate
surface position only where the endpoint pair proves the same occupied topology. No
half-wet interpolation transfers authoritative volume. Presentation may expose the cell
onset/pop; qualifying a smoother physical reconstruction is E1's task. A screen fade may
hide output only under an explicit cue approximation and may not claim physical shore
agreement. Rain receiver ownership and P3 queries consume physical support, not that fade.

For prescribed graph water, optical occupancy extends from the same physical evaluator
to the admitted bed/depth support. A visual residual has a separately bounded surface
offset and smooth exclusion mask. Position, normal and correspondence error are recorded;
both terms in the derivative of a masked residual are included. The residual vanishes
before dry/solid/interactive exclusions. It cannot bridge a dry cell, bank or optical seam.
Foam is a coverage replacement on occupied water, never a white overlay outside support.
An unresolved bubble population belongs to medium coefficients; an explicitly resolved
bubble interface removes its population from those coefficients.

Require a per-recipe cue budget `epsilon_shore`, `epsilon_surface`, `epsilon_normal` and
minimum preserved receiver/occluder thickness. Proposed first values are 1 cm surface
offset at a body/shore and 1 degree normal error away from a discontinuity; these are
qualification targets, not implied physical solver accuracy. Full-cell shore quantization
is model error of order the cell width and cannot be hidden inside the 1 cm mesh budget.
Report it separately. Near grazing, `epsilon_position / abs(n dot ray)` can diverge;
neither this expression nor tessellation refinement proves stable first-hit order there.
Overlapping hit-error intervals require refinement or `AmbiguousOrder`.

The distinct provisional capacity-HLL provider in the
[moving-shore design](MOVING-SHORE-EXPOSURE-DESIGN-2026-11-02.md) publishes a different
reconstruction: one stage per coarse hydraulic cell, actual source bed triangles and
wet support `bed(x)<stage_i`. It must use that exact cut for optical top/bed/side shell,
P3 and rain ownership. Positive retained volume remains wet even below the provider's
momentum cutoff. Fine bed triangles do not make neighboring stage values continuous.
Retain cell identity and any actual exposed height jump. Smoothing is admissible only
with the same wet boundary, dry exclusions and a bounded physical-to-render mapping;
an excessive stage jump requires physical refinement/provider change or explicit quality
rejection. Do not apply full-cell mask semantics to this separate provider or quietly
present its finer shore as the original HLL's physical result.

### 2.2 Occupancy and camera initialization

One water material initially has index 1.333 and one declared coefficient profile.
Equal-material connected physical domains form one optical union; W2 cell boundaries
are not optical boundaries. Different materials do not overlap in this profile. E1/F4
supplies disjoint union membership and boundary face adjacency. Overlapping arbitrary
closed solids with different interiors require a separately selected Boolean/priority
model and fail initial admission.

Initialize camera medium by the exact provider support/vertical query for graph/full-cell
domains. A certified optical-only shell uses an oriented crossing test with a deterministic
non-edge test ray and an independent winding/parity validation fixture. Both methods
include a conservative boundary-distance error band. In that band return `BoundaryBand`;
refine against the owning f64 physical/support query or certified shell predicate before
publishing a medium. If it remains uncertain, fail the current optical sample rather than
retaining the last medium indefinitely. A point exactly on a certified boundary uses the
declared air-side one-sided limit; it is reactive. For a perspective pinhole all rays share
that camera membership: subpixel image jitter cannot put the camera on both sides.
If a pixel footprint straddles a visible interface farther along its rays,
resolve its separate samples and average radiance; do not interpolate index
or media membership into a fictitious material. A finite retry with a different test
direction may remove a parity-edge ambiguity; exhaustion is explicit ambiguity.

Ordinary eye-height crossing needs no physics state transition. Topology/source replacement,
camera teleport, an unsupported shell or a classification disagreement invalidates the
view's optical history. Optical occupancy classification never grants a physical swimmer
permission. An animated clear curl needs a closed, oriented finite wall with nonintersection
and thickness certification at every accepted presentation sample. Such a shell can make
camera intersection meaningful, but remains outside R3-RegionalVirtual source admission
when its clear nonparallel interfaces are exposed to the Sun.

## 3. Complete regional geometry and query protocol

The regional product contains the entire admitted optical query volume plus source-ray
and reflection support. It includes offscreen and camera-occluded geometry. Purpose masks
select blockers/receivers/interfaces; they do not classify a straddling hull as permanently
dry or wet. A dry hull can block the Sun while the bed remains a refraction receiver.

| Geometry/material class | Actual producer and admitted optical role |
| --- | --- |
| Static terrain, bank, bed, rigid mesh | F4 indexed triangles/materials, accepted W1 transform; immutable BLAS plus TLAS instance |
| Binary cutout | Same canonical alpha source, UV/deformation, cutoff and two-sided convention as depth/shadow/forward; ray footprint chooses the declared alpha LOD/filter |
| Skin/morph/vegetation deformation | P4/V5 current positions evaluated once into V2 shared geometry; depth, alpha gather, shadow and BVH read those positions; refit leaf bounds then internal bounds before publication |
| Coverage-alpha surface | Nonrefractive affine surface operator; ray query keeps it as an event, never treats it as opaque or discards it from a complete proxy |
| Water boundary | Same certified surface/shell used by occupancy and raster; oriented media sides and interface type |
| Glass, arbitrary rough transmission, participating transparent solids | Unsupported material in this profile; must convert with authored loss or reject. A coverage pane is not a glass substitute |

A deformation envelope used only for culling does not supply ray geometry. A low-detail
proxy is admitted only with position/normal/coverage/thickness/material error; changing
its silhouette is a model change, not a visibility optimization. Alpha-ray filtering can
differ from one raster pixel footprint, but both evaluate the same coverage function and
record that sampling error. Alpha mip choice cannot silently erase a protected thin cue.
No f64 fallback is assumed in WGSL. Numerically uncertain f32 triangle/edge tests return
uncertainty; a CPU high-precision retry is a future loading/diagnostic option, not an
unbounded synchronous frame-path operation.

Retain the proposed stackless linear BVH, 32-byte bounds nodes and at most four triangles
per leaf as the first implementation. Pad bounds for the admitted intersection arithmetic.
Every closest-hit request traverses all nodes that can beat its current candidate. A screen
Newton root may provide a candidate, not proof of no earlier hidden hit. A finite node or
primitive visit cap returns `IncompleteTraversal` even if a plausible candidate exists.
Dynamic refit has finite depth-ordered dispatches and a changed-generation publication
barrier; a view never sees new vertices under old bounds. Rebuild off-world when a refit
degrades traversal beyond the recipe cap; old/new overlap is reserved first. Delaying a
required deformed BLAS makes that view unavailable, not transparent.

The result is a tagged sum, not a nullable hit:

| Result | Meaning and allowed consumer action |
| --- | --- |
| `CoveredHit(hit, error)` | First hit within complete declared support; continue the selected material operator |
| `CoveredMiss(exit, terminal)` | Traversal complete through the declared segment/support, with an admitted terminal boundary; only this may reach environment radiance |
| `UncoveredExit` | Required segment left supported geometry; cannot become sky, water darkness or zero source |
| `UnsupportedGeometry/Material/Regime` | A participating class lacks the selected operator; fail or use a prepared separately accepted profile |
| `AmbiguousOrder/BoundaryBand` | Error intervals or occupancy do not identify a unique interpretation; refine/sample or fail |
| `IncompleteTraversal/EventOverflow/SourceOverflow` | Finite work cap reached; no successful hit/miss/complete-light label |
| `StaleEndpoint/UnavailableDependency` | Geometry/source versions or readiness disagree; preserve previous complete generation if still valid, otherwise loading/failure |

Store geometry coverage and optical approximation/error separately. `CoveredHit` does not
certify source quadrature, proxy appearance, missing multiple scattering or virtual-plane
lighting. A dark material is a valid value; missing support is a status.

## 4. Bounded alpha capture: select append, retain peel as oracle

The seven-walk peel proposed by the render walk is a useful correctness comparator, but
is not selected as the performance default. A fixed per-pixel append array avoids repeated
skin/deformation/alpha evaluation and seven geometry submissions. This decision is based
on work/byte counts, not measured speed.

For `BoundedAlpha2`, each allocated pixel has a u32 counter and two 16-byte records:
`reverseDepth:f32`, `densePrimitiveId:u32`, `premultRGBA:2*u32` (RGBA16F packing).
Thus fixed storage is **36 bytes/allocated pixel**, versus 32 bytes plus overflow/owner
scratch for two peeled records. Coverage tiles use 16x16 pixels and a checked tile-to-buffer
table; reserve tiles for conservative draw bounds, not last frame's visible alpha. A tile
that could receive a fragment cannot lack storage. Missing tile or counter capacity sets
the same failure status as a third event. Full-screen counters are unnecessary when the
tile domain is correctly prepared.

One raster pass performs these operations:

1. Evaluate the shared geometry/material and reject zero coverage, masked-out texels and
   fragments behind the sampled opaque depth. Compare depth explicitly before side effects;
   do not depend on late/early depth implementation choices to bound atomics.
2. Obtain a primitive key without an optional primitive-index feature using the render
   walk's vertex-pulled nonindexed triangle-list interface. Check dense prefix/count overflow
   before submission; ordinary opaque indexed rendering is unchanged.
3. `slot = atomicAdd(counter[pixel],1)`. If `slot<2`, write the unique record. Otherwise
   set the GPU view-status flag and do not write beyond the array. No float atomics or
   intra-pass ordering are required. The admitted draw count bounds total invocations
   below u32 wrap; overflow of that preflight count rejects the packet.
4. End the pass. In the later compute resolve, read the completed counter/records, sort
   by decreasing reverse depth and ascending primitive ID, and resolve far-to-near.
   A coplanar opaque hit wins. A third fragment fails the current result even if the two
   arbitrary retained records happen to look plausible.

Record writes belong to distinct slots. Resolve cannot consume them inside the append
pass: shader atomic ordering alone is not a cross-fragment storage barrier. V1 inserts
the resource/pass dependency. Adjacent triangles obey one raster sample convention and
top-left coverage rule. Coplanar duplicate authored triangles are separate alpha events;
do not deduplicate by colour. A deterministic primitive tie orders coverage surfaces but
does not repair physically incompatible coincident dielectric volumes.

For R3, raster alpha capture ends at the **nearest water interface or opaque depth** on
the initial straight camera ray. An underwater camera uses the first exit/interface the
same way. V2 prepares that first double-sided interface depth/identity from the shared
geometry. This prevents alpha behind a refracting boundary from consuming the primary
segment's two slots. Once the ray bends, V3 uses the regional BVH and its separate alpha
counter; primary raster records are not reprojected as a supposed secondary-ray answer.
The initial region can instead use BVH alpha traversal for all segments in the reference
fixture; compare those results with raster capture before enabling the accelerator.

| Alternative | Work, memory and decision |
| --- | --- |
| Fixed `K=2` append | One alpha geometry walk, bounded 36A bytes, O(overdraw) contended u32 atomics, two-item sort. Selected; extreme coplanar overdraw may serialize and still shades failed fragments |
| Seven-walk peel | About seven times alpha geometry/coverage evaluation, bounded storage, deterministic depth/owner roles; retained test/reference and possible low-overdraw adapter alternative |
| Global linked list | One gather but allocator contention, pointer chasing, average capacity differs from per-pixel sort capacity; extra fragments can exhaust global storage. No advantage justifies it for this fixed two-event recipe |
| Weighted-blended/moments | Compact accumulation and no event sort, but loses required colour/order/interface identity. Allowed only as a separately accepted approximate particle/card provider, never an automatic exact-R3 fallback |
| Region-only ray gather | No alpha raster array, naturally orders bent paths, more BVH traffic for primary alpha. Strong R3 specialization when regional geometry is already required; qualify against append using actual coverage/overdraw |

WGSL permits storage atomics in the relevant execution model, but optional features,
limits, shader validation and actual fragment-storage behavior must pass the pinned
wgpu/Naga/browser admission. Use u32 atomics in buffers; no fragment float atomics,
read-write storage textures, subgroup size or hardware ray query is required.
See [WGSL atomics](https://gpuweb.github.io/gpuweb/wgsl/#atomic-builtin-functions)
and [WebGPU limits](https://gpuweb.github.io/gpuweb/#limits). Native/mobile tile-memory
cost and overdraw contention are measurements, not guaranteed wins.

## 5. Camera path interpreter and media composition

Camera interface depth `N_I`, alpha depth `N_A`, initial raster alpha capacity `K`, BVH
visit cap, medium substeps and total ray-tree work are different bounds. For D select
`N_I=4,N_A=4`; for P select `N_I=2,N_A=2`; `K=2` in both. Reset neither interface nor
alpha path count at a stream boundary. Each child inherits its parent's counts. Query
one further event at the cap to prove a valid terminal; a nonterminal child is overflow.
The absorption portable floor instead selects `N_I=1,N_A=2`. Initial proposed traversal
caps are 512 bounds nodes and 256 triangles per query, including source queries; path
support is at most 2 km air and 32 m water. These are recipe admission values, not proven
typical costs. Report cap stress and reject ordinary workloads that repeatedly exhaust
them; increasing a cap requires redoing the whole-frame work admission.

Each path node carries ray origin/direction, medium membership, local-to-world origin
epoch, accumulated RGB camera throughput, travelled distance, interface/alpha counters,
source/profile revisions and error. Maintain a depth-first stack of pending reflected
children. Integrate a segment's source into the pixel accumulator before visiting its
next event; multiply the continuation throughput by segment transmittance. There is no
WGSL recursion and no unordered float accumulation between pixel owners.

For water-to-air oriented normal n and a camera-side medium a / far-side medium b,
compute exact unpolarized dielectric Fresnel and Snell direction from the **geometric
optical normal**. The camera radiance operator is
`L_a = F L_r + (1-F)(n_a/n_b)^2 L_t`.
TIR creates only the reflected child. Reflection preserves medium; transmission switches
membership using the boundary's explicit adjacency. The index-squared factor is not
also applied to flux or receiver irradiance. Rough normal microstructure requires a
selected angular integration closure; it cannot change Snell direction with an unrelated
normal map while retaining smooth-interface claims. The regional first proof selects
smooth filtered interfaces; a rough-transmission extension remains outside this closure.

At a coverage-alpha event, use `L=alpha*C_surface+(1-alpha)*L_behind`, where `C_surface`
is the selected surface closure before view-medium integration and alpha is coverage.
It changes no medium or direction. Shading it once and then multiplying its colour by
alpha again is wrong. On a source ray it transmits `1-alpha`, without a glass index.
At an opaque hit, evaluate the finite material closure and stop. A surface hit outside
source support cannot invent a direct-light value.

This ordering resolves a pane before water by compositing its camera-side segment first;
a pane inside water is attenuated over its actual water distance; a pane behind water
is found on the refracted ray and can be hidden by a different opaque receiver. Cutout
holes continue the ray rather than create alpha events. A deforming actor affects both
first hit and source shadow at the same accepted geometry time. These cases cannot be
implemented by compositing a pre-fogged opaque scene copy behind a screen water colour.

Within each straight segment, split at support/medium entries and exits. Water displaces
air; local atmospheric fog is air-only unless its descriptor explicitly allows the water
phase. Compatible water/bubble coefficients sum before integration. Use RENDER-SPEC's
joint coefficient/source operator and stable `expm1`/small-tau limits:
`T_k=exp(-sigma_t*ds)`, `S_k=j_mid*(-expm1(-sigma_t*ds))/sigma_t`.
Compose `(T,S)` front-to-back; sum active scattering sources with their own phase functions.
Do not independently overlay fog and water images or divide a fogged image by small T.

The selected single-scatter closure is `j=sum_i sigma_s,i*p_i(mu)*E_perp`
plus the separately authored isotropic ambient term. Both directions point away from the
sample: `omega_light` points toward the source and `omega_camera` toward the camera.
Use `mu=dot(-omega_light,omega_camera)` and
`p_HG(mu)=(1-g*g)/(4*pi*(1+g*g-2*g*mu)^(3/2))`, so positive g favors the light's
forward propagation. A library using two outward directions needs the corresponding
sign conversion. `E_perp` is incident
beam irradiance before the receiving cosine. Use water HG phase with authored
`g in [-0.85,0.85]`, nonnegative RGB coefficients and initial water single-scatter albedo
at most 0.5 with scattering optical depth at most 0.25 over admitted camera/source paths.
These are conservative initial **model admission** limits, not a proof that omitted
multiple scattering is visually negligible. More turbid water requires another model.

Use up to 32 joint medium samples/ray in D and 16 in P, split at all actual boundaries,
with the canonical delta-tau/source-variation controls. Split source changes at blockers
or refine; a small volume step does not prove smooth lighting. Exhaustion gives
`ApproximationLimit` or rejects the promised profile, not arbitrary early darkness.
Each interface child carries its own path and source direction. The full tree has at
most 31 straight nodes for four binary interface levels, or 7 for two, **before alpha
splits and segment-boundary work**. Alpha does not branch but can split each node into
up to `N_A+1` intervals. This multiplier is not hidden under “one ray per water pixel”.

Do not terminate at a small throughput without an admitted bound on remaining radiance.
An optional exact error rule may prune when `max(weight)*L_max < epsilon_pixel`, using
a versioned finite emitter/environment/source bound; otherwise all finite branches run.
A four-interface TIR cavity with no terminal fifth query is unsupported, not black water.

## 6. Selected virtual-source operator, and its deliberate losses

### 6.1 Above-mean receiver defect and source domain

The old mean-plane construction is undefined for a physically wet point above its
plane: receiver y=+0.10 m under a crest y=+0.20 m cannot follow an upward water-to-Sun
ray to a mean plane y=0. This is a model-domain defect, not a floating-point tolerance.
Optical-A may retain its mean plane only for queries with a positive forward intersection.

For R3-RegionalVirtual select instead a horizontal **cover plane** `H_cover` at or above
the conservative maximum accepted wet surface over the source region and endpoint
pair. Require positive source vertical cosine, Sun altitude at least 15 degrees and a
finite receiver/source support. A point in physical water therefore has a forward
virtual interface. Equality at the cover plane uses the continuous zero-length water
segment. Source-region height variation and cover-to-actual offset have explicit
authoring bounds; propose at most 0.25 m for the first proof, excluding larger river
stage variation or exposed clear falls. This restricts the named optical profile, not
the physical shallow solver or general engine water requirements.

The cover-plane construction is defined for both prescribed waves and the provisional
capacity-stage reconstruction. For waves E1 supplies a conservative supremum over the
whole accepted time interval (for example tide maximum plus the sum of wave amplitudes,
plus admitted residual); endpoint height samples alone cannot bound a crest between them.
For capacity water use the maximum published per-cell stage/enclosure and residual bound.
The source approximation is the same regardless of whether a cell's actual wet support
is rectangular or a source-bed cut. Capacity stage jumps remain real camera interfaces;
an exposed clear fall or a failed dry-source eligibility query still rejects the source
profile. A fine source bed does not remove those restrictions.

The virtual water ray does **not** toggle on real water interfaces; this source model
replaces them with the cover plane. It can pass through real air above a wave trough
while paying virtual-water extinction. This is a declared lighting error, not a claim
that air physically contains water. Camera rays, occupancy, shore/contact and queries
retain the actual physical/visual geometry. Outside-water Sun queries follow actual
straight air paths; if one crosses a real water interface, it is **ineligible** for this
source model and returns `UnsupportedSourcePath`, never ordinary unattenuated dry Sun.
This condition matters at stepped shores and must be tested, not assumed from the Sun
angle alone. Clear folded/sheet geometry can make it fail and is outside this admission.
For source integration the interval x-to-q is labelled virtual water throughout: apply
water-compatible coefficients there and exclude air-only fog, even where physical air
would exist. The interval q-to-external-source is labelled air and includes its actual
air/fog/cloud products. Different blocker intersections caused by refraction at q rather
than the real surface are part of this approximation. Complete BVH traversal certifies
visibility along these virtual segments only; it is not complete actual-surface source
visibility. Source-support and cloud-plane heights must admit those segments explicitly.

### 6.2 Concrete query

Let `omega_a` point from the virtual interface toward the Sun in air. For the upward
plane normal, derive the upward internal water direction `omega_w` from Snell with the
same horizontal component and positive cosine. At submerged receiver/sample x,
`s_w=(H_cover-x.y)/omega_w.y` and `q=x+s_w*omega_w`. Query the complete regional
geometry from x to q, then q toward the finite external light boundary. Ignore actual
water boundary faces **only in this declared virtual-source query**. All opaque, mask,
deforming and alpha blockers remain. Reject negative distance, unsupported range,
uncovered exit, extra material or source-ray alpha overflow (`N_A,light=4` D, 2 P).

Start with unattenuated E1 Sun perpendicular illuminance `E0`. Apply cloud optical depth
at the E1-declared source-plane/light-ray intersection, air and local-fog attenuation on
the air part, source alpha transmittance and opaque visibility once. The transmitted
beam irradiance is
`Ew_perp=(1-F_plane)*E0*cos(theta_a)/cos(theta_w)*T_cloud*T_air_fog*T_alpha*exp(-sigma_t,w*s_w)`.
There is no second index-squared irradiance multiplier. Water source-path extinction
includes absorption and out-scattering; camera in-scattering uses `sigma_s` and phase
exactly once. For a homogeneous virtual segment the lost beam fraction is partitioned
into absorption and single-scattering removal in the ratio sigma_a:sigma_s. The latter
is not all deposited at the opaque receiver. No ordinary dry Sun or separate caustic
brightness texture is added to Ew.

At an opaque/alpha water-side receiver use `f_r*Ew_perp*max(n_receiver dot omega_w,0)`.
At a water medium sample use the same Ew and omega_w in the phase/source equation.
Thus a scalar painted caustic irradiance is never repurposed as directionless volumetric
illumination. Underwater local lights are excluded initially. Air retains the canonical
declared volumetric subset and source paths; a local-light path that crosses water is
unsupported unless separately admitted, rather than assuming inverse-square through it.

Air-source queries see the actual interface point for water-surface direct reflection;
environment contains no duplicated Sun disc. Camera reflected/refracted terminal rays
use the same light registry and its finite source radiance. A chosen explicit analytic
Sun-disc reflection and an environment Sun disc are exclusive representations. Cloud
tau interpolation occurs before exp; visible clouds are absent in this profile.

### 6.3 Accuracy and energy claim boundary

The virtual source conserves its own planar beam partition locally and avoids duplicate
illumination. It does **not** conserve the complete actual wavy-surface source partition:
actual camera reflection uses actual Fresnel/normal, whereas transmitted direct light
uses the plane's Fresnel and projected-area Jacobian. Wave focusing, defocusing, true
source-ray parallax, actual-surface self-shadowing and reciprocal source/camera paths
are omitted. Clear folds/falls and strong local surface slopes cannot be admitted by
renaming this term “mean caustics”.

For a height offset delta away from grazing, virtual path-length and horizontal shifts
are bounded by `delta/omega_w.y` and `delta*abs(omega_w.xz)/omega_w.y`; absorption
error can be compared through the corresponding exponential factors. These bounds say
nothing about a shadow discontinuity or concentrated caustic. At a knife-edge blocker,
an arbitrarily small shift can change the whole local direct light. At a focusing crest,
small height amplitude can still yield large concentration at suitable receiver depth.
No universal 2% image error or conservation guarantee follows from a 0.25 m height cap.

Qualification must compare the named virtual model with a converged actual-interface,
finite-Sun reference in ordinary motion, including a crest above mean, a wet/dry cell
edge, thin hull shadow, grazing underwater view and clear shallow bed. Report separate
losses in caustic contrast/motion, shadow parallax, Snell-window silhouette and water
colour. A recipe can accept those losses only if its cue requirements permit them.
If it needs the moving concentrated light, R3-RegionalVirtual fails that requirement;
more medium samples or a prettier foam texture cannot repair it.

### 6.4 Exact source-work removal before fidelity reductions

Share source facts and current regional geometry across both views. Analytic air/fog
integrals and plane Snell directions avoid marching those source segments. An optional
visibility cache may record **only a proved empty source-ray prism** for a spatial cell:
bound all virtual source segments for that cell, query the conservative BVH against that
prism and certify absence of opaque/cutout/alpha geometry. False-positive overlap falls
back to actual queries; an empty certificate removes traversal exactly, not shadow detail.
Coefficients/cloud tau still evaluate at the actual x/q/time.

Use finite lazily allocated cell records (proposed 8192 x 16 bytes = 128 KiB); unknown
cells query directly. Cache identity includes all geometry/deformation/source direction,
cover-plane and origin epochs. Clear on global source direction change; invalidate cells
whose prisms overlap swept changed geometry before publishing new geometry. A broad AABB
overlap is a valid conservative test even if it certifies few cells. No history-colour
reuse or trilinear shadow interpolation is presented as exact work removal. Caching a
scalar irradiance grid is a separately approximate provider needing its own edge/error gate.

## 7. Focused-source alternative: retain the unsolved producer

A credible candidate launches the incoming measure on an external Sun-perpendicular
plane times the finite solar disc. One source cell has incoming flux
`dPhi=E0_perp*dA_projected*angular_weight`. Start before **all** first regional interfaces,
including a fall over a pool; never launch full Sun again at every surface. Transport
that flux through ordered actual geometry/media events, splitting Fresnel reflection
and transmission, alpha coverage, absorption and scattering removal. Every terminal
receiver footprint is positive and area-normalized. Surface deposition uses flux/area;
the single-scatter source retains per-segment direction and phase rather than borrowing
a scalar caustic image. Both views query the same world-space source product.

That accounting is necessary but insufficient. A complete selection additionally needs:

1. A finite source-plane support proved to cover every path entering either view/source
   support, including reflected/refracted paths and medium scattering support.
2. A producer of conservative path/event and footprint bounds for **every source cell**,
   including unsampled blockers, silhouette slivers, TIR changes and interface critical sets.
3. A normalized receiver/support operator that does not splat through a wall or into the
   wrong medium. Kernel width/finite Sun regularization must have a declared model and
   local brightness/error bound, not an arbitrary HDR clamp.
4. Adaptive source subdivision and a ledger of unresolved flux with spatial support and
   a conversion to maximum possible receiver/source error. Small total missing flux alone
   is not a local irradiance bound when it can focus into a tiny receiver area.
5. A finite cap failure protocol before display. Invalidate the entire affected source
   partition and replace it with a complete **eligible** virtual/mean model, or fail it;
   dropped photons cannot become dark pixels. A virtual model is ineligible for a clear
   nonparallel fall, so that combination has no implicit fallback.

Counterexample: a thin aperture or refracting sliver lies between all source-cell corner,
centre and differential sample rays. All samples agree on a blocked/missing receiver,
yet the unsampled positive-area bundle can focus finite flux onto one pixel. Sample
variance is zero and emitted-packet flux balances, while the real feature is absent.
Finite Sun, more endpoint samples and ordinary Newton convergence do not establish a
bound on this missed contribution. An interval subdivision algorithm would need real
conservative geometry/optical mapping bounds; this repository currently has no such
implemented producer. The separate surface-bin challenger below now supplies a bounded
design construction for a restricted subset; naming interval arithmetic without its
actual predicates and error operators would not have been an implementable handoff.

The [2026 JCGT screen-space method](https://jcgt.org/published/0015/01/03/paper-lowres.pdf),
discussion and limitations on pp. 8–10, accelerates candidate intersections but explicitly
cannot represent camera-hidden geometry. NVIDIA's
[water-caustic engineering account](https://developer.nvidia.com/blog/generating-ray-traced-caustic-effects-in-unreal-engine-4-part-2/)
describes one-bounce rays and footprint/mesh reconstruction. Neither supplies the missing
all-cell conservative closure for arbitrary folds and occlusion. These are valuable
performance challengers after the support/error model is selected; no published frame
time transfers to Thrum's portable multi-view mixed-medium workload.

The subsequently supplied independent
[FocusedInterval challenger](R3-FOCUSED-LIGHT-CHALLENGER-2026-11-02.md) makes an important
refinement to that disposition. It supplies an actual conditional producer for **surface-bin
flux** on finite planar triangle shells: outward-rounded source ray tubes, conservative
BVH candidate enumeration, mandatory/possible interval triangle hits, first-hit ordering,
interval Snell/Fresnel/Beer and mandatory/possible receiver-bin flux sums. Every unknown
retains its flux upper bound; subdivision/candidate/branch caps refuse the partition before
presentation. The thin-aperture counterexample rejects sampled endpoint refinement, not
this conservative construction. Its restrictions exclude arbitrary deformation/transparent
geometry unless those bounds are supplied. Its approximately 20 MiB frontier and substantial
interval traversal work also require a new whole-scene admission, not spare portable budget.

Retain that operator as a conditional high-quality **surface** candidate, with correctness,
image quality and device performance unqualified. It does not close the full focused
single-scattering image: receiver-bin surface flux is not a bound on beam radiance inside
a medium. A voxel average can cross a thin wall or omit an angular concentration; a beam
bounds box plus direction does not supply the filtered beam-to-camera visibility/phase
integral and its error. Full `R3-FocusedActual` therefore stays open. Integration may
select a separate absorption-only restricted focused surface profile once its numerical
and resource gates pass, while keeping volumetric focused shafts and broad geometry open.

`ParallelSheet` remains useful independently. For a homogeneous absorption-only pair
of locally parallel interfaces use WATER-OPTICS's analytic infinite internal-reflection
collapse, including its grazing limit. Source content supplies finite thickness, sides,
IOR and absorption; the cook supplies parallelism/parallax/error bounds and closed edges.
It is not a substitute for a nonparallel barrel or an underwater camera in a zero-thickness
chart. Bubble-rich/scattering sheets require an actual scattered-source closure. Exposed
sheet/fall lighting stays outside the selected R3 source profile unless the entire source
path has a separately admitted analytic parallel-layer model.

## 8. Pass/resource graph and failure state machine

The selected sequence is:

`F1/E1/P4 commit -> extraction/readiness -> shared deformation/surface prepare -> BLAS/TLAS
refit/publication -> source/coverage readiness -> per-view opaque/shadow + first-interface
depth -> initial alpha append -> ray/medium resolve -> status reduction -> spatial V4/display
-> fences/retirement`.

Shadow preparation can overlap independent view targets, but any pass reading deformed
geometry waits for its generation. A source cache uses those same versions. View opaque
colour for optical consumers is **unfogged** surface radiance; the joint path adds view
media once. Opaque depth, first interface depth, refracted hit depth and V4 reconstruction
depth are distinct resources/meanings. A water depth cannot overwrite the opaque target
needed to terminate the transmitted ray.

| Resource/version | Producer -> consumer; lifetime |
| --- | --- |
| Current/prior geometry and regional BVH | F4/P4/E1/V2 -> shadows, raster, source and bent rays; shared current sample, leased prior samples only within C07 horizon |
| Opaque depth/scene-linear unfogged colour | Per-view V2/V3 -> initial event limit and optical terminal; retire after resolve |
| First-interface depth/ID | Per-view shared surface raster -> alpha gather/initial trace validation; covered tiles, retire after resolve |
| Alpha counters/records | Clear/gather -> later sort/resolve; independent allocation from interface events; reuse only after last GPU read |
| Ray batch stacks/accumulators | V3 -> one owner per primary pixel; finite batches below; retire after optical output |
| Optical colour/coverage/status | Resolve -> GPU reduction and V4; invalid data cannot enter temporal history |
| Joint-media coefficient/source samples | E1 descriptors plus V3 query -> current integration; no independently fogged background copy |
| View histories, if later enabled | V3/V4 -> same view/provider accepted sample; do not share merely because profiles match |
| Source visibility certificates | V3 -> both views; versioned cache side effect, explicitly retired when provider off |

At view start clear a GPU status word. Each pass atomically ORs typed failure bits and
writes valid output only within its bounds. The sequential state machine is
`Declared -> Cooked -> DeviceReady -> GeometryReady -> Resolving -> Validated -> Displayable`.
`SourceIneligible`, traversal/order/event overflow, stale dependency or arithmetic fault
branches to `FailedCurrent`; it never proceeds through Displayable on a later CPU promise.

After all producers, a GPU status reduction/resolve gate selects one of: complete current
result; an already prepared **whole accepted fallback profile**; or a diagnostic/loading
surface. It also disables affected V4 history publication for the failed generation.
The first late-overflow proof selects the diagnostic/loading surface through an
independent same-submission path. A ready fallback pipeline/product is not yet
a fallback image: a richer current-frame substitution needs its own encoded
execution path, complete current inputs independent of the failed producer,
selection mechanism and worst-case failed-primary-plus-fallback work/storage
account. If any of these is absent, the gate selects diagnostic/loading.
CPU readback arrives later for diagnostics and admission reports. It does not determine
whether this frame silently displays truncated layers. A previous complete image may be
held only with an explicit stale-view loading policy and bounded age; stale world pixels
are not current geometry proof. An old light solution cannot blend with a new incomplete
solution. Live profile switches prepare old/new overlap and publish at one extraction
barrier; failed preparation retains the old complete generation if still eligible.

## 9. Whole-scene cost and extreme-performance choices

Use the sole [whole-scene budgets](../BASELINE-SPEC.md#budget-allocation-and-failure-rules):
desktop CPU 5 ms/GPU 10 ms, browser CPU 8 ms/GPU 16 ms, mobile CPU 7 ms/GPU 16 ms,
with their respective shared residency envelopes. These are unmeasured admission goals.
Optical work competes with physics, all visible instances, both views, shadows, UI/audio,
other media and replacement peaks. It does not receive a second 16 ms budget.

### 9.1 Counts and traffic

At 720p, P=921600. With 25% optical coverage and half-width/height optical evaluation,
there are 57600 primary optical rays. Four interface levels permit 1,785,600 straight
tree nodes before alpha/medium subdivision; two permit 403,200. At an illustrative
32 visited 32-byte bounds nodes, node loads alone are about 1.83 GB / 0.413 GB per view,
before cache effects, triangles, alpha textures, medium source rays and output. These
are worst-tree arithmetic examples, not expected visits or measured bandwidth.

Single-scattering source queries can dominate: 57600 rays x 16 medium samples x one
source query is 921600 additional visibility queries **on the first branch alone**.
An empty-prism source certificate can remove most repeated shadow traversal in an open
pond, but cannot be assumed around moving hulls and cutout foliage. Count full source
queries and certified-empty hits separately; benchmark their worst invalidation frame.

Portable scattering is consequently **provisional**, with a concrete proposed easy
admission envelope: one 16x16 m broad graph-water pond, at most 25% main-view coverage,
half-linear optical resolution, one camera, broad planar bed, no exposed clear side
faces, no underwater local lights, and no more than 65536 uncached source visibility
queries across the whole view/frame. Preclassify the complete camera/source support
before first play. At the 921600 first-branch sample example this cap demands at least
92.9% exact source-query removal, and secondary branches can demand more. A blocker-free
source-prism field can prove 100% empty visibility; a dock, cutout roof or moving hull
can destroy that prediction. Such scenes need a different admitted source envelope.
Runtime query counting and cache invalidation precede the display gate; cap exhaustion
fails the current capability rather than retaining stale Sun or discarding scattering.
This count is a finite admission target, not proof 65536 rays fit 16 ms.

The lower `R3-RegionalAbsorption-P` floor selects sigma_s=0 for water and one graph/bed
volume with at most one interface along each supported camera path. Reflections terminate
at the covered bed/solid or admitted external environment; a path crossing another water
boundary fails that profile. It retains actual underwater classification, Snell window,
TIR, thickness-dependent absorption, alpha ordering and physical water/body/shore semantics.
It removes water in-scattering samples and their repeated source visibility queries and
caps the smooth interface tree at three straight nodes, rather than seven. It loses
underwater shafts and scattering haze/colour filling; this is an authored visual-profile
change, not a device secretly turning sigma_s off. Air/fog retains its separately budgeted
canonical operator. Even this floor still needs device qualification and whole-scene
budget admission; it is the concrete portable first proof, not advertised universal support.

Alpha gather cost is rasterized fragment invocations times material/coverage evaluation
plus one contended counter operation, not the two stored slots. A stress stack of 100
panes still shades/rejects many fragments before reporting overflow. Tight conservative
tile/draw culling, shared geometry preparation and material/source specialization remove
work without changing ordering. Admission bounds overdraw explicitly. Storing only two
records is not an overdraw cost bound.

CPU work scales with changed objects, dirty BLAS leaves, refit/build stages, source cache
invalidations, covered tile lists and draw/material batches; no CPU ray result readback
is on the ordinary frame path. Avoid rescanning unchanged static triangles per view.
Share one accepted current deformation/surface preparation across both views and shadow/
ray consumers. Include skin vertices x influencing joints and water vertices x wave
components; “shared geometry” still has a preparation cost and current/prior residency.

### 9.2 Concrete logical allocation worksheet

The following **illustrative P recipe** makes the existing 51.2 MiB browser transient
share inspectable. It is not automatic R3 admission. Reserve 30% of full-screen pixels
for 16x16 optical/alpha tiles, allowing padding beyond the 25% visible coverage;
A=276480. Optical evaluation uses half linear resolution. Spatial V4 avoids temporal
history in P. Shadow atlas is persistent; include it in total GPU residency even though
it is absent from the transient subtotal.

| Simultaneously live allocation at optical resolve | MiB, logical |
| --- | ---: |
| Full-view HDR unfogged colour RGBA16F + opaque D32 (12P) | 10.547 |
| Two alpha records + counter (36A) | 9.492 |
| Covered first-interface depth/ID (8A) | 2.109 |
| Half-resolution optical RGB/coverage RGBA16F (8P/4) | 1.758 |
| Half-resolution joint-medium T and S, each RGBA16F (16P/4) | 3.516 |
| Packed optical validity/reactivity, u32 per covered full-res pixel (4A) | 1.055 |
| Full-view final spatial/filterable HDR target (8P) | 7.031 |
| Explicit ray batch stack/accumulator/counter allowance | 2.000 |
| Tile tables, status/readback ring and alignment allowance | 0.500 |
| **Transient total** | **38.008** |
| One 2048-square D32 Sun atlas, persistent | 16.000 |
| Proposed 50k-triangle regional proxy/BVH | about 4.000 |
| Current/prior deforming/surface vertices, material/coverage records | recipe-specific; mandatory additional charge |
| Visibility certificates, shared persistent | 0.125 |

This subtotal deliberately keeps separate alpha and first-interface storage, status,
media pairs and a displayable target; a shared “water buffer” label cannot conceal them.
Depth-aware upsampling reads immutable depth and writes the final target. Alpha records
can be overwritten by the same pixel owner during resolve only after all their readers
finish; allocation reuse must follow actual last use. The worksheet conservatively
allows media intermediates and final output concurrently rather than assuming a saving.

At 1080p with identical coverage policy, pixel-dependent rows multiply by 2.25. Fixed
2.5 MiB scratch plus about 35.508 x 2.25 gives **82.393 MiB** for the main view. A
half-width/height secondary view costs about **22.473 MiB** if independently allocated
with the same fixed scratch. Both at once are about **104.866 MiB**, plus shared shadows,
regional products, source caches, ordinary geometry/textures, simulation and replacement.
These remain below the desktop transient planning share in isolation; that is not a
whole-scene fit or a GPU-time proof. Desktop temporal history/motion, if enabled, adds
its actual per-view targets; the regional initial recipe also selects spatial output.

Two 720p views allocated concurrently would use about **76.016 MiB** of transient
storage and fail the browser's 51.2 MiB share before other consumers. P therefore admits
one view. A proposed sequential dual-view variant can reuse scratch/targets only after
last use and while preserving each final output/history; its summed GPU work and actual
overlap still need a separate admission. Do not claim “shared scratch” erases two outputs.

No focused beam maps, source queues, caustic receivers or scattering grids are allocated
by R3-RegionalVirtual. They are not free latent resources. A focused challenger must add
all of them and transition overlap before any combined fit claim. A hypothetical 256-square
launch has 65536 source cells **per direction/light/cascade**, before adaptive splits,
reflections and source interval storage. It cannot be budgeted as one 256-square texture.

Reserve explicit finite ray work storage rather than assuming private shader arrays stay
in registers. A D depth-first pending-reflection stack can require roughly 4 x 48 bytes
per active primary ray plus accumulator/status. Processing all 230400 covered full-res
pixels that way could spill over 42 MiB even before registers affect occupancy. Select
bounded batches of at most 8192 primary rays with checked global stack slots and ≤2 MiB
scratch, serialized between batches within one view. Store pending ray origin/direction/
RGB weight and inherited counters in explicit aligned records. P uses at most two pending
levels. Report dispatch overhead and batch occupancy; this trades capacity for extra
dispatches, not free throughput. Device limits may require smaller batches.

Browser total admission also charges native/browser/Wasm retained buffers, all persistent
content, uploads, render targets, physical water/body state and driver-visible overhead.
A 4 MiB BVH replacement temporarily costs old+new+build scratch+upload, not 4 MiB total.
Live optical-only edit reserves it; initial portable substantive changes stop/load/restart.

### 9.3 Reduction order and rejection rules

First remove duplicate geometry/coverage evaluation, uploads, per-view source facts and
empty source visibility work. Then specialize material/purpose loops and batch data by
actual regional use. Retain the direct BVH path as the correctness fallback when Newton
or a cache is not valid. Investigate source-prism coverage, traversal node layout and
ray batches against matched scenes; coherent packets may improve cache reuse but may
also waste work on divergent TIR rays.

Quality changes have separate names: reduce optical sample rate while preserving thin
cue coverage, lower output resolution, reduce optional reflection detail to an admitted
environment/local capture, or select P's smaller admitted scene. Do not reduce interface
counts live and discard existing rays. Required underwater support, alpha events, wet
support, physical timestep and water/body history are not performance knobs. If P cannot
afford its source/ray multiplier, R3 is unsupported on that device/scene until a different
qualified mechanism is chosen. A prettier incomplete screen-only result is not a pass.

## 10. Creator, replacement and lifecycle walk

The following source schema/tool commands are **proposed, unbuilt interfaces**, not new
commands to execute. A game selects an optical profile in its composition recipe and
references stable water-domain, regional visibility and light-source IDs. Source records
contain IOR/coefficient/phase data, source-plane policy and maximum offset, camera/range/
sun envelopes, optical/alpha/visit/sample capacities, cue/error policy and permitted
fallbacks. Source-defined physical coefficients that affect rules remain in E1's semantic
manifest; presentation-only optical coefficients have a separate declared authority role.

F4 consumes ordinary admitted mesh/material/animation/water/terrain source records. It
validates coverage conversion, dependency closure, shell orientation/edges, preserved
features and deformation bounds; cooks static BLAS topology and material/alpha products;
and emits the profile/readiness manifest. P4/E1 feed accepted dynamic vertices at runtime.
F2 derives actual depth/interface/alpha/ray/source/media/display tuples; no creator-maintained
second shader list. An unsupported imported glass extension fails or presents an explicit
authored conversion loss. A required opaque/cutout/deforming participant cannot be omitted
to make the proxy smaller.

First use: admit source snapshot -> derive/cook complete closure -> reserve regional plus
view resources -> prepare pipelines -> publish endpoint/current geometry and source
versions -> classify camera -> execute/gate -> inspect actual shoreline, underwater view
and optical diagnostics. The first fixture uses one shallow domain, prescribed bounded
waves, one dry hull, offscreen bed, one moving skinned opaque blocker, one alpha pane,
one cutout fence, one fog box and the cloud-tau ramp; add a second view only in D. It is
small in content while retaining every selected coupling.

Edit absorption or source-plane policy: rehash the material/profile product and shader
demand as needed; preserve physical E1/P3 state if the fields are presentation-only;
prepare new source/profile state and reserve old/new GPU overlap; atomically switch
presentation generation; invalidate source certificates and optical/media/V4 histories.
Edit physical wet reconstruction, topology, source gameplay meaning or incompatible
deformation: follow the authority restart/replacement policy, rebuild the dependent bed/
collision/rain/optical closure, and never hot-swap just the render surface. A failed
replacement leaves the last complete eligible generation or enters loading.

Graphics-off retires alpha/interface buffers, optical BVH copies unused by any other
consumer, source certificates, media targets, render-only residual preparation and
histories after GPU fences and leases. Physical water, body queries, wet support and
causal source facts remain if their game consumers remain. Removing one optical provider
must first detach its declared consumers and substitute an admitted provider or reject
the recipe; it creates no private composition/plugin system. Device loss drops GPU
handles, rebuilds from pinned CPU/cooked products and invalidates histories while authority
continues under its own lifecycle. Save/reload records durable source/profile IDs and
physical state, not transient BVH handles, append counters or view images.

## 11. Implementor walk, fault injection and independent oracles

The first implementor slice is the counterexample, not a calm surface beauty shot.
Construct a wave crest +0.20 m over mean zero with a wet point +0.10 m, an adjacent
physically dry full cell, an alpha pane crossing the waterline, a dry offscreen Sun
blocker and a deforming cutout blocker. Use the same current mesh/evaluator products in
raster and regional queries. Put the camera underwater, then cross the interface at
ordinary speed and skip/resume a second view in D.

Execution from saved inputs:

1. Read the committed physical endpoint; derive exact wet shell and common current
   vertices with errors. A dry-cell surface or inconsistent bed revision fails now.
2. Refit static-instance/deforming/water spatial products; publish one ready generation.
   Verify that the offscreen blocker remains in source-purpose coverage.
3. Derive H_cover from the certified endpoint maximum; initialize actual camera medium.
   Evaluate the above-mean water source query; positive forward distance is required.
4. Capture opaque and first-interface depths; append initial alpha before that boundary.
   Order deterministic ties. A third initial pane sets failure without memory overwrite.
5. Trace each optical ray, integrating actual air/fog/water intervals and resolving
   actual alpha/interface events. Source samples use the named virtual path and same
   current blocker geometry. Bent rays do not read the primary alpha list as geometry.
6. Reduce every failure bit before V4; display current complete result or prepared
   diagnostic/fallback. Store history only for a validated accepted sample.
7. Compare source/path/medium records and image with the independent oracles below.
   Preserve physical and optical acceptance separately. A passing cell-volume residual
   cannot qualify an underwater image.

| Fault / plausible wrong substitute | Oracle or observation that rejects it |
| --- | --- |
| Mean-plane point above its plane returns negative distance or ordinary dry Sun | Analytic positive cover-plane path and explicit optical-A ineligibility; exact regression coordinates above |
| Smoothed shore makes dry cell appear occupied | Independent physical wet-mask/bed query overlay and wet-volume classification, including onset/disappearance; surface picture alone cannot pass |
| Coverage pane before/inside/behind water sorted only by camera depth | CPU f64 brute-force triangle/path interpreter with independent closest-hit implementation; compare hit/event sequence and per-segment radiance |
| Third alpha or fifth D interface disappears | Inject event-cap overflow, counter wrap preflight and scratch exhaustion; current GPU display gate must show failure before CPU readback |
| Screen Newton candidate misses hidden blocker | Place offscreen bed and camera-hidden hull; brute-force first-hit reference and CoveredMiss/UncoveredExit distinction |
| Cutout opaque in source or deformed actor absent from rays | Compare material alpha function/UV samples and current vertex identities across roles; animate blocker while camera remains still |
| Refit publishes stale bounds or shader numeric edge leaks | Move a needle triangle through old bounds; shoot edge/vertex rays; compare padded f32 traversal with high-precision independent triangle enumeration |
| Water applies fog twice or alpha colour pays medium twice | Closed-form homogeneous absorber before/after an emissive coverage pane; joint overlapping fog solution; unit-consistent T,S comparison |
| Missing index-squared, wrong TIR or wrong camera medium | Parallel dielectric slab radiance/flux test, critical-angle sweep, normal-incidence Fresnel and closed-shell parity/winding agreement |
| Virtual source called actual caustics | Finite-Sun actual-interface path-traced reference: moving wave/focusing depth, thin hull edge and crest receiver; measure and visually review the admitted losses |
| Scalar caustic used for phase or HG sign reversed | With g=0.6 compare camera on the source side (`omega_camera=omega_light`, mu=-1) against the opposite side (`omega_camera=-omega_light`, mu=+1): the forward/opposite-side HG value is 64 times the backward/source-side value. Rotate the camera and compare the full directional source; symmetric Rayleigh cannot detect the sign bug |
| Same-profile cameras share history or stale source cache | Independent view cadence/origin/cut; source-direction and deformed-blocker invalidation; expected source facts equal but history IDs distinct |
| Feature off retains hidden work | Pass/resource ownership census after fences plus allocator/GPU logical bytes; physical water queries still succeed |

The reference raster/path interpreter must not share the candidate's BVH traversal,
event sort or medium interval builder. It may consume the same immutable source geometry;
a second source-shore reference independently constructs occupancy from physical cells.
An offline converged finite-Sun path tracer is the source-model comparison, not the
shipping implementation or a way to smuggle unavailable data into it. Use analytic
plane/slab/Beer/phase tests to distinguish bugs shared by candidate and reference.

Qualification queue: (1) source/camera domain and crest/shore faults; (2) exact alpha and
regional/deformation first-hit semantics; (3) underwater/TIR and mixed medium ordering;
(4) joint source/alpha/cutout shadow motion; (5) ordinary native-scale and moving image
comparison against the actual-interface reference, including each virtual-model loss;
(6) two-view D/lifecycle; (7) sustained whole-scene native/browser/mobile frame and
memory proof under the canonical five-minute warmup/25-minute run. Report pipeline
closure, per-pass times, ray/tree/event/visit counts, empty-source-cache hit rate, peak
old/new bytes and all failure rates. A finite correct operator may still fail the
quality or device gate; no maturity promotion follows from this paper walk.

## 12. Evidence used, alternatives and access limits

These sources were substantively inspected as text/paper/code, not watched recordings.
The present recommendation combines published operators with authored bounds and
deductions. It does not claim to reproduce any production engine's private implementation.

| Evidence and inspected material | Decision consequence and limit |
| --- | --- |
| [Mayer, Assarsson, Sintorn, JCGT 15(1), 2026](https://jcgt.org/published/0015/01/03/paper-lowres.pdf), method/discussion/limitations pp. 4–10; text inspected 2 November | Recent Newton accelerator reduces candidate lookup work; explicit camera-hidden geometry limitation prevents it owning complete regional visibility or source coverage |
| [Woop, Benthin, Wald, JCGT 2(1), 2013](https://jcgt.org/published/0002/01/05/paper.pdf), abstract and numerical problem/algorithm discussion pp. 65–70 | Watertight intersection depends on conservative bounds and a rare higher-precision route. Portable f32 cannot claim the paper's double fallback automatically; uncertain edge cases need a status |
| [PBRT 4 dielectric operator](https://pbr-book.org/4ed/Reflection_Models/Dielectric_BSDF), smooth transmission and thin dielectric sections; [transmittance](https://www.pbr-book.org/4ed/Volume_Scattering/Transmittance) | Separates radiance eta factor, flux and homogeneous attenuation; thin-pair closure has a specific geometric domain, not arbitrary sheets |
| [Bevy v0.20.0 OIT preparation/source](https://raw.githubusercontent.com/bevyengine/bevy/v0.20.0/crates/bevy_core_pipeline/src/oit/mod.rs), settings 40–59, linked-list description 71–79, records/buffers 132–150, allocation/removal 203–286 | Pinned executable design distinguishes sort count, global average allocation, approximations and release. Its discard policy is incompatible with required R3 events. MIT/Apache project provenance must be rechecked before any code reuse; no source is imported |
| [WGSL editor draft](https://gpuweb.github.io/gpuweb/wgsl/#atomic-builtin-functions) and [WebGPU limits](https://gpuweb.github.io/gpuweb/#limits), observed 30 October 2026 draft on 2 November | Supports buffer u32 atomic approach and explicit limits; spec access is not adapter conformance or a guarantee of native tile-GPU performance |
| [Ouyang/Yang, NVIDIA water caustics, 2020](https://developer.nvidia.com/blog/generating-ray-traced-caustic-effects-in-unreal-engine-4-part-2/), workflow and reconstruction/performance discussion | Real production-motivated competing forward light approach; one-bounce/ray-guided reconstruction has different geometry/hardware scope. Its reported timings do not budget this proposed system |
| [SIGGRAPH 2026 Advances course](https://www.advances.realtimerendering.com/s2026/), current index; retained [render research audit](RENDER-RESEARCH-2026-11-02.md) of Smolder speaker notes | Current production mixed-media integration remains approximation-sensitive. This study read the index and existing local audit, not the 30 MB talk deck or recording; no fresh full-talk inspection is claimed |

PBRT's current mutable source files for dielectric sampling and media definitions were
also readable, but an attempted fixed-tag source URL failed. They were not pinned to
a resolved commit and do not establish a reusable code artifact; book algorithms above
are the substantive adopted reference. The NVIDIA branch's full code was not accessible
here and no blanket code-audit claim is made. No Broadsides files were modified or used
as engine dependencies. Existing Thrum research supplies historical provenance without
turning a prior demonstration into qualification of this new assembly.

## 13. Contract deltas for the canonical owners

- **C04–C06 / F2/F4/V1:** exact alpha append/interface/ray/source roles enter reachable
  pipeline closure; status read-after-write gates display/history; declare counter,
  batch-stack, record, source-cache and old/new resource peaks. GPU overflow decisions
  precede display independently of delayed CPU readback.
- **C07 / P4/E1/V2/V4:** publish shared current/prior evaluated geometry and conservative
  errors, full regional participant/material coverage and exact wet topology; distinguish
  opaque/interface/refracted/reconstruction depths and each view's accepted sample.
- **C08 / V3/E1:** choose SortedAlpha versus BoundedAlpha2 explicitly; independent
  raster/alpha/interface/traversal/source/sample caps; ordered coverage-alpha and dielectric
  operators; actual camera occupancy but named virtual-cover-plane source policy; one
  incident direction/irradiance producer shared by receiver and scattering; incomplete
  light is not darkness; focused actual-interface capability remains unselected.
- **C09 / E1/P3/W2/F4:** publish physical full-cell wet support/bed/storage and optical
  union adjacency; stream seams cancel internal equal-medium faces; residual/foam masks
  cannot create water on physically dry support. Optical shells grant no gameplay volume.
- **Maturity/build owners:** this is a proposed design-level regional expansion, not a
  correctness/visual/device result. Keep full focused/fall source closure open and
  qualification unrun. Canonical selection and independent resolution review are still
  required before dependent construction treats the recommendation as governing.
