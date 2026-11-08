# Mac M2 pacing witness

M2 is active. The reviewed plan and shipped sub-degree yaw step are on
main at 3d48f44e. The old 1° step snapped a 0.15° edit to 0°. The new
0.05° step preserves that edit and the keyboard's 5° turn. The focused
tests and full workspace check pass (602 tests).

The scratch release binary, recorder, source generator, timing fixture and
swap parser are in this directory. The app drives the existing AppKit
slider action at display-link targets, and the game log requires the
normal tick-zero pause acknowledgement. The recorder checks permission
without a prompt and refuses an ambiguous game window or an off-panel
placement. The runner waits for a complete captured sample before arming.
The trajectory holds 0° for 0.75 s, sweeps to 36° in 2 s and holds through
capture finalization. The trace and control use the same release binary,
noon preset, cooked root and recorder.

Preflight passed without launching the game. The built-in 3456×2234 panel
reports a 120 Hz mode. A historical export exercised swap parsing, including
a duplicate-stream and wrong-PID refusal. It does not qualify this run.
A headless Points of Interest probe compiled, but xctrace left its
target suspended without starting. The exact owned processes were stopped.
Fresh signpost export and same-trace join remain unverified.

## Granted live slot only (8 minutes)

First check no unexpected dialogs and no competing screen work. Stop on a
permission refusal or unexpected dialog. Each row has a 38-second wall
timeout, with a 16-second Game Performance trace and seven-second
window-only recording. No manual input is needed. Use fresh paths.

```sh
out=/Users/ryannel/Workspace/thrum/tmp/g3b-mac-m2
slot=$(mktemp -d "$out/slot.XXXXXX")
python3 "$out/run_pan.py" control "$slot/control" --owner-window
python3 "$out/run_pan.py" trace "$slot/trace" --owner-window
```

Release the slot once both launches are closed. Export TOC,
displayed-surfaces-interval, device-display-info, and PointsOfInterest
rows from the trace. Use the TOC launch PID, recorder's built-in
placement, valid pan markers and numeric Swap Time for attribution.
Keep the source logs and selected evidence until review. Do not infer
physical cadence from video PTS or app-side present calls.

## Hidden AppKit proof and trace follow-up

`THRUM_M2_PANEL_PREFLIGHT=1` runs the game's real control model
and AppKit target/action on an unshown panel. The accepted arm
dispatched all 240 requested actions and reached 35.999989° with
0.149990°–0.150005° accepted steps. It uses the same release diagnostic
binary as the planned slot. The proof exercises panel dispatch and
game control edits. It has no rendered window, actor or presentation.

A second headless Game Performance probe attached to the running
signpost command-line process, reached its four-second time limit,
then exceeded its 18-second wall cap while finalizing. The partial
trace export refused with Document Missing Template. The live runner
still bounds on-screen startup and recording to 38 seconds. After
the game writes its pan log and capture has closed, Instruments has
up to 75 seconds to finalize. Both phases fit in the requested slot.
The probes leave fresh trace-domain markers and display attribution
open.

## Rebuild from the diagnostic sources

Copy the tracked diagnostic source files into `tmp/g3b-mac-m2/` in the
main checkout and set `out` to that absolute path. In a fresh directory
with Rust 1.99.1, Xcode and the locked Cargo cache, run:

```sh
mkdir "$out/source"
git archive 3d48f44e | tar -x -C "$out/source"
xcrun clang -Wall -Wextra -Werror -c "$out/pan_signpost.c" -o "$out/pan_signpost.o"
xcrun ar rcs "$out/libthrum_m2_pan.a" "$out/pan_signpost.o"
python3 "$out/prepare_pan.py" "$out/source"
(cd "$out/source" && RUSTC_WRAPPER= rustup run 1.99.1 cargo build --locked --offline --release -p v1-game)
xcrun swiftc -module-cache-path "$out/swift-cache" -parse-as-library "$out/record_pan.swift" -o "$out/record_pan"
THRUM_M2_PANEL_PREFLIGHT=1 "$out/source/target/release/v1-game"
```

The runner defaults to the known local checkout and cooked store. Set
its worktree path for another checkout and pass the reproduced cooked
store with `--store`. Use the verified noon preset and the fixed cooked
root beginning `70d2baf8`.
