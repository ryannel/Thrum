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
"what the engine draws, simulates or sends". The owner's decisions of the same day, as the lead gave them: all
nine benches that nothing runs are deleted, and provenance records are tracked in Git, in place before `CLOUDS` M6
copies its first library.

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

**3. Script deletions land once, and the pinned comparison files are never touched.** `scripts/` is a covered
path of the exact baselines, so each landing there asks the main-branch check `exact-images` for a run. M1 deletes
every dead script in one landing. The registry pins seven comparison files by content, and no milestone edits
one: `exact_baselines.py`, `exact_baselines_native.py`, `apple_tile_m0.py`, `apple_tile_m1.py`,
`apple_tile_m2.py`, `compare_v1_pixels.py` and `apple_tile_m1_compare_browser.py`. A script one of them imports,
directly or through another script, is not pinned and may be edited, but it keeps its name and everything the
pinned files use from it. Imported directly: `run_v1_showcase.py`, `t2_writer_probe.py`, `capture_spool.py`,
`capture_v1_runtime.py`, `capture_result.py` and `repo_check.py`. Imported through `repo_check.py`:
`evidence_store.py` and `shape_rules.py`, and through that `shape_matchers.py`. M6, M3 and M4 edit four of these,
in the lines their rows name.

What M1 deletes, each the owner's call where the plan says so:

| What | Finding | Size |
|---|---|---|
| The headless release-trust toolchain, written 1 to 3 November to prepare and verify a sealed release build for an example since deleted, run by nothing, compiled by every check. Fifteen scripts: `prepare_headless_cargo_home`, `prepare_macos_tool_root_clone`, `inventory_headless_tool_roots`, `run_bounded_native_build`, `qualify_held_macos_native_ab`, `release_cargo_selection`, the three `hold_macos_*` and the six `verify_*`. Their thirteen `test_*` suites, and the binary `tools/xtask/src/bin/rustc_receipt.rs` | P1-01, P2-01, L5-01 | 8,500 lines |
| Thirteen one-shot milestone proof scripts and `creator_profile_probe.rs`, each written for one milestone of `APPLE-TILE`, `APPLE-PRESENTATION`, `COOKED-DRAW`, `CREATOR` or `TRANSPORT`. M1.2's check lists them | P1-03 | 1,650 lines |
| `culling_campaign.py` and its suite. `CULLING` is Done and its plan says no loaded campaign repeats | G2-05 | 350 lines with M4's half |

**4. The check finds script suites itself.** Today `./dev check` runs 8 of 29 `scripts/test_*.py` suites from a
hand-kept list, and the suites of the shape rules the check enforces run nowhere. After the dead suites are
gone, the check discovers every `scripts/test_*.py`, and every `scripts/test_*.mjs` where node is installed.
Three kinds of suite are excluded by name, each with its reason in the check:

- A suite the check cannot run.
- A suite that fails on the commit M1 starts from. It is listed with its failure and the module that owns its
  script, and is not changed here. `test_mac_m2_presented` is one: it ended with six errors when the planner ran
  it on 20 November.
- A suite of a script the cleanup review found dead and this module leaves for a second landing. Those are the
  three suites of `PLATFORM-RUNTIME-MAC`'s finished qualification scripts (`test_mac_m2_presented`,
  `test_mac_m3_commission` and `test_qualify_macos_display`; findings P1-02, P2-04 and L5-03, whose verifiers
  disagree on whether the scripts are still wanted). The check does not start running tests of code that may be
  deleted next.

A suite that takes over about a minute would be excluded too, with its measured time, and would join the
main-tier list of [GATE-TIME](../GATE-TIME/DESIGN.md#mechanisms). None does: every suite the check will newly
find took 5 seconds or less when the planner ran them. The time the new suites add is recorded.

**5. A bench is kept when something runs it.** Nine benches are named by no script, command or check, and
Clippy compiles them on every check: seven in `thrum-render` and `field_culling` and `material_reload` in the
game (L5-05). All nine go, by the owner's decision, and `thrum-render` drops the bench harness dependency only
they used. `./dev baseline-inventory` ran the fixed budgets of gate G0, which closed on 5 November; it goes with
the two benches only it ran, `g2r` and `frame` (L5-15, G2-03). The frame bench's two timing rows could build
without the command, but nothing else runs them, so the bench goes whole. Four benches stay, each run by a
script that is kept. Three more benches that the review did not look at also stay for now, as
[Not built here](#not-built-here) says, so seven benches are left in all:

| Bench | Run by |
|---|---|
| The game's `valley_culling` | `scripts/culling_cost.py` and `scripts/render2d.py` |
| The game's `cooked_edit` and the cooker's `edit_to_visible` | `scripts/check_g5_tripwires.sh` |
| `link-proof`'s `allocations` | `scripts/transport_link.py` |

**6. Evidence has three homes, a provenance record has a fourth, and `docs/evidence` is none of them.** The
merge refuses a new or changed file under `docs/evidence`, yet scripts, benches and tests still write and read
there.

- A **record of a run** is written to the working folder, `.orchestrate/evidence/<MODULE>/`, which every
  worktree shares and Git does not hold. Every cost script and tripwire bench writes there by default.
- **Test data**, which a check reads, is tracked in `tests/fixtures/<module>/` or beside the crate's tests, and
  moves there with the bytes it had. That covers the `RENDER-2D` cost receipts, the atmosphere's aerial volumes,
  `WATERS`' first-frame and island cost records, which later cost checks compare with, and the reference files
  the river, cloud-model and 2D conformance tests read.
- The **list of what left the tree** is one manifest under `.config/`, where today there are two with one schema,
  one of them inside the frozen folder beside a generated ignore file of 1,100 lines. `evidence_store.py`
  keeps its commands and reports the same entries.
- A **provenance record** lists each file a module copied or translated from elsewhere, with its source and its
  licence. A person has to read it to publish the code, so it is not evidence, and a clone has to get it, so it
  is tracked: `docs/provenance/<MODULE>.md`, beside the record of the 1 November import that Git already holds
  there. The merge accepts it, because its guard covers only the two evidence folders. Five modules have one
  today, in the working folder only: `SEA`, `WATERS`, `CLOUDS`, `PUDDLES` and `RIVERS` (L8-08). Each says nothing
  has been copied yet. M5 tracks them, and `CLOUDS` M6, the first milestone that copies a library, waits for it.

The 373 unread records are deleted in one commit that only deletes. Each has a copy in the working folder. For
some the copy differs from the bytes Git holds, because a later run rewrote it or its links were rewritten when
evidence moved: the review counted 14 such files among the 373. Before the deletion, the committed bytes of every
file whose working copy differs are saved to the working folder under a second name, measured on the commit the
builder starts from. The rule `evidence-path` of mechanism 8 then holds the shape: no source file under `crates`,
`examples`, `scripts` or `tools` names a path under `docs/evidence`, except the evidence store's own code that
restores history. Data files are not source, so the exact-image fixtures that record old paths are untouched.

**7. Test tools nothing arms go with their plumbing.** The culling route test carries a TCP timing coordinator
that only an environment variable nothing sets can arm, and a campaign mode that only `culling_campaign.py`
drove (G2-05). Both go. The exact, timing and exit-arm modes stay, because `./dev check` and the cost scripts
run them.

**8. Each shape this module removes is frozen first, then counted down.** Scripts under `scripts/` went from 94
files to 185 in a week, and shader text edits in engine code from 7 to 24, while the plan waited to add each
rule until its cleanup had landed. So M6, the second milestone built, adds four rules through
[REPO-RULES](../REPO-RULES/DESIGN.md)' rule engine, each entered at the count of the commit it lands on. The
engine holds a count that can only fall, so from then on a landing that adds one more fails `./dev check`, and
each later milestone here lowers a count as it deletes.

| Rule | What it counts | Who lowers it |
|---|---|---|
| `evidence-path` | A source file under `crates`, `examples`, `scripts` or `tools` that names a path under `docs/evidence`, except the evidence store's code that restores history | M3 and M4, to zero |
| `script-caller` | A file under `scripts/` that nothing standing names: not `dev`, `tools/xtask`, a CI workflow, a `Main-branch check` line, Rust source, a tracked document outside the plans folder, nor another script that is itself called. A suite is counted with the script it tests | M1, by its deletions |
| `bench-runner` | A `[[bench]]` target that no script, `./dev` command, CI workflow or tracked document outside the plans folder names | M2, to the benches left for a second landing |
| `shader-text-edit` | A `.replace(` or `.replacen(` in a non-test Rust file that mentions WGSL. It is coarse on purpose | Nobody in this track: [the design it waits for](../ARCHITECTURE.md#not-planned-in-track-6) is not planned yet |

`script-caller` is mechanism 1 as a rule, with one caller left out: a done-when item's check. A milestone's check
stops running when the milestone is Done, so counting it would raise the count on the day a milestone closed and
fail the next unrelated merge. The consequence is that a new script only its own milestone's check runs is
refused, and its message says what to do: give the script a standing caller, or keep a proof tool that only one
milestone runs in that milestone's evidence folder, where an item's check can still run it. That is a change to
how every module adds a proof tool, so it is an assumption in [the plan](PLAN.md). `shader-text-edit` rides here
because it is one landing in the same matcher file, and its message names the hook shape the engineering
patterns already ask for. A rule's message always says what to write instead.

## Interfaces

| Interface | For |
|---|---|
| Where evidence goes: run records in the working folder, test data in fixtures, and the rule `evidence-path` | Every builder and planner |
| The rules `script-caller`, `bench-runner` and `shader-text-edit`, each a count that can only fall | Every builder: a new script or bench needs something standing that runs it, and a new shader variant is not made by replacing text |
| A module's provenance record, tracked at `docs/provenance/<MODULE>.md` | Every module that copies or translates a file from elsewhere, and whoever publishes the code |
| The evidence manifest under `.config/`, with `evidence_store.py check` and `restore` as they were | Every builder and reviewer. It changes [REPO-RULES](../REPO-RULES/DESIGN.md) M2's interface only in where the manifest is kept |

## Files each milestone touches

The lead confirms these against the builders at work before it starts a milestone. A milestone touches nothing
outside its row; a builder that finds it must writes the proposal in the log and ends `BLOCKED`.
Three files are everyone's: `.config/shape-baseline.json`, `docs/design/maturity.json` and `Cargo.lock`. Change
them when your landing moves them, whatever your row says; a landing that meets a conflict there rebases and
makes the same edit again. The rows are in build order, so M5 is first and M6 second.

| Milestone | Files | Waits for | Who else is near |
|---|---|---|---|
| M5 | Five new files, `docs/provenance/SEA.md`, `WATERS.md`, `CLOUDS.md`, `PUDDLES.md` and `RIVERS.md`; the `docs/provenance/` row of `docs/REPOSITORY.md`; one line at the head of each module's `PROVENANCE.md` in the working folder, which Git does not hold | Nothing | `CLOUDS` M6 waits for this milestone. Nobody else writes under `docs/provenance` |
| M6 | The four rules in `scripts/shape_matchers.py` and `scripts/shape_rules.py`, their fixtures in `scripts/test_shape_rules.py`, and the sections of the engineering patterns their messages point at | Nothing | `TEST-SUPPORT` M5 adds two rules to the same files: whichever lands second rebases. Both scripts are imported through a pinned comparison file, so each keeps its name and what that file uses |
| M1 | The deleted files under `scripts/`; `tools/xtask/src/bin/rustc_receipt.rs`; the script-suite step of `tools/xtask/src/main.rs`; the three F4 and T2 headless-build designs under `docs/design/` and the research notes that link the toolchain; the toolchain's rows of `docs/design/maturity.json`, and its entries in `.config/shape-baseline.json` and `.config/architecture.toml` | Nothing | No builder is in these scripts. `CLOUDS` works in `scripts/reference/cloud/`, which is not touched. `GATE-TIME` M1 adds a command to the same `main.rs` |
| M2 | `crates/thrum-render/benches/` and the `[[bench]]` tables and bench dependency of `crates/thrum-render/Cargo.toml`, with the `Cargo.lock` entries that follow from it; `examples/v1-game/benches/field_culling.rs`, `material_reload.rs`, `g2r.rs` and `frame.rs`, and the `[[bench]]` tables of `examples/v1-game/Cargo.toml`; the `baseline-inventory` command in `tools/xtask/src/main.rs`; its row in `CONTRIBUTING.md` | Nothing | `HOST` edited `field_culling.rs` seven times in three days and its M3.2 asks about benches, so deleting first saves it work: tell its builder. `GATE-TIME` M2 edits the `[[test]]` tables of the same game manifest |
| M3 | The deletions under `docs/evidence/`; one link in `docs/design/decisions/G7-SHOWCASE-ROUTE.md`; `scripts/render2d.py`, `test_render2d.py` and the receipts they read; `crates/thrum-render/src/atmosphere/gpu/tests/aerial_ground.rs` and its volumes; `scripts/evidence_store.py`, its suite and the two manifests; `.gitignore`; the evidence text of `AGENTS.md`, `CONTRIBUTING.md`, `docs/ASSET-LIFECYCLE.md`, `docs/REPOSITORY.md`, the engineering patterns and `scripts/shape_matchers.py` | Nothing | `RENDER-2D` edits `render2d.py`, and its open M2.10, M2.11, M5.4 and M5.5 run `render2d.py costs`, which must read the same receipts after the move. `GATE-TIME` M2 edits `atmosphere/gpu/tests.rs`, which declares `aerial_ground` |
| M4 | The cost and proof scripts that name `docs/evidence` (`cloud_cost.py`, `water_cost.py`, `river_cost.py`, `rain_cost.py`, `scene_view_cost.py`, `sim2d_conformance.py`, `session_memory.py`, `session_prediction.py`, `ray_tracing_proof.py`, `transport_link.py`, `environment_products.py`, `cache_reconstruction.py`, `check_g5_tripwires.sh`, one line of `run_v1_showcase.py`); the tests and benches that do (`thrum-cloud-model`'s optics test, three `thrum-river-model` tests, the cooker's edit-to-visible bench, the game's `cooked_edit` bench, one line of the game's `host.rs`); the records they read, moved to fixtures; `examples/v1-game/tests/valley_route_support/` | `TEST-SUPPORT` M2, which is rewriting `valley_route_support` | The busiest set here. Open checks of `CLOUDS`, `WATERS`, `RENDER-2D`, `SESSION` and `RAY-TRACING` run these scripts, and their builders edit them: each command must run as it did and read the same prior record |

## Not built here

- **The other dead scripts the review confirmed at low payoff** (L5-02, P2-04, P1-02, P2-05, L2-07: about 4,700
  lines). They are outside the review's ranked sections. A second landing can take them, or REPO-RULES M4.1 moves
  them.
- **Three benches the review did not look at**: `thrum-core`'s `v1_workload`, `thrum-water-model`'s `surf`, and
  the game's `recipe_cost`, which `scripts/recipe_cost.py` runs from a binary built by hand. No script or check
  names the first two. They wait for a second landing, with the scripts above.
- **The frozen pre-sprite graph snapshot** (G2-09). It is a guard, not a dead tool: it is the only graph check
  driven by the game's own exit and valley setups. Keeping it is an assumption in the plan.
- **One harness for the eleven cost scripts and for the browser scripts** (P1-05, P1-04, P2-02, P2-08), which
  need a design of their own.
- **Retiring `evidence_store.py`'s `put` and `archive`** (L2-10), which the review did not verify.

## What it must not do

Change what the engine draws, simulates or sends. Edit or delete a file the exact-baseline registry pins. Delete
or rename a script a pinned file imports, directly or through another script, or change anything the pinned files
use from it. Change what a kept script's command does, prints or exits with, beyond where it writes a record by
default. Edit a Done item, a review or a module's design. Delete a record that has no copy. Change what a
provenance record says was copied, from where, or under what licence.

## Proof

A deletion is proved by the name's absence from everything that is left, by `./dev check` and `./dev check-docs`
passing, and by a reviewer applying mechanism 1 to what was deleted and to what was kept. A move of test data is
proved by Git blobs that match and by the tests that read it. The manifest is proved by the store's own check
and suite, with the counts it reported before. A rule is proved by its fixtures, which a reviewer reads, by its
count holding from the commit it landed on, and for `evidence-path` by its zero. A tracked
provenance record is proved by a reviewer reading it against the working-folder record it came from.
