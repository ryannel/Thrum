# G3b Mac frame runtime qualification

Design: [Platform runtime](PLATFORM-RUNTIME.md#g3b-world-and-presentation-lifecycle--selected-partial-implementation)
and [bounded frame host](../decisions/V1-BOUNDED-FRAME-HOST.md). Siblings:
[iOS](PLATFORM-RUNTIME-IOS-PLAN.md) and [web](PLATFORM-RUNTIME-WEB-PLAN.md).
Done when: [PLATFORM-RUNTIME-MAC-DONE.json](PLATFORM-RUNTIME-MAC-DONE.json) owns milestone criteria and proof status; the shared platform design above remains authoritative.

Reviews: M0 over `3049433d`: approved ([record](#log)). M1 `0c9a32dc`: both items pass ([review](../../evidence/render/mac-m1-recovery/README.md#independent-review)). M2 `0d53d3aa`: motion passes, cadence open ([review](../../evidence/render/mac-m2-pan/README.md#review-verdict-and-open-cadence-row)). Migration design review owed.
Milestone: M2 The cooked valley presents smoothly on built-in ProMotion
Now: M2 resumed 9 November; retained post-G3c diagnostic passes three-view offscreen parity. Live qualification is blocked by the locked console and unreadable trace.
Next: corroborate the diagnostic with an untraced live pan once the console is unlocked; resolve parked Instruments finalization before attempting the matched trace row. [Parity and remaining proof](../../evidence/render/mac-m2-pan/README.md#resume-first).
Blocked: cadence remains open. The live trace did not finalize, and export found no readable template. Instruments trace finalization is parked after three attempts.

## M0 Reviewed Mac qualification plan: done

M0.1 passed: independent review verified runtime ownership, the two game outcomes and instrumentation limits.
[Verdict record](#log) · [brief](#design-and-plan-review-brief).
The [build plan](../../BUILD-PLAN.md#active-gate-order-and-budgets) links this plan.

## M1 The same Mac game resumes after graphics loss: done

Owner review: [Ground E route, noon tile](../../evidence/v1-workload/g7-showcase/route-ground-e.png) (proposed, pending).
Use the checked-in noon preset and the same-build uninterrupted run as the exact image control. The reference supplies the expected intact composition.

M1.1 and M1.2 passed: held-drawable loss recovered in the same PID/window with continuing or paused authority,
no old-queue calls after destroy, identical paused pixels and visible input response after replacement.
[Raw events, audit and independent review](../../evidence/render/mac-m1-recovery/README.md).

Work:

- [x] Give an encoded frame an explicit lost-device abandon path. The Mac actor checks observed loss before and after drawable acquisition, including acquisition refusal. Abandoning releases the encoder and timing reservation without flushing the queue or committing view history. Healthy skip still flushes once. A real Metal test checks the queue call boundary. Replacing abandonment with ordinary drop produces an extra attempted submit and fails the test. [Focused evidence](../../evidence/render/V1-BOUNDED-FRAMES-2026-11-04.md#mac-lost-encoded-frame).
- [x] Build a one-shot diagnostic destroy after acquisition and before view creation or submit. Observe the normal generation-tagged callback, abandon the encoded frame and release the drawable through its owner. Ordinary builds carry no diagnostic hooks. [Witness build and events](../../evidence/render/mac-m1-recovery/README.md).
- [x] Run the detached Metal and paused readback preflights, then the cooked valley in the same diagnostic build with the arm disabled and with both loss modes. The arm-disabled row supplies the uninterrupted control with identical inputs.
- [x] Record device, surface, session and world identity, the acknowledged endpoint, queue calls, cooked generation, and the first restored submission. Retain the motion excerpt and independent review alongside the paired image results.

## M2 The cooked valley presents smoothly on built-in ProMotion: in progress

Owner review: [Ground E route, noon tile](../../evidence/v1-workload/g7-showcase/route-ground-e.png) (proposed, pending).
Use the same noon view and a slow input-driven pan. A no-trace run provides the motion and overhead control.

M2.1 motion passed for the matched `0d53d3aa` recordings, with decoded holds of 38.3 ms untraced and 51.7 ms traced
recorded. The later G3c build still needs corroboration. [Evidence and verdict](../../evidence/render/mac-m2-pan/README.md).
M2.2 cadence is open: require independently attributable app/window swaps on the built-in panel, separate from
display refresh and app submissions. No numeric performance threshold is a gate.

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

- 9 November, resumed M2. Under the GPU lock, the retained `474b4306` diagnostic produced byte-identical 0°/18°/36° noon PPMs against `framing.mKC7g4`; static parity is verified, live motion is not extended. Fresh trace export exits 10 (`Document Missing Template Error`), and console probes report locked. No live capture launched and no claim made. [Evidence](../../evidence/render/mac-m2-pan/README.md#9-november-blocker-probe).

- 9 November. Migrated the prose criteria to `PLATFORM-RUNTIME-MAC-DONE.json` without changing the design or acceptance scope. Carried M0/M1 completion and M2's reviewed motion pass; cadence stays open. M2 remains `in_progress` with runtime work paused; only Instruments trace finalization is parked. Initialized `fails` and `replans` to zero: the trace attempts and partial reviews were not failed reviews of a claimed whole milestone. Independent review of this migration is owed; no other agent was launched, as instructed by the owner.
- 8 November. M0 design and plan over `3049433d`, approved plan SHA-256 prefix `0f467bcb`: initial hold resolved and approved. [Review brief](#design-and-plan-review-brief). This is the carried M0 verdict, not a review of the JSON migration.
- 8 November. First M1 abandon seam over `f79e3434`: approved for observed post-encode loss. [Review brief](#lost-frame-review-brief).
- 8 November. M1 injected input over `43e1d488`: approved after separating capture cutoff from finalization. [Review brief](#injected-input-review-brief).
- 8 November. M1 live slot `slot-04.0qL58y`, diagnostic `0c9a32dc`: both done-when items passed. [Independent review](../../evidence/render/mac-m1-recovery/README.md#independent-review).
- 8 November. M2 pan plan over `aeb43fbc`, reviewed SHA-256 prefix `10e7730e`: approved after refining slider steps, recording brackets and trace attribution. [Review brief](#m2-pan-plan-review-brief).
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
