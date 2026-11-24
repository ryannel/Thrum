# EFFECT-GRAPH plan

Engine modules run their own simulation and drawing passes inside the one frame: the foundation `SEA` and `CLOUDS`
build on. [Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[module list](../ARCHITECTURE.md)

Read [Shared files and exact baselines](DESIGN.md#shared-files-and-exact-baselines),
[Proof and builder handoff](DESIGN.md#proof-and-builder-handoff) and
[Exact baselines](../ARCHITECTURE.md#exact-baselines) before starting any milestone.

## Open for the human

- Assumption: aim for about 2 s per fast test when run alone on a quiet Mac. The owner can revise this target.
- Assumption: this foundation is built now, ahead of the owner's decision on when the sea and cloud port starts; the port decision says to keep the foundation converging on readiness, and no sea or cloud pass is built here.
- Assumption: the atmosphere's existing passes and textures are not moved onto the new seam or the new texture kinds. They keep their exact output, and move later only for a measured reason.
- Assumption: the diagnostic effect stays in the V1 game example behind a selection that is off by default, as a test fixture; it is never part of the showcase.
- Assumption: WP1 starts as soon as this design passes and does not wait for `BASELINE-GUARD` WP1. Until that gate exists, WP1's own check covers the Mac baseline only and Chrome's is rechecked when the gate's builder first runs the verifier on main. The other choice is to add `BASELINE-GUARD:WP1` to WP1's `needs`, which holds WP1 until the gate is built. WP2 and WP3 do wait for it.
- Assumption: this foundation proves that an effect's motion, reactive coverage and reset reach reconstruction's inputs, and does not wait for TAAU. How an effect looks after reconstruction is judged in `SEA`'s and `CLOUDS`' own milestones.
- Assumption: WP3 does not wait for `CULLING` WP3 to stop the game's image varying under GPU load. Until then an exact-image run that fails only along triangle edges is recorded, checked by a reviewer and run again, and WP3 lands on a complete pass. The other choice adds `CULLING:WP3` to WP3's `depends_on`, which holds WP3 and the six modules waiting on it until that milestone is Done.
- Assumption: WP4 gives effects the display phase and leaves the sea's waves, 2D sprites and a game's camera and entities on the interpolation alpha. Under a display lead those step unevenly by the same arithmetic, which nobody has captured yet. Each is its owner's to plan, the camera and entities with the game contract in `HOST` WP4.
- Assumption: a frame on a world with its own thread shows a tick that was due by the frame's observation. So a delayed frame in the Mac window, a Mac capture or the iPhone app shows an earlier tick than it did before WP4, with a lead or without, where WP4's first plan said no frame without a lead would change. The other choice keeps every frame's tick and leaves those frames' display time wrong by up to the delay.
- Assumption: the phase describes the target the clock consumes, which is the frame's observed time plus its lead. WP4 makes the offscreen capture runner measure that lead from the same clock read it gives the world, so its captures consume the platform's target exactly. The Mac window and the iPhone keep reading two clocks a frame, so there the two targets differ by the time between the reads, which nobody has measured. The other choice pairs the two clocks once in both platforms' frame loops, with no WP4 item able to prove it.
- Assumption: after a pause or an overload, the first moving frame steps display time forward once, by the time its tick took to arrive plus the lead: 38 ms in the planner's run with a 30 ms lead. Alpha has always waited for the first tick after a resume, and WP4 leaves that. The other choice interpolates through that wait, which changes when every reader of alpha starts moving after a pause.

## WP1 An engine module can declare volume, cube and mipped-array textures in the frame, and wrong declarations are refused: done
[review](../../../.orchestrate/evidence/EFFECT-GRAPH/WP1-review.md)

## WP2 A test effect keeps its simulation in step with the world through dropped frames, pauses and a replaced device, on Mac and in Chrome: done
[review](../../../.orchestrate/evidence/EFFECT-GRAPH/WP2-review.md)

## WP3 A test effect draws through the scene as water or cloud would, hidden by nearer objects, and hands its motion to reconstruction, on Mac and in Chrome: done
[review](../../../.orchestrate/evidence/EFFECT-GRAPH/WP3-review.md)

## WP4 A test effect that drifts with time moves by even steps at the display's refresh, between ticks and past the newest one

[The display phase](DESIGN.md#the-display-phase-m4) is the design. [The timing model](DESIGN.md#the-timing-model)
in it holds every rule about a frame's time, and [the ruling](../ARCHITECTURE.md#seam-ruling-for-effect-graph-m4)
says whose files it is built in.

WP4's first two builds are on main: the phase in `crates/thrum-core/src/clock.rs`, the world lane with its
refused pair and the clocked world; `FrameWorld`, `EffectTime`, the frame sample and the capture stamp in
`thrum-render`; one clock read in `Offscreen::step_for`; the capture time record; the ledger row.
[Its second review](../../../.orchestrate/evidence/EFFECT-GRAPH/WP4-review.md) passed five items and failed WP4.2 on a frame after
a resume. The fault was the plan's: it wrote display time as a time on the host's clock. The items changed, so
every one is proved again.

The clock and the lane already keep the model
([the planner's check](../../../.orchestrate/evidence/EFFECT-GRAPH/WP4-timing/README.md)), so no phase, alpha or pair that a
frame is given today should change. What is missing is the due time of a tick, which `ClockSample` holds as
`last_time` and `fractional_tick` and does not state; its way to the frame's sample and the capture record,
through every implementer of `FrameWorld`; and tests through a stop. `spans.rs` and `lane_lifecycle.rs`
beside that check are those tests in outline, and so are your six frames in
`.orchestrate/evidence/EFFECT-GRAPH/WP4-resume/probe.rs` at `2f3f2db75` on `build/effect-graph`. C02 carries the
model's words already, written with this plan: change them only where WP4.4 would otherwise fail.

`StillWorld` in `crates/thrum-app/src/sky.rs` reports a phase of 1 and a clock that is not interpolating even
when it has a tick duration and its alpha moves, as it does in `examples/water`. That is WP4.10. Its alpha
feeds the sea's exact images and must not move by a bit.

`crates/thrum-app/src/app/render_actor.rs` and `examples/v1-game-ios/src/ios/render.rs` are not yours to
change. `thrum-clouds`, `thrum-sea` and the game's camera keep reading alpha, and effect time gains no field.
A test fixture in another module's crate that implements `FrameWorld` takes what the trait now asks and
nothing more. The snapshot ring in `crates/thrum-core/src/sim/snapshot_ring.rs` has four pages: if your runs
skip publications at 60 or 120 frames a second, write what you found in the log. If your change fails an exact
test or baseline another module owns, write what you found in the log and end `BLOCKED`; do not edit the
other module's test.

## WP5 The world lane’s resume origin is proved without real-time pacing

Keep the world-lane path and drive it with controlled time and explicit worker synchronization. Use the
fewest pre-pause, held and resumed observations that still distinguish a moving origin, a too-early
publication and an anchor outside the request/acknowledgment bracket. Replace the paced frames and pauses
with the same logical gap and pause cases on simulated time.

Record each listed test’s disposition and the coverage that replaces it if split. For each fast case,
record before and after per-test times on the Mac in the default nextest test build used by the workspace
check, excluding compilation. Measure the revised cases alone and run them as a focused batch alongside
normal builder work. Use the supplied run for the starting times where available. The batch passes the
existing ten-second limit on its first run.

These are the cases from the passing `747909d1c` workspace check, with seconds from that run.

- 6.213 s — `thrum-core::it clock_spans::lane::a_resumed_lane_keeps_one_origin_inside_the_resume_bracket`
