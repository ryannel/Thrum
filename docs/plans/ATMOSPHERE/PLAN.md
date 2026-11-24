# ATMOSPHERE plan

[Design](DESIGN.md) · [Done-when file](DONE.json) · [Log](LOG.md) · [Module list](../ARCHITECTURE.md)

## Open for the owner

- Assumption: WP3 ends with a CPU diagnosis. The authored-spur replay and an in-game correction remain open under G3s. A later proposal needs the original inputs and the fixed and realtime captures specified in the design.
- Assumption: WP4 compares the reconstructed geometry cases in one bounded CPU trial. The original ray inputs and exact finite-disk match remain open under G3s.

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
