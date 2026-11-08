# G3b Mac frame runtime qualification

Design: [Platform runtime](PLATFORM-RUNTIME.md#g3b-world-and-presentation-lifecycle--selected-partial-implementation)
and [bounded frame host](../decisions/V1-BOUNDED-FRAME-HOST.md). Siblings:
[iOS](PLATFORM-RUNTIME-IOS-PLAN.md) and [web](PLATFORM-RUNTIME-WEB-PLAN.md).
Reviews: Design and plan over `3049433d`, approved plan SHA-256 prefix `0f467bcb` · [brief](#design-and-plan-review-brief) · initial hold resolved. First M1 abandon seam over `f79e3434` · [brief](#lost-frame-review-brief) · approved for observed post-encode loss. M1 injected input over `43e1d488` · [brief](#injected-input-review-brief) · approved after separating capture cutoff from finalization. M1 live slot `slot-04.0qL58y`, diagnostic `0c9a32dc` · [brief](../../evidence/render/mac-m1-recovery/README.md#independent-review-brief) · both done-when items pass. M2 pan plan over `aeb43fbc`, reviewed SHA-256 prefix `10e7730e` · [brief](#m2-pan-plan-review-brief) · approved after refining slider steps, recording brackets and trace attribution. M2 live slot `slot.mQNT7T` over `0d53d3aa` · [briefs](../../evidence/render/mac-m2-pan/README.md#independent-review-briefs) · motion passes with recorded holds, cadence open and finalization parked.
Milestone: M2 The cooked valley presents smoothly on built-in ProMotion
Now: Paused by owner request (8 Nov 19:40 CET) for workflow review.
Next: run the three-view offscreen parity command in [Resume first](../../evidence/render/mac-m2-pan/README.md#resume-first) with the compiled `474b4306` diagnostic, the fixed noon store and the 0°, 18°, 36° witnesses from `framing.mKC7g4`.
Blocked: cadence remains open. The live trace did not finalize, and export found no readable template. Instruments trace finalization is parked after three attempts.

## M0 Reviewed Mac qualification plan: done

- [x] Independent review verifies the runtime ownership, the two game outcomes and the limits of their instrumentation. [Verdict and brief](#design-and-plan-review-brief). [Build plan](../../BUILD-PLAN.md#active-gate-order-and-budgets) links this plan.

## M1 The same Mac game resumes after graphics loss: done

Owner review: [Ground E route, noon tile](../../evidence/v1-workload/g7-showcase/route-ground-e.png) (proposed, pending).
Use the checked-in noon preset and the same-build uninterrupted run as the exact image control. The reference supplies the expected intact composition.

- [x] After a Metal destroy with a drawable held, the cooked valley resumed in the same PID and window and responded to injected E through the app key handler. Injected P traversed the same handler and Resume was acknowledged before arming. World ticks 80 and 81 committed during loss before the first replacement submit. Neither old queue submitted or presented after destroy. The paused row retained its tick, epoch, timeline and pause intent. [Raw event excerpts and audit](../../evidence/render/mac-m1-recovery/README.md).
- [x] The paused endpoint's 2,073,600 pixels matched before and after recovery and with the same binary's arm-disabled row. The independent reviewer watched the original recordings at 1× and identified visible composited camera response after the replacement submit and present calls. [Motion excerpt and verdict](../../evidence/render/mac-m1-recovery/README.md#independent-review).

Work:

- [x] Give an encoded frame an explicit lost-device abandon path. The Mac actor checks observed loss before and after drawable acquisition, including acquisition refusal. Abandoning releases the encoder and timing reservation without flushing the queue or committing view history. Healthy skip still flushes once. A real Metal test checks the queue call boundary. Replacing abandonment with ordinary drop produces an extra attempted submit and fails the test. [Focused evidence](../../evidence/render/V1-BOUNDED-FRAMES-2026-11-04.md#mac-lost-encoded-frame).
- [x] Build a one-shot diagnostic destroy after acquisition and before view creation or submit. Observe the normal generation-tagged callback, abandon the encoded frame and release the drawable through its owner. Ordinary builds carry no diagnostic hooks. [Witness build and events](../../evidence/render/mac-m1-recovery/README.md).
- [x] Run the detached Metal and paused readback preflights, then the cooked valley in the same diagnostic build with the arm disabled and with both loss modes. The arm-disabled row supplies the uninterrupted control with identical inputs.
- [x] Record device, surface, session and world identity, the acknowledged endpoint, queue calls, cooked generation, and the first restored submission. Retain the motion excerpt and independent review alongside the paired image results.

## M2 The cooked valley presents smoothly on built-in ProMotion: motion passed, cadence open

Owner review: [Ground E route, noon tile](../../evidence/v1-workload/g7-showcase/route-ground-e.png) (proposed, pending).
Use the same noon view and a slow input-driven pan. A no-trace run provides the motion and overhead control.

- [x] The release window on the M3 Max built-in ProMotion panel retains coherent input-driven motion. Independent 1× review of the matched `0d53d3aa` recordings passed. Decoded scene holds of 38.3 ms untraced and 51.7 ms traced are recorded. The later G3c build still needs corroboration. [Motion evidence and verdict](../../evidence/render/mac-m2-pan/README.md).
- [ ] The app’s displayed frame intervals on the built-in panel are independently attributable and recorded. Trace rows must distinguish window/app swaps from display refresh and app submissions. Without attributable swaps, the cadence box remains open. No numeric performance threshold is a gate.

Work:

- [ ] Use the same release diagnostic build, noon preset, extent and fixed cooked root for both arms. Qualify the shipped yaw slider with a step fine enough for a slow pan. At about 18°/s, a 1° step holds the view for about 56 ms even with perfectly regular presentation. Use a 0.05° slider step and preserve the 5° keyboard action. Verify the finer normal control path before building the live witness.
- [ ] Identify the unique game window on the built-in panel. Once the cooked view is ready and the recorder has a complete frame, hold the start view, inject the same elapsed-time yaw sweep at display-link opportunities through the existing AppKit slider target/action and normal app edit handler, then hold the final view before capture cutoff and finalization. Log requested, accepted and submitted yaw separately. Independently review the original recordings side by side at 1× against the proposed noon reference. Require the full sweep in both recordings and the attributed trace. Keep recording rate separate from physical display intervals and photon timing.
- [ ] Use Game Performance for the app-labelled Display rows. The prior 5 November export (`tmp/g3b-present-short-displayed.xml`) contains 210 labelled compositor swaps on the built-in display. The raw surface IDs rotate, and the trace says they were composited. This is a short historical trace, not direct scanout or qualification of this build. In the traced row use a fixed cooked store rather than a changing watcher:
  ```sh
  xcrun xctrace record --no-prompt --template 'Game Performance' --time-limit 16s \
    --output "$trace" --target-stdout "$game_log" \
    --env THRUM_SCENE=valley --env "THRUM_ASSET_STORE=$store" \
    --env "THRUM_PARAMS=$preset" --env THRUM_V1_FRAMES=10000 \
    --launch -- "$release_game"
  xcrun xctrace export --input "$trace" --toc
  xcrun xctrace export --input "$trace" \
    --xpath '/trace-toc/run/data/table[@schema="displayed-surfaces-interval"]'
  ```
  Keep the display export, named process/PID and surface evidence in scratch. Resolve XML references and use numeric Swap Time and one unambiguous app-labelled stream on the built-in display. Frame labels can skip and surface IDs rotate. Use same-process Points of Interest pan markers on the trace timeline. Accept a calibrated monotonic-clock join only with a correlated anchor and stated error. Leave cadence open if the layer or pan alignment is ambiguous.
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
- 8 November. The overseer removed human key presses from M1. The diagnostic sends P/E to the app's shared key handler so the same lifecycle and camera commands run without requiring a person at the keyboard. The live composited observation and independent motion review remain required.
- 8 November. Used the arm-disabled run of the exact diagnostic binary as the uninterrupted window control. This keeps the same build, render path and fixed endpoint for the pixel comparison.
- 8 November. The overseer directed M2 to inject its pan through the normal AppKit and app handlers and use independent review of the game-window recording at 1×. The screen slot supplies the real display and compositor. Neither input nor motion review requires the owner at the keyboard.

- 8 November. Initial M2 review identified authored holds from the one-degree yaw slider, incomplete recording brackets and a possible ambiguous trace join. Qualify a finer shipped yaw step, bracket the sweep in both recordings and use pan markers and actual compositor Swap Time. Keep the input schedule separate from the display result.
- 8 November, 18:26 CET. The granted slot's fresh preflight (`slot.tAxImE`) reported a locked console. Neither live row launched. The overseer's standing grant applies once the console is unlocked. M2 remains active for work that needs no screen.
- 8 November, 19:30 CET. Slot `slot.mQNT7T` recorded both full live sweeps and closed both game windows. Independent 1× review passed motion with short holds recorded. The trace process exceeded its finalization cap and export refused the bundle. Park Instruments trace finalization after the two headless attempts and this live attempt. Leave the physical cadence criterion unchanged.
- 8 November, 19:40 CET. Owner requested a pause for workflow review. The post-G3c diagnostic build finished. Its visual corroboration is the first resume step.

## M2 pan plan review brief

Verify the normal AppKit input route, separation of authored steps and
frame pacing, matched recording brackets, and whether Game Performance
can attribute one swap stream to the game's PID on the built-in panel.
Require trace-domain pan markers or a calibrated join and keep cadence
open if the trace cannot establish attribution.

## Injected input review brief

Compare the diagnostic App with `43e1d488`. Check that injected P/E use
the physical keys' shared app handler, with the same lifecycle and edits.
Require injected P before the normal Resume acknowledgement and E inside
the recording. Keep the actual recovered motion review separate.
