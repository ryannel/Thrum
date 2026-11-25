# WATERS plan

[Design](DESIGN.md), [done-when file](DONE.json), [log](LOG.md), [module list](../ARCHITECTURE.md)

First read [Proof and builder handoff](DESIGN.md#proof-and-builder-handoff) and
[Exact baselines](../ARCHITECTURE.md#exact-baselines). Build in order.

## Open for the human

- Assumption: a fast test aims for about 2 s, run alone on a quiet Mac.
- Assumption: "thinking about water as a whole ... built cohesively" (14 November) is met by one design for all water, built by `SEA`, this module, `RIVERS` and `PUDDLES`.
- Assumption: a river's flow is worked out once and stored, with no floods, dam breaks or travelling levels, which sets aside the water decision's rule that all water simulation runs on the GPU.
- Assumption: "breaking waves" (14 November) is met by the sea's own swell, turned, shortened and raised over the bed by wave theory and shaped as a breaker as each crest passes its breaking height, then by a bounded swash profile it drives at the still-water contour: neither baked nor a live solver, with no spray, splash, ride inside a barrel or current from surf.
- Assumption: the island's exposed beach gets a swell set for the look: several lines at once, and a period inside the surf cue's 6 s to 14 s.
- Assumption: from 1 km the surf is judged for fronts starting at different distances and running in; that a set's taller crest breaks farther out is held on the model (WP12.4), not by eye: a known gap.
- Assumption: the pitching face's finish seen close (tabs and strands under the lip, bands on its white, blunt crest ends, the bend's gray patch) is not foundational: WP13 passed with it as a known gap, and WP20 names and repairs each cause before WP10's motion review.
- Assumption: wet long-wave loss keeps WP7.5's distributed foam wherever no breaking face is available, swash included; an eligible crest concentrates part on its face and under a resolved lip; distant beaches take steady cover from the same loss.
- Assumption: as part of surf an island shelters its lee: a lee beach laps with short waves only.
- Assumption: the surf query's age is proved for the example's eight floats in a gale on the Mac (nine answers in ten within two ticks), not for 2,048 points in breakers or in Chrome; the coarse-cell rule is measured, not proved.
- Assumption: delivery, not a view's time, decides two-tick answers; no second path past the view is built; through a tick with no new answer a float rides its last one, to the maximum age.
- Assumption: surf on a lake is proved against the model, not by eye: the example's lake raises no long waves.
- Assumption: "scenery reflections" is met by drawing the scene again from the camera's mirror image for the two water levels with most water in view; a third reflects only what is on screen.
- Assumption: under water there are no lens drops, particles or bubbles, and the picture darkens with depth by a quarter of the light lost. One water surface in any column still holds.
- Assumption: only WP14 and the close wait for `SCENE-VIEWS`.
- Assumption: the owner's "it should start from first principles", said of clouds, governs water too, and "bring that across when it makes sense" (13 November) lets work packages start once their dependencies are Done, even before G7.
- Assumption: "you can copy that code out and reuse as is if it's useful" licenses what Broadsides' authors wrote; third-party material keeps its own licence, and nothing GPL or share-alike is copied.
- Assumption: every part beyond open water (surf, the mirror, the underwater view) is a capability a scene can leave out at no cost.
- Assumption: the three sea cues the 10 November spikes left open stay parked.
- Assumption: "persists after the hull stops or turns" is met inside windows that follow the craft: short waves to about 30 m behind, longer ones to about 250 m.
- Assumption: cost is recorded on the Mac only, at 2560×1440 until the showcase's extent exists; no timing figure decides an item (brief decision 3).
- Assumption: the module closes only after `APPLE-PRESENTATION`'s governor and MetalFX work packages.
- Assumption: under the owner's bar of 17 November, tests cover what a game reads, the frame contract and what one pass hands another; wakes, foam and underwater light are judged by eye.

## WP1 Where water lies is worked out on the CPU alone: a sea, a lake in a hollow and a channel with a current each have a level, a depth and a flow at every point

[Review](../../../.orchestrate/evidence/WATERS/WP1-review.md)

## WP12 On the CPU alone, a swell turns to face a beach, closes up, rises and breaks where the depth says, and an island calms its lee

[Review](../../../.orchestrate/evidence/WATERS/WP12-review.md)

## WP2 An open sea in a trade wind draws from the waterline to the horizon under the engine's own sky, and what it costs is on record against the display's refresh

[Review](../../../.orchestrate/evidence/WATERS/WP2-review.md)

## WP3 The same sea draws in Chrome from one command

[Review](../../../.orchestrate/evidence/WATERS/WP3-review.md)

## WP4 A lake lies in a hollow of the hills and the sea meets a beach: each ends where the ground rises, the lake stays calm while the sea runs, and waves fade as the water shallows and run the waterline up and down the sand

[Review](../../../.orchestrate/evidence/WATERS/WP4-review.md)

## WP5 A float and a small craft ride the waves they sit on, at sea and on the lake, from rest to 15 m/s, and a float in a current is carried by it

[Review](../../../.orchestrate/evidence/WATERS/WP5-review.md)

## WP6 The craft leaves a wake that spreads behind it, outlives it and lifts a float it crosses, and a stone dropped in the lake sends rings that slow in the shallows and die at the shore

[Review](../../../.orchestrate/evidence/WATERS/WP6-review.md)

## WP7 Whitecaps form as the wind rises, the craft leaves a foam trail that drifts, spreads and fades, and foam forms where the sea dies on the beach

[Review](../../../.orchestrate/evidence/WATERS/WP7-review.md)

## WP18 The submitted craft trail is checked through a minute of ageing in the fast suite

[Review](../../../.orchestrate/evidence/WATERS/WP18-review.md)

## WP17 Small water checks leave the full sea grids to the long tier

[Review](../../../.orchestrate/evidence/WATERS/WP17-review.md)

## WP13 Swell turns to face the beach, rises and breaks into white water that runs up the sand, on the Mac and in Chrome

[Review](../../../.orchestrate/evidence/WATERS/WP13-review.md)

## WP22 The water scene's tests compile for Linux and Windows again

`build/waters`' unmerged commits merge with this.

## WP21 With surf in view, a float gets the sea's height within two ticks and rides a hitch on its last answer

[Measure before changing the pass](DESIGN.md#a-surf-view-a-running-game-can-afford-wp21), and take WP21.6's reference stills first.

## WP19 Floats in a gale's breaking surf get the height of the crest they ride in time to ride it, with the game running

Resume from the [handoff](../../../.orchestrate/evidence/WATERS/WP19-handoff.md).
[After WP21](DESIGN.md#a-surf-query-a-running-game-can-use-wp19) says what a failing control means.

## WP8 Water reflects the sky, the Sun, its shores and the craft, shows the ground and hulls beneath it, glitters and dims under a cloud's shade, from dawn to night

## WP9 Water runs down a channel from the lake to the sea: its surface follows the slope, its ripples and foam travel with the current, and a float is carried down to the sea

If the window's mean current makes interaction waves visibly wrong in the example channel, log it for the `RIVERS` planner.

## WP15 The camera goes under water: the waterline crosses the lens, and the surface overhead shows the sky through a bright window

## WP16 Under water the Sun casts a moving net of light on the bed and the hull, shafts of light stand in the water, and the picture stays readable as the camera goes down

## WP20 A pitching crest seen close is drawn whole, with no tabs, strands, bands or flashing patch on its face, on the Mac and in Chrome

Begin with WP20.1 and WP20.4. [WP13's reading](LOG.md#wp13-face-finish-parked) is a lead that read no depth,
bend, crest end or Chrome. Repair only a stage a diagnosis names ([risk 18](DESIGN.md#risks)), on the
target and in the wind that show the cue. Without one, or after three failed looks at a repair, end `BLOCKED`.

## WP10 A visitor takes the water from glassy calm to gale, raises and lowers the lake, and goes from dawn to night with the controls, and it holds up in motion, on the Mac and in Chrome

A defect in `APPLE-PRESENTATION`'s reconstructor, not the water's inputs, goes to that module's log.

## WP14 Still water mirrors the hills, trees and far shore that the camera cannot see, on the Mac and in Chrome

Ask `SCENE-VIEWS` through [its interface](../SCENE-VIEWS/DESIGN.md#what-the-consumer-reads), and log what it lacks there as a proposal.

## WP11 On the Mac's Apple recipe the water runs with MetalFX and the governor, its cadence on the built-in display is measured, and the module is closed

Run WP11.1's check first; if it fails, end `BLOCKED needs_dependency`. WP11.6 needs the owner's attended sitting; do every other item first.
