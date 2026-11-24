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
nine benches that nothing runs are deleted, and provenance records are tracked in Git, in place before `CLOUDS` WP6
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
[REPO-RULES](../REPO-RULES/DESIGN.md) WP4.1 would otherwise do. A design or research note that describes a
deleted tool keeps its text and gains one line naming the last commit that held the code; its links to the
deleted files go, because the link check refuses a link to a missing file. A row of
[the maturity ledger](../../design/maturity.json) that describes a deleted tool says it is retired and names that
commit, and claims nothing as present.

**3. Script deletions land once, and the pinned comparison files are never touched.** `scripts/` is a covered
path of the exact baselines, so each landing there asks the main-branch check `exact-images` for a run. WP1 deletes
every dead script in one landing. The registry pins seven comparison files by content, and no work package edits
one: `exact_baselines.py`, `exact_baselines_native.py`, `apple_tile_m0.py`, `apple_tile_m1.py`,
`apple_tile_m2.py`, `compare_v1_pixels.py` and `apple_tile_m1_compare_browser.py`. A script one of them imports,
directly or through another script, is not pinned and may be edited, but it keeps its name and everything the
pinned files use from it. Imported directly: `run_v1_showcase.py`, `t2_writer_probe.py`, `capture_spool.py`,
`capture_v1_runtime.py`, `capture_result.py` and `repo_check.py`. Imported through `repo_check.py`:
`evidence_store.py` and `shape_rules.py`, and through that `shape_matchers.py`. WP6, WP3 and WP4 edit four of these,
in the lines their rows name.

What WP1 deletes, each the owner's call where the plan says so:

| What | Finding | Size |
|---|---|---|
| The headless release-trust toolchain, written 1 to 3 November to prepare and verify a sealed release build for an example since deleted, run by nothing, compiled by every check. Fifteen scripts: `prepare_headless_cargo_home`, `prepare_macos_tool_root_clone`, `inventory_headless_tool_roots`, `run_bounded_native_build`, `qualify_held_macos_native_ab`, `release_cargo_selection`, the three `hold_macos_*` and the six `verify_*`. Their thirteen `test_*` suites, and the binary `tools/xtask/src/bin/rustc_receipt.rs` | P1-01, P2-01, L5-01 | 8,500 lines |
| Thirteen one-shot work package proof scripts and `creator_profile_probe.rs`, each written for one work package of `APPLE-TILE`, `APPLE-PRESENTATION`, `COOKED-DRAW`, `CREATOR` or `TRANSPORT`. WP1.2's check lists them | P1-03 | 1,650 lines |
| Three files that only one of those scripts, `reconstruction_motion.py`, runs: `capture_mac_m2_ungranted.py`, `mac_m2_capture_launcher.swift` and `review_v1_playback.mjs`. Deleting their one caller leaves them dead, and the rule `script-caller` of mechanism 8 would count them | P1-02 and L5-02, in their verifiers' notes | 210 lines |
| `culling_campaign.py` and its suite. `CULLING` is Done and its plan says no loaded campaign repeats | G2-05 | 350 lines with WP4's half |

**4. The check finds script suites itself.** Today `./dev check` runs 8 of 29 `scripts/test_*.py` suites from a
hand-kept list, and the suites of the shape rules the check enforces run nowhere. After the dead suites are
gone, the check discovers every `scripts/test_*.py`, and every `scripts/test_*.mjs` where node is installed.
Three kinds of suite are excluded by name, each with its reason in the check:

- A suite the check cannot run.
- A suite that fails on the commit WP1 starts from. It is listed with its failure and the module that owns its
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
  has been copied yet. WP5 tracks them, and `CLOUDS` WP6, the first work package that copies a library, waits for it.

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
rule until its cleanup had landed. So WP6, the second work package built, adds four rules through
[REPO-RULES](../REPO-RULES/DESIGN.md)' rule engine, each entered at the count of the commit it lands on. The
engine holds a count that can only fall, so from then on a landing that adds one more fails `./dev check`, and
each later work package here lowers a count as it deletes.

| Rule | What it counts | Who lowers it |
|---|---|---|
| `evidence-path` | A source file under `crates`, `examples`, `scripts` or `tools` that names a path under `docs/evidence`, except the evidence store's code that restores history | WP3 and WP4, to zero |
| `script-caller` | A file under `scripts/` that nothing names: not `dev`, `tools/xtask`, a CI workflow, a `Main-branch check` line, Rust source, a done-when item's check, a tracked document outside the plans folder and `docs/evidence`, nor another script that is itself named. A suite is counted with the script it is named for, and a suite named for no script is not counted | WP1, by its deletions |
| `bench-runner` | A `[[bench]]` target that no script, `./dev` command, CI workflow or tracked document outside the plans folder and `docs/evidence` names | WP2, to the benches left for a second landing |
| `shader-text-edit` | A `.replace(` or `.replacen(` in a non-test Rust file that mentions WGSL. It is coarse on purpose | Nobody in this track: [the design it waits for](../ARCHITECTURE.md#not-planned-in-track-6) is not planned yet |

**`script-caller` is the merge's gate, and it is looser than mechanism 1 on purpose.** Mechanism 1 is the test a
person applies before deleting, and asks what runs a tool today. The rule asks only whether anything names the
script, and takes as naming it two things mechanism 1 does not: a document that merely mentions it, and a
done-when item's check whether its work package is open or Done. Each keeps the count from moving at a time no
builder controls. A count that followed open checks only would rise on the day a work package closed and fail the
next unrelated merge. So the rule refuses a script that nothing names at all, and its message says what to do:
name it from what runs it.

**A proof tool is tracked.** A script that only its own work package's check runs lives under `scripts/`, at the
path its item names. By the owner's rule that
[a file a check reads is test data, not evidence](../../../ORCHESTRATE.md#starting-from-a-clean-clone), it is
not kept in the evidence folder: there no reviewer would read it in the merge, and it could change after the
review without a trace. Its item's check is what names it, so it lands with no extra step. Nine scripts that
open items name and nobody has written yet land that way:

| Script | Open items that name it |
|---|---|
| `swarm.py`, `swarm_sim.py` | `SWARM` |
| `game_ui.py`, `game_ui_sim.py` | `GAME-UI`, `SWARM` |
| `render2d_sim.py` | `RENDER-2D`, `GAME-UI`, `SWARM` |
| `view_volume.py`, `atmosphere_cost.py` | `ATMOSPHERE` |
| `repo_layout.py`, `open_check_names.py` | `REPO-RULES`, `GAME-SPLIT` |

When its work package is Done a proof tool is history. Nothing runs it again, mechanism 1 calls it dead unless
something else has come to run it, and any landing may delete it, as
[ruling 1](../ARCHITECTURE.md#seam-rulings-for-track-6) allows and as WP1 does for the thirteen there are today.
Nothing deletes it on the day its work package closes, so such tools stay until a cleanup landing takes them. That
is the cost of tracking them, and an assumption in [the plan](PLAN.md).

**A deletion must not raise a count.** Deleting a script's last caller, or a bench's, leaves it counted where it
was not, and the engine refuses a count above the lowest it has held. Two things keep a correct deletion from
failing its own check:

- *Records are not callers.* `docs/evidence` and the working folder name scripts and benches as history, and WP3
  deletes 373 such records. Neither rule reads them, so WP3 moves no count.
- *The change that removes a last caller deletes what it orphans, or gives it another caller, in the same
  landing.* That holds for a deleted script, document or Rust file in any module, and for a planner who drops
  the last check that names a script. The rule's failure names the orphaned file. WP1 meets it once: deleting
  `reconstruction_motion.py` orphans the three files listed with what WP1 deletes, which go with it.

A suite named for no script, such as `test_shape_ratchet.py`, is not counted, because WP1.3 takes the hand-kept
list of suites out of the check: from then on the check runs every suite it finds and names the ones it
excludes, so no suite is unrun and unnamed. There are six today.

The planner ran an approximation of the rule on the planning commit, which matches by file name and Python
import, so its counts are rough: about 35 of 179 files counted, 11 of them deleted by WP1, and WP1's deletions
orphan the three files above and no other. The script and its output are in the working folder, under
`planning/`.

`shader-text-edit` rides here because it is one landing in the same matcher file, and its message names the
hook shape the engineering patterns already ask for. A rule's message always says what to write instead.

## Interfaces

| Interface | For |
|---|---|
| Where evidence goes: run records in the working folder, test data in fixtures, and the rule `evidence-path` | Every builder and planner |
| The rules `script-caller`, `bench-runner` and `shader-text-edit`, each a count that can only fall | Every builder and planner. A new script needs something that names it, which for a proof tool is its item's check, and a new bench needs something that runs it. A change that removes a script's or a bench's last caller deletes it or names it elsewhere in the same landing. A new shader variant is not made by replacing text. The modules whose open items name scripts not yet written (`SWARM`, `GAME-UI`, `RENDER-2D`, `ATMOSPHERE`, `REPO-RULES`, `GAME-SPLIT`) land them as their items say |
| A module's provenance record, tracked at `docs/provenance/<MODULE>.md` | Every module that copies or translates a file from elsewhere, and whoever publishes the code |
| The evidence manifest under `.config/`, with `evidence_store.py check` and `restore` as they were | Every builder and reviewer. It changes [REPO-RULES](../REPO-RULES/DESIGN.md) WP2's interface only in where the manifest is kept |

## Files each work package touches

The lead confirms these against the builders at work before it starts a work package. A work package touches nothing
outside its row; a builder that finds it must writes the proposal in the log and ends `BLOCKED`.
Three files are everyone's: `.config/shape-baseline.json`, `docs/design/maturity.json` and `Cargo.lock`. Change
them when your landing moves them, whatever your row says; a landing that meets a conflict there rebases and
makes the same edit again. The rows are in build order, so WP5 is first and WP6 second.

| Work package | Files | Waits for | Who else is near |
|---|---|---|---|
| WP5 | Five new files, `docs/provenance/SEA.md`, `WATERS.md`, `CLOUDS.md`, `PUDDLES.md` and `RIVERS.md`; the `docs/provenance/` row of `docs/REPOSITORY.md`; one line at the head of each module's `PROVENANCE.md` in the working folder, which Git does not hold | Nothing | `CLOUDS` WP6 waits for this work package. Nobody else writes under `docs/provenance` |
| WP6 | The four rules in `scripts/shape_matchers.py` and `scripts/shape_rules.py`, their fixtures in `scripts/test_shape_rules.py`, and the sections of the engineering patterns their messages point at | Nothing | `TEST-SUPPORT` WP5 adds two rules to the same files: whichever lands second rebases. Both scripts are imported through a pinned comparison file, so each keeps its name and what that file uses |
| WP1 | The deleted files under `scripts/`, with any file there that a deletion leaves with no caller; `tools/xtask/src/bin/rustc_receipt.rs`; the script-suite step of `tools/xtask/src/main.rs`; the three F4 and T2 headless-build designs under `docs/design/` and the research notes that link the toolchain; the toolchain's rows of `docs/design/maturity.json`, and its entries in `.config/shape-baseline.json` and `.config/architecture.toml` | Nothing | No builder is in these scripts. `CLOUDS` works in `scripts/reference/cloud/`, which is not touched. `GATE-TIME` WP1 adds a command to the same `main.rs` |
| WP2 | `crates/thrum-render/benches/` and the `[[bench]]` tables and bench dependency of `crates/thrum-render/Cargo.toml`, with the `Cargo.lock` entries that follow from it; `examples/v1-game/benches/field_culling.rs`, `material_reload.rs`, `g2r.rs` and `frame.rs`, and the `[[bench]]` tables of `examples/v1-game/Cargo.toml`; the `baseline-inventory` command in `tools/xtask/src/main.rs`; its row in `CONTRIBUTING.md` | Nothing | `HOST` edited `field_culling.rs` seven times in three days and its WP3.2 asks about benches, so deleting first saves it work: tell its builder. `GATE-TIME` WP2 edits the `[[test]]` tables of the same game manifest |
| WP3 | The deletions under `docs/evidence/`; one link in `docs/design/decisions/G7-SHOWCASE-ROUTE.md`; `scripts/render2d.py`, `test_render2d.py` and the receipts they read; `crates/thrum-render/src/atmosphere/gpu/tests/aerial_ground.rs` and its volumes; `scripts/evidence_store.py`, its suite and the two manifests; `.gitignore`; the evidence text of `AGENTS.md`, `CONTRIBUTING.md`, `docs/ASSET-LIFECYCLE.md`, `docs/REPOSITORY.md`, the engineering patterns and `scripts/shape_matchers.py` | Nothing | `RENDER-2D` edits `render2d.py`, and its open WP2.10, WP2.11, WP5.4 and WP5.5 run `render2d.py costs`, which must read the same receipts after the move. `GATE-TIME` WP2 edits `atmosphere/gpu/tests.rs`, which declares `aerial_ground` |
| WP4 | The cost and proof scripts that name `docs/evidence` (`cloud_cost.py`, `water_cost.py`, `river_cost.py`, `rain_cost.py`, `scene_view_cost.py`, `sim2d_conformance.py`, `session_memory.py`, `session_prediction.py`, `ray_tracing_proof.py`, `transport_link.py`, `environment_products.py`, `cache_reconstruction.py`, `check_g5_tripwires.sh`, one line of `run_v1_showcase.py`); the tests and benches that do (`thrum-cloud-model`'s optics test, three `thrum-river-model` tests, the cooker's edit-to-visible bench, the game's `cooked_edit` bench, one line of the game's `host.rs`); the records they read, moved to fixtures; `examples/v1-game/tests/valley_route_support/` | `TEST-SUPPORT` WP2, which is rewriting `valley_route_support` | The busiest set here. Open checks of `CLOUDS`, `WATERS`, `RENDER-2D`, `SESSION` and `RAY-TRACING` run these scripts, and their builders edit them: each command must run as it did and read the same prior record |

## Not built here

- **The other dead scripts the review confirmed at low payoff** (L5-02, P2-04, P1-02, P2-05, L2-07: about 4,700
  lines). They are outside the review's ranked sections. A second landing can take them, or REPO-RULES WP4.1 moves
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
passing with no frozen count raised, and by a reviewer applying mechanism 1 to what was deleted and to what was
kept. A move of test data is proved by Git blobs that match and by the tests that read it. The manifest is
proved by the store's own check and suite, with the counts it reported before. A rule is proved by its fixtures,
which a reviewer reads, by a reviewer's own count against the log's, by its count holding from the commit it
landed on, and for `evidence-path` by its zero. A tracked provenance record is proved by a reviewer reading it
against the working-folder record it came from.
