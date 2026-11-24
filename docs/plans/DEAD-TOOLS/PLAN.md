# DEAD-TOOLS plan

Scripts, benches and records that nothing runs or reads leave the tree, and no tool writes into `docs/evidence`.
[Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-6-the-cleanup-of-20-november)

Read [Mechanisms](DESIGN.md#mechanisms), [the files each work package touches](DESIGN.md#files-each-work-package-touches)
and [Exact baselines](../ARCHITECTURE.md#exact-baselines) before any work package. The full file list of each finding
is in the cleanup review's `findings/` and `verified/` folders, at the path the design gives. Main moves, so run
the design's test for "nothing runs it" again on the commit you start from, and leave alone anything that fails it.
The work packages are in build order: WP5 was added first, because `CLOUDS` WP6 waits for it, and WP6 second, so that
each count is frozen before the work package that lowers it.

## Open for the human

- Assumption: the headless release-trust toolchain is deleted, 8,500 lines: the scripts and binary written on 1 to 3 November to prepare and verify a sealed release build. Nothing has run them since, and they target an example that no longer exists. They stay in Git history, and the designs that describe them say so. The other choice is that a sealed headless release build is still wanted: then the toolchain stays whole and only its dead links are fixed.
- Assumption: `./dev baseline-inventory` and the bench only it drove go whole, and no plain simulation bench is kept in their place, because that bench measured the game's simulation, which leaves with the game. The other choice keeps one ordinary simulation cost bench, about 300 lines.
- Assumption: the game's frame bench goes too, 865 lines, because `./dev baseline-inventory` was the only thing that ran it and you saw no need to keep a bench nothing runs. The other choice keeps its two timing rows and adds a `./dev` command that runs them.
- Assumption: the culling campaign script and the campaign mode in the culling route test are deleted, because `CULLING` is Done and its plan says no loaded campaign repeats. The other choice keeps both for a later campaign.
- Assumption: the frozen pre-sprite graph snapshot stays, because it is the only graph check driven by the game's own exit and valley setups. The other choice retires it, about 326 lines: the test and its text file.
- Assumption: a proof tool that only its own work package's check runs is tracked under `scripts/`, because your rule is that a file a check reads is tracked and not evidence, and there a reviewer reads it. So `script-caller` takes any done-when item's check as naming a script and refuses only a script nothing names at all, and a tool whose work package is Done stays until a cleanup landing deletes it, as WP1 deletes thirteen now. The other choice keeps such tools in the work package's evidence folder, outside Git and outside review: that reverses your rule, and the open items of six modules that name nine scripts not yet written would have to be planned again.
- Assumption: three benches the review did not look at are left for a second landing: `thrum-core`'s `v1_workload`, `thrum-water-model`'s `surf` and the game's `recipe_cost`. Your decision named the nine. The other choice deletes any that nothing runs, in WP2.

## WP5 Each module's record of what it copied, from where and under what licence is tracked in Git, where a clone gets it

The five records are `PROVENANCE.md` in the working folders of `SEA`, `WATERS`, `CLOUDS`, `PUDDLES` and `RIVERS`,
`.orchestrate/evidence/<MODULE>/`. Their relative links were written for that folder, and some point at files only
the working folder holds, such as `CLOUDS`' `design-proof/` and its optical reference script. `docs/provenance/`
already holds the record of the 1 November import. The designs of the five modules still name the working-folder
file; `ORCHESTRATE.md` says that means the tracked record, so leave those designs alone.

## WP6 The check refuses a new script or bench that nothing runs, a new source file that names `docs/evidence`, and a new shader text edit: each count is frozen where it stands

The rule engine is `scripts/shape_rules.py`, with its matchers in `scripts/shape_matchers.py`; it compares each
rule's count for each file with `.config/shape-baseline.json`, and `tighten` records a lower count.
`pinned_names` there reads the checks of work packages that are not Done; `script-caller` reads every
work package's, Done or not. `TEST-SUPPORT` WP1 added `test-crate-edges` the same way. The engine's own suites,
`test_shape_rules.py` and `test_shape_ratchet.py`, run in no check until WP1.3 lands, so run them yourself.
`crates/thrum-asset-gpu/src/mesh_stages.rs` holds a `.replace(` that injects a fault for a test feature: it is
counted like the rest, since a frozen count needs no exemption.

## WP1 Scripts that nothing runs are gone in one landing, and the check finds the script suites that are left by itself

The suites `./dev check` runs are listed by hand in `tools/xtask/src/main.rs`, three as plain scripts and five
through copied `unittest discover` blocks. The Done item `T2-CAPTURE` WP3.3 greps that file for
`test_capture_unjudged`; that check is history. `reconstruction_timing.py` and `cooked_draw_seed_history.py` are on
the deletion list and are also named in findings this module does not build: delete them here. Leave
`scripts/reference/cloud/` alone. Among the suites the check will newly find, `test_game_gate.py` builds temporary
Git repositories and `test_sim2d_conformance.py` runs a conformance pass; each took 5 seconds or less when the
planner ran them, and `test_mac_m2_presented` failed. Deleting a counted script lowers `script-caller`, so
tighten the baseline in the same landing. Deleting `reconstruction_motion.py` removes the only caller of
`capture_mac_m2_ungranted.py`, `mac_m2_capture_launcher.swift` and `review_v1_playback.mjs`, so they go too;
`PLATFORM-RUNTIME-MAC`'s design names the first, and stays as it is. `GATE-TIME` WP1, WP6 and M7 edit other arms of the same `main.rs`, and WP5
its test step after this module's WP2.

## WP2 Benches and a command that nothing runs are gone: nine benches, and `./dev baseline-inventory` with the two benches only it ran

`material_reload.rs` and `g2r.rs` are in `.config/shape-baseline.json`, so tighten the baseline in the same
landing. The maturity ledger cites the inventory and a `material_reload` observation as history: leave the first
and reword the second as a past observation.

## WP3 The records nobody reads are out of the tree, the test data the check reads is in fixtures, and one manifest lists what left

Deleting under `docs/evidence` is allowed; adding or changing a file there is refused at the merge, so make the
deletions a commit of their own. The review found 14 deleted files whose working copies differ from Git's: five
`RENDER-2D` WP1 files a later run rewrote, and nine markdown records whose links were rewritten. The two manifests
are `docs/evidence/BULK.json` and `.config/relocated-evidence.json`. `docs/ASSET-LIFECYCLE.md` links the first and
tells builders to `git add` it, so it changes in the same commit or the link check fails. Keep a work package code
out of any new fixture path, for `REPO-RULES`' history-names rule.

## WP4 No script, bench or test reads or writes under `docs/evidence`, and the culling route test carries no tool nothing arms

`water_cost.py` reads `WATERS`' WP2 and WP4 cost records as priors, and asserts one of their paths as a string.
`scripts/run_v1_showcase.py` names one path under `docs/evidence`, at about line 492; two pinned files import that
script, so change that line and nothing they use. Open items of `PUDDLES`, `RIVERS` and `CLOUDS` name `--scene`
values these scripts do not have yet, so their builders will extend the scripts after you: leave each script's
commands and output shape as they are. The rule goes in through `REPO-RULES`' engine, with fixtures, as the track
4 rules did.
