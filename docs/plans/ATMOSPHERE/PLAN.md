# ATMOSPHERE plan

Closes gate G3s at M6; M7 and M8 are track 2. Milestones run in the order M5, M9, M6, M7, M8.
[Design](DESIGN.md) · [done-when file](DONE.json), which holds M5 to M9 · [log](LOG.md) ·
[module list](../ARCHITECTURE.md)

## Open for the owner

- Assumption: "the modern technique" (the lead's task, 14 November) is the camera-aligned aerial volume current engines ship, corrected so that the air goes on under the model's ground. Marching the air for every pixel, Unreal's costlier option, is not built.
- Assumption: the corrected air is the aerial product of every scene, with no switch. The showcase valley's far low ground gains a little haze, so M9 refreshes `APPLE-TILE`'s five exact reference sets once, in its own landing. The other choice keeps today's output as the default behind a selection, and leaves the defect in every scene that does not ask.
- Assumption: M9 is built now, ahead of M6 and TAAU, and is held by numbers and by where the retained images moved. Its look on a real sea is judged in `WATERS` M2, which now waits for it.
- Assumption: G3s closes with the west-lookout crossing (M3) still parked and named as a known defect, and with M4 deferred; neither is judged in M6.
- Assumption: the sky's closing review waits for TAAU, so it is judged once on the image the exit will show.
- Assumption: solar sky radiance and solar aerial transport stay unqualified against an all-orders reference, and under the owner's bar of 17 November no accuracy campaign is planned. The referees P1–P11, and the five handoff comparisons that repeat one through the lent library, stay in the checks as regression guards, which a deliberate change to the look gives new expected values. The handoff's contract comparisons, which read the stored product, stay fixed.
- Assumption: the lent sky and aerial lookups stay one hardware-filtered read, and their handoff bound allows one stored half-float step, because Metal returns filtered half-float reads at that precision. Blending exact texels in the shader for float32 results was rejected: it costs every sky pixel and lit fragment more reads and changes every scene's image, and M6 judges by eye whether a step shows.
- Assumption: "top quality atmosphere" (the owner, 14 November) gives this module two milestones after G3s closes, M7 and M8: haze the weather sets, shafts of light, and fog near the camera, integrated jointly with the air in one view volume. They gate nothing in the V1 exit, and the exit scene does not declare the view volume.
- Assumption: M7 and M8 wait for M6, so fog's first frame waits for TAAU and for G3s to close.
- Assumption: lamps and other local lights do not light haze or fog; the Sun, the sky and the ground's light do.
- Assumption: the weather sets the haze, so `ATMOSPHERE`'s fixed air (a visual range of about 244 km) becomes an input with that value as its default. Every scene that sets nothing draws as before.
- Assumption: the haze's kind is one Ångström exponent, defaulting to 0, today's grey aerosol; its single-scattering albedo and phase stay fixed. The check of 17 November (`docs/evidence/ATMOSPHERE/haze-kind-check.md`) found the exponent's colour visible at a low Sun and small at noon. Which exponent a weather sets, and whether moisture drives it, is the game's.
- Assumption: the terrain lip on the shipped Ground E pan is judged in M6.1 as a brightness step, and the held 17.4 h stop is judged; only the moving spur edge of the parked M3 crossing is left out.

## M1 The valley remains legible at moonless midnight: done
[record and reviews](../../evidence/ATMOSPHERE/M1-M4-records.md#m1-the-valley-remains-legible-at-moonless-midnight-done)

## M2 Stars emerge as the observer turns and time advances: done
[record and reviews](../../evidence/ATMOSPHERE/M1-M4-records.md#m2-stars-emerge-as-the-observer-turns-and-time-advances-done)

## M3 The west lookout’s shadow crosses the ground without a flash

<a id="m3-sunset-crossing"></a>

A parked spike, outside the done-when file: the stock graph ships and M6 does not judge the moving spur edge.
[The record](../../evidence/ATMOSPHERE/M1-M4-records.md#m3-the-west-lookouts-shadow-crosses-the-ground-without-a-flash-parked-spike)
holds its criteria D1 to D3, the proposed reference clip, the diagnosis handoff, the finished work and the work
left. It re-enters only as a new commission against those criteria.

## M4 Exact finite-disk shadow reference match

A deferred bounded spike, outside the done-when file. M3 and the other V1 gates proceed without it.
[The record](../../evidence/ATMOSPHERE/M1-M4-records.md#m4-exact-finite-disk-shadow-reference-match-deferred-bounded-spike)
holds its criteria, the finished work and the work left.

## M5 Other modules read the one sky and Sun, and one cloud optical depth dims direct sunlight once: done
[review](../../evidence/ATMOSPHERE/M5-review.md)

## M9 Haze near sea level deepens evenly to the horizon: it no longer thins just above the horizon or lies in bands across low ground and water, on the Mac and in Chrome

Read [The air at the ground](DESIGN.md#the-air-at-the-ground) and
[the module list's ruling on M9](../ARCHITECTURE.md#exact-baselines) first. The pass is
`crates/thrum-render/src/atmosphere/aerial.wgsl`, which ends a line at `ground_distance`. `altitude` in
`geometry.wgsl` holds every altitude at zero and the charts' lookups rely on that, so the depth under the
ground is the pass's own to work out. `gpu/tests/aerial.rs` reads a volume back and `products/tests/filtered.rs`
has the lent operation's blend in f64; the new tests are a module `aerial_ground` beside them, with tests
whose names begin `near_ground_reads` and `above_ground_bytes`, which the checks filter on.

The order is part of this milestone. Before any change, run the verifier at the starting commit and keep its
record, store M9.2's frames and take M9.8's first arm. The change and the refreshed references then land
together, because the gate passes a covered change only with a proof for its exact content: regenerate each
reference that moved from the changed commit with `APPLE-TILE`'s own scripts, then run the verifier for the
proof. `scripts/apple_tile_m0.py` made the Mac set, and the verifier archives each Chrome run's manifest. If
M9.1 misses its allowance, or a reference moves where M9.6 says none may, end
`BLOCKED` with the measurement. `SCENE-VIEWS` M1 and `CLOUDS`' rulings edit the same pass; integrate on top of
what has landed.

## M6 The valley's sky and sunlight pass the route in motion on Mac and Chrome, and G3s closes

Read [Closing G3s](DESIGN.md#closing-g3s) first. Record the commit M6 starts from before changing anything:
M6.4's reviewer compares the handoff tests with it, and M6.5 reruns it beside the final build. The handoff
tests are `crates/thrum-render/src/atmosphere/products/tests/` on Metal and `scripts/environment_referee.mjs` in
Chrome; a fix that moves a product on purpose follows that section's rule for guards in both, in the same
landing. `scripts/atmosphere_cost.py` is created here, in the shape of `scripts/cloud_cost.py`.

## M7 Haze thickens from crisp air to murk and takes the colour of its kind under a control, and shafts of sunlight stand in it below ridges and broken cloud, on the Mac and in Chrome

Read [The view volume](DESIGN.md#the-view-volume-haze-fog-and-light-near-the-camera) first. The aerial
producer is `crates/thrum-render/src/atmosphere/aerial.wgsl` with `AERIAL_EXTENT` in `gpu.rs`; the aerosol's
constants are written into shader source by `spectrum.rs`; the forward decoders are the `synthetic_aerial*.wgsl`
files, one of which splits solar in-scatter by the surface's shadow, which the volume must not repeat. The
volume's history uses `EFFECT-GRAPH`'s state kinds, as `examples/v1-game/src/effect` does. The control and the scene's declaration go behind a selection in `examples/v1-game` that is off by
default; that game is `SHOWCASE`'s, so leave its generator, routes and exit configuration alone. `CLOUDS`
builds rulings 1 to 4 in these same files; integrate on top of what has landed, and if its haze scale is
present make sure the volume does not take it. Record M7.1's base before changing anything.
[The look decides](DESIGN.md#how-the-view-volume-is-proved): the f64 quadrature is there to find the cause of a
picture that looks wrong, and no item compares the volume with it. The sky is drawn from `atmosphere/sky.wgsl` and the stars in `synthetic/stars.rs`; both take the
terminal pair. `scripts/view_volume.py` is created here. The aerosol's kind makes its eight band values differ;
the f64 reference in `atmosphere/gpu/tests/reference/` takes one Mie value per band, and
[the haze-kind check](../../evidence/ATMOSPHERE/haze-kind-check.md) holds a probe that sets them band by band. Part III says which grids band in thick, coloured haze; the builder chooses the grid from the stills.

## M8 A test bank of fog stands in the valley: the land fades into it, sunlight dims inside it, and the camera walks in and out of it, on the Mac and in Chrome

The cloud-factor function and its field are in `crates/thrum-render/src/atmosphere/cloud.rs` and `cloud.wgsl`,
and the labelled test cloud field in `examples/v1-game/src/host/scene_source/cloud.rs`; the test medium sits
beside it and is replaced by `CLOUDS` M14. What a provider's coefficients mean is in
[CLOUDS' design](../CLOUDS/DESIGN.md#condensate-that-meets-the-ground-mist-fog-and-low-cloud). If a forward pipeline cannot take the ground band inside
WebGPU's sampled-texture limit, write the proposal in the log and end `BLOCKED`.
