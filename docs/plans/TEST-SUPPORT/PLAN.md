# TEST-SUPPORT plan

Engine tests get a GPU device from one place, and a test that needs a GPU says so and is skipped with a reason
where there is none. [Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-4-the-code-review-and-the-games-separation)

Read [Mechanisms](DESIGN.md#mechanisms), [the files each milestone touches](DESIGN.md#files-each-milestone-touches)
and [Exact baselines](../ARCHITECTURE.md#exact-baselines) before any milestone. The milestones are in build order:
M5 was added after M3 and M4 and is built before them.

## Open for the human

- Assumption: on Linux and Windows, where nothing is qualified, a machine with no GPU skips GPU tests with a visible reason and still passes. On this Mac a missing GPU fails the run.
- Assumption: test files are not all moved to one layout. New tests follow the adopted layout, and an old file moves when a task already touches it.

## M1 A GPU test in the sea, asset and cooker crates gets its device from one helper, and with no GPU it is skipped with a line anyone running the tests sees, or fails when a GPU is required: done

[review](../../../.orchestrate/evidence/TEST-SUPPORT/M1-review.md)

## M2 Every GPU test in the engine goes through the gate, the graph tests that need no GPU run on every platform, and no test shares code by `#[path]`

This waits for `HOST` M1, so that the device is the recipe's, for the milestones other builders hold in the
renderer's tests, and for `GAME-SPLIT` M2, which removes the `#[path]` includes of game source; what is left is
sharing inside a crate. Record `cargo test --workspace`'s results before you start.

## M5 The test kit's readback, device call and wait exist, and the check refuses a new hand-written readback or deadline loop in a test

This is the first half of [the kit](DESIGN.md#the-kit): its functions and the two rules, with no test converted.
Each finding's sites are in the cleanup review's `results.json` and `verified/` folder, at the path the design
gives; read them to shape the functions, because M3 has to fit every site to them. `crates/thrum-render/src/test_device.rs`
already opens the recipe's portable device for the crate's unit tests, after a separate call to the gate. The rule
engine is `scripts/shape_rules.py` with its matchers in `scripts/shape_matchers.py`, and M1 added
`test-crate-edges` the same way. The engine's own suites run in no check until `DEAD-TOOLS` M1 lands, so run them
yourself. `DEAD-TOOLS` M6 adds four rules to the same files.

## M3 A GPU test reads back, opens its device and waits through the test kit, not through code copied into its file

This is the conversion. Record the results of `cargo test --workspace` before you start, as M2 did, and note
which run of the main-branch check `long-tests` was the last before you started: M3.6 compares against it.
`examples/water/tests/floating_gpu.rs` and `foam_scene_gpu.rs` hold long tests, and the renderer's
`runtime/tests/device.rs` and `tests/ray_instances_seam.rs` hold tests behind `creation-counters`; `./dev check`
compiles those and runs neither kind. `REVIEW-RENDER` M5.1 and `HOST` M3.2 run `frame_alloc`, which is the game's
and is left alone. `HOST` M3 reworks frame capture, so keep any capture helper thin.

## M4 Each crate's WGSL records are held to its Rust structs by one shared check, and the proving ground's cooked tests build their game through one fixture

`REVIEW-RENDER` M2 owns the renderer's checker in `crates/thrum-render/tests/wgsl_records.rs` and its plan
already asks for one shape shared with the sea, so this waits for it and starts from what it left. The cooked
tests are `cooked_assets_gpu.rs` and `cooked_material_gpu.rs` in `examples/v1-game`: their two device openers
differ on purpose. Time the two binaries before you start.
