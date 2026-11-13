# RENDER-2D planning probe, 13 November 2026

Throwaway calculations the planner ran for [the design](../../plans/RENDER-2D/DESIGN.md): three before writing it,
and a fourth after its design review. They
answer questions the design had to settle and that reading alone could not. None is engine code and none ran on a
GPU, so nothing here is a baseline: the milestones repeat each result through the renderer. The source stayed in
the planner's scratch folder and is not kept.

Machine: Apple M3 Max, macOS 26.6.2, while other agents were building (timings are contended). Python 3 for
probes 1 and 3 and the rule checks of probe 4, `rustc 1.99.1 -O` for probe 2 and the crowd of probe 4.

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

## 4. Does a kept order with a band stop reversals, and what does it cost?

[The design review](design-review.md) showed that the first draft's band, held around each row's own sort value,
lets two rows that travel together swap and swap back. This probe tried the rule that replaced it,
[mechanism 4](../../plans/RENDER-2D/DESIGN.md#4-order): the order is one list kept from tick to tick; two rows next
to each other in it exchange when the one behind has passed the one in front; for two **near** rows (bounding
circles, each grown by the band `b`, touch) "passed" means by more than `b`, and for any other two it means on the
exact key.

**The review's pair.** `b` is 1 and the rows overlap.

| Tick | True A | True B | Band on each row's own value | Kept order |
|---|---:|---:|---|---|
| 0 | 0.0 | 0.1 | A, B | A, B |
| 1 | 2.0 | 2.1 | A, B | A, B |
| 2 | 2.2 | 2.1 | B, A | A, B |
| 3 | 2.1 | 2.3 | A, B | A, B |

The third column reproduces the review's table. Under the kept order nothing exchanges, because A is never more
than 0.1 above B.

**The rule on random motion.** Rows are discs of diameter 1 in a small square, so that most pairs overlap; each
drifts, changes direction at random and jitters, and rows spawn and despawn. Four settings of 3, 8, 30 and 30 rows,
120 seeds each, 300 ticks each: 144,000 ticks. `b` is 1. Checked at every tick, by code separate from the update:

| Claim | Result |
|---|---|
| The order is a permutation of the live rows | Held at every tick |
| A change of relative order between two rows with history happens only by those two exchanging, and for a near pair only when the one moving in front is more than `b` above the other | 480,616 exchanges, 157,593 of them between near rows; none broke it |
| After a tick no two neighbours in the list are waiting to exchange | Held at every tick |
| A row with no history is in exact key order with the rows on either side | Held for every spawn |
| When an overlapping pair is seen in one order, the other, and the first again, one of the two moved more than `b` in between | 8,229 such returns; the smallest move was 1.27 `b` |
| Control: the same motion ordered by a band on each row's own value | 26,544 returns, 2,805 of them with no move over `b` |

With `b` at 0 the list equalled the exact (sort value, identity) order at every tick.

**The limit.** Three overlapping rows kept as A, B, C, with A 0.9 `b` above B and B 0.9 `b` above C, do not
exchange, and A stays behind C from 1.8 `b` above it. The rule bounds neighbours in the list, not every pair.

**A packed crowd.** Discs of diameter 1 in a square sized so that they lie three deep (51 units a side for
10,000). Each follows a path at up to 0.01 units a tick on each axis, changing direction on 1% of ticks, and is
drawn 0.03 units either side of it at random, so a row jitters over a range of 0.06. 600 ticks, the first 30 not
counted. Counts are over pairs that overlap. "All" puts the band between every two rows; "near" is the design's
rule. This crowd is made up, not `SIM-2D`'s, which M5.3 measures.

| Rows | Rule | `b` | Reversals a tick | Reverse and back within 30 ticks, a tick | Pairs over `b` out of order | Over `2b` | Worst | Kept order | Exact sort |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|
| 2,000 | exact | 0 | 220 | 168 | none | none | 0 | 19 µs | 13 µs |
| 2,000 | all | 0.06 | 50 | 1.3 | 0.54% | 0.030% | 3.8 `b` | 2 µs | 13 µs |
| 2,000 | near | 0.06 | 50 | 1.4 | 0.041% | 0.6 in a million | 2.4 `b` | 20 µs | 13 µs |
| 10,000 | exact | 0 | 1,103 | 844 | none | none | 0 | 105 µs | 77 µs |
| 10,000 | all | 0.06 | 248 | 5.8 | 1.2% | 0.18% | 4.6 `b` | 8 µs | 76 µs |
| 10,000 | near | 0.06 | 253 | 7.2 | 0.041% | 1 in a million | 2.4 `b` | 121 µs | 76 µs |
| 10,000 | all | 0.12 | 207 | 1.2 | 5.8% | 2.1% | 7.3 `b` | 8 µs | 75 µs |
| 10,000 | near | 0.12 | 232 | 2.1 | 0.30% | 0.004% | 3.4 `b` | 105 µs | 76 µs |

With paths five times faster (0.05 units a tick), 10,000 rows and `b` at 0.06: reverse and back fell from 428 a
tick to 61 under the near rule, 0.14% of pairs were over `b` out of order, 0.002% over `2b`, the worst 3.3 `b`,
and the kept order took 139 µs.

**Reading.**

- In the slow crowd a band the size of the jitter removes more than 99% of the reversals that undo themselves,
  and in the fast one 86%. Those left are rows whose paths really cross and cross back.
- The band between every two rows leaves far more overlapping pairs out of order, and the share doubles from
  2,000 to 10,000 rows: every row at nearly the same height joins the chain, wherever it is. With the near test
  the share does not grow with the crowd. That is why the design's rule has it.
- The kept order costs about one and a half exact sorts here. Its worst case is a tick in which every row with
  history takes a new random value: 25 million exchanges and 10.5 ms for 10,000 rows.
- Nothing here ran in the engine or in wasm. M2.13 proves the rule in the renderer and M5.3 measures the real
  crowd.
