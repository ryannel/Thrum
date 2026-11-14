# WATERS design review

14 November 2026. Reviewed checkout `b2d0778af38e5f685e9291d68486aab5cbc98e0c`.

**Design Changes Requested: four Must Fix findings.** The division into a shared water system, a wave-field module, rivers and puddles addresses the human's request. But building the specified mechanisms would not yet deliver seamless joins, isolation across land, or the promised moving beach edge. The finite-depth solver's bounded configuration also differs from the proof used to justify it.

This is a design review, not acceptance of implementation, visuals or device performance. There was no previous WATERS design review in this checkout. I did the reading and judging independently, without delegates.

## Must Fix findings

### 1. Body precedence does not produce a continuous join

**Must Fix — verified from the specified sampling rules.** Affects W6, M9.1, M9.3 and M11.8.

The [water map](../../plans/WATERS/DESIGN.md#the-water-map), lines 289–292, switches to a chart wherever it overlaps a basin or sea. The [gains](../../plans/WATERS/DESIGN.md#waves-on-a-body-gains), lines 306–314, differ by body and exclude swell from every chart and basin. The [surface rule](../../plans/WATERS/DESIGN.md#the-surface-and-the-frame-contract), lines 393–398, claims that drawing only the selected body avoids gaps. It avoids double ownership; it does not make the two displaced surfaces meet.

Take the planned channel entering the sea with matching mean levels, at a mouth deep enough that the depth limiter is inactive. At a 0.5 m swell crest the sea side is 0.5 m above its mean and the chart side has no swell. There is a displacement jump at the ownership boundary. Different cascade gains and current-carried phases create the same problem for shorter waves. No transition rule is specified for geometry, normals, current or query answers. The [local calculation](design-review/check-mechanisms.txt) records the swell case.

The stage control exposes another ordinary instance: M10 raises the lake while the prepared chart retains its level unless its own stage changes. A uniform chart stage cannot independently match both a raised lake and an unchanged sea. The approximation that chart current stays prepared does not settle the surface connection.

**What resolves it:** define how connected bodies meet, including their displaced surfaces and physical query ownership, under waves and the commissioned stage changes. Prove a channel-to-sea join with nonzero swell and a connected level change. Keep C04/C09's distinction between visual transitions and physical ownership; drawing a blend alone must not invent a physical coupling. The existing join items express the right outcome but need a mechanism capable of reaching it.

### 2. Zeroing dry cells does not stop this FFT operator transmitting across land

**Must Fix — verified by an independent numerical reproduction.** Affects W4, M6.4 and M6.10.

The [interaction design](../../plans/WATERS/DESIGN.md#interaction-waves), lines 471–473, says that zeroing height and potential on dry cells after each global transform, with damping in the adjacent cell, means no wave crosses a dry bar. A global spectral update couples wet cells on both sides before that mask is applied. Resetting the land cells afterward does not remove the wave already written into the other body.

I reproduced a 256-cell, 64 m cross-section at 60 Hz, with uniform 4 m wet depth, a two-metre land bar and a localized initial disturbance four metres to its left. The calculation uses a correct finite-depth height/potential rotation and completely zeros the adjacent wet cells as well as the land, giving the proposed absorption more help than partial damping. Over ten seconds, the peak height in the water on the other side reaches **0.031940 of the initial peak**; without the barrier it reaches 0.235890. This is a one-dimensional slice of a field constant along a straight land bar, so the same global coupling exists in two dimensions. It is not a GPU or visual measurement.

Reproduce with `python3 docs/evidence/WATERS/design-review/check_mechanisms.py`; [source](design-review/check_mechanisms.py) and [output](design-review/check-mechanisms.txt) are retained. The source paper also explicitly expresses the FFT step as spatial convolution; it does not give the design's dry-cell isolation guarantee. See [Tessendorf, eWave, equations 19–27](https://jtessen.people.clemson.edu/reports/papers_files/ewavealgorithm.pdf).

**What resolves it:** select and demonstrate boundary handling that keeps disconnected wet regions independent within a declared meaningful error bound. The proof must exercise the actual selected spatial operator across dry land. A uniform-depth dispersion check or a test that only reads zeros on the land itself cannot establish M6.4. Reflection from a wall need not be added; isolation is already promised.

### 3. The beach mechanism pins the waterline that the items require to run up and recede

**Must Fix — verified by a closed-form case and the ownership rules.** Affects M4.5 and M7.9; also the claimed remedy in risk 12.

The [map](../../plans/WATERS/DESIGN.md#the-water-map), lines 281–301, defines wetness from the body's mean level and stage, and contains no waves. The [depth limiter](../../plans/WATERS/DESIGN.md#waves-on-a-body-gains), lines 317–320, takes ambient wave height to zero at that shoreline. The surface shades only where the map selects water. Yet [M4.5](../../plans/WATERS/DONE.json) requires the waterline to run up and back on the sand with each wave; risk 12 says the long waves provide this “for nothing.”

Consider one regular wave on a gently sloping beach with fixed stage. Using the proof's spectral definition, its significant height is `4 * sigma = 2 * sqrt(2) * amplitude`. The selected cap makes its amplitude at most `0.6 / (2 * sqrt(2)) = 0.212132` times local depth. The trough therefore never exposes the bed at a wet point; the wave vanishes at zero depth; and the map refuses water beyond the mean shoreline. That admissible wave cannot move the shoreline either way. Scene-depth clipping cannot reveal water that the map and displacement rules never supply.

M7 adds another unconnected part: [foam transport](../../plans/WATERS/DESIGN.md#whitewater), lines 565–570, preserves foam on ground water has left, but its appearance is drawn by the water surface, which excludes that ground. The forward material's wet-ground integration is deferred to PUDDLES. The design specifies neither a transport mechanism for the requested wave-driven wash nor a drawing route for the exposed lace in M7.9.

**What resolves it:** reconcile the map's physical wet support, displaced surface and shore-foam rendering with the commissioned run-up and recession, including C07's shared-support requirement. Demonstrate an ordinary sloping beach at fixed stage. This does not ask for the expressly deferred overturning surf; it asks for the simpler shore behaviour the design and items already retain. If that behaviour is to be deferred too, make that scope decision explicit and reconcile the corresponding items and downstream wet-shore expectation.

### 4. The bounded finite-depth solver is not the solver its proof measures

**Must Fix — verified specification/proof mismatch.** Affects W4, M6.2 and the interaction cost account.

The [depth rule](../../plans/WATERS/DESIGN.md#interaction-waves), lines 464–469, requires factor-two depths from the shallowest wet cell, no lower than 0.125 m, up to half the level's longest wavelength, at most six per level. Each cell must interpolate between depths bracketing its own. A window containing 0.125 m shallows and 10 m water is ordinary at the planned shore. Its six bins are `0.125, 0.25, 0.5, 1, 2, 4 m`: they cannot bracket 10 m, much less cover the far level's long waves. No upper-range substitute or refusal is defined.

The cited [Part H](design-proof/water_proof.py), lines 25–35, instead uses **twelve** bins through 256 m. It blends scalar complex phases at one uniform depth; it never evolves the specified height/potential fields through a spatially varying depth map. Its 1.6% speed and 0.3% loss figures therefore do not qualify the selected capped solver. M6.2 tests two-metre waves at a few depths and could pass while long waves in the same mixed-depth window use an incorrect substitute.

There is also an unresolved equation change. The text calls finite depth “a different frequency” and still instructs the builder to apply eWave equations 19 and 20. Simply replacing `omega` there leaves the `k/omega` coefficient unchanged and is not a rotation: for a two-metre wave in 0.25 m of water at 60 Hz the determinant is 1.002941180, giving about **9.2% amplitude growth per second**. The retained reproduction shows this literal interpretation. A correctly generalized operator can avoid that error; the design must state that operator, rather than relying on a proof of scalar phase interpolation to disambiguate it.

**What resolves it:** specify the finite-depth state update and a bounded depth-selection policy covering the claimed domain, then qualify that actual configuration, including long waves across mixed shallow/deep water and the supported multi-tick step. Reconcile its resource cost and M6.2 with the resulting error envelope. This is a representation and validity decision, not a request for an exhaustive test procedure.

## Suggestions

1. **Resolve basin merging before PUDDLES builds against the seam — verified planning gap, deferred owner.** [PUDDLES' brief](../../plans/ARCHITECTURE.md#rivers-and-puddles) requires puddles to join when they overflow, while WATERS refuses overlapping bodies of the same kind and refills each basin independently when its stage changes. The later planner owns filling and merging, so this does not independently hold WATERS. Give that planner an explicit seam question: how merging and separation preserve body identity, stages and retained interaction state without a visible rebuild/reset. Likewise clarify the inconsistent planning order: WATERS' plan says its channel precedes planning the flow solver, while the module list schedules RIVERS planning before M9.

2. **Make the lake-foam comparison conditional — verified wording conflict.** M7.5 and M7.9 require no lake-shore foam in the same wind, yet a lake with nonzero short waves also loses wave power under the universal depth limiter. Specify the intended lake conditions or a physically stated birth threshold. Otherwise an implementation that correctly applies the shared cause can fail the example's absolute “none” requirement.

## Request coverage and acceptance review

I read the approved [project brief](../../plans/BRIEF.md) and the sole task in [WATERS' record](../../plans/WATERS/RECORD.json), under `briefs`, before judging the mechanisms. The human asks for cohesive water encompassing puddles, rivers, waterfalls, streams, lakes and more. The project brief keeps water outside the V1 exit, requires reuse, Mac and Chrome quality, and treats numeric performance figures as baselines rather than gates.

The design genuinely broadens the old sea scope. SEA keeps its two wave milestones unchanged; WATERS owns shared state and readers; RIVERS and PUDDLES have named briefs, consumers and build dependencies. Waterfalls are assigned sheet geometry and plunge inputs, and wet ground has a named future material owner. Nothing requires all of those implementations to be in this one module. Prepared flow, no underwater camera, no stacked surfaces, no surf and screen-space reflection limits are explicit human-facing assumptions, rather than hidden narrowing. I found no reason to require a different module split or to add those deferred capabilities in this review.

All **114 items across M1–M11** were read, including each proof and quality bar. Their order generally allows judgment with prior milestones built: CPU authority first, a costed first frame, Chrome, inland bodies, queries, interactions, foam, optics, a stand-in channel, temporal controls, then Apple qualification. M11's record check explicitly holds closure for the governor and MetalFX. The main acceptance problems are the four mechanisms above, not missing test commands or the absence of a rendered product before building starts.

The items have useful safeguards against plausible fakes: independent map classification, GPU queries that lift a float with a wake, transported foam after its source disappears, optical-unit comparisons, lifecycle cases and paired cost records. The depth item is the notable weak proof: its narrow wave samples do not qualify the broader solver claimed by the design. Existing join and shore items would reject a faithful implementation of the current mechanism, so leaving them to milestone review would postpone a design defect until after construction.

The boundary review included the relevant C02/C04/C06–C09/C12 clauses, SEA's wave interface, EFFECT-GRAPH's state/commit rules, the module list's WATERS rulings and future-module briefs, and WATERS' retained depth/gain proof. The depth-writing and gather extensions have named owners and rulings; the two persistent endpoint banks fit the seam's tentative-submission model on paper. I did not re-review all predecessor implementations or independently rerun the inherited deep-sea proofs, and I did not inspect Broadsides code. The source paper inspection and numerical cases above qualify only the specific objections stated, not the whole water model. No GPU run, screenshot, motion or panel-cadence judgment was made.

## Validation

- Ran the retained standard-library reproduction; its complete output is [check-mechanisms.txt](design-review/check-mechanisms.txt).
- `./dev check-docs` passed: 832 Markdown files, 5,895 local links, 30 module records, crate policy and current design hashes. It reported existing source-size notes, with no errors.
- Only this review and its `design-review/` evidence folder were added. Plans, done-when items and implementation remain unchanged.

DESIGN CHANGES: 4 must_fix findings
