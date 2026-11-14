# PUDDLES log

- 2026-11-14 The owner, about the `SEA` module: "Please make sure the sea work is actually thinking about water
  as a whole. So puddles, rivers, water falls, streams, lakes, and more. We want something that's built
  cohesively." The lead's task for this plan: "puddles and wet ground as part of the one water system, at the
  quality of shipped high-end games".
- 2026-11-14 Planned from [the module list's brief](../ARCHITECTURE.md#rivers-and-puddles) and
  [WATERS' design](../WATERS/DESIGN.md#what-the-later-modules-build-on). Seven milestones: two on the CPU after
  `WATERS:M1`, three on the island after `WATERS:M8`, the V1 valley after `SHOWCASE:M6`, and a close after
  `WATERS:M10`, `M14` and `M16`.
- 2026-11-14 The seam question the brief carried, how puddles that overflow merge and part, decided: all the
  hollows of a stretch of ground are one body in the map, a hollows body, whose level at a point is the level
  of that point's hollow. A tree of hollows is prepared from the bed (Barnes, Callaghan and Wickert's
  depression hierarchy), water volumes are stepped on the CPU by fill, spill and merge, and the map reads a
  label and a level. Nothing is rebuilt at a merge, so the body's identity, its query and its rings hold. Not
  chosen: a basin for each hollow, one flood level over a mask, and a live shallow-water solver.
  [Reproductions 1 to 6](../../evidence/PUDDLES/design-proof/README.md) were run for it.
- 2026-11-14 Replaced, from `WATERS`' design and the interface table: "a puddle is a basin whose stage rain
  raises" and "`PUDDLES`' drops are the same record". A bed 3.2 m square holds 20 hollows whose fills overlap
  in 10 pairs, and a square metre takes thousands of drops a second. Rain rings are two wave layers that are a
  function of time, summing each scheduled drop's own wave train; a footfall is the game's impulse record.
- 2026-11-14 Rings from a footfall, decided: `WATERS`' interaction operator on a third, fine level of 2 cm
  cells with a ladder from 5 mm and surface tension. Its near level's cell is a quarter of a puddle. The fine
  level steps one tick at a time: steps of eight ticks lose up to 42% of a wave's height a second (part 6).
- 2026-11-14 Wet ground, decided: a function of prepared fields and stepped stores, not a field of state. Two
  stores for each ground class, surface water and pores, at three exposures; darkening from the light the film
  returns to the ground; and water standing in the ground's relief drawn as `WATERS`' film, so wet paving
  mirrors scenery. Not chosen: one global wet level, and a wetness texture stepped on the GPU.
- 2026-11-14 Shelter, decided: prepared content, a canopy grid from the triangles a game says shelter, because
  it sets how fast a puddle fills and that may not depend on a camera. Rain falls straight.
- 2026-11-14 [Seven rulings](../ARCHITECTURE.md#seam-rulings-for-puddles) settle what is built in `WATERS`'
  crates, the forward shader and the two games. No item of another module changes.
- 2026-11-14 Read as summarised by fetch: Lagarde's Water drop 2b, 3a and 3b, and the method of Fill–Spill–Merge.
  Abstract only: Le Méhauté 1988; the Far Cry 6 weather session's description. Not found: any talk on the rain
  or wet ground of The Last of Us Part II, Red Dead Redemption 2, Forza Horizon, Spider-Man, Alan Wake 2 or
  Cyberpunk 2077. From memory: Marshall and Palmer's drop sizes, Atlas's fall speed and the form of Lekner and
  Dorf's model, which part 4 derives again and which gives 0.55 where Lagarde quotes about 0.68 for asphalt.
- 2026-11-14 Design review asked for changes: [five Must Fix findings](../../evidence/PUDDLES/design-review.md).
  All five are resolved in the design, the items and [the rulings](../ARCHITECTURE.md#seam-rulings-for-puddles),
  with [reproductions 7 to 10](../../evidence/PUDDLES/design-proof/README.md).
- 2026-11-14 The mirror (finding 1). A hollows body is no longer one plane at the mean of its ground, which
  cut the foot off everything standing in a low puddle. New ruling 8: it gives the mirror its levels in
  bands no taller than 5 cm, each cut at its lowest water, ranked with every other level; a lookup refuses
  a sample below its own surface. Part 7: one plane fails whichever height it takes. Declared: a strip
  behind a thing standing in water above its band's plane, and two views for however many bands. M7.2 is
  restated and M7.12 added. Not chosen: a cut for each thing at its own water, which ties a view to scenery.
- 2026-11-14 Rain that stops (finding 2). The rain state keeps the rate of each tick of the last second and
  a drop is born by the rate at its own tick, so rings run out their second after a stop and none is born
  in a dry spell (part 9). The bands' passes stop when the history is empty. M2.6, M4.1, M4.2 and M7.1 are
  restated to it, and M4.12 looks at a stop and a restart.
- 2026-11-14 First-hit receivers (finding 3). The canopy grid holds two shelters in a column and exposure is
  the product of those above a point; a third is refused (part 10). Rain on a puddle is the puddle's; the
  ground's stores describe uncovered ground only, with entries in the account when water covers ground and
  when it leaves. M1.5 has the crown under the roof; M1.13 and M2.11 judge each receiver alone.
- 2026-11-14 Saturation (finding 4). A ground class has a soak rate into its pores and a percolation rate out
  of them. Earth takes a sheen once its pores are full, 1,080 s into the review's shower, and only ground
  that passes water down faster than the rain never does (part 8). M1.4, M2.1 and M5.5 and cue P6 say so.
- 2026-11-14 The fine interaction level (finding 5) moved out of M2 and M4 into a new milestone, M8, built
  after M4 and waiting for `WATERS:M6`, whose reference it extends. M2 and M4 no longer need it.
- 2026-11-14 Suggestions taken, both. The damp band has a state rule: a fall adds, a rise takes away, and it
  decays at the pores' rate, so it is judgeable when a level stops, a puddle empties or drying is off
  (M2.3). A GPU bound measured first is accepted only at a tenth of what a named wrong evaluator makes on
  the same case (M4.1, M5.2, M8.3).
- 2026-11-14 Not run by the planner: any GPU check, the bands' ranking as a camera moves, and the stores with
  the covering transfers in a full fill step. Part 1's account was run without stores.
- 2026-11-14 `SCENE-VIEWS`' third design review: a view's air may start only at a level plane. A band's
  plane is its lowest water, which is level, so ruling 8 and M7 stand. No text, item or bound here changed.
