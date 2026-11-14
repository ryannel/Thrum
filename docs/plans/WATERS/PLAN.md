# WATERS plan

One water system for every body of water, built here on the sea, the shore and lakes. [Design](DESIGN.md) ·
[done-when file](DONE.json) · [log](LOG.md) · [module list](../ARCHITECTURE.md)

Before any milestone, read [Proof and builder handoff](DESIGN.md#proof-and-builder-handoff) and
[Exact baselines](../ARCHITECTURE.md#exact-baselines). Milestones are built in the order below, not in number order.

## Open for the human

- Assumption: "thinking about water as a whole ... built cohesively" (the owner, 14 November) is met by one design for all water, built by four modules: `SEA` (the wind's waves), this module (the shared system on the sea, the shore and lakes), then `RIVERS` and `PUDDLES`.
- Assumption: a river's flow is worked out once and stored. Floods, dam breaks and levels that travel downstream are not planned, which sets aside the water decision's rule that all water simulation runs on the GPU.
- Assumption: "breaking waves" (the owner, 14 November) is met by the sea's own swell, which wave theory turns, shortens and raises over the bed, and which takes a breaker's shape as each crest passes its breaking height. It is neither baked nor a live solver. There is no spray or splash, no ride inside a barrel, and no current from surf.
- Assumption: as part of surf an island shelters its lee from the swell, so a lee beach laps with short waves only.
- Assumption: "scenery reflections" is met by drawing the scene again from the camera's mirror image, for the two water levels with the most water in view; a third reflects only what is on screen.
- Assumption: the view from under water has no drops on the lens, particles or bubbles, and its picture darkens with depth by a quarter of the light lost. One water surface in any column still holds.
- Assumption: only M14 and the close wait for `SCENE-VIEWS`: surf's model follows M1, surf follows foam, the view from under water follows the channel, then its light.
- Assumption: the owner's "it should start from first principles", said of clouds, governs water too, and "bring that across when it makes sense" (13 November) means milestones start when their dependencies are Done, which can be before G7.
- Assumption: "you can copy that code out and reuse as is if it's useful" is the owner's licence for what Broadsides' authors wrote; third-party material keeps its own, and nothing GPL or share-alike is copied.
- Assumption: every part beyond open water (surf, the mirror, the view from under water) is a capability a scene can leave out at no cost.
- Assumption: the three sea cues the 10 November spikes left open stay parked.
- Assumption: "persists after the hull stops or turns" is met inside windows that follow the craft: short waves to about 30 m behind it, longer ones to about 250 m.
- Assumption: cost is recorded on the Mac only, at 2560×1440 until the showcase's extent exists, and no timing figure decides an item (brief decision 3).
- Assumption: the module closes only after `APPLE-PRESENTATION`'s unplanned governor and MetalFX milestones.

## M1 Where water lies is worked out on the CPU alone: a sea, a lake in a hollow and a channel with a current each have a level, a depth and a flow at every point

This creates `thrum-water-model`, registered in `.config/architecture.toml`, on `thrum-sea-model` from `SEA` M1: add what it needs there and keep that crate's tests passing. Parts I, K and L of `docs/evidence/WATERS/design-proof/water_proof.py` give M1.4's figures, the joins with M1.12's shallow ends, and the swash's closed form.

## M12 On the CPU alone, a swell turns to face a beach, closes up, rises and breaks where the depth says, and an island calms its lee

Surf is a capability: without it M1's answers and tests stay as they are. Parts N and O of `docs/evidence/WATERS/design-proof/surf_view_proof.py` sketch the solve, the blends, the exposure, the break point and a section that meets M12.5.

## M2 An open sea in a trade wind draws from the waterline to the horizon under the engine's own sky, and what it costs is on record against the display's refresh

This creates `thrum-water` as one effect on `EFFECT-GRAPH`'s seam, `examples/water` (package `water-scene`) as a game on `thrum-app`'s shell with its own ticking world, and `scripts/water_cost.py`. `examples/v1-game` is the working model. [Rulings 1 and 3](../ARCHITECTURE.md#seam-rulings-for-waters) are built here without changing the owning modules' items or tests; `CREATOR` M2 and M3 edit `thrum-app` too, so integrate on top of what has landed.

## M3 The same sea draws in Chrome from one command

`scripts/check_v1_browser.mjs` shows how a page is launched and checked; this creates `scripts/check_water_browser.mjs`.

## M4 A lake lies in a hollow of the hills and the sea meets a beach: each ends where the ground rises, the lake stays calm while the sea runs, and waves fade as the water shallows and run the waterline up and down the sand

Make the island's ground here as the example's content, and build the bed from its triangles: a lake about 300 m from the sea and well above it, under a ridge, with a second hollow beside it for M4.6 and a rim over its highest stage; a beach of about 1 in 10 facing the trade wind's swell; a lee shore and a headland, which M13 and M14 use. The profile has no surf until M13. M4.1's bound is measured first and then asserted.

## M5 A float and a small craft ride the waves they sit on, at sea and on the lake, from rest to 15 m/s, and a float in a current is carried by it

## M6 The craft leaves a wake that spreads behind it, outlives it and lifts a float it crosses, and a stone dropped in the lake sends rings that slow in the shallows and die at the shore

The seam's state protocol and the two endpoints start here.

## M7 Whitecaps form as the wind rises, the craft leaves a foam trail that drifts, spreads and fades, and foam forms where the sea dies on the beach

The foam pattern is the example's cooked product, lent by a `CONTENT-LOOP` lease as in `examples/v1-game/src/lent`; Broadsides' ranked pattern may be its source, with its MIT notice. The film is drawn by the water's own pass.

## M13 Swell turns to face the beach, rises and breaks into white water that runs up the sand, on the Mac and in Chrome

Both sea crates are this module's to extend now: the field cache gains the transform and the section, and `SEA`'s tests keep passing. Turn surf on in the example's profile. M13.1's bounds are measured first and then asserted.

## M8 Water reflects the sky, the Sun, its shores and the craft, shows the ground and hulls beneath it, glitters and dims under a cloud's shade, from dawn to night

The gather reads scene colour and depth under [ruling 2](../ARCHITECTURE.md#seam-rulings-for-waters). The cloud shade is a labelled stand-in provider, as in `examples/v1-game/src/environment.rs`. Put a tree on the lake's bank.

## M9 Water runs down a channel from the lake to the sea: its surface follows the slope, its ripples and foam travel with the current, and a float is carried down to the sea

The channel's chart is written by hand in the example: a level that falls from the lake's to below the lowest tide, a current that quickens in a narrows, a source end, and a mouth shallow enough at low tide for the depth limit to hold the swell. Cut the lake's outlet through ground above its highest stage, or preparation refuses the chart. If one mean current a window is visibly wrong for interaction waves there, log it for `RIVERS`' planner and change nothing.

## M15 The camera goes under water: the waterline crosses the lens, and the surface overhead shows the sky through a bright window

## M16 Under water the Sun casts a moving net of light on the bed and the hull, shafts of light stand in the water, and the picture stays readable as the camera goes down

[Rulings 5 to 7](../ARCHITECTURE.md#seam-rulings-for-waters) are built here, in `ATMOSPHERE`'s and `APPLE-PRESENTATION`'s files.

## M10 A visitor takes the water from glassy calm to gale, raises and lowers the lake, and goes from dawn to night with the controls, and it holds up in motion, on the Mac and in Chrome

A defect in `APPLE-PRESENTATION`'s reconstructor, not the water's inputs, goes to that module's log.

## M14 Still water mirrors the hills, trees and far shore that the camera cannot see, on the Mac and in Chrome

Ask `SCENE-VIEWS` through [its interface](../SCENE-VIEWS/DESIGN.md#what-the-consumer-reads), and log what it lacks there as a proposal.

## M11 On the Mac's Apple recipe the water runs with MetalFX and the governor, its cadence on the built-in display is measured, and the module is closed

Run M11.1's check first; if it fails, end `BLOCKED` with `needs_dependency`. M11.6 needs the owner's attended sitting; do every other item first.
