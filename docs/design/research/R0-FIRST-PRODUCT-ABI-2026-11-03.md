# R0 first render product and packet ABI — candidate

3 November 2026. This is an implementor-facing candidate for the
[render-source grammar](R0-F3-RENDER-SOURCE-GRAMMAR-2026-11-03.md) and
[first admitted handoff](R0-FIRST-ADMITTED-RENDER-SOURCE-2026-11-03.md).
No F4 render cook, V1 pipeline or device-ready build exists yet. The
[governing render ABI](../RENDER-SPEC.md#2-material-closure-and-binding-abi)
already fixes the 160-byte view, 128-byte auxiliary view, 64-byte material
and 192-byte object record sizes. This note resolves candidate meanings and
product boundaries for the first six-triangle R0 proof; promotion requires
producer and consumer agreement, Naga reflection and target validation.
The selected first constant-light cooker itself remains unimplemented and
currently specifies a **schema-2** entry-scene closure. Before R0 can use
`core.scene@3`, extend that F4 closure to accept its unchanged
`environment.source` edge and include the new scene definition hash in the
exact-source build binding. The 160-byte Sun product and its semantic key can
remain byte-identical when the selected environment/light meaning is equal;
the scene version and exact source revision must still create a new binding.

## One pin, two meanings of reuse

F4 opens one accepted F3 pin and follows the selected `core.scene@3` typed
render/environment edges. A shared versioned F3 decoder reads authenticated
root bytes once per cook. A checked semantic key includes the relevant
decoded values, built-in definition hash, material/geometry evaluator
revisions, target profile, cook/shader ABI and typed **semantic** dependency
digests. Exact source-file hashes and the root identity belong to the
separate build binding; putting them in every semantic product key would
defeat formatting-only reuse.
For identity-keyed render arrays, sort by stable UUID before keying and
product enumeration; preserve ordered vertex, triangle and texture bytes.
Formatting-only source edits may therefore reuse authenticated product bytes.

A separate exact-source build binding names the admitted `SourceRevision`,
entry scene/render root, verified product key→actual-byte-hash manifest and
complete required descriptor set. Reopen and hash every product before
publication. Reusing an equal semantic product never implies that its prior
build binding authorizes a new source root. An incomplete, mixed or failed B
candidate leaves A's selected build and leases intact. Neither a scene
product nor the selected but unimplemented `ConstantLightProductV1` contract
alone is a ready R0 build.

| Product | First representation and lifetime | Why it exists |
| --- | --- | --- |
| Exact mesh facts | Immutable compact f64 decoded-position and UV bits with negative zero canonicalized to positive zero, plus u32 source triangle order, retained once per ready build and leased by view certificates. Preserve enough exact values to reconstruct predicates. | Moving-camera source signs must not reopen/decode up to 16 MiB of pinned JSON per view. This CPU product is distinct from the f32 GPU mesh. |
| Flat GPU mesh | For the first at-most-six-triangle recipe only, expand corners to one nonindexed 32-byte stream: **mesh-local** f32 position XYZ at byte 0, geometric normal XYZ at 12, UV0 at 24. Shadow, depth and forward consume the same immutable stream. | Avoid an unproved normal-buffer lookup and avoid pretending a six-face layout is a scalable general mesh format. Broader scenes require measured corner sharing/indexing and a new geometry product profile. |
| Material/texture | Existing 64-byte `MaterialRecord`, three possible authored RGBA8 textures, bound typed white base/ORM/emissive and flat-normal defaults, one sampler per material. A source absent map gets the neutral binding; a present map must have a verified texture product. | The R0 shader uses the same factor/texel equations and sampled alpha across roles. A schema-1 normal map is invalid source. |
| Camera | Typed authored initial pose, vertical FOV, near/far and fixed exposure from the selected camera ID; W1 consumes initial pose while V2 uses the committed presentation pose. | Projection/exposure depend on source semantics; current viewport supplies aspect and is not a second authored camera value. |
| Sun/environment | The selected but unimplemented 160-byte `ConstantLightProductV1` remains a CPU f64 product contract with exact identity/units/provenance. | It is not a WGSL buffer; V3 packs target-range GPU light data from it after the admitted-source cooker exists. |
| Local lights | Four stable W1 identities and authored descriptors, with coherent W1 positions from the packet sample. | V3 owns one registry and no renderer-local light state. |
| WGSL/pipelines | Canonical expanded WGSL, Naga validation/reflection, pass ABI and full immutable descriptors. Six geometry tuples (`opaque/mask × shadow/depth/forward`) plus tone, spatial AA and output; optional UI adds its declared tuple. | Light count is runtime data within the first-profile four-local capacity, not a shader variant. Every required tuple is device-ready before the view is admitted. |

First-profile F4 refuses `two_sided=true`, nonidentity/animated instances,
source triangles whose representation/error certificate fails, unsupported
light distance or radius, and masked materials whose min/mag filter differ.
These are valid-source versus first-product distinctions except the schema-1
normal-map field, which F3 refuses. The exact CPU facts product is small for
six triangles; its retained memory, old/new overlap and per-view predicate
cost must still be charged. A future large-mesh profile cannot inherit its
unmeasured retention cost.

## Candidate GPU records and operation ownership

The 288-byte per-view light record uses only aligned `vec4<f32>`/`u32` and a
column-major `mat4x4<f32>` under the existing four-group layout. Offsets are
byte offsets, not Rust struct-layout guesses:

| Offset | Size | Meaning |
| ---: | ---: | --- |
| 0 | 16 | `u32` local count `4`, required Sun-shadow flag `1`, two reserved zeros; any other flag is an R0 readiness refusal. |
| 16 | 16 | Sun receiver-to-source XYZ and angular radius radians. |
| 32 | 16 | Sun photometric RGB perpendicular illuminance lux; fourth lane zero. |
| 48 | 16 | Isotropic ambient RGB irradiance lux; fourth lane zero. |
| 64 | 16 | Terminal background RGB radiance cd/m²; fourth lane zero. |
| 80 | 64 | View-origin-relative world→Sun-shadow clip matrix, column major; the shadow camera looks along source-to-receiver `−Sun` while the lighting vector remains receiver-to-source `Sun`. |
| 144 | 16 | Versioned shadow bias/texel parameters from the render preparation recipe; exact lane meanings remain to be pinned with the actual shadow projection. |
| 160 | 128 | Four 32-byte local-light entries, sorted by stable UUID: relative position XYZ/radius m, then photometric RGB luminous intensity cd/zero. |

V2/V3 pack this record from one coherent W1 presentation sample and the
authenticated F4 Sun/local-light products. V1 owns upload and a separate
record per in-flight packet credit; no CPU write may race a retained GPU read.
The retained angular radius does not make the first four-tap shadow a
physical penumbra; its visibility model remains the selected shadow recipe.
The front-lit receiver/offscreen-caster fixture must catch a shadow matrix
that uses the opposite Sun sign.
The Sun shadow matrix/parameters depend on the actual authored receiver
volume, map extent and bias recipe. Origin, camera and light pose changes
invalidate the relevant representation evidence and pack. The CPU stable
UUID registry and exact binding remain authoritative; GPU local ordinals do
not create new light identities. Local light positions and all normal/UV
coordinates are checked when narrowed to f32; a physically valid f64 source
can be F4 unsupported on a target.

The 64-byte material record's final four `u32` lanes need one frozen meaning
before any WGSL is authored. Candidate lanes at offset 48 are:
`flags`, `0`, `0`, `0`. No unrelated material's insertion may change this
material's product bytes. The CPU build binding maps stable UUIDs to products
and draw bindings; no GPU ordinal is needed for this first profile.
Candidate flag bits are 0
base-texture present, 1 ORM present, 2 emissive present, 3 alpha mask and 4
two-sided; bits 5–31 are zero and no bit grants unsupported schema-1 normal
mapping. Uniform
presence flags can skip neutral texture fetches without adding shader
variants; this optimization needs shader measurement and must preserve the
same decoded factor/texel result. F4 verifies flags against bindings and
pipeline alpha/cull mode before publication. At byte 16's fourth f32 lane,
opaque materials pack positive zero; masked materials pack the authored
cutoff after checked f64→f32 conversion and cutoff-boundary qualification.
The mask flag, shader specialization and cutoff lane must agree.

The current `ViewRecord` names byte-144 lanes
`preExposure_invExposure_jitterXY`, but V4's tone operation on stored `pL`
needs `e/p` to compute `C_e=eL`. Candidate revision-1 lanes are
`(storageScaleP, toneGainEOverP, jitterX, jitterY)`; V2/V4 compute and range
check `e/p` once per packet in f64, then pack f32. The first spatial profile
uses zero jitter. The `ViewAuxRecord` byte-80 lanes should state finite
`near`, `far` and two explicitly reserved zeros until a real view-scale use
is selected; do not have an unnamed `storage ratio` duplicate exposure
authority. These candidate changes to the governing ABI need producer/consumer
review, a single Rust offset table and Naga-reflected WGSL checks. The tone
shader should multiply by the precomputed gain, not divide each fragment.

## Source geometry and pass equivalence

F3 admits either nondegenerate winding by exact decoded-f64 cross product.
F4 derives a flat world geometric normal from that order for the first
identity instance and verifies f32 representation error. The GPU raster
corner positions are never cooked relative to a moving view origin. V2 packs
each packet's `ObjectRecord` model matrix with the world→current-view-origin
translation; for the identity source instance this is `−origin`. Its previous
matrix uses the same origin convention when correspondence is valid, or is
reset with history on an origin shift in the first spatial profile. Exact
source-sign facts stay in physical world f64 coordinates. The GPU raster
front-face/cull convention after right-handed world→WebGPU clip projection
must be pinned and tested with front/back cards; a winding swap cannot be
hidden by negating the shader normal. The offscreen caster must still write
Sun shadow depth and shadow a visible receiver region.

Depth, shadow and forward use one cooked corner/UV product and one versioned
alpha evaluator, with the same base alpha factor, sRGB texture's linear alpha
sample and mask cutoff. Equal min/mag filter and one mip remove a derivative
filter-choice disagreement, but WebGPU filtering precision is target
dependent. F4/V1 must prove cutoff-boundary behavior on the selected device
or refuse that masked product. All producer operations and intermediates,
including normalization, inverse-square distance, shadow bias, BRDF ratio,
pre-exposure and half-float storage, need range proofs before dangerous f32
operations. A final finite check is insufficient.

## Remaining closure

The render preparation recipe still needs an exact source/binding shape for
Sun receiver volume, light-space projection, 2048² map, four comparison taps,
dimensioned constant/slope bias, viewport/output extent, storage scale,
shader/pass/operator revisions and target limits. The provisional six-face
positions/texels need to be saved as admitted bytes and checked for offscreen
caster visibility, mask consistency, visual cards and moving output. Then
measure complete frame CPU/GPU time, overdraw, texture bandwidth, retained
exact-facts bytes, packet-credit overlap and physical/device high-water.
Neither the isolated GPU diagnostics nor the CPU predicate/index probes
establish those costs.
