# First R0 Sun-shadow recipe and offscreen-caster proof — candidate

3 November 2026. This is a bounded candidate recipe for the
[first R0 source](R0-FIRST-ADMITTED-RENDER-SOURCE-2026-11-03.md) and
[product ABI](R0-FIRST-PRODUCT-ABI-2026-11-03.md), not an admitted fixture,
selected complete F2 recipe or ready shader. It spells out one static
six-triangle shadow proof. The complete source bytes, constant bias, grazing
cutoff, target error and contact-quality limits remain to be pinned. Wider
scenes need a separately measured shadow profile; a single fixed 2048² map
is not a general directional-light solution.

## Geometry that exercises offscreen visibility

Use three +Z front-facing quads, each two triangles with indexed order
`(0,1,2),(0,2,3)` around bottom-left, bottom-right, top-right, top-left:

| Surface | World-space bounds, metres | First role |
| --- | --- | --- |
| Receiver | `x∈[-3,3]`, `y∈[-2,2]`, `z=0` | Opaque visible receiver. |
| Fence | `x∈[-0.6,0.6]`, `y∈[-1,1]`, `z=1` | Alpha-masked visible occluder; exact texture bytes/cutoff still need admission. |
| Caster | `x∈[3.4,3.8]`, `y∈[-0.4,0.4]`, `z=2` | Opaque caster outside camera view. |

With camera `(0,0,5)` looking −Z, 60° vertical FOV and 16:9 viewport,
the caster's z=2 horizontal half-frustum width is about 3.079 m, below its
minimum x=3.4 m. The Sun source value is nominally
`s=(0.6,0,0.8)` receiver-to-source; rays travel `−s`. A ray from the caster
at z=2 reaches the receiver at x shifted by −1.5 m, so its shadow occupies
receiver x about `[1.9,2.3]`, visibly inside the camera frustum. A shadow
pass culled by the camera frustum therefore fails this fixture. The fence
also needs its own masked shadow and matching depth/forward alpha decision.
These coordinates are proposed exact source inputs, not hard-coded engine
geometry or schema limits.

## Fixed light-space volume and reverse-Z map

Derive a normalized Sun `s` **from the authenticated 160-byte F4 constant-
light product**, using its pinned evaluator. Do not independently normalize
literal decimals in a second shadow implementation. For this nonvertical
Sun, define `r=normalize((0,1,0)×s)` and `u=s×r` with a versioned ordered f64
basis evaluator. The nominal basis is `r≈(0.8,0,-0.6)`, `u≈(0,1,0)`.
Set `ξ=r·P`, `η=u·P`, `ζ=s·P`. The first authored orthographic light box is
`ξ∈[-2.5,2.5] m`, `η∈[-2.1,2.1] m`, `ζ∈[-2,4] m` anchored at world origin.
The nominal bounds are:

| Surface | ξ interval m | η interval m | ζ interval m |
| --- | ---: | ---: | ---: |
| Receiver | `[-2.4,2.4]` | `[-2,2]` | `[-1.8,1.8]` |
| Fence | `[-1.08,-0.12]` | `[-1,1]` | `[0.44,1.16]` |
| Caster | `[1.52,1.84]` | `[-0.4,0.4]` | `[3.64,3.88]` |

F4 recomputes/checks all exact decoded source vertices against the actual
normalized product basis, its f32 matrix error and the full comparison-tap
receiver footprint. Nominal receiver margins are at least 0.1 m: about 41
texels across ξ and 49 across η at 2048². A failed containment/guard check
is typed unsupported, not clipping to a plausible shadow. The light volume
is independent of the moving camera; projection does not refit each frame.

Use one 2048×2048 D32 float depth texture, one mip, clear depth zero,
depth writes on and `greater` depth test in the Sun-shadow pass. Fixed-
function depth bias fields are zero. Nominal world→shadow clip coordinates
are `x=2(ξ+2.5)/5−1`, `y=2(η+2.1)/4.2−1`,
`d=(ζ+2)/6`, `w=1`; WebGPU depth lies in `[0,1]` and source side maps to
one. The world→texture UV mapping is `((x+1)/2,(1−y)/2)` with top-left
texture origin. The receiver's minimum nominal depth is about 0.033 and
the caster's maximum about 0.98. The shadow camera looks along `−s`, while
V3 direct-light cosine uses `+s`.

GPU object vertices are mesh-local f32. For view origin `O`, V2 packs the
object transform to origin-relative world coordinates; the first identity
instance has translation `−O`. Pack `M_shadow_rel=M_shadow_world·T(O)` in
the per-view light record, so in exact arithmetic
`M_shadow_rel·T(−O)·P=M_shadow_world·P`. A new origin invalidates the packed
matrix/error certificate, but **does not itself invalidate static world-space
shadow-map contents**. A retained map can be reused while source geometry,
Sun, light box, mask evaluator, recipe and device generation stay equal and
the retained **producer certificate** proves its old-origin rasterized depth
against the world-space source, a new-origin **consumer certificate** bounds
the current sampling matrix/reference against those stored depths, and V1
leases prove no write/read race. Equal keys alone are insufficient when
origin-dependent f32 producer rounding changes the depth map. An initial
implementation may redraw per frame as a measured schedule choice; it must
not claim that origin shifts semantically require a redraw.

## Four comparisons and receiver-side bias

Use a nearest comparison sampler with clamp-to-edge and anisotropy one.
For UV in map coordinates set `q=uv·2048−0.5`, `i=floor(q)`, and compare at
the four texel centers `i+(0,0)`, `i+(1,0)`, `i+(0,1)`, `i+(1,1)` using
`textureSampleCompareLevel` at mip zero. For each integer offset `δ`, the
normalized WGSL coordinate is
`(vec2<f32>(i)+vec2<f32>(δ)+vec2<f32>(0.5))*lane0`, where
`lane0=1/2048`;
sampling `i/2048` would land on texel boundaries. Each result has weight `1/4`.
Require `i.x,i.y∈[0,2046]` for the admitted receiver footprint instead of
depending on edge clamp. This 2×2 box has a phase that can shift its center
by up to half a texel from the query; verify edge drift and visual quality.
It is filtered visibility, not a penumbra derived from the Sun angular
radius. An independent four-`textureLoad` scalar-comparison oracle checks
each tap and the final average on each target.

For reverse-Z, a nearer caster stores a **larger** depth. Use comparison
`greater-equal` with receiver reference `d_receiver+biasDepth`: a caster at
stored depth 0.8 shadows a receiver reference 0.5, and a small positive
receiver bias reduces self-shadow acne. The bias is computed in physical
metres before converting by the box depth span `Lζ=6 m`. With geometric
receiver normal `n`, `n_s=n·s`, `n_r=n·r`, `n_u=n·u`, texel widths
`Δξ=5/2048 m`, `Δη=4.2/2048 m`, authored constant `b0_m≥0` and authored
slope coefficient `β≥0` measured in texels, the candidate is

`biasDepth = [b0_m + β(|n_r/n_s|Δξ + |n_u/n_s|Δη)]/6`.

Branch before division unless a certified lower bound on `n_s` is positive;
refuse a grazing or excessive-bias receiver before ready publication. Do not
silently clamp. The packed `d_receiver+biasDepth` must remain finite and in
`[0,1]` for every admitted receiver before comparison sampling. `b0_m` must
cover the actual f32 transform/raster/D32 error
and be checked against a fixture-specific maximum acceptable contact
detachment. Neither numeric value is selected here. A plausible study point
`β=1` gives about 1.831 mm of ξ slope allowance on the flat +Z receiver
before `b0_m`; it is not an approved bias.

The candidate 288-byte light ABI's offset-144 `vec4<f32>` lanes are
`(1/2048, b0_m/6, β·5²/(2·2048·6), β·4.2²/(2·2048·6))`, after checked
f64→f32 packing. Let `row0,row1` be the **linear XYZ rows** of the packed
world→shadow clip matrix. The WGSL operator must use

`biasDepth = lane1 + (lane2·abs(dot(row0,n)) + lane3·abs(dot(row1,n)))/n_s`.

This reproduces the physical slope formula because the rows scale `r` and
`u` by `2/5` and `2/4.2`. Using `n_r/n_s` directly with these pre-scaled
lanes would double-scale the bias. Naga reflection, row extraction from
column-major storage and an independent f64 oracle must check every lane.
The first lane supplies reciprocal map extent to the four-tap coordinate
operator. The complete bias and matrix path needs source-to-packed error
bounds before the shader division or comparison runs.

## Proof and cost gates

Save exact source/recipe bytes and build one accepted F3 pin with a complete
F4 binding. Check front/back winding, receiver/fence/caster projected bounds,
clear-map lit, opaque-caster shadowed, masked fence agreement across
shadow/depth/forward, a wrong-Sun-sign mutant and camera-frustum-culling
mutant. Compare f64 world and packed f32 projected points across moving
origins; prove cutoff and bias intervals before WGSL operations. Capture
shadow-map depths and final pixels against independent references; inspect
acne, detached contact, staircase edges and camera-motion stability.

The D32 map has 16 MiB logical texels before alignment, residency and old/new
overlap. Measure actual GPU shadow-pass time, map lifetime/reuse and whole
frame cost on each named target. A retained map needs V1 queue/fence and
packet-credit ownership through replacement/device loss. The existing
synthetic four-role GPU proofs and CPU numeric/index probes do not establish
this source-to-shadow result.

[WebGPU depth and samplers](https://gpuweb.github.io/gpuweb/),
[WGSL comparison sampling](https://gpuweb.github.io/gpuweb/wgsl/#texturesamplecomparelevel),
[Microsoft shadow-map projection guidance](https://learn.microsoft.com/en-us/windows/desktop/DxTechArts/cascaded-shadow-maps),
[NVIDIA PCF discussion](https://developer.nvidia.com/gpugems/gpugems/part-ii-lighting-and-shadows/chapter-11-shadow-map-antialiasing).
