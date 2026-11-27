# ATMOSPHERE plan

[Design](DESIGN.md) · [Done-when file](DONE.json) · [Log](LOG.md) · [Module list](../ARCHITECTURE.md)

## Open for the owner

- Assumption: WP3 ends with a CPU diagnosis. The authored-spur replay and an in-game correction remain open under G3s. A later proposal needs the original inputs and the fixed and realtime captures specified in the design.
- Assumption: WP4 compares the reconstructed geometry cases in one bounded CPU trial. The original ray inputs and exact finite-disk match remain open under G3s.
- Assumption: the governor's chart knob (4, 6 and 8 by tier) is the longest a chart refresh may be spread, and the atmosphere chooses each refresh's spread from how fast the Sun, the camera's altitude and the Sun's light are changing. The governor's registry and policy are unchanged. On the game's clock the charts take the knob's full value. A scrub shortens the spread, to every frame at 1.5° of Sun a frame, and costs what the charts cost with the governor off while it lasts.
- Assumption: a refresh may let the sky's inputs move 0.2° of Sun, 10 m of altitude or 1% of light. These are the planner's starting values, and the builder tunes them by eye against WP12.4 and WP12.5.
- Assumption: WP12 judges the knob at 4 in the Mac window and at 8 in Chrome, the longest spread at the slower display. Chrome at 4 is not captured.

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

Read [the chart cadence](DESIGN.md#the-chart-cadence-wp12) first. The knob registry is `APPLE-PRESENTATION`'s:
build the ungoverned value in the governor's files under
[the module list's ruling](../ARCHITECTURE.md#seam-ruling-for-atmosphere-wp12), keep the aerial knob and the
tests of `APPLE-PRESENTATION` D7.5 as they are, and merge one after the other with `APPLE-PRESENTATION` WP18,
which is built beside this in the frame host. Capture WP12.4 in a window: full screen is not this work
package's.

Start from the first build's verified checkpoint, `286a34491` on `build/atmosphere`, and keep its scheduler,
device probes, trace, route timing and compatibility fixes. What changes is
[how a refresh's spread is chosen](DESIGN.md#the-spread-follows-the-motion): the knob becomes the ceiling,
with nothing new in the governor's files. That checkpoint's schedule tests and probes step the Sun by 0.5° or
more, which now shortens the spread, so restate their steps as fractions of a tolerance. Its WP12.4 and WP12.5
takes are not reused: the game's clock, not the scrub, is now where the charts are spread. Its Chrome frames
150 to 165 at 8 stay as the example of the fault, and its
[handoff](../../../.orchestrate/evidence/ATMOSPHERE/WP12-handoff.md) lists what else is owed, a clean run of
the workspace's tests among it.
