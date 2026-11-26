"""Qualify fullscreen pacing and retain complete T2 cue windows on Mac Metal."""

import csv
from hashlib import sha256
import json
import shutil
import subprocess

from apple_paired_proof import window_arm
from apple_presentation import ROOT, scale_environment
from capture_v1_runtime import preview

CUE_LIMITS = (12, 72, 2560)
# Required rendered coverage inside the wider tap windows. These brackets
# include the start of the approach, the held and panned stars, the prop's
# passage past the rock, and settling time on both sides of the solar sweep.
CUE_BRACKETS = {
    "terrain": (180, 270),
    "stars": (182, 268),
    "moving-prop": (182, 254),
    "scrub": (199, 260),
    "scrub-reference": (199, 260),
}


def pacing_passed(take):
    return (take.get("requested_rate") == {"fixed": True, "forced": False, "preferred_hz": 60}
            and take["bounds_match"]
            and take["governor_intervals_ns"] == [16666667]
            and take["interval_outcomes"] == {"served": take["route_intervals"]})


def cue_routes(original):
    origin = original["origin_tick"]
    pose = original["keys"][0]
    pan = dict(pose, yaw_degrees=12.0)
    held = dict(original, cuts=[], checkpoints=[],
                keys=[dict(pose, tick=0), dict(pose, tick=126)])
    stars = dict(held, start={"solar_hours": 0.3, "hold_solar": True},
                 keys=[dict(pose, tick=0), dict(pose, tick=36),
                       dict(pan, tick=76), dict(pan, tick=126)])
    motion = dict(held, overlay="motion")
    scrub = dict(held, start={"solar_hours": 17.45})
    return [
        ("terrain", original, [origin, origin + 95], "metalfx", None),
        ("stars", stars, [origin, origin + 122], "metalfx", None),
        ("moving-prop", motion, [origin, origin + 84], "metalfx", None),
        ("scrub", scrub, [origin + 10, origin + 90], "metalfx", [[origin + 24, origin + 68], [17.45, 18.10]]),
        ("scrub-reference", scrub, [origin + 10, origin + 90], "off", [[origin + 24, origin + 68], [17.45, 18.10]]),
    ]


def read_tsv(path):
    with path.open() as source:
        return list(csv.DictReader(source, delimiter="\t"))


def retain_cue(arm, extent, ticks):
    images = read_tsv(arm / "images.tsv")
    ids = [int(row["submission_id"]) for row in images]
    submitted = [int(row["submission_id"]) for row in read_tsv(arm / "surface.tsv")
                 if row["presented"] == row["tap_enabled"] == "true"]
    extents = read_tsv(arm / "extents.tsv")
    digest = (arm / "hashes.txt").read_text().splitlines()
    if (not ids or submitted != ids or ids != list(range(ids[0], ids[-1] + 1))
            or len(ids) != len(digest)
            or [(int(row["submission_id"]), int(row["width"]), int(row["height"])) for row in extents]
            != [(submission, *extent) for submission in ids]):
        raise ValueError(f"{arm}: cue lacks a complete native-size submission/image join")
    raw = arm / "frames.rgba"
    frame_bytes = extent[0] * extent[1] * 4
    if raw.stat().st_size != frame_bytes * len(ids):
        raise ValueError(f"{arm}: cue has an incomplete raw spool")
    clock_origin = int(images[0]["submit_origin_ns"])
    records = [dict(submission_id=int(row["submission_id"]),
                    submit_ns=int(row["submit_origin_ns"]) - clock_origin,
                    tick=int(row["tick"]), alpha=float(row["alpha"]))
               for row in images]
    bracket = CUE_BRACKETS[arm.name]
    if (records[0]["tick"] > bracket[0] or records[-1]["tick"] < bracket[1]
            or any(not ticks[0] <= row["tick"] <= ticks[1] for row in records)
            or any(a["submit_ns"] >= b["submit_ns"] for a, b in zip(records, records[1:]))):
        raise ValueError(f"{arm}: captured time does not bracket the declared cue")
    raw_hash = sha256()
    with raw.open("rb") as source:
        for expected in digest:
            data = source.read(frame_bytes)
            if sha256(data).hexdigest() != expected:
                raise ValueError(f"{arm}: capture hash mismatch")
            raw_hash.update(data)
    png = arm / "png"
    png.mkdir()
    subprocess.run(["ffmpeg", "-v", "error", "-f", "rawvideo", "-pixel_format", "rgba",
                    "-video_size", f"{extent[0]}x{extent[1]}", "-i", str(raw), "-threads", "2",
                    "-start_number", str(ids[0]), str(png / "%06d.png")], check=True)
    receipt = dict(timebase="realtime", stage="submitted SDR stage on the live EDR window",
                   extent=extent, requested_ticks=ticks, required_bracket=bracket, frames=records,
                   output_sha256=raw_hash.hexdigest())
    preview(arm, receipt, {"interval_end_ns": records[-1]["submit_ns"] + 16666667}, records)
    (arm / "timeline.ffconcat").unlink()
    shutil.rmtree(png)
    archive = raw.with_suffix(".rgba.zst")
    subprocess.run(["zstd", "-q", "-T2", "-3", str(raw), "-o", str(archive)], check=True)
    archive_hash = sha256()
    with subprocess.Popen(["zstd", "-qdc", str(archive)], stdout=subprocess.PIPE) as decoded:
        for block in iter(lambda: decoded.stdout.read(4 * 1024 * 1024), b""):
            archive_hash.update(block)
        if decoded.wait() or archive_hash.hexdigest() != raw_hash.hexdigest():
            raise ValueError(f"{arm}: source archive failed its round trip")
    raw.unlink()
    pressure = read_tsv(arm / "capture-pressure.tsv")
    receipt["peak_occupancy"] = {
        boundary: {key: max(int(row[key]) for row in pressure if row["boundary"] == boundary)
                   for key in ("staging", "cpu")}
        for boundary in ("reserve", "collect")
    }
    (arm / "cue.json").write_text(json.dumps(receipt, indent=2) + "\n")
    return receipt


def capture_cue(output, binary, env, extent, cue):
    name, route, ticks, reconstruction, solar = cue
    route_path = output / f"{name}-route.json"
    route_path.write_text(json.dumps(route) + "\n")
    launch_env = dict(env, THRUM_ROUTE_OVERLAY=route["overlay"])
    if solar is not None:
        launch_env["THRUM_MAC_SOLAR_SWEEP"] = json.dumps(solar)
    if shutil.disk_usage(output).free < 21 * 1024**3:
        raise ValueError("fullscreen cue requires free space for its raw spool and packaging")
    take = window_arm(output, binary, launch_env, route_path, name, "apple-tile",
                      reconstruction, "2", replay=False, fullscreen=True, capture=True,
                      cue=(ticks, *CUE_LIMITS))
    take["capture"] = retain_cue(output / name, extent, ticks)
    return take


def mac_fullscreen_motion(output):
    binary, env, route = scale_environment(output, counters=True)
    displays = json.loads(subprocess.check_output(["system_profiler", "SPDisplaysDataType", "-json"]))
    screen, = [screen for gpu in displays["SPDisplaysDataType"]
               for screen in gpu.get("spdisplays_ndrvs", [])
               if screen.get("spdisplays_connection_type") == "spdisplays_internal"]
    extent = list(map(int, screen["_spdisplays_pixels"].split(" x ")))
    route["render_extent"] = extent
    route_path = output / "fullscreen-route.json"
    route_path.write_text(json.dumps(route) + "\n")
    takes = []
    for number in range(1, 13):
        take = window_arm(output, binary, env, route_path, f"capture-off-{number}", "apple-tile",
                          "metalfx", "2", replay=False, fullscreen=True)
        take["steady"] = pacing_passed(take)
        takes.append(take)
        (output / "capture-off-takes.json").write_text(json.dumps(takes, indent=2) + "\n")
        if sum(row["steady"] for row in takes) == 2:
            break
    else:
        raise ValueError("the fullscreen route did not qualify two unforced, steady 60 Hz takes")
    forced = window_arm(output, binary, env, route_path, "forced-120", "apple-tile",
                        "metalfx", "2", replay=False, fullscreen=True, frame_rate=120)
    captures = [capture_cue(output, binary, env, extent, cue) for cue in cue_routes(route)]
    return dict(target="mac-metal", timebase="realtime", extent=extent,
                revision=subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                                 text=True).strip(),
                binary_sha256=sha256(binary.read_bytes()).hexdigest(),
                observer_only=takes, forced_120=forced, cues=captures)
