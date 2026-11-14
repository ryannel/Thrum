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
