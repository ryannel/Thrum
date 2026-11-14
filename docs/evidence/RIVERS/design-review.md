# RIVERS design review

**Design changes requested: 3 Must Fix findings.** The design follows the requested one-water architecture,
but its dry-cell encoding and interaction-current rule cannot deliver two of its promised behaviours on ordinary
inputs. One acceptance item also contradicts the existing join contract.

Reviewed on 14 November 2026 at commit `51210c3771e938675bac99e66c00aca89635298c`.
This is a design review, not an implementation, visual-quality or device-performance qualification.
There was no previous RIVERS design review to reconcile.

## Must Fix findings

### 1. Dry samples can create water in disconnected hollows

**Must Fix — verified by the written rules and arithmetic reproduction.**
Affected: R1 and R6; M1.1, M1.6 and M3.2.

The chart cannot preserve the solver's wet/dry answer as designed. In
`docs/plans/RIVERS/DESIGN.md:232`, a dry cell takes the nearest wet cell's level, with the claim that this
puts the level below its bed. That implication is false for the disconnected hollow expressly required by
M1.1. The common footprint is the highest sample's wet cells plus a margin (`DESIGN.md:238`), so exclusion
from the low sample's footprint cannot repair it.

Consider a main channel at level 1.0 m beside a hollow whose bed is 0.2 m, separated by a ridge at 1.1 m.
The low-flow solve leaves the hollow dry. A high sample at 1.2 m overtops the ridge and wets the hollow,
putting it in the common footprint; these samples are only 0.2 m apart, within the proposed spacing.
At the low sample, copying the nearest wet level gives the hollow a level of 1.0 m and therefore **0.8 m
of invented water**, while its separating ridge remains dry. WATERS identifies chart water by level above
bed inside the footprint (`docs/plans/WATERS/DESIGN.md:441`). Both the rendered surface and the query would
report that invented water. This fails even at an exact sample, before interpolation error is considered.

The [reproduction](design-review/counterexamples.py) and [output](design-review/counterexamples.txt) implement
these written rules. Each section of the example can be widened to meet the four-cell channel requirement.
The supplied section-only gauge proof does not exercise changing connectivity.

**To resolve:** define an encoding and sample-selection rule that preserves dry support, including low hollows
inside the high sample's footprint. Make the solver-to-map and gauge proofs judge that support as well as
depth/current in already-wet cells. Show that the hollow stays dry until its connection wets, on the CPU and
through the shared GPU map. The planner should select the mechanism; this review does not prescribe one.

### 2. One weighted current still misplaces concurrent rings and wakes

**Must Fix — verified algebraically; not a GPU measurement.**
Affected: R8, the one-water composition, ruling 5 and M6.3.

`docs/plans/RIVERS/DESIGN.md:381` selects one energy/source-weighted current per region and says it carries
pool rings at the pool's current and a riffle wake at the riffle's. A pool connected to a riffle is one
region: WATERS splits regions by connected wet cells, not by current
(`docs/plans/WATERS/DESIGN.md:869`). One translation cannot satisfy both motions when both sources are active.

Use the cue's own currents: 0.2 m/s in the pool and 1.5 m/s in the riffle. Two spatially separated wave
packets with equal weights select 0.85 m/s. Over five seconds, each centre is displaced **3.25 m from its
own water's translation**, exceeding M6.3's 1.5 m pool tolerance. More generally their desired translations
differ by 6.5 m; no common translation, even one that changes over time, can put both within 1.5 m.
Increasing a riffle source's strength also changes the carriage of an existing pool packet before the packets
meet. The [same reproduction](design-review/counterexamples.py) records this calculation.

The design acknowledges that a packet spreading into another current uses an average. This case is different:
two separate packets still in their respective waters already interfere through the global weight. A fall's
continuous impulses and a float's wake make coexistence an ordinary use of this cohesive water system.
Part 7 of the supplied proof only substitutes the pool current after assuming all wave energy is in the pool;
it does not evolve waves or test coexistence. An implementation could pass that isolated-source demonstration
and M6.3's isolated stone case while retaining this defect.

**To resolve:** settle a carriage mechanism or a supported, bounded approximation that handles concurrent
sources in different currents. Judge the pool-centre requirement with another active source in the same wet
region, and establish what happens to both packets. Keep the approximation explicit, but do not count the
single-source arithmetic as proof that the combined water behaves as claimed.

### 3. M1.6 demands solver equality inside joins that deliberately change the row

**Must Fix — verified contract contradiction.**
Affected: M1.6 (`docs/plans/RIVERS/DONE.json:66`).

M1.6 requires that “the map's level and current at every wet cell's centre are the solver's”, while also
requiring the row to join the basin continuously through its stage range. WATERS mixes a chart's level and
current toward the receiving body's row over a nonzero join length, and increases that weight under backwater
(`docs/plans/WATERS/DESIGN.md:458`). RIVERS explicitly retains this mechanism and solves against the mouth's
lowest level (`docs/plans/RIVERS/DESIGN.md:175`).

For a solver current of 0.5 m/s, a basin current of zero and a wet cell at join weight 0.5, the map current
is 0.25 m/s. It cannot also be 0.5 m/s. At full backwater weight the difference is larger. The numerical
example is included in the [output](design-review/counterexamples.txt). This is an impossible acceptance
condition, not a request to replace the accepted WATERS join approximation.

**To resolve:** distinguish equality of the stored/raw chart to the solve from equality of the final map
answer to the specified joined row. Scope solver equality to the unblended interior and judge the join and
backwater against their actual contract, including both receiver stages. A correct implementation must be
able to pass without weakening WATERS or silently skipping joined cells.

## Suggestion

**Suggestion — verified coverage gap, not a build blocker:** the declared fall range is 0.75–12 m, viewed
from 2–200 m, but M6's visual inspection concentrates on the island's roughly 4 m fall at 5–60 m.
The analytical checks cover several discharges and heights; the sheet's visual mechanism is explicitly unrun.
Include representative short/long falls and near/far views when first judging the sheet, or state which
part of the advertised range remains unqualified. This sharpens existing proof rather than requiring a new
waterfall feature or mist.

## Fit to the human's request

I read the approved [project brief](../../plans/BRIEF.md) and the complete task in
[RECORD.json](../../plans/RIVERS/RECORD.json) before comparing the design. There is one task, dated
14 November 2026. It asks for streams, rivers and waterfalls within one water system at shipped high-end
game quality and records the owner's request to think about “water as a whole” and build it cohesively.
The [module list](../../plans/ARCHITECTURE.md#rivers-and-puddles) supplies the stream/pool/narrows, float,
foam and waterfall outcomes. The project brief remains authoritative; this module adds no V1 exit gate.

The overall division of responsibility fits that request. Prepared flow supplies WATERS charts; the existing
surface, query, optics, foam stocks and interaction waves remain the owners of runtime water. The flow bands
address calm-weather roughness instead of requiring wind. The waterfall supplies geometry and causes into the
same system. The source-level samples are a reasoned extension of the requested cohesion, recorded openly
as an assumption, and the seven architecture rulings document the producer/consumer changes.

Mist, spray, rain-fed runoff, travelling floods, overturning standing waves, grounding and audio are explicitly
deferred in the design and the plan's human assumptions. These are visible scope decisions, not hidden
claims that the features exist. I do not count them as omissions against this commission. The high-end look
still has to pass its later visual reviews; the algebra supplied here cannot establish it.

## Acceptance review

I read all **65 items**, including their checks, reviewer questions, inspections, quality bars and declared
dependencies. These are design-level assessments of judgeability, not milestone pass verdicts.

| Items reviewed | Assessment |
| --- | --- |
| M1.1–M1.11 | Closed forms, terrain-derived flow, conservation, refusal, determinism and cost records give meaningful CPU proof after WATERS M1. Findings 1 and 3 affect chart conversion and its acceptance. The CPU solver's disconnected-hollow check alone does not prove the exported map preserves it. |
| M2.1–M2.7 | Brink detection, launch/path arithmetic, discharge transfer, moving receiver level, dry impacts and unchanged map tests can be judged on the CPU. This milestone does not require a rendered sheet. |
| M3.1–M3.7 | Fresh intermediate solves and rejection of a provider stuck on one sample are useful checks. Finding 1 prevents approval of the wet/dry sample mechanism. Flow-band arithmetic and the f64 evaluator are judgeable before the GPU passes exist. |
| M4.1–M4.10 | WATERS M9 supplies the first chart, query, joins and game integration. The selection, packaged river files, sample-dependent GPU answers and moving-stage inspections are concrete. Excluding the undrawn fall sheet from M4's visual judgement is appropriate. |
| M5.1–M5.9 | Calm-weather roughness, tracked motion, source-off foam ageing and direct visual inspection reject a wind-only surface or a sliding texture. The measured GPU bound is supplemented by perceptual checks, not treated as proof of the look by itself. |
| M6.1–M6.9 | Sheet geometry, accelerating streaks, motion output, dry-lip behaviour and visual inspections are judgeable after the preceding work. Finding 2 affects the interaction-current mechanism and proof. The declared visual range deserves the suggestion above. |
| M7.1–M7.12 | WATERS M10/M16 supply the later reconstruction and underwater-light dependencies. Controls, temporal output, Sun visibility, caustics, underwater views, browser integration, public docs and honest maturity reporting have identifiable evidence. No physical iPhone or unpublished performance threshold is made a gate. |

The exact-baseline obligations preserve existing non-river scenes. The cost items correctly ask for comparable
records and investigation of tripwires rather than turning baseline timings into admission limits. No other
Must Fix was established from the remaining items.

## Evidence and limits

Read: the RIVERS design, plan, task record and every done-when item; the relevant architecture rulings and
water-map, join, query, interaction and downstream-light seams in WATERS; the applicable C02/C04/C07/C08/C09/C12
contracts; the engineering review guidance and project/skill calibration examples. No plans glossary was present.
I inspected the supplied proof script and its recorded results. I did not delegate the review or inspect an
earlier RIVERS implementation.

- `python3 docs/evidence/RIVERS/design-proof/river_proof.py` completed successfully; the
  [review rerun](design-review/river-proof-rerun.txt) matches the supplied `river-proof.txt` byte for byte.
- `python3 docs/evidence/RIVERS/design-review/counterexamples.py` completed successfully; its assertions
  and [output](design-review/counterexamples.txt) confirm the three written-rule counterexamples above.
- `./dev check-docs` failed on two pre-existing missing targets linked by
  `docs/evidence/APPLE-TILE/M10-handoff.md`: `M10/README.md` and `M10/rebase-source-identity.json`.
  The links are present in the reviewed commit and their targets are absent there. The check reported no
  defect in the new review. These unrelated files were left unchanged; the temporary check build was removed.
- The supplied river reproduction supports its selected section and small-grid cases. It does not prove
  connectivity under gauge interpolation, simultaneous-source carriage, a rendered waterfall or device cost.
- No engine/GPU/browser run, screenshot or motion inspection was performed. The external research sources
  were not independently revalidated; no verdict here relies on accepting their game-specific claims.

Only this review and its evidence folder were written. `DONE.json` and all planner fields are unchanged.

DESIGN CHANGES: 3 must_fix findings
