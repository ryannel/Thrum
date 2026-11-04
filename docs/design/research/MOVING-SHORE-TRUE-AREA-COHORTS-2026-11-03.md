# Exact true-area bridge for tide-only cohorts — 3 November 2026

**Result: canonical true-area cap/refill can be carried exactly on the reviewed 1:20 bank, without changing horizontal rain units or losing the restricted cohort closure.** Candidate and independent space-first oracle agree on all eleven named fragment fixtures. Actual irrational cap-root orderings, exact-cap zero-speed and a near-cap deposit case pass. The two previous eight-cohort failures remain failures at the same times. This closes the metric bridge for the stated mathematical test law, not f64 runtime, general terrain, full material physics or C09 adoption.

Owned scope is this report and `tmp/moving-shore-true-area-cohort-proof/`; the selected six-file source/fixture/result/measurement set is retained in the [evidence archive](../../evidence/ARCHIVE-ACCESS.md#archive-inventory). Read the [projected-law study](MOVING-SHORE-DENSITY-COHORTS-2026-11-03.md), [fixed-column rejection](MOVING-SHORE-SPATIAL-ADMISSION-2026-11-02.md), [C09](../CONTRACTS.md#c09-environmental-fields-and-physical-exchange--e1-with-p3) and [moving-shore design](MOVING-SHORE-EXPOSURE-DESIGN-2026-11-02.md). No shared contract or engine code is edited. The proof derives from the author's prior exact candidate/oracle, whose source hash is recorded in the manifest; it imports no external implementation.

## Canonical parameter conversion

The actual source triangles have `h=x/20` and therefore true/projected area ratio `beta=sqrt(401)/20`. Retain the source-aligned projected mass-density field `q(h)=a+b h`, but author material quantities in their canonical **true-area** units:

* Cap `r_w=1/10 kg/(true m²)` becomes `C_projected=beta/10`.
* Refill `k_w=1/50 kg/(true m² s)` becomes `K_projected=beta/50`.
* Initial true density rho becomes `q_initial=beta*rho`.
* A resolved true-density increment d(h) becomes `beta*d(h)`; its donor receipt is its exact integrated mass.
* Rain P remains kg/(horizontal projected m² s), with no beta multiplier. Static direct-first-hit eligibility remains a distinct source binding.

Mass is `integral q dA_horizontal`; true local density is q/beta. Film/coating are integrated against the actual endpoint wet/dry triangle cuts. A floor does not gain a second rain footprint because its true area is positive. The cohort operator remains the previous restricted affine tide, constant-rate, cap/refill/deposition law; absorption, evaporation, drainage, waves and finite-water grants are not added.

The main adversarial cases now **enable** canonical refill 0.02, rather than the previous study's zero-refill isolation controls. Their physical refill receipts therefore change as well as the metric. The retained zero-refill growth case is explicitly a disabled-refill control outside the fully enabled canonical-refill profile. The dry-deposition failure uses canonical refill 0.02, but never contacts water, so refill is inactive by actual support rather than silently omitted.

## Exact arithmetic and branch ordering

The validator represents each scalar as `a+b sqrt(401)` with two exact Fractions. Addition, multiplication and division stay in that field; division uses the rational norm `a²-401b²`. Values serialize as explicit rational coefficient pairs, not rounded decimal approximations. Rational values and pairs with b=0 have the same canonical value encoding and compatible equality/hash behavior.

Ordering never evaluates sqrt in floating point. If a and b have the same sign, their sum's sign is immediate. For opposite signs, compare rational `a²` with `401b²` and choose the dominant term's sign. Because 401 is nonsquare, a nonzero rational pair cannot cancel exactly. Zero is recognized algebraically; no epsilon classification or root snapping is used.

The scalar validation separately brackets sqrt(401) between rational bounds by bisection and compares interval signs with the exact squared-comparison result. Twelve witnesses include values cancelling to more than 220 bits, their opposites, tiny signed values and inverses. Zero scalar division explicitly refuses. This independent check validates the shared arithmetic layer; it is not a second engine implementation.

The candidate still evolves height-band affine fields by the reviewed closed formulas and integrates clipped polygon area/height moments. The independent oracle still follows symbolic **point event histories**, discovers its own affine branch roots, and integrates over edge-derived source-triangle width. It consumes neither candidate cohorts nor its moment integrator. Both now carry field-valued coefficients and algebraic height roots. Every final density coefficient, film/coating mass and each solid-rain/direct-water/refill/release/deposition receipt agrees exactly.

Candidate coverage is checked from the source height endpoints through every adjacent interval; normalized fields cannot leave gaps. The mixed cap/deposition case retains an actual irrational interior-root ordering witness `(left < algebraic_root < right)`, not merely a counter of unused roots outside the source. Root discovery, clipping and coefficient ordering all use the exact field comparison. There is no unresolved-order fallback in these fixtures.

This is a **bounded validation tool**, not a generic algebraic engine: D is fixed to 401; constructor-held rational numerator/denominator lengths above 4096 bits refuse. Oracle height-branch discovery retains its finite 4096-cut bound. These are validator guards, not a whole-process memory guarantee: intermediate arithmetic, input parsing and Python object overhead are additional work. Other source slopes generally introduce other radicals and are outside this one-field proof.

## Named cases and exact comparisons

Let beta denote the exact metric above. Rain geometry/timing is unchanged by material-unit conversion, so solid-rain and direct-water-rain integrals remain the same horizontal source quantities for a matched source history.

| Case | Exact true-area result / comparison | Peak cohorts |
| --- | --- | ---: |
| Two-fragment rain, A wets and B stays dry | A coating `1/40 + beta/200 kg`; B film `1/20 kg`. The extra beta/200 is actual enabled refill; the old isolation control had K=0 | 1 each |
| Three fragments: A dries, later B wets, C stays dry | A film `19 beta/600 kg`, B coating `beta/600 kg`, C zero. Their initial mass and actual refill remain local | 1 each |
| Rising/falling/re-rising with cap and timed affine deposition | Final coating `beta/10 kg`; film zero. Horizontal rain totals remain `22/525` solid and `143/2100` direct-water kg; deposit is `beta*(202/9375) kg`. Refill/release are exact field-valued results in the artifact | 4 |
| Rising tide for 60 F1-sized test steps | Exact field density and all integrals match the point-history/width oracle | 3 |
| Zero-speed stage, initial true density exactly at cap, quiet rain | Coating `3 beta/40`; film `beta/40 + 1/86400`; refill/release zero. Water rain `1/28800 kg` | 2 |
| Near-cap resolved deposits, epsilon=2^-80 | Initial density `0.1-epsilon`; canonical refill fills the wet deficit before deposit. Two true-density deposits of epsilon/2 and 2epsilon over the wet height band yield refill `3 beta epsilon/4` and release `9 beta epsilon/8`, both strictly positive | 2 |
| Previous 1 Hz rain-change **zero-refill control** | Ninth cohort at step 361, t=361/60 s | 9, refuse |
| Previous resolved dry deposits, canonical K enabled but no wet contact | Ninth cohort at step 240, t=4 s | 9, refuse |

For the first two adversaries, a matched projected-basis law with refill enabled would give A coating 3/100, A film 19/600 and B coating 1/600 respectively. Replacing beta by one in the new exact expressions produces those matched-law values. It would be incorrect to attribute the entire difference from the earlier K=0 controls to area conversion.

The two failure traces intentionally stop after the first failing interval; later unrestricted 14/11-cohort diagnostics from the previous study are not rerun or claimed bounded here. Frozen local state/receipt prefix bytes remain unchanged after the rejection. The retained other fixture counts match the prior projected study's counts where an earlier matching count exists. Unit conversion has not been used to merge different functions or relax eight.

**Decision:** the metric bridge succeeds for this restricted mathematical law; the general eight-cohort guarantee still fails. In particular the dry-deposition failure remains within the fully enabled canonical cap/refill law. No broader source/deposition regime is selected as a consequence of this bridge.

## Near-cap, zero-speed and input precision limits

The zero-speed branch never divides by v; it classifies constant stage against each source-height band and applies the constant wet/dry law. A constant cap-comparison expression with zero height coefficient has no root to divide by. Exact equality chooses the declared cap branch without manufacturing release. Negative density, gaps or inconsistent local balances would fail before any successful result.

The epsilon=2^-80 case deliberately lies below binary64 resolution near 0.1. It tests exact algebraic branch decisions, not a promise that the current F3 f64 source path preserves those authored rational strings. A shipping certificate must bind the values **actually decoded**, account for their parameter error relative to the requested canonical constants, and reject an unresolved/unsupported source precision. Rounding this fixture to f64 and then inheriting its exact positive-release result would be invalid.

The arithmetic library's order proof is shared by candidate and oracle; its separate rational-interval witnesses reduce, but do not remove, the need for independent review. Flat/shelf triangles, source critical heights with positive-area equality, and more general deposition localization remain outside the current triangle-height distribution.

## Cost and what remains for f64 runtime

The eleven retained cases use about 100,000 exact field operations and 16,000 algebraic ordering comparisons in the measured run, excluding the scalar-validation prelude. The cumulative maximum rational size is 445 bits, including the deliberate near-cancellation scalar witnesses. Per-case counts/timings are in `measurements.json`; they include independent references, moment clipping and validation. The package takes seconds to validate on the recorded host; this establishes no engine tick or device budget.

Each field scalar stores two rational values, each with numerator and denominator. The previous 64-byte f64 cohort layout is **not** the cost of exact algebraic storage. Six field scalars (bounds, surface/absorbed coefficient slots) contain 24 arbitrary-size integers before metadata. At a hypothetical fixed 445-bit payload per integer that is about 1344 bytes/cohort, or 84 MiB for 65536 cohorts before object overhead; at the 4096-bit validator cap it would be 768 MiB. These are illustrative integer payload accounts, not measured peak RSS or a justified runtime format. Intermediate arithmetic can exceed stored scalar width.

The exact field implementation is therefore a reference/cook-validation candidate. A future f64/interval runtime needs: beta and parameter conversion enclosures; stable normalized local coefficients; ordered root intervals and explicit ambiguous-order refusal; complete positive cohort coverage; stable band moments; nonnegative field and mass enclosures; per-kind/destination exchange error; and propagation through retained/save state. It may not independently round cap/refill in ways that contradict their common metric. The source height, material field, ray/physics geometry and runtime products must share their pinned revisions.

A conservative f64-interval thought experiment storing value plus radius for the six scalar slots and 16 B flags costs about 112 B/cohort: **7 MiB/endpoint**, 21 MiB for two committed plus staged and **28 MiB** with a separate save owner, before products and receipts. A tighter shared error representation could reduce that only after proof. The exact cohort count/cost still multiplies 8192 source fragments by eight cohorts and all retained owners; canonical units do not reduce count pressure. Old/new geometry/metric products, temporary split scratch and any world-transition overlap need their own charges.

No such f64 product, root/refusal adapter, metric cookie, admission binding, P7 transaction or device measurement exists in this slice. The retained source continues after count refusal only as an explicitly labelled local diagnostic; a real host must keep the previous complete X[n], source interval and receipt cursors unadvanced.

## Evidence, checks and handoff

This bridge is original arithmetic and an adaptation of the author's own exact candidate/oracle, pinned to prior source `ad5047ea97d1d8a4644eeacff28a6c20a669cf6a5e6c2f7b8dbce1a55cd4f92b`. Its mathematical requirement is the affine metric identity and closed quadratic-field arithmetic, so no extra paper or implementation is attributed as proof. The primary-source context remains the [prior cohort report](MOVING-SHORE-DENSITY-COHORTS-2026-11-03.md#evidence-and-source-identity). No third-party code, Broadsides write or dependency addition is involved.

Run `python3 -B tmp/moving-shore-true-area-cohort-proof/proof.py`, then `./dev check-docs`. The sibling manifest pins the exact interpreter, host, fixtures, code, semantic results and separate measurement artifact. The selected archive is 19,054 bytes, SHA-256 `897c5e41a3ccc5aefd3eb2f378d78bb5d234d44cb861191c46b666311d04c9a5`; all six members were byte-verified against the owned scratch and all manifest-listed hashes checked. The next gate is an independently reviewed f64 certificate for the **same bounded true-area law**, while retaining the two demonstrated count refusals. Waves, finite water, absorption and a general algebraic engine are explicitly not commissioned or qualified by this study.

Exact retained SHA-256 pins: `proof.py` `7b58ed86b0822895a39200f5fa16141931f9511ef48983d2dae80f64757bfb00`; `fixtures.json` `fa0faeb156d7abf39a17e280203b7692e9c8802a64a856c9a45b604a7b828d82`; semantic `results.json` `683c5dc06c9632aeae631630fc42fc85099f2a75fa46c00b11c365b086ab3afc`. All eleven candidate/oracle comparisons, per-step local balances, field comparisons, scalar-order checks, near-cap equalities and named count thresholds pass. Manifest-listed files were rehashed after execution. These are validation experiment identities, not admitted SourceRevision/BuildRevision.
