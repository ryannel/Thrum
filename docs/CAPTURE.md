# Engine-output capture on Mac

`scripts/capture_v1_runtime.py` records the ordinary V1 game through the renderer's submitted-output tap.
It requires a real Metal adapter, Python 3, Swift/CoreGraphics for a read-only permission preflight and
FFmpeg/FFprobe and the `zstd` CLI (independent source verification). It never requests Screen Recording permission or changes console lock. Unavailable hardware
and incomplete intervals fail explicitly.

Build and cook in this worktree:

```sh
rustup run stable cargo build --locked --offline --release -p v1-game
rustup run stable cargo build --locked --offline -p thrum-cook
target/debug/thrum-cook cook examples/v1-game/assets/showcase tmp/t2-capture-m0/valley
```

Set `ORCHESTRATE_LOOP` to the `build/scripts/agent_loop.py` beside the installed orchestrate
`build-milestone` skill directory. This location comes from the active skill installation.
Run the cooked noon pan with a new output directory:

```sh
python3 "${ORCHESTRATE_LOOP:?Set ORCHESTRATE_LOOP to the installed orchestrate build/scripts/agent_loop.py}" lock gpu --worktree . -- python3 scripts/capture_v1_runtime.py --binary target/release/v1-game --store tmp/t2-capture-m0/valley --output tmp/t2-capture-m0/pan --case pan --mode realtime
```

The explicit `--worktree .` also works around this installed lock script's parsing of the command separator.
Do not run compilers, cookers or other disk-heavy jobs concurrently with qualification/cost runs; the GPU lock
does not serialize disk traffic.

## Modes and controls

Use `--mode still` for one settled submitted image, `--mode fixed` for synthetic camera samples and
`--mode realtime` for actual monotonic elapsed time, including oversleep and stalls. Fixed samples may wait
between readbacks; their smoothness cannot prove runtime motion. Existing `THRUM_CAPTURE_SCENE` and
`FrameHost::capture_pixels` retain separate endpoint/no-history/alpha-one oracle semantics.
`--case checkpoint --mode fixed --params <file>` retains the first settled submission through the
fixed timebase with the chosen camera and game controls.

Mac realtime capture defaults to `--admission omit`. Opt-in `--admission wait` drains readbacks and writer
leases before taking another opportunity when all six staging slots are reserved. It preserves each actual
submission time and counts admission-wait gaps separately in `runtime.json` and the manifest. It can prove
that every submitted image was retained; only `omit` can qualify scheduling, cadence or observer cost.
The exact-image matrices select `wait` for their realtime motion arms; their fixed comparisons are unchanged.
The browser, simulator and windowed capture keep their existing admission.

A Mac runtime record with `complete: false` exits **75**, keeps an incomplete manifest and names the failed
conditions. Native matrices and the full exact-image verifier propagate that unjudged result. It covers no
commit and is never a pass. Compared-byte/identity differences, invalid joins/hashes/backend, crashes, absent
records and expired process/admission deadlines remain failures. A failure encountered before an incomplete
capture wins. There is no retry within a capture or verifier. The admission wait deadline is ten seconds.
`THRUM_CAPTURE_HOLD_FULL_MS=350` is a declared diagnostic that withholds image collection until the ring fills,
then releases it after 350 ms; it is disabled by default. Reproduce the bounded controls with
`scripts/capture_admission_proof.py ring-full` or `loaded` under the GPU lock.

The pan requests 120 opportunities/s at 1920×1080, holds yaw zero for one second, turns to 36° over two
seconds, then holds the accepted final state for at least one second. The world remains paused. A real
initial submission anchors zero; earlier request/accept observations retain signed times. Missing opportunity
indices name driver busy/oversleep. Submitted-image omissions invalidate continuous proof. Controls enter
normal parameter validation and `Game::apply`, the handler used by the native panel. The engine adapter
owns scheduling and supplies an engine texture through ordinary presenter/`SurfaceOutput` admission.
World work remains on its resident lane; rendering runs on the dedicated render-owner thread.
The engine opportunity clock targets the next 120 Hz opportunity. `Offscreen::step_for` consumes that target
with the native actor's shared `display_target_lead` policy (expired target: zero, maximum: 50 ms).
The offscreen runner reads the world clock once: that observation both measures the lead and advances/observes
the world. The host stamps `observation_ns`, `presentation_lead_ns`, `display_phase`, `alpha` and
`clock_interpolating` on each submitted image's sample, together with `tick_due_time_ns` while interpolating
(null otherwise). Observation, platform target and tick due time are host times; lead is a host duration.
The V1 time record also writes `platform_target_ns`
(the runner's scheduled opportunity target in the host origin) and `lead_status`: `future`, `late`,
`invalid` or `clamped`. Untargeted prime/warmup steps declare zero lead and no platform target/status.
The consumed target is observation plus lead: a future frame consumes the platform target, a late or invalid
frame consumes its observation, and a clamped frame consumes observation plus 50 ms. The display phase is
the fraction since the committed tick at that consumed target, capped at one plus 50 ms in ticks; alpha is
the phase held to one. When the clock is paused, overloaded, holding or queued, `clock_interpolating` is false
and both are one; those frames do not describe a consumed target. Endpoint oracle captures likewise give one.
Counts of lead statuses, bound phases and stopped clocks retain those frames rather than silently dropping them.
The tick's due time `t_C` gives the span's origin `O = t_C - C * d`, where `d` is the tick duration.
World display time is `(C - 1 + phase) * d`, or, below the bound, `(C - 1) * d + observation + lead - t_C`.
It is not a host timestamp: equivalently it is the consumed target less `O` and one tick. Equal origins
identify one running span; within it display-time differences equal consumed-target differences.
Across origins subtract the origin difference too, excluding paused/overloaded time and the discarded
part-tick. Stopped, queued and held frames show `C * d`. Counts retain origin changes as well.
Opportunity records also retain requests that produced no submitted frame, for example while an optional world
publication is unavailable. Those requests are distinct from omissions in the submitted-image tap.
This is an offscreen target policy, not a measured display target or a claim that its alpha equals a window's.

Additional cases use the same binary:

- `--case running`, without `--store`: moving diagnostic grid, with a running 60 Hz world between holds.
- `--case render-stall`: diagnostic 250 ms render-owner delay before the ordinary step.
- `--case input-stall`: diagnostic 250 ms dispatch delay before semantic handling; renders continue with held controls. Intermediate edits are coalesced to the newest value; requested/start coincide at diagnostic activation.
- `--case writer-slow`: running-world slow output, delaying 4 MiB writer calls 500 ms. Use `THRUM_CAPTURE_ENCODING=raw` for the saturation/refusal control; the compressed diagnostic grid can fit the finite encoded reserve and finish successfully.
- `--case off`: matched pan with capture disabled; no tap buffers or writer queue.
- `--case variants`: semantic noon/night/noon edits at one and three seconds, using the prepared star variants.
- `--case route --route examples/v1-game/routes/moving-v1.json`: validates the cooked root and runs the checked route on the resident world lane. Use a route copy with a 1920×1080 render extent for this full-resolution driver. The initial hold precedes normal world resume, and the final hold follows the last authored tick.
- `--case exit-route --route examples/v1-game/routes/exit-flythrough.json` and `--case exit-drift`: in
  real time these request 60 Hz with GPU pass timing disabled. Each frame samples the actual elapsed
  observation and records submission time, tick and interpolation. `--observer-only` runs the same scenario
  without image readback or a spool writer. The driver resumes a world that overloaded and retains that
  interval as incomplete evidence.

`--params PATH` selects a checked parameter file before warmup. `--source scene-linear` captures the existing
RGBA16Float scene colour after its last writer, including stars at night. It preserves all eight stored bytes
without float conversion. This source uses 32 CPU buffers and verification-only packaging. The default
`--source sdr` retains the submitted RGBA8 output and ordinary PNG/movie packaging. Both sources include the
actual GPU exposure packet as eight raw u32 words in each image's provenance. Its fields are storage scale,
inverse gain, EV100, lux, flags, submission serial low/high and display selector.

The complete APPLE-TILE M0 command, its fixed controls and retained sources are documented in
[the M0 evidence](../.orchestrate/evidence/render/apple-tile-m0/README.md).

Ordinary pan/running/off schedules contain no diagnostic actions.

For Weather and Shade, `THRUM_CLOUDS=shade` selects the real game contribution.
`THRUM_CLOUD_WIND` sets initial X wind in m/s and `THRUM_CLOUD_TIME_SCALE` sets
weather speed (1–60), both validated before world construction. With the cooked
valley, `THRUM_CAMERA_HEIGHT` selects the camera's absolute height in metres.
`THRUM_CAMERA_Z` optionally moves that elevated camera along the valley's Z axis.
`--case clouds-still --mode fixed` retains one held submitted checkpoint;
`--case clouds-drift --mode realtime` holds the view, runs the ordinary world
from one to 21 seconds, then pauses for a final hold. Its explicit 4,096-record
writer/decoder limit admits twenty seconds at 120 opportunities/s.

`THRUM_CLOUD_PRESET` selects `clear`, `fair`, `broken` or `overcast` when clouds
are enabled. The weather captures hold the initial Sun and camera through a
running world. Use `clouds-wind` at 120 opportunities/s for a 20-second drift
take. `THRUM_CLOUD_SEQUENCE=wind-turn` turns the wind 90° after ten world
seconds. Use `clouds-cycle` with `THRUM_CLOUD_SEQUENCE=presets` for the full
four-minute sequence. `clouds-life` at 8× and `clouds-life-fast` at 60× show a
full procedural cloud lifetime in a held view. The four-minute takes admit
16,384 records at 60 opportunities/s. Pixel queues, encoded queues and the
output-byte bound stay the same. Use a small extent for these longer takes.
For Chrome, a cloud view can supply `preset`, `weather_rate` and `wind`, plus
`duration_seconds` between 1 and 60 for an elapsed-time take.

`--case clouds-route --route PATH --mode realtime` records a cloud inspection
route in the same versioned route format. It admits at most 1,800 simulation
ticks including preparation and drain, with 4,096 submitted records and a
34.5-second host deadline. The existing `route` case keeps its 560-frame
admission and timing. This native inspection case supports the ten-second pan,
40 m walk and cut required by CLOUDS M3; pose, cut, cooked-root and checkpoint
validation still apply. The route sets solar time and camera, and the ordinary
world runs at 1x unless the selected parameters explicitly change it.

## Artifacts and bounds

`runtime.json` joins opportunities, requested/accepted controls, exact submission IDs, adjacent endpoint ticks,
timeline, interpolation alpha, consumed presentation lead, history epoch, device generation, engine presenter
generation, map observations and GPU/CPU samples. The renderer stamps device and presenter identities at actual
submission; presenter generation advances on prepare/resize/device publish and does not identify an OS lease.
Capture
is of the scene grid: surface viewport resize does not relabel old pixels as a new-sized screenshot.

`manifest.json` pins source revision, dirty diff and source files (including untracked implementation), binary,
shaders, cooked products, scenario/parameters, recipe/features, colour/stage and lock/permission observations.
The current V1 route has no independent selected recipe registry; its resolved recipe digest covers the route,
capability record and shaders. Native controls and OS composition are excluded.

The graph copies the actual final SDR engine texture in the ordinary submission, retaining temporal history.
There is no endpoint redraw, extra scene render, second queue or drawable acquisition/lease extension. Six GPU
slots and 96 reusable CPU buffers use 846,028,800 pixel bytes plus 192 GPU staging bytes for exposure within a 1 GiB allowance. This is a bounded
0.8-second CPU burst reserve at 120 records/s, not the four-second session in RAM. Earlier raw-spool
profiles overflowed on this normally 94–97%-full volume. The current writer hashes each original RGBA record,
XORs it with its predecessor, then uses Zstd level 1 with two workers and a 4 MiB window. It preserves every
channel exactly. The ARM SHA-256 path is enabled; the software-only hash was too slow in the feasibility probe.
Compression and disk I/O have separate workers. A finite 64 × 4 MiB encoded-buffer pool absorbs the moving
portion's byte burst; two full-frame codec scratch buffers and codec allocations are additional to pixel leases.
The encoded reserve is sized from the observed ~580 MB / two-second pan burst at a 200 MB/s disk cap, rather
than from the four-second raw session. The inherited file is `F_NOCACHE`; final `fsync` and `F_FULLFSYNC` are
charged after capture. This is not a guarantee on arbitrary scenes or slower disks: run the same series on a new
workload/device. Record all attempts, including refusals; never silently retry a failed continuous proof.
Nonblocking pixel admission and generation checks keep slow/failed capture from stopping the app. Exhaustion,
stale sessions, output-byte refusal, write/flush errors and finalization deadlines produce incomplete results.
Sessions are limited to 1024 full-size records. Raw encoding remains an explicit diagnostic control via
`THRUM_CAPTURE_ENCODING=raw`; it is not the reliable default on this disk.

Each arm records host load averages, a background-process snapshot and one-second `iostat -d -C -U` CPU/disk
samples (first data row averages since boot; default devices include spool disk0). The engine image record's
`readback_copy_gpu_bracket_ns` measures a GPU upper bracket around the texture-to-buffer and 32-byte exposure copies. Real compute
markers read the staging buffer before and after its blit write, forcing WAR/RAW dependencies; stage-boundary
counters measure before-marker end to after-marker end. The bracket includes the after-marker and encoder
transitions; it excludes mapping, CPU row unpacking and timestamp-resolution cost. It uses enabled native
`MAPPABLE_PRIMARY_BUFFERS` plus `TIMESTAMP_QUERY`; unsupported devices explicitly record null. Timers resolve
asynchronously in later coordinator work after completion. Scene GPU timings remain separate; neither measures
a whole-GPU span. Capture-disabled frames allocate no copy timers or marker resources.
Use `--profile` for separate matched CPU-sampled arms. `/usr/bin/sample` begins after the engine's pre-prime ready
marker (observed within 10 ms), samples at 1 ms through process exit, and writes `cpu-sample.txt`; sampling arms
are not used as unprofiled cost baselines.

Capture defaults to sleep-paced elapsed-time opportunities and requested user-interactive thread QoS.
QoS is recorded before/after; it is not CPU affinity. Each opportunity records active thread CPU time, wall time
and its start/end CPU numbers. The lighter sleeping OFF loop can migrate to efficiency cores and run at a
different CPU power state from ON. Do not subtract those noisy callback medians to claim a speedup or observer
cost. `THRUM_CAPTURE_PACING=active` is an explicit **diagnostic cost control**: both OFF and ON actively wait for
the same 120 Hz deadlines, keeping the render owner runnable. It burns CPU in pacing, outside the step/loop
regions; whole-child CPU includes that cost, preparation and warmup. Compare matched active arms for controlled
step/loop overhead, and retain ordinary sleep-paced runs separately. This is not a native/display performance
promotion. `THRUM_CAPTURE_QOS=inherited` retains the inherited class for diagnosis.

Packaging follows capture/drain and durable storage flush. `frames.zst` is itself the lossless source archive:
the independent `zstd` CLI decodes temporal differences, then Python reconstructs full RGBA and checks every
original **pre-encoding** writer SHA-256. Its record order is ordinal; manifest submission records own host time.
`--defer` records a pending, **not complete** manifest so consecutive sessions do not run heavy packaging between
captures. Afterwards run:

```sh
python3 scripts/capture_v1_runtime.py --output tmp/t2-capture-m0/pan --finalize --verify-only
```

This verifies every full source frame without duplicating PNGs/movie. Omit `--verify-only` to create all PNGs,
`timeline.ffconcat`, `preview-1x.mp4` and `contact.png`. PNG round trips must match the same original hashes.
The preview preserves host submission times (synthetic times in fixed mode), holding the previous image across
gaps. One labelled terminal duplicate is not a submission. FFprobe verifies frame coverage and PTS error.
`contact.png` is navigation; inspect native frames/crops and the source sequence. There is no pixel interpolation,
frame synthesis or time compression.

Legacy `THRUM_CAPTURE_MOTION_FRAMES` and `THRUM_CAPTURE_ROUTE` collect the same renderer tap. Their existing
fixed-time clocks, BGRA stream/checkpoint format and endpoint oracles remain unchanged. They are not live-motion proof.

## Retention and handoff

Create a task storage record before bulk work. A four-second session needs 16 GiB free and has a 16 GiB output
allowance including compressed source/PNG/preview coexistence. Run sessions serially. Verify the archive, timing records and
selected promoted evidence before deleting owned reconstructible expanded PNG files. Retain complete local sources
through review with an explicit expiry decision. Promote compact selected excerpts, hashes, timings and previews
to Git. The [current M0 evidence](../.orchestrate/evidence/render/t2-capture-m0/reliability/README.md) records the repeated series, all attempts and controlled observer arms. Local bulk archives are not durable backups; no remote/LFS store is configured or implied.

Archive record ordinal `k` maps to its `submission_id`, RGBA hash and `submit_ns` in `manifest.json`.
`scripts/capture_spool.py:decoded_frames` streams verified originals from `frames.zst` using the writer's hashes
in `runtime.json`. To rebuild PNGs/timeline/preview, run `--finalize` without `--verify-only` in the retained session.
Historical `source-lossless.mkv` archives use the old FFmpeg extraction recipe; they are not the new live format.

Mac M2.3 owns real-window tap/lease qualification; M2.4 owns bounded trace-finalization diagnosis. M2.2 owns
attended built-in-panel swaps with capture OFF and separate ON observer/content arms. Engine timestamps,
requested opportunities and movie PTS prove no compositor swaps, panel refresh, EDR or input-to-photon.
Mac M2 needs T2 M0 only; APPLE-TILE stays parked. Chrome endpoint captures retain fixed-time endpoint/oracle
scope until T2 M1; simulator OS captures retain function/UI scope until T2 M2 supplies submitted output.
Physical-device qualification remains separate.

## Chrome WebGPU

Build the portable game and cooker, then choose a new output directory under `tmp/`:

```sh
sh examples/v1-game/web/build.sh
rustup run stable cargo build --locked --offline -p thrum-cook
python3 "${ORCHESTRATE_LOOP:?Set ORCHESTRATE_LOOP from the installed build-milestone skill}" lock gpu --worktree . -- node scripts/capture_v1_browser.mjs --mode realtime --output tmp/t2-capture-m1/my-route
python3 scripts/package_v1_browser.py --output tmp/t2-capture-m1/my-route
```

If the compiler cache cannot start in the restricted runner, set `RUSTC_WRAPPER=` for the build.
The Chrome command requires Node 24+, this Mac's real Apple M3 Max/Metal WebGPU adapter, Chrome, Xcode's Swift
compiler, local ad-hoc codesigning and LaunchServices. It builds a local background launcher app under the output
folder. LaunchServices gives that app its own permission identity. The launcher only starts Chrome when its
read-only Screen Recording preflight is false, records it again after Chrome exits and never requests access.
The runner refuses software GPUs, unavailable hardware, incomplete exports and a changed permission preflight.
Console state is observed before and after the run. Accepted T2 M1 retains its locked-run capability witness.
For open checks and later consumers, either state or a transition is accepted as witness metadata. The denied
launcher preflights establish independence from desktop capture; no manual console-lock setup is required.

`--mode fixed` uses synthetic 60 Hz world samples and may wait for GPU completion and export. `--mode realtime`
feeds actual elapsed page time to the ordinary web shell. Its visible headless page uses 60 Hz timer opportunities,
with normal visibility, pause, overload and recovery policy. Browser flags and scheduling are recorded. This is a
controlled browser workload. It supplies no actual-tab display or panel timing claim. The original deterministic
`route.html` and its refusal controls retain their existing behavior.

Both modes install the checked-in cooked moving route at its ordinary tick barrier, capture an initial paused
hold, release the normal pause control, run the complete route including its cut, and retain a final hold. The
renderer copies its actual final submitted SDR output in the same coordinator submission. It preserves history,
three GPU slots and eight CPU leases. JS copies a ready image to a typed array and immediately returns its CPU
lease. Up to eight asynchronous HTTP exports are admitted, with a separate eight-request receiver bound. Node's
compression workers package each original RGBA record losslessly. The declared conservative export working
allowance is 32 full frames, in addition to the renderer's 11 frame buffers and codec/runtime allocations.
An export deadline, ring exhaustion, missing submission, write failure or output allowance refusal invalidates
continuous evidence while the ordinary page loop continues. Each session preflights 16 GiB free and caps output
at 16 GiB. Consecutive content qualification must use the consuming workload and retain every attempt.

For `--scene exit --mode realtime`, the ordinary and capture-off arms use the page's animation-frame
callbacks with GPU pass timing disabled. The ordinary 720p SDR arm transfers up to 64 reusable typed arrays
through a dedicated upload worker. The receiver writes originals at their submission offsets in one
bounded disk spool. It verifies and compresses each original after the page signals completion.

The V1 game page accepts `?clouds=shade` for ground shade and `?clouds=sky` for the visible layer and
shade. The capture runner accepts the same selections through `--clouds`. A fixed SDR run with
`--cloud-view path.json` takes one held submission from a cooked valley. The file supplies the camera
`eye` in metres, the pixel `extent` and the game's `params` for the held shot. A realtime cloud route
uses the same browser animation-frame callbacks and upload spool as the exit capture. Its runtime
record preserves the callback timestamps as `raf_ms`.

`manifest.json` pins source files, actual served Wasm/JS/route hashes, cooked root/generation, GPU/browser, launcher,
flags, encoding and permission/lock observations. `runtime.json` retains opportunities, world clocks, pauses,
submission IDs, source ticks/timeline, alpha, history/device/presenter generations and omissions. Renderer
`submit_ms` is monotonic time since the session's origin. The page records a bracket around session creation.
Pixel availability and HTTP completion never become sample times. Images include canvas scene/engine overlays.
DOM controls, browser/native chrome, OS composition and physical output are excluded.

The receiver hashes original RGBA before gzip. Final verification independently decompresses every source and
checks size, hash, contiguous submission IDs and complete head/tail coverage against the shell's presentation
counter. Final storage sync is recorded separately. Deferred packaging verifies PNG round trips and builds a 1×
preview from host submit times in real-time mode, or the declared synthetic times in fixed mode. A terminal
encoding duplicate is labelled. Skipped opportunities preserve elapsed gaps with the previous image held.
`packaging.json` records FFprobe coverage/PTS verification. Retain the compressed full sources through review,
then promote selected native frames, timeline records and previews under the storage rules.

Diagnostic controls use `--control stall`, `resize`, `saturation`, `slow`, `loss` or `off`. The stall blocks actual
page dispatch for 250 ms. Saturation withholds readback collection for two seconds. Slow output delays each
receiver request by 500 ms. Loss destroys the actual GPUDevice with a readback pending, proves authority progress
and stale-session refusal, then captures a replacement-device image. Negative controls return success only when
the expected incomplete outcome is proved, with `complete: false` and `negative_control_pass: true` in the manifest.
The off arm allocates no tap and is labelled `observer_only`. It is for matched observer cost. It cannot supply
image evidence. Rerun existing actual-tab/visibility/device-loss checks separately with `scripts/check_v1_browser.mjs`.

## iOS simulator submitted-output workflow

From the repository root, point `ORCHESTRATE_LOOP` to `skills/orchestration/lead/scripts/agent_loop.py`
in the installed orchestrate plugin and use a new output directory.

```sh
ORCHESTRATE_SCRIPT="${ORCHESTRATE_LOOP:?Set ORCHESTRATE_LOOP to the installed agent_loop.py}" python3 "$ORCHESTRATE_LOOP" lock simulator --worktree . -- python3 scripts/capture_v1_simulator.py --case all --output tmp/t2-capture-m2/my-route
```

The command takes the exclusive simulator lock for the whole run, including the build and cook. Keep the
turn within the 30-minute hold limit. The capture records the actual native foreground engine texture.
The [shared lifecycle](plans/SESSION/DESIGN.md#simulator-lifecycle-in-the-proof-harnesses-wp5) starts each run with a fresh app installation and data container, a new proof directory and cleared XCTest
runner state. The supervisor holds admission across the LaunchServices handoff and verifies shutdown before
release on completion, worker failure or cancellation. The owned device stays shut down for reuse.

The attributed LaunchServices parent performs denied read-only Screen Recording preflights before and after.
Console observations are witness metadata with no admission condition. `--no-build` reuses the linked app and
cooker and records their exact hashes.

Cases include disabled capture, three held-world cooked pans, running semantic axes, an eight-second render stall, pending surface/background/device retirement, ring saturation, explicit Raw slow-output refusal and the existing real UIKit touch/rotation regression. Simulator function is qualified separately from physical iPhone/display performance. Native UIKit and OS composition are excluded from the texture. The portrait diagnostic controlled actor can be outside the camera, so visible autonomous grid motion and committed semantic actor movement are separate observations.

The simulator uses six GPU slots, 32 CPU buffers and a 1 GiB pixel allowance, plus the shared finite 256 MiB encoded reserve and codec scratch. Writer output is bounded at 8 GiB, 1,024 records, 16 seconds and ten seconds to finalize. The complete driver preflights 20 GiB. Exhaustion/cancellation refuses continuous evidence. Callback dispositions and exact source extent, adjacent endpoint, alpha, history, surface/device identity and original hashes accompany every session. Source images retain native resolution.

`python3 scripts/package_v1_simulator.py --session <output>/01-pan` independently decodes every original, verifies PNG round trips and packages a VFR 1x viewing aid from actual submission time. Selected full-native PNGs and native crops accompany the preview. Keep full local spools through review. The driver restarts IconServices only in its owned simulator before app launch to recover headless fresh-device startup. See [M2 qualification](../.orchestrate/evidence/render/t2-capture-m2/qualified/README.md) for measured results, exact provenance, exceptions and retention.
