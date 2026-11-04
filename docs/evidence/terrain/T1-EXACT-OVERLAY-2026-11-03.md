# First terrain exact-overlay falsifiers — 3 November 2026

This bounded offline proof exercises two proposed checks in the
[terrain implementor recipe](../../design/research/TERRAIN-IMPLEMENTOR-DOGFOOD-2026-11-02.md):
error against the **final triangulation**, and wet area/volume against a
source bank. It does not cook or activate a terrain product. The selected
[four-member archive](../ARCHIVE-ACCESS.md#archive-inventory) is 3,240 bytes,
SHA-256 `b29f31928450c050eb395b8cb40301be5271372978ccaa32107d4db66f05aba8`.
Its manifest and members were byte-verified after packaging; `proof.py` uses
Python standard-library `Fraction` only. Run `python3 proof.py` from the
unpacked directory.

Two candidate triangles use the opposite diagonal from the two reference
triangles but retain identical authored corner heights. All corner errors
are zero. Rational polygon overlay and affine barycentric interpolation find
an exact **0.125 m** error at the centre. The 0.125 m height is exactly
representable in f32; output rounding cannot explain this difference. A
missing triangle and a duplicated triangle both fail the support/partition
check. The prototype checks pairwise candidate overlap and exact covered
area; its quadratic pairwise step is confined to this small diagnostic, not
a proposed whole-region algorithm.

The separate planar-bank source is 512 exact triangles over a 64 m by 1 m
strip, at 0.25 m X spacing with the selected SW–NE diagonal and height
`x/20`. Exact clipping at five stages and affine-depth triangle integration,
scaled by the remaining 64 m of identical Z support, equal the independent
closed forms `wet area = 64(20H+32)` m² and
`stored volume = 640(H+1.6)²` m³:

| Stage H (m) | Area (m²) | Volume (m³) |
| ---: | ---: | ---: |
| −1.6 | 0 | 0 |
| −1 | 768 | 230.4 |
| 0 | 2048 | 1638.4 |
| 0.5 | 2688 | 2822.4 |
| 1.6 | 4096 | 6553.6 |

The full 257²-sample region, a protected shelf/sill component graph,
roof/deck receiver ordering, final f32 quantization and conversion bounds,
seams, F3/F4 bindings, P3/P5 consumers, runtime visual quality and device
performance remain unproved. This proof establishes two useful falsifiers,
not a terrain-cooker certificate or physical shoreline implementation.
