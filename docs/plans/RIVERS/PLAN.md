# RIVERS plan

Streams, rivers and waterfalls in the one water system. [Design](DESIGN.md) · [done-when file](DONE.json) ·
[log](LOG.md) · [module list](../ARCHITECTURE.md) · [`WATERS`](../WATERS/DESIGN.md)

Before any work package, read [Proof and builder handoff](DESIGN.md#proof-and-builder-handoff),
[the rulings](../ARCHITECTURE.md#seam-rulings-for-rivers) and [Exact baselines](../ARCHITECTURE.md#exact-baselines).

## Open for the human

- Assumption: aim for about 2 s per fast test when run alone on a quiet Mac. The owner can revise this target.
- Assumption: from the bank, the depth a river writes may stand 4 pixels of a 2560×1440 frame from the surface a float rides (about 2 cm at 6 to 7 m); [the design](DESIGN.md#proof-and-builder-handoff) gives the reasoning. From above the bound stays 1 cm.
- Assumption: a waterfall has no mist: nothing drifts off it or hangs in the air. It is the weather's (`CLOUDS`). A heavy fall does have spray at its foot, falling back within half the fall's height, which the fall draws itself.
- Assumption: "built cohesively" reaches a river's link to its lake, so raising a lake swells its outlet river and its falls, by the outlet's own sill. The whole river answers at once: no rise travels downstream, and nothing floods or bursts.
- Assumption: a river's flow is still worked out ahead and stored, once for each of a few source levels, never while the game runs. A page in Chrome loads stored rivers and never solves one.
- Assumption: rivers are not fed by rain. Their sources are springs and lakes the game declares, and a spring's flow is a control.
- Assumption: rapids have no spray and no standing waves that overturn, and nothing in a river surges by itself: the flow is steady, and what moves is its ripples, its foam and what it carries.
- Assumption: a fall is one shape, a sheet that follows the water's free path or the rock's face, from 0.75 m to 12 m high, with layers about it where the fall is heavy. A lower drop is a chute in the stream. Only the island's fall, about 4 m, is judged by eye.
- Assumption: the water a fall lands in stands right up to the rock under it, as a plunge pool does, and its waves wash that rock as they wash a bank. A fall whose foot the water could cover to the lip at any level in its range is refused, whether by its own rise or by a lake backing up the reach below, since a drowned fall is not built.
- Assumption: until WP11 is Done the island's fall at the stream's highest gauge reads as a curtain, and WP6 closes with it so. In Chrome the heavy fall is judged by eye on the curtain and the strand ends alone.
- Assumption: the heavy fall's technique comes from published accounts and one frame study, with no footage studied ([design](DESIGN.md#research-and-selection)). Horizon Forbidden West sets the bar and publishes no technique; the mechanism is Season: A Letter to the Future's (2023) and Star Wars Jedi: Fallen Order's (2019). It gets one build and two rounds of tuning for each cue. If it still fails, main keeps WP6's fall and the owner is asked again whether to park the look.
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

WP6.10's figures are parts 9 and 12's: build the ladder in `thrum-water`'s interaction step. The game names a body in the plunge pool for the views that judge rings, because interaction windows follow the named body.

WP6 resumes from `7bafabb80` on `build/rivers` ([handoff](../../../.orchestrate/evidence/RIVERS/WP6-handoff.md)), where WP6.12 is built. Change nothing more in how the flood draws, because its look below the lip [is WP11's](DESIGN.md#the-heavy-fall-at-the-highest-gauge). What is left is the proof of WP6.4 and WP6.5 as they now read, with the cues the handoff says were not taken again; the rings' arrival at the banks is one. The nine items with a check have open results: run them on the resumed tree and record them. What stands behind the sheet is the selection's scenery and may change with it.

## WP11 With the stream in flood the waterfall reads as heavy falling water from the side and the bank, with no curtain and no strand that ends hard at the pool, drawn the way shipped games draw a heavy fall today

The technique is chosen, a layered fall with spray at its foot: read [the heavy fall](DESIGN.md#the-heavy-fall-at-the-highest-gauge) first.

Stop rule: build the foot first and judge it against WP11.3, then the layers against WP11.2. Then each cue gets two rounds of tuning: one change, then takes from beside and from the bank at the highest gauge. If either cue still fails, merge nothing that changes how the island's fall draws, log which point failed in which take, and end `BLOCKED`. Try no other mechanism.

Before the first change to how the fall is drawn, record the fixture's plunge sources at the highest gauge as WP11.4's test data. WP11.4's cases go in WP6's `falls_gpu` and `plunge_gpu`, which join the package's one test binary, `it`, here ([the settings' rule](../../../ORCHESTRATE.md#test-tiers)).

The flood is the spring's highest gauge: on the island, `spring.stage` at 1 (`THRUM_WATER_RIVER_GAUGE=1` in WP6's captures). `lake.stage` moves another body.

## WP7 A visitor raises the lake, sets the stream's flow and takes the day from dawn to night, and the rivers and the waterfall hold up in motion, in shade and from under water, on the Mac and in Chrome

If `WATERS` already takes the lent Sun visibility on its surface, WP7.3 adds only the sheet's and the heavy fall's. A defect in the reconstructor, and not in what the water writes, goes to `APPLE-PRESENTATION`'s log. WP7's views of the fall are at the middle gauge, except WP7.12's of the heavy fall.
