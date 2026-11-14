# CULLING plan

Closes gate G3c: only what the camera can see is drawn. [Design](DESIGN.md) · [done-when file](DONE.json), which
owns the criteria · [log](LOG.md) · [module list](../ARCHITECTURE.md)

Frustum, Occlusion on cubes, the bucket route and Sun-caster culling were built before the loop; the
[design](DESIGN.md#purpose-and-scope) lists them with their evidence. Read
[Proof and builder handoff](DESIGN.md#proof-and-builder-handoff) and
[Exact baselines](../ARCHITECTURE.md#exact-baselines) before starting any milestone.

## Open for the owner

- Assumption: G3c closes on the scenes that exist, the mixed-mesh field and the valley as it stands, before `SHOWCASE` assembles the exit scene. `SHOWCASE` reruns the same equivalence and cost commands there, and the default mode may change then by the same rule.
- Assumption: "GPU culling" in the V1 exit is met by Frustum as the default with Occlusion selectable, if the measured rows do not favour Occlusion on any shipped scene. The gate's Hi-Z clause is met by Occlusion running, equal to Direct, on Mac and Chrome.
- Assumption: the native-only culling recipes (subgroup compaction, first-instance draws) are not built for V1. They stay candidates for the performance phase the brief puts after the exit.
- Assumption: the culling mode is chosen when a scene is prepared, at launch or page load, and no control switches it while the game runs. The brief and gate G3c do not ask for a switch, and one that works needs a change across the frame runtime, the browser host and the contribution seam; the design says why and names the mechanism to cost if it is wanted.
- Assumption: M1 starts as soon as this design passes and does not wait for `BASELINE-GUARD` M1. Until that gate exists, M1's own check covers the Mac baseline only and Chrome's is rechecked when the gate's builder first runs the verifier on main. The other choice is to add `BASELINE-GUARD:M1` to M1's `needs`, which holds M1 until the gate is built. M2 and M3 do wait for it.
- Assumption: the forward pass's coverage output is on in every scene, with no switch, so that the game draws one image under GPU load. It changes pixels along shared triangle edges, so M3 refreshes `APPLE-TILE`'s reference sets once, in its own landing, and its cost in frame time is measured there, not known today. The other choice keeps today's output as the default behind a selection, and leaves the variation in every scene and every exact test that does not ask.

## M1 On the mixed-mesh field, Occlusion draws only what Direct shows, pixel for pixel: done
[review](../../evidence/CULLING/M1-review.md)

## M2 The game picks its culling mode from the device and the measurements, and says which mode ran: done
[review](../../evidence/CULLING/M2-review.md)

## M3 The valley's 10,000 entities are culled by default and match the unculled image pixel for pixel, on Mac and in Chrome

Criteria: M3.1–M3.13 in the done-when file. Read [One image under load](DESIGN.md#m3-one-image-under-load) and
[the module list's ruling on M3](../ARCHITECTURE.md#exact-baselines) first.

The work so far is on branch `build/culling`, rebased onto main on 15 November (`9170f291`), with
[the handoff](../../evidence/CULLING/M3-handoff.md) there. Commit `035e01a9` on it is the coverage output, which
`9170f291` reverses; `docs/evidence/CULLING/M3/triangle-merging/` holds its patch, the loaded-route and
qualification runners and the retained controls. The handoff names both commits by their ids before the rebase. [The failed review](../../evidence/CULLING/M3-review.md) found M3.3 open: star
visibility selected Direct for the cooked valley, and the legacy cost benchmarks measured Direct twice.

The order is part of this milestone, and [the design](DESIGN.md#qualified-before-it-lands) gives it. Run the
verifier at the starting commit and keep its record. Build the coverage output and the regenerated references as
a local candidate, and push nothing until M3.11's campaign, M3.12's difference images, which you judge by that
item's own list, and the verifier have all passed on it. Then `land` pushes the shader and the references
together, as M3's first checkpoint. `scripts/apple_tile_m0.py` made the Mac set, and the verifier archives each
Chrome run's manifest. If the claimed shader differs in the campaign, or a reference moves where M3.12 says none
may, push nothing and end `BLOCKED` with the payloads. Every other proof of M3 has to be drawn by the corrected
shader, so M3.3's rows and the captures follow the landing. `APPLE-TILE` M10 and `ATMOSPHERE` M9 also land
covered changes; integrate on top of what has landed, and the design says which proofs a rebase repeats.

M3.13's rows never stop the milestone or the landing. If they cross a tripwire, investigate the cause and record
a disposition, as the design's Cost paragraph says, and keep building.

Stop rules, for every milestone:
- If a mode differs from its oracle in any pixel, stop and diagnose. Never widen a comparison, sample fewer frames or change the oracle to pass.
- If your change fails an exact test or baseline another module owns, do not edit it. Write the proposal in the log and end `BLOCKED`. The one exception is the refresh the module list rules for M3's coverage output.
- Build every proof from the commit you claim. In M1, rerun the milestone's checks after every rebase that brings in commits under `crates/`, `examples/v1-game` or `scripts/`, before `land`. From M2, run the verifier before `land`, and again when `land` refuses a rebased commit for lack of a proof.
- A default mode changes only on the recorded rows the design's rule names. No speed claim rests on one run or on an uncontended figure that does not exist.
- Chrome and simulator checks need a browser or simulator launch outside the Codex sandbox. If a launch is refused, leave the item open and say so; do not substitute a Mac result.
