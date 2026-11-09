# Engine-output capture on Mac

`scripts/capture_v1_runtime.py` records the ordinary V1 game through the renderer's submitted-output tap.
It requires a real Metal adapter, Python 3, Swift/CoreGraphics for a read-only permission preflight and
FFmpeg/FFprobe. It never requests Screen Recording permission or changes console lock. Unavailable hardware
and incomplete intervals fail explicitly.

Build and cook in this worktree:

```sh
rustup run 1.99.1 cargo build --locked --offline --release -p v1-game
rustup run 1.99.1 cargo build --locked --offline -p thrum-cook
target/debug/thrum-cook cook examples/v1-game/assets/showcase tmp/t2-capture-m0/valley
```

Run the cooked noon pan with a new output directory:

```sh
python3 /Users/ryannel/.claude/skills/build/scripts/agent_loop.py lock gpu --worktree . -- python3 scripts/capture_v1_runtime.py --binary target/release/v1-game --store tmp/t2-capture-m0/valley --output tmp/t2-capture-m0/pan --case pan --mode realtime
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

Additional cases use the same binary:

- `--case running`, without `--store`: moving diagnostic grid, with a running 60 Hz world between holds.
- `--case render-stall`: diagnostic 250 ms render-owner delay before the ordinary step.
- `--case input-stall`: diagnostic 250 ms dispatch delay before semantic handling; renders continue with held controls.
- `--case writer-slow`: running-world saturation, delaying writer calls 50 ms; expected incomplete/failure.
- `--case off`: matched pan with capture disabled; no tap buffers or writer queue.

Ordinary pan/running/off schedules contain no diagnostic actions.

## Artifacts and bounds

`runtime.json` joins opportunities, requested/accepted controls, exact submission IDs, adjacent endpoint ticks,
timeline, interpolation alpha, history epoch, device generation, map observations and GPU/CPU samples. Capture
is of the scene grid: surface viewport resize does not relabel old pixels as a new-sized screenshot.

`manifest.json` pins source revision, dirty diff and source files (including untracked implementation), binary,
shaders, cooked products, scenario/parameters, recipe/features, colour/stage and lock/permission observations.
The current V1 route has no independent selected recipe registry; its resolved recipe digest covers the route,
capability record and shaders. Native controls and OS composition are excluded.

The graph copies the actual final SDR engine texture in the ordinary submission, retaining temporal history.
There is no endpoint redraw, extra scene render, second queue or drawable acquisition/lease extension. Six GPU
slots and 32 reusable CPU buffers use 315,187,200 pixel bytes within a 512 MiB allowance. The initial three/eight
profile was measured first; integrated bursts motivated the larger bounded profile. An uncached inherited file
description receives sequential raw RGBA on a dedicated writer. Nonblocking admission and generation checks
keep slow/failed capture from stopping the app. Exhaustion, stale sessions, output-byte refusal, write/flush
errors and finalization deadlines produce incomplete results.

Packaging follows capture/drain and storage `fsync` plus Darwin `F_FULLFSYNC`. Every decoded PNG hash must
match its raw frame. `source-lossless.mkv` is an archival frame sequence; every decoded RGBA hash, including
alpha, is verified. Its timestamps are ordinal only: retained submission records own host time.
`timeline.ffconcat` and `preview-1x.mp4` preserve host submission times (synthetic times in fixed mode), holding
the previous image across gaps. One labelled terminal duplicate is not a submission. FFprobe verifies frame
coverage and PTS error. `contact.png` is navigation; inspect native frames/crops and the source sequence.
There is no pixel interpolation, frame synthesis or time compression.

Legacy `THRUM_CAPTURE_MOTION_FRAMES` and `THRUM_CAPTURE_ROUTE` collect the same renderer tap. Their existing
fixed-time clocks, BGRA stream/checkpoint format and endpoint oracles remain unchanged. They are not live-motion proof.

## Retention and handoff

Create a task storage record before bulk work. A four-second session needs 16 GiB free and has a 16 GiB output
allowance including raw/PNG/archive coexistence. Run sessions serially. Verify the archive, timing records and
selected promoted evidence before deleting owned reconstructible raw/PNG files. Retain complete local sources
through review with an explicit expiry decision. Promote compact selected excerpts, hashes, timings and previews
to Git. Local bulk archives are not durable backups; no remote/LFS store is configured or implied.

Archive record ordinal `k` maps to its `submission_id`, RGBA hash and `submit_ns` in `manifest.json`. Extract a
native frame with `ffmpeg -i source-lossless.mkv -vf 'select=eq(n\,0)' -frames:v 1 -pix_fmt rgba restored.png`
(replace zero with `k`). Its decoded RGBA must match the record's hash. To rebuild deleted PNGs, decode in record
order and name each file by its submission ID before using the retained `timeline.ffconcat`.

Mac M2.3 owns real-window tap/lease qualification; M2.4 owns bounded trace-finalization diagnosis. M2.2 owns
attended built-in-panel swaps with capture OFF and separate ON observer/content arms. Engine timestamps,
requested opportunities and movie PTS prove no compositor swaps, panel refresh, EDR or input-to-photon.
Mac M2 needs T2 M0 only; APPLE-TILE stays parked. Chrome endpoint captures retain fixed-time endpoint/oracle
scope until T2 M1; simulator OS captures retain function/UI scope until T2 M2 supplies submitted output.
Physical-device qualification remains separate.
