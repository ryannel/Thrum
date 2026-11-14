# TEST-SUPPORT plan

Engine tests get a GPU device from one place, and a test that needs a GPU says so and is skipped with a reason
where there is none. [Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-4-the-code-review-and-the-games-separation)

Read [Mechanisms](DESIGN.md#mechanisms) and [Exact baselines](../ARCHITECTURE.md#exact-baselines) before any
milestone.

## Open for the human

- Assumption: on Linux and Windows, where nothing is qualified, a machine with no GPU skips GPU tests with a printed reason and still passes. On this Mac a missing GPU fails the run.
- Assumption: test files are not all moved to one layout. New tests follow the adopted layout, and an old file moves when a task already touches it.

## M1 A GPU test in the sea, asset and cooker crates gets its device from one helper, and with no GPU it is skipped with a printed reason, or fails when a GPU is required

The closest thing to a shared helper today is `device()` in `crates/thrum-render/tests/graph_kinds_support/support.rs`.
`thrum-sea`, `thrum-asset-gpu` and `thrum-cook` have no builder at work, so adoption starts there. `REVIEW-ENGINE`
M2 and M3 edit the same crates' tests and may be running: integrate on what has landed. The `dev-only-crates` rule
is added through `REPO-RULES`' rule engine with its fixtures.

## M2 Every GPU test in the engine goes through the gate, the graph tests that need no GPU run on every platform, and no test shares code by `#[path]`

This waits for `HOST` M1, so that the device is the recipe's, and for the milestones other builders hold in the
renderer's tests. Take the test list before you start. `GAME-SPLIT` M2 has by then removed the `#[path]` includes
of game source; what is left is sharing inside a crate.
