# PLATFORM-RUNTIME-IOS log

- 2026-11-09 Built M7. Shared scene/surface resize prepares a replacement ring and presenter, validates the
  current epoch before publication, drains pending frames, reuses identical extents and preserves fixed-render
  surface-only callers. Device candidates refit scene and presenter, then refresh current camera/lighting at
  publication. Contributions retain size-independent pipelines and receive C07 history reset with the new matrix.
  Full pre-change and final fresh-simulator routes passed. Final rotation has two-reply gaps, unchanged identities,
  zero old-extent presents, matrix-derived 65° vertical FOV, and matching actual targets/drawables in both orientations.
  Inspected native whole images/crops and both 60-frame strips. Resource overlap includes recovery's third ring.
  Evidence and limits are in [the M7 record](../../evidence/render/ios-extent/README.md).
  Initial native regression-test setup used an entry point that could skip unchanged publication; corrected it
  to the serial encoded-token submission used by iOS. The final gate covers that correction. Claim awaits lead review.

- 2026-11-09 Planned M7 from COOKED-DRAW M1.10's extent blocker at local checkpoint `b0dae34c`.
  Read the layout/source record, README and landscape/portrait screenshots with `git show`. G3b owns the current
  surface-to-scene extent relationship, including shared render-owner changes. G4's earlier framing deferral is
  superseded for native-resolution correctness. Added the loop child design and M7 definitions; historical M1–M6
  closeouts remain evidence, not new claims. COOKED-DRAW M1 now needs M7; M1.10 is unchanged. No agents launched,
  no implementation/visual/speed acceptance claimed; independent design review owed. No owner decision required.
- 2026-11-08 Overseer brief: M5 rotation and M6 device loss added; M4 marked done on Thrum Core's acceptance.
- 2026-11-08 Plan review: changes required. winit's iOS `inner_size` is the safe area while `Resized` is the full
  bounds (fixed: one source, `outer_size`); the epoch cannot stop the compositor scaling the last drawable between
  the bounds change and `Resized` (claim restated, old-extent presents counted, gap measured first); `destroy` fires
  the callback only from a poll. Thrum Core approved M6 with conditions (generations, revocation, one recovery,
  stale-generation witness), folded into M6.
- 2026-11-08 Daily main review (`chunk-E.md`): M2 fixed (witness I/O and thread naming only under the route); M1
  folded into M6; lows triaged into *Follow-ups*. Two full-route runs failed on host-load timing in existing rows,
  not on rotation: a 10 s held touch released before a slow `replace-surface` notification arrived (`route-full1`),
  and a revocation posted straight after `stall-render` landed before the stall held (`route-full2`). The held touch
  is now 30 s and the route checks the finger was still down at the replacement; the stall is 8 s and the route waits
  for the owner to report the held section before posting the revocation.
- 2026-11-08 M6 development runs (`tmp/g3b-ios-rotation/m6-dev1..6`). In dev run 1 the held-frame loss faulted
  the surface: wgpu-core's `discard` checks device validity before clearing the acquired texture, so the surface
  stays acquired for any device. A loss that held a drawable now replaces the surface (walk step 6). Three route
  defects followed:
  - the recovery hold sat before preparation, so the rotation never forced a refit;
  - the 90 s hold expired before a 109 s XCUITest rotation at load ~190;
  - the background row counted the designed resume reinstall as a replacement.
  The hold now sits after preparation with a 300 s cap. The route also gained the `stale-wake` row. Dev run 6 passed
  every device row: generations 1→2→3, stale wake and request refused, refit to 2622x1206, one publication after
  resume, and a swipe after recovery moved the actor 9.6 m.
- 2026-11-08 Full route `route-full5` (load 455 at start) failed M5's "zero presents on a resize-revoked epoch":
  one section that passed its checks before `Resized` presented once after it, at the old extent. That is the
  revocation race bound, which UIKit cannot close without waiting. The strip shows a rigid, aspect-preserved
  rotation and no squeezed frame. The done-when now names the bound: at most one, and it must be the only
  old-extent present. That change goes to the M5 close review.
- 2026-11-07 Plan written from Thrum Core's commission.
- 2026-11-07 Independent review: changes required. Revised: revocation guarantee, detach bridge, committed-position
  witness, touch before lifecycle, thresholds.
- 2026-11-07 First dev-simulator runs: Rust leaves the iOS main thread unnamed, so the witness asks
  `pthread_main_np`. `thrum-v1://` URLs raise an "Open in" prompt, so route commands moved to Darwin notifications.
  The simulator control tool needs a person's grant, so touch moved to a self-hosted XCUITest runner. All rows then
  passed on the dev device (`tmp/g3b-ios-lifecycle/dev-run7/`). Screens: startup matches the baseline, hidden view
  black, scene back after replacement and resume. The swipe's on-screen movement is not identifiable among 10,000
  grid entities under a fast day cycle; the committed-position witness (seam 2b) carries that row.
- 2026-11-07 Thrum Core approved seam 2 with conditions (answered in *Root conditions*). Added the `sample-actor`
  witness and the `stall-present` raced-present row. Each `simctl spawn` takes about 1.3 s, so waiting to observe a stall
  before posting the revocation spent most of the 5 s window (dev run 8 missed it); the route now posts the revocation
  straight after the stall request and proves it landed inside by the deferred release. Dev run 9
  (`tmp/g3b-ios-lifecycle/dev-run9/`) passed every row: exactly one raced present, swipes moved the committed actor
  9.7 m each, and the held finger's two old-window events moved it 0.000 m.
- 2026-11-07 M1–M3 close review (fresh subagent): M1 PASS. M2 blocked on visible movement (fixed: nudge row with
  cropped captures; `dev-run10` shows the lone square past the grid edge). M3 blocked: a resume during preparation
  left the display link stopped because only `Starting` started it on the prepared reply (fixed). Non-blocking notes
  taken: lock wording, stale-row bracketing logged. Others recorded for follow-up: in-section epoch check as a
  read-modify-write for a formal happens-before on configure; touches on a Busy-released window before
  `finish_release` go uncounted; stale detection keys on touch id; a real UIKit `Cancelled` phase is unit-tested only.
- 2026-11-07 Thrum Core accepted the race witness (exactly one admitted present, detach after release) and the XCUITest
  driver as the real simulator input witness. The `release_surface` and `Take::Busy` comments now describe the
  final-check race. Overloads on fresh runs (`route-fresh-7`: 1 overload, 893 refused ticks; `route-fresh-8`: 3, 799)
  come from host load on the shared Mac starving the simulated world lane. The fixed clock refuses the backlog and
  drains it before resume. The disposition is logged with each run, and these runs make no physical-device timing claim.
  One display-target callback took 1.6 s in `route-fresh-8` outside any stall, also host descheduling; it is reported
  and not gated.
