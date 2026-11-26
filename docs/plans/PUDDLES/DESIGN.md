# Puddles: rain, puddles and wet ground in the one water

14 November 2026; the rule for ground a puddle rises over and leaves, 16 November 2026; rain on held ground,
17 November 2026; the rain's share of a query, 26 November 2026.
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
| A teaching demo, often in Chrome: a shower over a yard, puddles that grow and join, rings from the rain, the ground drying afterward | Rain water that looks right, with only the parts the demo shows | Mechanisms with an oracle; every part optional, at no cost when left out |
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

**What exists.** `WATERS` WP1 builds the map and WP8 its optics; WP6 raises rings on water 0.25 m deep and WP7
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
| **P5** Puddles mirror: the sky, and what stands in and round them, joined to its foot at the waterline; the ground shows through from above and the mirror takes over toward a grazing view; muddy water hides its bed | Camera 0.3 m to 15 m up; a post in the water, a wall and a roof beside it; a pothole well below the paving round it; clear and muddy water; dawn to night; in rain and after | A sky-only reflection, or one mirror plane for a whole yard. Rejected by a post whose top is off the screen and whose foot stands in a pothole, and by a bed that must fade with depth by the water's type |
| **P6** Rain wets the ground: it darkens and its colour deepens, a sheen gathers on paving and in its joints and mirrors at a grazing view, and shelter stays dry | Paving, earth, sand, rock and metal; exposure from none to full; level ground to walls; props and cooked meshes | One darker, glossier look for everything. Rejected by ground under a roof, by metal that must not darken, by pale stone that must darken less than dark, by earth whose sheen must wait until its pores are full, and by sand, which passes the rain down and takes none |
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
  experiments that put about 1% of a drop's energy into its rings. [Part 3](../../../.orchestrate/evidence/PUDDLES/design-proof/README.md)
  works the same response out from the water's own dispersion.
- **Lekner and Dorf 1988**, "Why some things are darker when wet" (from memory, with Lagarde's summary): light
  the wet ground scatters is partly returned to it at the film's surface, so more is absorbed. Part 4 derives
  the form again.
- **Infiltration under ponded water** (three hydrology course texts on Horton's and Green and Ampt's models,
  as summarised by one search, 16 November 2026; no paper was read). Water ponds when rain passes the
  ground's infiltration capacity, and ponded water goes on infiltrating at that capacity, not at once. Taken:
  the pores under a puddle take its water at the class's soak rate. Not taken: a capacity that falls with
  time or with the depth infiltrated; a class has one soak rate.
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
| P1, P6 | A puddle fills the pores of ground it rises over at once, and owes what it cannot pay | Fails. The first runoff onto a floor with 3 mm of dry pores holds under a hundredth of their room, so no puddle forms on ground whose own soak rate, 1 mm/h, says one must (the WP2 handoff: the puddle never held more than 2.7 ml of a 208 ml shower). A debt is water the account does not hold |
| P1, P6 | Ground is not covered until the puddle can fill its pores | Fails the same soak rate, and keeps water for a hollow that stands nowhere |
| P1, P6 | **The pores under a puddle take its water at the class's soak rate; ground changes hands only in its class's state, and is held while the puddle cannot pay** | **Selected** |
| P1, P6 | Rain on held ground is its puddle's, because the hollow keeps that ground's account | Fails C09. Held ground stands above the water, so rain hits it first. On part 11d's dish at 1 mm/h the puddle was given 5.1e-8 m³ a tick where 2.8e-9 falls on water, and the ground's 5 mm/h percolation never met the rest (the design review of 17 November) |
| P1, P6 | Held ground keeps a film of its own, a third store for each hollow | Right, and not selected: a store with its own drying and handover for ground that is held in 90 of part 11's 3,528 cases |
| P1, P6 | **Rain on held ground meets the pores the hollow keeps for it, at the class's two rates, and the rest is runoff to the puddle** | **Selected** |
| P3 | A tiled texture of ring normals | Fails the calm disc, the crests and the film |
| P3 | Impulses into a simulated level | Fails the count, and a level that steps on the tick cannot show waves of 73 rad/s between ticks |
| P3 | **Two rain bands: a function of time and of the last second's rain, summing each scheduled drop's own wave train** | **Selected** |
| P9 | The rain's share of a query read from the bands' textures, as a cascade's is | Fails: the textures hold the display's time at the bands' cadence, and a batch is dispatched before the rate of the tick it answers for is committed |
| P9 | The schedule drawn again from its hash for every sample of every station's footprint, as first built | Right answers, and 30 to 57 ms for the example's 54 stations where a tick is 16.7 ms (part 12) |
| P9 | One list of the live drops a tick, with each footprint still integrated on the tick, two mips of sixteen points | The same answers to 1e-15, and 2.2 to 4.6 ms a tick for that batch, twice that where two departures blend. Not selected: an eighth to a quarter of a tick, and more on a current, for rings a hull cannot feel |
| P9 | **One list of the live drops a tick, shared by the batch, and a prepared footprint: the mean over a disc of the ring the bands draw, tabled from the ring table** | **Selected**: 0.4 to 0.8 ms |
| P4 | `WATERS`' near level as built | Fails: one cell is a quarter of the puddle |
| P4 | **A fine interaction level with a ladder from 5 mm** | **Selected** |
| P6, P7 | A wetness texture stepped on the GPU in a window | Not selected: it resets with the device, ends at its window and differs between machines |
| P6, P7 | **Two stores for each ground class, with a rate into the pores and a rate down out of them, read at a point through exposure, slope and relief** | **Selected** |

## Mechanisms

### What rain adds to the one water

| Part of [the system](../WATERS/DESIGN.md#one-water-in-four-parts) | What rain adds | Its state |
|---|---|---|
| The map | A fourth kind of body, hollows; a relief on its ground | Content, with numbers a game moves: a level for each hollow |
| Waves | Two rain bands on every body; a fine interaction level | The bands are a function of time and of the rain's rate over the last second, which the rain state keeps; the level is state, as the wake is |
| Material | Nothing. Rain makes no foam here | |
| Optics | The film over standing rain water, which is `WATERS`' film | None |
| Beside the water | The ground's wetness, which every forward surface reads | Game state on the CPU, stepped with the tick |

A **hollow** is a pit in the ground and the cells that drain to it. A **puddle** is the water standing at one
level in one hollow or in several that have joined. **Exposure** is the share of the rain that reaches a point,
from none under a roof to all under open sky. A **ground class** is a row of a game's table: what a kind of
ground absorbs, holds and looks like wet. **Open ground** is ground the class's stores describe. A hollow's
**mark** is a level at or above its puddle's: the hollow, and not the class's stores, keeps the account of
the ground below it. **Held ground** is ground under a mark and above the water, which the water has left
and the puddle cannot yet hand back. A mark says who keeps a stretch of ground's water. It never says what
the rain hits.

### Rain, and what it reaches

**The rain record.** A game puts one record in its tick page: the rain's flux, in kilograms a square metre a
second (a millimetre an hour is 1/3600 of one), and the drying rate in the same units. Both are uniform over
the scene and held over the tick (C09's interval). Until `CLOUDS` has rain they are the game's controls. A game
that wants nothing to dry while it rains sets the drying rate to nothing then; the example does. The rain
state keeps the flux of each tick of the last second, the **rate history**, for the rings
([below](#rain-rings)).

**Exposure.** Shelter is content. A game gives its **shelters**: each a set of triangles with one share of the
rain it lets through, none for a roof, a part for a tree's crown. Preparation rasterises them from above into a
**canopy grid**, at the profile's cell (0.1 m): for each cell, up to two **layers**, each a shelter's lowest
underside there and its share. A shelter counts once in a column however many of its triangles cross it.
Exposure at a world point is the product of the shares of the layers whose underside is above it, and 1 where
none is, read with a soft edge of the profile's width (0.3 m), which stands for splash and drift. So the
ground under a roof is dry, the roof's own top is wet, a crown under a roof is as dry as the ground below it
([part 10](../../../.orchestrate/evidence/PUDDLES/design-proof/README.md): 0.00 at the ground and between the two, where the
lowest canopy alone gave 0.50 and 1.00), and the function is the same on the CPU and the GPU and reads no
camera. A column that three or more shelters cross is refused at preparation, by name and place. Things that
move shelter nothing.

**One partition, by what the rain hits first.** Each tick's rain on a cell goes to exactly one receiver
(C09), found from the shelters, the bed and the water as it stands at the tick's start:

1. the canopy's named sink, by the share exposure withholds;
2. of the rest, where water stands on the cell, its hollow's level at the tick's start being above its
   bed, that hollow's puddle, by the exposure at the water's surface: it is the puddle's water at once and
   no ground sees it;
3. where the cell is held ground, the ground: the pores its hollow keeps for it take what they take, by
   the exposure at the ground, and what they do not is runoff to its hollow's puddle
   ([below](#the-mark-and-ground-that-changes-hands));
4. elsewhere, the cell's open ground: its class's stores take what they take
   ([below](#wet-ground)), and what they do not is runoff, to the hollow the cell drains to or over the
   body's edge, another named sink.

Water and ground are told apart by the level, never by a mark: a mark only says which of two accounts,
the hollow's or the class's, keeps a stretch of ground's water. Ground passes between those accounts when
a hollow's mark moves over it, by transfers the account holds. The receivers are proved apart from the
total: a total that closes does not show where a cell's rain went, so WP1 and WP2 hold each receiver's gain
alone as well as the account ([proof](#proof-and-builder-handoff)).

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
fixed order: a volume for each puddle, the ground's stores, and for each hollow a margin, a mark and the
water in the pores under the mark. The stores are read as they stand at the tick's start. For a puddle,
over a tick, in this order:

- **The marks.** Each of its hollows' marks moves to the puddle's level, if the ground between can change
  hands ([below](#the-mark-and-ground-that-changes-hands)).
- **In.** The rain on its water, where the level at the tick's start stands above the bed, by the exposure
  at the water's surface; the runoff of its hollows' held ground; and the runoff of the open ground that
  drains to its hollows. Each runoff is the rain exposure lets through at the ground, less what that
  ground took up that tick ([below](#wet-ground)). Runoff arrives at once.
- **Out.** Drying takes the drying rate over the puddle's water surface. The bed takes water as open
  ground's pores take rain: in each hollow, while the pores under the mark have room, the class's soak rate
  over the water's surface there, of which the pores keep what they have room for and pass the rest down,
  at no more than the percolation rate; and over full pores, and on ground with none, the percolation rate,
  also where that is above the soak rate. What goes down is a named sink. Both read the water's surface
  after the tick's rain and runoff. A puddle that holds less than the tick's drying and take gives each its
  share of what it holds and is empty.
- **Spill.** A puddle past its sill sends the excess to the hollow across it. If that neighbour is itself full
  to the same sill, the two are one puddle, holding both volumes: a **merge**. A root's excess leaves.
- **Part.** A puddle whose volume falls under what its two halves hold at the sill between them becomes two,
  each keeping its side's water.

A merge happens with both levels at the sill, and a parting leaves both there, so every hollow's level is
continuous through each (part 1: the largest change in a step with either falls with the step). Nothing is
rebuilt: the labels and the tree are content, the levels are numbers, and
[the interaction waves' regions](../WATERS/DESIGN.md#interaction-waves) are labelled again from the map, which
leaves their height and potential where they are; that is CPU work whenever a puddle's wet cells change,
which in rain is often, and WP4 records it, with WP8 for the fine level's. So rings on two puddles are still there on the one they
become, and can then cross the drowned sill.

The account: rain on the body equals the change in the stores over open ground, in the pores under the
marks and in the puddles, plus what dried, soaked or percolated, was withheld by canopy and left over the
edge. Rain, and water a game pours in, are its only sources, and the state keeps no quantity that stands
for water not yet delivered. Part 1 closes it to 1.5e-12 l in 199 l, for a fill with no stores; WP2.1 closes
it with the stores, the marks' transfers and the pores under the marks.

Part 2 gives the times this makes: a pothole of 4 l with six times its area draining to it fills in 8 minutes
at 10 mm/h on paving; on sand that passes 60 mm/h down no puddle forms. Real evaporation empties 20 mm in days, so
the drying rate is a control a demo turns up.

### The mark, and ground that changes hands

A class's stores are one state shared by all its open ground, so ground can join or leave them only in that
state, with exactly the water that state holds. A puddle does not always hold that water. So each hollow
keeps a mark. The ground below the mark is in the hollow's account: the class's stores do not count it,
and its pores' water is one volume the hollow keeps, the **pores under the mark**, spread evenly over that
ground. At the start of a tick each mark is compared with its puddle's level then:

- **The level is above the mark.** The ground between goes under the mark. Its surface water, read from its
  class's stores at each cell's exposure, joins the puddle, and its pore water joins the pores under the
  mark. Both are receipts, so neither can fall short. The mark becomes the level.
- **The level is below the mark.** The ground between is due back to the stores in its class's present
  state. Its pore water comes from its share, by area, of the pores under the mark: what the share holds
  beyond the class's fill is counted as soaked, a named sink, and what it lacks comes from the puddle. The
  class's surface water for that ground comes from the puddle too. If the puddle holds that much it pays,
  and the mark becomes the level. If it does not, nothing moves and the mark stays where it is: the ground
  is held.

Held ground is still in the hollow's account. It reads as held ([below](#wet-ground)), and all of it is
handed over on the first tick the puddle can pay: when rain has refilled the puddle, or when the class's
own surface water has dried, soaked in or gone down, so that less is due. Until then a level that moves
below the mark moves no ground. Hollows that share a puddle draw on it in the hollows' order. No transfer
is more than its donor holds when it is made, so nothing is made and nothing is owed.

**Rain on held ground** is the ground's, because the ground is what it hits. A held cell's rain, by the
exposure at the ground, meets its share, by area, of the pores under the mark by the step a class's store
takes ([below](#wet-ground)), with no relief: the pores take it at the class's soak rate while that share
has room, and from then only at the percolation rate, which goes down, a named sink. Held ground stands no
film, so what its pores do not take is runoff to its hollow's puddle, at once, and is the puddle's from
then. The share's room is read after the marks have moved, and held ground's rain is taken before the
bed's. [Part 11f](../../../.orchestrate/evidence/PUDDLES/design-proof/README.md) runs it on the design review's case, part
11d's dish holding 72 cells over full pores round 4 cells of water. When rain starts again at 1 mm/h,
under the class's 5 mm/h percolation rate, the puddle gains 2.8e-9 m³ a tick, the rain on its own water,
and all 4.8e-8 m³ that falls on the held ground goes down; the rule of 16 November gave the puddle all
5.1e-8. At 100 mm/h the held ground passes 2.5e-7 m³ down and 4.6e-6 runs to the puddle. On a floor held
with room in its pores they take all its rain. Each figure is summed there from the bed, the exposure and
the level, and then compared with the step's.

The pores under a puddle fill at the soak rate because that is what the class says of its ground: a puddle
that filled them at once would put 3 mm into ground that takes 1 mm/h.
[Part 11](../../../.orchestrate/evidence/PUDDLES/design-proof/README.md) runs the rule on one hollow. On the third
review's floor, 3 mm of dry pores that soak 1 mm/h under 60 mm/h for 50 s, with every cell draining to the
floor, the puddle holds 196 ml of the 208 ml that fell when the rain stops, where the first rule left it
2.7 ml; the pores under it take 11 ml; and it dries away with nothing held and the account closed to 6e-15
of the rain. Over 3,528 cases of floor, shelter, class, shower and drying, one class percolating faster
than it soaks, the account closes to 4e-13 and no puddle, pore store or ground goes below nothing. Ground
is held in 90 of them, of two kinds. A level floor half under a crown holds for up to five minutes in the
rain, and its pores take the rain that falls on it meanwhile. A dish as shallow as its class's 3 mm of
relief holds its floor for up to 28 minutes after the rain, until the film on the open ground has gone, since water that thin was the film's to begin with. A
level floor under open sky is held in none of its 1,512 cases: its puddle and the open film gain and lose
at the same rates, and the puddle started with the film.

Declared approximations: runoff has no travel time and makes no visible flow; a cell is all one hollow's;
the ground's stores are kept for a class and not for a cell, so ground a puddle has just left takes its
class's state at once, and only its margin shows that it was under water; the pores under a mark are one
fill for a hollow and not for a cell; held ground stands and shows no film though open ground beside it
may, for as long as it is held, which with no drying and no percolation is until it next rains harder
than its pores take, so its relief holds back none of its runoff; the pores under held ground do not dry
while it is held; and pore water beyond the class's fill is lost downward when ground is handed back. A store for each cell would need neither the
hold nor that loss, at the field of state [wet ground](#wet-ground) chose not to keep.

### Wet ground

**Two stores** (C09's film and absorbed liquid, one authority). A game's table has up to eight ground classes;
a hollows body names one, and a scene names one for every surface outside a hollows body. A class has two
rates: its **soak rate**, at which its pores take water until they are full, and its **percolation rate**, at
which full pores pass water on down, a named sink. For each class the rain state holds surface water, in
millimetres, and the pores' fill, from 0 to 1, for level open ground under open sky, and the same pair
for ground that half and a quarter of the rain reaches. Each tick, for each: rain arrives; the pores take it
at the soak rate until they are full, and from then only at the percolation rate; what is left stands as
surface water up to what the class's relief holds, and the rest is runoff; while the pores are full, surface
water goes down at the percolation rate; then the drying rate takes surface water, and once there is none
the pores dry in proportion to their fill.

So paving shows a sheen two minutes into a shower. Packed earth whose pores take 15 mm/h shows none while
they fill, 18 minutes at 10 mm/h, and a film from then, well before its puddles form
([part 8](../../../.orchestrate/evidence/PUDDLES/design-proof/README.md): pores full at 1,080 s, a film at 1,081 s, the
relief full at 1,440 s). Ground never takes a sheen only where it both soaks and passes water down at
least as fast as the rain falls, as sand does at 60 mm/h: a class that percolates faster than it soaks
still films while its pores fill. After the rain the sheen is gone long before the darkness (part 5). The first
design said earth that drinks faster than the rain never shows a sheen; its own stores do not do that once
the pores are full, and the design review ran them to show it.

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
- a puddle's **margin**: each hollow keeps one more number, `m`, the height of the damp band above its
  waterline. Each tick a level that fell adds its fall to `m`, a level that rose takes its rise from `m`,
  down to nothing, and then `m` shrinks by the factor `exp(−Δt · drying rate / pore capacity)` and is held
  under the height from the waterline to the hollow's rim. The pores' fill falls off above the waterline
  over `m`. So a puddle that shrinks slowly leaves a thin dark rim and one that drains fast a wide one; a
  level that stops falling leaves a band that fades at the pores' own rate; with drying off the band stays;
  and a hollow that has emptied keeps its band from its pit until it dries;
- a **shore**: inside a body's [wave support](../WATERS/DESIGN.md#the-water-map) the pores are full, and above
  it they fall off over the body's own margin, kept as a hollow's is from the fall of its stage. This is the
  wave support `WATERS`' film reads, so the dark sand and the sheen share one edge (C07).

It gives two numbers: the **pores' fill** and the **surface water** with its cover. Under a puddle it gives
full pores and no surface water: ground under water is wet at its surface whatever its pores hold. On held
ground it gives the fill of the pores under the hollow's mark, with the margin's band as open ground takes
it, and no surface water, which is what the account holds there, in rain too. A margin, a shore's band and the full
pores read under a puddle are appearance: they hold no water in the account (C09's authored approximation).

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
world's one field, beside the wind's cascades and the flow's bands. The scheduled periods are 6.4 m and
\(6.4\sqrt{5/4}\) m (about 7.155 m). Each exceeds the 6 m yard inspection span and their ratio is irrational.
The earlier 1.28 m and 2.1 m periods made individual sparse births visibly repeat even when the two bands'
combined pattern had a longer period. Both patches still use 512² samples. A 64² tile grid rejects drops
whose reach misses the tile. Periodic images are placed when building the lists, and hardware bilinearly
samples the normalized ring table once per contributing drop. The surface's filtering transition stays at
fixed world-space footprints as the periods grow. Drops fall on a schedule drawn from a hash of
place and time, by size class, and a texel's height and slopes are the sum of the rings of the drops that
landed within reach in the last second, each read from a prepared table of a ring's height by radius and age.

**What a drop's birth reads.** A scheduled drop falls if the rain's rate *at the tick it lands* passes its
threshold, whatever the rate is now. The rate history gives that: the flux of each tick of the last second,
one ring's life, 60 numbers at a 60 Hz tick, committed with the tick and uploaded with the rain record. So
each band is a function of the display's time and of that committed history, the same in f64, and keeps
nothing of its own from frame to frame. When the rain stops, the rings already spreading run out their
second and no new one is born; when it starts again no ring appears that would have fallen while it was dry
([part 9](../../../.orchestrate/evidence/PUDDLES/design-proof/README.md): 60, 41 and 27 rings alive 0.05, 0.3 and 0.59 s
after a stop, where births from the present rate give none, and 40 rings born in the dry spell at the
restart). No new drops is not no live rings: the bands' passes run while any tick of the history holds
rain, and stop when it holds none, when the surface is exactly what it is with no rain. A paused frame holds
the bands, and a capture shows its own tick's rings from that tick's history.

- **How many.** Marshall and Palmer's drops at 2, 10 and 50 mm/h number 14, 152 and 918 of 2 mm and over on a
  square metre each second. The largest are scheduled, up to the profile's count (128 a square metre a
  second, about 8 over a point); whether a scheduled drop falls is a soft threshold on the rate at its own
  tick, so a rate that changes slowly adds and removes rings without a pop. The drops below the cut are not
  drawn: their summed squared slope goes into the bands' moments, as everything under a pixel's footprint
  does, summed over the same history with a ring's decay, so it too fades over a second after the rain stops.
- **The gain** at a point is the exposure there times the **depth response**, the model's ratio of a ring's
  height on that depth to its height on deep water: 1 from 10 mm, 0.76 at 5 mm, 0.41 at 2 mm, 0.25 at 1 mm,
  none on dry ground. Every body's row has it, the sea's included.
- **A wave layer in every rule** `WATERS` has for one: the surface's pixels, the slope covariance that blurs a
  reflection and spreads the Sun's glitter, the depth limit, carriage by a current, the film, and the query,
  which reads them as below.

**What a query reads.** A cascade's share of an answer comes from the frame's textures. The rain's cannot:
a batch is dispatched a tick or two before the tick it answers for, when that tick's rate is not yet
committed, and a texture would tie the answer to the display's time and the bands' cadence. So the rain's
share is added on the CPU when the game drains the mailbox, for the tick each answer is used at, a held
answer's included. It is the bands' own function in f64, from the schedule, the use tick's committed
history and prepared tables, at the departures [carriage](../WATERS/DESIGN.md#bodies-and-the-query) gives the
station and at the station's gain. Its slopes are slopes in the world, so on a current that changes from
place to place they take the gradients of the departures and of their blend, as a cascade's do. It reads no
texture, so neither the display's time nor the bands' cadence reaches it. A station with no gain, under a
roof or on dry ground, reads nothing.

**The use tick's history.** An answer used at endpoint N holds the drops that fell up to N, and whether
one fell in the interval from N−1 to N is that interval's rate. So on the tick the game steps the rain
state and commits that interval's rate first, and drains the mailbox after, as the yard's tick does. The
history a request was posted with is a tick or two older and is never the answer's. A drain before the
commit reads a history that carries its last rate forward: where the rain starts between a request's
posting and its use the answer misses the first drops, and where it stops the answer holds drops that
never fell.

This work is on the game's tick, for every station of every body, so nothing in it is repeated for a
station ([part 12](../../../.orchestrate/evidence/PUDDLES/design-proof/README.md)):

- **One list a tick.** The drops alive at the tick are listed once for each band and binned by place, as
  the bands' pass bins them for a frame, and every station of every body in the batch reads that list. No
  station draws the schedule from its hash. The list is a function of the tick and the committed history, so
  whether it is made again each tick or carried from the last is the builder's, and the answers are the
  same either way.
- **A prepared footprint.** A station's footprint is the disc of its radius, and its share of a ring is
  the mean over that disc of the ring the bands draw. That ring is the ring table's: read between its
  radii and ages, faded over its outer 0.04 m, nothing from its reach of 0.32 m, and faded at the end of
  its second. The mean is tabled from that table: for a ladder of footprint radii from none, the ring's
  height, radial slope and rate averaged over the disc, by the distance from the drop to the disc's centre
  and by the ring's age, prepared with the ring table. A station reads the two rungs either side of its
  radius for each drop whose ring reaches its disc. Nothing is integrated on the tick. How the mean is
  prepared is the builder's. What it may not be is the mean of a ring the bands do not draw. The unfaded
  wave's spectrum with the disc's factor on each wavenumber, `2 J₁(kR)/(kR)`, is the mean of the wave
  before its fade and cut-off, and the fade falls inside an ordinary disc: for the island float's 0.128 m,
  a ring of the largest drop class 0.8 s old and one reach from the disc's centre, the review measured
  −0.235 µm for the drawn ring and +0.803 µm from that factor
  ([review](../../../.orchestrate/evidence/PUDDLES/design-review.md), an illustration and no budget). The
  patch's mean of that ring is −0.005 µm
  ([part 13](../../../.orchestrate/evidence/PUDDLES/design-proof/README.md)), so WP4.13 takes that case:
  a tolerance that tells the drawn ring's mean from the patch's there tells it from the factor's too.
- **A bound on the footprint.** Averaged over a disc 0.13 m in radius a 3 mm drop's ring is 2% of its peak,
  and over 0.28 m it is 0.1%. So the ladder ends at twice a ring's reach, 0.64 m. From one reach up the
  rings' share gives way to the band's mean over its whole patch, and by the ladder's end the mean is all of
  it, with no slope. Both are world distances, whatever the bands' periods. A patch's mean is one value for
  a band at a tick and is shared as the list is. So a station reads at most the drops within three reaches
  of it, about 370 in a band at the island's count, and its work does not grow with the periods. Only the
  list does, with the patches' area.

The example's craft and eight floats, 54 stations, are the batch this is proved on: WP4.13 holds their
answers and WP4.10 times their drain. How many stations a query may hold in rain is not claimed, as
[`WATERS`](../WATERS/DESIGN.md#bodies-and-the-query) does not claim it under thrown crests.

Declared approximations: one wave train serves every depth, scaled, though rings on water under 10 mm spread
up to a quarter faster; rings do not reflect at a puddle's edge; a ring is cut off after a second; a
splash's crown and jet are not waves and are not drawn; and a query's footprint in rain is a disc read
between two rungs, where a cascade's is a texel read between two mips.

### Rings from a footfall

A profile may name a **fine interaction level** ([ruling 2](../ARCHITECTURE.md#seam-rulings-for-puddles)),
built in WP8, after `WATERS:WP6` has built the interaction step it extends:
256² cells of 2 cm, a window 5.12 m across round the body the game names, with `WATERS`' operator, regions,
border and sources, and

- a depth ladder of 5, 10, 20, 40, 80 and 160 mm, and the deep operator from 320 mm, with surface tension
  in its frequency;
- the waves under 1 m long, and it alone holds them. The split is `WATERS`' rule one level down:
  complementary filters on the source, crossing at 1 m, whose shares add to the whole source at every
  wavelength, and outside the crossing a wave is whole on one level. The surface and the query add the
  levels, so a wave left on two of them is drawn and felt twice, and each copy still looks like a ring: a
  0.64 m wave fits both grids, 100 times across the near window and 8 times across the fine one. The split
  is by wavelength wherever the source lies, so a source outside the fine window raises no wave under 1 m, a
  declared approximation; the near level's 0.25 m cells held those at two to four cells a wave. With no fine
  level in the profile the near level keeps every wave under the far level's band, as `WATERS` built it;
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
optics: the ground shows through thin water and is lost under a muddy type, and rain's slope variance blurs
its reflections. The view from under water is `WATERS`' on any body and is not judged on water this shallow.
A float sits on it through the one query. From `WATERS:WP16` the film's and the puddle's Sun lobes take the
scene's Sun visibility, by [`RIVERS`' ruling 7](../ARCHITECTURE.md#seam-rulings-for-rivers) and ruling 7
here, so a puddle in a wall's shadow does not glint.

**Mirrored scenery** ([ruling 8](../ARCHITECTURE.md#seam-rulings-for-puddles)). `WATERS` cuts a mirror view
at its plane, so a view holds nothing below its plane. The first design gave a hollows body one plane, the
mean of its ground. A puddle 15 cm below that mean then loses the lowest 15 cm of everything standing in
it, and no lookup can bring back what the view never drew
([part 7](../../../.orchestrate/evidence/PUDDLES/design-proof/README.md): from 0.3 m every reflection on it is wrong and
no ray finds the post's foot). One plane at the yard's lowest water fails the other way: a puddle 13 cm
above it is read 26 cm off its own mirror point, and half of it is wrong from 0.3 m.

So a hollows body has no one level. It gives the mirror each puddle's level and, where its film stands, the
ground's height. Each frame these are grouped into **bands** no taller than the profile's tolerance for
hollows, 5 cm, and a band's plane is its lowest water, so a view's cut takes away nothing above any water it
serves. Bands rank for the profile's views with every other body's level, by the screen area of their water,
with `WATERS`' hysteresis. A lookup starts from its own surface point's height and refuses a sample that
lies below that height, which then takes the environment as any refused sample does. Over the top fifth of a
band the mirror's share gives way to the screen's reflection, so nothing switches at a band's edge. Rain
water in no band with a view takes the screen's reflection, as `WATERS`' third level does.

What that delivers, from part 7, from 0.3 m, 1.7 m and 15 m up:

- Water at its band's plane is exact: every ray that should find the post's foot finds it. A pothole 15 cm
  below the paving is a band of its own, and has a view when its water ranks.
- Water up to 5 cm above its band's plane, such as the film on the paving round a puddle, is read up to
  twice that height off its own mirror point: under 1% of its reflections are wrong.
- Declared residual: a thing standing in water that lies above its band's plane hides a strip of that water
  behind it from the mirrored camera, as wide as that height over the tangent of the view's elevation.
  Those samples are refused. On a puddle with a post in it, 2 cm and 5 cm above its plane, that is 5.5% and
  22% of the puddle from 0.3 m and none from 1.7 m.
- Declared limit: the first profile has two views. Water in more bands than there are views mirrors
  off-screen scenery in the bands that hold the most water on screen, and what is on screen in the rest.

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
  stores, their two rates and the wetness function; the ring tables and their footprint ladder, the drop
  schedule, its count and the rate history; the fine level and its ladder; the canopy's layers; the
  mirror's band tolerance.
- **Limits are inputs.** The labels, the fine bed and the canopy grid are textures, the levels and the stores
  small tables, and the rain bands layers of the cascades' arrays, all counted against the capability record
  at preparation. The forward stage gains one texture, the canopy grid, with both layers in it.
- **Knobs**, in `EFFECT-GRAPH`'s list with a declared invalidation: the rain bands' cadence. It changes no
  query answer.

### Check that it can work

- **P1, P2.** Part 1 runs the tree, the fill and the point test on a made bed through 10 merges and 10
  partings. The preparation in Rust at 640,000 cells, the map on the GPU and the look of a waterline at 5 cm
  cells are unrun; WP1 and WP3 are the first.
- **P3.** Part 3 is the ring's own integral, and part 9 the schedule through a stop and a restart. That a
  sum of scheduled rings reads as rain and not as a pattern, at 128 a square metre a second, is unrun; WP4 is
  the first look ([risk 2](#risks)). Part 3's mean square slope at 50 mm/h, 0.065, is a superposition and
  likely too high; WP4 judges it. Part 12 is the rain's share of a query: the disc's factor against a direct
  average of one ring, both on the wave before its fade, and the example's batch timed four ways. The one
  list gives the answers the first build gave, to 1e-15. So part 12 supports the times and not the
  footprint's answers: its factor is not the mean of the ring the bands draw, and two reads of the ring
  table stand in for the two rungs. The footprint prepared from the ring table is unrun, and WP4.13 is the
  first check of its answers.
- **P4.** Part 6 is the operator on uniform depth, wave by wave. Fields, regions and a puddle's edge are
  `WATERS`' as measured in its part H and J at larger scale, and unrun at this one; WP8 is the first.
- **P5.** Part 7 is a yard in section with puddles at two heights, a post, a kerb, a roof's edge and a wall:
  the planes and cuts above, ray by ray. `WATERS`' optics on water 5 mm deep, the bands' ranking as a camera
  moves and the look are unrun; WP7 is the first.
- **P6, P7.** Parts 4 and 5 are arithmetic, part 8 the stores through a shower long enough to fill the
  pores, and part 10 exposure under two shelters. Part 11 is the mark, the pores under it, held ground and the rain
  on it, on one hollow; hollows that share a puddle through a merge and a parting are unrun, and WP2.12 is
  the first.
  The look of the wet ground and of the film is unrun; WP5 is the first.
- **P8.** The wave support is `WATERS`'. Unrun.
- **Cost.** Unrun, but for the rain's share of a query, which part 12 times. Each work package records its
  own.

## The system as it runs

| State | Owner |
|---|---|
| The tree of hollows and its preparation, the fill step and its account, the canopy grid and exposure, the ground's stores and the wetness function, the ring response and its tables, the rain state | `thrum-rain-model` (this module), on `thrum-water-model`, no GPU dependency |
| The hollows kind in the map's function; the rain bands in the surface model's sum | Files this module adds to `thrum-water-model`, and its changes there under [rulings 1 to 3](../ARCHITECTURE.md#seam-rulings-for-puddles) |
| Hollows in the map on the GPU and their tiles; the rain bands' passes; the fine level; the film over standing water; the mirror's bands; the wetness provider and its tables | Files this module adds to `thrum-water` and `thrum-sea`, and its changes there under rulings 1 to 4 and 8 |
| The wetness read in the forward surface | The scene's shader and the cooked material, under ruling 5 |
| The map, the surface, optics, the query, interaction waves, the film, mirror views and their lookup | `WATERS`, `SCENE-VIEWS`; the bands of a hollows body under ruling 8 |
| The rain record, the ground classes and their patterns, the yard and its roof, the rain selection and its controls, footfalls | The games: `examples/water` and `examples/v1-game` |
| Sky, Sun, air, the cloud factor, the lent Sun visibility | `ATMOSPHERE` |

`thrum-water` takes the rain as data in `upload` and does not link `thrum-rain-model`; the game does.

**A frame, in order**, for what rain adds to [`WATERS`' frame](../WATERS/DESIGN.md#the-system-as-it-runs). On
the tick, in the game, in this order: the rain state steps and commits the rate of the interval that ends
at this endpoint, and then the drain adds the rain's share to the answers it takes, from that history.
`upload` takes the rain record and its rate history, each hollow's level, mark, margin and the fill of the
pores under its mark, and the stores, with the page's stages. `simulate`, every frame while the history
holds rain: the rain bands at `t`, with the cascades.
`simulate`, on a frame that advances the tick: the fine level's step, region by region. Forward: surfaces read
the wetness function. `after-opaque`: the water's gather, then its surface, puddles and film with the rest.

**Creator journey.** A game names the capabilities in its water profile, declares a hollows body over a
rectangle of its ground with a ground class, gives its shelters, keeps a rain state in its
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

1. A hollows body in the map (WP1 on the CPU, WP3 on the GPU).
2. A fine interaction level, which takes the waves under 1 m from the near level (WP8, on the GPU, in the
   interaction step `WATERS:WP6` builds; an f64 reference of it is a diagnostic a builder may add, as
   `WATERS`' is).
3. Rain bands: wave layers the wind does not fill, at a gain every body's row has (WP2 on the CPU, WP4 on the
   GPU).
4. The film is drawn over standing rain water (WP5).
5. A wetness provider on the forward surface, in the scene's shader and the cooked material (WP5).
6. A rain selection in `examples/water` (WP3) and in `examples/v1-game` (WP6).
7. The film's Sun lobe takes the lent Sun visibility, with the surface's (WP7).
8. A hollows body gives the mirror its levels in bands, each cut at its lowest water (WP7).

Four of these replace what [`WATERS`' design](../WATERS/DESIGN.md#what-the-later-modules-build-on) expected: a
puddle is not a basin with a seed, a drop is not an impulse record, the film is drawn off the shore, and a
hollows body is not one mirror level. Used as built: the bed's builder, the surface, the optics, the gather,
a mirror view's draw and its depth correction, the query, impulse records,
the film, the lease of a pattern, the typed controls. Nothing is asked of `EFFECT-GRAPH`, `CONTENT-LOOP`,
`SCENE-VIEWS` or `CLOUDS`.

## Cost account

Estimates, unmeasured, but for the rain's share of a query, which part 12 times on the model alone.

| Work | Rate | Size | Estimate |
|---|---|---|---|
| Preparing a hollows body | Prepared | One flood of its cells, a sort for each node. A yard 40 m by 40 m at 5 cm is 640,000 cells | Under a second on one core; WP1 records it. A node's curve of level for volume is the builder's to keep small |
| The fill step | Tick, on the CPU | A few operations for each puddle and each node that spills | Microseconds for hundreds of puddles; WP1 records it |
| Hollows in the map | Every read inside a hollows rectangle | A label, a level and the fine bed | Three reads on puddle pixels; none elsewhere. 640,000 cells are about 4 MiB |
| Puddle tiles | Frame | Cells that hold water and their neighbours, at the cell's spacing | A fifth of a yard under water is about 130,000 quads; WP3 records it |
| Rain bands | Frame, while the last second held rain and water is in view | Two patches of 512², with one bilinear table lookup per contributing drop. The schedule reads committed history before binning. | Under the cascades' cost; two more layer reads in the surface where the gain is not zero |
| Rain in the query | Tick, on the CPU, in the game's drain, while the last second held rain | One list of the live drops of both bands: 5,900 at the design's count and 11,800 at the island's. Then, for each station and departure, two table reads for each drop whose ring reaches its disc: 90 to 180 at the example's radii, 740 at the ladder's end. The ladder is prepared with the ring table, which takes about half a second on one core as built | Part 12, on the shared Mac, one thread, with stand-in reads: 0.4 to 0.8 ms a tick for the example's 54 stations, 1.0 to 1.9 ms with every footprint at the ladder's end, and the stations' part twice that where two departures blend. As first built, 30 to 57 ms. WP4.10 records the drain in rain and dry. A filtered ring is smooth at its footprint's scale and needs fewer radii and wavenumbers than the ring, so the ladder's preparation and bytes are expected under the ring table's own; the builder records both in the log |
| The fine level | Tick, while a source is in the window | 256²; one solve for each region and ladder depth in use, up to the profile's count | A few tenths of a millisecond; WP8 records it with the count of solves |
| The wetness function | Frame, every forward fragment in a scene with wet ground | One read of the canopy grid's two layers and arithmetic on a small table | One texture more in the forward stage, the second with `WATERS`' caustics; WP5 records the pair |
| The mirror's bands | Frame, on the CPU | The levels of the hollows in view, sorted and grouped; no view beyond the profile's | Negligible; a band's view costs what any mirror view does, and WP7.8 records it |
| The film | Frame, over a hollows body's ground with surface water | The film's pixels: the function, a pattern read, the reflection | Of the order of the surface on those pixels; WP5 records it |

Records and tripwires are [`WATERS`'](../WATERS/DESIGN.md#cost-account): `scripts/water_cost.py`, with scenes
this module adds, and `scripts/rain_cost.py` for preparation, the fill step and the valley. No timing figure
decides an item ([brief](../VISION.md#goal-and-exit-rule), decision 3).

One wire is this module's. The record of WP4.10 holds the time the game's tick spends draining the
example's batch in rain, beside the same batch dry, and a drain in rain over a tenth of the tick trips it.
That is about twice the designed work at the island's count, so growth is investigated in the work package
where it starts, before it is a stall. It is investigated as any wire is and decides nothing.

## Risks

1. **A waterline shows its cells**, or a puddle's far edge hangs in the air at a sill. Early sign: WP3's
   captures at 1.7 m. The designed remedies are a finer cell, a bed built from the highest ground in each
   cell as the map's is, and the surface carried one cell past the water. The wrong fix is a soft edge drawn
   in the ground's material.
2. **Rain bands read as a pattern**: a grid, a repeat, or rings too alike. Early sign: WP4's first captures.
   The designed remedies are the two patches' sizes, more size classes and the schedule's count. The wrong
   fix is a texture of rings.
3. **Rain makes all water matt**, because the moments of the drops below the cut are too large. Early sign:
   WP4's captures at 50 mm/h. The designed remedy is a lifetime on the moments' integral, as the rings have.
4. **The film fights the ground or floats over it**, `WATERS`' risk 16, now on paving. Early sign: WP5's
   captures at a grazing view. Its depth bias and the bed are the remedies.
5. **The film's joints miss the ground's joints.** Early sign: WP5's close views. The pattern is one texture
   read by both; the wrong fix is a second pattern tuned to look near enough.
6. **Wet ground reads as a tint**: flat, with no sheen. Early sign: WP5's captures. The film, the lower
   roughness and `p` by class are the mechanism; three failed looks on one cue go to the log.
7. **The forward pass will not take another texture in Chrome**, with `WATERS`' caustics. Early sign: WP5's
   preparation. The designed remedy is the canopy grid packed with the caustic level's array.
8. **Too many puddles for the fine level's solves.** Early sign: WP8's record. Regions are admitted by their
   latest source, so a walked path is solved and the rest lie at rest.
9. **`WATERS` and `RIVERS` move under this module's changes**, since three builders edit the same crates.
   Each ruling keeps its change behind a declaration that existing scenes do not make, and each work package
   runs their tests.
10. **A puddle without falling rain looks unexplained.** Early sign: WP4's review. Rain in the air is the
    remedy this plan does not have ([plan](PLAN.md#open-for-the-human)).
11. **Exposure's edge is a ruled line.** Early sign: WP5's captures under the roof. The soft edge's width is
    the remedy.
12. **Mirror bands swap as the camera moves, or a puddle's reflection breaks behind a post.** Early sign:
    WP7.2's pan and WP7.12's ranking. The designed remedies are `WATERS`' hysteresis, the fade at a band's
    top and a smaller tolerance. The wrong fix is one plane for the yard, which part 7 fails either way.
13. **A real scene stacks three shelters in a column**, a crown over a porch under an eave. Early sign: a
    refusal at preparation. The remedy is a third layer in the grid, a profile value; the wrong fix is
    dropping the refusal.
14. **Held ground shows as a matt patch** beside ground that shines, where a hollow is about as shallow as
    its class's relief is deep. Early sign: WP5.6's drying sequence. The designed remedies are a class's
    relief kept well under its hollows' depth, and deeper dips in the yard. The wrong fixes are to read the
    class's film on ground the account says has none, and to hand ground back on water the puddle does not
    hold.
15. **Rain stalls the game's tick**, because the query's share of it grows with the stations, the count or
    the periods. Early sign: WP4.10's drain in rain beside its dry one. The designed remedies are the one
    list a tick, the prepared footprint and its bound. The wrong fixes are a share read from the bands'
    textures, which follows the display; a footprint that is dropped or prepared from a ring the bands do
    not draw; a drain ahead of the tick's commit, to save the step; and rain left out of the answer.

## Proof and builder handoff

[`WATERS`' handoff](../WATERS/DESIGN.md#proof-and-builder-handoff) holds here word for word: targets, capture,
cost records, looks, the `gpu` lock, exact baselines, tests that hold no game content, Broadsides and
dependencies. In addition:

- **Scratch and evidence.** `tmp/puddles-<work-package>/` and `.orchestrate/evidence/PUDDLES/`. Copies are listed in
  the tracked [provenance record](../../provenance/PUDDLES.md); none is expected, and Barnes's code is not copied.
- **Other modules' files.** A change inside a file another module wrote is one a ruling names, behind a
  declaration no existing scene makes. Anything else it needs is a proposal in this module's log.
- **The bar.** `WATERS`' handoff carries [the owner's bar](../../../ORCHESTRATE.md#what-is-proved-and-how-hard) of 17 November 2026,
  and it holds here: WP3 to WP8 pass on their look and their speed. What a game reads stays tested against the
  CPU models at a tolerance each test states: the puddles in the map (WP3.3), the rain bands at the display's
  time (WP4.1) and the wetness function (WP5.2). Each of those tests also runs a named wrong evaluator and shows
  it miss: for WP4.1, bands evaluated half a tick late and bands that ignore the rate history; for WP5.2, wetness
  that ignores slope and wetness that reads the open store everywhere. The rain's share of a query is
  `thrum-water-model`'s footprint read, and WP4.13 holds it twice. First, the read against a direct average
  over the disc of the bands' point function, which reads the ring table and uses nothing the footprint was
  prepared with: in steady rain with a read at a point and the patch's mean as its wrong evaluators, and
  at a float's radius for one ring whose faded edge lies inside the disc, with the patch's mean as its
  wrong evaluator, where the mean of the unfaded wave is four times further off than the patch's mean is
  (part 13). Second, the mailbox's answers against the read from the use tick's committed history, in rain
  that stops between a request's posting and its use, with the read from the history at the posting as its
  wrong evaluator: it holds drops of the dry interval, so it misses wherever one fell in a station's disc.
  WP4 as first built compared the mailbox with the model only at a point and at a 3 m hull, where the
  footprint is none or all. The wet look, a ring's shape and the
  fine level's waves are judged by eye, with no f64 reference required and no bound measured first. Which
  level holds a wave is not a look: it is state with one owner, two copies of a ring pass every view, and
  WP8.9 tests it on the levels' own state, with the near level still given the short waves as its wrong
  evaluator. Parts 1 to
  11 are the source of the bounds WP1 and WP2 were proved with.
- **Receivers.** The account's total and the receivers are separate proofs. A test of a receiver sets a
  case where the total would close under the wrong assignment too, and checks that receiver's gain alone.
  It finds each receiver's cells from the bed, the shelters and the water's level at the tick's start. It
  reads a mark only to tell held ground from open ground, and never as the edge of the water.
- **Time in the views.** An inspected item that shows filling or drying may turn the rain and the drying rate
  up so the change fits the sequence; it says the rates it used.
- **Maturity.** A ledger row for `E1.rain` is added in WP3 and kept current.

## Open questions

| Question | Settled by |
|---|---|
| What does preparing a yard cost, and how small can a node's curve be kept? | WP1 records it |
| Does a 5 cm cell give a clean waterline at 1.7 m? | WP3; [risk 1](#risks) |
| Do 128 rings a square metre a second read as heavy rain, and what moments do the rest need? | WP4; [risks 2 and 3](#risks) |
| What `p` reads right for each ground, and does the film carry wet paving? | WP5; [risk 6](#risks) |
| How many sampled textures do the surface and the forward stage need with rain, in Chrome? | WP3 and WP5 count against the record |
| Do two mirror views serve a yard whose water stands at several heights, and is 5 cm the right band? | WP7.2 and WP7.12; [risk 12](#risks) |
| Does a rainy scene need rain in the air to meet its bar? | WP4's and WP7's reviews; the owner ([plan](PLAN.md#open-for-the-human)) |
