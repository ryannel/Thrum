# Bounded cloud Sun-depth profile

2 November2026. Selected bounded correction to the [complete cloud design](CLOUDS.md) after independent scalar challenge;
[reference and review](../../evidence/cloud-sun-depth-2026-11-02/README.md) precede GPU integration. The current lighting proof stays paused. This profile changes a derived Sun product, not source
geometry, lifecycle, extinction, weather authority or the liquid fit. Finite GPU compression and coupled native witnesses now exist; lateral point-T and5ms GPU admission fail. Full-profile/device qualification remains open.

## Decision and discriminating evidence

The first selected streaming minimum-local-error removal scheme has a concrete smooth-field failure. For257 exact-f32
endpoints of `tau=12*(x/128)^2`, K48 gives actual reduced-depth transmittance error about.014582 and accumulated bound
.033064, beyond the.005 compression allocation. It correctly rejects, but its repeated local removals and cumulative
error lose a simple gradual curve. Caching unchanged candidate errors removes redundant fitting work without fixing
that quality failure. Do not integrate that untested GPU draft as a passed foundation.

Select a one-pass **bounded tau tube** for the next experiment, after the independent real-arithmetic challenge. It chooses
original integration endpoints as anchors, never changes coefficients and guarantees the two transformed terms against
the piecewise-linear input tau curve in real arithmetic. The proposed K48 capacity remains a finite admission limit;
it is not a claim every cloud ray fits. A many-thin-step witness exceeds it and must request bounded direct/refined work.
This is our analytical specialization, not an algorithm attributed to the AVSM paper.

## Real-arithmetic derivation

Input depths x_i strictly increase, tau_i are finite, nonnegative and nondecreasing. Input defines piecewise-linear
ordinary cumulative tau, with the first light-facing endpoint tau=0. Let a range over1 and kappa=.25. For a new
anchor `(x_a,tau_a)`, select an internal transformed tolerance epsilon below the.005 allocation. Define:

```
T_a = exp(-a*tau_a)
delta_a = infinity                         if T_a <= epsilon
          -log(1-epsilon/T_a)/a            otherwise
delta = min(delta_1, delta_kappa)
```

If the old and approximated tau are both at least tau_a and differ by at most delta, then

`|exp(-a*tau_old)-exp(-a*tau_line)| <= T_a*(1-exp(-a*delta)) <= epsilon`.

For each encountered original knot, its allowable nonnegative line slope through the fixed anchor is

`[(tau_i-delta-tau_a)/(x_i-x_a), (tau_i+delta-tau_a)/(x_i-x_a)] intersect [0,infinity)`.

Maintain the intersection of these intervals. The actual slope to the next candidate endpoint is tested against the
**previous** intersection before adding the candidate's interval. Its own interval contains its actual slope. If accepted,
retain it as the last accepted input endpoint and intersect its interval. The candidate line therefore fits every
intervening original knot. Tau difference is linear between each pair of original knots, so its absolute maximum there
occurs at an endpoint and is at most delta. Both tau curves stay above the anchor by monotonicity. This extends the
transformed bound over every intervening depth, not only the sampled query locations.

At the first failed candidate, emit the immediately preceding accepted input endpoint as the new anchor. Its line was
already admitted against all prior knots. Reset delta/slope limits there and reprocess the current candidate. Finish by
emitting the final accepted endpoint. The resulting pieces have disjoint interiors, so the maximum transformed error
is bounded by epsilon; errors are not accumulated across successive pieces. This is different from repeatedly modifying
the same overlapping curve in the old removal scheme.

An opaque anchor whose two terms are already below epsilon needs no finite tube constraint: any nondecreasing continuation
stays below that absolute allowance. Still retain a final endpoint for an admitted complete curve. Zero/flat tau is valid.
Invalid order, decreasing/negative/nonfinite tau, duplicate depths or capacity overflow is an invalid result, not a partial
curve that a query may read. K48 overflow cannot silently replace an earlier accepted anchor or drop a late blocker.

## Implementation, numerical meaning and work

Each input endpoint is visited at most twice: once under the preceding anchor and, after a failure, once under its new
anchor. Working state is the current anchor, immediately previous accepted endpoint, slope interval, delta/opaque state
and counters. Completed anchors are written once to the bounded output. There is no K-way minimum scan or per-node
error array. Delta evaluation needs exponent/log work only at new anchors, at most K+1 times before admission fails.
The accepted curve stores at most K original endpoints; fixed input endpoint generation remains separately counted.

Do not calculate `epsilon*exp(a*tau_a)`, which can overflow: first compute T_a, branch when T_a<=epsilon, then evaluate
`-log1p(-epsilon/T_a)/a`. A WGSL stable finite equivalent, equality/opaque policy and conservative rounding allowance
must match its qualified input range. First numerical candidate reserves2e-6 once from the.005 transformed query
allocation (epsilon=.004998). It is not the old per-removal accumulation and is not a proved universal GPU error bound.
Report mathematical bound, proposed numerical reserve and measured CPU/GPU error separately. Float32 interpolation,
near-equal depths, zero slopes and opaque limits need actual backend witnesses. No certificate follows from the reserve's name.

The compression reference is the already integrated piecewise-linear tau input. Source/field integration accuracy is
independent. Full planned absolute query allocation remains.02 for both terms: source/field quadrature.005,
compression.005, lateral footprint.008 and storage.002. Passing this scalar compression does not establish the other
shares or a source/radiance error. An exhausted share invalidates the requested quality.

## First finite GPU product and query

Keep the current source/Sun frame and192-byte scene ABI. First local profile uses64 nominal perpendicular cells with
66 physical columns per axis (one guard centre on either side), K48,512 source-integration midpoint intervals per
column and f32 depth/tau nodes. It is a one-field proof adapter; full source-group/all-layer enumeration stays with the
canonical coefficient provider. No per-family independent opacity products may be added as a substitute.

Let Delta=extent/logical_dimensions. The reconstructed zero-extended field has half-voxel support beyond its box.
Project `extent/2+Delta/2` on absolute Sun-frame U,V,W axes. The first U/V half-width is
`H=projected_half/(1-4/64)`, providing two nominal-cell widths beyond source projection before the extra guard centres.
W spans its exact projected reconstructed support. Builder and query derive these same values instead of inheriting
the legacy point-tau map's smaller pad. Every curve integrates from the Sun-facing boundary along W=-Sun. Cumulative
ordinary tau is nonnegative. Depth before the start is clear; depth after the support reads the complete last value.
Lateral clear requires the entire four-tap footprint to miss conservative source projection, not only the query centre. Within the padded domain evaluate the outer clear taps normally; a centre-only raw-box early return would create a boundary seam. Outside-domain footprints cannot be silently averaged as clear.

At a query, evaluate each retained neighbouring curve at the requested depth, then bilinearly weight its primary and
kappa transmittance **separately**. Do not exponentiate an averaged tau or reconstruct kappa attenuation from averaged
primary T. Metadata marks internal compression admission; it does not certify quadrature/lateral/full query quality.
One stale/invalid tap invalidates the whole footprint. The public point-tau contract remains separate from this filtered
closure query; source code must not pretend filtered light terms are exact point tau.

Private first adapter layout: group0 source texture0/sampler1/scene4; build outputs group1 nodes4/metadata5; read-only
query nodes/metadata use group0 bindings11/12. K48 node storage is66²*48*8=1,672,704B. Metadata stride/work counters
are finalized with implementation and counted explicitly, not assumed free. Each rebuild has66²*512=2,230,272 coefficient
requests before filtering/field reads; source queries and packing costs remain visible. Completed output plus constant
working state replaces the old K-sized fit workspace. Actual bytes/preparation/old-new overlap/backend spill costs remain measurement gates. Uniformly replacing the full344064-column8/12/16-knot24MiB example with K48 would require126MiB nodes plus5.25MiB at16B metadata, or10.5MiB with this prototype's32B instrumentation metadata. The65536-column refinement pool atK48 would need24MiB nodes before metadata. The local profile does not silently promote those full-workload allocations or claim they fit512MiB.

First direct fallback admits4096 requests per complete graph with512 source samples each, lower than the full design's
65536×2048 ceiling. The same direct ray supplies both attenuation terms. Count requests globally; beyond the cap return
an explicit negative-term/invalid sentinel, mark the graph failed and stop playback visibly. The renderer may show a
failure colour, never invented clear/opaque light. A broken map must not cause millions of unbounded inner light marches.
If required witnesses overflow or fall back heavily, refine/change the product before claiming a viable profile.

Ordinary surface/air/fog attenuation uses the primary term; cloud primary and fitted source bank use their respective
terms with unchanged extinction. Main-hidden reflections/environment still request the same light generation. Source,
frame, grid, reconstruction support, Sun and profile revisions invalidate products; camera movement alone does not.
Disabled/unselected product allocation and complete provider retirement still require their own runtime qualification.

## Required checks and remaining coupling

Independent CPU tests retain the old smooth-field failure, validate the tube on clear/slab/two separated layers/tower/
gradual/opaque/grazing curves, exercise nonuniform depths and capacity failure, and compare extrema/dense queries against
the derived bound. Actual GPU probes then compare retained nodes, terms, invalid taps and direct fallbacks using exact
float32 input identities; query/endpoints/primitive/fallback counters and private/resource bytes accompany timing.

Native shadow/cloud/mirror images and sampled1x/16x/64x motion must remain coherent. Compare direct source paths at
partial depths/low Sun and clear-to-thick lateral boundaries. A better scalar curve alone does not qualify filtering,
complete lighting, air/fog ambient, the clouded surface environment, new cloud families or full-sky budgets. The next
bounded view proof keeps the retained detailed source, its.125/m historical calibration and its source-quality limits;
that does not silently admit it to the complete SKY-G.1/m source profile.

## Actual bounded result and next decision

The [actual checkpoint](../../evidence/cloud-sun-depth-2026-11-02/README.md) preserves exact loaded sources, GPU buffers,
input paths, independent validation, native comparisons, sampled1x/16x/64x motion and quiet raw cost rows. Compression
passes the selected finite curves; field cropping/producer specialization preserve all stored coefficient/mip bits.
The tightened64²footprint still fails strict point-T (.1215/.0917 lateral error vs.008), and its backlit32²reference
patch differs by up to21display codes. Fresh-source/light native GPU p50/p95 is12.976/17.075ms, above5ms.
Do not enlarge the grid blindly or relabel these failed gates as passed. Select the production filtered-visibility
meaning separately from the point-tau API, and use measured component costs to choose the query layout. All higher
full-sky/air/fog/environment claims remain unproved.

## Rejected interval-index cost experiment

The bounded interval-index experiment was tested before introducing another optical approximation. The tube anchors are original
endpoints of the uniform512-interval integration stream. For each admitted column, a512-byte table maps each original
input interval to its retained compressed segment. Pack4unsigned byte indices peru32, with one writer per word. The
extra4356×512=2,230,272B increases local nodes/metadata/index storage to4,042,368B. This is a local profile cost, not
a uniform allocation for the full cascade worksheet. Building it walks512intervals and at most47anchors per column:
2,230,272interval visits, at most204,732anchor advances and557,568word stores; no new source, exp or log evaluations.

Use the exact producer's f32depth grid, not host-recomputed support. At a finite interior query, choose a candidate
original interval with floor(depth/span×512), load its packed segment and actual anchor endpoints. Match the binary
search's boundary convention, correct at most one adjacent segment when rounding crosses a boundary, then assert
actual ordered finite endpoints bracket the query and the index is below count-1. Failed verification must use the
bounded existing search or an explicit invalid result, never a clamped guessed segment. Keep terminal values and the
count2/clear fast path unchanged. Anchor interpolation/exponent/filter arithmetic must remain identical.

Prediction: remove roughly4×log2(K)dependent midpoint node probes per four-column query in exchange for four packed
index reads and eight endpoint reads, plus rare checked correction. Metadata/frame checks and eight exponentials remain.
The added build/table traffic may offset the saving; the current overlapping pass intervals cannot prove dominance.
Reject unless all admitted captured tables/brackets and GPU point/native images agree with the unchanged binary
reference, failure/retirement paths hold, and matched integrated timing supports retention. Preserve prior optical
failures; this experiment changes lookup work only, not the visibility footprint or source quality.

Actual candidate GPU125point queries match forced binary terms bitwise, and native90s/Sun30 images match the earlier
selected frame exactly. Independent readback validates every504,832defined index entry. The same-build ABBA whole-graph
medians are15.172(indexed),16.974(binary),16.679(binary),17.891(indexed)ms; completed cadence medians31.0/30.0/30.0/31.1ms.
The reversal and worse indexed cadence do not establish a repeatable gain. Reject the index as the selected runtime
path, preserve exact candidate sources/evidence, and restore binary lookup without its2,230,272B table/build. The
nonadditive component-timestamp experiment is also retained as evidence; the selected renderer returns to the earlier
combined cache pass/whole-only measurement path. This restores a previously qualified diagnostic, not a new measured
performance win. Both cloud and complete-sky budgets remain open failures.
