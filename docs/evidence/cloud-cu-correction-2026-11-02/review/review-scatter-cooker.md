# Independent scatter-cooker/data review

2 November 2026. Read actual cook_scatter_profile.py/test_scatter_profile.py, selected CLOUD-LIGHT-PROFILE.md and
scatter-liquid/profile.json/A.f32/axes.f32. Ran only the13focused CPU tests and small read-only package/scalar checks;
no GPU, full recook/refinement campaign, implementation/canonical edit or commit.

**No blocking numerical, layout or claim defect found for the labelled empirical liquid comparison table.**
It is reasonable to use this package for the separately gated source-bank image comparison. This is not an
appearance pass, a uniform energy certificate, or qualification of a GPU lookup/body-column producer.

## Inspected identities and package checks

- cooker SHA-256: `04fa89c76e435bbeb4af266d692a7ee4d089b3be9ea98ea5f7a6b4ea5d97d347`;
- profile.json: `8de788d28c53935f803f3e78085aaf46eec4f8fcb346c857da64522693ea8604`;
- A.f32: `35aab6271d4119719c7ca536f1c22fc2cce492e7e7c8abab88f9fe71909a4ed4`;
- axes.f32: `efb148cbf3b059d8526aee2fe47eab3dc76d0bc0c82956f89fd33f33b16a21e8`.

The actual tool hash matches metadata. A is256x256little-endian f32 row-major[mu,T],262144bytes; axes are256T then256mu
f32 values,2048bytes. Binary axes exactly match the JSON values; endpoints/order are valid. All A values are finite,
nonnegative and below3.724; observed maximum3.682370662689209. Profile parameters/key material explicitly include
normalizedHG(.86),omega1,kappa.25,rho.8, axes, layout/interpolation/branch semantics, code version/hash and binary hashes.
The ice candidate phase key remains separate; package metadata does not activate an ice bank or silently reuse liquid.

## Formal-solution and interpolation review

The finite-bottom term uses the stable difference integral. Factoring by exp(-min(a,b)T) and using expm1 on the
nonnegative slope difference handles either ordering and the exact equal-slope T*exp(-aT) limit without near-zero
subtraction. Finite depth correctly retains both top and bottom escape; infinite-depth bottom is zero.

The mu0=0 branch uses the analytic top half-hemisphere limits and retains finite bottom transmission through the
ordinary view path. It does not substitute the semi-infinite A=.15 for every finite grazing slab. Clear T=0 explicitly
gives direct1/primary0/bank0; primary source is restored independently of A. The written formula and the normalized
phase sign/marginals agree with the selected top/backscatter versus bottom/forward geometry.

The fit separates F0 direct+escaped primary from unit-bank F1, then uses the prescribed gate/min cap. It neither caps
all generated scattering-event power at1 nor equates that generated integral with escaped flux. Named data deliberately
contains generated ratios above1 while separately reporting escaped values. This is consistent with the earlier
reviewed distinction; no converged multiple-scattering solver is claimed.

The exact f32 axes are passed as f64 when solving table nodes; lookup fractions use actual enclosing physical T and
mu endpoints. The interpolation test rejects substituting log(T) fractions. Values below the declared clear threshold
return0; T>=1000/infinity selects the last semi-infinite column with mu interpolation. That is the selected tail
policy, not an unbounded runtime fit. GPU f32 arithmetic, body quantization and world-column error are explicitly
outside this CPU lookup result. The near1e-4 boundary uses exact stored f32 axis values and the named threshold;
small threshold/rounding discrepancies are part of its recorded empirical approximation, not a new thin-cloud claim.

## Actual empirical result and meaningful test scope

All13tests pass (~0.12s). They cover profile rejection, known fit values, equal/unequal-slope bottom integrals,
grazing finite-bottom continuity, clear/exact-primary cases, finite cap/range, generated>1, required-angle quadrature
comparison, invalid domains, strict f32 axes, physical-T interpolation, row-order/hash roundtrip, bad payload rejection
and the hard source-ratio bound. The cap test shares the fitted flux functions, so it proves their algebraic contract,
not an independent physical reference solve. The separate analytic limits/known values and packaging checks make the
suite more meaningful than a same-function identity alone. None proves a uniform quadrature/interpolation error.

I reproduced the metadata's worst off-grid A case at T6.868038654327393,mu0.9901960790157318:
reference3.641218057309739 versus lookup3.6202598214149475, absolute error0.02095823589479151. The reported256² grid
therefore misses its0.02selection heuristic. The manifest truthfully records heuristics_met=false, empirical_certificate=false
and uniform_error_bound/uniform escaped-energy certification unproven. Reported off-grid maximum escaped excess is
5.197931419953505e-6; stored-node maximum is5.345704212622593e-8. These are sampled results, not exact global limits.
The maximum coefficient error can affect image brightness; the empirical label does not make it visually negligible.
It is nevertheless not a blocker on the explicitly allowed native appearance experiment, and does not justify an
unsolicited refinement/certification campaign.

The package's hard A bound and allowed phase peak/irradiance/ambient assumptions still imply the selected2e6
source-ratio cap. That is distinct from its approximate slab energy behavior. Actual GPU source/early-exit integration
must adopt the new bound, share ordinary all-cloud tau and preserve receiver Beer shadows/independent consumers.
A correct CPU table cannot close those unimplemented handoffs or fix the rejected smooth source geometry.

## Nonblocking operational limit

write_package uses pending-file replacement and publishes JSON last, but replaces fixed A.f32/axes.f32 before the
manifest. That is per-file atomicity, not transactional recooking of an already live package: interruption/concurrent
reading can leave the old manifest pointing at newly replaced bytes. The current inspected package is complete and
hash-valid. For this bounded comparison, cook into a fresh/versioned directory or ensure readers are quiescent and
verify the full closure before admitting it. If live package replacement is later promised, immutable payload names
plus atomic manifest publication are a small appropriate correction. No general engine storage framework is needed
for the present scratch product.

Proceed with the planned geometry diagnostic first and then the frozen-density/light/display bank comparison.
Native front/back/underside, finite puff, remnant-over-parent, thin ice, low Sun, overcast and reflected/environment
witnesses remain open, as do motion and complete workload cost. This review certifies none of them by association
with a passing scalar cook.
