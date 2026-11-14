# Puddles: rain, puddles and wet ground in the one water

14 November 2026.
[Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) · [module list](../ARCHITECTURE.md) ·
[the water system, `WATERS`](../WATERS/DESIGN.md) · [rivers, `RIVERS`](../RIVERS/DESIGN.md). C02, C04, C07, C08,
C09 and C12 in [CONTRACTS](../../design/CONTRACTS.md) govern, as they do for all water; C09 gives rain its units
and its one wetness authority. [The module list's brief](../ARCHITECTURE.md#rivers-and-puddles) and
[what `WATERS` leaves to this module](../WATERS/DESIGN.md#what-the-later-modules-build-on) are where this design
started. Where it differs from either, [the rulings](../ARCHITECTURE.md#seam-rulings-for-puddles) hold.

## The request

The owner, 14 November 2026:

> Please make sure the sea work is actually thinking about water as a whole. So puddles, rivers, water falls,
> streams, lakes, and more. We want something that's built cohesively.

The lead's task for this plan: "puddles and wet ground as part of the one water system, at the quality of shipped
high-end games". The module list's brief says what finished means: rain that wets the ground it can reach and
leaves shelter dry; puddles that form in the ground's hollows as it rains, join when they overflow, ripple under
drops and feet, reflect the sky and what stands in them, and dry out; sand that darkens where waves have run;
and the same on the island and in the V1 valley behind a selection, in Chrome too.

## Purpose and scope

| User | Needs | What that requires of the design |
|---|---|---|
| A teaching demo, often in Chrome: a shower over a yard, puddles that grow and join, rings from the rain, the ground drying afterward | Rain water that is right as well as good-looking, with only the parts the demo shows | Mechanisms with an oracle; every part optional, at no cost when left out |
| A game | Wet ground, puddles and their depth the same on every machine and whatever the camera does | Water held as game state on the CPU, stepped with the tick; one query for all water |
| `WATERS` | Rain water that needs no second surface, optics, foam or query | A puddle is data in the map; rings are wave layers and interaction waves; a film is the film it already draws |
| The forward pass | Wet surfaces without a second lighting model | One function a surface reads before it is lit, composed as the cloud factor is |

**Commissioned:** the rain a game sets and what shelters from it; puddles in the ground's hollows; rings from
the rain on every body of water; rings from a footfall in a puddle; wet ground, its film and its drying; wet
sand on a shore. Cues P1 to P9 below, on `mac-metal` and `chrome-webgpu`, on the water example's island and in the
V1 valley, each behind a selection.

**Not built**, with the reason:

- **Rain seen falling: streaks in the air, splashes and their crowns, drops on the lens, and the sound of rain.**
  Water in the air is a medium or particles; the engine's owner of the first is the weather (`CLOUDS`, which
  does not plan rain) and it has no owner of the second, nor audio. This is the largest thing a shipped rainy
  scene has that this plan does not ([plan](PLAN.md#open-for-the-human)). The rain record and the exposure
  field hold what a later rain in the air would read.
- **Rain that wind slants, and rain that varies across a scene.** Rain falls straight and at one rate over a
  scene, the rain profile C09 selects.
- **Runoff seen running:** sheets and rivulets down a slope, drips from eaves, streaks down walls. Runoff
  reaches its hollow at once.
- **Rain that feeds lakes and rivers, and pools the tide leaves.** Water that spills over a hollows body's edge
  is a named sink. A hollows body may not touch another body of water.
- **Tracks:** footprints and tyre marks in wet ground or mud, and a dry patch under a thing that has moved.
  Wetness holds no state at a point.
- **Sun and shade in drying.** Drying is one rate a game sets; shaded ground dries as fast as sunlit.
- **Snow and ice**, as in `WATERS`.

**What exists.** `WATERS` M1 builds the map and M8 its optics; M6 raises rings on water 0.25 m deep and M7
draws the film on sand the waves wash. Thrum has no rain, no wetness and no puddles. The forward shader composes
a cloud factor into its direct Sun and has no notion of a wet surface.

## Cues

The bar for puddles and wet ground is the streets of The Last of Us Part II's Seattle and the trails of Red Dead
Redemption 2 after rain; for rain on open water it is Red Dead Redemption 2's lakes. All in words, at the
display's full refresh.

| Cue | Range it must hold over | Plausible wrong substitute, and what rejects it |
|---|---|---|
| **P1** Puddles lie where the ground holds water: as it rains, water gathers in the ground's own hollows, each puddle level, with a clean waterline that creeps up the ground | Rain of 1 to 100 mm/h; hollows from four cells to 10 m across and 5 mm to 0.3 m deep, on cells of 2.5 to 10 cm; paving, earth and sand; from 1.7 m and from 15 m up | A mask from noise or a height map under one flood level. Rejected by two hollows that must stand at different levels, by a hollow below a slope that must fill before one on a rise, and by a float that must sit at the puddle's level |
| **P2** Puddles spill, join and part: a full hollow overflows into the next, two become one level puddle, and drying parts them again | Every pair of hollows a bed holds; filling and drying; rings on the water at the time | A body for each puddle, rebuilt when two meet. Rejected by both levels across the tick of a merge, by rings that must survive it, and by a query that must answer through it |
| **P3** Rain rings: each drop raises rings that spread, leaving calm water inside them, on every water the rain reaches, more and rougher as the rain grows | Rain of 1 to 100 mm/h; water from 1 mm deep to the sea; from 0.5 m and from 30 m; a roof over part of the water | A texture of circles that grow at one speed with one width. Rejected by the calm disc's growth against the least speed water's waves have, by crests that multiply with age, by rings that weaken on a film, and by none under the roof |
| **P4** A footfall or a dropped stone rings a puddle: rings cross it at the speed its depth gives and die at its edge | Puddles 10 mm to 0.2 m deep; two puddles a sill apart; before and after they join | A ripple decal. Rejected by the rings' speed at two depths, by a dry sill that passes nothing, and by rings that cross once the sill is under water |
| **P5** Puddles mirror: the sky, and what stands in and round them, joined to its foot at the waterline; the ground shows through from above and the mirror takes over toward a grazing view; muddy water hides its bed | Camera 0.3 m to 15 m up; a post in the water, a wall and a roof beside it; clear and muddy water; dawn to night; in rain and after | A sky-only reflection. Rejected by a post whose top is off the screen, and by a bed that must fade with depth by the water's type |
| **P6** Rain wets the ground: it darkens and its colour deepens, a sheen gathers on paving and in its joints and mirrors at a grazing view, and shelter stays dry | Paving, earth, sand, rock and metal; exposure from none to full; level ground to walls; props and cooked meshes | One darker, glossier look for everything. Rejected by ground under a roof, by metal that must not darken, by pale stone that must darken less than dark, and by sand that takes no sheen |
| **P7** It dries in order: the sheen goes first, from slopes before level ground and from high relief before low; darkness lingers and lifts last; a shrinking puddle leaves a dark margin | Drying rates of 0.1 to 600 mm/h; each ground | One fade of everything at once. Rejected by the order of the three |
| **P8** A wet shore: sand is dark where the waves have run, under the sheen and foam `WATERS` draws there | Calm to gale; the tide through its range | Darkness painted along the still waterline. Rejected by its edge in a calm against a gale, and by a tide that ebbs and leaves a dark band |
| **P9** One water: a puddle has the lake's light, optics, query, mirror and controls, and the rain rings the lake and the sea as it rings a puddle | Every body in one view | A puddle drawn by the ground's material. Rejected by the query, and by one rain on three waters |

## Research and selection

What each source was read for, and how far.

- **Lagarde, Remember Me** ([Water drop 2b](https://seblagarde.wordpress.com/2013/01/03/water-drop-2b-dynamic-rain-and-its-effects/),
  [3a](https://seblagarde.wordpress.com/2013/03/19/water-drop-3a-physically-based-wet-surfaces/) and
  [3b](https://seblagarde.wordpress.com/2013/04/14/water-drop-3b-physically-based-wet-surfaces/), as summarised by
  fetch). The shipped pattern most games still follow: one wet level and two flood levels for a whole scene,
  puddles from vertex colour and holes from a height map, a tiled texture of ring normals in four layers that
  switch on with intensity, a depth map from the rain's direction for shelter, and a wet look of
  `diffuse *= lerp(1, 0.2, porosity)` with gloss pulled toward water's. Its author's own limits: puddles share
  one depth, "it lacks true volume, local minima, and slope", rings are perfect circles whose mips "simply
  disappear", and porosity is a strength an artist tunes. Taken: two stores, film and pores; darkening by
  porosity; porosity read from roughness and metalness where a material gives none; the order of drying. Not
  taken: one flood level, a painted mask, and a texture of rings.
- **Far Cry 6** ([Weick and Zhou, GDC 2022](https://www.gdcvault.com/play/1027675/Simulating-Tropical-Weather-in-Far),
  session description only). Wetness on static and dynamic things, puddles on terrain, and drying as the sky
  clears, as one weather-driven system. Evidence that the bar includes drying. Its methods were not read.
- **Barnes, Callaghan and Wickert 2021**, [Fill–Spill–Merge](https://esurf.copernicus.org/articles/9/105/2021/)
  (abstract and method, as summarised by fetch). A landscape's depressions as a forest of binary trees: leaves
  are the smallest hollows, a parent is two that have merged, and each node links to the leaf its overflow
  enters. Water fills a leaf, spills to its sibling, and the two merge when both are full; the level follows
  from the volume and the cells' heights. Taken whole as the puddles' mechanism. Its code was not read and none
  is copied.
- **Le Méhauté 1988**, [Gravity–capillary rings generated by water drops](https://www.cambridge.org/core/journals/journal-of-fluid-mechanics/article/abs/gravitycapillary-rings-generated-by-water-drops/329DFD5008A3D8E83EB4D01FAC19ED51)
  (abstract). A drop's rings are waves near the wavenumber of least group speed, the wave train's shape does
  not depend on the drop and its height is in proportion to the drop's momentum. A search also found
  experiments that put about 1% of a drop's energy into its rings. [Part 3](../../evidence/PUDDLES/design-proof/README.md)
  works the same response out from the water's own dispersion.
- **Lekner and Dorf 1988**, "Why some things are darker when wet" (from memory, with Lagarde's summary): light
  the wet ground scatters is partly returned to it at the film's surface, so more is absorbed. Part 4 derives
  the form again.
- **The contracts.** [C09](../../design/CONTRACTS.md) already says rain is a mass flux partitioned once among
  first-hit receivers and named sinks, that one material owner may split its liquid into film and absorbed
  stores, and that appearance reads committed state. This design is built to it.
- **The older Thrum design.** [The appearance design](../../design/modules/WATER-APPEARANCE.md), which the
  brief names, is about whitewater (its first three sections were read). Of rain it says only that a deposit
  enters storage even when it is too small to make whitewater, which holds here: rain adds water and no foam.
  Its ledgers and receipts are not selected, for
  [the reasons `WATERS` gives](../WATERS/DESIGN.md#judged-from-first-principles).
- **Not found or not read:** any talk on the rain, puddles or wet ground of The Last of Us Part II, Red Dead
  Redemption 2, Forza Horizon, Marvel's Spider-Man, Alan Wake 2 or Cyberpunk 2077; two searches, one of them
  extended, returned tutorials and Lagarde. Their bar is taken from the games. Marshall and Palmer's drop sizes
  and Atlas's fall speed are from memory.

### Judged from first principles

| Inherited choice | Verdict | Why |
|---|---|---|
| A puddle is a basin: a seed and a level, in a hollow found from the bed (`WATERS`) | **Change** | Basins may not overlap, and puddles must. A bed 3.2 m square holds 20 hollows whose fills overlap in 10 pairs (part 1). All the hollows of a stretch of ground are one body, whose level at a point is the level of that point's hollow |
| Rain rings are a short wave layer (`WATERS`, the brief) | **Keep**, and for every body | A square metre takes thousands of drops a second, so rings cannot be records. Rain falls on the lake and the sea as it falls on a puddle |
| A drop is an impulse record (`WATERS`, the interface table) | **Change** | For the same reason. A footfall and a dropped stone are records, and the game's |
| Rings on shallow water are the interaction waves `WATERS` proves at 0.25 m (`WATERS`) | **Change** | Its finest cell is 0.25 m and its shallowest claimed depth 0.25 m; a puddle is 1 m across and 30 mm deep. A third, finer level carries the same operator |
| Wet ground is a wetness the ground's material reads (`WATERS`, the brief) | **Keep**, as a function | A function of prepared fields and a few stepped stores is the same on the CPU and the GPU and anywhere in a world, where a field of state would need windows, a reset and a camera |
| Micro-puddles live in a material's own height (the brief) | **Change** | Water in a material's relief is a mirror, and only the water's pass can mirror scenery. The map's ground gains a relief, and water standing in it is drawn as the film `WATERS` already draws on a shore |
| One global wet level and flood level (Lagarde, shipped practice) | **Not selected** | It cannot hold two puddles at two levels, a catchment, a spill or a query |
| Shelter from a depth map drawn from above each frame (Lagarde, common practice) | **Not selected** | Shelter decides how fast a puddle fills, which is game state and may not depend on a camera or a device. Shelter is prepared content |

### The candidates, decided against the cues

| Cue | Candidate | Verdict |
|---|---|---|
| P1, P2 | A mask under one flood level | Fails two levels, a catchment and a spill |
| P1, P2 | A basin for each hollow, merged by replacing two bodies with one | Fails P2: a body's identity changes, the water is rebuilt and its rings reset |
| P1, P2 | A live shallow-water solver over the ground | Right, and the same every-tick cost and device dependence `RIVERS` judged. Not selected |
| P1, P2 | **A prepared tree of hollows with water volumes stepped on the CPU, read by a point test in the map** | **Selected** |
| P3 | A tiled texture of ring normals | Fails the calm disc, the crests and the film |
| P3 | Impulses into a simulated level | Fails the count, and a level that steps on the tick cannot show waves of 73 rad/s between ticks |
| P3 | **Two rain bands: a function of time that sums each scheduled drop's own wave train** | **Selected** |
| P4 | `WATERS`' near level as built | Fails: one cell is a quarter of the puddle |
| P4 | **A fine interaction level with a ladder from 5 mm** | **Selected** |
| P6, P7 | A wetness texture stepped on the GPU in a window | Not selected: it resets with the device, ends at its window and differs between machines |
| P6, P7 | **Two stores for each ground class, read at a point through exposure, slope and relief** | **Selected** |

## Mechanisms

### What rain adds to the one water

| Part of [the system](../WATERS/DESIGN.md#one-water-in-four-parts) | What rain adds | Its state |
|---|---|---|
| The map | A fourth kind of body, hollows; a relief on its ground | Content, with numbers a game moves: a level for each hollow |
| Waves | Two rain bands on every body; a fine interaction level | The bands are a function of time; the level is state, as the wake is |
| Material | Nothing. Rain makes no foam here | |
| Optics | The film over standing rain water, which is `WATERS`' film | None |
| Beside the water | The ground's wetness, which every forward surface reads | Game state on the CPU, stepped with the tick |

A **hollow** is a pit in the ground and the cells that drain to it. A **puddle** is the water standing at one
level in one hollow or in several that have joined. **Exposure** is the share of the rain that reaches a point,
from none under a roof to all under open sky. A **ground class** is a row of a game's table: what a kind of
ground absorbs, holds and looks like wet.

### Rain, and what it reaches

**The rain record.** A game puts one record in its tick page: the rain's flux, in kilograms a square metre a
second (a millimetre an hour is 1/3600 of one), and the drying rate in the same units. Both are uniform over
the scene and held over the tick (C09's interval). Until `CLOUDS` has rain they are the game's controls. A game
that wants nothing to dry while it rains sets the drying rate to nothing then; the example does.

**Exposure.** Shelter is content. A game gives the triangles that shelter, each with the share of the rain it
lets through: none for a roof, a part for a tree's crown. Preparation rasterises them from above into a
**canopy grid**, at the profile's cell (0.1 m): for each cell, the height of the lowest underside and the share let through. Exposure at a
world point is 1 above that height or where no canopy stands, and the cell's share below it, read with a soft
edge of the profile's width (0.3 m), which stands for splash and drift. So the ground under a roof is dry, the
roof's own top is wet, and the function is the same on the CPU and the GPU and reads no camera. One canopy is
kept for a column. Things that move shelter nothing.

**One partition.** Each tick's rain on a square metre goes to exactly one place (C09): to the canopy's named
sink, by the share exposure withholds; to the ground's stores; to the hollow the ground drains to; or over the
edge of a hollows body, another named sink. The account of a hollows body closes to rounding
([below](#filling-spilling-merging-and-parting)).

### Hollows: the tree and the point test

**A hollows body** is a rectangle of ground with a bed of its own, at a cell of 2.5 to 10 cm, built from the
ground's triangles as the map's bed is, with a ground class and a water type. Preparation, in
`thrum-rain-model`, floods the bed once from its pits, lowest ground first, as Barnes's depression hierarchy
does, and writes:

- a **label** for each cell: the hollow whose pit the flood reached it from. Every cell is therefore reached
  from its hollow's pit by a path that never descends;
- the **tree**: each pair of neighbours with their **sill**, the lowest pass between them; the node the two
  become above it; and for each node the hollow its overflow enters. A root's overflow leaves the rectangle;
- for each node, its level for a volume of water, and its water surface for a level.

A hollow shallower than the profile's least depth (3 mm) is filled: its cells' bed is raised to its sill and
they join its neighbour, since water that thin is the film's. A hollow under four cells across is reported.
A body may hold at most the profile's count of hollows (4,096) and is refused by name beyond it.

**The point test.** At any moment each hollow has one level: its puddle's. A point's water is its cell's
hollow's level where that stands above the bed, and none otherwise. No flood runs while the game does. This is
exact: because each cell is reached from its pit without descending, and a puddle that spans several hollows
stands above every sill between them, a cell below its hollow's level is always joined to that puddle's water.
In part 1 the test and a flood from the pits name the same wet cells at every merge, every parting and every
fortieth step.

**In the map** ([ruling 1](../ARCHITECTURE.md#seam-rulings-for-puddles)). The hollows body is a fourth kind
beside the sea, the basin and the chart. The map holds its bed, its labels and a level for each hollow, which
the game sets with the tick as it sets a stage, and which is read at the display's time between the two ticks'
tables. So in the one tick of a merge the two waters can differ by less than a tick's rise, a declared
approximation. Inside the rectangle the map's bed is the body's own. A point's row has that level, no current, no
wind gain (a puddle has no fetch), the rain gain every body has, and the body's water type. The body's
identity and generation never change as its puddles join and part. Its shore margin is the cells that touch
its water.

**What preparation and the tick refuse**, by name: a hollows body with a cell in another body's water or shore
margin anywhere in that body's stage range; one over the profile's count of cells or hollows; and, for a tick,
a table that stands two different levels in wet cells that touch, which is held at the last table and counted.
The rain model never writes one.

**Drawn as any body is.** The water's tiles cover the cells that hold water and the cells that touch them. A
vertex takes its cell's hollow's level, and on a dry cell the highest level among the wet cells that touch it,
so no triangle joins two levels over water, and the ground's own depth cuts every waterline. Within a cell of
a sill that parts two puddles the higher one's surface is drawn, a declared approximation of the size of a
cell, at the place the water is in fact about to spill.

### Filling, spilling, merging and parting

The **rain state** is plain data in the game's world, stepped once a tick by `thrum-rain-model` in f64 in a
fixed order: a volume for each puddle, the ground's stores, and a margin for each hollow. For a puddle, over a
tick:

- **In.** The runoff of every cell that drains to its hollows: the rain exposure lets through, less what the
  ground's stores took up that tick ([below](#wet-ground)). Runoff arrives at once.
- **Out.** The drying rate and the ground class's soak rate, each times the puddle's water surface.
- **Spill.** A puddle past its sill sends the excess to the hollow across it. If that neighbour is itself full
  to the same sill, the two are one puddle, holding both volumes: a **merge**. A root's excess leaves.
- **Part.** A puddle whose volume falls under what its two halves hold at the sill between them becomes two,
  each keeping its side's water.

A merge happens with both levels at the sill, and a parting leaves both there, so every hollow's level is
continuous through each (part 1: the largest change in a step with either falls with the step). Nothing is
rebuilt: the labels and the tree are content, the levels are numbers, and
[the interaction waves' regions](../WATERS/DESIGN.md#interaction-waves) are labelled again from the map, which
leaves their height and potential where they are; that is CPU work whenever a puddle's wet cells change,
which in rain is often, and M4 records it. So rings on two puddles are still there on the one they
become, and can then cross the drowned sill.

The account: rain on the body equals the change in the ground's stores and in the puddles, plus what dried,
soaked, was withheld by canopy and left over the edge. Part 1 closes it to 1.5e-12 l in 199 l.

Part 2 gives the times this makes: a pothole of 4 l with six times its area draining to it fills in 8 minutes
at 10 mm/h on paving; on sand that soaks 60 mm/h no puddle forms. Real evaporation empties 20 mm in days, so
the drying rate is a control a demo turns up.

Declared approximations: runoff has no travel time and makes no visible flow; a cell is all one hollow's; and
the ground's stores are kept for a class and not for a cell, so ground a puddle covers is counted as open
ground.

### Wet ground

**Two stores** (C09's film and absorbed liquid, one authority). A game's table has up to eight ground classes;
a hollows body names one, and a scene names one for every surface outside a hollows body. For each class the
rain state holds
surface water, in millimetres, and the pores' fill, from 0 to 1, for level ground under open sky, and the same
pair for ground that half and a quarter of the rain reaches. Each tick, for each: rain arrives; the pores take
it at the class's soak rate until they are full; what is left stands as surface water up to what the class's
relief holds, and the rest is runoff; then the drying rate takes surface water, and once there is none the
pores dry in proportion to their fill. So paving shows a sheen two minutes into a shower and earth that drinks
faster than the rain never does, and after the rain the sheen is gone long before the darkness (part 5).

**At a point**, the same function in f64 and in WGSL, of prepared fields and the state, reading no camera:

- the stores for the point's **exposure**, read between the open, half and quarter stores and nothing. Part 5:
  read so, ground that 0.4 of the rain reaches is off by 0.23 of the pores at worst, for a few minutes, where
  scaling the open store is off by 0.60;
- the surface's **share of the sky**: all for ground, a quarter for a wall, none for an underside, from its
  normal, a declared closure;
- **slope**: sloping relief holds less, down to none at the class's steepest slope, so a slope sheds its film
  first;
- **relief**: a class's relief is a height, and where it has one a tiling pattern of heights. Surface water
  stands in the relief's lows first, so the film's cover at a point is whether the water's fill stands above
  the pattern there, and with no pattern it is the fill's share;
- a puddle's **margin**: each hollow keeps one more number, the height of the damp band above its waterline,
  which is the rate its level has lately fallen over the rate its ground dries. The pores' fill falls off
  above the waterline over that height, so a puddle that shrinks slowly leaves a thin dark rim and one that
  drains fast a wide one;
- a **shore**: inside a body's [wave support](../WATERS/DESIGN.md#the-water-map) the pores are full, and above
  it they fall off over the body's own margin, kept as a hollow's is from the fall of its stage. This is the
  wave support `WATERS`' film reads, so the dark sand and the sheen share one edge (C07).

It gives two numbers: the **pores' fill** and the **surface water** with its cover. Under a puddle the pores
are full. A margin and a shore's band are appearance: they hold no water in the account (C09's authored
approximation).

**The wet look** ([ruling 5](../ARCHITECTURE.md#seam-rulings-for-puddles)). A forward surface reads the pores'
fill before it is lit. Its base colour becomes, for each channel,

`a_wet = (1 − R)(1 − p) a / (1 − p a)`, mixed in by the pores' fill times the material's porosity,

with `R` = 0.02 and `p` the share of the ground's light the film returns to it: 0.47 for stone, asphalt and
concrete, from water's index, and up to 0.7 for sand and soil, a value of the ground class. So dark ground
darkens more than pale and every colour deepens (part 4: 0.55 at an albedo of 0.1, 0.83 at 0.8). Its roughness
falls toward the film's by the same weight, halved. Porosity is the ground class's, or a material's own where
it gives one, and otherwise read from roughness and metalness by Lagarde's rule, so metal does not darken.
Outside a hollows body, where no film is drawn, the surface also takes the surface water's share as lower
roughness. Nothing else in the shader changes: the light, the shadows, the air and the cloud factor are
applied as they were.

**The film** ([ruling 4](../ARCHITECTURE.md#seam-rulings-for-puddles)). Over a hollows body's ground,
[the film `WATERS` draws on a shore](../WATERS/DESIGN.md#the-surface-and-the-frame-contract) is drawn wherever
the function gives surface water and no puddle stands. Its weight is the cover. Its normal is the level's
where the water stands in relief and the ground's where the ground slopes, and the rain bands ring it. It
mirrors through the water's own reflection slot, so wet paving shows the wall beside it and not only the sky.
It is still a declared residual and not water: it writes no depth, the query answers dry under it, and the
ground shows through it as the forward pass drew it, darkened. The pattern a class names is the one its
ground's material uses for its own relief, which is the game's to keep true, as the bed is.

Declared approximations: wetness holds no state at a point, so nothing leaves a track and a thing that has
moved leaves no dry patch; ground under part shelter is read between three stores; drying knows no shade.

### Rain rings

**One drop.** A drop gives the water an impulse: its mass times its fall speed, spread over its crater, three
drop radii across. Water answers an impulse as [the interaction waves](../WATERS/DESIGN.md#interaction-waves)
say, `h' = (D/ω) sin(ωt) φ₀`, with surface tension in the frequency, `ω² = (g k + σk³/ρ) tanh(kd)`, and
viscosity's decay. Part 3 integrates that for a 3 mm drop: rings 0.30, 0.20 and 0.14 mm high after 0.2, 0.4
and 0.8 s; a calm disc inside them whose edge moves out toward water's least group speed, 17.8 cm/s; more
crests with age; and 1.1% of the drop's energy in the rings, where experiments put about 1%. Nothing in it is
tuned: the crater's width is the one closure, and one drop radius would put 29.5% there.

**Two rain bands** ([ruling 3](../ARCHITECTURE.md#seam-rulings-for-puddles)). Two more wave layers in the
world's one field, beside the wind's cascades and the flow's bands: periodic patches of 512², 1.28 m and near
2.1 m across, so that their pattern does not repeat in view. Each is a **function of time**. Drops fall on a
schedule drawn from a hash of place and time, by size class, and a texel's height and slopes are the sum of
the rings of the drops that landed within reach in the last second, each read from a prepared table of a
ring's height by radius and age. The bands are evaluated every presented frame at the display's time, hold no
state, and are the same in f64. A paused frame holds them and a capture shows its own tick's rings.

- **How many.** Marshall and Palmer's drops at 2, 10 and 50 mm/h number 14, 152 and 918 of 2 mm and over on a
  square metre each second. The largest are scheduled, up to the profile's count (128 a square metre a
  second, about 8 over a point); whether a scheduled drop falls is a soft threshold on the rate, so a
  changing rate adds and removes rings without a pop. The drops below the cut are not drawn: their summed
  squared slope goes into the bands' moments, as everything under a pixel's footprint does.
- **The gain** at a point is the exposure there times the **depth response**, the model's ratio of a ring's
  height on that depth to its height on deep water: 1 from 10 mm, 0.76 at 5 mm, 0.41 at 2 mm, 0.25 at 1 mm,
  none on dry ground. Every body's row has it, the sea's included.
- **A wave layer in every rule** `WATERS` has for one: the surface's pixels, the slope covariance that blurs a
  reflection and spreads the Sun's glitter, the depth limit, carriage by a current, the query and the film.

Declared approximations: one wave train serves every depth, scaled, though rings on water under 10 mm spread
up to a quarter faster; rings do not reflect at a puddle's edge; a ring is cut off after a second; and a
splash's crown and jet are not waves and are not drawn.

### Rings from a footfall

A profile may name a **fine interaction level** ([ruling 2](../ARCHITECTURE.md#seam-rulings-for-puddles)):
256² cells of 2 cm, a window 5.12 m across round the body the game names, with `WATERS`' operator, regions,
border and sources, and

- a depth ladder of 5, 10, 20, 40, 80 and 160 mm, and the deep operator from 320 mm, with surface tension
  in its frequency;
- the waves under 1 m long, which the near level then no longer holds;
- a step of one tick at a time and at most two in a frame. Ticks beyond are dropped and counted, as
  `WATERS` counts ticks past its bound. An 8 cm wave turns 27° a tick, and steps of eight ticks would take
  up to 42% of a wave's height a second (part 6).

Part 6: over waves of 8 cm to 1 m on 5 mm to 1 m of water the blended step is within 2.0% of the true
speed, never grows, and loses at most 0.2% a second at one tick and 1.9% at two. Drawn at 120 Hz from two
ticks' endpoints, an 8 cm wave is misstated by 2.7% of its height.

A footfall is an [impulse record](../WATERS/DESIGN.md#interaction-waves) the game posts: the water under the
foot times the speed the foot arrives at, 0.6 N s for a foot in 30 mm, which raises rings 3.4 mm high a
quarter of a metre off (part 6). Each puddle in the window is a region, so a dry sill passes nothing, exactly,
and the regions with the latest sources are the ones solved.

### What a puddle already has

Everything else is `WATERS`' and is used as built. A puddle is lit, seen through and mirrored by the one
optics: the ground shows through thin water and is lost under a muddy type; a puddle's level takes a mirror
view when its water is among the two largest in view, and the screen's reflection otherwise; rain's slope
variance blurs both. For a mirror view a hollows body counts as one level, the mean of its ground, and its
film's area counts with its puddles'; ground further than the plane tolerance from that level takes the
screen's reflection. The view from under water is `WATERS`' on any body and is not judged on water this
shallow. A float sits on it through the one query. From `WATERS:M16` the film's and the puddle's
Sun lobes take the scene's Sun visibility, by [`RIVERS`' ruling 7](../ARCHITECTURE.md#seam-rulings-for-rivers)
and ruling 7 here, so a puddle in a wall's shadow does not glint.

### Capabilities, recipes, limits and knobs

- **Capabilities**, added to [`WATERS`' list](../WATERS/DESIGN.md#capabilities-recipes-limits-and-knobs). A
  scene that leaves one out declares none of its passes or resources, and no shader is prepared with its
  reads.

  | Capability | What it adds | Needs |
  |---|---|---|
  | Hollows | Hollows bodies in the map, on the CPU and the GPU | A bed for each |
  | Rain bands | The two bands and their gain on every body | A rain record |
  | Fine rings | The fine interaction level | Interaction waves |
  | Wet ground | The wetness function on forward surfaces | A rain state |
  | Standing film | The film over a hollows body's ground | Hollows; wet ground |

- **The physical profile**, never a quality setting (C04): the tree, the fill step and the point test; the
  stores and the wetness function; the ring tables, the drop schedule and its count; the fine level and its
  ladder.
- **Limits are inputs.** The labels, the fine bed and the canopy grid are textures, the levels and the stores
  small tables, and the rain bands layers of the cascades' arrays, all counted against the capability record
  at preparation. The forward stage gains one texture, the canopy grid.
- **Knobs**, in `EFFECT-GRAPH`'s list with a declared invalidation: the rain bands' cadence. It changes no
  query answer.

### Check that it can work

- **P1, P2.** Part 1 runs the tree, the fill and the point test on a made bed through 10 merges and 10
  partings. The preparation in Rust at 640,000 cells, the map on the GPU and the look of a waterline at 5 cm
  cells are unrun; M1 and M3 are the first.
- **P3.** Part 3 is the ring's own integral. That a sum of scheduled rings reads as rain and not as a pattern,
  at 128 a square metre a second, is unrun; M4 is the first look ([risk 2](#risks)). Part 3's mean square
  slope at 50 mm/h, 0.065, is a superposition and likely too high; M4 judges it.
- **P4.** Part 6 is the operator on uniform depth, wave by wave. Fields, regions and a puddle's edge are
  `WATERS`' as measured in its part H and J at larger scale, and unrun at this one.
- **P5.** `WATERS`' optics and mirror. Unrun on water 5 mm deep.
- **P6, P7.** Parts 4 and 5 are arithmetic. The look of the wet ground and of the film is unrun; M5 is the
  first.
- **P8.** The wave support is `WATERS`'. Unrun.
- **Cost.** Unrun. Each milestone records its own.

## The system as it runs

| State | Owner |
|---|---|
| The tree of hollows and its preparation, the fill step and its account, the canopy grid and exposure, the ground's stores and the wetness function, the wet look's reference, the ring response and its tables, the rain state | `thrum-rain-model` (this module), on `thrum-water-model`, no GPU dependency |
| The hollows kind in the map's function; the rain bands in the surface model's sum; the fine level in the interaction reference | Files this module adds to `thrum-water-model`, and its changes there under [rulings 1 to 3](../ARCHITECTURE.md#seam-rulings-for-puddles) |
| Hollows in the map on the GPU and their tiles; the rain bands' passes; the fine level; the film over standing water; the wetness provider and its tables | Files this module adds to `thrum-water` and `thrum-sea`, and its changes there under rulings 1 to 4 |
| The wetness read in the forward surface | The scene's shader and the cooked material, under ruling 5 |
| The map, the surface, optics, the query, interaction waves, the film, mirror views | `WATERS`, `SCENE-VIEWS` |
| The rain record, the ground classes and their patterns, the yard and its roof, the rain selection and its controls, footfalls | The games: `examples/water` and `examples/v1-game` |
| Sky, Sun, air, the cloud factor, the lent Sun visibility | `ATMOSPHERE` |

`thrum-water` takes the rain as data in `upload` and does not link `thrum-rain-model`; the game does.

**A frame, in order**, for what rain adds to [`WATERS`' frame](../WATERS/DESIGN.md#the-system-as-it-runs). On
the tick, in the game: the rain state steps. `upload` takes the rain record, each hollow's level and margin
and the stores, with the page's stages. `simulate`, every frame: the rain bands at `t`, with the cascades.
`simulate`, on a frame that advances the tick: the fine level's step, region by region. Forward: surfaces read
the wetness function. `after-opaque`: the water's gather, then its surface, puddles and film with the rest.

**Creator journey.** A game names the capabilities in its water profile, declares a hollows body over a
rectangle of its ground with a ground class, gives the triangles that shelter, keeps a rain state in its
world and steps it, and sets the rain and the drying rate with the tick. Moving the ground or a roof prepares
the tree or the canopy grid again and rebuilds the water, as changing any body does. Changing the rain, the
drying rate or a ground class's numbers rebuilds nothing.

**The example scenes.** `examples/water` gains a **rain selection**, off by default
([ruling 6](../ARCHITECTURE.md#seam-rulings-for-puddles)). With it on, the island has a yard by the lake:
paving with dips and joints, a track of packed earth with ruts, a roof on posts, a wall, and a post standing
in a hollow; a marker that walks and posts footfalls; and controls for the rain and the drying rate. With it
off the island is what `WATERS` built. `examples/v1-game` gains the same selection, off by default: with it on,
the valley's floor and props wet and dry, and a patch of ground the selection adds holds puddles. With it off
the exit scene is unchanged, which the exact baselines hold.

## What PUDDLES asks of other modules

Settled in [the module list](../ARCHITECTURE.md#seam-rulings-for-puddles), each built by this module's builder,
with no item, test, bound or reference of another module changed and all kept passing. A scene with none of
these capabilities compiles to the shader text, passes and storage it has today, and `thrum-water-model` gives
it the same answers to the bit.

1. A hollows body in the map (M1 on the CPU, M3 on the GPU).
2. A fine interaction level (M2 on the CPU, M4 on the GPU).
3. Rain bands: wave layers the wind does not fill, at a gain every body's row has (M2 on the CPU, M4 on the
   GPU).
4. The film is drawn over standing rain water (M5).
5. A wetness provider on the forward surface, in the scene's shader and the cooked material (M5).
6. A rain selection in `examples/water` (M3) and in `examples/v1-game` (M6).
7. The film's Sun lobe takes the lent Sun visibility, with the surface's (M7).

Three of these replace what [`WATERS`' design](../WATERS/DESIGN.md#what-the-later-modules-build-on) expected: a
puddle is not a basin with a seed, a drop is not an impulse record, and the film is drawn off the shore. Used
as built: the bed's builder, the surface, the optics, the gather, mirror views, the query, impulse records,
the film, the lease of a pattern, the typed controls. Nothing is asked of `EFFECT-GRAPH`, `CONTENT-LOOP`,
`SCENE-VIEWS` or `CLOUDS`.

## Cost account

Estimates, unmeasured.

| Work | Rate | Size | Estimate |
|---|---|---|---|
| Preparing a hollows body | Prepared | One flood of its cells, a sort for each node. A yard 40 m by 40 m at 5 cm is 640,000 cells | Under a second on one core; M1 records it. A node's curve of level for volume is the builder's to keep small |
| The fill step | Tick, on the CPU | A few operations for each puddle and each node that spills | Microseconds for hundreds of puddles; M1 records it |
| Hollows in the map | Every read inside a hollows rectangle | A label, a level and the fine bed | Three reads on puddle pixels; none elsewhere. 640,000 cells are about 4 MiB |
| Puddle tiles | Frame | Cells that hold water and their neighbours, at the cell's spacing | A fifth of a yard under water is about 130,000 quads; M3 records it |
| Rain bands | Frame, while it rains on water in view | Two patches of 512²; about 8 rings a texel from a table | Under the cascades' cost; two more layer reads in the surface where the gain is not zero |
| The fine level | Tick, while a source is in the window | 256²; one solve for each region and ladder depth in use, up to the profile's count | A few tenths of a millisecond; M4 records it with the count of solves |
| The wetness function | Frame, every forward fragment in a scene with wet ground | One read of the canopy grid and arithmetic on a small table | One texture more in the forward stage, the second with `WATERS`' caustics; M5 records the pair |
| The film | Frame, over a hollows body's ground with surface water | The film's pixels: the function, a pattern read, the reflection | Of the order of the surface on those pixels; M5 records it |

Records and tripwires are [`WATERS`'](../WATERS/DESIGN.md#cost-account): `scripts/water_cost.py`, with scenes
this module adds, and `scripts/rain_cost.py` for preparation, the fill step and the valley. No timing figure
decides an item ([brief](../BRIEF.md#goal-and-exit-rule), decision 3).

## Risks

1. **A waterline shows its cells**, or a puddle's far edge hangs in the air at a sill. Early sign: M3's
   captures at 1.7 m. The designed remedies are a finer cell, a bed built from the highest ground in each
   cell as the map's is, and the surface carried one cell past the water. The wrong fix is a soft edge drawn
   in the ground's material.
2. **Rain bands read as a pattern**: a grid, a repeat, or rings too alike. Early sign: M4's first captures.
   The designed remedies are the two patches' sizes, more size classes and the schedule's count. The wrong
   fix is a texture of rings.
3. **Rain makes all water matt**, because the moments of the drops below the cut are too large. Early sign:
   M4's captures at 50 mm/h. The designed remedy is a lifetime on the moments' integral, as the rings have.
4. **The film fights the ground or floats over it**, `WATERS`' risk 16, now on paving. Early sign: M5's
   captures at a grazing view. Its depth bias and the bed are the remedies.
5. **The film's joints miss the ground's joints.** Early sign: M5's close views. The pattern is one texture
   read by both; the wrong fix is a second pattern tuned to look near enough.
6. **Wet ground reads as a tint**: flat, with no sheen. Early sign: M5's captures. The film, the lower
   roughness and `p` by class are the mechanism; three failed looks on one cue go to the log.
7. **The forward pass will not take another texture in Chrome**, with `WATERS`' caustics. Early sign: M5's
   preparation. The designed remedy is the canopy grid packed with the caustic level's array.
8. **Too many puddles for the fine level's solves.** Early sign: M4's record. Regions are admitted by their
   latest source, so a walked path is solved and the rest lie at rest.
9. **`WATERS` and `RIVERS` move under this module's changes**, since three builders edit the same crates.
   Each ruling keeps its change behind a declaration that existing scenes do not make, and each milestone
   runs their tests.
10. **A puddle without falling rain looks unexplained.** Early sign: M4's review. Rain in the air is the
    remedy this plan does not have ([plan](PLAN.md#open-for-the-human)).
11. **Exposure's edge is a ruled line.** Early sign: M5's captures under the roof. The soft edge's width is
    the remedy.

## Proof and builder handoff

[`WATERS`' handoff](../WATERS/DESIGN.md#proof-and-builder-handoff) holds here word for word: targets, capture,
cost records, looks, the `gpu` lock, exact baselines, tests that hold no game content, Broadsides and
dependencies. In addition:

- **Scratch and evidence.** `tmp/puddles-<milestone>/` and `docs/evidence/PUDDLES/`. Copies are listed in
  `docs/evidence/PUDDLES/PROVENANCE.md`; none is expected, and Barnes's code is not copied.
- **Other modules' files.** A change inside a file another module wrote is one a ruling names, behind a
  declaration no existing scene makes. Anything else it needs is a proposal in this module's log.
- **Bounds.** Parts 1 to 6 are the source of the bounds in M1 and M2. A bound on the GPU is measured first,
  on this Mac, and then asserted, with its date and commit in the record.
- **Time in the views.** An inspected item that shows filling or drying may turn the rain and the drying rate
  up so the change fits the sequence; it says the rates it used.
- **Maturity.** A ledger row for `E1.rain` is added in M3 and kept current.

## Open questions

| Question | Settled by |
|---|---|
| What does preparing a yard cost, and how small can a node's curve be kept? | M1 records it |
| Does a 5 cm cell give a clean waterline at 1.7 m? | M3; [risk 1](#risks) |
| Do 128 rings a square metre a second read as heavy rain, and what moments do the rest need? | M4; [risks 2 and 3](#risks) |
| What `p` reads right for each ground, and does the film carry wet paving? | M5; [risk 6](#risks) |
| How many sampled textures do the surface and the forward stage need with rain, in Chrome? | M3 and M5 count against the record |
| Does a rainy scene need rain in the air to meet its bar? | M4's and M7's reviews; the owner ([plan](PLAN.md#open-for-the-human)) |
