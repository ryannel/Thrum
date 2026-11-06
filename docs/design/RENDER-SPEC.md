# Rendering specification: portable shaders, materials and transport

1 November 2026, revised 3 November for the V1 renderer reset. This selects an initial portable render path behind
[selected architecture](ARCHITECTURE-DECISIONS.md),
[C4](CONTRACTS.md),
[rendering](modules/RENDERING.md) and
[environment](modules/ENVIRONMENT.md). An [isolated R0 GPU diagnostic](../evidence/render/R0-GPU-DIAGNOSTIC-2026-11-02.md)
has since run, but no integrated renderer was built or device-performance qualified.
The limits below are **design admission targets**, not measured performance,
hardware support floors or permission to substitute physical gameplay content. [The baseline specification](BASELINE-SPEC.md) owns
the combined workload; [N1](NETWORK-SPEC.md) owns the selected network protocol; F4 remains the only cook/product authority, V1 the only GPU
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
adapter permits and hold local loading until completion. A driver cache is discardable,
keyed to the actual adapter, driver and recipe; it is never the F4 product. Native targets
persist it across runs by default so repeat launches skip pipeline compilation.
[WebGPU pipeline creation](https://gpuweb.github.io/gpuweb/#dom-gpudevice-createrenderpipelineasync),
[wgpu pipeline-cache contract](https://docs.rs/wgpu/30.0.1/wgpu/struct.PipelineCache.html).

**Alternatives:** GLSL/HLSL plus translation adds a second authored language and more
cross-target failure modes with no initial content need. Arbitrary runtime shader graphs
or custom generated shader code make the finite closure unverifiable. A later specialized
native provider may supply a separately versioned shader path, but it is not the portable
baseline or an implied material compatibility promise. No binary plugin ABI or WebGL
fallback is selected now. G3 decides, in a time-boxed spike, between WGSL with Naga
specialization and a shading library such as Slang compiled to MSL, SPIR-V and WGSL through
wgpu passthrough. The decision is judged on recipe variant count, specialization constants,
f16, subgroup, mesh-shader and ray-query reach on each recipe
([hardware-native decision](decisions/HARDWARE-NATIVE-ENGINE.md)).

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
Exact ordered alpha is parked; sorted alpha remains an explicitly approximate baseline.

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
Missing optional slots bind typed multiplicative defaults (white base, flat
normal, ORM `(1,1,1)`, white emissive) so the material layout does not
multiply variants. Native factor defaults are white base RGBA, zero emissive
radiance, roughness `1`, metalness `0`, occlusion strength `1` and normal scale
`1`. Base/alpha, roughness, metalness and occlusion-strength factors are finite
in `[0,1]`; emissive radiance factors are finite and nonnegative in cd/m².
The effective closure uses decoded/filter-sampled texels and these equations:

```text
baseRGBA = baseFactorRGBA * baseTexelRGBA
emissionRGB = emissiveRadianceFactorRGB * emissiveTexelRGB
metalness = metallicFactor * ORM.b
perceptualRoughness = max(0.045, roughnessFactor * ORM.g)
alphaGGX = perceptualRoughness²
AO = 1 + occlusionStrength * (ORM.r - 1)
```

For `mask`, coverage uses `baseRGBA.a >= alphaCutoff`; equality is covered.
Depth, shadow and forward evaluate that same sampled alpha, factor and cutoff.
The source grammar keeps an opaque material's cutoff absent rather than
inventing an authored value for the unused ABI lane.
The R0 [ideal sampler law](research/R0-SAMPLER-IDEAL-AND-TARGET-2026-11-03.md)
fixes mathematical ties, addressing and transfer before filtering; F4/V3
version the finite target evaluator, support envelope and boundary proof.

The BRDF's `base`, `m` and `roughness` are these effective values.
Base/emissive RGB are decoded from sRGB before linear filtering and
multiplication; base alpha and ORM remain linear. The roughness floor applies
**after** factor/texture multiplication and **before** squaring. AO modulates
only selected indirect light, including the authored R0 ambient Lambert term;
it does not change direct Sun/local lighting, emission, geometric/cloud
visibility or terminal background. The native metalness default is dielectric;
an imported format's different defaults are materialized during conversion.
For example, glTF 2.0 gives absent emissive and metallic/roughness texture
components an implicit value of one, while its metallic factor defaults to
one; its independent texture coordinates, transforms and samplers require
lossless conversion or explicit refusal under Thrum's one-sampler ORM layout.
[Khronos glTF 2.0 materials](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#materials).
Texture-presence flags may skip absent reads but cannot change these results.
An absent normal map uses the admitted geometric/shading normal and does not
evaluate a tangent basis. On front-facing raster fragments, reflect a negative-view-cosine shading normal
across the view tangent plane before lighting; retain the original normal for shadow bias and leave real
backfaces and low-level BRDF hemisphere checks unchanged. This is a raster correction, not geometric visibility
([production rationale](https://github.com/Unity-Technologies/Graphics/blob/master/Packages/com.unity.render-pipelines.core/ShaderLibrary/CommonLighting.hlsl#L412-L447)). A present normal map requires a validated tangent
basis and a separately supported evaluator; the first bounded R0 proof refuses
it before ready-product publication. A later mapped-normal profile must cook
tangents with a declared method. Material factors, alpha cutoff and
texture-presence flags are uniform data, not compile-time variants. Structural shader
specialization is limited initially to alpha mode (`opaque/mask/blend`), geometry
interface (`static/skinned/instanced`), pass role, and chosen material closure/provider.
The closure creates only actually reachable tuples, not their full product. Surface
light count is runtime data and does not create a shader per number of lights.

Use four explicit bind groups, within WebGPU's guaranteed baseline: `0` view/frame;
`1` scene lights/clusters/shadows/environment/medium; `2` a storage-buffer material
table, four typed 2D texture arrays and samplers; `3` indexed instance/transform
storage and skin palette. F4 assigns texture layers within declared size/format
classes; class or layer overflow creates another prepared bucket or a declared
visual fallback, never an out-of-range shader index. Passes declare exact subsets
of this layout; extensions cannot silently add a fifth group or assume bindless
textures. `instance_index` selects the object record, material row and transform;
no dynamic per-object uniform binding is required. The integrated workload's 32 surface
lights per cluster (8 mobile) is an F2 capacity limit. Oversubscription keeps a
deterministic highest-contribution subset, counts omitted references and shows
the quality degradation in developer telemetry; no shader reads beyond the
prepared list. Resource/texture counts and storage sizes must pass the
queried adapter limits; wgpu's documented default WebGPU limits include four bind
groups, 16 sampled textures and 256-byte dynamic uniform offset alignment.
[wgpu limits](https://docs.rs/wgpu/30.0.1/wgpu/struct.Limits.html).

**CPU/WGSL ABI revision 2:** WGSL buffer structs use only `u32`, `f32`, `vec4` and
`mat4x4<f32>` in portable shared records; no `bool`, `vec3` or implicit Rust padding.
Matrices are column-major. A `ViewRecord` contains `viewProj` and
`previousViewProj` at byte offsets 0 and 64 (64 bytes each), then
`cameraXYZ_time` at 128 and `preExposure_invExposure_jitterXY` at 144; 160-byte
record, 256-byte dynamic-uniform stride. A `MaterialRecord` has four 16-byte
vectors at offsets 0/16/32/48: base RGBA, emissive RGB+alpha cutoff,
`(occlusionStrength,roughnessFactor,metallicFactor,normalScale)`, and four
`u32` semantic flags/IDs; the fifth `vec4<u32>` at 64 names base, normal,
ORM and emissive texture-array layers. It is 80 bytes. Material products, default textures, shaders and
representation certificates pin one material-semantic/evaluator revision in
their keys; equal byte layout alone cannot authorize mixed meanings.
An `ObjectRecord` stores current/previous model matrices (0/64), three normal
matrix columns padded as `vec4` (128/144/160), and a `vec4<u32>` of stable
instance index, material row, skin offset and source generation (176); 192-byte stride.
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
required joint-medium path never disappear silently. Reference-profile build validation
fails if that recipe exceeds **512 distinct pipeline descriptors**, with
**64 boot-critical** and **128 newly required in any one admitted live switch**
as initial reference-recipe caps. These are neither general SDK/format ceilings nor device limits;
BASELINE-SPEC gives the smaller R0/R2 boot closure and preparation estimates. Other game recipes declare their own
finite caps and admission analysis. The first small-game target needs fewer. An author must simplify a
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
reset. This reduces format saturation without inventing another lighting unit.
`p` and output gamut/tone policy are versioned view metadata. Authoring and
authoritative numeric boundaries still reject invalid light facts. Presentation
uses safe finite arithmetic, then clamps scene colour to the selected storage
range **after pre-exposure and before fp16 storage**. Count high/low clamps and
nonfinite repairs per pass and expose them in the profiler; developer builds
may assert on unexpected counts. Use a finite fallback colour for a
presentation-only calculation outside support, reset affected temporal
history, and keep the world tick and other passes running. Geometry and
floating-point comparisons use scale-aware tolerances checked against visible
error; sub-ULP edge cases do not require exact predicates on the render path.
The retained [arithmetic-envelope study](research/R0-ARITHMETIC-ENVELOPE-2026-11-03.md)
is not a V1 gate.

## 5. Initial atmosphere, fog and transport operator

Select Hillaire-style atmospheric precomputation for the first dynamic sky:
transmittance (proposed 256×64), multiple scattering (32²), sky view
(192×108) and aerial perspective (32³). Sky-view filtering separates the curved horizon’s sky/ground limits.
Planet boundary light includes a profile-cached ground sky-irradiance table, sampled with the terminal normal.
Use a spherical planet with Rayleigh, Mie and ozone coefficients; update LUTs only when their Sun,
weather, camera-height or profile dependencies change. The Sun disk and Moon
are separate celestial emitters with one named light identity each. Generate
sky-derived diffuse SH and a prefiltered specular cubemap from the same disk-free Sky LUT.
Initially complete each changed-sky cube coherently; add staggered cadence only when measured work needs it;
an authored HDRI is an optional override, no longer a mandatory terminal
background. Begin with [Hillaire 2020](https://sebh.github.io/publications/egsr2020.pdf);
G3s records LUT cost on this M3 Max. An independent all-orders scalar CPU referee first checks the same
declared coefficient bands; measured spectral/CIE validation remains distinct and open.

V3 owns one source-to-receiver transport closure. E1 supplies deterministic
headless Sun direction, light level, wind and visibility from seed, time and
weather state. Gameplay never samples the sky texture. V3 applies cloud
optical depth to each direct Sun path once; the sky/IBL, local fog, clouds and
water share the same source identity and applied-attenuation metadata. A
cloud-shadow map may exist without drawing a cloud. A skipped visual tier
keeps authoritative environmental effects and reports its presentation
fallback.

Desktop and WebGPU use a joint froxel integrator for aerial perspective,
height/local fog and participating lights. The proposed first grids are
160×90×64 desktop and 128×72×32 browser; mobile starts with the atmospheric
aerial-perspective LUT plus analytic height fog as an explicitly approximate
layering policy whose overlap error must be measured. Within each froxel, combine
extinction and source coefficients before integration. For a segment,
`L_out = T L_in + S`; compose ordered disjoint segments as
`(T_near T_far, S_near + T_near S_far)`. Independently lit overlapping
`(T,S)` images do not become exact by blending. Water and transparent
surfaces sample the same cumulative view transport at their depth. Temporal
reprojection tracks atmosphere, fog, Sun and cloud epochs and resets when the
source, camera or topology invalidates it. The froxel budget, light subset,
step count, error against a high-sample reference and GPU pass times are
measured in M5; the proposed dimensions are not qualification.

Presentation degrades explicitly when a LUT or froxel pass misses its budget:
reuse a valid history within its declared age, reduce resolution/steps, or
fall back to analytic fog/clear sky. Count and display quality downgrades.
Authority, content identity and gameplay visibility still fail closed on
invalid data. Exact cloud/fog/water overlap remains a selected joint-provider
responsibility, not independent screen-space overlays.

## 6. What this closes and what must be demonstrated

This selects the initial source language/toolchain path, material feature
closure, binding ABI, product/runtime identities, RGB/photometric convention
and actual joint medium algorithm. It excludes a mandatory shader graph,
runtime shader invention, universal area-light/multiple-scattering model,
unrestricted orbital cameras, arbitrary overlapping LUT composition and shader-byte
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

## 7. R3 transparency and regional optics — parked

R3 regional water optics and exact ordered-alpha research are retained in
[the earlier study](research/R3-OPTICS-AND-TRANSPARENCY-DESIGN-2026-11-02.md)
for M9, after a running renderer and measured water exist. V1 uses sorted
premultiplied alpha as an explicitly approximate baseline. If an optional
fragment or effect budget overflows, preserve the best available image,
record telemetry and lower presentation quality. An authoritative visibility
or gameplay query never relies on that approximation.

## 8. R0 first rendered game contract and proof

This section selects the first bounded V1–V4 integration from the
[render implementor walk](research/RENDER-IMPLEMENTOR-DOGFOOD-2026-11-02.md).
It is a **design and build contract**, not a claim that a renderer or target
pipeline already exists. M1 starts with a synthetic in-memory scene; A6 later
connects cooked products and exact source/build identity before production
content claims. Synthetic GPU timings cannot qualify the cook path.

**Fixture and authority.** One admitted R0 recipe has one perspective camera,
one static geometry interface, opaque and masked metallic-roughness materials,
one directional Sun with a shadow, four unshadowed local lights, fixed authored
ambient and exposure, and an optional flat UI overlay. There is no water,
weather, terrain, air/fog, animation, transparent geometry or temporal AA in
this first proof. F3/F4 identify the immutable source/build and actual reachable
mesh/material/shader/pass products; W1 supplies one committed world endpoint.
V2 extracts a view packet without stepping the world. V1 alone prepares GPU
resources and submits passes; V3 supplies light/material/visibility; V4 owns
scene-to-display conversion. No private feature queue or first-draw shader
compile is part of this profile.

**View packet and GPU ABI.** A packet names world/source/build, presentation,
device and view generations, source sample serial and committed endpoint,
stable camera/view identity, viewport, projection, current world-relative
origin and product leases. View identity is not a quality-profile ID. A
skipped, failed or unsubmitted view does not advance its history/sample
record. The first spatial profile consumes no retained colour history; it
still records the sample actually submitted so later providers cannot infer
one from global frame parity. A changed origin, camera cut, device generation,
size or incompatible source/product revision invalidates prior correspondence.
For a later temporal profile, submitted metadata and accepted-history
metadata are distinct: a GPU-rejected sample cannot advance the previous
pose, exposure, source epoch or history image used by its successor. The first
temporal proof resets history whenever acceptance is unknown to the CPU;
retaining history across delayed GPU acceptance requires an explicit GPU-owned
metadata path or bounded delayed lease protocol. Submission completion alone
does not certify an accepted sample.
Retain the 160-byte `ViewRecord` above. Group 0 binding 1 adds a versioned
128-byte `ViewAuxRecord`: unjittered inverse view-projection at 0, viewport
size/inverse size at 64, finite near/far plus view scale/storage ratio at 80,
split current/prior presentation times at 96, and u32 camera/history/source
flags at 112. Its byte layout and the existing records are checked against
Naga reflection; CPU packers write checked scalar offsets into zeroed storage.
Use right-handed Y-up world, camera forward −Z, WebGPU clip depth [0,1],
finite reverse-Z depth (clear 0, greater comparison) and unjittered matrices
for correspondence. Perspective and orthographic projection/reconstruction
must pass independent point/ray round trips before the wider view promise.

**First pass and resource closure.** V1's typed graph declares scope,
format/usage, initialized subresource reads and writes, clear/load behavior,
producer/pipeline generation, external side effect and last use. Its R0 order
is Sun cascade shadows, opaque/masked depth, opaque/masked forward lighting into
scene-linear RGBA16Float, tone/encode, spatial AA, optional UI and final output.
Visibility includes casters outside the camera frustum. The selected first V1
Sun-shadow tier uses two 1024² D32 cascades over finite perspective depth,
with a short overlap blend at the split. Each light-space receiver footprint
is stabilized to map texels and includes relevant off-camera casters. Four
comparison taps sample each selected map; constant/slope bias is an authored,
dimensioned recipe value converted through the actual map projection. The
map count and resolution are a quality tier to measure on named devices, not
a claim of visual or frame-budget qualification. Masked shadow, depth and forward
evaluate the same UV/alpha cutoff and geometry. A missing required role rejects
view readiness; it never becomes an opaque mask or omitted required shadow.
Render-derived light/instance demand over capacity uses a declared deterministic
quality fallback with visible counters, while invalid authored source still
refuses admission. Off-state
passes/resources retire after their final CPU reader and GPU completion.

**Light and display.** R0 ambient is a typed isotropic diffuse irradiance in
linear Rec.709 photometric RGB, in lux; the reference fixture uses
`[1000,1000,1000]`. Its Lambert contribution is
`AO*(1-metallic)*base*E_ambient/pi`, with no second hemisphere or cosine factor.
The separate terminal background is black radiance in cd/m². Sun lux and
local candela use section 4's direct-light convention; neither ambient nor
background stands in for an unselected reflection source. V4 keeps storage scale `p=1/1024` separate from camera gain
`e`. Explicit Fixed preserves the authored positive gain and CPU-packed reciprocal bits. The outdoor atmosphere
camera may instead select a horizontal incident meter: `E = sky_horizontal + Sun_perpendicular*max(sun.y,0)` in
physical RGB lux, with Rec.709 luminance weights. Sky irradiance already includes its cosine hemisphere integral.
One GPU resolve after diffuse lighting computes `EV100=clamp(log2(max(E_lux,E_floor)*100/250)-compensation,minEV,maxEV)`
and `e=2^(-EV100)/1.2`; the game initially authors compensation 0 and EV bounds 0–18.
`E_floor=2.5*2^(minEV+compensation)` reaches the lower EV bound without adding illumination; recorded lux stays unfloored.
This flat-receptor calibration follows [Filament's camera model](https://google.github.io/filament/main/filament.html#imagingpipeline/physicallybasedcamera)
and [illuminance conversion](https://github.com/google/filament/blob/main/filament/src/Exposure.cpp); no implementation is copied.
The separate GPU sample carries `p`, inverse gain, actual unfloored lux, EV, flags and submitted identity. Fixed mode
is flagged unmetered; invalid physical input uses the prepared fixed fallback. Resolve every encoded view, including
cached atmosphere views. Tone declares a read of this resource; policy and physical-light inputs are declared graph
reads. Two sample sides follow submitted-transform history, never prepared-view parity; cancellation/capture cannot
advance history. Future reconstruction retains the sample with its accepted image. The 160-byte view ABI and reserved
jitter fields are unchanged; its legacy inverse gain is fixed-request compatibility data, not a resolved meter value.
Display selection is independent of metering. Default `Reinhard` preserves `C_e=e*L`,
`C_t=C_e/(1+Y(C_e))`, `C_d=C_t/max(1,max(C_t.r,C_t.g,C_t.b))` and existing diagnostic pixels.
The game selects `NeutralHighlights`: identity below peak `K=0.76`; above it, with `P=max(C_e)`,
`N=1-(1-K)^2/(P+1-2K)`, `g=1/(1+0.15*(P-N))`, `C_d=N*mix(white,C_e/P,g)`.
This uses [Khronos's highlight shoulder/desaturation](https://github.com/KhronosGroup/ToneMapping/tree/main/PBR_Neutral)
without the material/Fresnel shadow offset; it is not a standard PBR Neutral reproduction or material-colour guarantee.
Evaluate black first and branch before computing reciprocal peak; the reciprocal formulation handles every checked
gain without forming overflowing exposed RGB. This adds scalar ALU per output pixel in the existing tone pass,
without another pass, texture or queue. The request/sample reserved word carries the operator; the view jitter ABI
and scene-linear colour remain untouched. `[0,1]` is relative to SDR display white; future HDR selects its transform
before SDR compression. [GT7's 2025 display pipeline](https://www.polyphony.co.jp/publications/) is a later cross-display
reference requiring explicit display nits and output gamut, rather than an undeclared replacement here.
Encode `C_d` to sRGB for the spatial filter, decode its result to
display-linear for UI composition, then encode the selected output once,
either in the shader or by an sRGB attachment conversion, never both.
Presentation repairs nonfinite radiance and clamps after pre-exposure before half-float storage, with per-pass counters and a visible developer diagnostic. This
SDR profiles are bounded initial choices, not an HDR or photographic-look promise. Incident metering is immediate;
histogram metering and temporal camera adaptation are not implemented.

**Finite demand and proof.** The spatial fixture reaches six geometry-role
tuples (opaque/mask × depth/shadow/forward), exposure resolve, tone, spatial AA and output,
plus one UI tuple only if the authored fixture uses it: ten or eleven boot
descriptors, within BASELINE-SPEC's eleven-tuple worksheet and the 16-tuple
R0 recipe cap. Velocity roles are available to a separately admitted
temporal profile/transition but are not prepared or retained by unused R0
spatial content. F4 enumerates the actual descriptors/keys and V1 verifies
target formats, layouts and device readiness before the first playable view.
Pass declarations and allocation traces account for shadow, HDR, display
intermediate, output, uploads and pending retirement at their simultaneous
peak; aliasing is counted only when lifetimes and descriptors permit it.
The first proof checks source/build and GPU ABI identity, mask agreement,
BRDF/colour cards, projection round trips, first input-to-visible result,
pipeline failure, stale packet, device loss and source restart. Inspect moving
native-resolution output, then record cold preparation, per-pass CPU/GPU,
allocation/bytes and sustained whole-frame pacing against BASELINE-SPEC's
named target. These are unrun gates.

The [bounded first resource-lifetime schedule](research/R0-RESOURCE-LIFETIME-SCHEDULE-2026-11-03.md)
selects one queue and four scratch textures for an isolated 720p spatial proof.
Its [logical oracle](../evidence/render/R0-RESOURCE-LIFETIME-ORACLE-2026-11-03.md)
tests frame credits, held submissions and resize retirement. It does not
establish a real output surface, F4 leases, GPU allocation or target viability.
The [synthetic four-role Metal chain](../evidence/render/R0-FOUR-ROLE-SYNTHETIC-2026-11-03.md)
executes those scratch formats and one output path with exact offscreen pixels;
it does not yet execute the authored R0 game, a two-credit held-frame resize
or measured device-memory residency.

**Spatial implementation input.** The selected spatial operator is FXAA 3.11
Quality on the encoded scene image. Its exact reference bytes, provenance and
first port configuration are [pinned here](../evidence/render/FXAA-SOURCE-SELECTION-2026-11-02.md).
An isolated [masked GPU diagnostic](../evidence/render/R0-GPU-DIAGNOSTIC-2026-11-02.md)
ran a first WGSL port on Metal. Before calling the pass complete, promote a
versioned port with its notices, offline Naga validation, independent
reference-image checks and whole-R0 cost. NVIDIA's
[FXAA integration paper](https://developer.download.nvidia.com/assets/gamedev/files/sdk/11/FXAA_WhitePaper.pdf)
describes the order and single-pass method; a redistributor's
[archived source copy](https://github.com/GameTechDev/CMAA2/blob/071c6b0857559f4e36f614362e6d2aab1b61938a/Projects/CMAA2/FXAA/Fxaa3_11.h)
exposes the 2014 NVIDIA redistribution terms. Neither a visually similar
filter nor an unverified licence assumption satisfies this port gate.
