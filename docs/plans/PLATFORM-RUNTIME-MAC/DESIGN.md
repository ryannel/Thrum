# Mac frame runtime qualification

9 November 2026. Mac child of the [shared platform runtime design](../../design/modules/PLATFORM-RUNTIME.md#g3b-world-and-presentation-lifecycle--selected-partial-implementation)
and [bounded frame host](../../design/decisions/V1-BOUNDED-FRAME-HOST.md).
[Plan](PLAN.md) · [done when](DONE.json).
This document collects the existing Mac mechanisms and qualification boundaries, including the owner's console
witness correction. Independent review of this consolidated design is owed to the lead/loop.

## Outcome and scope

On the M3 Max with Metal and the built-in 120 Hz ProMotion display, the same cooked valley, world, window and
controls survive graphics loss. A slow noon camera pan retains coherent rock/ridge placement, the authored sphere
sweep and exit, and yaw-consistent settled endpoints. Inspect full-resolution images, native-size crops and motion
at 1× for cuts, snaps, reversals, repeats and local holds. The Ground E view identifies scene and framing. The
milestone's worded inspection list and shipped high-end game quality bar govern visual acceptance.

M0/M1 retain their accepted plan/recovery evidence. M2 qualifies the current surface path through elapsed-time
engine motion, real-window tap correspondence and OS-reported presented times for the game's drawables on the
built-in panel. M3 repeats the
complete qualification after APPLE-TILE M3 changes the surface/present path. Each result identifies its actual
build, content and recipe. The done-when file owns criteria and proof status.

## Ownership and lifetime

Core's `ClockedWorld` and native `WorldLane` own authoritative time and tick admission under
[C02](../../design/CONTRACTS.md#c02-time-and-committed-state--f1-coordinator-w1-world). AppKit owns the window, view and
display-link delivery. The existing `thrum-app` render actor owns surface configuration, late drawable acquisition,
presentation calls and serialized device reconstruction. Display-link target time supplies interpolation lead.
`thrum-render::FrameHost` owns bounded frame slots, submission, completion and submitted history under
[C06](../../design/CONTRACTS.md#c06-gpu-execution-and-lifetime--v1) and
[C07](../../design/CONTRACTS.md#c07-render-scene-views-and-temporal-data--v2-with-p4v3v4).

Graphics loss closes old-generation GPU admission while retained authority and control state survive. The actor
checks loss before and after drawable acquisition. A lost encoded frame abandons its encoder and timing reservation
without queue work or history publication. Healthy frame skipping retains its existing staging-flush behavior.
Replacement uses the platform's fresh device and capability record, detached preparation and generation-checked
publication with current scene inputs. Old slots retire without waiting on lost fences. Surface and device epochs
remain distinct. The held-drawable configuration refusal has one retry, followed by a recoverable unavailable surface.

Accepted M1 proves this through arm-disabled, paused-loss and running-loss rows in the same diagnostic build,
including queue-call audit, retained PID/window, paused image equality and visible control response after recovery.
The [retained recovery evidence](../../evidence/render/mac-m1-recovery/README.md) owns those historical results.

## Engine motion and console witnesses

M2.1 consumes accepted T2 M0's renderer-owned submitted-output tap and offscreen opportunity adapter. Actual
monotonic elapsed time drives the ordinary world/render/control paths and presenter admission. With the world
paused, normal semantic camera edits produce fresh submissions while committed authority remains paused.

The workload is the 1920×1080 noon valley with 120 Hz requested opportunities, a one-second initial hold, a
two-second 0°–36° pan and a one-second final hold. Join requested, accepted and submitted yaw to image/submission,
world/interpolation/history and device/surface identities. Retain complete lossless source frames and host-time
records with the 1× movie. Capture omissions make the interval inconclusive. The requested opportunity rate and
movie timestamps establish their declared engine observations.

Record console state before and after as witness metadata. Accept locked, unlocked and changing state, and retain
a reason when the observation is unavailable. Denied read-only Screen Recording preflights before and after under
identified launcher/driver permission identities establish independence from desktop capture. Missing, failed or
granted preflights leave that proof incomplete. No manual console-lock wait belongs in this protocol.

The builder removes the combined lock/permission gate in `scripts/capture_mac_m2_ungranted.py` and separates its
qualification from `scripts/capture_v1_runtime.py`'s historical `locked_permission_unavailable_qualified` summary.
Retain attributed launcher preflights and add the driver after-run preflight. The plan commissions the precise
regression controls and fresh M2.1 run. Accepted lock witnesses remain valid capability history.

## Window output, panel cadence and trace tooling

M2.3 integrates the existing T2 tap with the real CAMetalLayer window and normal callback/lease/present path.
A known colour/channel/row pattern establishes that captured bytes are the engine output consumed by the actual
surface submission, with format conversions declared. Pending-readback surface loss/replacement must reject stale
publication and resume healthy presentation. Capture observes the existing submission without acquiring or extending
a drawable lease. Its disabled arm retains direct presentation with zero tap targets, copies, readbacks, workers or
per-frame capture allocations. T2 retains bounded readback/writer ownership under [C12](../../design/CONTRACTS.md#c12-budgets-evidence-and-acceptance--t2-with-all-owners).

M2.4 and M2.5 qualify the cadence witness. The witness is the OS-reported presented time of each game drawable:
the `ca-client-presented-handler` table that Instruments records for the launched game. The evidence reads it as
CoreAnimation's record of when each drawable reached the screen, the value `MTLDrawable.presentedTime` exposes:
it sits on the refresh grid while handler callback times jitter, and M2.5 validates it against the markers. The
game needs no change and no raw drawable bridge. Both 11 November traces hold it for every drawable in their
retained window, with three two-refresh intervals between them
([diagnosis](../../evidence/PLATFORM-RUNTIME-MAC/m2-display-event-diagnosis.md)). The Instruments display tables
are not the witness. Their kernel source emitted nothing in either trace while the rest of system-wide kernel
tracing worked, and the cause is not established. Export them with every recording: populated tables corroborate,
and empty ones are recorded without blocking.

M2.4 is one reusable command that turns a trace into the witness result, built with no new recording. It derives
presented time per drawable through a conversion validated against the same-process markers with stated error,
joins drawables by ordinal to the game's submission records, and reports intervals as multiples of the display's
reported refresh period. A missing ordinal, a zero or absent presented time and an unfamiliar status are unmatched
samples, never presented frames. The command must reproduce the three retained two-refresh intervals from the
11 November tables.

M2.5 is a fresh recording that covers the whole marked interval. Game Performance records as a ring buffer and
kept under three seconds on 11 November, starting after the pan began. The builder chooses a recording
configuration that is not windowed, or a window and instrument set whose retained rows span both `MacM2Whole`
markers, and proves coverage from the trace itself. A smaller instrument set is allowed when the witness table,
the same-process markers and display identity remain.

This re-plan grants six trace recordings across M2.5 and M2.2. Each has at most 16 seconds of recording, at most
75 seconds of finalization and finite export timeouts, and runs under the `gpu` lock. The 9 November allowance is
spent and is not renewed by renaming outputs. If the six are used without a qualified witness and both traced
arms, the builder stops with M2.5 and M2.2 open, and more recordings need a scoped re-plan.

After M2.3, M2.4 and M2.5, M2.2 runs matched capture-ON/untraced, capture-ON/traced and capture-OFF/traced arms
using the same release build, cooked root, recipe, extent and normal AppKit slider target/action sweep. ON arms retain
complete content motion and expose capture and tracing overhead. The OFF arm supplies shipping cadence through the
qualified witness: presented intervals for the full pan and both holds, attributed by game PID and by the window
record on the built-in display. The trace's display identity must list the built-in panel as the only display;
otherwise the window's display is identified separately. Every interval longer than one refresh period is listed
with its drawable, time and yaw, joined to the game's own records and investigated. Record run order and host load:
the lock covers the GPU, not other builders' CPU work, so name that load beside a missed refresh. An M2.5
recording that meets every OFF-arm condition may serve as that arm.

These window, panel and tool rows need an unlocked active built-in display. They need no Developer mode change
and no new Instruments authorization: both 11 November recordings collected system-wide kernel events without a
prompt. Engine pixels require no Screen Recording grant. OS-window or compositor observation, when separately
required by APPLE-TILE, has its own attended permission identity. Callback targets, opportunities, submission, GPU
completion, image times, movie PTS and OS-reported presented times remain distinct measurements. The witness is
the presented time the OS delivers to the game process. It is not the kernel's display-swap record, scan-out or
photons. Input-to-photon and physical iPhone qualification require their own evidence.

## Cost, sequence and review

Reuse the existing world lane, render actor, FrameHost and T2 producer. Capture is opt-in with bounded asynchronous
readback and writer resources. Record complete observer costs and investigate visible hitches or comparable
regressions under the existing performance policy. Timing figures remain observations under this qualification.

M2 needs only `T2-CAPTURE:M0` and can complete on the current path. In module order M3 follows Mac M2 and needs
`APPLE-TILE:M3`, then reruns motion, window-tap/tooling validation and all cadence arms on the integrated build.
It gets one recording per traced arm and no diagnostic recordings. COOKED-DRAW and T2 simulator M2 retain their
separate ownership and qualification. Independent review of every required item closes each qualification milestone.

## M2 and M3 qualification protocol

Moved from the module plan on 11 November 2026, when the plans took the orchestrate 1.7.0 shape.

The M2 and M3 work lists with their protocol detail. The plan carries a short list for each milestone and points
here. The cadence witness was revised on 11 November 2026 after both commissioned traces returned empty display
tables.

### M2 The cooked valley presents smoothly on built-in ProMotion

Uses [T2 capture](../T2-CAPTURE/DESIGN.md); `needs: T2-CAPTURE:M0`. This is milestone M2 on the M3 Max.
Scene/framing identity only: [Ground E noon tile](../../evidence/v1-workload/g7-showcase/route-ground-e.png).
Its owner image review is not an acceptance gate. Judge quality against the JSON worded list and milestone bar:
rock/ridge coherence, authored sphere sweep/exit, yaw-consistent settled endpoints, and no cut, snap or reversal;
record repeats, local holds and pauses at actual elapsed time. Use the same noon view and slow normal-control pan.
Preserve the historical `0d53d3aa` motion pass and decoded
38.3 ms untraced / 51.7 ms traced local holds in [evidence](../../evidence/render/mac-m2-pan/README.md).
That historical verdict does not pass the changed M2.1 or a later build. Current proof is in the done-when file and [current evidence](../../evidence/render/mac-m2-current/README.md).

**Surface-path ordering:** M2 qualifies the current path with only `T2-CAPTURE:M0` as its delivery prerequisite;
windowed-tap and trace-tooling work can proceed before G3t. APPLE-TILE cannot gate any M2 item. Its surface-graph/input-seam changes still require new qualification: Mac M3 below
needs `APPLE-TILE:M3` and reruns the full motion/cadence protocols on the changed assembly. Earlier verdicts qualify
only their recorded build. G3t D2.5/D3.5 separately require functional actual-window runs; those cannot replace Mac
panel cadence. No reverse Mac dependency is added to G3t. M3 remains a required post-change V1 qualification, not
a reason to hold current-path M2 open. 

- **M2.1 unattended — current-build runtime content motion.** After T2-CAPTURE M0, run the paused-world
  1920×1080 noon 0°–36° pan over two seconds, plus one-second settled endpoint holds, with 120 Hz requested offscreen
  opportunities. Use actual monotonic elapsed time and the normal semantic control/edit handler. Require denied
  read-only Screen Recording preflights before/after under identified launcher/driver permission identities. Record
  before/after console state as witness metadata, accepting either state and transitions, with unavailable
  observations recorded with a reason. Remove lock waits/refusals and request no grant. Retain every submitted
  output image,
  requested/accepted/submitted yaw and interpolation/history/sample IDs. Review the original sequence at 1× and
  native-size crops for coherent rock/ridge/sphere motion and record local holds. A fixed-step movie, endpoint
  redraw or three static parity images cannot replace this proof. This does not qualify AppKit/display cadence.
- **M2.4 unattended — cadence witness command.** Build the reusable command described under
  [window output, panel cadence and trace tooling](#window-output-panel-cadence-and-trace-tooling), with no new
  recording, against the retained
  [capture-OFF](../../evidence/PLATFORM-RUNTIME-MAC/m2-presented-off-traced.tsv.gz) and
  [capture-ON](../../evidence/PLATFORM-RUNTIME-MAC/m2-presented-on-traced.tsv.gz) tables. It must find the
  two-refresh intervals at drawables 526→527 and 544→545 (OFF) and 558→559 (ON), and join 544→545 to the game's
  15.47 ms present-call gap in the retained capture-OFF surface records. Those traces start after the pan began,
  so the command must refuse them as cadence evidence for the whole interval.
- **M2.5 attended setup — full-interval witness recording.** On the unlocked active panel, record the release
  game with a configuration that covers both `MacM2Whole` markers. Show full coverage from the trace itself, the
  validated time conversion, the ordinal join of every submission and the display tables' row counts. Recordings
  come from the six-recording allowance. Witness success alone does not pass the cadence item.
- **M2.3 attended setup — shared windowed tap.** Integrate the T2 renderer-owned tap in the real CAMetalLayer
  window, using the normal callback/lease/present path. With a known colour/channel/row pattern, prove captured
  pixels equal the engine output consumed by the normal present pass (with declared format conversions) and bind
  that producer to the actual surface submission. This proves engine output correspondence, not compositor pixels.
  Show capture neither acquires nor extends a drawable lease; test surface loss/replacement with readback pending,
  stale-generation refusal and healthy fresh presentation. With capture OFF, prove the current direct present
  path remains, with no tap target/copy/readback/worker/per-frame allocation. Use `./dev check` under `gpu` for
  regressions and a retained actual-window scenario; occluded/offscreen substitution cannot qualify this item.
  This is integration/qualification of T2's existing producer, not a second recorder or GPU owner.
- **M2.2 attended setup — capture-OFF panel cadence.** The built-in panel must be unlocked and active. The agent
  injects the normal AppKit slider target/action and reviews the output; the owner need not play. After M2.3,
  M2.4 and M2.5 pass, run three matched arms:

  | Arm | Required proof |
  | --- | --- |
  | Capture ON, untraced | Complete host-time source sequence, one-second initial hold, two-second 0°–36° sweep and one-second final hold; requested/accepted/submitted yaw; review content at 1× |
  | Capture ON, traced | Same complete content interval; trace overhead versus ON/untraced; capture-versus-OFF observer comparison. Label any presented intervals **under capture** |
  | Capture OFF, traced | Shipping configuration's cadence claim, with ordinary direct output and no capture resources or work. Normal frame and pan markers join the complete input interval to the qualified witness |

  Keep build, release settings, preset, extent, recipe and fixed cooked root identical to M2.1, or rerun M2.1.
  Keep the 0.05° slider step and normal 5° keyboard action. Both traced arms use M2.5's recording
  configuration and M2.4's command. Compare ON/OFF traced arms for capture overhead and ON traced/untraced arms for trace
  overhead; record run order and load. No Screen Recording grant is required for engine pixels. Engine images
  exclude native controls and final composition.
- For the OFF traced cadence arm, require a fresh readable trace whose retained rows span both `MacM2Whole`
  markers. Identify the unique game PID and window and the built-in display. Every submission in the pan and both
  holds has one presented row or is reported unmatched. Report presented intervals in refresh periods and list
  each interval longer than one period with drawable, time, yaw, the game's own present-call record and host load.
  Handler callback time and frame-label continuity are not presented time. Historical traces do not qualify this
  run. A locked display, an uncovered interval, an unvalidated conversion or ambiguous attribution leaves M2.2
  open; it does not block unattended visual work.
- Separate callback target, actual opportunity, actor acquisition/submit, GPU timestamps, engine image time,
  movie PTS and OS-reported presented time. Record unmatched samples, source/build, content, recipe, view
  resolution, display mode and run order/load. Kernel display swaps, scan-out, input-to-photon and physical iPhone
  timing or loss stay unmeasured. Native or compositor appearance needs separate attended evidence if a specific
  observation requires it. No raw drawable bridge, screen unlock automation or mandatory Screen Recording.

M2 cannot close until **all five items** pass independent review. M2.3, M2.4 and M2.5 must pass before M2.2's
final qualification run. The witness change replaces the source of the cadence measurement; it neither waives
real-panel cadence nor extends any earlier verdict.

### M3 The post-G3t Mac surface path requalifies runtime motion and built-in cadence

Delivery prerequisite: `APPLE-TILE:M3`; module ordering also requires Mac M2 done. This milestone is split from the
former post-G3t M2 obligation, with `replans: 4`, and preserves the complete acceptance bar. M3 cannot start on T2 completion alone. Its transitive chain is delivered T2 Mac M0/Chrome M1 then G3t M0–M3,
including attended D2.5/D3.5 OS-window evidence (unlocked session and Screen Recording permission for that observer)
and diagnostic simulator endpoint/function checks. It does not wait on cooked simulator COOKED-DRAW M1 or T2 M2.
G3t retains the SDR tone/intermediate and only exposes G4 pre-tone input; the changed scheduling/input path still
requires the full requalification. Mac cadence continues to use capture OFF with its own attended panel setup.

- **M3.1 unattended runtime motion.** Rerun the complete M2.1 protocol on the integrated post-G3t-M3 release
  assembly with fresh build/shader/content/recipe identity. Apply M2.1's denied permission preflights and console
  witness policy, accepting either state and transitions. Keep the elapsed-time pan, holds,
  source frames/crops, control/submission joins and worded visual inspection. No earlier verdict qualifies new output.
- **M3.2 attended built-in cadence and observer arms.** Revalidate the M2.3 windowed-tap correspondence,
  lease/loss and capture-OFF proofs on the changed path. Rerun M2.5's qualified recording configuration and
  M2.4's command and all M2.2 arms on M3.1's build: ON untraced, ON traced and OFF traced. Fresh presented times from the
  qualified witness, covering the full interval, qualify cadence only in OFF. Keep the unlocked active panel
  reason explicit. Functional window logs alone cannot pass. A tooling regression requires a scoped re-plan;
  this milestone grants one recording per traced arm and no diagnostic recordings.

Both items need fresh independent review before final Mac qualification of the changed V1 path. Historical M2
verdicts remain attached to their build. This split does not waive cadence.
