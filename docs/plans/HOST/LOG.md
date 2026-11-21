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
