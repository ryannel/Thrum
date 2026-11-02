# Selected cloud and atmosphere production baseline

2 November 2026. **Implementation specification selected; implementation/visual/device qualification unrun.**
This document locks the production mechanisms for [CLOUDS](CLOUDS.md) and the single
[build queue](CLOUD-MIGRATION.md). Existing experiments are comparators. They do not choose the new architecture.
The baseline uses published production techniques adopted through2026; its precise Thrum assembly and starting
parameters are ours. No publication establishes this entire combination or promises M2 timings.

## Locked choices and production lineage

| Area | Selected baseline | Production support and limits |
| --- | --- | --- |
| Authoring | **Houdini FX22.0.459 production build**, SOP modeling + sparse Pyro + VDB, Karma CPU references, one Thrum cook | SideFX production release1Oct2026; Nubis³ uses fluid/model-derived voxels; Smolder2026 uses offline volumes from Houdini/EmberGen. Tool/version is chosen; a licensed available seat is an operational prerequisite, not another tool study. |
| Animation/control | Offline authored finite world episodes, persistent event IDs, density/pose/wind tracks; linear adjacent optical-frame playback | Shipped Smolder volumes support baked playback; Epic5.8 animated SVT documents two frames plus interpolation weight and prefetch, but is Experimental, an implementation reference rather than shipping proof. Clips contain real structural evolution; no claim that Nubis detail scrolling supplies lifecycle. |
| Source | One sampled **3D optical volume** interface for every family, ordinary mip LOD and max-occupancy bricks | Nubis³ prepared voxels and production volume textures/SVT. Sc/St authoring can use columns, but exports3D density like other families. No required far-transfer codec or runtime implicit grammar. |
| Air | Hillaire transmittance/multiple-scatter/sky-view/aerial LUTs, RGB segment transmittance | Unreal atmosphere lineage; author/Bevy code references. Extra RGB aerial target is our explicit format adaptation. |
| Cloud light | Two-lobe HG, primary plus **one additional multiple-scatter approximation octave**, baked ambient accessibility and clear-sky incident radiance | Epic recommends one cloud MS octave for games; coarse baked occlusion has Smolder precedent. Authoring coefficients are look controls, not exact energy transport. |
| Shadow | Conventional cascaded **Beer shadow maps**, full-column lookup for receivers behind all casters; complete secondary Sun rays for interior queries | Epic documents both BSM and secondary ray marches. No arbitrary interior tail is inferred from a column summary; filtered map visibility remains approximate. |
| View |320x180 far and masked640x360 near every frame at720p; depth/motion-aware temporal reconstruction, spatial fallback | Production resolution/reprojection family in Nubis/Epic/Takram. Disjoint near-prefix/far-tail intervals compose once; no16-frame sparse refresh baseline. |
| Environment/reflections | Current-generation32² six-face cube -> SH9/roughness mips; independent320x180 reflected rays for low roughness | Epic sky capture and conventional independent reflection views. Cube capture is not a substitute for near parallax or actual water proof. |
| Fog/composition | One ordered RGB coefficient/affine-transfer integrator for overlapping air/fog/cloud | Standard volume transport and shared clear-air segments. No separately finished fog/air overlay over an already integrated cloud. |

Choosing mature methods is not a claim of accepted content. POCs now verify art, implementation, approximation settings
and cost. They are not competitions to invent the source model, light closure or far representation.

## Authoring: one executable chain

Use Houdini22.0.459 with a valid FX-capable production license. A compatible later22.0 production build may replace it
only with a recorded tool pin and regenerated output; no dependency on unproven22-only features. At first execution,
verify local installation/license and available disk. None has been installed/purchased by this plan. If unavailable,
report that operational prerequisite; do not quietly replace the pipeline with a new solver or another toolchain.

1. A Houdini project loads the episode's world metre/second frame, shared wind curves, fronts, event IDs, birth/death
   times and semantic precipitation/lightning tracks. Target four10-minute authored stories: FAIR, BROKEN, OVERCAST,
   STORM. These are finite proof/content profiles, not arbitrary weather forecasting or instant live retargeting.
2. Model/simulate each family's low-frequency changing structure using the recipes below. Apply cloud billowy/wispy
   detail in stable source coordinates, not final-image displacement. Author local source and loss, including mature
   replacement while older structure persists. Use full continuous Cu-to-Cb development from inception.
3. Evaluate **one shared world wind** during authoring. Export density in a moving event-local frame plus the exact
   event-to-world pose track; internal height shear/advection is already baked in the density. Runtime samples that
   pose and density and applies **no additional wind/shear**. Wind metadata and poses use the same world time as water,
   fog and semantic consumers. Unknown baked wind fails admission. Baseline playback is synchronized whole-episode playback; different cloud ages
   arise from authored birth times. Arbitrary runtime event phase offsets, cloning and wind/scale edits are unsupported.
4. Export time-ranged OpenVDB density/material fractions, pose/bounds, optional source velocity/correspondence and IDs.
   Initial runtime source cadence is4Hz with positive coefficient interpolation; compare against the24Hz authoring
   cache/reference. Increase source cadence where actual intermediate-frame cues demand it. This is sampling/art
   tuning, not a different animation algorithm. Keep one coherent local material frame across each frame pair.
5. Bake coarse **ambient accessibility** from the final episode density: at source sample x, average cloud-only
   exp(-tau) over16 fixed upper-hemisphere directions to the cloud-domain boundary, on a16³ local grid/frame.
   All episode cloud constituents are included, so anvil/deck occlusion is not invented at runtime. This conventional
   occlusion bake contains no Sun color or recursive scattering. Store a separate R8 stream keyed by episode revision, global time knot, event instance ID, complete occluder/pose
   revision and grid. It cannot be shared by master/frame alone or retained across independently retimed/cloned events.
6. Karma CPU supplies fixed-exposure source and light reference images/motion. It is an offline comparator, not runtime
   shading or a required GPU/hardware feature. Artist acceptance checks structure and motion, not just a solver label.
7. The single Thrum cooker maps VDB channels to fixed-calibration optical coefficients, builds mips/occupancy/pages and
   stages a complete manifest. Retain `.hip`, input curves/seeds, VDB or recoverable cook source, tool/cooker pins,
   reference rights, hashes and regeneration commands before admitting runtime output.

| Family | Concrete Houdini source recipe and required structural change |
| --- | --- |
| Cu | Animated localized emitter/temperature volumes -> sparse Pyro density/velocity -> altitude/base mask -> Cloud Billowy/Wispy shaping -> VDB. Replenishing emitters create new upper billows; old material advects/erodes. Not a global density ramp on a static shape. |
| Cb | Same persistent cell's taller sustained emitters, bounded cap and horizontal upper flow -> connected tower and spreading outflow density, fibrous anvil detail and separately authored core/anvil loss. A single continuous baked episode, no runtime branch solver or faded anvil disk. |
| Sc | Common shallow column/coverage field with locally animated source/drying masks and cellular organization; SOP Solver feedback with advected coverage and explicit local source-minus-loss masks -> Volume Wrangle vertical profile + relief/detail -> shared3D VDB tiles. Gaps close/open and cells renew. |
| St | Same SOP Solver/Volume Wrangle world-layer controls with weak relief and coherent sheet thickness; localized filling/thinning tears holes; export adjacent3D tiles from one field. No isolated puff ceiling or grey-screen opacity. |
| Ci | SOP Solver emits curves/points into upper wind and advects positions/age/width, stretch/fall and local age-dependent loss -> Volume Rasterize Attributes/VDB plus wispy detail. New heads feed old thinning tails. |

These recipes are ordinary authored VFX/content work, not a claim that a named shipped engine publishes this exact
five-family graph. The Sc/St control update is
`q_next=clamp(Advect(q,W_low,dt)+dt*(S*(1-q)-D*q+kappa*laplacian(q)),0,1)` with bounded timestep and spatial/time
source S and drying D; it is an authored coverage control, not atmospheric humidity. Cb liquid/ice fractions partition
one density across cap/core/anvil rather than adding a duplicate anvil. Ci emits new tagged points from persistent
heads while old tails continue advection and loss. Pyro is a smoke/velocity tool; masks and cloud shaping must produce cloud references, not smoke.
The SideFX Animated Cloud example is a reference setup, **not** our lifecycle source or a cache to download: its
advertised223GB cache is outside this campaign. First pilot: bounded source/motion intervals, peak scratch<=2GiB;
estimate/store full-episode masters separately before a larger bake. No bulk bake until storage/recovery is admitted.

Weather controls choose an admitted story/seed at load and play/pause/seek/rate on the **world** clock. Fronts, future
births and persistent survivors are authored together; their geometry/pose actually changes locally. This is not a
crossfade between rendered preset skies. Episode succession requires a shared authored boundary state/IDs and continuity
QA. Unsupported arbitrary midlife wind/branch edits require stop/recook/new admitted story; live response is optional.

## Cooked source and residency

The baseline format is ordinary sparse8³ interior bricks with one-voxel filtering guards,32-bit page indices and
**RG16F optical coefficients** (liquid/ice extinction). Scattering is each nonnegative coefficient times its declared
albedo; normalized phase lobes/material identity are metadata. Max-occupancy mips conservatively skip only certified
zero density. Regular optical mips serve footprint filtering; they do not claim exact filtered transmittance through
unresolved gaps. No custom four-Z layout, tau-tube compression, optical flow or coverage-conditioned transfer is required.

Each `(master, frame knot, mip, layout, story/wind revision)` density product is immutable and shared only where
identical. No per-age current3D field is required. Each event has a fixed-scale rigid pose M(t), translation Hermite
and rotation SLERP from the authored track. At world point X, compute xi=inverse(M(t))*X **once** and query
`q(X,t)=(1-alpha)*q_k(xi)+alpha*q_(k+1)(xi)` on the same local grid/material coordinates and mip. Residual structural
motion is already baked; the runtime adds no flow. Bounds include both endpoints and conservative swept pose support.
Compare residual displacement/interpolation at intermediate times against24Hz reference; increase cadence if needed.
Accessibility uses its separate complete-world keys and interpolates as an illumination approximation, not extinction. Exported velocity, where present, is a coarse RGBA8_SNORM field with a recorded scale/domain;
otherwise reject uncertain internal correspondence and use current spatial samples/reactive masks.

Select conventional **bounded frame-window streaming** from the start: coarsest admitted source frames first, requested
mips plus3 predicted future presentation-time frame pairs, scaled to1x/16x/64x. Fetch gzip-compressed binary page bundles
through the platform IO/decompress path into bounded workers/staging, upload prepared guarded pages, then publish a
complete source lease set. Three prefetched pairs do not establish throughput: the manifest reports unique requested
page bytes per presented frame at1x/16x/64x, IO/decode/upload latency and seek bursts. Skip frames never requested for
presentation; bound compressed/decode jobs and upload slots inside the staging reservation. Qualify sustainable
throughput plus the current-time coarse quality floor before admitting an episode. No novel GPU entropy decoder or full-animation residency assumption. Coarse fallback must
refer to the **current source time**, respect each consumer's declared quality floor and be counted; otherwise return
unavailable/missed-frame status. Never hold phase or silently replace a near cloud with an unacceptable low mip.

World-cell bounds index plus conservative screen/light tile candidate lists limit source tests; retain direct traversal
as correctness oracle. All consumers request their domains independently, including offscreen Sun casters. Far rays use
the same volumes at footprint mips with occupancy stepping. Full-sky source placement includes distant/horizon patches
in the same spherical world/episode; no painted far sky or camera-created cloud facts. No special far transfer bake.

Initial combined lifetime reservations are200MiB, now allocated as: encoded CPU+GPU copies48, physical decoded
source/guard/mip/accessibility/velocity pool64, bounds/index8, shadow/light8, view/history48, staging16, air/environment8.
These are pool objectives, not measured process headroom. Admission counts physical texture dimensions, all copies,
queued/old-new/fenced products, heaps and process/driver scope. The whole-browser target remains512MiB; no spare memory
is inferred by subtraction. Exact dimensions/cadence are quality/cost tuning within these selected mechanisms.

## Complete lighting and view path

**Air.**256x64 transmittance and32x32 multi-scatter are profile-only;192x108 sky view is Sun/height dependent;
32³ aerial source and a separate32³ RGB transmittance target are per view/Sun/profile. Use Rayleigh, Mie scattering/
absorption, ozone, planet/ground blocking and ground albedo as explicit profile assets. Above/top/ground/horizon cases
use stable boundaries; RGB aerial is a declared extension to the author's mean-alpha fast path. Space/limb travel is
outside the ground-sky profile. Clear-air angular mean radiance at cloud height is produced by64-direction quadrature
at8 altitude levels0/0.5/1/2/4/8/12/18km on Sun/profile change. Define
`L_sky_mean_upper=(1/(2*pi))*integral_upper(L_clear_sky dOmega)`, excluding the direct Sun disk, with no clouded-environment
feedback. The isotropic ambient approximation integrates this upper hemisphere over the normalized4pi phase domain.

**Cloud source.** For liquid: HG forward g=.8/back g=-.2 with weights.8/.2; ice starts .85/-.15 with weights.85/.15.
Each lobe is normalized. One additional scattering approximation octave uses contribution a=.5, optical-depth factor
b=.5 and phase-eccentricity factor c=.5; primary is i=0, additional is i=1. For each constituent,
`j_sun = sigma_s * E_TOA * T_air * T_fog * V_geometry * sum_i[a^i * P(g*c^i,mu) * T_cloud_i]`.
Cloud extinction in the view remains ordinary sigma_t; the extra octave changes source only. These are production
look parameters, not energy-exact higher-order transport. Ambient source is
`j_amb = sigma_s * 0.5 * L_sky_mean_upper(h) * (0.2 + 0.8*A_baked)`; the0.2 ambient floor is an explicit authored fill setting,
not a physical flux guarantee. No fitted source bank or runtime ambient-ray solver is required. Ground bounce is off.

**Sun/shadows.** Three conventional BSM cascades:256²/128²/128², Sun-perpendicular widths16/128/2048km, snapped and centred
on retained receiver/view demand. Caster depth bounds include **all** intersected episode sources, including upstream
ones outside main view. Each texel integrates up to32 occupied-interval samples at footprint mip and stores front
metre depth, **back metre depth**, full-column tau and validity in RGBA32F. The32-sample budget covers the entire
column, never just its first occupied intervals: insufficient sampling uses an admitted coarser quadrature/mip or
invalidates the texel and invokes the direct path; never truncate upstream casters. Use textureLoad and ordinary4-tap visibility
filtering; no optional float32-filterable feature or half-depth overflow. Evaluate exp(-b^i*tau) per column before
averaging; blend cascade visibility across overlap. Depth increases along incoming sunlight. A query behind every
valid tap's back depth receives the full-column map visibility; Sunward of all fronts receives1. Empty valid taps have
zero tau. Mixed/interior depth classification or missing map support takes the direct path below; never infer an
arbitrary interior tail from front/back/total tau.

For **any interior** cloud/air/fog lighting query, march the complete sample-to-Sun cloud path across every intersected
occupied source interval: initial8 samples per interval,32 total, occupancy skipping and footprint mips. This is a
conventional secondary ray, not a local prefix added to a BSM tail. It includes upstream/offscreen blockers. Both MS
octaves reuse this tau with exp(-b^i*tau). No cache/local double counting. Sampling is quadrature, not exact visibility;
when the cap is insufficient choose an admitted coarser sampling setting or report invalid, never omit blockers.
Receivers behind all cloud supports use cheap BSM. Shadowed air/fog and environment use the same classification/path.
Frequent direct paths or cap exhaustion must appear in the complete cost ledger and can fail a profile. Clear-air,
fog and geometry light factors apply once, separately. Filtered BSM cannot answer exact semantic point tau; diagnostic
direct rays remain the reference. Phase mu is dot(eye-to-sample direction, sample-to-Sun direction).

**Other Sun factors.** `T_air(x,Sun)` queries the common clear-air transmittance LUT along sample-to-Sun to the
atmosphere boundary, with planet blocking zero. `T_fog(x,Sun)=exp(-sum_v integral_ray sigma_t,fog,v ds)` is RGB and uses
the same bounded fog coefficient sampler as view transport: intersect all fog supports, analytic segment where its
profile permits, otherwise32 total quadrature samples distributed over the complete occupied support union. Sampling
insufficiency takes an admitted coarser/error-qualified setting or returns invalid; it never omits a volume. Geometry
visibility uses analytic fixture intersections initially and V3's common directional-shadow input in P8. Every direct
source/receiver/reflected sample uses these same-position/time factors once. Record fog ray work separately and charge
cloud-induced use in the cloud total. No finished fog image or inferred cloud darkness supplies this attenuation.

**View.** Intersect sorted cloud/fog/shadow supports and real geometry/top/planet terminals. Sum overlapping RGB extinction
and source before stable homogeneous Beer integration. Main far320x180, near-mask640x360 for cloud, fog **or cloud/geometry-shadowed air** intervals within
200m; output1280x720. Far integrates[200m,terminal], near integrates[0,min(200m,terminal)]; compose these disjoint
operators once. Only an unshadowed, cloud/fog-free near mask uses its clear-air LUT prefix; a terminal before200m makes the far operator identity.
Near/far boundary crossings invalidate affected history; no source interval is included in both. Use up
to64 occupied cloud samples (96 near), footprint mips, conservative zero skipping, blue-noise depth jitter and early
exit at RGB T<.005. Shadowed air/fog intervals use up to128 medium steps; compatible unshadowed clear intervals use the
LUT affine segment, with direct bounded fallback for ill-conditioned differences. For a fully clear/unshadowed ray, the sky-view LUT can replace the whole atmospheric integral; its separate Sun
disk uses clear-air T once. For a joint ray, the terminal at atmosphere top is space (zero here) plus **unattenuated**
Sun-disk radiance E_TOA/solar-solid-angle when intersected. The interval chain supplies all air/cloud/fog attenuation.
Never use a camera-origin full sky-view result as the far terminal behind already integrated air/clouds. No full-camera
sky/AP overlay after joint integration. Numerical caps never silently truncate a nonopaque ray: use a coarser admitted step/mip or report
invalid; reference/visual tests decide acceptable sampling.

Each view writes premultiplied RGB source, RGB T, depth moments, motion/confidence and IDs. Reproject with that view's
actual prior time/transform and exported pose/velocity;3x3 neighborhood clipping, current blend.15,2% depth disagreement
rejection. Birth/loss, invalid flow, cuts, seeks and high-contrast light changes receive current samples. No optical-flow
source reconstruction or shared main/reflection history. Spatial bilateral current upsample is the fallback; problematic
near silhouettes receive bounded native fresh samples. V4 applies one exposure/display transform and pre-exposure
for HDR storage; compensate history exposure changes, never scale T. FP16 stores pre-exposed radiance, not raw solar radiance.

The48MiB view pool has a concrete720p ledger. Each ray target set uses RGBA16F S(8B), RGBA16F RGB-T(8B), RGBA32F depth/
motion guide(16B) and R32Uint compact-ID/flags(4B):36B/pixel. Current+old/new history sets for320x180 far,640x360 near
and320x180 reflection total35.60MiB. Full1280x720 RGBA16F resolved color7.03MiB plus R32F geometry depth3.52MiB total46.15MiB;
remaining1.85MiB bounds fresh tiles/scratch. These targets are reused on one ordered GPU queue: current scratch plus
two ping-pong history sets, with final readers encoded before the next overwrite. Up to3 CPU-submitted graphs do not
allocate3 additional image sets; independent async queues or retained external image readers require additional
physical allocations and a revised ledger. Only within-graph optical consumers read these mutable view targets;
retained capture copies are separately bounded/accounted. No full-resolution optical history is hidden outside the pool. Guides use
float32 metre depth; textureLoad/manual filtering where required. Independent terminal/scene resources belong to the
actual fixture ledger too. Recount multiplicities if lifetimes/layout change; the ledger is a proposal, not measured peak.

**Environment and surfaces.** Capture all six32² cube faces from one current source/Sun/air/fog/geometry/anchor generation each frame,
up to32 occupied cloud steps per ray, using the same BSM-behind/direct-interior Sun classification; derive SH9 diffuse and roughness mips. Exclude the direct Sun disk from this lighting cube: the direct Sun
light owns surface diffuse/specular solar energy. Air/cloud-scattered solar radiance remains in the cube. This is low-frequency lighting. Its
terminals are space/ground/direct-only inputs, never the environment currently being produced. Surfaces consume that
clouded irradiance once, with no second cloud AO multiplier; direct Sun uses common BSM/air/fog/geometry once.
Independent mirror/low-roughness rays use their own320x180 projection/history and the same joint integrator from the
actual interface; the eye-to-interface operator applies once. The cube cannot qualify near parallax or actual rough
water; the existing P3b/P4b rough-reflection/depth proxy and P8 real-water/engine gates remain.

## Ownership, schedule and allowed tuning

E1/W1 owns admitted episode facts/time/wind; F4 Houdini/cooked provenance, admission and source leases; V1 pages/fences/
submission/device epoch; V2 independent view demand; V3 source/light/transport; V4 reconstruction/display. No cloud clock,
private final upscaler or gameplay optical readback. Main-hidden retains demanded shadows/environment/reflections;
last optical demand removed retires cloud GPU work while requested headless facts persist. Code boundaries follow the
migration module table; no speculative crate or copied prototype coordinator.

Frame order: select episode/world time -> acquire endpoint pages/poses -> profile/sky/air products -> BSM -> cloud
source using baked accessibility/clear sky -> current environment -> surface terminals -> main/reflected joint transport
-> independent reconstruction/interface composition -> atomic FrameOptics publication -> fenced retirement. Strict
current-generation reference is retained; source-independent tables and unchanged regions reuse exact keys. Bounded
queues/nonblocking observation/cold pipeline and worker rules remain in CLOUD-PERFORMANCE. Missing IO or work retains an
explicit old timestamp/missed-frame status; it never manufactures current density from a held source phase.

Cloud5ms objectives include source.45, shadows.55, cloud source.45, main1.90, reflection.35, environment.25,
reconstruction.65, reserve.40ms; unshadowed clear air/composition.8ms additional. All cloud-induced interior Sun rays,
shadowed air/fog integration and native fresh silhouette work are charged to the5ms cloud total, not hidden in the
clear-air allocation; report both the total and subaccount even if the target is missed. Native3/10 and browser CPU.5/full8 remain targets.
These are allocations, not predictions.98,304 BSM columns*32 and57,600 far rays*64 already produce millions of samples,
with local light, reflected and source-index multipliers counted. Qualify the entire accepted fixture on actual M2;
P7 standalone5/16/512 is not P8 engine/water admission. Beauty, lifecycle and consumer gates cannot be traded away.

Normal implementation tuning may change voxel/frame/mip/page counts, step limits, cascade sizes, material coefficients,
history blend and source detail within this architecture. Keep matched images/motion and complete-cost receipts.
A failure changes the responsible recipe, bug, capacity or quality setting first. An architecture change requires a
recorded contradictory observation and explicit revision; do not reintroduce an unselected experiment quietly.

## Optional experiments — not prerequisites

Runtime fluid/thermodynamic solver; new analytic cohort/lobe generators; arbitrary live wind/phase retargeting or
continuation graphs; runtime affine shear/optical-flow reconstruction;16-subray far transfer summaries; K48/deep tau
compression or interval accelerators; fitted liquid scattering bank; subgroup adaptive light sharing; neural/Gaussian
clouds; specialized BC/ASTC formats; time-sliced environment. None blocks or determines a required baseline POC.
Investigate only after a working baseline and a measured deficit show a useful gain. References/tests survive; failed
or optional mechanisms do not acquire required status because code already exists.
