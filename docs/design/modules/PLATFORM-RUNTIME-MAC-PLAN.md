# G3b Mac frame runtime qualification

Design: [Platform runtime](PLATFORM-RUNTIME.md#g3b-world-and-presentation-lifecycle--selected-partial-implementation)
and [bounded frame host](../decisions/V1-BOUNDED-FRAME-HOST.md). Siblings:
[iOS](PLATFORM-RUNTIME-IOS-PLAN.md) and [web](PLATFORM-RUNTIME-WEB-PLAN.md).
Done when: [PLATFORM-RUNTIME-MAC-DONE.json](PLATFORM-RUNTIME-MAC-DONE.json) owns milestone criteria and proof status; the shared platform design above remains authoritative.

Reviews: M2 historical `0d53d3aa`: motion passed, cadence open ([record](../../evidence/render/mac-m2-pan/README.md#review-verdict-and-open-cadence-row)). Independent review of the JSON migration and 9 November capture re-plan is owed; historical M0/M1 reviews remain in the log.
Milestone: M2 The cooked valley presents smoothly on built-in ProMotion (replanned, open)
Now: 9 November evidence re-plan separates unattended engine runtime motion from attended built-in-panel cadence; no new capture or runtime proof claimed.
Next: independent design/JSON review, then T2-CAPTURE M0, then fresh M2.1 unattended motion. M2.2 remains a separate attended display/tool row.
Blocked: `T2-CAPTURE:M0` is planned, not built. M2.2 needs an unlocked active built-in display and any Instruments authorization; trace finalization remains parked. Screen Recording permission is no longer required for engine image proof.

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

## M2 The cooked valley presents smoothly on built-in ProMotion: open after re-plan

Uses [T2 capture](T2-CAPTURE.md); `needs: T2-CAPTURE:M0`. This is milestone M2 on the M3 Max.
Owner reference: [Ground E noon tile](../../evidence/v1-workload/g7-showcase/route-ground-e.png) (proposed, pending).
Use the same noon view and slow normal-control pan. Preserve the historical `0d53d3aa` motion pass and decoded
38.3 ms untraced / 51.7 ms traced local holds in [evidence](../../evidence/render/mac-m2-pan/README.md).
That verdict is not a fresh pass of the changed M2.1 or the later build; both new items are open. M0/M1 stay done.

- [ ] **M2.1 unattended — current-build runtime content motion.** After T2-CAPTURE M0, run the paused-world
  1920×1080 noon 0°–36° pan over two seconds, plus settled endpoint holds, with 120 Hz requested offscreen
  opportunities. Use actual monotonic elapsed time and the normal semantic control/edit handler. Record locked-console
  and unavailable Screen Recording conditions without requesting permission. Retain every submitted output image,
  requested/accepted/submitted yaw and interpolation/history/sample IDs. Review the original sequence at 1× and
  native-size crops for coherent rock/ridge/sphere motion and record local holds. A fixed-step movie, endpoint
  redraw or three static parity images cannot replace this proof. This does not qualify AppKit/display cadence.
- [ ] **M2.2 attended setup — current-build panel cadence.** The built-in panel must be unlocked and active; any
  OS authorization for Instruments must already be available or be explicitly arranged. The agent injects the
  AppKit slider pan and reviews evidence; the owner need not operate the game. Run same-build, preset, extent and
  fixed cooked-root control/traced arms. Keep the 0.05° slider step and normal 5° keyboard action. Engine capture
  of both arms replaces mandatory ScreenCaptureKit movies, so no Screen Recording grant is required for pixels.
  Require initial/final holds and the complete sweep in both taps and trace, with requested/accepted/submitted yaw.
  Review content at 1× and compare observer overhead. Engine pixels exclude native controls and final composition.
- [ ] Require a fresh readable Game Performance trace and `displayed-surfaces-interval` export. Identify unique
  game window/PID, active built-in display and one app-labelled swap stream; resolve rotating surface IDs/XML refs.
  Use numeric Swap Time, not frame-label continuity. Same-process pan markers or a calibrated monotonic join with
  correlated anchor and stated error must bracket the full pan. Historical 5 November swaps do not qualify this run.
  Useful existing command shape (not a successful new result):
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
  The builder adds the implemented capture arguments from T2's runbook. **Instruments trace finalization remains
  parked** after three attempts; only a separately resumed tool investigation may spend further attempts on it.
  A locked display or failed/unreadable trace leaves M2.2 open and does not block unattended visual work.
- [ ] Separate callback target, actual opportunity, actor acquisition/submit, GPU timestamps, engine image time,
  movie PTS, VSync and app-attributed swaps. Record unmatched samples, source/build, content, recipe, view resolution,
  display mode and run order/load. If the build differs from M2.1, rerun its unattended row. Input-to-photon and
  physical iPhone timing/loss stay unmeasured. Native/compositor appearance needs separate attended evidence if a
  specific observation requires it. No raw drawable bridge, screen unlock automation or mandatory Screen Recording.

M2 cannot close until both items pass independent review. This changes the image acquisition method and separates
its claims; it neither waives real-panel cadence nor declares unattended engine motion proof already implemented.

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

- 9 November, unattended-evidence re-plan. Settled T2 ownership of capture and its V1/V4/F1 handoffs in C12 with
  Mac/G3t consumers. M2 now needs `T2-CAPTURE:M0`. Reset changed M2 items to open, incremented `replans` to 1,
  kept `fails` at 0 and preserved historical M0/M1 and motion evidence. M2.1 requires fresh elapsed-time engine
  motion while locked; M2.2 keeps fresh attributable built-in swaps and is marked attended setup with reason.
  Screen Recording is removed from the engine-image prerequisite. Instruments finalization remains parked.
  No implementation or fresh evidence claimed; independent design/migration review owed, no agents launched.

- 9 November 01:13 UTC: unlocked-console preflight directly refused Screen Recording permission (exit 3), including outside the sandbox under the GPU lock. No live row launched; M2 remains in progress. [Evidence](../../evidence/render/mac-m2-pan/README.md#unlocked-console-probe-9-november-0113-utc).

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

Start from [the unattended-evidence brief](T2-CAPTURE-BRIEF.md), then review the T2 design/JSON, C12, Mac M2
items and the G3t consumer dependency. Check that M2.1 can expose runtime stalls without any desktop grant;
synthetic fixed-time movie smoothness cannot pass. For M2.2 require normal AppKit routing, same-build capture
brackets and fresh independent Game Performance swap attribution for the complete pan. Engine taps supply image
content, not final compositor/native-widget appearance, physical panel observation or photon timing. Confirm that
attended setup is justified by real display/tool needs and the parked trace failure has not been waived.

## Injected input review brief

Compare the diagnostic App with `43e1d488`. Check that injected P/E use
the physical keys' shared app handler, with the same lifecycle and edits.
Require injected P before the normal Resume acknowledgement and E inside
the recording. Keep the actual recovered motion review separate.
