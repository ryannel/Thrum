"""Run the diagnostic game and a PID-filtered window recording in an approved slot."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import time

here = Path(__file__).resolve().parent
worktree = here.parent / "g3s-stars-main-wip"
binary = here / "live-diagnostic/source/target/release/v1-game"
preset = worktree / "examples/v1-game/params/showcase-noon.params.json"
default_store = worktree / "tmp/g7-showcase/runs/check-20261108-162905/store"
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("row", choices=("control", "paused", "running"))
parser.add_argument("output", type=Path)
parser.add_argument("--store", type=Path, default=default_store)
mode = parser.add_mutually_exclusive_group(required=True)
mode.add_argument("--owner-window", action="store_true")
mode.add_argument("--preflight-only", action="store_true")
args = parser.parse_args()
output = args.output.resolve()
if not output.is_relative_to(here) or output.exists():
    parser.error("output must be fresh and inside the witness directory")
state = subprocess.check_output(["/usr/sbin/ioreg", "-n", "Root", "-d", "1"], text=True)
locked = re.search(r'"(?:CGSSessionScreenIsLocked|IOConsoleLocked)"\s*=\s*Yes', state)
if args.preflight_only:
    print("Mac console is locked" if locked else "Mac console reports unlocked")
    raise SystemExit(0)
if locked:
    raise SystemExit("Mac console is locked")
store = args.store.resolve()
head = (store / "HEAD").read_bytes()
assert len(head) == 64 and head[:8] == b"THRMHEAD"
output.mkdir()
meta = {
    "row": args.row,
    "revision": "43e1d488b417f8dcf3677d6b1d80e98981d48d18",
    "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
    "params_sha256": hashlib.sha256(preset.read_bytes()).hexdigest(),
    "recorder_sha256": hashlib.sha256((here / "record_owned_window").read_bytes()).hexdigest(),
    "store_root": head[24:56].hex(),
    "cooked_generation": int.from_bytes(head[16:24], "little"),
    "started_unix_ns": time.time_ns(),
}
(output / "inputs.json").write_text(json.dumps(meta, indent=2) + "\n")
evidence = output / "pixels"
game_log = output / "game.log"
video = output / "window.mp4"
capture = None
destroy = args.row != "control"
deadline = time.monotonic() + 65

def wait_for(test, purpose):
    while time.monotonic() < deadline:
        result = test()
        if result:
            return result
        if game.poll() is not None:
            raise RuntimeError(f"game exited {game.returncode} while waiting for {purpose}")
        if capture and capture.poll() not in (None, 0):
            raise RuntimeError(f"recorder exited {capture.returncode} while waiting for {purpose}")
        time.sleep(0.05)
    raise RuntimeError(f"timed out waiting for {purpose}")

def seen(text):
    return text in game_log.read_text(errors="replace")

def cooked_swaps():
    matches = re.findall(rf"m1 game time=\S+ drawn=Some\({meta['cooked_generation']}\) swaps=(\d+)",
                         game_log.read_text(errors="replace"))
    return int(matches[-1]) if matches else 0

def stop_owned(child):
    if child and child.poll() is None:
        child.terminate()
        try:
            child.wait(timeout=4)
        except subprocess.TimeoutExpired:
            child.kill()
            child.wait(timeout=4)

with game_log.open("wb") as game_output, (output / "recorder.log").open("wb") as recorder_output:
    game = subprocess.Popen(
        [str(binary)], env=dict(os.environ, THRUM_SCENE="valley",
                                THRUM_ASSET_STORE=str(store), THRUM_PARAMS=str(preset),
                                THRUM_V1_FRAMES="10000", THRUM_M1_WITNESS=str(evidence),
                                THRUM_M1_DESTROY=str(int(destroy))),
        stdout=game_output, stderr=subprocess.STDOUT)
    print(f"{args.row}: pid {game.pid}", flush=True)
    try:
        wait_for(lambda: seen("m1 app pause-ack request=Pause"), "pause acknowledgement")
        initial_swaps = wait_for(cooked_swaps, "cooked valley draw")
        (evidence / "capture-before").touch(exist_ok=False)
        wait_for(lambda: seen("m1 image phase=before generation=1"), "healthy image")
        if args.row == "running":
            (evidence / "press-p").touch(exist_ok=False)
            wait_for(lambda: seen("m1 app pause-ack request=Resume"), "normal resume acknowledgement")
        capture = subprocess.Popen(
            [str(here / "record_owned_window"), str(game.pid), str(video), "18"],
            stdout=recorder_output, stderr=subprocess.STDOUT)
        wait_for(lambda: video.with_suffix(".mp4.ready").exists(), "window recording")
        if destroy:
            time.sleep(1)
            (evidence / "arm").touch(exist_ok=False)
            wait_for(lambda: cooked_swaps() > initial_swaps, "cooked valley draw on replacement")
            (evidence / "capture-after").touch(exist_ok=False)
            wait_for(lambda: seen("m1 image phase=after generation=2"), "restored image")
            (evidence / "press-e").touch(exist_ok=False)
            wait_for(lambda: seen("m1 injected input key=KeyE"), "post-recovery camera input")
            time.sleep(1)
            if capture.poll() is not None:
                raise RuntimeError("camera input was too late for the recording")
        capture.wait(timeout=max(1, deadline - time.monotonic()))
        if capture.returncode != 0 or not video.with_suffix(".mp4.done").exists():
            raise RuntimeError("window recording did not finalize")
        (evidence / "finish").touch(exist_ok=False)
        game.wait(timeout=max(1, deadline - time.monotonic()))
        if game.returncode != 0:
            raise RuntimeError(f"game exited {game.returncode}")
    finally:
        stop_owned(capture)
        stop_owned(game)
assert (store / "HEAD").read_bytes() == head
