# EFFECT-GRAPH plan

Engine modules run their own simulation and drawing passes inside the one frame: the foundation `SEA` and `CLOUDS`
build on. [Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[module list](../ARCHITECTURE.md)

Read [Shared files and exact baselines](DESIGN.md#shared-files-and-exact-baselines),
[Proof and builder handoff](DESIGN.md#proof-and-builder-handoff) and
[Exact baselines](../ARCHITECTURE.md#exact-baselines) before starting any work package.

## Open for the human

- Assumption: aim for about 2 s per fast test when run alone on a quiet Mac. The owner can revise this target.
- Assumption: this foundation is built now, ahead of the owner's decision on when the sea and cloud port starts; the port decision says to keep the foundation converging on readiness, and no sea or cloud pass is built here.
- Assumption: the atmosphere's existing passes and textures are not moved onto the new seam or the new texture kinds. They keep their exact output, and move later only for a measured reason.
- Assumption: the diagnostic effect stays in the V1 game example behind a selection that is off by default, as a test fixture; it is never part of the showcase.
- Assumption: WP1 starts as soon as this design passes and does not wait for `BASELINE-GUARD` WP1. Until that gate exists, WP1's own check covers the Mac baseline only and Chrome's is rechecked when the gate's builder first runs the verifier on main. The other choice is to add `BASELINE-GUARD:WP1` to WP1's `needs`, which holds WP1 until the gate is built. WP2 and WP3 do wait for it.
- Assumption: this foundation proves that an effect's motion, reactive coverage and reset reach reconstruction's inputs, and does not wait for TAAU. How an effect looks after reconstruction is judged in `SEA`'s and `CLOUDS`' own work packages.
- Assumption: WP3 does not wait for `CULLING` WP3 to stop the game's image varying under GPU load. Until then an exact-image run that fails only along triangle edges is recorded, checked by a reviewer and run again, and WP3 lands on a complete pass. The other choice adds `CULLING:WP3` to WP3's `depends_on`, which holds WP3 and the six modules waiting on it until that work package is Done.
- Assumption: WP4 gives effects the display phase and leaves the sea's waves, 2D sprites and a game's camera and entities on the interpolation alpha. Under a display lead those step unevenly by the same arithmetic, which nobody has captured yet. Each is its owner's to plan, the camera and entities with the game contract in `HOST` WP4.
- Assumption: a frame on a world with its own thread shows a tick that was due by the frame's observation. So a delayed frame in the Mac window, a Mac capture or the iPhone app shows an earlier tick than it did before WP4, with a lead or without, where WP4's first plan said no frame without a lead would change. The other choice keeps every frame's tick and leaves those frames' display time wrong by up to the delay.
- Assumption: the phase describes the target the clock consumes, which is the frame's observed time plus its lead. WP4 makes the offscreen capture runner measure that lead from the same clock read it gives the world, so its captures consume the platform's target exactly. The Mac window and the iPhone keep reading two clocks a frame, so there the two targets differ by the time between the reads, which nobody has measured. The other choice pairs the two clocks once in both platforms' frame loops, with no WP4 item able to prove it.
- Assumption: the tick stage answers questions from the state frames leave, and steps no simulation of its own. Wakes, foam and clouds still advance only in drawn frames, so they stand still while a window is hidden. The other choice steps effect state every tick with no frame drawn, which nothing has asked for.
- Assumption: after a restore, a new world or a replaced device, an effect's answers to a game say they had no state to answer from, or do not come, until the first frame of that world is drawn. While frames are being skipped that lasts as long as the skipping. The other choice resets an effect's state with no frame drawn, which is the same work as stepping it there and which nothing has asked for.
- Assumption: WP6 proves the iPhone's part, that no tick pass is submitted outside the foreground lease, by tests of the lease that run on the Mac and a reviewer's reading of the render owner. It does not run a tick pass on the iPhone simulator, because no scene the iPhone app runs has one yet. The other choice adds a simulator route with a backgrounding step to WP6.
- Assumption: after a pause or an overload, the first moving frame steps display time forward once, by the time its tick took to arrive plus the lead: 38 ms in the planner's run with a 30 ms lead. Alpha has always waited for the first tick after a resume, and WP4 leaves that. The other choice interpolates through that wait, which changes when every reader of alpha starts moving after a pause.

## WP1 An engine module can declare volume, cube and mipped-array textures in the frame, and wrong declarations are refused: done
[review](../../../.orchestrate/evidence/EFFECT-GRAPH/WP1-review.md)

## WP2 A test effect keeps its simulation in step with the world through dropped frames, pauses and a replaced device, on Mac and in Chrome: done
[review](../../../.orchestrate/evidence/EFFECT-GRAPH/WP2-review.md)

## WP3 A test effect draws through the scene as water or cloud would, hidden by nearer objects, and hands its motion to reconstruction, on Mac and in Chrome: done
[review](../../../.orchestrate/evidence/EFFECT-GRAPH/WP3-review.md)

## WP4 A test effect that drifts with time moves by even steps at the display's refresh, between ticks and past the newest one: done
[review](../../../.orchestrate/evidence/EFFECT-GRAPH/WP4-review.md)

## WP5 The world lane’s resume origin is proved without real-time pacing: done
[review](../../../.orchestrate/evidence/EFFECT-GRAPH/WP5-review.md)

## WP6 A game's question to an effect is answered every tick from the state the last drawn frame left, even when that tick's frame is skipped, on the Mac and in Chrome

[The tick stage](DESIGN.md#the-tick-stage-wp6) is the design, and
[the ruling](../ARCHITECTURE.md#seam-ruling-for-effect-graph-wp6) says whose files it is built in. `WATERS` WP19
waits for this work package and for nothing else.

The body query on main shows the shape a consumer has today: `crates/thrum-water/src/query/gpu.rs` stages its
inputs in `encode_upload`, dispatches in a `simulate` pass, copies in `encode_effect_readback` and starts its
map in `submitted`. Leave it there. Moving it is WP19's, on its own candidate, and `thrum-water` builds and
passes its tests unchanged when you land.

Two shells change, each in one place, and nothing else in either:
[the Mac's render actor](DESIGN.md#the-tick-stage-wp6) makes the asking before its wait for a frame slot, and
the iPhone's render owner gives its host a submission permission made of its lease. Leave the wait's length,
the lead, the lease's protocol and every call into the host as they are. A game states a post's endpoint, so
the test contribution's posts and the diagnostic effect's carry one.

`runtime/tests/without_rendering.rs` holds `advance_without_rendering` to touching no device, and those tests
stand. `APPLE-PRESENTATION` WP16 is not built and will hold a host to one timestamp query set for each frame
slot, so add none. Other builders are in the frame host's files: integrate on top of what has landed.
`contribution.rs`'s module documentation says a contribution runs only inside the frame, and the seam's row in
the maturity ledger says what the seam offers: bring both up to date. If a rule of the stage cannot be built
as written, write what you found in the log and end `BLOCKED`.
