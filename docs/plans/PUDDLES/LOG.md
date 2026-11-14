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
