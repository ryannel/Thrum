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


def mac_binary():
    env = {key: value for key, value in os.environ.items() if not key.startswith("THRUM_")}
    env.update(CARGO_TARGET_DIR=str(ROOT / "target"), RUSTC_WRAPPER="")
    run("rustup", "run", "stable", "cargo", "build", "--locked", "--offline", "--release",
        "-p", "v1-game", "--bin", "v1-game", env=env, timeout=900)
    return ROOT / "target/release/v1-game", env


def mac_extent(output):
    binary, env = mac_binary()
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
            run("/usr/bin/caffeinate", "-d", "-i", "-u", str(binary), env=launch, timeout=90)
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


def mac_arm(output, binary, env, recipe, name, frames=165, expected_conflict=False, capture=False):
    arm = output / f"{recipe}-{name}"
    arm.mkdir()
    launch = dict(env, THRUM_RECIPE=recipe, THRUM_V1_FRAMES=str(frames),
                  THRUM_MAC_TRANSITION=str(arm), THRUM_MAC_TRANSITION_ARM=name)
    if capture:
        launch.update(THRUM_MAC_TRANSITION_TAP=str(arm), THRUM_MAC_TAP="on")
    with (arm / "game.log").open("w") as log:
        result = subprocess.run(["/usr/bin/caffeinate", "-d", "-i", "-u", str(binary)],
                                env=launch, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT,
                                timeout=120, check=False)
    if expected_conflict:
        if result.returncode == 0 or "control acquired conflicting drawable extents" not in (
            arm / "game.log"
        ).read_text():
            raise ValueError(f"restore control did not see the descriptor mismatch in {arm}")
    else:
        result.check_returncode()
    rows = [json.loads(line) for line in (arm / "frames.jsonl").read_text().splitlines()]
    if (arm / "display.jsonl").exists():
        annotate_display_ticks(arm, rows)
    if not rows or not any(row["presented"] for row in rows):
        raise ValueError(f"no presenting window frames in {arm}")
    return rows


def annotate_display_ticks(arm, rows):
    by_sequence = {}
    for row in rows:
        by_sequence.setdefault(row["sequence"], []).append(row)
    switches = [row["switch"] for row in rows if row["switch"] and row["switch"]["configure_calls"]]
    ticks = [json.loads(line) for line in (arm / "display.jsonl").read_text().splitlines()]
    for tick in ticks:
        render = by_sequence.get(tick["sequence"], [])
        configure = next((switch for switch in switches
                          if switch["configure_started"] <= tick["media"] <= switch["configure_ended"]), None)
        if any(row["presented"] for row in render):
            tick["outcome"] = "presented"
        elif configure:
            tick["outcome"] = "configure"
            tick["configure_ms"] = configure["configure_ms"]
        elif render:
            tick["outcome"] = render[-1]["status"]
        else:
            tick["outcome"] = "render worker busy" if tick["render_busy"] else "display target coalesced"
    (arm / "display-receipt.json").write_text(json.dumps(ticks, indent=2) + "\n")


def transition_receipt(rows):
    switches = [row for row in rows if row["switch"] and row["switch"]["configure_calls"]]
    window_sizes = {tuple(row["window"]) for row in rows}
    if len(switches) != 2:
        raise ValueError(f"expected extent and scale switches, observed {len(switches)} configure calls")
    if len(window_sizes) < 10 or len({row["scale"] for row in rows}) != 2:
        raise ValueError("the sweep did not change the window and display record")
    previous = rows[0]["surface"]
    held = 0
    for row in rows:
        if row["switch"]:
            switch = row["switch"]
            if (switch["configure_calls"], switch["taken"], switch["restores"]) != (1, 1, 0):
                raise ValueError(f"unexpected surface calls during sweep: {row}")
            previous = switch["extent"]
            if previous != row["window"]:
                raise ValueError(f"switch took an intermediate size: {row}")
        if row["surface"] != previous:
            raise ValueError(f"surface changed outside of a switch: {row}")
        if row["presented"]:
            if row["drawable"] != [previous, previous]:
                raise ValueError(f"drawable does not match the active generation: {row}")
            held += row["window"] != previous
        elif row["status"] == "NotAttempted":
            raise ValueError(f"unexplained frame without a present: {row}")
    if held < 5 or rows[-1]["surface"] != rows[-1]["window"]:
        raise ValueError("no held presentation through resize, or no settled surface")
    return dict(held_presents=held, distinct_window_grids=len(window_sizes),
                switches=[row["switch"] for row in switches])


def wall_tripwire(rows):
    times = sorted(row["wall_ms"] for row in rows)
    return dict(rows=len(times), median_ms=times[len(times) // 2],
                p95_ms=times[int((len(times) - 1) * .95)], worst_ms=times[-1])


def mac_transition(output):
    binary, env = mac_binary()
    console = subprocess.check_output(["/usr/sbin/ioreg", "-n", "Root", "-d1"], text=True)
    report = {}
    for recipe in RECIPE:
        baseline = mac_arm(output, binary, env, recipe, "baseline")
        sweep = mac_arm(output, binary, env, recipe, "sweep")
        control = mac_arm(output, binary, env, recipe, "synchronous", frames=75)
        synchronous_calls = sum(bool(row["switch"]) for row in control)
        if synchronous_calls < 10:
            raise ValueError("synchronous control did not repeatedly configure")
        try:
            transition_receipt(control)
        except ValueError as error:
            rejection = str(error)
        else:
            raise ValueError("synchronous control passed the sweep checks")
        baseline_wall, sweep_wall = wall_tripwire(baseline), wall_tripwire(sweep)
        report[recipe] = dict(**transition_receipt(sweep), control_calls=synchronous_calls,
                              control_rejection=rejection, baseline_wall=baseline_wall,
                              sweep_wall=sweep_wall,
                              wall_tripwire=sweep_wall["worst_ms"] > 2 * baseline_wall["p95_ms"])
    return dict(target="mac-metal", console_locked='"IOConsoleLocked" = Yes' in console, recipes=report)


def mac_transition_capture(output):
    binary, env = mac_binary()
    pattern = os.environ.get("THRUM_MAC_PATTERN") == "1"
    if pattern:
        env["THRUM_MAC_PATTERN"] = "1"
    else:
        assets = os.environ.get("THRUM_ASSET_STORE")
        if not assets:
            raise ValueError("transition capture needs THRUM_ASSET_STORE with the cooked valley")
        env.update(THRUM_ASSET_STORE=str(Path(assets).resolve()), THRUM_SCENE="valley")
    return dict(target="mac-metal", mode="realtime", scene="grid" if pattern else "valley",
                recipes={recipe: transition_receipt(mac_arm(output, binary, env, recipe, "sweep", capture=True))
                         for recipe in RECIPE})


def mac_configure_fault(output):
    binary, env = mac_binary()
    report = {}
    for name in ("restore", "skip-restore"):
        rows = mac_arm(output, binary, env, "portable", name, frames=125,
                       expected_conflict=name == "skip-restore")
        index = next(i for i, row in enumerate(rows) if row["switch"])
        switch = rows[index]["switch"]
        old = rows[index - 1]["surface"]
        acquired = next(row for row in rows[index:] if row["drawable"])
        descriptor, metal = acquired["drawable"]
        if not switch["configure_error"] or switch["taken"] or metal != old:
            raise ValueError(f"configure did not refuse and keep its former Metal drawable: {acquired}")
        if name == "restore":
            if switch["restores"] != 1 or not switch["restored"] or descriptor != old or not acquired["presented"]:
                raise ValueError(f"restore did not return to coherent presentation: {acquired}")
        elif descriptor == metal or acquired["presented"]:
            raise ValueError(f"control did not reveal a mismatch: {acquired}")
        report[name] = dict(switch=switch, first_acquire=acquired, previous_extent=old)
    return dict(target="mac-metal", arms=report)


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
    parser.add_argument("check", choices=["extent", "transition", "configure-fault", "transition-capture"])
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
              ("transition", "ios-sim"): ios_transition,
              ("transition", "mac-metal"): mac_transition,
              ("configure-fault", "mac-metal"): mac_configure_fault,
              ("transition-capture", "mac-metal"): mac_transition_capture}
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
