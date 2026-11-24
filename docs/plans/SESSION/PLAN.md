# SESSION plan

Two to four players share one run and keep seeing the same crowd. [Design](DESIGN.md) ·
[done-when file](DONE.json), which owns the criteria · [log](LOG.md) · [module list](../ARCHITECTURE.md)

Read [Mechanisms](DESIGN.md#mechanisms), [Proof and builder handoff](DESIGN.md#proof-and-builder-handoff) and
[the seam rulings](../ARCHITECTURE.md#seam-rulings-for-session) before starting any milestone, and
[Exact baselines](../ARCHITECTURE.md#exact-baselines) before landing. Follow the [builder rules](DESIGN.md#builder-rules).

## Open for the human

- Assumption: aim for about 2 s per fast test when run alone on a quiet Mac. The owner can revise this target.
- Assumption: a world that steps when a match says is engine work, so it is built here (WP7) with the reference crowd in a window, and not later inside the game. It is the one milestone of this module that needs the GPU, and it waits for `RENDER-2D` WP5.
- Assumption: when a connection hiccups, the crowd on that player's screen holds still, then carries on from where it stopped, up to a tenth faster than normal until it has caught up. A player more than about half a second behind sees the crowd skip forward once instead, with a notice the game can show. Nothing is drawn ahead of what the host has confirmed.
- Assumption: each player sees their own character answer at once, and sees the crowd and the other players a little late: about 80 ms behind the host on a 100 ms connection. Rewinding and re-stepping the whole crowd to hide that delay is not built, because at 10,000 enemies one correction costs about six frames. How the delay feels is yours to judge in the game.
- Assumption: a native process hosts, and the match ends when it stops. Chrome and the iPhone simulator only join. Host migration, matchmaking and accounts are not built.
- Assumption: anyone who has the host's join ticket and the same build can take a free place. There are no passwords, and no way to remove a player.
- Assumption: every player's machine holds the whole arena. A modified client cannot change what the others see, but it can read everything; nothing is hidden and no anti-cheat is built.
- Assumption: a player whose machine or browser tab cannot keep up with the match is dropped from it. The match never slows down for one player.
- Assumption: a player who loses their connection gets their place back if they return within a grace period the game sets; their character stands idle meanwhile.
- Assumption: a match is proved with a host and four players for ten minutes at 10,000 bodies, between separate processes on this Mac, through `TRANSPORT`'s relay. No second machine or internet path is run.
- Assumption: Thrum keeps one dedicated simulator available for reuse and shuts it down at the end of each lock turn.

## WP1 A running crowd of 10,000 is copied into a fresh process on the Mac, in Chrome and on the iPhone simulator, and carries on in step

[Review](../../../.orchestrate/evidence/SESSION/WP1-review.md)

## WP2 Four players on a simulated bad connection steer one crowd of 10,000, and every player's copy stays identical

[Review](../../../.orchestrate/evidence/SESSION/WP2-review.md)

## WP3 A player joins mid-game, another drops and returns, a copy that has gone wrong is repaired, and a stuck player never holds the others up

[Review](../../../.orchestrate/evidence/SESSION/WP3-review.md)

## WP4 A player's own character answers at once, while the crowd stays exact

[Review](../../../.orchestrate/evidence/SESSION/WP4-review.md)

## WP8 A small in-memory match catches reliable-lane recovery regressions

Use the [fixture design](DESIGN.md#reliable-lane-recovery-fixture-m8) for the cases in the
[workspace case list and baseline times](LOG.md#m8-workspace-timing-baseline).

Record each case’s disposition, any replacement coverage and its before-and-after time in the workspace
check’s default Nextest build on the Mac, excluding compilation. Measure the fast cases alone and in a
focused batch under normal builder load. The batch passes the existing ten-second limit on its first run.

## WP5 Players on a Mac, in Chrome and on an iPhone simulator play one match over real connections, through delay, jitter and loss

Criteria: WP5.1–WP5.14 in the done-when file.

`TRANSPORT` left the three socket adapters, the relay and its proof programs in `examples/link-proof` with
`scripts/transport_link.py`, which show how each target is started and how a page reaches the host. Use the relay.

Adopt the [shared simulator lifecycle](DESIGN.md#simulator-lifecycle-in-the-proof-harnesses-m5) before
rerunning the socket proofs. Limit changes to other proof commands, including `scripts/transport_link.py`, to
simulator admission, clean starting state and cleanup. Use the short device proof to verify cleanup, the
CPU record to verify outer lock release on cancellation and presentation timeout, and the existing socket
fault run to verify the live peer restart on one boot. Set `ORCHESTRATE_SCRIPT` to the installed
`agent_loop.py` and run `python3 scripts/session_match.py image --target ios-sim` once as WP5 evidence. The
command manages its own admission and records the full comparison across eight simulator turns.

## WP6 A game maker can put a game in a match from the guide, and a game without multiplayer contains none of it

Criteria: WP6.1–WP6.8 in the done-when file.

The owning design is `docs/design/modules/NETWORKING.md`, the contract is C13 in `docs/design/CONTRACTS.md`, the
decision records are the A0 and A5 files in `docs/design/decisions/`, and the maturity ledger is
`docs/design/maturity.json`.

## WP7 The reference crowd is played in a window by a host and by clients on the Mac and in Chrome, each world stepped by its match

Criteria: WP7.1–WP7.9 in the done-when file.

The world lane is `crates/thrum-core/src/sim/world_lane.rs` with `clocked_world.rs`, and the frame host reads a
world through `FrameWorld` in `crates/thrum-render/src/runtime.rs`; the lane's task already owns its clock, so
read how far that carries a driven world before changing anything under seam ruling 3. The observer already
evaluates a clock sample at each frame's display time (`presentation_alpha_at`), which is the route a driver's
position takes. `TRANSPORT`'s relay can cut a path for a time, and the sets lost in a cut arrive together in
the next datagram: that is how WP7.5's three cases are made. `RENDER-2D` left the
crowd view in `examples/crowd-2d-view`, a program on `thrum-app`'s window and page; the windowed match is that
view with a session attached, in a package of its own so the view still links no network crate.
