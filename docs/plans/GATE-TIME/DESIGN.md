# Gate time: a merge's check takes about five minutes, because it runs the fast tests its change can reach, side by side and each once, and everything else runs on main after the merge

20 November 2026, revised 21 November for [four changes to how tests are written and built](#measured-again-on-21-november),
and again that day for [the guard rules](#the-guard-rules) that hold them. Revised 28 November for
[the feature tests' own processes on main](#feature-tests-in-processes-of-their-own), which WP16 builds, after
the main-branch check `long-tests` failed about every other run. It changes how `./dev long-tests` runs the
tests behind a Cargo feature, and nothing WP1 to WP15 built for a merge's check.
[Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-6-the-cleanup-of-20-november) ·
[test tiers](../../../ORCHESTRATE.md#test-tiers), the policy this module applies ·
[what is proved, and how hard](../../../ORCHESTRATE.md#what-is-proved-and-how-hard)

The findings this module answers are cited by id, such as L1-01. They are in the cleanup review of 20 November,
which is kept in the main checkout at `/Users/ryannel/Workspace/thrum/tmp/cleanup-2026-11-20/`: `REPORT.md` ranks
them, `results.json` holds each finding with its verifier's note, and `verified/` holds the notes in full. Where a
verifier corrected a finding, the verifier is right. `HARDENING.md` beside them is the second read of this plan
that the third revision applied.

## The request

The owner, 20 November 2026: "We want to do a code cleanup and consolidation for Thrum." The lead's task for the
plan: make Thrum cheaper to change, with "a shorter merge gate, no tools, records or code that nothing uses, and
no proof heavier than the fault it guards", and without changing "what the engine draws, simulates or sends".
Gate time comes first, "because every merge pays it".

The owner again that day, on the ten-minute gale: "we can't have tests that run for this long in the regular test
suite. We need to think carefully about what risks we're willing to take and when we run everything vs when our
tests can be more targeted. I think it has to stay off every merge check." The lead's task from it: no long run
in any merge's check, "not even a 60-second slice", unless a short run is shown to catch a kind of fault nothing
else catches, and a written test-tier policy applied to every test over about a minute.

And the owner's decision later that day, as the lead gave it: the merge tier is targeted. "Every merge runs only
the tests for what the change touches, and the full workspace runs on main. Replace the whole-workspace merge
check." The design must say how the selection is computed, what a shared input selects, that a selection it
cannot compute falls back to the whole workspace, and the risk accepted, and the saving is measured before the
switch is turned on.

The owner once more that day, with WP1 landed and a merge's check at 21 to 24 minutes: "A 20min merge check still
sounds crazy. Tests are meant to be fast." The lead's task from it: set the merge check's goal at a few minutes for
a typical landing, start from a measurement of where a check spends its time, and choose the approach that best
gets there. Every test left in the merge tier is fast, a slow behaviour test belongs in the main tier with its
risk written down, and a test that is slow because of how it was written can be made fast.

And on 21 November, of four changes the lead put to the owner to make tests faster: "These all sound really
reasonable." They are: a merge-tier test proves its behaviour on the smallest world that shows it, and full-size
runs belong to the main tier; logic is tested without a device, and a GPU test covers only what a GPU alone can
show, at a small target; the 265 test binaries go towards one a package, to cut link and first-run time; and where
a builder's session goes is measured before any rule about it changes. The lead's task from it: place each in a
work package with items a builder can prove, as a standing rule for new tests where it is a rule, and plan the last
as "a measurement, not a rule". Guard text for the skills and lint rules were being agreed with the owner apart
from that plan.

They were agreed later that day: "Yes, plan it." The lead's task from it: the fast-test habits (small worlds, CPU
first, few binaries, long runs off the merge path) hold "without anyone remembering them". What must hold every
time goes in the merge check, because Codex and Claude builders both pass through it, and not in a hook, which a
Codex builder never runs. Four rules go there, beside the counts that only fall: a test over the limit is stopped
and fails, at sixty seconds now and ten once WP9 lands; the tests exempt from that limit can only shrink; the
main-tier list can only shrink unless a plan says otherwise; and the number of test binaries can only go down.
The check's report gains the summed test time and the GPU tests' seconds, and no gate is put on a check's total
time, which "would fail good work" at the load checks run under. The owner "accepted the 10 s limit and that a
new long test needs a plan decision". The same task gives [TEST-SUPPORT](../TEST-SUPPORT/DESIGN.md) a block in
the engineering skill on writing a test, and the project settings three rules for the tests a plan adds.

## The goal

**The median merge Rust check takes at most 300 seconds of wall clock on this Mac while builders work.** WP15
lands [the build-cost reduction](#cargo-cache-for-merge-verification). WP5 judges its effect using the checks
orchestrate saved for the first 20 ordinary Rust landings after rollout. WP7 and WP8 retain their earlier timing
records. WP5 keeps both the thirty landings before `14a4a7c83` and the failed sample from `b69c9812f` as history.

At rollout, record the main-branch push that activates the WP15 test command and its timestamp. Start the sample
strictly after that push. Match each later landing in push order to the successful invocation used to verify
its tree, using the saved check and its run ledger. The invocation must have started after rollout and finished
before the push. A batched push is one landing, and a retry or memo reuse of the same timed invocation supplies
one observation. List untimed, plan-only and no-Rust landings separately with their reasons. A Rust landing is
one whose check reports that it selected at least one Rust test binary. Count package selections, whole-workspace
selections, cache misses and local fallbacks in order, including slow runs. Report a missing or legacy check for a
Rust landing as a gap and resolve the provenance before judging the window. Use ordinary landings while the
builders keep working. Replays, warm-up runs and landings made for the count supply no observations.

Judge the fixed 20 once. The median must be at most 300 seconds and at most five of the 20 may exceed 300 seconds.
Five is the largest count with a one-sided fair-coin probability at or below the previous 1-in-32 rule at 20
observations. Shared machine load can correlate the observations, so this is an acceptance rule for the observed
window. The record gives each landing's subject and module, how many counted landings are GATE-TIME's, how many
selected the whole workspace, and the available load and cache-mode data. Measure the entire command from the
orchestrate run ledger, including cache setup, any cache lock wait and the launcher build. Also retain the check's
step times and explain runs over ten minutes. An incomplete window waits for ordinary landings. A complete window
that misses either bound goes back to planning with the time breakdown.

Two rules follow from the goal, and [the test tiers](../../../ORCHESTRATE.md#test-tiers) state both:

- **A test in the merge tier is fast: it takes about ten seconds or less**, run alone on this Mac in the build the
  check uses. A test that takes longer is made fast where the time comes from how it was written or built, and
  otherwise is a long test, which runs on main after merges with the fault it leaves unguarded written beside it.
  The owner accepted ten seconds on 21 November.
- **No number here is a budget for the engine.** The goal is about how long the check takes, and its proof is a
  record of real merges. The one thing that fails a merge on time is a single test: one that runs for over a
  minute, and from WP13 one that runs for over ten seconds and does so again when run alone. Nothing fails on how
  long a whole check takes.

## Purpose and who it is for

`./dev check` is the test command, so every merge of every builder waits for it. This module's users are the
builders and the lead: a shorter check means more merges an hour, a shorter wait behind the merge ahead, and
fewer runs lost to a time limit. Nothing a player sees changes.

## Where a merge's check spends its time

Measured on 20 November at main `9b3a90ddb`, after WP1, from the script's saved output of each merge's check
(`.git/orchestrate/checks`), the merge lock's records and the planner's own runs.
[The record](../../../.orchestrate/evidence/GATE-TIME/planning/merge-check-breakdown.md) has every figure, the
scripts and the logs. `./dev check` prints no step times, so the split was rebuilt from what cargo and libtest
print. WP8 makes the check print it.

The ten holds of the merge lock after WP1 ran 21 to 27 minutes, median 23.

| Part of one merge's check | Time |
|---|---|
| Test binaries, 346 of them, run one after another | 19.4 to 21.7 minutes, median 20.2 |
| Building the tests, compile and link | 36 s at the median of 102 checks; 50 to 96 s when every crate is rebuilt |
| Lints, five Clippy runs | 21 s at the median; 60 to 185 s when every crate is rebuilt |
| Script suites, repository guards, profile guards, format check, doc tests | about 65 s, of which the 18 script suites are 46 |
| Rebase, push and records | about a minute |

- **Tests are 20 of the 23 minutes, and the time is in a few of them.** Four binaries take over a minute and are
  7.5 minutes together: the foam scene's 240 s, which waits for WP3, and the game's lent-product, cooked-asset and
  cooked-row binaries at 65 to 75 s each. 24 binaries take over 10 s and are 79% of the run. 257 take a second
  or less, 31 s in all.
- **Building and linting are not the problem.** They take under a minute and a half together on a typical
  landing. The five minutes of lints an earlier revision assumed was wrong.
- **Run twice.** The 14 cooked-asset tests run in the workspace pass and again whole in the draw-probe pass, 70 s
  each time. The cloud crate's four GPU binaries, 105 s, make two distinct runs between them. A merge that fails
  is checked a second time without its branch.
- **Waits.** The check takes no GPU lock. A merge waits for the merge ahead of it: three of the seven merges
  after WP1 took 37 to 43 minutes, because each waited 13 to 20 behind another's check. Inside five test files a
  device mutex makes the tests take turns: the 14 cooked-asset tests take 73 s one at a time, and the longest of
  them alone takes about 22.
- **One test, many cases.** The slowest tests of the lent products (63 and 76 s) and the cooked rows (up to 92 s)
  each loop over 8 to 12 cases inside one test, cooking a store and building a host for each.
- **An unoptimised build.** Workspace crates and every dependency are built at `opt-level` 0 for tests, the
  texture encoders' C and C++ among them. Four model crates already carry an override because their tests were
  too slow without one.

The planner then ran the whole workspace's tests, less the foam's moored run, two other ways, once each, with the
load average at 30 to 40:

| Arrangement | Result |
|---|---|
| Side by side through cargo-nextest 0.9.150, each test in a process of its own, the build as it is | All 1,308 tests pass. The test run takes 3.7 minutes, where one binary after another takes about 16 |
| The same, with dependencies built at `opt-level` 3 and workspace crates at 1 | All pass, in 2.2 minutes. Building everything from nothing takes 141 s against 41, and a rebuild after `thrum-render` changes 63 s against 24 |

So running tests side by side is most of the answer, and no test needed a change to pass that way. The optimised
build takes the cloud crate's 47 s tests to 9 s, `match-proof`'s 45 s test to 5 s and the lent products' to about
20 s. It does not help the wake scene's three tests, the shore basin, three of the sea's or the cooked rows'
slowest, which [the table below](#every-test-over-about-ten-seconds) takes one by one. One run each shows what a
mechanism can do; WP8 times them properly before it switches either on.

Not measured: how often the script's run before Ready for Review finds another merge has landed and so runs the
whole workspace again, and whether the side-by-side run passes every time. WP8 runs it five times.

The first measurement, before WP1, is in [the log](LOG.md): tests alone took 42 to 56 minutes, 31 of them in the
six long tests. [A replay](../../../.orchestrate/evidence/GATE-TIME/planning/selection-replay.md) of the selection
of mechanism 4 over the 177 landings since 13 November put the tests a landing selects at a little over half of
the whole on average: a quarter of landings select no Rust test, half select some packages, and a quarter fall
back to the whole workspace. It is the planner's estimate, and WP7 measures the selection as built before turning
it on.

<a id="measured-again-on-21-november"></a>

**Measured again on 21 November**, after WP8 and WP7. The lead's figures come from the last 24 hours of
`.git/orchestrate/locks.jsonl` and WP8's merge-check records. The planner's come from one saved whole-workspace
check (`.git/orchestrate/checks/c6c57db52a584b5183371f69-97fbad59fed9d769.stderr`, 329.6 s of wall time),
`cargo metadata` and the open items of every module, at main `6cd8545ef`, and are in
[the planning record](../../../.orchestrate/evidence/GATE-TIME/planning/test-speed-2026-11-21.md).

- **The time is still in a few tests.** The lead: a merge check runs 1,309 tests that sum to 1,395 s; 897 take
  under 0.1 s and are 1% of it, and 64 take over 5 s and are 81%. The river model, the water scene and the game
  hold 74%. The planner's check: 1,312 tests sum to 920 s, and the 18 that ran over ten seconds are 58% of it.
  Every one of them is WP9's or WP3's.
- **GPU tests are half the time, and the slow ones are most of that.** The lead: 221 tests with a GPU in their
  name hold 53%. The planner's check: 279 tests with `gpu` or `metal` in their name sum to 549 s, the 14 over ten
  seconds hold 408 s of it, and the other 265 hold 141 s between them.
- **A binary costs time before any test in it runs.** The check ran 263 test binaries, and 106 of them hold one
  test. macOS checks each newly linked binary the first time it runs: the lead puts that at 85 to 105 s a check.
  The planner copied twelve built test binaries of 3 to 12 MB and ran each twice: the first run took 0.26 to
  0.37 s whatever the size, 3.7 s for the twelve, and the second 0.06 s for the twelve. cargo-nextest's
  [macOS notes](https://nexte.st/docs/installation/macos/) and
  [a write-up of Rust builds on the Mac](https://nnethercote.github.io/2025/09/04/faster-rust-builds-on-mac.html)
  give the cause: XProtect scans each new executable once, on one thread, so the cost follows the number of
  binaries, and Cargo makes each integration test file a binary.
- **Most test binaries are free to merge.** The workspace has 277 integration test targets in 25 packages:
  `thrum-render` 59, the game 33, the cloud model 24, the river model 19. Open items name 29 of them with
  `--test`. Open items also name 147 binaries that nobody has written yet, in 17 modules.
- **Not measured: where a builder's session goes.** Builder sessions ran 40 to 210 minutes for each cleanup
  work package, and nothing splits that between the model's own time, test runs and waits. WP10 measures it.

For the guard rules the planner read the 18 whole-workspace checks the script saved in the 30 hours to 09:45
that day, and counted at main `469a28671`, after WP2 landed.
[The second planning record](../../../.orchestrate/evidence/GATE-TIME/planning/test-guards-2026-11-21.md) has
the tables.

- **Load alone takes a short test past ten seconds.** Leaving out the exempt tests, 8 of the 18 checks ran a
  test for over ten seconds that takes under ten at its median. Six of them had one to three such tests, with
  medians of 6 to 9.7 s. The two that ran at the same time had 13 and 22, and 7 and 15 of those take under five
  seconds at their median: one of 0.1 s took 10.6 s.
- **Six tests off the exempt list are over ten seconds in every check**, at 11 to 25 s at their median: five of
  the game's and one of the water scene's. WP8 left them off because each is under ten seconds alone.
- **What the three counts stand at.** Twelve names are exempt from the limit, eight under WP9 and four under WP3.
  The main-tier list has ten entries, and five tests are marked `long:`. Of the 275 integration test targets,
  43 are named by no done-when item's check and not by the main-tier list.
- **The whole run on main takes six to eleven minutes**, of which the tests are about four.

## Every test over about a minute

Until this revision [the test tiers](../../../ORCHESTRATE.md#test-tiers) made a long test one that takes over about a
minute in `./dev check`. This is every such test on 20 November. WP1 moved all but the foam's, which is WP3's.

| Test | In the check | Finding | Where it goes | What the merge tier still runs | Risk accepted |
|---|---|---|---|---|---|
| The gale on Metal: ten minutes with the craft at rest, at 5 m/s and at 15 m/s, 59,400 frames | 13 to 21 min | L1-01 | Main tier, whole | In the same file: 80 frames of the fleet on Metal in calm and in gale, ten minutes of gale on the CPU model at the three speeds, and hulls settling to their draft | A body that loses contact, capsizes or gains energy only on the GPU's surface, and only over minutes |
| The three flythroughs of the game's dense field, culled against direct | 438 s | L1-03 | Main tier, whole. In release they run to tick 1,930, past the first cut; the debug check stopped at tick 460 | The renderer's own culled-against-direct tests | A culling fault that only the game's dense field shows |
| The moored foam scenes, ten minutes on the sea and on the lake | 183 to 254 s | L1-04 | Main tier. The wind sweep in the same test is another behaviour and takes seconds: it stays, as a test of its own | The wind sweep, and the file's trail and beach tests | A foam stock that leaves zero in calm water |
| The exit scene's stepped hour, 216,000 ticks of 10,000 entities | 117 to 213 s | L1-09 | Main tier, whole | The file's replay test | A mover off its authored loop, or a static entity that moves. Positions are a closed form of the tick, so nothing builds up and the risk is small |
| The shore blend against each component's own solve, already built optimised | 64 s | L1-10 | Main tier, whole | The shore model's other tests | A blend that drifts from its solve, which costs `WATERS` alone |

No short run of any of them stays. A 60-second gale at three speeds is 18% of the frames, so it would itself take
two to four minutes in the check, and what it would catch the main tier catches within the hour. One turn of every
authored loop would take about 20 s, by proportion, and a short moored window about as long, but each catches only
what its long test catches, which is the case the owner's rule leaves out.

Three more binaries took 66 to 74 s and stayed under that rule: the lent products, the cooked assets and the
cooked rows. Each holds 3 to 14 tests of cooked formats, leases and recovery, which is the silent and costly kind
the bar keeps before a merge. They stay in the merge tier, and [the next table](#every-test-over-about-ten-seconds)
makes them fast.

## Every test over about ten seconds

The owner's rule of 20 November is that tests are fast, and the test tiers now set a merge-tier test at about ten
seconds or less. This is every test the planner measured over that, at main `9b3a90ddb`, outside the long tests
above. "Alone" is the test's own time with its binary run by itself, in the build as it is. "Side by side" is its
time in the planner's optimised side-by-side run at a load of 40, which is the worst it was seen to do. The route
is what the planner expects from reading the test; [mechanism 9](#mechanisms) says what a builder may do where
the route turns out wrong. WP8 measures again, and its list of exempt tests is the one WP2, WP9 and WP3 work from.

| Test | Alone | Side by side | Why it is slow | Expected route | In |
|---|---|---|---|---|---|
| The game's lent products: `leased_texels_commit_edit_capture_discard_and_recovery`, `overlapping_edits_reuse_the_ready_publication` | 76 s, 63 s | 22 s, 18 s | Each runs 4 to 8 cases in turn, and cooks a 128-cubed volume several times in an unoptimised encoder | The build, then one case to a test | WP9 |
| The game's cooked rows: `visual_rebuilds_preserve_writing_ready_discarded_and_newly_committed_rows`, `edited_rows_equal_fresh_game_in_every_culling_mode`, `supersession_discarded_protected_switch_and_retired_space_reuse` | up to 92 s, 38 s, 25 s | 53 s, 23 s, 15 s | 12, 12 and several cases in turn, each with its own cooked store and host | One case to a test | WP9 |
| The game's cooked assets: `a_newer_publication_supersedes_an_upload_still_being_filled`, `a_publication_larger_than_staging_streams_through_the_bounded_ring`, `a_fetched_generation_draws_like_the_mapped_store` | 22 s, 15 s, 14 s | 4 s, under 4 s, 7 s | The unoptimised encoder; and the binary's 14 tests take turns behind one mutex, 73 s in all | The build. The turns are mechanism 8's | WP9 |
| The game's `frame_alloc`, two tests | 18 s, 21 s | 14 s | Not read. They take turns behind a mutex | The builder finds the cause | WP9 |
| The river model's `no_step_is_taken_beyond_the_budget` | 38 s | 100 s | 43 solves of `valley(0.25)` in one test, 36 of them run to within a dozen steps of convergence, to prove a rule about the step budget. The solve uses a thread pool | The smallest world: the rule holds on any grid whose solve takes more than a few steps; counted as heavy | WP9 |
| The river model's other 70 tests | 71 s for all of them, one binary at a time | about 10 s each, 474 s summed | Fourteen processes each start a thread pool | Counted as heavy, in WP8. None is slow alone | WP8 |
| The water model's `enclosed_water_has_finite_lags_turning_and_fetch_energy` (`shore_basin`), `waves_arrive_round_land_and_exposure_closes_far_downwind` (`shore_island`) | 34 s, 11 s | 51 s, 20 s | Dense f64 shore solves over millions of cells, already built at `opt-level` 2 | A smaller basin if every assertion holds on it; otherwise a long test beside the shore blend. Risk: a lag, turning or fetch error in enclosed water, which costs `WATERS` alone | WP9 |
| The sea's `forward_fields_and_full_lean_pyramids_match_the_model`, `default_webgpu_limits_and_named_profile_refusals`, `frame_script_and_continuous_wind_change`, `metal_ambient_matches_independent_model` | 22 s, 16 s, 16 s, 17 s | 14 s, 18 s, 17 s, 16 s | Not read. The optimised build helps only the first | The builder finds the cause. These hold GPU state to the CPU query a game reads, so they are the last to move | WP9 |
| The cloud crate's four GPU conformance tests | 26 s, 26 s, 26 s, 19 s | 9 s each | A CPU reference at `opt-level` 0 | The build. WP2 cuts the four to two | WP8, WP2 |
| The cloud model's `band_tracks_slant_paths_and_rejects_wrong_substitutes`, `footprint_is_the_pointwise_voxel_integral` | 21 s, 17 s | about 1 s | `opt-level` 0 | The build | WP8 |
| `match-proof`'s `sixteen_bit_controls_keep_five_worlds_equal_on_a_conditioned_link` | 29 s | 5 s | Five worlds of 2,000 bodies for 600 ticks, twice, at `opt-level` 0 | The build | WP8 |
| `thrum-render`'s atmosphere tests `analytic_fields_preserve_direction_colour_and_physical_units`, `near_ground_reads_match_their_own_column`, and two at 11 s; `scene_view_air_shaded_gpu`; `visibility_buckets`' mixed field host; the game's `parallel_movers_match_the_straight_corrected_run_after_churn_and_refusal` | 11 to 26 s | 6 s or less | `opt-level` 0 | The build | WP8 |
| The water scene's wake tests: `moored_craft_keeps_its_draft_and_refuses_reactions_by_capacity`, `moored_craft_keeps_its_draft_when_only_one_level_admits_pressure`, `float_twenty_metres_off_track_responds_to_the_public_query` | 35 s or less each: the binary takes 35 s | 38 s, 39 s, 31 s | Each steps a GPU scene for 1,200 to 1,800 ticks, 20 to 30 simulated seconds | The two moored tests are long tests, whole. Risk: a moored craft that loses its draft only after tens of seconds of wake. The float's test holds the public query to the GPU's wake, so the builder first tries a float nearer the track | WP3 |
| The water scene's foam tests `beach_births_follow_lost_power_and_the_film_leaves_ground_depth` and `craft_trail_outlives_removed_source_and_spreads` | Not known: they wait behind the moored run's mutex | 18 s each | Not read | The builder finds the cause; a run whose length is what it tests is a long test | WP3 |

If WP8's measurement leaves the build as it is, every row marked "the build" is WP9's, by the same routes.

WP8 switched the optimised build on, and its exempt list holds the tests over ten seconds alone. Six tests of the
table are under that alone and over it in every check, beside the others: the lent products' two, the cooked
rows' `edited_rows_equal_fresh_game_in_every_culling_mode` and
`supersession_discarded_protected_switch_and_retired_space_reuse`, `frame_alloc`'s
`late_surface_submit_reports_main_thread_allocation_cost`, and the foam scene's
`beach_births_follow_lost_power_and_the_film_leaves_ground_depth`. They stay WP9's and WP3's, by the routes the
table gives: WP9.1 and WP3.3 hold a test to ten seconds in the check as well as alone, because
[the stop at ten seconds](#the-stop-at-ten-seconds) would otherwise catch each of them in every check.

## Mechanisms

**1. A long test is marked, not shortened.** Each test in the table gets `#[ignore]` with a reason that starts
`long:` and names the command that runs it. Its body and assertions do not change. So `./dev check` skips it,
anyone diagnosing a fault runs it with one flag, and nobody has to show that a shorter run asserts the same thing.
The foam test is the one that is split first, because it holds two behaviours: the ten-minute moored run becomes
the long test, and the wind sweep becomes a test `./dev check` runs. The moored run starts itself again as a
child process by name, and an ignored test's child skips unless it is told to run ignored tests, so WP3 also
changes the child's arguments in the water scene's `with_cooked_island`.

**2. One command runs the main tier's tests from one list, and a main-branch check runs it.** `./dev long-tests`
runs the tests the main-tier list names and nothing else. The list is one tracked file,
`.config/long-tests.txt`. For each long test it gives the package, the test binary and the build; mechanism 3
says what it gives for the tests behind a Cargo feature. The command runs the ignored tests of each long-test
binary, so a binary on the list holds no ignored test that is not a long test: other ignored tests in the
workspace need a cooked store, or are measurements. It uses the toolchain `./dev check` uses and requires a GPU,
so a test that finds no adapter fails where it would have skipped. The list names the foam's binary from WP1: a
binary with no ignored test passes.

The build is release with overflow checks on. Debug assertions stay off, because the flythrough chooses its
length by them and would stop at tick 460. WP1 starts by running the six once in that build, because five of them
have never run in it. A test that fails there is still marked and still leaves the merge check: the list gives it
the debug build, the log gives the release failure and names the module that owns the test, and nobody changes
the test here. Without that rule one release failure would leave WP1 open, with every other landing paying the old
price behind it.

WP1 adds one line to `ORCHESTRATE.md`, in these words:

```
- **Main-branch check long-tests:** `./dev long-tests`; every 1h; lock gpu; timeout 60m  *(owner, 20 Nov 2026: long tests stay off every merge check)*
```

The builder keeps every word but the timeout, which is 60m or twice the time its own first run took, whichever
is longer; WP6 sets it again the same way when the tests behind a feature join. The script runs the check on
main's tip after merges, in its own checkout, at most once an hour. It runs a failure again once, then traces one
that holds to the first commit that fails, and that commit's module fixes it while the others keep merging, as
for [exact images](../ARCHITECTURE.md#exact-baselines). Each step of that trace is a whole run under a `gpu`
lock, so a fault a long test catches is named some hours after its merge. That line and the one of mechanism 4
are the only things in `ORCHESTRATE.md` a builder of this module writes, and after each merges the lead
fast-forwards the main checkout, because the script reads the settings there.

**3. A test behind a Cargo feature is compiled by every merge and run on main.** Three features hide tests from
`cargo test --workspace`: `creation-counters` in `thrum-render` (27 files, among them twelve ray-tracing test
binaries that run zero tests in the check); `draw-probe` and `thrum-asset-gpu/fault-probe` in the game (the
ray-tracing proof, the mesh-stage tests and the cooked fault test); and `thrum-asset-gpu`'s own `draw-probe`,
which gates one test of its error inventory. Until now they ran only in the item checks that name them, so a
sweep that broke one merged green and stopped another module's builder days later.

Only two of the game's test targets say so in a manifest, with `required-features`. Every other one gates inside
the file, with `#[cfg(feature = …)]`: the binary builds without the feature and runs fewer tests, or none. So
the list and its guard work from the test files, not only from the manifests.

- *The list covers a package's feature tests whole or by binary.* An entry names a package and its features.
  With no binary named it covers every test binary of the package, and `./dev long-tests` runs them all with
  the features on, so a new gated file runs with no edit to the list. With binaries named it covers those.
  `thrum-render` is listed whole with `creation-counters`: its gated tests are spread over the library's tests
  and a set of binaries that `RAY-TRACING` adds to each week. The game is listed by binary, because its package
  is a third of the workspace's test time.
- *At every merge*, `./dev check` builds and lints the test targets of each package the list names, with the
  features the list gives it, under the lints the rest of the workspace gets. The build of an unchanged crate is
  reused, so a merge that touches none of them pays seconds.
- *On main*, `./dev long-tests` runs them with their features on and a GPU required, in the debug build, as the
  item checks that run them do. It runs the tests that are not ignored: an ignored test there is a measurement
  that a cost script runs. From WP16 it runs each in a process of its own
  ([mechanism 16](#feature-tests-in-processes-of-their-own)).
- A test that fails at the commit WP6 starts from is listed as not run, with its failure and its owning module,
  and is not changed here. The builder's handoff names each one, so the lead can give it to its owner: until
  then it runs in no tier.
- *Membership is held by a check that reads the tests.* `./dev check-docs` fails when a test target requires a
  Cargo feature, or a `cfg` in a test file names one, and the list does not cover that file's package with that
  feature: whole, or by the binary the file builds into. Where the check cannot tell which binary a file
  builds into, as with a module several binaries share, only a whole-package entry covers it. The message names
  the file, the feature and the list. So a new gated test file either runs on main through a whole-package entry
  or fails the docs check until the list names it, and no feature takes a test out of every tier unnoticed.

**4. A merge runs the tests its change can reach, and main runs them all.** `./dev check` selects test binaries
by package. `./dev check-all` runs everything, whatever changed, and a main-branch check runs it.

- *The change* is every path that differs between the checkout and its merge base with the main branch
  (`origin/main`, as the settings name it), with uncommitted and untracked paths included. The script runs
  `./dev check` with no argument in a builder's worktree, in a merge after its rebase, and on a batch of several
  branches rebased together, and this definition gives the right change in each.
- *The selection* is each workspace package that holds a changed path, and every workspace package that depends
  on one, directly or through others, as a normal, build or dev dependency. The graph is Cargo's own, read from
  `cargo metadata`. The check runs the unit and integration test binaries of the selected packages. The
  draw-probe pass of the cooked-asset tests is one of the game's, and runs when the game's package is selected.
- *A declared edge* covers what Cargo's graph cannot show: a test that reads another package's files when it
  runs. It is a path rule that names a path and the package whose tests a change under that path selects as
  well, kept and tested with the other path rules. It adds that package's own test binaries and nothing that
  depends on it, because only the reader reads the path. One is needed today: `crates/thrum-asset-gpu/src/`
  selects `thrum-render`, whose guard that every cooked pipeline is created through the scene surface scans that
  folder. `thrum-asset-gpu` depends on `thrum-render`, not the reverse, so without the edge the one change the
  guard exists to catch would merge green. The planner's search of test code for a reader outside its own
  package found no other that needs one: the rest reach a dependency of their own package (the river model
  reads the water model's manifest), `docs/evidence`, which selects everything, or scratch under `tmp/`, which
  Git does not hold. A module that adds such a reader adds its edge in the same landing.
- *What is not selected.* The build and the lints cover the whole workspace on every merge, with the compile of
  mechanism 3, the repository guards, the script suites and the doc tests. They are what would stop every
  builder, an unchanged crate's build is reused, the Python steps take about a minute, and the doc tests 0.1 s
  in all.
- *The binaries are the whole workspace's.* The check builds the workspace's test binaries once, with the
  workspace's features, and the selection decides only which packages' tests the runner of mechanism 8 runs, by
  a filter on packages. Building a subset with `-p` is not used, because Cargo resolves features for the packages
  named, so a subset can build a different binary from the one main runs and rebuild shared dependencies.
- *A shared input selects the whole workspace*: the root `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`,
  `clippy.toml`, `.cargo/`, `dev`, `dev.ps1`, `tools/xtask`, and the test runner's own configuration
  (`.config/long-tests.txt` and `.config/nextest.toml`). The lockfile could be followed through its
  own graph, but the replay puts that at 36 s a landing, which does not pay for a reader that has to be right.
- *Paths that reach no Rust test select none*: Markdown anywhere, `docs/` outside `docs/evidence`, the plans,
  `.agents/`, `.github/`, `scripts/`, and the data under `.config/` that only the guards read. No Rust test reads
  a script or that data: a search finds `scripts/` in test code only inside `#[ignore]` reasons and one bench. The
  guards and suites that do read them run on every merge.
- *Anything else selects the whole workspace*: a path with no rule above, such as the root `tests/fixtures/`,
  `docs/evidence` or a new top-level folder; a checkout with no merge base, as in a shallow clone or CI; a
  checkout that does not differ from the main branch, which is what the main tier runs, and the script's run
  before Ready for Review; and a dependency graph the check cannot read. The check prints what it selected and why.
- *On main*, WP7 adds one line to `ORCHESTRATE.md`, in these words:

```
- **Main-branch check workspace-tests:** `./dev check-all`; every 1h; timeout 60m  *(owner, 20 Nov 2026: a merge runs the tests its change touches, and the full workspace runs on main)*
```

  It takes no lock, as `./dev check` never has: it is the load one merge's check puts on the GPU, once an hour.
  The lead asked on 21 November whether it should take `gpu`, since it runs the GPU tests beside builders
  unqueued, and it stays without. A settings line holds its lock for the whole command, so a slot would be kept
  through the build, the lints, the script suites and the CPU tests: six to eleven minutes of each hour, which is
  what the owner cut that day when main-branch checks held the GPU 12.7 hours a day. The lock counts the runs
  that need a slot for their whole length; it does not make the GPU quiet, because every merge's check and every
  builder's own run the same tests with no lock. What the unqueued run costs a builder is time, not a result:
  timings are tripwires taken under load, and captures compare pixels. Moving its GPU tests to the four-hourly
  `long-tests` run was the other choice, and it needs a way to tell a GPU test from a CPU test that nothing has
  but names.
  The builder keeps every word but the timeout, which is 60m or twice the time the check's first run took,
  whichever is longer, because that first run builds the workspace from nothing in a checkout of its own.
- *The risk accepted*, which [the test tiers](../../../ORCHESTRATE.md#test-tiers) state: the whole workspace is
  still built and linted at every merge, so no compile-time edge can be missed. What can be missed is a test
  whose result depends on something Cargo's graph does not show and no edge declares, such as a new test that
  reads a file another package owns at run time. Nothing finds such a reader when it is added: WP7's reviewer
  searches once, and after that the whole run on main is what catches it. The missed edge reaches main and sits
  there until the next whole run, up to an hour and one run, and some hours more to trace. Until it is fixed or
  reverted the script counts main as passing, because the merge that broke it passed its own check, and two
  things fail for a fault they did not cause:
  - a later merge whose selection holds the failing test;
  - every work package's Ready for Review. After a builder merges, the script runs the test command once more on
    main's tip in its own checkout, before it sets Ready for Review. That checkout does not differ from the main
    branch, so the run is the whole workspace. A pass on the same files is reused, so the run costs nothing
    when no other merge has landed since the work package's own, and with several builders one usually has.

  So no merge waits for the whole run, but a work package's Ready for Review does run it, and a fault on main holds
  every module's work packages there, not only the module that caused it. A builder whose merge or Ready for Review
  fails in a test its change cannot reach reads `python3 "$ORCHESTRATE_SCRIPT" main-check workspace-tests`
  before it debugs. Where that shows the test failing on main, the builder leaves the test alone and ends
  `BLOCKED main_broken:` with the test's name, and the lead has the commit fixed or reverted. Selecting the
  Ready for Review run by the work package's own change is the script's to offer, because only the script knows
  that range; it is an assumption in [the plan](PLAN.md) and a change to raise with the orchestrate skill.
- *Finer than a package is not built.* Nothing says which test binary of a package a source file reaches without
  a coverage map, and a wrong map misses edges silently. The game's package is a third of what is left and
  depends on everything; it leaves the engine's check when [GAME-SPLIT](../GAME-SPLIT/DESIGN.md) moves the game.

**5. A slow test cannot enter the merge tier unnoticed.** The gale reached 13 to 21 minutes in the check with
nothing objecting. Four things now notice, and [the guard rules](#the-guard-rules) hold what they rest on:

- The runner of mechanism 8 reports each test that runs for over ten seconds, and stops and fails one that runs
  for over sixty. The failure names the test and says what to do: make it fast, or mark it
  `#[ignore = "long: …"]` and list it. Ten seconds is the test tiers' bound for a merge-tier test. Sixty is six
  times it, because the check runs beside builders: in the planner's side-by-side runs at a load of 30 to 40 a
  test took up to twice its time alone, and a test that starts a thread pool far more, which mechanism 8 answers
  by counting it as heavy. A test that fails the limit from load alone is counted as heavy; the limit is not
  raised for it. The owner accepted the hard stop on 21 November, and ten seconds for it:
  [mechanism 13](#the-stop-at-ten-seconds) lowers it.
- A test is exempt from the sixty seconds only by name, in a list kept with the runner's configuration. WP8 writes
  the list from its own measurement: every test that takes over ten seconds alone at its starting commit, each
  with the work package that removes it, which is WP2 for a test WP2 deletes, WP3 for a test of the water scene and WP9
  for any other. So the list is also the work those work packages have left. It is empty when WP3 is Done, apart
  from a test that mechanism 9 leaves to its own module's planner, and no work package of any module adds a name to
  it: a module with a slow test makes it fast or makes it a long test.
- `./dev check-docs` fails when a test marked `long:` sits in a binary the main-tier list does not name, so a
  test cannot be marked and then run in no tier.
- The feature check of mechanism 3.

**6. A run that proves what another run already proves is deleted, never both kept.** Five pairs:

- `thrum-water`'s swash test is its own test binary and is also compiled into `query_gpu` by a module
  declaration, so it runs twice (E1-07). It stays in one binary.
- `thrum-clouds` has four GPU test binaries that make two distinct conformance runs: two make the same call, and
  a third is the first part of that run (E2-01). Two tests stay, with the names they have:
  `coarse_sun_paths_match_the_cpu_reference` on the native device and
  `weather_and_sun_path_work_with_default_limits` on WebGPU's default limits, with every assertion the four made.
- The game's frustum-only flythrough of the mixed-mesh field repeats a subset of the occlusion flythrough, which
  already builds the direct and frustum hosts and makes the frustum assertions (L1-03). It goes, and the two
  flythroughs left, both long tests, share one binary.
- `clouds_display_time_gpu.rs` is an older copy of the display-phase test, which `CLOUDS` WP2.11 replaced it with
  (G2-02). It goes whole: its one differing assertion encodes the formula WP2.11 rejected.
- `./dev check` runs the 14 cooked-asset tests in the workspace pass and again in its draw-probe pass, where four
  of them assert on the probe (L1-06). The probe pass runs only the tests that read the probe. The workspace pass
  still runs all 14, because it is the only run against the production build. WP8 builds this, in the check's
  test step.

Deleting a duplicate removes no assertion. Each case is shown by naming, for every assertion of the deleted
run, the run that still makes it.

**7. A reference the 17 November bar made a diagnostic leaves the check.** The owner's rule: "A physical or f64
reference is a diagnostic", and no item holds a GPU product to one within a bound. Four tests still do, and each
is the owner's call, carried as an assumption in [the plan](PLAN.md):

| Test | Finding | What goes | What stays or replaces it |
|---|---|---|---|
| The atmosphere's Monte Carlo reference | R4-02 | The f64 path tracer, quadrature and random walks, their 198-case run and the 571 lines that test the reference itself: about 2,200 lines | The night checks and the twilight guard, which [ATMOSPHERE](../ATMOSPHERE/DESIGN.md)'s design names as guards P3, P4 and P11. The twilight guard keeps the three expected values written in it, its 2.5% bound on each channel and its atlas-plane checks. Today it is called only from inside the 198-case run, so it becomes a test of its own |
| The mirror pool's rounding-allowance proof | X1-03 | The assertion that the pool composes within computed half-float bounds, and its air and panel witnesses | A test that the pool prepares and draws. The Chrome equality of a secondary view with the main view stays whole: it is output held to a simple statement of itself |
| The cooked cloud march against an f64 transport sum | A1-04 | The 431-line test that slices the production shader | A short test through the real contribution: zero density keeps the sky and density darkens it. It goes through the cloud-factor shader variant and fails if that variant's text replacement did not match, because this is the only test that reaches it |
| Scene-view shader hashes frozen at a 16 November commit | L5-06 | The test that patches later modules' shader edits back out, and its patch table | The comparison made at one commit, with default features, for each of the four air and shadow variants: the main shader composed with an empty observer equals the one composed with an observer that declares a view slot and leaves it idle, and holds no `secondary_view` text. Today that comparison is behind a feature `./dev check` does not turn on, and covers one variant |

**8. Tests run side by side, on the build a measurement picks, and the check says where its time went.** This is
what brings a merge's check from twenty minutes to a few, and WP8 builds it.

- *The runner.* `./dev check` and `./dev check-all` run tests through cargo-nextest: each test in a process of
  its own, as many at once as the machine has cores, across every test binary, from one build of the workspace's
  tests. It is what large Rust workspaces such as wgpu's and Zed's run their tests with, and Zed's CI selects the
  tests of changed packages and their dependants through its filters, as mechanism 4 does. Doc tests, which it
  does not run, go through `cargo test --doc` as before. A failed test is not run again: a retry would hide a
  test that fails one time in ten. `./dev long-tests` keeps `cargo test`, one binary at a time, for the long
  tests, because a long test is run alone on purpose. WP6 ran the tests behind a Cargo feature the same way,
  and [mechanism 16](#feature-tests-in-processes-of-their-own) moves those to this runner.
- *It is required.* The check names the oldest cargo-nextest it accepts and, where that is not installed, stops
  before any step with the command that installs it. One runner means one behaviour to trust: a fallback to
  `cargo test` would be the slow path nobody runs. The manual CI workflow installs it.
- *A test is alone in its process.* It shares no statics with another test, so a mutex in a test file no longer
  makes tests take turns. Five files hold one today: `cooked_assets_gpu.rs` and `frame_alloc.rs` in the game,
  `foam_scene_gpu.rs` in the water scene, `interaction_fixture.rs` in `thrum-water` and
  `generation_transition/metal.rs` in `thrum-render`. Where tests must still take turns, the runner's
  configuration puts them in a group that runs one at a time, beside everything else. WP8 keeps a file's tests in
  such a group unless they pass five whole runs without it and measure nothing another test could move, such as
  GPU memory in residence. The planner's two runs passed with no group at all, which shows it is possible, not
  that it is safe.
- *A test that fills the machine says so.* A test that starts a thread pool, as every river-model test does, is
  counted as several tests by the runner, so fourteen of them do not start fourteen pools. Without that the river
  model's 71 tests took 474 s summed, against 71 s for the package run alone.
- *The build.* Test builds may optimise dependencies fully and workspace crates lightly. WP8 times the whole
  workspace's tests three ways on one commit, each twice, alternating, each from the same starting state with its
  build counted: as the check ran them, side by side, and side by side on the optimised build. Side by side
  goes on if both its runs are shorter than both runs as it was. The optimised build goes on only if both its runs
  are shorter than both side-by-side runs without it. Only test builds change: what `./dev scene`, the showcase
  and the exact images build is as it was. Debug assertions and overflow checks stay on, and the four model
  crates keep their overrides, so no test's meaning changes. Every test that passed before passes after, compared
  name by name.
- *The check says where its time went.* It ends, pass or fail, with the wall time of each step and the ten
  slowest tests. The measurement above had to be rebuilt from cargo's and libtest's lines; the next one is read
  from the check's own output, and WP7 and WP5 make their records from it.
- *The draw-probe pass runs four tests.* It is a second build of one game binary with a feature on, and it ran
  all 14 cooked-asset tests again, 70 s. It runs only the tests that assert on the probe (mechanism 6).
- *A capture waits for its own readback.* Tests side by side load the machine, and the first fault the load
  found is in the engine, older than WP8. `FrameHost::capture_pixels` waited for its submission, looked once for
  its readback and failed with `capture readback incomplete` when another thread's poll of the same device had
  taken the readback's completion and not yet delivered it. The cooked loader polls that way, and
  `lent_products_gpu` failed so in one merge check. After its submission completes, `capture_pixels` keeps
  polling until its own readback arrives, whichever thread delivers it, and fails only when `GPU_WAIT` has
  passed. No pixel a capture returns changes, and nothing a frame records. WP8 builds it, with a test that fails
  on the code before it: a second thread that only polls does not, and
  [the planner's probe](../../../.orchestrate/evidence/GATE-TIME/planning/capture-race-probe.md) shows what does.
- *Steps that share nothing may run at once.* The Python guards and script suites, about a minute, read the tree
  and build nothing the Rust steps use. WP8 may run them beside the build if its timing shows a gain, with each
  step's output printed whole.

**9. A test over ten seconds is made fast, or leaves for the main tier with its risk written down.** WP2, WP9 and
WP3 empty the exempt list of mechanism 5, a test at a time, by the first of these routes that works. WP9 and WP3
take the same routes to a test that is under ten seconds alone and over it in the check, which
[the table](#every-test-over-about-ten-seconds) names:

- *The build.* If WP8 switched the optimised build on, many tests are already under ten seconds and only lose
  their exemption.
- *One case to a test.* A test that runs several cases in turn becomes one test for each case, with the names
  saying which, so the runner spreads them over the cores. No case is dropped.
- *The smallest world.* A test proves its behaviour on the smallest world that shows it: the grid, valley, cooked
  store, count of entities, ticks or frames it builds to run on. The river model's
  `no_step_is_taken_beyond_the_budget` is the example: it solves `valley(0.25)` 43 times to prove that no step is
  taken past a budget, which the smallest grid whose solve takes more than a few steps also proves. A world, a
  fixture or a run is made smaller only where every assertion still means what it meant: the log gives the old
  and new size and why. Setup that several tests repeat is built once, on disk, and read by each: every test is
  a process of its own, so a static is built again for each test. A run whose length or
  size is what it tests, such as a soak, is not shortened: it is a long test. Where full size shows a fault the
  small world cannot, the full-size run is a long test as well, and the small test stays because it proves the
  behaviour itself. That is not the shortened run mechanism 1 refuses, which proves only part of what its long
  test proves.
- *Without a device.* Logic that a CPU can decide leaves the GPU test for a test that asks for no device: a
  selection, a layout, a state machine, a record's bytes, a refusal. The GPU test that stays covers what a device
  alone can show, at the smallest target and for the fewest frames that show it. Each assertion is still made by
  one of the two, and the log says what the GPU test still shows. GPU state held to the CPU query a game reads is
  what a device alone can show, so those tests get a smaller target and keep their comparison.
- *A long test.* Otherwise the test is marked and listed as mechanisms 1 and 2 say, whole, with its body
  unchanged, and the fault that now reaches main unseen is written beside its line in the main-tier list, where
  the next reader of the list finds it. It runs in release unless it fails there.

Three limits. WP9 and WP3 rename no test binary, because open items of other modules run binaries by name;
[mechanism 10](#one-test-binary-a-package) says which binaries WP11 folds. A test is
not marked as a long test while an open item of another module depends on what it asserts: before marking one,
search the open items of every module for its binary and read each item that runs it. An item that needs only that
the binary passes, as `PUDDLES` WP5.4 does of the foam scene's, does not hold the test. Where an item does, the
test keeps its exemption and the log names it for the planner of that module. WP9.3, WP3.1 and WP3.4 are the
done-when items that ask for these long tests, so from WP12 the landing that marks one raises the version of the
rule `main-tier`, as [mechanism 12](#the-guard-rules) says. And the proofs the bar keeps before
a merge (cooked formats, leases and recovery, transport, determinism, and GPU state held to the CPU query a game
reads) are the last to move: one is marked as a long test only when the log says what was tried to make it fast.

<a id="two-rules-for-new-tests"></a>

Two of mechanism 9's routes, the smallest world and without a device, are also standing rules for every test
written or changed from 21 November 2026, in any module, and [the test tiers](../../../ORCHESTRATE.md#test-tiers)
state them. WP9 and WP3 apply them to the tests over ten
seconds that exist. No work package sweeps the tests under ten seconds: [Not built here](#not-built-here) says why.

<a id="one-test-binary-a-package"></a>

**10. A package's integration tests are one test binary.** WP11 builds it. Cargo makes every file in a package's
`tests/` folder a binary of its own, each linked against the package's whole dependency graph and each checked by
macOS on its first run. So each test file becomes a module of one test target in its package, named `it` in every
package, and a file's tests run as `cargo test -p <package> --test it <file>::`. Cargo's own repository and
rust-analyzer keep their integration tests this way. The files stay where they are wherever Rust's module rules
allow, because a mass move would collide with every builder.

- *What stays a target of its own.* A file whose binary something names, or that has to run apart:
  - a binary an open item's check names with `--test`, in any module, until that item's work package is Done;
  - a binary the main-tier list names, and any file that holds a long test or a test behind a Cargo feature.
    When WP11 was built, `./dev long-tests` ran all of those through `cargo test`, where the tests of one
    binary share a process, and folding them would have made tests share statics, creation counters and
    device mutexes they never shared. It still runs the long tests so. These files stay targets of their own
    after WP16 too, which folds nothing;
  - a target with `required-features` or `harness = false`, and one built for another target, such as the wasm
    tests of `thrum-asset`;
  - a binary a script or a tool runs by name.
- *What the count becomes.* By the planner's count, about 110 test binaries where a whole run has 263: 248 of the
  277 integration test targets are named by no open item, and about 20 of those hold a long test or a test behind
  a feature. WP11's record gives the real figure, and what a freshly linked set costs to link and to run for the
  first time, before and after. If the later timings are not shorter, the builder ends `BLOCKED` with the record.
- *No test is lost.* A file that no target declares is compiled by nothing, and Rust does not say so. So the
  repository guards fail a Rust file under a package's `tests/` folder that no test target compiles, with a
  message that says where to declare it, and WP11 compares the names of every test before and after.
- *Names that move.* A folded test's name gains its file's module, so the groups, heavy counts and exempt names
  of `.config/nextest.toml` are written again to select the same tests, and a test that starts itself again by
  name, as the water scene's cooked-island tests do, passes its new name.
- *A target with no test in it is not a test binary.* A `bin` or `example` target that holds no test sets
  `test = false`, so the runner does not build and start it.
- *From WP11 on.* A new test file is a module of its package's `it`. A planner's new item runs it as
  `cargo test -p <package> --test it <file>::`, and a planner who re-plans a module writes the checks of its
  unstarted work packages the same way. An item approved before WP11 keeps its check: its builder adds the file as a
  target of its own, and the file joins `it` in the first landing that edits it after the item's work package is
  Done. So the count goes towards one a package as the open items are built, and no item is changed for it.

**11. Where a builder's session goes is measured before any rule about it changes.** WP10 makes a record and
changes nothing else. For each builder session of track 6's work packages it splits the wall time into the model's
own time, the commands the builder ran by kind, and waits: for a lock, behind another merge, and in a command
that only sleeps or polls. The sources exist today. Each session's transcript gives the time of every tool call
and of its result (`~/.claude/projects/` for a Claude builder, `~/.codex/sessions/` for a Codex one, with the
session's id in `.git/orchestrate/usage.jsonl`), and the orchestrate script logs the jobs it starts, every lock
asked for, got and released, and each merge's check (`jobs.jsonl`, `locks.jsonl`, `checks/` and `merge-queue/`
under `.git/orchestrate`). The record ends with what it shows and proposes no rule: which rule to change, if
any, is the owner's to decide from it. It is built straight after WP2 and ahead of WP9, where the lead asked for
new work to go last: it touches no tracked file but this module's records, it needs none of the later
work packages, three of which wait for other modules, and the owner asked for it before any rule changes.

WP11 is likewise built ahead of WP3 and WP4, which wait for `WATERS` WP7 and `REVIEW-RENDER` WP2. It needs neither,
the binaries their items name stay targets of their own, and every merge pays the first-run cost until it lands.
It follows WP9, whose tests the runner's exempt list still names by binary.

<a id="the-guard-rules"></a>

**12. Three counts that only fall hold the habits, in the merge check.** WP12 builds them. The owner's purpose is
that the fast-test habits hold "without anyone remembering them", and a merge's check is the one place every
builder passes, on Codex or Claude. Each is a rule in `REPO-RULES`' rule engine, as `test-readback` is: it
enters at the count of the commit it lands on, that count can only fall, and a landing that raises it fails
`./dev check` and `./dev check-docs` with a message that says what to do and links to its entry in
[the engineering patterns](../../../.agents/skills/thrum-engineering/references/patterns.md#tests), which
[TEST-SUPPORT](../TEST-SUPPORT/DESIGN.md#writing-a-test) WP6 writes. The engine itself does not change.

- *`limit-exempt`: the tests exempt from the limit.* It counts the test names in the overrides of
  `.config/nextest.toml` that change the limit on one test: twelve today. An override that changes the limit
  names its tests one by one, exactly; one that names anything wider, such as a binary or a package, fails
  outright, because it cannot be counted. So does a default limit later than the test tiers': sixty seconds,
  and ten from WP13. WP2, WP9 and WP3 lower the count as they empty the list.
- *`main-tier`: what runs only on main.* It counts the entries of `.config/long-tests.txt` and the tests marked
  `#[ignore = "long: …"]`: ten and five today. Both are counted because a new long test in a binary the list
  already names adds no line. The owner accepted that "a new long test needs a plan decision", and the rule
  engine never grants an increase over Git history, so a plan raises the count through the rule's version: the
  engine keeps a low mark for each version of a rule, and a new version starts again from the count it lands
  with. A landing may raise the version only where a done-when item asks for the main-tier test it adds, and
  says what only that test can show, as [the project settings](../../../ORCHESTRATE.md#what-is-proved-and-how-hard)
  ask of a planner. The version is one number in the rule's registration, so the raise shows in the landing's
  diff, and that item's reviewer holds it against the item. Any other landing that adds an entry or a long test
  fails, and its message says to make the test fast, and that a builder with no such item writes the proposal
  in its module's log for a planner. A wrongly added long test costs minutes of a four-hourly run and nothing
  in a merge, which is why a visible raise is enough and no check ties a line of the list to an item.
- *`test-binary`: the test binaries no plan names.* The whole number cannot be frozen: approved open items
  name 147 binaries that nobody has written yet, and each will be a new target when its work package is built. So
  the rule holds the others. A plan names an integration test target, which is a `[[test]]` entry or a file
  Cargo finds under `tests/`, when a done-when item's check runs it with `--test`, in any module, open or Done,
  or when the main-tier list names it. Open and Done items both count, as the rule `script-caller` already
  reads them, so closing a work package changes nothing. The targets no plan names are written once, by package
  and name, in a new tracked list, `.config/unplanned-test-binaries.txt`: 43 of the 275 today. Two things then
  hold. Every target is named by a plan or is on the list, so a new test file that makes a target nobody
  planned fails, with a message that says to add the test to a test file that exists. And the list only
  shrinks: the rule counts its lines, and a line whose target is gone fails until it is removed. A package's one
  test target `it`, which WP11 creates, is the test tiers' own and needs neither a name nor a line. WP11 folds
  what the list names into `it` and takes each folded target's line off: the freeze comes first and the
  consolidation second, as the owner asked. The list, and not a count of unnamed targets, is what the engine counts, because a count
  that a plan's wording can move would fail the next merge in every module whenever a planner named an old
  binary in a new item. One case is left to planners, as with `script-caller`: a re-plan that drops the last
  check naming a binary which exists leaves it unnamed, so a planner keeps that name until the file has joined
  `it`. This reading of "can only go down" is an assumption in [the plan](PLAN.md).

WP12 is built after WP10 and ahead of WP9, so that the counts are frozen before WP9 and WP11 lower them. It waits for
`TEST-SUPPORT` WP6, because each message links to an entry WP6 writes.

<a id="the-stop-at-ten-seconds"></a>

**13. A test is stopped at ten seconds, and one that load alone took there passes on a second run.** WP13 builds
it, straight after WP9: the owner accepted the ten-second limit, to drop "once GATE-TIME WP9 lands". The runner
stops a test at ten seconds where it stopped it at sixty. The test tiers define the bound for a test run alone,
and the check runs beside builders, where 8 of the last 18 whole checks took a test of under ten seconds past
it. So a test the limit stopped is run once more by the same check, alone, after the others, under the same
limit. It fails the check unless it passes within ten seconds, and the closing lines name each test run again
with its time alone. That is the limit the owner accepted, measured the way the tiers state it. A stop with no
second run would fail good work for the reason a gate on a check's total time would. It is an assumption in
[the plan](PLAN.md).

- A test that fails any other way is still not run again, so a test that fails one time in ten still fails the
  check. What the second run can hide is a test that hangs one time in ten: it is named in the closing lines of
  every check in which it was stopped, and nothing fails.
- The exempt tests, which after WP9 are the water scene's until WP3, are reported and not stopped, as before.
- Every test off the exempt list was under ten seconds alone when WP8 wrote the list, so each passes its second
  run. One that is over ten seconds in every check would be stopped and run again in every check, which is why
  WP9.1 and WP3.3 hold a test to ten seconds in the check too, and WP13.2 records three whole runs.
- The bound a test is written to is unchanged: about ten seconds or less alone, and far under it wherever the
  smallest world allows.

**14. The check reports the tests' summed time and the GPU tests' share.** WP12 adds two lines to the closing
lines of `./dev check` and `./dev check-all`: the summed time of every test the run ran, and the summed time of
its GPU tests. A GPU test here is one whose binary or name says `gpu` or `metal`, which is how the measurement
of 21 November counted 221 of them at 53% of the time; a GPU test named otherwise is not counted, and the line
says how it counts. Both are reports. Nothing fails on them, or on a step's time or the whole check's, because
a check's length follows the load it runs under: the same tests summed to 820 s and to 1,471 s in two checks
of 21 November.

<a id="workspace-reruns-under-builder-load"></a>

**15. The workspace check gives a stopped test one bounded serial rerun under builder load.** WP14 builds this
before WP5. The runner already waits for its parallel nextest invocation to exit and runs exact timeout cases
with one test thread. That releases its own test jobs. It leaves the builders, other main checks and `exact-images`
running. On 24 November, at about 40 load on 14 cores with nine builders, both attempts reached ten seconds for
different CPU and GPU cases across the three runs at 02:45:14, 03:54:31 and 04:00:03. The latter two checks even
ran the same commit. The same sea case passed its rerun in 5.5 s in the earlier check and timed out in the later
one. The rain and river CPU cases also passed in other runs.

Use the normal ten-second stop for the side-by-side pass of both commands. In `./dev check-all` only, allow up to
30 seconds for each serial rerun of a timed-out test, after the parallel invocation has exited. Use the same test
build, features and exact test identity. Apply this to the workspace pass and the draw-probe pass. A successful
rerun requires a passing result for that exact case. Assertion failures, repeat timeouts, missing results and
runner errors still fail the check. The report gives the case, both times and the verdict, and calls out reruns
that passed above ten seconds so they remain visible for speed work. `./dev check` uses ten seconds for both
attempts, even when its selection runs the whole workspace. The fast-test target, the empty exemption list and
the guard counts stay as in WP12 and WP13.

Allowing more time on the workspace rerun covers both CPU scheduling and GPU contention and costs extra wall time
only for a test that already stopped. A test that now takes more than ten seconds can pass `check-all` within the
rerun cap and is reported as over the ten-second target. The merge check still enforces that target. Taking a
shared `gpu` slot would still allow other GPU holders to run. Taking `gpu.alone` would queue behind other work,
leave uncoordinated GPU work running and leave CPU contention in either case. Moving the serial pass to the end of
the command would release no additional nextest jobs and would leave the same external load. Keep the rerun after
its own parallel pass. The 30-second cap is provisional until the loaded proof passes.

The incident cases are `thrum-rain-model::it conformance::the_harness_passes_rain_bands_against_themselves_and_reports_ignored_exposure`,
`thrum-river-model::it chop::each_sample_of_a_river_file_carries_its_own_gains`,
`thrum-river-model::it wetting::the_hollow_and_the_terrace_rise_from_nothing_with_no_step`,
`thrum-river-model::it wetting::the_rules_the_wetting_replaces_would_fail`,
`water-scene::foam_scene_gpu craft_trail_outlives_removed_source_and_spreads` and
`thrum-sea::it portable_limits_gpu::default_webgpu_limits_match_the_model_a_day_in`.

<a id="feature-tests-in-processes-of-their-own"></a>

**16. On main, a test behind a Cargo feature runs in a process of its own.** WP16 builds this before WP5. The
main-branch check `long-tests` failed in five of its nine runs from 27 November 15:07 to 28 November 00:34,
each time in `thrum-render`'s library tests with `creation-counters`, with
`Timing(TimestampQuerySetsExhausted)` from one or two of the `runtime::tests::device::native::provider` tests.
The same commit failed one run and passed the next, so the trace could name no commit.

What the planner measured on 28 November
([record](../../../.orchestrate/evidence/GATE-TIME/WP16-plan/README.md)):

- Nothing leaks. Run one test at a time, every test's first request for a timestamp query set finds none live
  in the process, and the binary passes. The engine's own holdings are small: a frame host holds two, and five
  with a submitted capture on.
- The fault is what shares the process. Metal gives one process 32 live timestamp query sets
  ([`APPLE-PRESENTATION`'s design](../APPLE-PRESENTATION/DESIGN.md#timestamp-query-sets-and-metals-limit-wp16)).
  Two of the provider tests hold 17 each at their peak, with three capture hosts alive at once, and a third
  holds 12. The library's tests have a cap of their own from 8 November, `timestamp_permit.rs`: three such
  tests at once, on the assumption that the heaviest holds 8. Three can now hold 46, and 11 tests that hold 4
  to 7 take no permit at all. The three provider tests alone in one process failed 20 runs of 20, and the
  whole binary 7 of 20.
- No other tier can see it. `./dev check` runs each test in a process of its own, where it has all 32, and an
  item check runs the one test or the few it names. `./dev long-tests` is the only tier that runs a whole
  binary's tests in one process, and it is the third time in a week that this alone failed it: `APPLE-PRESENTATION` WP16 and WP19 were the first two.
  `MESH-SHADING`'s `mesh_stage_gpu` is listed as not run because it fails run whole and passes one test at a
  time.

So the cause is in this module's runner, and a larger or better-kept cap in one crate would leave the next
process-wide limit to fail the same way. [The test tiers](../../../ORCHESTRATE.md#test-tiers) already say that a
test runs in a process of its own and relies on no static it shares with another. WP6 made the main tier the
one place where that is untrue.

- *The runner.* `./dev long-tests` runs each feature line of the main-tier list through cargo-nextest, as
  mechanism 8 runs a merge's tests: every test in a process of its own, side by side across the line's
  binaries. A line still means what mechanism 3 says: its package, its features, every test binary or the ones
  it names, the tests that are not ignored, the debug build, a GPU required. One line is one run, because
  features differ between lines, and a line that fails does not stop the lines after it. The long tests do
  not change: `cargo test`, one binary at a time, in the build the list gives.
- *The limit is a hang guard.* These runs use a profile of their own in `.config/nextest.toml`, `main-tier`.
  It stops a test at 120 seconds, runs no test again and reports every failure of a line. The ten-second rule
  is the merge tier's and does not move here: these tests run in no merge's check, one of them takes 24 s
  alone, and a wrongly stopped test costs a trace of whole runs over some hours. In the planner's trial the
  ten-second limit stopped 1 to 4 of `thrum-render`'s 601 tests in each run under builder load, and a
  30-second one stopped a game test once, on the first run after its build, that takes 1.9 s alone. The
  script already runs a failed check once more whole, so the profile retries nothing. The groups and heavy
  counts of the default profile apply to these runs as they do to a merge's.
- *It needs the runner installed.* `./dev long-tests` stops before any step when cargo-nextest is missing or
  older than `./dev check` accepts, with the same message.
- *No test file changes.* A test that fails only because tests of its binary now run at once in separate
  processes joins a group in `.config/nextest.toml`, by WP8's rule. The planner's trial needed none: with a
  30-second limit all 601 tests of `thrum-render`'s line passed in both of two runs, both lines of
  `thrum-asset-gpu` and `thrum-clouds`' one passed, and the game's 96 passed in two runs of three, the first
  losing the one test to the limit as above. A mutex a test file keeps for `cargo test` stays and is never
  contended.
- *What it leaves alone.* The main-tier list and its format, the `not-run` lines, the settings line of the
  check, and every test file. The owner of a `not-run` binary may find that it passes now, and moving it to a
  feature line is that owner's landing.

The trial also timed the runs: `thrum-render`'s line took 34 to 47 s through cargo-nextest against 91 s one
binary at a time, and the game's 20 to 38 s against 48 s, so the check holds its `gpu` slot for less.

## Cargo cache for merge verification

The pre-rollout WP5 record found 404.6 seconds median across 45 Rust landings, with 37 over five minutes. In the
slow group the median test build was 142.0 seconds, compared with 12.7 seconds in the group at or under five
minutes. The Nextest run median across all 45 was 59.3 seconds. Package-selected and whole-workspace checks both
missed the wall-time target. The production runner uses a workspace build and filters which of its binaries run.
The lead also removes idle `.git/orchestrate/verify/*/target` directories to protect free disk on the 14-core Mac
shared by ten builders. The timings mix load, changes and cache states, so WP15 measures reuse directly before
switching on the new route. [Baseline](../../../.orchestrate/evidence/GATE-TIME/WP5/record.md).

WP15 configures `THRUM_MERGE_VERIFY=1 ./dev check`. The environment variable opts into the cache on WP15-capable
checkouts. Earlier launchers receive the existing argument list and run their ordinary check. Orchestrate's
broken-main diagnosis uses the current command on earlier revisions, so replay the configured invocation on a
known-good revision before the WP15 launcher was introduced and verify that its ordinary check passes.

On this Mac the cache runs the existing `./dev check` pipeline in one stable, detached checkout and reuses a
Cargo target directory owned by that mode under the common Git directory, outside `orchestrate/verify`. The same
route serves singleton and batch candidate trees. Admit a caller only when its check inputs can be represented by
its committed revision in the shared checkout, with the caller's merge base and selection inputs preserved. Any
staged or unstaged tracked change or non-ignored untracked path takes the ordinary check path in the caller's
worktree. Use that path also when a checkout-local input differs or equivalence cannot be established. A commit
ID alone cannot represent the local changes included by [mechanism 4](#mechanisms).

Cargo owns freshness and invalidation. Keep rustc incremental compilation disabled. Keep the existing test
selection, workspace features, lints, doc tests, draw-probe, GPU requirement and failure handling. Run guards
and selected tests for the candidate normally. The persistent files are build artifacts, and orchestrate still
owns reuse of successful check results. [Cargo build cache](https://doc.rust-lang.org/cargo/reference/build-cache.html).

One process exclusively owns the checkout and target through the full check, including test execution and
maintenance. Ordinary developer commands and the scheduled main-branch checks use their existing directories.
If the cache is disabled, unavailable or busy, or admission declines the shared checkout, the configured
command reports the reason and runs the ordinary check in the caller's worktree with its original inputs.
Only cache setup or admission can choose the fallback. A failure once the check starts returns its verdict.
The shared checkout tracks one candidate at a time. Keep generated per-run state isolated, and check that
changes to source, features, profile and toolchain rebuild what they affect. Recovery after interruption starts
from the next candidate's clean inputs.

Start with an 8 GiB retained build-cache cap and the lead's 30 GB free-space floor. Define a measured headroom
allowance for transient build growth. Before a run that cannot fit, and after an oversized run, reclaim only
this mode's idle or obsolete artifacts under its lock. In low space, release this mode's retained data as needed
and use the ordinary check path if the shared cache cannot be admitted. The floor controls this cache's retention
and admission on a shared volume. Report retained bytes, available space, and cache use, resets and fallbacks.
The contributor guide tells the lead where the cache lives, how disk pressure is handled and how to disable it.

Use three distinct representative candidate transitions on the Mac under ordinary builder contention. Include a
package-selected and a whole-workspace check, and exercise the configured command on singleton and batch inputs.
Compare cold ordinary checks with the warmed route on the same candidate/base pairs, preserving the production
selection and check steps. Use a preceding checkout to warm the cache for each candidate. An unchanged-tree
repeat is a useful diagnostic. For promotion, the three candidate-route checks have a median of at most 300 seconds
end to end and a lower median test build time than the matched cold checks. Record load, disk use, Cargo rebuilds,
test build and run times, verdicts and executed coverage. If reuse falls short, WP15 may also tune the test profile's
debug-info generation, preserving test assertions and diagnostics and repeating the paired proof. Switch the project
test command only after it passes this proof. WP5 measures the real rollout independently of the replay.

## Interfaces

| Interface | For |
|---|---|
| A long test: marked `#[ignore]` with a reason that starts `long:`, in a binary the main-tier list names; the main-branch check `long-tests` runs it | Any module with a test that takes over about ten seconds and cannot be made fast. Its builder adds the binary to the list, with the fault the test leaves unguarded before a merge |
| The main-tier list, `.config/long-tests.txt`: each long-test binary with its package and build, and each package with tests behind a Cargo feature, with those features, whole or by test binary. The docs check fails a test file or target that names a feature the list does not cover | Any module that adds a long test, or a test behind a Cargo feature in a package the list does not cover whole |
| The merge selection: `./dev check` runs the tests of the packages a change can reach, by Cargo's graph and the path rules and declared edges of mechanism 4; `./dev check-all` runs the whole workspace | Every builder. A module that adds a folder outside a package gets the whole workspace on every change there until it adds a path rule. A module that adds a test reading another package's files at run time declares the edge in the same landing |
| The runner, from WP8: a test runs in a process of its own, beside tests of every other binary, with cargo-nextest installed. From WP13 a test is stopped at ten seconds and rerun once serially under the same limit. From WP14 only `./dev check-all` gives that serial rerun 30 seconds and reports passes above ten seconds. `.config/nextest.toml` holds the groups of tests that take turns and the tests counted as heavy. From WP16 `./dev long-tests` runs the tests behind a Cargo feature the same way, a process to each test, under the profile `main-tier`, whose only limit is a hang guard of 120 seconds | Every module that writes a test. A test relies on no other test of its binary having run and on no static it shares with one. A test that needs a device to itself joins a group, and one that starts a thread pool is counted as heavy, in the same landing |
| Two rules for a test written or changed from 21 November 2026, which [the test tiers](../../../ORCHESTRATE.md#test-tiers) state: it uses the smallest world that shows its behaviour, and it tests logic without a device, leaving a GPU test only what a device alone can show | Every module that writes a test, and every planner and reviewer |
| The guard rules, from WP12: `limit-exempt`, `main-tier` and `test-binary`, each a count that can only fall. `main-tier` rises only through its version, in a landing that a done-when item asks for. `test-binary` counts the lines of `.config/unplanned-test-binaries.txt`, and fails a test target that is neither on that list nor named by a plan | Every builder: no new exemption from the limit, no main-tier test without an item that asks for it, and a new test in a test file that exists. A landing in any module that deletes, renames or folds a target on that list takes its line off the list, whatever files the module's plan gives it. Every planner: an item that adds a main-tier test says what only it can show; a binary a builder is to create is named by an item's check; and the last check naming a binary that exists is not dropped |
| The check's closing lines, from WP8 and WP12: the time of each step, the ten slowest tests, the summed time of the tests and of the GPU tests. Reports, never a gate | The lead and every builder, and WP5's record |
| The orchestrate test command, from WP15: `THRUM_MERGE_VERIFY=1 ./dev check`, with a bounded local Cargo cache for equivalent committed inputs and the ordinary check verdict in the caller's worktree otherwise. Pre-M15 launchers run their ordinary `./dev check`. | Orchestrate singleton and batch verification, builder checks, broken-main diagnosis, `REPO-RULES`, the lead who manages free disk, and WP5's record |
| One test binary a package, from WP11: a test file under a package's `tests/` folder is a module of the package's `it` target, and runs as `cargo test -p <package> --test it <file>::`. The repository guards fail a file there that no target compiles. A file is a target of its own only while something [mechanism 10](#one-test-binary-a-package) lists holds it | Every module that adds a test file, and every planner who writes a check that runs one |

## Whose code this is built in

Every file belongs to another module, as with [track 4](../ARCHITECTURE.md#seam-rulings-for-track-4): `WATERS`'
water scene and shore tests, `SHOWCASE`'s exit scene test, `CULLING`'s field flythroughs, `CLOUDS`' conformance
tests, `ATMOSPHERE`'s reference tests, `SCENE-VIEWS`' pool and scene-view tests, `CONTENT-LOOP`'s cooked tests,
`RIVERS`' and `SEA`'s model and conformance tests, and `REPO-RULES`' check command and repository guards.
Ownership does not move. WP11 changes how every package's test files are built into binaries, and nothing a test
asserts. The Done items that name these tests keep their text, because
[a Done item's check is its record](../../../ORCHESTRATE.md#targets).

One change is to the engine and not to a test: the wait in `FrameHost::capture_pixels` (mechanism 8), in
`APPLE-TILE`'s frame host. The lead gave it to WP8 on 21 November, because it is about ten lines that change
nothing drawn or timed, no builder was at work in `thrum-render`'s runtime, and starting a finished module for it
would cost more. `APPLE-TILE`'s log records the hand-over, its design stays the authority for the frame host, and
the test belongs to it once landed. Any other engine fault a work package here finds is still a proposal to its
owner.

## Files each work package touches

The lead confirms these against the builders at work before it starts a work package. A work package touches nothing
outside its row; a builder that finds it must writes the proposal in the log and ends `BLOCKED`. Four files are
everyone's: `.config/shape-baseline.json`, `.config/unplanned-test-binaries.txt`, `docs/design/maturity.json` and
`Cargo.lock`. Change them when your landing moves them, whatever your row says. The rows are in build order.

| Work package | Files | Waits for | Who else is near |
|---|---|---|---|
| WP1 | `examples/water/tests/floating_gpu.rs`; `examples/v1-game/tests/exit_scene.rs`; `examples/v1-game/tests/support/field_gpu.rs` and `field_occlusion_gpu.rs`; `crates/thrum-water-model/tests/shore_blend.rs`; the new command and the help text in `tools/xtask/src/main.rs` and its line in `dev`; the new `.config/long-tests.txt`; the listed-binary check in `scripts/repo_check.py` with its suite; the one settings line in `ORCHESTRATE.md` | Nothing. It merges first in wave 1 | No open item runs any of these tests. `DEAD-TOOLS` WP1 and WP2 edit other arms of `tools/xtask/src/main.rs` and the same help text. `scripts/repo_check.py` is imported by a pinned comparison file: it keeps its name and what that file uses. The check keeps a checkout and a release build of its own, so the lead checks free disk first: builders stall below about 16 GiB |
| WP6 | The lint step and the long-test command in `tools/xtask/src/main.rs`; `.config/long-tests.txt`; the feature check in `scripts/repo_check.py`, which already tells a test file from production code, with its suite; the timeout of the settings line | Nothing | `DEAD-CODE` WP2 and WP6 and `TEST-SUPPORT` WP3 wait for it. `DEAD-TOOLS` WP1 edits the script-suite step of the same `main.rs` |
| WP8 | The check command in `tools/xtask/src/` (how it runs tests and the draw-probe pass, the step times, the cargo-nextest version it asks for) and its tests; the new `.config/nextest.toml`; the `[profile]` tables of the root `Cargo.toml`; the install step of `.github/workflows/ci.yml`; the line of `CONTRIBUTING.md` that says what `./dev check` needs installed. Where a test fails only because it now has a process to itself, that test file. The wait in `FrameHost::capture_pixels`, in `crates/thrum-render/src/runtime.rs`, and its test under `crates/thrum-render/src/runtime/tests/`: no other line of the engine's source | Nothing | `REVIEW-RENDER` and `HOST` build in `thrum-render`'s runtime too, so the lead starts neither beside this. `DEAD-TOOLS` WP2 edits the `baseline-inventory` command and the help text of the same `main.rs`, and other rows of `CONTRIBUTING.md`. `DEAD-CODE` WP5 edits the workspace list of the root `Cargo.toml`, a different table. A changed test profile rebuilds the test artifacts of every worktree once, so the lead checks free disk first. `TEST-SUPPORT`'s design says `GATE-TIME` WP5 narrows the draw-probe pass: that is this work package now, in the check command only |
| WP7 | The check command in `tools/xtask/src/` (the selection with its path rules and declared edges, `check-all`) and its tests; one settings line in `ORCHESTRATE.md` | Nothing | `DEAD-TOOLS` WP2 edits the `baseline-inventory` command and the help text of the same `main.rs`. The main-branch check keeps a fourth checkout with a debug build of the whole workspace, 8 to 16 GiB in the checkouts that hold one today, so the lead checks free disk first |
| WP2 | `crates/thrum-water/tests/query_gpu.rs`, `swash_gpu.rs` and `crates/thrum-water/Cargo.toml`; the four conformance files, `support.rs` and `Cargo.toml` of `crates/thrum-clouds/tests`; `examples/v1-game/tests/support/field_gpu.rs` and `field_occlusion_gpu.rs`, `examples/v1-game/tests/clouds_display_time_gpu.rs`, and the `[[test]]` tables of `examples/v1-game/Cargo.toml`; `.config/long-tests.txt`; the names of deleted tests in `.config/nextest.toml`; `crates/thrum-render/src/atmosphere/gpu/tests.rs` and `tests/reference*` | `TEST-SUPPORT` WP2, which rewrote these test files | `CLOUDS` builds in `thrum-clouds`. No open item names any of these test binaries. `DEAD-TOOLS` WP2 edits the `[[bench]]` tables of the same `Cargo.toml`, and its WP3 edits `atmosphere/gpu/tests/aerial_ground.rs`, which `atmosphere/gpu/tests.rs` declares |
| WP10 | No tracked file but this module's log and done-when file: the record and its script are this work package's evidence. It reads, and changes nothing in, the builders' transcripts under `~/.claude/projects/` and `~/.codex/sessions/` and the orchestrate script's logs under `.git/orchestrate` | Nothing | It can run beside any work package |
| WP12 | The three rules in `scripts/shape_matchers.py` and `scripts/shape_rules.py`, with their fixtures in `scripts/test_shape_rules.py`; the new list `.config/unplanned-test-binaries.txt`, with a path rule in the selection if it needs one to select no Rust test; `scripts/repo_check.py` only where the limit's own value is checked; the rules' sentences in the Tests section of the engineering patterns; the closing lines of the check command in `tools/xtask/src/` and its tests. It changes no test, no manifest, and neither the exempt list nor the main-tier list: the counts enter as they stand | `TEST-SUPPORT` WP6, whose pattern entries the messages link to | `TEST-SUPPORT` WP3 edits the exemptions of `test-readback` in `scripts/shape_matchers.py`: whichever lands second rebases. Both rule scripts are imported through a pinned comparison file, so each keeps its name and what that file uses. Every landing that lowers a count rewrites `.config/shape-baseline.json`, and from this work package a module that deletes a test file, a long test or an exempt name does too |
| WP9 | The test files that hold a test on the exempt list outside `examples/water`, or one that [the table](#every-test-over-about-ten-seconds) names as over ten seconds in the check, with the support files those tests use. By the planner's measurement: `lent_products_gpu.rs`, `cooked_rows_gpu.rs`, `cooked_assets_gpu.rs` and `frame_alloc.rs` under `examples/v1-game/tests`; `report.rs` under `crates/thrum-river-model/tests`; `shore_basin.rs` and `shore_island.rs` under `crates/thrum-water-model/tests`; `forward_cache_gpu.rs`, `portable_limits_gpu.rs`, `frames_gpu.rs` and `ambient_gpu.rs` under `crates/thrum-sea/tests`. Also `.config/nextest.toml` and `.config/long-tests.txt` | Nothing | Open items run four of these binaries by name and need them to keep passing: `DEAD-CODE` WP5.2 (`cooked_rows_gpu`), `HOST` WP3.2 and `REVIEW-RENDER` WP5.1 (`frame_alloc`), `TEST-SUPPORT` WP4.2 (`cooked_assets_gpu`). `DEAD-CODE` WP5 edits `cooked_rows_gpu.rs` and `TEST-SUPPORT` WP4 `cooked_assets_gpu.rs`. `TEST-SUPPORT` WP3 converts the test files of every crate under `crates/`, so the lead does not run it beside this where it can choose |
| WP13 | The check command in `tools/xtask/src/` (the second run of a stopped test, the closing lines, the help text) and its tests; the default limit in `.config/nextest.toml`, and a group or a heavy count there for a test the record finds stopped in every run; the limit's value in the rule `limit-exempt` | Nothing | `.config/nextest.toml` is a shared input, so this landing's own check runs the whole workspace. WP11 writes the same file's filters again, after it |
| WP11 | The `[[test]]` tables, the `autotests` line and the `test` keys of each workspace package's manifest; one new root file in each package's `tests/` folder; in each test file it folds, the lines that declare or import shared helpers, and a test's own name where the test starts itself again by name; the root file of a folder-style target, where it has to be renamed to stay a module; `.config/nextest.toml` and `.config/long-tests.txt`, for the names that moved; the lines of `.config/unplanned-test-binaries.txt` for the targets it folds; the new guard in the repository guards under `scripts/`, with its suite; the check command in `tools/xtask/src/` only where it names a test binary | Nothing | The widest set in this module. Every module that adds a test adds a file or a `[[test]]` table in these folders and manifests, so the lead starts it when few builders are in test files, and whichever lands second rebases. `TEST-SUPPORT` WP3 converts the test files of every crate and WP4 edits the cooked tests: the lead does not run either beside this where it can choose. `REVIEW-RENDER` and `HOST` build in the renderer's tests, and `DEAD-CODE` WP5 edits `cooked_rows_gpu.rs`. The binaries this module's own open items name stay targets of their own: `foam_scene_gpu` (WP3.1), `pool_gpu` (WP4.1) and `scene_view_absent_gpu` (WP4.4). A changed set of test targets links every worktree's tests again once, so the lead checks free disk first |
| WP3 | `examples/water/tests/foam_scene_gpu.rs`, `wake_scene_gpu.rs` and any other test file of the water scene that holds a test on the exempt list; the child's arguments in `examples/water/tests/support/island.rs`; `.config/long-tests.txt` and `.config/nextest.toml` | `WATERS` WP7, whose open items run the foam test for its ten minutes and which is building in these files | `PUDDLES` WP5.4 runs the foam scene's binary and WP3.2 `stage_gpu`, and each needs only that the binary passes. `stage_gpu`, `inland_floating_gpu` and `bodies_gpu` also start a child through `island.rs`, each with a test that is not ignored, so running ignored tests in the child changes nothing for them |
| WP4 | `examples/mirror/tests/pool_gpu/` and the result parsing of `scripts/check_mirror_browser.mjs`; `crates/thrum-asset-gpu/src/draw/tests.rs`; `crates/thrum-render/tests/scene_view_absent_gpu.rs` and `tests/pre_record_source/` | `REVIEW-RENDER` WP2, which is extending `pre_record_source` | `CLOUDS` and `RAY-TRACING` edit `thrum-asset-gpu`'s `draw.rs`, not its tests |
| WP14 | The check runner, its tests and help in `tools/xtask/src/`, `.config/nextest.toml`, and the corresponding timeout text in `ORCHESTRATE.md` and `CONTRIBUTING.md`. Load evidence in the working folder | The landed WP13 runner | `REPO-RULES` owns the check command outside the test runner. WP5 uses the contributor guide's merge timing |
| WP15 | `dev` and its merge-verification helper and tests under `scripts/`, the check entry point and selection inputs in `tools/xtask/src/` if needed, the test-command line in `ORCHESTRATE.md`, the cache operation notes in `CONTRIBUTING.md`, and the test-profile debug-info setting in `Cargo.toml` only if the paired proof needs it | The landed WP14 check runner | `REPO-RULES` owns the check command. Its guards and callers keep their contracts. The lead coordinates the test-command cutover and verifier cleanup |
| WP16 | The long-test command, its help text and its tests in `tools/xtask/src/`; the new profile and the comments that say which command runs what in `.config/nextest.toml`; the `./dev long-tests` row of the commands table in `CONTRIBUTING.md`. No test file | Nothing | `.config/nextest.toml` is a shared input, so this landing's own check runs the whole workspace. `REPO-RULES` owns the check command outside the test runner. WP5 edits the same table of `CONTRIBUTING.md`, after it |
| WP5 | The commands table and test-command text of `CONTRIBUTING.md`, plus the closing record in the working folder | WP15 and `DEAD-TOOLS` WP2 | `DEAD-TOOLS` WP3 edits other text of `CONTRIBUTING.md`. `REPO-RULES` WP3, the one rename, waits for this work package |

## Not built here

- **A sweep of the tests under ten seconds** for the two rules. WP9 and WP3 take the 18 tests that hold 58% of the
  time. The 265 GPU-named tests under ten seconds hold 141 s between them, summed over 14 cores, and each is
  another module's proof to read and rewrite. The rules bind one when it is next changed. This is an assumption
  in [the plan](PLAN.md).
- **A lint that tells a GPU test should have been a CPU test, or a world a smaller one.** Nothing in a test's
  text says so. The guard rules hold what can be counted, the limit on one test catches the worst case, and the
  judgement is the reviewer's, by [the project settings](../../../ORCHESTRATE.md#what-is-proved-and-how-hard).
  The block in the engineering skill on writing a test is `TEST-SUPPORT` WP6's.
- **A gate on a check's total time, or on the tests' summed time.** Checks run at load averages of 19 to 23, and
  the same tests summed to 820 s in one check and 1,471 s in another. The figures are reported.
- **A hook in a builder's host.** A Codex builder never runs a Claude hook, so what must hold is in the check.
- **A `gpu` lock for the whole run on main**, which mechanism 4 settles.
- **A check that ties each line of the main-tier list to the item that asked for it.** The version's raise is
  seen in a diff, and a wrong long test costs minutes on main.
- **Changing other modules' open items so their binaries fold at once.** Open items name 176 binaries in 17
  modules, and changing a check sends its module's design back for review. Mechanism 10 lets each fold after it
  is built. This is an assumption in [the plan](PLAN.md).
- **Folding the long tests and the tests behind a Cargo feature.** `./dev long-tests` would first have to run
  each test in a process of its own. WP16 does that for the tests behind a feature and stops there: folding
  them renames binaries that open items and the main-tier list name, and no work package here does it.
- **Repairing or removing the cap in `thrum-render`'s library tests** (`crates/thrum-render/src/timestamp_permit.rs`).
  Its numbers are out of date ([mechanism 16](#feature-tests-in-processes-of-their-own)). After WP16 no tier
  runs two of those tests in one process, so nothing rests on it, and item checks that run a few of the
  library's tests through `cargo test` still pass through it. It is the frame runtime's to keep or delete.
- **Exempting the builders' apps from macOS's first-run check.** Listing an app under Developer Tools in System
  Settings, as cargo-nextest's notes describe, removes the first-run cost for any number of binaries. It turns
  off an OS security check for everything that app starts, on the owner's Mac, so it is the owner's to choose,
  and [the plan](PLAN.md) lists it.
- **The grid-parity and CIEDE2000 gates in `run_v1_showcase.py`** (P1-10). They compare the engine with itself
  across targets, open `SHOWCASE` items are built on them, and the CIEDE2000 bound is `APPLE-PRESENTATION`'s
  recorded choice.
- **A shared test world for the water tests** (the rest of E1-07) and **the test kit**, which are
  [TEST-SUPPORT](../TEST-SUPPORT/DESIGN.md)'s.
- **A selection finer than a package**, and **a pass reused when a landing touches nothing the check reads**,
  which is the script's to offer.
- **A standing guard that finds a new run-time reader of another package's files.** A search by text misses a
  reader that reaches the path through a helper, and the whole run on main catches what it misses. The declared
  edge and WP7's one search are what is built.
- **A shorter main tier.** The long tests take about 26 minutes in release and run every four hours, by the
  owner's decision of 21 November. Nobody waits for them.
- **Two changes to the orchestrate script**, which the lead can raise with the skill: the Ready for Review run
  selected by the work package's own change, and a "was main already broken" run that does not reuse a selected
  pass, so that a failure outside a merge's selection is not blamed on the incoming branch.

## What it must not do

Change what the engine draws, simulates or sends. Change a long test's body or assertions when marking it, or a
test behind a feature when listing it. Remove an assertion without naming where it still runs, except the four
references of mechanism 7. Drop a case when one test becomes several. Rename a test binary in WP9 or WP3, or in WP11 one that an open item, the
main-tier list or a script names. Drop a test, or leave one in a file nothing compiles, when files become modules
of one binary. Change a rule, a setting or a skill on the strength of WP10's record. Fail a check on its total time. Add a name to
the exempt list, or raise a guard rule's count or version in WP12 or WP13. Edit a Done item.
Change a registered exact baseline, its comparison or its references. Turn a test that ran on this Mac into one
that skips. Run a failed test again to get a pass, but for the one second run of a test the limit stopped. Select fewer tests than the whole workspace where the selection
cannot be computed. Change how anything but a test is built.

## Proof

A long test is proved by its absence from what `./dev check` runs, by its presence as an ignored test with an
unchanged body, which a reviewer reads, and by the main-branch check `long-tests` passing on main. A test behind
a feature is proved by a scratch fault that fails the merge check, by a scratch gated test file that the list
covers or the docs check refuses, and by the same main-branch check. The runner is proved by its record: the
timings of each arrangement with the load, the test names and results before and after, and five whole runs in a
row that pass. The capture's wait is proved by its test, which fails on the code before the change. The limit
is proved by a scratch test that sleeps past it. The selection is proved by its own
tests, which a reviewer reads, by scratch changes whose selection the reviewer predicts from the rules and the
declared edge, and by the record of what it saves, made before it is switched on. A test made fast is proved by
the reviewer's comparison of what it asserted with what it asserts, case by case, and by its time in the record.
A deleted duplicate is proved by the reviewer's list of where each of its assertions still runs. A retired
reference is proved by the test that replaces it, which the reviewer reads. WP15 uses matched cold and warm
candidate runs, a committed candidate with a failing test and its passing revert, and cache-pressure and
concurrent-use cases. With the cache warm, it also runs the configured command on a passing commit with a failing
assertion edited locally, then restores the file and checks that it passes. A replay of the configured command on
a known-good pre-M15 revision proves the invocation works across activation. WP16 is proved by five runs in
a row of `thrum-render`'s feature line under the new profile and one of each other line, by the runner's own
test of the command it builds for a line, which a reviewer reads, and afterwards by the main-branch check
`long-tests` on main, which no item waits for. WP5 uses the saved wall times in
[the fixed post-rollout window](#the-goal).

One test binary a package is proved by the names of every test before and after, which a reviewer lists again; by
the count of test targets; by a check that no open item lost the binary it names; by a scratch test file that the
guard refuses until it is declared; and by the record of what a freshly linked set costs, before and after. The
record of a builder's session is proved by a reviewer who runs its script on sessions of the reviewer's choice
and reads a stretch of one transcript beside it.

Each guard rule is proved by its fixtures in the rule engine's suite, which a reviewer reads, and by scratch
changes the reviewer makes: the thing the rule refuses, added, fails with the message, and the plan's way through
passes. The closing lines are proved by a reviewer who adds up one saved check's own test lines. The stop at ten
seconds is proved by three scratch tests in one run of the check: one always over the limit, one over it only the
first time it runs, and one that fails an assertion; and by the record of three whole runs beside builders.

The workspace rerun is proved by fast runner tests and a passing full `./dev check-all` on the fixed revision
under contention comparable to the incident. Each of the six incident cases must pass under comparable load,
either on a serial rerun in the full check or during a focused replay. Either run can supply the recovery of a real
test stopped on its first attempt whose serial rerun passes in more than ten and at most thirty seconds.
A focused replay limits the test selection on that same revision and uses the production workspace runner's
initial ten-second stop, exact-case serial retry and final verdict. Only output from that runner counts as
recovery evidence.
