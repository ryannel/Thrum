"""Prove the Crowd status display window and the packaged WebGPU page."""

import json
import os
from pathlib import Path
import select
import signal
import subprocess
import tempfile
import time

from game_ui_status import wake_window_display
from t2_writer_probe import package

EXAMPLE = "examples/crowd-2d-status"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def verify_mac(root, out, scratch):
    cargo = ["cargo", "build", "--release", "--locked", "--offline"]
    command = [*cargo, "-p", "crowd-2d-status", "-p", "thrum-cook"]
    subprocess.run(command, cwd=root, check=True, timeout=600)
    target = Path(os.environ.get("CARGO_TARGET_DIR", root / "target")).resolve()
    store = scratch / "store"
    subprocess.run([target / "release/thrum-cook", "cook", root / EXAMPLE / "assets", store],
                   cwd=root, check=True, timeout=300)
    exe = target / "release/crowd-2d-status"
    before, awake = wake_window_display()
    require(not awake["display_idle"], "the Mac window console did not wake")
    env = dict(os.environ, THRUM_ASSET_STORE=str(store))
    steps = [
        {"action": "key", "code": "ArrowRight", "down": True},
        {"action": "key", "code": "ArrowRight", "down": False, "after": 2},
        {"action": "cursor", "point": [330, 390]},
        {"action": "button", "down": True},
        {"action": "cursor", "point": [430, 375]},
        {"action": "button", "down": False, "after": 2},
        {"action": "observe", "after": 2},
    ]
    script = out / "input.json"
    witness = out / "input-witness.json"
    script.write_text(json.dumps(steps, indent=2) + "\n")

    def window(name, **settings):
        with (out / f"{name}.log").open("w") as log:
            subprocess.run(["/usr/bin/caffeinate", "-d", "-i", exe], cwd=root, timeout=120,
                           check=True, stdout=log, stderr=subprocess.STDOUT,
                           env=dict(env, THRUM_FRAMES="150", **settings))

    window("window-input", THRUM_CROWD_INPUT=str(script), THRUM_CROWD_WITNESS=str(witness))
    motion = json.loads(witness.read_text())
    poses = [step["before"] for step in motion["steps"]]
    require(len(poses) == len(steps) and poses[1]["presses"][0] > poses[0]["presses"][0]
            and poses[1]["pan"] != poses[0]["pan"], "window key did not route through the interface")
    require(poses[-1]["presses"][1] > poses[0]["presses"][1]
            and poses[-1]["pan"] != poses[1]["pan"], "window pointer drag did not route through the interface")
    require(all(pose["live"] == pose["rows"] for pose in poses), "a live body has no world sprite row")

    window("window-capture", THRUM_CROWD_CAPTURE=str(out))
    capture = json.loads((out / "submitted.json").read_text())
    require(capture["paint_scale"] in (1, 2, 3) and capture["capture_extent"] == capture["drawable"],
            "the engine capture omitted the resident interface")
    raw = (out / "submitted.rgba").read_bytes()
    return dict(console_before=before, console_at_launch=awake, capture=capture,
                rgba_sha256=package(raw, out / "submitted.png", *capture["capture_extent"]))


def verify_browser(root, out, scratch):
    environment = dict(os.environ, THRUM_SCENE_OUTPUT=str(scratch / "scene"))
    with (out / "package.log").open("wb") as log:
        process = subprocess.Popen(["python3", "-u", "scripts/scene_web.py", EXAMPLE], cwd=root,
                                   env=environment, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                   start_new_session=True)
        try:
            deadline = time.monotonic() + 600
            lines = b""
            url = None
            while time.monotonic() < deadline:
                if not select.select([process.stdout], [], [], 1)[0]:
                    continue
                chunk = os.read(process.stdout.fileno(), 65536)
                if not chunk:
                    break
                log.write(chunk)
                log.flush()
                lines = (lines + chunk)[-65536:]
                for line in lines.split(b"\n")[:-1]:
                    if line.startswith(b"SCENE_URL="):
                        url = line.partition(b"=")[2].decode().strip()
                        break
                if url:
                    break
            require(url, "the shared scene packager did not serve the Crowd page")
            with (out / "chrome.log").open("w") as browser_log:
                subprocess.run(["node", "scripts/game_ui_crowd_browser.mjs", url, out, scratch / "profile"],
                               cwd=root, stdout=browser_log, stderr=subprocess.STDOUT,
                               check=True, timeout=300)
        finally:
            if process.poll() is None:
                try:
                    os.killpg(process.pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
                process.wait(timeout=15)
    browsers = json.loads((out / "browser.json").read_text())
    for browser in browsers:
        raw = out / browser["raw"]
        png = raw.with_suffix(".png")
        saved = package(raw.read_bytes(), png, *browser["capture"]["extent"])
        require(saved == browser["rgba_sha256"], "Chrome capture bytes did not round-trip")
    return dict(runs=browsers)


def run(root, target, output, revision):
    output.mkdir(parents=True, exist_ok=True)
    scratch = root / "tmp/game-ui-wp6"
    scratch.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=target + "-", dir=scratch) as work:
        func = verify_mac if target == "mac-metal" else verify_browser
        record = func(root, output, Path(work))
    record.update(commit=revision, target=target, load=list(os.getloadavg()))
    result = output / "result.json"
    result.write_text(json.dumps(record, indent=2) + "\n")
    print(result)
