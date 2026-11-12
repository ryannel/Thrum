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

## M1 On the mixed-mesh field, Occlusion draws only what Direct shows, pixel for pixel

Criteria: M1.1–M1.5 in the done-when file.

- [x] Key the visible-last bits by logical slot on the bucket route, clear the next side each frame, and lift the refusal in mesh admission.
- [x] Add a depth-only twin of each bucket class pipeline and draw the prepass per bucket from the prepass list.
- [x] Witness equality with bucket-ordered Direct: layout churn, the release flythrough, resize and device replacement, and the mesh tie at the far turned pose, with the design's mutants recorded as failing.
- [x] Search the reference for a pose that needs the footprint widening. Record the witness and its failing no-widening mutant, or the poses searched.
- [x] Rerun `APPLE-TILE`'s M0 matrix on a release build of the commit to be claimed.

### Review round 1 — MF1, mixed-mesh host lifecycle (M1.4)

- [x] Exercise a running `runtime::FrameHost` with the game's full mixed-mesh field, a surface presenter and hidden slots in the named `visibility_buckets` target.
- [x] Call the host's surface resize and replacement-device preparation/publication APIs, compare every next frame with bucket-ordered Direct, and assert resolved Occlusion, retained scene identity and renewed hiding.
- [ ] Retain fresh Metal output and source identity for M1.4, re-verify affected checks and merge the repaired milestone for review.

## M2 The game picks its culling mode from the device and the measurements, and says which mode ran

Criteria: M2.1–M2.5 in the done-when file.

- [ ] Resolve the requested mode from the capability record, report requested and resolved modes with the reason, and make every equality witness assert the resolved mode.
- [ ] Run the simulator with a culled mode requested and record the Direct fallback.
- [ ] Give Chrome the mixed-mesh field and its oracle case.
- [ ] Record paired Direct, Frustum and Occlusion rows on the mixed-mesh field at 1920×1080 from two runs, and set the field's default by the design's rule.

## M3 The valley's 10,000 entities are culled by default with no pixel changed, on Mac and in Chrome

Criteria: M3.1–M3.10 in the done-when file.

- [ ] Add the route comparison against Direct on both targets, and the opt-in mesh selection for the valley's entities.
- [ ] Record paired rows on the moving route and set the valley's default by the design's rule.
- [ ] Prove equality with temporal sampling selected, and make the material table readable from compute.
- [ ] Capture the valley for the reviewer's motion inspection; update the maturity ledger and the evidence page.

Stop rules, for every milestone:
- If a mode differs from its oracle in any pixel, stop and diagnose. Never widen a comparison, sample fewer frames or change the oracle to pass.
- If your change fails an exact test or baseline another module owns, do not edit it. Write the proposal in the log and end `BLOCKED`.
- Build every proof from the commit you claim. In M1, rerun the milestone's checks after every rebase that brings in commits under `crates/`, `examples/v1-game` or `scripts/`, before `land`. From M2, run the verifier before `land`, and again when `land` refuses a rebased commit for lack of a proof.
- A default mode changes only on the recorded rows the design's rule names. No speed claim rests on one run or on an uncontended figure that does not exist.
- Chrome and simulator checks need a browser or simulator launch outside the Codex sandbox. If a launch is refused, leave the item open and say so; do not substitute a Mac result.
