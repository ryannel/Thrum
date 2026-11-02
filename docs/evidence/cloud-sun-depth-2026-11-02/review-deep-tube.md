# Independent challenge: monotone Sun-depth tube compression

2026-11-02. Read current `CLOUDS.md` shared-light, all-layer/footprint/error/work clauses and the independent `deep_light_reference.py` / `test_deep_light_reference.py` minimum-removal reference and negative gradual witness. Reviewed the root's proposed replacement independently of a replacement implementation. No GPU, renderer/canonical changes or retained candidate implementation authored here. Inline Python calculations below are an independent small experiment, not a numerical-backend certificate.

## Mathematical result

I found no invalidating real-arithmetic counterexample to the proposed algorithm under its stated finite, strictly ordered-depth, monotone nonnegative piecewise-linear-tau input contract. The endpoint-selection order is essential.

Let anchor be `(xa,ta)`, and let d be the minimum of the proposed d_a over every active positive exponent a. For each earlier input knot i, feasible slope interval is `[(ti-ta-d)/(xi-xa),(ti-ta+d)/(xi-xa)]`, intersected with nonnegative slopes. Test a prospective **actual endpoint** slope against the intersection accumulated before that endpoint. If it passes, add that endpoint's interval and remember it as the last accepted actual endpoint. The candidate slope is automatically inside its own new interval. Consequently the intersection cannot become empty on that accepted iteration.

When the next endpoint fails, the segment to the remembered endpoint was already certified against all preceding knots. Emit that remembered actual endpoint, establish it as the new anchor and reprocess only the failed current endpoint. With the reset unrestricted nonnegative slope range, that first new endpoint necessarily passes in exact arithmetic. Every input endpoint is therefore processed once or twice, and the final remembered endpoint must be emitted at stream completion. Every output knot is an original cumulative sample, so sorted depth and monotone tau are preserved. A one-endpoint stream cannot establish a nonzero curve domain and should remain invalid.

On an emitted segment, original and replacement tau are affine between each pair of original input knots. Their difference is affine there, so the endpoint constraints imply `|old-new|<=d` throughout. Both original and replacement tau are at least the anchor tau: original by monotonicity, replacement by its nonnegative slope. For any active a, write `m=min(old,new)`, `h=|old-new|`; then

`|exp(-a*old)-exp(-a*new)| = exp(-a*m)*(1-exp(-a*h)) <= exp(-a*ta)*(1-exp(-a*d)) <= epsilon`.

If `exp(-a*ta)<=epsilon`, both transformed terms already lie in `[0,epsilon]`, so that transform imposes no finite d constraint. Otherwise solving the last inequality gives exactly the proposed `d_a=-log(1-epsilon*exp(a*ta))/a`. Taking the minimum covers both ordinary visibility and the kappa=.25 bank. Output segments have disjoint interiors and agree at shared actual endpoints, so the maximum compression error is epsilon globally; no triangle sum across emitted segments is needed. This is a different invariant from repeated modification of an already approximated curve. The old removal sum and its per-removal allowance must not be inherited as if they proved this new invariant.

This also covers arbitrary partial depth queries inside an admitted curve, not only queries at input/output knots. It does not establish accuracy of the input tau curve itself.

## Independent finite checks

I separately implemented the stated endpoint/intersection rule in a disposable inline Python invocation, using `log1p`, the existing explicitly float32-rounded named inputs, and dense direct piecewise-linear input comparisons. This code was not read from a replacement compressor or written to a candidate source file.

| Input (257 endpoints) | Output knots | Endpoint processings | Sampled max primary / reduced error |
| --- | ---: | ---: | --- |
| clear |2|256|0 / 0|
| slab |4|258|2.28e-8 / 2.38e-8|
| separated thin layers |6|260|0 / 0|
| dense tower |4|258|1.11e-16 / 1.11e-16|
| gradual `12*(x/128)^2` |14|268|0.0045463162 / 0.0043142140|
| opaque |9|263|0.0001267850 / 0.0026063986|
| grazing |4|258|1.40e-15 / 5.55e-10|

The separate initial-GPU-shaped513-endpoint float32 gradual stream also needs14 knots (524 endpoint processings), with sampled primary/reduced maxima0.0048200180 /0.0043553433. The real-arithmetic argument supplies the continuous bound; dense samples are only a sanity check. Also checked200 deterministic random monotone streams of251 points with nonuniform depths/plateaus and varied increments. Sampled maxima were0.0049583250 /0.0049991522; largest output was46 knots; every stream obeyed at most two processings per non-anchor endpoint.

A concrete capacity rejection witness remains: start `(0,0)` and for i=0..99 supply `(i+.49,i*.03)` then `(i+.50,(i+1)*.03)`. The same rule needs120 output knots, with sampled errors0.0049845035 /0.0047523654. Thus K48 is not a general quality admission even for finite monotone thin/clear structure. It must visibly invalidate; it must never collapse/drop the remaining layers to keep a valid bit. This witness is a useful CPU negative test, not a claim that the engine must admit arbitrary100-layer content without a larger/direct path.

## Required implementation/integration conditions

1. **Floating arithmetic remains separately unqualified.** The exact proof assumes reliable slope containment and d. Compute the saturation test before `epsilon*exp(a*ta)` to avoid overflow; for nonsaturated transforms use `-log1p(-epsilon*exp(a*ta))/a`, not cancellation-prone `log(1-q)`. At/near the saturation threshold, use a conservative branch/equality rule. NaN/overflow, unresolved depth spans, duplicate rounded depths, or loss of ordering must invalidate. False admission from rounded slope bounds is possible if an implementation simply uses epsilon=.005 with no numerical allowance. Reserve a stated working margin or independently bound/check the actual arithmetic/storage error within the existing total budget. The old2e-6/removal heuristic is not a backend proof for the tube. No uniform GPU certificate is required before a labelled experiment, but do not report a proven .005 hardware maximum from this real-arithmetic review.

2. **Persist exact endpoint/overflow/domain semantics.** Retain the initial endpoint and final accepted endpoint, distinguish pending last endpoint from emitted knots, and count the final store against capacity. Do not emit the currently failing endpoint as the end of the old segment. A K48 or endpoint-cap failure invalidates the entire column; any diagnostic partial curve is unavailable to consumers. Replaying one current sample is local compression work, not permission to requery the source with a new time/generation. Direct/refined fallback can reintegrate the same immutable all-source generation. Constant extension beyond the domain requires a conservative proof of absent additional upstream support; it cannot follow from endpoint saturation or opacity alone.

3. **Preserve all-layer and footprint contracts.** The stream must still be the cumulative integral of every intersecting virtual source/layer along that Sun ray, with one shared source/light generation. Do not run the proof separately on owners and then assume their transformed error allocations add for free; perform it on their summed cumulative tau or provide the appropriate new accounting. All active transforms must be included; a future branch with a smaller exponent changes the constraint set. Public point/segment ordinary tau remains a point quantity. Filter branch transmittances/closure terms separately across columns, invalidate a footprint when any required tap/domain/generation is invalid, and never reuse `-log(mean primary)` for the reduced branch. The tube does not repair lateral undersampling/clear-cloud mixtures, missing offscreen blockers, low-Sun projection bounds, source quadrature, stale light, finite-light paths or cascade double attenuation.

4. **Update work and memory together.** Compression is O(N) with at most2(N-1) endpoint attempts, O(1) slope/delta state in addition to retained output/pending points, and at most one new anchor per failure. There are two transform delta evaluations per anchor; exp/log1p work scales with emitted segments, not K removals per input endpoint. Integration/index/field/primitive work and fallback remain separate, and none of these counts establish a frame-time benefit. If all344064 base columns reserve48 nodes, depth32+tau32 nodes require126MiB, plus5.25MiB of16-byte metadata:131.25MiB before private state, alignment, old/new generations and other light products. The current24MiB node example applies to8/12/16 knots and cannot also describe uniform K48. A65536-column48-knot refinement pool is24MiB nodes, versus the current16-knot8MiB example. Variable smaller capacities are legitimate only with their own measured failure/refinement rates. Keeping K48 private state does not erase global output allocation.

The existing source/field quadrature .005 + compression .005 + lateral footprint .008 + storage .002 total remains .02 by triangle accounting, provided each component actually meets its allocation for both branches. Weighted source/phase/radiance and native appearance are additional gates. The tube's nonaccumulating compression bound neither relaxes nor proves those other allocations. Ordinary surface direct visibility continues to use exp(-tau), while the selected bank alone uses reduced depth; this change is private light representation, not a new optical source model.

## Disposition

The proposed tube is mathematically sound for the declared input and is a credible replacement for the rejected local-minimum-removal stream. The gradual witness specifically supports the change: the replacement needs14 knots in my independent finite calculation, while the existing K48 removal stream is an explicitly retained failure. Closure here is **design-level compression proof**, conditional on the endpoint rules, visible overflow and numerical scope above. Replacement CPU implementation, endpoint/storage arithmetic, GPU work/private spills, real all-source columns, lateral accuracy, fallback rates and integrated rendering remain separate unrun qualifications. No claim that K48 generally suffices or that total query/radiance quality has passed follows from this review.
