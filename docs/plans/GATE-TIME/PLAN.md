# GATE-TIME plan

A merge's check takes about five minutes: it runs the fast tests its change can reach, side by side, and
everything else runs on main after the merge. [Design](DESIGN.md) · [done-when file](DONE.json) · [log](LOG.md) · [module list](../ARCHITECTURE.md#track-6-the-cleanup-of-20-november) ·
[test tiers](../../../ORCHESTRATE.md#test-tiers)

Read [the goal](DESIGN.md#the-goal), [Mechanisms](DESIGN.md#mechanisms) and
[the files each milestone touches](DESIGN.md#files-each-milestone-touches) before any milestone. Every file here is
another module's: change nothing those tests assert about the engine. The milestones are in build order.

## Open for the human

- Assumption: a merge's check takes about five minutes for a typical landing: the median of real merges on this Mac while builders work, counted since the check took its present shape, at least five that ran tests. You said "Tests are meant to be fast" and gave no figure. The other choice is another figure.
- Assumption: a test stopped at ten seconds is run once more, alone, and fails the check only if it is over ten seconds again. Load took a test of under ten seconds past it in 8 of the last 18 whole checks, 22 tests in one. The other choice fails at the first stop.
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

## M5 The record shows a typical landing's merge check takes about five minutes, and the contributor guide says what each tier runs

The times come from the checks the script saved, `.git/orchestrate/checks`. The first builder's record, with its
script `landing_times.py`, is in `.orchestrate/evidence/GATE-TIME/M5/`: its thirty merges are the trend, and
[the goal](DESIGN.md#the-goal) says which merges are counted and why at least five. Before anything else, count the
merges that qualify. With fewer than five, end `BLOCKED` saying how many more M5 waits for, and land nothing to make one: a merge made to be counted is
not a typical landing. If the counted median is over five minutes, say where the time goes and end `BLOCKED`: the
plan is what has to change. `CONTRIBUTING.md`'s `./dev check` row quotes the trend's median: make it agree with the
record you close on.
