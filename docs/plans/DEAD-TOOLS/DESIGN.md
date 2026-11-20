# Dead tools: scripts, benches and records that nothing runs or reads leave the tree

20 November 2026. [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-6-the-cleanup-of-20-november) ·
[exact baselines](../ARCHITECTURE.md#exact-baselines) · [REPO-RULES](../REPO-RULES/DESIGN.md), whose rule engine and
evidence store this module uses

The findings this module answers are cited by id, such as P1-01. They are in the cleanup review of 20 November,
which is kept in the main checkout at `/Users/ryannel/Workspace/thrum/tmp/cleanup-2026-11-20/`: `REPORT.md` ranks
them, `results.json` holds each finding with its verifier's note, and `findings/` and `verified/` hold the
write-ups, with every file list in full. Where a verifier corrected a finding, the verifier is right.

## The request

The owner, 20 November 2026: "We want to do a code cleanup and consolidation for Thrum." The lead's task for the
plan: make Thrum cheaper to change, with "no tools, records or code that nothing uses", and without changing
"what the engine draws, simulates or sends".

## Purpose and who it is for

About 13,000 lines of scripts, benches and tool code have no caller, and 373 of the 406 files tracked under
`docs/evidence` are read by nothing. Each one costs something on every change: Clippy compiles the benches and a
dead binary on every check, a rename has to carry files nobody runs, and an agent landing cold reads a tool as
if it were in use and copies its shape. This module's users are builders and planners. Nothing a player sees
changes, and no tool that something runs changes what it does.

## Mechanisms

**1. One test for "nothing runs it".** A script, bench, command or test tool is dead when none of these names
it: a `./dev` command, a CI workflow, an open item's check, a script or test that is itself kept, or a tracked
document that tells a reader to run it. A name that only a Done item uses does not keep a tool alive, because
[a Done item's check is its record](../../../ORCHESTRATE.md#targets). The test is run with `git grep` for the
tool's name over the whole tree, on the commit the builder starts from, since main moves.

**2. A dead tool is deleted, and Git is the archive.** It is not moved to a folder of old tools, which
[REPO-RULES](../REPO-RULES/DESIGN.md) M4.1 would otherwise do. A design or research note that describes a
deleted tool keeps its text and gains one line naming the last commit that held the code; its links to the
deleted files go, because the link check refuses a link to a missing file. A row of
[the maturity ledger](../../design/maturity.json) that describes a deleted tool says it is retired and names that
commit, and claims nothing as present.

**3. Script deletions land once.** `scripts/` is a covered path of the exact baselines, so each landing there
asks the main-branch check `exact-images` for a run. M1 deletes every dead script in one landing. The seven
comparison files the registry pins by content are never touched: `exact_baselines.py`,
`exact_baselines_native.py`, `apple_tile_m0.py`, `apple_tile_m1.py`, `apple_tile_m2.py`, `compare_v1_pixels.py`
and `apple_tile_m1_compare_browser.py`. The six scripts they import keep their names and everything the pinned
files use from them: `run_v1_showcase.py`, `t2_writer_probe.py`, `capture_spool.py`, `capture_v1_runtime.py`,
`capture_result.py` and `repo_check.py`.

What M1 deletes, each the owner's call where the plan says so:

| What | Finding | Size |
|---|---|---|
| The headless release-trust toolchain: 15 scripts that prepare and verify a sealed release build, their suites, and the binary `tools/xtask/src/bin/rustc_receipt.rs`. Written 1 to 3 November for an example since deleted, run by nothing, compiled by every check | P1-01, P2-01, L5-01 | 8,500 lines |
| Thirteen one-shot milestone proof scripts and `creator_profile_probe.rs`, each written for one milestone of `APPLE-TILE`, `APPLE-PRESENTATION`, `COOKED-DRAW`, `CREATOR` or `TRANSPORT` | P1-03 | 1,650 lines |
| `culling_campaign.py` and its suite. `CULLING` is Done and its plan says no loaded campaign repeats | G2-05 | 350 lines with M4's half |

**4. The check finds script suites itself.** Today `./dev check` runs 8 of 29 `scripts/test_*.py` suites from a
hand-kept list, and the suites of the shape rules the check enforces run nowhere. After the dead suites are
gone, the check discovers every `scripts/test_*.py`, and every `scripts/test_*.mjs` where node is installed. A
suite the check cannot run is excluded by name with its reason. The added time is recorded.

**5. A bench is kept when something runs it.** Nine benches are named by no script, command or check, and
Clippy compiles them on every check: seven in `thrum-render` and `field_culling` and `material_reload` in the
game (L5-05). They go, and `thrum-render` drops the bench harness dependency only they used. `./dev
baseline-inventory` ran the fixed budgets of gate G0, which closed on 5 November; it goes with the one bench
only it drove and the inventory printer and batch modes in the frame bench (L5-15, G2-03). The benches that the
cost scripts and the content tripwire run stay as they are.

**6. Evidence has three homes, and `docs/evidence` is none of them.** The merge refuses a new or changed file
there, yet scripts, benches and tests still write and read there.

- A **record of a run** is written to the working folder, `.orchestrate/evidence/<MODULE>/`, which every
  worktree shares and Git does not hold. Every cost script and tripwire bench writes there by default.
- **Test data**, which a check reads, is tracked in `tests/fixtures/<module>/` or beside the crate's tests, and
  moves there with the bytes it had. That covers the `RENDER-2D` cost receipts, the atmosphere's aerial volumes,
  `WATERS`' first-frame and island cost records, which later cost checks compare with, and the reference files
  the river, cloud-model and 2D conformance tests read.
- The **list of what left the tree** is one manifest under `.config/`, where today there are two with one schema,
  one of them inside the frozen folder beside a generated ignore file of 1,100 lines. `evidence_store.py`
  keeps its commands and reports the same entries.

The 373 unread records are deleted in one commit that only deletes. Each has a copy in the working folder; five
`RENDER-2D` M1 files differ there because a later run rewrote them, so their committed bytes are saved to the
working folder under a second name first. A rule added through [REPO-RULES](../REPO-RULES/DESIGN.md)' rule
engine, `evidence-path`, then holds the shape: no source file under `crates`, `examples`, `scripts` or `tools`
names a path under `docs/evidence`, except the evidence store's own code that restores history. Data files are
not source, so the exact-image fixtures that record old paths are untouched.

**7. Test tools nothing arms go with their plumbing.** The culling route test carries a TCP timing coordinator
that only an environment variable nothing sets can arm, and a campaign mode that only `culling_campaign.py`
drove (G2-05). Both go. The exact, timing and exit-arm modes stay, because `./dev check` and the cost scripts
run them.

## Interfaces

| Interface | For |
|---|---|
| Where evidence goes: run records in the working folder, test data in fixtures, and the rule `evidence-path` | Every builder and planner |
| The evidence manifest under `.config/`, with `evidence_store.py check` and `restore` as they were | Every builder and reviewer. It changes [REPO-RULES](../REPO-RULES/DESIGN.md) M2's interface only in where the manifest is kept |

## Files each milestone touches

The lead confirms these against the builders at work before it starts a milestone. A milestone touches nothing
outside its row; a builder that finds it must writes the proposal in the log and ends `BLOCKED`.
`.config/shape-baseline.json` and `docs/design/maturity.json` are shared by every module: a landing that
lowers a count or retires a row rebases and makes the same edit again.

| Milestone | Files | Waits for | Who else is near |
|---|---|---|---|
| M1 | The deleted files under `scripts/`; `tools/xtask/src/bin/rustc_receipt.rs`; the script-suite step of `tools/xtask/src/main.rs`; the three F4 and T2 headless-build designs under `docs/design/` and the research notes that link the toolchain; the toolchain's rows of `docs/design/maturity.json` | Nothing | No builder is in these scripts. `CLOUDS` works in `scripts/reference/cloud/`, which is not touched |
| M2 | `crates/thrum-render/benches/` and the `[[bench]]` tables and bench dependency of `crates/thrum-render/Cargo.toml`; `examples/v1-game/benches/field_culling.rs`, `material_reload.rs`, `g2r.rs` and `frame.rs`, and the `[[bench]]` tables of `examples/v1-game/Cargo.toml`; the `baseline-inventory` command in `tools/xtask/src/main.rs`; its row in `CONTRIBUTING.md` | Nothing | `HOST` edited `field_culling.rs` seven times in three days and its M3.2 asks about benches, so deleting first saves it work: tell its builder. `GATE-TIME` M2 edits the `[[test]]` tables of the same game manifest |
| M3 | The deletions under `docs/evidence/`; one link in `docs/design/decisions/G7-SHOWCASE-ROUTE.md`; `scripts/render2d.py`, `test_render2d.py` and the receipts they read; `crates/thrum-render/src/atmosphere/gpu/tests/aerial_ground.rs` and its volumes; `scripts/evidence_store.py`, its suite and the two manifests; `.gitignore`; the evidence text of `AGENTS.md`, `CONTRIBUTING.md`, `docs/ASSET-LIFECYCLE.md`, `docs/REPOSITORY.md`, the engineering patterns and `scripts/shape_matchers.py` | Nothing | `RENDER-2D` edits `render2d.py`, and its open M2.10, M2.11, M5.4 and M5.5 run `render2d.py costs`, which must read the same receipts after the move |
| M4 | The cost and proof scripts that name `docs/evidence` (`cloud_cost.py`, `water_cost.py`, `river_cost.py`, `rain_cost.py`, `scene_view_cost.py`, `sim2d_conformance.py`, `session_memory.py`, `session_prediction.py`, `ray_tracing_proof.py`, `transport_link.py`, `environment_products.py`, `cache_reconstruction.py`, `check_g5_tripwires.sh`, one line of `run_v1_showcase.py`); the tests and benches that do (`thrum-cloud-model`'s optics test, three `thrum-river-model` tests, the cooker's edit-to-visible bench, the game's `cooked_edit` bench, one line of the game's `host.rs`); the records they read, moved to fixtures; the rule in `scripts/shape_rules.py` with its fixtures; `examples/v1-game/tests/valley_route_support/` | `TEST-SUPPORT` M2, which is rewriting `valley_route_support` | The busiest set here. Open checks of `CLOUDS`, `WATERS`, `RENDER-2D`, `SESSION` and `RAY-TRACING` run these scripts, and their builders edit them: each command must run as it did and read the same prior record |

## Not built here

- **The other dead scripts the review confirmed at low payoff** (L5-02, P2-04, P1-02, P2-05, L2-07: about 4,700
  lines). They are outside the review's ranked sections. A second landing can take them, or REPO-RULES M4.1 moves
  them.
- **The frozen pre-sprite graph snapshot** (G2-09). It is a guard, not a dead tool: it is the only graph check
  driven by the game's own exit and valley setups. Retiring it is the owner's or `RENDER-2D`'s call.
- **One harness for the eleven cost scripts and for the browser scripts** (P1-05, P1-04, P2-02, P2-08), which
  need a design of their own.
- **Retiring `evidence_store.py`'s `put` and `archive`** (L2-10), which the review did not verify.
- **A tracked home for provenance records** (L8-08), which is the owner's call and an assumption in the plan.

## What it must not do

Change what the engine draws, simulates or sends. Edit or delete a file the exact-baseline registry pins, or delete, rename or
change the behaviour of a script those files import. Change what a kept script's command does, prints or exits with, beyond where it
writes a record by default. Edit a Done item, a review or a module's design. Delete a record that has no copy.

## Proof

A deletion is proved by the name's absence from everything that is left, by `./dev check` and `./dev check-docs`
passing, and by a reviewer applying mechanism 1 to what was deleted and to what was kept. A move of test data is
proved by Git blobs that match and by the tests that read it. The manifest is proved by the store's own check
and suite, with the counts it reported before. The rule is proved by its fixtures and by its zero.
