# REPO-RULES plan

`./dev check` refuses the code shapes the 14 November review found, bulk files leave the tracked tree, and names
and layout are put right once. [Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria ·
[log](LOG.md) · [module list](../ARCHITECTURE.md#track-4-the-code-review-and-the-games-separation)

Read [Mechanisms](DESIGN.md#mechanisms) and [Exact baselines](../ARCHITECTURE.md#exact-baselines) before any
milestone. `scripts/` is a covered path, so every landing here needs a verifier proof.

## Open for the human

- Assumption: the rules bind every builder from the day M1 lands, running builders included. A rule only stops new violations, never code that was already there, and its message says what to write instead. The cap on evidence files binds from M2, when a refused file has somewhere to go.
- Assumption: the cap on an evidence file is 2 MiB, the review's figure. About 2.3 GiB of 3.7 GiB leaves the checkout and about 1.4 GiB of smaller files stays. A lower cap removes more; say so and M2's cap changes.
- Assumption: files that leave stay recoverable from Git history on this Mac and on the remote, by one command. New large captures go to a folder beside the repository, with no remote copy and no service, so a lost disk loses them.
- Assumption: `thrum-render` stays one crate. The review suggested splitting it into three; a rule holds the direction of its layers instead, and a split is looked at again after `RENDER-2D` M1 builds the seam between the frame and the scene.
- Assumption: three of the review's themes get no fix milestone now, because its order of attack does not schedule them: shader source assembled four ways (T7), environment variables read as configuration (T8, beyond a rule that stops new reads in libraries) and mechanisms built several times (T11).

## M1 A change that adds a string error, a hidden reach across packages, an environment read in a library or a copied device request is refused by `./dev check`

`scripts/repo_check.py` runs every check with no arguments, and `tools/xtask` calls it for both `check` and
`check-docs`; its self-test is `scripts/test_repo_check.py`. The patterns reference already names each shape, so a
rule's message points at its section and the reference gains a line for each rule. Running builders meet these
rules at their next `./dev check`, so the baseline must hold the true count for every file at the commit you land
on: rebase, then tighten, then land.

## M2 The checkout no longer carries bulk media: every file that left is listed, comes back by one command, and every link still resolves

Evidence folders belong to their modules, and several have builders at work. Remove files only by the rule, never
by judging another module's evidence, and leave every review, record and design as written: the link check learns
about the manifest so that nothing needs editing. `docs/ASSET-LIFECYCLE.md` and `CONTRIBUTING.md` gain the rule for
new captures. Measure the tree before and after.

## M3 Names say what things do: the scene renderer is called the scene renderer, milestone codes are out of file names, and the renderer's layers point one way

This lands as one change so other builders rebase once. Take the list of names that open checks use from every
`docs/plans/*/DONE.json` at the commit you start from, and again before you land, because other modules' plans move
while you work. The showcase game's files are `SHOWCASE`'s and the proving ground's selections are their modules':
rename inside them only what the renames above force.

## M4 The repository is tidy: scripts are findable, superseded designs are marked, pins are declared once, and feature-gated code is checked

`scripts/` is a covered path and its top-level names are used by hundreds of approved checks, so the layout is
built around the names that are in use. Documents under `docs/design/` are linked from approved designs; mark and
list, do not move.
