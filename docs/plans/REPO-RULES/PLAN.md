# REPO-RULES plan

`./dev check` refuses the code shapes the 14 November review found, bulk files leave the tracked tree, and names
and layout are put right once. [Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria ·
[log](LOG.md) · [module list](../ARCHITECTURE.md#track-4-the-code-review-and-the-games-separation)

Read [Mechanisms](DESIGN.md#mechanisms) and [Exact baselines](../ARCHITECTURE.md#exact-baselines) before any
work package. `scripts/` is a covered path, so every landing here needs a verifier proof.

## Open for the human

- Assumption: the rules bind every builder from the day WP1 lands, running builders included. A rule only stops new violations, never code that was already there, and its message says what to write instead. The cap on evidence files binds from WP2, when a refused file has somewhere to go.
- Assumption: code that reaches main before the rules do counts as already there. `WATERS` WP1 added eight `#[path]` attributes to its tests on 15 November, against the patterns reference and before any check could refuse them; they enter the baseline, and `TEST-SUPPORT` WP2 removes them with the rest. The other choice, which WP1's builder asked for, holds WP1 until `WATERS` removes them, and holds it again for each violation another builder lands first.
- Assumption: the cap on an evidence file is 2 MiB, the review's figure. About 2.3 GiB of 3.7 GiB leaves the checkout and about 1.4 GiB of smaller files stays. A lower cap removes more; say so and WP2's cap changes.
- Assumption: files that leave stay recoverable from Git history on this Mac and on the remote, by one command. New large captures go to one folder beside the main checkout, `/Users/ryannel/Workspace/thrum-evidence-store`, shared by every worktree, with no remote copy and no service, so a lost disk loses them.
- Assumption: a rule counts a kind of violation across the whole repository and that count can only go down. A change that removes one violation and adds another of the same kind passes, because the repository is no worse; that is what lets a file be moved or split without an exception.
- Assumption: `thrum-render` stays one crate, and its frame runtime and presenter go on naming the scene renderer's types. The review wanted that dependency turned round and the crate split in three. A rename cannot do it and no module plans the interface it needs, so a rule stops the dependency growing and holds the graph and timing clean. Say so and the interface is planned as its own work package.
- Assumption: nothing is built now to stop a stalled browser tick from changing one record's identity in a Chrome capture, which is what stopped WP1. It is known twice, each costing one more verifier run of about 17 minutes, and no image is wrong. The fix changes a covered path, so landing it alone makes every builder with an open proof run the verifier again. Say so and it becomes an item of `HOST` WP2, which rewrites that policy, or a work package of its own now.
- Assumption: three of the review's themes get no fix work package now, because its order of attack does not schedule them: shader source assembled four ways (T7), environment variables read as configuration (T8, beyond a rule that stops new reads in libraries) and mechanisms built several times (T11).

## WP1 A change that adds a string error, a hidden reach across packages, an environment read in a library or a copied device request is refused by `./dev check`

The rule engine, its fixtures, the ratchet tests and the finishing helper are built on branch `build/repo-rules`
at `9ac94c46`, and the handoff on that branch, `.orchestrate/evidence/REPO-RULES/WP1-handoff.md`, says what each run
proved. WP1.1 to WP1.5 have their proof there and WP1.6 is open: the verifier at `db4a38fc` refused one Chrome
identity with every image equal. [The module list](../ARCHITECTURE.md#a-fixed-capture-that-stalls) settles that
as a stalled tick. Nothing is owed by `APPLE-TILE` and no reference moves, so the handoff's wait for an owning
resolution is over: rebase, run the verifier unchanged, and complete the record that ruling asks for, whose
failed half is on the branch. The eight `#[path]` attributes in
`crates/thrum-water-model/tests/` reached main before the rules, so they enter the baseline under
[main moves before a rule lands](DESIGN.md#mechanisms). Leave them, and every other module's code, as written:
`TEST-SUPPORT` WP2.3 takes them to zero. The branch's own commits hold baselines from before main moved, down to
33 for `path-attribute`, and the check reads them as accepted history, so the branch can land only once each rule
is first listed with the counts of the main it lands on. Builders land while you work: measure again whenever
your base moves, and put each difference in WP1.5's record with the commit that made it. WP1.2 gained the
moved-base case and WP1.5 the differences; a pass on any item needs proof at the commit you claim.
`.config/architecture.toml` has no `test_role` yet (`TEST-SUPPORT` WP1 adds it), so `library-env`'s exemption is
proved on a scratch registry.

## WP2 The checkout no longer carries bulk media: every file that left is listed, comes back by one command, and every link still resolves

Evidence folders belong to their modules, and several have builders at work. Remove files only by the rule, never
by judging another module's evidence, and leave every review, record and design as written: the link check learns
about the manifest so that nothing needs editing. `.gitignore` already ignores archives and `.webm` under
`.orchestrate/evidence/`. A check must leave the loop's clean checkout unchanged, so tests work in scratch. `docs/ASSET-LIFECYCLE.md` and `CONTRIBUTING.md` gain the rule for
new captures. Measure the tree before and after.

## WP5 The repository gate passes with migrated plans

Build WP5 first, independently of WP3 and WP4. The orchestrate migration at `77c70eaac` already fixed the
renamed-heading and brief links it could resolve. Before WP5 merges, the lead routes the one remaining plan
link to an ATMOSPHERE planner: in [its design](../ATMOSPHERE/DESIGN.md), repoint “Apple tile M0.4” to
[APPLE-TILE WP0](../APPLE-TILE/PLAN.md#wp0-a-repeatable-offscreen-valley-proves-the-comparison-path-done).
The plan-only change can land while the gate is red. ATMOSPHERE owes a design review for its changed design.
WP5 needs the link correction on main and can merge while that review is pending.

Read `work_packages` in `done_when_checks()` and `pinned_names()` and exercise both readers in the existing
shape-rule suite. Keep the rule versions and baseline. In the same landing, remove these three tracked paths
from the checkout and record their original bytes as history entries in `.config/evidence-manifest.json`:

- `docs/evidence/CULLING/M2.md`
- `docs/evidence/render/cooked-draw/BASE`
- `docs/evidence/v1-workload/G7-CHROME-VALLEY-2026-11-07.md`

Keep a working-folder copy of each record, preserving any copy already there. The history entries keep existing
links and Done proof references recoverable. The repository guard uses the manifest and tracked paths it already
reads to skip outbound links from restored Markdown. Extend the existing evidence-store suite in scratch using
the original G7 bytes. Run the production Markdown walk and link check with the record absent and again after
`restore`, with a tracked page linking to its `#checks` heading in both runs. Confirm that adding a missing-file
link to the tracked page still yields a diagnostic after restore. WP5 lands through the normal `./dev check`
gate. The shared `exact-images` proof follows the merge.

## WP3 Names describe their jobs and the renderer's layers point one way

This lands as one change so other builders rebase once. Take the list of names that open checks use from every
`docs/plans/*/DONE.json` at the commit you start from, and again before you land, because other modules' plans move
while you work. The showcase game's files are `SHOWCASE`'s and the proving ground's selections are their modules':
rename inside them only what the renames above force. Preserve the pinned APPLE-TILE comparison and its two
environment keys as specified in [mechanism 5](DESIGN.md#mechanisms). WP3.4's check names `src/graph.rs`, `src/graph/`,
`src/timing.rs` and `src/timing/` in `thrum-render`: the split keeps the graph and timing under those paths.
`runtime.rs` imports the scene renderer in about fifteen places today and `present.rs` in one; they stay.

## WP4 The repository is tidy: scripts are findable, superseded designs are marked, pins are declared once, and feature-gated code is checked

`scripts/` is a covered path and its top-level names are used by hundreds of approved checks, so the layout is
built around the names that are in use. Documents under `docs/design/` are linked from approved designs; mark and
list, do not move.
