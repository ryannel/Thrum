# WATERS design proofs

Two reproductions run while [the design](../../../plans/WATERS/DESIGN.md#check-that-it-can-work) was written, on 14
November 2026. They continue the lettering of [the sea's proofs](../../SEA/design-proof/README.md), parts A to G, which
the design also cites. They are evidence for the design's choices and the source of the bounds in M1 and M6 of
[the done-when file](../../../plans/WATERS/DONE.json). They are not engine code and no engine test depends on them.

| Script | Output | What it shows |
|---|---|---|
| `water_proof.py` | `water-proof.txt` | H: waves on water of any depth from uniform-depth operators a factor of two apart, blended per cell: speed within 1.6% and a false loss under 0.3% a second, where one operator at a pond's mean depth is 23% wrong. I: the share of an open sea's wave layers a sheltered body takes by fetch, and the depth limit near a shore |
