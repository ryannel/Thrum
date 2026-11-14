# ATMOSPHERE design review

Reviewed commit `ea124871d9744fee4264447e8d4a99c6bcae4dac`, 14 November 2026.
Result: **design changes requested — three Must Fix findings**.

This judges the design and acceptance criteria, not an implementation or a visual result. The division of
responsibility answers the request: CLOUDS owns weather, condensate and haze amount; ATMOSPHERE owns air optics
and joint near-camera transport. The added milestones preserve the approved V1 exit. However, the two modules
do not yet agree on the Sun-path product, terminal composition is inconsistent, and the new numerical criteria
cannot establish acceptable correctness.

## Previous review

The review at `156912c341ff` had no Must Fix findings. Its two suggestions remain open: the half-float relative
precision/visibility wording, and the exposure negative control. Neither becomes a blocker here.

All six earlier Must Fix findings it tracked remain resolved in their original M5/M6 scope: product-specific
P/H acceptance, reflection handoff, per-sample cooked-volume attenuation, lookup support and generation semantics,
matched M6 cost comparisons, and wrong-lookup rejection. I compared the acceptance fields programmatically:
M5/M6 statements, checks, reviewer questions, inspections, targets and locks are unchanged. Their state/history
changes do not weaken their definitions. The new view-volume path requires its own assessment below; the older
approval does not qualify that replacement.

## Findings

### 1. Must Fix — the ground-band producer and consumer count upper cloud twice

**Verified interface contradiction.** Name: `ground-band-double-counts-upper-cloud`.

[ATMOSPHERE DESIGN](../../plans/ATMOSPHERE/DESIGN.md#mechanism-1), lines 615–619, and
[done-when M8.3](../../plans/ATMOSPHERE/DONE.json), lines 378–385, add the ground band's optical depth to the
2D cloud band's. Architecture ruling 7 says the same. But
[CLOUDS DESIGN](../../plans/CLOUDS/DESIGN.md#condensate-that-meets-the-ground-mist-fog-and-low-cloud), lines
702–710, fills the ground band by marching to its exit **plus the cloud band's value there**, and explicitly
says a point reads the ground band below its top or the cloud band above. CLOUDS M14.3 then asks its consumer
to sum both again.

An ordinary counterexample is an upper cloud with optical depth `ln(2)` and no local fog at a ground-band
receiver. CLOUDS supplies `ln(2)` in the ground band. The specified ATMOSPHERE consumer yields
`exp(-(ln(2) + ln(2))) = 0.25`; the correct factor is `0.5`. I evaluated those expressions independently.
A bank makes the same duplication harder to notice, rather than removing it. This breaks C08's once-only
attenuation and makes the air, surfaces and provider's solar kernel disagree.

Resolve the stored band's meaning and lookup rule jointly across the producer, consumer, architecture ruling
and affected items. A ground-band receiver with upper cloud and zero local condensate must receive that cloud's
attenuation once; adding local condensate must add only its own path contribution. M8 can prove this with its
stand-in before CLOUDS M14 exists. Neither a prettier fog capture nor a test of two arbitrarily chosen additive
textures resolves the interface mismatch.

### 2. Must Fix — the terminal correction does not cover the light paths it claims

**Verified algebra and existing draw semantics.** Name: `terminal-correction-source-ownership`.

[DESIGN](../../plans/ATMOSPHERE/DESIGN.md#mechanism-1), lines 610–613, says sky, Sun disc and stars become
`R*L + deltaS`, with `R=1, deltaS=0` whenever no medium provider contributes. Lines 629–631 derive
`deltaS = S_joint - R*S_air`. Two normal cases contradict the blanket rule:

- M7 shadows the air's single scattering through cascades and cloud optical depth (lines 596–603), even with
  no condensate coefficients. Its joint source then differs from the clear-air source used by the existing
  sky. For `R=1`, `S_air=2`, `S_joint=1`, the identity requires `deltaS=-1`, not zero. Keeping the prescribed
  identity correction leaves the sky with unshadowed haze while distant geometry reads shadowed haze. The
  design has no separately specified shadow-aware sky baseline that would make the zero claim true.
- Stars are additive radiance contributions, as the retained design says at lines 90–97 and the current
  [star pipeline](../../../crates/thrum-render/src/synthetic/stars.rs), lines 83–87, implements. Adding
  `deltaS` to both the sky and each star adds the same medium source repeatedly. With `R=0.5`, `deltaS=0.5`,
  clear-air sky `4.4` and star contribution `4`, one correction of their sum gives `4.7`; correcting each
  separately gives `5.2`. I checked this arithmetic. The additive star needs attenuation without another
  copy of the background's source term. M8.2 currently repeats the same ambiguous per-source rule.

Specify which complete background receives the source correction once, which emitter contributions receive
only transmission, and how shadowed air reaches sky pixels when no condensate is present. Define the clear-air
baseline used in the subtraction consistently with that composition. Acceptance must distinguish a combined
sky-plus-emitter pixel from separately corrected draws and cover shadowed haze without condensate. This is a
transport ownership correction, not a request for a different fog technique.

### 3. Must Fix — observed error alone defines success for the new numerical transport

**Verified acceptance gap.** Name: `view-volume-self-selected-correctness-bound`.

[Done-when M7.3 and M8.1](../../plans/ATMOSPHERE/DONE.json), lines 235–242 and 354–361, require agreement
within a bound the milestone first measures and then asserts. The plan repeats that instruction. There is no
acceptable ceiling, error model, required relationship to the old volume's accuracy, or decision that rejects
an excessive measured error. Recording the old volume beside it is not such a decision.

For example, a new solar integrator with a systematic gain error can be compared with the independent f64
reference, have its maximum error recorded as the bound, and pass the same probes thereafter. P5 constrains
only its night part. The visual items do not prove the lent solar quantities. M8.1's requirement that one wrong
separate composition fail is useful, but a biased joint result whose error is smaller than that deliberately
wrong result can still set its own passing bound. Doubling the slices has no required convergence or accuracy
relationship either. An independent reference does not supply acceptance when any measured discrepancy is
acceptable.

Give the reviewer a correctness decision independent of the candidate's own maximum error. That can be a
justified tolerance for the declared approximation or a bounded qualification step that returns to planning
when the intended accuracy is not attainable. It need not open an all-orders physical-accuracy campaign or
invent a performance budget. Keep the independent references, preserved P bounds and wrong-composition control;
make excessive error capable of failing the new items.

### Suggestion — qualify the handover evidence for an actual bank at the proposed distance

**Verified limitation of the cited experiment.** The new design relies on
[CLOUDS' handover proof](../CLOUDS/design-proof/whole_atmosphere_proof.py), whose part I integrates a uniform medium from the camera and
centres its ramps at 1,500 m. The proposed first ramp is 300–500 m, and a fog bank can begin there. Foreground
attenuation in the existing experiment suppresses the contribution of the overlap, especially for dense fog.

I independently integrated a bank occupying 300–500 m, with visual range 400 m, smoothstep near/far weights
across that interval, and source per unit extinction changing from 1 to 0.1 at 400 m. Using exact homogeneous
steps of 0.01 m, the split pair gives `S=0.63598849`; joint integration gives `0.64740148`, an absolute difference
of `0.01141299`. Transmission agrees at `0.14142298`. This does not invalidate the transmission identity cited
by ATMOSPHERE. It does show why the quoted small in-scatter errors are measurements of those earlier cases,
not a bound for ordinary banks. CLOUDS M13/M15 already name a bank crossing the ramp; retain that challenge
and qualify the design's accuracy claim accordingly. This review does not demand exact far-cloud integration.

### Suggestions carried from the previous review

**Verified, unchanged wording.** DESIGN lines 244–247 and 301–305 and PLAN's precision assumption still describe
a half-float step as universally at most 0.1% and dismiss interpolation's accuracy/visibility benefit. Keep `Q`
as an absolute stored-spacing allowance, qualify the percentage, and leave visibility to the visual review.
DESIGN lines 410–412 still allow any exposure scale other than one: choose a nonzero case whose deliberately
wrong scaling exceeds the bound. These remain suggestions, as in the prior review.

## Request, mechanisms and proof order

I read all five complete planner briefs in RECORD.json, oldest first, and the approved project brief. The
latest owner words ask to include mist/fog and plan for top-quality atmosphere. The accompanying lead task
names ground fog, valley mist, water banks, enterable low cloud, consistent lighting and weather/time changes.
The brief preserves sea/cloud work outside the V1 exit. M7/M8 obey that boundary; CLOUDS owns the actual
weather/density behaviours, while ATMOSPHERE first proves their transport with a labelled stand-in. Local
lights, indoor fog and second-view volumes are explicitly excluded; local lights are also presented as an
owner-facing assumption. They are not silently claimed as delivered.

The inspection list was: revisit previous findings; compare every design addition and all new items; trace
haze and medium coefficients through both module designs; walk a receiver-to-Sun path and a camera-to-sky
path; challenge terminal and handover equations; assess whether each milestone can prove its claims without
future CLOUDS work; and distinguish correctness, visual and device evidence.

The frustum grid is a supported technique choice. The primary
[Frostbite presentation](https://www.slideshare.net/DICEStudio/physically-based-and-unified-volumetric-rendering-in-frostbite)
describes media properties, lighting and final integration as separate stages.
[Epic's volumetric-fog documentation](https://dev.epicgames.com/documentation/en-us/unreal-engine/volumetric-fog-in-unreal-engine)
confirms the camera-aligned volume, finite slice resolution and temporal-history limitations. Those precedents
support investigating this mechanism; their measured costs do not qualify Thrum's much longer domain or history.
The planned paired cost and motion reviews remain necessary.

M7 follows M6, which follows APPLE-PRESENTATION M3; M8 follows M7 in module order. The diagnostic field already
exists and the test bank is built in M8, so neither milestone needs CLOUDS M14 to run. Later handover and real
weather quality belong to CLOUDS. The changed assumptions and ownership are stated in the plan and architecture.

## Assessment of every added item

These are design assessments, not milestone verdicts.

| Item | Assessment |
|---|---|
| M7.1 | Judgeable: recorded starting commit and both-target off parity cover unchanged scenes. |
| M7.2 | Judgeable: default bytes, independent P1/P2 bounds at changed amounts, declared lag and epoch semantics. |
| M7.3 | Must Fix 3: the measured discrepancy supplies its own acceptance bound. |
| M7.4 | Judgeable for cells and surfaces; Must Fix 2 concerns the missing terminal consequence. |
| M7.5 | Judgeable: submitted-history progression, reset completeness and runtime creation are observable. |
| M7.6 | Judgeable visual range/horizon/gradient comparison; terminal design must first be corrected. |
| M7.7 | Judgeable: geometry and diagnostic cloud gaps expose aligned shafts. |
| M7.8 | Judgeable: elapsed-time pan, walk, scrub and first cut frame. |
| M7.9 | Judgeable by its statement inheriting all three lists; its shorter inspection mentions only the pan. Follow the full statement, including walk, scrub and cut. |
| M7.10 | Judgeable: paired on/off costs, resource bytes, grid choices and investigations on both targets. |
| M7.11 | Judgeable: separate maturity claims and explicit exclusions. |
| M7.12 | Judgeable: retained-baseline verifier preserves existing scenes. |
| M8.1 | Must Fix 3; joint-reference and wrong-composition checks are otherwise useful. |
| M8.2 | Must Fix 2: define terminal baseline and once-only source ownership before judging its equality. |
| M8.3 | Must Fix 1: its sum disagrees with the intended provider's stored quantity. |
| M8.4 | Judgeable near-share support/continuation; does not itself prove full near/far composition, which CLOUDS later owns. |
| M8.5 | Judgeable: bank, receivers, Sun, direction-dependent light and terrain boundaries at named views. |
| M8.6 | Judgeable: entry, pan, exit and moving-edge history/exposure. |
| M8.7 | Judgeable by both inherited lists; its abbreviated inspection must not omit the pan or mover. |
| M8.8 | Judgeable: per-pipeline capability counts, corroborated by the real Chrome scene used elsewhere. |
| M8.9 | Judgeable: paired medium costs and named pass times on both targets. |
| M8.10 | Judgeable: the diagnostic medium must remain labelled as a stand-in. |
| M8.11 | Judgeable: retained-baseline preservation. |

## Evidence and limits

Read the complete current ATMOSPHERE design and plan, every new M7/M8 item, unchanged M6 definitions and the
M5/M6 acceptance-field diff, relevant architecture interfaces/build order, C08, CLOUDS producer/compositor
sections and dependent items, and the cited handover proof's source/output. Also read README, ORCHESTRATE,
Thrum's review rules and both graded-example sets; no plans glossary exists. The prompting-agents skill was
used to write actionable findings. No agents were delegated the reading or judgement.

The counterexamples above are CPU arithmetic and document/interface checks. No GPU run, capture, native-size
image inspection or motion review was performed. No prior implementation, visual or performance result is
newly qualified. Only this review file is changed, left uncommitted for the review script; DONE.json and all
plans remain untouched.

Validation: `./dev check-docs` passed (844 Markdown files, 6,102 local links, 30 module records,
crate policy and current design hashes). `git diff --check` passed.

DESIGN CHANGES: 3 must_fix findings
