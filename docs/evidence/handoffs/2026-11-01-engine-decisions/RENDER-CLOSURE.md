# D2 initial render closure — concrete selection for integration

1 November 2026. This closes the remaining *initial portable render path* decisions behind
[selected architecture](../../../design/ARCHITECTURE-DECISIONS.md),
[C3](../../../design/CONTRACTS.md),
[rendering](../../../design/modules/RENDERING.md) and
[environment](../../../design/modules/ENVIRONMENT.md). No code was built or
device measured. The limits below are **design admission targets**, not measured performance,
hardware support floors or permission to substitute physical gameplay content. The root owns
the combined workload and N1; F4 remains the only cook/product authority, V1 the only GPU
device/submission authority, V3 transport and E1 environmental fields.

## 1. Shader language and toolchain

**Select authored WGSL** as the initial portable shader source. Shader modules are UTF-8
`.wgsl` files with explicit entry points; small static include expansion is a cook operation
over named, hashed dependencies, never arbitrary runtime string assembly. Lock the reviewed
candidate `wgpu = "=30.0.1"` and direct offline validator `naga = "=30.0.1"`
exactly in the future manifest, and commit the resolved `Cargo.lock`.
The lock, feature flags, Rust compiler, shader cook/ABI schema and target profile versions
are product-key inputs. Native wgpu translates WGSL through Naga to the selected API;
browser WebGPU receives WGSL for the browser implementation to compile. A browser and a
native device therefore need independent pipeline preparation/qualification; a validated
Naga module is not a browser-compiled pipeline. Current wgpu documentation explicitly
describes these different paths and notes possible WGSL-version differences.
[wgpu 30.0.1](https://docs.rs/wgpu/30.0.1/wgpu/),
[wgpu dependency/feature manifest](https://docs.rs/crate/wgpu/30.0.1/source/Cargo.toml),
[Naga 30.0.1](https://docs.rs/naga/30.0.1/naga/).

The cook first resolves includes deterministically, validates WGSL with the pinned Naga,
checks entry-point IO/resource reflection against the declared pass ABI, and emits immutable
canonical WGSL plus a descriptor/ABI manifest. On each actual target/device, V1 creates
the module/pipeline and reports validation/translation/driver errors. Native pipeline
creation can still stall; WebGPU `createRenderPipelineAsync` resolves only after creation
and may reject. wgpu adapter behavior must be checked at the selected version rather than
promising one universal async call. Prepare required work off the draw path where the
adapter permits and hold local loading until completion. A driver cache is optional and
discardable, keyed to the actual adapter; it is never the F4 product.
[WebGPU pipeline creation](https://gpuweb.github.io/gpuweb/#dom-gpudevice-createrenderpipelineasync),
[wgpu pipeline-cache contract](https://docs.rs/wgpu/30.0.1/wgpu/struct.PipelineCache.html).

**Alternatives:** GLSL/HLSL plus translation adds a second authored language and more
cross-target failure modes with no initial content need. Arbitrary runtime shader graphs
or custom generated shader code make the finite closure unverifiable. A later specialized
native provider may supply a separately versioned shader path, but it is not the portable
baseline or an implied material compatibility promise. No shader compiler framework,
binary plugin ABI or WebGL fallback is selected now.

## 2. Material closure and binding ABI

The first material closure is metallic-roughness PBR for opaque and alpha-masked surfaces:
Lambert diffuse plus GGX/Smith/Schlick microfacet specular, dielectric `F0=0.04`, metal
interpolation, normal map, emissive term and two-sided flag. This is an energy-accounted
*single-layer approximation*; coat, subsurface, anisotropy and physically exact fabric
are unsupported until a named closure is added. Opaque and masked materials have depth,
shadow, forward and velocity roles. Masked coverage uses one authored cutoff and the
same alpha texture/UV/deformation in depth, shadow and forward; it cannot turn opaque
because a role is missing. Transparent baseline uses premultiplied alpha, depth-tested
forward shading, sorted by declared group and writes no opaque depth; intersecting or
refractive transparency remains explicitly approximate. Water uses its own typed
material/interface provider, not a hidden metallic-roughness flag. A required imported
extension must map to this closure with stated loss or fail admission.

For `c_i=n·ω_i>0`, `c_o=n·ω_o>0`, `c_h=n·h`, `h=normalize(ω_i+ω_o)`, use
`f_r=(1-m)(1-F)base/π + D_GGX G_2 F/(4c_i c_o)`, otherwise zero.
Set `F0=mix(vec3(0.04),base,m)`, `F=F0+(1-F0)(1-ω_o·h)^5`, and
`α=max(0.045,roughness)^2`. Then
`D_GGX=α²/[π(1+c_h²(α²-1))²]` and choose height-correlated Smith
`G_2=2c_i c_o/[c_i sqrt(α²+(1-α²)c_o²)+c_o sqrt(α²+(1-α²)c_i²)]`.
The minimum *perceptual* roughness is 0.045, so the minimum `α` is
0.002025; the floor is a declared highlight-width approximation, not an
accidental 0.2 roughness floor. ORM occlusion modulates only selected
indirect light, not direct light or a second shadow. This single-scatter
microfacet BRDF omits microfacet interreflection and layered transmission.
Its hemisphere-energy and reciprocity checks are implementation
conformance, not a decision deferred to visual trial.
[Heitz's masking-shadowing derivation](https://jcgt.org/published/0003/02/03/paper.pdf),
[Frostbite production closure](https://blog.selfshadow.com/publications/s2014-shading-course/frostbite/s2014_pbs_frostbite_slides.pdf).

Texture inputs are four fixed slots: base-colour/alpha, tangent-space normal, ORM
(occlusion/roughness/metalness) and emissive, plus one material sampler. Base/emissive
colour textures decode sRGB to the linear working basis; data maps remain linear.
Missing optional slots bind neutral typed defaults (white base, flat normal, ORM
`(1,1,0)`, black emissive) so the material layout does not multiply variants.
Normal mapping requires a validated tangent basis; otherwise author a no-normal-map
material or cook tangents with a declared method. Material factors, alpha cutoff and
texture-presence flags are uniform data, not compile-time variants. Structural shader
specialization is limited initially to alpha mode (`opaque/mask/blend`), geometry
interface (`static/skinned/instanced`), pass role, and chosen material closure/provider.
The closure creates only actually reachable tuples, not their full product. Surface
light count is runtime data and does not create a shader per number of lights.

Use four explicit bind groups, within WebGPU's guaranteed baseline: `0` view/frame;
`1` scene lights/clusters/shadows/environment/medium; `2` material uniform, the four
textures and sampler; `3` object transforms/skin palette. Passes declare exact subsets
of this layout; extensions cannot silently add a fifth group or unbounded texture
array to the portable profile. Use scene/object storage buffers and indexed records,
not one dynamic uniform binding per object. The integrated workload's 32 surface
lights per cluster (8 mobile) is an F2 capacity limit with diagnosed overflow,
not shader truncation. Resource/texture counts and storage sizes must pass the
queried adapter limits; wgpu's documented default WebGPU limits include four bind
groups, 16 sampled textures and 256-byte dynamic uniform offset alignment.
[wgpu limits](https://docs.rs/wgpu/30.0.1/wgpu/struct.Limits.html).

**CPU/WGSL ABI revision 1:** WGSL buffer structs use only `u32`, `f32`, `vec4` and
`mat4x4<f32>` in portable shared records; no `bool`, `vec3` or implicit Rust padding.
Matrices are column-major. A `ViewRecord` contains `viewProj` and
`previousViewProj` at byte offsets 0 and 64 (64 bytes each), then
`cameraXYZ_time` at 128 and `preExposure_invExposure_jitterXY` at 144; 160-byte
record, 256-byte dynamic-uniform stride. A `MaterialRecord` is four 16-byte
vectors at offsets 0/16/32/48: base RGBA, emissive RGB+alpha cutoff,
`(occlusion,roughness,metalness,normalScale)`, and four `u32` flags/IDs; 64 bytes.
An `ObjectRecord` stores current/previous model matrices (0/64), three normal
matrix columns padded as `vec4` (128/144/160), and a `vec4<u32>` of stable
instance index, skin offset, flags and source generation (176); 192-byte stride.
GPU matrices and `cameraXYZ` are f32 relative to the declared view origin;
a world-origin change invalidates prior samples unless both frames are
explicitly transformed into the same origin convention.
The cook records an ABI revision and verifies Naga-reflected offsets, sizes,
bindings and vertex IO against one explicit Rust layout table. CPU packers
write zeroed byte buffers at those offsets using checked scalar encoding;
never `transmute` arbitrary Rust structs into GPU bytes. Buffer binding offsets
obey queried alignment; dynamic view records use the 256-byte baseline stride
only when the device reports alignment no greater than 256, otherwise round up.
WGSL's alignment/size rules, not Rust's layout intuition, govern the contract.
[WGSL memory layout](https://gpuweb.github.io/gpuweb/wgsl/#memory-layout),
[WGSL matrix semantics](https://gpuweb.github.io/gpuweb/wgsl/#matrix-types).

## 3. Shader closure, keys and readiness

F2 derives a demand tuple from `target/backend-feature tier × admitted profile
plan × actual material closure/alpha × geometry interface × pass role × concrete
pipeline formats/layout/overrides`. It includes startup content, declared streamed
asset families, external-content admission envelopes, necessary shadow/velocity
roles, permitted live transitions and their already approved fallbacks. The
author need not maintain a second shader list. Dynamic lookups outside the
declared roots are rejected or undergo a new F4/F2 admission transaction before
activation. Imported material conversion and unknown extensions are explicit.

F4 product identity hashes canonical expanded WGSL, dependency hashes, material/pass
ABI revision, compiler and cook version, toolchain lock, target backend/features,
entry points, specialization values and full immutable pipeline descriptor. V1
runtime identity adds queried adapter capabilities, device generation and actual
surface/attachment formats/sample count. A native driver cache may accelerate an
equal device key but may be discarded; browser disk cache is not readiness.
W2/F4 publish content/manifest at the existing barrier, then V1 publishes local
presentation at extraction only after its required pipelines are ready. A
profile change reserves old/new overlap, prepares new pipelines, swaps one
presentation generation and invalidates all affected histories; failed work
leaves the old generation active. Device loss clears V1 handles and repeats
preparation from retained F4 products. The N1 replica may remain synchronized
while the local view loads; shader identity is not a network gameplay lock.

Use distinct diagnostic states: `declared → cooked → delivered → CPU-verified →
device-preparing → device-ready` or `failed/unsupported`. A first playable scene
has all required tuples delivered and device-ready. A later stream frontier
loads visibly; it cannot draw a required object with an unknown variant.
An optional presentation fallback is a separately included and prepared tuple
with authored fidelity/semantic limits. Masked shadow, depth/velocity and a
required joint-medium path never disappear silently. Target build validation
fails if the finite demand exceeds **512 distinct pipeline descriptors**, with
**64 boot-critical** and **128 newly required in any one admitted live switch**
as initial design caps. These are workload controls, not device limits; the
first small-game target should need far fewer. An author must simplify a
closure, move content to explicitly on-demand loading, or define a qualified
alternative if a cap is exceeded. Report count by material/geometry/pass/provider
axis, cooked bytes, preparation count/time, and old/new peak memory. Runtime
pipeline requests outside the closure are errors, never first-draw compilation.

## 4. Scene colour and light quantities

Select **linear sRGB/Rec.709 primaries, D65 white**, with unclamped HDR values
as the single V3 scene-working basis. The primaries are R `(0.640,0.330)`,
G `(0.300,0.600)`, B `(0.150,0.060)` and white `(0.3127,0.3290)` in CIE xy.
The photopic luminance projection is `Y = 0.2126 R + 0.7152 G + 0.0722 B`
(rounded matrix coefficients). Imported colour sources declare profile/transfer,
decode and convert once at import/cook; untagged sources require an authored
assumption, never an implicit display transfer. Base reflectance is dimensionless
and bounded `[0,1]` after a declared gamut conversion; out-of-gamut loss is
reported. Emission/light radiance is unbounded in scene-linear float. V4 alone
applies exposure, tone/gamut mapping and target output encoding. A display-P3
or HDR target changes V4 output transform, not V3 working primaries.
[W3C sRGB definition](https://www.w3.org/Graphics/Color/sRGB),
[W3C sRGB specification](https://www.w3.org/Graphics/Color/srgb.pdf).

ACEScg or another wide-gamut working space would retain more saturated
spectral/asset colours, but it adds import/display conversion and still does
not make three-channel scattering spectrally exact. Rec.709/D65 matches the
first browser/SDR interchange path and keeps one explicit basis; wide-gamut
losses are recorded at cook. A later wider working basis is a versioned
transport/profile migration, not an invisible display toggle.

Use **photometric RGB** for the initial visible-light renderer. A unit-colour
vector `c` is nonnegative and normalized to `Y(c)=1`; intensity is separately
specified. Directional Sun/light input is perpendicular illuminance in lux;
point/spot input is luminous intensity in candela, giving incident illuminance
`E_rgb=I_rgb/r²` lux outside its declared emitter radius and then the normal
cosine at the surface. Area/emissive surfaces use luminance in cd/m² and
solid-angle integration; the initial portable light list admits directional,
point and spot lights, while area-light approximation is a separately labelled
provider. Sky/environment and emitted-surface radiance are cd/m² (per RGB
channel by the stated Y normalization). A BRDF has sr⁻¹, so `f_r E`
produces cd/m². Transmittance and visibility are dimensionless; medium
extinction/scattering are m⁻¹; medium in-scattering `j` is cd/m²/m.
`683 lm/W` is the candela definition only at the specified monochromatic
frequency, not a broad-spectrum solar-energy conversion. E1 thermal/plant
physics receives independently authored or spectrally integrated radiometric
W/m² and may not compute heat from the RGB lux value. A spectral atmosphere
may integrate internally then project to this RGB basis with a declared
colour/energy error; incompatible units/bases fail F2 admission.
[BIPM candela](https://www.bipm.org/en/si-base-units/candela),
[CIE photometric integral](https://files.cie.co.at/756_CIE_TN_002-2014.pdf).

Perform V3 transport in these scene units with f32 shader arithmetic. Store
scene colour in a qualified RGBA16Float intermediate with a per-view scalar
pre-exposure `p`; every producer writes `p L` and V4 knows `p`. Histories
rescale by `p_current/p_history` only when their inputs remain valid; otherwise
reset. This prevents format saturation without inventing a second lighting
unit. `p` and output gamut/tone policy are versioned view metadata. Reject
NaN/negative light intensity and invalid exposure at authoring/runtime
boundaries; large HDR values are not silently clamped before V4.

## 5. Initial atmosphere, fog and transport operator

Select a small **joint single-scattering air path**, not a universal multiple-
scattering solver: one flat-ground exponential atmosphere
`σ_{t,a}(y)=σ_{t,0} exp(-max(y,0)/H)` with RGB default sea-level
scattering/extinction `(6,12,24)×10^-6 m^-1`, `H=8000 m`, albedo 1 and
Rayleigh phase `p_R(μ)=3(1+μ²)/(16π)`. These RGB values are an authored
Earth-like approximation, not a measured spectral fit. Admit authored
`σ_{t,0}` channels in `[0,5×10^-5] m^-1` and `H∈[4000,12000] m`.
Add at most two overlapping bounded homogeneous local fog spheres/boxes
per ray, each with `σ_t∈[0,0.05] m^-1`, `σ_s/σ_t∈[0,1]`, radius or box
half-extent ≤250 m and Henyey–Greenstein `g∈[-0.85,0.85]`; the default
is `σ_t=0.015 m^-1`, albedo 0.9, `g=0.6`. The admitted ground/shore camera
envelope is altitude 0–500 m and view-path length ≤2 km, with no camera
below ground or orbital/planetary-curvature claim. Fog may be revised only
at a committed E1 sample; all light/media history carries its source epoch.
Each E1 source declares world support, update time, model/source revision,
light dependence and whether coefficients are authoritative for gameplay
or presentation-only. The range excludes arbitrary 3D density noise.

The R1/R3 air profile requires an authored equirectangular HDRI (exact asset
hash and colour profile in F4) as *terminal background*. Its initial
calibration is D65 with mean scene luminance 1000 cd/m² and no Sun disk;
V3 cooks a diffuse environment convolution and a specular reflection mip
chain from that same source. The separate directional Sun supplies direct
light and is not added again from the HDRI. An HDRI supplies no path
extinction: the joint air model above supplies aerial perspective. The R0
small game can omit the air/HDRI capability and use an explicitly authored
constant environment-light source. Clouds initially supply a source-direction
optical-depth/attenuation field under its declared angle/altitude validity,
not a second overlapping visible volume. This cloud factor multiplies direct
Sun **once** at each queried light path; the joint air integrator computes
air/fog attenuation itself and does not consume an already air-attenuated
Sun. Transparent cloud cards, if authored, are approximate surfaces.
The initial field is an authored nonnegative `τ_cloud(x,z,t,ω_s)` sampled
at the receiver/source position for its declared Sun-direction range, with
`T_cloud=exp(-τ_cloud)`; outside that range it is unavailable unless an
explicit clear-sky or static-source fallback was admitted. If a game uses
cloud visibility for rules, that source/query belongs to E1 authority and
N1's semantic gameplay projection, not the client's shadow texture.
[Bruneton's documented Earth atmosphere inputs](https://ebruneton.github.io/precomputed_atmospheric_scattering/atmosphere/demo/demo.cc.html).

For a camera ray from `a` to `b`, split at every fog-volume entry/exit, water
interface and opaque depth. Within each subinterval, sum the active
`σ_t=Σσ_{t,i}`. Evaluate a common source at sample `s`:
`j(s)=Σ_i σ_{s,i}(s)[p_i(μ_s) E_sun(s) + Σ_{l∈V} p_i(μ_l) E_l(s)] + j_amb(s)`.
`V` is the explicitly selected volumetric-light subset: Sun plus at most
**4 local lights per media cell** on desktop/browser and **2** mobile,
chosen by declared light participation/priority, not arbitrary truncation
from the 32/8 surface-light lists. More media-affecting lights reject the
profile or require an authored alternative. `E_sun(s)` includes global-air,
local-fog and selected cloud optical depth on the *light ray* once;
`E_l(s)` includes inverse-square/geometric source intensity, shadow and
total participating-medium attenuation to that sample. The direct Sun is
not pre-attenuated by atmosphere and then attenuated a second time.
`j_amb=σ_s L_amb` is an optional authored isotropic ambient approximation,
not a claim to solved multiple scattering. `p_i` is a normalized phase
function in sr⁻¹. Direct and ambient contributions have distinct ownership.

For a straight path in exponential air, use the analytic optical-depth
integral. If `y(s)=y0+s d_y`, then over length `d` above ground,
`τ_a=σ_{t,0}e^{-y0/H}(H/d_y)(1-e^{-d_y d/H})`, with the continuous
limit `σ_{t,0}e^{-y0/H}d` as `d_y→0`; clip at declared ground/support
boundaries. Homogeneous fog contributes its chord length times `σ_{t,f}`.
The same integrals evaluate source-ray attenuation to the Sun/local light.
For a view substep of length `Δs`, use exact constant-coefficient attenuation
`T_k=e^{-σ_t Δs}` and midpoint source `S_k=j_mid(1-T_k)/σ_t`, with
the `σ_t→0` limit `j_mid Δs`. Compose front to back as
`(T,S) ← (T T_k, S + T S_k)`. This combines overlapping coefficients
before integration. The midpoint source, lighting sample and isotropic
ambient are declared approximations; the operator is exact only for its
piecewise-constant source/coefficients. Independently cooked atmosphere and
fog `(T,S)` pairs are **never** treated as an exact overlap. Even disjoint
view-ray pairs compose exactly only when their source-light assumptions
include the same whole-scene light-path attenuation.

Initial volume image resolution is half the main view for 1080p desktop
and 720p browser/mobile, with depth-aware spatial upsample and no mandatory
temporal reprojection. Split at all support boundaries; subdivide where
`Δτ>0.5` or estimated source variation exceeds 10%, up to **32 samples/ray
desktop** and **16 browser/mobile**. Early termination at `T<10^-3`
requires an upper bound on omitted background and source radiance to keep
the resulting absolute error declared; it is not unconditional. Preflight
uses conservative support, optical-depth and closest-light bounds; it
rejects a declared camera/content envelope when they require more than
the cap, or labels an explicit lower-fidelity optical policy. These step
thresholds do **not** prove image error near sharp lights and boundaries.
Initial comparison targets against a high-sample joint reference
are ≤2% relative or 0.1 cd/m² absolute scene-luminance error for desktop,
≤5% or 0.2 cd/m² for browser/mobile in supported non-edge pixels; silhouettes,
fog boundaries, Sun shafts, water interfaces and motion need separate
visual review. These thresholds are design goals to falsify, not measured
facts or a proof that the sample caps suffice. The cost model is
`O(pixels × samples × participating lights)` plus source-ray intersection;
F2/T2 must include both views, light subsets, upload/transition peaks and
the root's 10/16 ms GPU envelopes before admission claims.

Optional precomputed atmosphere LUTs may supply background and direct-source
outputs in their stated parameter/ray domain. An aerial-perspective `(T,S)`
LUT cannot be independently overlaid with local fog in an overlap. Such a
profile either exposes atmosphere coefficients to the joint path, uses a
single provider that jointly recomputes the pair, or explicitly selects a
measured approximate layering policy. No LUT is required by the initial
baseline. A ray outside camera altitude/range/support, or through a water
boundary without the chosen optical interface, returns unsupported or an
approved fallback; it never silently clamps coordinates. Opaque depth
terminates air, water starts a separately owned underwater segment, and V4
receives only the final scene-linear colour/depth/history metadata.

## 6. What this closes and what must be demonstrated

This selects the initial source language/toolchain path, material feature
closure, binding ABI, product/runtime identities, RGB/photometric convention
and actual joint medium algorithm. It excludes a mandatory shader graph,
runtime shader invention, universal area-light/multiple-scattering model,
planetary atmosphere, arbitrary overlapping LUT composition and shader-byte
equality across peers. A gameplay-relevant fog/visibility field remains
authority-owned by E1/N1 and cannot be weakened by a client visual tier;
the local joint fog pipeline and its histories remain per peer. Source edits
cannot mutate a pinned server match; local GPU rebuild can leave that peer's
view loading while replication continues within N1's bounded queue policy.

Later qualification checks the pinned wgpu/Naga/browser WGSL features and
reflected ABI on each named adapter, all material/pass roles,
first-play and device-loss preparation, colour-card/lux luminance units,
joint-fog absorber-before/after-emitter and disjoint-light-path cases,
water-interface depth, multi-view history, cap rejection and a reduced
browser/mobile clip. Record source/toolchain hashes, target limits, driver,
pipeline counts/preparation time, visual error and complete frame/residency
trace. A failed numerical or device target narrows that profile or requires
an explicit provider redesign; it does not silently redefine the selected
transport or physical world. No such evidence exists in this handoff.
