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
