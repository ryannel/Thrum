# ATMOSPHERE plan

Closes gate G3s at WP6. WP7 and WP8 are track 2. Work packages run in the order below.
[Design](DESIGN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
[module list](../ARCHITECTURE.md)

## Open for the owner

- Assumption: aim for about 2 s per fast test when run alone on a quiet Mac. The owner can revise this target.
- Assumption: "the modern technique" (the lead's task, 14 November) is the camera-aligned aerial volume current engines ship, corrected so that the air goes on under the model's ground. Marching the air for every pixel, Unreal's costlier option, is not built.
- Assumption: the corrected air is the aerial product of every scene, with no switch. The showcase valley's far low ground gains a little haze, so WP9 refreshes `APPLE-TILE`'s five exact reference sets once, in its own landing. The other choice keeps today's output as the default behind a selection, and leaves the defect in every scene that does not ask.
- Assumption: WP9 is built now, ahead of WP6 and TAAU, and is held by numbers and by where the retained images moved. Its look on a real sea is judged in `WATERS` WP2, which now waits for it.
- Assumption: G3s closes with the west-lookout crossing (WP3) still parked and named as a known defect, and with WP4 deferred; neither is judged in WP6.
- Assumption: the sky's closing review waits for TAAU, so it is judged once on the image the exit will show.
- Assumption: solar sky radiance and solar aerial transport stay unqualified against an all-orders reference, and under the owner's bar of 17 November no accuracy campaign is planned. The referees P1–P11, and the five comparisons in H1–H4 that repeat one through the lent library, stay as regression guards, which a deliberate change to the look gives new expected values. The handoff's contract comparisons, which read the stored product, stay fixed. H5's reflection-model differences are only recorded.
- Assumption: the lent sky and aerial lookups stay one hardware-filtered read, and their handoff bound allows one stored half-float step, because Metal returns filtered half-float reads at that precision. Blending exact texels in the shader for float32 results was rejected: it costs every sky pixel and lit fragment more reads and changes every scene's image, and WP6 judges by eye whether a step shows.
- Assumption: "top quality atmosphere" (the owner, 14 November) gives this module two work packages after G3s closes, WP7 and WP8: haze the weather sets, shafts of light, and fog near the camera, integrated jointly with the air in one view volume. They gate nothing in the V1 exit, and the exit scene does not declare the view volume.
- Assumption: WP7 and WP8 wait for WP6, so fog's first frame waits for TAAU and for G3s to close.
- Assumption: lamps and other local lights do not light haze or fog; the Sun, the sky and the ground's light do.
- Assumption: the weather sets the haze, so `ATMOSPHERE`'s fixed air (a visual range of about 244 km) becomes an input with that value as its default. Every scene that sets nothing draws as before.
- Assumption: the haze's kind is one Ångström exponent, defaulting to 0, today's grey aerosol; its single-scattering albedo and phase stay fixed. The check of 17 November (`.orchestrate/evidence/ATMOSPHERE/haze-kind-check.md`) found the exponent's colour visible at a low Sun and small at noon. Which exponent a weather sets, and whether moisture drives it, is the game's.
- Assumption: with its false strip gone (WP10), a balloon casts no shadow on ground beyond the Sun's shadow maps, about 512 m from the camera in the exit scene. The other choices are a longer reach for that scene, which `SHOWCASE` may set and its WP8 judges, or a new far-shadow method for things in the air, which is a re-plan with its own cost.
- Assumption: the terrain lip on the shipped Ground E pan is judged in WP6.1 as a brightness step, and the held 17.4 h stop is judged; only the moving spur edge of the parked WP3 crossing is left out. `SHOWCASE` WP9 fixes the lip and the sunrise stop in the valley's content, and WP6 waits for it.

## WP1 The valley remains legible at moonless midnight: done
[record and reviews](../../../.orchestrate/evidence/ATMOSPHERE/WP1-M4-records.md#m1-the-valley-remains-legible-at-moonless-midnight-done)

## WP2 Stars emerge as the observer turns and time advances: done
[record and reviews](../../../.orchestrate/evidence/ATMOSPHERE/WP1-M4-records.md#m2-stars-emerge-as-the-observer-turns-and-time-advances-done)

## WP3 The west lookout’s shadow crosses the ground without a flash

<a id="m3-sunset-crossing"></a>

A parked spike, outside the done-when file: the stock graph ships and WP6 does not judge the moving spur edge.
[The record](../../../.orchestrate/evidence/ATMOSPHERE/WP1-M4-records.md#m3-the-west-lookouts-shadow-crosses-the-ground-without-a-flash-parked-spike)
holds its criteria D1 to D3, the proposed reference clip, the diagnosis handoff, the finished work and the work
left. It re-enters only as a new commission against those criteria.

## WP4 Exact finite-disk shadow reference match

A deferred bounded spike, outside the done-when file. WP3 and the other V1 gates proceed without it.
[The record](../../../.orchestrate/evidence/ATMOSPHERE/WP1-M4-records.md#m4-exact-finite-disk-shadow-reference-match-deferred-bounded-spike)
holds its criteria, the finished work and the work left.

## WP5 Other modules read the one sky and Sun, and one cloud optical depth dims direct sunlight once: done
[review](../../../.orchestrate/evidence/ATMOSPHERE/WP5-review.md)

## WP9 Haze near sea level deepens evenly to the horizon: it no longer thins just above the horizon or lies in bands across low ground and water, on the Mac and in Chrome
[review](../../../.orchestrate/evidence/ATMOSPHERE/WP9-review.md)

## WP10 A balloon adrift over the valley no longer lays a dark bar across the far ground or blinks one on and off: beyond the Sun's shadow maps, only the ground and what stands on it cast shadow
[review](../../../.orchestrate/evidence/ATMOSPHERE/WP10-review.md)

## WP11 Small Metal probes keep the atmospheric lookup checks fast

Use the [fixture design](DESIGN.md#atmosphere-lookup-regressions-m11) for the cases in the [case list and baseline times](LOG.md#m11-workspace-timing-baseline).

Record each case’s disposition, any replacement coverage and its before-and-after time in the workspace
check’s default Nextest build on the Mac, excluding compilation. Measure the fast cases alone and in a
focused batch under normal builder load. The batch passes the existing ten-second limit on its first run.

## WP6 The valley's sky and sunlight pass the route in motion on Mac and Chrome, and G3s closes

Read [Closing G3s](DESIGN.md#closing-g3s) first. Record the commit WP6 starts from before changing anything:
M6.4's reviewer compares the handoff tests with it, and WP6.5 reruns it beside the final build. The handoff
tests are `crates/thrum-render/src/atmosphere/products/tests/` on Metal and `scripts/environment_referee.mjs` in
Chrome; a fix that moves a product on purpose follows that section's rule for guards in both, in the same
landing. `scripts/atmosphere_cost.py` is created here, in the shape of `scripts/cloud_cost.py`.

WP6.7's and WP6.8's comparisons are new on both targets, and H5's closed-form and quadrature assertions
become records in both. Neither the split-sum table nor the cube in `reflection.rs` has copy
usage: `reflection/tests.rs` reads the table back on Metal with its own reader, the Chrome referee needs a
texel load, and a pass the test owns stores the probe.
`products/tests/filtered.rs` has the filtered-decode bound. The Metal tests' names contain
`split_sum_reads_stored` and `reflection_reads_stored_cube`, which the checks filter on.

Use main’s valley for the stops, motion and cost pair, with the terrain and sunrise hour from SHOWCASE WP9.
[The ruling](../ARCHITECTURE.md#ruling-on-the-valley-fix) covers a look change that moves `APPLE-TILE`'s
references.

## WP7 Haze thickens from crisp air to murk and takes the colour of its kind under a control, and shafts of sunlight stand in it below ridges and broken cloud, on the Mac and in Chrome

Record WP7.1’s starting commit before making changes. Follow the
[view-volume handoff](DESIGN.md#view-volume-builder-handoff).

## WP8 A test bank of fog stands in the valley: the land fades into it, sunlight dims inside it, and the camera walks in and out of it, on the Mac and in Chrome

The cloud-factor function and its field are in `crates/thrum-render/src/atmosphere/cloud.rs` and `cloud.wgsl`,
and the labelled test cloud field in `examples/v1-game/src/host/scene_source/cloud.rs`; the test medium sits
beside it and is replaced by `CLOUDS` WP14. What a provider's coefficients mean is in
[CLOUDS' design](../CLOUDS/DESIGN.md#condensate-that-meets-the-ground-mist-fog-and-low-cloud). If a forward pipeline cannot take the ground band inside
WebGPU's sampled-texture limit, write the proposal in the log and end `BLOCKED`.
