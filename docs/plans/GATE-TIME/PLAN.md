# GATE-TIME plan

A merge's check takes about five minutes: it runs the fast tests its change can reach, side by side, and
everything else runs on main after the merge. [Design](DESIGN.md) · [done-when file](DONE.json) · [log](LOG.md) · [module list](../ARCHITECTURE.md#track-6-the-cleanup-of-20-november) ·
[test tiers](../../../ORCHESTRATE.md#test-tiers)

Read [the goal](DESIGN.md#the-goal), [Mechanisms](DESIGN.md#mechanisms) and
[the files each work package touches](DESIGN.md#files-each-work-package-touches) before any work package. Every file here is
another module's: change nothing those tests assert about the engine. The work packages are in build order.

## Open for the user

- Assumption: merge verification shares a single local Cargo cache across candidate worktrees on this Mac. It starts with an 8 GiB retention cap and honors the lead's 30 GB free-space floor. Cargo keeps the existing checks and test coverage. Test-profile debug-info tuning is available if the cache alone falls short in the paired trial.
- Assumption: the five-minute goal is judged once on the first 20 qualifying Rust landings after rollout, allowing at most five over 300 seconds under the previous tail-count rule. This sample includes ordinary cold starts and fallbacks.
- Assumption: the 30-second serial retry qualified for the workspace check is long enough for a merge under overlapping CPU and Metal work. WP17 tests that assumption against the 28 November failures. Ten seconds remains the target for a fast test run on a quiet Mac.
- Assumption: "the number of test binaries can only go down" counts the binaries no plan names, 43 of 275 today, because approved open items name 147 not yet written; WP11 takes the whole number from 263 to about 110. The other choice rewrites the 176 binary names in 17 modules' open items now, and freezes the whole number.
- Assumption: the stepped hour, the foam's ten-minute moored run, the three field flythroughs and the shore-blend solve run whole on main after merges, like the gale. The other choice keeps a short run of any in every merge's check.
- Assumption: the whole workspace's tests run on main at most once an hour, with no `gpu` lock, which would hold a GPU slot through the build. The other choice is every four hours under the lock, like the long tests and exact images.
- Assumption: a work package's Ready for Review still runs the whole workspace's tests, so a missed edge on main holds every module's until it is fixed or reverted. The other choice selects that run by the work package's own change, a change to the orchestrate script.
- Assumption: the tests behind a Cargo feature run on main with the long tests, and every merge compiles them. The other choice runs them only in the item checks that name them.
- Assumption: a release point is the V1 exit's closing record or anything published, and every tier runs there on one commit; `SHOWCASE`'s next re-plan adds the long tests to its closing work package. The other choice names other release points.
- Assumption: the atmosphere's Monte Carlo reference is deleted, about 2,200 lines; the night checks and the twilight guard stay in the check. The other choice keeps it as an ignored diagnostic.
- Assumption: three more tests that hold GPU output to a reference are replaced by short tests of the behaviour, about 1,400 lines: the mirror pool's rounding-allowance proof, the cooked cloud march's f64 reference and the scene-view shader hashes. The other choice keeps any of the three.
- Assumption: the smallest world and no device for logic bind tests written or changed from 21 November, and WP9 and WP3 apply them to the tests over ten seconds. The other choice also sweeps the 1,290 faster tests, for at most 141 s summed.
- Assumption: on main, the tests behind a Cargo feature run each in a process of its own, as a merge's tests do, and are stopped only as a hang, at 120 seconds. The other choice keeps a binary's tests in one process and repairs the cap in `thrum-render`'s library tests, which the next process-wide limit would pass again.
- Assumption: macOS's check of each new binary stays on. The other choice is yours alone: listing the builders' apps under Developer Tools in System Settings removes that cost, and an OS security check with it.

## WP1 A merge's check no longer runs the ten-minute gale, the stepped hour, the field flythroughs or the shore-blend solve, and they pass on main after each merge: done
[review](../../../.orchestrate/evidence/GATE-TIME/WP1-review.md)

## WP6 A change that breaks a test behind a Cargo feature fails its merge's check, and those tests run on main after each merge: done
[review](../../../.orchestrate/evidence/GATE-TIME/WP6-review.md)

## WP8 A merge's check runs its tests side by side and takes minutes where it took twenty: it says where its time went, and a test that runs over a minute fails it: done
[review](../../../.orchestrate/evidence/GATE-TIME/WP8-review.md)

## WP7 A merge's check runs only the tests of the packages its change can reach, and the whole workspace's tests run on main after each merge: done
[review](../../../.orchestrate/evidence/GATE-TIME/WP7-review.md)

## WP2 Nothing in a merge's check is built and run twice, and the atmosphere's Monte Carlo reference no longer runs in it: done
[review](../../../.orchestrate/evidence/GATE-TIME/WP2-review.md)

## WP10 A record shows where a builder's session goes: the model's own time, each kind of command and each wait: done
[review](../../../.orchestrate/evidence/GATE-TIME/WP10-review.md)

## WP12 A merge's check refuses a new test-limit exemption, main-tier test or test binary that no plan asked for, and reports the tests' summed time and the GPU tests' share: done
[review](../../../.orchestrate/evidence/GATE-TIME/WP12-review.md)

## WP9 Every test in a merge's check takes about ten seconds or less, outside the water scene: each slower one was made fast, or runs on main after merges with its risk written down: done
[review](../../../.orchestrate/evidence/GATE-TIME/WP9-review.md)

## WP13 A test over ten seconds in a merge's check is stopped, and fails the check unless it passes within ten seconds alone: done
[review](../../../.orchestrate/evidence/GATE-TIME/WP13-review.md)

## WP11 Each package's integration tests build into one test binary, and a merge's check spends less time linking them and running each for the first time: done
[review](../../../.orchestrate/evidence/GATE-TIME/WP11-review.md)

## WP3 The water scene's slow tests leave a merge's check or are made fast: the foam's ten-minute moored run passes on main after each merge, and the check still runs the wind sweep: done
[review](../../../.orchestrate/evidence/GATE-TIME/WP3-review.md)

## WP4 Three tests that held GPU output to a reference are replaced by short tests of the behaviour: the mirror pool, the cooked cloud march and the scene-view shader text: done
[review](../../../.orchestrate/evidence/GATE-TIME/WP4-review.md)

## WP14 The full workspace check passes when a fast test needs more time on a busy Mac: done
[review](../../../.orchestrate/evidence/GATE-TIME/WP14-review.md)

## WP15 Merge verification reuses a bounded local Cargo cache: done
[review](../../../.orchestrate/evidence/GATE-TIME/WP15-review.md)

## WP17 A busy Mac can verify a merge when fast tests time out on their first attempt

Build this from main before WP16. The caster assertion belongs to ATMOSPHERE and WP16 already has a saved
implementation. Keep those changes on their branches. Use [the merge retry design](DESIGN.md#merge-timeouts-under-load)
for the five repeat timeouts and the loaded and quiet qualifications. Bring the runner help, profile comments
and timeout guidance in the test tiers and contributor guide into line when the policy ships.

## WP16 The long tests on main pass every run: each test behind a Cargo feature runs in a process of its own, as tests do before a merge

Build this before WP5: the main-branch check `long-tests` fails about every other run until it merges. Read
[mechanism 16](DESIGN.md#feature-tests-in-processes-of-their-own) first.
[The planner's record](../../../.orchestrate/evidence/GATE-TIME/WP16-plan/README.md) has the runs that show the
fault and the trial of each feature line through cargo-nextest, with the two limits that proved too short under
load. The settings line of the check does not change, so the lead has nothing to fast-forward for. Bring the
comments of `.config/nextest.toml` and the `./dev long-tests` row of `CONTRIBUTING.md` into line with what the
command now runs, with the time your own runs took.

## WP5 The post-rollout record shows a five-minute median for Rust landings

Start after WP15's activation push. Use `.git/orchestrate/checks` and the check-run ledger to assemble the fixed
window in [the goal](DESIGN.md#the-goal). Keep both earlier samples from `.orchestrate/evidence/GATE-TIME/WP5/`
and use the configured command at activation to match the new runs, including if reusing `landing_times.py`.
Record the number still needed while the window fills, and use only ordinary landings. Send a completed sample
that misses either bound back to planning with its cache, build and run timing breakdown. Update the timing in
`CONTRIBUTING.md` to agree with the closing record. The test guide and `cross-target` instructions were verified
at `039ac3b9f`. Reuse the WP5.2 evidence and check the guide against WP15's command before closing.
