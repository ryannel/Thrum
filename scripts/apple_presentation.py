#!/usr/bin/env python3
"""Verify the presentation path on its named target."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
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
    rows = read_rows(arm / "frames.jsonl")
    calls = read_rows(arm / "configure.jsonl")
    ticks = annotate_display_ticks(arm, rows, calls, expected_conflict)
    if not rows or not any(row["presented"] for row in rows):
        raise ValueError(f"no presenting window frames in {arm}")
    return rows, calls, ticks


def read_rows(path):
    return [json.loads(line) for line in path.read_text().splitlines()]


def annotate_display_ticks(arm, rows, calls, expected_conflict):
    by_sequence = {}
    for row in rows:
        by_sequence.setdefault(row["sequence"], []).append(row)
    ticks = read_rows(arm / "display.jsonl")
    expected_numbers = list(range(1, len(ticks) + 1))
    if (not ticks or [tick["callback"] for tick in ticks] != expected_numbers
            or [tick["sequence"] for tick in ticks] != expected_numbers
            or not set(by_sequence).issubset(expected_numbers)):
        raise ValueError(f"display callback coverage is incomplete in {arm}")
    game_log = (arm / "game.log").read_text()
    if not expected_conflict:
        summary = re.search(r"; presented=(\d+); callbacks=(\d+); display_link_callbacks=(\d+);", game_log)
        counts = (sum(row["presented"] for row in rows), len(rows), len(ticks))
        if not summary or tuple(map(int, summary.groups())) != counts:
            raise ValueError(f"frame and callback logs disagree with the host's counts in {arm}: {counts}")
    presents = [row for row in rows if row["present_media"] is not None]
    if any(row["presented"] != (row["present_media"] is not None) for row in rows):
        raise ValueError(f"present timestamps disagree with the frame records in {arm}")
    work = [(row["sequence"], stage, started, ended)
            for row in rows for stage, started, ended in row["work"]]
    for index, tick in enumerate(ticks):
        successor = ticks[index + 1] if index + 1 < len(ticks) else None
        start = tick["media"]
        end = successor["media"] if successor else float("inf")
        if not start < min(end, tick["target"]) or tick["dispatched"] < start:
            raise ValueError(f"invalid display timestamp in {arm}: {tick}")
        # The last callback owns the remainder of the run. Present calls are
        # assigned by time, while frame work is also joined to its own target.
        tick["interval_end"] = successor["media"] if successor else None
        presented = [row for row in presents if start <= row["present_media"] < end]
        configure = [call for call in calls if call["started"] < end and start < call["ended"]]
        frames = [dict(status=row["status"], submission=row["submission"],
                       work_started=row["work"][0][1], work_ended=row["work"][-1][2],
                       present_media=row["present_media"])
                  for row in by_sequence.get(tick["sequence"], [])]
        if any(frame["work_started"] < tick["dispatched"] for frame in frames):
            raise ValueError(f"frame work precedes its display target in {arm}: {tick}")
        missed = [frame["status"] for frame in frames
                  if frame["present_media"] is None and frame["work_ended"] <= end]
        active = [dict(sequence=sequence, stage=stage,
                       overlap_ms=(min(end, ended) - max(start, started)) * 1000)
                  for sequence, stage, started, ended in work
                  if max(start, started) < min(end, ended)]
        tick["target_frames"] = frames
        if configure:
            tick["configure_calls"] = configure
        if active and not presented:
            tick["active_stages"] = active
        if not frames and successor:
            # The host keeps one pending target, overwritten by the next
            # callback. Complete frame coverage proves this target was unused.
            tick["superseded_by"] = successor["sequence"]
        if presented:
            tick["outcome"] = "presented"
            tick["presented_submissions"] = [row["submission"] for row in presented]
        elif any(end <= call["ended"] for call in configure):
            tick["outcome"] = "configure"
        elif tick["dispatched"] >= end:
            tick["outcome"] = "display callback dispatched after its interval"
        elif frames and frames[0]["work_started"] >= end:
            tick["outcome"] = "target frame work started after its interval"
        elif missed:
            tick["outcome"] = missed[-1]
            tick["frame_deferrals"] = missed
        elif not frames:
            tick["outcome"] = ("display target superseded before frame work" if successor
                               else "run ended with the last display target pending")
        else:
            # Use this target's stage at the close, even if work from another
            # target occupied more of the interval.
            finishing = next((stage for sequence, stage, started, ended in work
                              if sequence == tick["sequence"] and started < end <= ended), None)
            if finishing is None:
                raise ValueError(f"no observed reason for the missed display interval in {arm}: {tick}")
            tick["outcome"] = finishing
    if sum(len(tick.get("presented_submissions", [])) for tick in ticks) != len(presents):
        raise ValueError(f"not every present belongs to a recorded display interval in {arm}")
    (arm / "display-receipt.json").write_text(json.dumps(ticks, indent=2) + "\n")
    return ticks


def transition_receipt(rows, calls, ticks):
    switches = [row for row in rows if row["switch"] and row["switch"]["taken"]]
    window_sizes = {tuple(row["window"]) for row in rows}
    if len(switches) != 2 or len(calls) != len(switches):
        raise ValueError(f"expected two switches and two configure calls, observed {len(switches)} and {len(calls)}")
    for row, call in zip(switches, calls):
        if (call["path"] != "switch" or not call["succeeded"]
                or (call["sequence"], call["extent"]) != (row["sequence"], row["switch"]["extent"])
                or not row["switch"]["configure_started"] <= call["started"] <= call["ended"]
                <= row["switch"]["configure_ended"]):
            raise ValueError(f"configure did not belong to its successful generation switch: {call}")
    if len(window_sizes) < 10 or len({row["scale"] for row in rows}) != 2:
        raise ValueError("the sweep did not change the window and display record")
    previous = rows[0]["surface"]
    last_size_change = max(index for index in range(1, len(rows))
                           if rows[index]["window"] != rows[index - 1]["window"])
    last_scale_change = max(index for index in range(1, len(rows))
                            if rows[index]["scale"] != rows[index - 1]["scale"])
    scale_switch = rows.index(switches[1])
    if (switches[0]["surface"] == previous or switches[0]["scale"] != rows[0]["scale"]
            or switches[1]["surface"] != switches[0]["surface"]
            or switches[1]["scale"] == switches[0]["scale"]):
        raise ValueError("expected a settled extent switch followed by a backing-scale-only switch")
    held = held_in_motion = held_at_new_scale = 0
    for index, row in enumerate(rows):
        if ((not row["presented"] and row["status"] in ("NotAttempted", "SurfacePreparationBusy"))
                or any(stage == "synchronous extent build" for stage, _, _ in row["work"])):
            raise ValueError(f"unexplained frame or frame waiting for extent preparation: {row}")
        if row["switch"] and row["switch"]["configure_calls"]:
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
            if row["window"] != previous:
                held += 1
                held_in_motion += index <= last_size_change
            held_at_new_scale += last_scale_change <= index < scale_switch
    if held_in_motion < 5 or rows[-1]["surface"] != rows[-1]["window"]:
        raise ValueError("no held presentation through resize, or no settled surface")
    return dict(held_presents=held, distinct_window_grids=len(window_sizes),
                held_during_motion=held_in_motion, held_at_new_scale=held_at_new_scale,
                switches=[row["switch"] for row in switches], configure_attempts=calls,
                display_callbacks=len(ticks), ticks_without_present=sum(
                    tick["outcome"] != "presented" for tick in ticks))


def wall_tripwire(rows):
    times = sorted(row["wall_ms"] for row in rows)
    return dict(rows=len(times), median_ms=times[len(times) // 2],
                p95_ms=times[int((len(times) - 1) * .95)], worst_ms=times[-1])


def mac_transition(output):
    binary, env = mac_binary()
    console = subprocess.check_output(["/usr/sbin/ioreg", "-n", "Root", "-d1"], text=True)
    report = {}
    for recipe in RECIPE:
        baseline, baseline_calls, _ = mac_arm(output, binary, env, recipe, "baseline")
        if baseline_calls:
            raise ValueError(f"the baseline made unexpected configure calls: {baseline_calls}")
        sweep = mac_arm(output, binary, env, recipe, "sweep")
        control = mac_arm(output, binary, env, recipe, "synchronous", frames=75)
        synchronous_calls = len(control[1])
        if synchronous_calls < 10:
            raise ValueError("synchronous control did not repeatedly configure")
        try:
            transition_receipt(*control)
        except ValueError as error:
            rejection = str(error)
        else:
            raise ValueError("synchronous control passed the sweep checks")
        baseline_wall, sweep_wall = wall_tripwire(baseline), wall_tripwire(sweep[0])
        report[recipe] = dict(**transition_receipt(*sweep), control_calls=synchronous_calls,
                              control_rejection=rejection, baseline_wall=baseline_wall,
                              sweep_wall=sweep_wall,
                              wall_tripwire=sweep_wall["worst_ms"] > 2 * baseline_wall["p95_ms"])
    return dict(target="mac-metal", console_locked='"IOConsoleLocked" = Yes' in console,
                binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(), recipes=report)


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
                recipes={recipe: transition_receipt(*mac_arm(output, binary, env, recipe, "sweep", capture=True))
                         for recipe in RECIPE})


def mac_configure_fault(output):
    binary, env = mac_binary()
    report = {}
    for name in ("restore", "skip-restore"):
        rows, calls, _ = mac_arm(output, binary, env, "portable", name, frames=125,
                                 expected_conflict=name == "skip-restore")
        index = next(i for i, row in enumerate(rows) if row["switch"])
        switch = rows[index]["switch"]
        old = rows[index - 1]["surface"]
        acquired = next(row for row in rows[index:] if row["drawable"])
        descriptor, metal = acquired["drawable"]
        if not switch["configure_error"] or switch["taken"] or metal != old:
            raise ValueError(f"configure did not refuse and keep its former Metal drawable: {acquired}")
        if not calls or calls[0]["path"] != "switch" or calls[0]["succeeded"]:
            raise ValueError(f"the native configuration did not report the refused call: {calls}")
        if name == "restore":
            restore = calls[1:2]
            if (switch["restores"] != 1 or not switch["restored"] or descriptor != old
                    or not acquired["presented"] or not restore or restore[0]["path"] != "restore"
                    or not restore[0]["succeeded"] or restore[0]["extent"] != old):
                raise ValueError(f"restore did not return to coherent presentation: {acquired}")
        elif descriptor == metal or acquired["presented"] or len(calls) != 1:
            raise ValueError(f"control did not reveal a mismatch: {acquired}")
        report[name] = dict(switch=switch, first_acquire=acquired, previous_extent=old,
                            configure_attempts=calls)
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
