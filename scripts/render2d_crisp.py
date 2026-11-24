"""Capture and validate native sprite backing-size proofs."""

import csv
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

from apple_presentation import annotate_display_ticks, mac_binary, read_rows, run_mac_arm
from capture_v1_runtime import console_observation

ROOT = Path(__file__).resolve().parents[1]


def revision():
    return subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()


def verify_card(raw, source, size, scale=2):
    width, height = size
    if len(raw) != width * height * 4 or len(source) != 256 * 128 * 4:
        raise ValueError("Submitted pixels and the authored card must be packed RGBA8")
    left, top = width // 2 - 128 * scale, height // 2 - 64 * scale
    clear = bytes((0, 0, 0, 255))
    rows = []
    for y in range(128):
        texels = source[y * 1024:(y + 1) * 1024]
        expanded = b"".join(texels[i:i + 4] * scale for i in range(0, 1024, 4))
        rows.append(clear * left + expanded + clear * (width - left - 256 * scale))
    blank = clear * width
    for y in range(height):
        expected = rows[(y - top) // scale] if top <= y < top + 128 * scale else blank
        if raw[y * width * 4:(y + 1) * width * 4] != expected:
            raise ValueError(f"Sprite card differs from authored texels at submitted row {y} for {width}×{height}")
    return hashlib.sha256(raw).hexdigest()


def require_occluded(result, folder, frames):
    log = (folder / "game.log").read_text()
    match = re.search(
        rf"^v1-game: {frames} presented frames unavailable: presented=0, tick=\d+, "
        r"display_link_callbacks=\d+, render_callbacks=\d+, unavailable=(\d+), "
        r"occluded=([1-9]\d*), surface_faults=0, dropped_views=\d+, "
        r"dropped_stale_views=\d+, dropped_surface=\d+$", log, re.MULTILINE,
    )
    if result.returncode != 1 or match is None:
        raise ValueError(f"Window run did not report an occluded surface in {folder}: {log[-4000:]}")
    unavailable, occluded = map(int, match.groups())
    return {"status": "Occluded", "presented": 0, "unavailable": unavailable, "occluded": occluded,
            "drawable": None, "host_report": match[0]}


def verify_grid(frame, backing):
    for field in ("source", "destination"):
        if frame[field] != backing:
            raise ValueError(f"{field} does not match the window backing: {frame}")
    if frame["viewport"] != [0, 0, *backing] or frame["attachment_extents"] != [[0, 0], backing, [0, 0], backing]:
        raise ValueError(f"Sprite/present extents did not match the window: {frame}")


def card_capture(binary, env, output, points, *, allow_occluded):
    folder = output / f"{points[0]}x{points[1]}"
    folder.mkdir()
    launch = dict(env, THRUM_V1_FRAMES="12", THRUM_MAC_EXTENT_PROOF=str(folder),
                  THRUM_MAC_EXTENT_WINDOW=f"{points[0]}x{points[1]}")
    with (folder / "game.log").open("w") as log:
        result = subprocess.run(["/usr/bin/caffeinate", "-d", "-i", "-u", str(binary)],
                                env=launch, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT,
                                timeout=90, check=False)
    window = json.loads((folder / "window.json").read_text())
    backing = window["backing"]
    if window["fullscreen"] or window["points"] != points or backing != [point * window["scale"] for point in points]:
        raise ValueError(f"Window backing does not match requested points: {window}")
    if window["scale"] != 2 or window["monitor_backing"] != [3456, 2234]:
        raise ValueError(f"Expected the built-in Retina display: {window}")
    if result.returncode and allow_occluded:
        observation = require_occluded(result, folder, 12)
        if observation["occluded"] != observation["unavailable"] or any(folder.glob("submitted.*")):
            raise ValueError(f"Expected only occlusion with no submitted image in {folder}: {observation}")
        return dict(result="geometry_only", window=window, surface=observation)
    result.check_returncode()
    submitted = json.loads((folder / "submitted.json").read_text())
    verify_grid(submitted, backing)
    for field in ("capture_extent", "render_extent", "drawable"):
        if submitted[field] != backing:
            raise ValueError(f"{field} does not match the window backing: {submitted}")
    return dict(result="presented", window=window, submission=submitted, rgba_sha256=verify_card(
        (folder / "submitted.rgba").read_bytes(), (folder / "card-source.rgba").read_bytes(), backing
    ))


def occluded_transition(result, arm, frames):
    observation = require_occluded(result, arm, 90)
    if any(row["presented"] or row["drawable"] is not None or row["transition_error"] for row in frames):
        raise ValueError(f"Resize run acquired a drawable, presented or reported an error in {arm}")
    initial = next((row for row in frames if row["status"] == "Occluded" and row["window"] == [1920, 1080]), None)
    resized = next((row for row in frames if row["status"] == "Occluded" and row["window"] == [1552, 896]), None)
    if initial is None or resized is None or initial["sequence"] >= resized["sequence"]:
        raise ValueError(f"Missing occluded AppKit observations before and after resize in {arm}")
    if initial["scale"] != 2 or resized["scale"] not in (2, 1.5):
        raise ValueError(f"Expected Retina backing during resize in {arm}")
    return dict(result="geometry_only", surface=observation, initial=initial, resized=resized)


def transition_capture(binary, env, folder, *, allow_occluded):
    arm, result = run_mac_arm(folder, binary, env, "portable", "sweep", frames=90, capture=True)
    frames = read_rows(arm / "frames.jsonl")
    if result.returncode and allow_occluded:
        return occluded_transition(result, arm, frames)
    result.check_returncode()
    calls = read_rows(arm / "configure.jsonl")
    annotate_display_ticks(arm, frames, calls, expected_conflict=False)
    presented = [row for row in frames if row["presented"]]
    stale = next((row for row in presented if row["surface"] != row["window"]), None)
    if stale:
        raise ValueError(f"Presented a sprite scene on an obsolete backing grid: {stale}")
    changed = next((
        row for previous, row in zip(presented, presented[1:])
        if row["surface"] != previous["surface"]
    ), None)
    if changed is None:
        observations = {(tuple(row["window"]), tuple(row["surface"]), row["status"], row["transition_error"])
                        for row in frames}
        raise ValueError(f"No resized frame. Window and surface observations: {observations}; configure: {calls}")
    if (changed["window"] != changed["surface"] or changed["drawable"] != [changed["surface"]] * 2
            or not changed["switch"] or changed["switch"]["extent"] != changed["surface"]):
        raise ValueError(f"First resized submission used an old extent: {changed}")
    frame = changed["frame_extent"]
    if frame is None or frame["submission"] != changed["submission"]:
        raise ValueError(f"Missing matching viewport and targets for the first resized submission: {changed}")
    verify_grid(frame, changed["surface"])
    with (arm / "extents.tsv").open() as metadata, (arm / "frames.rgba").open("rb") as spool:
        for row in csv.DictReader(metadata, delimiter="\t"):
            size = [int(row["width"]), int(row["height"])]
            length = size[0] * size[1] * 4
            if int(row["submission_id"]) == changed["submission"]:
                if changed["surface"] != size:
                    raise ValueError(f"Captured resized frame at the wrong extent: {row}")
                sha = verify_card(spool.read(length), (arm / "card-source.rgba").read_bytes(), size)
                return {"result": "presented", "first_resized_frame": changed, "rgba_sha256": sha}
            spool.seek(length, os.SEEK_CUR)
    raise ValueError("First resized submission has no engine-capture record")


def window(output, binary=None, *, allow_occluded=False):
    if binary is None:
        binary, env = mac_binary()
    else:
        binary = binary.resolve()
        env = {key: value for key, value in os.environ.items() if not key.startswith("THRUM_")}
    env.update(THRUM_SCENE="sprites", THRUM_SPRITE_SCALE="2", THRUM_RECIPE="portable")
    console_before, _ = console_observation()
    windows = [card_capture(binary, env, output, points, allow_occluded=allow_occluded)
               for points in ([640, 360], [1280, 720], [1707, 933])]
    resize = transition_capture(binary, env, output, allow_occluded=allow_occluded)
    console_after, _ = console_observation()
    with binary.open("rb") as file:
        identity = hashlib.file_digest(file, "sha256").hexdigest()
    result = "presented" if all(row["result"] == "presented" for row in [*windows, resize]) else "geometry_only"
    return dict(target="mac-metal", commit=revision(), result=result, console_before=console_before,
                console_after=console_after, binary_sha256=identity, windows=windows, resize=resize)
