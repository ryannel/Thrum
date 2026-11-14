# WATERS plan

One water system for every body of water, built here on the sea, the shore and lakes. [Design](DESIGN.md) ·
[done-when file](DONE.json), which owns the criteria · [log](LOG.md) · [module list](../ARCHITECTURE.md)

Before any milestone, read [Proof and builder handoff](DESIGN.md#proof-and-builder-handoff) and
[Exact baselines](../ARCHITECTURE.md#exact-baselines). Their limits hold for every milestone.

## Open for the human

- Assumption: "thinking about water as a whole ... built cohesively" (the owner, 14 November) is met by one design for all water, here, built by four modules in order. `SEA` keeps only the wind's waves, with its M1 and M2 unchanged. This module builds the shared system on the sea, the shore and lakes, from `SEA`'s old M3 to M10. `RIVERS` and `PUDDLES` follow.
- Assumption: still water comes first. The shared system is proved on a sea, a lake and a hand-made channel before `RIVERS`' flow solver is planned, so a river adds data and no second surface, optics, foam or query.
- Assumption: a river's flow is worked out once and stored, not simulated while the game runs. Floods, dam breaks and levels that travel downstream are not planned. This sets aside the water decision's line that all water simulation runs on the GPU; wakes, rings and foam on a river still do.
- Assumption: at a shore waves fade and foam where the water shallows. They do not shoal, turn toward the beach or plunge: surf is not planned.
- Assumption: water reflects the sky, registered bodies and what is on screen. Scenery off screen is not reflected; that needs a probe of the scene round the camera, which no module plans.
- Assumption: one water surface in any column, a camera that stays above the water, and no caustics.
- Assumption: the owner's "it should start from first principles", said of clouds, governs water too: the sea design, the water and port decisions and the older water designs were judged as inputs.
- Assumption: "you can copy that code out and reuse as is if it's useful" is the owner's licence for what Broadsides' authors wrote. Third-party material in it keeps its own licence, and none that is GPL or share-alike is copied.
- Assumption: "bring that across when it makes sense" (13 November) means milestones start when their dependencies are Done, which can be before G7.
- Assumption: every part beyond open water is a capability a scene can leave out at no cost (M10.12).
- Assumption: the three sea cues the 10 November spikes left open stay parked.
- Assumption: "persists after the hull stops or turns" is met inside windows that follow the craft: waves under about 12 m to about 30 m behind it, longer ones to about 250 m, and the foam trail further.
- Assumption: cost is recorded on the Mac only, at 2560×1440 until the showcase's extent exists, and no timing figure decides an item (brief decision 3).
- Assumption: the module closes only after `APPLE-PRESENTATION`'s governor and MetalFX milestones, which are not planned yet. Everything a visitor sees is finished by M10.

## M1 Where water lies is worked out on the CPU alone: a sea, a lake in a hollow and a channel with a current each have a level, a depth and a flow at every point

This creates `thrum-water-model`, with no dependency on the renderer, registered in `.config/architecture.toml`. It uses `thrum-sea-model`, which `SEA` M1 built, for the spectrum's band energies and the wave evaluators; add what it needs there and keep that crate's tests passing. `docs/evidence/WATERS/design-proof/water_proof.py` gives the figures M1.4 and M1.5 are bounded from.

## M2 An open sea in a trade wind draws from the waterline to the horizon under the engine's own sky, and what it costs is on record against the display's refresh

This creates `thrum-water` as one effect on `EFFECT-GRAPH`'s seam and `examples/water` (package `water-scene`) as a game on `thrum-app`'s shell with its own ticking world; `examples/v1-game` is the working model for the shell, the tick page and the capture tap. `thrum-sea`, from `SEA` M2, has the wave field's passes: declare them inside the water's contribution. The surface writes scene depth under [ruling 1](../ARCHITECTURE.md#seam-rulings-for-waters), which this builder adds in `EFFECT-GRAPH`'s files without changing that module's items or tests. Ruling 3 covers what the shell or the tap still assumes of the V1 game; `CREATOR` M2 and M3 edit `thrum-app` too, so integrate on top of what has landed. Only the sea is declared here. This creates `scripts/water_cost.py`; its first record is the baseline later ones are read against.

## M3 The same sea draws in Chrome from one command

`scripts/check_starter_browser.mjs` and `scripts/check_v1_browser.mjs` show how a page is launched and checked; this creates `scripts/check_water_browser.mjs`. A Chrome launch needs a run outside the Codex sandbox.

## M4 A lake lies in a hollow of the hills and the sea meets a beach: each ends where the ground rises, the lake stays calm while the sea runs, and waves fade as the water shallows

The island's ground is the example's content: make it here, cook it as any mesh is, and build the bed from its triangles. Put the lake about 300 m from the sea and well above it, with a second hollow beside it for M4.6. The visible waterline comes from the scene's depth, never from the map's cells. M4.1's bound is measured first and then asserted.

## M5 A float and a small craft ride the waves they sit on, at sea and on the lake, from rest to 15 m/s, and a float in a current is carried by it

The engine side is the query, its ring and its counters. The craft, the floats, their hydrostatics, drag, thrust and rigid-body step are game code in `examples/water`, where Broadsides' hydrostatics may serve.

## M6 The craft leaves a wake that spreads behind it, outlives it and lifts a float it crosses, and a stone dropped in the lake sends rings that slow in the shallows and die at the shore

The seam's state protocol and the two endpoints start here. In `docs/evidence/SEA/design-proof/`, `wake_proof.py` sketches the two banded levels, the border and the Fourier-space source, and part A of `revision_proof.py` the step of several ticks; part H of this module's `water_proof.py` sketches the depths. Engine tests use a generic pressure patch, never the example's hull. The body's own dent is added back in the query; filtering the craft out of the wake is the wrong fix.

## M7 Whitecaps form as the wind rises, the craft leaves a foam trail that drifts, spreads and fades, and foam forms where the sea dies on the beach

The foam pattern is the example's cooked product, lent by a `CONTENT-LOOP` lease; `examples/v1-game/src/lent` is a working lease consumer. Broadsides' ranked pattern may be the source, with its MIT notice. Tune the look against the inspected items with a fresh opinion before review. After three looks that fail the same cue, record them in the log and end `BLOCKED`.

## M8 Water reflects the sky, the Sun, its shores and the craft, shows the ground and hulls beneath it, glitters and dims under a cloud's shade, from dawn to night

The gather reads scene colour and depth under [ruling 2](../ARCHITECTURE.md#seam-rulings-for-waters), added as ruling 1 was. Its conversion to radiance uses `ATMOSPHERE`'s lent aerial lookup and exposure scale as they are; parts C and E of `docs/evidence/SEA/design-proof/revision_proof.py` sketch the receiving rectangles and the conversion. The cloud shade uses a stand-in provider in the example, labelled as one; `examples/v1-game/src/environment.rs` has one to read. Put a tree or a post on the lake's bank for M8.5.

## M9 Water runs down a channel from the lake to the sea: its surface follows the slope, its ripples and foam travel with the current, and a float is carried down to the sea

The channel's chart is written by hand in the example: a level that falls from the lake's to the sea's, a current that quickens in a narrows, and no loss. Nothing here solves a flow. If one mean current a window is visibly wrong for interaction waves in the channel, record it in the log for `RIVERS`' planner and change nothing.

## M10 A visitor takes the water from glassy calm to gale, raises and lowers the lake, and goes from dawn to night with the controls, and it holds up in motion, on the Mac and in Chrome

TAAU is `APPLE-PRESENTATION`'s: a defect in the reconstructor, and not in the water's inputs, goes to that module's log as a proposal. The knobs are declared through `EFFECT-GRAPH`'s list; no governor is needed here.

## M11 On the Mac's Apple recipe the water runs with MetalFX and the governor, its cadence on the built-in display is measured, and the module is closed

Run M11.1's check first. If it fails, end `BLOCKED` with `needs_dependency` and build nothing. M11.6 needs the owner's attended sitting; do every other item first.
