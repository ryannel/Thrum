# ATMOSPHERE design review

Reviewed integrated commit `631e222b6a5ef4c7ca3fed88d85a2da247f54a0b`, 11 November 2026.
Result: **design approved — no blocking findings**.

This is the third design review. The previous round is this file at
`27473469f1715ebfc77cdd4a6cd24137a29b1415`. This review judges the revised design and future acceptance
criteria, not implementation, visual quality or completion of M5/M6. No other agents were launched.

## Request and inspection scope

Read all three entries in [the accepted planner briefs](../../plans/ATMOSPHERE/RECORD.json),
whole and oldest first: the project commission to plan the remaining V1 road and consolidate water and
clouds, followed by both requests to revise ATMOSPHERE against its design review. For this module that means
one shared sky, true Sun, light-transmittance and reflection handoff for independent consumers; cloud optical
depth applied once to direct sunlight; no avoidable work with the provider absent; and a reviewable close of
the V1 sky, sunlight, aerial perspective and time controls on Metal and hardware Chrome WebGPU.

The inspection list was: resolve the previous Must Fix first; read both changed items and all accompanying
design/plan changes; challenge the replacement predicate with the previous wrong-slice substitute and with
wrong screen/chart filtering; check reference independence, units and target coverage; trace the corrected
requirements into the unchanged M6 repeats; check for weakened preservation, visual or cost obligations.

Read the project purpose and V1 exit, architecture brief requirements, module interface/readiness records,
C08, the M5/M6 design and plan, and the complete current done-when file. No module-folder BRIEF or GLOSSARY
is present. This is a scoped follow-up to the previous review, not a reopening of M1–M4 or the separate
sea/cloud designs.

## Previous findings

| Previous finding | Disposition |
|---|---|
| 1. Must Fix: the texel-range rule accepts a wrong aerial lookup | **Resolved.** H2 and H4 now compare with an independently evaluated filtered decode of read-back texels, at the requested coordinates, using an explicit bound. Both live products and varying known fields are required. Single-texel, single-row/column and single-slice substitutes must fail on both targets. The former constant-transmittance fixture is retained only as one preservation/decode check, not as the sole interpolation witness. |
| 2. Suggestion: correct pre-exposure wording | **Resolved.** The lent-products section now returns physical, un-pre-exposed scene-linear quantities, identifies dimensionless values, and applies the lent scale once after lighting and aerial composition when writing the scene target. Each H row gains a non-unit exposure control. See the small proof-strengthening suggestion below. |

Earlier resolved findings remain resolved: all five operations include reflection; cooked volumes use the
shared cloud factor at each lit march sample; support and generation semantics remain explicit; final costs
use a fresh paired starting-commit run. This revision does not narrow those requirements.

## Why the replacement rule closes the blocker

The [filtered-decode rule](../../plans/ATMOSPHERE/DESIGN.md#numerical-acceptance), lines 344–379,
defines a single expected value and the per-channel allowance `C/32 + 1e-5 × M`. The expected evaluation is
written independently of the lent library. The sky reference respects each horizon half; the aerial reference
blends in screen position and then by physical distance between the selected depth slices, with the 32 km
clamp. That agrees with the existing [aerial sampler](../../../crates/thrum-render/src/synthetic_aerial.wgsl),
lines 21–37: square-root distance selects the slice pair, while the final depth weight uses physical distance.
The sky mapping's separated horizon rows are visible in
[common.wgsl](../../../crates/thrum-render/src/atmosphere/common.wgsl), lines 80–107.

I reran the previous arithmetic counterexample in Python without writing files. Endpoints `0.875` and
`0.625` have a new allowance of `0.00782125`. At weights `0.25`, `0.5` and `0.75`, returning the lower
slice gives errors `0.0625`, `0.125` and `0.1875`; all now fail. The existing P10 fixture still stores the
same transmission in every layer, but the new known-field arm expressly requires unequal neighbours along
every blended axis, so that fixture alone cannot qualify H4.

I also checked that the requested negative controls can be discriminating together. On a unit texel cell,
use per-channel fields `1 + c·p`, with sky coefficient vectors `(1,1)`, `(1,2)`, `(2,1)` and aerial vectors
`(1,1,1)`, `(1,2,3)`, `(3,1,2)`. Enumerating all quarter/half/three-quarter weight combinations and the three
named wrong lookups gives 27 sky and 81 aerial comparisons. Every wrong result exceeds the bound in at least
one channel; the smallest rejecting-channel error/bound ratio is over 2.665. Nearest-coordinate ties were
rounded upward. Scaling all values by a common positive factor preserves that ratio, so transmission fixtures
can stay within `[0,1]`. This establishes arithmetic feasibility, not a chosen fixture or a GPU qualification.

The tolerance's sampler-precision premise remains an explicitly declared assumption in the plan log. Two
bilinear sampler axes explain the `C/32` allowance for per-axis weight errors of at most `1/64`; aerial depth
interpolation is explicit arithmetic. Actual mapping/arithmetic/filter behaviour must still pass on each
named target. A miss requires measurements and a plan change, not a builder widening the bound. The new rule
qualifies the handoff and decode, not independent physical accuracy of the stored solar transport.

## Changed-item assessment

| Item | Design assessment |
|---|---|
| M5.2 | **Judgeable; prior blocker resolved.** Its statement and reviewer question require H1–H5 through the independent contribution pipeline, independent f64 expected filtering on live and known fields, rejection of the wrong lookups, and exposure controls. P1–P11 retain their references, cases and bounds. |
| M5.3 | **Judgeable; prior blocker resolved.** Hardware Chrome runs the same rows and wrong-lookup controls, computes acceptance against the references rather than Metal output, and fails missing or unresolved rows. The plan explicitly carries the exposure control into Chrome too. |
| M6.3/M6.4, unchanged | Their final-build H1–H5 requirements inherit the revised H2/H4 rules, including negative controls. No separate edit is needed to carry the correction forward. |

The other items are unchanged. Reading them alongside the revision found no new conflict: off parity,
per-sample cloud attenuation, both-target visuals, held sunset and Ground E coverage, cost observations and
maturity exclusions are retained.

## Findings

### Suggestion — make the exposure control visibly discriminating to the numerical check

**Verified wording weakness, not a blocker.** The exposure control in the design, lines 377–379, allows any
scale other than 1. A scale arbitrarily close to 1 can leave an incorrectly scaled result inside the numerical
allowance, and a zero-valued reference cannot detect scaling. Choose a nonzero lighting case and a scale far
enough from 1 that applying it inside the lookup necessarily fails the row's bound, for example a suitable
case at scale 2. This sharpens the added control; the corrected units and existing physical-reference checks
already settle the prior documentation inconsistency.

## Evidence and limits

Read the requested review skill, ORCHESTRATE settings, generic and project graded examples, and Thrum's review
rules. Compared every changed line in ATMOSPHERE's design, plan and done-when file since `27473469f171`.
Source inspection was limited to the affected sky mapping, aerial sampler and P10 fixture, reflection reader,
and the forward pass's final exposure application. No GPU runs, images or motion were judged; those remain
implementation proof obligations. No capture folder was needed.

Validation: `./dev check-docs` passed (623 Markdown files, 2,892 local links, 30 module records,
crate policy and current design hashes). `git diff --check` passed. Only this review file changed; no
implementation, acceptance definitions or milestone state were edited, and no commit was made.

DESIGN OK
