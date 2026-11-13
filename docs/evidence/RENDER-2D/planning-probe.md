# RENDER-2D planning probe, 13 November 2026

Three throwaway calculations the planner ran before writing [the design](../../plans/RENDER-2D/DESIGN.md). They
answer questions the design had to settle and that reading alone could not. None is engine code and none ran on a
GPU, so nothing here is a baseline: the milestones repeat each result through the renderer. The source stayed in
the planner's scratch folder and is not kept.

Machine: Apple M3 Max, macOS 26.6.2, while other agents were building (timings are contended). Python 3 for
probes 1 and 3, `rustc 1.99.1 -O` for probe 2.

## 1. Does coverage sampling keep texels the same size while the view scrolls?

**Method.** One row of 64 random texels is drawn at a scale of `s` output pixels per texel, at 200 sub-pixel
scroll offsets. Each output pixel is computed four ways: nearest; plain bilinear; **coverage sampling** (move the
sample to the texel centre, except within half an output pixel of a texel edge, where it ramps across the edge:
`seam = floor(t + 0.5); t' = seam + clamp((t - seam) / w, -0.5, 0.5)`, bilinear at `t'`, with `w` the texels one
output pixel spans); and the exact box filter, which is the area of each texel under the pixel. The table gives
the largest difference from the box filter over every pixel and offset, and how many output pixels one texel
covers under nearest sampling.

| Scale `s` | Coverage against box | Bilinear against box | Nearest against box | Texel width under nearest |
|---|---|---|---|---|
| 1.00 | 9e-15 | 0.000 | 0.431 | 1 |
| 1.25 | 9e-15 | 0.086 | 0.431 | 1 to 2 |
| 1.50 | 2e-14 | 0.144 | 0.431 | 1 to 2 |
| 2.00 | 9e-15 | 0.216 | 0.431 | 2 |
| 2.37 | 1e-14 | 0.249 | 0.431 | 2 to 3 |
| 3.00 | 2e-14 | 0.287 | 0.431 | 3 |
| 4.00 | 8e-15 | 0.349 | 0.465 | 4 |
| 6.00 | 2e-14 | 0.387 | 0.465 | 6 |
| 0.75 | 0.048 | 0.060 | 0.191 | 1 |
| 0.50 | 9e-15 | 0.000 | 0.000 | 1 |

**Reading.**

- For every scale of 1 or more, coverage sampling **is** the box filter, to rounding error. Each texel therefore
  puts the same total weight on screen at every scroll offset: no texel grows or shrinks as the view moves.
- Nearest sampling makes a texel 2 or 3 pixels wide by turns at a scale of 2.37, and 1 or 2 at 1.5. That change
  of width from frame to frame is the shimmer.
- Bilinear is stable and wrong: it blurs every texel edge across a whole texel.
- At a whole-number scale with pixel centres on the texel grid, coverage sampling returns the texel itself (equal
  to nearest within 1e-15), so an aligned sprite is exact.
- Below a scale of 1 it is no longer exact (0.048 at 0.75): one output pixel then spans more than two texels. That
  range belongs to mip levels.

This is arithmetic in double precision. A GPU's sampler blends with a fixed number of weight bits, so the
renderer's difference from the box filter will be larger than these figures; the design has a milestone measure
and retain it.

## 2. What does ordering thousands of rows cost on the CPU?

**Method.** A 64-bit key for each row (8 bits of layer, 32 bits of an order-preserving `f32`, 24 bits of slot),
rebuilt and sorted with `sort_unstable` after every row moved a little. Best of 194 rounds, one thread.

| Rows | Build keys and sort | Order list at 4 bytes a row |
|---|---|---|
| 2,000 | 12 µs | 7 KiB |
| 5,000 | 32 µs | 19 KiB |
| 10,000 | 71 µs | 39 KiB |
| 50,000 | 423 µs | 195 KiB |

**Reading.** Ordering 10,000 rows once a tick is under a tenth of a millisecond natively, so the CPU can own the
order and no GPU sort or depth buffer is needed for it. Wasm in Chrome was not measured.

## 3. Do authored 8-bit colours survive the half-float scene target?

**Method.** For each of the 256 sRGB codes: decode to linear, store as a 16-bit float, encode and round.

**Result.** All 256 codes return to themselves, and the worst lands 0.04 of a code from its centre. So an opaque
texel can leave the frame as the code it was authored with, provided the output adds no tone curve and no dither,
and the GPU's sRGB decode and the display shader's encode are accurate to within the remaining 0.46 of a code.
That last condition is not established here; milestone M1 tests it on each target.

The same calculation shows what blending in linear light does to soft art: white at half alpha over black leaves
the frame as code 188, where a paint tool that blends in display space shows 128.
