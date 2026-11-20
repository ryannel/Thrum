# Gate time: a merge's check runs the fast tests its change can reach, each once, and everything else runs on main after the merge

20 November 2026. [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-6-the-cleanup-of-20-november) ·
[test tiers](../../../ORCHESTRATE.md#test-tiers), the policy this module applies ·
[what is proved, and how hard](../../../ORCHESTRATE.md#what-is-proved-and-how-hard)

The findings this module answers are cited by id, such as L1-01. They are in the cleanup review of 20 November,
which is kept in the main checkout at `/Users/ryannel/Workspace/thrum/tmp/cleanup-2026-11-20/`: `REPORT.md` ranks
them, `results.json` holds each finding with its verifier's note, and `verified/` holds the notes in full. Where a
verifier corrected a finding, the verifier is right. `HARDENING.md` beside them is the second read of this plan
that the third revision applies.

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

## Purpose and who it is for

`./dev check` is the test command, so every merge of every builder waits for it. This module's users are the
builders and the lead: a shorter check means more merges an hour and fewer runs lost to a time limit. Nothing a
player sees changes.

## What was measured, and what was not

The cleanup review ran no cargo command. The times here come from the script's saved logs of merge checks
(`.git/orchestrate/checks`), read while planning.

- The ten latest full passes of `./dev check` on 20 November spent 42 to 56 minutes in tests alone (mean 48), over
  about 350 test binaries and doc-test targets, on a Mac that builders load to about six times its cores. Two
  holds of the merge lock that week ran to the script's 60-minute limit for the test command, and the lock was
  held 19 to 21 hours a day on 16 to 19 November.
- The six long tests of the table below are about 31 of those 48 minutes. With them gone the whole workspace's
  tests take about 18 minutes, and the lints about 5.
- [A replay](../../../.orchestrate/evidence/GATE-TIME/planning/selection-replay.md) of the selection of mechanism 4
  over the 177 landings since 13 November puts the tests a landing selects at about 9 minutes of those 18 on
  average: a quarter of landings select no Rust test, half select some packages, and a quarter fall back to the
  whole workspace. It is the planner's estimate, and M7 measures the selection as built before turning it on.
- The same binary's time varied about 1.6 times between runs under load.

Not measured: how long the long tests take in a release build, and whether each passes there. Only the occlusion
flythrough has run in release before, in `CULLING` M1.2, and in release the flythroughs fly to tick 1,930 where
the check stopped at tick 460. Whether the tests behind a Cargo feature pass today is not measured either. M1 and
M6 each start with that measurement, and M5 measures before it changes how the check builds or runs tests.

## Every test over about a minute

[The test tiers](../../../ORCHESTRATE.md#test-tiers) say a test that takes over about a minute in `./dev check`
is a long test, and leaves the merge tier whole. This is every such test on 20 November.

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

Three more binaries take 66 to 74 s and stay: the lent products, the cooked assets and the cooked rows. Each holds
3 to 14 short tests of cooked formats, leases and recovery, which is the silent and costly kind the bar keeps
before a merge, and they are slow together because they share one GPU. The cooked asset tests run twice, and M5
removes the second run.

## Mechanisms

**1. A long test is marked, not shortened.** Each test in the table gets `#[ignore]` with a reason that starts
`long:` and names the command that runs it. Its body and assertions do not change. So `./dev check` skips it,
anyone diagnosing a fault runs it with one flag, and nobody has to show that a shorter run asserts the same thing.
The foam test is the one that is split first, because it holds two behaviours: the ten-minute moored run becomes
the long test, and the wind sweep becomes a test `./dev check` runs. The moored run starts itself again as a
child process by name, and an ignored test's child skips unless it is told to run ignored tests, so M3 also
changes the child's arguments in the water scene's `with_cooked_island`.

**2. One command runs the main tier's tests from one list, and a main-branch check runs it.** `./dev long-tests`
runs the tests the main-tier list names and nothing else. The list is one tracked file,
`.config/long-tests.txt`. For each entry it gives the package, the test binary, the Cargo features if any, and
the build. The command runs the ignored tests of each long-test binary, so a binary on the list holds no ignored
test that is not a long test: other ignored tests in the workspace need a cooked store, or are measurements. It
uses the toolchain `./dev check` uses and requires a GPU, so a test that finds no adapter fails where it would
have skipped. The list names the foam's binary from M1: a binary with no ignored test passes.

The build is release with overflow checks on. Debug assertions stay off, because the flythrough chooses its
length by them and would stop at tick 460. M1 starts by running the six once in that build, because five of them
have never run in it. A test that fails there is still marked and still leaves the merge check: the list gives it
the debug build, the log gives the release failure and names the module that owns the test, and nobody changes
the test here. Without that rule one release failure would leave M1 open, with every other landing paying the old
price behind it.

M1 adds one line to `ORCHESTRATE.md`, in these words:

```
- **Main-branch check long-tests:** `./dev long-tests`; every 1h; lock gpu; timeout 60m  *(owner, 20 Nov 2026: long tests stay off every merge check)*
```

The builder keeps every word but the timeout, which is 60m or twice the time its own first run took, whichever
is longer; M6 sets it again the same way when the tests behind a feature join. The script runs the check on
main's tip after merges, in its own checkout, at most once an hour. It runs a failure again once, then traces one
that holds to the first commit that fails, and that commit's module fixes it while the others keep merging, as
for [exact images](../ARCHITECTURE.md#exact-baselines). Each step of that trace is a whole run under a `gpu`
lock, so a fault a long test catches is named some hours after its merge. That line and the one of mechanism 4
are the only things in `ORCHESTRATE.md` a builder of this module writes, and after each merges the lead
fast-forwards the main checkout, because the script reads the settings there.

**3. A test behind a Cargo feature is compiled by every merge and run on main.** Three features hide tests from
`cargo test --workspace`: `creation-counters` in `thrum-render` (27 files, among them twelve ray-tracing test
binaries that run zero tests in the check), and `draw-probe` and `thrum-asset-gpu/fault-probe` in the game (the
ray-tracing proof, the mesh-stage tests and the cooked fault test). Until now they ran only in the item checks
that name them, so a sweep that broke one merged green and stopped another module's builder days later.

- *At every merge*, `./dev check` builds and lints every test target of `thrum-render` with `creation-counters`
  on, and of the game with `draw-probe` and `thrum-asset-gpu/fault-probe` on, under the lints the rest of the
  workspace gets. The build of an unchanged crate is reused, so a merge that touches none of them pays seconds.
- *On main*, `./dev long-tests` runs them with their features on and a GPU required. The list names each one's
  package, binary and features. They run in the debug build, as the item checks that run them do. For these
  binaries the command runs the tests that are not ignored: an ignored test there is a measurement that a cost
  script runs.
- A test that fails at the commit M6 starts from is listed as not run, with its failure and its owning module,
  and is not changed here.
- *Membership is held by a check.* `./dev check-docs` fails when a test target requires a Cargo feature that the
  list does not name, so a new feature cannot take a test target out of every tier unnoticed. A `cfg` on a
  feature inside a test file is found by search when M6 is reviewed.

**4. A merge runs the tests its change can reach, and main runs them all.** `./dev check` selects test binaries
by package. `./dev check-all` runs everything, whatever changed, and a main-branch check runs it.

- *The change* is every path that differs between the checkout and its merge base with the main branch
  (`origin/main`, as the settings name it), with uncommitted and untracked paths included. The script runs
  `./dev check` with no argument in a builder's worktree, in a merge after its rebase, and on a batch of several
  branches rebased together, and this definition gives the right change in each.
- *The selection* is each workspace package that holds a changed path, and every workspace package that depends
  on one, directly or through others, as a normal, build or dev dependency. The graph is Cargo's own, read from
  `cargo metadata`. The check runs the unit and integration test binaries of the selected packages.
- *What is not selected.* The build and the lints cover the whole workspace on every merge, with the compile of
  mechanism 3, the repository guards, the script suites and the doc tests. They are what would stop every
  builder, an unchanged crate's build is reused, the Python steps take seconds, and the doc tests 0.1 s in all.
- *The binaries are the whole workspace's.* The check builds the workspace's test binaries once, with the
  workspace's features, and the selection decides only which of them it runs: the check runs the selected
  binaries itself, one at a time, each from its package's folder with what cargo gives a test. `cargo test -p`
  for a subset is not used, because Cargo resolves features for the packages named, so a subset can build a
  different binary from the one main runs and rebuild shared dependencies.
- *A shared input selects the whole workspace*: the root `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`,
  `clippy.toml`, `.cargo/`, `dev`, `dev.ps1`, `tools/xtask`, and the test runner's own configuration
  (`.config/long-tests.txt`, and `.config/nextest.toml` if M5 adds it). The lockfile could be followed through its
  own graph, but the replay puts that at 36 s a landing, which does not pay for a reader that has to be right.
- *Paths that reach no Rust test select none*: Markdown anywhere, `docs/` outside `docs/evidence`, the plans,
  `.agents/`, `.github/`, `scripts/`, and the data under `.config/` that only the guards read. No Rust test reads
  a script or that data: a search finds `scripts/` in test code only inside `#[ignore]` reasons and one bench. The
  guards and suites that do read them run on every merge.
- *Anything else selects the whole workspace*: a path with no rule above, such as the root `tests/fixtures/`,
  `docs/evidence` or a new top-level folder; a checkout with no merge base, as in a shallow clone or CI; a
  checkout that does not differ from the main branch, which is what the main tier runs; and a dependency graph the
  check cannot read. The check prints what it selected and why.
- *On main*, M7 adds one line to `ORCHESTRATE.md`, in these words:

```
- **Main-branch check workspace-tests:** `./dev check-all`; every 1h; timeout 60m  *(owner, 20 Nov 2026: a merge runs the tests its change touches, and the full workspace runs on main)*
```

  It takes no lock, as `./dev check` never has: it is the same load one merge's check put on the GPU before.
  The builder keeps every word but the timeout, which is 60m or twice the time the check's first run took,
  whichever is longer, because that first run builds the workspace from nothing in a checkout of its own.
- *The risk accepted*, which [the test tiers](../../../ORCHESTRATE.md#test-tiers) state: the whole workspace is
  still built and linted at every merge, so no compile-time edge can be missed. What can be missed is a test
  whose result depends on something Cargo's graph does not show, such as a file another folder owns that it reads
  at run time. That edge reaches main and sits there until the next whole run, up to an hour and one run, and
  some hours more to trace. Until then the script counts main as passing, because the merge that broke it passed
  its own check, so a later merge that selects the failing test is refused for a fault it did not cause.
- *Finer than a package is not built.* Nothing says which test binary of a package a source file reaches without
  a coverage map, and a wrong map misses edges silently. The game's package is a third of what is left and
  depends on everything; it leaves the engine's check when [GAME-SPLIT](../GAME-SPLIT/DESIGN.md) moves the game.

**5. A slow test cannot enter the merge tier unnoticed.** The gale reached 13 to 21 minutes in the check with
nothing objecting. Three checks now notice:

- `./dev check` fails when one test binary's run takes over three minutes by the check's own clock. The message
  names the binary and says what to do: make the test fast, or mark it `#[ignore = "long: …"]` and list it. Three
  minutes is about 2.4 times the slowest binary the tier keeps (74 s), past the 1.6 times load was seen to add,
  so a correct landing does not fail on someone else's test. The foam scene's binary is exempt by name until M3.
  A hard stop is the owner's call and an assumption in [the plan](PLAN.md).
- `./dev check-docs` fails when a test marked `long:` sits in a binary the list does not name, so a test cannot
  be marked and then run in no tier.
- The feature check of mechanism 3.

libtest's own "has been running for over 60 seconds" line is not used: half of what it flags are tests waiting
behind a device mutex, and the names differ from run to run.

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
- `clouds_display_time_gpu.rs` is an older copy of the display-phase test, which `CLOUDS` M2.11 replaced it with
  (G2-02). It goes whole: its one differing assertion encodes the formula M2.11 rejected.
- `./dev check` runs the 14 cooked-asset tests in the workspace pass and again in its draw-probe pass, where four
  of them assert on the probe (L1-06). The probe pass runs only the tests that read the probe. The workspace pass
  still runs all 14, because it is the only run against the production build. M5 builds this, in the check's
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

**8. The check's machinery changes only on a measurement.** Two candidates: optimising dependencies in test
builds, so that wgpu, naga and zstd are not built at `-O0` (L1-02); and running test binaries side by side with
nextest, GPU tests held to the GPU's capacity of two (L1-07). Each is timed on one commit against the
whole-workspace run as it is, twice each, alternating, with the load recorded. One is switched on only if both of
its runs are shorter than both runs without it. Where both win they are switched on together, and the run that
gives the time afterwards judges the pair: if it is not shorter than both runs as it was, the one that won by
less comes off. Debug assertions and overflow checks stay on, so no test's meaning changes, and every test that
passed before must pass after, compared result by result. A change that does not win is not made, and the record
says so. Where nextest is switched on it takes over the jobs of mechanisms 4 and 5 without loosening them: the
merge check still runs only the selected packages' tests, on binaries built for the whole workspace, and a test
or binary over the limit still fails.

## Interfaces

| Interface | For |
|---|---|
| A long test: marked `#[ignore]` with a reason that starts `long:`, in a binary the main-tier list names; the main-branch check `long-tests` runs it | Any module with a test that takes over about a minute. Its builder adds the binary to the list |
| The main-tier list, `.config/long-tests.txt`: each long-test binary and each test behind a Cargo feature, with its package, features and build | Any module that adds a long test or a Cargo feature that gates a test |
| The merge selection: `./dev check` runs the tests of the packages a change can reach, by Cargo's graph and the path rules of mechanism 4; `./dev check-all` runs the whole workspace | Every builder, and any module that adds a folder outside a package: until it has a path rule, a change there selects the whole workspace |

## Whose code this is built in

Every file belongs to another module, as with [track 4](../ARCHITECTURE.md#seam-rulings-for-track-4): `WATERS`'
water scene and shore tests, `SHOWCASE`'s exit scene test, `CULLING`'s field flythroughs, `CLOUDS`' conformance
tests, `ATMOSPHERE`'s reference tests, `SCENE-VIEWS`' pool and scene-view tests, `CONTENT-LOOP`'s cooked draw
test, and `REPO-RULES`' check command and repository guards. Ownership does not move. The Done items that name
these tests keep their text, because [a Done item's check is its record](../../../ORCHESTRATE.md#targets).

## Files each milestone touches

The lead confirms these against the builders at work before it starts a milestone. A milestone touches nothing
outside its row; a builder that finds it must writes the proposal in the log and ends `BLOCKED`. Three files are
everyone's: `.config/shape-baseline.json`, `docs/design/maturity.json` and `Cargo.lock`. Change them when your
landing moves them, whatever your row says. The rows are in build order.

| Milestone | Files | Waits for | Who else is near |
|---|---|---|---|
| M1 | `examples/water/tests/floating_gpu.rs`; `examples/v1-game/tests/exit_scene.rs`; `examples/v1-game/tests/support/field_gpu.rs` and `field_occlusion_gpu.rs`; `crates/thrum-water-model/tests/shore_blend.rs`; the new command and the help text in `tools/xtask/src/main.rs` and its line in `dev`; the new `.config/long-tests.txt`; the listed-binary check in `scripts/repo_check.py` with its suite; the one settings line in `ORCHESTRATE.md` | Nothing. It merges first in wave 1 | No open item runs any of these tests. `DEAD-TOOLS` M1 and M2 edit other arms of `tools/xtask/src/main.rs` and the same help text. `scripts/repo_check.py` is imported by a pinned comparison file: it keeps its name and what that file uses. The check keeps a checkout and a release build of its own, so the lead checks free disk first: builders stall below about 16 GiB |
| M6 | The lint step and the long-test command in `tools/xtask/src/main.rs`; `.config/long-tests.txt`; the feature check in `scripts/repo_check.py` with its suite; the timeout of the settings line | Nothing | `DEAD-CODE` M2 and M6 and `TEST-SUPPORT` M3 wait for it. `DEAD-TOOLS` M1 edits the script-suite step of the same `main.rs` |
| M7 | The check command in `tools/xtask/src/` (the selection, the run of test binaries, `check-all`, the limit on one binary) and its tests; one settings line in `ORCHESTRATE.md`; the exemption of the foam's binary, kept with the list | Nothing | `DEAD-TOOLS` M1 and M2 edit the script-suite step, the `baseline-inventory` command and the help text of the same `main.rs`. The main-branch check keeps a fourth checkout with a debug build of the whole workspace, 8 to 16 GiB in the checkouts that hold one today, so the lead checks free disk first |
| M2 | `crates/thrum-water/tests/query_gpu.rs`, `swash_gpu.rs` and `crates/thrum-water/Cargo.toml`; the four conformance files, `support.rs` and `Cargo.toml` of `crates/thrum-clouds/tests`; `examples/v1-game/tests/support/field_gpu.rs` and `field_occlusion_gpu.rs`, `examples/v1-game/tests/clouds_display_time_gpu.rs`, and the `[[test]]` tables of `examples/v1-game/Cargo.toml`; `.config/long-tests.txt`; `crates/thrum-render/src/atmosphere/gpu/tests.rs` and `tests/reference*` | `TEST-SUPPORT` M2, which is rewriting these test files | `CLOUDS` builds in `thrum-clouds`. No open item names any of these test binaries. `DEAD-TOOLS` M2 edits the `[[bench]]` tables of the same `Cargo.toml`, and its M3 edits `atmosphere/gpu/tests/aerial_ground.rs`, which `atmosphere/gpu/tests.rs` declares |
| M3 | `examples/water/tests/foam_scene_gpu.rs`; the child's arguments in `examples/water/tests/support/island.rs`; the foam binary's exemption, kept with `.config/long-tests.txt` | `WATERS` M7, whose open items run this test for its ten minutes | `PUDDLES` M5.4 runs the binary and needs only that it passes. `stage_gpu`, `inland_floating_gpu` and `bodies_gpu` also start a child through `island.rs`, each with a test that is not ignored, so running ignored tests in the child changes nothing for them |
| M4 | `examples/mirror/tests/pool_gpu/` and the result parsing of `scripts/check_mirror_browser.mjs`; `crates/thrum-asset-gpu/src/draw/tests.rs`; `crates/thrum-render/tests/scene_view_absent_gpu.rs` and `tests/pre_record_source/` | `REVIEW-RENDER` M2, which is extending `pre_record_source` | `CLOUDS` and `RAY-TRACING` edit `thrum-asset-gpu`'s `draw.rs`, not its tests |
| M5 | The `[profile]` tables of the root `Cargo.toml`; the test step of the check command in `tools/xtask/src/`; `.config/nextest.toml`; `.github/workflows/ci.yml`; the commands table and test-command text of `CONTRIBUTING.md` | `DEAD-TOOLS` M2, the last milestone there that edits `tools/xtask/src/main.rs` | `REPO-RULES`, `TEST-SUPPORT` and `HOST` also change what the check runs in `tools/xtask/src/main.rs`. `DEAD-TOOLS` M2 and M3 edit other rows and text of `CONTRIBUTING.md`. A changed test profile rebuilds the test artifacts of every worktree once, so the lead clears disk first |

## Not built here

- **One test binary for each water model crate** (W2-01). Nine open items of `WATERS`, `RIVERS` and `PUDDLES`
  run those binaries by name, and the saving is link time nobody measured.
- **The grid-parity and CIEDE2000 gates in `run_v1_showcase.py`** (P1-10). They compare the engine with itself
  across targets, open `SHOWCASE` items are built on them, and the CIEDE2000 bound is `APPLE-PRESENTATION`'s
  recorded choice.
- **A shared test world for the water tests** (the rest of E1-07) and **the test kit**, which are
  [TEST-SUPPORT](../TEST-SUPPORT/DESIGN.md)'s.
- **A selection finer than a package**, and **a pass reused when a landing touches nothing the check reads**,
  which is the script's to offer.

## What it must not do

Change what the engine draws, simulates or sends. Change a long test's body or assertions when marking it, or a
test behind a feature when listing it. Remove an assertion without naming where it still runs, except the four
references of mechanism 7. Edit a Done item. Change a registered exact baseline, its comparison or its
references. Turn a test that ran on this Mac into one that skips. Select fewer tests than the whole workspace
where the selection cannot be computed.

## Proof

A long test is proved by its absence from what `./dev check` runs, by its presence as an ignored test with an
unchanged body, which a reviewer reads, and by the main-branch check `long-tests` passing on main. A test behind
a feature is proved by a scratch fault that fails the merge check, and by the same main-branch check. The
selection is proved by its own tests, which a reviewer reads, by scratch changes whose selection the reviewer
predicts from the rules, and by the record of what it saves, made before it is switched on. A deleted duplicate
is proved by the reviewer's list of where each of its assertions still runs. A retired reference is proved by the
test that replaces it, which the reviewer reads. The machinery is proved by its record: the timings, the load,
the decision, and the test results before and after.
