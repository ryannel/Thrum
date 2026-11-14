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

**What exists.** `WATERS` M1 builds the map and its chart; its M9 draws a hand-made chart, carries detail and foam
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
| **R7** One water: the river has the lake's light, foam, query, mirror and view from beneath, and no seam where it leaves the lake, lands below a fall or meets the sea | The joins of [W6](../WATERS/DESIGN.md#cues), with a solved chart; a swell at the mouth; dawn to night; a gorge in shadow | A river drawn by its own shader. Rejected by the surface either side of each join, and by a fall in a gorge's shadow, which the Sun must not light |
| **R8** Rings on running water: a stone dropped in a pool sends rings that stay with the pool's water, beside a riffle that runs | A pool at 0.2 m/s beside a riffle at 1.5 m/s in one window | Rings carried at one speed for the whole window. Rejected by the centre of the rings after five seconds |

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
  [part 1](../../evidence/RIVERS/design-proof/README.md). SWASHES' transcritical case is from memory of its
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
| One mean current for an interaction window (`WATERS`, left to this module to judge) | **Change** | Rings in a pool beside a riffle slide 4.7 m in 5 s (part 7). The mean is weighted by where the waves are |
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
deep at 0.25 m cells and 3% at 0.125 m (part 1c); M1.2 holds the solver to no worse. A second-order
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

- **Level**, over the whole footprint. In a cell the sample leaves dry it is the level of the nearest wet cell, so
  it lies under the bed there and the waterline is where the surface meets the ground.
- **Current**, the discharge over the depth; none in a dry cell.
- **Loss**: the whitewater power, in watts for each square metre of surface ([below](#whitewater-from-the-flow)).
- **Flow gains**: one for each flow band ([below](#the-flows-own-waves)).

And once for the reach: its **footprint**, the wet cells of the highest sample grown by a margin of cells whose
bed stands above the level; its **ends**, each a source end, a mouth or a **brink end**; its join lengths; and
the **gauge level** of each sample.

For each fall, a **fall record**: along its lip, for each sample, the path as a line of points with the time the
water takes to reach each, the discharge for each metre of lip, and the launch depth and speed; the reach above and the
body below; and stable IDs for its segments.

The river file holds the loss in watts a square metre. It is handed to the map's chart in the units that chart
documents ([`WATERS` M9.6](../WATERS/DONE.json)); its meaning is fixed here.

### A river answers its source

A river has one **gauge**: the level at its source end. For a river with a source body the gauge is that body's
level, its stage included. For a spring-fed river it is the river's own stage, which the game sets inside the
range the solve reports.

The river is solved once for each **flow sample**: for a source body, at levels across its stage's range no more
than 0.3 m apart; for springs, at the discharges the game declares. Each sample records its gauge level. At any
moment every chart row of the river (level, current, loss and gains) and every fall record is the mix, linear in
the gauge, of the two samples whose gauge levels bracket it. Because each sample's level runs under the bed
where that sample is dry, the mixed level crosses the bed along a line that moves up the bank as the gauge
rises, and a channel that only the high samples wet fills from its upstream end.

The mix is the map's, in `thrum-water-model` and in WGSL alike, so every reader has it and no reader sees a
river the query does not ([ruling 2](../ARCHITECTURE.md#seam-rulings-for-rivers)). A chart with one sample is
exactly `WATERS`' chart, and its stage adds to its level as before.

Part 3: between samples 0.3 m apart the mixed row is within 5.6 mm in depth and 19% in speed of a solve at that
level; 0.15 m apart, 2.6 mm and 10%. Declared approximations: the mix is not itself a steady flow, so between
samples discharge is not quite the same through every section; and the whole river answers at once. A rise does
not travel. A prepared travel time for each cell, read against a short record of the gauge, would make it
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
  fall is dry, and are the same causes on every machine. `α` is a declared closure. Its bound is physical: the
  waves' power never exceeds the fall's.

Rings exist where the interaction windows are, which is round the body the game names
([`WATERS`' reach](../WATERS/DESIGN.md#interaction-waves)). A path that lands on bare ground gives neither.

### Rings on running water

`WATERS` carries a window's interaction waves by the mean current of its wet cells. Each region's mean is instead
weighted by the wave energy each cell holds and by the step's sources, and is the plain mean where there is
neither ([ruling 5](../ARCHITECTURE.md#seam-rulings-for-rivers)). Rings in a pool are then carried by the pool's
current and a wake in a riffle by the riffle's. On a uniform current, and on still water, nothing changes. The
difference between a cell's current and the weighted mean is still not carried, a declared approximation: rings
that spread from a pool into a riffle are carried at a speed between the two.

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

  A solved river needs neither: it is then drawn as `WATERS` M9 draws a chart.
- **The physical profile**, never a quality setting (C04): the flow samples and the gauge's mix, the brink rule,
  the chop's law and bands, the loss, the sheet's laws, the wave share and the segments of a plunge.
- **Limits are inputs.** The flow bands are layers of the cascades' arrays and the samples layers of a chart's,
  counted against the capability record at preparation.
- **Knobs**, in `EFFECT-GRAPH`'s list with a declared invalidation: the flow bands' cadence, and the sheet's
  spacing along its path. Neither changes what a query returns.

### Check that it can work

- **R1.** Parts 1, 2 and 6 run the scheme: at rest, a jump within a cell of its closed form, friction against a
  made bed, a steep thin sheet, a pool level to 15 mm behind its sill, a narrows where the water quickens 2.5
  times as its width falls to 40%, and discharge the same through every section within 0.2%. The reproduction
  is first order in space and 64 by 16 cells in plan. The solver in Rust, its second order and a whole valley
  are unrun; M1 is the first.
- **R2.** Part 2 gives the chop's size along a section from the law. That two bands at those sizes, carried by
  the current, read as a riffle and not as a small sea is unrun; M5 is the first look ([risk 2](#risks)). Carriage
  by two cross-faded samples is Uncharted 4's, and `WATERS` M9 measures it.
- **R3.** The current is the map's and the query is `WATERS`'. Unrun here.
- **R4.** Parts 2 and 6 put the loss at the ledge, the chute and the rock. How much foam that power makes with
  the shallows' constant is unrun; M5 records it.
- **R5.** Part 4 is arithmetic, and a section for the depth at a lip. The sheet's look is unrun; M6 is the
  first ([risk 4](#risks)).
- **R6.** Part 3, in section. The waterline's travel between samples in plan is unrun; M3 tests it.
- **R8.** Part 7 is arithmetic on part 2's flow. The weighted mean on the GPU is unrun; M6 measures the rings.
- **Cost.** Unrun. M1 records the solve and M4 the first frame.

## The system as it runs

| State | Owner |
|---|---|
| The solver, the brink rule, the energy account, the river file's writer | `thrum-river-model` (this module), on `thrum-water-model`, no GPU dependency |
| A river's samples and gauge in the map's function; brink ends; fall records and the sheet's f64 reference; the plunge's births and impulses as a function of the tick; the flow bands' spectrum, evaluator and gains; the river file's reader | Files this module adds to `thrum-water-model`, and its changes to the map's function there under [rulings 1 to 3](../ARCHITECTURE.md#seam-rulings-for-rivers) |
| Samples in the map on the GPU; the flow bands' passes; births from loss; sheets; plunges; the weighted mean | Files this module adds to `thrum-water` and `thrum-sea`, and its changes there under rulings 2 to 7 |
| The map, the surface, optics, the query, interaction waves, foam, carriage | `WATERS` |
| The rivers' declarations, the island's valley, spring, sill, rocks and cliff, Manning's `n`, the river selection, the river controls | `examples/water` |
| Sky, Sun, air, the lent Sun visibility | `ATMOSPHERE` |

**A frame, in order**, for what a river adds to [`WATERS`' frame](../WATERS/DESIGN.md#the-system-as-it-runs).
`upload` takes the gauges with the page's stages. `simulate`, every frame: the flow bands at `t`, with the
cascades. `simulate`, on a frame that advances the tick: births from loss with the foam's births; the plunges'
births and impulses; each region's weighted mean before its interaction step. `after-opaque`: the sheets, then
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

1. A chart's end may be a brink (M2).
2. A chart may hold several flow samples that a gauge reads between (M3 on the CPU, M4 on the GPU).
3. Flow bands: wave layers the wind does not fill, at gains a chart holds (M3 on the CPU, M5 on the GPU).
4. Births from a chart's loss and at a plunge (M5 and M6), which `WATERS`' design already books.
5. A region's mean current is weighted by its waves (M6).
6. A river selection in `examples/water` (M4).
7. The sheet is drawn in the water's contribution before its gather (M6), and water's direct Sun takes the
   lent Sun visibility (M7, after `WATERS:M16` lends it).

Used as built: the chart and its joins, carriage, the foam stocks, impulse records, the query, the foam
pattern's lease, the typed controls. Nothing is asked of `EFFECT-GRAPH`, `ATMOSPHERE`, `CONTENT-LOOP` or
`SCENE-VIEWS`.

## Cost account

Estimates, unmeasured.

| Work | Rate | Size | Estimate |
|---|---|---|---|
| The solve | Prepared, once for each flow sample | Wet cells times steps. A 350 m stream 4 m wide at 0.25 m cells is 22,400 wet cells and about 24,000 steps: 540 million cell updates a sample (part 5) | Tens of seconds a sample on one core; M1 records it. Each cell's own time step saves a third; a filled start and a coarser grid first save little in a section |
| A river file | Loaded at preparation | For each reach: its rectangle's cells × samples × 6 values at 16 bits. A 300 m by 30 m reach at 0.25 m cells with 6 samples is about 10 MiB | Cutting a winding river into more reaches is the remedy where a rectangle is mostly ground |
| Samples in the map | Every read of a chart with more than one sample | Two layers in place of one | Two reads on river pixels; none elsewhere |
| Flow bands | Frame, while a river is inside the field's reach | Two transforms of 256² and their mips | About half the cascades' cost again; two more layer reads in the surface where a gain is not zero |
| Births from loss | Tick | One read of the chart's loss in the foam's births | Inside the foam's pass |
| Sheets | Frame, while a fall is in view | A lip's cells × up to 32 points; the sheet's pixels, blended | Small beside the surface; M6 records it |
| Plunges | Tick | A record and a birth for each segment | Negligible |
| The weighted mean | Tick, for each region | One reduction of a 256² level | Under the interaction step's own cost |

Records and tripwires are [`WATERS`'](../WATERS/DESIGN.md#cost-account): `scripts/water_cost.py`, with scenes this
module adds, and `scripts/river_cost.py` for the solve. No timing figure decides an item
([brief](../BRIEF.md#goal-and-exit-rule), decision 3).

## Risks

1. **The solve is slow, or will not settle**: a pool that sloshes, a wet and dry edge that flickers. Early sign:
   M1's report. The designed remedies are each cell's own step, a start from filled pools, a stopping rule on
   discharge between sections, and a coarser grid first. The wrong fix is stopping on a step count.
2. **Flow bands read as a small sea, or pulse.** Early sign: M5's first captures. The designed remedies are the
   bands' sizes and spectrum, a period that varies from place to place (`WATERS`' risk 13) and `c`. The wrong
   fixes are a painted amplitude and a scrolled normal map.
3. **Too much foam, or none.** One constant serves a beach and a ledge. Early sign: M5's record of cover at the
   test valley's stations. The designed remedy is a constant of the water type for flow, recorded with its
   reason. The wrong fix is foam where the surface is steep.
4. **The sheet reads as a card**: flat, evenly lit, hard at its foot. Early sign: M6's captures. The designed
   remedies are the pattern's relief as displacement, the white share and presence, and the plunge's foam and
   air at the foot. Three failed looks on one cue go to the log. Mist is the remedy this plan does not have.
5. **A seam where a river meets a lake, a fall's foot or the sea.** Early sign: M4's joins. The map's joins are
   `WATERS`'; the solve gives a level at each end that already matches the body there.
6. **`WATERS` moves under this module's changes**, since both builders edit its crates. Early sign: a rebase
   that fails `WATERS`' tests. Each ruling keeps its change behind a declaration that existing scenes do not
   make, and each milestone here runs `WATERS`' tests.
7. **A reach's rectangle is mostly ground.** Early sign: M4's bytes. Remedy: more reaches.
8. **Samples in the map cost the surface too much.** Early sign: M4's record. Remedy: mix the two samples into one
   set of layers when the gauge moves, a few times a second, and read one.
9. **A steady flow looks frozen**: nothing surges. Early sign: M5's sequences. The flow bands, the carried
   cascades and the foam are what moves; a slow wander of the gauge is the game's. The wrong fix is noise in the
   level.

## Proof and builder handoff

[`WATERS`' handoff](../WATERS/DESIGN.md#proof-and-builder-handoff) holds here word for word: targets, capture,
cost records, looks, the `gpu` lock, exact baselines, tests that hold no game content, Broadsides and
dependencies. In addition:

- **Scratch and evidence.** `tmp/rivers-<milestone>/` and `docs/evidence/RIVERS/`. Copies are listed in
  `docs/evidence/RIVERS/PROVENANCE.md`; none is expected.
- **`WATERS`' crates.** A change inside a file `WATERS` wrote is one a ruling names, behind a declaration no
  existing scene makes. Anything else it needs is a proposal in this module's log.
- **Bounds.** Parts 1 to 7 are the source of the bounds in M1 to M3. A bound on the GPU is measured first, on
  this Mac, and then asserted, with its date and commit in the record.
- **Maturity.** A ledger row for `E1.rivers` is added in M4 and kept current.

## Open questions

| Question | Settled by |
|---|---|
| What does a solve cost in Rust, for the test valley and the island? | M1 and M4 record it |
| Does `c = 2` read right, and do two bands suffice? | M5; [risk 2](#risks) |
| Does the shallows' birth constant serve a river? | M5 records the cover; [risk 3](#risks) |
| What wave share makes rings that read and stays under the fall's power? | M6 measures first, then asserts |
| How many reaches does a winding river need? | M4's bytes; [risk 7](#risks) |
| Does a fall need mist to meet its bar? | M6's review; the owner ([plan](PLAN.md#open-for-the-human)) |
