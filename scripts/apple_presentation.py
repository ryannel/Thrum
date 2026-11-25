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
from statistics import median

import v1_simulator_route


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


def mac_binary(*, cook=False, counters=False, crowd=False):
    env = {key: value for key, value in os.environ.items() if not key.startswith("THRUM_")}
    env.update(CARGO_TARGET_DIR=str(ROOT / "target"), RUSTC_WRAPPER="")
    package = "crowd-2d-view" if crowd else "v1-game"
    targets = ["-p", package, "--bin", package]
    if counters:
        feature = "thrum-app/creation-counters" if crowd else "v1-game/creation-counters"
        targets += ["--features", feature]
    if cook:
        targets += ["-p", "thrum-cook", "--bin", "thrum-cook"]
    run("rustup", "run", "stable", "cargo", "build", "--locked", "--offline", "--release",
        *targets, env=env, timeout=900)
    return ROOT / "target" / "release" / package, env


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


def run_mac_arm(output, binary, env, recipe, name, frames, capture):
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
    return arm, result


def mac_arm(output, binary, env, recipe, name, frames=165, expected_conflict=False, capture=False):
    arm, result = run_mac_arm(output, binary, env, recipe, name, frames, capture)
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
    v1_simulator_route.presentation("extent", route)
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
    v1_simulator_route.presentation("transition", route)
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


# The fixed route keeps the two history arms at the same submitted state, including phase.
SCALE_STEPS = {36: 85, 72: 75, 108: 67, 144: 50, 180: 67, 224: 75, 260: 85, 300: 100}


def scale_environment(output, *, counters=False):
    from run_v1_showcase import MOVING_ROUTE, SOURCE, store_root

    binary, env = mac_binary(cook=True, counters=counters)
    store = output / "store"
    run(str(ROOT / "target/release/thrum-cook"), "cook", str(SOURCE), str(store))
    route = json.loads(MOVING_ROUTE.read_text())
    root = store_root(store)
    if root != route["cooked_root"]:
        raise ValueError(f"cooked root {root} differs from the route's {route['cooked_root']}")
    return binary, dict(env, THRUM_ASSET_STORE=str(store), THRUM_SCENE="valley",
                        THRUM_RECIPE="portable", THRUM_RECONSTRUCTION="taau",
                        THRUM_ROUTE_OVERLAY=route["overlay"]), route


def scale_route(output, binary, env, route_path, arm, windows=()):
    capture = output / arm
    run(str(binary), env=dict(env, THRUM_CAPTURE_SCENE=str(capture),
        THRUM_CAPTURE_ROUTE=str(route_path), THRUM_ROUTE_RENDER_SCALE=arm,
        THRUM_CAPTURE_WINDOWS=json.dumps(windows)), timeout=300)
    header, *rows = read_rows(capture / "route.jsonl")
    if (header["backend"], header["recipe"], header["reconstruction"]) != ("Metal", "Ok(Portable)", "Taau"):
        raise ValueError(f"wrong scale target: {header}")
    return capture, header, {row["route_tick"]: row for row in rows}


def scale_history(output, binary, env):
    from run_v1_showcase import MOVING_ROUTE

    windows = [[tick, tick] for tick in SCALE_STEPS]
    live_path, header, kept = scale_route(output, binary, env, MOVING_ROUTE, "trace", windows)
    reset_path, _, reset = scale_route(output, binary, env, MOVING_ROUTE, "reset", windows)
    state_fields = ("tick", "alpha_bits", "rendered_solar_bits", "eye", "yaw", "pitch", "epoch",
                    "render_extent", "jitter_phase")
    rows = []
    for tick, scale in SCALE_STEPS.items():
        before, actual, control = kept[tick - 1], kept[tick], reset[tick]
        expected = [(axis * scale + 50) // 100 for axis in header["render_extent"]]
        if actual["render_extent"] != expected or actual["render_extent"] == before["render_extent"]:
            raise ValueError(f"scale did not step at routed tick {tick}")
        if (actual["history"][1] != before["history"][0] or control["history"][1] != 0
                or actual["jitter_phase"] != (before["jitter_phase"] + 1) % 32
                or any(actual[key] != control[key] for key in state_fields)):
            raise ValueError(f"history, jitter or state mismatch at routed tick {tick}")
        stem = f"tick-{actual['tick']:06}"
        def image(path):
            identity = json.loads((path / "windows" / f"{stem}.identity.json").read_text())
            return identity, (path / "windows" / f"{stem}.rgba8").read_bytes()
        a, live = image(live_path)
        b, without_history = image(reset_path)
        if a["exposure_gpu_words"] != b["exposure_gpu_words"] or a["timeline"] != b["timeline"]:
            raise ValueError(f"independent control did not hold exposure or timeline at {tick}")
        different = sum(left != right for left, right in zip(
            memoryview(live).cast("I"), memoryview(without_history).cast("I"), strict=True))
        if len(live) != 4 * header["render_extent"][0] * header["render_extent"][1] or different == 0:
            raise ValueError(f"submitted colour did not detect reset at routed tick {tick}")
        rows.append(dict(tick=tick, scale=scale, drawn=expected, jitter=actual["jitter_phase"],
                         previous_jitter=before["jitter_phase"], live_history=actual["history"],
                         reset_history=control["history"], different_pixels=different,
                         live_sha256=hashlib.sha256(live).hexdigest(),
                         reset_sha256=hashlib.sha256(without_history).hexdigest()))
    return dict(header=header, steps=rows)


def scale_reference(output, env, *, metalfx=False):
    from reconstruction_difference import compare, self_check
    from run_v1_showcase import ROUTE

    self_check()
    frames = output / "still-frames"
    env = dict(env, THRUM_RECONSTRUCTION_OUTPUT=str(frames),
               THRUM_RECONSTRUCTION_SCALE_STEPS="1", THRUM_REQUIRE_GPU="1")
    if metalfx:
        env = dict(env, THRUM_RECIPE="apple-tile", THRUM_METALFX_QUALITY="1")
    with (output / "reference.log").open("w") as log:
        subprocess.run(["cargo", "test", "--locked", "--offline", "--release", "-p", "v1-game",
                        "--lib", "reconstruction_reference_on_submitted_game_frames", "--",
                        "--ignored", "--nocapture"], cwd=ROOT / "examples/v1-game", env=env,
                       stdout=log, stderr=subprocess.STDOUT, check=True, timeout=900)
    checkpoints = {}
    arms = {"taau": 100, "taau-85": 85, "taau-75": 75, "taau-67": 67, "taau-50": 50}
    header = b"P6\n640 360\n255\n"
    for name in ROUTE:
        folder = frames / name
        def pixels(mode):
            raw = (folder / f"{mode}-64.ppm").read_bytes()
            if not raw.startswith(header) or len(raw) != len(header) + 640 * 360 * 3:
                raise ValueError(f"invalid whole-frame reference image: {name} {mode}")
            return raw[len(header):]
        rows = read_rows(folder / "frames.jsonl")
        held = {(r["state"], json.dumps(r["celestial"]), tuple(r["exposure"][:4]),
                 r["cooked_generation"]) for r in rows}
        if len(held) != 1 or not json.loads((folder / "linear-cycles.json").read_text())["within_float_rounding"]:
            raise ValueError(f"{name}: M3 reference arms did not hold the same state")
        reference = pixels("reference")
        results = {}
        for arm, percent in arms.items():
            expected = [(axis * percent + 50) // 100 for axis in (640, 360)]
            renders = [r for r in rows if r["mode"] == arm]
            if len(renders) != 64 or any(r["render_extent"] != expected for r in renders):
                raise ValueError(f"{name}: {arm} missing frames at drawn extent {expected}")
            results[str(percent)] = dict(drawn=expected, **compare(pixels(arm), reference))
            if metalfx:
                native = arm.replace("taau", "metalfx", 1)
                renders = [r for r in rows if r["mode"] == native]
                if len(renders) != 64 or any(r["render_extent"] != expected for r in renders):
                    raise ValueError(f"{name}: {native} missing frames at drawn extent {expected}")
                results[str(percent)] = dict(drawn=expected, taau=compare(pixels(arm), reference),
                                             metalfx=compare(pixels(native), reference))
        no_aa = compare(pixels("off"), reference)
        checkpoints[name] = dict(scales=results, no_aa=no_aa, held=next(iter(held)))
    return checkpoints


def mac_metalfx(output):
    from run_v1_showcase import MOVING_ROUTE

    binary, env, _ = scale_environment(output, counters=True)
    capture = output / "valley"
    command = dict(env, MTL_DEBUG_LAYER="1", THRUM_RECIPE="apple-tile",
                   THRUM_RECONSTRUCTION="metalfx", THRUM_ROUTE_RENDER_SCALE="trace",
                   THRUM_CAPTURE_SCENE=str(capture), THRUM_CAPTURE_ROUTE=str(MOVING_ROUTE))
    with (output / "metal-validation.log").open("w") as log:
        subprocess.run([binary], cwd=ROOT, env=command, stdout=log,
                       stderr=subprocess.STDOUT, check=True, timeout=300)
    validation = (output / "metal-validation.log").read_text()
    if "Metal API Validation Enabled" not in validation or re.search(
            r"Metal API Validation.*(?:error|failed)|validation error", validation, re.I):
        raise ValueError("missing validation activation or Metal validation error")
    header, *frames = read_rows(capture / "route.jsonl")
    if (header["backend"], header["recipe"], header["reconstruction"]) != (
            "Metal", "Ok(AppleTile)", "MetalFx"):
        raise ValueError(f"wrong MetalFX target: {header}")
    selected = json.loads((capture / "native-provider.json").read_text())
    if (selected["native_instrumentation"] != "Counted"
            or selected["native_creations"] != {"build": 1, "transition": 0, "play": 0}
            or selected["pipeline_creations_in_play"] != 0 or selected["twin_reason"] is not None
            or selected["completed"] != selected["submitted"] or selected["discards"]):
        raise ValueError(f"unexpected native provider lifecycle: {selected}")
    scales = {}
    for frame in frames:
        native = frame["native_segment"]
        outcome = re.fullmatch(r"Timed\(\[(.+), (.+)\]\)", native["outcome"])
        if (native["pass"] != selected["pass"] or native["submission"] != frame["submission"]
                or not outcome or not 0 < float(outcome[1]) < float(outcome[2])):
            raise ValueError(f"missing actual MetalFX time for {frame}")
        percent = frame["render_scale"]
        expected = [(axis * percent + 50) // 100 for axis in header["render_extent"]]
        if frame["render_extent"] != expected:
            raise ValueError(f"incorrect MetalFX input content for {frame}")
        scales.setdefault(percent, []).append(1000 * (float(outcome[2]) - float(outcome[1])))
    if set(scales) != {100, 85, 75, 67, 50}:
        raise ValueError(f"missing MetalFX scales: {scales.keys()}")
    return dict(target="mac-metal", header=header, provider=selected,
                by_scale={scale: dict(frames=len(times), median_native_ms=median(times))
                          for scale, times in scales.items()},
                binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest())


def mac_metalfx_quality(output):
    _, env, _ = scale_environment(output)
    return dict(target="mac-metal", checkpoints=scale_reference(output, env, metalfx=True))


def mac_metalfx_motion(output):
    log = output / "motion.log"
    with log.open("w") as stream:
        subprocess.run(["cargo", "test", "--locked", "--offline", "-p", "thrum-render",
                        "--features", "creation-counters", "--test", "metalfx_motion"],
                       cwd=ROOT, stdout=stream, stderr=subprocess.STDOUT,
                       env=dict(os.environ, THRUM_REQUIRE_GPU="1", RUST_TEST_NOCAPTURE="1",
                                MTL_DEBUG_LAYER="1"), check=True, timeout=300)
    return dict(target="mac-metal", metal_validation=True, log=str(log))


def mac_metalfx_exposure(output):
    _, env, _ = scale_environment(output, counters=True)
    frames = output / "frames"
    for option in ("THRUM_METALFX_EXPOSURE_LINEAR", "THRUM_METALFX_EXPOSURE_SCALE_67"):
        if option in os.environ:
            env[option] = os.environ[option]
    with (output / "exposure.log").open("w") as log:
        subprocess.run(["cargo", "test", "--locked", "--offline", "--release", "-p", "v1-game",
                        "--features", "creation-counters", "--lib",
                        "metalfx_exposure_on_submitted_game_frames", "--", "--ignored", "--nocapture"],
                       cwd=ROOT / "examples/v1-game", stdout=log, stderr=subprocess.STDOUT,
                       env=dict(env, THRUM_REQUIRE_GPU="1", THRUM_RECONSTRUCTION_OUTPUT=str(frames)),
                       check=True, timeout=900)
    off = read_rows(frames / "off.jsonl")
    repeat = read_rows(frames / "repeat.jsonl")
    grain = max(abs(a["mean_luminance"] - b["mean_luminance"])
                for a, b in zip(off, repeat, strict=True))
    (output / "noise-floor.json").write_text(json.dumps(dict(max_abs_mean_luminance=grain), indent=2))
    taau = read_rows(frames / "taau.jsonl")
    metalfx = read_rows(frames / "metalfx.jsonl")
    rows = []
    for a, b, c in zip(off, taau, metalfx, strict=True):
        if len({(row["arm"], row["index"], tuple(row["exposure"][:4]))
                for row in (a, b, c)}) != 1 or c["reconstructor"] != "MetalFx":
            raise ValueError(f"mismatched exposure arm: {a}, {b}, {c}")
        observed = [row["mean_luminance"] for row in (a, b, c)]
        portable = abs(observed[0] - observed[1])
        native = abs(observed[0] - observed[2])
        rows.append(dict(arm=a["arm"], index=a["index"], means=observed,
                         taau_error=portable, metalfx_error=native,
                         metalfx_excess=native - portable, history=c["history"],
                         normalization_ns=c["normalization_ns"]))
    if {row["exposure"][0] for row in off if row["arm"] == "storage-step"} != {1056964608, 1073741824}:
        raise ValueError("the GPU sample did not record both forced storage scales")
    repairs = {arm: json.loads((frames / f"{arm}-repairs.json").read_text())
               for arm in ("scrub", "storage-step")}
    if not all(row["normalization_ns"] and row["normalization_ns"] > 0 for row in rows):
        raise ValueError("normalization GPU cost is missing")
    receipt = dict(target="mac-metal", exposure_path="normalized-scene-linear",
                   noise=grain, repairs_by_arm=repairs, rows=rows)
    (output / "comparison.json").write_text(json.dumps(receipt, indent=2))
    if any(row["metalfx_excess"] > grain for row in rows):
        raise ValueError("MetalFX normalization exceeds TAAU outside the measured noise floor")
    return receipt


def mac_scale(output):
    binary, env, _ = scale_environment(output)
    build_sha256 = hashlib.sha256(binary.read_bytes()).hexdigest()
    history = scale_history(output, binary, env)
    (output / "history.json").write_text(json.dumps(history, indent=2) + "\n")
    stills = scale_reference(output, env)
    (output / "stills.json").write_text(json.dumps(stills, indent=2) + "\n")
    misses = [name for name, row in stills.items()
              if not (row["scales"]["100"]["p99"] <= 3
                      and row["scales"]["100"]["p99"] < row["no_aa"]["p99"])]
    if misses:
        raise ValueError(f"scale 1.0 missed M3 tolerance at {', '.join(misses)}")
    return dict(target="mac-metal", history=history, stills=stills, binary_sha256=build_sha256)


def scale_cost_rows(rows):
    """Require complete timestamps. Times and ratios only describe the observed load."""
    output = []
    for scale in (100, 85, 75, 67, 50):
        frames = [row for tick, row in rows.items() if row["render_scale"] == scale
                  and tick >= 12 and all(rows.get(tick - n, {}).get("render_scale") == scale
                                        for n in range(1, 9))]
        if not frames:
            raise ValueError(f"missing a settled interval at {scale}%")
        records = []
        for frame in frames:
            times = frame["gpu_pass_ns"]
            if (not times or times.get("synthetic forward opaque") is None
                    or times.get("portable TAAU") is None):
                raise ValueError(f"missing forward or TAAU timestamp at {frame['route_tick']}")
            atmosphere = [value for name, value in times.items()
                          if name.startswith("atmosphere ") and name != "atmosphere stars"]
            if not atmosphere or any(value is None for value in atmosphere):
                raise ValueError(f"missing atmosphere timestamps at {frame['route_tick']}")
            records.append(dict(forward_sky=times["synthetic forward opaque"]
                                + (times.get("atmosphere stars") or 0),
                                taau=times["portable TAAU"], atmosphere=sum(atmosphere)))
        width, height = frames[0]["render_extent"]
        output.append(dict(scale=scale, drawn=[width, height], area=width * height,
                           frames=len(records), p50_ns={name: median(r[name] for r in records)
                                                       for name in records[0]}))
    return output


def mac_scale_costs(output):
    from t2_load import LoadRecord

    binary, env, route = scale_environment(output)
    build_sha256 = hashlib.sha256(binary.read_bytes()).hexdigest()
    route["render_extent"] = [1920, 1080]
    route_path = output / "moving-1080p.json"
    route_path.write_text(json.dumps(route) + "\n")
    runs = []
    for index in (1, 2):
        directory = output / f"run-{index}"
        directory.mkdir()
        load = LoadRecord(directory)
        try:
            _, header, rows = scale_route(directory, binary, env, route_path, "trace")
            costs = scale_cost_rows(rows)
        finally:
            observed = load.finish()
        native = costs[0]["p50_ns"]
        for row in costs:
            measured = row["p50_ns"]
            expected_forward = native["forward_sky"] * row["area"] / costs[0]["area"]
            row["cost_account"] = dict(pixel_bound_forward_sky_ns=expected_forward,
                                       fixed_atmosphere_ns=native["atmosphere"],
                                       taau_estimate_ns=[200_000, 500_000])
            row["tripwires"] = {
                "forward_sky_differs_from_pixel_ratio_by_over_25_percent":
                    abs(measured["forward_sky"] - expected_forward) > 0.25 * expected_forward,
                "taau_outside_estimate_0_2_to_0_5_ms":
                    not 200_000 <= measured["taau"] <= 500_000,
                "atmosphere_differs_from_native_by_over_25_percent":
                    abs(measured["atmosphere"] - native["atmosphere"]) > 0.25 * native["atmosphere"],
            }
        runs.append(dict(index=index, header=header, load=observed, costs=costs))
    return dict(target="mac-metal", build_sha256=build_sha256, runs=runs, timing_decides_pass=False)

def ios_edr(output):
    route = output / "route"
    v1_simulator_route.presentation("edr", route)
    return dict(target="ios-sim", scope="simulator function only",
                before=str(route / "edr-before.txt"), after=str(route / "edr-after.txt"))


def ios_reset(output):
    route = output / "route"
    v1_simulator_route.presentation("reset", route)
    state = dict(line.split("=", 1) for line in (route / "reset-completed.txt").read_text().splitlines()
                 if "=" in line)
    discarded, prior = map(int, state["background_discard"].split(","))
    resumed, predecessor, phase = map(int, state["background_resume_history"].split(","))
    return dict(target="ios-sim", reconstructor="TAAU",
                background_discards=int(state["background_discards"]),
                discarded_order=discarded, discarded_predecessor=prior,
                resumed_order=resumed, resumed_predecessor=predecessor, resumed_jitter_phase=phase,
                simulator_build=json.loads((route / "identity.json").read_text()))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("check", choices=["extent", "transition", "configure-fault", "transition-capture", "scale", "scale-costs", "edr", "edr-identity", "edr-modes", "edr-headroom", "edr-route", "governor", "governor-identity", "governor-thermal", "reset", "metalfx", "metalfx-quality", "metalfx-exposure", "metalfx-motion"])
    parser.add_argument("--target", choices=["mac-metal", "ios-sim"], required=True)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--steady", action="store_true", help="qualify the unloaded governor arm")
    parser.add_argument("--realtime-capture", action="store_true", help="retain the T2 governor tap")
    args = parser.parse_args()
    if args.output is None:
        scratch = ROOT / "tmp"
        scratch.mkdir(exist_ok=True)
        output = Path(tempfile.mkdtemp(prefix="presentation-extent-", dir=scratch))
    else:
        output = args.output.resolve()
        output.mkdir(parents=True)
    from apple_display_proof import mac_edr, mac_edr_modes, mac_edr_headroom, mac_edr_identity, mac_edr_route

    from apple_governor_proof import mac_governor, mac_governor_identity, mac_governor_thermal

    if (args.steady or args.realtime_capture) and args.check != "governor":
        parser.error("governor options require the governor check")
    checks = {("governor", "mac-metal"): lambda output: mac_governor(
                  output, steady=args.steady, capture=args.realtime_capture),
              ("governor-identity", "mac-metal"): mac_governor_identity,
              ("governor-thermal", "mac-metal"): mac_governor_thermal,("edr", "ios-sim"): ios_edr,
              ("reset", "ios-sim"): ios_reset,
              ("metalfx", "mac-metal"): mac_metalfx,
              ("metalfx-quality", "mac-metal"): mac_metalfx_quality,
              ("metalfx-exposure", "mac-metal"): mac_metalfx_exposure,
              ("metalfx-motion", "mac-metal"): mac_metalfx_motion,
              ("edr", "mac-metal"): mac_edr,
              ("edr-modes", "mac-metal"): mac_edr_modes,
              ("edr-headroom", "mac-metal"): mac_edr_headroom,
              ("edr-identity", "mac-metal"): mac_edr_identity,
              ("edr-route", "mac-metal"): mac_edr_route,
              ("scale", "mac-metal"): mac_scale, ("scale-costs", "mac-metal"): mac_scale_costs,
              ("extent", "mac-metal"): mac_extent, ("extent", "ios-sim"): ios_extent,
              ("transition", "ios-sim"): ios_transition,
              ("transition", "mac-metal"): mac_transition,
              ("configure-fault", "mac-metal"): mac_configure_fault,
              ("transition-capture", "mac-metal"): mac_transition_capture}
    check = checks.get((args.check, args.target))
    if check is None:
        parser.error(f"{args.check} is unavailable on {args.target}")
    commit = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    report = check(output)
    report["commit"] = commit
    report_path = output / f"{args.check}.json"
    report_path.write_text(json.dumps(report, indent=2) + "\n")
    print(f"{args.target} {args.check} passed in {report_path}")


if __name__ == "__main__":
    main()
