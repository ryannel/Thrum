# REPO-RULES log

- 2026-11-14 Planned from the owner's request to plan and drive the 14 November code review's fixes ("can you not
  just plan and drive that work?"). The lead's task set the H03 decision: no history rewrite, untrack at HEAD,
  keep links resolvable. Measured while planning: 9,054 tracked evidence files, 3.71 GiB, of which 730 files over
  2 MiB hold 2.32 GiB. The review was copied to `docs/evidence/code-review-2026-11-14/`.
- 2026-11-14 Decided: rules are ratchets against a checked-in baseline, so no rule waits for a cleanup and no
  running builder is stopped by old code. Bulk files stay recoverable from Git history through a manifest, so no
  review or design is edited. `thrum-render` is not split into crates; a layering rule holds its direction.
