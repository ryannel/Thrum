# ATMOSPHERE design review

Reviewed integrated commit `27473469f1715ebfc77cdd4a6cd24137a29b1415`, 10 November 2026.
Result: **changes requested — one blocking finding**.

This is the second design review. The previous round is this file at
`4bbe44a433598650be47c1753c57da79f5d1df93`. No other agents were launched. This review judges the
M5–M6 design and acceptance definitions, not implementation or completion of a milestone.

## Request and inspection scope

Read both entries in [the accepted planner briefs](../../plans/ATMOSPHERE/RECORD.json), whole
and in order: the project commission to plan the remaining V1 road and finish/consolidate water and clouds,
then the request to revise ATMOSPHERE against the previous review. The governing module obligations remain:

- One shared sky, true Sun, light-transmittance and environment-reflection handoff usable by independent
  consumers on Metal and hardware Chrome WebGPU.
- Cloud optical depth applied once to direct sunlight, consistent across receivers, with absent providers
  adding no avoidable rendering work.
- A reviewable close of sky, sunlight, aerial perspective and time controls on the V1 route, keeping
  correctness, visuals and device measurements distinct.

Inspection list: resolve each previous Must Fix;
check every changed item's reference, quantity, cases and bounds; trace the reflection and cooked-volume
consumers; challenge the new handoff checks with incorrect lookups; retain both targets, off-parity controls,
the held sunset stop and Ground E pan; distinguish fresh paired measurements from historical records.

The [build-plan exit](../../plans/BRIEF.md), architecture brief, module list, C08 and the SEA/CLOUDS briefs
support this scope. The diagnostic provider remains justified foundation work. The explicitly recorded
deferral of broad solar transport accuracy is consistent with the previous review's requested distinction
between preservation and new qualification; this review does not demand an all-orders accuracy campaign.

## Previous findings

| Previous finding | Disposition at this revision |
|---|---|
| 1. Must Fix: undefined numerical acceptance | **Partly resolved; still blocking for the new handoff range rule below.** P1–P11 now name scoped references, cases and bounds; H1–H5 name both-target handoff obligations; the all-orders solar report is explicitly not an accuracy pass. M6.3/M6.4 separate Metal and Chrome, and M6.6 records the unqualified products. However, H4 can accept an incorrect transmittance lookup at every query. |
| 2. Must Fix: independent-consumer proof omits reflection | **Resolved.** The fifth operation includes GGX-filtered radiance and the split-sum pair, with private mip/gain conventions, roughness and view-cosine support. H5 and M5.2/M5.3 require independent-pipeline evaluations, direction/roughness/source changes, sky rewrite and a separate absolute night term. |
| 3. Suggestion: explicit cooked-volume migration | **Resolved.** The design and plan name the existing march's directional-light input, a shared factor evaluated at each lit world-space sample, unchanged no-provider lighting, and its per-step cost. M5.4 and M5.6 add shader preservation and a spatially varying witness. |
| 4. Suggestion: support and generation semantics | **Resolved.** The operations table states observer/view limits and the 32 km clamp; non-camera-start segments are explicitly not offered. The cloud field declares its plane, region, valid elevation and identity fallback. Resource/model replacement is distinguished from time-driven chart updates. |
| 5. Suggestion: fresh paired costs | **Resolved.** New M6.5 compares the final build with a fresh run of the milestone's starting commit under matched configuration and alternated order. M1/M2 figures remain history. |

## Findings, worst first

### 1. Must Fix — the new texel-range rule accepts a wrong aerial lookup

**Verified by the acceptance predicate and the existing fixture; not a claim of a current runtime defect.**
[The revised numerical table](../../plans/ATMOSPHERE/DESIGN.md#numerical-acceptance), lines 333–340,
makes H2/H4 accept any result between the minimum and maximum of the relevant texels, widened by a storage
step. It then claims wrong mappings fail. Being inside that interval is necessary for interpolation, but does
not establish the requested interpolation at the requested point.

H4 has a concrete substitute that passes its transmittance checks: return the lower depth slice's `T` instead
of blending the two slices. Every channel remains inside the allowed texel range, at every sampled distance.
The existing [P10 fixed-texel fixture](../../../crates/thrum-render/src/atmosphere/gpu/tests/aerial.rs),
lines 377–395, writes `T = 0.5` into every layer, so this substitute also passes its exact decode check.
P5 checks night in-scatter, not transmittance; leaving `S` correct preserves that check. The preserved producer
referees and unchanged forward path do not test a faulty new contribution-facing result. In contrast,
[the production sampler](../../../crates/thrum-render/src/synthetic_aerial.wgsl), lines 21–37, explicitly
interpolates `T` using physical distance between the two depth slices.

A small CPU arithmetic check reproduced the predicate's hole without changing repository files. For adjacent
values `0.875` and `0.625`, blend weights `0.25`, `0.5`, `0.75` give `0.8125`, `0.75`, `0.6875`.
Returning `0.875` passes the range rule in all three cases. With the P10 fixture's equal endpoints, both
implementations return `0.5`. These are illustrative filter values, not measured atmosphere error. The
counterexample applies to any unequal endpoint values. H2's range predicate has the same inability to distinguish
filter weights, though its separate P3/P11 rows constrain some results.

**Consequence:** M5.2/M5.3, and their final-build repeats M6.3/M6.4, can qualify a handoff whose aerial
transmission changes in depth steps rather than matching the existing product. The first independent surface
consumer can disagree with the scene even though the old forward rendering and preserved referees pass.
This is the convincing-substitute problem in the revised answer to previous finding 1, not a request for
stronger physical accuracy or an additional domain.

**Resolve before building:** make the handoff acceptance distinguish correct interpolation/addressing from
this substitute on both targets. For example, compare with an independently evaluated filtered decode of the
read-back texels at the requested coordinates, with a declared allowance for storage/arithmetic/filter
precision; or add a nonconstant known-field fixture with interior queries and bounded expected values.
Include a negative control that returns one contributing slice/texel and must fail. Apply the correction to
both H2 and H4's range-based claims. The existing producer approximations and physical-accuracy exclusions
can stay unchanged.

### 2. Suggestion — correct the handoff's pre-exposure wording

**Verified documentation inconsistency.** The new text in [the lent-products section](../../plans/ATMOSPHERE/DESIGN.md#mechanism),
lines 225–230, says the existing functions return values “pre-exposed like the forward pass's.” Those
lighting functions actually return physical scene-linear quantities. The forward
[shade function](../../../crates/thrum-render/src/synthetic.wgsl), lines 539–550, combines direct, indirect,
emissive and aerial radiance before multiplying by `scene_storage_scale()`. The
[reflection reader](../../../crates/thrum-render/src/synthetic_environment.wgsl), lines 23–27, explicitly
leaves pre-exposure to its caller. Transmittance and the split-sum pair are dimensionless.

The operation table and physical-unit referees already indicate the intended convention, so this is a
wording correction rather than a separate blocker. State that the lent lighting values are decoded,
un-pre-exposed scene-linear quantities and identify where a custom consumer applies the lent exposure scale
once. A non-unit exposure control would make the convention especially clear.

## Item-by-item design assessment

These are assessments of future judgeability, not implementation verdicts. Each item named as changed in the
review request was read against the previous round. Renumbered M6 items were compared by their meaning as
well as their ID.

| Item | Assessment |
|---|---|
| M5.1 changed | Judgeable; replacement versus recompute is now explicit, with profile/device/time-scrub controls retained. |
| M5.2 changed | **Blocked by finding 1.** H1/H3/H5 have scoped references, reflection coverage is repaired, but H4 can accept the wrong lookup. See also the exposure wording suggestion. |
| M5.3 changed | **Blocked by finding 1.** Real hardware Chrome, independent expected values and failure on unresolved rows are improvements; they still execute the same insufficient predicate. |
| M5.4 changed | Judgeable; the new cooked-volume shader parity requirement strengthens the no-provider proof. Frozen base, common cooked store, exact bytes, repeat and negative controls remain. |
| M5.5 unchanged | Judgeable; retains Chrome off parity in one browser session. |
| M5.6 changed | Judgeable; direct-term isolation, spatially varying world points, per-sample volume coverage and support fallbacks reject the previously noted omissions. |
| M5.7 unchanged | Judgeable; world drift, cross-receiver boundaries, retained sky light and geometric-shadow overlap remain observable. |
| M5.8 unchanged | Judgeable; retains the same visual witnesses in Chrome. |
| M5.9 changed | Judgeable; now a clearly Mac-specific paired cost and texture-limit row. No numeric performance gate is introduced. |
| M5.10 added | Judgeable; explicitly executes Chrome paired whole-frame measurements and validates every pipeline against browser adapter limits. This restores the separate target coverage previously buried in M5.9. |
| M6.1 changed | Judgeable; preserves the held 17.4 h stop and Ground E terrain lip while narrowly excluding the parked moving spur edge. TAAU, native crops and 1x motion remain required. |
| M6.2 unchanged | Judgeable; carries the same route, scrub and defect list into Chrome. |
| M6.3 changed | **Blocked by finding 1 for H2/H4.** Preservation of P1–P11 and reporting of all 198 solar rows are now honest, bounded obligations rather than a general accuracy claim. |
| M6.4 changed | **Blocked by finding 1.** This is now an explicit final-build Chrome H1–H5 check, not the old cost item. |
| M6.5 changed | Judgeable; the old cost obligation moves here with a fresh matched comparator and hitch investigation. |
| M6.6 added | Judgeable; carries the old ledger obligation and explicitly adds unqualified solar sky/aerial accuracy. It keeps physical cadence and iPhone claims separate. |

## Evidence and limits

Read the requested review skill, ORCHESTRATE settings, generic and project graded examples, Thrum review and
performance guidance, both verbatim planner briefs, the prior review, the full current design and done-when
file, and all changes to the design/plan/items since `4bbe44a43359`. Read relevant historical plan constraints,
the project brief/exit and module interfaces, C08, the port decision and SEA/CLOUDS consumer briefs. No
module-folder BRIEF or GLOSSARY is present.

Source inspection covered the contribution seam, forward lighting/aerial/reflection readers, cooked-volume
march and the relevant Sun, night, aerial, diffuse, reflection and twilight referees. This was a scoped
design/interface review, not a whole-module code audit or a re-review of M1–M4. No GPU execution, visual
captures or playback were needed for these findings; future proof scripts being unimplemented is not a
design failure. Only this review file was changed. No definitions, implementation or milestone state were
edited, and no commit was made.

Validation: `./dev check-docs` passed (619 Markdown files, 2,845 local links, 30 module records,
crate policy and current design hashes). `git diff --check` passed. `git status --short` lists only this
review file as changed.

DESIGN CHANGES: 1 blocking findings
