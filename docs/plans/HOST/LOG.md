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
