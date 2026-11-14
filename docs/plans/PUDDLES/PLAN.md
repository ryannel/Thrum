# PUDDLES plan

Rain, puddles and wet ground in the one water system. [Design](DESIGN.md) · [done-when file](DONE.json) ·
[log](LOG.md) · [module list](../ARCHITECTURE.md) · [`WATERS`](../WATERS/DESIGN.md)

Before any milestone, read [Proof and builder handoff](DESIGN.md#proof-and-builder-handoff),
[the rulings](../ARCHITECTURE.md#seam-rulings-for-puddles) and [Exact baselines](../ARCHITECTURE.md#exact-baselines).

## Open for the human

- Assumption: rain is not drawn falling. There are no streaks in the air, no splashes, no drops on the lens and no sound; rain is seen by what it does to the ground and the water. Shipped rainy scenes have all four; here they need an owner for particles and a seam with the weather, which nothing plans. It is the largest thing this plan leaves out.
- Assumption: rain is one rate the game sets for the whole scene, falling straight down, and nothing ties it to the clouds. Wind does not slant it, so the ground under an open-sided roof is dry to the roof's edge.
- Assumption: drying is one rate the game sets, and a demo turns it up, since real puddles last hours. Shaded ground dries as fast as sunlit ground.
- Assumption: "built cohesively" reaches every water, so rain rings the lake, the sea and the rivers as it rings a puddle.
- Assumption: puddles stand only on stretches of ground a game marks out, up to about 100 m across, where the ground's own dips are modelled at 2.5 to 10 cm. Everywhere else the ground darkens and shines and holds no puddle, and reflects the sky but not the scenery.
- Assumption: rain does not feed lakes or rivers, water that runs off a marked stretch is lost, and no pool is left by the tide: a marked stretch may not touch other water.
- Assumption: wet ground keeps no memory of what crossed it. There are no footprints or tyre tracks, no mud, no dry patch where a parked thing stood, and no water seen running down a slope or dripping from a roof.
- Assumption: "ripple under feet" is met by rings from a footfall the game reports. No game here has a character, so the island shows a marker that walks and a dropped stone.
- Assumption: every surface the forward pass draws gets wet by one rule, props and rocks included, with how much each darkens read from its roughness and metalness unless its material says.
- Assumption: in the V1 valley rain sits behind a selection that is off by default, so the exit scene is unchanged, and it waits for the populated valley (`SHOWCASE` M6).
- Assumption: this module builds inside `WATERS`' crates and the forward shader under seven rulings, and three of them replace what `WATERS`' design expected: a puddle is not a lake with a seed point, a raindrop is not an impulse record, and the shore's film is also drawn on wet paving.
- Assumption: the module closes only after `WATERS` has its mirror view, its look in motion and its light under water (M14, M10 and M16), because a puddle that mirrors only what is on screen is below the bar.
- Assumption: cost is recorded on the Mac only, at 2560×1440 on the island and the showcase's extent in the valley, and no timing figure decides an item (brief decision 3).

## M1 On the CPU alone, rain fills the hollows of a stretch of ground: puddles stand level where it dips, spill into each other, join into one and part again as they dry, and ground under a roof stays dry

This creates `thrum-rain-model`, on `thrum-water-model` from `WATERS` M1, which it changes only as ruling 1 says. `WATERS`' and `RIVERS`' builders edit that crate too; land on top of what is there and leave its tests as they are. Part 1 of `docs/evidence/PUDDLES/design-proof/puddle_proof.py` is a sketch of the tree, the fill and the point test, and keeps every node's sorted bed, which is too large at 640,000 cells; how a node's curve is kept is yours, and M1.9 records its bytes. The ground's stores come in M2, so M1's ground stores nothing.

## M2 On the CPU alone, ground wets and dries as ground does, sheen first and darkness last, and a raindrop's rings spread as water's own waves do

Rulings 2 and 3 change the interaction reference and the surface model in `thrum-water-model`; `RIVERS` adds its flow bands to the same sum, and whichever lands second integrates. Parts 3 to 6 give the bounds. The ring tables are data the model makes; the surface model in `thrum-water-model` takes them as data and does not depend on `thrum-rain-model`.

## M3 On the island, rain fills the hollows of a yard: puddles lie level in the ground, grow, join and shrink with a clean waterline, show the ground through them and mirror the sky, and a float sits in one

The island's ground, bodies, `scripts/water_cost.py` and `scripts/check_water_browser.mjs` are `WATERS`'; add the yard, cases and scenes behind the selection and change none of theirs. The yard's ground has to hold its dips as real triangles, since the ground's depth cuts each waterline. `thrum-water` takes the rain as data in `upload` and does not link `thrum-rain-model`. If `RIVERS`' selection has landed, the two selections are independent.

## M4 Rain rings every water it falls on, the puddles, the lake and the sea, more and rougher as it grows and not at all under the roof, and a footfall in a puddle sends rings to its edges

The rain bands' passes join the cascades' in `thrum-sea`, which is `WATERS`' to extend by now. Measure M4.1's and M4.6's bounds first, then assert them. The game names the walking marker as the body the interaction windows follow for the views that judge footfalls. The schedule's count and the moments of the drops below the cut are the two values to tune against M4.4 and M4.5; record both.

## M5 The ground itself gets wet: paving and earth darken as the rain starts, a sheen gathers in the paving's joints and mirrors what stands on it, the ground under the roof stays dry, sand is dark where waves have run, and it dries sheen first and darkness last

Ruling 5 edits `thrum-render`'s `synthetic/scene_source.rs` and `synthetic.wgsl` and `thrum-asset-gpu`'s material, which other modules own; compose the read as `compose_with_cloud` composes the cloud factor, and leave the text of a scene with no provider as it is. `WATERS` M16 adds a water factor to the same shader; whichever lands second integrates. Measure M5.2's bound first. Each class's `p` and the film's roughness are the values to tune against M5.5 and M5.6; record them.

## M6 In the V1 valley, behind a selection, a shower darkens the desert floor and its rocks and leaves puddles in a patch of ground, and with the selection off the exit scene is what it was

`examples/v1-game` is `SHOWCASE`'s, and `CLOUDS` has a selection there too. The valley's terrain is a cooked mesh too coarse to hold a puddle, so the selection adds its own patch of ground with dips; it does not edit the terrain, the generator, a route or the exit configuration.

## M7 A visitor starts and stops the rain, sets how fast things dry and takes the day from dawn to night, and the puddles and the wet ground mirror the scenery round them and hold up in motion, on the Mac and in Chrome

If `RIVERS` or `WATERS` already gives the surface's Sun lobe the lent Sun visibility, M7.3 adds only the film's. A defect in the reconstructor, and not in what the water writes, goes to `APPLE-PRESENTATION`'s log.
