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

## M1 On the mixed-mesh field, Occlusion draws only what Direct shows, pixel for pixel

Criteria: M1.1–M1.5 in the done-when file.

- [ ] Key the visible-last bits by logical slot on the bucket route, clear the next side each frame, and lift the refusal in mesh admission.
- [ ] Add a depth-only twin of each bucket class pipeline and draw the prepass per bucket from the prepass list.
- [ ] Witness equality with bucket-ordered Direct: layout churn, the release flythrough, resize and device replacement, and the mesh tie at the far turned pose, with the design's three mutants recorded as failing.
- [ ] Rerun `APPLE-TILE`'s M0 matrix on a release build of the commit to be claimed.

## M2 The culling mode follows the device and the measurements, and switches while the game runs

Criteria: M2.1–M2.7 in the done-when file.

- [ ] Resolve the requested mode from the capability record, report requested and resolved modes with the reason, and make every equality witness assert the resolved mode.
- [ ] Run the simulator with a culled mode requested and record the Direct fallback.
- [ ] Give Chrome the mixed-mesh field and its oracle case.
- [ ] Record paired Direct, Frustum and Occlusion rows on the mixed-mesh field at 1920×1080 from two runs, and set the field's default by the design's rule.
- [ ] Add the typed mode control on the Mac and in Chrome, switching through scene republication off the frame path.

## M3 The valley's 10,000 entities are culled by default with no pixel changed, on Mac and in Chrome

Criteria: M3.1–M3.10 in the done-when file.

- [ ] Add the route comparison against Direct on both targets, and the opt-in mesh selection for the valley's entities.
- [ ] Record paired rows on the moving route and set the valley's default by the design's rule.
- [ ] Prove equality with temporal sampling selected, and make the material table readable from compute.
- [ ] Capture the valley for the reviewer's motion inspection; update the maturity ledger and the evidence page.

Stop rules, for every milestone:
- If a mode differs from its oracle in any pixel, stop and diagnose. Never widen a comparison, sample fewer frames or change the oracle to pass.
- If your change fails an exact test or baseline another module owns, do not edit it. Write the proposal in the log and end `BLOCKED`.
- After every rebase that brings in commits under `crates/thrum-render`, `crates/thrum-asset-gpu` or `examples/v1-game`, rerun the milestone's checks before `land`, and build every proof from the commit you claim.
- A default mode changes only on the recorded rows the design's rule names. No speed claim rests on one run or on an uncontended figure that does not exist.
- Chrome and simulator checks need a browser or simulator launch outside the Codex sandbox. If a launch is refused, leave the item open and say so; do not substitute a Mac result.
