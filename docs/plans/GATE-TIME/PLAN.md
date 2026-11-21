# GATE-TIME plan

A merge's check takes about five minutes: it runs the fast tests its change can reach, side by side and each once,
and everything else runs on main after the merge. [Design](DESIGN.md) · [done-when file](DONE.json), which owns the
criteria · [log](LOG.md) · [module list](../ARCHITECTURE.md#track-6-the-cleanup-of-20-november) ·
[test tiers](../../../ORCHESTRATE.md#test-tiers)

Read [the goal](DESIGN.md#the-goal), [Mechanisms](DESIGN.md#mechanisms) and
[the files each milestone touches](DESIGN.md#files-each-milestone-touches) before any milestone. Every file here is
another module's: change nothing those tests assert about the engine. The milestones are in build order.

## Open for the human

- Assumption: a merge's check should take about five minutes for a typical landing, meaning the median of real merges on this Mac while builders work. You said "Tests are meant to be fast" and gave no figure. The other choice is another figure: ten minutes is met by running tests side by side alone, and two would need a faster build as well.
- Assumption: a test in a merge's check takes about ten seconds or less. A slower one is made fast, or runs on main after merges with its risk written down; about 30 tests are over ten seconds today. The other choice is a higher bound, such as 30 seconds, which keeps the wake scene's three tests and the shore basin in every merge's check.
- Assumption: a test that runs for over a minute in a merge's check is stopped and fails that check. The other choice reports it and fails nothing, which is how the gale reached 21 minutes unnoticed.
- Assumption: the exit scene's stepped hour and the foam's ten-minute moored run leave `./dev check` as the gale does, whole, and run on main after merges, with no short run of either kept. The other choice keeps a short run of each in every merge's check.
- Assumption: the three field flythroughs and the shore-blend solve also run on main and not in a merge's check. The other choice keeps any of them in every merge's check.
- Assumption: the whole workspace's tests run on main at most once an hour, so a fault they catch is named with its commit some hours after the merge. The other choice is another interval, as you chose every four hours for the long tests and exact images on 21 November.
- Assumption: a milestone's Ready for Review still runs the whole workspace's tests, so a missed edge on main holds every module's Ready for Review until its commit is fixed or reverted. The other choice selects that run by the milestone's own change, which needs a change to the orchestrate script.
- Assumption: the tests behind a Cargo feature run on main with the long tests, and every merge compiles them. The other choice runs them only in the item checks that name them.
- Assumption: a release point is the V1 exit's closing record or anything published, and there every tier runs on the one commit; `SHOWCASE`'s next re-plan adds the long tests to its closing milestone. The other choice names other release points, or asks less of one.
- Assumption: the atmosphere's Monte Carlo reference is deleted, about 2,200 lines, and stays in Git history; the night checks and the twilight guard stay in the check. The other choice keeps it as an ignored diagnostic.
- Assumption: three more tests that hold GPU output to a reference are replaced by short tests of the behaviour, about 1,400 lines: the mirror pool's rounding-allowance proof, the cooked cloud march's f64 reference and the scene-view shader hashes frozen at a 16 November commit. The other choice keeps any of the three as it is.
- Assumption: the 21 November rules, the smallest world and no device for logic, bind tests written or changed from now on, and M9 and M3 apply them to the tests over ten seconds. The other choice also sweeps the 1,290 faster tests, most of them other modules', for at most 141 s summed over 14 cores.
- Assumption: M11 takes test binaries from 263 to about 110, and the rest fold as open items are built. The other choice rewrites the 176 binary names in 17 modules' open items now, with a design review for each module.
- Assumption: macOS's check of each new binary stays on. The other choice is yours alone: listing the builders' apps under Developer Tools in System Settings removes that cost, and turns off an OS security check for whatever they start.

## M1 A merge's check no longer runs the ten-minute gale, the stepped hour, the field flythroughs or the shore-blend solve, and they pass on main after each merge: done
[review](../../../.orchestrate/evidence/GATE-TIME/M1-review.md)

## M6 A change that breaks a test behind a Cargo feature fails its merge's check, and those tests run on main after each merge: done
[review](../../../.orchestrate/evidence/GATE-TIME/M6-review.md)

## M8 A merge's check runs its tests side by side and takes minutes where it took twenty: it says where its time went, and a test that runs over a minute fails it: done
[review](../../../.orchestrate/evidence/GATE-TIME/M8-review.md)

## M7 A merge's check runs only the tests of the packages its change can reach, and the whole workspace's tests run on main after each merge: done
[review](../../../.orchestrate/evidence/GATE-TIME/M7-review.md)

## M2 Nothing in a merge's check is built and run twice, and the atmosphere's Monte Carlo reference no longer runs in it

`query_gpu.rs` uses `swash_gpu::{Clock, Source}` and `swash_gpu::beach`, so it cannot just drop `mod swash_gpu;`:
remove the `swash_gpu` `[[test]]` entry so the test runs only inside `query_gpu`, or move the helpers. The Done
item `CULLING` M1.2 names the binary `field_occlusion_gpu` and nothing names `field_gpu`, so the flythroughs are
cheapest to keep under the first name; the main-tier list follows. In the atmosphere's reference folder,
`gpu/night.rs`, `gpu/readback.rs` and `gpu/query.rs` use none of the Monte Carlo code, and the twilight guard is
`shipping_spectral_twilight` in `reference/gpu.rs`, called today only from
`gpu_transport_reference_reports_every_prescribed_row`. Where a test you delete is on the runner's exempt list,
take its name off.

## M10 A record shows where a builder's session goes: the model's own time, each kind of command and each wait

Measure, and change nothing else: [mechanism 11](DESIGN.md#mechanisms) says what the record splits and where its
sources are. Much of a session is inside commands, so say what each command was waiting for.

## M9 Every test in a merge's check takes about ten seconds or less, outside the water scene: each slower one was made fast, or runs on main after merges with its risk written down

The runner's exempt list is the work: every name on it that M9 owns. The design's table gives the planner's
measurement of each and the route it expects, and M8's record gives the times on the build the check now uses.
Measure each test alone before and after. Mechanism 9 has two new routes, the smallest world and without a
device; its example is the river model's `no_step_is_taken_beyond_the_budget`.

## M11 Each package's integration tests build into one test binary, and a merge's check spends less time linking them and running each for the first time

Measure first: if M11.1's later timings are not shorter, end `BLOCKED` with the record. Each test file declares
`mod support;` for itself, which resolves elsewhere once the file is a module, and the rule `path-attribute`
refuses `#[path]`. The water scene's `with_cooked_island` starts a test again with `--exact <name>`. Before
folding a file, search `scripts/` and `tools/xtask` for its binary as well as the open items M11.4 reads.

## M3 The water scene's slow tests leave a merge's check or are made fast: the foam's ten-minute moored run passes on main after each merge, and the check still runs the wind sweep

The moored test is `ten_minutes_moored_and_wind_sweep` in `examples/water/tests/foam_scene_gpu.rs`. It runs itself
again by name as a child process, through `with_cooked_island`, which passes `--exact <name> --nocapture`. The
file's other tests waited behind its device mutex under `cargo test`, so their own times are in M8's record, not in
older logs.

## M4 Three tests that held GPU output to a reference are replaced by short tests of the behaviour: the mirror pool, the cooked cloud march and the scene-view shader text

In `examples/mirror/tests/pool_gpu/`, `view.rs`, `retire.rs` and the wasm dev-dependencies serve the Chrome
equality of a secondary view with the main view, which stays. In `scene_view_absent_gpu.rs` the empty observer is
`Observer::new(0)`, which the test labels "no declaration", and the observer with an idle view slot is
`Observer::new(1)`. `SCENE-VIEWS` M2.3 and M3.1 are Done and their checks run these tests: write in this module's
log what those checks no longer prove.

## M5 The record shows a typical landing's merge check takes about five minutes, and the contributor guide says what each tier runs

The times come from the checks the script saved, `.git/orchestrate/checks`, each of which ends with its own step
times since M8. If the median is over five minutes, say where the time goes and end `BLOCKED`: the plan is what
has to change.
