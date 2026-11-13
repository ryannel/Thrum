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
- Assumption: a world is finite. Positions keep millimetre resolution within 16 km of the origin at one unit per metre, and an endless map is not built.
- Assumption: devices agree for one build of one rule set. A recording made on one engine version is not promised to replay on a later one.
- Assumption: no budget is set for the crowd's tick. Costs are recorded, and the one hard line is a tick at 10,000 bodies that takes longer than its own 16.7 ms period.

## M1 A crowd of 10,000, stepped from one recorded trace, holds the same state tick for tick on the Mac, in Chrome and on the iPhone simulator

Criteria: M1.1–M1.11 in the done-when file.

- [ ] Add the profile number and its 2-vector to `thrum-core`, with retained vectors for every operation, and the random function with its published test vectors.
- [ ] Add the state encoding and the world digest over the design's coverage list, with one partial digest per component for diagnosis, and the refusal of a non-finite stored value before commit.
- [ ] Create `thrum-sim2d` with the index, circle overlap and nearest-k in the design's order. Register it in `.config/architecture.toml`.
- [ ] Write the reference crowd in `examples/crowd-2d` on the registered runtime, with history, and its Crowd trace with scheduled rewinds.
- [ ] Write the conformance command for the three targets. Retain the digests from a straight Mac run with the corrected inputs and no rewinds, then make every target reproduce them while rewinding.
- [ ] Add the control and show it diverging in Chrome.
- [ ] Record the costs on the Mac and in Chrome, twice each, and put a slice of the trace in the Mac's test command.

## M2 A game finds overlaps, neighbours and the nearest body among 10,000, and every answer matches a brute-force search

Criteria: M2.1–M2.11 in the done-when file.

- [ ] Add box and capsule overlaps and the oversize list, keeping the stated order.
- [ ] Write the brute-force oracle and the hand-worked predicate cases, and show a warmed build and query allocating nothing.
- [ ] Add the Mixed trace, with oversize bodies and projectiles that would tunnel without the capsule, and retain its digests. Leave the Crowd trace's retained file untouched.
- [ ] Run the crowd's steering in the engine's parallel chunks and prove both traces' digests with forced-parallel execution.
- [ ] Record the costs for both traces, serial and forced-parallel on the Mac and serial in Chrome, with time per neighbour visited.

## M3 A rule that would make devices disagree fails the project's check, and a game maker can write and prove rules from the guide

Criteria: M3.1–M3.9 in the done-when file.

- [ ] Add rule packages to the registry and the guard: allowed dependencies, the profile's one Clippy configuration, and no suppression. Register the reference crowd.
- [ ] Add one failing fixture for each class the design names, and the command that runs them.
- [ ] Make the conformance command take any registered rule package, with the control as its second user.
- [ ] Write the guide and link it from the creator guide.
- [ ] Bring the maturity ledger's W1 and P3 rows up to date, and rerun the three targets at the closing commit.

Stop rules, for every milestone:
- If the three targets cannot be made equal inside the profile's operations, keep the first differing tick, the component and both values, and end `BLOCKED`. Do not widen the profile, compare fewer values or drop a target.
- A difference that comes from `thrum-core`'s own tick is fixed in place only when G2S's tests and every existing configuration still pass unchanged. Otherwise write the proposal in the log and end `BLOCKED needs_dependency`.
- The V1 game stays outside the profile. Leave its rules, tests and exact comparisons as they are.
- A retained trace's digests change only when its rules change on purpose. Record the cause and the old and new fold in the log; from M2 on, the Crowd trace's do not change at all.
- If the cost command fails on the 16.7 ms line, record the rows and end `BLOCKED`. Do not thin the crowd, shorten the trace or raise the line.
- Record only runs of the commit being claimed, with the load on the machine.
- Chrome and the simulator start outside the Codex sandbox. If a launch is refused, leave the item open and say so; do not substitute Node or another target's result.
- Run the exact-baseline verifier before `land`, and again when `land` refuses a rebased commit for lack of a proof.
