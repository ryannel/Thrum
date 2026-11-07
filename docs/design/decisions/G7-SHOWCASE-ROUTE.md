# G7 decision: local showcase launch and fixed review route

7 November 2026. This is the governing design for a bounded slice of build-plan item G7. It covers local launch
and one review route for the cooked valley on Mac and in Chrome. Plan: [G7-SHOWCASE-ROUTE-PLAN](G7-SHOWCASE-ROUTE-PLAN.md).

**Status.** Designed; not built. The cooked valley already runs natively and in Chrome
([G7 Chrome valley](../../evidence/v1-workload/G7-CHROME-VALLEY-2026-11-07.md)), but launching it takes several
hand-assembled commands. No check fails if Chrome quietly draws the diagnostic scene instead.

## Commission

From a clean checkout, one command launches the existing cooked valley locally, on the Mac or in Chrome. Both
targets read the same cooked input, and a reviewer walks one named route of camera and solar settings using the
controls the game already exposes. The route is checked into the repository. A visible check proves that each
target drew the valley and not a fallback. The route is repeatable and has a first baseline.

Portable qualifies this slice. The final Apple tile, EDR, sky and cadence evidence stays with their own gates. This
slice records what final G7 still needs and does not tick it.

**Texture profiles differ by target.**
- **Chrome** fetches Portable products.
- **Native** keeps the device's default preference (`texture_preference: None` in `cooked.rs`), which on Apple GPUs
  selects the ASTC variant first.

"Same cooked input" therefore means the same cooked publication, not the same texture variant. The parity numbers
below include the ASTC-versus-Portable difference, and the evidence says so. Native does not report which profile it
resolved, and pinning it to Portable would need a seam in `cooked.rs`. That is asked of root rather than assumed.

## Constraints

- **Reuse what exists.** That means the cooker (`thrum-cook`), the native game (`v1-game`), the WebGPU page
  (`examples/v1-game/web/`) and typed parameter files (`THRUM_PARAMS`, `?params=`). The browser checker is
  reused read-only as a command, and its qualification harness is not forked.
- **No new binaries, dependencies, renderer changes or deployment.** The orchestration is a stdlib Python script.
- **Owned files.**
  - `scripts/run_v1_showcase.py`;
  - `examples/v1-game/params/showcase-*.params.json`;
  - this design and its plan;
  - an extension of the G7 Chrome valley evidence page.
- **Shared files.** The checker, both game entrypoints, the web page, the valley assets and recipe, and the sky and
  frame runtime belong to others. A missing seam in one of them is posted in `tmp/g7-boundary/NOTES.md` and
  waits for root's approval before any change.

## The route

The route is an ordered list of stops. Each stop is one parameter preset,
`examples/v1-game/params/showcase-<stop>.params.json`. It holds `sky.solar_time` (hours), `camera.yaw` and
`camera.pitch` (degrees), and `world.paused: true`, so the stop holds still. Unpaused, the valley's day lasts about
five minutes, and sunrise would reach noon within a minute. These are keys the game's typed controls already declare.

A preset file is the one authority for its stop. The script reads the stop list from the files and never keeps its
own copy of the values.

| Stop | Hours | Yaw° | Pitch° | Shows |
|---|---:|---:|---:|---|
| `sunrise` | 6.1 | 0 | 0 | authored framing, low warm light, long ridge shadows |
| `noon` | 12 | 0 | 0 | authored framing, overhead light, short shadows |
| `sunset-clear-sun` | 17.4 | -90 | 0 | the Sun disk still above the ridge, horizon gradient |
| `sunset-sun` | 17.9 | -90 | 0 | the Sun just behind the ridge: in-scatter and silhouette |
| `twilight` | 18.48 | 0 | 0 | authored framing after sunset, sky-lit only |
| `night-upslope` | 0 | 0 | 7 | night sky over the ridge line |

The two sunset stops are distinct cues. The route catches a lost Sun disk at 17.4 h and a lost horizon gradient at
17.9 h, where the Sun on the current valley is occluded by the ridge.

**Matched poses.**
- **Native.** Capture (`THRUM_CAPTURE_SCENE`) already saves every stop at the same values: `sunrise`, `noon`,
  `sunset-sun-clear`, `sunset-sun`, `twilight` and `night-upslope`.
- **Chrome.** The checker's `valley` case saves five of the stops through the page's own preset buttons and sliders:
  `phase-scene-sunrise`, `phase-paused-scene-noon`, `phase-sunset`, `phase-scene-twilight` and
  `phase-upslope-midnight`. The 17.4 h clear-Sun view has no Chrome counterpart until the checker route seam lands,
  so M1 compares five stops across targets and shows the sixth on the Mac only.

M1 therefore needs no seam. The drift check takes the poses each target actually used from its own artifacts:
- **Native:** the records in `captures.json`.
- **Chrome:** each phase's `requested` and `edited.controls` values in the checker's `run.json`.

Each pose must equal its preset, so if either side drifts, the check stops before it compares anything.

## Launch

`scripts/run_v1_showcase.py` has three commands. Each command works in its own run directory under `tmp/g7-showcase/`.
- **Builds.** Each command builds what it needs first, unless `--no-build` is given: release `thrum-cook` and `v1-game`,
  and the web page through `examples/v1-game/web/build.sh`.
- **Source.** Each command cooks or watches a copy of `examples/v1-game/assets/showcase`, so an edit made during a
  session never touches the checked-in source.
- **Recording.** Each command records the commit, the build identities (binary and Wasm SHA-256) and the run
  arguments in `run.json`.

**`mac [--stop <name>] [--frames <n>]`**
- Starts `thrum-cook watch <copy> <store>`.
- Runs the windowed game with `THRUM_ASSET_STORE=<store>`, `THRUM_SCENE=valley` and `THRUM_PARAMS=<preset>`.
- `--frames` sets `THRUM_V1_FRAMES`, a bounded run used for scripted verification.
- The preset applies when rendering starts. `L` re-applies it after an edit to the file.
- Other stops come from the game's own keys and panel, or from relaunching with another `--stop`.
- The watcher stops when the game exits.

**`chrome [--stop <name>] [--no-open]`**
- Starts `thrum-cook watch <copy> <store> --serve 127.0.0.1:0`.
- Starts a loopback static server for the built web root and the presets directory.
- Opens Chrome at `index.html?assets=<origin>&params=params/showcase-<stop>.params.json`.
- `--no-open` prints the URL instead.
- Both servers stop on Ctrl-C.

**`check`** runs the visible check below and exits non-zero if it fails.

## Visible check: the valley, not a fallback

Both targets can silently draw something other than the valley.
- **Chrome.** Without `?assets` the page draws the diagnostic scene, and its status text reads the same either way.
- **Native.** Without `THRUM_SCENE=valley` the game draws the diagnostic scene.

The check compares pictures, not status. It runs in four steps:

1. **Cook.** Cook one store from a source copy with `thrum-cook cook`.
2. **Capture native.** Run native capture against that store with `THRUM_SCENE=valley`.
   - That setting makes `attach_cooked_scene` require the store.
   - Capture then refuses, exiting non-zero, unless the cooked content is drawn within 30 s with no loader errors.
   - This is the native fallback guard. The `scene` field in `captures.json` only echoes the environment, so it proves
     nothing.
3. **Capture Chrome.** Run `check_v1_browser.mjs --case valley`, read-only, with explicit `--cook`, `--web-root`,
   `--assets-source` (the same source copy) and a new `--output`. The checker cooks its own copy, serves it, and
   asserts that generation 1 was drawn.
   - The script requires the checker's `actualWasmSha256` to equal the Wasm it just built, so a stale web root fails.
   - Then it runs `--case sky-reflections`, the same procedure without `?assets`, as the negative control.
4. **Compare.** For each Chrome-matched stop, compare the Chrome valley frame with the native frame:
   - Box-average both frames to a 32×18 grid in 8-bit sRGB, which absorbs the 1252×704 versus 1280×720 extents and
     sub-pixel differences.
   - Take the per-cell mean absolute RGB difference.
   - A stop passes when the median, 95th-percentile and maximum cells all fall under their thresholds. The maximum
     cell catches a missing rock or sphere, which the median and 95th percentile would not see.

The check fails in any of these cases:
- **Native fallback:** capture refuses.
- **Chrome fallback:** the cooked frames diverge from native at any stop.
- **Same cooked input:** the generation-1 root named by the native store's `HEAD` is absent from the checker's
  content-addressed store. The root covers every product hash, so its presence means the checker cooked the identical
  first publication.
- **Negative control:** the diagnostic frames do not exceed ten times the threshold on at least one statistic at
  every stop.
- **Positive control:** the checker's own `valley-before-edit` and `valley-edited` frames, which differ only by the
  retinted texture, do not exceed the thresholds. This proves the metric sees a wrong texture, not just a wrong
  scene.

**Calibration rule, fixed before the first run.**
- Each statistic's threshold is max(1.0, 3 × the worst valley-to-native value across the stops).
- Both sets of numbers are recorded.
- If either control fails under that rule, the metric is wrong, and the check goes back to design rather than being
  tuned to pass.

The exp2 experiment measured valley-to-native at a median of at most 0.07, a 95th percentile of at most 0.48 and a
maximum of at most 2.2. Diagnostic-to-native was at least 9.3, 30 and 107 respectively, the closest being the night
stop. Under the rule, the thresholds would be 1.0, 1.44 and 6.5.

Pixel parity is a regression tripwire. It is not visual review. Each milestone review still looks at the frames.

## Repeatable route and baseline

The route is repeatable when two consecutive `check` runs from clean scratch produce stop frames that agree with
each other under the same metric, on both targets.
- **Baseline.** The first accepted run's stop frames are kept as raw files under `tmp/g7-showcase/baseline/` and as one
  compact contact sheet in the evidence page. `tmp/` is not checked in, so a clean checkout repeats the route only if the
  per-stop 32×18 grids (a few KB of JSON) are checked in too. That file is a new path, and its location is asked of
  root in M2.
- **Diffs.** Later runs report their per-stop difference from the baseline.
- **Updates.** Changing a baseline is an explicit act with a reason in the evidence. It never happens automatically.

**Preset-driven route frames.** Today both targets reach the stops through pose lists that only match the presets.
Making the presets themselves drive the captured frames needs a seam in each of two shared files:
- **Native.** Capture mode honours route presets as an optional input, for example
  `THRUM_CAPTURE_ROUTE=<preset files>`, applied through the existing checked parameter path. A missing or refused
  preset fails the route. Without the option, capture output is unchanged. The file is
  `examples/v1-game/src/bin/window/capture.rs`.
- **Chrome.** The checker loads `?params=<preset>` per stop and saves that frame, for example `--case route` with
  preset files. It reuses the existing server and capture helpers, proves that the preset was applied and that the
  cooked generation drew, and leaves the existing case dispatch and assertions unchanged. The file is
  `scripts/check_v1_browser.mjs`.

The route sequence's source stays in the shared presets. Root approved both seams in principle
(`tmp/g7-boundary/NOTES.md`, "Root verdict on G7 route seams").
- **Native seam.** It starts after M1.
- **Checker seam.** It starts after root's real-tab checkpoint is on main. G3c's 16-view field oracle is retested
  alongside it.

Each exact shared diff goes to root before it is pushed. Until a seam lands, its target uses the matched poses with the
drift assertion.

## What is measured, and what is kept apart

- **Cold startup** is recorded as two proxies, labelled as such and never as windowed startup:
  - **Native:** time from the capture process launching to the content being drawn, offscreen.
  - **Chrome:** time from navigation to `cooked_status().drawn` in the checker's headless run.

  Build time is recorded separately and never counted. Windowed cold startup on the Mac waits on the owner's screen.
- **Visible edit and pacing** is watched on screen in the running game: a live edit reaching the view, and motion
  along the route. It needs a presenting window. While the Mac screen is locked, the Mac items wait on the owner's
  screen grant, and Chrome's are taken through the checker's timeline. That timeline is headless, so it says
  nothing about physical cadence.
- **Scripted capture cost** covers offscreen readback, CDP screenshots and comparison. It is reported only as run
  time, so a reader never takes it for frame time.

The GPU is shared, so each timing carries its load and is labelled as contended. No speedup is claimed from one
run.

## Alternatives rejected

- **Headless `chrome --screenshot`.** It exits about 2.5 s after start, with the page still at "Starting WebGPU…",
  so it cannot reach a cooked frame.
- **A private CDP driver in the script.** It would fork the checker's harness, which is forbidden. The route-case
  seam keeps one harness.
- **Checking status text or `cooked_status()` alone.** That is state, not what the viewer sees. The checker already
  asserts it. This slice adds the picture.
- **Golden images with tight tolerances.** Chrome and native differ slightly in resolve and dither, so a tight
  golden would flake. Coarse parity against native plus a negative control fails on the substitutions that matter.

## Still owed for final G7

This slice ticks none of these:
- Apple tile products on the Mac.
- EDR presentation.
- Final sky look.
- Physical presentation cadence and stutter, recorded as whole frames on screen.
- The owner-screen Mac walk of the route.
- Agent visual review of the full exit scene.
- Device runs beyond this M3 Max.
