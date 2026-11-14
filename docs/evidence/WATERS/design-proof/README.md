# WATERS design proofs

Reproductions run while [the design](../../../plans/WATERS/DESIGN.md#check-that-it-can-work) was written, on 14
November 2026. They continue the lettering of [the sea's proofs](../../SEA/design-proof/README.md), parts A to G, which
the design also cites. Part H was run again, and parts J to L added, after the first design review of the same
day. After the second, part K gained the assembled surface and part M was added. Both reviews are in
[the folder above](../). They are evidence for the design's choices and the source
of the bounds in M1, M6, M7 and M9 of [the done-when file](../../../plans/WATERS/DONE.json). They are not engine code and
no engine test depends on them. H and J are 1D sections in f64; none is a GPU or a visual measurement.

Run `python3 water_proof.py`, about a minute; its output is `water-proof.txt`.

| Part | What it shows |
|---|---|
| H | The interaction step at depth as the design selects it: the finite-depth update is a rotation where eWave's equations with only the frequency replaced grow 9% a second; a ladder of depths a factor of two apart, blended per cell, holds phase speed within 2.1% and group speed within 4.4% over each level's band, in steps of 1 and 8 ticks, and never grows; fields through a beach, a window of 0.125 m shallows and 10 m water, and rough beds with dry patches |
| I | The share of an open sea's wave layers a sheltered body takes by fetch, and the depth limit near a shore |
| J | Dry ground: one transform carries 3.2% of a pulse across a 2 m bar; a solve for each region carries none; what thin land inside one region lets through |
| K | Joins: a channel from a lake to the sea, with the lake's stage and the tide moved under a swell crest. Hard precedence leaves steps of 0.5 to 1.5 m; joins in the map leave none. After the depth limit, a swash depth kept once for each body leaves 0.21 m at a mouth 0.25 m deep (the second review's case); taken from the row it leaves none. Beside a mouth, backwater from the sea's still level leaves its sheet up to 0.10 m from the chart's; from its reach, none |
| L | The swash: under the depth limit with a swash depth, the waterline's run-up and rundown match the closed form; with the limit alone it does not move |
| M | Foam in the swash: the drift a rising sheet gives carries parcels and a patch of foam cells to the run-up line and leaves them there, in steps of 1 and 8 ticks; the current's and the wind's drift alone moves nothing |

## The revision for surf, mirror views and the view from under water

Parts N to Q were run on 14 November 2026, when the owner asked for breaking waves, reflection of scenery that
is off screen and a camera under water. Run `python3 surf_view_proof.py`, about ten seconds; its output is
`surf-view-proof.txt`. They are the source of the bounds in M12 and M14. All are f64 arithmetic on sections
and small grids; none is a GPU or a visual measurement.

| Part | What it shows |
|---|---|
| N | The shore transform: a long wave's phase lag from an eikonal solve on the bed's grid is within 0.34 rad of Snell's closed form on a plane beach at 2 m cells, and its crests turn as Snell's law gives; lags kept for wavelengths a factor of two apart blend to within 2.0% of the true wavenumber; lags kept for stages 0.5 m apart blend to within 2.6% in a metre of water or more; round an island the transformed wave meets every shore from the sea, at most 60° off its normal, and the exposure of a 45° train calms the lee |
| O | Breaking: a regular wave under linear shoaling breaks 9% under Komar and Gaughan's height; the largest crests of a set break furthest out; a crest thrown forward in proportion to its height overhangs and never crosses itself; the swash's closed form (part L) is unchanged with the transform on |
| P | The mirror view: a reflected ray looked up in an image drawn from the camera's mirror image, corrected twice against the mirror's depth, lands within a median of 0.09% of the true ray's length under ripples and waves, where the plain planar lookup is 11% to 13% off |
| Q | Under water: Snell's window at 48.6° and the Fresnel split from beneath; the closed form for light scattered into a path through water lit from above matches a numerical integral |
