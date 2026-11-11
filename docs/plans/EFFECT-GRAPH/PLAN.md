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

## M1 An engine module can declare volume, cube and mipped-array textures in the frame, and wrong declarations are refused

Criteria: M1.1–M1.5 in the done-when file.

- [ ] Give the texture descriptor a dimension, with allocation, range flattening and transient reuse for each kind. Stay inside the graph's descriptor, validation and allocation code.
- [ ] Add the preparation-time refusals the design lists, including the checks against the capability record, and record the limits Chrome reports on this Mac.
- [ ] Prove each kind on Metal by exact readback, through resize and device replacement, and pin the compiled passes of the existing scene variants.
- [ ] Rerun `APPLE-TILE`'s M0 matrix on a release build of the commit to be claimed.

## M2 A test effect keeps its simulation in step with the world through dropped frames, pauses and a replaced device, on Mac and in Chrome

Criteria: M2.1–M2.8 in the done-when file.

- [ ] Let a scene hold an ordered list of contributions, and let a contribution declare resources and passes at the `upload`, `simulate`, `shadow` and `opaque` stages through the public seam.
- [ ] Give each frame its tick, timeline and tick duration; commit state tick, history side and wrapping anchor only in `submitted`.
- [ ] Add the cadence rule to the compiler and the runtime skip, with a forced first run per epoch.
- [ ] Build the diagnostic effect in `examples/v1-game` with an integer rule and its CPU oracle; add the scripted run on the Mac and the Chrome case.
- [ ] Capture the trail for the reviewer; record costs with and without the effect.

## M3 A test effect draws through the scene as water or cloud would, hidden by nearer objects and stable in motion, on Mac and in Chrome

Criteria: M3.1–M3.8 in the done-when file.

- [ ] Add the `after-opaque` stage after the star composite and before the pre-tone input, with stored depth and the shared refraction copy declared only on request.
- [ ] Write motion, reactive coverage and invalid history from the slab under the temporal contribution.
- [ ] Add the async-eligible flag with its check and listing, the knob list with commit at a frame boundary, and "not run" timing under pass names.
- [ ] Run Chrome and both native recipes; record the copy and pass-break costs; document the seam; add the ledger row.

Stop rules, for every milestone:
- With no effect attached, an existing scene must compile to the same passes and draw the same bytes. If it does not, stop and fix that before anything else.
- If your change fails an exact test or baseline another module owns, do not edit it. Write the proposal in the log and end `BLOCKED`.
- `graph.rs`, `scene_graph.rs`, `contribution.rs` and the frame host are shared with `APPLE-TILE` M3, `APPLE-PRESENTATION` M2, `CULLING` and `CREATOR` M2. Integrate on top of what has landed, never restore an older copy to resolve a conflict, and rerun the milestone's checks after every rebase, before `land`.
- A mechanism no named consumer needs is not built. A stage or resource kind the design lacks goes to the log as a proposal.
- Chrome checks need a browser launch outside the Codex sandbox. If a launch is refused, leave the item open and say so; do not substitute a Mac result.
