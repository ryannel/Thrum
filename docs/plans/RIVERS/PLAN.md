# RIVERS plan

Streams, rivers and waterfalls in the one water system. [Design](DESIGN.md) · [done-when file](DONE.json) ·
[log](LOG.md) · [module list](../ARCHITECTURE.md) · [`WATERS`](../WATERS/DESIGN.md)

Before any milestone, read [Proof and builder handoff](DESIGN.md#proof-and-builder-handoff),
[the rulings](../ARCHITECTURE.md#seam-rulings-for-rivers) and [Exact baselines](../ARCHITECTURE.md#exact-baselines).

## Open for the human

- Assumption: a waterfall has no mist or spray cloud. Shipped waterfalls have one; here it needs a seam with the weather (`CLOUDS`) that cannot be built until late in its plan. The fall already records what a mist would need.
- Assumption: "built cohesively" reaches a river's link to its lake, so raising a lake swells its outlet river and its falls, by the outlet's own sill. The whole river answers at once: no rise travels downstream, and nothing floods or bursts.
- Assumption: a river's flow is still worked out ahead and stored, once for each of a few source levels, never while the game runs. A page in Chrome loads stored rivers and never solves one.
- Assumption: rivers are not fed by rain. Their sources are springs and lakes the game declares, and a spring's flow is a control.
- Assumption: rapids have no spray and no standing waves that overturn, and nothing in a river surges by itself: the flow is steady, and what moves is its ripples, its foam and what it carries.
- Assumption: a fall is one shape, a sheet that follows the water's free path or the rock's face, from 0.75 m to 12 m high. A lower drop is a chute in the stream.
- Assumption: the island shows the solved rivers and the waterfall only when a selection is turned on, until `WATERS` closes, so that `WATERS`' own reviews see the island they were written for.
- Assumption: this module builds inside `WATERS`' crates under seven rulings, and two of them replace what `WATERS`' design expected: a stream's ripples are made by its flow and not by the wind, and a reach follows its source's level.
- Assumption: the module closes without waiting for the governor and MetalFX, which `WATERS`' last milestone waits for; its two quality controls are declared for the governor to use.
- Assumption: a float does not ground on the bed, and running water makes no sound, because the engine has no physics or audio owner.
- Assumption: cost is recorded on the Mac only, at 2560×1440, and no timing figure decides an item (brief decision 3).

## M1 On the CPU alone, a spring's water runs down a valley, fills a pool to its sill, quickens through a narrows and reaches a lake, and the solved flow is a chart the water map takes

This creates `thrum-river-model`, on `thrum-water-model` from `WATERS` M1, which it does not change here. Part 1 of `docs/evidence/RIVERS/design-proof/river_proof.py` is a first-order sketch of the scheme and the source of M1.2's bounds; parts 2 and 6 give the section and the kind of valley M1.1 and M1.4 name. The river file's reader goes in `thrum-water-model` as a new file. If that crate's chart has no loss grid yet, the river file keeps the loss and M4 hands it over.

## M2 On the CPU alone, where the ground drops away the stream leaves it as a waterfall: its path, speed and thickness are worked out, and its water lands in the pool below and runs on

Ruling 1 changes the map's preparation in `thrum-water-model`, which `WATERS`' builder is also editing; land on top of what is there and leave its tests as they are. Part 4 gives M2.4's closed forms.

## M3 On the CPU alone, the river answers its source: it runs deeper, wider and faster as its lake rises, rough in its riffles and smooth in its pools, in a calm as in a gale

Rulings 2 and 3 change the map's function and the surface model in `thrum-water-model`. The flow bands' spectrum and evaluator live there, not in `thrum-sea-model`, which is `SEA`'s until its last milestone. Part 3 gives M3.2's bounds.

## M4 On the island a stream runs from a spring through a pool and a narrows to the lake, and a river runs from the lake to the sea; raise the lake and the river swells, and a float rides from the spring to the sea

The island's ground, its channel's cut and `scripts/water_cost.py` and `scripts/check_water_browser.mjs` are `WATERS`'; add the valley, cases and scenes behind the selection and change none of theirs. River files are generated beside the example's cooked content and are not committed. The fall is in the map from here and is drawn in M6.

## M5 On a windless day the stream still reads as running water: its riffles are rough and glitter, its pools lie glassy, and foam forms below its ledges and behind its rocks and trails away downstream

The flow bands' passes join the cascades' in `thrum-sea`, which is `WATERS`' to extend by now. Measure M5.1's bound first, then assert it. `c` and the birth constant are the two values to tune against M5.4 and M5.5; record both.

## M6 A waterfall pours over its lip in a smooth tongue, breaks into white strands as it gathers speed and plunges into its pool, which boils, foams and sends out rings

Measure M6.1's bound and M6.3's wave share first, then assert them. The game names a body in the plunge pool for the views that judge rings, because interaction windows follow the named body.

## M7 A visitor raises the lake, sets the stream's flow and takes the day from dawn to night, and the rivers and the waterfall hold up in motion, in shade and from under water, on the Mac and in Chrome

If `WATERS` already takes the lent Sun visibility on its surface, M7.3 adds only the sheet's. A defect in the reconstructor, and not in what the water writes, goes to `APPLE-PRESENTATION`'s log.
