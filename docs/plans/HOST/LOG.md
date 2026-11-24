# HOST log

- 2026-11-14 Planned from two requests: the code review's theme T4, and the owner's "let's separate out the game
  as much as we can", for which engine host code leaves the game. Decided: the browser keeps today's request as
  a named portable profile, so no Chrome baseline moves; pause and overload policy goes to `thrum-core` beside
  the clock; the iPhone app's UIKit move stays `GAME-UI` M6's.
- 2026-11-15 Revised after [the first design review](../../../.orchestrate/evidence/HOST/design-review.md) (one Must Fix). The
  drift table names the milestone that closes each row, M2.3 judges only M1's and M2's rows, and the render
  owner's frame job and reply (review finding M03) are shared in M4 with a new item, M4.5. Settled with
  `TEST-SUPPORT`: the recipe takes an optional restriction, which only lowers limits and removes features, for
  tests that need a reduced device. A recorded uncaptured error leaves C07's generation reset as it is.
- 2026-11-19 Built M1's shared device request, fault watch and SDR surface configuration.
  Verified native limits and restriction refusal on Metal, loss and recovery in the Mac window and
  Chrome, and cooked ASTC engine output through rotation and foregrounding in the iPhone simulator.
  The registered image baselines are covered by the shared main-branch check. [Evidence](../../../.orchestrate/evidence/HOST/M1-proof.md)
- 2026-11-19 Reran M1.4 from clean main at `a319249fb` with the display held awake. All three Mac window
  rows and all eight Chrome cases passed. [Evidence](../../../.orchestrate/evidence/HOST/M1-proof.md#m14-mac-window-and-chrome-recovery)
- 2026-11-19 Addressed the M1.1 and M1.2 review findings. Verified the measurement entry points and recipe limits
  on Metal, the expanded device-request guard, and the starter and V1 WebAssembly builds.
  [Evidence](../../../.orchestrate/evidence/HOST/M1-proof.md#m11-one-request)
- 2026-11-19 Fixed M1.2's native device-generation finding. Mac capture and UIKit use the engine's first
  generation, and capture recovery starts from the opened device. Both Metal checks and both native
  compile checks passed. [Evidence](../../../.orchestrate/evidence/HOST/M1-proof.md#m12-generation-review-fix)
- 2026-11-19 Built M2's shared world lifecycle and surface acquire policies. Verified the resume race,
  overload recovery and device replacement on the named targets. The original iPhone route passed through
  background, foreground, rotation and device loss. The simulator evidence covers function.
  [Evidence](../../../.orchestrate/evidence/HOST/M2/README.md)
- 2026-11-20 Addressed M2's review findings. The resume-race test fails without its guard, browser replacement
  reconfigures a lost drawable before reporting persistent failure, and offscreen uses the shared lifecycle policy.
  On Mac Metal the offscreen driver recorded one overload, resumed and submitted later frames. The Mac extent
  switch and recovery use the same scoped configure operation. [Evidence](../../../.orchestrate/evidence/HOST/M2/README.md#review-fixes)
- 2026-11-20 Planner, from the cleanup review (L8-05): the checks of M3.2 and M4.5 used `! grep` on one path, which passes when that path has moved. Each now counts across `crates` and `examples` and passes only when exactly one definition is left (the engine's `FrameHost`; `thrum-app`'s `SAMPLE_SLOTS`). Neither statement changed. Both milestones were Not Started.
- 2026-11-20 Restored the original Chrome recovery check for a single canvas configure exception. The
  page exposes Restore after the rejected configuration, and a separate acquire test verifies a
  transient loss recovers on the same candidate. Both cases passed in Chrome. [Evidence](../../../.orchestrate/evidence/HOST/M2/README.md#m24-preserve-restore-after-one-configure-exception)
- 2026-11-21 Built M3. The game, the iPhone app and the benches hold `thrum_render`'s `FrameHost<GameWorld, GameScene>`;
  the forwarding host is gone and `GameHost` holds the game's own controls. `render_once` draws without a readback
  and `await_drawn` is the one cooked wait (90 s, typed errors). Scene capture moved from `src/bin/window/` onto the
  offscreen driver, with settings parsed in the binary. On Mac Metal the item checks passed, engine capture ran
  still, fixed and realtime, and the APPLE-TILE native matrices matched byte-exact. [Evidence](../../../.orchestrate/evidence/HOST/M3/)
- 2026-11-21 Fixed M3's review Must Fix: the sky kit's cooked-content wait had its own 30 s deadline. The one wait is now
  `thrum_asset_gpu::await_content`, a draw step and a content probe under `DRAWN_DEADLINE` and one failure rule.
  `await_drawn`, the kit's `ContentStatus` wait and the native-witness route preparation all use it. On Mac Metal the
  wait tests, the kit's sky tests, the starter's live reload (10 saves) and still capture, and a Metal-witness route
  capture passed. [Evidence](../../../.orchestrate/evidence/HOST/M3/)
- 2026-11-23 Built M4's hosted game contract, browser control binding, caller-supplied Mac qualification
  observer and shared render-owner frame request and reply. V1 and the sky kit use the shared preparation
  path. Verified the starter on Mac Metal and in Chrome, V1's Mac held-drawable recovery and overload,
  all eleven V1 Chrome lifecycle cases, and the simulator's G3b route and overload. The Mac collector
  produced capture-on grid replacement and
  capture-off valley records. The registered images are covered by the shared main-branch verifier.
  [Evidence](../../../.orchestrate/evidence/HOST/M4/README.md)
- 2026-11-23 Fixed the merged iOS build by compiling the route render-scale helper only for its Mac and WebAssembly
  callers. The full exact-image verifier passed on `f582a2af1214`, which contains the initial HOST M4 merge. The
  verifier bisected the later `apple-tile-native` failure on main to APPLE-PRESENTATION M5 (`32dfd3c14a0f`).
  [Exact-image records](../../../.orchestrate/evidence/HOST/M4/README.md#m47-registered-exact-images)
- 2026-11-23 Fixed the M4 review findings. The offscreen driver applies frame edits and observes readback.
  Games receive an engine-owned physical key on Mac and Chrome, and the shared page runtime owns control
  edits, frame callbacks, visibility and recovery. The simulator command reuses only an ancestor's lock.
  Verified changing offscreen readback on Metal, a plain Game in Chrome through device replacement,
  and the starter and V1 Chrome cases. [Review-fix evidence](../../../.orchestrate/evidence/HOST/M4/review-fixes/README.md)
- 2026-11-23 Fixed recovery replay on the offscreen and browser hosts and made the shared browser driver
  refresh recovery controls independently of frame callbacks. Metal and Chrome each applied a replay-only
  edit before the first replacement submission. In Chrome, an ordinary game using only the shared recovery
  button exposed it after real device loss and presented on generation 2 after a click.
  [Device replacement evidence](../../../.orchestrate/evidence/HOST/M4/recovery-fixes/README.md)
- 2026-11-23 Matched the replay helper's target predicate to its macOS offscreen and Wasm browser callers.
  The iPhone simulator's strict Clippy check and release build passed with the helper excluded.
  [Target reruns](../../../.orchestrate/evidence/HOST/M4/target-checks/README.md)
- 2026-11-24 Reverified the Metal game contract, the starter on Mac and in Chrome, and the iPhone 18 Pro
  simulator route and overload recovery using the revised check. The full Mac and Chrome exact-image verifier
  passed on `d30c839b236f`, covering the merged HOST fixes. The current V1 Mac witness built and reached a
  locked console before recording. Its window evidence comes from the prior target pass.
  [Target evidence](../../../.orchestrate/evidence/HOST/M4/resumed/README.md)
- 2026-11-24 The merged check passed the Mac window and overload cases before Chrome lost its copy of
  the Wasm file in the checkout build cache. The lifecycle runner now keeps both web pages and its cooker
  in the run directory, where the browser can compare the served Wasm with the same file at case completion.
  [Cache-removal regression and target runs](../../../.orchestrate/evidence/HOST/M4/isolated-web/README.md)
- 2026-11-24 The merged M4.6 check passed the Mac window and overload cases. Chrome's normal case completed
  all 600 GPU samples before its shutdown watchdog fired, and its retry stayed hidden. The lifecycle runner
  now wakes the display, waits for the WebGPU document to unload, and verifies that the owned Chrome process
  group exits. It records the watchdog separately from a graceful exit after a passing scenario. All eleven
  hardware Chrome cases passed with the original verifier's Wasm and cooker.
  [Target evidence](../../../.orchestrate/evidence/HOST/M4/chrome-shutdown/README.md)
- 2026-11-24 Restored M4.6's clean Chrome exit requirement after review. The harness retires the game's
  WebGPU devices before unloading its page and records watchdog diagnostics on failure. Real Chrome
  completed the recovery-retry scenario in both exit tests. The harness rejected an injected watchdog
  exit 2 and accepted a clean exit 0. The full Mac and Chrome command passed with eleven clean Chrome
  exits. [Exit regression and target receipts](../../../.orchestrate/evidence/HOST/M4/strict-shutdown/README.md)
- 2026-11-24 Replanned WP5.3 after orchestrate 3.0.6 refused the main-check query inside the item runner.
  The item now checks the reference pins. The shared `exact-images` run compares rendered output on main.
  HOST WP5 waits for the REPO-RULES WP5 plan-reader repair at merge.
- 2026-11-24 Built WP5's typed game, shell, sky kit and capture writer errors. The Mac string-error check
  and error-type tests passed, along with the Metal app suite and strict Wasm and iPhone simulator-target
  Clippy. The branch also passes the exact-baseline reference guard. WP5.3 remains open because
  orchestrate 3.0.5 refuses its main-branch query in the item runner. The repository check also fails on
  clean main because its plan reader expects the pre-migration schema.
  [Proof](../../../.orchestrate/evidence/HOST/WP5/README.md)
  [Handoff](../../../.orchestrate/evidence/HOST/WP5-handoff.md)
- 2026-11-24 Ran the replanned WP5.3 check on the Mac after rebasing onto the updated plan. The
  reference guard verified all five registered Mac and Chrome baseline pins.
  [Reference guard](../../../.orchestrate/evidence/HOST/WP5/WP5.3-reference-guard.log)
