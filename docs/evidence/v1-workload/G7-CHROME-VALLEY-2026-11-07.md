# G7 cooked valley in Chrome — 7 November 2026

Chrome now draws the authored cooked valley, not only the diagnostic scene. This is the first slice of
[the build plan](../../BUILD-PLAN.md)'s G7 item: native and web share the scene. The page fetches one
publication from the G5 dev server and draws it with the same placements, camera, sky, shadows and exposure as
native. Chrome fetches Portable products. The native store can select compressed variants.

![Chrome 156 WebGPU on the M3 Max. Sunrise at the authored framing, sunset, twilight at the authored framing,
and noon looking up](g3s-first-light/chrome-valley.png)

## Change

- **Product source.** `thrum-asset-gpu` takes a `CookedSource`. It is either a store path, followed by the native
  loader thread as before, or one `RemoteGeneration` fetched by G5's `DevClient`.
- **Browser preparation.** The browser has no loader thread. A fetched generation is prepared on the calling
  thread when the contribution starts, then posted to the same mailbox. Staging, the per-frame copy budget,
  swap, culling, draw and Sun casting are unchanged. A new device generation prepares it again.
- **Game.** `fetch_cooked_valley(origin)` and `BrowserGame::attach_cooked_valley` load and attach the embedded
  showcase recipe. `index.html?assets=<origin>` does both before the first frame. The showcase recipe and
  `serde` now build for wasm32. The environment product stays native.
- **Check.** `scripts/check_v1_browser.mjs --case valley` cooks `examples/v1-game/assets/showcase` into a fresh
  store with `thrum-cook watch --serve`. It then runs the sky-reflections procedure against it (seven held
  phases and 24 camera and Sun samples) and requires the cooked generation to be drawn with no loader errors.
  This is the dev server's first Chrome run with an `Origin` header, so its CORS path is now exercised.

## Checks

- **Equivalence.** A Metal test fetches a generation from `DevServer` over loopback HTTP. It draws that
  generation at the same world time as the mapped store with Portable selected. No pixel differs by more than
  12 codes in any RGB channel. The fetched path uses the device's default preferences with BC compression enabled.
- **Handback recheck.** The Metal fetched-generation and edit cases passed. Chrome's `valley` and
  `valley-recovery` passed with Wasm SHA-256
  `172060dec8af3d7c432e5597ff81bb75e39e35f84f47905374b8adb5e81f3427`. The edit uploaded 5,376 bytes
  after the initial 1,523,712-byte upload. Two real device replacements each restaged the valley.
  The first valley run passed its scene assertions and timed out during Chrome shutdown. The repeat exited cleanly.
- **Chrome.** The `valley` case passed: generation 1 drawn after one swap, 1.52 MB uploaded, no loader errors.
  Wasm SHA-256 `81dba38fab05cf39e26f58e94eeb63dde7545d9be05f1c0c720f96a6c720df12`. The existing
  `sky-reflections` case passed with the same Wasm.
- **Repository.** `./dev check` passed.

## Live edits

![Chrome, paused world at noon. Left: generation 1. Right: after retinting the shared base-colour texture
in the watched source](g3s-first-light/chrome-live-edit.png)

- **Feed.** `CookedSource::Remote` now takes a `RemoteFeed`. `RemoteFeed::publish` hands in a newer fetched
  generation, and the contribution prepares it on the render thread at its next upload. It reuses every product
  the drawn set already holds and swaps in the new set through the same staging. A replacement device prepares
  the newest generation.
- **Page.** It long-polls the dev server with `CookedValley::next_publication`. Overlapping polls are refused,
  and failures are logged and retried after one second.
- **Chrome check.** The `valley` case now watches a copy of the showcase source. After its captures it retints
  `white.png` and requires generation 2 to be drawn with no errors, after exactly one more swap, uploading
  less than the first set. Result: 5,376 bytes uploaded against 1,523,712, with the watcher publishing 43 ms
  after the edit. The swap happened with the world paused.
- **Metal test.** A second publication through the feed swaps in with a smaller upload than the first, the
  dev client reuses the unchanged mesh, and the edit is visible.

## Device recovery

![Chrome after two GPU device replacements with the cooked valley attached](g3s-first-light/chrome-valley-recovered.png)

`check_v1_browser.mjs --case valley-recovery` runs the existing device-recovery procedure with the valley
attached. It destroys the actual `GPUDevice` twice and restores it with the real button, on the same world.
Each replacement must prepare the fetched generation again. Result: generation 1 still drawn, swaps 1 → 3, each
restage a full 1,523,712-byte upload, frames submitted 2 → 107, and no loader errors. `cooked_status` now
reports submitted frames. With Wasm SHA-256
`150993227878c5f7cd05d0f2af06a7ea1d2d41639f918ed9cd5f6be8992e5ae4`, the `valley`, `sky-reflections` and `device-recovery`
cases also passed, as did `./dev check`.

## Independent review

A fresh-context reviewer saw only Chrome and native stills and a 24-sample Chrome motion tile. It found:
- **Parity.** Chrome matches native at the authored framing: terrain, sphere, rock, sky colour and haze. There
  is no missing geometry and no corrupt region.
- **Lit floor turning grey in one step (samples 4–5).** The samples are 17.67 h and 17.73 h, 3.4 minutes of
  solar time apart. Native half-minute captures show the ridge shadow crossing the floor between 17.67 h and
  17.70 h, so this is sparse sampling of a real crossing, not a Chrome defect.
- **Saturated orange glow and a hard step at the near-floor silhouette (Sun-facing presets).** Native
  `sunset-sun` at the same time and direction shows both identically. They were shared look items: in-scatter
  over shadowed distant terrain. Fixed by [shadowed aerial in-scatter](G3S-SHADOWED-AERIAL-2026-11-07.md).
- **Noon preset.** It pitches up to the sky. The reviewer read it as dull and noted faint column banding and
  ridge fringes. The banding was 8-bit contouring, now dithered
  ([evidence](G3S-DISPLAY-DITHER-2026-11-07.md)). The fringes are a smooth crest fall-off, not a defect.

## Edit and replacement timing

Instrumented headless Chrome WebGPU on the shared M3 Max, with the paused cooked valley.
Preparation runs from the fetched-generation handoff to filled staging. Callback time is
`game.frame` wall time. Values are ranges from repeated cases.

| Swap | Bytes uploaded | Prepare (ms) | Game callback (ms) |
|---|---:|---:|---:|
| Initial set | 1,523,712 | 2.1–2.8 | 4.8–5.3 |
| Shared texture edit | 5,376 | 0.2–1.6 | 0.6–2.2 |
| Terrain mesh edit | 1,457,968 | 0.7–1.0 | 1.2–1.9 |
| Replacement restage | 1,523,712 | 0.6–0.8 | 4.3–6.0 |

The next animation callback came after 16.6–16.7 ms at every sampled swap. Keep the
current scheduling for this workload. End-to-end startup, edit-to-visible time and
physical presentation cadence remain open. Raw timestamps, load and harness are in
`tmp/handback-review/browser-prepare/`.

## Showcase launch and fixed review route

[The showcase route decision](../../design/decisions/G7-SHOWCASE-ROUTE.md) adds a local launch and a fixed route
for reviewing the cooked valley. Six paused presets in `examples/v1-game/params/showcase-*.params.json` hold the
stops. `scripts/run_v1_showcase.py` launches either target at one stop. Its `check` command proves both targets
drew the cooked valley rather than the diagnostic scene, and compares each stop with a checked-in baseline. Since M2
each stop's frame comes from applying its preset on both targets: native capture with `THRUM_CAPTURE_PARAMS`, and
Chrome through the checker's `route` case, one fresh page per preset.

**Build and input.** Each check run cited below built its own binaries from a clean tree (`built_by_this_run`,
`dirty: false` in each `run.json`). All are in `tmp/g7-showcase/runs/`.
- **M2, preset-driven frames:**
  - check-163503 at c2e1ff45 saved the current baseline.
  - check-163650 at 4101c3a5, the baseline commit, compared against it.
  - check-164639 rebuilt after the rebase on 6fa34365 and the review fixes, at dbcde616, and compared against it.
  - These local commits were later rebased, and the commits after them change only the checker's preset
    expectation, the script's baseline handling and documentation, not game, cook or web code.
  - Binaries: release `v1-game` `d8986d4f…` (with the capture seam) in all three, `thrum-cook` `e2fbdcfd…` and
    default-page Wasm `46d3eb04…`.
  - The drift demonstration (check-164805, -164859 and -164948) ran with `--no-build` on those binaries from the
    clean dbcde616 tree.
  - Not cited: check-163312 ran on a dirty tree before the seams were committed, with an earlier game binary
    (`ff8fa6f0…`).
- **M1, matched poses:** check-161058, check-161243 (the first baseline), check-161350 (against it), mac-161439 and
  chrome-161454.
  - These used `v1-game` `6855e94d…` and the same cook and Wasm.
  - Their `run.json` files name local commits from before M1 was squashed and rebased. Between those and the landed
    M1 commit, only this script, its baseline and documentation changed.
  - check-161940 reran with `--no-build` while the source-root guard was an uncommitted edit, and passed.
  - Not cited: check-160935 was the calibration run, and check-161144 ran with an uncommitted script edit.
- **Source:** the valley with the authored ground from 80e519ec (`ground.png` `ff297966…`). Its generation-1 root is
  `59f0484f…` in every run.

The M3 Max GPU was shared with other agents throughout.

![Native (left) and Chrome (right), each from its preset, at sunrise, noon, the 17.4 h clear Sun, 17.9 h sunset,
twilight and night looking upslope](g7-showcase/route-mac-vs-chrome.png)

| Check | Result |
|---|---|
| Chrome against native | At all six stops, the Chrome frames differ from native by at most 0.027 (median), 0.061 (95th percentile) and 0.160 (maximum cell). These are mean absolute 8-bit RGB values over a 32×18 grid. The thresholds are 1.0 each, unchanged: the comparator and source root are the same, and the numbers did not move. The 17.4 h clear-Sun stop, Mac-only in M1, reads 0.026, 0.039 and 0.092. At full size it shows a crisp Sun disc, about 10 pixels across, above the ridge on both targets; the disc is too small to see in the contact sheet. |
| Same input | The native store's generation-1 root is in both checker stores. Every page's loaded Wasm equals this run's build. Each Chrome stop's preset hash equals the repository's. The page reported nothing refused, and the native capture accounted for every preset key. Every pose equals its preset on both targets, and every Chrome stop held one paused tick through its capture. |
| Negative control | The diagnostic scene differs from native by 9.2–39 (median), 30–70 (95th percentile) and 92–117 (maximum) at the five stops with a diagnostic phase: more than ten times a threshold at each. The checker has no diagnostic phase at 17.4 h, so that stop has no negative control. |
| Positive control | The checker's frames before and after its texture retint, cropped to the canvas, differ by 34.4 in the maximum cell. The retint now reaches only the sphere and rock, because the ground has its own texture, so the median and 95th percentile stay at 0. The maximum cell is the statistic that catches a missing or wrong object. A wrong ground texture is caught by the Chrome-to-native comparison, not by this control. |
| Repeatability and baseline | The three clean M2 runs pass with byte-identical stop frames on both targets. Each differs from the baseline by at most 0.005, the stored rounding, over all 12 frames. M1's three clean runs, on the matched poses, agreed in the same way. |
| Drift is reported | Against a scratch baseline with one 8-cell patch of Chrome noon raised by 6 levels, `check --baseline` fails and names `chrome/noon` (maximum 6.002). `--save-baseline` then replaces that scratch baseline, recording the drift it replaced, and the next run passes within 0.005. The checked-in baseline was not touched. |
| Native capture seam | Tested on the committed, rebased binary `d8986d4f…` (`tmp/g7-showcase/seam/rebased/`). Without `THRUM_CAPTURE_PARAMS`, all 134 output files are byte-identical to the pre-seam binary's capture of the same store. With it, the other 133 files are unchanged too, and `captures.json` differs only by its `route` key. With the six presets, four route frames byte-equal the matched-pose captures. The 17.4 h and 18.48 h frames differ by at most 28 levels on about 1,000–2,600 pixels near the Sun. That comes from the controls snapping solar time to the 0.01 h slider stop (17.4 h becomes 17.400002 h), which Chrome applies too. Each of these fails the capture with its reason: a missing file, a file that is not JSON, a refused key (the message names it and suggests the right one), a preset that does not pause the world, two presets with one output name, and an empty list. |
| Checker route case | Each of the six checked-in presets launches a fresh page with `?assets` and `?params`, and the six pass in 7–12 s per run. The web build must ship each preset byte for byte and no others: a missing, a stale and an extra built preset each fail before any page loads. A typo in a checked-in preset is not shown on the current code: before the hold, a built preset with a typo failed its stop on the page's 0-refused check, which is unchanged, and the script's own key check refuses such a preset before any capture. G3c's 16-view field oracle, rerun after both changes, is byte-equal across Direct and Frustum. |
| `mac --stop sunset-clear-sun --frames 240` | Exit 0. The preset applied (4 changed, 0 refused) and the window presented 240 frames with no surface faults. The window was briefly occluded (5 occluded, 5 dropped surface frames). `run.json` records the store and scene. The window was not watched on screen. |
| `chrome --stop twilight --no-open` | The printed URL, opened in the desktop app's browser pane, showed the paused twilight valley with the authored ground (`pane-observation.jpg` and `.json` in the run). The preset applied (3 changed, 1 unchanged, 0 refused). The server's `requests.log` shows the page fetching the preset, the scripts and the Wasm, and the Wasm the page served equals the build. Stopping the launcher stopped both servers. |

**Comparator.** The first comparator assigned whole pixels to grid cells. That moved cell edges by up to a pixel
between Chrome's 1252-pixel and native's 1280-pixel frames, and it produced most of the measured difference: a
maximum of 2.15 against 0.148 with exact area coverage on the same frames. The independent review found that a
one-pixel shift brought the first comparator within 5% of failing. Comparator 2 weights each pixel by its exact
area share. A one-pixel shift of native now matters: shifted diagonally, it reads up to 1.24 (95th percentile)
and fails, and shifted vertically it fails at every stop. Shifted horizontally only, it fails at noon (maximum 1.94)
but passes at the sunset and night stops (maximum 0.60 and 0.53), where little detail runs across the view.
Thresholds were recalibrated by the design's rule for comparator 2 on this source
root, and every value fell to the 1.0 floor. The script refuses those thresholds on any other source root, so a
content change brings a reviewed recalibration rather than silently absorbing a parity change.

**Baseline.** `g7-showcase-baseline.json` (52 KB) holds the per-stop grids for both targets. Each grid is stored as
uint16 hundredths of an 8-bit level. The file also records:
- the identity: comparator version, frame-source version, source root and preset hashes;
- the build;
- the thresholds;
- how to reproduce and replace it.

`check` compares against the baseline only when its identity matches. Otherwise it reports the mismatch.
`check --save-baseline` replaces it, after a passing run whose frames have been reviewed. The first baseline was
run `check-20261107-161243`, on the matched poses. M2 replaced it with `check-20261107-163503`, reviewed frame by
frame, because the frames now come from the presets. That is recorded in the identity as frame source 2, so the old
baseline reported a different identity rather than a drift. The new baseline gains the Chrome 17.4 h stop. The
thresholds are unchanged.

**Texture profiles.** Chrome fetches Portable products. Native uses the device's preferred profile, Apple ASTC on
this GPU, by `texture_profiles`. That is read from the code; the run does not log it. Root kept this intended
difference, and parity holds across it.

**Timing, kept apart.** These are proxies on a contended GPU, not windowed startup. The ranges cover the ten cited
check runs: M1's 161058, 161243, 161350 and 161940; M2's 163503, 163650 and 164639; and the drift runs 164805, 164859
and 164948. Build time covers only the six that built.
- Cook: 0.06–0.60 s with warm binaries.
- Native capture launch to its first capture file: 0.11–2.73 s, offscreen, including one readback.
- Chrome navigation to the first observed cooked draw: see M3 below.
- Build time (warm, incremental): 1.2–8.1 s.
- Scripted capture cost: the native capture run took 1.5–6.1 s. The route case took 7.1–11.8 s for six fresh pages,
  over the six M2 runs. The valley and diagnostic checker cases took 15–31 s each. This is harness time, not frame
  time.

**Still open for G7.**
- The on-screen walk of the route on the Mac waits on screen control.
- Renderer findings from the review are listed in the design's "Still owed for final G7".
- Both stay with root. The Chrome startup proxy and the edited-source run are M3, below.

### M3: Chrome startup proxy and a route run from edited source

Root commissioned these two screen-independent proofs. Every run below is on commit f38166f0, which is main
6459379d plus the M3 commits; a later main commit, 00d34bfb, changed only documentation. The tree was clean in each
run. These runs record `origin_main`, the main ref at run time, which reads 00d34bfb in all but the first. Their
main revision is the merge base, 6459379d; the script now records it as `main_base`.

The commits after f38166f0 change only documentation and three script details. The startup scope wording is fixed.
`--recalibrate` now refuses the calibrated root, and a refused canonical save no longer leaves a run directory. The
checker's scope and usage wording is fixed too. These details do not change any number below, and the integration
runs at the end reproduce the proofs on current main. The canonical control check built the binaries, and the other runs used them with `--no-build`:
- release `v1-game` `5c0b7953…`;
- `thrum-cook` `e2fbdcfd…`;
- default-page Wasm `9405ae36…`.

The game binary and Wasm differ from M2's because main changed simulation code. The M3 Max GPU was shared with other
agents throughout, so the timings are contended. All runs are in `tmp/g7-showcase/runs/`, and the checker seam tests
are in `tmp/g7-showcase/m3/seam-final/`.

**Canonical control.** check-174420 passes on the checked-in valley and baseline:
- root `59f0484f…` and the fixed thresholds;
- all 12 frames within 0.005 of the baseline;
- every stop frame on both targets byte-identical to M2's check-165459.

**Chrome startup proxy.** `startup --runs 5` (startup-175135) ran the checker's route case five times.
- **Launch:** each launch is a fresh headless Chrome process and profile on hardware WebGPU (Metal). The game is
  already built, and the cook server serves generation 1 before navigation.
- **Cache:** the HTTP cache is disabled for each page. Every page transferred the full 2,056,659-byte Wasm.
- **Pages:** each launch opens the six stops in turn. The first page is the cold one: a fresh browser process and
  profile.
  - The OS file cache and any GPU shader cache outside the profile stay warm.
  - The route order makes the cold page always the night stop, so cold and stop are confounded.
  - The other five pages share the browser and GPU process and are warm.
  - The observer's polling and its search for the game object add to every bound.
- **Clock and interval:** each page is timed on its own clock, `performance.now()` from navigation start. The first
  cooked draw lies between the last observation that proves no draw and the first that sees one. A proof of no draw
  is either the page's exact pre-loop label (`Starting WebGPU…`, `Fetching the cooked valley…`) or a present
  `cooked_status()` with no generation drawn. An observation that proves neither leaves the lower bound absent, and
  navigation start is then the only bound.

Lower–upper bound in ms, raw:

| Page (order) | Launch 1 | Launch 2 | Launch 3 | Launch 4 | Launch 5 |
|---|---|---|---|---|---|
| 1 night-upslope (cold) | 157–169 | 117–162 | 112–152 | 137–173 | 127–175 |
| 2 noon | 116–124 | 106–122 | 113–128 | 63–123 | 104–114 |
| 3 sunrise | 63–130 | 70–169 | 36–164 | 112–122 | 78–131 |
| 4 sunset-clear-sun | 116–129 | 112–138 | 114–130 | 63–118 | 114–125 |
| 5 sunset-sun | 108–120 | 126–134 | 76–124 | 84–124 | 65–122 |
| 6 twilight | 107–120 | 62–108 | 126–140 | 100–116 | 55–113 |

- Cold upper bound: 152–175 ms, median 169 (n 5). Warm: 108–169 ms, median 124 (n 25).
- Granularity (upper minus lower): 8–128 ms, median 31. Every page had an observed lower bound.
  - The gap is widest at the third page (sunrise) and wherever the page's main thread was busy attaching the valley.
    Each observation goes through CDP and waits for that thread.
  - The checker's seam tests show both bound shapes. A normal run's six pages had gaps of 15–61 ms. A test run with
    the first observation delayed by 3 s recorded all six lower bounds as absent, with no granularity.
  - Earlier during review, one undelayed page was first seen already at Running and was recorded as absent.
- Where the time went (medians over the 30 pages):
  - navigation response at 2 ms;
  - Wasm loaded by 21 ms;
  - `Fetching the cooked valley…` first seen at 66 ms, over the 27 pages where it was observed;
  - the cooked products fetched from 65 to 78 ms;
  - `Running…` first seen at 116 ms.

  The cooked server is cross-origin and sends no Timing-Allow-Origin, so its transfer sizes are hidden. The page
  reports 1,882,112 bytes uploaded.
- **Same artifacts:** every page loaded this build's Wasm and the repository's preset bytes, and all five launches
  cooked root `59f0484f…`.
- **Kept apart, not counted:**
  - build: 22.3 s, in check-174420, after main's simulation change;
  - cook: 0.4 s in that check;
  - each launch's checker run, with its own cook and six pages: 6.7–7.6 s.
- **Scope:** a headless CDP polling proxy for navigation to the first observed cooked draw, from one contended
  session. It is not physical first-paint latency.

**Edited source.** `tmp/g7-showcase/m3/retint.py` copies the valley and scales the ground albedo `ground.png` by
(1.0, 0.55, 0.45), turning the ground red-brown (`ff297966…` to `e732b3ff…`). Nothing else changes. The checked-in
scene, assets and baseline are the control and were not touched.

![Per stop, left to right: canonical native, edited native, edited Chrome, canonical Chrome](g7-showcase/route-edited-source.png)

The sheet is built from check-174420 (canonical) and check-174724 (edited) by `tmp/g7-showcase/m3/sheet_edit.py`,
with rows in route order: sunrise, noon, the 17.4 h clear Sun, 17.9 h sunset, twilight and night upslope.

| Check | Result |
|---|---|
| New source identity | `check --source tmp/g7-showcase/m3/edited-source` cooks generation-1 root `49041ec0…`, found in the native store and both checker stores. |
| Fixed thresholds refused | Without `--recalibrate`, check-175035 fails: the thresholds were calibrated on `59f0484f`, not `49041ec0`. |
| Recalibrated parity | With `--recalibrate`, check-174724 passes. The binding facts are that the rule lands on the 1.0 floor again (three times the worst valley is below 1), the same values as the fixed thresholds, and that both controls pass. Chrome differs from native by at most 0.033 (median), 0.175 (95th percentile) and 0.281 (maximum). The controls: the diagnostic scene differs by 7.3–27 (median), 28–71 (95th percentile) and 85–132 (maximum) at the five stops with one, and the positive control's maximum cell reads 34.4. |
| Canonical baseline refused | The checked-in baseline reports `other identity ['source_root']` and is not compared. |
| Changed region, both targets | As information only, the edited frames differ from the canonical baseline at each stop by up to 4.9–20.7 (median), 23–32 (95th percentile) and 25–35 (maximum). At night the median is 0.004 and the 95th percentile 9.2, because the ground is dark. Native and Chrome agree on these numbers to within 0.12. In the sheet the ground is red on both edited columns, and the sky, sphere and rock match the canonical frames. |
| Scratch replacement | check-174832 saved `tmp/g7-showcase/m3/edited-baseline.json` after a passing run, with its frames reviewed in the sheet. The rerun, check-174940, compares against it: all 12 frames within 0.005. |
| Canonical save refused | `--recalibrate --save-baseline` aimed at the checked-in baseline refuses before any work. |

**Checker seam.** Root reviewed the route-specific startup observation in five rounds and approved 10ea0392
(`tmp/g7-boundary/m3-checker-startup-v5.diff`); the rebase did not change it. On f38166f0:
- the `route`, `params`, `controls` and delayed-observer runs passed;
- G3c's `field-oracle` was 16/16 byte-equal.

`route` and `field-oracle` first failed after every page and view had passed: Chrome exited with code 2 during the
checker's shared cleanup, with display-link errors in its log. Both passed on rerun. The failed runs are kept beside
the passing ones.

**Integration on main 935ddd14.** Main then gained G3c's culling-count changes, which touched the browser and
window hosts and the shared checker. The M3 commits were rebased onto them, and the checker seam's own lines are
unchanged. Root approved the rebased patch (`m3-checker-startup-v6-on-935ddd14.diff`). On the rebased commit 01cb799d,
with a clean tree and `main_base` 935ddd14:
- check-180843 built the binaries (`v1-game` `e5ae52df…`, Wasm `cd2bdb81…`) and passed against the canonical
  baseline. Its 12 stop frames are byte-identical to check-174420's.
- check-181018 ran the edited source with `--recalibrate` and passed. Its 12 frames are byte-identical to
  check-174724's.
- startup-181110 (one launch) passed: cold upper bound 159 ms, warm 103–121 ms.
- The checker's `route`, delayed-observer, `field-oracle`, `params` and `controls` cases passed, and `./dev check`
  passed (`tmp/g7-showcase/m3/integrated/`).
- Smoke runs on 002865b7 showed the new refusals: `--recalibrate` on the calibrated root fails after the cook,
  and a refused canonical save leaves no run directory.
