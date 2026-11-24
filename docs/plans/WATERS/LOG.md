# WATERS log

- 2026-11-15 Repaired the second M1 review's connected-fill finding. Seeding,
  flood propagation, exterior-sea validation and continuous lookup now share
  the bilinear saddle's strict wet connectivity. Margin growth starts at the
  waterline, keeping it out of chart-owned water. All 32 water-model tests,
  strict Clippy and documentation checks pass, including mirrored/translated
  seed cells, remote diagonal necks, reversible stages, surface answers and
  stage/sea spill refusals. [M1 evidence](../../../.orchestrate/evidence/WATERS/M1.md) identifies
  CPU and Mac/Chrome proofs; GPU water quality and performance remain unclaimed.

- 2026-11-15 Fixed both verified findings from the first M1 review. Triangle
  rasterization tracks actual intersections separately from the uncovered-cell
  fallback. Continuous map lookup now checks monotone reach inside a bilinear
  cell, so a supported crest cannot lend support downhill. All 26 water-model
  tests pass, including below-fallback overlap order and subcell sea, basin and
  chart ridge regressions. The retained [M1 proof](../../../.orchestrate/evidence/WATERS/M1.md)
  identifies CPU and exact-baseline source revisions; no water GPU visuals or
  device performance are claimed.

- 2026-11-15 Resumed the surviving integration job after its shared GPU wait.
  It rebased cleanly onto main as `2c2679be` and refreshed every CPU item check,
  the sea-model tests, strict Clippy and documentation checks; all passed.
  Re-read the independent map, gains, conformance, joins and swash oracles and
  the provenance declaration. The fresh exact verifier is running on this source;
  [the retained target record](../../../.orchestrate/evidence/WATERS/M1-baselines.json) identifies
  the complete pass once promoted, separately from the earlier failed captures.

- 2026-11-15 Resolved the older CREATOR, CULLING and CLOUDS link repairs by
  retaining their upstream replacements; documentation checks passed. Consolidated
  unpublished WATERS work onto the current main base to avoid replaying superseded
  integration repairs. CPU evidence is refreshed if its revision is no longer an
  ancestor of the work being merged. Target proof and official merge remain owed.

- 2026-11-15 CPU proofs passed on `32382d0a`. The exact verifier then stopped at
  an incomplete native route capture: 434 submissions, 292 images and 142 ring
  omissions, with host load near 100. This is not a fresh baseline pass.
  [Diagnostic record](../../../.orchestrate/evidence/WATERS/M1-runtime-admission.json) retains
  the identity and observed contention; references and comparisons are unchanged.

- 2026-11-14 Resumed M1 with 91 GiB free, inspected the CPU oracle tests and
  provenance, and resolved the integration-only handoff conflict on rebase.
  The GPU-locked driver refreshes target proofs before the official merge;
  [M1 evidence](../../../.orchestrate/evidence/WATERS/M1.md) retains the source identities.

- 2026-11-14 After owned cache cleanup, retry `20261114-142437-51691` rebuilt all
  verifier artifacts and cooked content on `3ec5e9f6`, then the first native
  capture refused less than 16 GiB free. The observed remainder was 15.9546 GiB.
  [Admission record](../../../.orchestrate/evidence/WATERS/M1-disk-admission.json) and
  [handoff](../../../.orchestrate/evidence/WATERS/M1-handoff.md) preserve the source and remaining
  requirement. Shared disk headroom is needed for M1.11 and integration; no fresh
  baseline pass or merge is claimed. Owned rebuilt cache is selected for cleanup.

- 2026-11-14 A strict retry stopped before capture at the 16 GiB disk-admission
  floor. Verified all 481 decoded frames in the earlier identity-only failure;
  kept its discrepant frame and metadata, releasing 1.49 GB of matching bulk.
  Selected the owned reconstructible build cache for cleanup before retrying.
  No reference, tolerance, criterion or other builder's scratch was changed.

- 2026-11-14 Rebased with SEA's new crate registered alongside the water model.
  Fresh verification on `7cb8dd05` passed both native matrices and three Chrome
  sets. The fourth matched every pixel hash and exposure word but refused ordinal
  62's tick/alpha identity (182/0 versus 181/1). This suggests an interpolation
  boundary variation; its cause is unconfirmed. The failed candidate is retained
  in `tmp/baseline-guard/verify-20261114T123313.738319Z/` for diagnosis; references
  and tolerances are unchanged. [Continuation](../../../.orchestrate/evidence/WATERS/M1-handoff.md).

- 2026-11-14 Resumed integration: both native baseline matrices and all four
  Chrome sets passed on `343e6bd3` in 910.41 seconds; retained the fresh record and
  CPU test log in [M1 evidence](../../../.orchestrate/evidence/WATERS/M1.md). The first merge gate
  found two upstream APPLE-TILE handoff links to evidence not yet retained on main.
  Preserved their paths as text so repository link checks can run without claiming
  those artifacts are present. M1 criteria and builder results are unchanged.

- 2026-11-14 M1 builder implemented the CPU map, stages, joined rows, wave limits,
  surface oracle, triangle bed and conformance harness. All 24 water-model and ten
  sea-model tests passed, with strict Clippy and documentation checks; an analytic
  plane test covers the corrected chart level slope. [Evidence](../../../.orchestrate/evidence/WATERS/M1.md).
  Both native matrices and all four Chrome baselines passed on `2aba4634`.
  No GPU water surface, visual quality or device-performance result is claimed.

- 2026-11-14 Integration resumed after CLOUDS registration conflicts and low-disk
  admission. The rebase preserves both crates and the new design; M1 criteria are
  unchanged. The next native attempt stopped before its night capture when free
  space reached 15.989 GiB. All preceding comparisons matched. Removed 32 owned
  duplicate reference copies and superseded scratch after checking retained proofs;
  free space was then 20.7 GiB. `finish_m1.py` aligns the branch before verification,
  promotes only matching target proofs and runs the GPU-locked full gate and merge.

- 2026-11-14 Owner, through the lead, about `SEA`: "Please make sure the sea work is actually thinking about water
  as a whole. So puddles, rivers, water falls, streams, lakes, and more. We want something that's built
  cohesively." Earlier the same day, of clouds: "The agent should look at that work from first principles and
  figure out how to design the best possible atmospheric system for the thrum game engine."
- 2026-11-14 Planned from the lead's task on those words: one design for all water, a plan and a done-when file
  with eleven milestones. The module is named `WATERS` because `.orchestrate/evidence/water` already holds the 1 November
  water evidence and this Mac's file system does not tell `WATER` from `water`.
- 2026-11-14 Planner's decisions. Water is a map (where it lies and how it moves in bulk), waves on it, material
  it carries and one optics, and each kind of water is data for those. Bodies are a sea, a basin or a chart.
  Every body reads `SEA`'s one wave field through four gains, from its fetch and its depth. Flow is prepared
  once, not simulated live. The surface is drawn body by body and shaded by the body the map names at each pixel.
- 2026-11-14 Changed from the sea design: the scene is copied wherever water is thin enough to see through or
  still enough to mirror (nothing on an open sea), water reflects what is on screen, interaction waves take the
  depth from a few computed depths blended per cell, die on dry ground and are carried by the current, and foam
  is born where the depth takes power from the waves. Kept: the query, eWave on two banded levels, the
  reaction-force source, four foam stocks, the frame contract, the two endpoints, capabilities, cost records.
- 2026-11-14 Milestones. M1 is the CPU map and needs `SEA:M1`. M2 to M11 carry `SEA`'s old M3 to M10: each item
  those claims reach was written again, and items were added for the map, a lake and a beach (M4), floating
  inland, rings at depth, shore foam, the bed and the shore's reflection, and a hand-made channel (M9) that
  proves the chart `RIVERS` will write. `CLOUDS` M11 now waits on M8 here.
- 2026-11-14 Planner's reproductions, in [the evidence folder](../../../.orchestrate/evidence/WATERS/design-proof/). Part H:
  depth bins a factor of two apart carry waves within 1.6% of the finite-depth speed and take under 0.3% of
  their height a second; one operator at a pond's mean depth is 23% fast in its shallows. Part I: at 10 m/s a
  lake with 1 km of fetch takes 0, 0.25, 1 and 1 of the four wave layers, and 1 m of depth holds a 2.1 m sea to
  0.28 of its height.
- 2026-11-14 Asks of other modules: the three seam rulings `SEA` had, now
  [rulings for `WATERS`](../ARCHITECTURE.md#seam-rulings-for-waters), and nothing new. The bed is the game's to
  give. `RIVERS` and `PUDDLES` have briefs in the module list; the wetness of ground will need a ruling with the
  forward pass's owner when `PUDDLES` is planned.
- 2026-11-14 Not read for the design: the Far Cry 5 water talk, Uncharted 4's rapids talk, Fluid Flux's
  documentation and Barnes 2014. Unreal's, HDRP's and Crest's pages were read as fetched summaries. Thornton and
  Guza's ratio and the JONSWAP growth relations are from memory; part I reproduces the second.
- 2026-11-14 [Design review](../../../.orchestrate/evidence/WATERS/design-review.md): Design Changes Requested, four Must Fix
  findings. Hard precedence left a step where two bodies meet; zeroing dry cells did not stop the FFT step
  crossing land; the depth limit pinned the waterline the items asked to move; and the depth bins neither
  covered a mixed window nor were the operator the proof measured.
- 2026-11-14 Re-planned for finding 1. Joins are in the map: a chart has a source end and a mouth, and its row is
  mixed toward the joined body's by a prepared weight and, at the mouth, by backwater. Stages have declared
  ranges, and preparation refuses a chart its banks do not hold and a basin that spills. M1.2, M1.3, M9.1, M9.3,
  M9.4 and M10.4 written again; M1.12 added. Part K: steps of 0.5 to 1.5 m become none.
- 2026-11-14 Re-planned for finding 2. Each connected region of a window's still water is solved alone, so
  nothing passes between two waters; thin land inside one region leaks up to 3%, declared. A level makes at most
  16 solves. M6.4, M6.10 and M6.12 written again. Part J reproduces the review's 3.2% and measures 0 with regions.
- 2026-11-14 Re-planned for finding 3. The depth limit takes a swash depth, the map names a shore margin, and
  the waterline runs up and back; sand the waves wash is drawn as a film that holds the foam. Run-up is kept, not
  deferred. M1.5, M1.6, M4.2, M4.5, M5.1 and M7.9 written again; M1.13, M4.10 and M7.15 added. Part L: 6.7 m of
  travel on a 1 in 10 beach in a 2.1 m sea, against none before.
- 2026-11-14 Re-planned for finding 4. The update carries the depth in its coefficients (`D = k tanh(kd)`); each
  level has a fixed ladder of depths a factor of two apart ending in the deep operator, blended per cell in the
  square root of depth, with a rest operator below. M6.2 written again from part H, which now measures that
  operator on fields: phase speed within 2.1%, group speed within 4.4%, no growth, steps of 1 and 8 ticks. A
  factor of three on the far level was tried and dropped: group speed off by 10%.
- 2026-11-14 The review's suggestions, both taken. Basin merging is a named seam question in `PUDDLES`' brief,
  and the plan's order now matches the module list (`RIVERS` is planned before M9). Foam on the lake's shore is
  in proportion to the power its waves lose, not none: M7.5 and M7.9 say so.
- 2026-11-14 Second [design review](../../../.orchestrate/evidence/WATERS/design-review.md): Design Changes Requested, two Must
  Fix findings. The swash depth, one number for each body, sat outside the join's mix, so the surface after its
  depth limit could step at a shallow mouth (0.21 m in its case); and M7.9 asked foam to ride the sheet up the
  sand while the transport said nothing carries it shoreward.
- 2026-11-14 Re-planned for finding 1. A reader takes nothing from a body but its row: the swash depth, the wave
  support and the depth limit follow from the row's gains at the point, so they meet across an end as the row
  does. M1.4 says what a swell adds to a body's own gains (nothing) in place of "enters no basin and no chart".
  M1.5, M1.12, M9.1 and M9.3 written again, with a shallow end in each join item. Part K now assembles the
  surface after the limit: the review's 0.21 m becomes none.
- 2026-11-14 Found while fixing finding 1, and fixed. Backwater measured from the sea's still level left the
  sea's swash sheet up to 0.10 m from the chart's on a low bank beside a mouth. It is now measured from the
  sea's reach, its level plus 0.175 of the margin height (part K, M1.12).
- 2026-11-14 Re-planned for finding 2, by adding the mechanism and keeping M7.9's words. While the swash's sheet
  rises at a place the foam's drift gains the velocity of its lines of equal thickness, bounded; while it falls,
  nothing, so foam is pushed up the sand and left. The transport keeps what it moves. M7.2 written again, M7.16
  added. Part M: a patch moves 3 m up a 1 in 10 beach in three waves, in steps of 1 and 8 ticks; with the
  current's and wind's drift alone, not at all. Not chosen: narrowing M7.9 to foam born under the sheet.
- 2026-11-14 The second review's suggestions, both taken. The film writes no scene depth, so M4.10's depth line
  is the sheet's alone (M4.10, M7.15). Part J's leak figures are stated with their cases and as examples, not a
  bound for every obstacle.
- 2026-11-14 The plan's note under M5 was removed to keep the plan inside its word limit: the design already
  says the craft and its physics are game code.
- 2026-11-14 Owner, through the lead, overturning three assumptions of this plan: "Scenery reflections we must
  have, that's a main stay in high quality modern games. That's the standard we're after. Camera being able to
  go under water we need to support as well. Breaking waves is something we'd like to have as well."
- 2026-11-14 Re-planned on those words. Replaced: "surf is not planned", "scenery off screen is not reflected;
  no module plans the probe", and "a camera that stays above the water". Kept: one water surface in a column,
  and no caustics, which the owner did not name. Cues W7 to W9 added, with their research and reproductions
  N to Q in [the evidence folder](../../../.orchestrate/evidence/WATERS/design-proof/) (`surf_view_proof.py`).
- 2026-11-14 Surf, decided. Each long wave carries a phase lag solved over the bed (an eikonal first arrival,
  for the lattice's eight directions, wavelengths a factor of two apart and stages 0.5 m apart), a shoaling
  height and an exposure, and a crest takes one breaker section by its breaking index, as Horizon bakes one. The depth
  limit and the swash stand. Not chosen: a shore wave train apart from the sea's own, a pointwise transform
  (crests drift), baked simulations along drawn fronts, and a live Boussinesq window, which stays the route
  if the section cannot reach the look. Tried in part N and dropped: blending stages 1 m apart (14% off).
- 2026-11-14 Mirrored scenery, decided. The scene is drawn again from the camera's mirror image, and the
  water reads it with two fixed-point corrections against its depth (part P: 0.09% median error, where the
  plain planar lookup is 11% off). The draw is a new module's, `SCENE-VIEWS`, with a brief and an interface
  in the module list; only M14 waits for it. Not chosen: a probe (parts a reflection from its foot), ray
  query (experimental on Metal in wgpu, absent from WebGPU), a screen trace alone.
- 2026-11-14 Under water, decided. A lens test for each pixel against the surface's own function, the surface
  drawn two-sided with Snell's window and total reflection, and the water between composed from the gather
  in the water's own pass (part Q).
- 2026-11-14 Milestones. Added M12 (surf on the CPU, after M1), M13 (surf drawn, after M7), M15 (under water,
  after M9) and M14 (mirrored scenery, after M10, on `SCENE-VIEWS:M1`). Existing numbers are kept because
  `CLOUDS` M11 and the briefs name them, so the file's order is the build order. Written again: M8.5, M10.8,
  M10.9, M10.12, M11.2, M11.8 and M11.9; M10.14 added. M1 to M7 and M9 are unchanged, word for word.
- 2026-11-14 M1 is not changed by this. Surf is a capability, so M1's model, items and tests stand without it,
  and M12 extends the crate M1 creates. The section fades out inshore of the draw-back depth so that M1.13's
  closed form holds with surf on (part O).
- 2026-11-14 `SEA` is asked for nothing and its files are unchanged: the transform extends its two crates in
  M13, after its last milestone. `CLOUDS` is unchanged.
- 2026-11-14 Not read for the revision: a frame study of Red Dead Redemption 2 or Sea of Thieves, and Abreu
  2010. Ruessink 2012, Komar and Gaughan's formula and McCowan's ratio are from memory or abstracts; part O
  holds the second.
- 2026-11-14 The lead, on the bar the owner's words of that day set ("Scenery reflections we must have ...
  That's the standard we're after. Camera being able to go under water we need to support as well."): "The
  WATERS re-plan of that day left out caustics, underwater light shafts and exposure that adapts under water,
  which are standard in high-end underwater views; bring the underwater view up to that bar in WATERS (or the
  owning module), and plan the new module SCENE-VIEWS that WATERS M14 depends on. Choose methods from first
  principles and current practice. Do not change WATERS M1, which is being built."
- 2026-11-14 Re-planned on that task. Replaced: "no caustics and no shafts of light", "the exposure scale is
  the sky's, so deep water is dark" and risk 24 as a declared defect. Cues W10 to W12 added, with their
  research and reproductions R to T in [the evidence folder](../../../.orchestrate/evidence/WATERS/design-proof/)
  (`under_light_proof.py`). Still not built, and now an assumption: drops on the lens, particles and bubbles.
- 2026-11-14 Light under water, decided: one caustic field. The surface's light is carried down its refracted
  rays onto thirteen depths a factor of √2 apart, each cell spread over where it lands, blurred by depth times
  the slope the cells do not resolve. What lies under water takes it on its direct Sun in the forward pass,
  so a shadow holds no net. Not chosen: a played texture, one plane for every depth (part R5: r = 0.30 at
  4 m), a backward one-sheet lookup (does not conserve light), ray tracing (not portable).
- 2026-11-14 Shafts, decided: the beam's once-scattered light, with the water type's phase function, as a
  closed form plus a march of what the field and the scene's Sun visibility change, in the water's own
  composition. Not chosen: screen streaks from the Sun, and `ATMOSPHERE`'s view volume, whose Sun path is
  straight. Under water the Sun's path bends once for every reader, so shadows on the bed, the dark under a
  hull and the beams agree.
- 2026-11-14 The meter, decided: V4's incident meter reads the light at the lens. The model gives the factor
  and the game puts it, raised to an adaptation share, in its camera request. Not chosen: a meter on the
  picture. The share starts at 0.75 and is tuned in M16; it is an assumption in the plan.
- 2026-11-14 Milestones. M16 added after M15 with thirteen items. Written again where the task's words reach:
  M15's quality bar, M10.12, M10.14 and M11.9. M14 now waits for `SCENE-VIEWS:M3`, the milestone with cooked
  placements and Chrome, in place of its M1. M1's items and its note are unchanged, word for word, and M16
  asks nothing of the crate M1 builds beyond new functions.
- 2026-11-14 Asks of other modules: [rulings 5 to 7](../ARCHITECTURE.md#seam-rulings-for-waters), a Sun-path
  factor and a lent Sun visibility with `ATMOSPHERE`, and a medium factor on the meter with
  `APPLE-PRESENTATION`. No item of either changes. The mirror section now says the water filters the view's
  one level itself and asks for a rigid camera, as [SCENE-VIEWS](../SCENE-VIEWS/DESIGN.md) lends it.
- 2026-11-14 Not read for the second revision: how Sea of Thieves or Subnautica draw caustics and shafts,
  Wallace's article itself (a search summary was read), and any measured caustic contrast. Cox and Munk's
  slope law and Petzold's mean cosine are from memory.
- 2026-11-14 `RIVERS` was planned ([design](../RIVERS/DESIGN.md)). [Seven rulings](../ARCHITECTURE.md#seam-rulings-for-rivers)
  say what its builder adds in this module's crates: brink ends, flow samples a gauge reads between, two flow
  bands, births from a chart's loss and at a plunge, a region's mean current weighted by its waves, a river
  selection in the example, and a fall's sheet drawn before the gather. Two replace what
  [this design](DESIGN.md#what-the-later-modules-build-on) expected of `RIVERS`: roughness as a gain on the
  wind's cascades, and a reach that keeps its prepared flow whatever its source's stage. No item, test or
  bound here changes, and M9's stand-in channel stays as built.
- 2026-11-14 `PUDDLES` planned. [Seven rulings](../ARCHITECTURE.md#seam-rulings-for-puddles) say what its
  builder adds in this module's crates: a hollows body in the map, a fine interaction level, rain bands, and
  the film drawn over standing rain water. Three replace what
  [this design](DESIGN.md#what-the-later-modules-build-on) expected of `PUDDLES`: a puddle as a basin whose
  stage rain raises, a drop as an impulse record, and a film only on a shore. The question this design left
  it, how basins that overflow merge and part, is settled without changing a basin: two basins that would
  merge are still refused. No item, test or bound here changes.
- 2026-11-14 `SCENE-VIEWS`' design review found that an image holding the air of the whole unfolded path cannot
  be undone by subtracting the camera's lent in-scatter: the phase angle, the Sun's visibility and any fog on
  the camera's stretch differ from the straight line's. Settled with `SCENE-VIEWS` and the module list: a
  mirror view's air starts at its plane, the image holds the water-to-body stretch alone, and the optics
  equation applies the camera-to-water stretch once. The mirror section's units, request and declared
  approximations and M14.2's question now say so. No other item changed.
- 2026-11-14 `PUDDLES`' design review found that a hollows body mirrored about the mean of its ground loses
  the foot of anything standing in a lower puddle. [Ruling 8 for `PUDDLES`](../ARCHITECTURE.md#seam-rulings-for-puddles)
  gives a hollows body bands of levels in the mirror's selection, built by `PUDDLES` M7. This module's rule
  for its own bodies is unchanged; the same cut applies to a channel that lies below the lake it shares a
  plane with, which M14.5's inspection of the channel's source end is the place to see.
- 2026-11-14 `RIVERS`' design review replaced the weighted mean current of its ruling 5 with a current ladder
  in the interaction step, and gave a chart's samples a wetting gauge (ruling 2). Both are built by `RIVERS`
  behind declarations no scene of this module makes.
- 2026-11-14 `SCENE-VIEWS`' second design review settled what a mirror view's air is held to: `ATMOSPHERE`'s
  aerial product for an observer where the line crosses the plane, drawn by the main view from that point
  ([its design](../SCENE-VIEWS/DESIGN.md#what-a-views-air-is-held-to)). M14.2's water-to-panel stretch can
  take its expected value the same way. `RIVERS`' second review had the solve write a chart's wetting gauges
  from each inflow as well as from the lower sample's water (ruling 2). No text, item or bound here changed.
- 2026-11-14 `SCENE-VIEWS`' third design review: air from the plane now needs a level plane, and a view's
  volume is `ATMOSPHERE`'s for an observer at the plane's height
  ([its design](../SCENE-VIEWS/DESIGN.md#what-a-view-owns-and-what-it-shares)). A mirror's plane here is a
  body's level, so no text, item or bound here changed. For M14's builder: near sea level `ATMOSPHERE`'s
  product thins haze just above the horizon, in the main view and in a mirror alike
  ([part F](../../../.orchestrate/evidence/SCENE-VIEWS/design-proof/README.md)), so M14.2's panel wants a line well above it.
- 2026-11-14 `ATMOSPHERE` planned its M9, [the air at the ground](../ATMOSPHERE/DESIGN.md#the-air-at-the-ground):
  the aerial pass carries a line of sight on under the model's ground, so the sea and whatever stands just
  above its horizon take the haze of their own lines. M2 now waits for `ATMOSPHERE:M9`, because M2.2 and M2.3
  judge the horizon and the far sea's haze from 2 m and 40 m, where the old stop took up to 72% of the sea's
  in-scatter, in bands that moved with the camera. No text, item or bound here changed. The note above for
  M14's builder holds only until M9 is Done.
- 2026-11-14 [The design review of the surf, mirror and underwater revisions](../../../.orchestrate/evidence/WATERS/design-review.md)
  asked for changes, three Must Fix. Re-planned; M1 is untouched. (1) Water that land encloses has no path to
  the bed's edge, so its long waves now start at its own waterline where that faces the wave, and their
  exposure is the share of the declared fetch upwind; the sea's rule and its lee shelter stand. M12.1 says
  which water its edge rule covers, M12.11 is new, and M13.1's bed gains such a lake. (2) Under a thrown
  crest the query, the model and the lens find the highest surface by a bounded scan of the throw, held to
  a cut of the section made in the test; M12.6, M13.3 and M15.1 rewritten. Continued below.
- 2026-11-14 The same re-plan, continued. (3) The two blends together are 4.1% and 9.3% out against the
  dispersion relation, and the first-order solve adds up to 6%, so M12.2 now holds the blend to a lag solved
  for the component's own wavelength and stage, within 7% and 13% where the design measured 5.0% and 8.0%,
  and a component with no blend must fail. Suggestions taken: the foam's conservation is scoped to cover
  under the cap, and the caustic mask's use of the level is declared. The figures are parts U to W of
  [the design proof](../../../.orchestrate/evidence/WATERS/design-proof/README.md).
- 2026-11-14 [The fifth design review](../../../.orchestrate/evidence/WATERS/design-review.md) asked for changes, one Must Fix:
  the scan of the throw inverts a straight section and not the assembled sheet, where a surface arrives from
  beside the line through the point and from an envelope larger than the point's. Re-planned; M1 is untouched.
  The highest surface is now a cut of the sheet: forward evaluation on a 0.25 m lattice of rest places fixed in
  the world, over the cells within a throw reach that needs no wave evaluated, refined on the assembled map.
  Part X of [the design proof](../../../.orchestrate/evidence/WATERS/design-proof/README.md) holds it to a cut eight times
  finer on three sheets, the review's among them. Continued below.
- 2026-11-14 The same re-plan, continued. M12.12 is new and takes the highest surface from M12.6; M13.3 and
  M15.1 put their lip under two crossing trains; M12.8 and M13.12 record the cut's cost, about 1,200 to 5,500
  nodes a point where the scan took 40; M13.2 names the bed's pyramid. The breaker's look, the lags and every
  seam with `SEA`, `RIVERS`, `PUDDLES`, `SCENE-VIEWS`, `ATMOSPHERE` and `CLOUDS` are unchanged. Neither
  Suggestion was taken: M16's waterline crossing and a lake's surf by eye stay as the review left them.
- 2026-11-14 M11 now waits for `APPLE-PRESENTATION:M7` and `M8`, the governor and MetalFX, which have done-when
  entries from today. M11.1 is unchanged. The design is unchanged.
- 2026-11-15 Built M12's CPU lag ladder, sector exposure, shoaling, breaker section and highest-sheet cut,
  with optional surf on the existing map and shared join weights. No GPU surf or game content was added.
  [CPU proof](../../../.orchestrate/evidence/WATERS/M12.md) records 59 passing model tests, strict Clippy and independent
  Snell, fetch, swash and fine forward-cut references, including the actual prepared surface and reach.
  [Cost](../../../.orchestrate/evidence/WATERS/M12-preparation.md) records preparation bytes/time and six column probes.
  Earlier incomplete baseline captures remain diagnostic history; the current shared main-branch
  `exact-images` check supplies M12.10's proof after integration. GPU surf and its appearance remain for M13.
- 2026-11-16 M12's post-merge gate exposed a watcher publication timeout. Added bounded stat-cache
  reconciliation to recover delayed or absent notifications, retaining the original debounce and test deadline.
  [Recovery proof](../../../.orchestrate/evidence/WATERS/M12-watch-recovery.md) records the native trace, no-notification
  edit/add/remove/idle regression, cooker regressions and strict Clippy. The water model is unchanged.
- 2026-11-16 Reconciled M12's retained watcher regression with creator's landed shared fixture and adaptive
  sweep API after the merge gate found duplicate helper types. The original ten-second watchdog remains.
  [Integration proof](../../../.orchestrate/evidence/WATERS/M12-watch-recovery.md#integration-with-the-creators-sweep)
  records eight watcher tests, four server/watch tests and strict cooker Clippy passing on the Mac.

- 2026-11-16 M2 stopped on M2.2: from 40 m up the crests were round and a smooth belt lay under the horizon, after
  eight tuning attempts ([handoff](../../../.orchestrate/evidence/WATERS/M2-handoff.md)). The lead's task was to settle the approach
  at the same bar, preferring what current engines ship. The planner read the branch's shaders and found two causes
  in the representation, not the tuning. The long-wave cache stores the map inverted onto a fixed world grid, and
  vertices move only vertically, so a crest turns over a texel. Every layer is read from storage buffers at mips
  0, 4 and 8, chosen by the footprint's longest side, which collapses the sea to one variance about 600 m out.
- 2026-11-16 Parts Y and Z of [the design proof](../../../.orchestrate/evidence/WATERS/design-proof/README.md) measure both. Part Y
  predicts the belt 61 rows under the horizon, where the opinions saw 50 to 70. In part Z, the forward form draws
  a 53 m crest within 1.3 to 2.3 times its width, where the world grid draws it 8 to 28 times too wide.
  Decided, in [the surface](DESIGN.md#the-surface-and-the-frame-contract): layers are drawn forward from the rest
  place, read as mipped LEAN textures through anisotropic filtering, keep everything they drop as covariance, take
  far light from the slope distribution's BRDF with no ground-side sky, and carry the thin-crest term from M2.
- 2026-11-16 The bar stands. Its "band" now has the meaning M2.2's inspection gives: a strip with an edge or a tone
  step, which excludes the last rows, where only statistics are physical. M2.10 is added, and `SEA`'s cache test
  changes its oracle to the forward evaluation, under the same bounds. M2's other items are unchanged, word for word.
  Risk 30 covers devices that admit less anisotropy. Not chosen: moving the long waves into FFT cascades, because
  the analytic layer is not what failed.
- 2026-11-16 The M2 continuation put M2.10 before the next M2.2 inspection. Its integration notes identified
  the concurrent `thrum-app` edits in `CREATOR` M2 and M3 and the `thrum-sea` transform repack in
  `REVIEW-ENGINE` M5.
- 2026-11-16 Built M2's open-sea contribution, ticking water example, depth-writing late pass and capture/cost tools.
  Mac Metal probes verify submitted depth, CPU-model motion on tick/repeat frames, history resets and live stage changes.
  Corrected coherent Fourier phases, shared Fresnel/exposure composition and world-space cascade normals.
  Three fresh builder opinions still find rounded crests; the design's three-look stop rule was reached.
  The profile diagnostic identifies an untried larger permitted choppiness, not a proof that the model cannot work.
  [Evidence](../../../.orchestrate/evidence/WATERS/M2.md) separates that visual finding from functional and device proof.
- 2026-11-16 Resumed M2 at `69a7ba617`; its scene, frame-contract and stage checks pass again on Metal.
  Preserved the selected rounded-crest failure and verified recovery from the local evidence store.
  The first paired cost run failed with missing GPU durations and timing faults; its cause is unconfirmed.
  No fourth visual attempt was made under the design's three-look stop rule.
  [Handoff](../../../.orchestrate/evidence/WATERS/M2-handoff.md) asks the lead/planner for direction and records both issues.
- 2026-11-16 The owner directed M2 to continue after the three-look stop. Testing the existing profile's
  larger permitted long-wave choppiness, 0.9 with the same 0.45 contraction budget.
  Diagnosed the missing cost times as Metal's invalid end timestamps on empty Sun-shadow passes.
  The paired scene now keeps the same calibration float and sky in both arms at the same world times;
  only water differs. Empty-caster timing reporting is a renderer follow-up, not a zero-duration claim.
- 2026-11-16 The larger choppiness and a developing-wind fetch each failed fresh opinions on crest shape.
  Diagnosed the spectrum's broad swell weighting: it spread its energy into crossing directions.
  Corrected directional weights inside the existing spectrum model, conserving each band's variance and
  swell height on rotation; the wind, swell, component count, phase identities and GPU evaluator remain shared.
  The angular regression and spectrum's independent energy checks pass; the next submitted look judges quality.
- 2026-11-16 Continued the numerical diagnosis under the owner's carry-on direction: the CPU oracle already
  admits contraction below one, but the GPU's old 24-iteration cap forced a 0.5 restriction and q≤1.
  Expanded the existing profile to q≤4 and contraction below 0.9, with a 256-iteration cap and the same early exit.
  Native and portable Metal probes at contraction 0.89 match the independent oracle; the old cap fails a
  near-crest slope control. The example selects a 0.85 bound. No wave mechanism or CPU oracle was replaced.
- 2026-11-16 Tightened that numerical account: the shared CPU model now owns the largest eigenvalue of
  sum(q*a/k · k*kᵀ), a uniform bound on every phase's symmetric displacement differential.
  This retains contractivity while avoiding the trace's false rejection of orthogonal trains.
  A separable crossing-wave closed form passes on the CPU; native/portable GPU probes include that case.
  Water's admission and the example's profile use that same authority, without an allocation in frame validation.
- 2026-11-16 Corrected the geometry-to-BRDF transition: sky reflection and transmission now use the visible
  unresolved slope distribution's mean dielectric reflectance, instead of a perfectly flat facet at the horizon.
  The full short-wave covariance is transported through the horizontal map before that average.
  A Metal probe agrees with independent Box-Muller integration and rejects the flat-mirror negative control;
  the zero-variance limit and lent record layouts pass. The same lent sky, Sun and exposure remain the only light.
- 2026-11-16 The paired diagnostic exposed 34–43 ms long-wave passes: fixed-point iterations were stalling
  at binary32's phase/coordinate precision after the cap increased. Reduced the base phase before adding the
  local inverse offset and stopped at a 10 micrometre plus relative residual. Compressed native/portable
  probes remain below 0.011 mm height error and 0.000059 slope error, with the old-cap control still failing.
  The scene now orbits one declared inspection area, so both wind bearings face the same wave train.
- 2026-11-16 Current source `ef259d1f1` passes scene, frame-contract, stage and paired-cost Metal checks.
  Fixed shared-light inspection and fresh elapsed-motion opinion support M2.3 and M2.5; M2.7 now has valid timings.
  Stronger swell and delayed cache LOD still fail fresh opinions on crest shape and the elevated horizon belt.
  M2.2 remains open; proposing planner diagnosis of geometric/cache-to-statistical transition without lowering its bar.
  [Handoff](../../../.orchestrate/evidence/WATERS/M2-handoff.md) retains attempts, current proofs and the unmerged checkpoint.
- 2026-11-16 Built the approved forward rest-sheet cache and full filterable LEAN pyramids, with 16x
  anisotropic reads, omitted-wave covariance and sky-only facet reflection. Native/portable texture errors
  stay below 0.932 mm height and 0.000122 slope; [proof](../../../.orchestrate/evidence/WATERS/M2-forward.md).
  Fresh opinions pass the named crest/horizon/light cues after height/compression crest in-scatter;
  rectangular near-field highlights remain a disclosed limitation. The motion proof selects jittered TAAU.
  [Cost](../../../.orchestrate/evidence/WATERS/M2-cost.md): forward cache about 0.354 ms, above the 0.2–0.3 ms estimate;
  qualified cadence and remaining optimization stay with M11. Integration runs the full project gate.
- 2026-11-16 Extended M2.10's readback to SEA's 20 m/s envelope: single-half displacement missed the
  height bound by 6.17 mm. Paired high/residual displacement in the same five texture planes now keeps
  every texel below 0.0392 mm height and 0.000244 slope on native/portable Metal, at both ages and winds.
  Full mip averages retain the residual carry; optional float16 arithmetic is unnecessary. The new
  frame-contract probe passes. Allocation is unchanged; [paired cost](../../../.orchestrate/evidence/WATERS/M2-cost.md)
  records the extra vertex read. The queued pre-correction merge was withdrawn before resubmission.
- 2026-11-17 Integrated M2's three remaining precision commits on current main after confirming the
  earlier checkpoint was already present under rewritten hashes. The full `./dev check` passed and
  `fa59e2679` merged. Refreshed six fixed views and a 310-frame realtime pan on that merged source,
  with zero omissions; [final evidence](../../../.orchestrate/evidence/WATERS/M2-merged.md) records recovery and inspection.
  Rectangular foreground highlights and concurrent-load cost tripwires remain disclosed; physical
  cadence stays unqualified. The script owns the milestone's next status and review.
- 2026-11-17 Corrected the first review's near-water lattice by reconstructing magnified LEAN moments
  with positive cubic weights. A Metal probe matches independent convolution, retains covariance and
  leaves minified reads unchanged; [proof and fresh opinion](../../../.orchestrate/evidence/WATERS/M2-reconstruction.md).
  The first new look passes the six spatial/light views. A one-second observer now fits its existing
  queues without writer progress; its 94-frame pan has zero omissions and sampled motion passes.
  Surface sampling costs about 26% more in the concurrent-load paired run; the named tripwires and
  M11 follow-ups are in [cost](../../../.orchestrate/evidence/WATERS/M2-cost.md). No physical cadence is qualified.
- 2026-11-17 Merged the reconstruction correction at `387d5a69d` after the full project gate passed.
  Rebuilt and captured that merged source: all six fixed views are pixel-identical to the fresh-opinion
  candidate; the final bounded pan has 101 frames and zero omissions. [Selected proof](../../../.orchestrate/evidence/WATERS/M2-r1.md)
  records source comparisons, recovery and inspection. Independent review and physical cadence remain separate.
- 2026-11-17 Built M3's Chrome route, hidden-world clock, graphics recovery and bounded submitted-output tools.
  Hardware Chrome passes the first-frame and seven-checkpoint CPU-oracle trace, including real tab visibility
  and destruction/replacement of its device, with no validation faults. [Numerical proof](../../../.orchestrate/evidence/WATERS/M3.md)
  keeps these target results separate from the fixed/motion inspections recorded after integration.
  The browser check is `scripts/check_water_browser.mjs`, following `scripts/check_v1_browser.mjs`.
- 2026-11-17 M3 merged at `07cecc854` after the full project gate passed for its integration batch.
  Captured four Chrome and six portable Metal fixed views, plus 62 contiguous Chrome pan frames with no omissions.
  Builder and fresh visual inspection pass the named cues; the clip's phase-sampling limit and page clock remain explicit.
  [Final proof](../../../.orchestrate/evidence/WATERS/M3.md) records pixel comparisons and verified archive recovery. Physical cadence is unqualified.
- 2026-11-17 Corrected M3's reviewed resume clock: pause discards the partial tick, and resume holds the
  committed endpoint until a new full tick commits. Rust regressions and ten hardware Chrome field checkpoints
  pass, including independent hold/commit expectations, a hitch and device replacement, with no validation faults.
  [Resume proof](../../../.orchestrate/evidence/WATERS/M3-resume.md) records exact source/Wasm identity and numerical bounds.
  The passed visual judgments remain scoped to their retained captures; physical cadence remains unqualified.
- 2026-11-17 M3's resume correction merged at `e5de7027e` after the full project gate passed.
  [Fresh submitted output](../../../.orchestrate/evidence/WATERS/M3-resume-visual.md) matches all retained Chrome and portable
  Metal fixed pixels exactly. All 53 consecutive Chrome pan crops were inspected, with zero capture omissions.
  A 99.9 ms page callback gap and 116.6 ms submission gap are recorded separately from field continuity;
  neither this capture-ON run nor its page clock qualifies physical cadence.
- 2026-11-17 Diagnosed M3.5's shared-check wait: Chrome capture failed before rendering because the
  repository-wide source listing exceeded Node's output buffer. The driver's owner already merged
  `8c6dc8847`; a CPU check confirms its scoped list preserves every hashed path at both historical commits.
  [Follow-up](../../../.orchestrate/evidence/WATERS/M3-exact-check-followup.md) separates that verified cause and fix from
  the shared runner's still-pending coverage proof. No baseline, comparison or WATERS runtime changed.
- 2026-11-17 Diagnosed the clean-checkout M3 gate's unstable rustc incremental fingerprints.
  Both verification launchers now disable the incremental query cache; other commands retain their setting.
  Chrome first-frame and lifecycle checks pass, and `clock_spans` executes without the compiler panic.
  The builder's full gate reached later GPU tests before its 600-second limit; the merge gate proves integration.
  [Compiler-cache evidence](../../../.orchestrate/evidence/WATERS/M3-compiler-cache.md) separates these results and limits.

- 2026-11-17 Human: "Ok let's lower the bar." The most plausible result as fast as possible, not physical accuracy; references become
  diagnostics. Plan, for M4 onward (M3, Ready for Review, is untouched): dropped M6.1, M6.2, M7.1, M7.16, M8.8, M15.2 and M16.3, which
  held CPU references and optics equations to closed forms. Rewritten without reference bounds: M4.1, M4.10, M6.3, M6.7, M6.11, M6.13,
  M7.2, M7.11, M7.13, M7.15, M9.1, M9.2, M13.1, M13.10, M14.1, M15.7, M16.1, M16.2, M16.4 and M11.4. Kept: the map, the long waves
  and the body query against the model, the frame contract and lifecycle. Ten cost records pass on their check alone. The design's
  handoff states the bar. No item of a Done milestone changed.

- 2026-11-18 Design review of `8de6cef92` asked for six changes; all taken, under the owner's bar (no item holds a look to a reference).
  M16.1 no longer asks the caustic field to fade with depth (its contrast rises; the receiver's falls), and new M16.14 holds the bed's
  shadow and the shafts' to one Sun-visibility point, named in M16.10. New M8.12 and M15.11 test exposure and each medium applied once.
  New M6.15, and M6.11 in Chrome, hold a query's wake height to the wake's own state. M7.13 states a hitch as one elapsed step, ageing
  included. M14.1 and M14.8 keep only the flat-water projection; M14.4 judges attachment as wind rises. The cue table marks diagnostics
  (the review's Suggestion). The see-through ray's down-path rate now reads `K`, as under water. M3 and Done milestones are untouched.

- 2026-11-18 Design review of `0fa0fb62710a` passed five of the six earlier findings and kept one: M16.14 found the shafts' shadow as
  water that "stores no marched light", which a correct march fails, because under a blocker the marched part is negative (−0.24 of
  `J_s p` at the item's 6 m and 20° Sun) and zero beside it. M16.14 now finds that patch, in a calm, as the water just above the bed
  that the shaft pass leaves darker than with the box taken away, and M16.10 names the same criterion for Chrome. The design states
  that the marched part is signed and kept signed to the sum. No brightness or displacement is held to a number. The review made no
  Suggestions. No other item changed.

- 2026-11-18 M3 now reads its browser frame clock directly from successful submissions.
  Hardware Chrome passed 11 oracle checkpoints and agreed with captured submission 632 after the
  completion history filled. The runtime merged at `e9a449b08` after `./dev check` passed.
  [Target evidence](../../../.orchestrate/evidence/WATERS/M3-frame-report.md) includes four new Chrome/Metal fixed
  pairs and a 62-frame Chrome pan on the merged runtime. Callback timing measures the page clock.

- 2026-11-18 Built M4's camera-independent map sampler, indexed body draws and the island's sea, lake and pond.
  Metal checks passed for map and gains, pixel ownership and hidden overlap, stage and history, and the submitted
  swash cut. Hardware Chrome drew all three bodies and passed the f64 map comparison inside its binding limits.
  [M4 evidence](../../../.orchestrate/evidence/WATERS/M4.md) links fixed and realtime inspections and the paired island cost.
  The thin sheet blends directly into the opaque colour. Refracted lookup and qualified device timing are later work.

- 2026-11-18 M4.6 compares submitted coverage against an opaque-depth control at the sill and bank.
  The Metal control confirmed that reset history makes dry ground reactive. The lake sweep still checks the
  submitted level and motion across the stage edits. [Target run](../../../.orchestrate/evidence/WATERS/M4-checks.txt).

- 2026-11-18 M4 review fixes sample stage and sill connectivity at presentation time and keep the
  previous submitted level for motion. Metal passed 40 fractional-stage samples. The calm shoreline
  uses an optical path fade at the depth cut and a rounded, locally refined pond bank.
  [Target captures and stage series](../../../.orchestrate/evidence/WATERS/M4-review-fixes.md).

- 2026-11-18 Built M5's tick query, mapped mailbox and example-owned hull dynamics.
  Metal passed query, expiry, refusal, draft, gale, sheltered-lake and current checks. Hardware Chrome
  delivered body replies and counted the unavailable station uses across a deliberate hitch.
  [M5 evidence](../../../.orchestrate/evidence/WATERS/M5.md) links twelve elapsed-time sequences and the capture-off cost.
  Empty Sun shadow timestamps prevented a complete native or portable GPU timing report.

- 2026-11-19 Preserved accepted query pages across catch-up frames and sampled authored stages and chart slopes at the use tick.
  The craft follows the encountered waterline rate. Metal passed ten minutes of the scene gale at each commanded speed.
  Chrome reconciled individual reply ages through the hitch. The Metal candidate records the wider beam-sea buoy course.
  [Review fixes and target evidence](../../../.orchestrate/evidence/WATERS/M5-r1.md).

- 2026-11-19 Recompose in-flight replies when a stage edit changes the use-tick map. Metal covered an edited chart,
  a late reply for the prior stage and a newly supported swash point. The launch has more freeboard.
  Twelve submitted Metal sequences and the extended gale clip show the deck clear through the reported crests.
  [Merged-build evidence](../../../.orchestrate/evidence/WATERS/M5-r2.md).

- 2026-11-19 Built M6's regional dispersive solver, submitted endpoints, body reactions and impulses.
  Metal verified isolation, current, impulses, frame lifecycle, craft draft, float response and query height.
  The lake views centre an untracked impact using any admitted tick of a hitch. Chase and overhead clips
  cover both speeds, a turn, a stop and restart, trade wind and the lake. The cost record includes both
  1440p views and the sea timestamp faults. Hardware Chrome verified the float, surviving wake, lake rings
  and 29 public query samples. [Target evidence](../../../.orchestrate/evidence/WATERS/M6/README.md).

- 2026-11-19 M6 review fixes tie the query's dent to admitted endpoint pressure and keep queued impulses until submission.
  Metal verified zero height for a refused source, the earliest of 33 future drops, and discard before initial publication.
  All eight floats near the moored craft measured under 0.24 mm of waterline variation after settling.
  The 1440p cost record adds water-absent arms and compares the two water-present arms with the prior same-view record.
  The sea timing diagnostic confirmed stale shadow end counters. [Target evidence](../../../.orchestrate/evidence/WATERS/M6/README.md).
  The endpoint test suite uses one Metal device at a time and collects no pass timestamps for its correctness assertions.

- 2026-11-20 M6 adds back the moored craft's static pressure from whichever wake bands admitted it.
  On Metal the settled draft moved 0.38 mm with the fine level alone and 1.16 mm with the coarse level alone.
  The body used all six public query replies on each step. The existing Metal tests also passed for the float,
  the island's isolated waters and the settled waterlines around the moored craft.
  [Target evidence](../../../.orchestrate/evidence/WATERS/M6/README.md).

- 2026-11-20 Built M7's transported foam stocks, shore film and cooked pattern lease.
  Metal verified endpoint rollover, drift, ageing, dry ground depth and pattern editing through device
  replacement. Hardware Chrome measured whitecaps and the trail after craft removal.
  The paired 1440p cost record retains the sea timestamp fault and completion brackets.
  [Target evidence](../../../.orchestrate/evidence/WATERS/M7/README.md).
- 2026-11-20 Planner, from the cleanup review (L8-07): M11.4 and M11.10 named files under `docs/evidence`, which the merge refuses; they now name this milestone's evidence and the provenance record in the module's evidence folder. Where provenance records are kept in Git is an open assumption in [DEAD-TOOLS' plan](../DEAD-TOOLS/PLAN.md). M11 was Not Started. [Track 6](../ARCHITECTURE.md#track-6-the-cleanup-of-20-november) moves the ten-minute gale and foam runs out of `./dev check`; no item of this module changed for it.
- 2026-11-20 Planner, on the owner's decision of that day as the lead gave it: provenance records are tracked in Git. M11.10 now names the module's provenance record tracked under `docs/provenance`, where `DEAD-TOOLS` M5 puts it. The design still names the working-folder file; `ORCHESTRATE.md` says that means the tracked record, and the design is corrected at this module's next re-plan.

- 2026-11-20 Fixed M7's shoreline wet support, air-to-film transfer, stranded foam coverage and powered stern trail.
  Metal read dry columns with zero births, drift and diffusion, and verified reactive coverage over unchanged
  ground depth and motion. Both Metal and Chrome retained the moving craft's trail after source removal.
  Captures show open turquoise between stern foam and a receding sheet leaving broken lace over the sand.
  A bounded wave inverse cut the direct film pass's measured cost. The 1440p record retains the remaining
  tripwires under shared load. [Target evidence](../../../.orchestrate/evidence/WATERS/M7/REVISION.md).

- 2026-11-21 Fixed M7.8's squared turquoise halo around the moving craft. A Metal readback showed the bow of the
  trailed craft at the 32 m fine window's front edge, where 2 m coarse cells drew the box. The foam windows are
  now 256 cells, so the fine level spans 64 m and holds the craft in the chase and 40 m views. The aerated tint
  is also broken by world-space bubble clouds. Verified on Metal: the foam suites pass and captures at both speeds
  and headings show no straight-edged tint. Not verified: the whitewater step's cost rises with the four-fold
  cell count, which the M7.12 record retakes. [Target evidence](../../../.orchestrate/evidence/WATERS/M7/HALO.md).

- 2026-11-21 Fixed M7.7's straight line where gale whitecaps met the coarse foam window. Past that window the
  estimator now carries the open-sea mean the coarse level holds, measured each step in a ring inside its edge,
  and hands over across 80 m rather than 28 m. Gale cover had also saturated near 73%; a breaking onset of 0.76
  holds it near 43%, with 36% in a strong breeze and 14% in a trade wind. Verified on Metal: all foam suites pass,
  and 40 m and 2 m captures show no line in any wind. Chrome's foam check passes. Design proposal: the design's
  stateless far estimator now carries that one measured mean per endpoint.
  Target evidence: `.claude/worktrees/waters/tmp/waters-m7-r4/README.md`.

- 2026-11-21 Fixed M7.7's steady gale, which saturated into a lace sheet and a white far sea. Only the most
  compressed crests now break, at a strong rate; fresh film spreads faster, and dense foam varies in density.
  Verified on Metal over 72 s: steady fine-level cover is 15% in a gale, about 9% in a strong breeze and 2% in a
  trade wind, ordered at every reading. Caps stand on open water from 2 m and 40 m, and the far sea is darker than
  the sky. The foam suites pass, and the trail and beach captures are unchanged in kind. Design proposal: the design's
  stateless far estimator now carries one measured mean per endpoint (earlier entry, open).
  Target evidence: `.claude/worktrees/waters/tmp/waters-m7-r5/README.md`.

- 2026-11-23 M13 paused at `0fa2c501e` on `build/waters`. The builder's handoff in
  `.claude/worktrees/waters/.orchestrate/evidence/WATERS/M13-handoff.md` records a 40 m wave with wet depth-limit
  loss below breaking index 1. M13.6 now covers that case and the breaking case. M13.14 records the fast-tier
  trail test requirement after the hourly workspace timeout. The isolated builder-branch run took 7.19 s.

## M17 workspace timing baseline

- 2026-11-23 These are the cases from the passing `747909d1c` workspace check, with seconds from that run.

| Seconds | Test |
|---:|---|
| 9.643 | `thrum-sea::it forward_cache_gpu::forward_fields_and_full_lean_pyramids_match_the_model_portable_high_wind` |
| 9.176 | `water-scene::it inland_floating_gpu::lake_remains_sheltered_in_the_gale_and_settles_in_calm` |
| 9.163 | `thrum-sea::it forward_cache_gpu::forward_fields_and_full_lean_pyramids_match_the_model_portable_light_wind` |
| 8.859 | `thrum-water::it query_gpu::chart_queries_keep_both_tick_pages_and_sample_their_own_stage` |
| 8.547 | `thrum-water-model::it shore_basin::enclosed_water_has_finite_lags_turning_and_exposure` |
| 8.456 | `water-scene::it inland_floating_gpu::query_current_carries_a_float_and_the_dry_bank_stops_it` |
| 8.048 | `thrum-water-model::it surf_cut::production_sheets_and_their_own_reach_match_forward_reference` |
| 7.425 | `water-scene::it scene_gpu::scene_ticks_and_declares_one_water_contribution` |
| 7.146 | `water-scene::foam_scene_gpu lake_beach_births_follow_its_share_of_lost_power` |
| 6.717 | `thrum-water::it foam_gpu::a_regular_wave_carries_foam_up_a_plane_beach` |

- 2026-11-23 M17 design review, Must Fix 1. Removed the proposed fast portable GPU case from the design and
  M17.3 because the two full sweeps already catch its faults. M17.3 checks that both sweeps keep their full coverage
  and move to the long tier. M17.4 still requires both cases to pass on main with the repair included. Took the
  provenance-path suggestion so the builder handoff points directly to the tracked record.

- 2026-11-23 M17 retained the basin and three production surf oracles in compact fixtures. CPU force tests keep
  the drift, bank and settling cases, with live Metal queries and submitted film and contribution checks. The
  focused batch passed all ten cases on its first run, with a 3.219 s maximum. Both unchanged portable sweeps
  passed in the release long tier on the branch.
  [Fast-tier evidence](../../../.orchestrate/evidence/WATERS/M17/M17.1-2-fast-tier.md).
  [Portable sweep evidence](../../../.orchestrate/evidence/WATERS/M17/M17.3-long-tier.md).

- 2026-11-23 M17's run started with the worktree still carrying M13's local `in_progress` status and evidence
  from `576a7a69c`. The switch to `build/waters-m17` restored M13's entry from main, so the run's guard reported
  a change outside M17. M13 on main is `not_started` with open items, as authorized in `896ab42c9` and present
  at the M17 design review in `1adcbf197`. M13 has no recorded milestone review. Its unmerged progress remains
  on `build/waters` at `2eda168cb`. Start a new M17 run from `build/waters-m17` so it takes the authorized M13
  entry as its starting point.

- 2026-11-24 The official Mac Metal `long-tests` run passed on main at `d10ed33175dd`, which contains the
  M17 repair. Both full portable sea sweeps passed in 3.43 s combined. The sprite inventory check also passed
  after the RENDER-2D fix. [Covering run](../../../.orchestrate/evidence/WATERS/M17/M17.4-main-long.md).

- 2026-11-24 The craft-trail guard reached the ten-second limit both in the workspace batch and alone
  at `299376222cb1`. It already timed out in the batch at `747909d1cd25` and retried in 5.112 s.
  The cost is the number of full wake-scene submissions. The M13 branch's 50 ms test tick kept all
  five submitted-stock reads through 60.1 s and passed in 1.827 s. M18 brings that fixture forward.
  [Metal trail run](../../../.orchestrate/evidence/WATERS/M13/trail-tick.log).

- 2026-11-24 M18 reduced the craft-trail test from 692 submissions to 182 with a 32 × 24 target and a 50 ms
  test tick. Metal verified all five submitted stock readings through 60.1 s with zero dropped foam ticks.
  The unchanged cover, width and peak bounds passed on the first attempt in default Nextest, in 2.532 s
  alone and 3.405 s within the foam binary under builder load.
  [Target evidence](../../../.orchestrate/evidence/WATERS/M18/README.md).

<a id="wp13-restart-decision"></a>

- 2026-11-24 Keep the approved phase-lag transform, breaker section and bounded, contour-driven swash. On
  `build/waters` the retained 16.0–17.8 s cut at `z = 0 m` shows two shoreward crests closing from 65.03 m
  to 55.49 m while both lose height. The narrow-swell take hid the second ridge in glints and showed a
  doubled lip. Diagnose the approach and the pitching lip separately.
  [Retained profile](../../../.orchestrate/evidence/WATERS/WP13/review1440-retained-profile.log) ·
  [Three takes](../../../.orchestrate/evidence/WATERS/WP13/bounded-swash-iteration.md)

- Tune the incident spectrum and start time of the island's roughly 10 s, 45° trade-wind swell to show two
  consecutive coherent ridges of unequal height. Track both across neighbouring shore-normal cuts. Read
  turn, normal spacing and each crest's elevation and trough relief before it breaks. Compare the transformed
  wave before the depth limit, after the limit and on the final sheet. Give refraction and shoaling room on
  the exposed shelf before depth limiting dominates. If reshaping the shelf, prepare the bed from the same
  terrain the scene draws. If more rise is needed, lift and sharpen the crest smoothly in the pre-throw rows
  of the existing section. Keep the phase, exposure and envelope-driven break distance.

- Compare a fine forward cut with submitted triangles at the same crest, time and stage, including a ring
  transition. If the cut is intact and the draw splits or facets, refine the mesh locally in the pitching
  band through the tested tide and sea states. Include the lip's reach, stitch in rest coordinates and sample
  both presentations for motion. Reuse the portable field cache. If the cut itself doubles, ease competing
  long-wave chop and smooth the throw ramp in the shared CPU and WGSL section. Keep a genuine overhang, the
  highest-surface query, connected bore, wet edge, wet lost-power foam source and the plane-beach regular-wave
  run-up and draw-back. [Guerrilla's wavefront tessellation](https://advances.realtimerendering.com/s2022/SIGGRAPH2022-Advances-Water-Malan.pdf)

- Start with submitted 2560 × 1440 trade-wind motion on portable Metal. Frame both ridges and the break zone
  from 40 m, and read rise and pitch from 2 m. Use an oblique view and Sun angle that separate the face from
  the trough. Judge the unmarked motion and use the cuts to diagnose it. Then complete the existing gale,
  offshore, foam handoff, lee, tide and matched Chrome inspections. Refresh the affected query and frame
  checks and paired cost record. If the controlled set and resolved mesh expose a remaining defect in the
  shared sheet, return the matched cuts and captures under the three-look rule to reconsider the live window.

- 2026-11-24 M13 design review, Must Fix 1. The 1 km camera's coarse foam window ends before the prepared bed.
  The whitewater design and M13.7 keep that view for distant surf and use a separate view of a breaking front
  crossing the coarse window's outer edge to judge the handoff. M13.11 requires both views at matching
  trade-wind settings on Chrome and portable Metal and retains the submitted-time receipt.

<a id="wp13-approach-decision"></a>

- 2026-11-25 WP13.4 blocked again after three Metal looks could not follow two ridges through both turning and
  narrowing. Keep the shared sheet. The live window is not selected for this cue: it takes the same swell at
  its border and draws the same approach, and as a height field it holds no thrown lip.
  [Reasons and figures](../../../.orchestrate/evidence/WATERS/WP13/approach-decision.md)

- 2026-11-25 The cause is the swell's scale. At 10 s and 1.5 m the two ridges steep enough to see differ by 2°
  and close by 7 m, and the ridge that turns and closes most is hidden by the wind sea. WP13.4 now takes a
  period inside the cue's 6 s to 14 s and judges three ridges in one frame. A 6 s to 7 s, 2 m swell is the
  starting point. This is the last look at the approach: a further miss goes to the lead as a question about
  the item or the bar.

- 2026-11-25 WP17's and WP18's builder notes left the plan file with their reviews passed. WP17 landed the
  fixture reductions ahead of the long tier and recorded each case's time; WP18 brought the compact Metal trail
  fixture forward from `52345b514`. Their designs and the entries above hold the detail.

<a id="wp19-query-decision"></a>

- 2026-11-25 Design review, Must Fix 1: the body query in a gale's breakers took 18.9 s a tick, so no running
  game could use it. Added WP19 after WP13, with the query's age contract as its criterion: ticks on the wall
  clock, the eight floats in the breakers, no answer unavailable. The design now makes the cut coarse to fine
  and says the two passes are the mechanism, with a risk, a cost row and a `water_cost.py running` command.
  WP13's items are unchanged.
  [Review](../../../.orchestrate/evidence/WATERS/design-review.md)

- 2026-11-25 Part AA, the planner's f64 study on the island's gale at `6bad9e6e8`: the reach is 87.5 m and the
  highest surface's rest place is 0.6 m from its column at the median, 16.6 m at the furthest. A coarse pass of
  2 m cells with a 0.5 m margin kept every surface over 1,600 columns, for about 73,000 nodes a tick where the
  whole cut is 24 to 47 million. Tighter ceilings were not tried, and the coarse pass does not need them.
  [Study](../../../.orchestrate/evidence/WATERS/WP19/query-reach-study.md)

- 2026-11-25 Took both of the review's suggestions. The cues are W1 to W12 again, as before the migration to
  work packages renamed them WP1 to WP12, restored token by token from `24aba8b80`. The builder handoff no
  longer gives every work package the exact-image verifier, which the ruling of 15 November moved to main.


<a id="wp19-coarse-rule-decision"></a>

- 2026-11-25 Design review, Must Fix 1: all 48 stations seven ticks late on every tick passed WP19.3, since
  each was inside the 8-tick maximum. WP19.3 now also needs nine answers in ten within two ticks of their
  post, in the breakers and in the open-water control, and a pair whose control delivers fewer does not
  pass. The 8-tick expiry, the fallback and blocking-read counts and the lateness record stay. The design's
  age contract and WP19's running run say the maximum age is for hitches and is not the delivery.
  [Review](../../../.orchestrate/evidence/WATERS/design-review.md)

- 2026-11-25 Design review, Must Fix 2: 2 m coarse cells with a fixed 0.5 m margin answered −0.806 m at the
  review's column in the island's gale, where the whole cut answers +1.103 m. A cell's landing margin is now
  0.5 m plus its corner spread, which lost no surface at 51,658 columns where the fixed margin lost the
  highest in 91. WP19.1 adds the island's gale: the review's column and 24 or more columns under folds.
  [Part AB](../../../.orchestrate/evidence/WATERS/WP19/coarse-rule-study.md)

- 2026-11-25 Design review, Must Fix 3: WP19.1 held the public answer to the highest triangle's interpolated
  height within 1 cm, and the existing cut refines that triangle on the sheet, 1.13 cm higher at one of the
  production columns. WP19.1 now compares the public answer with the whole cut after the same refinement,
  within 1 cm, and its question to the reviewer names the unrefined height as the wrong reference. The
  triangle stage is not a separate claim: the two passes change nothing after the highest triangle is found.

<a id="wp13-set-decision"></a>

- 2026-11-25 WP13.5 blocked after three Metal looks from 1 km could not rank two crests' heights at break onset.
  Chose a nearer view over refining the offshore mesh: a 1 m crest is a pixel or two from 1 km under a game's
  lens, so its height is not a cue a player reads. WP13.5 now judges from 1 km that fronts start at different
  distances and run in, in both winds, and judges the taller crest breaking farther out in the gale from a
  nearer view: the 3.0 m and 0.8 m crests with onsets near `x = −223 m` and `x = −126 m`.
  [Looks, cuts and mesh sampling](../../../.orchestrate/evidence/WATERS/WP13/break-distance-inspection.md)

- 2026-11-25 The lead asked whether the surf query's 47 million nodes and 18.1 s a tick need a plan change before
  WP19. They do not: the [WP19 decision](#wp19-query-decision) was taken on that measurement, and WP19 makes the
  cut coarse to fine and holds it to the age contract.

<a id="wp13-set-rank-retired"></a>

- 2026-11-25 WP13.5 blocked again after three nearer Metal looks at the gale's 3.0 m and 0.8 m crests, the sixth
  failed look at ranking two crests by height. The smaller crest breaks inside the surf zone, in water already
  39–43% white, so it has no first white to place against the outer crest's. Retired the cue from WP13.5 and
  from the work package's bar, on the lead's direction and the owner's park rule; moved to no later work package, since
  the same looks would be repeated there. WP12.4 still holds on the model that a set's highest crests reach
  breaking index 1 in deeper water, WP13.1 holds Metal's long waves to that model, and WP13.5 still judges
  from 1 km that fronts start at different distances. The gap is recorded as an assumption in the plan file.
  [Three nearer looks](../../../.orchestrate/evidence/WATERS/WP13/near-gale-set-inspection.md)

- 2026-11-25 Not taken: an optics change so a live breaking face reads over transported lace, and a different
  crest pair. Neither is certain. `foam_surface` takes the maximum of lace and face cover, so the first would
  retune the whitewater WP13.7 has passed, and an inner crest breaks in old foam on a real beach too. If the
  owner asks for the cue back, start a spike from the builder's patch-contrast reading.
  [Patch contrast](../../../.orchestrate/evidence/WATERS/WP13/gale-onset-brightness.log)
- 2026-11-24 Recovered the M13 surf build onto the current water test fixtures. Kept M17's
  compact fixtures, M18's trail guard and the current done-when criteria.
- 2026-11-24 Repeated the portable Metal elevated trade-wind view after the recovery and
  tried a weaker throw with a wider taper into the bore. A fresh inspection still found
  angular whitewater on the crest and a fragmented transition into swash. The f64
  projected sheet retained all 189 coarse three-surface probes at eight times the
  resolution. The two earlier inspections also found split crests, so M13 reaches the
  design's three-look stop. The wider taper is retained as an experiment.
  [Target inspection and diagnostic](../../../.orchestrate/evidence/WATERS/M13/metal-crest-comparison.md).
  Chrome WebGPU checked 14,275 transformed nodes with 0.422 mm maximum error and no faults.
  Native Metal returned all 48 gale stations at 1440p. The captured-off query took 12.39 s
  for 51.11 million cut nodes under shared load. The isolated Metal trail guard passed in
  6.022 s. [Recovered target records](../../../.orchestrate/evidence/WATERS/M13/recovered-device-checks.md).

- 2026-11-24 Kept the incoming phase through the swash and drove foam uprush from the
  transformed wave. A plane-beach test verifies that the section can fade while the
  incoming crest stays in phase. Metal verified the field, capability, float, frame
  and shore-birth checks. The sea shadow now recovers over the bed's border, with the
  interior sector rays retained. The existing round-island reference and a compact
  shadow-to-ambient transition both passed.
  [Metal guards](../../../.orchestrate/evidence/WATERS/M13/continuous-metal-guards.log) ·
  [Model checks](../../../.orchestrate/evidence/WATERS/M13/continuous-shadow-and-film-check.log).
- 2026-11-24 An asymmetric island section produced a narrower fold in the f64
  projection and in the elevated Metal view. Neither it nor the optical edge trial
  removed the disconnected strip of water early in the sequence. The f64 long-wave
  cut finds 30 alongshore rows in the sampled view where an exposed strip lies
  between two wet regions. The change in edge appearance cannot repair that
  geometry. A subsequent design decision should address the close-shore trough of a
  coherent set while retaining the regular-wave draw-back rule.
  [Long-wave cut](../../../.orchestrate/evidence/WATERS/M13/continuous-swash-cut.log) ·
  [Metal frame](../../../.orchestrate/evidence/WATERS/M13/soft-edge-metal-0015.png).
- 2026-11-24 The Chrome capture initially spent part of its interval at tick zero.
  The page's initial visibility event set a clock origin before the harness's
  synthetic timestamps. The surf preview now warms cooked content and drives
  its capture from the page's animation timestamps. The hardware preview
  submitted 117 frames over 12.0162 s with advancing observations and zero
  faults or omissions. A wider 40 m Metal framing and the sand view showed the
  approaching ridge and the pitch, but left crest spacing unclear. The
  alternative lee view still showed a raised wave. Kept these cameras as
  experiments. [Chrome timing](../../../.orchestrate/evidence/WATERS/M13/continuous-chrome-clock-receipt.json) ·
  [Metal overview stills](../../../.orchestrate/evidence/WATERS/M13/overview-fixed-receipt.json).

- 2026-11-24 Implemented the revised bounded swash with references on the prepared
  still-water contours. The CPU model and GPU share the shoreward phase, the
  wet floor under the displaced bore and the clearance taper for short geometry.
  The plane beach retains both regular-wave extrema with a resolved short wave.
  The Chrome fixed clock submits at the requested checkpoints. At 2.000 s
  hardware Chrome compared 15,983 cache nodes with 0.759 mm maximum error and
  zero faults. Native Metal recorded 48 gale stations and 51,108,744 cut nodes
  in the paired 1440p cost run, with 19.02 s in the query pass under concurrent
  load. [Target receipts and scope](../../../.orchestrate/evidence/WATERS/M13/bounded-swash-iteration.md).

- 2026-11-24 Three fresh reads of submitted 40 m Metal motion could follow
  one breaking front into the wash. None could follow two adjacent ridges
  turning and packing together across the approach. The first take still
  exposed a sand-colored patch. The second reduced the patch and left a
  local facet. An authored narrow swell made the entrance more coherent
  in the third take, where an isolated sand patch and a doubled lip returned.
  Retained the second take. Review the visible scale of the crest and the
  combined displacement at the bore before tuning again.
  [Three takes and the measured positions](../../../.orchestrate/evidence/WATERS/M13/bounded-swash-iteration.md).
