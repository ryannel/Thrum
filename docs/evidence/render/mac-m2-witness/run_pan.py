"""Run one matched M2 game-window row, only in a granted slot."""
from __future__ import annotations

import argparse
import csv
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import time

HERE = Path(__file__).resolve().parent
WORKTREE = HERE.parent / "g3s-stars-main-wip"
BINARY = HERE / "source/target/release/v1-game"
RECORDER = HERE / "record_pan"
PRESET = WORKTREE / "examples/v1-game/params/showcase-noon.params.json"
DEFAULT_STORE = WORKTREE / "tmp/g7-showcase/runs/check-20261108-162905/store"


def stop(child):
    if child is not None and child.poll() is None:
        os.killpg(child.pid, signal.SIGTERM)
        try:
            child.wait(timeout=3)
        except subprocess.TimeoutExpired:
            os.killpg(child.pid, signal.SIGKILL)
            child.wait(timeout=3)


def wait_for(predicate, deadline, description, *children):
    while time.monotonic() < deadline:
        if result := predicate():
            return result
        for child in children:
            if child.poll() is not None:
                raise RuntimeError(f"child {child.pid} exited {child.returncode} awaiting {description}")
        time.sleep(0.025)
    raise RuntimeError(f"timed out awaiting {description}")


def inspect_pan(path):
    with path.open() as file:
        rows = list(csv.DictReader(file))
    result = {}
    for kind in ("action", "accepted", "present-called"):
        group = [row for row in rows if row["kind"] == kind]
        yaw = [float(row["yaw_degrees"]) for row in group]
        if len(set(yaw)) < 2 or abs(yaw[-1] - 36.0) > 0.01:
            raise RuntimeError(f"{kind} does not contain a complete pan to 36 degrees")
        if any(a > b + 0.001 for a, b in zip(yaw, yaw[1:])):
            raise RuntimeError(f"{kind} yaw reversed during a one-way pan")
        result[kind] = {"samples": len(yaw), "first": yaw[0], "last": yaw[-1]}
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("row", choices=("control", "trace"))
    parser.add_argument("output", type=Path)
    parser.add_argument("--store", type=Path, default=DEFAULT_STORE)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--owner-window", action="store_true")
    mode.add_argument("--preflight-only", action="store_true")
    args = parser.parse_args()
    output = args.output.resolve()
    if not output.is_relative_to(HERE) or output.exists():
        parser.error("output must be fresh and under the M2 scratch directory")
    state = subprocess.check_output(["/usr/sbin/ioreg", "-n", "Root", "-d", "1"], text=True)
    if re.search(r'"(?:CGSSessionScreenIsLocked|IOConsoleLocked)"\s*=\s*Yes', state):
        raise SystemExit("Mac console is locked")
    subprocess.run([str(RECORDER), "--preflight"], check=True)
    if args.preflight_only:
        print("console reports unlocked and game-window recording permission is available")
        return

    output.mkdir()
    store = args.store.resolve()
    head = (store / "HEAD").read_bytes()
    if len(head) != 64 or head[:8] != b"THRMHEAD":
        raise RuntimeError("invalid cooked store HEAD")
    identity = {
        "row": args.row,
        "base": "0d53d3aa",
        "cooked_root": head[24:56].hex(),
        "cooked_generation": int.from_bytes(head[16:24], "little"),
    }
    for key, path in (("binary", BINARY), ("preset", PRESET), ("recorder", RECORDER)):
        identity[f"{key}_sha256"] = hashlib.sha256(path.read_bytes()).hexdigest()
    (output / "inputs.json").write_text(json.dumps(identity, indent=2) + "\n")
    game_log = output / "game.log"
    video = output / "window.mp4"
    settings = {
        "THRUM_SCENE": "valley",
        "THRUM_ASSET_STORE": str(store),
        "THRUM_PARAMS": str(PRESET),
        "THRUM_M2_DIR": str(output),
        "THRUM_V1_FRAMES": "10000",
    }
    launcher_env = {key: value for key, value in os.environ.items() if not key.startswith("THRUM_")}
    recorder = None
    deadline = time.monotonic() + 38
    with (output / "recorder.log").open("wb") as recorder_log, (output / "launcher.log").open("wb") as launch_log:
        if args.row == "trace":
            if shutil.disk_usage(output).free < 10 * 1024**3:
                raise RuntimeError("insufficient free space for a bounded Instruments trace")
            trace_scratch = output / "trace-scratch"
            trace_scratch.mkdir()
            command = ["xcrun", "xctrace", "record", "--no-prompt", "--template", "Game Performance",
                       "--time-limit", "16s", "--output", str(output / "pan.trace"),
                       "--target-stdout", str(game_log)]
            for key, value in settings.items():
                command += ["--env", f"{key}={value}"]
            command += ["--launch", "--", str(BINARY)]
            game = subprocess.Popen(command, env=launcher_env | {"TMPDIR": str(trace_scratch)},
                                    start_new_session=True,
                                    stdout=launch_log, stderr=subprocess.STDOUT)
        else:
            with game_log.open("wb") as game_output:
                game = subprocess.Popen([str(BINARY)], env=launcher_env | settings,
                                        start_new_session=True, stdout=game_output, stderr=subprocess.STDOUT)
        try:
            def game_ready():
                if not (output / "cooked-ready").exists() or not game_log.exists():
                    return None
                log = game_log.read_text(errors="replace")
                if not re.search(r"m2 lifecycle Pause SnapshotId .*tick: 0", log):
                    return None
                return re.search(r"m2 app pid=(\d+) window=", log)

            game_match = wait_for(game_ready, deadline, "paused cooked game", game)
            pid = int(game_match.group(1))
            if args.row == "control" and pid != game.pid:
                raise RuntimeError("control window owner does not match the launched PID")
            recorder = subprocess.Popen([str(RECORDER), str(pid), str(video), "7"],
                                        start_new_session=True, stdout=recorder_log, stderr=subprocess.STDOUT)
            wait_for(lambda: video.with_suffix(".mp4.ready").exists()
                     and video.with_suffix(".mp4.complete").exists(),
                     deadline, "complete game-window frame", game, recorder)
            (output / "arm").touch(exist_ok=False)
            wait_for(lambda: (output / "end").exists(), deadline, "sweep endpoint", game, recorder)
            recorder.wait(timeout=max(1, deadline - time.monotonic()))
            if recorder.returncode != 0 or not video.with_suffix(".mp4.done").exists():
                raise RuntimeError("game-window recording failed to finalize")
            (output / "finish").touch(exist_ok=False)
            wait_for(lambda: (output / "pan.csv").exists(), deadline, "pan log")
            # Instruments may spend longer finalizing than it spent recording.
            game.wait(timeout=75 if args.row == "trace" else max(1, deadline - time.monotonic()))
            if game.returncode != 0:
                raise RuntimeError(f"launcher exited {game.returncode}")
        finally:
            stop(recorder)
            stop(game)
    if (store / "HEAD").read_bytes() != head:
        raise RuntimeError("cooked store changed during the row")
    print(json.dumps(inspect_pan(output / "pan.csv"), indent=2))


if __name__ == "__main__":
    main()
