# Source-height density cohorts — exact tide-only feasibility, 3 November 2026

**Result: the restricted constant-rate, affine-tide density law is closed and agrees exactly with an independent space-first oracle, but eight cohorts are not a general weather/deposition guarantee.** Steady 60 Hz tide, refill with 1 Hz rain changes, and repeated reversals stay within 3/6/4 cohorts. Zero-refill rain changes require a ninth cohort at `361/60 s`; four resolved dry-band deposits require a ninth at 4 s. Those bounded profiles refuse, without discarding history or advancing their committed prefix. The proposed replacement remains a challenger, not canonical C09 adoption or a general coast runtime.

Read the [fixed-column failure](MOVING-SHORE-SPATIAL-ADMISSION-2026-11-02.md), [material chronology study](MOVING-SHORE-MATERIAL-CHRONOLOGY-2026-11-02.md), [moving-shore design](MOVING-SHORE-EXPOSURE-DESIGN-2026-11-02.md), [C09](../CONTRACTS.md#c09-environmental-fields-and-physical-exchange--e1-with-p3) and [PLAY](../PLAY-ENVIRONMENT-SPEC.md#e1-moving-shore-precipitation-and-the-distinct-capacity-profile). Editable experiment: `tmp/moving-shore-cohort-proof/`; its selected source/fixture/result/measurement set is retained in the [six-file evidence archive](../../evidence/ARCHIVE-ACCESS.md#archive-inventory). No shared contract or engine code is edited. Names are experiment/proposed-interface names, not SDK source schemas.

## Representation, metric and deliberately bounded law

A fixed source triangle carries intervals in its actual affine bed height h. On each interval, local surface-liquid **projected density** is `q(h)=a+b h`, kg per horizontal projected m². A cohort owns that function over its source-height band, not a uniform pool over whichever region is currently wet. Absorbed liquid is zero in this proof. At an endpoint, integrate q over the triangle's actual `h<eta` and `h>=eta` cuts to obtain coating and film. The same q remains attached to the same material coordinates through a reversal. No representative point labels the whole triangle and no wet/dry-subset homogenization changes the field.

The exact Fraction test law authors cap C and refill K **per projected m²**. This is an explicit material challenger, not canonical true-area `r_w,k_w` with a silently rounded metric. Dry rain P retains its original kg/(horizontal m² s) meaning and static first-hit eligibility. For a 1:20 source bank, true/projected ratio is `beta=sqrt(401)/20`; derived true density is q/beta. All source triangles really lie on that bank, rather than pretending the bank area is horizontal. True-area adoption is a separate bridge below.

Select nonnegative constant P/K within each source span, affine prescribed stage, retention on ebb, wet-entry coating cap and resolved **height-band** deposition at named source barriers. Deposits may be affine in h, uniform along the corresponding level set inside a triangle. Arbitrary localized two-dimensional splashes are unsupported. Linear time-varying rain, absorption, evaporation, drainage, snow, finite-water grants and waves are outside this proof. These restrictions are declared, not assumed neutral substitutes for required consumers.

Each source fragment binds one stable material/geometry identity, static first-hit eligibility and one prescribed water owner/driver. It cannot cross a material, rain-cell, roof/shelter or water-owner boundary. Sheltered floor eligibility remains zero even when its material is wet; roofs and exports keep their own source receipt ownership. This experiment tests the local density operator, not an F4 overlap cooker. The opposing-fragment adversary supplies separate local stage programs; it is not a single global tide or a wave solver.

The source height range is fixed by the triangle. Mutable cohort intervals cover that range disjointly; adjacent intervals merge **only when their density coefficients are exactly equal** and their immutable bindings agree. Cohort ordinals are private state layout, not objects or delivery identity. The initial condition here is uniform q, no higher-dimensional hidden state. A fresh cook/run establishes it; no save migration or remap is implemented.

## Closed chronological operator

For span duration dt and stage `eta(t)=eta0+v(t-t0)`, a point's wet crossing time is affine in h when v is nonzero. Split the source-height intervals at eta0/eta1, existing cohort boundaries, and roots introduced by cap/refill comparisons. No epsilon snapping is used.

* Always dry: `q_new=q_old+P dt`.
* Always wet: `q_new=min(C,q_old+K dt)`; its increase is a named prescribed-water refill.
* Rising stage, newly wet height: dry duration `d_d=(h-eta0)/v`, wet duration `d_w=(eta1-h)/v`. Set `q_pre=q_old+P d_d`, `release=max(q_pre-C,0)`, `q_new=min(C,q_pre+K d_w)`, `refill=q_new-q_pre+release`. Direct-water rain is P d_w; solid rain is P d_d.
* Falling stage, newly dry height: `d_w=(eta0-h)/(-v)`, `d_d=(h-eta1)/(-v)`. Set `q_kept=min(C,q_old+K d_w)`, `refill=q_kept-q_old`, `q_new=q_kept+P d_d`. Ebb does not create a target film.
* At a resolved deposition barrier, add its affine density over its exact height band. Where wet, release excess above C immediately to the prescribed owner; where dry, retain it. This recipient/deposition law is explicit, not a delta source smeared over the tick.

Every right-hand expression is affine on the branches selected by affine inequalities. Their roots therefore divide height into a finite set of intervals on which the same P1 field is exact. Cap/refill can increase the interval count; mathematical closure does not promise an eight-interval bound. Static stage has an explicit no-division branch. The proof triangles have nonconstant height, so an equality-height cut has zero area; flat/shelf height distributions and tangent impulses need their own producer, not this argument.

The per-fragment mass identity is checked after every candidate step:

`surface_new + cumulative_release = surface_initial + cumulative_solid_rain + cumulative_refill + cumulative_deposition`.

Direct-water plus solid rain equals the eligible horizontal footprint's exact source integral. Refill/release/deposition are separate named exchanges with equal reservoir/donor entries in a future assembly. Neither absorption nor a finite-water debit is fabricated here. Film/coating are exact endpoint integrals of retained density; they are not independently mutable rival masses.

## Candidate and independent oracle

The candidate stores affine density coefficients and height intervals, advances them by the formulas above and normalizes exact equal neighbors. It integrates each density using **clipped-source polygon area and first-height moments**: `mass=a delta(M0)+b delta(M1)`. M0/M1 are computed from actual triangle clipping and a fan decomposition; no point-column approximation remains. The experiment uses exact Fractions for geometry, times, roots, coefficients and receipts.

The independent oracle does not call cohort advance/deposition or consume its intervals/coefficient list. It follows the event history at a symbolic source height: source-time spans, that point's crossing, its dry rain, capped refill, wet-entry cap and timed deposition. Each affine comparison records its root; independently partitioning height at those branch changes gives the exact point-history function. Geometry is integrated **space first** using the triangle's piecewise-linear width directly from its three projected edges, including the Jacobian `dx/dh=20`. This is a different geometry and temporal formulation, not fitting candidate outputs at a few heights.

Independent source compaction merges adjacent collinear equal-rate driver spans except across deposit barriers. Thus the reference can integrate one six-second source span while the candidate executes 360 physical test steps. Density coefficients are compared over the union of both independently discovered height partitions, in addition to every film/coating and exchange integral. Equal totals alone cannot pass.

The earlier two-fragment rain adversary is represented by actual sloped triangles, each projected area 1/2 m². A's geometric wetting times vary over its triangle but their mean is 1/2 s; B remains dry. Exact A coating is 1/40 kg, B film 1/20 kg, direct-water rain 1/40 kg. A wrong total-dry-rain remix yields 3/80 kg at each fragment and fails. The three-fragment adversary uses area-1/3 triangles: wet A dries, a separate local stage later wets B, and C stays dry. All 3/100 kg remains at A; B/C remain zero. The 1/100-each homogenization mutant fails. This is the geometric version of the earlier histories, not a claim that all spatial points crossed at their old single-point timestamps.

## Exact results and count stress

All ordinary tick stresses below run the candidate at 1/60 s. The simpler adversarial source windows are 1–3 second mathematical fixtures, not F1 runtime steps. After the first eight-cohort refusal, an **unbounded diagnostic** continues for the oracle comparison. Its later results are explicitly not committed bounded state. The last valid bounded prefix is retained separately and its canonical bytes are asserted unchanged after rejection.

| Fixture | Peak / final cohorts | Eight-cohort outcome |
| --- | ---: | --- |
| Two-fragment rain, A and B | 1 / 1 each | exact local oracle and remix rejection pass |
| Three-fragment opposite histories | 1 / 1 each | exact local oracle and remix rejection pass |
| Rising/falling/re-rising, timed affine deposition, binding cap | 4 / 1 | passes; final film 0, coating 1/10 kg |
| Steady tide, six seconds / 360 ticks | 3 / 3 | passes restricted profile |
| Rain changes once/second, K=0.02, twelve seconds / 720 ticks | 6 / 3 | passes restricted profile |
| Same rain changes, K=0 | 14 / 14 diagnostic | ninth cohort at tick 361, t=361/60 s; bounded prefix stops at 6 s |
| Sixteen rise/fall spans / 960 ticks | 4 / 4 | passes restricted profile |
| Five resolved dry height-band deposits, once/second | 11 / 11 diagnostic | ninth cohort at tick 240, t=4 s; reject that whole interval before the fourth deposit commits |

The mixed cap/deposition fixture's exact totals are: solid rain `22/525 kg`, direct-water rain `143/2100 kg`, refill `41987/3780000 kg`, release `464167/18900000 kg`, resolved deposition `202/9375 kg`, final coating `1/10 kg`. The independent point-history/width oracle agrees on each kind and local endpoint. These values are test-law balances, not calibrated soil behavior.

The varying-rain stress uses P=`1/360+i/36000` kg/(m² s) during second i, slowly rising stage speed 0.00015 m/s and initial q=0.05 kg/m². Its mild once-per-second authored rate changes create distinct retained slopes when refill is absent. Default refill eventually erases some old differences through physical saturation, explaining its bounded count; this is not permission to add artificial refill to keep storage small.

The dry-deposit stress has no wet contact, five disjoint height bands, a 0.001 kg/m² projected increment in each, normal quiet rain and one deposit per second. Four persistent localized bands plus the intervening base field already need nine intervals. It is not an extreme time-resolution artifact. A general resolved-deposition profile therefore cannot promise eight exact cohorts. Height cohorts also cannot represent arbitrary across-level-set localization in the first place.

**Feasibility decision:** retain this as a credible restricted tide-only field mechanism; reject an unconditional <=8-cohort guarantee for reusable coast weather/deposition. Nine of the eleven fragment fixtures stay bounded; two deliberately relevant source profiles fail. No extra limit or unproved approximate merge is introduced to hide those failures. A later profile needs independently bounded moment/field-error merging, a larger honestly charged budget, supported-content restrictions, or another representation.

## Receipts, refusal and persistence boundary

Coalesce exact exchange mass by stable source fragment, kind, donor and destination over the physical delivery interval. Production identity must include world epoch, source/control revision, stream, actual time interval and fixed donor/receiver/kind; cohort ordinal, cap-root ordinal, numerical subdivision and tolerance are not delivery identity. The fixture strings are names in its own proof namespace, not F3 opaque IDs.

The toy hashes canonical normalized cohort state **and** positive exchange rows with kind/donor/receiver/mass. Equivalent evaluation of the same affine source in one or two spans produces identical final normalized fields and coalesced receipts. A changed exchange mass produces a different digest with the same retained identity table. This is a deterministic digest/conflict probe; it is **not** a persistence/concurrent commit implementation or a proof of finite-water donor validity.

At a cohort-count refusal the candidate source interval and its exchange ledger cannot publish. A production host must retain the complete last X[n] and its receipt/source cursors and stop or choose a reviewed new physical profile/migration; it cannot skip rain or advance source time against the old material field. The experiment's immutable frozen-prefix check covers its local pure state only. P7 save/restart, W2 activation and shared water/material atomic publication remain unimplemented.

## True-area adoption bridge

On this affine bank canonical true-area cap/refill require `C_projected=beta*r_w`, `K_projected=beta*k_w`, and initial projected density `q0=beta*rho_true`. Precipitation stays P per horizontal area. Changing the material basis does not change the actual source triangle, physical mass or units. The present C=0.1 and K=0.02 rational projected inputs correspond to slightly different true-area controls and **must not** be substituted for canonical r_w=0.1, k_w=0.02.

Two legitimate future routes are (a) exact algebraic coefficients in `Q(sqrt(401))`, including exact ordering of cap roots and height moments; or (b) a certified rational beta interval propagated through coefficients, branch/root enclosures and integrated masses. Picking a beta midpoint and inheriting this proof is insufficient. Uncertain cap ordering must refine/fork within a bounded validator or refuse. The same affine-field closure formulas remain algebraically applicable, but neither route is implemented here and the canonical full law is unproved.

The two count failures are structural, not a rounding artifact: the zero-refill stress stays below cap by a generous margin, and all five deposition bands remain dry. Metric conversion does not remove their distinct nonzero density slopes/bands. Absorption, evaporation or additional physical diffusion might erase them only under its actual independently specified law; that is new physics, not an optimizer.

## Bytes, work and numerical gates

At 8192 source fragments and eight cohorts each, the maximum is 65536 cohorts. A proposed 64 B record can hold two height bounds, projected surface and reserved absorbed affine coefficients, error and flags; the exact experiment currently needs only surface coefficients. Mutable state is **4 MiB per endpoint**, 12 MiB for two committed plus one staged, and **16 MiB** with a separate save owner. Cohort topology is part of those intervals, not an uncharged child mesh. Source identity/triangle geometry and prepared area/first-moment laws are additional immutable products; a provisional 256 B/source-fragment account is 2 MiB, or 4 MiB during equal-size old/new product overlap. Receipt/pending scratch and original water/collision/render products remain additional owners.

One ordinary global tick at the full bound visits at most 65536 cohorts; sixteen physical barriers multiply that to 1048576 visits before source knots, cap roots, deposits and integrations. The six-second steady diagnostic records 1078 visits over 360 steps for one triangle, about three per step; the refill/rain-change diagnostic records 3232 over 720 steps, about 4.5. The exact prototype repeatedly clips polygons to establish moment identities; a proposed runtime instead evaluates prepared band moments, reuses them across exchange fields, and streams fragments through bounded scratch. That optimized product/layout does not exist yet.

Counts/timings in `measurements.json` include independent reference work and endpoint/ledger checks. They are Python rational diagnostic measurements, not game/device results. A single source triangle's 720-step refill case takes seconds in Python; no native speed extrapolation follows. Diagnostic work after count refusal is not admitted runtime work.

For a bounded implementation, reserve candidate scratch separately from the eight committed cohorts. A source step plus one resolved deposition can transiently split more intervals than it ultimately merges; an initial proposal is 96 scratch records per active fragment worker, with explicit refusal before overflow. Permit at most one deposition per fragment/barrier in that profile unless a larger counted recipe is adopted. Existing source/material/rain domains are prebound; no schema/UUID lookup or allocation per cohort. The experiment has not qualified this scratch bound for arbitrary sources.

Exact Fraction equality is not an f64 accuracy certificate. A float product needs source-coordinate/metric enclosures, nonnegative integrated density, complete height coverage, cap/root ordering, deterministic cancellation-aware reductions and independently derived local mass/field error. Store coefficients in local normalized cohort coordinates to avoid subtracting large global intercepts across narrow bands. Compute stable band moments rather than assuming subtraction of nearly equal cumulative moments is accurate. Do not epsilon-merge different fields or drop a thin cohort. Zero/near-zero stage speed, cap coefficient degeneracy, narrow intervals and exhausted error/count budget are explicit refusal/constant-case branches. Exact mass conservation alone cannot authorize an inaccurate density field.

## Narrow next proof queue

1. Review this restricted closure and count failures before any canonical adoption. Select its exact supported material/source profile, metric bridge and finite count/error budget explicitly.
2. Prove canonical true-area coefficients with one of the two declared metric routes and independent reference integrals. Qualify stable f64 moments/roots and a local field-plus-receipt error envelope, including known bad near-zero/near-cap cases.
3. If general deposition/variable weather remains required under eight cohorts, commission a **bounded, conservative approximation/merge proof** against the retained exact histories. Require local density and each exchange bound, not only total mass; otherwise reject or raise the charged physical-profile budget.
4. Only then couple prescribed/finite water, W2/P7/failure/retry in a complete headless transaction. Finite grants/pending stage effects, generic two-dimensional deposition, flat/shelf distributions, absorption and analytic-wave grazing are separate gates. None is inferred from this tide-only study.

## Evidence and source identity

This is original commissioned code and algebra. The previously inspected [Corcos, Saye and Sethian 2024 coating-flow paper](https://math.lbl.gov/~saye/1-s2.0-S0021999124002092-main.pdf), §§3.1.2–3.1.4, supports treating polynomial material fields, geometric quadrature and temporal transfer as separate design obligations. Its solver/physics is not imported and does not establish these game laws or costs. No broader literature sweep or third-party implementation is needed to decide this exact restricted count question.

Run `python3 -B tmp/moving-shore-cohort-proof/proof.py`, then `./dev check-docs`. The sibling manifest pins source, editable fixtures, semantic results, separate measurements and Python executable. The selected archive is 40,251 bytes, SHA-256 `3cb7fd405d8b05c5421f1836b6129fa29ac6ab0bb207d45ac853d00230da6d81`; all six members were byte-verified against the owned scratch and all manifest-listed hashes checked. Semantic results distinguish successful bounded fixtures from post-refusal diagnostics. No Broadsides file, engine module or canonical contract was changed by this commission.

Exact retained SHA-256 pins: `proof.py` `ad5047ea97d1d8a4644eeacff28a6c20a669cf6a5e6c2f7b8dbce1a55cd4f92b`; `fixtures.json` `629de08396c4c0c8a77f27b52424aa75442f976b62279eb73f5779b233978c4f`; semantic `results.json` `e80e451fe37737028a230a8c8ff9f0e5f66e5a24b8eebf021616706f075a0d85`. All eleven fragment-oracle comparisons, exact local balances, stated growth thresholds, remix rejection and equivalent-subdivision checks pass; manifest-listed file hashes were verified. These are experiment identities, not SourceRevision/BuildRevision or runtime qualification.
