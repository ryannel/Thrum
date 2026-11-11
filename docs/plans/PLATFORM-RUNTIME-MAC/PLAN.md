# G3b Mac frame runtime qualification

Design: [Mac runtime qualification](PLATFORM-RUNTIME-MAC.md), under the shared
[platform runtime](PLATFORM-RUNTIME.md#g3b-world-and-presentation-lifecycle--selected-partial-implementation)
and [bounded frame host](../decisions/V1-BOUNDED-FRAME-HOST.md). Siblings:
[iOS](PLATFORM-RUNTIME-IOS-PLAN.md) and [web](PLATFORM-RUNTIME-WEB-PLAN.md).
Done when: [PLATFORM-RUNTIME-MAC-DONE.json](PLATFORM-RUNTIME-MAC-DONE.json) owns milestone criteria and proof status; the shared platform design above remains authoritative.

Reviews: [T2/Mac design review 3](../../evidence/T2-CAPTURE/design-review-3.md) clears Mac M2 to build; its minor Mac M3 finding is outside this milestone. Historical M0/M1 reviews remain in the log.
Milestone: M2 The cooked valley presents smoothly on built-in ProMotion (blocked on attended setup)
Now: fresh M2.1 permission-denied runtime motion has 480 exact originals, complete holds and no capture omissions.
M2.3 has actual-window channel/row correspondence, pending-readback replacement, stale-image refusal and OFF-path
builder proof on the same release binary. The console witness policy accepts either state and transitions.
Next: use the [one-command attended runbook](../../evidence/render/mac-m2-20261110/RUNBOOK.md) for M2.4 tooling
and M2.2 capture-OFF cadence. The four untraced window arms are retained. The owner need only arrange the active
unlocked built-in panel and any Instruments authorization. The builder performs the remaining collection/review.
Blocked: M2.2/M2.4 await attended trace setup and fresh attributed swaps/full-pan clock alignment. Zero of the two
new 9 November diagnostic recordings were used. No other module delivery is missing for M2. Selected current
proof and scope are in [10 November evidence](../../evidence/render/mac-m2-20261110/README.md).
M3 handoff: Mac M3 still needs Mac M2 and APPLE-TILE M3. Its chain is delivered T2 M0/M1 then sequential G3t M0–M3, including attended OS-window observer/grant proof and diagnostic simulator function; simulator cooked/T2 M2 is no longer on that chain.

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
That historical verdict does not pass the changed M2.1 or a later build. Current proof/status is in the JSON and [current evidence](../../evidence/render/mac-m2-current/README.md); M0/M1 stay done.

**Surface-path ordering:** M2 qualifies the current path with only `T2-CAPTURE:M0` as its delivery prerequisite;
windowed-tap and trace-tooling work can proceed before G3t. APPLE-TILE is parked after its second failed design
review and cannot gate any M2 item. Its surface-graph/input-seam changes still require new qualification: Mac M3 below
needs `APPLE-TILE:M3` and reruns the full motion/cadence protocols on the changed assembly. Earlier verdicts qualify
only their recorded build. G3t D2.5/D3.5 separately require functional actual-window runs; those cannot replace Mac
panel cadence. No reverse Mac dependency is added to G3t. M3 remains a required post-change V1 qualification, not
a reason to hold current-path M2 open. The lead has commissioned the scoped re-plan answering its seven findings;
fresh independent design approval/unpark through the loop remains owed before build.

- [x] **M2.1 unattended — current-build runtime content motion.** After T2-CAPTURE M0, run the paused-world
  1920×1080 noon 0°–36° pan over two seconds, plus one-second settled endpoint holds, with 120 Hz requested offscreen
  opportunities. Use actual monotonic elapsed time and the normal semantic control/edit handler. Require denied
  read-only Screen Recording preflights before/after under identified launcher/driver permission identities. Record
  before/after console state as witness metadata, accepting either state and transitions, with unavailable
  observations recorded with a reason. Remove lock waits/refusals and request no grant. Retain every submitted
  output image,
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
- [x] **M2.3 attended setup — shared windowed tap.** Integrate the T2 renderer-owned tap in the real CAMetalLayer
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
former post-G3t M2 obligation, with `replans: 4`, and preserves the complete acceptance bar. APPLE-TILE remains parked pending fresh independent design review/unpark of its lead-commissioned scoped re-plan;
M3 cannot start on T2 completion alone. Its transitive chain is delivered T2 Mac M0/Chrome M1 then G3t M0–M3,
including attended D2.5/D3.5 OS-window evidence (unlocked session and Screen Recording permission for that observer)
and diagnostic simulator endpoint/function checks. It does not wait on cooked simulator COOKED-DRAW M1 or T2 M2.
G3t retains the SDR tone/intermediate and only exposes G4 pre-tone input; the changed scheduling/input path still
requires the full requalification. Mac cadence continues to use capture OFF with its own attended panel/Instruments setup.

- [ ] **M3.1 unattended runtime motion.** Rerun the complete M2.1 protocol on the integrated post-G3t-M3 release
  assembly with fresh build/shader/content/recipe identity. Apply M2.1's denied permission preflights and console
  witness policy, accepting either state and transitions. Keep the elapsed-time pan, holds,
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

## Console witness correction

The builder changes `scripts/capture_mac_m2_ungranted.py` to stop requiring
`locked_permission_unavailable_qualified` from `scripts/capture_v1_runtime.py`. Qualify capture independence from
successful child execution and denied attributed launcher/driver Screen Recording preflights, retaining the existing
launcher before/after checks and adding a driver after-run preflight alongside its existing before-run observation
in `scripts/capture_v1_runtime.py`. Separate that proof from the runtime's combined lock/permission summary. Preserve the combined field as historical metadata if retained; its lock values cannot
gate open M2.1/M3.1. Preserve actual console witnesses, including false or changed values and reasons for unavailable
observations. Missing, failed or granted permission preflights still fail capture-independence qualification.
Add focused acceptance/refusal controls, rerun the documented M2.1 pan under `gpu` and retain fresh visual/timing/
completeness proof before review. M3.1 repeats this corrected protocol after APPLE-TILE M3 on its new build.

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
