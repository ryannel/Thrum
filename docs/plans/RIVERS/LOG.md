# RIVERS log

- 2026-11-14 The owner, about the `SEA` module: "Please make sure the sea work is actually thinking about water
  as a whole. So puddles, rivers, water falls, streams, lakes, and more. We want something that's built
  cohesively." The lead's task for this plan: "streams, rivers and waterfalls as part of the one water system,
  at the quality of shipped high-end games".
- 2026-11-14 Planned from [the module list's brief](../ARCHITECTURE.md#rivers-and-puddles) and
  [WATERS' design](../WATERS/DESIGN.md#what-the-later-modules-build-on). Seven milestones: three on the CPU
  after `WATERS:M1`, three on the island after `WATERS:M9`, and a close after `WATERS:M10` and `M16`.
- 2026-11-14 The solve, decided: the steady state of the 2D shallow-water equations with Manning friction over
  the map's bed, marched in pseudo-time on wet cells from filled pools, stored as a river file with a key. Not
  chosen: a spline, routing by steepest slope (no level pool, no jump), an offline particle simulation, and a
  live solver. [Reproductions 1 to 7](../../evidence/RIVERS/design-proof/README.md) were run for it.
- 2026-11-14 Found in the reproduction and written into the design as properties of the scheme: Audusse's
  reconstruction stands a thin sheet on steep ground 23% too deep, where Chen and Noelle's is within 5%; and
  friction taken at the new discharge inside a pseudo-time step shifts the steady state with the step; and a
  lip modelled as a drop to a dry cell draws the water above it down to 0.22 of its critical depth, which
  would throw a sheet 3.2 times too far, where a face that passes the flow at its critical depth does not.
- 2026-11-14 Replaced, from `WATERS`' design: "a reach keeps its prepared flow whatever its source's stage".
  Over 0.6 m of lake level an outlet's discharge changes 80 times (part 3), so a river is solved at source
  levels 0.3 m apart and the map reads between them by a gauge.
- 2026-11-14 Replaced, from `WATERS`' design: "roughness from the flow is a gain on the shortest cascades". A
  gain on the wind's waves gives a stream nothing in a calm. The flow has two wave layers of its own, at a gain
  from the bed's friction: the chop's rms height is twice the depth times the friction slope, capped at the
  depth limit.
- 2026-11-14 The chart's loss, decided: the power that breaks the surface, head lost beyond bed friction plus
  friction's power beyond the chop's cap. Foam births read it by the shallows' own constant.
- 2026-11-14 A fall, decided: a brink is a property of the bed alone; the sheet follows the water's free path
  or the rock's face; its pattern is read by the time each part left the lip, so streaks fall and stretch with
  the water; its plunge gives births from the fall's power and impulses from the unsteady share of its
  momentum. Not chosen: a card with a scrolling texture, and particles. Mist is not planned.
- 2026-11-14 One mean current for an interaction window, judged: it slides rings in a pool 4.7 m in 5 s beside
  a riffle (part 7). Each region's mean is weighted by its waves.
- 2026-11-14 [Seven rulings](../ARCHITECTURE.md#seam-rulings-for-rivers) settle what is built in `WATERS`' crates.
  No item of `WATERS` or `SEA` changes.
- 2026-11-14 Read whole: the Uncharted 4 rapids slides and notes, and pages 26 to 36 of the Horizon Forbidden
  West water notes. Not read: the Far Cry 5 water talk, any account of Red Dead Redemption 2's rivers or
  falls, and measurements of a falling sheet's breakup. Horeni's length, the weir relation and the surface's
  turbulent energy are from memory.
