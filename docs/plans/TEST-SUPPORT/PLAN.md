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
- Assumption: nothing runs the tests on Linux or Windows, so M2 proves from this Mac that they build there, by a type-check, and proves by reading that no CPU test is hidden and no GPU test fails for want of Metal. Linking, the C and C++ build scripts, doc tests and GPU tests on another backend stay unproved. The other choice is a real run, in a Linux container on this Mac or on GitHub's Ubuntu and Windows runners, which builds the whole workspace each time and, on GitHub, spends runner minutes.
- Assumption: that type-check allows warnings. Off macOS the test code has about 139, mostly helpers that only macOS-gated tests use, so a Linux or Windows `./dev check` still stops at Clippy before it runs a test. The other choice clears them and fails the check on a new one, which every module that writes a macOS-gated test then pays for.

## M1 A GPU test in the sea, asset and cooker crates gets its device from one helper, and with no GPU it is skipped with a line anyone running the tests sees, or fails when a GPU is required: done

[review](../../../.orchestrate/evidence/TEST-SUPPORT/M1-review.md)

## M2 Every GPU test in the engine goes through the gate, the workspace's tests build for Linux and Windows with no CPU test hidden, and no test shares code by `#[path]`

[The log](LOG.md) says why this milestone's items changed, and [the design](DESIGN.md#other-platforms) says what
is proved for Linux and Windows and what is not. The gate, the recipe's device and the declared modules are in
the tree; what is left is the type-check and what it finds. The third review left a working one in this
milestone's evidence, `M2-review/cross-target-check.sh` with its stand-in compilers in `M2-review/c-shims/`: start
from it. It returns only the Windows run's exit code, writes links into its own folder and uses one fixed folder
under `/tmp`. Both std targets are installed on this Mac, and a cold run of both took about a minute, so run it
after every change rather than compiling named targets.

The type-check belongs to the main tier, not to `./dev check`: nothing builds or runs on Linux or Windows, so a
break there stops no builder, and it costs its own module a fix. M2.8 declares it, and the design has the line.
`GATE-TIME` owns the tiers: its main-tier list, `.config/long-tests.txt`, needs no entry, because this is not a
test binary, and its next re-plan names the check in the Main row of the settings' test tiers and in the
`CONTRIBUTING.md` text its M5.5 asks for.

It fails in two packages today. `thrum-asset-gpu`'s `draw/tests.rs` uses `serde_json`, which the manifest gives
only Apple targets. The water scene's 11 test targets reach `thrum-app`'s macOS shell (`Game`, `Gpu`,
`offscreen`, and `SkyGame`'s methods) through `tests/support.rs` and `tests/realtime.rs`. That shell is macOS-only
on purpose and is not this module's: gate the water tests one by one, as `macos_gpu_test!` does, and leave
`thrum-app`'s exports alone. `GATE-TIME` M1 marks the gale in `floating_gpu.rs` `#[ignore]` with a `long:` reason
and adds a `Main-branch check` line beside yours: whichever lands second rebases, and the gale keeps its
attribute and reason.

A review shows what one reviewer tried, so the earlier reviews' lists are not the job: the items are. The workspace
comparison M2.4 names is in this milestone's evidence (`M2/README.md`) and three reviews reproduced it, so don't
make it again. M2.5 passes from the shared run. Tell the lead when you merge, because the script reads the
settings from the main checkout.

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
