#!/usr/bin/env python3
"""Verify the V1 sprite scene through UIKit on the iPhone 18 Pro simulator."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import runpy
import subprocess
import sys
import time

import simulator_lifecycle as simulator
from capture_spool import decoded_frames
from capture_v1_simulator import load, validate_session, wait
from render2d import PASSES, ROOT, digest, git
from t2_writer_probe import package

BUNDLE = "org.thrum.v1"
ROWS = 10_000
SCALE = 2


def compare_card(raw, expected, extent):
    width, height = extent
    card_width, card_height = 256 * SCALE, 128 * SCALE
    left, top = width // 2 - card_width // 2, height // 2 - card_height // 2
    if left < 0 or top < 0 or len(expected) != 256 * 128 * 4:
        raise ValueError("authored card does not fit the drawable")
    for output_y in range(card_height):
        source_y = output_y // SCALE
        texels = expected[source_y * 1024:(source_y + 1) * 1024]
        row = b"".join(texels[index:index + 4] * SCALE for index in range(0, len(texels), 4))
        address = ((top + output_y) * width + left) * 4
        if raw[address:address + len(row)] != row:
            raise ValueError(f"aligned card differs on engine row {top + output_y}")
    return [left, top, card_width, card_height]


def worker(out):
    out.mkdir(parents=True)
    device = simulator.worker_device()
    app = ROOT / "target/v1-game-ios-sim/ThrumV1.app"
    report = dict(target="ios-sim", scope="function only", device=device, commit=git("rev-parse", "HEAD"),
                  artifact_sha256=digest(app / "ThrumV1"), commands=[], orientations=[])

    def run(*args, env=None):
        line = ["xcrun", "simctl", *args]
        result = subprocess.run(line, cwd=ROOT, text=True, capture_output=True, env=env, timeout=150)
        report["commands"].append(dict(command=line, exit=result.returncode, stdout=result.stdout,
                                       stderr=result.stderr))
        result.check_returncode()
        return result.stdout.strip()

    def state():
        error = temp / "thrum-capture-error.txt"
        if error.exists():
            raise RuntimeError(error.read_text())
        raw = witness.read_text() if witness.exists() else ""
        if raw.startswith("fault="):
            raise RuntimeError(raw)
        return dict(line.split("=", 1) for line in raw.splitlines() if "=" in line)

    def grab(label, session, observed):
        prior_pauses = int(state()["pause_replies"])
        route["post"]("hold-world")
        held = wait(state, lambda s: s.get("world_paused") == "true"
                    and int(s.get("pause_replies", 0)) > prior_pauses, "held photo endpoint")
        held_tick = int(held["pause_tick"])
        route["post"]("capture-begin")
        source = temp / f"thrum-capture-{session}"
        wait(lambda: load(source / "pending.json"), lambda s: s.get("images", 0) > 0, "held submitted image")
        screen = out / f"{label}-simctl.png"
        run("io", device, "screenshot", str(screen))
        route["post"]("capture-end")
        receipt = wait(lambda: load(source / "runtime.json"), bool, "held session retirement", 40)
        validate_session(receipt)
        header = load(source / "identity.json")
        (out / f"{label}-capture.json").write_text(json.dumps(receipt, indent=2))
        if header["passes"] != list(PASSES) or header["backend"] != "Metal":
            raise ValueError("the iOS sprite scene compiled a foreign pass")
        frames = receipt["images"]
        expected_extent = [int(side) for side in observed["presented_extent"].split("x")]
        for frame in frames:
            if (frame["tick"], frame["extent"], frame["device_generation"], frame["surface_epoch"],
                frame["sprite_world_rows"], frame["sprite_frame_rows"], frame["sprite_draws"]) != (
                    held_tick, expected_extent, int(held["device_generation"]), int(held["presented_epoch"]), ROWS, 1, 1):
                raise ValueError(f"held image has the wrong endpoint, extent, drawable or scene: {frame}")
        width, height = expected_extent
        raw = None
        for frame in decoded_frames(source / "frames.spool", header["encoding"], receipt["writer"]["hashes"],
                                    width * height * 4):
            if raw is not None and frame != raw:
                raise ValueError("the held sprite scene changed between submitted frames")
            raw = frame
        expected = (temp / "thrum-sprite-card.rgba").read_bytes()
        bounds = compare_card(raw, expected, expected_extent)
        png = out / f"{label}-engine.png"
        package(raw, png, width, height)
        for name, image in [("engine", png), ("simctl", screen)]:
            subprocess.run(["ffmpeg", "-v", "error", "-i", str(image), "-vf",
                            "crop=" + ":".join(map(str, [*bounds[2:], *bounds[:2]])),
                            "-frames:v", "1", str(out / f"{label}-{name}-crop.png")], check=True, timeout=20)
        report["orientations"].append(dict(name=label, state=held, passes=header["passes"], bounds=bounds,
                                           image=frames[0], card_sha256=hashlib.sha256(expected).hexdigest(),
                                           simctl_sha256=digest(screen), engine_png_sha256=digest(png)))
        route["post"]("run-world")
        return wait(state, lambda s: s.get("world_paused") == "false" and int(s.get("world_tick", 0)) > held_tick,
                    "ticks resumed after held frame")

    try:
        run("install", device, str(app))
        data = Path(run("get_app_container", device, BUNDLE, "data"))
        temp = data / "tmp"
        witness = temp / "thrum-v1-ios-state.txt"
        child = dict(os.environ, SIMCTL_CHILD_THRUM_V1_ROUTE="1", SIMCTL_CHILD_THRUM_SCENE="sprites",
                     SIMCTL_CHILD_THRUM_SPRITE_ROWS=str(ROWS), SIMCTL_CHILD_THRUM_SPRITE_SCALE=str(SCALE))
        run("launch", device, BUNDLE, env=child)
        before = wait(state, lambda s: int(s.get("presented", 0)) >= 10, "initial sprite presentation", 100)
        arguments = sys.argv
        sys.argv = ["route.py", device, str(witness)]
        os.environ["THRUM_V1_ROUTE_OUT"] = str(out / "route")
        try:
            route = runpy.run_path(str(ROOT / "examples/v1-game-ios/ios/route.py"))
        finally:
            sys.argv = arguments
        portrait = grab("portrait", 1, before)
        driver = route["gesture"]("testLandscape")
        try:
            landscape = wait(state, lambda s: int(s.get("resize_events", 0)) > int(portrait["resize_events"])
                             and int(s.get("presented_epoch", 0)) > int(portrait["presented_epoch"])
                             and route["extent"](s)[0] > route["extent"](s)[1],
                             "presented landscape drawable", route["TOUCH_WINDOW"])
        finally:
            route["finish"](driver)
        if any(landscape[key] != portrait[key] for key in ("pid", "installs", "device_generation")):
            raise ValueError("rotation replaced the scene or device")
        returned = grab("landscape", 2, landscape)
        run("launch", device, "com.apple.Preferences")
        suspended = wait(state, lambda s: s.get("state") == "Suspended" and s.get("world_paused") == "true",
                         "background lifecycle")
        run("launch", device, BUNDLE)
        resumed = wait(state, lambda s: s.get("state") == "Active"
                       and int(s.get("presented", 0)) > int(suspended["presented"])
                       and int(s.get("world_tick", 0)) > int(suspended["world_tick"]), "foreground lifecycle", 100)
        if resumed["pid"] != returned["pid"]:
            raise ValueError("foreground replaced the scene process")
        grab("foreground", 3, resumed)
        report.update(background=suspended, foreground=resumed, passed=True)
    finally:
        (out / "report.json").write_text(json.dumps(report, indent=2) + "\n")
        print(out / "report.json", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--worker", action="store_true", help=argparse.SUPPRESS)
    args = parser.parse_args()
    os.chdir(ROOT)
    if not args.worker:
        simulator.admission()
        env = dict(os.environ, RUSTC_WRAPPER="")
        for command in [["examples/v1-game-ios/ios/build-simulator.sh", "--proving"],
                        ["examples/v1-game-ios/ios/build-uitest.sh"]]:
            subprocess.run(command, cwd=ROOT, env=env, check=True, timeout=600)
        simulator.exec_with_admission([sys.executable, simulator.__file__, "fresh", "--",
                                        sys.executable, __file__, *sys.argv[1:], "--worker"])
    out = args.output or ROOT / "tmp" / f"render2d-sim-{time.time_ns()}"
    worker(out.resolve())


if __name__ == "__main__":
    main()
