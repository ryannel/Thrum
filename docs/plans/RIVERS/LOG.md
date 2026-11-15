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
- 2026-11-14 Second design review asked for changes: [one Must Fix finding](../../evidence/RIVERS/design-review.md).
  A reach whose lower sample is wholly dry had no wet cell to flood from, so none of its cells took a
  wetting gauge and it never wetted.
- 2026-11-14 Wetting restated: water spreads from where it is. The flood starts from the lower sample's
  wet cells and from each inflow place the lower sample leaves dry, a spring's cell or where water arrives
  from the reach above, keyed at the gauge that inflow starts; reaches are flooded from the sources down.
  A wet cell passes water to a neighbour when its own level stands over that neighbour's bed. A part of a
  fall that only the upper sample runs starts at its lip cell's wetting gauge.
  [Part 10](../../evidence/RIVERS/design-proof/README.md): the review's 32 cells all take the lip's gauge,
  and part 8's answers are unchanged. Not chosen: an extra flow sample at the gauge a lip starts, which
  costs solves and still needs the seed.
- 2026-11-14 Replaced in that rule: a cell's own crossing, taken from its two stored levels. Above a wet
  channel it held a newly fed branch dry until 0.98 of the way, and in a dry reach it depended on the
  margin a dry level is stored at (part 10). The solve now writes the gauges; the map still only reads them.
- 2026-11-14 Items. M3.2 keeps the mix and says its depth and speed bound is for cells both samples wet
  (suggestion 1, taken: the design already said so). New M3.8 holds the wetting: the hollow, and a reach
  that goes from dry to running. M3.3 and M4.3 gain the lip that starts to run and the dry reach on the GPU.
- 2026-11-14 Suggestion 2 taken, in the design only: carriage currents are vectors chosen so their span
  holds a region's currents, not picked from its cells. Part 11: round a 90° bend at 1.5 m/s, three of the
  cells' own leave cells 0.11 m/s outside the span, 0.57 m after 5 s; rest and three on the bend's tangents
  hold them all. Ruling 5 says so. M6.10 is unchanged and still measures a turning window.
- 2026-11-14 Not run by the planner: the solver's own samples with a lip that starts to run, wetting gauges
  on the GPU, and waves blended between three carriage currents.
- 2026-11-15 M1 built ([evidence](../../evidence/RIVERS/M1.md)). Verified: the valley holds discharge to 0.139%,
  pool level to 5.3 mm at 1.73 critical depths over the sill; closed forms within the items' bounds; WATERS
  takes the chart; one and eight threads give the same bytes; 1.31 s on one thread, 9,798 updates a wet cell.
  By construction, not measured: the energy account closes because the face-based head loss telescopes;
  friction power is computed apart in the test. M1 limits: first order in space; a mouth must be one straight
  row or column. Suspected, not tested: the active-span cost stays flat for rectangles much larger than 2×.
