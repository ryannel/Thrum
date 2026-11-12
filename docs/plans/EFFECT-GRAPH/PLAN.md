# EFFECT-GRAPH plan

Engine modules run their own simulation and drawing passes inside the one frame: the foundation `SEA` and `CLOUDS`
build on. [Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[module list](../ARCHITECTURE.md)

Read [Shared files and exact baselines](DESIGN.md#shared-files-and-exact-baselines),
[Proof and builder handoff](DESIGN.md#proof-and-builder-handoff) and
[Exact baselines](../ARCHITECTURE.md#exact-baselines) before starting any milestone.

## Open for the owner

- Assumption: this foundation is built now, ahead of the owner's decision on when the sea and cloud port starts; the port decision says to keep the foundation converging on readiness, and no sea or cloud pass is built here.
- Assumption: the atmosphere's existing passes and textures are not moved onto the new seam or the new texture kinds. They keep their exact output, and move later only for a measured reason.
- Assumption: the diagnostic effect stays in the V1 game example behind a selection that is off by default, as a test fixture; it is never part of the showcase.
- Assumption: M1 starts as soon as this design passes and does not wait for `BASELINE-GUARD` M1. Until that gate exists, M1's own check covers the Mac baseline only and Chrome's is rechecked when the gate's builder first runs the verifier on main. The other choice is to add `BASELINE-GUARD:M1` to M1's `needs`, which holds M1 until the gate is built. M2 and M3 do wait for it.
- Assumption: this foundation proves that an effect's motion, reactive coverage and reset reach reconstruction's inputs, and does not wait for TAAU. How an effect looks after reconstruction is judged in `SEA`'s and `CLOUDS`' own milestones.

## M1 An engine module can declare volume, cube and mipped-array textures in the frame, and wrong declarations are refused

Criteria: M1.1–M1.5 in the done-when file.

- [x] Give the texture descriptor a dimension, with allocation, range flattening and transient reuse for each kind. Stay inside the graph's descriptor, validation and allocation code.
- [x] Add the preparation-time refusals the design lists, including the checks against the capability record, and record the limits Chrome reports on this Mac.
- [x] Prove each kind on Metal by exact readback, through resize and device replacement, and pin the compiled passes of the existing scene variants.
- [x] Rerun `APPLE-TILE`'s M0 matrix on a release build of the commit to be claimed.

## M2 A test effect keeps its simulation in step with the world through dropped frames, pauses and a replaced device, on Mac and in Chrome

Criteria: M2.1–M2.8 in the done-when file.

- [x] Let a scene hold an ordered list of contributions, and let a contribution declare resources and passes at the `upload`, `simulate`, `shadow` and `opaque` stages through the public seam.
- [x] Give each frame its tick, timeline and tick duration, and apply the design's state table: keep on an equal tick, advance on a later one, reset on an earlier one or a new timeline. Commit the state tick and the wrapping anchor only in `submitted`.
- [x] Give each history pair its own committed side with the `previous`, `next` and `result` bindings, flipped at submission only when its producer ran.
- [x] Add the three cadence classes, the compiler's refusals and full frames on a new epoch and on reset.
- [x] Build the diagnostic effect in `examples/v1-game` with an integer rule: two histories at different cadences, the consumer the tests read back, and the time-sliced volume. Write the scripted run's expectations from the state table; add the Mac run and the Chrome case.
- [x] Capture the trail for the reviewer; record costs with and without the effect from the executor's named pass timings.

## M3 A test effect draws through the scene as water or cloud would, hidden by nearer objects, and hands its motion to reconstruction, on Mac and in Chrome

Criteria: M3.1–M3.8 in the done-when file.

- [ ] Add the `after-opaque` stage after the star composite and before the pre-tone input. Store depth for the union of its readers, and declare the shared refraction copy only when an effect requests it.
- [ ] Write motion, reactive coverage and invalid history from the slab under the temporal contribution.
- [ ] Add the async-eligible flag with its check and listing, the knob list with commit at a frame boundary, and "not run" timing under pass names.
- [ ] Run Chrome and both native recipes; record the copy and pass-break costs; document the seam; add the ledger row.

Stop rules, for every milestone:
- With no effect attached, an existing scene must compile to the same passes and draw the same bytes. If it does not, stop and fix that before anything else.
- If your change fails an exact test or baseline another module owns, do not edit it. Write the proposal in the log and end `BLOCKED`.
- `graph.rs`, `scene_graph.rs`, `contribution.rs` and the frame host are shared with `APPLE-TILE` M3, `APPLE-PRESENTATION` M2, `CULLING` and `CREATOR` M2. Integrate on top of what has landed, and never restore an older copy to resolve a conflict.
- Build every proof from the commit you claim. In M1, rerun the milestone's checks after every rebase, before `land`. From M2, run the verifier before `land`, and again when `land` refuses a rebased commit for lack of a proof.
- Do not change the host's frame-sample timing fields before `APPLE-TILE:M3` is Done; M2 reads the executor's named pass timings.
- A mechanism no named consumer needs is not built. A stage or resource kind the design lacks goes to the log as a proposal.
- Chrome checks need a browser launch outside the Codex sandbox. If a launch is refused, leave the item open and say so; do not substitute a Mac result.
