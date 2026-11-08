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
- M3 @ 3672cf62 (proofs on f38166f0), brief `tmp/g7-showcase/review/M3-REVIEW-BRIEF.md` (local), accept with changes,
  no blocker. Four should-fix findings and six nits, all taken (see the log).
- M4 C @ 6015e4e5, brief `tmp/g7-showcase/review/M4-REVIEW-BRIEF.md` (local), accept with changes, no blocker.
  Findings taken (see the log).
- M4 C against E, brief `tmp/g7-showcase/review/M4-512-REVIEW-BRIEF.md` (local): E canonical, skip the 8 m trial. Root
  approved E's exact source and root `70d2baf8` (G7 NOTES).
Milestone: M4 Continuous valley ground material · Updated: 2026-11-07
Now: M1–M4 done. M4 landed E as its own change (route evidence "M4: continuous valley ground material")
Next: root's night baseline refresh on E; acquire-to-display latency waits on G4 exposing presented time
Blocked: none. The Mac screen walk ran under the owner's grant and root accepted its held views (G7 NOTES).

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

## M3 Screen-independent proofs: done
Commission: root, `tmp/g7-boundary/NOTES.md` "Root commission for the remaining screen-independent G7 proofs". The
on-screen Mac walk and the renderer findings stay with root.

Done when:
- [x] Chrome startup proxy: repeated fresh-process rows from navigation to the first observed cooked draw, with
      the clock, poll granularity, cache state and artifact hashes; cold and warm pages apart; range and median.
      Build, cook and scripted time kept separate. The checker's instrumentation seam is approved by root before
      it is pushed. (startup-175135: cold upper 152–175 ms, warm 108–169 ms; root approved 10ea0392 after five
      rounds; evidence "M3")
- [x] Edited source: the ground texture is retinted in a scratch copy, cooked through the normal route, and every
      stop runs on both targets. The run shows the new source root, the edited region on both targets, parity, and
      the canonical baseline refusing the new identity. (check-174724: root `49041ec0`, parity 0.033/0.175/0.281,
      `other identity`; sheet `route-edited-source.png`)
- [x] A reviewed replacement is saved at a scratch baseline path, and a rerun compares against it. (check-174832
      saved, check-174940 within 0.005)
- [x] A canonical check in the same session passes against the checked-in baseline, as the control. Both proofs
      name the main revision they ran on. (check-174420 built f38166f0 on main 6459379d, within 0.005)
- [x] An independent milestone review agrees; its brief and verdict are on the Reviews line. (accept with changes,
      all findings taken)

Work:
- [x] Checker: route-specific startup observation in `qualifyRoute`; exact diff posted to root
      (`tmp/g7-boundary/m3-checker-startup.diff`).
- [x] Script: `startup` command; `--source` and `--recalibrate` for `check`.
- [x] Runs and evidence.
- [x] Independent review.

## M4 Continuous valley ground material: done
Commission: root, `tmp/g7-boundary/NOTES.md` "Root commission after G7 M3: continuous valley ground material"; the
look item in the [atmosphere plan](../modules/ATMOSPHERE-PLAN.md). Chrome and offscreen Metal only; the owner-screen
walk stays separate. Root owns camera, terrain geometry, shadows, exposure, sky, shaders, sampler, the checker and the
shared scene; the 17.4 h brightness step and the shadow findings are root's and are kept apart from material
boundaries.

Done when:
- [x] One material variant, authored only in the ground albedo and terrain UVs, makes fine ground grain legible at
      native size at chase distance, with continuous detail into the middle distance, and removes the visible
      material edge and flat band.
- [x] It stays stable as the view pans, with no repeating stamp, moiré, glare or loss of the ridge, watched in a slow
      pan from the real game.
- [x] Positions, normals, indices, winding, placement and silhouette are proven unchanged.
- [x] The same route runs in Chrome and offscreen Metal through the normal cook, with paired noon and 17.4/17.9 h
      views, within the established cross-target tolerance.
- [x] An independent review agrees; its brief and verdict are on the Reviews line.
- [x] Root approves the exact source and the paired views, and only then is the canonical baseline refreshed.
      `./dev check` passes; source identities, captures and the review are recorded in the evidence.

Work (each names the done-when item it serves, numbered from the top):
- [x] A scratch variant under `tmp/g7-showcase/m4/` whose identity proof shows only UVs and albedo changed. (Variant
      C; A and B rejected. Serves 1 and 3)
- [x] Route checks and slow pans on both targets; prospective cooked root and comparisons posted to root.
      (check-184529, root `451cc790`; NOTES "G7 M4 prospective ground material". Serves 2 and 4)
- [x] Independent review of C: accept with changes, findings taken. (Serves 5)
- [x] 512² comparison at the same 16 m repeat (E, root `70d2baf8`): stops, 1° pans, route check, paired startup.
      (Serves 1, 2 and 4)
- [x] Independent review of the 512² comparison: E chosen, 8 m skipped; the exact source diff posted to root.
      (Serves 5 and 6)
- [x] Root approves E's three files, root `70d2baf8` and the unchanged `[1, 1, 1]`. (NOTES "Root verdict on ground
      variant E and its baseline". Serves 6)
- [x] E in `examples/v1-game/assets/showcase/` and `CALIBRATED_ROOT` set; a fresh build of current main passes the
      route with the refreshed baseline and the night exposure main already has. (Serves 4 and 6; check-224619 saved,
      check-224747 rechecked)
- [x] `./dev check` passes, and the evidence records identities, captures, both reviews and cost. (Serves 6)

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
- 2026-11-07 Root confirmed the correction and asked that only the page's exact pre-loop labels prove no draw, in
  both reads; any other status proves nothing. Taken, rechecked on the normal and late-observer cases, and the final
  runs were repeated on it. Root approved the seam to land after that change.
- 2026-11-07 Main moved by simulation code (6459379d), so the final proofs ran again on the rebased f38166f0: the
  canonical control (byte-identical frames to M2), five startup launches, the edited-source runs and both refusals,
  and the seam cases. Chrome exited with code 2 in the checker's shared cleanup after `route` and `field-oracle` had
  passed every page; both passed on rerun. `./dev check` passed. Evidence written; review commissioned.
- 2026-11-07 Independent M3 review: accept with changes, no blocker; every cited number recomputed. Taken:
  - Runs recorded `origin/main` at run time, which had moved; they now record the merge base as `main_base`, and the
    evidence names 6459379d.
  - `--recalibrate` refuses the calibrated root, where it could only loosen the check.
  - The startup scope no longer claims a bound within one poll. The cold page's warm OS and shader caches, its fixed
    stop and the observer's overhead are stated.
  - Nits: the checker's route scope and usage text now name the startup bracket (sent to root as a wording-only
    delta); the Fetching median names its 27 pages; a refused canonical save leaves no run directory; the sheet names
    its runs and rows; the parity row says what binds; the cleanup fault is recorded for both cases.
- 2026-11-07 M4 added from root's ground-material commission after the overseer audited M3.
- 2026-11-07 M4 variants. A (value-noise octaves, camera-centred tiled UVs) printed a lattice on the far slopes; B
  (isotropic spectral grain only) left the far ground flat; C adds a weak 1–2.7 m band and passes the route check on
  both targets (root `451cc790`). The game snaps yaw to 1°, so the real-game pans are 1° steps. Posted to root.
- 2026-11-07 Review of C: accept with changes. Scoped the band claim, extended the identity proof, added low-Sun
  canonical pan controls and wrote the PNG with Up filters (same pixels). Root then asked for a 512² tile before any
  canonical change, as C adds 5.24 MB to the Portable upload. E (C's field box-filtered to 512², same repeat) keeps
  far and mid detail and loses 27% of near gradient energy; it uploads 2.93 MB against C's 7.12 MB, and its paired cold first draw was about 75 ms sooner on localhost.
  The review chose E and advised against an 8 m repeat. Posted E to root.
- 2026-11-07 Root approved E and root `70d2baf8` with `[1, 1, 1]`. Landed on main 85a7c281: check-224619 built, passed
  and saved the baseline; check-224747 reran against it; `./dev check` passed. The Mac screen walk ran under the
  owner's grant and root accepted its held views; presentation cadence and latency stay for an owner-present window.
- 2026-11-08 Mac presentation cadence on glass, owner present: 120.2–120.8 presents/s on the 120 Hz panel, 3000
  presented from 3002–3004 display-link ticks, with no occlusion or drops. The median display interval is inferred
  at 8.3 ms. Acquire-to-display latency cannot be observed through wgpu and moves to G4 (evidence "Mac presentation
  cadence on glass").
