# Cloud contract/cost closure review

1 November 2026. Read the actual revised canonical text, including the final packing, phase-ratio, A0/refinement/environment caps and propagated source bound. Reviewed CLOUDS SHA-256 `67e92ad2a3944c9489ff50d70fd47bc6a3930e3b543c2775c69bc11ef589fb2f`; CLOUD-CAPABILITIES `4ef919ceb068e1514fb86dd10b1e59544b0dd88c86a00d9e9dcce8e2ccae99be`. No canonical changes, code, GPU runs, benchmarks or commits.

**Finding:** the previous six structural findings have concrete design dispositions. No outstanding P1 contradiction was found in these revised mechanisms. This is analytical design closure at the named seams, not implementation, experienced quality, numerical certification or whole-sky cost admission. The large counted ceilings and existing adverse timing remain open gates.

## Previous findings against actual revisions

| Previous finding | Actual disposition / review result |
| --- | --- |
| Ambient recursion/double air/Sun | CLOUDS509–552 defines ordinary depth → A0 containing only directly Sun-lit air/fog, all extinction, no cloud source/ambient and no disk → fixed cloud source → final joint views/probe. Final probe does not feed A0. This is acyclic and counts the relevant air/direct/cloud terms once under the declared truncated closure. Black ground/no ground bounce and absent cloud interreflection are explicit approximations. |
| 768 km all-path boundary | CLOUDS359 onward makes768 km a view worksheet rather than a source cutoff. World virtual cube-face population, area correction, padded neighbors, canonical ID deduplication and continuous world fields supply actual view/Sun/ambient/probe requests. The x400/x780 ambient counterexample is now an explicit test. Outside-cache queries do not silently become clear. Fixed-axis height-dependent rotation integrates Omega(h(t)); no separate camera wind/sky clock is introduced. |
| Uncounted light/fallback work | Cascades8/64/512 km wide, endpoint/primitive/fit ceilings, refinement2048 endpoints, direct fallback pool, A0 anchors/directions/endpoints and environment rays now have explicit finite candidate capacities. Overflows remain failed proof/admission, not permission to omit required behavior. See the aggregate arithmetic below. |
| Curve-fit versus query error | Each branch and primary T has total0.02 tolerance split0.005 quadrature/field,0.005 compression,0.008 lateral,0.002 storage; removal error persists through subsequent appends. Invalid taps invalidate the footprint. Each filtered exponential term is retained separately; mean-primary-T is not reinterpreted as a universal tau. Point tau remains a point query. Weighted radiance remains a separate gate. |
| Sun/source/half bound | CLOUDS566–584 derives A0<=960000 from bounded direct-only sources/terminals, then cloud/all-media B_source<=1920000. The former ambiguous20000 ambient-source assumption is removed. Sun disk bound, positive Beer remaining-radiance inequality, composed2e9 maximum and2^-16 storage exposure are explicit. Emissive lightning remains a bounded typed seam, not an unbounded admitted optical light. |
| 4Z centres/guards/mips | CLOUDS304–318 now explicitly uses q=u−0.5, floor(q), fract(q), mathematical negative floor and channel/group selection, fixing the earlier half-cell ambiguity. Odd/tiny sizes, zero padding/guards, logical eight-parent mips, point level0 and qualified footprint filtering are specified. Base plus two additional levels is explicit. Scalar remains unproven against the one-channel golden. |

The FAIR high-layer witness is now unambiguously required in the capability ledger.

## Energy and once-only source check

The new fixed weights/b factors have a useful analytical bound; they do not recover physical multiple scattering. For cloud-only optical depth tau, each branch integrates `w_n b_n exp(-b_n tau)` to at most w_n, hence the fixed common nonnegative weights summing to one do not allocate one beam repeatedly across liquid/ice components. Phase functions must remain normalized and scattering cannot exceed extinction.

The direct-source inequality also extends to competing air/fog along the same beam under the stated assumptions. Let cloud extinction be sigma_c, other extinction sigma_o, and `T_n=exp(-b_n tau_c-tau_o)`. Because0<b_n<=1, ordinary `T_0<=T_n`. Thus the sum of generated cloud power and ordinary other-media direct scatter is bounded by

`sum_n w_n (b_n sigma_c + sigma_o) T_n`,

whose beam integral is at most one. Geometry and albedo can only reduce it. This does not prove the spatial approximation, angular field, numerical map or final picture. A0 adds a separately truncated air→cloud transport route and must retain its stated exclusions; no iterative cloud diffuse energy is being certified.

The corrected thin statement is important: sum(w*b)=0.82125 is the **angle-integrated** source ratio. Ice with equal phases has that ratio directionally. For liquid g=[.86,.43,.215,.1075], the thin directional ratio versus the first phase is approximately0.70506 at cos(theta)=1,1.42636 at0 and1.68620 at−1. The actual text now acknowledges directional gains/losses. Thin/backlit appearance must be judged against that redistribution rather than a universal17.9% loss claim.

## Sun bound arithmetic

With E<=120000 per RGB channel and alpha>=0.0045, `E/(pi*sin(alpha)^2)` is at most approximately1.8863e9 cd/m², below the admitted2e9 bound. The direct-only A0 source ratio is bounded by120000×8=960000; positive Beer transport with terminals<=20000 keeps A0 and its normalized angular mean below960000. The stated cloud bound1920000 conservatively adds a directional source ceiling960000 and ambient960000. Mixtures weighted by total extinction preserve that ceiling. Consequently the positive-transport maximum2e9 remains valid, and multiplying by2^-16 gives30517.6, below half maximum65504.

These bounds require the declared maxima to cover the **aggregate active** medium/light mixture, not silently only each independent constituent. Tests must reject inputs beyond admission instead of clipping coefficients. Underflow, quantization, actual disk filtering and final display remain unrun.

Carry the terminal bound through reflected paths: a main ray not aimed at the Sun may terminate on a mirror containing the Sun. Its near segment therefore inherits the far-leg terminal bound; a straight-main-direction disk test alone is insufficient. This is a required realization of the existing “actual possible terminal” rule, not a new reflection API.

## Total ceiling and memory arithmetic

For one active direction, the light allocations are24 MiB base nodes +5.25 MiB base metadata +8 MiB refinement nodes +1 MiB refinement metadata = **38.25 MiB**, before private/spill state, source/detail/index fields, air/A0/probe products, view targets, CPU mirrors and in-flight generations. A second active direction duplicates that directional closure. This does not establish peak residency or fit the old88 MiB reservation.

Maximum endpoint visits under the chosen stress caps:

| Stage | Endpoints |
| --- | ---: |
| Base OVERCAST/STORM depth columns |234881024 |
| Refinement columns |134217728 |
| Direct light fallback |134217728 |
| A0 pool |402653184 |
| Direct A0 fallback |50331648 |
| Final environment rays |25165824 |
| **Subtotal before main/mirror/secondary views** |**981467136** |

These are ceilings, not predicted simultaneous means or measurements. Base light endpoints alone times64 contributing primitives permits15.03 billion primitive contributions; direct light fallback permits another8.59 billion. Some queries use precomputed fields rather than evaluating every primitive, but those savings must be observed. Streaming fit comparisons, conservative index traversal, nested directional-light lookups, filtering and allocation/update work are additional. Do not call the endpoint subtotal total engine operations, nor convert it into milliseconds.

The selected caps now make the proposal falsifiable. They do not suggest the graph is remotely admitted to5 ms cloud/16 ms complete browser by arithmetic alone. An overflow or sustained budget failure requires a different qualified representation/capacity, preserving required witnesses. The draft explicitly retains that rule.

## Concrete carrying obligations

No further structural redesign is required by this review before the bounded correctness/family work. Carry these conditions into its existing gates:

1. **P2 instrumentation scope:**64 contributing primitives after indexing does not bound rejected candidate/index-node visits. Count traversal/materialization/request queue work separately and cap it under the same explicit failure policy; report it alongside endpoints, fits and nested light lookups. A finite sphere/path depth alone is not a cheap-query claim.
2. **P2 reflection-bound test:** add the bright-Sun-in-mirror case described above to exit-bound tests, with near air/cloud in front of the interface. Propagate the far-leg bound instead of assuming direct-main-ray geometry establishes its terminal maximum.
3. Enforce aggregate optical/source bounds and invalid generation/footprint rules, including cube-face seams and main-hidden retained consumers. The named slab/curve/packing/air oracles test the actual algorithms, not just these equations.
4. Retain all numerical/appearance/performance failures separately. Current Q1/coupled source and cost receipts remain limited witnesses. Every required family, changing-weather scene and complete workload still needs the commissioned native/motion/consumer proof and actual work/residency/tail measurements.

No new module, external API, engine construction dependency, performance certification or reduced commission is proposed.
