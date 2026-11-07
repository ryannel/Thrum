# G7 showcase route plan

Design: [G7-SHOWCASE-ROUTE](G7-SHOWCASE-ROUTE.md)
Reviews: design and plan @ first draft, brief `tmp/g7-showcase/review/DESIGN-REVIEW-BRIEF.md` (local, not checked in), accept
with changes. Nine findings, all taken: the target texture profiles, a max-cell metric with a positive control, a fixed
calibration rule, the real native fallback guard, explicit checker inputs with a Wasm identity check, paused presets,
poses read from artifacts, split M1 boxes, and a baseline that survives a clean checkout.
Milestone: M1 Local cooked valley on both targets · Updated: 2026-11-07
Now: building M1 (since 2026-11-07)
Next: native capture route seam (after M1); checker route case (root's real-tab checkpoint c80b6346 is on main)
Blocked: none. The Mac window walk waits on the owner's screen grant, because the screen is locked.

## M1 Local cooked valley on both targets: in progress
Done when:
- [ ] `run_v1_showcase.py mac --frames <n>` runs the windowed valley from a clean checkout with a route preset, exits
      0, and records the store, scene and preset. The on-screen walk is owed to the owner's screen grant.
- [ ] `run_v1_showcase.py chrome --no-open` serves the cooked valley and the preset. Its printed URL, loaded in a real
      browser tab (the desktop app's browser pane), reports `cooked_status().drawn = 1` and the applied preset.
      The served Wasm equals the build.
- [ ] `run_v1_showcase.py check` passes:
      - the five Chrome-matched stops are within the median, 95th-percentile and maximum thresholds against native;
      - native capture drew the content;
      - the native generation-1 root is present in the checker's store;
      - the checker's Wasm SHA-256 equals the build;
      - every artifact pose equals its preset.
- [ ] Controls hold under the fixed calibration rule. The diagnostic frames exceed ten times a threshold at every stop,
      and the checker's before-edit and edited frames exceed the thresholds. Both sets of numbers are recorded.
- [ ] Each target's texture profile is recorded: Chrome Portable, native the device default. Root is asked whether
      native should pin Portable.
- [ ] The cold-startup proxies are recorded per target, labelled as proxies, apart from build time and scripted
      capture cost.
- [ ] An independent reviewer looks at the Chrome and native stop frames and the comparison numbers, and agrees.

Work:
- [ ] Write the six paused `showcase-<stop>.params.json` presets, including the 17.4 h clear-Sun stop.
- [ ] Script: builds and `--no-build`, run directory, and `run.json` with the commit, binary and Wasm SHA-256 and
      arguments.
- [ ] Commands: `mac`, `chrome` (a loopback static server for the web root and presets) and `check` (cook, native
      capture, checker `valley` and `sky-reflections`, parity, root-hash check, controls, drift check).
- [ ] Add a stdlib PNG and PPM reader and the 32×18 block parity metric, with thresholds from the fixed rule.
- [ ] Extend the evidence page with the build, the numbers, a contact sheet and the review brief.

## M2 Repeatable route and first baseline: not started
Done when:
- [ ] Two consecutive clean `check` runs agree per stop on both targets.
- [ ] The per-stop 32×18 baseline grids are checked in at a path root approves, so a clean checkout can compare
      against them.
- [ ] The baseline is saved: raw frames in `tmp/g7-showcase/baseline/`, and the contact sheet and per-stop numbers in
      evidence.
- [ ] `check --baseline` reports each stop's difference from the baseline. A deliberate change, such as
      a retinted texture in a scratch copy, is reported as a difference.
- [ ] Route frames come from the preset files on both targets, including Chrome's 17.4 h clear-Sun stop, through
      the two seams root approved in principle. A seam not yet landed is recorded as open, and its target uses the matched poses.
- [ ] An independent milestone review agrees, and what final G7 still needs is recorded.

Work:
- [x] Post the seams to `tmp/g7-boundary/NOTES.md`. Root approved both in principle.
- [ ] Native capture seam (after M1). Bring the exact diff to root before pushing.
- [ ] Checker route case (after root's real-tab checkpoint is on main). Retest G3c's 16-view field oracle alongside it,
      and bring the exact diff to root before pushing.
- [ ] Add the baseline save and compare.
- [ ] Switch the stop frames to the preset-driven path. Recalibrate only if the numbers move.

## Log
- 2026-11-07 Plan written with the design.
- 2026-11-07 Root approved both seams in principle and asked for the 17.4 h clear-Sun stop as a distinct cue. Added it
  as a sixth stop, Mac-only until the checker seam lands, and recorded the seam ordering.
- 2026-11-07 Took all nine findings from the independent design review. M1 started.
