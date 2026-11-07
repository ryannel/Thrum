# G7 showcase route plan

Design: [G7-SHOWCASE-ROUTE](G7-SHOWCASE-ROUTE.md)
Reviews: design and plan @ first draft, brief `tmp/g7-showcase/review/DESIGN-REVIEW-BRIEF.md` (local, not checked in), accept
with changes. Nine findings, all taken: the target texture profiles, a max-cell metric with a positive control, a fixed
calibration rule, the real native fallback guard, explicit checker inputs with a Wasm identity check, paused presets,
poses read from artifacts, split M1 boxes, and a baseline that survives a clean checkout.
- M1 @ 5acb0bca, brief `tmp/g7-showcase/review/M1-REVIEW-BRIEF.md` (local), accept with changes, no blocker. Eight
  findings, all taken (see the log).
- M1 fixes @ f8c46d49, same reviewer, brief: the fix list in its follow-up (report
  `tmp/g7-showcase/review/M1-REREVIEW-REPORT.md`, local), accept. Five wording and guard points, all taken.
- M2 @ 46f8112d, brief `tmp/g7-showcase/review/M2-REVIEW-BRIEF.md` (local), accept with changes, no blocker. Seven
  findings, all taken (see the log).
- M2 fixes @ fe9e26d5, same reviewer, brief: the fix list in its follow-up (report
  `tmp/g7-showcase/review/M2-REREVIEW-REPORT.md`, local), accept. Three wording nits, all taken.
Milestone: M3 Screen-independent proofs · Updated: 2026-11-07
Now: M1 and M2 done (on main at 664a4ff0). M3, commissioned by root, has started from main 961541f0
Next: the revised checker seam to root after root's hold; the startup rows and the edited-source run from a clean commit
Blocked: none. The on-screen Mac walk waits on screen control (one request denied).

## M1 Local cooked valley on both targets: done
Done when:
- [x] `run_v1_showcase.py mac --frames <n>` runs the windowed valley from a clean checkout with a route preset, exits
      0, and records the store, scene and preset. The on-screen walk is owed to the owner's screen grant. (240
      frames at 17.4 h, preset 4 changed 0 refused, built by the run at f8c46d49; evidence "Showcase launch")
- [x] `run_v1_showcase.py chrome --no-open` serves the cooked valley and the preset. Its printed URL, loaded in a real
      browser tab (the desktop app's browser pane), shows the cooked valley and the applied preset. The served Wasm
      equals the build. (Run chrome-20261107-161454: `requests.log`, pane observation and screenshot. The wording
      changed from `cooked_status().drawn = 1`; see the log.)
- [x] `run_v1_showcase.py check` passes:
      - the five Chrome-matched stops are within the median, 95th-percentile and maximum thresholds against native;
      - native capture drew the content;
      - the native generation-1 root is present in the checker's store;
      - the checker's Wasm SHA-256 equals the build;
      - every artifact pose equals its preset.
- [x] Controls hold under the fixed calibration rule. The diagnostic frames exceed ten times a threshold at every stop,
      and the checker's before-edit and edited frames exceed the thresholds. Both sets of numbers are recorded.
- [x] Each target's texture profile is recorded: Chrome Portable, native the device default (Apple ASTC by code path).
      Root is asked whether native should pin Portable.
- [x] The cold-startup proxies are recorded per target, labelled as proxies, apart from build time and scripted
      capture cost. (Native 0.34–2.73 s over five runs; Chrome's waits for the checker seam, which adds timestamps.)
- [x] An independent reviewer looks at the Chrome and native stop frames and the comparison numbers, and agrees.
      (Accept after fixes; see Reviews)

Work:
- [x] Write the six paused `showcase-<stop>.params.json` presets, including the 17.4 h clear-Sun stop.
- [x] Script: builds and `--no-build`, run directory, and `run.json` with the commit, binary and Wasm SHA-256 and
      arguments.
- [x] Commands: `mac`, `chrome` (a loopback static server for the web root, which holds the presets) and `check` (cook, native
      capture, checker `valley` and `sky-reflections`, parity, root-hash check, controls, drift check).
- [x] Add a stdlib PNG and PPM reader and the 32×18 block parity metric, with thresholds from the fixed rule.
- [x] Extend the evidence page with the build, the numbers and a contact sheet. The review brief follows the review.

## M2 Repeatable route and first baseline: done
Done when:
- [x] Two consecutive clean `check` runs agree per stop on both targets. (M1: three clean building runs after the
      rebase. M2: check-163503 and check-163650, the second within 0.005 of the baseline over 12 frames)
- [x] The per-stop 32×18 baseline grids are checked in at a path root approves, so a clean checkout can compare
      against them. (`docs/evidence/v1-workload/g7-showcase-baseline.json`, with identity, build and thresholds)
- [x] The baseline is saved: raw frames in its run directory (`tmp/g7-showcase/runs/check-20261107-163503/`, which
      replaced M1's `check-20261107-161243`), and the contact sheet and per-stop numbers in evidence.
- [x] `check` reports each stop's difference from the baseline when the identity matches. (Every run since the
      baseline reports all 12 frames, at most 0.005 apart.)
- [x] A difference beyond the thresholds fails `check` and names the stop, and a reviewed replacement can be saved
      through it. (A scratch baseline with one 8-cell patch of Chrome noon raised by 6 levels: check-164805 fails
      naming `chrome/noon` at 6.002; check-164859 saves it with `--save-baseline`, recording the replaced drift;
      check-164948 then passes within 0.005. A whole run from a deliberately edited source is still not shown.)
- [x] Route frames come from the preset files on both targets, including Chrome's 17.4 h clear-Sun stop, through
      the two seams root approved in principle. (Native `THRUM_CAPTURE_PARAMS`, checker `--case route`; Chrome's
      17.4 h stop reads 0.026/0.039/0.092 against native. Root approved both exact diffs.)
- [x] An independent milestone review agrees, and what final G7 still needs is recorded. (Accept with changes, then
      accept on re-review. What is owed is in the design's "Still owed for final G7" and the evidence's "Still open")

Work:
- [x] Post the seams to `tmp/g7-boundary/NOTES.md`. Root approved both in principle.
- [x] Native capture seam (after M1). Exact diff with root (`tmp/g7-boundary/m2-capture-seam.diff`).
- [x] Checker route case (after root's real-tab checkpoint, 20a59f47). G3c's 16-view field oracle retested
      byte-equal. Exact diff with root (`tmp/g7-boundary/m2-checker-route.diff`).
- [x] Add the baseline save and compare (`check --save-baseline`, automatic compare).
- [x] Switch the stop frames to the preset-driven path. Recalibrate only if the numbers move. (They did not.)

## M3 Screen-independent proofs: in progress
Commission: root, `tmp/g7-boundary/NOTES.md` "Root commission for the remaining screen-independent G7 proofs". The
on-screen Mac walk and the renderer findings stay with root.

Done when:
- [ ] Chrome startup proxy: repeated fresh-process rows from navigation to the first observed cooked draw, with
      the clock, poll granularity, cache state and artifact hashes; cold and warm pages apart; range and median.
      Build, cook and scripted time kept separate. The checker's instrumentation seam is approved by root before
      it is pushed.
- [ ] Edited source: the ground texture is retinted in a scratch copy, cooked through the normal route, and every
      stop runs on both targets. The run shows the new source root, the edited region on both targets, parity, and
      the canonical baseline refusing the new identity.
- [ ] A reviewed replacement is saved at a scratch baseline path, and a rerun compares against it.
- [ ] A canonical check in the same session passes against the checked-in baseline, as the control. Both proofs
      name the main revision they ran on.
- [ ] An independent milestone review agrees; its brief and verdict are on the Reviews line.

Work:
- [x] Checker: route-specific startup observation in `qualifyRoute`; exact diff posted to root
      (`tmp/g7-boundary/m3-checker-startup.diff`).
- [x] Script: `startup` command; `--source` and `--recalibrate` for `check`.
- [ ] Runs, evidence and review.

## Log
- 2026-11-07 Plan written with the design.
- 2026-11-07 Root approved both seams in principle and asked for the 17.4 h clear-Sun stop as a distinct cue. Added it
  as a sixth stop, Mac-only until the checker seam lands, and recorded the seam ordering.
- 2026-11-07 Took all nine findings from the independent design review. M1 started.
- 2026-11-07 M1 built: presets, script and evidence. The first `check` passed both controls, which fixed the
  thresholds at 1.0, 1.447 and 6.461; a second run from the clean commit gave byte-identical stop frames. The Mac
  window ran bounded, and the Chrome URL showed the valley in the browser pane. Independent review commissioned.
- 2026-11-07 Independent M1 review: accept with changes, eight findings, no blocker. Taken:
  - Build provenance is now recorded per run (`built_by_this_run`), and evidence cites only runs that built their
    own binaries from a clean tree.
  - The calibration and dirty runs are named as such.
  - The Chrome launch keeps a request log, and the pane observation is saved in its run.
  - Comparator 2 uses exact area coverage. Integer cells misaligned the 1252- and 1280-pixel frames, and that was
    most of the measured difference.
  - The positive control is cropped to the canvas.
  - The Mac record names its store and scene.
  - Process cleanup, timeouts, decoder checks and the paused check were added.
  - Timings are reported as ranges.
- 2026-11-07 Decision: the Chrome launch box no longer asks for `cooked_status().drawn` from the launched page. The
  page keeps the game object out of page scope, so only CDP (the checker) can read it. The launch is evidenced by
  its request log, the applied preset, the served Wasm and a screenshot; `check` keeps the `drawn` assertion.
- 2026-11-07 Rebased onto 80e519ec, the authored ground material and a new source root (59f0484f). Thresholds
  were recalibrated by the rule for comparator 2 on that root: worst 0.027, 0.062, 0.161, so every value is at the
  1.0 floor. Saved and reviewed the first baseline (run check-20261107-161243), and a clean run against it passed.
  The renderer findings are recorded in the design for final G7.
- 2026-11-07 Re-review: accept. Taken: the evidence names each cited run and the uncited ones, states the one-pixel
  shift by direction and stop, and says what the positive control does not cover. The script now refuses its
  thresholds on any source root other than the calibrated one. A check at that guard passed against the baseline.
  M1 done.
- 2026-11-07 M1 closed on main at 43f039f0; the overseer audited and accepted it.
- 2026-11-07 Built both seams and tested each. Native: refusals, unchanged output without the option over 134 files,
  and exact poses. The controls snap 17.4 h to the 0.01 h slider stop, so two route frames differ slightly from the
  matched-pose captures; Chrome snaps the same way. Checker: 6/6 stops, a typo'd preset fails, and the field oracle
  is byte-equal. Sent the exact diffs to root. Switched `check` to preset-driven frames on both targets, versioned
  the frame source in the baseline identity, and replaced the baseline from a clean building run. The thresholds
  stand, and the clear-Sun stop has no negative control because the checker has no diagnostic phase at 17.4 h.
- 2026-11-07 Independent M2 review: accept with changes, seven findings, no blocker. Taken:
  - `check --save-baseline` can now replace a baseline that drifted under the same identity. Before, drift blocked
    the save it exists for. `--baseline PATH` checks or saves a scratch copy.
  - The deliberate-change box was split. It had cited the positive control; drift reporting is now shown on a whole
    run against a perturbed scratch baseline, with the replacement path.
  - Design status, the baseline's location and identity, and the seam approval wording now match what happened.
  - The checker's change to `--case all` is stated in the design.
  - Seam tests were rerun on the committed, rebased binary, with its hash. The dirty run check-163312 is named as
    uncited, and the build-time range now covers M2's runs.
  - The 17.4 h Sun disc is too small for the contact sheet; the evidence says it is judged at full size. The warm-lit
    17.9 h ridge joins the renderer findings.
- 2026-11-07 Root approved the native seam and held the checker seam: its expected stops came from the same build it
  checked. The case now takes the checked-in presets as its expectation and requires the build to ship each byte for
  byte. A missing, a stale and an extra built preset each fail. Fixed the same class in the script's Chrome
  launcher, which now refuses a web root whose presets differ from the checked-in ones. Rebased on 6fa34365; the
  game binary is unchanged (`d8986d4f…`) and `./dev check` passed.
- 2026-11-07 Re-review of the M2 fixes and root's checker hold: accept. Took its three nits: the timing ranges now
  name the runs they cover, the checked-in-typo claim says what is and isn't shown on the current code, and the
  `--baseline` help says it is relative to the current directory.
- 2026-11-07 Root approved the revised checker diff (03f1476c) and kept the native approval. After a clean building
  check, field-oracle, params and controls passed on the main tip, M2 landed as a fast-forward at 664a4ff0. M2 done.
- 2026-11-07 Root commissioned M3: the Chrome startup proxy and a whole route run from an edited source. Started on
  a fresh branch from main 961541f0.
- 2026-11-07 Posted the checker's startup observation (24a0253e) to root. Trial runs passed: the retinted ground drew
  red on both targets at parity, and the startup proxy kept raw rows. Root held one measurement edge: a status-only
  read that already says Running may follow a draw, so it cannot set the lower bound. Now only a pre-loop status or
  a zero-draw `cooked_status()` reading does; otherwise the lower bound is recorded as absent (navigation start) with
  no granularity. A test-only delay of the first observation exercises that case.
- 2026-11-07 Root approved the revised seam (dd2ff503) with two precision changes, both taken: after Running, only an
  explicit zero from `cooked_status()` advances the lower bound (an absent report is an unknown count), and the scope
  no longer claims "up to one poll interval". Rebased on main f3743809 (documentation only).
- 2026-11-07 The final runs showed `cooked_status().drawn` is the drawn generation, null in a present report until
  the first draw, never an explicit zero. So "only an explicit zero" would never advance the lower bound after Running.
  A present report with no generation drawn now establishes no draw; an absent report after Running stays unknown.
  Posted to root as a fourth revision; the final runs were repeated on it.
