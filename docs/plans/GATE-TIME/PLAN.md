# GATE-TIME plan

A merge's check runs each proof once, and the long soaks run on main after the merge. [Design](DESIGN.md) ·
[done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-6-the-cleanup-of-20-november)

Read [Mechanisms](DESIGN.md#mechanisms) and [the files each milestone touches](DESIGN.md#files-each-milestone-touches)
before any milestone. Every file here is another module's: change the tests the design names and nothing those
tests assert about the engine. A finding's full text is in `results.json` of the cleanup review, at the path the
design gives.

## Open for the human

- Assumption: the ten-minute gale leaves `./dev check` and runs on main after each merge, as the main-branch check `soak`; 60 seconds of gale stay in the check. A gale fault can then sit on main for about two hours before it is traced to its commit. The other choice keeps the ten minutes in every merge's check, at 13 to 21 minutes a merge.
- Assumption: the exit scene's stepped hour and the foam's ten-minute moored run move to `soak` the same way, each with a short run left in the check. Nothing is dropped. The other choice keeps them in the check, at about 2 to 4 minutes each a merge.
- Assumption: the atmosphere's Monte Carlo reference is deleted, about 2,200 lines, and stays in Git history; the night checks and the twilight guard stay in the check. The other choice keeps it as an ignored diagnostic, which the check still compiles but never runs.
- Assumption: three more tests that hold GPU output to a reference are replaced by short tests of the behaviour, about 1,400 lines: the mirror pool's rounding-allowance proof, the cooked cloud march's f64 reference and the scene-view shader hashes frozen at a 16 November commit. The other choice keeps any of the three as it is.

## M1 A merge's check runs 60 seconds of gale and one turn of every authored loop, and the ten-minute gale and the stepped hour pass on main after each merge

The gale is `actual_gale_keeps_the_fleet_in_contact_on_metal` in `examples/water/tests/floating_gpu.rs`, and the
hour is `movers_keep_their_authored_loops_after_a_stepped_hour` in `examples/v1-game/tests/exit_scene.rs`. The same
file's `ten_minutes_of_gale_stay_bounded_at_all_three_speeds` is a CPU test and stays as it is. `WATERS` M5.4 and
`SHOWCASE` M5.3 are Done and say ten minutes and an hour: leave their text alone. M1.3 passes from the shared run
on main, so set it `pass` with the evidence `main-branch check soak` and don't wait for a run of your own; tell
the lead when you merge, because the script reads the settings from the main checkout.

## M2 Nothing in a merge's check is built and run twice, and the atmosphere's Monte Carlo reference no longer runs in it

`scripts/culling_capture.py` and the Done item `CULLING` M1.2 name the binary `field_occlusion_gpu`, and nothing
names `field_gpu`, so the flythroughs are cheapest to keep under the first name. Two three-host flythroughs in one
binary share the GPU, so expect about three minutes saved, not the sum. In the atmosphere's reference folder,
`gpu/night.rs`, `gpu/readback.rs` and `gpu/query.rs` use none of the Monte Carlo code. Tell the `CLOUDS` builder
before you land in `crates/thrum-clouds/tests`.

## M3 The foam's ten-minute moored run passes on main after each merge, and a short window of it stays in the check

The test is `ten_minutes_moored_and_wind_sweep` in `examples/water/tests/foam_scene_gpu.rs`. It holds the file's
device mutex, so the other foam tests wait behind it today.

## M4 Three tests that held GPU output to a reference are replaced by short tests of the behaviour: the mirror pool, the cooked cloud march and the scene-view shader text

In `examples/mirror/tests/pool_gpu/`, `view.rs`, `retire.rs` and the wasm dev-dependencies serve the Chrome
equality of a secondary view with the main view, which stays. `SCENE-VIEWS` M2.3 and M3.1 are Done and their
checks run these tests: write in this module's log what those checks no longer prove.

## M5 The check builds and runs its tests the fastest way a measurement supports, and the record says how long a merge's check takes now

Measure first: M5.1's record decides M5.2. `cargo-nextest` 0.9.150 is installed and unused. Four test files guard
GPU residency or device loss with an in-process mutex, which stops working when each test is its own process.
Time the runs while other builders work and record the load; nobody is paused for a timing.
