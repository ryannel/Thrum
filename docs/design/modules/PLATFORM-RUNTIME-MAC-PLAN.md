# G3b Mac frame runtime qualification

Design: [Platform runtime](PLATFORM-RUNTIME.md#g3b-world-and-presentation-lifecycle--selected-partial-implementation)
and [bounded frame host](../decisions/V1-BOUNDED-FRAME-HOST.md). Siblings:
[iOS](PLATFORM-RUNTIME-IOS-PLAN.md) and [web](PLATFORM-RUNTIME-WEB-PLAN.md).
Reviews: Design and plan over `3049433d`, approved plan SHA-256 prefix `0f467bcb` · [brief](#design-and-plan-review-brief) · initial hold resolved on lost-token cleanup, the recovery oracle, display attribution and platform distinctions. First M1 abandon seam over `f79e3434` · [brief](#lost-frame-review-brief) · approved for observed post-encode loss.
Milestone: M1 The same Mac game resumes after graphics loss
Now: The lost-frame abandon test and detached Metal recovery helper pass. The opt-in V1 witness builds in scratch.
Next: run the arm-disabled, paused-loss and running-loss rows, then review the recovered window in motion.
Blocked: the live composited M1 witness is queued for an owner-present window.

## M0 Reviewed Mac qualification plan: done

- [x] Independent review verifies the runtime ownership, the two game outcomes and the limits of their instrumentation. [Verdict and brief](#design-and-plan-review-brief). [Build plan](../../BUILD-PLAN.md#active-gate-order-and-budgets) links this plan.

## M1 The same Mac game resumes after graphics loss: planned

Owner review: [Ground E route, noon tile](../../evidence/v1-workload/g7-showcase/route-ground-e.png) (proposed, pending).
Use the checked-in noon preset and the same-build uninterrupted run as the exact image control. The reference supplies the expected intact composition.

- [ ] The cooked valley presents successfully before arming. After an actual Metal device destroy with an acquired drawable, the same PID and window resume visible frames and camera response. The running row resumes the paused noon preset through the normal input and waits for authority acknowledgement before arming. An authority sample strictly between loss and first replacement submit shows progress without old-device submission. The paused row keeps the acknowledged tick, epoch, timeline and pause intent.
- [ ] At a matching paused endpoint, the restored scene is byte-equal to the pre-loss scene and the same diagnostic binary with the arm disabled. Separately observe a live composited frame after replacement submit and the present call, then watch recovery and camera motion at 1×. Offscreen images and present calls alone cannot establish that the window recovered.

Work:

- [x] Give an encoded frame an explicit lost-device abandon path. The Mac actor checks observed loss before and after drawable acquisition, including acquisition refusal. Abandoning releases the encoder and timing reservation without flushing the queue or committing view history. Healthy skip still flushes once. A real Metal test checks the queue call boundary. Replacing abandonment with ordinary drop produces an extra attempted submit and fails the test. [Focused evidence](../../evidence/render/V1-BOUNDED-FRAMES-2026-11-04.md#mac-lost-encoded-frame).
- [ ] Add a one-shot, opt-in destroy after a successful drawable acquisition and before view creation or submit. Keep the drawable held for bounded callback delivery using nonblocking maintenance, as the existing Metal device-watch test does. Route the normal generation-tagged notification and outcome, abandon the lost token and release the drawable through its owner. Time out as a failed witness, abandon without submitting, and distinguish “never fired” from “notification absent.” Record acquisition → destroy → notification → abandonment and the first new-generation submit. Keep ordinary builds free of diagnostic frame work.
- [ ] Run the detached helper and device/replay tests before the live game. Build the normal valley with `python3 scripts/run_v1_showcase.py check`. Use `python3 scripts/run_v1_showcase.py mac --stop noon --frames 600` for the uninterrupted window. Record the exact diagnostic build and invocation when the arm exists.
- [ ] Record session and world identity, admitted/committed ticks, pause, controls, cooked root and generation, old/new surface and device generations, first restored submission and per-generation faults. Capture before arming and after recovery has drawn the retained cooked root, with fixed acknowledged tick, view and extent. Plan a diagnostic render-owner scene readback outside the timed run, with the same warmup and no UI overlay. The current game readback is only solar time. Freeze an identical admitted endpoint for the arm-disabled run and refuse a mismatched endpoint before comparing RGB. Keep the no-injection control and the independent review. Run `./dev check`.

## M2 The cooked valley presents smoothly on built-in ProMotion: planned

Owner review: [Ground E route, noon tile](../../evidence/v1-workload/g7-showcase/route-ground-e.png) (proposed, pending).
Use the same noon view and a slow input-driven pan. A no-trace run provides the motion and overhead control.

- [ ] The release window on the M3 Max built-in ProMotion panel retains smooth input-driven motion. Review the normal-speed pan alongside the untraced control. A visible stall, jump or repeated image during continuous input is a finding to resolve or record before closing visual motion.
- [ ] The app’s displayed frame intervals on the built-in panel are independently attributable and recorded. Trace rows must distinguish window/app swaps from display refresh and app submissions. Without attributable swaps, the cadence box remains open. No numeric performance threshold is a gate.

Work:

- [ ] Run `python3 scripts/run_v1_showcase.py mac --stop noon --frames 600` as the untraced control. Use the same preset, extent and fixed cooked root for both runs. Drag the existing yaw slider continuously between marked stops at the same speed after the cooked view is ready. In the owner window observe the real panel and capture the pan at normal speed. Mark pan start and end in the run record, and require a complete settled sweep in both the recording and the attributed trace. Retain the capture rate and limits. A screen recording can support motion review without proving panel intervals or photon timing.
- [ ] Use Game Performance for the app-labelled Display rows. The prior 5 November export (`tmp/g3b-present-short-displayed.xml`) contains 210 labelled compositor swaps on the built-in display. The raw surface IDs rotate, and the trace says they were composited. This is a short historical trace, not direct scanout or qualification of this build. In the traced row use a fixed cooked store rather than a changing watcher:
  ```sh
  xcrun xctrace record --template 'Game Performance' --time-limit 12s \
    --output "$trace" --target-stdout "$game_log" \
    --env THRUM_SCENE=valley --env "THRUM_ASSET_STORE=$store" \
    --env "THRUM_PARAMS=$preset" --env THRUM_V1_FRAMES=600 \
    --launch -- "$release_game"
  xcrun xctrace export --input "$trace" --toc
  xcrun xctrace export --input "$trace" \
    --xpath '/trace-toc/run/data/table[@schema="displayed-surfaces-interval"]'
  ```
  Keep the display export, named process/PID and surface evidence in scratch. Reuse the row parser in `scripts/qualify_macos_display.py` without its retired fixed-duration and numeric acceptance rules. Require the intended built-in display, PID and one unambiguous game layer and frame sequence. Do not require a constant raw compositor surface ID. Use a short Metal System Trace with the same launch inputs for acquire/GPU diagnosis if needed.
- [ ] Keep callback targets, actor acquisition and submit, GPU timestamps and app-attributed display swaps separate. Align the observation to the same run and pan interval using recorded process and time markers. Report unmatched samples explicitly. Display VSync alone shows display opportunity. HUD intervals can supplement the trace. Input-to-photon remains unmeasured without a correlated input and physical displayed response. No raw drawable bridge.
- [ ] Record view resolution, display mode, source/build, recipe, run order/load and CPU/GPU frame rows. Headless 1× captures diagnose scene motion. M2’s live motion and physical cadence witnesses use the real panel. Physical iPhone timing and loss detection remain open.

## Design and plan review brief

Check the code and contracts over `3049433d`. Trace the world and platform
owners, loss notification around the held drawable, fresh generation
admission and the no-injection oracle. Check that each proposed
observation supports its stated conclusion. In particular distinguish
display link, submit, GPU completion, VSync and attributed presentation,
and find any claim that the proposed tools cannot measure. Confirm scope
separation from G3t/G4 and the shipped browser/iOS paths.

Sources: [Apple’s CADisplayLink documentation](https://developer.apple.com/documentation/QuartzCore/CADisplayLink) ·
[Instruments display and Metal events](https://developer.apple.com/documentation/xcode/analyzing-the-performance-of-your-metal-app) ·
[Metal HUD intervals](https://developer.apple.com/documentation/xcode/monitoring-your-metal-apps-graphics-performance) ·
[MTLDrawable presentedTime](https://developer.apple.com/documentation/metal/mtldrawable/presentedtime).

## Lost frame review brief

Review the token and Mac actor diff over `f79e3434` against C06.
Verify timing and drop accounting, healthy staging flush, loss checks
before and after acquisition, and the real Metal queue-attempt control.
Keep encode-time failure and live-window recovery outside this proof.

## Log

- 8 November. The owner superseded same-turn goal chaining after the successor record was lost. At M0 close, mark the goal complete and end the turn. The owner starts M1 separately.
