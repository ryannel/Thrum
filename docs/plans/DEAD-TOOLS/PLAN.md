# DEAD-TOOLS plan

Scripts, benches and records that nothing runs or reads leave the tree, and no tool writes into `docs/evidence`.
[Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-6-the-cleanup-of-20-november)

Read [Mechanisms](DESIGN.md#mechanisms), [the files each milestone touches](DESIGN.md#files-each-milestone-touches)
and [Exact baselines](../ARCHITECTURE.md#exact-baselines) before any milestone. The full file list of each finding
is in the cleanup review's `findings/` and `verified/` folders, at the path the design gives. Main moves, so run
the design's test for "nothing runs it" again on the commit you start from, and leave alone anything that fails it.

## Open for the human

- Assumption: the headless release-trust toolchain is deleted, 8,500 lines: the scripts and binary written on 1 to 3 November to prepare and verify a sealed release build. Nothing has run them since, and they target an example that no longer exists. They stay in Git history, and the designs that describe them say so. The other choice is that a sealed headless release build is still wanted: then the toolchain stays whole and only its dead links are fixed.
- Assumption: all nine benches that nothing runs are deleted. The other choice keeps the two or three a performance phase would run again, each with a comment naming the command that runs it; say which.
- Assumption: `./dev baseline-inventory` and the bench only it drove go whole, and no plain simulation bench is kept in their place, because that bench measured the game's simulation, which leaves with the game. The other choice keeps one ordinary simulation cost bench, about 300 lines.
- Assumption: the culling campaign script and the campaign mode in the culling route test are deleted, because `CULLING` is Done and its plan says no loaded campaign repeats. The other choice keeps both for a later campaign.
- Assumption: the provenance records of `SEA`, `WATERS`, `CLOUDS`, `PUDDLES` and `RIVERS`, which list files copied from Broadsides with their source and licence, stay in the working folder, where they have always been and where a clone does not get them. Each says nothing has been copied yet. The other choice tracks them in Git, under `docs/provenance/`, before `CLOUDS` M6 copies its first library.

## M1 Scripts that nothing runs are gone in one landing, and the check finds the script suites that are left by itself

The suites `./dev check` runs are listed by hand in `tools/xtask/src/main.rs`, three as plain scripts and five
through copied `unittest discover` blocks. The Done item `T2-CAPTURE` M3.3 greps that file for
`test_capture_unjudged`; that check is history. `reconstruction_timing.py` and `cooked_draw_seed_history.py` are on
the deletion list and are also named in findings this module does not build: delete them here. Leave
`scripts/reference/cloud/` alone. `GATE-TIME` M5 edits the test step of the same `main.rs` after this module's M2.

## M2 Benches and a command that nothing runs are gone: nine benches, and `./dev baseline-inventory` with the bench only it drove

`material_reload.rs` and `g2r.rs` are in `.config/shape-baseline.json`, so tighten the baseline in the same
landing. The frame bench keeps its two divan rows. The maturity ledger cites the inventory and a `material_reload`
observation as history: leave the first and reword the second as a past observation.

## M3 The records nobody reads are out of the tree, the test data the check reads is in fixtures, and one manifest lists what left

Deleting under `docs/evidence` is allowed; adding or changing a file there is refused at the merge, so make the
deletions a commit of their own. The two manifests are `docs/evidence/BULK.json` and
`.config/relocated-evidence.json`. `docs/ASSET-LIFECYCLE.md` links the first and tells builders to `git add` it, so
it changes in the same commit or the link check fails. Keep a milestone code out of any new fixture path, for
`REPO-RULES`' history-names rule.

## M4 No script, bench or test reads or writes under `docs/evidence`, and the culling route test carries no tool nothing arms

`water_cost.py` reads `WATERS`' M2 and M4 cost records as priors, and asserts one of their paths as a string.
Open items of `PUDDLES`, `RIVERS` and `CLOUDS` name `--scene` values these scripts do not have yet, so their
builders will extend the scripts after you: leave each script's commands and output shape as they are. The rule
goes in through `REPO-RULES`' engine, with fixtures, as the track 4 rules did.
