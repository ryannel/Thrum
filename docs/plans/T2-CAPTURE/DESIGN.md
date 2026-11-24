# Unattended frame evidence — T2 capture

[Brief](BRIEF.md) · [plan](PLAN.md) · [criteria](DONE.json) · [log](LOG.md).
This is a child of [Diagnostics and delivery](../../design/modules/TOOLS-DELIVERY.md), not a new engine subsystem or package.
The author inspected current capture code before completing this design; this is not a blind study.

## Outcome and proof boundaries

Agents must be able to produce and review the actual engine's images and evolving motion without a visible desktop,
a person at the keyboard or OS Screen Recording access. All later visual work packages use this contract, including
new recipes, scene contributors, temporal effects and engine UI. Operating range: fixed and moving cameras; paused
and running worlds; day/night and cuts; ordinary and overloaded frames; resize, loss and retirement. Each consuming
work package still supplies its own view range and shipped-game quality bar. A technically valid capture cannot judge
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
   off the world/render lanes. No per-frame GPU wait, pipe write or encoder backpressure on those lanes. The one
   declared exception is a capture that selects `wait` admission ([a full ring](#a-full-ring-and-a-run-that-could-not-judge)).
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
clock origins, opportunity rate, duration, warm-up and load. Record actual console state before and after as witness
metadata, accepting locked, unlocked and changed state, with unavailable observations recorded with a reason.
Open no-desktop-capture proofs require denied read-only Screen Recording preflights before and after under the
identified launcher/harness permission identity. Missing, failed or granted preflights cannot establish that claim.
No permission is requested or changed. Neither console observation values nor manual locking gate a run or pass.
Accepted WP0/WP1 definitions and their recorded lock witnesses retain their historical meaning.

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
On queue exhaustion the tap never blocks the renderer. Under `omit` admission, the default, it reports the missing
capture and an incomplete interval; under `wait` the driver holds the next opportunity back instead
([a full ring](#a-full-ring-and-a-run-that-could-not-judge)). On output limits,
stop capture with a reason while the app remains healthy. Finalization has a deadline and retains usable incomplete
diagnostics. Do not allocate one unbounded array of the entire session or keep GPU frame slots hostage to encoding.

The first WP0 feasibility step measured raw-spool throughput on this disk at 1920×1080 and 120 records/s, before
building the full scenario. Its initial three GPU staging slots and eight reusable CPU frame buffers (about 91.2 MB
combined, excluding the output texture and metadata) did not sustain this disk. The qualified WP0 writer uses six
GPU slots and 96 CPU buffers under a 1 GiB pixel cap, live XOR/Zstd and a separate 256 MiB encoded reserve. Record service rate, queue high-water, actual disk writes and
drain time; a cached write-acceptance rate alone does not establish sustainable storage throughput. Include the
final flush and packaging costs. Reject a strategy that overflows over the full four-second interval (one-second
hold, two-second pan, one-second hold). If sequential raw spill cannot sustain it, revise the writer mechanism from
measurements before proceeding; do not hide the deficit in a session-sized RAM queue or reduce required fidelity.
Capacities may be tuned within explicitly recorded byte caps. WP0 throughput is qualified only for its measured
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

## A full ring, and a run that could not judge

Added 16 November 2026, for WP3. On that day the main-branch check `exact-images` failed four times at a realtime
motion arm of `APPLE-TILE`'s matrices, at a different arm each time and twice on one commit: `portable-variants-motion`,
`apple-tile-route-motion`, `portable-route-motion` and `apple-tile-render-stall-motion`. The retained record of one
reports `complete: false` with 129 ring omissions, none from the writer, 419 images and 547 GPU records, while five
modules' GPU tests ran beside it ([handoff](../../../.orchestrate/evidence/REVIEW-ENGINE/WP3-handoff.md)). Those tests run inside
`./dev check`, which takes no lock, so no lock keeps them away. The completeness rule was right to refuse the run.

**The cause, to be confirmed first by WP3.1.** A realtime capture asks for 120 opportunities a second on elapsed
host time. Each submission reserves one of six staging slots until its readback is mapped and copied out. When
other GPU work delays readbacks, all six stay reserved, the tap admits no copy, and the frame is submitted without
one: an omission. These arms (`variants`, `route`, `render-stall`) are not on a fixed clock. `route` and the stall
control are refused in `fixed` mode, because a running world and a stall need real elapsed time. A `fixed` or
`still` capture already waits for each image and cannot lose one this way.

**Admission is a declared policy of a realtime capture: `omit` or `wait`.**

- **`omit`** is the behaviour above and stays the default. Only an `omit` run may support a claim about
  scheduling, cadence, dropped views or observer cost, because the capture never holds the render owner back.
- **`wait`.** The driver does not take a presentation opportunity while the tap can admit no copy. It keeps
  draining completed readbacks and returned writer leases until the tap can admit one, then takes the next
  opportunity the elapsed clock offers. So the ring is never full at a submission, every submitted frame has its
  image, and load shows as fewer submissions, further apart.
- **It stays honest.** A wait is the same kind of gap the render-stall control makes. The world lane keeps its
  own clock, each image keeps its host submission time, and movie PTS show the gap. Nothing is re-timed,
  synthesised or dropped, so a slow run still looks slow, as the brief requires.
- **Where it lives.** In the driver that owns the opportunity loop, not inside `FrameHost`. The tap still never
  blocks, still counts an omission whenever it is asked to admit with no free slot, and that count stays the
  referee: under `wait`, a nonzero count is a defect. The completeness rule does not change under either policy.
- **The record.** `runtime.json` and the manifest name the policy. Each opportunity the driver let pass while
  it waited is counted with that reason and the time waited, apart from "driver busy or host oversleep".
- **The bound.** The wait has a deadline, as fixed mode's readback wait has. When it expires the capture ends in
  error with no completed record, as that one does: a ring that frees no slot for that long is stuck, not busy,
  and has to show as a failure.
- **Memory.** The existing staging, CPU-buffer and byte limits hold. `wait` needs no larger ring.

It covers any load under which readbacks still complete, both recipes, both capture stages and every realtime
scenario of the headless Mac driver. The windowed tap, the browser and the simulator keep `omit`: the OS owns
their opportunities, and none of them is a realtime arm of the exact-images check.

**Who selects `wait`.** The realtime motion arms of `scripts/apple_tile_m1.py` and `scripts/apple_tile_m2.py`,
under [the ruling](../ARCHITECTURE.md#ruling-on-a-capture-that-drops-frames). Those arms compare no pixels; they
assert a complete, hash-verified sequence under each recipe. Another module selects `wait` when its claim is
that every submitted image was kept, and never for a claim only `omit` supports.

**A run that could not judge.** With `wait`, load no longer empties a sequence, but a capture can still end
incomplete: the final drain or the writer can miss its deadline, and an `omit` run can still drop. That says
nothing about the commit, so it is reported apart from a failure, with exit status 75 (`EX_TEMPFAIL`):

- `scripts/capture_v1_runtime.py` exits 75 when the game wrote a runtime record that says `complete: false`. The
  manifest keeps `complete: false` and names each condition that failed: the interval, the image count, ring
  omissions, writer omissions or the writer's result.
- Each native matrix exits 75 when a capture it ran exited 75 and nothing it had compared differed.
  `scripts/exact_baselines.py verify --full` exits 75 when a matrix did. Its record says the run could not judge
  and names the capture and the condition, and is never a pass.
- Everything else stays a failure, non-zero and never 75: a difference in compared bytes or identities, a wrong
  backend, a hash or join mismatch, a crash, a missing runtime record, a process deadline and an expired wait.
- From orchestrate 2.3.6 the script tries a run that exits 75 again, up to three attempts, then keeps it as
  unjudged: it covers nothing, fails nothing and starts no bisect. Earlier versions count 75 from the full run as
  a failure, as today.
- **Known limit.** A commit that makes a capture end incomplete every time shows as unjudged on every run, and
  no bisect names it. The verifier's record names the condition for the lead, and the tap and writer tests in
  `./dev check` are what stop such a commit before it merges.

**Rejected.** A larger ring only postpones the loss: that run was short by 129 frames of about 548 in eight
seconds, and any fixed ring empties under a sustained deficit. A lock or a quiet machine: the competing tests take
no lock. A fixed clock for these arms: `route` and the stall control need elapsed time. A retry inside the capture
or the verifier: [BASELINE-GUARD](../BASELINE-GUARD/DESIGN.md) forbids one, and the script already retries a whole
run. Taking the motion arms out of the check would also remove the dependence on load, at the price of no longer
running motion on main; [the plan](PLAN.md#open-for-the-human) holds that choice for the human.

## Owned Chrome shutdown after capture

On 24 November 2026, `exact-images` on `7236c735c9f6` passed both native matrices and both Chrome day
comparisons. The night SDR page reported complete with 481 images, and the host validated and synced all 481.
A read-only comparison of the retained frames also matched every registered identity and decoded RGBA8 digest.
The verifier stopped before invoking the night comparator because the capture command exited 1. The capture
recorded a one-minute load average of about 245 at the end of the route on the 14-core Mac. The commit itself
changed only the status page.

`capture_v1_browser.mjs` starts `/usr/bin/open -n -W` for the LaunchServices permission identity.
`open` waits for the Swift app, which starts and waits for Chrome. The shared `cleanup` in
`check_v1_browser.mjs` assumes its child is Chrome. On the ordinary fixed route it sends `Browser.close`,
waits 10 seconds for the child, then records a failure and sends `SIGTERM` to `open`. It reports that wrapper
signal as Chrome's exit and skips profile removal because the error list is already nonempty. The capture
then reads the launcher's still-startup-only receipt and clears `manifest.complete`. The retained launcher
receipt eventually recorded Chrome exit status 2 and a denied postflight, and Chrome logged a teardown
watchdog expiry. The receipt available to the verifier at finalization did not contain those results.

Use the actual browser lifetime for shutdown. On the LaunchServices path, the Swift launcher is Chrome's
supervisor and stays alive to reap it and atomically record the final receipt, with the actual termination
reason and the same app's denied Screen Recording postflight. Establish ownership of the browser and its
helpers at launch, including the interval before DevTools is available. The harness requests escalation
from the live supervisor through a run-specific channel so the supervisor signals only the browser and
helpers it owns. The wrapper's exit, a CDP disconnect, and a PID read from an old receipt cannot establish
that the owned processes are gone. Keep the direct-child path for other callers of the shared helper and
preserve their current qualification and exit requirements.

Start teardown with `Browser.close` when CDP is available. Give the owned browser a bounded grace period, then
use bounded TERM and KILL escalation targeted at the owned browser and helpers still running. Keep the actual
supervisor available to reap the browser and publish its receipt even during escalation. Account for the CDP
request and delayed receipt within finite shutdown deadlines. Close the owned CDP and HTTP resources on success
and failure. Remove the run's profile once its browser and helpers are confirmed gone, including after
escalation or when the capture itself failed.

After all existing capture validation and sync have succeeded, a grace timeout and the actual termination
status during requested shutdown are diagnostics if the owned processes are gone, the profile is removed
and both attributed preflights are denied. This includes Chrome exiting on its own with status 2 after
`Browser.close` is requested. Once the final receipt confirms that exit, the command exits 0 and keeps
the manifest complete, with Chrome's status 2 recorded. Keep the shutdown outcome, escalation and actual
exit status or signal in the manifest. An unexpected browser exit before shutdown, capture or validation
failure, missing, failed or granted preflight, unconfirmed process exit, or failed cleanup still fails the
command. Preserve the original capture error when teardown also fails and record the cleanup error separately.
The verifier continues to compare the completed candidate against the same retained bytes and identities.

## Research and reconciliation

Research consulted 9 November 2026; official sources were inspected for the following mechanisms and limits:

- [wgpu 30.0.1 CommandEncoder](https://docs.rs/wgpu/30.0.1/wgpu/struct.CommandEncoder.html), texture-to-buffer copy
  and mapping-on-submit APIs, and [Buffer](https://docs.rs/wgpu/30.0.1/wgpu/struct.Buffer.html), mapping/lifetime and
  web polling: support renderer-owned asynchronous readback. Mapping does not by itself supply bounded queues or provenance.
  Rechecked mapping exclusivity and texture-copy APIs for this revision; live lossless compression, bounded encoded spill and deferred PNG/movie packaging are this
  design's qualified WP0 throughput strategy, not a throughput guarantee supplied by wgpu.
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

Consulted 16 November 2026, for WP3:

- [OBS Studio's status indicators](https://obsproject.com/forum/resources/status-indicators-and-what-they-mean.957/updates)
  and its forum's reports: a realtime recorder counts frames "missed due to rendering lag" when it cannot get
  frames off a loaded GPU in time, and the remedy offered is less load. A recorder that must not hold its source
  back drops under load by construction, which is `omit`.
- [Unreal's Movie Render Pipeline](https://dev.epicgames.com/documentation/en-us/unreal-engine/movie-render-pipeline-in-unreal-engine),
  overview: Epic's offline image-sequence renderer, which
  [a course page](https://www.studiomatrx.org/students/real-time-vr-for-architecture/movie-render-queue-and-output)
  describes as decoupled from the frame budget. Epic's page does not say how it handles readback, so this is a
  reading, not a source: a recorder that must keep every frame holds its source back, which is `wait`. Its fixed
  time step is not copied, because these arms need elapsed time.
- [BSD `sysexits.h`](https://stuff.mit.edu/afs/athena/astaff/reference/4.3network2/include/sysexits.h):
  `EX_TEMPFAIL` is 75, a temporary failure the caller is invited to retry, which is what orchestrate 2.3.6 reads
  it as.

Code read for WP3: the opportunity loop and completeness rule in `examples/v1-game/src/window/capture.rs`, the
elapsed clock in `crates/thrum-app/src/offscreen/clock.rs`, the tap in
`crates/thrum-render/src/runtime/submitted_capture.rs`, `scripts/capture_v1_runtime.py`, `APPLE-TILE`'s
matrices and `scripts/exact_baselines.py`.

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
critical path. WP0 margin applies to its measured scene compressibility. Each WP1 content workload needs its own
back-to-back capture series and sink-rate refusal measurements before throughput is qualified.

Locked-console feasibility is supported by the [retained design review's probe account](../../../.orchestrate/evidence/T2-CAPTURE/design-review.md#what-checks-out):
it read `tmp/platform-runtime-mac-m2/console-probe.txt` at 01:59 CET (`IOConsoleLocked = Yes`,
`CGSSessionScreenIsLocked = Yes`) and the successful 02:00 CET [offscreen parity record](../../../.orchestrate/evidence/render/mac-m2-pan/parity-20261109.json).
The second review locates probe scratch in the separate platform-runtime-mac worktree; the parity JSON alone
does not record lock state. This is historical feasibility evidence, not a fresh T2 run or runtime-motion
qualification. Accepted WP0/WP1 subsequently retained locked-run witnesses with their own proof.
The owner's console-witness correction applies to open work and later consumers. Record state during the actual
run without arranging or waiting for a locked console. Capture independence rests on the denied, attributed
permission preflights and the actual engine-output path. Never lock an active user's session to arrange a test
or unlock it by automation.

## Rollout and attended boundaries

WP0 delivers Mac unattended still/fixed-time/real-time capture and the reusable engine tap. Mac qualification WP2 consumes
it on the current surface path, without waiting for Chrome, simulator rollout or parked APPLE-TILE. Mac WP3
needs `APPLE-TILE:WP3` and owns fresh post-change motion/cadence requalification; that later obligation cannot
gate Mac WP2. Mac WP2.3 owns integration/qualification of this same tap on a real
CAMetalLayer surface: known-pattern output correspondence, lease lifetime, capture-off direct presentation and
pending-capture surface loss. It needs attended unlocked setup and does not block unattended WP0. G3t WP0 consumes
WP0 for shared output capture and adds its own scene-linear comparison cases. WP1 and WP2 of this tooling plan qualify Chrome and iOS simulator respectively;
new submitted-output/runtime-motion work on each target needs that target's work package. Until then, existing Chrome
canvas endpoint captures remain valid for fixed-time image/oracle claims with endpoint/no-history provenance;
`simctl` screenshots/video remain valid for simulator function and visible UI/lifecycle observations within their
recorded timing limits. Neither substitutes for exact submitted temporal output, elapsed-time engine motion or
physical display proof. Keep those stronger rows dependent/unavailable. No dependency runs back from capture to G3t/G4/Mac WP2.

| Check | Attendance and reason |
| --- | --- |
| Engine image/effect evolution and application motion on Mac | Accepted WP0 includes locked, permission-denied capability proof. Open consumer rows require denied, attributed Screen Recording preflights and record either console state as a witness |
| Chrome canvas / engine-rendered UI | Accepted WP1 includes real WebGPU and locked-run proof. Later open rows record either console state with denied, attributed preflights; browser scheduling is labelled, never Mac panel evidence |
| iOS simulator engine output | WP2 proves the route with denied, attributed preflights; either console state is witness metadata. `simctl` image/video is supplemental and never physical-device timing |
| Mac WP2 windowed tap and built-in-panel swaps/display-link qualification | **Attended setup**: unlocked active built-in display and any Instruments authorization required by the OS. Mac WP2.3 qualifies the tap; WP2.4 commissions bounded trace-finalization diagnosis. WP2.2 uses a capture-off traced arm for cadence and separate capture-on content/observer arms. Agent injects inputs, runs and reviews; owner need not play. No Screen Recording grant is required for engine pixels |
| Composited/native widgets, OS focus/input, browser chrome | **Attended when the target/tool requires it**: engine output excludes these surfaces. Prefer OS automation without screen recording; if a compositor image is essential, declare that adapter's OS grant/session requirement before the row |
| Physical HDR/EDR brightness, input-to-photon or physical-device interaction | **Attended/instrumented**: needs actual display/device and correlated observation or sensor; an SDR movie and GPU completion cannot prove it. No physical iPhone is available; keep that proof pending rather than asking for it |

The locked-console experiment establishes feasibility on a named configuration, not a promise to bypass logout,
sleep, power loss or arbitrary OS GPU suspension. An unavailable GPU/tool returns unavailable, never a success or
an instruction to weaken system security. A newly discovered attended exception names the exact failed preflight,
required observation and why an engine/tool adapter cannot supply it; it does not halt other visual work.

## Ownership and sequence

Moved from the module plan on 11 November 2026, when the plans took the orchestrate 1.7.0 shape.

T2 owns this child work of TOOLS-DELIVERY; no new crate. Build sequentially. Existing committed pages/world lane,
FrameHost, cooked content, platform loss/lease and scene/controls are available now. WP0 must preserve
`PLATFORM-RUNTIME-MAC:WP1` (done). WP1/WP2 here follow WP0; no dependency on G3t/G4 or Mac qualification WP2.
Mac qualification WP2 needs only `T2-CAPTURE:WP0`; Mac WP3 separately needs `APPLE-TILE:WP3` for post-change requalification.
Later visual work packages use WP0 on Mac,
WP1 on Chrome and WP2 on the simulator at the first proof requiring this tooling. Existing completed evidence is not
invalidated. Captures cannot close an attended panel item.
Mac WP2.3 commissions and proves integration of the shared tap on a real CAMetalLayer window; Mac WP2.4 owns bounded
trace-finalization diagnosis. Neither task is hidden inside unattended WP0, and neither adds a reverse dependency.
Before Chrome WP1/simulator WP2, existing canvas endpoint images may prove fixed-time endpoint/oracle claims and
`simctl` captures may prove simulator function/UI/lifecycle within their recorded limits. New submitted-temporal or
elapsed-time engine-motion claims still need the target's T2 work package.

Implementation ownership: `thrum-render` owns graph tap, targets, staging, fences and generation cancellation;
`thrum-app` owns headless/windowed scheduling and normal control dispatch; core keeps its existing world clock;
existing scripts own manifests, lossless/movie packaging and validation. V1 keeps only authored scenarios, presets,
content and bindings. G3t owns linear recipe comparison and G4 owns output-provider/EDR qualification; both use this
capture service rather than copying it. Follow CONTRIBUTING and the architecture registry if dependencies change.

WP3 follows WP2 and waits on no other module. Its builder also edits `APPLE-TILE`'s matrices and
`BASELINE-GUARD`'s verifier, only as [the ruling](../ARCHITECTURE.md#ruling-on-a-capture-that-drops-frames)
allows, and records the changed pins as that ruling says. It changes no retained reference, compared checkpoint,
sampling or equality rule. If the loss WP3.1 reproduces is not a full ring, the builder ends `BLOCKED` with the
record, because the mechanism above rests on that cause.

## Proof, retention and review handoff

Moved from the module plan on 11 November 2026, when the plans took the orchestrate 1.7.0 shape.

Run `./dev check` for implementation, focused real-device/lifecycle and negative-control checks under `gpu`, and the
new documented scenario commands. Checks must fail on unavailable required hardware. This planning revision uses
`./dev check-docs`. Exact source images, sample logs and decisive failure controls live in `tmp/t2-capture-<work-package>/`
with a storage record, byte allowance and closeout condition. Promote compact reproducible proof and selected excerpts
to `.orchestrate/evidence/render/` under ASSET-LIFECYCLE; ignored scratch alone is not durable selected evidence.
The default four-second SDR session allowance is 16 GiB including raw/packaged coexistence; run bulk sessions
serially and verify retained selected roots before reusing their space. Longer/HDR sessions declare their own cap.

The design reviewer starts with the brief and independently derives coverage before reading the design/JSON. Check
single ownership, final submitted-image correspondence, real-time versus synthetic clocks, output-stage/UI limits,
nonblocking bounded readback and the attended Mac cadence boundary. No proof item is passed by this plan.
