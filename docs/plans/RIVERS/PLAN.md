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

Build this repair first. It can land independently of the island work in WP4, which waits for WATERS WP9.

Use the [fixture design](DESIGN.md#compact-river-solver-fixtures-wp8) for the cases in the
[workspace case list and baseline times](LOG.md#m8-workspace-timing-baseline).

Record each case’s disposition, any replacement coverage and its before-and-after time in the workspace
check’s default Nextest build on the Mac, excluding compilation. Measure the fast cases alone and in a
focused batch under normal builder load. The batch passes the existing ten-second limit on its first run.

## WP9 Chart gauges and solver determinism run within the fast-suite limit

Land this repair before WP4. Use the [compact fixtures](DESIGN.md#compact-chart-gauge-and-determinism-fixtures-wp9)
for the two tests that timed out on their isolated retry. Record the work each fixture does and the test
times with the default Nextest profile, excluding compilation, alone and together under normal builder load.
Both pass on the first attempt under the existing ten-second limit.

## WP10 The check that a river file keeps each sample's own gains runs on a small reach

Build this repair first. It lands independently of WP4. Use the
[fixture design](DESIGN.md#compact-river-file-gains-fixture-wp10). Record the test's CPU time and its time
alone, before and after, in the workspace check's default Nextest build on the Mac, excluding compilation;
`/usr/bin/time -p` on the test binary with `--exact` gives both. Change no other test.

## WP4 On the island a stream runs from a spring through a pool and a narrows to the lake, and a river runs from the lake to the sea; raise the lake and the river swells, and a float rides from the spring to the sea

The island's ground, its channel's cut and `scripts/water_cost.py` and `scripts/check_water_browser.mjs` are `WATERS`'; add the valley, cases and scenes behind the selection and change none of theirs. River files are generated beside the example's cooked content and are not committed. The fall is in the map from here and is drawn in WP6.

WP4.3 was re-planned after its fifth review. What is new to build is the bank camera's readings, the two 7 s series and the views of the floats from above; [the design](DESIGN.md#proof-and-builder-handoff) says why. Nothing asks for a change to the surface's oblique search unless the bank bound or the inspection fails. If the longer series take `river_gpu` past the merge tier's limit, they run in the main tier, because only a whole swell period shows what one frame cannot.

## WP5 On a windless day the stream still reads as running water: its riffles are rough and glitter, its pools lie glassy, and foam forms below its ledges and behind its rocks and trails away downstream

The flow bands' passes join the cascades' in `thrum-sea`, which is `WATERS`' to extend by now. `c` and the birth constant are the two values to tune against WP5.4 and WP5.5; record both. WP5.10's expected answer comes from the band evaluator WP3 left in `thrum-water-model` (`flow.rs`), and its tolerance from the query test `WATERS` WP5 wrote. The query dispatch and the cascades' cadence are `WATERS`'; [a query's demand](DESIGN.md#the-flows-own-waves) adds only when the flow bands' own producer runs, and WP5.6 tests it.

## WP6 A waterfall pours over its lip in a smooth tongue, breaks into white strands as it gathers speed and plunges into its pool, which boils, foams and sends out rings

WP6.10's figures are parts 9 and 12's: build the ladder in `thrum-water`'s interaction step. No f64 reference of it is required. The wave share is tuned by eye against WP6.5 and recorded, and nothing asserts the fall's power as its ceiling. The game names a body in the plunge pool for the views that judge rings, because interaction windows follow the named body.

## WP7 A visitor raises the lake, sets the stream's flow and takes the day from dawn to night, and the rivers and the waterfall hold up in motion, in shade and from under water, on the Mac and in Chrome

If `WATERS` already takes the lent Sun visibility on its surface, WP7.3 adds only the sheet's. A defect in the reconstructor, and not in what the water writes, goes to `APPLE-PRESENTATION`'s log.
