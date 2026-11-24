# REPO-RULES log

- 2026-11-16 M2 removed 828 oversized evidence files (2,803,858,438 bytes), all recoverable through Git-backed
  manifest entries; 55 registered reference files remain. Added the shared local store, restore/put, size rule
  and fixtures, tracked-or-manifest links, and contributor instructions. At `1ce3cd4ac`, the item commands and
  four recovery test groups passed, including clean-clone hashes and three original review links. The initial
  full gate timed out after 600 seconds in GPU tests without an assertion failure; merge uses its longer budget.
  Exact images cite the shared main check. [Measured results and retained logs](../../../.orchestrate/evidence/REPO-RULES/M2.md).
  Rebased onto `63c0d04cd`, preserving TEST-SUPPORT's `test-crate-edges` beside `evidence-size` in the registry.
  At `072082aef`, 15 shape/ratchet tests and the whole-tree shape check passed after that conflict resolution.

- 2026-11-16 Refreshed M1.5 at `6fec2d24b`, after main `81138094a` landed EFFECT-GRAPH and WATERS:
  four path attributes were removed and two existing device requests moved. Every baseline entry equals its
  present file count; the retained recipe and difference record attribute all changes to main's commits.
  The first pending merge was withdrawn before admission so this record could accompany the landing.
  The post-cleanup documentation gate passed 7,871 links. [M1's record](../../../.orchestrate/evidence/REPO-RULES/M1.md).
- 2026-11-15 Resumed M1 under the proofs-run-once-on-main ruling. Stopped the old queued GPU helper before
  acquisition and removed its obsolete rebase/prove/merge helpers. M1.6 now cites `main-branch check exact-images`;
  the merged check supplies its target proof. Current inventory still equals every baseline entry and retains
  all main-commit attributions. Earlier CPU passes and failed captures remain separate historical evidence in
  [M1's record](../../../.orchestrate/evidence/REPO-RULES/M1.md). M1 remains `in_progress` for the lead's checks.
- 2026-11-14 Planned from the owner's request to plan and drive the 14 November code review's fixes ("can you not
  just plan and drive that work?"). The lead's task set the H03 decision: no history rewrite, untrack at HEAD,
  keep links resolvable. Measured while planning: 9,054 tracked evidence files, 3.71 GiB, of which 730 files over
  2 MiB hold 2.32 GiB. The review was copied to `.orchestrate/evidence/code-review-2026-11-14/`.
- 2026-11-14 Decided: rules are ratchets against a checked-in baseline, so no rule waits for a cleanup and no
  running builder is stopped by old code. Bulk files stay recoverable from Git history through a manifest, so no
  review or design is edited. `thrum-render` is not split into crates; a layering rule holds its direction.
- 2026-11-15 Revised after [the first design review](../../../.orchestrate/evidence/REPO-RULES/design-review.md) (five Must
  Fix). The ratchet's accepted state is each rule's lowest total in the baseline's first-parent Git history, so a
  change that grows the code and the baseline together is refused, and a move carries its allowance (new item
  M1.2). The local evidence store is one folder beside the main checkout, found through Git's common directory,
  and the manifest has history and local entries. `layering` is absolute for the graph and timing and a ratchet
  for the frame runtime and presenter, whose dependency on the scene renderer no module removes. `history-names`
  covers file names, environment variables, Cargo targets and descriptions (measured: 28, 19, 1 and 2).
  `library-env` exempts the registered test-support package, settled with `TEST-SUPPORT`.
- 2026-11-15 M1's builder ended Blocked: Needs Dependency ([handoff](../../../.orchestrate/evidence/REPO-RULES/M1-handoff.md)).
  `WATERS` M1 (`2ebe8f20`) put eight `#[path]` attributes on main while the rules were still on the build branch,
  whose own commits held 33 for `path-attribute`, so the history half refused 41. Decided here, not in `WATERS`:
  a rule is registered when it reaches main, and code that reaches main first is in its baseline. The builder
  lands a branch that first lists each rule with the counts of the main it lands on. The engine is unchanged.
  M1.2 gained the moved-base case and M1.5 the record of what main added. `WATERS`' plan is untouched;
  `TEST-SUPPORT` M2.3 (`path-attribute` at zero under `crates/`) already removes the eight.
- 2026-11-15 M1 continuation implements that ruling by consolidating unpublished registration history onto
  main with present counts. The new moved-base scratch-Git test passes, as do nine shape groups, five ratchet
  groups, 26 contributor self-tests, the whole-tree rule check and `./dev check-docs` at `78fa1f63`.
  [The proof record](../../../.orchestrate/evidence/REPO-RULES/M1.md) separates the starting inventory and main's additions.
  The retained qualification helper records fresh target proof before setting builder passes and merging.
- 2026-11-15 M1 was blocked on `APPLE-TILE` for the Chrome identity refusal at `db4a38fc`. The planner's
  reading: not drift and not a stale key. All 481 digests match, the one identity that differs is the same
  instant written two ways, and the same content key passed the whole verifier within the hour before. A stall
  of the page left one due tick for the next callback. [The module list](../ARCHITECTURE.md#a-fixed-capture-that-stalls)
  gained a ruling that lets such a run be set aside with a record. No milestone was added anywhere and no item
  changed; M1's note says what to do next. [The reading](../../../.orchestrate/evidence/REPO-RULES/M1-replan/README.md).
- 2026-11-15 M1's subsequent clean-main gate found newer APPLE-PRESENTATION log links without their evidence.
  The prepared restoration was superseded while queued by APPLE-PRESENTATION's own M3 landing. Preserved
  its newer source and evidence, and main's BASELINE-GUARD status-link correction. The finishing helper
  refreshes M1's inventory and retained target proofs when upstream changes covered content, then uses
  the normal merge and check. [Reconciliation record](../../../.orchestrate/evidence/REPO-RULES/M1-link-repair.json).
- 2026-11-15 Rebased the continuation onto current main. Nine fixture groups, five ratchet groups and the
  whole-tree rule check passed at `db4a38fce`. Both native matrices and Chrome day SDR then passed, but
  Chrome day scene-linear refused one tick/alpha identity despite all 481 image hashes matching.
  [The retained observation](../../../.orchestrate/evidence/REPO-RULES/M1-identity-observation.md) distinguishes the measured
  result from the possible tick-work-yield cause. Retried the unchanged full verifier through the normal
  finishing helper; neither references nor comparison keys were changed.
- 2026-11-15 The queued retry was cancelled before GPU acquisition (exit 143): the architecture's
  load-variation exception requires every identity to match, which this refusal does not. Proposal:
  APPLE-TILE resolves the tick/alpha identity at unchanged image hashes, with its comparison intact.
  M1.1–M1.5 have refreshed CPU and inventory records; M1.6 is open and the continuation is unmerged.
  [The handoff](../../../.orchestrate/evidence/REPO-RULES/M1-handoff.md) names the failed proof and next step.
- 2026-11-15 Resumed under the planner's stalled-capture ruling: the failed run's
  [record](../../../.orchestrate/evidence/REPO-RULES/M1-identity-observation.md) now links its complete image/identity
  reading, callback times and the landing proof. The full verifier runs unchanged; this record sets
  the failure aside only when that full run passes. No reference or comparison is changed.
- 2026-11-15 The unchanged retry at `b449a81b` built and cooked successfully but stopped before Chrome:
  the native variants motion capture omitted 83 samples, reached six staging slots in use and reported
  incomplete; its writer finished successfully. No pixel/identity comparison refused output in this run.
  [The record](../../../.orchestrate/evidence/REPO-RULES/M1-native-capture-failure-record.json) and compressed runtime/manifest
  retain the diagnosis. CPU/inventory evidence was refreshed independently. Retrying the full verifier
  unchanged tests whether the capture pressure is transient; no capacity, comparison or reference changes.
- 2026-11-20 Planner, for [track 6](../ARCHITECTURE.md#track-6-the-cleanup-of-20-november): M3 now also waits for `GATE-TIME` M5, `DEAD-TOOLS` M4, `DEAD-CODE` M6 and `TEST-SUPPORT` M4, in place of `TEST-SUPPORT` M2. The cleanup deletes and converts code in the files M3 renames and splits, so the one rename lands after it and carries no dead code through. No item changed. M3 was Not Started.
- 2026-11-24 Added WP5 before WP3 and WP4. The 3.0 migration (`e9a3923fa`) renamed `milestones` to `work_packages`
  in the done-when files. The stale reader lost script callers and test target names, breaking `workspace-tests`
  on main. WP5 updates both readers and checks the production reader against a checked-in done-when file.
- 2026-11-24 Design review, Must Fix 1. Migration `77c70eaac` cleared the renamed headings and brief links
  apart from the legacy G7 record. WP5 removes the three remaining tracked evidence records, retains their
  bytes in Git history and proves the full repository guard. The lead routes the remaining ATMOSPHERE anchor
  to a plan-only correction before WP5 merges. This can land on a red gate. WP5 stays ahead of WP3 and WP4,
  and ATMOSPHERE's subsequent design review belongs to that module.
- 2026-11-24 Design review, Must Fix 2. WP3.2 preserves APPLE-TILE's registered comparison and the two exact
  environment keys its V1 capture consumers read. The zero check reports the comparison as the pin source
  and keeps counting unrelated uses. The owner has no comparison refresh to make.
- 2026-11-24 Design review, Must Fix 3. WP3.2 extends `history-names` to standalone `WP<n>` codes, with
  fixtures on the naming surfaces and a new rule version. WP5 keeps the existing matcher, versions and baseline.
- 2026-11-24 Design review, Must Fix 4. Mechanism 4 and the interface table point to the tracked
  `.config/evidence-manifest.json` delivered by DEAD-TOOLS WP3. The original WP2 paths describe its recorded
  proof, which still stands.
- 2026-11-24 Review suggestion. Mechanism 1 now describes per-file counts and the repository-wide bound.
  The shared track 4 wording remains for a project-wide edit.
- 2026-11-24 Final design review, Must Fix 1. WP5 skips outbound links from untracked Markdown listed in the
  evidence manifest. WP5.2 exercises the production link check before and after restoring the original G7 bytes,
  including a valid heading link and a broken link from a tracked page. WP5 still lands before WP3 and WP4.
  Mechanism 4 also names the manifest WP2 committed at `7a5d16f30`, as the review suggested.
- 2026-11-24 WP5 restored the two done-when readers and archived the last three tracked evidence records.
  The shape-rule suite and history recovery passed on the Mac. The G7 fixture resolves `#checks` before and
  after restore and still reports broken links from tracked Markdown. The existing working-folder G7 copy
  was preserved, with the original bytes saved beside [the WP5 proof](../../../.orchestrate/evidence/REPO-RULES/WP5/).
  `./dev check-docs` passed after rebasing onto the ATMOSPHERE planner's anchor correction in `6f77f8151`.
  The shared `exact-images` check supplies the post-merge image proof.
