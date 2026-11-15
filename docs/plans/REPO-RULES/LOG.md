# REPO-RULES log

- 2026-11-14 Planned from the owner's request to plan and drive the 14 November code review's fixes ("can you not
  just plan and drive that work?"). The lead's task set the H03 decision: no history rewrite, untrack at HEAD,
  keep links resolvable. Measured while planning: 9,054 tracked evidence files, 3.71 GiB, of which 730 files over
  2 MiB hold 2.32 GiB. The review was copied to `docs/evidence/code-review-2026-11-14/`.
- 2026-11-14 Decided: rules are ratchets against a checked-in baseline, so no rule waits for a cleanup and no
  running builder is stopped by old code. Bulk files stay recoverable from Git history through a manifest, so no
  review or design is edited. `thrum-render` is not split into crates; a layering rule holds its direction.
- 2026-11-15 Revised after [the first design review](../../evidence/REPO-RULES/design-review.md) (five Must
  Fix). The ratchet's accepted state is each rule's lowest total in the baseline's first-parent Git history, so a
  change that grows the code and the baseline together is refused, and a move carries its allowance (new item
  M1.2). The local evidence store is one folder beside the main checkout, found through Git's common directory,
  and the manifest has history and local entries. `layering` is absolute for the graph and timing and a ratchet
  for the frame runtime and presenter, whose dependency on the scene renderer no module removes. `history-names`
  covers file names, environment variables, Cargo targets and descriptions (measured: 28, 19, 1 and 2).
  `library-env` exempts the registered test-support package, settled with `TEST-SUPPORT`.
- 2026-11-15 M1's builder ended Blocked: Needs Dependency ([handoff](../../evidence/REPO-RULES/M1-handoff.md)).
  `WATERS` M1 (`2ebe8f20`) put eight `#[path]` attributes on main while the rules were still on the build branch,
  whose own commits held 33 for `path-attribute`, so the history half refused 41. Decided here, not in `WATERS`:
  a rule is registered when it reaches main, and code that reaches main first is in its baseline. The builder
  lands a branch that first lists each rule with the counts of the main it lands on. The engine is unchanged.
  M1.2 gained the moved-base case and M1.5 the record of what main added. `WATERS`' plan is untouched;
  `TEST-SUPPORT` M2.3 (`path-attribute` at zero under `crates/`) already removes the eight.
- 2026-11-15 M1 continuation implements that ruling by consolidating unpublished registration history onto
  main with present counts. The new moved-base scratch-Git test passes, as do nine shape groups, five ratchet
  groups, 26 contributor self-tests, the whole-tree rule check and `./dev check-docs` at `78fa1f63`.
  [The proof record](../../evidence/REPO-RULES/M1.md) separates the starting inventory and main's additions.
  The retained qualification helper records fresh target proof before setting builder passes and merging.
- 2026-11-15 M1 was blocked on `APPLE-TILE` for the Chrome identity refusal at `db4a38fc`. The planner's
  reading: not drift and not a stale key. All 481 digests match, the one identity that differs is the same
  instant written two ways, and the same content key passed the whole verifier within the hour before. A stall
  of the page left one due tick for the next callback. [The module list](../ARCHITECTURE.md#a-fixed-capture-that-stalls)
  gained a ruling that lets such a run be set aside with a record. No milestone was added anywhere and no item
  changed; M1's note says what to do next. [The reading](../../evidence/REPO-RULES/M1-replan/README.md).
- 2026-11-15 M1's subsequent clean-main gate found newer APPLE-PRESENTATION log links without their evidence.
  The prepared restoration was superseded while queued by APPLE-PRESENTATION's own M3 landing. Preserved
  its newer source and evidence, and main's BASELINE-GUARD status-link correction. The finishing helper
  refreshes M1's inventory and retained target proofs when upstream changes covered content, then uses
  the normal merge and check. [Reconciliation record](../../evidence/REPO-RULES/M1-link-repair.json).
- 2026-11-15 Rebased the continuation onto current main. Nine fixture groups, five ratchet groups and the
  whole-tree rule check passed at `db4a38fce`. Both native matrices and Chrome day SDR then passed, but
  Chrome day scene-linear refused one tick/alpha identity despite all 481 image hashes matching.
  [The retained observation](../../evidence/REPO-RULES/M1-identity-observation.md) distinguishes the measured
  result from the possible tick-work-yield cause. Retried the unchanged full verifier through the normal
  finishing helper; neither references nor comparison keys were changed.
- 2026-11-15 The queued retry was cancelled before GPU acquisition (exit 143): the architecture's
  load-variation exception requires every identity to match, which this refusal does not. Proposal:
  APPLE-TILE resolves the tick/alpha identity at unchanged image hashes, with its comparison intact.
  M1.1–M1.5 have refreshed CPU and inventory records; M1.6 is open and the continuation is unmerged.
  [The handoff](../../evidence/REPO-RULES/M1-handoff.md) names the failed proof and next step.
