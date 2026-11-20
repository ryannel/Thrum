# DEAD-TOOLS plan

Scripts, benches and records that nothing runs or reads leave the tree, and no tool writes into `docs/evidence`.
[Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-6-the-cleanup-of-20-november)

Read [Mechanisms](DESIGN.md#mechanisms), [the files each milestone touches](DESIGN.md#files-each-milestone-touches)
and [Exact baselines](../ARCHITECTURE.md#exact-baselines) before any milestone. The full file list of each finding
is in the cleanup review's `findings/` and `verified/` folders, at the path the design gives. Main moves, so run
the design's test for "nothing runs it" again on the commit you start from, and leave alone anything that fails it.
The milestones are in build order: M5 was added first, because `CLOUDS` M6 waits for it.

## Open for the human

- Assumption: the headless release-trust toolchain is deleted, 8,500 lines: the scripts and binary written on 1 to 3 November to prepare and verify a sealed release build. Nothing has run them since, and they target an example that no longer exists. They stay in Git history, and the designs that describe them say so. The other choice is that a sealed headless release build is still wanted: then the toolchain stays whole and only its dead links are fixed.
- Assumption: `./dev baseline-inventory` and the bench only it drove go whole, and no plain simulation bench is kept in their place, because that bench measured the game's simulation, which leaves with the game. The other choice keeps one ordinary simulation cost bench, about 300 lines.
- Assumption: the game's frame bench goes too, 865 lines, because `./dev baseline-inventory` was the only thing that ran it and you saw no need to keep a bench nothing runs. The other choice keeps its two timing rows and adds a `./dev` command that runs them.
- Assumption: the culling campaign script and the campaign mode in the culling route test are deleted, because `CULLING` is Done and its plan says no loaded campaign repeats. The other choice keeps both for a later campaign.
- Assumption: the frozen pre-sprite graph snapshot stays, because it is the only graph check driven by the game's own exit and valley setups. The other choice retires it, about 326 lines: the test and its text file.
- Assumption: three benches the review did not look at are left for a second landing: `thrum-core`'s `v1_workload`, `thrum-water-model`'s `surf` and the game's `recipe_cost`. Your decision named the nine. The other choice deletes any that nothing runs, in M2.

## M5 Each module's record of what it copied, from where and under what licence is tracked in Git, where a clone gets it

The five records are `PROVENANCE.md` in the working folders of `SEA`, `WATERS`, `CLOUDS`, `PUDDLES` and `RIVERS`,
`.orchestrate/evidence/<MODULE>/`. Their relative links were written for that folder, and some point at files only
the working folder holds, such as `CLOUDS`' `design-proof/` and its optical reference script. `docs/provenance/`
already holds the record of the 1 November import. The designs of the five modules still name the working-folder
file; `ORCHESTRATE.md` says that means the tracked record, so leave those designs alone.

## M1 Scripts that nothing runs are gone in one landing, and the check finds the script suites that are left by itself

The suites `./dev check` runs are listed by hand in `tools/xtask/src/main.rs`, three as plain scripts and five
through copied `unittest discover` blocks. The Done item `T2-CAPTURE` M3.3 greps that file for
`test_capture_unjudged`; that check is history. `reconstruction_timing.py` and `cooked_draw_seed_history.py` are on
the deletion list and are also named in findings this module does not build: delete them here. Leave
`scripts/reference/cloud/` alone. Among the suites the check will newly find, `test_game_gate.py` builds temporary
Git repositories and `test_sim2d_conformance.py` runs a conformance pass: time them first. `GATE-TIME` M5 edits
the test step of the same `main.rs` after this module's M2.

## M2 Benches and a command that nothing runs are gone: nine benches, and `./dev baseline-inventory` with the two benches only it ran

`material_reload.rs` and `g2r.rs` are in `.config/shape-baseline.json`, so tighten the baseline in the same
landing. The maturity ledger cites the inventory and a `material_reload` observation as history: leave the first
and reword the second as a past observation.

## M3 The records nobody reads are out of the tree, the test data the check reads is in fixtures, and one manifest lists what left

Deleting under `docs/evidence` is allowed; adding or changing a file there is refused at the merge, so make the
deletions a commit of their own. The review found 14 deleted files whose working copies differ from Git's: five
`RENDER-2D` M1 files a later run rewrote, and nine markdown records whose links were rewritten. The two manifests
are `docs/evidence/BULK.json` and `.config/relocated-evidence.json`. `docs/ASSET-LIFECYCLE.md` links the first and
tells builders to `git add` it, so it changes in the same commit or the link check fails. Keep a milestone code
out of any new fixture path, for `REPO-RULES`' history-names rule.

## M4 No script, bench or test reads or writes under `docs/evidence`, and the culling route test carries no tool nothing arms

`water_cost.py` reads `WATERS`' M2 and M4 cost records as priors, and asserts one of their paths as a string.
`scripts/run_v1_showcase.py` names one path under `docs/evidence`, at about line 492; two pinned files import that
script, so change that line and nothing they use. Open items of `PUDDLES`, `RIVERS` and `CLOUDS` name `--scene`
values these scripts do not have yet, so their builders will extend the scripts after you: leave each script's
commands and output shape as they are. The rule goes in through `REPO-RULES`' engine, with fixtures, as the track
4 rules did.
