# RIVERS plan

Streams, rivers and waterfalls in the one water system. [Design](DESIGN.md) · [done-when file](DONE.json) ·
[log](LOG.md) · [module list](../ARCHITECTURE.md) · [`WATERS`](../WATERS/DESIGN.md)

Before any work package, read [Proof and builder handoff](DESIGN.md#proof-and-builder-handoff),
[the rulings](../ARCHITECTURE.md#seam-rulings-for-rivers) and [Exact baselines](../ARCHITECTURE.md#exact-baselines).

## Open for the human

- Assumption: aim for about 2 s per fast test when run alone on a quiet Mac. The owner can revise this target.
- Assumption: from the bank, the depth a river writes may stand 4 pixels of a 2560×1440 frame from the surface a float rides (about 2 cm at 6 to 7 m). The lead ruled the fifth review's reading of 1.9 cm there, 3.5 pixels, no fault; rounding it to 4 is the planner's choice, and [the design](DESIGN.md#proof-and-builder-handoff) gives the reasoning. From above the bound stays 1 cm.
- Assumption: a waterfall has no mist or spray cloud. Shipped waterfalls have one; here it needs a seam with the weather (`CLOUDS`) that cannot be built until late in its plan. The fall already records what a mist would need.
- Assumption: "built cohesively" reaches a river's link to its lake, so raising a lake swells its outlet river and its falls, by the outlet's own sill. The whole river answers at once: no rise travels downstream, and nothing floods or bursts.
- Assumption: a river's flow is still worked out ahead and stored, once for each of a few source levels, never while the game runs. A page in Chrome loads stored rivers and never solves one.
- Assumption: rivers are not fed by rain. Their sources are springs and lakes the game declares, and a spring's flow is a control.
- Assumption: rapids have no spray and no standing waves that overturn, and nothing in a river surges by itself: the flow is steady, and what moves is its ripples, its foam and what it carries.
- Assumption: a fall is one shape, a sheet that follows the water's free path or the rock's face, from 0.75 m to 12 m high. A lower drop is a chute in the stream. Only the island's fall, about 4 m, is judged by eye.
- Assumption: the water a fall lands in stands right up to the rock under it, as a plunge pool does, and a fall whose pool could rise over its lip is refused, since a drowned fall is not built.
- Assumption: how soon a fall turns white is set by eye for each game's water, and not by the laboratory figure for when a falling sheet breaks up, which draws the island's flood clearer than its middling flow.
- Assumption: the island shows the solved rivers and the waterfall only when a selection is turned on, until `WATERS` closes, so that `WATERS`' own reviews see the island they were written for.
- Assumption: this module builds inside `WATERS`' crates under seven rulings, and three of them replace what `WATERS`' design expected: a stream's ripples are made by its flow and not by the wind, a reach follows its source's level, and rings are carried at each water's own current where a pool meets a riffle.
- Assumption: the module closes without waiting for the governor and MetalFX, which `WATERS`' last work package waits for; its two quality controls are declared for the governor to use.
- Assumption: a float does not ground on the bed, and running water makes no sound, because the engine has no physics or audio owner.
- Assumption: cost is recorded on the Mac only, at 2560×1440, and no timing figure decides an item (brief decision 3).
- Assumption: the owner's bar of 17 November applies as in `WATERS`: what a game reads stays tested (the river in the map and the query, with the flow bands in its answers), and the look of the sheet, the flow bands and the river's caustics is judged by eye.

## WP1 On the CPU alone, a spring's water runs down a valley, fills a pool to its sill, quickens through a narrows and reaches a lake, and the solved flow is a chart the water map takes

[Review](../../../.orchestrate/evidence/RIVERS/WP1-review.md)

## WP2 On the CPU alone, where the ground drops away the stream leaves it as a waterfall: its path, speed and thickness are worked out, and its water lands in the pool below and runs on

[Review](../../../.orchestrate/evidence/RIVERS/WP2-review.md)

## WP3 On the CPU alone, the river answers its source: it runs deeper, wider and faster as its lake rises, rough in its riffles and smooth in its pools, in a calm as in a gale

[Review](../../../.orchestrate/evidence/RIVERS/WP3-review.md)

## WP8 Small river fixtures keep the solver checks fast

[Review](../../../.orchestrate/evidence/RIVERS/WP8-review.md)

## WP9 Chart gauges and solver determinism run within the fast-suite limit

[Review](../../../.orchestrate/evidence/RIVERS/WP9-review.md)

## WP10 The check that a river file keeps each sample's own gains runs on a small reach

[Review](../../../.orchestrate/evidence/RIVERS/WP10-review.md)

## WP4 On the island a stream runs from a spring through a pool and a narrows to the lake, and a river runs from the lake to the sea; raise the lake and the river swells, and a float rides from the spring to the sea

[Review](../../../.orchestrate/evidence/RIVERS/WP4-review.md)

## WP5 On a windless day the stream still reads as running water: its riffles are rough and glitter, its pools lie glassy, and foam forms below its ledges and behind its rocks and trails away downstream

[Review](../../../.orchestrate/evidence/RIVERS/WP5-review.md)

## WP6 A waterfall pours over its lip in a smooth tongue, breaks into white strands as it gathers speed and plunges into its pool, which boils, foams and sends out rings

WP6.10's figures are parts 9 and 12's: build the ladder in `thrum-water`'s interaction step. No f64 reference of it is required. The wave share is tuned by eye against WP6.5 and recorded, and nothing asserts the fall's power as its ceiling. The game names a body in the plunge pool for the views that judge rings, because interaction windows follow the named body.

WP6 resumes from `380a78fc7` on `build/rivers` ([handoff](../../../.orchestrate/evidence/RIVERS/WP6-handoff.md)). [The water below a fall stands to the rock](DESIGN.md#falls-inside-the-solve), whichever body receives it, and the upper reach keeps its brink and its unmixed row. Build WP6.12 before returning to the looks. The island's plunge pool stays a reach, because a basin has no current to carry the foam off, which WP6.3 and WP6.5 need. A case in `brink_end` or `falls` that pins ground under the receiving level as no body changes to the new rule; say which in the log.

[The sheet's lengths are the game's](DESIGN.md#the-falls-sheet): tune them and the tear thickness against WP6.4 and record them. If the flood still reads as a curtain once they are tuned and the pool meets the sheet, stop on that cue and say so in the log ([risk 4](DESIGN.md#risks)), so the lead can decide. What stands behind the sheet is the selection's scenery and may change with it.

## WP7 A visitor raises the lake, sets the stream's flow and takes the day from dawn to night, and the rivers and the waterfall hold up in motion, in shade and from under water, on the Mac and in Chrome

If `WATERS` already takes the lent Sun visibility on its surface, WP7.3 adds only the sheet's. A defect in the reconstructor, and not in what the water writes, goes to `APPLE-PRESENTATION`'s log.
