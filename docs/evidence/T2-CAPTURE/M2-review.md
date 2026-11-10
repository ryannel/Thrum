# T2-CAPTURE M2 review: iOS simulator engine motion without desktop capture

Reviewer: Claude Opus 5.5 (claude-opus-5-5), fresh context, 10 November 2026. Reviewed commit
`60b79a2b0401de3f82498e5c20156cf0286b117d` in the loop's clean checkout. Milestone diff read: `480bcff3`
(implementation and qualification) and `60b79a2b` (simulator boot-state record), plus the loop's block/unblock commits.

**Verdict: PASS.** M2.1 pass, M2.2 pass. No blocking findings; nine advisory findings below.

Scope: simulator function only. Nothing here is device speed, physical-iPhone or physical-display evidence.

## What I ran

All on this M3 Max, macOS 26.6.2 (25G83), from the clean checkout, under `agent_loop.py lock gpu`.

1. Built the app and cooker from the checkout: `examples/v1-game-ios/ios/build-simulator.sh` (pinned 1.99.1,
   `aarch64-apple-ios-sim` release) and `cargo build --locked --offline -p thrum-cook`.
2. Ran the documented route, `capture_v1_simulator.py --no-build --case all`, all twelve cases. **One deviation:** the
   disk had 16 GiB free and the driver refuses below 20 GiB. I ran a copy at
   `tmp/t2-m2-review/capture_v1_simulator_review.py` that differs only in that threshold (11 GiB), a fixed `ROOT`
   and a `sys.path` entry for `scripts/`. The full series fit; peak use was about 7 GB of spools. The route
   exited 0 with `pass: true` and `route_pass: true`.
3. Ran `package_v1_simulator.py` on my `01-pan`, `04-running`, `05-stall`, `06-surface`, `07-background` and `08-loss`.
   This independently decoded all 1,679 originals in those sessions against the writer hashes, round-tripped every PNG
   and checked preview PTS. Maximum PTS error was 0.5 µs.
4. Wrote my own motion analysis (`tmp/t2-m2-review/motion.py`). It decodes all 734 pan originals, estimates the
   per-frame horizontal shift on three native rows by cross-correlation, and finds repeated frames and opacity.
5. Ran `./dev check` under `gpu`: exit 0, 62 test suites, 0 failed, 0 ignored. It passed with ordinary parallel
   tests, so the builder's `RUST_TEST_THREADS=1` workaround wasn't needed this time. The new
   `stopping_admission_drains_in_submission_order_while_rendering_continues` test ran and passed.
6. Checked that 232 of the committed evidence files match `retained-sha256.json`, with no mismatch. Diffed
   `driver-measured.py` against the current driver: the only change is the `export_session` retention refactor.

### My run's identities

| Field | Value |
|---|---|
| Commit | `60b79a2b…` (clean; `git status` empty apart from ignored `tmp/`) |
| Device | owned fresh `FA51A34B-…`, `iPhone-18-Pro`, `SimRuntime.iOS-27-2`; deleted on exit (shutdown/delete exit 0) |
| Adapter | `Apple iOS simulator GPU`, backend Metal; downlevel lacks `INDIRECT_EXECUTION`, cooked path `Some(Direct)` |
| Source grid | 1206×2622, row stride 4824, every image |
| Preflight | LaunchServices launcher `org.thrum.cooked-draw.qualification.98ef5c79…`, `CGPreflightScreenCaptureAccess` **false before and false after**; launcher PID 68319 = harness `launcher_pid`, driver PID 68321 = `harness_pid`; bundle identifier equals expected |
| Console | `IOConsoleLocked = Yes` at all 14 observations. The builder's run was unlocked, then locked. The route ran and passed in both states, and nothing gates on it |
| Pre-existing simulators | `iPhone 18 Pro 47494973…` and `Thrum I1 iPhone 13 90298BEA…` were still Booted and untouched after my run |

### My session results

| Case | Images | Expected | Omissions (renderer/writer) | Complete | Notes |
|---|---|---|---|---|---|
| 00-off | none | – | – | – | no `thrum-capture-*` artifacts; world advanced 239→359 |
| 01/02/03-pan | 734 / 735 / 735 | same | 0/0 | yes | 60 Hz cadence, max gap 23.2 ms (01) |
| 04-running | 260 | 260 | 0/0 | yes | semantic axes moved the committed actor x 0.891→12.891 |
| 05-stall | 138 | 138 | 0/0 | yes | 8.036 s gap retained; world ticks 186→668 across it |
| 06-surface | 182 fresh | 182 | 0/0 | yes | 5 pending at transition; retired with 0 images; fresh gen 1, epoch 2→4 |
| 07-background | 183 fresh | 183 | 0/0 | yes | 6 pending; suspended counters (presented 141, cooked 141, tick 147) unchanged over 1 s |
| 08-loss | 182 fresh | 182 | 0/0 | yes | 4 pending; retired with "device generation lost"; fresh device/presenter gen 2; lost empty submits 0, recover failed 0 |
| 09-saturation | 6 | 183 | 177/0 | **no (refused)** | world advanced 122→306 |
| 10-slow (Raw) | 81 | 303 | 222/0 | **no (refused)** | world advanced 123→602 |
| 11-ui-regression | – | – | – | – | existing XCUITest touches and rotation pass; tap disabled; no host OS grant needed |

## Inspection list (written before viewing any capture)

1. Native-size first image of a pan: sky, terrain, rock and sphere all present; opaque; no channel swap, row skew or
   stride tear at 1206×2622.
2. Pan initial hold: consecutive frames identical, with no flicker.
3. Pan motion: yaw monotone, objects moving coherently one way, with no back-steps, repeats or jumps beyond the
   recorded gaps; measure the image-to-image content shift against submit time.
4. Pan final hold: stable, with the scene still complete after the motion.
5. Running/diagnostic: moving actors continuous frame to frame; controlled-actor response recorded independently of
   whether it's visible.
6. Stall: world progress across the gap (not a frozen world), and the gap preserved in timing.
7. Surface/background/device recovery: the first fresh-session image is complete current geometry, not black and not
   stale.
8. Normal speed: on the 1x timeline, the pan's source-time duration is about the authored duration and frame intervals
   are about the display cadence.
9. Labelled simulator function only, with no device-speed claim.

### Judgements against the list (my captures)

1. **Pass.** `01-pan/png/000121.png` at full native size shows sky, layered dunes, textured ground, a mirrored sphere
   with a specular highlight and its shadow, and the rock with its shadow. All decoded pixels are opaque (alpha 255).
   Colours are natural, so there's no BGRA swap. The 512×512 native centre crop (347,1055) shows clean rows with no
   skew or tear. Pre-existing look issues sit outside M2: stair-stepped shadow edges and rock-surface striping.
2. **Pass.** 121 of the 122 frames before motion are byte-identical to their predecessor.
3. **Pass.** The motion runs from 2.046 s to 10.029 s of source time: 480 frames, every one shifting the same way
   (−4 px in 469, −5 in 8, −3 in 2, −6 in 1). There are no zero-shift frames and no repeated frames inside the
   motion. The total is −1,928 px. In the selected strip, the sphere, then the rock, exit left coherently, and the dune
   silhouettes translate continuously.
4. **Pass.** All 131 frames after motion are byte-identical; the final frame is complete terrain and sky.
5. **Pass.** The diagnostic scene (instance grid, checker panel, cube, smoke volume) moves visibly across samples
   64 frames apart. The adjacent native crops 305/306 (ticks 308→310, alpha 1.0→0.98) are coherent, with a small
   displacement and no tear. All 260 running images are unique. The controlled actor's committed position changed by
   12 units through the normal semantic axes handler; the actor itself is off-camera (finding A4).
6. **Pass, by metadata.** The image at submit 1.037 s has tick 185; the next arrives at 9.074 s (tick 186, the frame
   in flight when the stall began), then tick 668. The world advanced about 482 ticks during the 8.036 s render stall,
   and the gap survives packaging (PTS error 0.5 µs). The valley is static, so the before/after pixels look the same
   (finding A2).
7. **Pass.** The first fresh images of surface (239), background (239) and loss (240) each show the complete valley:
   sky, dunes, sphere, rock and shadows. None is black or partial. Freshness is shown by metadata: fresh device and
   presenter generations (2 after loss), first fresh ticks 244/247/246 well past the retirement ticks, and retired
   sessions with zero published images. Pixels can't show it, because the scene is static (finding A2).
8. **Pass.** The authored 8 s pan measured 7.98 s in source time. The median interval is 16.7 ms, with no gap over
   23.2 ms in either pan I checked numerically.
9. **Pass.** Identity, README, maturity ledger and CAPTURE.md all label this simulator function only.

## Item verdicts

### M2.1: pass

- **Statement.** The ordinary UIKit foreground route (`THRUM_V1_ROUTE=1`) captures native engine stills and
  elapsed-time motion through the shared `FrameHost` submitted-output tap (`crates/thrum-render/src/runtime/submitted_capture.rs`)
  and the shared `thrum_app::capture_writer`. The tap is opt-in per session; capture-off produced no artifacts. The
  observer submits no GPU work: `capture.rs` only enables, collects and recycles. Semantic input goes through the
  normal `send_axes` → `controls.set_controls` path. The pan uses the shared validated `set_camera_yaw` edit.
- **Ask.** I reran the documented route myself and checked every identity in the "My run's identities" table. The
  denied preflights are attributed to the actual parent launcher: `validate_preflights` checks bundle identifier, API,
  and launcher/driver PIDs against the harness's own `getppid`/`getpid`. An unrelated helper can't satisfy it. The
  console witness is recorded per phase and is read nowhere as a condition. My run was locked throughout and the
  builder's changed from unlocked to locked; both passed. Semantic input (`capture-move`/`capture-rest`) and real
  UIKit XCUITest touches are separate cases. The XCUITest runs inside the simulator and needed no host OS grant, so
  no attended row is owed.
- **Inspect.** Entries 1–5, 8 and 9 above pass.

### M2.2: pass

- **Statement and ask.** I traced the leases and generations in `render.rs`/`capture.rs`:
  - `frame()` cancels any session and returns `NoLease` while `background` is set.
  - `before()` cancels on a lease-epoch or device-generation mismatch, and `after()` on an epoch change or background.
  - A device loss and `recover()` both cancel before rebuilding.
  - Cancellation drops the tap (`disable_submitted_capture` sets it to `None`) without submitting or waiting.
  - A new session builds a new tap whose slots check generation and extent.
  - `stop_submitted_capture` stops admission while the drain proceeds oldest-first, which the new unit test covers.

  At runtime, all three transitions had 4–6 readbacks pending and retired with zero images. Background counters
  held. Fresh sessions carried only new-generation metadata, lost empty submits and recovery failures stayed 0, and
  every fresh session completed with zero omissions.

  Bounds are finite and enforced: 6 GPU slots, 32 CPU buffers, 1 GiB pixels, an 8 GiB spool, 1,024 records,
  16 s and a 10 s finalize. Saturation and Raw slow output refuse with the world advancing. PTS and coverage are
  validated by packaging.

  `./dev check` passed under `gpu`. The target-specific simulator command was rerun on the target and fails rather
  than skips: `simctl` failures raise, and a missing runtime fails `simctl create`. The physical iPhone row stays
  pending with its owner.
- **Check.** `./dev check` exit 0 (`tmp/t2-m2-review/check.log`).

## Findings

None blocking. Advisory, worst first.

- **A1 (advisory, verified by reading).** The per-image `surface_epoch` in `runtime.json` is the session's epoch,
  copied from `s.epoch` (`examples/v1-game-ios/src/ios/capture.rs:253`), not an identity carried by the image. The
  driver's fresh-session stale check on `surface_epoch` (`scripts/capture_v1_simulator.py:311`) therefore can't fail.
  The no-stale proof actually rests on the tap's real `device_generation`/`presenter_generation`/`submission_order`
  metadata and on each session building a new tap. Those hold, but the surface-replacement row has no independent
  per-image surface witness. Fix: carry the presented lease epoch through `FrameSample`/`SubmittedImage`, or drop the
  tautological comparison.
- **A2 (advisory, verified by running).** The stall and all three lifecycle rows use the static held valley, where a
  stale image and a fresh one are pixel-identical. Post-stall progress and freshness after recovery are shown only by
  tick and generation metadata, never by pixels. A future revision could run the lifecycle and stall rows on the
  running diagnostic scene, so the first fresh image visibly differs from the last retired frame.
- **A3 (advisory, verified by running).** The driver refuses to start below 20 GiB free
  (`scripts/capture_v1_simulator.py:110`). The whole series retains about 7 GB of spools: my run peaked near that on
  a 16 GiB-free disk. On this nearly full disk the documented command can't run as written. Make the preflight
  per-case or measured.
- **A4 (advisory, verified by reading and running).** The visible pan is driven by the route-only `cooked-pan`
  command (`ios.rs:592`, `render.rs:699`), which feeds the shared validated yaw edit, not a UIKit control. The
  semantically controlled actor moves 12 units but is outside the portrait camera, so captured output never shows an
  input-driven response; the committed-position witness proves it instead. This satisfies the statement as written,
  but a camera framing the controlled actor would make input response visible.
- **A5 (advisory, verified by running).** A background-pending session isn't cancelled when the app backgrounds; the
  frame loop isn't called while suspended. It's cancelled on the first foreground frame, by the epoch change
  ("foreground/surface/device lease retired"). Its readbacks, mapped before suspension, stay owned through
  background. No submission occurs and nothing is published, so the item holds, but cancellation timing differs from
  the README's wording.
- **A6 (advisory, from the builder's record).** While qualifying, the builder reset the shared CoreSimulatorService.
  That stopped two pre-existing owner simulators and lost their in-app session state; their boot state was restored
  and recorded in `simulator-restoration.json`. The committed driver no longer does this: it restarts only its owned
  device's IconServices. Shared-service resets should stay out of builder workflows.
- **A7 (advisory, informational).** The builder's `02-pan` recorded a 145 ms renderer gap under host load
  (load average around 45 before the run). Mine had no gap over 23 ms. Capture recorded the gap honestly; it isn't a
  capture defect.
- **A8 (advisory, informational).** The builder ran full Check with `RUST_TEST_THREADS=1` because of existing
  running-world `Overload(13)` tests under contention. My parallel `./dev check` passed. The flake belongs to those
  tests' owner.
- **A9 (advisory, outside M2 scope).** The valley shows stair-stepped shadow edges and the known rock-surface striping.
  These are cooked-draw/lighting look issues, not capture defects.

I found no fingerprints of faked work: no hard-coded returns, canned data, defaulted-on flags, test-only branches,
weakened or deleted tests, or silenced warnings. The only build warning, a `thrum-cook` linker note about
`intel_tex_2` ISPC objects, predates M2. Committed outputs are selected evidence whose hashes I checked and whose
sources I regenerated independently.

## Retention

My review scratch is in ignored `tmp/t2-m2-review/` (about 7 GB of spools plus packaged selections). It's owned by
this review and can be deleted once this verdict lands.

PASS M2
