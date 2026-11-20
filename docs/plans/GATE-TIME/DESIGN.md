# Gate time: a merge's check runs each proof once, and the long soaks run after the merge

20 November 2026. [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-6-the-cleanup-of-20-november) ·
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

## Purpose and who it is for

`./dev check` is the test command, so every merge of every builder waits for it. On 20 November it took 33 to 38
minutes. One test, `actual_gale_keeps_the_fleet_in_contact_on_metal`, steps 59,400 frames in a debug build and took
13 to 21 minutes of that, and two builder runs that morning were stopped inside it. This module's users are the
builders and the lead: a shorter check means more merges an hour and fewer runs lost to a time limit. Nothing a
player sees changes.

## What was measured, and what was not

The review ran no cargo command. The gale's 13 to 21 minutes comes from builders' job logs, and so do the moored
foam run's 192 to 254 s, the stepped hour's 117 to 213 s and the two field flythroughs' 214 s and 294 s. Every
other saving is an estimate. M5 measures before it changes how the check builds or runs tests.

## Mechanisms

**1. A soak is one test body run at two lengths.** A soak is a test that needs minutes of simulated time to show
a fault that builds up: a body that slowly gains energy, a foam stock that creeps off zero, a mover that drifts
off its loop. Each soak's assertions live in one function that takes the length. `./dev check` calls it with a
short length. A second test in the same binary, marked `#[ignore]`, calls it with the full length. So the two can
never assert different things, and the full run is one flag away for anyone diagnosing a fault.

| Soak | Finding | Full length, after the merge | In `./dev check` |
|---|---|---|---|
| The gale, with the craft at rest, at 5 m/s and at 15 m/s | L1-01 | 600 s at each speed | 60 s at each speed |
| The exit scene's movers on their authored loops | L1-09 | One simulated hour, 216,000 ticks | Past the end of the longest authored loop, so every mover has wrapped at least once |
| Moored foam on the sea and on the lake | L1-04 | Ten minutes | A window long enough for a stock to leave zero if it is going to; the builder picks it and says why |

The 60-second gale starts with the craft at rest, and its start-up lag is 0.1 to 0.25 m/s against a 0.3 m/s
tolerance, so the short run takes its speed baseline after the craft has spun up. The movers' positions are a
closed form of the tick (`Drift::at`), set after the simulation's own step, so nothing accumulates over the hour
and one wrap of each loop shows what the hour shows; the hour still runs after the merge.

**2. The full lengths run as the main-branch check `soak`.** M1 adds one line to `ORCHESTRATE.md`, in these
words:

```
- **Main-branch check soak:** `cargo test --release --locked --offline -p water-scene --test floating_gpu --test foam_scene_gpu -- --ignored && cargo test --release --locked --offline -p v1-game --test exit_scene -- --ignored`; every 2h; lock gpu; timeout 60m
```

The script runs it on main's tip after merges, in its own checkout, in a release build, at most once every two
hours. A failure that holds is traced to the commit that caused it, and that commit's module fixes it while the
others keep merging, as for [exact images](../ARCHITECTURE.md#exact-baselines). So a soak fault can sit on main for
about two hours. That is accepted because a soak guards one module's behaviour, and nothing ships from main. The
line names `foam_scene_gpu` from the start: a binary with no ignored test passes, so M3 adds the foam soak with
no further change to the settings. That one line is the only thing in `ORCHESTRATE.md` a builder of this module
writes. After M1 merges, the lead fast-forwards the main checkout, because the script reads the settings there.

**3. A run that proves what another run already proves is deleted, never both kept.** Four pairs:

- `thrum-water`'s swash test is its own test binary and is also compiled into `query_gpu` by a module
  declaration, so it runs twice (E1-07). It stays in one binary.
- `thrum-clouds` has four GPU test binaries that make two distinct conformance runs: two make the same call, and
  a third is the first part of that run (E2-01). Two tests stay, one on the native device and one on WebGPU's
  default limits, with every assertion the four made.
- The game's frustum-only flythrough of the mixed-mesh field repeats a subset of the occlusion flythrough, which
  already builds the direct and frustum hosts and makes the frustum assertions (L1-03). It goes, and the two
  flythroughs left share one binary.
- `clouds_display_time_gpu.rs` is an older copy of the display-phase test, which `CLOUDS` M2.11 replaced it with
  (G2-02). It goes whole: its one differing assertion encodes the formula M2.11 rejected.

Deleting a duplicate removes no assertion. Each case is shown by naming, for every assertion of the deleted
run, the run that still makes it.

**4. A reference the 17 November bar made a diagnostic leaves the check.** The owner's rule: "A physical or f64
reference is a diagnostic", and no item holds a GPU product to one within a bound. Four tests still do, and each
is the owner's call, carried as an assumption in [the plan](PLAN.md):

| Test | Finding | What goes | What stays or replaces it |
|---|---|---|---|
| The atmosphere's Monte Carlo reference | R4-02 | The f64 path tracer, quadrature and random walks, their 198-case run and the 571 lines that test the reference itself: about 2,200 lines | The night checks and the twilight guard, which [ATMOSPHERE](../ATMOSPHERE/DESIGN.md)'s design names as guards P3, P4 and P11. The twilight guard's expected value is the number the reference gives at the starting commit, written into the test and the log |
| The mirror pool's rounding-allowance proof | X1-03 | The assertion that the pool composes within computed half-float bounds, and its air and panel witnesses | A test that the pool prepares and draws. The Chrome equality of a secondary view with the main view stays whole: it is output held to a simple statement of itself |
| The cooked cloud march against an f64 transport sum | A1-04 | The 431-line test that slices the production shader | A short test through the real contribution: zero density keeps the sky and density darkens it. It goes through the cloud-factor shader variant and fails if that variant's text replacement did not match, because this is the only test that reaches it |
| Scene-view shader hashes frozen at a 16 November commit | L5-06 | The test that patches later modules' shader edits back out, and its patch table | The comparison made at one commit: with no view declared, each of the four air and shadow variants composes the same shader text as with an empty observer. Today that comparison is behind a feature `./dev check` does not turn on, and covers one variant |

**5. The check's machinery changes only on a measurement.** Two candidates: optimising dependencies in test
builds, so that wgpu, naga and zstd are not built at `-O0` (L1-02); and running test binaries side by side with
nextest, GPU tests held to the GPU's capacity of two (L1-07). Each is timed on one commit
against the check as it is, twice each, alternating, with the load recorded. One is switched on only if both of
its runs are shorter than both runs without it. Debug assertions and overflow checks stay on, so no test's
meaning changes, and every test that passed before must pass after, compared result by result. A change that
does not win is not made, and the record says so.

## Interfaces

| Interface | For |
|---|---|
| A soak: one body, a short call in `./dev check` and an ignored full-length test, run by the main-branch check `soak` | Any module with a test that needs minutes of simulated time. A later soak in another test binary is added to the settings line by the lead |

## Whose code this is built in

Every file belongs to another module, as with [track 4](../ARCHITECTURE.md#seam-rulings-for-track-4): `WATERS`'
water scene tests, `SHOWCASE`'s exit scene test, `CULLING`'s field flythroughs, `CLOUDS`' conformance tests,
`ATMOSPHERE`'s reference tests, `SCENE-VIEWS`' pool and scene-view tests, and `CONTENT-LOOP`'s cooked draw test.
Ownership does not move. The Done items that name these tests keep their text, because
[a Done item's check is its record](../../../ORCHESTRATE.md#targets).

## Files each milestone touches

The lead confirms these against the builders at work before it starts a milestone. A milestone touches nothing
outside its row; a builder that finds it must writes the proposal in the log and ends `BLOCKED`.

| Milestone | Files | Waits for | Who else is near |
|---|---|---|---|
| M1 | `examples/water/tests/floating_gpu.rs`; `examples/v1-game/tests/exit_scene.rs`; the one settings line in `ORCHESTRATE.md` | Nothing | No open item runs either test. `TEST-SUPPORT` M2 sweeps test files for the GPU gate |
| M2 | `crates/thrum-water/tests/query_gpu.rs`, `swash_gpu.rs` and `crates/thrum-water/Cargo.toml`; the four conformance files, `support.rs` and `Cargo.toml` of `crates/thrum-clouds/tests`; `examples/v1-game/tests/support/field_gpu.rs` and `field_occlusion_gpu.rs`, `examples/v1-game/tests/clouds_display_time_gpu.rs`, and the `[[test]]` tables of `examples/v1-game/Cargo.toml`; `crates/thrum-render/src/atmosphere/gpu/tests.rs` and `tests/reference*` | `TEST-SUPPORT` M2, which is rewriting these test files | `CLOUDS` builds in `thrum-clouds`. No open item names any of these test binaries. `DEAD-TOOLS` M2 edits the `[[bench]]` tables of the same `Cargo.toml` |
| M3 | `examples/water/tests/foam_scene_gpu.rs` | `WATERS` M7, whose open items run this test for its ten minutes | `PUDDLES` M5.4 runs the binary and needs only that it passes |
| M4 | `examples/mirror/tests/pool_gpu/` and the result parsing of `scripts/check_mirror_browser.mjs`; `crates/thrum-asset-gpu/src/draw/tests.rs`; `crates/thrum-render/tests/scene_view_absent_gpu.rs` and `tests/pre_record_source/` | `REVIEW-RENDER` M2, which is extending `pre_record_source` | `CLOUDS` and `RAY-TRACING` edit `thrum-asset-gpu`'s `draw.rs`, not its tests |
| M5 | The `[profile]` tables of the root `Cargo.toml`; the test step of `tools/xtask/src/main.rs`; `.config/nextest.toml`; `.github/workflows/ci.yml` | `DEAD-TOOLS` M2, the last milestone there that edits `tools/xtask/src/main.rs` | A changed test profile rebuilds the test artifacts of every worktree once; builders stall below about 16 GiB of free disk, so the lead clears space first |

## Not built here

- **One test binary for each water model crate** (W2-01). Nine open items of `WATERS`, `RIVERS` and `PUDDLES`
  run those binaries by name, and the saving is link time nobody measured.
- **Selecting tests by the packages a merge touches** (L1-12), which the 15 November ruling already lists as
  later work.
- **The grid-parity and CIEDE2000 gates in `run_v1_showcase.py`** (P1-10). They compare the engine with itself
  across targets, open `SHOWCASE` items are built on them, and the CIEDE2000 bound is `APPLE-PRESENTATION`'s
  recorded choice.
- **A shared test world for the water tests** (the rest of E1-07) and **the test kit**, which are
  [TEST-SUPPORT](../TEST-SUPPORT/DESIGN.md)'s.

## What it must not do

Change what the engine draws, simulates or sends. Remove an assertion without naming where it still runs, except
the four references of mechanism 4. Edit a Done item. Change a registered exact baseline, its comparison or its
references. Turn a test that ran on this Mac into one that skips.

## Proof

A soak is proved by its short run in `./dev check`, by the ignored test's presence in the binary, and by the
main-branch check `soak` passing on main. A reviewer reads that both lengths call one body. A deleted duplicate
is proved by the reviewer's list of where each of its assertions still runs. A retired reference is proved by
the test that replaces it, which the reviewer reads. The machinery is proved by its record: the timings, the
load, the decision, and the test results before and after.
