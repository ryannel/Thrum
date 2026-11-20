# GATE-TIME plan

A merge's check runs only fast tests, each once, and the long tests run on main after the merge.
[Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-6-the-cleanup-of-20-november) ·
[test tiers](../../../ORCHESTRATE.md#test-tiers)

Read [Mechanisms](DESIGN.md#mechanisms) and [the files each milestone touches](DESIGN.md#files-each-milestone-touches)
before any milestone. Every file here is another module's: change the tests the design names and nothing those
tests assert about the engine. A finding's full text is in `results.json` of the cleanup review, at the path the
design gives.

## Open for the human

- Assumption: the exit scene's stepped hour and the foam's ten-minute moored run leave `./dev check` as the gale does, whole, and run on main after merges. Nothing is dropped. By the owner's rule for the gale, no short run of either stays in the check; the foam's wind sweep, a different behaviour that takes seconds, stays. The other choice keeps a short run of each in every merge's check: one turn of every authored loop, about 20 seconds, and a short moored window.
- Assumption: the tier policy also takes the three field flythroughs, 438 seconds a merge, and the shore-blend solve, 64 seconds, out of `./dev check`; they run on main in release, where the flythroughs go further along their path than the check took them. The other choice keeps any of them in every merge's check.
- Assumption: the long tests run on main at most once an hour, so a fault one catches is named with its commit an hour or two after the merge. The run's length in release is an estimate until M5 records it. The other choice is a shorter interval, which takes more of the GPU from builders, or a longer one.
- Assumption: the merge check keeps running every fast test in the workspace, not a selection by the files a merge changed. The scenes' tests depend on 18 packages, so a selection would have skipped them for about one commit in eleven. The other choice builds the selection into `./dev check`, a medium-sized change that falls back to the full suite for most merges.
- Assumption: a release point is the V1 exit's closing record or anything published, and there every tier runs on the one commit. `SHOWCASE`'s closing milestone does not yet ask for the long tests; its next re-plan adds that. The other choice names other release points, or asks less of one.
- Assumption: the atmosphere's Monte Carlo reference is deleted, about 2,200 lines, and stays in Git history; the night checks and the twilight guard stay in the check. The other choice keeps it as an ignored diagnostic, which the check still compiles but never runs.
- Assumption: three more tests that hold GPU output to a reference are replaced by short tests of the behaviour, about 1,400 lines: the mirror pool's rounding-allowance proof, the cooked cloud march's f64 reference and the scene-view shader hashes frozen at a 16 November commit. The other choice keeps any of the three as it is.

## M1 A merge's check no longer runs the ten-minute gale, the stepped hour, the field flythroughs or the shore-blend solve, and they pass in release on main after each merge

The tests are `actual_gale_keeps_the_fleet_in_contact_on_metal` in `examples/water/tests/floating_gpu.rs`,
`movers_keep_their_authored_loops_after_a_stepped_hour` in `examples/v1-game/tests/exit_scene.rs`, the three
flythroughs in `examples/v1-game/tests/support/field_gpu.rs` and `field_occlusion_gpu.rs`, and the one test of
`crates/thrum-water-model/tests/shore_blend.rs`. Mark them and leave their bodies alone. `./dev` sets
`THRUM_REQUIRE_GPU=1` for `check` only, in the `dev` script. Other ignored tests in the workspace need a cooked
store or are measurements, and `examples/v1-game/tests/valley_culling_gpu.rs` holds one, so the command runs the
binaries it lists and never a whole package's ignored tests. `WATERS` M5.4 and M12.2, `SHOWCASE` M5.3 and `CULLING`
M1.2 are Done and name these tests: leave their text alone. M1.3 passes from the shared run on main, so set it
`pass` with the evidence `main-branch check long-tests` and don't wait for a run of your own; tell the lead when
you merge, because the script reads the settings from the main checkout.

## M2 Nothing in a merge's check is built and run twice, and the atmosphere's Monte Carlo reference no longer runs in it

`query_gpu.rs` uses `swash_gpu::{Clock, Source}` and `swash_gpu::beach`, so it cannot just drop `mod swash_gpu;`:
remove the `swash_gpu` `[[test]]` entry so the test runs only inside `query_gpu`, or move the helpers.
`scripts/culling_capture.py` and the Done item `CULLING` M1.2 name the binary `field_occlusion_gpu`, and nothing
names `field_gpu`, so the flythroughs are cheapest to keep under the first name; the list of `./dev long-tests`
follows. In the atmosphere's reference folder, `gpu/night.rs`, `gpu/readback.rs` and `gpu/query.rs` use none of
the Monte Carlo code, and the twilight guard is `shipping_spectral_twilight` in `reference/gpu.rs`, called today
only from `gpu_transport_reference_reports_every_prescribed_row`. Tell the `CLOUDS` builder before you land in
`crates/thrum-clouds/tests`.

## M3 A merge's check no longer runs the foam's ten-minute moored run, which passes on main after each merge, and still runs the wind sweep

The test is `ten_minutes_moored_and_wind_sweep` in `examples/water/tests/foam_scene_gpu.rs`. It holds the file's
device mutex, so the other foam tests wait behind it today. It runs itself again by name as a child process,
through `with_cooked_island`, with `--exact`: an ignored test's child has to be started so that it still runs.

## M4 Three tests that held GPU output to a reference are replaced by short tests of the behaviour: the mirror pool, the cooked cloud march and the scene-view shader text

In `examples/mirror/tests/pool_gpu/`, `view.rs`, `retire.rs` and the wasm dev-dependencies serve the Chrome
equality of a secondary view with the main view, which stays. In `scene_view_absent_gpu.rs` the empty observer is
`Observer::new(0)`, which the test labels "no declaration", and the observer with an idle view slot is
`Observer::new(1)`. `SCENE-VIEWS` M2.3 and M3.1 are Done and their checks run these tests: write in this module's
log what those checks no longer prove.

## M5 The check builds and runs its tests the fastest way a measurement supports, and the record says how long a merge's check and the long tests take now

Measure first: M5.1's record decides M5.2. `cargo-nextest` 0.9.150 is installed and unused. Four test files guard
GPU residency or device loss with an in-process mutex, which stops working when each test is its own process.
The four cooked-asset tests that assert on the draw probe are the fetched edit, the publication larger than
staging, the newer publication that supersedes, and the adapter-less host. Time the runs while other builders
work and record the load; nobody is paused for a timing.
