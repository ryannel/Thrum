# TEST-SUPPORT plan

Engine tests get a GPU device from one place, and a test that needs a GPU says so and is skipped with a reason
where there is none. [Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-4-the-code-review-and-the-games-separation)

Read [Mechanisms](DESIGN.md#mechanisms) and [Exact baselines](../ARCHITECTURE.md#exact-baselines) before any
milestone.

## Open for the human

- Assumption: on Linux and Windows, where nothing is qualified, a machine with no GPU skips GPU tests with a visible reason and still passes. On this Mac a missing GPU fails the run.
- Assumption: test files are not all moved to one layout. New tests follow the adopted layout, and an old file moves when a task already touches it.

## M1 A GPU test in the sea, asset and cooker crates gets its device from one helper, and with no GPU it is skipped with a line anyone running the tests sees, or fails when a GPU is required

The closest thing to a shared helper today is `device()` in `crates/thrum-render/tests/graph_kinds_support/support.rs`.
`thrum-sea`, `thrum-asset-gpu` and `thrum-cook` have no builder at work, so adoption starts there. `REVIEW-ENGINE`
M2 and M3 edit the same crates' tests and may be running: integrate on what has landed. The `test-crate-edges`
rule is added through `REPO-RULES`' rule engine with its fixtures; `crates/thrum-fixtures` itself is created by
`GAME-SPLIT` M2, so here the rule is proved on scratch manifests. Record the three crates' test results before you
change anything: M1.4 compares against them.

## M2 Every GPU test in the engine goes through the gate, the graph tests that need no GPU run on every platform, and no test shares code by `#[path]`

This waits for `HOST` M1, so that the device is the recipe's, for the milestones other builders hold in the
renderer's tests, and for `GAME-SPLIT` M2, which removes the `#[path]` includes of game source; what is left is
sharing inside a crate. Record `cargo test --workspace`'s results before you start.

## M3 A GPU test reads back, opens its device and waits through the test kit, not through code copied into its file

This is [the kit](DESIGN.md#the-kit), from section 5 of the cleanup review of 20 November; each finding's sites
are in that review's `results.json` and `verified/` folder, at the path the design gives. It waits for `GATE-TIME`
M2 and `DEAD-CODE` M2, which delete duplicate tests and about 140 stub methods in the same files, so there is
less to convert. Record the test results before you start, as M2 did. `REVIEW-RENDER` M5.1 and `HOST` M3.2 run
`frame_alloc`, which is the game's and is left alone. `HOST` M3 reworks frame capture, so keep any capture helper
thin.

## M4 Each crate's WGSL records are held to its Rust structs by one shared check, and the game's cooked tests build their game through one fixture

`REVIEW-RENDER` M2 owns the renderer's checker in `crates/thrum-render/tests/wgsl_records.rs` and its plan
already asks for one shape shared with the sea, so this waits for it and starts from what it left. The cooked
tests are `cooked_assets_gpu.rs` and `cooked_material_gpu.rs` in the game: their two device openers differ on
purpose, and serialising the material tests adds wall time, so say in the log what the fixture did to the run's
length.
