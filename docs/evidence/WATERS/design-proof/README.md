# WATERS design proofs

Reproductions run while [the design](../../../plans/WATERS/DESIGN.md#check-that-it-can-work) was written, on 14
November 2026. They continue the lettering of [the sea's proofs](../../SEA/design-proof/README.md), parts A to G, which
the design also cites. Part H was run again, and parts J to L added, after
[the design review](../design-review.md) of the same day. They are evidence for the design's choices and the source
of the bounds in M1, M6 and M9 of [the done-when file](../../../plans/WATERS/DONE.json). They are not engine code and
no engine test depends on them. H and J are 1D sections in f64; none is a GPU or a visual measurement.

Run `python3 water_proof.py`, about half a minute; its output is `water-proof.txt`.

| Part | What it shows |
|---|---|
| H | The interaction step at depth as the design selects it: the finite-depth update is a rotation where eWave's equations with only the frequency replaced grow 9% a second; a ladder of depths a factor of two apart, blended per cell, holds phase speed within 2.1% and group speed within 4.4% over each level's band, in steps of 1 and 8 ticks, and never grows; fields through a beach, a window of 0.125 m shallows and 10 m water, and rough beds with dry patches |
| I | The share of an open sea's wave layers a sheltered body takes by fetch, and the depth limit near a shore |
| J | Dry ground: one transform carries 3.2% of a pulse across a 2 m bar; a solve for each region carries none; what thin land inside one region lets through |
| K | Joins: a channel from a lake to the sea, with the lake's stage and the tide moved under a swell crest. Hard precedence leaves steps of 0.5 to 1.5 m; joins in the map leave none |
| L | The swash: under the depth limit with a swash depth, the waterline's run-up and rundown match the closed form; with the limit alone it does not move |
