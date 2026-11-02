# Selected runtime cloud and atmosphere baseline

2 November 2026. **Runtime design selected; implementation, visual quality and device qualification unrun.**
The [governing design](CLOUDS.md), [whole-atmosphere coverage](ATMOSPHERE-SYSTEM.md),
[replaceable provider contract](CLOUD-PROVIDERS.md) and single [build queue](CLOUD-MIGRATION.md) describe one system.
The preceding Houdini/baked-world-episode choice at13a19ad is superseded: it restricted the requested live weather.
No Houdini installation, commercial authoring licence or complete prerecorded lifecycle is required.

## Selection: demonstrated visual success first

**Visual reference: Horizon Forbidden West: Burning Shores / Nubis³.** Independent reviews specifically praise its
3D clouds; Guerrilla publishes a concrete voxel morphology/rendering/lighting account. This is a defensible reference,
not a universal reviewer ranking or proof of a private dynamic lifecycle. [Review and comparison](../../evidence/cloud-dynamic-revision-2026-11-02/visual-selection.md)
separates praise, disclosed mechanisms and our adaptation. A newer publication alone does not win. Public material
through2026 supplies no controlled equal-quality M2/browser comparison proving a single fastest technique.

**Default: native runtime-generated sparse3D optical fields, detailed and rendered with Nubis³-style economies.**
Keep real3D morphology, footprint detail, conservative occupancy traversal, shared voxel lighting and independent
reconstruction. E1 supplies live weather/fronts/height wind; a reduced art-directed source evolves3D support at runtime.
That source evolution is an explicit Thrum adaptation of standard transport/source-loss methods, **not** a recovered
Nubis³ algorithm or a shipped proof. Physical meteorology is unnecessary. A cheaper2.5D weather-map layer is a separate
reduced-distance/provider option; it cannot silently stand in for the selected close-cloud target.

| Decision | Selected mechanism / public support | Qualification |
| --- | --- | --- |
| Weather | Runtime regional fields/front controls, one committed clock/wind, persistent cell/layer IDs | Required live control response; no finite story/movie as authority |
| Source | Sparse3D liquid/ice optical proxy, prescribed-velocity transport plus local source/loss and genus recipes below | Standard numerical/VFX operators; exact five-family combination and morphology unproved |
| Inputs | Repo-native deterministic noise/masks/curves and recipe data, optional properly licensed imported static shapes | No paid app, external asset pack or full animated cache needed |
| Render | Prepared current3D source, bounded detail, occupied bricks/mips, mixed-resolution rays and independent histories | Nubis³ production lineage; actual shader/LOD quality and full work must pass |
| Light | Hillaire air, two-lobe HG/one extra MS octave, runtime voxel Sun/ambient caches with local Sun samples | Nubis³ slides125–127/147/154 support decoupled voxel light; our dimensions/cadence are qualified choices |
| Coupling | One RGB cloud/air/fog integrator, receiver BSM, current environment and independent reflected rays | Retain reviewed transport rules; no baked whole-scene AO |
| Substitution | F2 validates weather/source/light/air provider domains at load; private layouts remain private | No universal plugin ABI or seamless arbitrary hot swap |

At equivalent accepted images **and motion/behaviour**, prefer lower complete CPU/GPU/memory/IO cost. Shipped console
numbers are supporting context, not target predictions. If close3D quality cannot fit a tier, report an explicit reduced
profile or failed target; do not rename an inferior image an equal-quality optimization.

## Native content and live source path

1. Version plain recipe/control data, generator source and seeds in the package. Generate reusable Perlin–Worley/
   cellular base noise, erosion noise and directional strand masks with a deterministic repo-native tool; inspect
   channels/spectra/mips and actual cloud images. Initial base128³ RG8 and detail32³ RGBA8 have no lifecycle frames.
   These are reusable ingredients, not preformed clouds faded in/out. Optional imported3D shapes remain inputs under
   rights/provenance checks; none is a prerequisite or a fallback that makes dynamic response optional.
2. E1's weather provider owns regional authored-control fields (coverage/moistening, drying, lift, cloud base/top/cap,
   ice supply, storm activity) and world wind W(x,h,t) in m/s. Controls are normalized art parameters where not physical;
   no claimed humidity, water conservation or forecast. Front positions/strengths change at runtime. Initial fixed
   weather step1s; smooth controls/trajectories between committed ticks. Controls use bounded advection plus exact relaxation toward spatial front/region targets:
   `c_next=c_adv*exp(-dt/tau)+c_target*(1-exp(-dt/tau))`, tau>0. Fronts advance by their specified trajectory/velocity;
   c_target is the clamped sum/priority blend of identified front kernels and baseline regional recipe, not a sky image.
   Wind changes interpolate the shared vector profile under a declared slew limit. External changes enter the next world step.
3. The source provider owns sparse world-domain3D optical proxies q_l,q_i, stable region/cell IDs and recipe state.
   At initial fixed source step dt=.25s, backward-trace through prescribed velocity u and apply positive source/loss:
   use bounded MacCormack semi-Lagrangian advection: forward trace, reverse trace and corrected forward value, limited
   to donor extrema and reverting to the first-order traced value at invalid/unknown donors. This preserves more
   local detail than first-order advection; count its passes/scratch. First-order is the oracle/fallback, not an
   unrecorded lower-quality replacement. Name the limited corrected result q_adv. Each q_j is a normalized optical-amount
   control in[0,1], q_cap=1; sigma_j=kappa_j*q_j uses a fixed class calibration kappa_j(m^-1). It is not physical water mass. With k=S+D,
   `q_next=q_adv*exp(-k*dt)+(S*q_cap/k)*(1-exp(-k*dt))`, using q_adv when k=0.
   S,D are local nonnegative rates(s^-1); this solves dq/dt=S*(q_cap-q)-D*q for frozen rates after transport.
   Recipe/material calibration determines S/D masks and kappa, not q_cap or a global sky alpha. No pressure/thermodynamic/parcel solver.
4. u contains shared world wind **once**, plus explicitly visual internal updraft/outflow/fall terms. Weather consumers
   use W; cloud internal motion is not another world-wind authority. World-domain halo exchanges share one old snapshot
   and material coordinates across region/chart boundaries; no independent cloud boxes or duplicated overlap. Bound
   backtrace displacement by halo support and initial0.5source-voxel accuracy limit; substep if needed, never clamp to
   an edge or import a wrong neighbour. Activate bricks from the union of transported old support, its growth halo
   **and prospective nonzero S/front/emitter support**, including empty regions. A new emitter first creates its
   writable brick/neighbour halo before updating; missing capacity is unavailable, never silently no cloud birth.
   Rebuild conservative occupancy after reactions/transfer/detail bounds, before any light/view reader.
5. Localized base sources renew existing formations while older material moves and is erased by spatial dry regions,
   boundary mixing/loss and class decay. Under steady weather, source activation still renews: each admitted cell
   generates a bounded stream of seeded staggered emitter pulses at rate lambda= lambda0*moistening*lift (s^-1),
   with finite smooth rise/hold/fall windows. `S(x,t)=S0*c_moist*sum_e(a_e(t)*K_e(x))`; K_e is a compact textured
   **injection band**, not rendered cloud geometry. Windows overlap under persistent body ID; new pulses continue
   beyond the test trace. Expire pulse records and bound count; excess is explicit capacity failure. Local flank/dry
   masks vary independently, so new connected billows and old loss coexist instead of relaxing to a frozen Eulerian
   steady shape. S/D varies over3D structure and forcing; no whole-cloud scalar fade substitutes
   for breakup. Birth/survival/erasure diagnostics distinguish transported matter from new support. IDs identify source
   cells/layers and lineage; they need not name each visible noise lobe. Weather labels are not the changing density.
6. Stage a complete current optical source/occupancy snapshot before optical readers. Runtime detail uses a3-component region-local normalized rest-coordinate carrier (RGBA16F incl validity), advected
   with the same u at half linear q resolution (one-eighth voxel count), shared coefficients and bounded support.
   Halo exchange converts to a common chart before interpolation. Seed new support coordinates from the deterministic
   world recipe; excessive strain/folds/missing donors reset that carrier region and mark source/history reactive.
   Count carrier ping-pong/correction/mips/scratch inside source96MiB; no free motion field or independent per-view noise.
   Strong residual changes reject temporal correspondence. Every optical reader uses one V2 presentation time between
   retained source ticks, with positive current/previous coefficient interpolation. Interpolate valid previous/current
   unwrapped rest coordinates in the common chart at that **same** source fraction before sampling material detail.
   Invalid carrier correspondence uses the deterministic spatial recipe coordinates at this presentation time and
   marks the region reactive; every light/view query uses that same fallback. No wrapped-coordinate blend or separate
   detail clock. Presentation has at most one source-step lag. WeatherFrame revision/tick used by the source and control interpolation are recorded. A new
   control takes effect by the next weather+source step (initial<=1.25sim-seconds), plus<=.25s presentation lag.
   Missing prior/correspondence takes current spatial samples with honest time; no per-view source stepping. q is a visual optical state, not gameplay water mass.
7. Recovery selects an explicit policy at load. Default `reset-optics` preserves CPU weather/event IDs but, on a
   long all-optical-off interval, device loss or incompatible seek, starts a **new optical epoch**, initializes q from
   the current seeded genus/control field and invalidates all optical histories/caches. Publish unavailable/progress
   until ready; do not claim uninterrupted material continuity across this reset. Normal live play never resets to
   create required lifecycle. A `continuous-optics` profile instead retains the source update obligation even with
   drawing off and requires completed recoverable q/carrier checkpoints at<=60source-second intervals plus bounded
   forcing history of<=60s; on missed checkpoint/replay capacity report unavailable until recovered or explicitly
   reject continuity. Checkpoint readback is asynchronous off the authority/frame path and charged to16MiB staging;
   durable chunks follow F4 capacity/recovery policy. No optional checkpoint with an unlimited log/start replay promise.
   Headless weather needs no q/GPU. A headless **optical tau** requirement separately selects a CPU/analytic coefficient
   provider with its own accuracy/cost; weather potential cannot silently answer that query.

| Family | Live3D recipe and actual structural producer |
| --- | --- |
| Cu | Heterogeneous base emitters shaped by cellular masks under a common condensation-base profile; bounded positive vertical velocity carries new q upward, local mixing/dry masks remove flanks. Continued asynchronous sources replace older lobes during maturity. Fine billows arise from3D morphology/detail, not summed smooth ellipsoids. |
| Cb | Same-ID Cu source strengthens under storm/lift controls. Updraft slows under cap; a smooth radial divergent upper velocity spreads advected q into connected outflow. Height/temperature-proxy recipe partitions one liquid amount into ice (no duplicate anvil); core supply and anvil loss differ, leaving fibrous outflow after the core weakens. No inserted disk or arbitrary branch clip. |
| Sc | Shallow3D layer with cellular/roll-organized localized source/drying, common base, varied top; horizontal W advects banks and local S/D closes/opens gaps. The actual3D field retains thickness and connected relief. |
| St | Same persistent low-layer source with broad moistening and weak relief; local drying penetrates/open holes and moist fronts reconnect existing banks. No separately inserted sheet or grey overlay. |
| Ci | Ice-channel supply along persistent seeded high-level ribbons/heads; anisotropic strand masks orient to shared upper wind/shear. Advect emitted ice with height-dependent W plus prescribed fall; source continues at heads while old tails advect/sublimate under local D. Curved high fibres are3D, not opaque bars or flattened Cu. |

For Cb use an overlapping cap band[h_cap-d,h_cap+d], d>one vertical source cell: updraft remains positive into its
lower half and fades to zero only at the band's upper edge. Horizontal outflow ramps through the same band with
`u_h=W_h+v_out(h)*r/sqrt(dot(r,r)+r0^2)*radial_falloff(r)`, r0>0, avoiding an undefined axis. Existing arriving q therefore
reaches the outflow before vertical shutdown. After transport/reaction, k_freeze is a nonnegative band rate weighted
by upward supply; transfer `delta=min(q_l,1-q_i,q_l*(1-exp(-k_freeze*dt)))`, subtract delta from q_l and add it to q_i at
the same cells. Outflow then carries that ice away. No new disk source independent of arriving tower matter. Conversion
preserves the normalized proxy sum, while distinct kappa_l/kappa_i may change calibrated extinction; this is not water
or radiative-energy conservation. Initial masks/emitter spacing, rise/outflow speeds, loss rates and genus profiles
are authored recipe parameters. They must be pinned before the pilot and corrected against references, not accepted
because the operators sound physical. [Source review](../../evidence/cloud-dynamic-revision-2026-11-02/source-review.md)
retains the alternative study and limitations. Exact lifecycle success remains the earliest hard visual gate.

The four FAIR/BROKEN/OVERCAST/STORM scenarios are reproducible **forcing tests**, not finite prerecorded skies.
Perturb wind/lift/drying/storm strength mid-development; existing state must respond within declared update latency.
Run beyond the scripted trace and inject a new front without loading a clip or resetting survivors. At16x/64x retain
fixed update semantics and report work/backlog; never skip required state updates or relabel a held image. Accelerated
preview can miss presentation deadlines, explicitly reported; the primary5ms admission is normal1x, with preview
throughput/controls separately measured. No unlimited catch-up loop may block input.

## Source representation and cost

Use sparse8³ interior RG16F liquid/ice optical-proxy bricks with guards and32-bit pages, plus max occupancy/mips.
sigma_t is the nonnegative calibrated q times bounded material detail; scattering uses constituent albedos/phase.
Detail is zero outside certified support. Occupancy bounds include every possible fine contribution; distance/empty
skipping never relies on stale q. Fine current detail is evaluated once by the common query or materialized only when
all-reader cost justifies it. No full current field per age or per view. Region ID/coordinates are world-owned; view
requests choose work/quality within a declared error domain, not source placement or weather history.

Current/past/in-flight source products and halo/guard/mip/occupancy/update scratch count physically. Initial200MiB
lifetime objective is now: dynamic source96, reusable noise/static GPU resources8, CPU weather/cause/control state8,
bounds/index8, light8, views/history48, staging/checkpoint16, air/environment8. Source admission uses actual allocated
voxels*bytes*versions plus guards/mips/scratch;96MiB is not96MiB of useful base voxels. Immutable static data can be
regenerated; no retained CPU copy is free. Measure complete512MiB process peak and all old/new/fenced overlaps.

Initial source-cadence/work model is O(active physical source voxels*source steps + emitter/control visits), plus
occupancy/mips, all optical consumers and live lighting below. For example2M useful voxels*4B*5 state/scratch copies is
40MiB **before** halos/mips/extra reader overlap, leaving a bounded margin within96; it is a planning example, not an admitted full
sky. At64x the source tick multiplier is real work. Fixeddt, source/recipe spatial resolution and active-domain quality
are tunable only while actual motion/coverage/coupling pass. No animation-frame streamer or all-episode AO stream is
required. Static inputs use normal F4 content IO; changing weather uses bounded updates and dirty source/light products.

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
`j_amb = sigma_s * 0.5 * L_sky_mean_upper(h) * (0.2 + 0.8*A_current)`; the0.2 ambient floor is an explicit authored fill setting,
not a physical flux guarantee. No fitted source bank is required; current ambient accessibility is provided by the shared runtime light cache. Ground bounce is off.

**Sun/shadows.** Three conventional BSM cascades:256²/128²/128², Sun-perpendicular widths16/128/2048km, snapped and centred
on retained receiver/view demand. Caster depth bounds include **all** intersected world sources, including upstream
ones outside main view. Each texel integrates up to32 occupied-interval samples at footprint mip and stores front
metre depth, **back metre depth**, full-column tau and validity in RGBA32F. The32-sample budget covers the entire
column, never just its first occupied intervals: insufficient sampling uses an admitted coarser quadrature/mip or
invalidates the texel and invokes the direct path; never truncate upstream casters. Use textureLoad and ordinary4-tap visibility
filtering; no optional float32-filterable feature or half-depth overflow. Evaluate exp(-b^i*tau) per column before
averaging; blend cascade visibility across overlap. Depth increases along incoming sunlight. A query behind every
valid tap's back depth receives the full-column map visibility; Sunward of all fronts receives1. Empty valid taps have
zero tau. Mixed/interior depth classification or missing map support takes the point-cache/direct path below; never infer an
arbitrary interior tail from front/back/total tau.

**Voxel lighting.** Retain the complete32-sample sample-to-Sun ray as the reference/fallback. The production path
decouples its expensive tail from view rays, as Nubis³ slides125–127 describe. Each current-generation light-cache
voxel traces its **complete** cloud path to the source/atmosphere boundary, including offscreen/out-of-grid blockers,
and stores T0=exp(-tau), T1=exp(-.5*tau). At a view sample integrate2 local Sun segments over200m, then sample the3D
cache **at that endpoint** and use `T_cloud_i=exp(-b^i*tau_local)*C_i(endpoint)`. The cache stores a true point-to-boundary
approximation, unlike the rejected2D-column interior tail. Prefix and cached tail are disjoint. Never add a full-column
BSM to the local prefix. Initial three nested world-aligned light domains have32³/16³/16³ cells and widths16/128/2048km,
with vertical/depth support covering admitted cloud heights and actual Sun rays. Rebase changes cache coordinates,
not cloud facts. Domain unions serve all retained views/receivers; unavailable/outside or invalid corners use complete
direct rays. No missing blocker is treated as clear. Linear interpolation filters per-octave T, not tau.

In the same RGBA16F cache, store ambient accessibility A as the mean of4 fixed upper-hemisphere complete cloud rays,
initial32 occupied samples/ray. Use16+direction direct quadrature as the reference, not a full-world bake. Fourth
channel marks validity; interpolation requires valid contributing corners. These are coarse illumination approximations
with same-source optical/image tests, including near surfaces, thin gaps, deck-over-Cu and low Sun. Cache error or
frequent direct fallback fails its quality/cost gate; ordinary cache resolution/local-step tuning is allowed.
Build current caches before any main/reflected/environment consumer; keys include complete relevant source revisions,
world time, domain/anchor and Sun for directional channels. Source-only ambient can omit Sun from its own key. No
8-frame lag is presumed safe for changing weather: a later slower cadence needs explicit age/error qualification.

Receiver positions behind all map casters keep cheap cascaded BSM. Interior air/fog/environment uses this same
point-to-boundary cache/direct path (without a duplicated local prefix). Geometry/fog/clear-air factors multiply once.
Exact semantic point tau cannot come from filtered caches; direct queries remain a separate fidelity. Phase mu is
dot(eye-to-sample direction, sample-to-Sun direction). The cloud5ms account includes cache builds and fallbacks.

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
up to32 occupied cloud steps per ray, using the same BSM-behind/point-cache-interior Sun classification; derive SH9 diffuse and roughness mips. Exclude the direct Sun disk from this lighting cube: the direct Sun
light owns surface diffuse/specular solar energy. Air/cloud-scattered solar radiance remains in the cube. This is low-frequency lighting. Its
terminals are space/ground/direct-only inputs, never the environment currently being produced. Surfaces consume that
clouded irradiance once, with no second cloud AO multiplier; direct Sun uses common BSM/air/fog/geometry once.
Independent mirror/low-roughness rays use their own320x180 projection/history and the same joint integrator from the
actual interface; the eye-to-interface operator applies once. The cube cannot qualify near parallax or actual rough
water; the existing P3b/P4b rough-reflection/depth proxy and P8 real-water/engine gates remain.

## Ownership, schedule and allowed tuning

E1/W1 owns live weather causes/time/wind and persistent event identities; the selected source provider owns its optical
state/evolution without inventing weather authority. F2 registers/validates the finite recipe; F4 content/checkpoint
provenance/admission/leases; V1 GPU resources/queue/fences/device epoch; V2 independent view demand; V3 source/light/
transport; V4 reconstruction/display. [Provider examples and lifecycle](CLOUD-PROVIDERS.md) specify actual combinations.
Build internal units from the migration table; do not create speculative crates or copy the old viewer coordinator.

Frame order: committed weather/commands -> fixed source update/halo/material conversion -> complete source/occupancy
snapshot -> air/profile products -> receiver BSM + current voxel light -> environment -> surfaces -> main/reflected joint
transport -> per-view reconstruction/display -> compatible FrameOptics publication -> fenced retirement. No camera
advances source state. Main-hidden retains demanded shadows/environment/reflections; no optical demand under reset-optics retires optical
GPU work while requested E1 weather/causes persist; continuous-optics retains its charged source update obligation. Source checkpoints/seek/recovery rebuild under explicit epochs,
bounded queues and source-time receipts. Missing work reports old timestamp/missed deadline, never invented current state.

Normal1x cloud5ms objectives: source/update.65, shadow/light-cache1.0, main1.65, reflection.35, environment.25,
reconstruction.65, reserve.45ms; unshadowed clear air/composition.8ms additional. These are allocations, not predictions.
Source weather CPU.5/full8ms, browser totalGPU16ms/process512MiB and native cloud3/full10ms remain unrun targets.
Three light domains total40,960 voxels; one32-step Sun +4x32-step ambient rays imply6,553,600 coarse source samples
before skips, plus local2-step view Sun work, map, main/reflection/environment. This explicit count must be measured;
none is hidden in the clear-air budget. P4a screens total work before libraries; P7 fixture is not P8 engine admission.

Tune resolution/cadence within fixed-step/error rules, brick/mip/step/cache counts, look coefficients and history
weights against matched native images and personally watched normal/accelerated motion. Required runtime controls,
3D family cues and coherent consumers cannot be traded away. Architecture changes need contradictory evidence and an
explicit revised selection. Code/asset sunk cost is not a selection criterion.

## Optional experiments — not prerequisites

Full physical fluids/parcels/thermodynamics; Houdini or another paid content package; full animated VDB episodes;
imported hero morphology; deep/interval shadow compression; custom far-transfer codecs; neural/Gaussian clouds;
hardware-specific adaptive light sharing; specialized BC/ASTC; source optical flow; time-sliced stale environment.
Static SDF acceleration is an optional optimization after conservative occupancy works; a dynamic SDF must be rebuilt
with safe distance/error bounds. These do not determine required baseline POCs. Prepared sources may be alternative
providers, but cannot satisfy a profile requiring arbitrary runtime weather response unless they actually provide it.
