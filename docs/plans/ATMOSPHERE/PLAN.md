# ATMOSPHERE plan

[Design](DESIGN.md) · [Done-when file](DONE.json) · [Log](LOG.md) · [Module list](../ARCHITECTURE.md)

## Open for the owner

- Assumption: WP3 ends with a CPU diagnosis. The authored-spur replay and an in-game correction remain open under G3s.
- Assumption: WP4 compares the reconstructed geometry cases in one bounded CPU trial. The original ray inputs and exact finite-disk match remain open under G3s.
- Assumption: the governor's chart knob (4, 6 and 8 by tier) is the longest a chart refresh may be spread, and the atmosphere chooses each refresh's spread from how fast the Sun, the camera's altitude and the Sun's light are changing. The governor's registry and policy are unchanged. A scrub shortens the spread, to every frame at 1.5° of Sun a frame, at the governor-off cost while it lasts.
- Assumption: a refresh may let the sky's inputs move 0.2° of Sun, 10 m of altitude or 1% of light, and the Sun's value may tighten toward the horizon, to a rebuild every frame through the sharpest twilight. Other modules may rely on every chart being within three tolerances of the frame at the widest values, 0.6° of Sun.
- Assumption: WP12.3 counts the saving as kept when four-frame refreshes cover at least three quarters of the frames of the Mac route's moving segment.
- Assumption: with the governor off, a cloud's reflection under a cloud producer that has stopped writing keeps the old sunlight after the Sun's brightness or colour changes, until the producer next writes. That is today's behaviour, and under the governor it is corrected.
- Assumption: WP12 judges the knob at 4 in the Mac window and at 8 in Chrome, the longest spread at the slower display. Chrome at 4 is not captured.
- Assumption: under WP13 a small thing stops casting a shadow where its thickest part, such as a cactus's trunk, would be under about a texel and a half, and every Sun shadow fades out by 2,048 m from the camera into the caster height map's ground shadows. The other choice is every prop casting to the horizon, which costs more maps or a different technique.
- Assumption: WP13 leaves two faint things out: the soft tip of a small prop's long shadow at a low Sun is trimmed, and a flat sheet the Sun grazes edge-on may shimmer. The showcase has no sheet that casts.
- Assumption: WP13 starts from nine 2048² cascades, about 150 MB of depth against 12 MB today, and the builder tunes count, size and reach by eye and by the cost record.
- Assumption: "mind the cost" is a paired cost record with named ways to cut it, not a budget, for the Sun's shadows (WP13.8) and for occlusion (WP14.6). The Apple rows are full screen at the 67% output scale. `APPLE-PRESENTATION` WP17 owns 120 Hz at full screen and waits for neither. If its closing sitting predates one of them and that record shows the full-screen frame slower, the builder asks the lead for the sitting again. The other choice makes WP17 wait, in `APPLE-PRESENTATION`'s plan.
- Assumption: WP13 is not behind a selection and has no governor knob. A cascade redrawn every few frames would judder under a balloon's shadow at about 20 m/s.
- Assumption: "the default path" means the showcase. The valley and the exit scene show screen-space ambient occlusion in every launch, on the Mac and in Chrome, and an engine scene that does not ask gets none. The other choice turns it on for every 3D scene. The starter scene and the SDK's 3D scenes are left to `CREATOR` and `GAME-SDK`.
- Assumption: the darkening is one frame late, as ray-traced occlusion already is, and follows a thing that arrives or leaves within a few frames. The other choice is a depth prepass, a second draw of the scene every frame.
- Assumption: a thing outside the picture, or hidden behind another, darkens nothing, and a mirror shows no darkening. Ray-traced occlusion has neither limit.
- Assumption: a scene that asks for ambient occlusion is never drawn without it. Where rays cannot run it gets the screen-space method, and over a device's binding limits the scene is refused, as today. The valley in Chrome has one sampled texture to spare. The other choice turns occlusion off there with a reason.

## WP5 Other modules read the one sky and Sun, and one cloud optical depth dims direct sunlight once

## WP9 Haze near sea level deepens evenly to the horizon: it no longer thins just above the horizon or lies in bands across low ground and water, on the Mac and in Chrome

## WP10 A balloon adrift over the valley no longer lays a dark bar across the far ground or blinks one on and off: beyond the Sun's shadow maps, only the ground and what stands on it cast shadow

## WP11 Small Metal probes keep the atmospheric lookup checks fast

## WP6 The valley's sky and sunlight pass the route in motion on Mac and Chrome, and G3s closes

## WP7 Haze thickens from crisp air to murk and takes the colour of its kind under a control, and shafts of sunlight stand in it below ridges and broken cloud, on the Mac and in Chrome

## WP8 A test bank of fog stands in the valley: the land fades into it, sunlight dims inside it, and the camera walks in and out of it, on the Mac and in Chrome

## WP3 A renderer developer can reproduce the shadow sampling failure and assess one filter offline

## WP4 A renderer developer can locate finite-disk shadow errors on reproducible terrain and caster cases

## WP12 The sky's charts are rebuilt over several frames under the governor, so they cost each frame a fraction of what they did and nothing in the sky is seen to update

## WP13 Sun shadows hold still as the Sun moves, start at the foot of what casts them, and thin out with distance with no line where they stop, on the Mac and in Chrome

Read [the Sun's cascades round the camera](DESIGN.md#the-suns-cascades-round-the-camera-wp13) first, then the
module list's [seam ruling](../ARCHITECTURE.md#seam-ruling-for-atmosphere-wp13) and
[baseline ruling](../ARCHITECTURE.md#ruling-on-atmosphere-wp13). Leave `SHOWCASE`'s two thresholded baselines
as they are.

Start from [the diagnosis](../../../.orchestrate/evidence/ATMOSPHERE/WP13-diagnosis/README.md): its frames
are the "before" of each inspection and its `routes/` are the ones the items name. Leave them as they are
and keep any changed copy in WP13's evidence folder. A route's `render_extent` must equal the take's game
grid, so a row that draws another grid needs a copy with only that field changed. WP13.7's Chrome takes come
from `scripts/capture_v1_browser.mjs --route-file`.

WP13.1 and WP13.2 select tests by name: `sun_cascade_layout` and `sun_cascades_gpu`. `WATERS` WP16 builds in
the same Sun files, so whichever lands second carries the other's change. The cost record goes to
`.orchestrate/evidence/ATMOSPHERE/WP13/cost.md` and ends with
[the hand-off to WP17](DESIGN.md#wp13-hand-off-to-wp17). Its full-screen takes force the 67% output scale and
120 Hz by launch setting. Hold the display awake for those takes alone.

## WP14 Things standing in shade look grounded: a soft darkening where they meet the ground, steady while flying, on the Mac and in Chrome

Read [ambient occlusion on the default path](DESIGN.md#ambient-occlusion-on-the-default-path-wp14) first, then
the module list's [seam ruling](../ARCHITECTURE.md#seam-ruling-for-atmosphere-wp14) and
[baseline ruling](../ARCHITECTURE.md#ruling-on-atmosphere-wp14).

Start from `RAY-TRACING` WP5's occlusion in `thrum-render`'s `synthetic/raytrace/ambient*`. Record each file
ported from XeGTAO or Bevy in `docs/provenance/ATMOSPHERE.md`.

WP14.1, WP14.2 and WP14.10 select tests by name: `ambient_occlusion_selection`, `screen_occlusion_gpu` and
`occlusion_launch`. [The design](DESIGN.md#ray-tracings-tests-of-the-old-fallback) says how two assertions of
`RAY-TRACING`'s are restated. List any other you find in the log.

Keep the route copies in `.orchestrate/evidence/ATMOSPHERE/WP14/routes/`: the three with a changed hour,
`rock-dusk.json`, and an off copy of each route an item judges. Run WP14.8's five comparisons with occlusion
off before you regenerate a reference, while the tree still holds the old ones. The cost record goes to
`.orchestrate/evidence/ATMOSPHERE/WP14/cost.md` and ends with
[the hand-off to WP17](DESIGN.md#wp14-hand-off-to-wp17). If `GAME-SPLIT` WP4 has moved the game, the game's
request for occlusion is made where the game lives.
