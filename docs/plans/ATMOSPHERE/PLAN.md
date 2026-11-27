# ATMOSPHERE plan

[Design](DESIGN.md) · [Done-when file](DONE.json) · [Log](LOG.md) · [Module list](../ARCHITECTURE.md)

## Open for the owner

- Assumption: WP3 ends with a CPU diagnosis. The authored-spur replay and an in-game correction remain open under G3s. A later proposal needs the original inputs and the fixed and realtime captures specified in the design.
- Assumption: WP4 compares the reconstructed geometry cases in one bounded CPU trial. The original ray inputs and exact finite-disk match remain open under G3s.
- Assumption: the governor's chart knob (4, 6 and 8 by tier) is the longest a chart refresh may be spread, and the atmosphere chooses each refresh's spread from how fast the Sun, the camera's altitude and the Sun's light are changing. The governor's registry and policy are unchanged. On the game's clock the charts take the knob's full value while the Sun is 10° or more above the horizon, about 7 to 17 solar hours. A scrub shortens the spread, to every frame at 1.5° of Sun a frame, and costs what the charts cost with the governor off while it lasts.
- Assumption: a refresh may let the sky's inputs move 0.2° of Sun, 10 m of altitude or 1% of light. These are the planner's starting values, and the builder tunes them by eye against WP12.4 and WP12.5. The builder may tighten the Sun's value toward the horizon, so the charts may be rebuilt as often as every frame through the sharpest twilight, at the governor-off cost for those minutes, if the picture needs it. Other modules may rely on every chart being within three tolerances of the frame, counted in the widest values, 0.6° of Sun at the starting value, and not in a tightened one.
- Assumption: WP12.3 counts the saving as kept when four-frame refreshes cover at least three quarters of the frames of the Mac route's moving segment. The planner chose the fraction. The first build's fixed spread covered about 94%.
- Assumption: with the governor off, a cloud's reflection under a cloud producer that has stopped writing keeps the old sunlight after a change of the Sun's brightness or colour, until the producer next writes. That is today's behaviour, kept so that no governor-off picture changes. Under the governor it is corrected.
- Assumption: WP12 judges the knob at 4 in the Mac window and at 8 in Chrome, the longest spread at the slower display. Chrome at 4 is not captured.
- Assumption: under WP13 a small thing stops casting a shadow at a distance set by its size, where the shadow would be under about a texel and a half, and every Sun shadow fades out by 2,048 m into the caster height map's ground shadows. The other choice is every prop casting to the horizon, which costs more maps or a different technique.
- Assumption: WP13 starts from nine 2048² cascades, about 150 MB of depth against 12 MB today. These are the planner's starting values, and the builder tunes count, size and reach by eye against WP13.4 to WP13.6 and by the cost record.
- Assumption: "mind the cost" is a paired cost record on the Mac with named ways to cut it (WP13.8), not a budget. The record feeds `APPLE-PRESENTATION` WP17, which owns 120 Hz full screen.
- Assumption: WP13 is not behind a selection and has no governor knob. A cascade redrawn every few frames was rejected: on the game's clock a balloon's shadow crosses the ground at about 20 m/s, so a held map would judder.
- Assumption: "the default path" means the showcase. The valley and the exit scene show screen-space ambient occlusion in every launch, on both of the Mac's recipes and in Chrome, and an engine scene that does not ask gets none, so no other module's tests move. The other choice turns it on for every 3D scene. The starter scene and the SDK's 3D scenes are left to `CREATOR` and `GAME-SDK`.
- Assumption: the darkening is one frame late, as ray-traced occlusion already is, and follows a thing that arrives or leaves within a few frames. The other choice is a depth prepass, a second draw of the scene every frame.
- Assumption: a thing outside the picture, or hidden behind another, darkens nothing, and a mirror shows no darkening. Ray-traced occlusion has neither limit.
- Assumption: for WP14, "the frame budget at 120 Hz full screen in mind" is a paired cost record with named ways to cut it (WP14.6), not a budget. It feeds `APPLE-PRESENTATION` WP17.

## WP5 Other modules read the one sky and Sun, and one cloud optical depth dims direct sunlight once

## WP9 Haze near sea level deepens evenly to the horizon: it no longer thins just above the horizon or lies in bands across low ground and water, on the Mac and in Chrome

## WP10 A balloon adrift over the valley no longer lays a dark bar across the far ground or blinks one on and off: beyond the Sun's shadow maps, only the ground and what stands on it cast shadow

## WP11 Small Metal probes keep the atmospheric lookup checks fast

## WP6 The valley's sky and sunlight pass the route in motion on Mac and Chrome, and G3s closes

## WP7 Haze thickens from crisp air to murk and takes the colour of its kind under a control, and shafts of sunlight stand in it below ridges and broken cloud, on the Mac and in Chrome

## WP8 A test bank of fog stands in the valley: the land fades into it, sunlight dims inside it, and the camera walks in and out of it, on the Mac and in Chrome

Use the [WP7 probe](../../../.orchestrate/evidence/ATMOSPHERE/WP7-review-2.md#suggestion--verified-scheduled-for-m82) as the starting case for WP8.2.

## WP3 A renderer developer can reproduce the shadow sampling failure and assess one filter offline

<a id="m3-sunset-crossing"></a>Start with the [reduced stock reproduction](DESIGN.md#which-inputs-survive).
The [fixed authored-spur screen](DESIGN.md#fixed-witness-and-motion) defines the evidence for a later in-game verdict.

## WP4 A renderer developer can locate finite-disk shadow errors on reproducible terrain and caster cases

Run the [bounded CPU trial](DESIGN.md#wp4-finite-disk-shadow-diagnostic) on the reconstructed cases.

## WP12 The sky's charts are rebuilt over several frames under the governor, so they cost each frame a fraction of what they did and nothing in the sky is seen to update

## WP13 Sun shadows hold still as the Sun moves, start at the foot of what casts them, and thin out with distance with no line where they stop, on the Mac and in Chrome

Read [the Sun's cascades round the camera](DESIGN.md#the-suns-cascades-round-the-camera-wp13) first, then the
module list's [seam ruling](../ARCHITECTURE.md#seam-ruling-for-atmosphere-wp13) for the files of other modules
this work package builds in, and its [baseline ruling](../ARCHITECTURE.md#ruling-on-atmosphere-wp13) for the
references it refreshes in its own landing. Leave `SHOWCASE`'s two thresholded baselines as they are.

Start from [the diagnosis](../../../.orchestrate/evidence/ATMOSPHERE/WP13-diagnosis/README.md): its frames
are the "before" of each inspection, its capture line reproduces them, and its `routes/` are the ones the
items name. Leave those as they are and keep any changed copy in WP13's own evidence folder. A route's
`render_extent` must equal the take's game grid, 1280 × 720 as written, so a row that draws another grid
needs a copy with only that field changed.
The Chrome takes of WP13.7 use `scripts/capture_v1_browser.mjs --route-file` and the Chrome arm of
`scripts/run_v1_showcase.py moving`.

The checks of WP13.1 and WP13.2 select tests by name: `sun_cascade_layout` for the CPU rules and
`sun_cascades_gpu` for the Metal tests. `WATERS` WP16 builds in the same Sun files, so whichever lands
second carries the other's change. The cost record goes to `.orchestrate/evidence/ATMOSPHERE/WP13/cost.md`.

## WP14 Things standing in shade look grounded: a soft darkening where they meet the ground, steady while flying, on the Mac and in Chrome

Read [ambient occlusion on the default path](DESIGN.md#ambient-occlusion-on-the-default-path-wp14) first, then
the module list's [seam ruling](../ARCHITECTURE.md#seam-ruling-for-atmosphere-wp14) and
[baseline ruling](../ARCHITECTURE.md#ruling-on-atmosphere-wp14).

Start from `RAY-TRACING` WP5's occlusion in `thrum-render`'s `synthetic/raytrace/ambient*`, whose history, read
and application the screen-space method shares. Record each file ported from XeGTAO or Bevy in
`docs/provenance/ATMOSPHERE.md`. WP14.1 and WP14.2 select tests by name: `ambient_occlusion_selection` and
`screen_occlusion_gpu`. Keep the three route copies in `.orchestrate/evidence/ATMOSPHERE/WP14/routes/` and the
cost record at `.orchestrate/evidence/ATMOSPHERE/WP14/cost.md`. If `GAME-SPLIT` WP4 has moved the game, the
game's own request for occlusion is made where the game lives.
