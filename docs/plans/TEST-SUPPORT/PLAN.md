# TEST-SUPPORT plan

Engine tests get a GPU device from one place, and a test that needs a GPU says so and is skipped with a reason
where there is none. [Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-4-the-code-review-and-the-games-separation)

Read [Mechanisms](DESIGN.md#mechanisms), [the files each work package touches](DESIGN.md#files-each-work-package-touches)
and [Exact baselines](../ARCHITECTURE.md#exact-baselines) before any work package. The work packages are in build order:
M5 and WP6 were added after WP3 and WP4 and are built before them.

## Open for the human

- Assumption: on Linux and Windows, where nothing is qualified, a machine with no GPU skips GPU tests with a visible reason and still passes. On this Mac a missing GPU fails the run.
- Assumption: test files are not all moved to one layout. New tests follow the adopted layout, and an old file moves when a task already touches it.
- Assumption: nothing runs the tests on Linux or Windows, so WP2 proves from this Mac that they build there, by a type-check, and proves by reading that no CPU test is hidden and no GPU test fails for want of Metal. Linking, the C and C++ build scripts, doc tests and GPU tests on another backend stay unproved. The other choice is a real run, in a Linux container on this Mac or on GitHub's Ubuntu and Windows runners, which builds the whole workspace each time and, on GitHub, spends runner minutes.
- Assumption: that type-check allows warnings. Off macOS the test code has about 139, mostly helpers that only macOS-gated tests use, so a Linux or Windows `./dev check` still stops at Clippy before it runs a test. The other choice clears them and fails the check on a new one, which every module that writes a macOS-gated test then pays for.

## WP1 A GPU test in the sea, asset and cooker crates gets its device from one helper, and with no GPU it is skipped with a line anyone running the tests sees, or fails when a GPU is required: done

[review](../../../.orchestrate/evidence/TEST-SUPPORT/WP1-review.md)

## WP2 Every GPU test in the engine goes through the gate, the workspace's tests build for Linux and Windows with no CPU test hidden, and no test shares code by `#[path]`: done

[review](../../../.orchestrate/evidence/TEST-SUPPORT/WP2-review.md)

## WP5 The test kit's readback, device call and wait exist, and the check refuses a new hand-written readback or deadline loop in a test: done

[review](../../../.orchestrate/evidence/TEST-SUPPORT/WP5-review.md)

## WP6 A GPU test draws to the test kit's one small target, and the engineering skill says in one short block how to write a test

[Mechanism 8](DESIGN.md#writing-a-test) gives the five lines, the anchors and what an entry says. The kit's
modules are in `crates/thrum-test-support/src/`, and the Tests section of
`.agents/skills/thrum-engineering/references/patterns.md` already has the unit-test layout, the GPU gate and the
kit: add the entries to it and repeat nothing. The limits are the project settings': link to
[the test tiers](../../../ORCHESTRATE.md#test-tiers) and state none. `GATE-TIME` WP12's rules do not exist yet,
so an entry names no rule.

## WP3 A GPU test reads back, opens its device and waits through the test kit, not through code copied into its file

This is the conversion. Its first build merged as `3a4b8874c` and `cc8decdf4` from the starting commit
`4592e88cb`, so both earlier runs are fixed and already saved in `.orchestrate/evidence/TEST-SUPPORT/WP3/`: record
no new ones. `before.stdout` and `before.stderr` are `cargo test --workspace` at `4592e88cb`, for WP3.4, and
`long-tests-baseline.*` is the `long-tests` run at `0870118d6`, for WP3.6. Other modules have removed tests
since: `GATE-TIME` WP2 in `dbe1cac8d`, after the `long-tests` run, and `DEAD-CODE` WP5 in `ac62118e4`, after
both. [Proof](DESIGN.md#proof) says how those count, and the first review's lists of what went missing are in
`.orchestrate/evidence/TEST-SUPPORT/WP3-review/outcome-comparison.json`. Restore none of them.
`examples/water/tests/floating_gpu.rs` and `foam_scene_gpu.rs` hold long tests, and the renderer's
`runtime/tests/device.rs` and `tests/ray_instances_seam.rs` hold tests behind `creation-counters`; `./dev check`
compiles those and runs neither kind. `REVIEW-RENDER` WP5.1 and `HOST` WP3.2 run `frame_alloc`, which is the game's
and is left alone. `HOST` WP3 reworks frame capture, so keep any capture helper thin.

## WP4 Each crate's WGSL records are held to its Rust structs by one shared check, and the proving ground's cooked tests build their game through one fixture

`REVIEW-RENDER` WP2 owns the renderer's checker in `crates/thrum-render/tests/wgsl_records.rs` and its plan
already asks for one shape shared with the sea, so this waits for it and starts from what it left. The cooked
tests are `cooked_assets_gpu.rs` and `cooked_material_gpu.rs` in `examples/v1-game`: their two device openers
differ on purpose. Time the two binaries before you start.
