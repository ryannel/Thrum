# GATE-TIME plan

A merge's check takes about five minutes: it runs the fast tests its change can reach, side by side, and
everything else runs on main after the merge. [Design](DESIGN.md) · [done-when file](DONE.json) · [log](LOG.md) · [module list](../ARCHITECTURE.md#track-6-the-cleanup-of-20-november) ·
[test tiers](../../../ORCHESTRATE.md#test-tiers)

Read [the goal](DESIGN.md#the-goal), [Mechanisms](DESIGN.md#mechanisms) and
[the files each milestone touches](DESIGN.md#files-each-milestone-touches) before any milestone. Every file here is
another module's: change nothing those tests assert about the engine. The milestones are in build order.

## Open for the user

- Assumption: merge verification shares a single local Cargo cache across candidate worktrees on this Mac. It starts with an 8 GiB retention cap and honors the lead's 30 GB free-space floor. Cargo keeps the existing checks and test coverage. Test-profile debug-info tuning is available if the cache alone falls short in the paired trial.
- Assumption: the five-minute goal is judged once on the first 20 qualifying Rust landings after rollout, allowing at most five over 300 seconds under the previous tail-count rule. This sample includes ordinary cold starts and fallbacks.
- Assumption: the merge check reruns a test stopped at ten seconds once, serially, under the same limit. Load took a test of under ten seconds past it in 8 of the last 18 whole checks, 22 tests in one.
- Assumption: `./dev check-all` gets a 30-second cap for its one serial rerun, subject to proof under the builder load that caused the failures. Ten seconds remains the fast-test target and the merge limit.
- Assumption: "the number of test binaries can only go down" counts the binaries no plan names, 43 of 275 today, because approved open items name 147 not yet written; M11 takes the whole number from 263 to about 110. The other choice rewrites the 176 binary names in 17 modules' open items now, and freezes the whole number.
- Assumption: the stepped hour, the foam's ten-minute moored run, the three field flythroughs and the shore-blend solve run whole on main after merges, like the gale. The other choice keeps a short run of any in every merge's check.
- Assumption: the whole workspace's tests run on main at most once an hour, with no `gpu` lock, which would hold a GPU slot through the build. The other choice is every four hours under the lock, like the long tests and exact images.
- Assumption: a milestone's Ready for Review still runs the whole workspace's tests, so a missed edge on main holds every module's until it is fixed or reverted. The other choice selects that run by the milestone's own change, a change to the orchestrate script.
- Assumption: the tests behind a Cargo feature run on main with the long tests, and every merge compiles them. The other choice runs them only in the item checks that name them.
- Assumption: a release point is the V1 exit's closing record or anything published, and every tier runs there on one commit; `SHOWCASE`'s next re-plan adds the long tests to its closing milestone. The other choice names other release points.
- Assumption: the atmosphere's Monte Carlo reference is deleted, about 2,200 lines; the night checks and the twilight guard stay in the check. The other choice keeps it as an ignored diagnostic.
- Assumption: three more tests that hold GPU output to a reference are replaced by short tests of the behaviour, about 1,400 lines: the mirror pool's rounding-allowance proof, the cooked cloud march's f64 reference and the scene-view shader hashes. The other choice keeps any of the three.
- Assumption: the smallest world and no device for logic bind tests written or changed from 21 November, and M9 and M3 apply them to the tests over ten seconds. The other choice also sweeps the 1,290 faster tests, for at most 141 s summed.
- Assumption: macOS's check of each new binary stays on. The other choice is yours alone: listing the builders' apps under Developer Tools in System Settings removes that cost, and an OS security check with it.

## M1 A merge's check no longer runs the ten-minute gale, the stepped hour, the field flythroughs or the shore-blend solve, and they pass on main after each merge: done
[review](../../../.orchestrate/evidence/GATE-TIME/M1-review.md)

## M6 A change that breaks a test behind a Cargo feature fails its merge's check, and those tests run on main after each merge: done
[review](../../../.orchestrate/evidence/GATE-TIME/M6-review.md)

## M8 A merge's check runs its tests side by side and takes minutes where it took twenty: it says where its time went, and a test that runs over a minute fails it: done
[review](../../../.orchestrate/evidence/GATE-TIME/M8-review.md)

## M7 A merge's check runs only the tests of the packages its change can reach, and the whole workspace's tests run on main after each merge: done
[review](../../../.orchestrate/evidence/GATE-TIME/M7-review.md)

## M2 Nothing in a merge's check is built and run twice, and the atmosphere's Monte Carlo reference no longer runs in it: done
[review](../../../.orchestrate/evidence/GATE-TIME/M2-review.md)

## M10 A record shows where a builder's session goes: the model's own time, each kind of command and each wait: done
[review](../../../.orchestrate/evidence/GATE-TIME/M10-review.md)

## M12 A merge's check refuses a new test-limit exemption, main-tier test or test binary that no plan asked for, and reports the tests' summed time and the GPU tests' share: done
[review](../../../.orchestrate/evidence/GATE-TIME/M12-review.md)

## M9 Every test in a merge's check takes about ten seconds or less, outside the water scene: each slower one was made fast, or runs on main after merges with its risk written down: done
[review](../../../.orchestrate/evidence/GATE-TIME/M9-review.md)

## M13 A test over ten seconds in a merge's check is stopped, and fails the check unless it passes within ten seconds alone: done
[review](../../../.orchestrate/evidence/GATE-TIME/M13-review.md)

## M11 Each package's integration tests build into one test binary, and a merge's check spends less time linking them and running each for the first time: done
[review](../../../.orchestrate/evidence/GATE-TIME/M11-review.md)

## M3 The water scene's slow tests leave a merge's check or are made fast: the foam's ten-minute moored run passes on main after each merge, and the check still runs the wind sweep: done
[review](../../../.orchestrate/evidence/GATE-TIME/M3-review.md)

## M4 Three tests that held GPU output to a reference are replaced by short tests of the behaviour: the mirror pool, the cooked cloud march and the scene-view shader text: done
[review](../../../.orchestrate/evidence/GATE-TIME/M4-review.md)

## M14 The full workspace check passes when a fast test needs more time on a busy Mac

Build this before M5. Use [the workspace rerun design](DESIGN.md#workspace-reruns-under-builder-load).
Keep the replay, its checker `check_loaded.py`, the raw check output and the load samples in
`.orchestrate/evidence/GATE-TIME/M14/`. Capture a successful `./dev check-all` on the fixed code during
representative contention. Use a focused replay through the production workspace runner on the same revision
for incident cases without a passing serial rerun in the full run, or to capture a recovery above ten seconds
and within thirty if the full run has none. Use active builders and a concurrent Metal workload, or reproduce
the incident's CPU and Metal contention with controlled competing work. Capture the one-minute load during
the runs, including the reruns, the core and builder counts, and the overlapping GPU work.

Have `check_loaded.py` read the retained records for the passing full check, the six incident case results and
the load evidence. Accept the over-ten-second recovery from either run when the record ties the first-attempt
timeout, passing serial retry within thirty seconds and production verdict to the same real case and fixed
revision. Keep the checker cheap so milestone checks can rerun from the records.

## M15 Merge verification reuses a bounded local Cargo cache

Build and qualify [the cached check](DESIGN.md#cargo-cache-for-merge-verification) before switching the project
test command. Replay both merge entry points with scratch candidates. Before activation, exercise the configured
test command through orchestrate's `check` in a builder worktree with a failing local edit. Replay the same
invocation on a known-good pre-M15 revision. Coordinate the cutover with the lead so new requests load the updated
test command. Record the activation push and time, the paired trials and the replay logs in
`.orchestrate/evidence/GATE-TIME/M15/`.

## M5 The post-rollout record shows a five-minute median for Rust landings

Start after M15's activation push. Use `.git/orchestrate/checks` and the check-run ledger to assemble the fixed
window in [the goal](DESIGN.md#the-goal). Keep both earlier samples from `.orchestrate/evidence/GATE-TIME/M5/`
and use the configured command at activation to match the new runs, including if reusing `landing_times.py`.
Record the number still needed while the window fills, and use only ordinary landings. Send a completed sample
that misses either bound back to planning with its cache, build and run timing breakdown. Update the timing in
`CONTRIBUTING.md` to agree with the closing record. The test guide and `cross-target` instructions were verified
at `039ac3b9f`. Reuse the M5.2 evidence and check the guide against M15's command before closing.
