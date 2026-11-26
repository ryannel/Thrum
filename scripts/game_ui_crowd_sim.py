"""Run the Crowd app's real touch, rotation and foreground route."""

import hashlib
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import sys
import tempfile

from capture_v1_simulator import wait
from simulator_ui_runner import build as build_runner
import simulator_lifecycle as simulator

ROOT = simulator.ROOT
APP_SOURCE = ROOT / "examples/crowd-2d-status-ios/ios"
BUNDLE = "org.thrum.crowd"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def run(command, **options):
    return subprocess.run(command, cwd=ROOT, check=True, timeout=600, **options)


def prepare(out):
    out.mkdir(parents=True, exist_ok=True)
    scratch = ROOT / "tmp/game-ui-wp6"
    scratch.mkdir(parents=True, exist_ok=True)
    revision = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    target = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target")).resolve()
    cargo = ["cargo", "build", "--locked", "--offline", "-p", "thrum-cook", "--bin", "thrum-cook"]
    run(cargo)
    app = target / "crowd-2d-status-ios-sim/Crowd.app"
    run([APP_SOURCE / "build-simulator.sh"], env=dict(os.environ, RUSTC_WRAPPER=""))
    with tempfile.TemporaryDirectory(prefix="ios-", dir=scratch) as temporary:
        work = Path(temporary)
        store = work / "store"
        run([target / "debug/thrum-cook", "cook", ROOT / "examples/crowd-2d-status/assets", store])
        runner = build_runner(app, APP_SOURCE / "uitest/CrowdJourney.swift", work / "uitest", "CrowdUITests")
        require(revision == subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
                "source revision changed while building")
        env = dict(os.environ, THRUM_CROWD_PROOF_COMMIT=revision,
                   **{simulator.RECORD_ENV: str(out / "simulator-lifecycle.jsonl")})
        command = ["/usr/bin/caffeinate", "-d", "-i", "-u", sys.executable, simulator.__file__, "fresh",
                   "--", sys.executable, __file__, str(out), str(app), str(store), str(runner)]
        if simulator.admission()[0] is None:
            command = simulator.lock_command(command)
        # A queued lock has no device time. The simulator supervisor bounds the
        # run after admission and verifies shutdown before releasing the lock.
        subprocess.run(command, cwd=ROOT, env=env, check=True)


def worker(out, app, store, runner):
    device = simulator.worker_device()
    record = {
        "target": "ios-sim", "scope": "function", "device": device,
        "commit": os.environ["THRUM_CROWD_PROOF_COMMIT"],
        "app_sha256": hashlib.sha256((app / "Crowd").read_bytes()).hexdigest(),
        "font_head_sha256": hashlib.sha256((store / "HEAD").read_bytes()).hexdigest(),
        "load": list(os.getloadavg()), "states": {}, "screens": {}, "tests": [],
    }

    def simctl(*args, env=None):
        return run(["xcrun", "simctl", *args], env=env, capture_output=True, text=True).stdout.strip()

    def test(name):
        case = f"CrowdUITests/CrowdJourney/{name}"
        command = ["xcodebuild", "test-without-building", "-xctestrun", runner, "-destination", f"id={device}",
                   "-parallel-testing-enabled", "NO", "-resultBundlePath", out / f"{name}.xcresult",
                   f"-only-testing:{case}"]
        log = out / f"{name}.log"
        with log.open("w") as stream:
            run(command, stdout=stream, stderr=subprocess.STDOUT)
        record["tests"].append(dict(case=case, log=log.name, exit=0))

    try:
        inventory = json.loads(simctl("list", "devices", "-j"))
        record["runtime"], record["model"] = next(
            (runtime, model["deviceTypeIdentifier"]) for runtime, devices in inventory["devices"].items()
            for model in devices if model["udid"] == device and model["state"] == "Booted")
        simctl("install", device, app)
        container = Path(simctl("get_app_container", device, BUNDLE, "data"))
        shutil.copytree(store, container / "Documents/crowd-store")
        witness = container / "tmp/thrum-crowd-ios-state.txt"

        def state():
            try:
                source = witness.read_text()
            except FileNotFoundError:
                return {}
            require(not source.startswith("fault="), source)
            row = dict(line.split("=", 1) for line in source.splitlines() if "=" in line)
            row["crowd"] = json.loads(row.pop("crowd_status", "{}"))
            return row

        def ready(row, landscape=None):
            shown = row.get("crowd", {}).get("drawn")
            if not shown or row.get("state") != "Active":
                return False
            w, h = shown["extent"]
            return landscape is None or (w > h) == landscape

        def screenshot(label, landscape):
            row = wait(state, lambda s: ready(s, landscape), f"sent {label} viewport", 120)
            card = row["crowd"]["drawn"]
            w, h = card["extent"]
            scale = card["scale"]
            left, top, right, bottom = card["insets"]
            (x0, y0), (x1, y1) = card["bounds"]
            require(scale == 3 and bottom > 0 and (left > 0 and right > 0 if landscape else top > 0),
                    f"missing UIKit insets for {label}: {card}")
            require(left <= x0 < x1 <= w / scale - right and top <= y0 < y1 <= h / scale - bottom,
                    f"status display outside the safe area: {card}")
            path = out / f"{label}.png"
            simctl("io", device, "screenshot", path)
            pixels = struct.unpack(">II", path.read_bytes()[16:24])
            require(pixels == (w, h), f"screen capture {pixels} differs from the drawable {(w, h)}")
            crops = []
            card_crop = (round(x0 * scale), round(y0 * scale), round((x1 - x0) * scale), round((y1 - y0) * scale))
            corners = [(0, 0), (w - 160, 0), (0, h - 160), (w - 160, h - 160)]
            for name, (x, y, width, height) in zip(
                    ["status", "top-left", "top-right", "bottom-left", "bottom-right"],
                    [card_crop, *((x, y, 160, 160) for x, y in corners)], strict=True):
                file = out / f"{label}-{name}.png"
                run(["ffmpeg", "-y", "-loglevel", "error", "-i", path, "-vf",
                     f"crop={width}:{height}:{x}:{y}:exact=1", "-frames:v", "1", file])
                crops.append(file.name)
            record["states"][label] = row
            record["screens"][label] = dict(screen=path.name, crops=crops, extent=[w, h])
            return row

        simctl("launch", device, BUNDLE, env=dict(os.environ, SIMCTL_CHILD_THRUM_CROWD_ROUTE="1"))
        portrait = screenshot("portrait-before", False)
        test("testDragAndRotate")
        landscape = screenshot("landscape-after-rotation", True)
        require(landscape["pid"] == portrait["pid"] and
                landscape["device_generation"] == portrait["device_generation"] and
                int(landscape["presented_epoch"]) > int(portrait["presented_epoch"]),
                "rotation did not retain the process and device on a new surface epoch")
        require(landscape["crowd"]["presses"][2] > portrait["crowd"]["presses"][2] and
                landscape["crowd"]["pan"] != portrait["crowd"]["pan"],
                "the UIKit drag did not pass the interface and change the submitted camera")
        landscape = wait(state, lambda s: ready(s, True) and s["crowd"]["tick"] > portrait["crowd"]["tick"],
                         "the rotated replay advanced", 120)

        simctl("launch", device, "com.apple.Preferences")
        suspended = wait(state, lambda s: s.get("state") == "Suspended", "backgrounded Crowd", 120)
        record["states"]["suspended"] = suspended
        require(int(suspended["world_tick"]) < 3600, "the replay finished before backgrounding")
        test("testReturnToForeground")
        resumed = wait(state, lambda s: ready(s, True) and int(s["world_tick"]) > int(suspended["world_tick"]),
                       "foreground replay progress", 120)
        record["states"]["resumed"] = resumed
        require(resumed["pid"] == portrait["pid"] and resumed["device_generation"] == portrait["device_generation"]
                and int(resumed["resumes"]) > int(suspended["resumes"]),
                "the foreground run restarted the game")
        test("testPortrait")
        screenshot("portrait-after-return", False)
        final = wait(state, lambda s: ready(s, False) and s["crowd"].get("digest"),
                     "the retained endpoint", 480)
        retained_path = ROOT / "examples/crowd-2d/retained/crowd.txt"
        expected = next(line.split()[-1] for line in retained_path.read_text().splitlines()
                        if line.startswith("checkpoint 2000 3600 "))
        require(final["crowd"]["tick"] == 3600 and final["crowd"]["digest"] == expected,
                f"the live app's endpoint differs from the retained trace: {final['crowd']}")
        require(final["crowd"]["rows_checked"] >= 3600 and final["crowd"]["row_mismatches"] == 0
                and final["crowd"]["rows"] == final["crowd"]["live"] > 0,
                f"the app did not map every live body to a world row: {final['crowd']}")
        record["states"]["endpoint"] = final
        record["expected_digest"] = expected
        record["complete"] = True
    finally:
        (out / "result.json").write_text(json.dumps(record, indent=2) + "\n")
        print(out / "result.json", flush=True)


if __name__ == "__main__":
    worker(*map(Path, sys.argv[1:]))
