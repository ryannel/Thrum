# Bounded independent deep-light CPU reference

2 November 2026. Scope: selected ordinary cumulative-tau curve compressor only. No source integration, source geometry, GPU, air, lateral cache or consumer edit. The hierarchy source remains visually rejected. This does not promote a family or qualify a coupled light product.

The initial commission implemented the min-removal negative witness below. Root then commissioned the separate tau-tube replacement review/reference in the final section. Both candidates and the original failure are retained. The canonical hashes below identify the design read before that replacement; root owns its subsequent revision.

## Selected contract and current baseline

Read `docs/design/modules/CLOUDS.md` shared-light/error/work sections (SHA256 `21839fd027c1f930c90612dde9c45f0f69ae26619dbfd001ffe71b5ebc0a518b`) and `CLOUD-LIGHT-PROFILE.md` (SHA256 `bf6bf450fc8dcc8a6edd12eeb850700e021fd53e3052fca81bfa58782f867164`). API and numerical allowance were coordinated with sky_contract_cost before final handoff. The current `cu-caches.wgsl` baseline still integrates 64 uniform midpoint intervals and stores cumulative tau in an rgba16float 66³ texture. `cu-preview.wgsl` interpolates this ordinary-tau texture before exponentiation. Those facts establish neither the proposed compression tolerance nor source quadrature/footprint/storage quality.

The new files are independent Python standard-library code:

- `scripts/cloud_sky_proof/deep_light_reference.py`, SHA256 `7cf11972b80f75a6b455f1a095c87a1c6fa4e980e632de061df246ae94fe9ee2`.
- `scripts/cloud_sky_proof/test_deep_light_reference.py`, SHA256 `eb4adfc6235f66413d221bf4a9c74dafd4475c580bef0da92795c519a1deac45`.

`compress_tau_samples(iterable, max_knots=48, allocation=.005, max_samples=2048)` consumes ordered `(depth_m, cumulative_tau)` pairs. Depth must strictly increase, tau must be finite/nonnegative/nondecreasing. It returns retained ordered original knots, two accumulated analytic estimates, two separate accumulated numerical allowances, their sum as bounds, validity/reason, allocation and work counters. Invalid output is inspectable but `curve_terms(result, depth)` refuses it. Refinement or actual direct integration is required; invalid is not clear. `interpolate_tau` is a diagnostic scalar interpolation helper and does not confer admission. Out-of-domain queries fail rather than extrapolating clear.

Inputs are Python doubles unchanged. `float32_samples(iterable)` explicitly converts endpoints so the caller can pin exactly the float32 sample locations/tau used by the GPU helper. Conversion/storage error is outside compression. Rounded duplicate depths are rejected. `local_removal_error(A,B,C)` exposes the independent local analytic calculation. There is no general curve framework or cloud-source sampler.

## Calculation and triangle argument

Append an endpoint. If the count exceeds K, test every interior knot and delete the one minimizing the maximum local error across `exp(-tau)` and `exp(-.25*tau)`. Ties select the earliest interior knot. Retained taus stay original and monotone; both ends are protected.

For each old half-segment use normalized position `u` and write old tau `a+d*u`, replacement tau `b+e*u`. Nonnegative slopes follow from cumulative tau. An interior extremum of the transformed difference satisfies

`log(d/e) = k * ((a-b) + (d-e)*u)`.

The code uses log1p for nearby slopes and exp/expm1 for small transformed differences. Equal slopes or either zero slope need no root division: endpoints suffice. A/C are exactly shared, hence their differences are zero. Evaluate B once and at most one interior extremum on each half for each transform. This is at most six transformed comparison locations per candidate, matching the canonical count. Negative/nonfinite samples, duplicate depths, nonfinite complete spans and unresolved normalized depth splits fail visibly.

In real arithmetic let the previous approximation differ from the original sampled curve by at most B_k. Deleting a knot changes only its two-segment support by local sup error e_k. Triangle inequality gives new bound B_k+e_k over the whole processed domain. Appending preserves the old endpoint and adds an identical segment to the original/approximation, so it never resets B_k. Every deletion adds its selected local error even after another append or deletion. The reference retains no removal-history array or original sample list.

This proves the accounting structure, not arbitrary floating implementations. The reference calculates analytical estimates with Python float64/libm and separately accumulates at least **2e-6 per actual removal per transform**, as coordinated for the first GPU comparison. A small optical-scale float64 engineering guard is also computed locally; the larger allowance is used. Upward nextafter protects scalar sum rounding but does not turn libm/root selection into interval arithmetic. The selected 2e-6 backend allowance has no uniform WGSL certificate. Dense scalar tests are empirical checks, reported as such. Source quadrature has its own .005 allocation; lateral footprint .008 and storage .002 remain separate. A .005 compression result does not establish total .02 query quality or radiance accuracy.

## Tests and retained failure

Command actually run: `PYTHONDONTWRITEBYTECODE=1 python3 scripts/cloud_sky_proof/test_deep_light_reference.py`. Eleven tests pass, including dense independent query comparisons, nonendpoint extrema, equal/zero/near-equal slopes, deterministic random triples, all removals/append accumulation, ordinary-primary-black/reduced-depth-sensitive rejection, invalid inputs, float32 collisions, translations/depth stretching and visible endpoint capacity failure.

For 257 exact float32 endpoints, K48 uses 209 removals, 9,823 candidate evaluations and peak 49 knots. Each transform receives .000418 numerical allowance. Dense 64-way subdivision comparisons give:

| Curve | Actual errors (primary, quarter) | Total reported bounds | Admission |
| --- | --- | --- | --- |
| Clear | 0, 0 | .000418, .000418 | valid |
| Slab | 1.21e-8, 1.13e-8 | .000418087, .000418078 | valid |
| Two separated thin layers | 0, 0 | .000418, .000418 | valid |
| Dense tower | 1.11e-16, 1.11e-16 | .000418, .000418 | valid |
| Gradual `tau=12*(x/128)^2` | .000554750, **.0145821** | .0115232, **.0330637** | **invalid** |
| Opaque | 2.87e-48, 2.69e-13 | .000418, .000418001 | valid |
| .2-degree grazing slab | 3.36e-279, 7.60e-75 | .000418, .000418 | valid |

The known gradual failure is an explicit regression. More endpoints do not rescue this greedy bounded stream: with 513 float32 endpoints (initial GPU comparison count), its actual errors are .00114447/.0165993 and total bounds .0185431/.0417543. It uses 465 removals, 21,855 candidate evaluations, 44,148 actual transformed comparison locations and peak 49 knots. The result correctly requires fallback. This is evidence against assuming K48 generally admits smooth integrated fields, not against all cumulative-light representations. The fixture is a prescribed tau curve, not a claim about the current cloud source.

## Counted limits and next boundary

At 513 endpoints the worst location bound is `6*47*465=131130` per column. At the 2048-endpoint ceiling, 2000 removals imply 94,000 candidates, at most 564,000 transformed comparison locations and at least .004 numerical allowance per transform. This leaves only .001 of the .005 budget for analytic compression. Each comparison location evaluates two exponential terms; log/root work is additional. These counts can be prohibitive across many columns despite bounded memory. GPU/private-array spill, synchronization, densities and fallback rates remain unmeasured here.

Input endpoint overflow stops visibly, with processed-prefix counts and invalid status; no prefix may be used as the complete light path. Quality exhaustion remains invalid while bounded input consumption continues for diagnosis. No invalid column may be averaged as clear. Root/sky_contract_cost own actual field integration, GPU helper, direct fallback, generation joins and cost/consumer evidence. If this greedy stream fails quality or cost on the required views, representation/sharing must change; this note does not authorize extra endpoint counts, dropped layers, altered sources or a fabricated certificate.


## Separately commissioned tau-tube replacement

After the initial K48 failure, root proposed a constant-tau-tube segment builder and asked for an analytical challenge before implementation. The real-arithmetic argument is sound under the following exact endpoint rule; it is not a late resampling of the failed greedy curve.

`compress_tau_tube(iterable, max_knots=48, allocation=.005, max_samples=2048)` and `tube_delta(anchor_tau, epsilon)` are additional functions. The result carries `method="tau_tube"`; its `analytic_bounds` property reports maxima across disjoint pieces. The reused result field `accumulated_analytic` means a triangle sum for `min_removal`, a disjoint-piece maximum for `tau_tube`. The original min-removal code/evidence remains available.

For each anchor `(xa,ta)` take `epsilon=.005-2e-6=.004998`. For each exponent a, if `exp(-a*ta)<=epsilon`, its downstream transformed difference cannot exceed epsilon for any monotone nonnegative line. Otherwise define

`delta_a = -log1p(-epsilon/exp(-a*ta))/a`, then `delta=min(delta_1,delta_.25)`.

The implementation never forms `epsilon*exp(+a*ta)`; it branches on the decaying term first. If both branches are opaque, delta is infinite. A one-step downward nextafter is an engineering adjustment, not a libm certificate. Degenerate unrepresentable arithmetic remains unsupported rather than fabricated.

Maintain lower/upper feasible slopes intersected with nonnegative slopes. For every seen input knot `(xi,ti)` the interval is `[(ti-ta-delta)/(xi-xa), (ti-ta+delta)/(xi-xa)]`. Crucially test the proposed endpoint's **actual** slope `(ti-ta)/(xi-xa)` against the **previous** intersection first. Its own interval necessarily contains that actual slope. On failure, emit the immediately previous accepted input endpoint, whose actual line already fitted all earlier constraints, make it the new anchor, reset the range, and reprocess the current endpoint. Do not select an arbitrary feasible slope and invent an endpoint tau. The first input after a new anchor must fit; an unresolved second failure is invalid. Emit the final actual endpoint if it was not already emitted.

Every accepted line differs from the original input PL tau by at most delta at every original knot. The difference is linear between input knots, so that bound holds throughout. Original and replacement tau are both >=ta because samples and output slopes are monotone. Thus the transformed error is at most `exp(-a*ta)*(1-exp(-a*delta))<=epsilon`. Accepted output pieces have disjoint interiors. Their error combines by maximum, not sum; resetting the slope interval at an exact source endpoint does not erase an error inherited from another overlapping approximation. This is why this replacement does not contradict the original never-reset triangle rule.

The implementation reports per-transform `max(min(epsilon,exp(-a*anchor_tau)))` across emitted pieces, plus one separate **2e-6 numerical allowance** per query. This yields .005 for initial clear-anchor cases. This is conservative about the admitted tau tube but not a proven uniform floating-backend bound. It does not certify a WGSL slope comparison, exp/log implementation or final curve storage. The direct fallback and future backend probes remain necessary. Source/field integration retains its separate .005 allocation.

Final command `PYTHONDONTWRITEBYTECODE=1 python3 scripts/cloud_sky_proof/test_deep_light_reference.py` passes **17 tests**. Tube tests cover the opaque analytic branch, finite delta for both transforms, repeated actual endpoints, restart at previous endpoint, all seven original dense-query witnesses, random monotone curves, translated/grazing depth scales, invalid input/endpoint caps and a genuine K48 thin-layer overflow.

Tube results for the same 257 input samples:

| Curve | Retained original endpoints | Actual errors (primary, quarter) |
| --- | --- | --- |
| Clear |2|0,0|
| Slab |4|2.28e-8,2.38e-8|
| Two separated thin layers |6|0,0|
| Dense tower |4|1.11e-16,1.11e-16|
| Gradual |14|.00454632,.00431421|
| Opaque |9|.000126785,.00260640|
| .2-degree grazing slab |4|1.40e-15,5.55e-10|

Every row is within the reported .005/.005 bounds in the independent dense scalar queries. This is a useful bounded CPU result, not a GPU/source-quality pass. At513 exact float32 gradual samples, tube retains14 endpoints with actual errors .00482002/.00435534 (128 subdivisions per original interval),524 endpoint visits,13 anchor builds/pieces and peak14 anchors. That is substantially less work than the retained min-removal witness's21,855 candidate evaluations. Each input gets at most two slope visits; exp/log delta work occurs only when a new anchor is created. The bound on live state is K retained anchors plus previous/current endpoints and scalar ranges/counters. There is no full input list or candidate scan.

Capacity remains substantive: the monotone many-thin-layer witness `(depth=i,tau=.02*floor(i/2))`,1000 samples, exhausts48 knots at the50th input. It returns an unavailable incomplete prefix; `curve_terms` rejects it. No blocker is declared absent. This is a required fallback/cost witness for the new method, not permission to silently raise K or drop layers.

The root/contract reviewer owns the canonical replacement selection, GPU arithmetic/data layout, actual ordered source integration, generation/fallback joins and build/read costs. No source geometry, renderer, server, GPU job or canonical document was changed by this CPU commission.
