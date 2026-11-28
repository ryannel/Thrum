# SIM-3D plan

A 3D game's characters and shots find what is near them, what a ray meets and where they are stopped, the same
on every device, and a character walks over hills, steps and solid models.
[Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-7-the-game-sdk)

Read [Mechanisms](DESIGN.md#mechanisms) and [Proof and builder handoff](DESIGN.md#proof-and-builder-handoff)
before any work package. [The planning probe](../../../.orchestrate/evidence/SIM-3D/planning-probe.md) says what
was measured before the design was written; its code is a stand-in, so build from the design.

## Open for the user

- Assumption: shapes are spheres, capsules and cuboids, and a level's fixed collision is models' triangles, terrain heights and boxes, as the game SDK's design names them. A cuboid can be turned and scaled, but nothing spins or is pushed: no rigid bodies.
- Assumption: a model's triangles and a terrain stop a shape from both sides and have no inside. A box is solid through.
- Assumption: the sizes proved are 10,000 colliders with up to 2,000 of them asking each tick, V1's representative scene, and worlds within 32,768 units of the origin, which holds the showcase's 36 km terrain. A question asked farther out is refused, because a contact there would be coarser than the game is told it is.
- Assumption: a shape stops within about 1.5 cm of what it meets (1/64 of a unit at most), and a walking character rests that near the ground. The exact figure is measured and set by the first work package.
- Assumption: a character's move covers at most 4,096 units in one tick. Farther than that is a teleport, which the game writes.
- Assumption: a character that climbs a step, or is set back down on the ground, covers the climb at one even speed, within the time its travel over the step took. So a trigger on the way up or down is met at a moment and at a speed, and a plain slide or stop keeps the times the game SDK's design gives.
- Assumption: a character rides about 1 cm (three quarters of the stopping distance above) off the ground and walls it moves along, so the joins between floor pieces never stop it. In a gap less than about 2 cm wider than the character it may stop at a join.
- Assumption: the devices are the Mac, Chrome and the iPhone simulator, and devices agree for one build of one rule set, as in `SIM-2D`. The simulator proves the code runs there, never speed.

## WP1 A rule asks whether two 3D shapes touch, where a ray meets one and where a moving one is stopped, and gets the same answer on the Mac, in Chrome and on the iPhone simulator

Items: WP1.1 to WP1.11 in the done-when file.

`thrum_core::profile` has `Number`, a 2-vector, `StateEncode` and `StateDecode`, and the keyed random function
`random_u64`; it has no literal constructor, so a constant is made from integers and divisions.
`crates/thrum-sim2d` and its tests show the house style for a query crate. The new crate is
`crates/thrum-sim3d`, registered in `.config/architecture.toml` with `rule_package = true` and `thrum-core` as
its one dependency. `scripts/sim2d_profile.py` then finds it by itself, copies the profile's `clippy.toml` into
it with `sync`, and lints it with its tests on three targets. That holds the crate's tests to the profile too:
no raw floats, no thread types, no `#[allow]`, no dependency for tests and, once `GAME-SDK` WP2 lands, no file
reads. So a test that
reads the retained vectors, and from WP2 the one that counts allocations, lives in `examples/sim3d-run`, the
package whose programs `scripts/sim3d_conformance.py` runs on each target. `examples/crowd-2d-run`,
`scripts/sim2d_conformance.py`, `scripts/sim2d_browser.mjs` and `scripts/simulator_lifecycle.py` show how
`SIM-2D` runs one program natively, as Wasm in Chrome and in the simulator, each under its own lock. They are
`SIM-2D`'s: call them or copy their shape, and change none of them.

The order matters in two places. WP1.1's record sets the contact tolerance before the tests that use it are
fixed. And the vectors are retained from the Mac's release build first, then every other build and target
reproduces them.

## WP2 A rule finds what is near a point, along a ray and in a moving shape's path among 10,000 colliders, and every answer matches a brute-force search

Items: WP2.1 to WP2.9 in the done-when file.

WP1 left the pair tests, the constants and the conformance command. This work package adds
[the tree](DESIGN.md#3-one-tree-used-three-times) and [the index](DESIGN.md#4-the-collider-index) over them
and changes no pair test: WP1's retained vectors stay as they are and the index's are added beside them. Write
the tree so that WP3 can put triangles and pieces under it as it puts colliders here.

## WP3 A level built from models' triangles, a terrain's heights and boxes answers the same questions and the height of the ground, and opening a gate builds only the gate

## WP4 A character walks over hills, climbs steps, is stopped by walls and steep slopes, keeps to the ground downhill, and says what it touched

## WP5 A walk through a small 3D world steps the same on the Mac, in Chrome and on the iPhone simulator, its cost is on record, and a game maker can use 3D queries from the guide
