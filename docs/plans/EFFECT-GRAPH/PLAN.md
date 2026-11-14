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
- Assumption: M3 does not wait for `CULLING` M3 to stop the game's image varying under GPU load. Until then an exact-image run that fails only along triangle edges is recorded, checked by a reviewer and run again, and M3 lands on a complete pass. The other choice adds `CULLING:M3` to M3's `depends_on`, which holds M3 and the six modules waiting on it until that milestone is Done.

## M1 An engine module can declare volume, cube and mipped-array textures in the frame, and wrong declarations are refused: done
[review](../../evidence/EFFECT-GRAPH/M1-review.md)

## M2 A test effect keeps its simulation in step with the world through dropped frames, pauses and a replaced device, on Mac and in Chrome: done
[review](../../evidence/EFFECT-GRAPH/M2-review.md)

## M3 A test effect draws through the scene as water or cloud would, hidden by nearer objects, and hands its motion to reconstruction, on Mac and in Chrome

Criteria: M3.1–M3.9 in the done-when file.

The milestone is built and not merged. It is on branch `build/effect-graph-m3`, in the worktree
`.claude/worktrees/effect-graph-m3`, last at `baeb6d1f`, and its handoff is
`docs/evidence/EFFECT-GRAPH/M3-handoff.md` on that branch. What stopped it was one Chrome record of 481 at
`324586ee`. [The module list's ruling](../ARCHITECTURE.md#a-frame-that-varies-under-load) now covers that
failure: `APPLE-TILE` owes nothing, and its references, the comparisons and the verifier stay as they are.
[The planner's reading](../../evidence/EFFECT-GRAPH/M3-replan/README.md) of the two retained frames is the
difference image to check your own against.

The order is part of the task. The ruling's fourth test needs a complete verifier pass on the content that
failed, so first run the unchanged verifier on that content. `baeb6d1f` differs from `324586ee` only in
evidence, plans and the ledger, so its content key should be the failed run's, `dc5b66d1…`; confirm that before
the run, and use a checkout of `baeb6d1f` if a rebase has already changed the covered content. On a pass,
complete the record M3.9 asks for, then qualify the rebased commit as the handoff lists: the verifier again if
the content key changed, the full gate, the native captures and costs, and `land`. If the same record fails
again, or the failure misses any of the four tests, end `BLOCKED` with both runs. Once the record is complete,
release the scratch the handoff kept for the diagnosis.

Stop rules:
- With no effect attached, an existing scene must compile to the same passes and draw the same bytes. If it does not, stop and fix that before anything else.
- If your change fails an exact test or baseline another module owns, do not edit it. Write the proposal in the log and end `BLOCKED`. The one exception is a verifier run that meets all four tests of the ruling above.
- `graph.rs`, `scene_graph.rs`, `contribution.rs` and the frame host are shared with `APPLE-TILE`, `APPLE-PRESENTATION`, `CULLING` and `CREATOR`. Integrate on top of what has landed, and never restore an older copy to resolve a conflict.
- Build every proof from the commit you claim. Run the verifier before `land`, and again when `land` refuses a rebased commit for lack of a proof.
- A mechanism no named consumer needs is not built. A stage or resource kind the design lacks goes to the log as a proposal.
- Chrome checks need a browser launch outside the Codex sandbox. If a launch is refused, leave the item open and say so; do not substitute a Mac result.
