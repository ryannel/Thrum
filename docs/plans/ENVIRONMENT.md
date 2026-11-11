# Environment — proposed module-family design

**Initial implementation specification:** [PLAY-ENVIRONMENT-SPEC.md](../PLAY-ENVIRONMENT-SPEC.md) now selects the concrete baseline mechanisms,
limits and failure rules. This family document retains the wider responsibility; specification does not imply runtime qualification.

Status: analytical architecture proposal, 31 October 2026, with the A04/A05 selections reconciled on 1 November in [architecture decisions](../ARCHITECTURE-DECISIONS.md). No environment provider has been implemented, profiled or visually accepted for this engine. This document commissions detailed modules against the [architecture brief](../ARCHITECTURE-BRIEF.md) and [rendering contract](RENDERING.md); sources and their limits are in [the evidence ledger](../research/RENDERING-SOURCES.md). It is deliberately independent of the existing water implementation. A custom engine that uses each device's hardware, modern native desktop/Mac, browser and mobile app profiles, AI-assisted creation, and flexible 3D cameras are the governing requirements. Structural choices below follow from conservation, transport, scene geometry and ownership; experiments validate predicted error and hardware cost rather than discover a foundation.

Detailed water planning is in the [water commission](WATER.md). It preserves the selected initial operators
while carrying the wider interaction, connected-transfer, whitewater and optics obligations into D2. No new runtime
or family-wide qualification follows from that alignment. The [sea and cloud port decision](../decisions/SEA-AND-CLOUD-PORT.md)
selects the spectral sea profile and weather-map volumetric clouds over the E1 cloud source, mapped from the Broadsides
Rust work, with hardware and frame-contract obligations; implementation follows V1. The only V1 environment work is
the [G3s](../../BUILD-PLAN.md#active-gate-order-and-budgets) physical sky, sun and aerial perspective under a time-of-day control.

## Required outcomes and separation

The engine can construct outdoor and indoor 3D worlds and selectively supply terrain, vegetation, sky/air, cloud shadows, weather/wind, precipitation, water and persistent surface response. Their effects agree at a place and time, even when one visible representation is absent or lower fidelity. A game can use a cloud-shadow field without drawing clouds; a room can use exterior daylight through windows without rendering a full outdoor sky. A desert or dungeon need not allocate a water solver. Feature composition validates retained consumers and substitutes, rather than copying or forking one “environment state” per view. The top-down, third-person and first-person cases are peers.

| ID | Required experience/contract | Owner; proof |
| --- | --- | --- |
| E1 | One named time, Sun/Moon or authored-light and weather source feeds lighting, wind, precipitation, cloud state and ambient audio/gameplay queries at declared fidelity | Environment coordinator plus source providers; time-changing scene |
| E2 | Terrain/material regions and streamed world content stay aligned with physics/nav/render identities | W2 owns construction/activation; surface module owns changing surface state; streamed crossing |
| E3 | Vegetation has authored placement, LOD, deformation and interaction without inventing its own wind/light | Vegetation module consumes wind/physics, publishes render/obstacle products; moving forest |
| E4 | Sky/air/cloud appearance and light/shadow effects can be independently selected, with coherent occlusion and colour | Atmosphere/cloud providers with V3/V4; hidden-cloud top-down and exterior |
| E5 | Rain/snow/fog/wetness and water contact change local scene surfaces, effects and queries without double counting | Weather/surface/water plus V5/P3; storm/shore |
| E6 | Distinct sea, lake, river, puddle and no-water worlds can select dynamics and appearance separately | Water region/dynamics/query and render adapters; contact scenes |
| E7 | Off and reduced-quality paths save real work/residency, keep gameplay state, and migrate source states at supported transitions | F2 plus each provider; profile trace |
| E8 | Human and AI authoring has causal controls, reproducible scenes and measured feedback instead of hidden shader-only tuning | T1/T2 and cooked schemas; edit/cook/compare loop |

These are engine capabilities, not a requirement for every game to enable every phenomenon. Realistic physical weather and all ten cloud genera are a specific Broadsides game direction in [DESIGN](../../provenance/CONTEXT.md#source-779e25e8668e), not the mandatory lowest common denominator for all engine users. The reusable contracts allow a game-specific high-fidelity weather provider. No universal atmosphere/hydrology/vegetation solver is proposed.

## Authority and the causal chain

**Single source, explicit consumers.** W1/F1 owns world clock/region identity; an environment source owns a versioned `EnvironmentFrame` at simulation time `t_sim` with location/altitude and validity region. It may be authored constant, keyframed, data-driven weather or simulated meteorology. A game selects exactly one authority for a quantity within an overlapping spatial/time domain; where local volumes or overrides apply, composition order and priority are authored and visible. It produces source facts, not every image: celestial direction/irradiance; wind vector field `u(x,y,z,t)` in m/s; air temperature K, pressure Pa, water-vapour/aerosol/cloud-related fields where the chosen provider supports them; precipitation flux/type; ambient boundary conditions; and discontinuity/version markers. Unknown quantities are declared unsupported rather than fabricated as physically coupled. A simpler provider may directly author sunlight attenuation, fog extinction or wetness rate without a meteorological model. It must label those as authored approximations. The current forest review found independently authored humidity and haze; that is a useful warning against claiming one physical cause where controls remain separate ([foundation review](../../provenance/CONTEXT.md#source-17ab82cb1175)).

**Update order.** At world step start, W2 activates region content and persistent deltas. F1 advances the selected environment source once; weather/cloud/wind source fields are sampled or updated at their own documented cadence. Consumers integrate their authoritative state: water dynamics, precipitation deposits/evaporation, vegetation interaction and physics forces, then commit one timestamped snapshot. Render representations read/interpolate this snapshot. Slow cloud/aerosol fields may update less often than camera frames, with `sample_time` and interpolation policy; they may not silently run at each view's frame rate and drift apart. Feedback that changes weather belongs to the next source update, not a render pass. Persistent wetness/snow and water contact state survive stream-out through W2/P7 ownership; sky LUTs and visual cloud histories do not.

**One cause may have multiple representations.** Cloud optical depth in the Sun direction may be represented by a cheap authored 2D attenuation field, a layered cloud model, or a derived map from a volumetric cloud field. Visible cloud pixels and shadowing can select different fidelity, but when derived from the *same* source they must agree in world position, time, direction and broad mass/coverage. The visible path can be off while the light attenuation path remains. Conversely, a decorative cloud image with no causal shadow is allowed only as a labelled art-only presentation. Epic's [volumetric cloud documentation](https://dev.epicgames.com/documentation/unreal-engine/volumetric-cloud-component-in-unreal-engine) uses ray-marched and Beer shadow-map choices with different accuracy/reach, illustrating a provider tradeoff. Its platform table lists no mobile volumetric-cloud rendering, so mobile needs a separately qualified visible representation, not a promise to downsample the desktop one.

### Selected bounded authored source and light handoff

The authored profiles select one authority per quantity: `constant-light-v1` (schema 1, the R0 Sun,
isotropic ambient and independent background) and proposed `authored-vertical-rain-v1` (schema 2).
The constant staged-source proof does not qualify a runnable build; schema 2 remains refused by F3.
The constant profile has no clock, wind/rain/cloud grid, sky draw, GPU resource or weather job. Rain adds
prescribed phase maps, wind, periodic cloud optical depth, vertical rain and optional gas facts; fog is separate.
The [recipe](../research/ENVIRONMENT-IMPLEMENTOR-DOGFOOD-2026-11-02.md) fixes the fixture/operators;
[schema-2 proof](../research/E1-AUTHORED-RAIN-F3-SCHEMA-2026-11-03.md) retains proposed decisions and spelling.

| Source quantity | Producer and semantic handoff | Retained consumer rule |
| --- | --- | --- |
| Sun | E1 drives the one W1 light identity with geometric receiver-to-source unit direction, normalized linear-Rec.709 photometric colour, raw perpendicular illuminance in lux, angular radius, source epoch and valid time/support. No cloud, air, terrain or geometry attenuation is baked into this value. | V3 forms each direct source-to-receiver path and applies its selected geometric visibility and named cloud/air/fog factors once. P3/thermal consumers receive separately declared radiometric W/m² when required; RGB lux is no heat authority. |
| Ambient and background | The R0 authored ambient is isotropic incident diffuse irradiance in RGB lux; terminal background is independent radiance in cd/m². The first fixture uses `[1000,1000,1000]` lux and black background. | V3's R0 Lambert fill uses `E_ambient/pi` with no second cosine or cloud factor. It does not invent environment specular or an HDRI. V4 alone converts scene light to display. |
| Wind | E1 publishes point velocity `u(world_y,t)` in world m/s from the one layered, phase-driven source; requests carry sample height, world time, support and source epoch. | Vegetation, P3, water and audio name their own sample/filter/response, but do not privately advance a wind clock or add an independent prevailing-wind authority. |
| Cloud | E1/F4 owns nonnegative periodic nodal vertical optical depth and its world-space advection displacement. A source-ray query below its plane returns path `tau_cloud`, receiver/source direction, approximation tag and validity; V3 computes `exp(-tau_cloud)` once. | A shadow-only projection may stay resident with visible clouds off. The first thin-plane model returns a tagged authored clear fallback below Sun `s_y=0.1`, fades to its ordinary projection by `s_y=0.2`, and refuses above-plane receivers. It supplies neither a 3D density nor a terminal sky. |
| Precipitation | E1 publishes liquid/solid **cell-mean horizontal** flux in kg/(m²·world s) and exact identified interval mass in kg through the selected first-hit exposure product. The static footprint does not advect with the cloud map. | Surface/water owners receive each interval once by stable receiver identity and commit mass/ledger/cursor with X[n+1]. Visual particles and audio sample the same source but cannot create or erase physical mass. Moving wet support uses C09's separate bounded partition. |
| Air facts and optics | Optional temperature K, local absolute pressure Pa and vapour gas mass fraction are independent source facts; RH/dew point are diagnostics derived from them. Optional air/fog coefficients declare their own units, support, source epoch and gameplay versus presentation role. | Missing thermal irradiance, pollutants, fog, Moon or terminal sky is typed unsupported. V3 jointly integrates overlapping optical coefficients under C08; gas facts alone do not synthesize haze, heat or breathing exposure. |

F1 maps integer ticks to world seconds and advances each authored phase from
its piecewise constant rational rate. A held phase freezes its controls, not
physical time or cloud motion under a held nonzero wind. A pre-tick E1 plan
binds one source revision, control epoch, interval `[n,n+1)`, wind integral and
exposure product. P3 reads committed X[n]; E1 stages exact interval receipts
and response candidates; W1/F1 publishes the source endpoint, receivers and
cursors together as X[n+1] or publishes none. Two views extract from the same
committed pair and may sample an admitted presentation time without advancing
the source. No view-local cloud or weather phase is authoritative.

Every point/interval query reports quantity, units, spatial and temporal
support, source/product revision, sample/valid interval, authority role and a
typed unavailable/outside/stale result. Required consumers refuse missing or
mismatched input before staging effects. F2 may bind a separately admitted
substitute; absence never silently becomes calm, clear, zero rain or a plausible
temperature. Live rain gating is a discrete pre-tick control published with the
next commit; editing source tracks or geometry remains source admission and
restart work. Source-off stops new input but preserves existing surface/water
stores and response tails; each disabled interval still advances a zero-mass
receipt/cursor with the same world commit. A visible-cloud toggle releases only presentation
work if source-ray attenuation remains required. Headless execution prepares
no HDRI, GPU cloud map or view history. The first bounded storm has at most
32,768 static exposure rows per tick (1,966,080 row visits/s at 60 Hz) and
under 0.8 MiB of proposed CPU source/exposure product payload before allocator
and old/new overlap; these are workload estimates, not measurements or
engine-wide limits.

### Selected first F4 constant-light product and binding

The first F4 slice cooks **one immutable CPU `ConstantLightProductV1`** from
an authenticated, pinned admitted-source lease. That lease supplies the
selected committed authority root, verified `SourceRevision` and manifest,
project ID, and hash-checked immutable record access. A provisional staged
candidate may exercise the identical cooker for a labelled proof, but cannot
authorize a ready build. F4 never reads the mutable checkout. The selected
closure is the sole entry scene's schema-2 `environment.source` → authored
environment schema 1 → `sun.light_ref` light identity schema 1. F3 checks the
complete declared source namespace; F4 cooks only this reachable active
closure. An unselected source stays valid content without a light product,
environment tick job or GPU allocation.

The product carries the stable environment and light UUIDs; normalized f64
world-space geometric receiver-to-source direction; normalized nonnegative
linear Rec.709/D65 photometric RGB colour with `Y=1`; raw nonnegative f64
perpendicular Sun illuminance in lux; f64 angular radius in radians; three
f64 isotropic ambient irradiance channels in lux; three f64 independent
background-radiance channels in cd/m²; and the schema's exact seven unsupported
quantities. It is constant over all positions and times. It contains no tick,
view, world epoch, exposure/pre-exposure, visibility, cloud attenuation,
weather state or GPU handle. Angular radius is retained as authored geometry;
R0's fixed four-tap shadow filter does not claim a physical penumbra from it.

Cook once from the F3-owned typed decoder over the lease's immutable bytes.
First apply schema-1 source validation, then compute direction length using
the ordered f64 expression `sqrt(((dx*dx)+(dy*dy))+(dz*dz))` and divide each
component by that length. Compute colour luminance as
`((0.2126*r)+(0.7152*g))+(0.0722*b)` and divide each channel by it. Replace
every zero result with positive zero. Recheck finite, nonnegative colour and
`1e-12` unit-length/`Y=1` residuals in the cooked result; refuse an invalid
result rather than clamping. The cooker implementation/evaluator revision,
compiler, target and floating-point mode are part of its cache class;
cross-target bit identity is not assumed until independently proved. The
same accepted decoded numeric value, including a signed-zero spelling,
must yield the same cook key and product bytes within one pinned class.

Serialize the product explicitly as **160 bytes**, not a Rust struct layout:

| Byte offset | Fixed little-endian representation |
| --- | --- |
| 0–7 | ASCII `THRME1C` followed by one zero byte |
| 8–9 | `u16` format version `1` |
| 10–11 | `u16` reserved flags `0` |
| 12–15 | `u32` unsupported mask exactly `0x7f`: bits 0–6 are wind, cloud tau, precipitation, air, thermal, Moon and air quality, respectively; all higher bits zero |
| 16–31 | Environment UUID, 16 octets in canonical displayed hexadecimal order with hyphens removed |
| 32–47 | Light UUID, same octet order |
| 48–71 | Three IEEE-754 binary64 direction components, X/Y/Z |
| 72–95 | Three binary64 normalized linear Rec.709 colour channels, R/G/B |
| 96–103 | Binary64 raw perpendicular illuminance in lux |
| 104–111 | Binary64 angular radius in radians |
| 112–135 | Three binary64 ambient-irradiance channels in lux, R/G/B |
| 136–159 | Three binary64 background-radiance channels in cd/m², R/G/B |

Decode requires exact length, magic, version, zero reserved flags, exact mask,
valid RFC-4122-form UUID octets, finite **signed** direction components,
finite nonnegative colour, illuminance, radius, ambient and background,
canonical positive zero and the cooked normalization residuals above. A wrong or corrupt
product is unavailable, never accepted as a stale but plausible Sun. The
serialized payload is 160 bytes; allocator residency, leases and old/new
overlap are measured separately.

Keep three identities separate. `BlobHash = SHA-256(the 160 product bytes)`.
The domain-separated `CookKey` hashes a canonical recipe of decoded **known**
source values after signed-zero normalization, source/light UUIDs, fixed light
driver binding, environment and light built-in definition hashes, product
format, cooker/normalizer/evaluator revisions and the declared toolchain/target
cache class. A product manifest binds that key to `BlobHash`; its own digest is
the product revision. The build binding pins the exact admitted
`SourceRevision`, project/entry-scene/source/light IDs, scene definition hash,
profile/target lock and product revision. Labels, documentation, opaque fields,
file path and unrelated source content do not enter `CookKey`; a change to any
of them still changes the exact source revision and produces a new binding.
The cache hit is verified against the product manifest and bytes before that
new binding is published. Changed light values, identity, fixed definition or
cooker class invalidate the key. Rebinding a scene can change only the build
binding if its selected product is already valid. A wider `BuildRevision`
includes this binding alongside its other required products; this product
revision alone is not a runnable build.

The selected constant-light product performs no per-tick evaluation or
allocation, weather-grid work, interval receipt, cloud upload or view-local
advance. The first fixture reads one scene, one source and one light from a
shared build lease, decodes bounded source bytes once, does constant-count
normalization/hash work and emits 160 payload bytes plus a small binding.
The full manifest/index acquisition cost is shared across the build, never
repeated per light. Two views borrow the same immutable CPU product. V3 applies surface
cosine, geometric shadow and later named cloud/air/fog factors on their own
paths exactly once; V4 owns display conversion. F2/V1 must separately refuse
f64→f32 overflow, an incompatible radiance/pre-exposure envelope or missing
GPU pipeline before R0 view readiness. A failed cook, stale async result or
source edit cannot mix a new Sun with old ambient/background; the prior
identified ready build remains until a complete replacement is published.

### Selected mathematical relationships and domains

The fixed Earth profile uses logical 256×64 transmittance (T) and 512×32 multiple-scattering (M) charts.
Two RGBA16F planes carry eight wavelengths through T, M and ground irradiance. A separate T plane projects the unit
Sun to Rec.709 for the disk and receiver-local lookup, including shell entry. The CPU meter uses the same spectrum.
True-height planet blockage precedes the ~6.151 m first-row remap. Packed planes add 385 KiB. Lit steps sample two
T and two M planes. M's two banks integrate 20,971,520 volume samples per profile refresh. Final sky and aerial
in-scatter project after transport with luminance-preserving gamut compression. Aerial surface attenuation uses
three bounded representative bands for arbitrary RGB. The M chart test isolates interpolation from closure error.

G3s sky diffuse uses a full-sphere nine-coefficient irradiance polynomial, including the planet boundary but
excluding the separate Sun disk. With `M0=∫L`, `M1=∫Lω`, `Q=∫L(ωωᵀ−I/3)`, evaluate
`E(n)=M0/4+n·M1/2+15 nᵀQn/32` on the final world shading normal. Keep signed coefficients; clamp only the
final reconstructed irradiance. The kernel differs from clamped cosine by at most `3/32`, giving absolute
channel error ≤`3M0/32` plus quadrature error, not a universal relative tolerance. Compare independently
against direct cosine integration of the same filtered LUT before promotion; transport/spectral error is separate.
Keep the 256-sample horizontal meter independent so SH truncation does not drive exposure. A private 144-byte
tail follows the 64-byte light prefix; Lambert `/π`, AO, solar colour/lux and storage scale apply once. One cached diffuse workgroup
adds 1024 samples: horizon-split GL16 in squared μ-distance ×32 azimuths, ~10 KiB shared storage, no new pass,
readback or per-pixel atomics. Fixed authored ambient is unchanged. Sixty-four normal probes record negative raw
RGB risk (count/minimum, not affected pixels); exposure flag16 is informational and never changes gain/history.
This follows [Ramamoorthi/Hanrahan](https://cseweb.ucsd.edu/~ravir/papers/envmap/), re-derived without copied code.

G3s specular selection uses the same disk-free Sky LUT: a 64² six-face RGBA16F cube, seven roughness mips,
`r=.045+.955*m/6`, and a 65² endpoint-inclusive DFG table. GGX split-sum assumes `N=V=R` in prefiltering;
one camera-height environment lights nearby objects, with no local geometry reflection. Start with 128 GGX
sample attempts per cube texel (normalize accepted positive weights) and 1024 VNDF samples per DFG texel,
matching correlated Smith and `alpha=r²`.
Prepare the 896 invariant local GGX directions/weights once per device in a 14KiB storage buffer;
changed-sky texels rotate/reuse them rather than repeat the same trigonometry.
With `s(x)=sqrt(alpha²+(1-alpha²)x²)`, VNDF weight is `NoL*(NoV+s(NoV))/(NoL*s(NoV)+NoV*s(NoL))` for
`NoL>0`; rejected samples count as zero. `A=<W*(1-Fc)>`, `B=<W*Fc>`, `Fc=(1-VoH)^5`; thus `A+B≤1`.
Atmosphere indirect uses `Es=F0*A+B`, specular `prefiltered*Es` and diffuse `(1-metalness)*base*(1-Es)*E/π`.
This hemispherical energy allocation and single-scatter GGX loss are approximations; Fixed lighting stays exact.
Cube and aerial RGB storage share one WGSL gain1024 declaration; forward decodes before solar RGB, then applies pre-exposure once.
All changed-sky mips complete in one submitted frame; cancelled work never becomes valid. Cube dependencies match
Sky; DFG/sample preparation depends only on device/BRDF. ~0.30MiB resident and ~4.19M fetches per changed Sky are cost accounts,
not timing claims. Validate analytic DFG/energy, cube axes/seams/horizon, source scaling, cache/cancel/replacement
and noon/twilight materials before retention. [Karis](https://cdn2.unrealengine.com/Resources/files/2013SiggraphPresentationsNotes-26915738.pdf)
and [Heitz VNDF](https://jcgt.org/published/0007/04/01/paper.pdf) supply the technique; code is independently derived.

The shared environment contract carries **fields and fluxes**, not a mandatory global fluid solver. For a physically driven provider, material transport follows `∂_t q + ∇·(q u) = S_q − D_q` for a conserved concentration/coverage proxy `q` with wind/flow `u`, source `S_q` and sink `D_q`. Authored/keyframed providers may publish the same query fields without solving this PDE; they are marked authored and cannot claim dynamical conservation. A front, gust or cloud cell must therefore be represented as a time-varying spatial field, not a camera-tied screen animation. `u(x,y,z,t)` uses C01's right-handed Y-up frame and is sampled at the consumer's height; plant drag, rain drift, cloud advection and water forcing use the same field with their own response laws. Grid spacing, timestep and numerical diffusion bound a simulated provider; the interface exposes interpolation support and a validity region rather than concealing that error.

For sunlight under cloud, optical depth is `τ_c(x,ω_s,t)=∫_ray σ_{t,c}(x+sω_s,t)ds`, and direct transmittance is `T_c=e^{-τ_c}`. An authored 2D field may store `τ_c` for one chosen light direction/altitude regime; it cannot predict side views, cloud self-shadowing or a rapidly moving Sun without reprojecting or recomputing. A volumetric `σ_{t,c}` can produce both view-ray cloud radiance and source-ray shadow attenuation, at greater sample/bandwidth cost. Thus the structural selection is **one cloud source field with independent shadow and visible projections**, with explicit approximation domain. A cloud-free source yields `τ_c=0`, `T_c=1`. Atmosphere and fog use the ordered `(transmittance, in-scattered radiance)` segment algebra only on **disjoint ordered ray intervals**; overlapping media require joint coefficient/source integration or one provider that precomposes the entire overlap with shared lighting. Independently lit preintegrated overlapping pairs cannot be composed exactly. A declared approximate layer policy may be selected for a named profile with its limits exposed. A display-only overlay is excluded from light/visibility authority.

For a wet/snowy surface cell, keep liquid and solid mass per area `m_l,m_s` (kg/m²). A minimal balance is `dm_l/dt = P_l + C_l + M − F − E_l − R_l` and `dm_s/dt = P_s + C_s + F − M − E_s − R_s`, where precipitation `P`, contact deposition `C`, melting `M`, freezing `F`, evaporation/sublimation `E` and exported runoff/removal `R` use kg/(m²·s). Melting/freezing cancel in the total derivative: phase transfer alone cannot create or destroy water. Runoff must name its receiving domain or an explicitly exported boundary flux. Material, temperature, wind and slope choose constitutive rates; a simple game provider may author them, while a physical provider must state its approximations and boundary drainage. Coverage/roughness and friction are functions of the same `m_l,m_s`, material and temperature, with possible hysteresis, so the visible gloss and physics traction cannot silently disagree. If a region is unloaded, W2/P7 either integrates a reduced aggregate with these fluxes or records a last update and catches up on load; it cannot drop the mass while audio/gameplay assumes continuing rain.

The selected [moving-shore exposure/response profile](../research/MOVING-SHORE-EXPOSURE-DESIGN-2026-11-02.md)
refines this liquid balance into exposed film, submerged bound coating and absorbed
mass, plus a separately identified pending water exchange. It allocates one
interval rain mass to a geometry- and water-dependent first-hit receiver and
moves real material between dry/wet subsets during inundation and recession.
For the tide-only prescribed scalar-stage profile, F4's prepared affine-area
polynomials let E1 integrate changing first-hit area against rain over source
time; numerical partitions do not change the physical receipt identity. The
[exact offline first-hit proof](../../evidence/water/MOVING-SHORE-FIRST-HIT-2026-11-02.md)
supports that source operator only. Film/coating exchange, finite-water
hydraulics, floating-point error and assembled runtime cost remain unproved.
The initial bounded physical closure stays pinned while active; it has no
unqualified unloaded-region catch-up. The original full-cell shallow solver,
the prescribed sea and the provisional fine-bed capacity solver publish
different wet supports, so they cannot share a pixel-derived shore mask.

Water uses **authored physical domains with selected model families**, not one numerical solver or one compulsory free-surface payload. Static pool height is a prescribed bounded function; it does not claim finite mass storage. A deep-water wave component can use dispersion `ω²=gk tanh(kh)` and phase `φ=k·x−ωt` under small-amplitude, approximately irrotational conditions; it does not describe breaking, river flow or contact forcing by itself. A shallow-flow domain can use depth `h` and depth-averaged velocity `u` with continuity `∂_t h+∇·(hu)=S_h` and momentum terms from pressure gradient, friction, terrain slope and forcing; closure and wet/dry boundaries must be specified by its module. A local contact solver may add a displacement/impulse field only with a specified support and reaction/transfer rule. The common query envelope carries domain ID, time, support, units, scale/error and typed failure. A graph surface may offer height/normal/velocity/depth; volume occupancy, pressure, underwater flow and breaking are separate requested capabilities. E1's query router selects one physical owner at a point and vertical context; overlaps require authored layered topology, and boundary ties are stable. It never blends two heights into physical truth. Geometry, contact and optics agree at interaction scale within a declared error; decorative ripples cannot alter buoyancy. These equations identify valid families, not a universal sea/lake/river discretization.

Every domain adjacency declares an edge mode: **closed/disconnected** (zero physical transfer), **prescribed reservoir/open** (a named source supplies admitted boundary forcing and the finite receiver logs its signed exchange), or **visual-only** (a render seam makes no through-flow or traversable-contact claim). **Conservative coupled** is admitted only with a fully specified shared interface flux/traction operator, wet/dry rule and substep schedule; until then a physical sea–river or river–falls–pool transfer requiring it is unsupported and fails F2/T1 admission. For finite storage at common density, `ΔV_i = Σ_e ΔV_(e→i) + sources − sinks + ε_i`; a coupled internal edge uses equal/opposite integrated volume entries under one edge/tick ID, while a prescribed sea is an *external reservoir* and does not receive an equal/opposite simulated state update. Thus local mass accounting can close without global momentum or energy conservation. Matching or blended surface heights cannot provide flux or pressure traction. Sea/tide level may force a river mouth only through that river solver's admitted boundary law; an instantaneous spectral crest alone is not a discharge law. PLAY-ENVIRONMENT-SPEC now fixes the initial numerical operators, validity thresholds and tolerances. Additional regimes require their own complete specification.

Vegetation placement is stable authored/seeded content; branch deformation is a response, not weather authority. A practical hierarchy samples wind at plant/branch location and advances bend state `θ` with a damped response such as `I θ¨+c θ˙+kθ = M_wind(u,shape)+M_contact`, with species/material parameters and bounded displacement. A lower tier may use a quasi-static or procedural phase approximation, but it must publish current and previous deformed position for velocity and conservative culling/shadow bounds. Dense canopy/light transport may use an aggregate extinction field derived from assets, while nearby visible leaves use geometry; the aggregate cannot pretend to include trunks unless it does. This separation follows the current [forest review](../../provenance/CONTEXT.md#source-17ab82cb1175) finding of different approximations for surface and mist receivers.

These models make coupling explicit without requiring every game to run expensive meteorology, structural dynamics or hydrodynamics. The chosen provider declares its approximation domain; a client that requires outside-domain behavior selects a stronger provider or receives a validation error. Unknown coefficients and target tolerances are content/commission questions, but the producer, operator and state are defined.

**Cost model before tier design.** A gridded scalar/vector source with `N_f` cells and `S_f` update operations costs roughly `O(N_f S_f)` per source step and `O(N_f)` storage; the camera count does not multiply that authoritative update. A projected cloud-shadow field costs its map samples only for active light/receiver domains, while a visible volumetric cloud costs approximately shaded pixels × ray samples × lighting samples and its own temporal storage. Terrain and vegetation residency scale with active instances/tiles and cooked LODs, not merely visible pixels; foliage overdraw adds fragment work and shadow invalidation. A water heightfield with `N_w` cells costs `O(N_w)` per numerical step under local stencils, while a spectral sea may have transform cost around `O(N_w log N_w)` per update; neither figure includes rendering, contact queries or wake forcing. Wetness deposition scales with active surface cells/events and catch-up policy. These symbolic costs identify the work each off switch must actually remove. Device-specific constants, bandwidth and target thresholds remain to be measured.

## Consequential boundaries

**Shared tick/source closure:** C09 selects pre-step X[n] surface state for P3 traction; E1 reservoir integration uses
tick-n forcing and publishes X[n+1] at the world commit. New contact deposition feeds the next explicit update once.
The declared one-tick coupling lag needs a game tolerance; a tighter model requires a single explicit coupled/substep
schedule. Physical thermal forcing belongs to E1 (prescribed field or selected energy model), using shared W1/E1 light
source facts and declared exposure data rather than render pixels. Missing radiation/thermal inputs require a named
authored approximation without an energy-accuracy claim. Runoff identifies its receiving patch/water region or exported
boundary. No implicit V3/GPU dependency is introduced into headless environmental authority.

Water-domain edges follow the same world tick. E1 samples named boundary sources at their declared tick/substep times,
integrates one edge flux register once per world commit and publishes domain state plus source/edge IDs in X[n+1].
Independent solvers may not each invent an interface flux at different times. P3 samples the declared X[n] water
state and returns identified body disturbances at the next specified update; tighter same-step reaction requires one
joint substep schedule. Solver domains are independent of W2 streaming cells: W2 activates required bank, boundary,
query, collision and navigation products together, pins live solver support across cell unload, or uses a qualified
reduced-state transfer. A missing product closes the traversal frontier or fails activation, not a zero-flux or dry
fallback. P7 saves durable finite-domain state, boundary/source revision and needed forcing/ledger data at a committed
barrier; restore prepares the matching model and content before exposing queries. A prescribed analytic field can save
parameters/phase only within its proven replay envelope. Headless/offline authority uses the same physical model
without GPU products.

Common world coordinates are metres with a documented geospatial/local transform, handedness and origin epoch. A large-world or planet option needs its own precision policy; it is not implicitly required. Every field/query names its support volume, spatial resolution/error, valid time interval, interpolation/extrapolation policy, provider/version, and failure result. F3 owns unit and schema definitions; game-specific climate rules stay outside the reusable renderer.

| Boundary | Owner → consumer; data and timing | Off, stream and failure behavior |
| --- | --- | --- |
| Environment facts | Source provider → weather, water, vegetation, V3, audio, gameplay; Sun direction unit vector, irradiance in declared spectral/RGB basis, wind m/s, thermodynamic fields in SI, `t_sim`/validity | Authored constant source is a valid low-cost substitute; changing source requires a declared state transfer or reset, with visual history invalidation. Unsupported fields yield a typed absence and a validated substitute, never zero interpreted as calm/clear by accident. |
| Weather and local air | Weather/fog provider → V3/V4/V5, gameplay/audio; precipitation rate/type, extinction `σ_t` m⁻¹ and shared-light source data for joint overlap, or a `(T,S)` pair for a disjoint segment/jointly precomposed overlap, with support and time | F2 rejects independently preintegrated overlapping media without one joint integrator/owner or an explicit approximate policy. Dynamic weather off removes updates; static fog/wind can remain. No fog yields identity transmittance and zero in-scattering. |
| Cloud state and shadow | Cloud provider → V3 directional visibility, sky/volume renderer, water/terrain/vegetation receivers; optical depth/transmittance `T∈[0,1]` by source direction, area, altitude and time | Visible draw, light attenuation, aerial shadow and reflected cloud appearance are separate consumers with dependency checks. If cloud shadow is retained, source preparation remains and is measured; if all consumers off, retire source caches. Missing data uses documented clear-sky or authored constant fallback, never stale texture. |
| Terrain construction and surfaces | W2/F4 → terrain/surface module → V2/P3/P5/V3; stable tile/material IDs, geometry and collision/nav proxies, surface layers/normal/height in metres, version | W2 decides activation; surface module holds weather-driven state per persistent region. Render LOD may differ from physics but contact positions/normals agree within declared tolerance. Stream-out saves state/deltas and releases derived meshes/textures after fences. |
| Vegetation | W2/F4 + wind/interaction → vegetation → V2/P3/P5/V3; stable plant IDs, placement, bounds, current/previous deformation, obstacle/contact proxy, material coverage | Vegetation off retires draws, animation and derived GPU data; gameplay obstacle/harvest state may require a retained nonvisual representation. Wind off selects calm deformation without erasing durable breakage. LOD/shadow/proxy transitions must preserve occupancy and temporal classification. |
| Water domain, edge and contact | E1 physical model + named edge/source → P3/gameplay/audio/V2/V3/V5; unique domain ID, model/source/edge revision, snapshot time, supported height/flow/volume/force capability and accuracy, or typed `outside/ambiguous/stale/unsupported` | W2 supplies required cooked/streamed products but does not define solver boundaries. Missing physical queries or a needed conservative edge block activation/crossing. A lower visual tier may change ripples/foam, not body force, storage or edge flux. No renderer does not remove required buoyancy/navigation queries. |
| Surface response | Precipitation/contact/temperature/wind + material region → surface state → V3/P3/V5/audio; wetness or snow mass/coverage, absorption, roughness/friction changes, runoff/event IDs and time | Visual wetness can be omitted but friction/gameplay state persists where required. If response absent, a dry/base-material substitute is explicit. P7 retains persistent deposits; cosmetic splashes retire by lifespan. |

**No circular light source.** Atmosphere may compute direct Sun transmittance and skylight/ambient distribution; cloud state attenuates sunlight and contributes a sky appearance; V3 combines these into the light presented to surfaces and volumes. Surface bounce/indirect providers consume the resulting direct and environment radiance; they do not redefine Sun/weather. Water and vegetation receive light as material/geometry consumers. Reflection probes may capture sky/cloud/surfaces at a declared time and resolution, but that capture is derived presentation, not environmental authority. Overlapping media follow the selected joint ownership/integration or an explicit approximate partition; a mask/weight alone cannot establish exact optical composition. The [Hillaire atmosphere paper](https://sebh.github.io/publications/egsr2020.pdf) supports a scalable dynamic sky/aerial candidate; [Bruneton's tested implementation](https://ebruneton.github.io/precomputed_atmospheric_scattering/index.html) is an alternate reference with different recomputation properties. Neither supplies local fog/weather by itself.

For [N1 server-authoritative sessions](NETWORKING.md), E1 physical water/medium fields and queries belong to the authority world. The gameplay manifest pins physical model/schema, domain/edge/source revisions and relevant cooked hashes; clients may have different mesh, foam, optical and shader products, but not silently different bathymetry, boundary laws or collision semantics. Water replication groups carry a committed domain state/query projection with its boundary/source revision and sample time before the replica marks a physical query usable. Client visual detail and GPU histories never feed server forces or flux. Water-contact prediction is off by default; a selected provider may opt in only with complete P3/E1 body, domain, forcing and disturbance capture/replay under N1's bounded correction policy. A correction, domain/source change or model switch invalidates affected water, underwater, medium and image histories. P7 saves authority state and external forcing under the pinned build; a client replica cannot write that root. Standalone/headless play retains the same physical semantics without network or renderer.

## Geometry, media and temporal composition

Environment products obey [RENDERING](RENDERING.md)'s V2–V4 contracts. Terrain, vegetation, water and cloud representations publish conservative bounds and current/previous positions or semantic motion/reativity. A terrain stream-in, vegetation spawn, water-topology change, cloud field reset or source switch invalidates affected shadow, reflection, GI and temporal caches by stable ID/version/region, not by an unbounded global reset when local invalidation is safe. Screen-space methods cannot reconstruct missing offscreen terrain/clouds; sky/environment and probe/planar fallbacks are declared.

The air contract distinguishes a spatial medium from a drawn background. For a view ray, **nonoverlapping ordered intervals** may each supply a preintegrated transmittance and in-scattered radiance pair `(T,S)`; V3 composes those pairs front to back. **Overlapping supports** instead provide compatible extinction/scattering/source inputs to one V3 joint integrator with shared Sun/cloud/local-light attenuation, or one named provider consumes all participating media and returns their jointly precomposed pair. F2 rejects independent overlapping `(T,S)` pairs as an exact solution: their spatial source/extinction order and cross-attenuated lighting cannot be recovered from the aggregate pairs. A named profile may select an explicit homogeneous/ordered-layer approximation with stated range, omissions and visual error policy, never an exact overlap claim. E1 declares each medium's identity, support, representation, light/source dependencies and validity; V3 owns integration and no-double-counting, V1 validates the graph, and V4 receives only completed scene-linear colour. Clouds supply their own depth/coverage/transmittance and shadow the appropriate air/surface portions. Opaque terrain/water truncates the ray at actual depth; water's interface and underwater medium are separate segments. Top-down views can omit visible sky while keeping aerial perspective and cloud shadows. A medium, light, source, water-interface or profile revision invalidates affected E1/V3 effect histories and V4 image histories by view/region/version. First-person movement through fog, above clouds or underwater also changes interval order and invalidates/reprojects histories as appropriate. The [temporal composition study](../research/TEMPORAL-COMPOSITION.md#boundary-changes-to-return-to-architecturerendering-owners) identifies this boundary. [Epic cloud](https://dev.epicgames.com/documentation/unreal-engine/volumetric-cloud-component-in-unreal-engine) and [fog](https://dev.epicgames.com/documentation/unreal-engine/volumetric-fog-in-unreal-engine) document cost and representation tradeoffs; they do not prove their composition for this engine.

Vegetation deformation, water waves and moving cloud detail must expose prior-state evaluation at the same world point/identity, or mark unreliable velocity/reactive regions. Otherwise temporal reconstruction can trail despite a correct rigid transform. Foliage shadow caches must invalidate only to the extent deformation changes light visibility; Epic's [VSM documentation](https://dev.epicgames.com/documentation/unreal-engine/virtual-shadow-maps-in-unreal-engine) specifically notes high invalidation cost for wind/world-position-offset foliage. Separately, water surface geometry must be the source of water reflection depth and contact appearance; a sky background cannot imply a sea. Broadsides records this scene decision in [DESIGN](../../provenance/CONTEXT.md#source-779e25e8668e), but it is also a sound generic world/render boundary.

## Family responsibilities and plausible providers

| Family | Governing content/state and candidate representations | Research status and risk |
| --- | --- | --- |
| Terrain and surface identity | W2 owns authored/procedural height/mesh/caves/tiles, activation, stable IDs and collision/nav products. Surface module owns material regions, biome tags and mutable state. Render provider chooses mesh LOD, clipmap or virtual geometry. | No one terrain geometry suits caves, interiors and landscapes. Heightfield is an optional provider, not the world model. Prove seams, edits and stream churn before virtualizing. [World Partition](https://dev.epicgames.com/documentation/unreal-engine/world-partition-in-unreal-engine) is production precedent for streaming ownership, not a required grid. |
| Vegetation | Cooked plant assets with explicit trunks/leaves/alpha coverage/LODs; placement as authored or seeded recipes with stable IDs; deformation from sampled wind plus local interaction; optional growth is a separate simulation provider. | Growth is not required for every game. GPU instancing and cards/geometry compete by view/platform. Shadow invalidation, alpha shimmer and animation velocity are first-order proof risks. Epic's [foliage authoring](https://dev.epicgames.com/documentation/unreal-engine/foliage-mode-in-unreal-engine) shows mass placement workflows, not a quality guarantee. |
| Weather, wind and precipitation | Constant/keyframed/weather-sim source fields; wind sampled by altitude/space/time; precipitation event/flux source plus distinct visible streaks, audio and deposit response. | A moving texture is not a wind/weather simulation. The Broadsides game can choose a stronger physical provider; shared engine queries remain stable. GPU particle counts are never gameplay rainfall truth. |
| Atmosphere, sky, clouds, local fog | Separate direct/ambient transport, aerial perspective, visible sky, cloud state/attenuation, visible cloud representation and local fog. V1 selects Hillaire (2020) transmittance, multiple-scattering, sky-view and aerial-perspective LUTs checked against a CPU spectral oracle (G3s); analytical/volumetric local fog and layered/volumetric clouds remain candidates. | Sky-only, cloud-shadow-only and visible-cloud modes are supported combinations. High-altitude 2D plus lower 3D cloud layering has production precedent in [Guerrilla's slides](https://advances.realtimerendering.com/s2015/The%20Real-time%20Volumetric%20Cloudscapes%20of%20Horizon%20-%20Zero%20Dawn%20-%20ARTR.pdf); mobile may require cards/layers and low-cost shadows. Local and far air boundaries remain a correctness risk. The visible default is weather-map volumetric clouds over the E1 `τ` source, marched at reduced resolution with V4 reconstruction ([port decision](../decisions/SEA-AND-CLOUD-PORT.md)); the projected `τ` field remains the shadow-only and portable representation. |
| Water | Physical domain and edge topology independent of W2 cells; separately chosen static/shallow/spectral/local dynamics; typed query and force adapter; opaque/translucent visual surface and underwater optics where needed; contact effects and wetness. | Sea/lake/river need not use one solver. F2 selects by required regime and rejects unsupported physical transfer/crossing; visual tiers cannot change the model. Caustics require light, water geometry, receiver depth and visibility. No existing water experiment is accepted as the engine baseline. |

For each family, editable sources and procedural recipes are separate from cooked per-platform payloads. T1 provides human and agent interfaces to set region boundaries, causally meaningful environmental inputs, material/vegetation assets, cloud/fog volumes and quality policies. It can preview a selected camera and time, compare light/air/geometry isolations, inspect query values at a world point, and display missing-provider or unsupported-profile diagnostics. Operations are deterministic and schema-validated; an AI agent can submit a patch, cook, capture and read quantitative/visual differences without relying on an editor's hidden transient state. F4 tracks dependency hashes and provenance, including attribution and licenses for borrowed open-source code/assets. Epic's [PCG overview](https://dev.epicgames.com/documentation/unreal-engine/procedural-content-generation-overview) is a production example of authored procedural tools; its [GPU processing](https://dev.epicgames.com/documentation/unreal-engine/using-pcg-with-gpu-processing-in-unreal-engine) is labelled beta and is not a required engine foundation.

## Supported compositions and quality transitions

| Composition | Retained service | Optional presentation/substitute | Invalid combination |
| --- | --- | --- | --- |
| Top-down with cloud shadows, no visible clouds/sky | Weather or authored cloud attenuation, Sun/ambient light, shadows, aerial perspective if selected | Draw no cloud pixels; simple 2D attenuation provider; static ambient probe | Disabling the sole attenuation provider while cloud shadows remain required |
| Third-person exterior | Terrain, wind/vegetation, weather, sky/air, selected water, light/reflection | Cloud layer or full volume; analytical or froxel fog; selected water dynamics | Using visual rain to drive wetness when rain particles are quality-cullable |
| First-person interior | Local light/air, optional weather visible through openings, material response | No terrain/vegetation/water unless scene uses them; environment map or external light source | Removing external light provider if window illumination remains required |
| Portable and constrained recipes | Same source facts and gameplay queries, cooked lower-res assets and lower-cost representations; tile recipes keep volumetric clouds where measured within budget | Shadow mask instead of volumetric cloud where the recipe cannot afford it, cheaper water visuals/foliage/particles, and a fog approximation only under its declared optical policy | A visual quality switch that silently changes buoyancy, water flux, wind, persistent deposits or exact-overlap claim |

Build-time omission can remove unused family code/assets. Load-time provider choice may rebuild state and caches. Live toggles are permitted only where an explicit state/history transition is implemented and measured. An effect-off profile removes avoidable simulation/preparation/draw/residency while retained consumers get a validated alternate. A visible-sky toggle alone should not destroy sunlight, cloud-shadow or reflection sources. New views allocate only their own histories. Environment state is shared across views.

## Worked operating cases

1. **Hidden clouds over an isometric town.** At `t_sim`, a constant wind and time-indexed cloud optical-depth source updates once. A cheap shadow projection samples the field along the Sun direction and gives V3 `T(x,z,t)` for ground and roofs. The sky draw and volume march are absent. Vegetation reads the same wind; ambient probe remains. A second minimap view samples the same field without advancing it. If cloud shadow is toggled off and no other consumer needs cloud state, F2 retires its texture/update; V3 uses clear-Sun visibility by validated policy.
2. **Storm crosses a vegetated shore.** A weather front changes wind and precipitation fields. Water responds at its selected dynamics step, trees deform and publish previous/current pose at each temporal view's current/previous presentation times or mark invalidity, surface state receives rain flux and contact splashes, audio reads the flux, V5 emits quality-cullable streaks/spray. A streamed terrain cell loads material/physics/nav products before becoming active; its wetness is restored from P7. V3 receives one Sun/cloud attenuation and surface wetness changes its material response. V1 validates V3/E1/V5's scene-linear graph, which composes cloud/fog/water at their depths before V4 reconstructs/displays. Lowering mobile visual quality may remove volumetric clouds and many streaks while retaining wind, precipitation, wetness and water contact queries.
3. **Camera enters fog or passes underwater.** View P2 changes pose, not environment state. The air/water region intersection defines ray intervals; surface depth resolves which medium precedes which. A local fog overlapping the far atmosphere needs a joint medium integrator or jointly precomposed owner; two independently lit whole-interval pairs are rejected. E1/V3/V4 reject affected media and water histories where projection, interface or disocclusion changes. Physics/audio queries continue at the same world location/time independent of whether the volume pixels were drawn.
4. **Region unload and reload.** W2 owns activation and persistent delta transactions; P7 retains durable plant damage, wetness/snow and E1 water state under provider codecs. V2/V3 retire mesh/shadow/reflection products after fences; weather source keeps global time. W2 cannot unload live solver support merely because one cell leaves a view. On reload, stable IDs and model/source revisions return, dynamics resume or integrate a qualified reduced state, derived caches rebuild, and histories reject the changed pixels. A background rain field cannot continue depositing on unloaded regions unless a cheap aggregate/offline policy is explicitly selected.
5. **Prescribed sea meets a river and decorative falls.** The sea's named tide supplies an admitted downstream boundary to a finite shallow river; the river computes a signed mouth exchange into the external-sea reservoir ledger, not an update to the prescribed sea. A decorative falls/pool seam is visual-only or separately authored inlet/outlet, with no claim of exact river-to-pool mass transfer or traversable boat contact. F2 rejects a requested physical connection until a complete interface flux/traction and crossing operator exists. W2 cells may cross either solver domain; both cameras share the committed state, and differing visual ripples never alter P3 forces or the ledger.

## Predictions, risks and qualification order

| Structural prediction and risk | Bounded validation and rejection condition |
| --- | --- |
| Shared source and conservation | Prediction: one time-indexed field plus declared transport/source terms gives consistent Sun/cloud/wind/rain/wetness across views and systems. Check a prescribed clear-to-storm input with independent point/flux calculations, then inspect the image. Reject if conservation residual or time/location disagreement exceeds chosen tolerance; revise closure/step, not arbitrary visual tuning. |
| Cloud shadow without visible cloud | Prediction: `T_c=e^{-τ_c}` from one cloud source yields correct attenuation irrespective of sky draw. Check against direct integration of a known analytic density with moving Sun and orthographic camera; inspect visible cloud alignment separately. Reject the 2D approximation outside its documented light-angle/altitude regime, not because another texture happens to look nicer. |
| Air/volume/depth composition | Prediction: ordered `(T,S)` is valid for disjoint intervals; joint coefficients/source lighting or one precomposing owner handles declared overlaps. Compare a front-versus-back absorber/emitter counterexample, shared-Sun local fog and an air/water/underwater ray before moving first-person/top-down routes. Reject independent overlapping pairs, double integration, depth reversal, boundary bands or history ghosts; qualify any approximate layer policy separately. |
| Terrain/vegetation representation | Projected-error bounds and stable IDs predict no geometric seams or contact ambiguity under stream/LOD changes. Check known-height/known-plant fixtures in two cameras with physics/nav, shadows and prior deformation. Reject if proxies violate declared position/normal/coverage tolerances or shadow invalidation grows beyond modeled support. |
| Water dynamics family and edges | Dispersion and shallow-flow equations predict different regimes; a boundary law must name its source/flux and query owner. Validate a finite wave, solver-local shallow-flow balance, reservoir ledger, shoreline body contact and unsupported falls crossing. Reject a provider when its regime fails, a ledger hides a sink, a seam invents a height/force, or a required coupled operator is absent. These checks are future qualification, not evidence already obtained. |
| Whole-scene budgets | Symbolic work predicts which profile switches should save CPU/GPU/memory. On this Mac and Chrome for V1, then Windows, Android and physical iOS on rented hardware after V1, measure sustained frame distributions, residency, loading, pipeline stalls and thermal behavior with simultaneous systems. Reject a tier only against the agreed experience and budget; no targets or measurements exist yet. |

**Suggested module commissions:** (1) shared environment source/schema, transition and spatial-query semantics; (2) atmosphere/cloud/fog with light and volume composition; (3) terrain/vegetation streaming, deformation and shadow/temporal coherence; (4) water family dynamics, queries and rendering interfaces; (5) surface response/precipitation persistence. Each should first close its model, boundaries, numerical assumptions and predicted errors from first principles and primary research, then validate only the consequential predictions. Existing sea/forest code may be reconciled after the target is selected by this reasoning, not promoted by proximity.

N1 product compatibility is role-specific: exact authoritative solver-product hashes are match provenance. A replica
using confirmed query projections need not download the server solver. A client evaluating/predicting physical queries
needs its declared compatible product closure and exact hashes mapped to the same physical source/model semantics;
bytewise equality across target cooks is not assumed. Visual-only products cannot establish this compatibility.

Even disjoint `(T,S)` segments compose exactly only relative to a shared declared transport closure: each source
term must use incident lighting consistent with the combined scene, including other media on its light paths.
View-ray disjointness alone does not justify independently lit preintegration. Otherwise recompute jointly or
declare the lighting approximation and its validity.

## Initial source and network recipe scope

C4 uses file-first source admission and restart for gameplay-affecting environment edits. Initial portable/server
worlds preload their required physical support; visual residency and no-sky rendering remain independent. Later live
physical-domain streaming needs its own complete transfer/conservation/admission design, not mandatory full-world
resume serialization in every initial provider.

Multiplayer water shares causes, not state, under the [water decision](../decisions/E1-GPU-WATER.md); its event
records are an optional E1 network recipe in NETWORK-SPEC, not a generic N1 requirement. Gameplay force meaning
cannot be changed by client visual tier.
R4's first multiplayer workload contains no water or prediction; the separate R3/R4 combination has its own admission
calculation. This removes optional codec/solver cost from unrelated games without losing physical coupling contracts.
