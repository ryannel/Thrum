"""Verify the launch setting and animated AppKit mode changes on the Mac display."""

import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

from apple_governor_proof import awake_display
from apple_presentation import RECIPE, ROOT, read_rows, scale_environment

BACKING = [3456, 2234]
SCALED = [2592, 1676]


def launch(arm, binary, launch_env, *, mode=False, value=None, capture=False):
    arm.mkdir()
    env = dict(
        launch_env,
        THRUM_V1_FRAMES="160" if mode else "1",
        THRUM_MAC_TRANSITION=str(arm),
        THRUM_MAC_TRANSITION_ARM="output-scale-mode" if mode else "output-scale-fullscreen",
        THRUM_MAC_EDR="2",
    )
    if value is not None:
        env["THRUM_MAC_OUTPUT_SCALE"] = str(value)
    if mode:
        env["THRUM_MAC_OUTPUT_SCALE_PROOF_REGISTRY"] = "75"
    if capture:
        display = arm / "display"
        display.mkdir()
        env.update(THRUM_MAC_EDR_PROOF=str(display), THRUM_MAC_EDR_TAP="1")
    with (arm / "game.log").open("w") as log:
        result = subprocess.run(
            [binary], cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT,
            timeout=150, check=False,
        )
    if value == 80:
        message = (arm / "game.log").read_text()
        if result.returncode == 0 or "THRUM_MAC_OUTPUT_SCALE must be" not in message:
            raise ValueError(f"{arm}: invalid output scale was not refused by name")
        return {"exit_code": result.returncode, "message": message.strip()}
    if result.returncode:
        raise ValueError(f"{arm}: game exited {result.returncode}: {(arm / 'game.log').read_text()[-4000:]}")
    return (
        read_rows(arm / "output-requests.jsonl"),
        read_rows(arm / "frames.jsonl"),
        read_rows(arm / "configure.jsonl"),
        json.loads((arm / "prepared.json").read_text()),
    )


def require_frame(row, extent):
    frame = row["frame_extent"]
    if (row["surface"] != extent or row["drawable"] != [extent, extent]
            or frame["reconstruction"] != extent or frame["destination"] != extent
            or frame["viewport"] != [0, 0, *extent] or frame["attachment_extents"] != [extent] * 4):
        raise ValueError(f"expected a complete {extent} frame, got {row}")


def require_output_records(arm, presented):
    fields = ("backing_extent", "output_scale", "output_extent", "output_scale_forced")
    expected = {row["submission"]: tuple(row[key] for key in fields) for row in presented}
    game = read_rows(arm / "game-output.jsonl")
    for source in ("frame", "display"):
        rows = [row for row in game if row["source"] == source]
        outputs = {row["submission"]: tuple(row[key] for key in fields) for row in rows}
        if len(rows) != len(outputs) or outputs != expected:
            raise ValueError(f"{arm}: game {source} records did not match the submitted drawables")
    return game


def require_output_policy(row, request):
    actual = (row["backing_extent"], row["output_extent"], row["output_scale"],
              row["output_scale_forced"])
    expected = (request["backing"], request["output"], request["output_scale"], request["forced"])
    if actual != expected:
        raise ValueError(f"presented policy {actual} does not match generation {expected}")


def capture_resize(arm, binary, launch_env, scale):
    arm.mkdir()
    env = dict(launch_env, THRUM_V1_FRAMES="120", THRUM_MAC_OUTPUT_SCALE=str(scale),
               THRUM_MAC_EXTENT_WINDOW="640x360", THRUM_MAC_EXTENT_RESIZE="800x450",
               THRUM_MAC_EXTENT_PROOF=str(arm))
    with (arm / "game.log").open("w") as log:
        subprocess.run([binary], cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT,
                       timeout=150, check=True)
    window = json.loads((arm / "window.json").read_text())
    if window["points"] != [640, 360]:
        raise ValueError(f"{arm}: unexpected initial window: {window}")
    results = {}
    for stem, points in (("submitted", (640, 360)), ("resized", (800, 450))):
        row = json.loads((arm / f"{stem}.json").read_text())
        raw = (arm / f"{stem}.rgba").read_bytes()
        extent = [(round(axis * window["scale"]) * scale + 50) // 100 for axis in points]
        if (any(row[field] != extent for field in ("capture_extent", "destination", "drawable"))
                or row["viewport"] != [0, 0, *extent] or len(raw) != extent[0] * extent[1] * 4
                or (stem == "resized" and row["first_resized_submission"] != row["submission"])):
            raise ValueError(f"{arm}: {stem} did not capture the submitted {extent} drawable: {row}")
        results[stem] = dict(row, rgba_sha256=hashlib.sha256(raw).hexdigest())
    if results["resized"]["submission"] <= results["submitted"]["submission"]:
        raise ValueError(f"{arm}: the resized capture did not follow the original")
    return results


def mac_output_scale(output):
    report = {}
    with tempfile.TemporaryDirectory(prefix="output-scale-store-", dir=ROOT / "tmp") as scratch:
        binary, env, _ = scale_environment(Path(scratch), counters=True)
        report["binary_sha256"] = hashlib.sha256(binary.read_bytes()).hexdigest()
        report["recipes"] = {}
        with awake_display():
            for recipe, resolved in RECIPE.items():
                recipe_report = {}
                env.update(THRUM_RECIPE=recipe,
                           THRUM_RECONSTRUCTION="metalfx" if recipe == "apple-tile" else "taau")
                for value, scale, extent in [(75, 75, SCALED), (None, 100, BACKING)]:
                    arm = output / f"{recipe}-{scale}"
                    requests, frames, configures, identity = launch(
                        arm, binary, env, value=value, capture=True)
                    if len(requests) != 1 or configures:
                        raise ValueError(f"{arm}: the launch changed its initial generation")
                    request = requests[0]
                    layer = request["layer"]
                    if (request["event"] != "request" or not request["fullscreen"]
                            or request["backing"] != BACKING or request["monitor_extent"] != BACKING
                            or request["output"] != extent
                            or request["output_scale"] != scale or request["forced"] != (value is not None)
                            or layer["backing"] != BACKING
                            or layer["gravity"] != "resize" or layer["magnification_filter"] != "linear"):
                        raise ValueError(f"{arm}: fullscreen output or layer mismatch: {request}")
                    if identity["recipe"] != resolved:
                        raise ValueError(f"{arm}: wrong recipe {identity['recipe']}")
                    presented = [row for row in frames if row["presented"]]
                    for row in presented:
                        require_frame(row, extent)
                        require_output_policy(row, request)
                    game_records = require_output_records(arm, presented)
                    capture_rows = read_rows(arm / "display" / "frames.jsonl")
                    images = {row["capture_submission"]: row for row in capture_rows
                              if "capture_submission" in row and row["presented"]}
                    if not presented or set(images) != {row["submission"] for row in presented}:
                        raise ValueError(f"{arm}: engine tap did not collect every presented submission")
                    if any(image["capture_source"] != "Edr" or image["extent"] != extent
                           or image["pixel_bytes"] != extent[0] * extent[1] * 8 for image in images.values()):
                        raise ValueError(f"{arm}: a float drawable capture has the wrong extent")
                    recipe_report[scale] = dict(
                        request=request, frame=presented[0]["frame_extent"],
                        drawable=presented[0]["drawable"], captured_frames=len(images),
                        capture=next(iter(images.values())), game_records=game_records,
                    )
                recipe_report[80] = launch(output / f"{recipe}-80", binary, env, value=80)
                report["recipes"][recipe] = recipe_report

            env.update(THRUM_RECIPE="apple-tile", THRUM_RECONSTRUCTION="metalfx")
            arm = output / "apple-tile-forced-100"
            requests, frames, _, _ = launch(arm, binary, env, value=100)
            presented = [row for row in frames if row["presented"]]
            if not presented or len(requests) != 1 or not requests[0]["forced"]:
                raise ValueError(f"{arm}: expected a forced 100% generation")
            for row in presented:
                require_frame(row, BACKING)
                require_output_policy(row, requests[0])
            report["forced_100"] = require_output_records(arm, presented)
            report["window_resize"] = {
                scale: capture_resize(output / f"window-resize-{scale}", binary, env, scale)
                for scale in (100, 75)
            }
    return dict(report, target="mac-metal")


def mac_output_scale_mode(output):
    with tempfile.TemporaryDirectory(prefix="output-scale-mode-store-", dir=ROOT / "tmp") as scratch:
        binary, env, _ = scale_environment(Path(scratch), counters=True)
        env.update(THRUM_RECIPE="apple-tile", THRUM_RECONSTRUCTION="metalfx")
        with awake_display():
            rows, frames, configures, identity = launch(output / "mode", binary, env, mode=True)
    report = verify_mode(rows, frames, configures, identity)
    require_output_records(output / "mode", [row for row in frames if row["presented"]])
    return dict(report, target="mac-metal", binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest())


def verify_mode(rows, frames, configures, identity):
    if identity["recipe"] != RECIPE["apple-tile"]:
        raise ValueError(f"wrong recipe {identity['recipe']}")
    requests = [row for row in rows if row["event"] == "request"]
    changes = [row for row in rows if row["event"] in ("start", "complete", "limit")]
    if len(requests) != 3 or [(row["event"], row["fullscreen"]) for row in changes] != [
            ("start", True), ("complete", True), ("start", False), ("complete", False)]:
        raise ValueError(f"expected exactly two complete mode changes and three requests: {changes}, {requests}")
    window = requests[0]["backing"]
    native_fullscreen = changes[1]["backing"]
    if native_fullscreen[0] != BACKING[0] or not window[1] < native_fullscreen[1] <= BACKING[1]:
        raise ValueError(f"AppKit did not report the built-in display's fullscreen grid: {native_fullscreen}")
    native_output = [(axis * 75 + 50) // 100 for axis in native_fullscreen]
    for row, backing, output_extent, scale in zip(
            requests, [window, native_fullscreen, window], [window, native_output, window], [100, 75, 100], strict=True):
        if (row["backing"] != backing or row["output"] != output_extent or row["output_scale"] != scale
                or row["monitor_extent"] != BACKING or row["panel_hz"] <= 60
                or row["forced"] or row["registry_proof_input"] != 75
                or row["fullscreen"] != (scale == 75)):
            raise ValueError(f"mixed or incorrectly resolved request: {row}")

    builds = [row["build"] for row in frames if row["build"]]
    switches = [row["switch"] for row in frames if row["switch"] and row["switch"]["taken"]]
    if len(builds) != 2 or len(switches) != 2 or len(configures) != 2:
        raise ValueError(f"expected two builds, configures and takes: {builds}, {configures}, {switches}")
    presented = [row for row in frames if row["presented"]]
    initial = [row for row in presented if row["present_media"] < changes[0]["media"]]
    if not initial:
        raise ValueError("mode proof has no initial windowed frames")
    revision = initial[0]["frame_extent"]["revision"]
    for row in presented:
        generation = row["frame_extent"]["revision"] - revision
        if not 0 <= generation < len(requests):
            raise ValueError(f"unexpected presented generation: {row}")
        require_frame(row, requests[generation]["output"])
        require_output_policy(row, requests[generation])

    intervals = []
    for index, (start, complete) in enumerate(zip(changes[::2], changes[1::2], strict=True)):
        request = requests[index + 1]
        build, switch, configure = builds[index], switches[index], configures[index]
        start_time, end_time = start["media"], complete["media"]
        if end_time - start_time <= 0.100:
            raise ValueError(f"the AppKit animation did not span the debounce: {start}, {complete}")
        if not (end_time <= request["media"] < build["media"]
                <= configure["started"] <= configure["ended"] <= switch["configure_ended"]):
            raise ValueError(f"generation work crossed the AppKit hold: {request}, {build}, {configure}")
        for row in (build, switch):
            if (row["backing"], row["output_scale"], row["extent"]) != (
                    request["backing"], request["output_scale"], request["output"]):
                raise ValueError(f"built or took a mixed request: {row}")
        if (switch["configure_calls"], switch["restores"], switch["taken"]) != (1, 0, 1):
            raise ValueError(f"expected one configure without restore: {switch}")
        if not configure["succeeded"] or configure["path"] != "switch" or configure["extent"] != request["output"]:
            raise ValueError(f"wrong Metal configure: {configure}")
        held = [row for row in presented if start_time <= row["present_media"] < end_time]
        backing = [row for row in rows if row["event"] == "held_backing"
                   and start_time <= row["media"] < end_time]
        old = requests[index]
        attempts = [row for row in frames if row["work"]
                    and start_time <= row["work"][0][1] < end_time]
        occluded = sum(row["status"] == "Occluded" for row in attempts)
        if not backing or not attempts or not (held or occluded):
            raise ValueError("mode animation has no backing observations or old-surface activity")
        if any((row["window"], row["requested_output"]["scale"], row["surface"])
               != (old["backing"], old["output_scale"], old["output"]) for row in attempts):
            raise ValueError("the render owner received a different grid during the AppKit hold")
        for row in held:
            require_frame(row, old["output"])
        next_start = changes[2]["media"] if index == 0 else float("inf")
        taken = [row for row in presented if switch["configure_ended"] <= row["present_media"] < next_start]
        if not taken:
            raise ValueError(f"new generation was not presented after {complete}")
        for row in taken:
            require_frame(row, request["output"])
        old_revision = initial[0]["frame_extent"]["revision"] + index
        if (any(row["frame_extent"]["revision"] != old_revision for row in held)
                or {row["frame_extent"]["revision"] for row in taken} != {old_revision + 1}):
            raise ValueError("a mode change took more than one generation")
        intervals.append(dict(start=start, complete=complete, request=request, build=build,
                              configure=configure, switch=switch, seconds=end_time - start_time,
                              held_backing=backing, old_generation_frames=len(held),
                              held_frame_requests=len(attempts), occluded_acquisitions=occluded,
                              new_generation_frames=len(taken)))
    return dict(recipe=identity["recipe"], initial=requests[0], intervals=intervals)
