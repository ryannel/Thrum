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
- Assumption: puddles stand only on stretches of ground a game marks out, up to about 100 m across, where the ground's own dips are modelled at 2.5 to 10 cm. Everywhere else the ground darkens and shines and holds no puddle, and reflects the sky but not the scenery. Scenery that is off the screen is mirrored only in the water at the two heights that fill most of the view; the rest mirrors what is on screen.
- Assumption: rain does not feed lakes or rivers, water that runs off a marked stretch is lost, and no pool is left by the tide: a marked stretch may not touch other water.
- Assumption: wet ground keeps no memory of what crossed it. There are no footprints or tyre tracks, no mud, no dry patch where a parked thing stood, and no water seen running down a slope or dripping from a roof.
- Assumption: "ripple under feet" is met by rings from a footfall the game reports. No game here has a character, so the island shows a marker that walks and a dropped stone.
- Assumption: shelter is at most two things over any spot, such as a tree under a roof; a third is refused when the scene is prepared.
- Assumption: every surface the forward pass draws gets wet by one rule, props and rocks included, with how much each darkens read from its roughness and metalness unless its material says.
- Assumption: in the V1 valley rain sits behind a selection that is off by default, so the exit scene is unchanged, and it waits for the populated valley (`SHOWCASE` WP5).
- Assumption: this module builds inside `WATERS`' crates and the forward shader under eight rulings, and four of them replace what `WATERS`' design expected: a puddle is not a lake with a seed point, a raindrop is not an impulse record, the shore's film is also drawn on wet paving, and a yard of puddles is mirrored at several heights.
- Assumption: the module closes only after `WATERS` has its mirror view, its look in motion and its light under water (M14, M10 and M16), because a puddle that mirrors only what is on screen is below the bar.
- Assumption: the owner's bar of 17 November applies as in `WATERS`: puddles, rain bands and wetness stay tested against the model, and the wet look and rings are judged by eye.
- Assumption: cost is recorded on the Mac only, at 2560×1440 on the island and the showcase's extent in the valley, and no timing figure decides an item (brief decision 3).

## WP1 On the CPU alone, rain fills the hollows of a stretch of ground: puddles stand level where it dips, spill into each other, join into one and part again as they dry, and ground under a roof stays dry: done
[review](../../../.orchestrate/evidence/PUDDLES/WP1-review.md)

## WP2 On the CPU alone, ground wets and dries as ground does, sheen first and darkness last, and a raindrop's rings spread as water's own waves do: done
[review](../../../.orchestrate/evidence/PUDDLES/WP2-review.md)

## WP3 On the island, rain fills the hollows of a yard: puddles lie level in the ground, grow, join and shrink with a clean waterline, show the ground through them and mirror the sky, and a float sits in one

The island's ground, bodies, `scripts/water_cost.py` and `scripts/check_water_browser.mjs` are `WATERS`'; add the yard, cases and scenes behind the selection and change none of theirs. The yard's ground has to hold its dips as real triangles, since the ground's depth cuts each waterline. `thrum-water` takes the rain as data in `upload` and does not link `thrum-rain-model`. If `RIVERS`' selection has landed, the two selections are independent.

## WP4 Rain rings every water it falls on, the puddles, the lake and the sea, more and rougher as it grows and not at all under the roof, and the rings run out when the rain stops

The rain bands' passes join the cascades' in `thrum-sea`, which is `WATERS`' to extend by now. WP4.1's test states its own tolerance. The schedule's count and the moments of the drops below the cut are the two values to tune against WP4.4 and WP4.5; record both.

## WP8 A footfall or a dropped stone in a puddle sends rings across it at the speed its depth gives, and they die at its edge and stop at a dry sill

Built after WP4 and before WP5. Ruling 2 extends the interaction step `WATERS` WP6 built, in `thrum-water`; leave its tests as they are. The game names the walking marker as the body the interaction windows follow. `WATERS` WP6.3's test reads only its near and far levels, so it passes with a short wave on both the near and the fine level; WP8.9 is the test that reads the near level with the fine level on.

## WP5 The ground itself gets wet: paving and earth darken as the rain starts, a sheen gathers in the paving's joints and mirrors what stands on it, the ground under the roof stays dry, sand is dark where waves have run, and it dries sheen first and darkness last

Ruling 5 edits `thrum-render`'s `synthetic/scene_source.rs` and `synthetic.wgsl` and `thrum-asset-gpu`'s material, which other modules own; compose the read as `compose_with_cloud` composes the cloud factor, and leave the text of a scene with no provider as it is. `WATERS` WP16 adds a water factor to the same shader; whichever lands second integrates. WP5.2's test states its own tolerance. Each class's `p` and the film's roughness are the values to tune against WP5.5 and WP5.6; record them.

## WP6 In the V1 valley, behind a selection, a shower darkens the desert floor and its rocks and leaves puddles in a patch of ground, and with the selection off the exit scene is what it was

`examples/v1-game` is `SHOWCASE`'s, and `CLOUDS` has a selection there too. The valley's terrain is a cooked mesh too coarse to hold a puddle, so the selection adds its own patch of ground with dips; it does not edit the terrain, the generator, a route or the exit configuration.

## WP7 A visitor starts and stops the rain, sets how fast things dry and takes the day from dawn to night, and the puddles and the wet ground mirror the scenery round them and hold up in motion, on the Mac and in Chrome

If `RIVERS` or `WATERS` already gives the surface's Sun lobe the lent Sun visibility, WP7.3 adds only the film's. Ruling 8 changes the mirror's choice of levels and its lookup in `thrum-water`, behind a hollows body; part 7 of `revision_proof.py` is WP7.12's model. A defect in the reconstructor, and not in what the water writes, goes to `APPLE-PRESENTATION`'s log.
