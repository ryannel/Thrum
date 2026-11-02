# Original-endpoint interval index: bounded mathematical review

2 November2026. **Recommend the exact index accelerator for a reviewed implementation experiment**, preserving the existing K48 operator and its independent point/lateral failures. No candidate implementation, shader or GPU job was made here. Root owns canonical integration and later correctness/device qualification.

## Contract and table meaning

The selected builder integrates512 uniform intervals over one committed light-column span L. Its original endpoints are `d_i=f32(i*h)`, `h=f32(L/512)`, i=0…512. Current compressed anchors are an ordered subset of these original endpoints, retaining their exact depth/tau values. For each column and raw interval k=0…511, store the compressed segment j whose half-open span contains `[d_k,d_(k+1))`. With48 anchors there are at most47 segments; valid j is0…46 and fits u8. Four entries per u32 yield128words/column. Invalid columns must never acquire clear semantics from an uninitialized table byte.

Build by a monotone walk across actual raw endpoints and retained anchors. Do not recover anchor indices by flooring an approximate depth/h inverse: the very boundary alias the table addresses can affect that operation. Source input grid and table/node generations must agree bitwise. The query must use the SAME producing f32 span/grid; a host recomputation from support dimensions is not an interchangeable authority. Future adaptive/nonuniform endpoints do not meet this accelerator's admission.

For a valid nonterminal query, estimate raw interval `k=clamp(floor(f32(depth/L*512)),0,511)`, load j, then compare with actual node depths. Correct j by at most one segment when below its lower anchor or at/above its upper anchor. Match the old binary search's exact tie policy; the reviewed half-open convention is `lo<=depth<hi`, with the final endpoint handled by the existing terminal branch. Equality at hi advances. Keep existing tau interpolation/exp arithmetic order if bitwise query identity is wanted; choosing a different segment at equality can otherwise change rounding in a mix expression.

**Always validate the final actual-node bracket.** If it fails, use the already bounded old search or report unavailable/invalid with a counter, never interpolate an unverified segment. Existing invalid-generation/column, before-source, after-source and source-off branches remain. Two-node rows should bypass the index: their old search already has zero interior iterations. Unused rows may remain physically stale only if that branch guarantees they are never read; a newly active row must be populated before consumers see its generation.

## IEEE argument and limits

For the selected normal finite grid, division by512 is a binary exponent scaling. With correctly rounded binary32 arithmetic, `|fl(i*h)-i*h|/h` is at most about512u and the interval-coordinate division/multiply error is also about512u, u=2^-24. Their combined uncertainty is far below one raw interval. A classification ambiguity therefore crosses at most one original boundary. Strictly increasing compressed anchors are a subsequence of those boundaries, so compressed segment distance is also at most one. Queries whose ratio rounds to512 just below L are clamped to511; true terminal queries are handled before index lookup.

This is a conditional arithmetic argument, not a claim that every WGSL compiler implements one correctly rounded division or preserves the same expression tree. It excludes underflowed/duplicate grids, nonfinite domains, mismatched generations, different endpoint construction, stale/corrupt indices and future adaptive sampling. The final bracket guard is what keeps those cases from silently changing the operator. Query world-projection roundoff is not repaired by this index; it remains the same as the old lookup.

## Actual captured-data challenge

[Bounded results](endpoint-index-comparison.json), SHA256 `15f3d0227c45f0481fd703288de813ebb043a8e706dd16c97dbcfa22072f468d`. Inputs are the same actual full/cropped capture receipts and SHA-pinned node/metadata/path binaries documented in the earlier validation notes. No regenerated source samples or candidate lookup implementation was imported.

For both captures, all16 path streams have bit-identical depth grids. Their513 depths reconstruct bitwise as `f32(i*f32(L/512))`. Every active stored anchor across all4356 columns bit-matches a member of that actual original grid. Queries cover all raw endpoints, both float32 nextafter neighbours, raw midpoints and supplied point-query depths, excluding existing terminal branches. NumPy float32 emulates divide then power-of-two multiply; comparison/correction uses actual node depths.

| Witness | Full128 | Cropped128 |
| --- | ---: | ---: |
|L, metres|854.967956543|416.917419434|
|h, metres|1.669859290|.814291835|
|Admitted columns|4356|4356|
|Retained anchors checked|11,481|19,367|
|Two-node rows|4106|3370|
|Rows needing interior search|250|986|
|Distinct nonterminal tested depths|2063|2064|
|Column/depth bracket cases|8,986,428|8,990,784|
|Max raw-interval /compressed-segment proxy distance|1 /1|1 /1|
|One-step corrections|710|1121|
|Final bracket failures|0|0|

These~17.98M finite bracket checks support the selected accelerator. They do not replace actual backend/point-query tests or establish every floating input in a uniform domain. All u8 segment classes fit0…46. The earlier sharp point-T/lateral error remains exactly the same approximation: this proposal changes how the same compressed segment is found, not the curve or four-column filter.

## Counted work and recommendation

Full table reservation is `4356*128*4=2,230,272bytes`, added to1,812,096bytes nodes/metadata for4,042,368bytes total before lifetime overlap/staging. Full build scans at most512raw intervals plus46anchor advances per column (~2.43M simple visits), emits557,568u32 words, performs no new source coefficient query and requires no new exp/log fitting.

If two-node rows bypass both table build/read, the cropped capture needs986×512=504,832 interval visits plus10,655 anchor advances, writing126,208u32 words (504,832bytes). Allocation may stay the simple fixed-size2.23MB table. The full capture would need128,000 interval visits plus2769 advances. This bypass is material because most rows currently have only two anchors; it must not be described as omitting blockers.

For rows that actually search, a query replaces up to~5–6 dependent node probes with one packed-index word and two bracketing node reads, plus one extra node on a correction. Conceptual search-related traffic is about20bytes per corner versus up to~64bytes before, excluding unchanged metadata, terminal paths, cache blocks and compiler reuse. Four-corner lookup and its up to eight exponentials remain. Most clear/two-node queries have no search to accelerate. Whole-view distribution, GPU storage latency, compile code and memory/cache residency determine the actual win; none is measured here.

Proceed with this exact bounded-index experiment rather than the failed dense T lattice. Require generation/shape/index admission, exact-node tie/bracket tests and real GPU equivalence before measuring main/reflection/source-update graphs. Retain direct fallback/visible failure for an impossible correction. Do not claim5ms, filtered-coverage acceptance, point-shadow accuracy or a new cloud-family pass from this mathematical result.
