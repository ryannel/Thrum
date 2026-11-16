# WATERS log

- 2026-11-15 Repaired the second M1 review's connected-fill finding. Seeding,
  flood propagation, exterior-sea validation and continuous lookup now share
  the bilinear saddle's strict wet connectivity. Margin growth starts at the
  waterline, keeping it out of chart-owned water. All 32 water-model tests,
  strict Clippy and documentation checks pass, including mirrored/translated
  seed cells, remote diagonal necks, reversible stages, surface answers and
  stage/sea spill refusals. [M1 evidence](../../evidence/WATERS/M1.md) identifies
  CPU and Mac/Chrome proofs; GPU water quality and performance remain unclaimed.

- 2026-11-15 Fixed both verified findings from the first M1 review. Triangle
  rasterization tracks actual intersections separately from the uncovered-cell
  fallback. Continuous map lookup now checks monotone reach inside a bilinear
  cell, so a supported crest cannot lend support downhill. All 26 water-model
  tests pass, including below-fallback overlap order and subcell sea, basin and
  chart ridge regressions. The retained [M1 proof](../../evidence/WATERS/M1.md)
  identifies CPU and exact-baseline source revisions; no water GPU visuals or
  device performance are claimed.

- 2026-11-15 Resumed the surviving integration job after its shared GPU wait.
  It rebased cleanly onto main as `2c2679be` and refreshed every CPU item check,
  the sea-model tests, strict Clippy and documentation checks; all passed.
  Re-read the independent map, gains, conformance, joins and swash oracles and
  the provenance declaration. The fresh exact verifier is running on this source;
  [the retained target record](../../evidence/WATERS/M1-baselines.json) identifies
  the complete pass once promoted, separately from the earlier failed captures.

- 2026-11-15 Resolved the older CREATOR, CULLING and CLOUDS link repairs by
  retaining their upstream replacements; documentation checks passed. Consolidated
  unpublished WATERS work onto the current main base to avoid replaying superseded
  integration repairs. CPU evidence is refreshed if its revision is no longer an
  ancestor of the work being merged. Target proof and official merge remain owed.

- 2026-11-15 CPU proofs passed on `32382d0a`. The exact verifier then stopped at
  an incomplete native route capture: 434 submissions, 292 images and 142 ring
  omissions, with host load near 100. This is not a fresh baseline pass.
  [Diagnostic record](../../evidence/WATERS/M1-runtime-admission.json) retains
  the identity and observed contention; references and comparisons are unchanged.

- 2026-11-14 Resumed M1 with 91 GiB free, inspected the CPU oracle tests and
  provenance, and resolved the integration-only handoff conflict on rebase.
  The GPU-locked driver refreshes target proofs before the official merge;
  [M1 evidence](../../evidence/WATERS/M1.md) retains the source identities.

- 2026-11-14 After owned cache cleanup, retry `20261114-142437-51691` rebuilt all
  verifier artifacts and cooked content on `3ec5e9f6`, then the first native
  capture refused less than 16 GiB free. The observed remainder was 15.9546 GiB.
  [Admission record](../../evidence/WATERS/M1-disk-admission.json) and
  [handoff](../../evidence/WATERS/M1-handoff.md) preserve the source and remaining
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
  and tolerances are unchanged. [Continuation](../../evidence/WATERS/M1-handoff.md).

- 2026-11-14 Resumed integration: both native baseline matrices and all four
  Chrome sets passed on `343e6bd3` in 910.41 seconds; retained the fresh record and
  CPU test log in [M1 evidence](../../evidence/WATERS/M1.md). The first merge gate
  found two upstream APPLE-TILE handoff links to evidence not yet retained on main.
  Preserved their paths as text so repository link checks can run without claiming
  those artifacts are present. M1 criteria and builder results are unchanged.

- 2026-11-14 M1 builder implemented the CPU map, stages, joined rows, wave limits,
  surface oracle, triangle bed and conformance harness. All 24 water-model and ten
  sea-model tests passed, with strict Clippy and documentation checks; an analytic
  plane test covers the corrected chart level slope. [Evidence](../../evidence/WATERS/M1.md).
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
  with eleven milestones. The module is named `WATERS` because `docs/evidence/water` already holds the 1 November
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
- 2026-11-14 Planner's reproductions, in [the evidence folder](../../evidence/WATERS/design-proof/). Part H:
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
- 2026-11-14 [Design review](../../evidence/WATERS/design-review.md): Design Changes Requested, four Must Fix
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
- 2026-11-14 Second [design review](../../evidence/WATERS/design-review.md): Design Changes Requested, two Must
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
  N to Q in [the evidence folder](../../evidence/WATERS/design-proof/) (`surf_view_proof.py`).
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
  research and reproductions R to T in [the evidence folder](../../evidence/WATERS/design-proof/)
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
  ([part F](../../evidence/SCENE-VIEWS/design-proof/README.md)), so M14.2's panel wants a line well above it.
- 2026-11-14 `ATMOSPHERE` planned its M9, [the air at the ground](../ATMOSPHERE/DESIGN.md#the-air-at-the-ground):
  the aerial pass carries a line of sight on under the model's ground, so the sea and whatever stands just
  above its horizon take the haze of their own lines. M2 now waits for `ATMOSPHERE:M9`, because M2.2 and M2.3
  judge the horizon and the far sea's haze from 2 m and 40 m, where the old stop took up to 72% of the sea's
  in-scatter, in bands that moved with the camera. No text, item or bound here changed. The note above for
  M14's builder holds only until M9 is Done.
- 2026-11-14 [The design review of the surf, mirror and underwater revisions](../../evidence/WATERS/design-review.md)
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
  [the design proof](../../evidence/WATERS/design-proof/README.md).
- 2026-11-14 [The fifth design review](../../evidence/WATERS/design-review.md) asked for changes, one Must Fix:
  the scan of the throw inverts a straight section and not the assembled sheet, where a surface arrives from
  beside the line through the point and from an envelope larger than the point's. Re-planned; M1 is untouched.
  The highest surface is now a cut of the sheet: forward evaluation on a 0.25 m lattice of rest places fixed in
  the world, over the cells within a throw reach that needs no wave evaluated, refined on the assembled map.
  Part X of [the design proof](../../evidence/WATERS/design-proof/README.md) holds it to a cut eight times
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
  [CPU proof](../../evidence/WATERS/M12.md) records 59 passing model tests, strict Clippy and independent
  Snell, fetch, swash and fine forward-cut references, including the actual prepared surface and reach.
  [Cost](../../evidence/WATERS/M12-preparation.md) records preparation bytes/time and six column probes.
  Earlier incomplete baseline captures remain diagnostic history; the current shared main-branch
  `exact-images` check supplies M12.10's proof after integration. GPU surf and its appearance remain for M13.
- 2026-11-16 M12's post-merge gate exposed a watcher publication timeout. Added bounded stat-cache
  reconciliation to recover delayed or absent notifications, retaining the original debounce and test deadline.
  [Recovery proof](../../evidence/WATERS/M12-watch-recovery.md) records the native trace, no-notification
  edit/add/remove/idle regression, cooker regressions and strict Clippy. The water model is unchanged.
- 2026-11-16 Reconciled M12's retained watcher regression with creator's landed shared fixture and adaptive
  sweep API after the merge gate found duplicate helper types. The original ten-second watchdog remains.
  [Integration proof](../../evidence/WATERS/M12-watch-recovery.md#integration-with-the-creators-sweep)
  records eight watcher tests, four server/watch tests and strict cooker Clippy passing on the Mac.

- 2026-11-16 M2 stopped on M2.2: from 40 m up the crests were round and a smooth belt lay under the horizon, after
  eight tuning attempts ([handoff](../../evidence/WATERS/M2-handoff.md)). The lead's task was to settle the approach
  at the same bar, preferring what current engines ship. The planner read the branch's shaders and found two causes
  in the representation, not the tuning. The long-wave cache stores the map inverted onto a fixed world grid, and
  vertices move only vertically, so a crest turns over a texel. Every layer is read from storage buffers at mips
  0, 4 and 8, chosen by the footprint's longest side, which collapses the sea to one variance about 600 m out.
- 2026-11-16 Parts Y and Z of [the design proof](../../evidence/WATERS/design-proof/README.md) measure both. Part Y
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
- 2026-11-16 Built M2's open-sea contribution, ticking water example, depth-writing late pass and capture/cost tools.
  Mac Metal probes verify submitted depth, CPU-model motion on tick/repeat frames, history resets and live stage changes.
  Corrected coherent Fourier phases, shared Fresnel/exposure composition and world-space cascade normals.
  Three fresh builder opinions still find rounded crests; the design's three-look stop rule was reached.
  The profile diagnostic identifies an untried larger permitted choppiness, not a proof that the model cannot work.
  [Evidence](../../evidence/WATERS/M2.md) separates that visual finding from functional and device proof.
- 2026-11-16 Resumed M2 at `69a7ba617`; its scene, frame-contract and stage checks pass again on Metal.
  Preserved the selected rounded-crest failure and verified recovery from the local evidence store.
  The first paired cost run failed with missing GPU durations and timing faults; its cause is unconfirmed.
  No fourth visual attempt was made under the design's three-look stop rule.
  [Handoff](../../evidence/WATERS/M2-handoff.md) asks the lead/planner for direction and records both issues.
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
