# EFFECT-GRAPH plan

Engine modules run their own simulation and drawing passes inside the one frame: the foundation `SEA` and `CLOUDS`
build on. [Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[module list](../ARCHITECTURE.md)

Read [Shared files and exact baselines](DESIGN.md#shared-files-and-exact-baselines),
[Proof and builder handoff](DESIGN.md#proof-and-builder-handoff) and
[Exact baselines](../ARCHITECTURE.md#exact-baselines) before starting any milestone.

## Open for the human

- Assumption: this foundation is built now, ahead of the owner's decision on when the sea and cloud port starts; the port decision says to keep the foundation converging on readiness, and no sea or cloud pass is built here.
- Assumption: the atmosphere's existing passes and textures are not moved onto the new seam or the new texture kinds. They keep their exact output, and move later only for a measured reason.
- Assumption: the diagnostic effect stays in the V1 game example behind a selection that is off by default, as a test fixture; it is never part of the showcase.
- Assumption: M1 starts as soon as this design passes and does not wait for `BASELINE-GUARD` M1. Until that gate exists, M1's own check covers the Mac baseline only and Chrome's is rechecked when the gate's builder first runs the verifier on main. The other choice is to add `BASELINE-GUARD:M1` to M1's `needs`, which holds M1 until the gate is built. M2 and M3 do wait for it.
- Assumption: this foundation proves that an effect's motion, reactive coverage and reset reach reconstruction's inputs, and does not wait for TAAU. How an effect looks after reconstruction is judged in `SEA`'s and `CLOUDS`' own milestones.
- Assumption: M3 does not wait for `CULLING` M3 to stop the game's image varying under GPU load. Until then an exact-image run that fails only along triangle edges is recorded, checked by a reviewer and run again, and M3 lands on a complete pass. The other choice adds `CULLING:M3` to M3's `depends_on`, which holds M3 and the six modules waiting on it until that milestone is Done.
- Assumption: M4 gives effects the display phase and leaves the sea's waves, 2D sprites and a game's camera and entities on the interpolation alpha. Under a display lead those step unevenly by the same arithmetic, which nobody has captured yet. Each is its owner's to plan, the camera and entities with the game contract in `HOST` M4.

## M1 An engine module can declare volume, cube and mipped-array textures in the frame, and wrong declarations are refused: done
[review](../../evidence/EFFECT-GRAPH/M1-review.md)

## M2 A test effect keeps its simulation in step with the world through dropped frames, pauses and a replaced device, on Mac and in Chrome: done
[review](../../evidence/EFFECT-GRAPH/M2-review.md)

## M3 A test effect draws through the scene as water or cloud would, hidden by nearer objects, and hands its motion to reconstruction, on Mac and in Chrome: done
[review](../../evidence/EFFECT-GRAPH/M3-review.md)

## M4 A test effect that drifts with time moves by even steps at the display's refresh, between ticks and past the newest one

[The display phase](DESIGN.md#the-display-phase-m4) is the design, and
[the ruling](../ARCHITECTURE.md#seam-ruling-for-effect-graph-m4) says whose files it is built in.

Today `ClockSample::alpha_at` in `crates/thrum-core/src/clock.rs` holds the fraction to one tick. The world lane
and the clocked world pass it on, and the frame host asks `FrameWorld::presentation_alpha_at` in
`crates/thrum-render/src/runtime.rs` with the lead `set_presentation_lead` stored. `EffectTime` is in
`contribution/state.rs`; the frame sample and the capture stamp in `runtime/submitted_capture.rs` carry alpha
and the lead. `examples/v1-game/src/host/world.rs` and `world_thread.rs`, `crates/thrum-app/src/sky.rs` and the
test hosts implement the trait. `examples/v1-game/src/window/capture.rs` writes the time record, and the iPhone
app's `ios/capture.rs` writes its own and must still build.
`examples/v1-game/tests/clouds_display_time_gpu.rs` drives a host at scripted times.

Add to those files and change nothing a reader of alpha gets. `thrum-clouds`, `thrum-sea` and the game's camera
keep reading alpha here: `CLOUDS` M2 is the first reader, in its own crate. If a frame's tick and its clock
sample can disagree on a threaded world, so that M4.2 cannot hold as written, or your change fails an exact test
or baseline another module owns, write what you found in the log and end `BLOCKED`; do not edit the other
module's test.
