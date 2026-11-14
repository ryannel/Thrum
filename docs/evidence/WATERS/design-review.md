# WATERS design review

14 November 2026. Reviewed checkout `def29817814c836c63ebbd616669c1a3c77c3107` against the previous review at `f3c0021c8b361702c2d29048e8bfd8b59f17b477`.

**Design OK. Both previous Must Fix findings are resolved at design level.** The design answers the human's request for cohesive water through one shared map, surface, optics, foam and query, with explicit ownership for the later river, waterfall and puddle work. There is one Suggestion about the scope of a numerical evidence claim. No Must Fix remains.

This is an independent design review, performed without delegates. It does not qualify implementation, rendered appearance or device performance. No GPU output, screenshots, motion or panel cadence were inspected.

## Previous Must Fix findings

### Body precedence does not produce a continuous join — resolved

The [map rule](../../plans/WATERS/DESIGN.md#the-water-map), lines 311–331, now makes the complete surface calculation consume the mixed row. Swash depth and wave support follow the row's gains at the point; neither is retained separately per body. The bed and wave field are shared. At an endpoint whose weight is one, the limiter consequently receives the same inputs on both sides. Shore-margin points inherit the adjoining water's row too. M1.4 no longer forbids the swell admitted by the mouth's join.

Part K now includes the limited surface at shallow ends and the prior review's no-wind, 0.5 m swell case. My independent calculation reproduces its old heights, 0.263033009 m and 0.053033009 m, and the 0.21 m step. The revised row rule makes the step zero. A further 1,000 endpoint cases with four layers and varied phases, depths and levels also agree. This checks the specification's endpoint arithmetic; it does not establish the rasterised join. [Independent source](design-review/check_third_revision.py) and [results](design-review/third-independent-check.txt).

M1.12 now requires the surface **after** the depth limit at both deep and shallow ends. M9.1 requires a low-tide mouth where the test confirms that the limit actually binds, and M9.3 inspects that mouth in motion. These reject the former shortcut of testing only a deep end or an unbounded raw wave.

Backwater now includes the regular-wave reach. The design also declares the remaining approximation beside banks where simultaneous crests exceed that reach, and beside the source's shore margin (lines 355–362). Endpoint equality is established; these lateral shore contacts are not a universal continuity proof. The retained visual join criteria remain responsible for their appearance in the example. I found no demonstrated new blocker in that declared approximation.

### The beach mechanism pins the waterline that the items require to run up and recede — resolved

This is the previous review's tracking name. Its remaining defect was foam transport; the moving waterline and residual film had already been repaired.

The new [swash transport](../../plans/WATERS/DESIGN.md#whitewater), lines 728–765, adds drift from the rising sheet's thickness rate and gradient, bounded by `sqrt(g × swash depth)`, only on wet support and weighted through the swash band. Transport retains material arriving at the edge; exposed material remains and ages. The design explicitly limits this to uprush, with no backwash transport and no resolved carriage on the coarse far level. There is now a mechanism that moves existing material, rather than relying on births at successive positions.

I independently evaluated the single-wave height with analytic derivatives and transported a 10%-cover patch on 0.25 m cells, with no wind, current, births, ageing or spreading. After three waves on a 1:10 beach, its centre was 2.550 m with one-tick steps and 2.591 m with eight-tick steps, against a 2.622 m run-up line. Both retained all material and stayed below full cover. With the old drift the centre stayed at −0.500 m. The source and output are linked above.

M7.16 supplies the independent parcel comparison and stationary controls; M7.2 carries the comparison onto Metal. Both can be judged within M7 using the existing surface model. M7.9 still requires the carried foam and exposed lace to look right in motion. Risk 17 explicitly covers excessive gathering into a hard line. The numerical result makes the mechanism credible; it is not a visual pass.

The earlier disconnected-region and finite-depth findings remain resolved as recorded in the previous review. Their mechanisms and acceptance items were not weakened by this revision.

## Suggestion

### Scope the foam conservation claim to unsaturated cover

**Suggestion — verified evidence-description mismatch.** [DESIGN.md](../../plans/WATERS/DESIGN.md#whitewater), lines 754–756, says the grid patch keeps all its foam. However, [part M](design-proof/water_proof.py), lines 658–677, starts with full cover and never applies the declared saturation sink. Its output reports peaks of 3.00 and 3.47, although cover is bounded by one. The output's final note acknowledges this; the prose summary does not.

In my independent run, applying the declared cap to that full-cover initial patch retained 37.75% with one-tick steps and 29.58% with eight-tick steps. A thin 10%-cover patch retained 100% in both cases and still reached the run-up line. These figures exclude ageing and spreading, as part M does.

Describe the retained result as uncapped transport conservation, or use an unsaturated patch for that claim. M7.16 already specifies a thin patch, and the design already names saturation loss, so this does not prevent building or make the acceptance item impossible. The later implementation should account for the named sink when judging a converging dense patch.

## Changed-item review

I read all 119 current items, their proof fields and milestone quality bars, and compared the planner fields with `f3c0021c8b36`. These are design judgments, not milestone verdicts.

| Changed item | Assessment |
|---|---|
| M1.4 | The body's own fetch-limited energy is distinguished from the joined row. The no-wind and added-swell checks are judgeable on the CPU and no longer contradict the mouth's swell. |
| M1.5 | Swash depth is local to the row, including inside a join; the limiter no longer retains the missing body-specific input. |
| M1.12 | Tests the assembled limited surface, swash depth and support at shallow and deep ends through stage changes. Its reviewer question explicitly rejects the previous per-body shortcut. |
| M4.10 | The sheet alone writes depth. This resolves the previous suggestion about identifying physical water after M7 adds a film; M4 itself needs only the sheet. |
| M7.2 | Adds GPU swash transport against the CPU reference while preserving wind drift and window-shift checks. M7 builds both sides of this comparison. |
| M7.15 | Adds a direct depth read under the residual film, alongside the dry query and zero births. It cannot pass solely on the film's appearance. |
| M7.16, new | Requires actual movement of pre-existing foam with no wind/current, an independently computed path, conservation for a thin patch, and stationary controls. The independent calculation above demonstrates a viable case. |
| M9.1 | Adds a mouth whose limiter demonstrably binds at low tide. The prior M1 model, surface, query and optics exist by this milestone. |
| M9.3 | Adds swell and held low tide to the temporal join inspection, covering the regime that concealed the former defect. |

Both previous Suggestions were taken: sheet and film depth are explicitly separated in the design and items, and the thin-obstacle leakage figures are now scoped to their 1D pulse, geometry and ten-second observation (design lines 628–635), rather than asserted as a universal bound.

The plan still reaches a first Mac frame in M2 and Chrome in M3, then adds inland water, queries, interaction waves, foam, optics and the stand-in channel before final temporal and Apple qualification. Numeric performance remains a measured baseline with investigation tripwires. No physical iPhone or new external approval is introduced by these changes.

## Request coverage and review scope

I read the approved [project brief](../../plans/BRIEF.md) and all three verbatim tasks in [RECORD.json](../../plans/WATERS/RECORD.json), oldest first. The two re-plan tasks ask for repairs; they do not supersede the human's request or the project brief.

The human asks that sea work consider puddles, rivers, waterfalls, streams, lakes and more as a cohesive system. WATERS supplies the common system and proves its seams on still and running water. SEA supplies the wave field; RIVERS owns the prepared flow solver and falls; PUDDLES owns rain-filled hollows and wet ground, including the explicit merge/separation seam question. This remains a reasonable division of the requested work. It does not claim those later experiences already exist. Prepared flow, no overturning surf, no underwater camera, no stacked water and screen-space reflection limits remain declared assumptions. Water remains outside the V1 exit.

The review covered the current design and plan, every changed design line and item, the previous review, revised proof sections K and M and retained results, the module-list ownership and seam rulings, and relevant C02/C04/C06–C09/C12 requirements. I did not requalify predecessor implementations, Broadsides or external research claims, or independently re-derive the unchanged H–J and inherited SEA proofs. No glossary exists in the plans folder.

## Validation

- `python3 -B docs/evidence/WATERS/design-proof/water_proof.py` completed successfully. Its [rerun output](design-review/third-proof-rerun.txt) is byte-for-byte identical to the retained output.
- `python3 -B docs/evidence/WATERS/design-review/check_third_revision.py` passed its assertions. This independent specification check imports neither engine code nor the planner's proof.
- `./dev check-docs` passed: 839 Markdown files, 5,978 local links, 30 module records, crate policy and current design hashes. Existing source-size notes reported no errors. [Check output](design-review/third-check-docs.txt). `git diff --check` also passed.
- Only this review and new files inside `docs/evidence/WATERS/design-review/` were changed. Existing captures, plans, DONE.json, records and implementation were preserved. No commit was made.

DESIGN OK
