# Rivers: streams, rivers and waterfalls in the one water

14 November 2026.
[Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) · [module list](../ARCHITECTURE.md) ·
[the water system, `WATERS`](../WATERS/DESIGN.md) · [the wave field, `SEA`](../SEA/DESIGN.md). C02, C04, C07, C08,
C09 and C12 in [CONTRACTS](../../design/CONTRACTS.md) govern, as they do for all water.
[The module list's brief](../ARCHITECTURE.md#rivers-and-puddles) and
[what `WATERS` leaves to this module](../WATERS/DESIGN.md#what-the-later-modules-build-on) are where this design
started. Where it differs from either, [the rulings](../ARCHITECTURE.md#seam-rulings-for-rivers) hold.

## The request

The owner, 14 November 2026:

> Please make sure the sea work is actually thinking about water as a whole. So puddles, rivers, water falls,
> streams, lakes, and more. We want something that's built cohesively.

The lead's task for this plan: "streams, rivers and waterfalls as part of the one water system, at the quality of
shipped high-end games". The module list's brief says what finished means: a stream that runs from a spring down
a valley the game's ground gives it, pools behind a sill, quickens in a narrows and reaches the lake or the sea;
water that reads as running at the bar of Red Dead Redemption 2's rivers; floats carried and turned by it; foam
where it loses height and at rocks; and a waterfall, with its sheet, its plunge, its foam and its rings in the
pool below.

## Purpose and scope

| User | Needs | What that requires of the design |
|---|---|---|
| A teaching demo, often in Chrome: a stream into a lake, a fall into a pool, a river that swells when the lake is raised | Running water that is right as well as good-looking, with nothing solved in the page | Flow from the ground itself, prepared ahead and stored; every runtime part optional |
| A game | Floats and craft carried by a current that is the same on every machine and whatever the camera does | Flow as content in the water map, read through the one query |
| `WATERS` | Running water that needs no second surface, optics, foam or query | A river is charts in the map, wave layers at gains, births of foam and impulse records |
| The planner of `PUDDLES`, later | A place for water that runs off | The solve takes any declared source; rain is not one here |

**Commissioned:** the flow solver and its stored charts; a river that answers its source's level; the flow's own
roughness; foam from the flow; waterfalls. Cues R1 to R8 below, on `mac-metal` and `chrome-webgpu`, on the water
example's island behind a selection.

**Not built**, with the reason:

- **Mist and spray at a fall, and spray from rapids.** Water that leaves the surface as drops is a medium in the
  air, and the engine's one owner of that is the weather (`CLOUDS`) through `ATMOSPHERE`'s view volume, which
  comes late in the build order. A fall record publishes its power so a mist can be added without a change here.
  It is the largest thing a shipped waterfall has that this plan does not ([plan](PLAN.md#open-for-the-human)).
- **Bulk water that moves while the game runs:** a flood wave, a dam break, a rise that travels downstream. A
  river answers its source at once along its whole length ([below](#a-river-answers-its-source)).
- **Rain as a source, and runoff.** Sources are declared springs and bodies.
- **Standing waves that overturn, wave trains that stand below a drop, eddies that shed and wander.** The flow is
  steady. What stands still on a river here is what the solve resolves in its level.
- **A float that grounds on the bed, and the sound of water.** The engine has no physics owner and no audio.
- **Water above water in one column**, as in `WATERS`: a fall's sheet is not a body and answers no query.

**What exists.** `WATERS` WP1 builds the map and its chart; its WP9 draws a hand-made chart, carries detail and foam
along it and joins it to a lake and the sea. Thrum has no solver. Broadsides' inland code was never accepted and
is not used.

## Cues

The bar for streams and rivers is Red Dead Redemption 2's; for falls it is Horizon Forbidden West's. All in
words, at the display's full refresh.

| Cue | Range it must hold over | Plausible wrong substitute, and what rejects it |
|---|---|---|
| **R1** Water finds its own way: from a spring it runs down the valley the ground gives it, fills a hollow to its sill and spills, narrows and quickens between rocks, and reaches a lake or the sea | Discharges of 0.01 to 20 m³/s; channels 1 m to 40 m wide and at least four cells; beds from level pools to slopes of 1.5; cells of 0.25 m to 1 m | A level and a current drawn along a spline. Rejected by a pool that must stand level behind its sill, by discharge that must be the same through every section, and by a hollow beside the channel that must stay dry |
| **R2** It reads as running: the surface is rough and glittering where it runs fast and shallow, glassy where it is slow and deep, and its detail travels at the water's own speed | Currents of 0.05 to 4 m/s; a dead calm to a gale; from the bank at 2 m, along and across, and from 40 m up; noon and low Sun | Ripples that need wind, or one texture sliding at one speed. Rejected by a calm, where a riffle must still be rough and a pool still, and by tracked detail against the current at two speeds |
| **R3** It carries what floats: a float goes down with the current, slows in the pool, quickens in the narrows and turns in the shear | A float from the spring to the sea across every join; rest to 4 m/s | A float moved along a path. Rejected by the query's own current at the float, and by a float released off the centre line |
| **R4** Foam where the flow breaks: white below a ledge, behind a rock and down a steep chute, trailing downstream in streaks that thin and vanish; none on a glide or a pool but what drifts in | Head lost from none to a metre in a step; the same water type as the lake; calm to gale | Foam painted where the artist saw rapids, or foam from the surface's present shape. Rejected by a trail that outlives a source turned off, and by a rock moved in the ground, whose foam must move with it |
| **R5** A waterfall: a smooth tongue at the lip, a sheet that thins and breaks into white strands as it gathers speed, and a plunge that boils, foams and sends rings across the pool | Falls of 0.75 m to 12 m; 0.005 to 1 m² of water a second for each metre of lip; the pool's level moved through its range; from 2 m to 200 m, in front, beside and behind | A scrolling texture on a card. Rejected by strands that must stretch as they fall, by a trickle that must be strands where a flood is a sheet, and by rings and foam that must stop when the fall is dry |
| **R6** A river answers its source: raise the lake and its outlet runs deeper, wider and faster, the fall thickens and the foam grows; lower it and the river thins | The source's level through its declared range; a spring's flow through the range the game declares | One prepared flow with the level added. Rejected by discharge at two lake levels against the outlet's own sill, and by a waterline that must move up the bank |
| **R7** One water: the river has the lake's light, foam, query, mirror and view from beneath, and no seam where it leaves the lake, lands below a fall or meets the sea | The joins of [WP6](../WATERS/DESIGN.md#cues), with a solved chart; a swell at the mouth; dawn to night; a gorge in shadow | A river drawn by its own shader. Rejected by the surface either side of each join, and by a fall in a gorge's shadow, which the Sun must not light |
| **R8** Rings on running water: a stone dropped in a pool sends rings that stay with the pool's water, beside a riffle that runs | A pool at 0.2 m/s beside a riffle at 1.5 m/s in one window, with rings on both at once | Rings carried at one speed for the whole window. Rejected by the centre of each set of rings after five seconds |

## Research and selection

What each source was read for, and how far.

- **Uncharted 4** ([Gonzalez-Ochoa, SIGGRAPH 2016 course](https://advances.realtimerendering.com/s2016/index.html),
  all 86 slides and their notes read as extracted text). A river's motion is ocean wave grids carried along a
  flow map by two samples half a period apart, cross-faded (slide 37). The low-frequency grids are slowed "to
  simulate the standing waves" and the high-frequency ones sped up (slide 30), with a painted amplitude map for
  each grid (slides 39 and 81). The flow map and the height map come from an offline FLIP simulation over the
  level's ground, corrected with curves artists draw (slides 74 to 80). Taken: prepared flow from a simulation
  of the ground, wave layers carried by it, amplitude that varies along the river. Not taken: amplitude that is
  painted, and a flow that artists correct by hand.
- **Horizon Forbidden West** ([Malan, SIGGRAPH 2022 course notes](https://advances.realtimerendering.com/s2022/SIGGRAPH2022-Advances-Water-Malan.pdf),
  pages 26 to 36 read as extracted text). Standing waves round rocks, eddies and waterfall impacts are baked
  Houdini simulations, stored as a deformation volume of 128 frames and placed by hand and by rule: "the core of
  the water system is baking offline sims for runtime use". Its author's own limits: the deformations on rivers
  are "so flat" for want of triangles, and foam in vertex colour pops. Taken: what stands on a river is prepared.
  Not taken: a library of baked effects placed apart from the flow, which would not know the discharge.
- **Fluid Flux** ([author's page](https://imaginaryblend.com/2021/09/26/fluid-flux/), as summarised by fetch). A
  live shallow-water solver on a height field, after Müller's, whose frames can be baked "and rendered without
  the cost of running the simulation", which its author calls especially useful for rivers. Domains are
  axis-aligned and at most a kilometre square; results differ between GPUs. Evidence for a shallow-water
  solution stored as the river.
- **Crimson Desert** (a Digital Foundry interview, as reported and summarised by search). A live particle solver
  of the shallow-water equations near the camera, the same for shores, brooks and waterfalls. It is the newest
  shipped approach and a live one; its cost and its reach beyond the camera are not published.
- **Red Dead Redemption 2.** No talk or frame study of its rivers was found. Its bar is taken from the game: flow
  that is faster where a river narrows and shallows, foam at rocks, and beds seen through clear water.
- **Shallow-water schemes**, from the abstracts of Audusse et al. 2004 (hydrostatic reconstruction), Bollermann,
  Chen, Kurganov and Noelle 2014, and Michel-Dansac et al. on Manning friction, and from memory of Chen and
  Noelle 2017 and of Delestre et al. 2012, who show Audusse's reconstruction losing the slope's pull where the
  bed falls more across a cell than the water is deep. Both reconstructions are implemented and compared in
  [part 1](../../../.orchestrate/evidence/RIVERS/design-proof/README.md). SWASHES' transcritical case is from memory of its
  parameters and is checked against its own closed form there.
- **Hydraulics, from memory and not re-read:** Manning's relation; the weir relation, a head of 1.5 critical
  depths over a broad sill; the head a hydraulic jump loses; the depth at a free overfall's lip, about 0.71 of
  the critical depth (Chanson, confirmed by search); Horeni's breakup length for a falling sheet, about
  `6 q^0.32` metres for `q` in m²/s (its form confirmed by search, its numbers not); Nezu and Nakagawa's
  turbulent energy near a channel's surface, about 0.65 of the friction velocity squared.
- **The older Thrum designs.** [The dynamics design](../../design/modules/WATER-DYNAMICS.md#finite-fall-moving-receiver-and-continuous-arrival)
  holds two results kept here: a sheet's thickness is its discharge over its speed, and a steady fall on a
  steady pool presses steadily and raises no ring each tick. Its cohorts, ancestry and work ledger are not
  selected, for [the reasons `WATERS` gives](../WATERS/DESIGN.md#judged-from-first-principles).
- **Not read:** the Far Cry 5 water talk (no slides found), any account of Red Dead Redemption 2's or
  Ghost of Tsushima's falls, Peytavie et al. 2019 beyond its abstract, and measurements of a fall's breakup.
  A search for river or waterfall talks at GDC and SIGGRAPH in 2023 to 2026 found none beyond those above.

### Judged from first principles

| Inherited choice | Verdict | Why |
|---|---|---|
| A river is a chart: a prepared level and current in the map (`WATERS`) | **Keep** | It is what Uncharted 4, Horizon and Fluid Flux's baked rivers ship, it is the same on every machine, and it exists far from the camera |
| The flow comes from a steady shallow-water solve of the game's ground (the brief) | **Keep** | A pool level behind its sill, a jump below a ledge and water that splits round a rock all follow from the ground. A painted flow knows none of them |
| A reach keeps its prepared flow whatever its source's level (`WATERS`) | **Change** | Over a lake's range of 0.6 m an outlet's discharge changes 80 times, and a flow prepared at the middle is 63% too little at one end and 30 times too much at the other (part 3). The river is solved at several levels and the map reads between them |
| The flow's roughness is a gain on the wind's shortest cascades (`WATERS`) | **Change** | A gain of at most 1 on waves the wind makes gives a stream no ripples in a calm or under shelter, which is where streams are. The flow makes its own waves: two wave layers the wind does not fill, at a gain the flow sets |
| Foam is born from the height a chart's flow loses (`WATERS`) | **Keep**, made exact | Bed friction roughens the water and whitens it only on a steep chute. What whitens it is head lost in jumps, ledges and the wakes of rocks. The chart's loss is that power |
| A fall is a sheet with the water's optics and pattern, and a plunge of impulses and births (`WATERS`) | **Keep**, with the pattern carried by the falling water itself | A fall's motion is known in closed form, so its pattern needs no cross-fade |
| One mean current for an interaction window (`WATERS`, left to this module to judge) | **Change** | Rings in a pool beside a riffle slide 4.7 m in 5 s (part 7). No one current serves a pool and a riffle that both hold waves, however it is weighted: each set is 3.25 m off after 5 s (part 9). The step carries at a few currents and each cell blends the two that bracket its own |
| Live shallow water near the camera (Crimson Desert, Fluid Flux, the water decision) | **Not selected** | It pays every tick for a flow that does not change, differs between devices, ends at its window and needs a GPU to answer a float. A live solver would write the same map, so it can come later |

### The candidates, decided against the cues

| Cue | Candidate | Verdict |
|---|---|---|
| R1 | A spline with a width, a level and a speed | Fails: no pool, no narrows that follow the ground, no dry hollow |
| R1 | Routing by steepest descent with a depth from Manning at each cell | Fails a pool, which is level whatever the bed under it does, and a jump |
| R1 | An offline particle simulation, as Uncharted 4 and Horizon use | Right, and needs a fluid pipeline and artists to clean its output. Not selected |
| R1 | **The steady state of the 2D shallow-water equations with Manning friction over the map's bed** | **Selected** |
| R2 | The wind's cascades at a gain | Fails a calm |
| R2 | Baked standing-wave volumes placed at rocks (Horizon) | Not selected: placed apart from the flow, at one discharge |
| R2 | **Relief the solve resolves, and two flow bands carried by the current at a gain from the bed's friction** | **Selected** |
| R5 | A card with a scrolling texture | Fails: one speed from lip to foot |
| R5 | Particles | The engine has no particle owner; a sheet of a thousand strands is a surface |
| R5 | **A sheet along the water's own path, patterned by the time each part of it left the lip** | **Selected** |
| R6 | One flow, its level raised by the chart's stage | Fails part 3 |
| R6 | A live solve of the reach | Not selected, as above |
| R6 | **Flow samples a gauge reads between** | **Selected** |

## Mechanisms

### What a river is in the one water

| Part of [the system](../WATERS/DESIGN.md#one-water-in-four-parts) | What a river adds | Its state |
|---|---|---|
| The map | Charts a solve writes, each with one or more flow samples; brink ends; fall records | Content, with one number a game moves: the gauge |
| Waves | Two flow bands, at gains the charts hold | A function of time, as the cascades are |
| Material | Births from the chart's loss and at a fall's plunge | `WATERS`' foam stocks |
| Optics | The fall's sheet, which uses the water's light and pattern | None |

A **river** is everything one solve writes: its charts, its falls and its gauge. A **reach** is one of its
charts. Nothing here draws a river's surface, answers a query on it or moves its foam: `WATERS` does, as for any
chart.

### The solve

**What is solved.** The steady state of the depth-averaged shallow-water equations, with Manning friction, over
the map's bed inside a rectangle the game declares, at the bed's cell size. The unknowns in each cell are depth
and discharge in two directions. The inputs are the bed, the sources, the bodies the water may reach, Manning's
`n` (one value, or a grid) and the river's flow samples.

- **Sources.** A **spring** is a place and a discharge. A **source body** is a basin or another river's chart: the
  solve holds that body's level along the source end and the flow that leaves is the solve's own, so an outlet's
  discharge follows from its sill.
- **Mouths.** Water that reaches a declared sea, basin or chart leaves the solve there, against that body's level
  at the low end of its stage's range, so a chart reaches down to its mouth's lowest level as
  [the map requires](../WATERS/DESIGN.md#the-water-map). A higher stage drowns the lower reach through the map's
  backwater, which is `WATERS`'.

**The scheme's properties.** These are what the items hold the solver to; how it meets them is the builder's.

1. *Finite volumes with a Riemann flux*, so discharge is conserved to rounding and a jump is captured where
   momentum puts it.
2. *Still water stays still*, over any bed and round dry ground, to rounding. A pool with no inflow is level.
3. *Depth is never negative*, and a cell may wet and dry.
4. *A thin sheet on steep ground runs at its normal depth.* Audusse's reconstruction loses half the slope's pull
   where the bed falls more across a cell than the water is deep, and stands 23% too deep; Chen and Noelle's
   stands 1% to 5% too deep (part 1d). A reconstruction with that property is required.
5. *Friction is taken at the state it balances.* Taken at the new discharge inside a pseudo-time step it shifts
   the steady state with the step; linearised about the old one it does not (part 1).
6. *Deterministic:* f64, a fixed order of sums, the same bytes for any thread count.

The order in space is the builder's. The reproduction is first order, and stands a fast reach on a slope 6% too
deep at 0.25 m cells and 3% at 0.125 m (part 1c); WP1.2 holds the solver to no worse. A second-order
reconstruction is the designed remedy if a steep reach on the island shows it.

**Reaching the steady state.** The solve marches in pseudo-time, each cell at its own step, from pools filled to
their sills, until the level's rate and the imbalance of discharge between sections fall under the profile's
tolerances. It works only on wet cells and their neighbours, so its cost follows the water and not the
rectangle. It reports its steps, cell updates, time and residual. A solve that does not converge inside its step
budget writes nothing and says so by name.

**What a solve refuses**, by name and place: water that reaches the rectangle's edge; a source on ground with no
way down to a declared body; a source body whose level, anywhere in its range, lies below the outlet's sill. It
reports each stretch narrower than four cells, where a channel is not resolved.

### Falls inside the solve

A **brink** is a run of cell faces where the bed, followed downhill, drops at least the profile's least fall
(0.75 m) at a slope steeper than the profile's steepest chart slope (1.5). It is a property of the bed alone, so
a river has the same brinks at every flow sample. A lower or gentler drop stays in the chart, as a chute with a
jump below it.

At a brink the water above leaves freely, as at a free overfall: the brink's faces pass the flow at its critical
depth where the flow above is slow, and as it arrives where it is fast, and the solve sees no water beyond them.
The last cell above then stands near the critical depth (1.06 to 1.15 of it in part 4). A lower dry cell in
place of that face draws it down to 0.22 of the critical depth, and a sheet launched from it would be thrown
3.2 times too far. The sheet's **launch** is the lip's: 0.715 of the critical depth where the flow above is
slow, the flow's own depth where it is fast, at the speed that carries the face's discharge. Each face's
discharge is carried down the **path** from its lip and enters the solve again where the path lands, with its
horizontal momentum.

The path is the free path of the water from the lip, launched at the lip's speed along the chart's current
there, wherever that clears the rock; where the rock stands in its way the path follows the rock's face. It lands on a declared body,
or on ground gentler than the steepest chart slope, where a new reach begins. So discharge is the same above and
below a fall, a river is cut into reaches at its falls, and the face of the cliff is in no body.

### What a solve writes

For each reach, a chart as [the map defines one](../WATERS/DESIGN.md#the-water-map), holding for each flow sample:

- **Level**, over the whole footprint. In a cell the sample leaves dry it lies under the bed: it is the level of
  the nearest wet cell where that is under the cell's bed, as on a bank, so the waterline is where the surface
  meets the ground; and where that level is over the bed, as in a hollow the water has not reached, or the
  sample wets no cell of the reach, it is held a margin under the bed. A dry cell is never water, whatever
  stands beside it.
- **Current**, the discharge over the depth; none in a dry cell.
- **Wetting gauge**, where the river has a higher sample: the gauge at which the cell first holds water on
  the way to that sample ([below](#a-river-answers-its-source)). The solve finds it, reach by reach from the
  river's sources down, because it knows where each reach's water enters.
- **Loss**: the whitewater power, in watts for each square metre of surface ([below](#whitewater-from-the-flow)).
- **Flow gains**: one for each flow band ([below](#the-flows-own-waves)).

And once for the reach: its **footprint**, the cells any sample wets, grown by a margin of cells whose bed
stands above the level beside them, so a dry cell whose bed is below that level is in the footprint only if
some sample wets it; its **ends**, each a source end, a mouth or a **brink end**; its join lengths; and
the **gauge level** of each sample.

For each fall, a **fall record**: along its lip, for each sample, the path as a line of points with the time the
water takes to reach each, the discharge for each metre of lip, and the launch depth and speed; the reach above and the
body below; and stable IDs for its segments.

The river file holds the loss in watts a square metre. It is handed to the map's chart in the units that chart
documents ([`WATERS` WP9.6](../WATERS/DONE.json)); its meaning is fixed here.

### A river answers its source

A river has one **gauge**: the level at its source end. For a river with a source body the gauge is that body's
level, its stage included. For a spring-fed river it is the river's own stage, which the game sets inside the
range the solve reports.

The river is solved once for each **flow sample**: for a source body, at levels across its stage's range no more
than 0.3 m apart; for springs, at the discharges the game declares. Each sample records its gauge level. At any
moment every chart row of the river (level, current, loss and gains) and every fall record is the mix, linear in
the gauge, of the two samples whose gauge levels bracket it, in every cell both samples wet.

**Where the two samples differ in what is wet**, a cell holds water only from its **wetting gauge**, and
water spreads from where it is. Preparation floods outward in order of gauge from two kinds of place:

- every cell the lower sample wets, from the lower sample's gauge;
- every **inflow place** the upper sample feeds and the lower leaves dry, from the gauge its inflow starts:
  the cell where a spring enters, from the lower sample's gauge, since its discharge rises from nothing
  there; and the cells where water arrives from the reach above, where a fall's path lands or at an end two
  reaches share, from the wetting gauge of the cell above that feeds them. A river's reaches are flooded
  from its sources down, so that gauge is known before the reach below is flooded, and every chart reads
  the one gauge.

A wet cell passes water to a neighbour that the upper sample wets at the least gauge at which its own level
stands at or over that neighbour's bed: at once to a neighbour downhill, and to a bank, a notch or a ridge
when its level has risen that far. A cell's wetting gauge is the least gauge at which any neighbour passes
it water. A cell the upper sample wets that no neighbour's level ever tops, because the water climbs to it
by its speed, takes the wetting gauge of the neighbour the upper sample's flow enters it from. Below its
wetting gauge a cell has no water, no wave support and no shore margin, and the level the map gives there
lies under its bed. From its wetting gauge to the upper sample its level runs linearly from its bed to the
upper sample's level, and its current, loss and gains from nothing to the upper sample's.

What follows from the rule:

- **A bank** beside water both samples hold wets where that water's mixed level meets its bed, and from
  there rises with it, so the waterline moves up a bank as it did.
- **A hollow behind a ridge** waits for the ridge. It is dry until the water beside the ridge tops the
  ridge's lowest point, and then fills from its bed as a level pool.
- **A reach the lower sample leaves dry** is dry until the lip above it runs, and from that gauge is wet
  from its landing down, deepening from nothing. A part of a fall that only the upper sample runs gives
  nothing below its lip cell's wetting gauge; from there its discharge and launch depth rise linearly from
  nothing, along the upper sample's path at the upper sample's launch speed. So a sheet never pours onto a
  dry reach, and no reach is wet under a dry lip.
- A cell the lower sample wets and the upper leaves dry dries where its mixed level meets its bed.

[Part 8](../../../.orchestrate/evidence/RIVERS/design-proof/README.md) runs the first design review's case, a channel beside
a hollow behind a ridge with a notch. The first design's rule, a dry cell at the nearest wet level, puts
0.80 m of water in the hollow at the low sample. With wetting gauges the hollow is dry until the notch wets
at a quarter of the way between the samples, and the wet cells are those of a quasi-static flood at every
gauge tried, each joined to the channel.

[Part 10](../../../.orchestrate/evidence/RIVERS/design-proof/README.md) runs the second review's case and the rule as it
now stands. A sloping reach between two falls, dry in the lower sample, has no wet cell to flood from, and
a flood seeded from the lower sample's water alone gives none of its 32 cells a gauge. Seeded where the
fall lands, at the gauge the lip above wets (0.30 of the way), all 32 take that gauge: the reach is dry
below it, wet and joined to its landing at every gauge above, 0.029 m deep at the half-way gauge, and the
upper sample's to rounding at the upper sample. Part 8's case gives the same gauges and wet cells as
before, and the hollow fills level. In a dry reach that ends in a pool, a hollow behind a ridge stays dry
until the pool tops the ridge's notch, at 0.65. A branch 2 m above a wet channel wets from its own landing
at its lip's gauge; by the first revision's rule, the crossing of a cell's own two stored levels, it would
have waited until 0.98, and a ridge in a dry reach would have wetted with the pool beside it 0.075 m under
its bed. A run that climbs a rise by its speed is reached through the neighbour that feeds it.

The mix is the map's, in `thrum-water-model` and in WGSL alike, so every reader has it and no reader sees a
river the query does not ([ruling 2](../ARCHITECTURE.md#seam-rulings-for-rivers)). A chart with one sample is
exactly `WATERS`' chart, and its stage adds to its level as before.

Part 3: between samples 0.3 m apart the mixed row is within 5.6 mm in depth and 19% in speed of a solve at that
level; 0.15 m apart, 2.6 mm and 10%. Declared approximations: the mix is not itself a steady flow, so between
samples discharge is not quite the same through every section; a cell that waits for its connection fills
linearly from its wetting gauge, not at the rate its inflow would give; a reach that starts to run is wet
along its whole length at once, and a pool on it fills as the gauge rises, where a steady flow would have it
full from the first trickle; a cell reached by the water's speed wets with the cell that feeds it, which may
be early; and the whole river answers at once.
A rise does not travel. A prepared travel time for each cell, read against a short record of the gauge, would make it
travel with no change to a reader, and is not built.

### The flow's own waves

**Relief.** Whatever the solve resolves is in the chart's level and stands still: the draw-down over a sill, the
chute between two rocks, the rise in front of a rock (0.12 m where the water runs at 1.1 m/s, part 6), the step of a jump. It is drawn,
lit and queried as level, by `WATERS`.

**Flow bands.** Two more wave layers in the world's one field, beside the wind's three cascades and of their
kind: periodic patches of 256², near 3 m and 13 m long, each a band of an isotropic spectrum, evolved to the
frame's time by its own dispersion on the cascades' whole-number loop, with slope and Jacobian moments in its
mips. The wind does not fill them and their power never changes. They are carried by the current as the
cascades are, by `WATERS`' two cross-faded samples, so a ripple heading upstream at the water's own speed stands
still over the bed and the rest run down.

**Their gain is the flow's.** Bed friction is what stirs a river's surface. Where the depth is `d` and the
friction slope `S = n² u² / d^(4/3)`, the chop's rms height is

`η = c · d · min(S, S_cap)`, with `c = 2` in the first profile and `S_cap = 0.6 / (4c)`.

So the roughness is the share of the depth that the friction slope is: with no speed there is none, and no gain
is tuned by hand. The cap is where the chop's significant height, four times its rms, meets
[`WATERS`' depth limit](../WATERS/DESIGN.md#waves-on-a-body-gains) of 0.6 of the depth. The height is split
between the two bands by depth: all in the short band at 0.15 m and shallower, all in the long one at 1 m and
deeper, by the logarithm of the depth between. Each band's gain is its share over the band's own rms height.
Part 2: 7.7 mm in a riffle 0.14 m deep at 1.1 m/s, 0.1 mm in a pool, 14.7 mm in a chute at its cap.

A flow band is a wave layer in every rule `WATERS` has for one: the surface's vertices and pixels, the slope
covariance that sets roughness and glitter, the depth limit and the swash at a bank, the query, the Jacobian the
foam's births read, the caustic field and the lens test. A body that is not a river has no flow gain. The
wind's layers stay on a river at the gains its fetch gives.

**What a game reads.** [`WATERS`' query](../WATERS/DESIGN.md#bodies-and-the-query) answers the flow bands as it
answers a cascade: from the frame's evaluation, at the mip the point's footprint chooses, at the chart's gains
at the point, with the current's carriage applied as the field is read, and brought from the frame's `t` to
the point's use tick by their rate of change. So a float in a windless riffle rides the chop drawn under it,
and a hull wider than a band's waves does not feel that band. WP5.10 holds the dispatch to the model's bands
(WP3.5) read the same way, at the tolerance `WATERS` WP5.1's test states for a cascade. A declared approximation,
`WATERS`' own for anything brought forward in time: the short band's strongest wave, 0.75 m long, passes a
fixed point in a 1.1 m/s current at up to 18 rad/s, so brought two ticks forward it is misstated by up to a
fifth of its height, which is millimetres. The band's shortest waves are not followed to the phase, and hold
little of its height.

**The query's demand.** That needs the bands evaluated at the `t` of the frame that answers, and the bands'
producer is one a cadence may skip. A skipped producer leaves the field it last wrote, which the display
carries forward by its rate of change with its slopes held. A game must not read that: at a crest the held
rate is nothing, so one tick on the carried wave still has its whole height, where the 18 rad/s wave above
has 95.5% of it and is falling, and brought on to the use tick the two answers part further. So the producer
runs on a frame when either reader asks:

- **the display**, at the bands' cadence, while a chart that holds a flow gain is inside a view's reach;
- **the query**, on every frame whose dispatch holds a posted point over a chart that holds a flow gain,
  whatever the cadence and wherever the camera is. The scene decides it on the CPU from the tick page's
  points, before `simulate`. Its test may ask when it need not, as a point anywhere in such a chart's
  rectangle does, and never fails to ask.

The producer writes the whole field from `t` alone and runs before the dispatch, so every dispatch that reads
a flow band reads it as evaluated at its own frame's `t`. An answer is then the same to the bit at any cadence
and with the camera anywhere, the river out of every view included, and WP5.6 holds the mailbox to that. There
is still one field: an evaluation the query asked for is the display's newest too, and the display carries on
from it. What a lower cadence saves is the frames no dispatch needs: all of those it skips while no posted
point is on a river, and the repeat frames between ticks while one is. It is `EFFECT-GRAPH`'s
[skippable pass](../EFFECT-GRAPH/DESIGN.md#cadence), which runs or not by a runtime value, so nothing is asked
of that module.

`c` is the one constant of the look. It starts at 2, between the turbulent energy at a channel's surface (0.65)
and a bed of cobbles, and is tuned once against R2's inspection, between 1 and 4.

### Whitewater from the flow

The solve keeps an energy account. In each cell the head the flow loses, times its discharge, is the power it
gives up there; part 2 closes it within 0.3% against the discharge times the head between the ends. That power
has two parts: bed friction's, and the rest, which is lost in jumps, at ledges and in the wakes of rocks. The
chart's **loss** is the power that breaks the surface:

`W = ρ g · ( max(0, lost − friction's) + |q| · max(0, S − S_cap) )`

The first term is the head lost beyond friction. The second is friction's own power beyond what the chop can
hold, which is what whitens a steep chute. Part 2: 529 W/m² at a 0.4 m ledge and the jump below it, 66 W/m² down
a chute of 12%, under 7 W/m² everywhere else. Part 6: 80% of the head lost beyond friction is lost round a rock
and below a narrows, in 34% of the wet cells.

**Births.** Where a chart has loss, `WATERS`' foam is born at the bounded rate `dc/dt = r (1 − c)` with `r` in
proportion to `W`, by the constant the shallows' births use for the power the depth limit takes, so one water
type foams alike on a beach and below a ledge. A share of the birth goes to submerged air, so the water below a
ledge is pale before it is white. Nothing else is new: the stocks, their chain, their carriage by the current
and their look are `WATERS`'. A rock is ground, so its foam is the loss of the flow round it; a post or a hull
standing in the current foams by [`WATERS`' contact births](../WATERS/DESIGN.md#whitewater).

### The fall's sheet

**Geometry.** A sheet is a strip along the fall's lip, drawn down the record's paths to the level of the body
below, where it is cut. Its points are the record's, mixed by the gauge.

**What the water does on the way down**, at a point the water reaches after time `τ` and a drop `z`:

| Quantity | Value | Source |
|---|---|---|
| Speed | `v = √(v₀² + 2 g z)` | Energy; on a face the rock holds it is the same speed, a declared approximation |
| Thickness | `δ = q / v` | Discharge for each metre of lip is kept |
| White share | Rises smoothly from 0 at the lip to 1 at the breakup length `L = 6 q^0.32` m of path | Horeni's length; a profile value |
| Presence | `min(1, δ / δ_tear)`, with `δ_tear` 5 mm in the first profile | A sheet thinner than this is strands with gaps |

Part 4: a trickle of 0.005 m²/s is past its breakup length after 1.1 m and is 1 mm thick, so it is a few
strands; 1 m²/s holds a whole sheet for 6 m and is still 64 mm thick after 12 m.

**Pattern.** Each part of the sheet left the lip at a known time, `a = t − τ`, at a known place along it. The
sheet reads `WATERS`' foam pattern at (the place along the lip, `a` times the launch speed), on the cascades'
loop. So every streak falls with the water that carries it, at the water's speed, and stretches as that speed
grows, with no cross-fade and no state. The white share and the presence threshold the pattern, as cover
thresholds it on a surface: where the sheet is whole and clear it is a slab of the water type that shows the
rock behind it; where it is white it is foam; where presence fails there are gaps.

**Light and the frame.** The sheet is lit by the water's own functions: the environment's reflection, the true
Sun through the air, the cloud factor once and the scene's Sun visibility
([ruling 7](../ARCHITECTURE.md#seam-rulings-for-rivers)), and foam's diffuse light. It is drawn from both sides,
in a pass of the water's contribution that runs before the water's gather. So the pool mirrors a fall that is on
screen and shows it through its surface, as it does the land, and the sheet needs no copy of the scene: what
lies behind a clear sheet shows by blending. It writes scene depth where presence passes one half, motion from
where each part of the pattern was at the last presentation, and reactive coverage where it is white. A fall is
a function of time, so a pause holds it and a capture shows it at the capture's time.

### The plunge

Each tick, for each segment of a fall whose path meets water, at the place it meets the level:

- **Births.** Foam and submerged air, at the rate the fall's power gives: `ρ g q z` for each metre of lip, over a
  footprint as wide as the sheet is thick there and never under the foam's cell. The air's share is the
  profile's. It surfaces downstream as `WATERS`' chain says.
- **Impulses.** One [impulse record](../WATERS/DESIGN.md#interaction-waves) with the segment's stable ID. A steady
  fall presses steadily, and pressed whole on the surface its momentum would dent it half a metre (part 4): the
  water goes under. What makes rings is the unsteady part. The impulse is
  `α · ρ q v_n · w · Δt · ζ`, where `v_n` is the speed into the surface, `w` the segment's width, `ζ` the
  sheet's own pattern arriving at that segment at that tick, with its mean over the pattern's loop taken out
  and its variance 1, and
  `α` the profile's wave share, 0.1 at first. So rings come from the strands a viewer sees land, stop when the
  fall is dry, and are the same causes on every machine. `α` is a declared closure, tuned by eye against
  WP6.5's rings and recorded. Its ceiling is physical, the waves' power never exceeding the fall's, and it is a
  diagnostic for a plunge that looks wrong: `thrum-water-model` gives the fall's power and each impulse
  (WP2.5), and no item asserts the ceiling.

Rings exist where the interaction windows are, which is round the body the game names
([`WATERS`' reach](../WATERS/DESIGN.md#interaction-waves)). A path that lands on bare ground gives neither.

### Rings on running water

`WATERS` carries a window's interaction waves by the mean current of its wet cells. A pool joined to a riffle
is one region, and no one current carries both: with rings on each, any single current leaves each set
3.25 m from its own water after 5 s, and a stronger source on the riffle drags the pool's rings.

So the step has a **current ladder**, as it has a depth ladder
([ruling 5](../ARCHITECTURE.md#seam-rulings-for-rivers)). When a window's regions are labelled, each region
takes a few **carriage currents**, four at most in the first profile. A current is a vector. The carriage
currents are chosen, in the plane of current vectors, so that their span holds the currents of the region's
wet cells: the line between them where a reach runs straight, and the triangle or quadrilateral between
them where it turns. A carriage current need not be any cell's own. The step transforms the state once,
transforms back once for each carriage current in use at each ladder depth, and each cell takes the blend
of the outputs carried at the two or three carriage currents whose span holds its own, weighted by where
its current lies between them. Those weights are never negative, sum to one and give back the cell's own
current. A current the span does not hold takes the nearest point of it, and is carried at a current that
differs from its own by that distance; the step reports each region's largest such difference. On still
water and on a uniform current the ladder has one current, and the step is what `WATERS` built, to the bit.

A **solve** is now one region at one ladder depth and one carriage current, inside the profile's same count
of 16. Where a level would pass it, regions give up carriage currents first, down to the one mean, before a
region is left at rest, and the step reports what it dropped.

[Part 9](../../../.orchestrate/evidence/RIVERS/design-proof/README.md), a section with a pool at 0.2 m/s and a riffle at
1.5 m/s in one region and a ring set raised on each at once: with the ladder each set's centre is within
0.10 m of where its own water carries it after 5 s, in steps of 1 and of 8 ticks, and the pool's is
unmoved when the riffle's source is five times stronger. Declared approximations: a wave on water whose
current lies between two carriage currents is a blend of two carried copies and loses energy, 0.2% a
second midway between four currents and 1.3% between two, and never gains; rings that cross from a pool
into a riffle pass through that blend.

Part 11 is arithmetic on the span alone, for a pool at 0.2 m/s and a bend that turns a 1.5 m/s current.
Carriage currents taken from the cells' own, the pool's and three on the bend, leave the bend's cells up to
0.11 m/s outside their span through a turn of 90°, which is 0.57 m after 5 s, and 0.03 m/s through 45°. Rest
and three currents on the bend's tangents hold every cell.

**A ring survives the blend.** The blend is a blur: a cell that mixes copies carried at different currents
spreads its waves by the distance those copies part in one step. Part 12 takes it from the step's own
multiplier, in plan, for a ring 0.5 m wide. Each run gives every cell the one current and the one set of
weights, and the figure is the share of the ring's height energy left after 5 s, against the same ring
carried at one current. In steps of 1 tick the ring keeps 93.6% midway between 0.2 and 1.5 m/s on a straight
reach. For a 90° bend carried by rest and three tangent currents, the runs sampled are five speeds from the
pool's to the riffle's at headings every 5° round the turn, and the least kept among them is 88.7%. So a ring
on blended water softens and is still a ring, and WP6.10 fails a step that leaves less than half. Steps of 8
ticks blur more: 5 s of nothing but such steps would keep 50% to 72% in the same runs. A hitch gives one such
step, and no item asserts its energy. A ring that travels through weights that change from cell to cell, as
on a real bend, is a different calculation, which part 12 does not make: WP6.10's ring on the bend is its
first run here.

The ladder is built in `thrum-water`'s interaction step. An f64 reference of it in `thrum-water-model` is a
diagnostic a builder may add, as [`WATERS` says of its own step](../WATERS/DESIGN.md#the-system-as-it-runs),
and is not required.

### The chart store

A solve's output is content: charts, fall records and the gauge's levels, as one **river file**. Its **key** is a
hash of the bed inside the rectangle, the declaration, the solve's profile and the solver's version. The solver
and the file's writer are `thrum-river-model`; the reader is in `thrum-water-model`, so a game that ships solved
rivers does not link the solver.

A game loads a river file by its key at preparation. If none matches, a native game may solve and write one, and
says that it did. A page never solves: its package holds the files, and a missing or stale one is refused by
name. Changing the ground changes the key, so the next preparation solves again; nothing is solved while the
game runs. River files are generated, like cooked content, and are not committed. They are not a `CONTENT-LOOP`
product: the map needs them on the CPU, and a lease lends views on the GPU.

### Capabilities, recipes, limits and knobs

- **Capabilities**, added to [`WATERS`' list](../WATERS/DESIGN.md#capabilities-recipes-limits-and-knobs). A scene
  that leaves one out declares none of its passes or resources, and the surface's shader is prepared without
  its reads.

  | Capability | What it adds | Needs |
  |---|---|---|
  | Flow bands | The two bands and their gains on charts | Waves |
  | Falls | Sheets, and plunges where the scene has whitewater or interaction waves | A river with a fall record; a foam pattern |

  A solved river needs neither: it is then drawn as `WATERS` WP9 draws a chart.
- **The physical profile**, never a quality setting (C04): the flow samples and the gauge's mix, the brink rule,
  the chop's law and bands, the loss, the sheet's laws, the wave share and the segments of a plunge.
- **Limits are inputs.** The flow bands are layers of the cascades' arrays and the samples layers of a chart's,
  counted against the capability record at preparation.
- **Knobs**, in `EFFECT-GRAPH`'s list with a declared invalidation: the flow bands' cadence, and the sheet's
  spacing along its path. Neither changes what a query returns: the cadence is the display's, and
  [a query's demand](#the-flows-own-waves) runs the bands whatever it is set to.

### Check that it can work

- **R1.** Parts 1, 2 and 6 run the scheme: at rest, a jump within a cell of its closed form, friction against a
  made bed, a steep thin sheet, a pool level to 15 mm behind its sill, a narrows where the water quickens 2.5
  times as its width falls to 40%, and discharge the same through every section within 0.2%. The reproduction
  is first order in space and 64 by 16 cells in plan. The solver in Rust, its second order and a whole valley
  are unrun; WP1 is the first.
- **R2.** Part 2 gives the chop's size along a section from the law. That two bands at those sizes, carried by
  the current, read as a riffle and not as a small sea is unrun; WP5 is the first look ([risk 2](#risks)). Carriage
  by two cross-faded samples is Uncharted 4's, and `WATERS` WP9 measures it.
- **R3.** The current is the map's and the query is `WATERS`'. Unrun here. That a float's answer over a riffle
  does not change with the bands' cadence or the camera follows from the schedule alone: the bands are a
  function of `t`, and [a query's demand](#the-flows-own-waves) evaluates them on the frame that answers. WP5.6
  is the first run.
- **R4.** Parts 2 and 6 put the loss at the ledge, the chute and the rock. How much foam that power makes with
  the shallows' constant is unrun; WP5 records it.
- **R5.** Part 4 is arithmetic, and a section for the depth at a lip. The sheet's look is unrun; WP6 is the
  first ([risk 4](#risks)).
- **R6.** Part 3, in section, for cells both samples wet; parts 8 and 10, in plan, for what is wet between
  two samples: a hollow behind a ridge, and a reach that goes from dry to running. Both work on levels
  written by hand. The solver's own samples of a valley, and its own lip that starts to run, are unrun; WP3
  tests them.
- **R8.** Part 7 is arithmetic on part 2's flow. Part 9 steps two ring sets at once on a pool and a riffle in
  one region, in section. Part 11 is the span of the carriage currents round a bend, and part 12 what the
  blend costs a ring's height energy, in plan, from the step's multiplier, with one set of weights in every
  cell. The ladder stepped in plan and on the GPU, and a ring crossing weights that change along a bend, are
  unrun; WP6.10 is the first.
- **What the looks cover.** The island's fall is about 4 m, and WP6 judges it from 2 m to 200 m at three
  gauges. Falls of other heights in R5's range, 0.75 m to 12 m, are held to the sheet's closed forms (WP2.4)
  and are not judged by eye: the ledger says so until a game shows one.
- **Cost.** Unrun. WP1 records the solve and WP4 the first frame.

## The system as it runs

| State | Owner |
|---|---|
| The solver, the brink rule, the energy account, the wetting gauges, the river file's writer | `thrum-river-model` (this module), on `thrum-water-model`, no GPU dependency |
| A river's samples and gauge in the map's function; brink ends; fall records and the sheet's f64 reference; the plunge's births and impulses as a function of the tick; the flow bands' spectrum, evaluator and gains; the river file's reader | Files this module adds to `thrum-water-model`, and its changes to the map's function there under [rulings 1 to 3](../ARCHITECTURE.md#seam-rulings-for-rivers) |
| Samples and wetting gauges in the map on the GPU; the flow bands' passes; births from loss; sheets; plunges; the current ladder | Files this module adds to `thrum-water` and `thrum-sea`, and its changes there under rulings 2 to 7 |
| The map, the surface, optics, the query, interaction waves, foam, carriage | `WATERS` |
| The rivers' declarations, the island's valley, spring, sill, rocks and cliff, Manning's `n`, the river selection, the river controls | `examples/water` |
| Sky, Sun, air, the lent Sun visibility | `ATMOSPHERE` |

**A frame, in order**, for what a river adds to [`WATERS`' frame](../WATERS/DESIGN.md#the-system-as-it-runs).
`upload` takes the gauges with the page's stages. `simulate`, with the cascades and before the query's
dispatch: the flow bands at `t`, on a frame the display's cadence or
[a query's demand](#the-flows-own-waves) asks for them. `simulate`, on a frame that advances the tick: births from loss with the foam's births; the plunges'
births and impulses; the interaction step with each region's current ladder. `after-opaque`: the sheets, then
the water's gather and surface as before.

**Creator journey.** A game declares a river: a rectangle, its sources, the bodies it may reach, Manning's `n`
and its flow samples. It adds the capabilities it wants to its water profile. A build step, or a native game's
first preparation, solves the river and stores it. The game sets a spring-fed river's stage with the tick, as it
sets a lake's. Moving a rock, a sill or the valley changes the key, so the river is solved again and its foam,
its relief and its falls move with the ground. Changing a river's declaration rebuilds the water, as changing
any body does.

**The example scene.** `examples/water` gains a **river selection**, off by default
([ruling 6](../ARCHITECTURE.md#seam-rulings-for-rivers)). With it on, the island has a valley in the hills above
the lake with a spring, a riffle, a pool behind a sill, a narrows with rocks and a fall of about 4 m into a
plunge pool that runs on to the lake; and the river from the lake to the sea is solved, in the channel's cut,
in place of the hand-made chart. With it off the island is what `WATERS` built, to the byte.

## What RIVERS asks of other modules

Settled in [the module list](../ARCHITECTURE.md#seam-rulings-for-rivers), each built by this module's builder in
`WATERS`' crates, with no item, test, bound or reference of `WATERS` or `SEA` changed and all kept passing. A
scene with no river and neither capability compiles to the shader text, passes and storage it has today.

1. A chart's end may be a brink (WP2).
2. A chart may hold several flow samples that a gauge reads between, with a wetting gauge for each cell (WP3 on
   the CPU, WP4 on the GPU).
3. Flow bands: wave layers the wind does not fill, at gains a chart holds (WP3 on the CPU, WP5 on the GPU).
4. Births from a chart's loss and at a plunge (WP5 and WP6), which `WATERS`' design already books.
5. A current ladder in the interaction step (WP6).
6. A river selection in `examples/water` (WP4).
7. The sheet is drawn in the water's contribution before its gather (WP6), and water's direct Sun takes the
   lent Sun visibility (WP7, after `WATERS:WP16` lends it).

Used as built: the chart and its joins, carriage, the foam stocks, impulse records, the query, the foam
pattern's lease, the typed controls. Nothing is asked of `EFFECT-GRAPH`, `ATMOSPHERE`, `CONTENT-LOOP` or
`SCENE-VIEWS`.

## Cost account

Estimates, unmeasured.

| Work | Rate | Size | Estimate |
|---|---|---|---|
| The solve | Prepared, once for each flow sample | Wet cells times steps. A 350 m stream 4 m wide at 0.25 m cells is 22,400 wet cells and about 24,000 steps: 540 million cell updates a sample (part 5) | Tens of seconds a sample on one core; WP1 records it. Each cell's own time step saves a third; a filled start and a coarser grid first save little in a section |
| A river file | Loaded at preparation | For each reach: its rectangle's cells × samples × 7 values at 16 bits. A 300 m by 30 m reach at 0.25 m cells with 6 samples is about 12 MiB | Cutting a winding river into more reaches is the remedy where a rectangle is mostly ground |
| Samples in the map | Every read of a chart with more than one sample | Two layers in place of one, and the wetting gauge | Three reads on river pixels; none elsewhere |
| Flow bands | Frame, at their cadence while a chart with a flow gain is inside a view's reach; and every frame that answers a query point over such a chart, wherever the camera is | Two transforms of 256² and their mips | About half the cascades' cost again; two more layer reads in the surface where a gain is not zero. With a float on a river the cadence saves only repeat frames, half the frames at a 60 Hz tick and 120 Hz |
| Births from loss | Tick | One read of the chart's loss in the foam's births | Inside the foam's pass |
| Sheets | Frame, while a fall is in view | A lip's cells × up to 32 points; the sheet's pixels, blended | Small beside the surface; WP6 records it |
| Plunges | Tick | A record and a birth for each segment | Negligible |
| The current ladder | Tick, for each region whose currents differ | One more transform back for each carriage current beyond the first at each ladder depth, inside the profile's 16 solves a level; choosing the currents is CPU work when regions are labelled | Up to four times a still region's step where a pool and a riffle share a window; nothing on still water or a uniform current. WP6.8 records it |

Records and tripwires are [`WATERS`'](../WATERS/DESIGN.md#cost-account): `scripts/water_cost.py`, with scenes this
module adds, and `scripts/river_cost.py` for the solve. No timing figure decides an item
([brief](../VISION.md#goal-and-exit-rule), decision 3).

## Risks

1. **The solve is slow, or will not settle**: a pool that sloshes, a wet and dry edge that flickers. Early sign:
   WP1's report. The designed remedies are each cell's own step, a start from filled pools, a stopping rule on
   discharge between sections, and a coarser grid first. The wrong fix is stopping on a step count.
2. **Flow bands read as a small sea, or pulse.** Early sign: WP5's first captures. The designed remedies are the
   bands' sizes and spectrum, a period that varies from place to place (`WATERS`' risk 13) and `c`. The wrong
   fixes are a painted amplitude and a scrolled normal map.
3. **Too much foam, or none.** One constant serves a beach and a ledge. Early sign: WP5's record of cover at the
   test valley's stations. The designed remedy is a constant of the water type for flow, recorded with its
   reason. The wrong fix is foam where the surface is steep.
4. **The sheet reads as a card**: flat, evenly lit, hard at its foot. Early sign: WP6's captures. The designed
   remedies are the pattern's relief as displacement, the white share and presence, and the plunge's foam and
   air at the foot. Three failed looks on one cue go to the log. Mist is the remedy this plan does not have.
5. **A seam where a river meets a lake, a fall's foot or the sea.** Early sign: WP4's joins. The map's joins are
   `WATERS`'; the solve gives a level at each end that already matches the body there.
6. **`WATERS` moves under this module's changes**, since both builders edit its crates. Early sign: a rebase
   that fails `WATERS`' tests. Each ruling keeps its change behind a declaration that existing scenes do not
   make, and each work package here runs `WATERS`' tests.
7. **A reach's rectangle is mostly ground.** Early sign: WP4's bytes. Remedy: more reaches.
8. **Samples in the map cost the surface too much.** Early sign: WP4's record. Remedy: mix the two samples into one
   set of layers when the gauge moves, a few times a second, and read one.
9. **A steady flow looks frozen**: nothing surges. Early sign: WP5's sequences. The flow bands, the carried
   cascades and the foam are what moves; a slow wander of the gauge is the game's. The wrong fix is noise in the
   level.
10. **The current ladder multiplies a window's solves** where a pool and a riffle share it. Early sign: WP6.8's
    record, and carriage currents dropped in WP6.10's report. Remedy: fewer carriage currents, which part 9
    prices at 1.3% of a midway wave's energy a second for two. The wrong fix is one current again.

## Proof and builder handoff

[`WATERS`' handoff](../WATERS/DESIGN.md#proof-and-builder-handoff) holds here word for word: targets, capture,
cost records, looks, the `gpu` lock, exact baselines, tests that hold no game content, Broadsides and
dependencies. In addition:

- **Scratch and evidence.** `tmp/rivers-<work-package>/` and `.orchestrate/evidence/RIVERS/`. Copies are listed in
  `.orchestrate/evidence/RIVERS/PROVENANCE.md`; none is expected.
- **`WATERS`' crates.** A change inside a file `WATERS` wrote is one a ruling names, behind a declaration no
  existing scene makes. Anything else it needs is a proposal in this module's log.
- **The bar.** `WATERS`' handoff carries [the owner's bar](../../../ORCHESTRATE.md#what-is-proved-and-how-hard) of 17 November 2026,
  and it holds here: WP4 to WP7 pass on their look and their speed, and what a game reads is tested against
  `thrum-water-model`: the river in the map and the query at the tolerance `WATERS` WP4.1's test states, and
  the flow bands in the query's answers (WP5.10) at the one `WATERS` WP5.1's test states, the same to the bit
  at any cadence and camera (WP5.6). The look of the sheet,
  the flow bands and the river's caustics is not held to an f64 reference or to a bound measured first, and
  nor are the rings: WP6.10 compares the step with itself. Parts 1 to 10 are the source of the bounds WP1 to WP3
  were proved with and of WP6.10's carriage figures, and part 12 of its half.
- **The depth a river writes (WP4.3).** Lead's decision, 26 November 2026, after two reviews failed WP4.3 on
  one clause: the depth the water writes inside a river stood up to 1.4 mm from `thrum-water-model`'s
  surface in the first and 5.2 mm in the second, against the 0.040 mm of `WATERS` WP4.1's test. Each of WP4.3's three bounds is now set at the size
  at which its fault shows, and the draw is held to the map's wet and dry at each wetting:
  - *The map's rows, at that tolerance.* The map is what the query, the floats and the surface all read
    ([ruling 2](../ARCHITECTURE.md#seam-rulings-for-rivers)). A wrong sample mix or wetting gauge there is
    silent in a frame and costly to a game, and a compute pass reads the rows exactly.
  - *Along a river, within 0.010 m.* 0.040 mm is no bound for a pixel. It is about one single-precision step
    of a world coordinate at the island's rivers (0.031 mm at 330 m), so a place rebuilt from a frame's depth
    barely resolves it. The drawn surface is a mesh that carries its wave displacement between vertices,
    where the model evaluates each place; the two part by millimetres where the level curves and where the
    swell enters, smoothly, with nothing drawn beside the water to show it. 0.010 m is the size this design
    already works at: between samples the mix is itself up to 5.6 mm from a solve at that level
    ([part 3](#a-river-answers-its-source)), and WP9's off-sample comparison allows 1 cm. At 0.010 m a
    waterline moves at most 0.1 m on a bank of 1 in 10, and a float, which rides at the query's level, sits
    at most 1 cm off the drawn surface, inside its own bob. The bound still fails water drawn from the wrong
    sample or off its chart, which is wrong by centimetres. It is read along each river's whole wet length
    and off the mesh's vertices, because both reviews found their misses between the stations the test had
    chosen.
  - *Either side of each wetting, drawn wet or dry as the map has it.* Design review, 26 November 2026. The
    map's rows can be right and the draw wrong: water drawn in the hollow from the upper sample while the
    ridge is dry is a pool cut off from its source, with every wet place at its height and every join whole.
    So the scene that proves [the wetting gauges](#a-river-answers-its-source) in the map is drawn too, and
    read at the hollow and along the dry reach at gauges either side of each wetting: where the map has the
    place dry the water writes nothing above the ground, and where the map has it wet the written depth is
    within the same 0.010 m. That allowance holds wherever water is drawn, and excuses none drawn where the
    map has none.
  - *Across each end's edge, at that tolerance.* A step there is a crack along a line, seen from the bank
    however small, and [risk 5](#risks) is this. The step is what the two sides differ by beyond what the
    model's surface differs by between the same two places, so a wave both sides share cancels and the
    tight bound is one a frame can meet: WP4 measured 4.8 µm at the worst.
- **Maturity.** A ledger row for `E1.rivers` is added in WP4 and kept current.

## Open questions

| Question | Settled by |
|---|---|
| What does a solve cost in Rust, for the test valley and the island? | WP1 and WP4 record it |
| Does `c = 2` read right, and do two bands suffice? | WP5; [risk 2](#risks) |
| Does the shallows' birth constant serve a river? | WP5 records the cover; [risk 3](#risks) |
| What wave share makes rings that read? | WP6.5's look, tuned by eye from 0.1 and recorded. The fall's power is a diagnostic ([the plunge](#the-plunge)), and no item asserts it |
| How many reaches does a winding river need? | WP4's bytes; [risk 7](#risks) |
| Does a fall need mist to meet its bar? | WP6's review; the owner ([plan](PLAN.md#open-for-the-human)) |

## Compact river solver fixtures (WP8)

Use small CPU fixtures for the listed cases. For closed-form, stopping and serialization checks, use a
short reach with enough steps to exercise the solver and its report. Keep the terrain features the other
assertions distinguish: riffle and pool, sill and narrows, ledge and rock, and the lake joins. A cost
comparison must still double dry padding around the same wet reach. Where a test reads a gauge, flow band,
or fall record, use only the samples and geometry needed to exercise the interpolation, wetting and
dry-reach cases. Use already-made charts for checks whose subject is sampling, with the solver-to-chart
and river-file round trips still exercised by the tests that own those boundaries.

## Compact chart gauge and determinism fixtures (WP9)

The chained-gauge test builds two 136 × 64 lake valleys at 0.25 m spacing, solves both across their
gauge samples and solves the lower valley again at an interior gauge. Most of that work sets up a
map test. Use compact charts for the lake → upper chart → lower river chain, with a gauge point
between four cell centres and sample endpoints and intermediate levels that change the lower
charts and falls. Keep refusals for a chart source without a place, a non-chart or out-of-footprint
source, a self-source, insufficient gauge range, reusing a chart already in a river, and a
manual gauge write to the dependent river. Compare the lower charts with the independent sample
mix at wet points beyond join blends and check that fall discharge rises with the lake. Keep an
independent fresh solve on a compact lower reach for the off-sample comparison, with the existing
1 cm depth and 25% speed tolerances.

The determinism test runs the 10,752-cell valley to convergence three times. Use the smallest
quickly converging reach that still crosses the stepper's parallel threshold of 1,024 active span
cells with work in multiple row chunks. Verify that the eight-thread solve actually exercises that
branch. Keep two one-thread solves and one eight-thread solve and compare the same serialized river
file and the raw depth, both discharge components and loss bytes. This guards the real serial and
parallel solver paths, including their stable result ordering.

## Compact river-file gains fixture (WP10)

`chop::each_sample_of_a_river_file_carries_its_own_gains` was outside WP8's nineteen cases and WP9's two. It
still solves the 10,752-cell valley at 0.25 m twice, on the stepper's worker threads. Alone on 26 November
2026 it took between 4.6 s and 33 s and used between 5 s and 14 s of CPU. Its steps wait on those threads,
so on a busy Mac it waits far longer than it works: 33 s for 5.5 s of CPU. The workspace check stopped it
at ten seconds beside other tests and again at 30 seconds alone ([measurements](../../../.orchestrate/evidence/RIVERS/WP10/workspace-timing.md)).

The test's subject is the river file: each sample's two chop gains are stored with that sample, the first in
the reach and the later ones in the reach's upper samples, and two discharges give different gains. Neither
needs the valley's pool, sill, ledge or rock. Use the smallest spring-fed reach on which both discharges
leave wet footprint cells with gains above zero that differ between the samples. WP8's short spring reach
solves in about 0.01 s and the valley at 0.5 m in about 0.2 s. Keep the two solves and the file made from
them, because the boundary from solve to file is what this test owns: a prepared chart would not cross it.
Keep the exact equality at every wet cell of each reach's footprint in both samples, the count above zero,
and the greater gain at the whole discharge at a cell on the slope.

Keep each solve on the calling thread: a reach under the stepper's 1,024 active cells, or one worker
thread. A one-thread solve slows only in proportion to the Mac's load. The eight-thread path keeps its own
guard in WP9's determinism test, and the riffle against the pool keeps its guard in
`chop::the_chop_follows_the_friction_slope_and_is_far_rougher_in_the_riffle_than_the_pool` on the 0.5 m
valley. The test stays in the fast tier and asks for no device.

The measure of the repair is the test process's CPU time alone, user and system together, because a busy
Mac moves it far less than the clock: 2 s or less, from [the plan's target](PLAN.md#open-for-the-human)
for a fast test.
