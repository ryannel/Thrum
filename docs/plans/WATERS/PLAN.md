# WATERS plan

[Design](DESIGN.md) · [done-when file](DONE.json) · [log](LOG.md) · [module list](../ARCHITECTURE.md)

Before any milestone, read [Proof and builder handoff](DESIGN.md#proof-and-builder-handoff) and
[Exact baselines](../ARCHITECTURE.md#exact-baselines). Milestones are built in the order below.

## Open for the human

- Assumption: aim for about 2 s per fast test when run alone on a quiet Mac. The owner can revise this target.
- Assumption: "thinking about water as a whole ... built cohesively" (14 November) is met by one design for all water, built by four modules: `SEA` (the wind's waves), this module (the shared system on the sea, the shore and lakes), then `RIVERS` and `PUDDLES`.
- Assumption: a river's flow is worked out once and stored. Floods, dam breaks and levels that travel downstream are not planned, which sets aside the water decision's rule that all water simulation runs on the GPU.
- Assumption: "breaking waves" (14 November) is met by the sea's own swell, which wave theory turns, shortens and raises over the bed, and which takes a breaker's shape as each crest passes its breaking height. It is neither baked nor a live solver. There is no spray or splash, no ride inside a barrel, and no current from surf.
- Assumption: after the break, a bounded swash profile driven by the incoming swell at the still-water contour is sufficient for the shipped-game look. The same sheet lowers the bore, limits the coherent trough and preserves the regular-wave run-up and draw-back. The builder tunes the bore decay and handoff width by eye.
- Assumption: wet long-wave loss keeps M7.5's distributed foam wherever a breaking face is unavailable, including the swash. An eligible crest concentrates part of that birth on its face and under a resolved lip. Distant beaches use the same wet lost-power source for their steady cover.
- Assumption: as part of surf an island shelters its lee, so a lee beach laps with short waves only.
- Assumption: surf on a lake is proved against the model, not judged by eye: the example's lake raises no long waves.
- Assumption: "scenery reflections" is met by drawing the scene again from the camera's mirror image, for the two water levels with the most water in view; a third reflects only what is on screen.
- Assumption: the view from under water has no drops on the lens, particles or bubbles, and its picture darkens with depth by a quarter of the light lost. One water surface in any column still holds.
- Assumption: only M14 and the close wait for `SCENE-VIEWS`: surf's model follows M1, surf follows foam, the view from under water follows the channel, then its light.
- Assumption: the owner's "it should start from first principles", said of clouds, governs water too, and "bring that across when it makes sense" (13 November) means milestones start when their dependencies are Done, which can be before G7.
- Assumption: "you can copy that code out and reuse as is if it's useful" is the owner's licence for what Broadsides' authors wrote; third-party material keeps its own, and nothing GPL or share-alike is copied.
- Assumption: every part beyond open water (surf, the mirror, the view from under water) is a capability a scene can leave out at no cost.
- Assumption: the three sea cues the 10 November spikes left open stay parked.
- Assumption: "persists after the hull stops or turns" is met inside windows that follow the craft: short waves to about 30 m behind it, longer ones to about 250 m.
- Assumption: cost is recorded on the Mac only, at 2560×1440 until the showcase's extent exists, and no timing figure decides an item (brief decision 3).
- Assumption: the module closes only after `APPLE-PRESENTATION`'s governor and MetalFX milestones.
- Assumption: under the owner's bar of 17 November, tests stay on what a game reads, the frame contract and what one pass hands another; the look of wakes, foam and underwater light is judged by eye.

## M1 Where water lies is worked out on the CPU alone: a sea, a lake in a hollow and a channel with a current each have a level, a depth and a flow at every point

[Review](../../../.orchestrate/evidence/WATERS/M1-review.md)

## M12 On the CPU alone, a swell turns to face a beach, closes up, rises and breaks where the depth says, and an island calms its lee

[Review](../../../.orchestrate/evidence/WATERS/M12-review.md)

## M2 An open sea in a trade wind draws from the waterline to the horizon under the engine's own sky, and what it costs is on record against the display's refresh

[Review](../../../.orchestrate/evidence/WATERS/M2-review.md)

## M3 The same sea draws in Chrome from one command

[Review](../../../.orchestrate/evidence/WATERS/M3-review.md)

## M4 A lake lies in a hollow of the hills and the sea meets a beach: each ends where the ground rises, the lake stays calm while the sea runs, and waves fade as the water shallows and run the waterline up and down the sand

[Review](../../../.orchestrate/evidence/WATERS/M4-review.md)

## M5 A float and a small craft ride the waves they sit on, at sea and on the lake, from rest to 15 m/s, and a float in a current is carried by it

[Review](../../../.orchestrate/evidence/WATERS/M5-review.md)

## M6 The craft leaves a wake that spreads behind it, outlives it and lifts a float it crosses, and a stone dropped in the lake sends rings that slow in the shallows and die at the shore

[Review](../../../.orchestrate/evidence/WATERS/M6-review.md)

## M7 Whitecaps form as the wind rises, the craft leaves a foam trail that drifts, spreads and fades, and foam forms where the sea dies on the beach

[Review](../../../.orchestrate/evidence/WATERS/M7-review.md)

## M18 The submitted craft trail is checked through a minute of ageing in the fast suite

Land the [compact Metal trail fixture](DESIGN.md#compact-metal-craft-trail-m18) ahead of unfinished
water work. Adapt the trail-test portion of `52345b514` from `build/waters` to the current main
fixture. Record the default Nextest time on the Mac, excluding compilation, for the exact test alone
and within the foam binary under normal builder load. Both runs pass on the first attempt under the
existing ten-second limit. M13.14 continues to verify the guard when the surf work lands.

## M17 Small water checks leave the full sea grids to the long tier

Land the fixture reductions first if a long-tier entry would hold them behind failures in other
packages. WATERS owns the sea field and cache regressions.

Use the [fixture design](DESIGN.md#water-fixtures-and-full-sea-sweeps-m17) for the cases in the
[workspace case list and baseline times](LOG.md#m17-workspace-timing-baseline).

Record each case’s disposition, any replacement coverage and its before-and-after time in the workspace
check’s default Nextest build on the Mac, excluding compilation. Measure the fast cases alone and in a
focused batch under normal builder load. The batch passes the existing ten-second limit on its first run.

## M13 Swell turns to face the beach, rises and breaks into white water that runs up the sand, on the Mac and in Chrome

Resume `build/waters` from the handoff at `.claude/worktrees/waters/.orchestrate/evidence/WATERS/M13-handoff.md` in the main checkout.

## M8 Water reflects the sky, the Sun, its shores and the craft, shows the ground and hulls beneath it, glitters and dims under a cloud's shade, from dawn to night

## M9 Water runs down a channel from the lake to the sea: its surface follows the slope, its ripples and foam travel with the current, and a float is carried down to the sea

If the window's mean current makes interaction waves visibly wrong in the example channel, log the case for the `RIVERS` planner.

## M15 The camera goes under water: the waterline crosses the lens, and the surface overhead shows the sky through a bright window

## M16 Under water the Sun casts a moving net of light on the bed and the hull, shafts of light stand in the water, and the picture stays readable as the camera goes down

## M10 A visitor takes the water from glassy calm to gale, raises and lowers the lake, and goes from dawn to night with the controls, and it holds up in motion, on the Mac and in Chrome

A defect in `APPLE-PRESENTATION`'s reconstructor, not the water's inputs, goes to that module's log.

## M14 Still water mirrors the hills, trees and far shore that the camera cannot see, on the Mac and in Chrome

Ask `SCENE-VIEWS` through [its interface](../SCENE-VIEWS/DESIGN.md#what-the-consumer-reads), and log what it lacks there as a proposal.

## M11 On the Mac's Apple recipe the water runs with MetalFX and the governor, its cadence on the built-in display is measured, and the module is closed

Run M11.1's check first; if it fails, end `BLOCKED` with `needs_dependency`. M11.6 needs the owner's attended sitting; do every other item first.
