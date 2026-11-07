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
drew the cooked valley rather than the diagnostic scene, and compares each stop with a checked-in baseline.

**Build and input.** Each run cited below built its own binaries from a clean tree (`built_by_this_run`,
`dirty: false` in each `run.json`):
- Runs: check-161058, check-161243 (the baseline run), check-161350 (against the baseline), mac-161439 and
  chrome-161454, all in `tmp/g7-showcase/runs/`. check-161940 then reran the check with `--no-build`, reusing the
  same binaries (identical SHA-256s), while the source-root guard was still an uncommitted script edit; it passed. Their `run.json` files name the
  local commits before this work was squashed and rebased. The commits between them and the landed commit change
  only this script, its baseline and documentation, not game, cook or web code.
- Not cited: check-160935 was the calibration run, and check-161144 ran with an uncommitted script edit.
- Binaries: release `v1-game` `6855e94d…`, `thrum-cook` `e2fbdcfd…` and default-page Wasm `46d3eb04…`, the same in
  every run.
- Source: the valley with the authored ground from 80e519ec (`ground.png` `ff297966…`). Its generation-1 root is
  `59f0484f…`.

The M3 Max GPU was shared with other agents throughout.

![Native (left) and Chrome (right) at sunrise, noon, the Mac-only 17.4 h clear Sun, 17.9 h sunset, twilight and
night looking upslope](g7-showcase/route-mac-vs-chrome.png)

| Check | Result |
|---|---|
| Chrome against native | At the five matched stops, the Chrome frames differ from native by at most 0.027 (median), 0.062 (95th percentile) and 0.161 (maximum cell). These are mean absolute 8-bit RGB values over a 32×18 grid. The thresholds are 1.0 each. |
| Same input | The native store's generation-1 root is in the checker's own cooked store. The Wasm Chrome loaded equals this run's build. Every capture's pose equals its preset on both targets, and Chrome's phases were recorded paused. |
| Negative control | The diagnostic scene differs from native by 9.2–39 (median), 30–70 (95th percentile) and 92–117 (maximum) at every stop: more than ten times a threshold at each. |
| Positive control | The checker's frames before and after its texture retint, cropped to the canvas, differ by 34.4 in the maximum cell. The retint now reaches only the sphere and rock, because the ground has its own texture, so the median and 95th percentile stay at 0. The maximum cell is the statistic that catches a missing or wrong object. A wrong ground texture is caught by the Chrome-to-native comparison, not by this control. |
| Repeatability and baseline | The three clean runs pass with the same stop frames. The run against the baseline differs from it by at most 0.005, the stored rounding, over all 11 frames. |
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
- the identity: comparator version, source root and preset hashes;
- the build;
- the thresholds;
- how to reproduce and replace it.

`check` compares against the baseline only when its identity matches. Otherwise it reports the mismatch.
`check --save-baseline` replaces it, after a passing run whose frames have been reviewed. The first baseline is
run `check-20261107-161243`, reviewed frame by frame.

**Texture profiles.** Chrome fetches Portable products. Native uses the device's preferred profile, Apple ASTC on
this GPU, by `texture_profiles`. That is read from the code; the run does not log it. Root kept this intended
difference, and parity holds across it.

**Timing, kept apart.** These are proxies on a contended GPU, not windowed startup, over the five post-rebase runs:
- Cook: 0.25–0.54 s with warm binaries.
- Native capture launch to its first capture file: 0.34–2.73 s, offscreen, including one readback.
- Chrome navigation to cooked content drawn: not measured. The checker logs no timestamps, so this waits for the
  checker route seam.
- Build time (warm): 1.2–1.8 s.
- Scripted capture cost: the native capture run took 1.7–4.2 s, and each checker case 16–20 s. This is harness time,
  not frame time.

**Still open for G7.**
- The on-screen walk of the route on the Mac waits on screen control.
- The 17.4 h stop is Mac-only until the checker route seam lands.
- Preset-driven native capture is the other approved seam.
- Renderer findings from the review are listed in the design's "Still owed for final G7".
