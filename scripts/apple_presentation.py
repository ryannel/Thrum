#!/usr/bin/env python3
"""Verify the presentation path on its named target."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile


ROOT = Path(__file__).resolve().parents[1]
RECIPE = {"portable": "Ok(Portable)", "apple-tile": "Ok(AppleTile)"}


def run(*argv, env=None, timeout=300):
    return subprocess.run(argv, cwd=ROOT, env=env, check=True, timeout=timeout)


def check_pixels(raw, extent):
    width, height = extent
    if len(raw) != width * height * 4:
        raise ValueError("capture is not a tightly packed RGBA8 backing grid")
    view = memoryview(raw)
    for y in range(height):
        row_start = y * width * 4
        row = view[row_start:row_start + width * 4]
        expected = bytes(
            channel for x in range(width)
            for channel in (x % 251, y % 241, (x // 251 + y // 241 * 17) % 239, 255)
        )
        if row != expected:
            raise ValueError(f"submitted capture does not match the backing grid at row {y}")
    return hashlib.sha256(raw).hexdigest()


def mac_extent(output):
    env = {key: value for key, value in os.environ.items() if not key.startswith("THRUM_")}
    env.update(CARGO_TARGET_DIR=str(ROOT / "target"), RUSTC_WRAPPER="")
    run("rustup", "run", "stable", "cargo", "build", "--locked", "--offline", "--release",
        "-p", "v1-game", "--bin", "v1-game", env=env, timeout=900)
    binary = ROOT / "target/release/v1-game"
    rows = []
    console = subprocess.check_output(
        ["/usr/sbin/ioreg", "-n", "Root", "-d1"], text=True, cwd=ROOT,
    )
    for recipe, resolved in RECIPE.items():
        for setting in ("1280x720", "1920x1080", "fullscreen"):
            arm = output / f"{recipe}-{setting}"
            arm.mkdir()
            launch = dict(env, THRUM_RECIPE=recipe, THRUM_V1_FRAMES="12", THRUM_MAC_PATTERN="1",
                          THRUM_MAC_EXTENT_PROOF=str(arm), THRUM_MAC_EXTENT_WINDOW=setting)
            run("/usr/bin/caffeinate", "-d", "-i", str(binary), env=launch, timeout=90)
            window = json.loads((arm / "window.json").read_text())
            submission = json.loads((arm / "submitted.json").read_text())
            backing = window["backing"]
            if setting == "fullscreen":
                if not window["fullscreen"]:
                    raise ValueError(f"window did not enter fullscreen: {window}")
            elif window["fullscreen"] or window["points"] != list(map(int, setting.split("x"))):
                raise ValueError(f"requested window point size did not reach AppKit: {window}")
            if submission["recipe"] != resolved:
                raise ValueError(f"{recipe} resolved to {submission['recipe']}")
            for field in ("capture_extent", "render_extent", "source", "destination", "drawable"):
                if submission[field] != backing:
                    raise ValueError(f"{field} does not match window backing {backing}: {submission}")
            if submission["viewport"] != [0, 0, *backing] or (
                submission["attachment_extents"] != [backing] * 4
            ):
                raise ValueError(f"submitted frame does not cover every backing pixel: {submission}")
            raw = arm / "submitted.rgba"
            digest = check_pixels(raw.read_bytes(), backing)
            rows.append(dict(window=window, submission=submission, rgba_sha256=digest))
    return dict(
        target="mac-metal", rows=rows,
        console_locked='"IOConsoleLocked" = Yes' in console,
        binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
    )


def ios_extent(output):
    route = output / "rotation"
    build = ROOT / "examples/v1-game-ios/ios/check-simulator.sh"
    run(str(build), "--extent", env=dict(os.environ, THRUM_V1_ROUTE_OUT=str(route)), timeout=1500)
    rows = [
        json.loads((route / f"extent-{name}-extent.json").read_text())
        for name in ("portrait", "landscape", "return-portrait")
    ]
    portrait, landscape, returned = (row["drawable"] for row in rows)
    if not (portrait[0] < portrait[1] and landscape[0] > landscape[1] and portrait == returned):
        raise ValueError(f"unexpected oriented surface sequence: {rows}")
    return dict(target="ios-sim", scope="orientation and simulator function only", rows=rows,
                simulator_build=json.loads((route / "identity.json").read_text()))


def ios_transition(output):
    route = output / "rotation"
    build = ROOT / "examples/v1-game-ios/ios/check-simulator.sh"
    run(str(build), "--transition", env=dict(os.environ, THRUM_V1_ROUTE_OUT=str(route)), timeout=1500)
    rows = []
    for name in ("transition-landscape", "transition-portrait"):
        state = dict(line.split("=", 1) for line in (route / f"{name}.txt").read_text().splitlines()
                     if "=" in line)
        epoch, drawable, replies = state["resize_first_present"].split(",")
        rows.append(dict(orientation=name.removeprefix("transition-"), epoch=int(epoch),
                         drawable=[int(side) for side in drawable.split("x")], replies=int(replies),
                         pid=int(state["pid"]), installs=int(state["installs"]),
                         replacements=int(state["replacements"]),
                         device_generation=int(state["device_generation"]),
                         unconfigured_acquires=int(state["unconfigured_acquires"]),
                         resize_raced_presents=int(state["resize_raced_presents"]),
                         old_extent_presents=int(state["old_extent_presents"])))
    return dict(target="ios-sim", scope="simulator function only", rotations=rows,
                simulator_build=json.loads((route / "identity.json").read_text()))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("check", choices=["extent", "transition"])
    parser.add_argument("--target", choices=["mac-metal", "ios-sim"], required=True)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    if args.output is None:
        scratch = ROOT / "tmp"
        scratch.mkdir(exist_ok=True)
        output = Path(tempfile.mkdtemp(prefix="presentation-extent-", dir=scratch))
    else:
        output = args.output.resolve()
        output.mkdir(parents=True)
    checks = {("extent", "mac-metal"): mac_extent, ("extent", "ios-sim"): ios_extent,
              ("transition", "ios-sim"): ios_transition}
    check = checks.get((args.check, args.target))
    if check is None:
        parser.error(f"{args.check} is unavailable on {args.target}")
    report = check(output)
    report["commit"] = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    report_path = output / f"{args.check}.json"
    report_path.write_text(json.dumps(report, indent=2) + "\n")
    print(f"{args.target} {args.check} passed in {report_path}")


if __name__ == "__main__":
    main()
