# Gate time: a merge's check runs only fast tests, each once, and the long tests run on main after the merge

20 November 2026. [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-6-the-cleanup-of-20-november) ·
[test tiers](../../../ORCHESTRATE.md#test-tiers), the policy this module applies ·
[what is proved, and how hard](../../../ORCHESTRATE.md#what-is-proved-and-how-hard)

The findings this module answers are cited by id, such as L1-01. They are in the cleanup review of 20 November,
which is kept in the main checkout at `/Users/ryannel/Workspace/thrum/tmp/cleanup-2026-11-20/`: `REPORT.md` ranks
them, `results.json` holds each finding with its verifier's note, and `verified/` holds the notes in full. Where a
verifier corrected a finding, the verifier is right.

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

## Purpose and who it is for

`./dev check` is the test command, so every merge of every builder waits for it. On 20 November it took 33 to 45
minutes, and two builder runs that morning were stopped inside one test. This module's users are the builders and
the lead: a shorter check means more merges an hour and fewer runs lost to a time limit. Nothing a player sees
changes.

## What was measured, and what was not

The review ran no cargo command. Its times come from the script's records of merge checks. The latest that day
(09:51Z, on a Mac that builders load to about six times its cores) spent 2,591 s running 348 test binaries. Eleven
binaries of 45 s or more took 1,940 s of that, and the other 337 took 652 s. The times in the table below are from
that record and from builders' job logs. How long the long tests take in a release build is not measured: the
flythrough's own source says a release build drives it about seven times faster. M5 measures before it changes how
the check builds or runs tests, and records what the long tests cost on main.

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

Three more binaries take 66 to 73 s and stay: the lent products, the cooked assets and the cooked rows. Each holds
3 to 14 short tests of cooked formats, leases and recovery, which is the silent and costly kind the bar keeps
before a merge, and they are slow together because they share one GPU. The cooked asset tests run twice, and M5
removes the second run.

## Mechanisms

**1. A long test is marked, not shortened.** Each test in the table gets `#[ignore]` with a reason that starts
`long:` and names the command that runs it. Its body and assertions do not change. So `./dev check` skips it,
anyone diagnosing a fault runs it with one flag, and nobody has to show that a shorter run asserts the same thing.
The foam test is the one that is split first, because it holds two behaviours: the ten-minute moored run becomes
the long test, and the wind sweep becomes a test `./dev check` runs.

**2. One command runs the long tests, and a main-branch check runs it.** `./dev long-tests` runs every long test
and nothing else. It keeps the list of the test binaries that hold them and runs their ignored tests, so a binary
on the list holds no ignored test that is not a long test: other ignored tests in the workspace need a cooked
store, or are measurements. It builds in release with overflow checks on, on the toolchain `./dev check` uses, and
it requires a GPU, so a test that finds no adapter fails where it would have skipped. Debug assertions stay off,
because the flythrough chooses its length by them and would stop at tick 460. The list names the foam's binary
from M1: a binary with no ignored test passes. M1 adds one line to `ORCHESTRATE.md`, in these words:

```
- **Main-branch check long-tests:** `./dev long-tests`; every 1h; lock gpu; timeout 60m  *(owner, 20 Nov 2026: long tests stay off every merge check)*
```

The script runs it on main's tip after merges, in its own checkout, at most once an hour. It runs a failure again
once, then traces one that holds to the first commit that fails, and that commit's module fixes it while the
others keep merging, as for [exact images](../ARCHITECTURE.md#exact-baselines). So a fault a long test catches is
named an hour or two after its merge. That one line is the only thing in `ORCHESTRATE.md` a builder of this module
writes, and after M1 merges the lead fast-forwards the main checkout, because the script reads the settings there.

**3. A run that proves what another run already proves is deleted, never both kept.** Five pairs:

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

**4. A reference the 17 November bar made a diagnostic leaves the check.** The owner's rule: "A physical or f64
reference is a diagnostic", and no item holds a GPU product to one within a bound. Four tests still do, and each
is the owner's call, carried as an assumption in [the plan](PLAN.md):

| Test | Finding | What goes | What stays or replaces it |
|---|---|---|---|
| The atmosphere's Monte Carlo reference | R4-02 | The f64 path tracer, quadrature and random walks, their 198-case run and the 571 lines that test the reference itself: about 2,200 lines | The night checks and the twilight guard, which [ATMOSPHERE](../ATMOSPHERE/DESIGN.md)'s design names as guards P3, P4 and P11. The twilight guard keeps the three expected values written in it, its 2.5% bound on each channel and its atlas-plane checks. Today it is called only from inside the 198-case run, so it becomes a test of its own |
| The mirror pool's rounding-allowance proof | X1-03 | The assertion that the pool composes within computed half-float bounds, and its air and panel witnesses | A test that the pool prepares and draws. The Chrome equality of a secondary view with the main view stays whole: it is output held to a simple statement of itself |
| The cooked cloud march against an f64 transport sum | A1-04 | The 431-line test that slices the production shader | A short test through the real contribution: zero density keeps the sky and density darkens it. It goes through the cloud-factor shader variant and fails if that variant's text replacement did not match, because this is the only test that reaches it |
| Scene-view shader hashes frozen at a 16 November commit | L5-06 | The test that patches later modules' shader edits back out, and its patch table | The comparison made at one commit, with default features, for each of the four air and shadow variants: the main shader composed with an empty observer equals the one composed with an observer that declares a view slot and leaves it idle, and holds no `secondary_view` text. Today that comparison is behind a feature `./dev check` does not turn on, and covers one variant |

**5. The check's machinery changes only on a measurement.** Two candidates: optimising dependencies in test
builds, so that wgpu, naga and zstd are not built at `-O0` (L1-02); and running test binaries side by side with
nextest, GPU tests held to the GPU's capacity of two (L1-07). Each is timed on one commit
against the check as it is, twice each, alternating, with the load recorded. One is switched on only if both of
its runs are shorter than both runs without it. Where both win they are switched on together, and the run that
gives the check's time afterwards judges the pair: if it is not shorter than both runs of the check as it was,
the one that won by less comes off. Debug assertions and overflow checks stay on, so no test's meaning changes,
and every test that passed before must pass after, compared result by result. A change that does not win is not
made, and the record says so.

## Interfaces

| Interface | For |
|---|---|
| A long test: marked `#[ignore]` with a reason that starts `long:`, in a binary on the list of `./dev long-tests`, which the main-branch check `long-tests` runs in release | Any module with a test that takes over about a minute. Its builder adds the binary to the command's list |

## Whose code this is built in

Every file belongs to another module, as with [track 4](../ARCHITECTURE.md#seam-rulings-for-track-4): `WATERS`'
water scene and shore tests, `SHOWCASE`'s exit scene test, `CULLING`'s field flythroughs, `CLOUDS`' conformance
tests, `ATMOSPHERE`'s reference tests, `SCENE-VIEWS`' pool and scene-view tests, and `CONTENT-LOOP`'s cooked draw
test. Ownership does not move. The Done items that name these tests keep their text, because
[a Done item's check is its record](../../../ORCHESTRATE.md#targets).

## Files each milestone touches

The lead confirms these against the builders at work before it starts a milestone. A milestone touches nothing
outside its row; a builder that finds it must writes the proposal in the log and ends `BLOCKED`.

| Milestone | Files | Waits for | Who else is near |
|---|---|---|---|
| M1 | `examples/water/tests/floating_gpu.rs`; `examples/v1-game/tests/exit_scene.rs`; `examples/v1-game/tests/support/field_gpu.rs` and `field_occlusion_gpu.rs`; `crates/thrum-water-model/tests/shore_blend.rs`; the new command in `tools/xtask/src/main.rs` and its line in `dev`; the one settings line in `ORCHESTRATE.md` | Nothing | No open item runs any of these tests. `TEST-SUPPORT` M2 sweeps test files for the GPU gate: this adds one attribute to a test. `DEAD-TOOLS` M1 and M2 edit other parts of `tools/xtask/src/main.rs` |
| M2 | `crates/thrum-water/tests/query_gpu.rs`, `swash_gpu.rs` and `crates/thrum-water/Cargo.toml`; the four conformance files, `support.rs` and `Cargo.toml` of `crates/thrum-clouds/tests`; `examples/v1-game/tests/support/field_gpu.rs` and `field_occlusion_gpu.rs`, `examples/v1-game/tests/clouds_display_time_gpu.rs`, and the `[[test]]` tables of `examples/v1-game/Cargo.toml`; the list of `./dev long-tests` in `tools/xtask/src/main.rs`; `crates/thrum-render/src/atmosphere/gpu/tests.rs` and `tests/reference*` | `TEST-SUPPORT` M2, which is rewriting these test files | `CLOUDS` builds in `thrum-clouds`. No open item names any of these test binaries. `DEAD-TOOLS` M2 edits the `[[bench]]` tables of the same `Cargo.toml`, and its M3 edits `atmosphere/gpu/tests/aerial_ground.rs`, which `atmosphere/gpu/tests.rs` declares |
| M3 | `examples/water/tests/foam_scene_gpu.rs` | `WATERS` M7, whose open items run this test for its ten minutes | `PUDDLES` M5.4 runs the binary and needs only that it passes |
| M4 | `examples/mirror/tests/pool_gpu/` and the result parsing of `scripts/check_mirror_browser.mjs`; `crates/thrum-asset-gpu/src/draw/tests.rs`; `crates/thrum-render/tests/scene_view_absent_gpu.rs` and `tests/pre_record_source/` | `REVIEW-RENDER` M2, which is extending `pre_record_source` | `CLOUDS` and `RAY-TRACING` edit `thrum-asset-gpu`'s `draw.rs`, not its tests |
| M5 | The `[profile]` tables of the root `Cargo.toml`; the test step of `tools/xtask/src/main.rs`; `.config/nextest.toml`; `.github/workflows/ci.yml`; the commands table and test-command text of `CONTRIBUTING.md` | `DEAD-TOOLS` M2, the last milestone there that edits `tools/xtask/src/main.rs` | `REPO-RULES`, `TEST-SUPPORT` and `HOST` also change what the check runs in `tools/xtask/src/main.rs`. `DEAD-TOOLS` M2 and M3 edit other rows and text of `CONTRIBUTING.md`. A changed test profile rebuilds the test artifacts of every worktree once; builders stall below about 16 GiB of free disk, so the lead clears space first |

## Not built here

- **Selecting a merge's tests by the files it changed** (L1-12). The scenes' tests depend on 18 packages, so the
  selection would have skipped them for 27 of 294 commits, and it changes what the proof of record proves. It is
  an assumption in the plan, with the other choice.
- **One test binary for each water model crate** (W2-01). Nine open items of `WATERS`, `RIVERS` and `PUDDLES`
  run those binaries by name, and the saving is link time nobody measured.
- **The grid-parity and CIEDE2000 gates in `run_v1_showcase.py`** (P1-10). They compare the engine with itself
  across targets, open `SHOWCASE` items are built on them, and the CIEDE2000 bound is `APPLE-PRESENTATION`'s
  recorded choice.
- **A shared test world for the water tests** (the rest of E1-07) and **the test kit**, which are
  [TEST-SUPPORT](../TEST-SUPPORT/DESIGN.md)'s.

## What it must not do

Change what the engine draws, simulates or sends. Change a long test's body or assertions when marking it. Remove
an assertion without naming where it still runs, except the four references of mechanism 4. Edit a Done item.
Change a registered exact baseline, its comparison or its references. Turn a test that ran on this Mac into one
that skips.

## Proof

A long test is proved by its absence from what `./dev check` runs, by its presence as an ignored test with an
unchanged body, which a reviewer reads, and by the main-branch check `long-tests` passing on main. A deleted
duplicate is proved by the reviewer's list of where each of its assertions still runs. A retired reference is
proved by the test that replaces it, which the reviewer reads. The machinery is proved by its record: the
timings, the load, the decision, and the test results before and after.
