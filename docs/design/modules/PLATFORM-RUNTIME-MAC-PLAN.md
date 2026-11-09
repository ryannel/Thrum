# G3b Mac frame runtime qualification

Design: [Platform runtime](PLATFORM-RUNTIME.md#g3b-world-and-presentation-lifecycle--selected-partial-implementation)
and [bounded frame host](../decisions/V1-BOUNDED-FRAME-HOST.md). Siblings:
[iOS](PLATFORM-RUNTIME-IOS-PLAN.md) and [web](PLATFORM-RUNTIME-WEB-PLAN.md).
Done when: [PLATFORM-RUNTIME-MAC-DONE.json](PLATFORM-RUNTIME-MAC-DONE.json) owns milestone criteria and proof status; the shared platform design above remains authoritative.

Reviews: [T2/Mac design review 2 at `6317102f`](../../evidence/T2-CAPTURE/design-review-2.md): DESIGN CHANGES (2 findings); revised here. Independent re-review of T2/Mac criteria and the M2/M3 split is owed; historical M0/M1 reviews remain in the log.
Milestone: M2 The cooked valley presents smoothly on built-in ProMotion (replanned, open)
Now: 9 November second-review revision lets M2 qualify the current surface path after T2 M0; new M3 owns post-G3t requalification. No fresh proof claimed.
Next: independent design/JSON re-review; T2-CAPTURE M0; Mac builder diagnoses trace tooling (M2.4), qualifies windowed tap (M2.3), and runs current-path motion/cadence (M2.1/M2.2). M3 follows only when APPLE-TILE M3 is delivered.
Blocked: M2 needs design re-review and unbuilt `T2-CAPTURE:M0`, not APPLE-TILE. M2.2 additionally needs M2.3/M2.4 and attended unlocked active panel/Instruments setup. M3 is blocked on parked APPLE-TILE (two failed design reviews), transitively T2 M1/M2 and G3t M0–M3; the lead must recommission its scoped re-plan, followed by independent design approval before build. M2.4 owns bounded trace diagnosis; it is not solved. Screen Recording is not required for engine images.

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
Scene/framing identity only: [Ground E noon tile](../../evidence/v1-workload/g7-showcase/route-ground-e.png).
Its owner image review is not an acceptance gate. Judge quality against the JSON worded list and milestone bar:
rock/ridge coherence, authored sphere sweep/exit, yaw-consistent settled endpoints, and no cut, snap or reversal;
record repeats, local holds and pauses at actual elapsed time. Use the same noon view and slow normal-control pan.
Preserve the historical `0d53d3aa` motion pass and decoded
38.3 ms untraced / 51.7 ms traced local holds in [evidence](../../evidence/render/mac-m2-pan/README.md).
That verdict is not a fresh pass of the changed M2.1 or the later build; all four M2 items are open. M0/M1 stay done.

**Surface-path ordering:** M2 qualifies the current path with only `T2-CAPTURE:M0` as its delivery prerequisite;
windowed-tap and trace-tooling work can proceed before G3t. APPLE-TILE is parked after its second failed design
review and cannot gate any M2 item. Its surface-graph/tone changes still require new qualification: Mac M3 below
needs `APPLE-TILE:M3` and reruns the full motion/cadence protocols on the changed assembly. Earlier verdicts qualify
only their recorded build. G3t D2.5/D3.5 separately require functional actual-window runs; those cannot replace Mac
panel cadence. No reverse Mac dependency is added to G3t. M3 remains a required post-change V1 qualification, not
a reason to hold current-path M2 open. The parked module restarts only on an explicit lead commission to resolve
its seven design findings and obtain fresh independent design approval before build.

- [ ] **M2.1 unattended — current-build runtime content motion.** After T2-CAPTURE M0, run the paused-world
  1920×1080 noon 0°–36° pan over two seconds, plus one-second settled endpoint holds, with 120 Hz requested offscreen
  opportunities. Use actual monotonic elapsed time and the normal semantic control/edit handler. Record locked-console
  and unavailable Screen Recording conditions without requesting permission, locking an active user or unlocking
  by automation. Retain every submitted output image,
  requested/accepted/submitted yaw and interpolation/history/sample IDs. Review the original sequence at 1× and
  native-size crops for coherent rock/ridge/sphere motion and record local holds. A fixed-step movie, endpoint
  redraw or three static parity images cannot replace this proof. This does not qualify AppKit/display cadence.
- [ ] **M2.4 trace tooling — explicit bounded resumption.** The Mac M2 qualification builder owns this investigation
  after this re-plan passes independent design review and T2-CAPTURE M0 is done, without waiting for APPLE-TILE. The previous park
  prevented unchanged retries; this item commissions a changed investigation, not another claim that unlock fixes it.
  First inspect retained failed bundles/logs and document a falsifiable changed hypothesis. Record only with unlocked
  active panel and existing Instruments authorization. At most **two new diagnostic record attempts total** are
  allowed under the 9 November tooling commission, each 16 s recording plus 75 s maximum finalization;
  this dependency re-plan and the M3 split do not renew the allowance; use finite recorded export timeouts.
  Stop only the owned trace/game processes at the deadline. Do not repeat an unchanged failed hypothesis. Success
  requires a fresh readable TOC and `displayed-surfaces-interval` export with real app/window swaps plus a reusable
  command. No justified hypothesis, unavailable authorization or exhausted attempts leaves M2.4/M2.2 open and
  explicitly blocked in status/build-plan; record the cause and continue unblocked work. Further attempts require
  another scoped re-plan. Tooling success alone does not pass the final full-pan cadence row.
- [ ] **M2.3 attended setup — shared windowed tap.** Integrate the T2 renderer-owned tap in the real CAMetalLayer
  window, using the normal callback/lease/present path. With a known colour/channel/row pattern, prove captured
  pixels equal the engine output consumed by the normal present pass (with declared format conversions) and bind
  that producer to the actual surface submission. This proves engine output correspondence, not compositor pixels.
  Show capture neither acquires nor extends a drawable lease; test surface loss/replacement with readback pending,
  stale-generation refusal and healthy fresh presentation. With capture OFF, prove the current direct present
  path remains, with no tap target/copy/readback/worker/per-frame allocation. Use `./dev check` under `gpu` for
  regressions and a retained actual-window scenario; occluded/offscreen substitution cannot qualify this item.
  This is integration/qualification of T2's existing producer, not a second recorder or GPU owner.
- [ ] **M2.2 attended setup — capture-OFF panel cadence.** The built-in panel must be unlocked and active; required
  Instruments authorization must already be available or explicitly arranged. The agent injects the normal AppKit
  slider target/action and reviews the output; the owner need not play. After M2.3/M2.4 pass, run three matched arms:

  | Arm | Required proof |
  | --- | --- |
  | Capture ON, untraced | Complete host-time source sequence, one-second initial hold, two-second 0°–36° sweep and one-second final hold; requested/accepted/submitted yaw; review content at 1× |
  | Capture ON, Game Performance | Same complete content interval; trace overhead versus ON/untraced; capture-versus-OFF observer comparison. Label any swap figures **under capture** |
  | Capture OFF, Game Performance | Shipping configuration's cadence claim, with ordinary direct output and no capture resources/work. Normal frame/pan markers join the complete input interval to independent OS swaps |

  Keep build, release settings, preset, extent, recipe and fixed cooked root identical to M2.1, or rerun M2.1.
  Keep the 0.05° slider step and normal 5° keyboard action. Compare ON/OFF traced arms for capture overhead and
  ON traced/untraced arms for trace overhead; record run order/load. No Screen Recording grant is required for
  engine pixels. Engine images exclude native controls and final composition.
- [ ] For the OFF traced cadence arm, require a fresh readable Game Performance trace and
  `displayed-surfaces-interval` export. Identify unique game window/PID, active built-in display and one app-labelled
  swap stream; resolve rotating surface IDs/XML refs. Use numeric Swap Time, not frame-label continuity.
  Same-process pan markers or a calibrated monotonic join with correlated anchor and stated error must bracket
  the full pan and holds. Historical 5 November swaps do not qualify this run.
  Existing command shape for M2.4 to repair/verify (not a successful new result):
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
  Use the implemented T2 capture switch for the ON arms and explicitly disable it for the cadence arm. A locked
  display, unreadable export or ambiguous swap attribution leaves M2.2 open; it does not block unattended visual work.
- [ ] Separate callback target, actual opportunity, actor acquisition/submit, GPU timestamps, engine image time,
  movie PTS, VSync and app-attributed swaps. Record unmatched samples, source/build, content, recipe, view resolution,
  display mode and run order/load. Input-to-photon and physical iPhone timing/loss stay unmeasured.
  Native/compositor appearance needs separate attended evidence if a specific observation requires it. No raw
  drawable bridge, screen unlock automation or mandatory Screen Recording.

M2 cannot close until **all four items** pass independent review. M2.3 and M2.4 must pass before M2.2's final
qualification run. This re-plan commissions explicit tooling/window integration and separates observer roles;
it neither waives real-panel cadence nor claims capture or the trace failure already solved.

## M3 The post-G3t Mac surface path requalifies runtime motion and built-in cadence: open, blocked

Delivery prerequisite: `APPLE-TILE:M3`; module ordering also requires Mac M2 done. This milestone is split from the
former post-G3t M2 obligation, with `replans: 4`, and preserves the complete acceptance bar. APPLE-TILE is parked;
M3 cannot start on T2 completion alone. Its transitive chain includes T2 Chrome M1, simulator M2 and G3t M0–M3.

- [ ] **M3.1 unattended runtime motion.** Rerun the complete M2.1 protocol on the integrated post-G3t-M3 release
  assembly with fresh build/shader/content/recipe identity. Keep the locked-console elapsed-time pan, holds,
  source frames/crops, control/submission joins and worded visual inspection. No earlier verdict qualifies new output.
- [ ] **M3.2 attended built-in cadence and observer arms.** Revalidate the M2.3 windowed-tap correspondence,
  lease/loss and capture-OFF proofs on the changed path. Rerun the qualified M2.4 trace/export command and all
  M2.2 arms on M3.1's build: ON untraced, ON traced and OFF traced. Fresh attributed numeric swaps with a calibrated
  full-interval join qualify cadence only in OFF. Keep the unlocked active panel and Instruments authorization
  reasons explicit. Functional window logs alone cannot pass. A tooling regression requires a scoped re-plan;
  this milestone grants no new diagnostic attempts.

Both items need fresh independent review before final Mac qualification of the changed V1 path. Historical M2
verdicts remain attached to their build. This split does not waive cadence or resolve APPLE-TILE's seven findings.

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

- 9 November, second T2 design review F1/F2. Removed `APPLE-TILE:M3` from M2 prerequisites, qualifying the current
  surface path after T2 M0 only. Split the full post-change motion/cadence rerun into Mac M3 with an explicit
  `APPLE-TILE:M3` dependency; its windowed-tap/tooling preflight is rerun on that build. M2 `replans` increments
  from 3 to 4, `fails` stays 0, state/items stay open; new M3 inherits `replans: 4`. M0/M1 and all historical
  evidence remain unchanged. The two-attempt tooling allowance is not renewed. Ground E is identity only;
  pan quality uses worded criteria. APPLE-TILE is parked after two failed design reviews, with no automatic
  restart when T2 lands. Independent T2/Mac design re-review is owed; no agents launched.

- 9 November, T2 review revision. Addressed F1–F3 with Mac-owned M2.3 real-window tap integration/proof, M2.4 bounded
  trace-finalization resumption (two changed-hypothesis attempts, owner/trigger/stop rule), and M2.2's capture-OFF
  traced cadence arm plus separate ON content/overhead arms. Cleared M2.2's carried historical evidence field (F10).
  Incremented M2 `replans` from 2 to 3, reset `fails` to 0 and kept all four items open. Retained T2 M0 and Apple tile
  M3 dependencies, M0/M1 completion and old motion history. No new evidence or qualification; independent re-review owed.

- 9 November, Apple tile design-review finding 7. Ordered final Mac M2 qualification after `APPLE-TILE:M3`,
  with both motion and cadence rerun on its changed surface path; added the JSON dependency and incremented M2
  `replans` to 2, keeping both items open and M0/M1 unchanged. G3t M2/M3 now require their own functional Mac
  window proofs. Existing runs remain historical; no cadence or implementation promotion. Independent review owed.

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
items and the split M3 post-G3t dependency. Confirm no M2 item waits on parked APPLE-TILE, and that M3
retains fresh motion, windowed-tap and panel/tooling proof on the changed path without renewing diagnostic attempts.
Check that M2.1 can expose runtime stalls without any desktop grant;
synthetic fixed-time movie smoothness cannot pass. For M2.2 require normal AppKit routing, same-build capture
brackets and fresh independent Game Performance swap attribution for the complete pan. Engine taps supply image
content, not final compositor/native-widget appearance, physical panel observation or photon timing. Confirm that
attended setup is justified by real display/tool needs, M2.3 proves the real-window tap, M2.4 owns a bounded
resumption of trace finalization, and only the capture-OFF traced arm qualifies shipping cadence. No parked failure
is treated as solved by this plan.

## Injected input review brief

Compare the diagnostic App with `43e1d488`. Check that injected P/E use
the physical keys' shared app handler, with the same lifecycle and edits.
Require injected P before the normal Resume acknowledgement and E inside
the recording. Keep the actual recovered motion review separate.
