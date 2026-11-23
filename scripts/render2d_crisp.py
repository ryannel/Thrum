"""Capture and validate sprite coverage and native backing-size proofs."""

import csv
import hashlib
import json
import os
from pathlib import Path
import subprocess

from apple_presentation import mac_arm, mac_binary
from capture_v1_runtime import console_observation

ROOT = Path(__file__).resolve().parents[1]
RETAINED = ROOT / "crates/thrum-render/tests/support/coverage-mac-metal.json"
RECIPES = ("portable", "apple-tile")
SCALES = (1.0, 1.25, 1.5, 2.0, 2.37, 3.0, 4.0, 6.0)
MODES = ("Pixel", "NearestControl", "BilinearControl")


def revision():
    return subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()


def coverage():
    command = [
        "cargo", "test", "--locked", "--offline", "-p", "thrum-render", "--test", "sprite_crisp",
        "records_scene_linear_coverage_for_both_metal_recipes", "--", "--nocapture",
    ]
    result = subprocess.run(command, cwd=ROOT, env=dict(
        os.environ, THRUM_REQUIRE_GPU="1", THRUM_MEASURE_SPRITE_COVERAGE="1",
    ), stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    if result.returncode:
        raise ValueError(result.stdout)
    rows = [json.loads(line.removeprefix("coverage ")) for line in result.stdout.splitlines()
            if line.startswith("coverage ")]
    record = dict(target="mac-metal", commit=revision(), offsets=256, rows=rows)
    record["bounds"] = coverage_bounds(rows)
    return record


def coverage_bounds(rows):
    index = {(row["recipe"], round(row["scale"], 2), row["sampling"]): row for row in rows}
    if len(index) != len(RECIPES) * len(SCALES) * len(MODES) or len(index) != len(rows):
        raise ValueError("Incomplete Metal coverage sweep")
    limits = {}
    for recipe in RECIPES:
        measured = []
        for scale in SCALES:
            pixel, nearest, bilinear = (index[recipe, scale, mode] for mode in MODES)
            if pixel["pixel_at"] is None or not 0 < pixel["pixel_error"] < 0.01:
                raise ValueError(f"Invalid pixel error for {recipe} at {scale}")
            if pixel["weight_at"] is None or not 0 < pixel["weight_error"] < 0.05:
                raise ValueError(f"Invalid weight error for {recipe} at {scale}")
            if not scale.is_integer() and not pixel["pixel_error"] < min(
                nearest["pixel_error"], bilinear["pixel_error"]
            ):
                raise ValueError(f"A control beat coverage on {recipe} at {scale}")
            measured.append(pixel["weight_error"])
        limits[recipe] = max(measured)
    return limits


def prove_coverage():
    retained = json.loads(RETAINED.read_text())
    if retained.get("target") != "mac-metal" or retained.get("offsets") != 256:
        raise ValueError("Invalid retained Metal coverage record")
    if coverage_bounds(retained["rows"]) != retained["bounds"]:
        raise ValueError("A retained bound differs from the largest measured weight error")
    return coverage()


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


def card_capture(binary, env, output, points):
    folder = output / f"{points[0]}x{points[1]}"
    folder.mkdir()
    launch = dict(env, THRUM_V1_FRAMES="12", THRUM_MAC_EXTENT_PROOF=str(folder),
                  THRUM_MAC_EXTENT_WINDOW=f"{points[0]}x{points[1]}")
    with (folder / "game.log").open("w") as log:
        subprocess.run(["/usr/bin/caffeinate", "-d", "-i", "-u", str(binary)],
                       env=launch, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT,
                       timeout=90, check=True)
    window = json.loads((folder / "window.json").read_text())
    submitted = json.loads((folder / "submitted.json").read_text())
    backing = window["backing"]
    if window["points"] != points or backing != [point * window["scale"] for point in points]:
        raise ValueError(f"Window backing does not match requested points: {window}")
    if window["scale"] != 2 or window["monitor_backing"] != [3456, 2234]:
        raise ValueError(f"Expected the built-in Retina display: {window}")
    for field in ("capture_extent", "render_extent", "source", "destination", "drawable"):
        if submitted[field] != backing:
            raise ValueError(f"{field} does not match the window backing: {submitted}")
    if submitted["viewport"] != [0, 0, *backing] or submitted["attachment_extents"] != [[0, 0], backing, [0, 0], backing]:
        raise ValueError(f"Sprite/present extents did not match the window: {submitted}")
    return dict(window=window, submission=submitted, rgba_sha256=verify_card(
        (folder / "submitted.rgba").read_bytes(), (folder / "card-source.rgba").read_bytes(), backing
    ))


def transition_capture(binary, env, folder):
    frames, calls, _ = mac_arm(folder, binary, env, "portable", "sweep", frames=90, capture=True)
    arm = folder / "portable-sweep"
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
    with (arm / "extents.tsv").open() as metadata, (arm / "frames.rgba").open("rb") as spool:
        for row in csv.DictReader(metadata, delimiter="\t"):
            size = [int(row["width"]), int(row["height"])]
            length = size[0] * size[1] * 4
            if int(row["submission_id"]) == changed["submission"]:
                if changed["surface"] != size:
                    raise ValueError(f"Captured resized frame at the wrong extent: {row}")
                sha = verify_card(spool.read(length), (arm / "card-source.rgba").read_bytes(), size)
                return {"first_resized_frame": changed, "rgba_sha256": sha}
            spool.seek(length, os.SEEK_CUR)
    raise ValueError("First resized submission has no engine-capture record")


def window(output, binary=None):
    if binary is None:
        binary, env = mac_binary()
    else:
        binary = binary.resolve()
        env = {key: value for key, value in os.environ.items() if not key.startswith("THRUM_")}
    env.update(THRUM_SCENE="sprites", THRUM_SPRITE_SCALE="2", THRUM_RECIPE="portable")
    console_before, _ = console_observation()
    windows = [card_capture(binary, env, output, points) for points in ([640, 360], [1280, 720], [1707, 933])]
    resize = transition_capture(binary, env, output)
    console_after, _ = console_observation()
    with binary.open("rb") as file:
        identity = hashlib.file_digest(file, "sha256").hexdigest()
    return dict(target="mac-metal", commit=revision(), console_before=console_before,
                console_after=console_after, binary_sha256=identity, windows=windows, resize=resize)
