# Mac frame runtime qualification

9 November 2026. Mac child of the [shared platform runtime design](PLATFORM-RUNTIME.md#g3b-world-and-presentation-lifecycle--selected-partial-implementation)
and [bounded frame host](../decisions/V1-BOUNDED-FRAME-HOST.md).
[Plan](PLATFORM-RUNTIME-MAC-PLAN.md) · [done when](PLATFORM-RUNTIME-MAC-DONE.json).
This document collects the existing Mac mechanisms and qualification boundaries, including the owner's console
witness correction. Independent review of this consolidated design is owed to the lead/loop.

## Outcome and scope

On the M3 Max with Metal and the built-in 120 Hz ProMotion display, the same cooked valley, world, window and
controls survive graphics loss. A slow noon camera pan retains coherent rock/ridge placement, the authored sphere
sweep and exit, and yaw-consistent settled endpoints. Inspect full-resolution images, native-size crops and motion
at 1× for cuts, snaps, reversals, repeats and local holds. The Ground E view identifies scene and framing. The
milestone's worded inspection list and shipped high-end game quality bar govern visual acceptance.

M0/M1 retain their accepted plan/recovery evidence. M2 qualifies the current surface path through elapsed-time
engine motion, real-window tap correspondence and independently attributed built-in-panel swaps. M3 repeats the
complete qualification after APPLE-TILE M3 changes the surface/present path. Each result identifies its actual
build, content and recipe. The done-when file owns criteria and proof status.

## Ownership and lifetime

Core's `ClockedWorld` and native `WorldLane` own authoritative time and tick admission under
[C02](../CONTRACTS.md#c02-time-and-committed-state--f1-coordinator-w1-world). AppKit owns the window, view and
display-link delivery. The existing `thrum-app` render actor owns surface configuration, late drawable acquisition,
presentation calls and serialized device reconstruction. Display-link target time supplies interpolation lead.
`thrum-render::FrameHost` owns bounded frame slots, submission, completion and submitted history under
[C06](../CONTRACTS.md#c06-gpu-execution-and-lifetime--v1) and
[C07](../CONTRACTS.md#c07-render-scene-views-and-temporal-data--v2-with-p4v3v4).

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
per-frame capture allocations. T2 retains bounded readback/writer ownership under [C12](../CONTRACTS.md#c12-budgets-evidence-and-acceptance--t2-with-all-owners).

M2.4 owns the bounded trace-finalization investigation. Inspect retained failures and document a changed falsifiable
hypothesis before recording. The existing 9 November allowance permits at most two new diagnostic attempts total,
each with 16 seconds recording, at most 75 seconds finalization and finite export timeouts. A readable fresh TOC
and displayed-surfaces-interval export must identify actual app/window swaps. Exhausted attempts, unavailable setup
or an unjustified hypothesis leave the tooling and cadence rows blocked. Further attempts need a scoped re-plan.

After M2.3/M2.4, M2.2 runs matched capture-ON/untraced, capture-ON/traced and capture-OFF/traced arms using the same
release build, cooked root, recipe, extent and normal AppKit slider target/action sweep. ON arms retain complete
content motion and expose capture/tracing overhead. The OFF arm supplies shipping cadence through numeric Swap Time,
unique game PID/window/built-in-display attribution and resolved rotating surface identities. Same-process markers
or a calibrated monotonic join with stated error must cover the full pan and holds. Ambiguous joins or unreadable
exports leave cadence open. Record run order, load, intervals, gaps and unmatched samples.

These window/panel/tool rows need an unlocked active built-in display and any required Instruments authorization.
Engine pixels require no Screen Recording grant. OS-window/compositor observation, when separately required by
APPLE-TILE, has its own attended permission identity. Callback targets, opportunities, submission, GPU completion,
image times, movie PTS, VSync and attributed swaps remain distinct measurements. Input-to-photon and physical
iPhone qualification require their own evidence.

## Cost, sequence and review

Reuse the existing world lane, render actor, FrameHost and T2 producer. Capture is opt-in with bounded asynchronous
readback and writer resources. Record complete observer costs and investigate visible hitches or comparable
regressions under the existing performance policy. Timing figures remain observations under this qualification.

M2 needs only `T2-CAPTURE:M0` and can complete on the current path. In module order M3 follows Mac M2 and needs
`APPLE-TILE:M3`, then reruns motion, window-tap/tooling validation and all cadence arms on the integrated build.
Its trace work uses the existing allowance. COOKED-DRAW and T2 simulator M2 retain their separate ownership and
qualification. Independent review of every required item closes each qualification milestone.
