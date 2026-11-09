# Unattended frame evidence — T2 capture

Status: Mac M0 passed its third milestone review. Chrome M1 is implemented and claimed for lead review,
9 November 2026. Simulator M2 remains open. See the [plan](T2-CAPTURE-PLAN.md) for current proof.
[Brief](T2-CAPTURE-BRIEF.md) · [plan](T2-CAPTURE-PLAN.md) · [criteria](T2-CAPTURE-DONE.json).
This is a child of [Diagnostics and delivery](TOOLS-DELIVERY.md), not a new engine subsystem or package.
The author inspected current capture code before completing this design; this is not a blind study.

## Outcome and proof boundaries

Agents must be able to produce and review the actual engine's images and evolving motion without a visible desktop,
a person at the keyboard or OS Screen Recording access. All later visual milestones use this contract, including
new recipes, scene contributors, temporal effects and engine UI. Operating range: fixed and moving cameras; paused
and running worlds; day/night and cuts; ordinary and overloaded frames; resize, loss and retirement. Each consuming
milestone still supplies its own view range and shipped-game quality bar. A technically valid capture cannot judge
whether its sea, sky or motion looks good.

| Evidence mode | Driver and output | What it can establish |
| --- | --- | --- |
| Fixed-time offscreen | Declared world/sample schedule; full-resolution lossless stills/sequence and optional 1× movie at the declared simulated rate | Image quality, evolution and deterministic/oracle comparisons; no runtime pacing claim |
| Real-time offscreen | Monotonic elapsed time drives the ordinary world lane and render actor; a declared presentation-opportunity rate replaces the display-link source | Actual application scheduling, input acceptance, interpolation, dropped views and scene motion under capture on this device; no display-link/compositor/panel claim |
| Real-time windowed tap | Ordinary platform callback and surface lease; same submitted output is also captured inside the engine | Engine image associated with actual callback/submit records; display attribution still needs independent OS evidence |
| OS/compositor or physical observation | Separate explicitly scoped adapter/tool or observer | Native/DOM UI, composition, display cadence or physical output, only to the extent measured |

Real-time offscreen uses elapsed host time, including oversleep and stalls. Never increment a synthetic frame counter
as the world clock in this mode. Fixed-time movies can be perfectly smooth when rendering is slow; they cannot close
a live-motion item. A requested 120 Hz opportunity stream is a workload, not an observed 120 Hz display result.

## Mechanism and ownership

T2 supplies the scenario, artifact writer and result validator in existing tool/script infrastructure. It requests a
capture from the existing app/renderer seam; it neither creates a device nor submits its own queue work.
`thrum-render::FrameHost` owns an optional submitted-output tap and generation-owned GPU/readback resources.
`thrum-app` owns headless and windowed drivers, worker/lifecycle policy and delivery of control requests. Core's
existing clock/world lane remains the only authority clock. The game binds scenario actions to its normal controls;
the engine does not learn about V1 yaw, Sun presets or cooked valley schema.

The headless runtime adapter replaces only the OS opportunity/surface source. Retain ordinary threaded world work,
render-actor admission, interpolation, two-frame limit, overload/drain and history rules. Supply an engine-owned output
target through the ordinary presenter and `SurfaceOutput` path instead of a drawable, and record that substitution.
`FrameHost::finish_advance` currently skips a no-tick advance without a surface; supplying this output is essential
for paused-world camera edits to produce fresh submissions. Do not add a capture-only admission branch or advance
the paused world to force rendering. This proves shared runtime motion; AppKit display-link
routing, window loss, picking/focus and platform composition remain separate qualification. Factor shared dispatch
when needed: headless requests enter the same validated semantic edit handler, while windowed tests can still invoke
the real AppKit slider target/action. Log requested, accepted and submitted values independently.

Walk one frame:

1. The scenario becomes due on its declared timebase and requests a normal control edit. The world lane admits ticks
   through its existing clock. The render actor receives an opportunity with actual elapsed time and declared lead.
2. FrameHost extracts/interpolates, renders and advances histories under the ordinary submitted-frame rules. After
   reconstruction, exposure, tone/gamut mapping and engine overlays, the tap copies the **same final engine output**
   to a free staging slot. It must not redraw an endpoint with `capture_pixels()` or reset history to make the image.
3. The copy is declared in the graph and encoded in the existing coordinator's ordered submission. No second queue,
   out-of-band game submit, raw drawable bridge or extra scene render is admitted. When windowed output is not copyable,
   produce one final engine texture feeding both the normal present pass and readback. Capture neither acquires nor
   extends a drawable lease. Preserve G3t's direct-output optimization when capture is disabled.
4. Completion/map callbacks carry capture-session, frame and device generations. Copy complete pixels into a bounded
   CPU buffer pool, unmap, and recycle only after both GPU and CPU ownership allow it. A dedicated writer hashes original RGBA records, then losslessly compresses consecutive-frame XOR differences
   with two Zstd level-1 workers during capture. A separate I/O worker drains a finite 256 MiB encoded reserve
   to the sequential spool; frame IDs join records to submissions. After capture/drain and durable flush,
   packaging reconstructs every original record and checks its hash before creating lossless PNGs and the host-time movie. Disk I/O and packaging happen
   off the world/render lanes. No per-frame GPU wait, pipe write or encoder backpressure on those lanes.
5. The writer atomically finalizes artifacts and their manifest after a bounded drain. Missing or invalid pixels,
   late callbacks and write/encode failure produce explicit incomplete outcomes. Never report successful evidence
   merely because a movie file exists. Device-loss cancellation needs no lost-fence wait or old-queue flush.

For fixed-time mode, waiting between explicit samples is allowed and labelled; this still uses normal submitted
history for sequences. The existing endpoint-capture API retains its distinct no-history/alpha=1 semantics for still
oracles. Scene-linear checkpoints (G3t) attach an additional declared readback to this owner with explicit stage and
format. Future G4 reconstruction/EDR captures identify their post-provider output and colour encoding; a converted
SDR preview cannot certify EDR brightness or the physical display transform.

## Completeness, clocks and artifacts

The manifest pins source revision plus dirty diff digest, executable/web bundle and shader hashes, cooked root and
product hashes/generation, scenario/control/parameter hashes, run/session IDs, target/device/backend, recipe and
registry digest, extent, capture stage, channel order, row stride, colour/transfer/exposure/overlay inclusion,
clock origins, opportunity rate, duration, warm-up and load. Record actual console lock and permission preflight
status without requesting a grant. A denied OS capture preflight is metadata, not a prerequisite to engine capture.

Per opportunity/submission retain monotonic host request/accept/submit times, committed endpoint pair/timeline,
interpolation alpha, view/history/cut and content/device/surface generations, submission ID, copy/map completion and
image digest. Record skipped render opportunities, no-free-frame-slot, overload and capture-ring/writer omissions
with reasons. Capture IDs must join to the submitted image, never to whichever endpoint exists at map completion.
GPU timestamps, callback target time, submission time, capture availability, movie PTS and OS swap time are separate
fields/timebases; absent/unsupported measurements stay absent. GPU timestamp availability is not a capture requirement.

Retain full-resolution lossless source frames and time records for decisive excerpts, a contact sheet and native-size
crops, plus a convenient movie. Real-time movie PTS derive from host sample/submission time relative to the run origin,
not copy completion or a fixed output frame index. A skipped render keeps the previous image over the elapsed gap;
the player may duplicate it for encoding, but those duplicates are labelled and never counted as new render samples.
No interpolation, frame synthesis, time compression or concealed omission. Inspect the lossless sequence and timeline
when a player cannot reproduce high-refresh PTS accurately. Capture overflow means the interval's visual result is
inconclusive, even if render logs are complete; reduce a declared capture workload or improve throughput and rerun.
Do not shrink resolution or silently sample fewer frames to pass a full-resolution continuous-motion item.

A completed evidence result validates the required interval from settled initial hold through scenario transition to
settled final hold. It checks file/frame/ID coverage, hashes and timing consistency. A source clock stall, unknown gap,
misjoined frame, wrong content/recipe or unfinalized manifest refuses completion. Known repeated image content is
reported and reviewed, never rejected just for being repeated: paused/static scenes are legitimate.

## Cost and failure account

Disabled capture creates no targets, copies, mappings, worker queues or per-frame heap work. Enabled SDR readback
moves at least `aligned_row_bytes × height × captured_submissions` bytes. 1920×1080 RGBA8 is 8,294,400 bytes/frame;
120 samples/s is about 0.995 GB/s raw before encoding. A three-slot staging example holds 24.9 MB, excluding the final
texture and CPU writer queue. A ten-second raw sequence is about 9.95 GB. These are cost estimates, not budgets.
General widths need WebGPU row alignment and padding removal; channel order and odd extents need explicit tests.
HDR/linear stages cost more and must be charged separately. Unified memory does not remove bandwidth/synchronization.

Preflight a finite session duration, extent, staging/CPU-queue byte caps, output-byte allowance and free space.
Choose and record capacities before allocation. They are configurable operational limits, not frame-count semantics.
On queue exhaustion, never block the renderer: report the missing capture and incomplete interval. On output limits,
stop capture with a reason while the app remains healthy. Finalization has a deadline and retains usable incomplete
diagnostics. Do not allocate one unbounded array of the entire session or keep GPU frame slots hostage to encoding.

The first M0 feasibility step measured raw-spool throughput on this disk at 1920×1080 and 120 records/s, before
building the full scenario. Its initial three GPU staging slots and eight reusable CPU frame buffers (about 91.2 MB
combined, excluding the output texture and metadata) did not sustain this disk. The qualified M0 writer uses six
GPU slots and 96 CPU buffers under a 1 GiB pixel cap, live XOR/Zstd and a separate 256 MiB encoded reserve. Record service rate, queue high-water, actual disk writes and
drain time; a cached write-acceptance rate alone does not establish sustainable storage throughput. Include the
final flush and packaging costs. Reject a strategy that overflows over the full four-second interval (one-second
hold, two-second pan, one-second hold). If sequential raw spill cannot sustain it, revise the writer mechanism from
measurements before proceeding; do not hide the deficit in a session-sized RAM queue or reduce required fidelity.
Capacities may be tuned within explicitly recorded byte caps. M0 throughput is qualified only for its measured
scene and device; subsequent content must repeat the series.

Under [ASSET-LIFECYCLE](../../ASSET-LIFECYCLE.md), each four-second SDR session has a **16 GiB local output allowance**
covering live compressed spool (or diagnostic raw spool, about 3.98 GB), deferred PNG/movie packaging, preview
and metadata at peak coexistence. This is an operational
storage cap, not a performance gate. Run one bulk session at a time, preflight free space and declare a separate
allowance for longer/HDR cases before allocation. The task's storage record names owner, inputs, expected peak,
retained excerpts and expiry. Reuse the allowance only after verifying retained selected evidence and deleting owned
reconstructible bulk; stop on insufficient space. Do not accumulate full raw and packaged suites unaccounted.

Under the exclusive `gpu` lock, pair capture-on/off runs of the same build, scene and schedule. Record CPU/GPU work,
missed opportunities, frame intervals, memory/copy bytes, disk and encoding throughput and queue peaks; alternate order
when drift matters. Investigate visible hitches, about 25% comparable regression or unexpected growth under C12.
Readback can perturb the workload even when asynchronous: captured application timing is never an uncaptured baseline.
Keep numeric observations separate from correctness/visual completion gates.

Stall controls are explicit diagnostic scenario actions, absent from ordinary control schedules and off by default.
One action delays render-actor work; another delays input dispatch before the normal semantic handler. Each logs
requested/start/end host times and duration, and the scenario hash pins it. Test an ordinary run to prove neither
hook fires without its diagnostic action. These controls must expose real elapsed gaps in the images/PTS, not edit
timestamps after capture. Writer saturation is a separate capture-failure control.

## Research and reconciliation

Research consulted 9 November 2026; official sources were inspected for the following mechanisms and limits:

- [wgpu 30.0.1 CommandEncoder](https://docs.rs/wgpu/30.0.1/wgpu/struct.CommandEncoder.html), texture-to-buffer copy
  and mapping-on-submit APIs, and [Buffer](https://docs.rs/wgpu/30.0.1/wgpu/struct.Buffer.html), mapping/lifetime and
  web polling: support renderer-owned asynchronous readback. Mapping does not by itself supply bounded queues or provenance.
  Rechecked mapping exclusivity and texture-copy APIs for this revision; live lossless compression, bounded encoded spill and deferred PNG/movie packaging are this
  design's qualified M0 throughput strategy, not a throughput guarantee supplied by wgpu.
- [Apple ScreenCaptureKit sample](https://developer.apple.com/documentation/screencapturekit/capturing-screen-content-in-macos),
  setup, stream configuration and output processing: OS window capture requires Screen Recording permission and has
  its own queue/rate. The local denied preflight is therefore not fixed by unlocking alone.
- [Godot movie creation](https://docs.godotengine.org/en/stable/tutorials/animation/creating_movies.html), opening
  comparison and warning: offline output can hide runtime stutter. Retain fixed-time quality proof but add a distinct
  elapsed-time runtime mode rather than relabelling a fixed-step movie as live.
- [Apple presentedTime](https://developer.apple.com/documentation/metal/mtldrawable/presentedtime), discussion: this
  drawable timestamp can be zero for an unpresented/dropped frame. It is not available as a qualified observer through
  the current wgpu surface path. Do not add a raw drawable bridge just to replace Instruments. A future supported
  presentation observer needs its own attribution/lifecycle proof; it would still require an active real display.

Current code inspected: `examples/v1-game/src/bin/window/{capture,motion,route}.rs` shares cooked content and
FrameHost but owns a game-side target/copy and waits on readback; motion steps one world tick per frame. Retain its
fixtures and image baselines, migrate sequence readback to the renderer tap, and explicitly keep fixed-time labels.
`thrum-app/src/sky/live_capture.rs` is a file-watch/reload proof with synchronous endpoint captures, not real-time
motion. Retain that behavior and label. `FrameHost::{begin_capture,try_finish_capture}` already checks owner and
generation for endpoint stills; preserve those semantics and reuse appropriate readback primitives. Browser's current
capture similarly redraws an endpoint; do not mistake it for a tap of an interpolated canvas frame.

Rejected defaults: ScreenCaptureKit/desktop screenshots retain permission/session dependency; fixed-step video alone
conceals runtime pacing; standalone game readback duplicates GPU lifetime/submission ownership. A new native video
encoder path or complete Metal recorder is not needed for the first proof. Live XOR/Zstd reduces storage demand while deferred PNG/movie packaging removes PNG throughput from the
critical path. M0 margin applies to its measured scene compressibility. Each M1 content workload needs its own
back-to-back capture series and sink-rate refusal measurements before throughput is qualified.

Locked-console feasibility is supported by the [retained design review's probe account](../../evidence/T2-CAPTURE/design-review.md#what-checks-out):
it read `tmp/platform-runtime-mac-m2/console-probe.txt` at 01:59 CET (`IOConsoleLocked = Yes`,
`CGSSessionScreenIsLocked = Yes`) and the successful 02:00 CET [offscreen parity record](../../evidence/render/mac-m2-pan/parity-20261109.json).
The second review locates probe scratch in the separate platform-runtime-mac worktree; the parity JSON alone
does not record lock state. This is historical feasibility evidence, not a fresh T2 run or runtime-motion
qualification. M0 records lock state alongside
its own run, opportunistically when the console is already locked. Never lock an active user's session to arrange
the test and never unlock it by automation. Until that condition occurs, the locked row stays open and other work proceeds.

## Rollout and attended boundaries

M0 delivers Mac unattended still/fixed-time/real-time capture and the reusable engine tap. Mac qualification M2 consumes
it on the current surface path, without waiting for Chrome, simulator rollout or parked APPLE-TILE. Mac M3
needs `APPLE-TILE:M3` and owns fresh post-change motion/cadence requalification; that later obligation cannot
gate Mac M2. Mac M2.3 owns integration/qualification of this same tap on a real
CAMetalLayer surface: known-pattern output correspondence, lease lifetime, capture-off direct presentation and
pending-capture surface loss. It needs attended unlocked setup and does not block unattended M0. G3t M0 consumes
M0 for shared output capture and adds its own scene-linear comparison cases. M1 and M2 of this tooling plan qualify Chrome and iOS simulator respectively;
new submitted-output/runtime-motion work on each target needs that target's milestone. Until then, existing Chrome
canvas endpoint captures remain valid for fixed-time image/oracle claims with endpoint/no-history provenance;
`simctl` screenshots/video remain valid for simulator function and visible UI/lifecycle observations within their
recorded timing limits. Neither substitutes for exact submitted temporal output, elapsed-time engine motion or
physical display proof. Keep those stronger rows dependent/unavailable. No dependency runs back from capture to G3t/G4/Mac M2.

| Check | Attendance and reason |
| --- | --- |
| Engine image/effect evolution and application motion on Mac | Unattended by design; M0 must actually prove a locked-console, permission-unavailable run before claiming this |
| Chrome canvas / engine-rendered UI | Unattended after M1 proves the real WebGPU adapter while locked; browser scheduling is labelled, never Mac panel evidence |
| iOS simulator engine output | Unattended after M2 proves the route while locked; `simctl` image/video is supplemental and never physical-device timing |
| Mac M2 windowed tap and built-in-panel swaps/display-link qualification | **Attended setup**: unlocked active built-in display and any Instruments authorization required by the OS. Mac M2.3 qualifies the tap; M2.4 commissions bounded trace-finalization diagnosis. M2.2 uses a capture-off traced arm for cadence and separate capture-on content/observer arms. Agent injects inputs, runs and reviews; owner need not play. No Screen Recording grant is required for engine pixels |
| Composited/native widgets, OS focus/input, browser chrome | **Attended when the target/tool requires it**: engine output excludes these surfaces. Prefer OS automation without screen recording; if a compositor image is essential, declare that adapter's OS grant/session requirement before the row |
| Physical HDR/EDR brightness, input-to-photon or physical-device interaction | **Attended/instrumented**: needs actual display/device and correlated observation or sensor; an SDR movie and GPU completion cannot prove it. No physical iPhone is available; keep that proof pending rather than asking for it |

The locked-console experiment establishes feasibility on a named configuration, not a promise to bypass logout,
sleep, power loss or arbitrary OS GPU suspension. An unavailable GPU/tool returns unavailable, never a success or
an instruction to weaken system security. A newly discovered attended exception names the exact failed preflight,
required observation and why an engine/tool adapter cannot supply it; it does not halt other visual work.
