# Water surface and optical transport

**V1 reset (3 November 2026): R3 regional optics and exact alpha are parked until after V1. The selection below is retained research, not an active implementation gate.**

1 November 2026, revised 2 November. **Bounded regional optics selected in design;
actual-interface focused surface-plus-volume light remains open. Unimplemented and
unqualified.** This is the rendering side of the [water commission](WATER.md) under
C07/C08. It does not replace the smaller sorted-alpha renderer baseline or declare
a portable ray-tracing engine. V3 owns light/material/media transport; V2 supplies
geometry/visibility; V1 executes resources/passes; V4 reconstructs/displays. E1
supplies the committed water, not a private renderer or Sun. The
[R3 optical decision](../research/R3-OPTICS-AND-TRANSPARENCY-DESIGN-2026-11-02.md)
is the implementor recipe and cost account; the
[focused-light challenge](../research/R3-FOCUSED-LIGHT-CHALLENGER-2026-11-02.md)
records the remaining source-coverage/image gap.

The selected `R3-RegionalVirtual` profile traces camera paths through actual
admitted occupied water and complete supported regional geometry, while
underwater direct Sun uses a named virtual cover-plane source approximation.
It omits actual wave focusing and clear focused illumination through arbitrary
falls. Desktop's four interface/four alpha-event envelope and portable's
smaller absorption-first envelope are distinct, neither device-qualified.
`BoundedAlpha2` gives two exact ordered nonrefractive coverage events; grouped
sorted alpha remains an explicitly approximate alternative. A wider focused
profile is a conditional research candidate, not selected merely because
the mechanism and equations below describe what it would require.

## Common interface and reconstruction

An immutable endpoint identifies world/epoch/tick, provider/domain/topology/geometry revisions, physical profile and
readiness. A surface reference identifies domain, branch and chart revision. A sample names its endpoint pair/time,
chart coordinate q, purpose and footprint, and returns P(q,t), J=[P_q1,P_q2], P_t, oriented normal/media sides,
material velocity, wet support/coverage, derivatives/correspondence validity and physical/render error.

Statuses distinguish Valid, Dry, OutsideDomain, UnavailableDependency, UnsupportedRegime, AmbiguousBranch and
StaleEndpoint. A graph query cannot pretend to describe a folded chart; a visual fold cannot grant physical occupancy.
The [appearance design](WATER-APPEARANCE.md) defines projected material transport on moving charts. P_t and crest
phase are not the water carrier. Previous evaluation belongs to the actual image retained by each view.

One evaluator drives the physical base plus explicitly bounded visual residual, surface mesh, optical intersections
and caustic source geometry. For residual `P_v=P_p+m d`, derivatives include both `m partial(d)` and `d partial(m)`.
Masks vanish smoothly before residuals bridge dry/solid/interactive exclusions. Wet area comes from the owning
storage/bed reconstruction. A larger prepared curl is a separately admitted approximation, not small normal detail.

Allocate each detail band once: resolved displacement, filtered normal variance, then unresolved positive-semidefinite
slope covariance in the microfacet closure. Omitted resolved energy becomes covariance rather than being added again.
Filter coverage, optical populations and source masks at the same footprint. Adaptive geometry uses projected
position/normal error and shared-edge continuity; render mesh resolution does not redefine the force surface.

## Dielectric, foam and underwater material

Use scene-linear RGB Rec.709/D65 under C08 with declared index, absorption and scattering. The physical water
provider does not invent a second colour/fog/solar multiplier. For radiance from the far side b to the camera side a,
`L_a=F L_reflected+(1-F)(n_a/n_b)^2 L_transmitted`, under the selected radiance convention. Flux packets use their
corresponding energy convention; never apply the index factor twice. Total internal reflection gives F=1.

For a water segment, `T=exp(-integral sigma_t ds)` and `L_transmitted=T L_receiver+S`. Water and compatible bubble
coefficients enter one integration and shared incident-light closure. Surface foam replaces covered clear water:
`f=(1-c)f_water+c f_foam`, `c=1-exp(-sum A_i K_i)`, with area-normalized kernels. It is not additive white emission.
Bubble extinction is `sum n_i Q_ext(r_i) pi r_i² / V` in m^-1. Explicit bubble interfaces and unresolved extinction
must partition the same population rather than render it twice.

## Visibility must distinguish missing geometry from darkness

Select a bounded **water-region optical proxy product**, owned by F4/V2: static bed/solid BLAS, rigid-instance TLAS,
primitive/material IDs, source pins and complete-support mask. Ray masks distinguish dry incident-light blockers,
submerged receivers/occluders and reflected geometry. A dry hull can block the Sun without hiding the bed product
needed by refraction. Use the same material cutoff/double-sidedness and binary-mask dependencies as V3.
These masks denote query purposes, not exclusive wet/dry object classes: a straddling solid may participate in several.
The actual ray segment/media intervals select the relevant hit; changing water level cannot make its geometry vanish.

The initial supported set is rigid triangle/bed geometry and binary cutouts. Deforming or transparent scene objects
need an admitted mapping/optical rule; they cannot disappear from purported complete coverage. Proxy error includes
position, normals, material and minimum preserved feature. `delta_t≈epsilon_geometry/|n dot ray|` is useful only away
from grazing or ambiguous hit ordering; otherwise refine or report approximation/ambiguity.

Use an iterative stackless linear BVH with escape indices, 32-byte bounds nodes, up to four triangles per leaf,
vec4 positions and indexed material records. No WGSL recursion or hardware-ray feature is assumed. Node/primitive
visit caps return Incomplete with reason/candidate, never Hit/Miss. A CoveredMiss reaches only the declared complete
support/range. UncoveredExit and UnsupportedGeometry are separate results. TLAS follows shared accepted poses;
static BLAS does not rebuild per view or per frame. Layout is a selected implementation candidate, not a new public API.

[Newton screen-space intersections](https://jcgt.org/published/0015/01/03/paper-lowres.pdf), §§2–4, are a current
accelerator: obtain a candidate/upper bound, then validate its identity/material and test the preceding segment for
occlusion. Otherwise use bounded closest-hit traversal. A converged camera-depth hit alone is not first-hit proof.
Compare this hybrid against direct regional tracing; extra capture/fetch work may outweigh saved traversal.

Camera/light captures carry projection/range, primitive/time/coverage and supported footprint. Reflection fallback
is a validated scene hit, an admitted planar/local capture, then a supported environment/probe approximation.
Environment radiance is not a substitute for an unknown submerged receiver. A cheaper screen-only tier must label
its missing-geometry approximation and preserve any game-significant cue envelope. Missing data is not shadow.

## Caustics partition a light, rather than add another light

This section states the **required conservation/source partition for a future
actual-interface focused profile**, not a claim that the selected regional
virtual-source profile implements focused caustics. Its finite-Sun first-hit
source-cell producer, receiver-bin flux bounds and refusal route are explored
in the [independent challenge](../research/R3-FOCUSED-LIGHT-CHALLENGER-2026-11-02.md).
Practical real-time admission and bounded focused single-scatter image
reconstruction remain unselected. The regional virtual profile transports
one complete source within its deliberately flatter model and does not add
a caustic texture on top of ordinary Sun.

For incident source l and source area dA,
`dPhi_in=E_perpendicular,l max(0,n dot omega_l) dA Visibility_l T_air,l`.
Reflect F of that flux; transmit 1-F, then account water absorption/scattering. Surviving beam power is
`Phi(s)=Phi0 exp(-integral sigma_t ds)`. Normalized finite-footprint splats deposit `Phi_hit K_area`,
`integral K_area dA=1`; finite source angular extent and footprint prevent singular intensity without an arbitrary
brightness clamp. Receiver irradiance and underwater in-scattering use the same admitted refracted incident field.

V3 partitions ordinary dry direct illumination from through-water illumination. A through-water source tile/light
uses **either** focused caustics **or** the complete declared mean-refracted solution, never ordinary Sun plus another
caustic multiplier. Environment Sun-disc radiance and explicit solar specular also have one exclusive owner.
Do not attenuate a receiver again after its incident beam already paid that water path.

Emit incident flux only at the **first** water interface on each light ray. Folds or a fall over a pool cannot each
launch the full Sun independently. V3 light-space visibility orders water/self-occlusion events using the same surface
evaluator; subsequent interfaces transport the already-partitioned beam. The event/overflow bound applies to light
paths too, with complete mean-path fallback when eligible. A view-depth capture alone cannot supply this product.

Focused mode needs complete source interface, blockers and receiving support. If any required source tile cannot
complete tracing, invalidate its focused solution and use its mean-interface transport for the same source partition.
Failed photons do not become darkness. If even mean transport lacks coverage, return an explicit approximation or
reject the promised optical capability. Track unresolved flux. Blend only two valid complete solutions for the same
flux partition. The regional proxy and footprint/energy accounting are Thrum adaptations, not the JCGT paper's guarantee.

## Layers, falls and closed optical volumes

Retain the initial two-event path for its supported scenes. The selected desktop
regional profile admits at most four oriented water-interface events **per camera
branch**, with separate bounded alpha events and explicit overflow; it is not
unrestricted recursive reflection or a focused source-light path. A clear
nonparallel fall requiring actual focused source transport remains outside it.
Each branch requires oriented media and finite wall thickness/closed optical volume where camera medium selection
is promised. A bare prepared chart does not supply occupied water or an air cavity.
Even from outside, clear transmission through a curl requires entry/exit thickness. Select a cooked oriented
watertight chart/base shell validated against animation self-intersection, or an explicitly reflection/opaque-foam
fallback with no clear transmitted-barrel promise. The optical shell remains noncontact and grants no P3 occupancy.

For an admitted locally parallel thin fall, collapse two interfaces analytically. With interface Fresnel F,
`A=exp(-sigma_t thickness/|d_water dot n|)`,
`R=F+(1-F)^2 F A²/(1-F² A²)`, `T=(1-F)^2 A/(1-F² A²)`.
Handle the lossless grazing limit F=A=1 explicitly as R=1,T=0, avoiding a numerical 0/0.
`1-R-T` is removed direct flux, not entirely absorption unless sigma_s=0. In homogeneous material, divide it into
sigma_a/sigma_t absorption and sigma_s/sigma_t out-scattering; outgoing scattered radiance still needs its own closure.
The initial collapsed-sheet tier is absorption-only clear water. Bubble-rich/scattering sheets require the actual
V3 source treatment or a labelled omission bound. Thickness/parallax error and local parallelism are admitted limits.

Thick/curved sheets and eligible folded tubes need actual entry/exit events. A fifth active interface cannot vanish:
scene/profile admission must bound overlap, or return Unsupported/ApproximateOptics with a diagnostic and declared
fallback. Noncontact breaking charts remain outside protected camera traversal support unless an oriented finite-wall
optical volume and its traversal are admitted. Optical classification does not grant gameplay swimming/contact.

## Frame, temporal products and lifecycle

F1 commits E1/P3; extraction leases the immutable endpoint/products; V2 prepares shared interfaces/rigid visibility;
V3 prepares light-space transport and receiver shading; the view obtains required opaque/submerged products and
ordered interfaces; V3 composes reflection/refraction/media; V4 receives scene-linear colour and layer/confidence/
reactivity/motion. V1 owns GPU execution, device resources and fences throughout.

Surface motion is the same reference chart evaluated at each view's current and last accepted presentation time.
Foam/material motion is separate; reflected/refracted backgrounds need their own hit identity/confidence. A surface
motion vector alone cannot validate those histories. Repartitioned detail, topology, correction, capture/probe,
source/model/device changes invalidate the affected histories. Skipped views reset after their retained horizon;
they do not pin an unbounded world/solver snapshot ring.

Graphics-off retires optical captures, GPU visibility copies when unused, caustics, render-only residual preparation
and histories after leases/fences, while retained physical queries and coarse causal tails remain. Device loss
rebuilds derived products from CPU/cooked pins; it does not reset the authority. Gameplay edits restart initially;
qualified optical-only replacement reserves old/new peaks, swaps atomically and invalidates histories.

## Cost and ranked reductions

At 720p with 25% water coverage there are 230,400 water pixels; half-width/height rays give 57,600. Four Newton iterations
cost 230,400 depth/normal fetch pairs before validation. An illustrative 32-node average traversal moves about 59 MB of
node data per frame/view before cache effects, triangles and materials. These are operation/traffic estimates, not timings.
A 50k-triangle regional product is roughly 4 MB for nodes, indices and worst-case unshared vertices, with replacement,
uploads and leases additional. Typical traversal, masks and overflow rates remain empirical.

One 16-byte full-screen interface layer at 720p costs 14.75 MB; four cost 59 MB, exceeding the browser's whole-scene transient
share before other systems. A full-resolution nearest layer plus three half-resolution secondary layers costs about
25.8MB. Covered-tile allocation can reduce work but must reserve counts/scratch and report overflow. Do not allocate
four unrestricted full-screen layers by default. A 256² caustic tile emits 65,536 beams; every tile/light/capture multiplies cost.

Reduction order is an unmeasured hypothesis: remove duplicate preparation/captures/uploads; filter unresolved detail;
classify work by covered water tiles and reduce distant ray rate; reduce focused-caustic detail/rate under motion
bounds; replace focused light with complete mean-refracted light; replace local reflections with admitted probes;
use eligible thin sheets/reduced secondary layers; then lower internal output resolution. Record lost concentration,
parallax, dynamic contact, thickness and overlap cues separately. Physical timestep/grid/source/history are not graphics controls.

The [integration proof](WATER-INTEGRATION.md) must separately exercise disocclusion, a dry hull over a visible bed,
offscreen receiver/blocker, finite Sun, masked materials, underwater crossing, branch change, source-off, two views
and layer/visit overflow. A correct shader or successful Newton root does not qualify the assembled water image.
