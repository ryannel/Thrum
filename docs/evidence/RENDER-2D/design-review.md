**RENDER-2D design review — approved**

Reviewed on 13 November 2026 at `e119993ac1c087014571ad0862668d33b912841a`, against the previous review of `fe5c89f9a19238e83e829db8775eb79b0eedf063`. **Both previous Must Fix findings are resolved. There are no new Must Fix findings.** The revised design can deliver this module's part of the human's request: reusable 2D engine support, alongside 3D, for the swarm-survival game. This approves the design and future proof obligations; it does not qualify an implementation, visual output or device performance.

**Previous Must Fix — `sort-band-does-not-prevent-reversals`: resolved.**

[Mechanism 4](../../plans/RENDER-2D/DESIGN.md#4-order) replaces independent held sort values with a persistent list and adjacent exchanges. For persistent world rows on the same layer, a near pair exchanges only when its sort-value difference exceeds the band in the required direction. Changing other rows cannot reverse that pair without exchanging the pair itself. Every allowed exchange removes an inversion of the exact key, so settling a tick terminates; keeping a list supplies a total order without relying on a non-transitive tolerance comparator.

I independently implemented that rule in [order_probe.py](design-review/order_probe.py), with [retained results](design-review/order-probe.json). The previous review's travelling pair stays A,B at all four ticks. A subsequent pass exceeding the band changes it to B,A, so stability is not obtained by freezing the list. The stated three-row chain remains A,B,C despite the endpoints being 1.8 bands out of exact order. Across 750 three-row starting states and 2,000 randomized ticks of 24 persistent same-layer rows, the independent final-pair checks found no forbidden near-pair change or unsettled adjacent pair. The random run exercised 260,299 exchanges and changes in proximity and size. With a zero band the result always equalled exact sorting.

This reproduction checks the replacement's central rule, not the planner's reported crowd measurements. It does not test the renderer, insertion/removal, GPU pixels or timing. M2.13 adds those implementation obligations, including churn and reused slots, independent relation checking, pixel readback of the travelling pair, and rejection of the old held-value substitute.

The revision also states the cost of its choice: chains can leave non-adjacent entries more than one band out of height order. M5.3 records that effect on the real crowd, and M5.6 explicitly requires a visibly lower overlapping body to appear in front. Thus the planner has neither claimed an impossible global bound nor removed the visual acceptance condition. The synthetic probe's favourable figures do not establish that the actual crowd will pass. If it does not, the design already calls for reconsidering the mechanism. Its disclosed quadratic exchange case is a performance risk to measure under the existing tripwires, not an invented numeric gate.

**Previous Must Fix — `c07-no-reconstruction-exception-unresolved`: resolved.**

[C07](../../design/CONTRACTS.md#c07-render-scene-views-and-temporal-data--v2-with-p4v3v4), the [hardware-native frame contract and governor](../../design/decisions/HARDWARE-NATIVE-ENGINE.md#frame-contract), [seam ruling 1](../../plans/ARCHITECTURE.md#seam-rulings-for-render-2d), and the design's mechanisms 1–2 and interface table now agree. Every 3D view keeps the full form. A 2D view declares no reconstruction at preparation, publishes projection, exposure and output extent, and has no provider inputs, history key or governor render scale. The registry resolves no reconstruction, frame-interpolation or denoising provider and refuses an explicit provider request by name. The producing frame runtime and consuming presentation code are assigned to M1 under the existing ruling.

M1.2 now checks absent resources and selections with a 3D positive control that actually resolves a reconstructor. M1.3 checks explicit refusal wherever the eventual runtime exposes a request route. These can be built and judged without fabricating the buffers that M1.2 prohibits. No exception remains pending this review, and no 3D obligation is weakened.

**Previous Suggestions: both addressed.**

The coverage measurements are now separate: M3.1 retains both per-pixel error and the summed-weight departure of an isolated lit texel from its exact area. M3.2 consumes the weight bound; M3.7 retains a Chrome-specific weight bound. The design explicitly rejects deriving one measurement from the other. The unextended graph is now identified as the fixture without contributions in M1.2; M1.3 and mechanism 1 explicitly add upload and simulate contributions through the existing seam.

**Suggestion — scope the pair-rule shorthand to its stated exceptions. Verified wording issue.**

Mechanism 4, lines 223–225, correctly distinguishes near rows **on one layer** from all other pairs, which use the exact key. Its later pair-rule guarantee at lines 237–239 and M2.13's statement omit the same-layer qualifier. Also, “Any other two rows … cannot overlap on screen” is false for rows on different layers: those may overlap and are deliberately ordered by layer. Qualify the shorthand as persistent same-layer world rows, and restrict the non-overlap explanation to spatially distant rows. The algorithm's explicit branch and its layer-first key already settle the behaviour, so this is a clarification, not a reason to hold the build.

**Changed-item review.**

I compared the proof-bearing fields with `fe5c89f9a192`, read every changed item in full, and reread the complete current design, plan and all 59 items. The other DONE.json changes are record-schema changes, not altered acceptance criteria.

| Changed item | Assessment at its milestone boundary |
|---|---|
| M1.2 | Judgeable with embedded sheets; the contribution-free fixture and positive reconstruction control address both earlier concerns. Resource and pipeline accounting still rejects hiding a running 3D scene. |
| M1.3 | Judgeable in the extended host after the declared V1 dependencies. Named refusal and executed upload/simulate contributions test both accepted and rejected requests. |
| M2.2 | Exact ordering is now explicitly the zero-band case. Overlap pixels, identity ties and reused slots retain a meaningful test of what is drawn. |
| M2.13, added | Separately proves the banded rule with history, multiple rows, churn, the counterexample, a chain and a negative control. M1's embedded sheets and M2's existing simulation source suffice; SIM-2D is not needed yet. Apply the wording suggestion above. |
| M3.1 | Measures the two errors independently, against CPU texel areas, with nearest and bilinear controls. The retained figure must remain an observed sampler error, not a tolerance enlarged to admit a wrong sampler. |
| M3.2 | Uses the appropriate retained weight bound and requires the nearest control to fail. It can follow M3.1 within this milestone. |
| M3.7 | Uses a bound measured in Chrome rather than importing native precision assumptions; density and resize checks remain. |
| M5.3 | Tests the pair rule independently on SIM-2D's Crowd trace and records both reversal reduction and the resulting order error. It no longer relies on the disproved per-row filter argument. |
| M5.6 | Adds a visual check for the disclosed chain risk without dropping crowd legibility, motion, clean lifecycle transitions or effects. Real-time sequences and full-size stills remain separate evidence. |

The unchanged proof sequence remains coherent: M1 reaches the actual Mac and Chrome frame with embedded sheets; M2 adds tick rows; M3 proves sampling; M4 uses CONTENT-LOOP's publication and leases for cooked sheets; M5 uses SIM-2D's existing Crowd trace and adds simulator function and public-interface reuse. No changed proof needs a later milestone to supply its inputs. Exact-baseline checks still protect the 3D path throughout.

**Request authority and limits of this review.**

I read both tasks under [RECORD.json's briefs](../../plans/RENDER-2D/RECORD.json), oldest first: the initial “Plan module RENDER-2D. It needs: SHOWCASE, APPLE-TILE, CONTENT-LOOP” and the subsequent instruction to resolve the two review findings and account for Suggestions. Neither is a new human requirement. The authority is [BRIEF.md](../../plans/BRIEF.md), decision 7, and the human's words on [the next-goal page](../../design/NEXT-GOAL-SWARM-GAME.md): 2D games as well as 3D, proved by an original networked swarm survivor. The module list assigns rendering here, game policy/art to SWARM, and networking to TRANSPORT/SESSION.

Within that division the design covers sprite scenes, crowds, animation and interpolation, stable layering, colour and effects, sheets and hot reload, lifecycle, and reuse without engine game types. Its narrower sprite-based interpretation, excluded 2D features, sheet limit and linear-light blending remain explicit assumptions for the human in PLAN.md. The revision adds no unrelated scope and does not silently redefine the human's request.

I also read the project review guidance, graded examples and the planner's probe. I did the reading and arithmetic myself. No running sprite implementation, image sequence or physical display was inspected, and no GPU or device performance was measured. The probe above establishes only its stated arithmetic checks. Future engine, visual and device evidence remains required; the existing panel, EDR and physical-device omissions remain disclosed.

Validation: independent ordering probe and `git diff --check` passed. `CARGO_TARGET_DIR=/private/tmp/thrum-render2d-design-review-target ./dev check-docs` failed in `scripts/repo_check.py` on existing links in other evidence files to renamed `ACCEPTED.json` records (including APPLE-PRESENTATION, ATMOSPHERE, CONTENT-LOOP and SIM-2D). It reported no bad link in this review. Those files are outside this review's permitted edits and were left unchanged; this is not a new RENDER-2D design defect. Repository status shows only this review and its evidence folder changed; planner fields and milestone verdicts were left untouched.

DESIGN OK
