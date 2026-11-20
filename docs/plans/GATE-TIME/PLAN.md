# GATE-TIME plan

A merge's check runs the fast tests its change can reach, each once, and everything else runs on main after the
merge. [Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-6-the-cleanup-of-20-november) ·
[test tiers](../../../ORCHESTRATE.md#test-tiers)

Read [Mechanisms](DESIGN.md#mechanisms) and [the files each milestone touches](DESIGN.md#files-each-milestone-touches)
before any milestone. Every file here is another module's: change the tests the design names and nothing those
tests assert about the engine. The milestones are in build order: M6 and M7 are built second and third.

## Open for the human

- Assumption: the exit scene's stepped hour and the foam's ten-minute moored run leave `./dev check` as the gale does, whole, and run on main after merges. By your rule for the gale, no short run of either stays; the foam's wind sweep, a different behaviour that takes seconds, stays. The other choice keeps a short run of each in every merge's check: one turn of every authored loop, about 20 seconds, and a short moored window.
- Assumption: the tier policy also takes the three field flythroughs, 438 seconds a merge, and the shore-blend solve, 64 seconds, out of `./dev check`; they run on main in release, where the flythroughs go further. The other choice keeps any of them in every merge's check.
- Assumption: the long tests and the whole workspace each run on main at most once an hour, so a fault one of them catches is named with its commit some hours after the merge. The other choice is a shorter interval, which takes more of the machine from builders, or a longer one.
- Assumption: a test binary that runs for over three minutes in a merge's check fails that check. Three minutes is about 2.4 times the slowest binary the check keeps. The other choice reports the slow binary and fails nothing, which is how the gale reached 21 minutes unnoticed.
- Assumption: a milestone's Ready for Review still runs the whole workspace's tests, because the script runs `./dev check` on main's tip, where nothing differs from main. So a missed edge on main holds every module's Ready for Review until its commit is fixed or reverted. The other choice selects that run by the milestone's own change, which needs a change to the orchestrate script.
- Assumption: the tests behind a Cargo feature (the renderer's creation counters, the ray-tracing proof, the mesh-stage tests and the cooked fault test) run on main once an hour with the long tests, and every merge compiles them. The other choice runs them only in the item checks that name them, so a break in one is found days later by another module's builder.
- Assumption: a release point is the V1 exit's closing record or anything published, and there every tier runs on the one commit; `SHOWCASE`'s next re-plan adds the long tests to its closing milestone. The other choice names other release points, or asks less of one.
- Assumption: the atmosphere's Monte Carlo reference is deleted, about 2,200 lines, and stays in Git history; the night checks and the twilight guard stay in the check. The other choice keeps it as an ignored diagnostic, which the check still compiles but never runs.
- Assumption: three more tests that hold GPU output to a reference are replaced by short tests of the behaviour, about 1,400 lines: the mirror pool's rounding-allowance proof, the cooked cloud march's f64 reference and the scene-view shader hashes frozen at a 16 November commit. The other choice keeps any of the three as it is.

## M1 A merge's check no longer runs the ten-minute gale, the stepped hour, the field flythroughs or the shore-blend solve, and they pass on main after each merge

This milestone merges before any other of track 6, so keep it small. Before you mark a test, run the six once
as `./dev long-tests` will run them (release, overflow checks on, a GPU required), and keep the output in this
milestone's evidence and the wall time in the log. Only the occlusion flythrough has run in release
before. A test that fails in that build is not yours to fix: the design says what the list and the log then say.

The tests are `actual_gale_keeps_the_fleet_in_contact_on_metal` in `examples/water/tests/floating_gpu.rs`,
`movers_keep_their_authored_loops_after_a_stepped_hour` in `examples/v1-game/tests/exit_scene.rs`, the three
flythroughs in `examples/v1-game/tests/support/field_gpu.rs` and `field_occlusion_gpu.rs`, and the one test of
`crates/thrum-water-model/tests/shore_blend.rs`. `./dev` sets `THRUM_REQUIRE_GPU=1` for `check` only, in the `dev`
script. `examples/v1-game/tests/valley_culling_gpu.rs` holds an ignored test that is not a long test. `./dev help`
is one string literal on one source line, and `DEAD-TOOLS` M1 and M2 edit it too: put each command on a line of its
own. `WATERS` M5.4 and M12.2, `SHOWCASE` M5.3 and `CULLING` M1.2 are Done and name these tests: leave their text
alone. M1.3 passes from the shared run on main, so set it `pass` with the evidence `main-branch check long-tests`
and don't wait for a run of your own; tell the lead when you merge, because the script reads the settings from
the main checkout.

## M6 A change that breaks a test behind a Cargo feature fails its merge's check, and those tests run on main after each merge

`./dev check` already lints the game with `draw-probe` and runs one probe binary with it; it never turns on
`creation-counters` for `thrum-render`'s own tests, nor `thrum-asset-gpu/fault-probe`. Only the game's
`cooked_fault_gpu` and `ray_tracing_gpu` use `required-features`; the rest gate inside the file, as
`crates/thrum-asset-gpu/tests/error_types.rs` does. `scripts/repo_check.py` has `is_test_file`. Start by running
the tests behind each feature once with a GPU required, `thrum-render`'s whole package included, and record
which pass: that decides what the list says. Some of these binaries hold ignored tests that are measurements. Open items of seven modules
run these tests by name, so rename nothing.

## M7 A merge's check runs only the tests of the packages its change can reach, and the whole workspace's tests run on main after each merge

Measure first: M7.1's record comes before the selection is switched on, and if it shows that a selected run is
not shorter than the whole on average, end `BLOCKED` with the record, because the owner decided this expecting a
saving. The design links the planner's replay. The script runs `./dev check` with no
argument in three places: a builder's worktree, a merge after its rebase onto `origin/main` (sometimes several
branches rebased together on a detached checkout), and its own checkout of main's tip, where a pass on the same
files is reused. The guard the declared edge serves is in
`crates/thrum-render/src/runtime/tests/creation_sites.rs`. The foam scene's binary runs for over three minutes until M3.

## M2 Nothing in a merge's check is built and run twice, and the atmosphere's Monte Carlo reference no longer runs in it

`query_gpu.rs` uses `swash_gpu::{Clock, Source}` and `swash_gpu::beach`, so it cannot just drop `mod swash_gpu;`:
remove the `swash_gpu` `[[test]]` entry so the test runs only inside `query_gpu`, or move the helpers. The Done
item `CULLING` M1.2 names the binary `field_occlusion_gpu` and nothing names `field_gpu`, so the flythroughs are
cheapest to keep under the first name; the main-tier list follows. In the atmosphere's reference folder,
`gpu/night.rs`, `gpu/readback.rs` and `gpu/query.rs` use none of the Monte Carlo code, and the twilight guard is
`shipping_spectral_twilight` in `reference/gpu.rs`, called today only from
`gpu_transport_reference_reports_every_prescribed_row`.

## M3 A merge's check no longer runs the foam's ten-minute moored run, which passes on main after each merge, and still runs the wind sweep

The test is `ten_minutes_moored_and_wind_sweep` in `examples/water/tests/foam_scene_gpu.rs`. It holds the file's
device mutex, so the other foam tests wait behind it today. It runs itself again by name as a child process,
through `with_cooked_island`, which passes `--exact <name> --nocapture`.

## M4 Three tests that held GPU output to a reference are replaced by short tests of the behaviour: the mirror pool, the cooked cloud march and the scene-view shader text

In `examples/mirror/tests/pool_gpu/`, `view.rs`, `retire.rs` and the wasm dev-dependencies serve the Chrome
equality of a secondary view with the main view, which stays. In `scene_view_absent_gpu.rs` the empty observer is
`Observer::new(0)`, which the test labels "no declaration", and the observer with an idle view slot is
`Observer::new(1)`. `SCENE-VIEWS` M2.3 and M3.1 are Done and their checks run these tests: write in this module's
log what those checks no longer prove.

## M5 The check builds and runs its tests the fastest way a measurement supports, and the record says how long a merge's check and the main tier take now

Measure first: M5.1's record decides M5.2. `cargo-nextest` 0.9.150 is installed and unused. Five test files guard
GPU residency or device loss with an in-process mutex, which stops working when each test is its own process.
The four cooked-asset tests that assert on the draw probe are the fetched edit, the publication larger than
staging, the newer publication that supersedes, and the adapter-less host. Time the runs while other builders
work and record the load; nobody is paused for a timing.
