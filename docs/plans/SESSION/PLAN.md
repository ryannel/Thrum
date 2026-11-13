# SESSION plan

Two to four players share one run and keep seeing the same crowd. [Design](DESIGN.md) ·
[done-when file](DONE.json), which owns the criteria · [log](LOG.md) · [module list](../ARCHITECTURE.md)

Read [Mechanisms](DESIGN.md#mechanisms), [Proof and builder handoff](DESIGN.md#proof-and-builder-handoff) and
[the seam rulings](../ARCHITECTURE.md#seam-rulings-for-session) before starting any milestone, and
[Exact baselines](../ARCHITECTURE.md#exact-baselines) before landing.
[The planning probe](../../evidence/SESSION/planning-probe.md) is a timing model with no engine code; its source
was thrown away on purpose, so build from the design.

## Open for the human

- Assumption: SESSION waits for the V1 exit, because the modules it stands on do: `SIM-2D` and `TRANSPORT` both wait for `SHOWCASE` M8, since the owner's page puts the game after V1.
- Assumption: each player sees their own character answer at once, and sees the crowd and the other players a little late: about 80 ms behind the host on a 100 ms connection. Rewinding and re-stepping the whole crowd to hide that delay is not built, because at 10,000 enemies one correction costs about six frames. How the delay feels is yours to judge in the game.
- Assumption: a native process hosts, and the match ends when it stops. Chrome and the iPhone simulator only join. Host migration, matchmaking and accounts are not built.
- Assumption: anyone who has the host's join ticket and the same build can take a free place. There are no passwords, and no way to remove a player.
- Assumption: every player's machine holds the whole arena. A modified client cannot change what the others see, but it can read everything; nothing is hidden and no anti-cheat is built.
- Assumption: a player whose machine or browser tab cannot keep up with the match is dropped from it. The match never slows down for one player.
- Assumption: a player who loses their connection gets their place back if they return within a grace period the game sets; their character stands idle meanwhile.
- Assumption: a match is proved with a host and four players for ten minutes at 10,000 bodies, between separate processes on this Mac, through `TRANSPORT`'s relay. No second machine or internet path is run.
- Assumption: builders and reviewers ask to run the socket proofs outside the Codex sandbox, one request for each run, as `TRANSPORT`'s do. The other choices are owner decision 11 in the module list.

## M1 A running crowd of 10,000 is copied into a fresh process on the Mac, in Chrome and on the iPhone simulator, and carries on in step

Criteria: M1.1–M1.10 in the done-when file.

`SIM-2D` left the state encoding and the digest in `thrum-core`, the reference crowd in `examples/crowd-2d` with
its runners for the three targets in `crowd-2d-run`, and `scripts/sim2d_conformance.py`. The runtime's capture
and restore in `crates/thrum-core/src/sim/runtime.rs` and `registered_world.rs` work inside one runtime only; the
image is that operation through bytes. This milestone edits `thrum-core` and the crowd's component types under
[seam ruling 1](../ARCHITECTURE.md#seam-rulings-for-session): `SIM-2D`'s items, tests and retained files stay as
they are. Both of those files are already flagged by the repository check for their size, so the image goes in
files of its own. New packages are registered in `.config/architecture.toml`.

## M2 Four players on a simulated bad connection steer one crowd of 10,000, and every player's copy stays identical

Criteria: M2.1–M2.11 in the done-when file.

`thrum-network` holds the envelope, A5's input batch (`input_batch.rs`, used byte for byte), the staging window
(`future_input.rs`) and the state-delta codec, which this module leaves alone. `TRANSPORT` left the link's
interface and the in-memory conditioned link in `thrum-transport`. The session, the match settings and the
headless host's loop go in `thrum-network`; the four-player proofs and the scripted players go in a new package,
`examples/match-proof`; the tests' second rule set stays with `thrum-network`'s tests. The reference crowd's
rules take no actions, which is why commands and refusals are proved on the second rule set. The runtime's
refusal of a step is `TickError` in `crates/thrum-core/src/sim/runtime.rs`; M2.11's failure is that refusal,
raised on the host by the second rule set.

## M3 A player joins mid-game, another drops and returns, a copy that has gone wrong is repaired, and a stuck player never holds the others up

Criteria: M3.1–M3.12 in the done-when file.

The in-memory link can cut a path, stall one and duplicate datagrams; those are the faults to use. A fault that
needs a client's world changed is made by the proof harness, which owns that world. A client that steps slowly
is made by the harness stepping that client's world less often than its session asks, while its link is drained
and its datagrams sent at the full pace.

## M4 A player's own character answers at once, while the crowd stays exact

Criteria: M4.1–M4.7 in the done-when file.

[A5's controller replay](../../design/decisions/A5-CONTROLLER-REPLAY.md) describes the ring; its nine-step cap and
its tolerances are replaced as the design says. Exposing the crowd's player movement as one function edits
`examples/crowd-2d` under [seam ruling 2](../ARCHITECTURE.md#seam-rulings-for-session). `SIM-2D`'s cost record,
which M4.6 quotes, is in `docs/evidence/SIM-2D/`.

## M5 Players on a Mac, in Chrome and on an iPhone simulator play one match over real connections, through delay, jitter and loss

Criteria: M5.1–M5.10 in the done-when file.

`TRANSPORT` left the three socket adapters, the relay and its proof programs in `examples/link-proof` with
`scripts/transport_link.py`, which show how each target is started and how a page reaches the host. Use the
relay; leave `thrum-transport` and its script as they are.

## M6 A game maker can put a game in a match from the guide, and a game without multiplayer contains none of it

Criteria: M6.1–M6.8 in the done-when file.

The owning design is `docs/design/modules/NETWORKING.md`, the contract is C13 in `docs/design/CONTRACTS.md`, the
decision records are the A0 and A5 files in `docs/design/decisions/`, and the maturity ledger is
`docs/design/maturity.json`.

Stop rules, for every milestone:
- If peers cannot be held equal, keep the first differing tick, the component and both digests, and end `BLOCKED`. Do not compare fewer ticks, send state to a client, or drop a target.
- `SIM-2D`'s retained digests do not move. If an image or the predictor's function cannot be built without changing one, write the proposal in the log and end `BLOCKED needs_dependency`.
- Leave `thrum-transport`, `thrum-sim2d` and A5's input-batch bytes as they are. A change one of them needs is a proposal in the log and `BLOCKED needs_dependency`.
- The V1 game stays outside all of this. Leave its rules, tests and exact comparisons as they are.
- Nothing that opens a socket, starts Chrome or calls the simulator runs under `cargo test`'s defaults or in `./dev check`. Those proofs go through the match command, outside the Codex sandbox. If a launch is refused, leave the item open and say so; do not substitute the in-memory link, Node or another target's result.
- No time, byte count or allocation count stops the build (brief, decision 3). Record a row that trips a tripwire as measured, note what you found in the log, and keep building. The one figure that fails a run is the design's 95% floor on ticks that admit a player's own frame.
- Record only runs of the commit being claimed, with the load on the machine.
- Run the exact-baseline verifier before `land`, and again when `land` refuses a rebased commit for lack of a proof.
