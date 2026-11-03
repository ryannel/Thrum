# First R0 Sun-shadow numeric probe

3 November 2026. This is bounded **CPU evidence** for the candidate
[first Sun-shadow recipe](../../design/research/R0-FIRST-SUN-SHADOW-RECIPE-2026-11-03.md).
It is not an admitted F3 source, an F4 binding, a selected F2 recipe, or a
target-device proof. The [reproducible archive](R0-FIRST-SHADOW-NUMERIC-PROBE-2026-11-03.tar.gz)
contains the exact fixture bytes, generator, probe, verifier, and result JSON.
The 6,641-byte archive has SHA-256
`71e77c090a5c1b9967cea400878de0f34c8aa30e958bcebf9df449cc3363244a`.

## Fixture and oracle boundary

The 696-byte `fixture_geometry.json` has SHA-256
`80180064ecc1d6261b70da9071162e868a29e7e196d6daac4b059b9aff8b60f6`.
`generate.py` is its byte generator: sorted, compact ASCII JSON with one
terminal newline. The three +Z quads are each indexed `(0,1,2),(0,2,3)`:
receiver `[-3,3]×[-2,2]@z=0`, checker-masked fence
`[-0.6,0.6]×[-1,1]@z=1`, and offscreen caster
`[3.4,3.8]×[-0.4,0.4]@z=2`. The synthetic 4×4 alternating 0/1 alpha
pattern and 0.5 cutoff exist only in this CPU fixture. The candidate camera,
Sun, 2048² map, and `ξ/η/ζ` light box are exact fields in the retained JSON.

The projection oracle uses rational arithmetic over **decoded binary64**
fixture values and the explicitly calculated binary64 Sun/basis values. It
is exact for those inputs, rather than for the printed decimal ideals. A
separate ordered f64 matrix path checks the formula. The packed path rounds
each world-to-shadow row, origin-relative identity `ObjectRecord` position,
and each multiply/add of a non-fused row dot product to f32. Its eight view
origins are `(0,0,0)`, `(0,0,5)`, `(±500,0,0)`, `(0,0,±500)`,
`(300,300,0)`, and `(0.125,-0.25,4.875)` metres. Each origin tests 12
source vertices and a 41×41 receiver grid: 13,544 projections total.

| Finite-set observation | Maximum absolute difference | Where |
| --- | ---: | --- |
| Packed clip X versus rational | `1.3671875e-5` clip units, `0.0140` map texel | origin `(500,0,0)`, receiver `(2.7,-2,0)` |
| Packed clip Y versus rational | `1.67120071e-5` clip units, `0.0171131` map texel | origin `(300,300,0)`, receiver `(-3,-0.8,0)` |
| Packed depth versus rational | `9.56217448e-6` depth units, `57.373 µm` in the 6 m ζ span | origin `(0,0,500)`, fence vertex `(0.6,-1,1)` |
| Ordered f64 depth versus rational | `1.11e-16` depth units | selected point |
| Direct rational-to-D32 cast versus rational | `2.70208e-8` depth units, `0.162 µm` | selected point |

Twice the **observed** maximum packed depth error gives `0.000114746 m`
(`0.115 mm`) as a selected-point two-sided producer/consumer study value.
It is not a certified upper bound: the sampled producer and receiver maxima
need not occur at the same location, and this probe excludes GPU vertex
execution, raster interpolation/coverage, D32 storage of actual raster
fragments, derivative behavior, and implementation-specific f32
reassociation/FMA. It cannot justify `b0_m` or a retained-map error
certificate. The first recipe still needs a measured contact detachment
limit to bound bias from above.

Changing the view origin changes the packed consumer matrix and its error
certificate. It does not, by itself, change static world-space geometry or
the Sun. Reusing a stored map would additionally need its old-origin
**producer** raster-depth certificate and a new-origin consumer certificate;
this study supplies neither device-side certificate.

## Four-tap CPU checks and mutants

An analytic light ray intersects each quad at the queried texel center;
the largest rational reverse-Z depth is stored. Four nearest texels use
`i=floor(uv·2048−0.5)` and centers `i+(0,0),(1,0),(0,1),(1,1)`.
The CPU comparator evaluates `reference >= stored`, with
`reference=d_receiver+biasDepth`. The table lists lit fraction, not a
device-filtered PCF sample:

| Receiver query | β=0, b0=0 | β=1, b0=0 | Perturbation |
| --- | ---: | ---: | --- |
| Caster shadow `(2.1,0,0)` | 0 | 0 | Camera-culling caster: **1**; wrong `less-equal`: **1** |
| Clear receiver `(-2,0,0)` | 0.5 | 1 | Zero slope allowance produces self-shadow in this point model. |
| Fence region `(-0.75,0.5,0)` | 0.25 | 0.5 | Alternating alpha yields partial four-tap coverage. |

For this +Z receiver, β=1 contributes `0.0018310546875 m` of physical
slope allowance. That is a **study point**, not an approved β or b0. The
caster is outside the camera's x half-width `3.0792014 m` at z=2, yet
`−Sun` moves its receiver shadow to x≈`[1.9,2.3]`. The wrong-sign geometric
mutation moves it to x≈`[4.9,5.3]`, outside the receiver. The separate
verifier checks this projected interval; the wrong-sign case is not a third
four-tap visibility run.

## What remains before admission

The CPU study specifies a reproducible arithmetic schedule and catches
three semantic mistakes. It does **not** prove GPU winding/culling, masked
depth/forward agreement, edge coverage, the actual four
`textureSampleCompareLevel` results, bias adequacy, retained-map reuse,
camera-motion stability, or contact/edge quality. A full fixture must
admit source/texture bytes, pin the F4 Sun evaluator and packed ABI, use a
WGSL implementation on named targets, capture map depths and pixels against
independent references, and measure GPU pass/residency/frame cost.

The diagnostic Python scan is only 13,544 projection points and 32 analytic
texel-ray evaluations for these selected cases, so its runtime is not an
engine performance estimate. The proposed map alone is 16 MiB of logical
D32 texels; alignment, residency overlap and whole-frame cost require
target measurements. The matrix is small per-view work; the important
performance decisions remain the map schedule/reuse and per-pixel four
comparison samples.

Reproduce by extracting the archive and running `python3 generate.py`,
`python3 probe.py`, then `python3 verify.py` in the extracted directory.
