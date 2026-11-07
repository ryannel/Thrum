# R0 exact represented-dot optimization diagnostic

3 November 2026. This isolates representation and work reduction for the
[raw-`u32` exact-dot Metal diagnostic](R0-EXACT-REPRESENTED-DOT-METAL-2026-11-03.md).
The candidate WGSL uses one signed ten-`u32` accumulator instead of four
ten-word arrays, adds/subtracts each shifted product starting at its first
occupied limb, and extracts the normalized significand with a bounded limb
scan instead of a bit-by-bit scan. Input domain and the complete 17-word
diagnostic output retain their previous meanings. The
[reviewable source/results archive](../ARCHIVE-ACCESS.md#archive-inventory)
has SHA-256
`1e663b68cbb0f812e7a6de4a8458edc1cfb807efc2179ff44cccc8179f83c28a`.

The unchanged independent Fraction oracle matched all 17 words for the
189,803-record corpus, including 66 invalid inputs, `2^-298` cancellation,
long carries/borrows and normalization ties. Three final paired full-output
processes matched every dispatch; 138 dispatches account for 445,277,838
word comparisons including warmups. Two additional 8,192-record base sets
were independently oracled for normal and deliberately ambiguous inputs.
The archive retains 131 verified members, exact source/host/log hashes,
per-pass timing samples and nine selected complete GPU readbacks. The parent
reran its portable verifier and all nine readback comparisons. The much larger
scratch build and repeated readbacks are excluded from repository evidence.

| Apple M3 Max Metal, public wgpu/Naga 30.0.1 | Original | Signed ten-word candidate |
| --- | ---: | ---: |
| 189,803 records, full 17-word output; three process median range | 1.039–1.057 ms | 0.204–0.234 ms |
| 1,048,576 records, normal compact output; three process median range | 14.655–16.029 ms | 0.731–0.813 ms |
| 1,048,576 records, all-ambiguous compact output; three process median range | 11.842–12.240 ms | 1.193–1.277 ms |
| 2,073,600 records × five dispatches, normal compact output; one paired block | 177.499 ms | 20.508 ms |
| 2,073,600 records × five dispatches, all-ambiguous compact output; one paired block | 143.821 ms | 33.928 ms |

The normal workload consists of normalized independent uniform-cube samples;
its angular distribution is **not** uniform over the sphere. The ambiguous
workload includes planar cancellations and exact zeros. Both execute the
exact path for every record, so these timings do not measure a filter or
real fallback frequency. Each compact variant still computes the exact
magnitude but writes seven rather than 17 diagnostic words. The five-dispatch
control equates a dot count of 10,368,000 to one dot per 1080p pixel/light;
it has no rasterization, BRDF, textures, half-vector, shadows or frame.

Paired A/B/A controls favor the signed candidate, but same-shader A/A
controls and scale/repetition experiments show material device/process
variation. The direct five-dispatch numbers cannot be converted into a
stable fragment or whole-frame budget, and the improvement cannot be assigned
solely to register reduction without compiler/hardware counters. The
observed 20–34 ms all-pixel control is enough to reject an unmeasured
assumption that exact integer evaluation is cheap when common. The signed
kernel is selected only as the next **exact represented-input research
candidate**. Source-real error, float-bit recovery, a magnitude-accuracy
fast filter, mixed-wave occupancy, ratio/half-vector/BRDF closure, actual
shader binding, moving-image quality and device cost remain open.
