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
- 2026-11-14 Design review asked for changes: [three Must Fix findings](../../evidence/RIVERS/design-review.md).
  All three are resolved in the design, the items and [rulings 2 and 5](../ARCHITECTURE.md#seam-rulings-for-rivers).
- 2026-11-14 Dry support. A dry cell's level always lies under its bed, and between two samples each cell
  has a prepared wetting gauge: the least gauge at which wet cells join it to the lower sample's water. Below
  it the cell has no water, wave support or margin, so a hollow behind a ridge is dry until the ridge wets.
  A bank cell's answer is the linear mix it was. [Part 8](../../evidence/RIVERS/design-proof/README.md)
  reproduces the review's 0.80 m of invented water and its removal. M1.6, M3.2 and M4.3 judge the wet
  cells as well as depth and current; a river file gains a seventh value.
- 2026-11-14 Rings on running water. The weighted mean is replaced by a current ladder in the interaction
  step: a few carriage currents for each region, each cell blending the two that bracket its own, as the
  depth ladder does. Part 9: two ring sets at once on a pool and a riffle stay within 0.10 m of their own
  water, where any one current leaves each 3.25 m off. New item M6.10 judges it with a second source in
  the same region; M6.3 keeps the plunge. Declared: 0.2% to 1.3% of a midway wave's energy lost a second.
- 2026-11-14 M1.6 no longer asks for the solver's row inside a join. The stored chart equals the solve; the
  map equals it where the join's weight is nothing, and `WATERS`' joined row, backwater included, elsewhere.
- 2026-11-14 Suggestion taken in part: M6.4 looks at the fall from 2 m and 200 m as well, and the design and
  M7.11's ledger say that only the island's fall of about 4 m is judged by eye. Short and tall falls for
  the looks alone were not added to the island: they are scenery no cue of the island needs.
- 2026-11-14 Not run by the planner: the ladder in plan or on the GPU, and wetting gauges on a solved valley.
