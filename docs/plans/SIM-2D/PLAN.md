# SIM-2D plan

A game's rules give the same world on every device, and the game finds neighbours and hits among thousands of
bodies each tick. [Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[module list](../ARCHITECTURE.md)

Read [Mechanisms](DESIGN.md#mechanisms), [Proof and builder handoff](DESIGN.md#proof-and-builder-handoff) and
[Exact baselines](../ARCHITECTURE.md#exact-baselines) before starting any milestone.
[The planning probe](../../evidence/SIM-2D/planning-probe.md) says what was measured before the design was written;
its code was thrown away on purpose, so build from the design.

## Open for the human

- Assumption: SIM-2D waits for the V1 exit (`SHOWCASE` M8), because the owner's page puts the game after V1 ("Once V1 lands, the next goal is an original game"). It shares no file with the renderer, so it could start sooner if the owner says; each landing would still pay one exact-baseline verifier run, because it edits `crates/`.
- Assumption: "thousands" is proved at 2,000, 5,000 and 10,000 bodies, and 10,000 is the largest crowd claimed.
- Assumption: the devices are the Mac, Chrome and the iPhone simulator. The simulator shares the Mac's processor, so it proves only that the rules build and run there. A physical iPhone, an x86 PC, Android and other browsers are designed for and not run.
- Assumption: bodies are circles. Boxes and capsules are shapes a game can ask about, and no rigid-body physics (stacking, joints, spinning) is built.
- Assumption: a world is finite. Positions keep millimetre resolution within 16 km of the origin at one unit per metre, queries are refused beyond 16,777,216 units, and an endless map is not built.
- Assumption: devices agree for one build of one rule set. A recording made on one engine version is not promised to replay on a later one.

## M1 A crowd of 10,000, stepped from one recorded trace, holds the same state tick for tick on the Mac, in Chrome and on the iPhone simulator

Criteria: M1.1–M1.12 in the done-when file.

The crowd steps through `thrum_core::sim`'s registered runtime and its history, which G2S left proven; the V1
game in `examples/v1-game/src/simulation.rs` shows one game using them and is not to be copied for its `f64`
rules or its `std::hash` state hash. `InputFrame`, with raw `f32` axes, is in
`crates/thrum-core/src/sim/input.rs`. The input wire's codec is in `crates/thrum-network/src/input_batch.rs`
(`encode`, `decode`, `canonical_axes`), with its byte oracles in that crate's tests; it belongs to `SESSION`, so
use it and leave it as it is. New packages are registered in `.config/architecture.toml`.

The order matters in one place: retain the digests from the straight Mac run first, then make every target
reproduce them while rewinding. Record costs last, at the commit being claimed.

## M2 A game finds overlaps, neighbours and the nearest body among 10,000, and every answer matches a brute-force search

Criteria: M2.1–M2.12 in the done-when file.

M1 left circle overlap and nearest-k in `thrum-sim2d`, and the Crowd trace with its retained file, which this
milestone does not touch. The Mixed trace is new and has its own retained file.

## M3 A rule that would make devices disagree fails the project's check, and a game maker can write and prove rules from the guide

Criteria: M3.1–M3.9 in the done-when file.

The repository guard is `scripts/repo_check.py`, which reads `.config/architecture.toml`; rule packages are a new
kind of entry in it. The maturity ledger is `docs/design/maturity.json`.

Stop rules, for every milestone:
- If the three targets cannot be made equal inside the profile's operations, keep the first differing tick, the component and both values, and end `BLOCKED`. Do not widen the profile, compare fewer values or drop a target.
- A difference that comes from `thrum-core`'s own tick is fixed in place only when G2S's tests and every existing configuration still pass unchanged. Otherwise write the proposal in the log and end `BLOCKED needs_dependency`.
- The V1 game stays outside the profile. Leave its rules, tests and exact comparisons as they are.
- A retained trace's digests change only when its rules change on purpose. Record the cause and the old and new fold in the log; from M2 on, the Crowd trace's do not change at all.
- No time and no allocation count stops the build (brief, decision 3). Record a row that trips a tripwire as measured, note what you found in the log, and keep building. Do not thin the crowd or shorten the trace to improve a row.
- Record only runs of the commit being claimed, with the load on the machine.
- Chrome and the simulator start outside the Codex sandbox. If a launch is refused, leave the item open and say so; do not substitute Node or another target's result.
- Run the exact-baseline verifier before `land`, and again when `land` refuses a rebased commit for lack of a proof.
