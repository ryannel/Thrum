# WATERS log

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
