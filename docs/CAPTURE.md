# Engine-output capture on Mac

`scripts/capture_v1_runtime.py` records the ordinary V1 game through the renderer's submitted-output tap.
It requires a real Metal adapter, Python 3, Swift/CoreGraphics for a read-only permission preflight and
FFmpeg/FFprobe and the `zstd` CLI (independent source verification). It never requests Screen Recording permission or changes console lock. Unavailable hardware
and incomplete intervals fail explicitly.

Build and cook in this worktree:

```sh
rustup run 1.99.1 cargo build --locked --offline --release -p v1-game
rustup run 1.99.1 cargo build --locked --offline -p thrum-cook
target/debug/thrum-cook cook examples/v1-game/assets/showcase tmp/t2-capture-m0/valley
```

Run the cooked noon pan with a new output directory:

```sh
python3 "$HOME/.claude/skills/build/scripts/agent_loop.py" lock gpu --worktree . -- python3 scripts/capture_v1_runtime.py --binary target/release/v1-game --store tmp/t2-capture-m0/valley --output tmp/t2-capture-m0/pan --case pan --mode realtime
```

The explicit `--worktree .` also works around this installed lock script's parsing of the command separator.
Do not run compilers, cookers or other disk-heavy jobs concurrently with qualification/cost runs; the GPU lock
does not serialize disk traffic.

## Modes and controls

Use `--mode still` for one settled submitted image, `--mode fixed` for synthetic camera samples and
`--mode realtime` for actual monotonic elapsed time, including oversleep and stalls. Fixed samples may wait
between readbacks; their smoothness cannot prove runtime motion. Existing `THRUM_CAPTURE_SCENE` and
`FrameHost::capture_pixels` retain separate endpoint/no-history/alpha-one oracle semantics.

The pan requests 120 opportunities/s at 1920×1080, holds yaw zero for one second, turns to 36° over two
seconds, then holds the accepted final state for at least one second. The world remains paused. A real
initial submission anchors zero; earlier request/accept observations retain signed times. Missing opportunity
indices name driver busy/oversleep. Submitted-image omissions invalidate continuous proof. Controls enter
normal parameter validation and `Game::apply`, the handler used by the native panel. The engine adapter
owns scheduling and supplies an engine texture through ordinary presenter/`SurfaceOutput` admission.
World work remains on its resident lane; rendering runs on the dedicated render-owner thread.
The engine opportunity clock targets the next 120 Hz opportunity. `Offscreen::step_for` consumes that target
with the native actor's shared `display_target_lead` policy (expired target: zero, maximum: 50 ms).
The host stamps the actual consumed lead on each submitted image; untargeted prime/warmup steps declare zero.
This is an offscreen target policy, not a measured display target or a claim that its alpha equals a window's.

Additional cases use the same binary:

- `--case running`, without `--store`: moving diagnostic grid, with a running 60 Hz world between holds.
- `--case render-stall`: diagnostic 250 ms render-owner delay before the ordinary step.
- `--case input-stall`: diagnostic 250 ms dispatch delay before semantic handling; renders continue with held controls. Intermediate edits are coalesced to the newest value; requested/start coincide at diagnostic activation.
- `--case writer-slow`: running-world slow output, delaying 4 MiB writer calls 500 ms. Use `THRUM_CAPTURE_ENCODING=raw` for the saturation/refusal control; the compressed diagnostic grid can fit the finite encoded reserve and finish successfully.
- `--case off`: matched pan with capture disabled; no tap buffers or writer queue.

Ordinary pan/running/off schedules contain no diagnostic actions.

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
slots and 96 reusable CPU buffers use 846,028,800 pixel bytes within a 1 GiB allowance. This is a bounded
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
`readback_copy_gpu_bracket_ns` measures a GPU upper bracket around the texture-to-buffer copy. Real compute
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
to Git. The [current M0 evidence](evidence/render/t2-capture-m0/reliability/README.md) records the repeated series, all attempts and controlled observer arms. Local bulk archives are not durable backups; no remote/LFS store is configured or implied.

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
