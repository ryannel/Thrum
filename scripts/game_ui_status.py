"""Submitted-window and release-cost checks for the V1 status display."""

import json
import os
from pathlib import Path
import subprocess

from capture_v1_runtime import console_observation
from t2_writer_probe import package

SOURCE = "tests/fixtures/game-ui/font-source"
EXAMPLE = "interface_status"


def prepare_store(root, scratch, profile):
    target = root / "target" / profile
    args = ["cargo", "build", "--locked", "--offline"]
    if profile == "release":
        args.append("--release")
    subprocess.run([*args, "-p", "thrum-cook", "--bin", "thrum-cook"], cwd=root, check=True)
    store = scratch / "store"
    subprocess.run([target / "thrum-cook", "cook", root / SOURCE, store], cwd=root, check=True)
    return store


def compare_window(sample, window, directory, stem, oracle, store):
    width, height = sample["capture_extent"]
    scale = sample["paint_scale"]
    if scale != 2 or sample["capture_extent"] != sample["drawable"]:
        raise ValueError(f"unexpected submitted extent or scale: {sample}")
    insets = [inset * window["scale"] / scale for inset in window["insets"]]
    data = subprocess.check_output(
        [oracle, store],
        input=(json.dumps([[width, height], scale, insets, sample["tick"]]) + "\n").encode())
    header, _newline, packed = data.partition(b"\n")
    region, size, mask_size = json.loads(header)
    expected, mask = packed[:size], packed[size:]
    [[left, top], [right, bottom]] = region
    actual = (directory / f"{stem}.rgba").read_bytes()
    if len(actual) != width * height * 4 or len(mask) != mask_size:
        raise ValueError(f"{stem}: incomplete pixel record")
    for y in range(top, bottom):
        for x in range(left, right):
            ref = (y - top) * (right - left) + x - left
            at = (y * width + x) * 4
            for ch in range(4):
                a, b = actual[at + ch], expected[ref * 4 + ch]
                if abs(a - b) > (mask[ref] if ch < 3 else 0):
                    raise ValueError(f"{stem} pixel {x},{y} channel {ch}: {a} != {b}")
    image = directory / f"{stem}.png"
    return {**sample, "status_pixels": mask_size, "region": region,
            "rgba_sha256": package(actual, image, width, height), "png": str(image.name)}


def windows(root, destination, scratch, revision, environment):
    subprocess.run(["cargo", "build", "--locked", "--offline", "-p", "v1-game",
                    "--bin", "v1-game", "--example", EXAMPLE], cwd=root, check=True)
    store = prepare_store(root, scratch, "debug")
    target = root / "target/debug"
    game = target / "v1-game"
    oracle = target / "examples" / EXAMPLE
    arms = []
    for points, resized in [([640, 360], [896, 504]), ([1280, 720], None), ([1707, 933], None)]:
        label = "x".join(map(str, points))
        directory = destination / label
        directory.mkdir(exist_ok=True)
        for stem in ("submitted", "resized", "window"):
            for ext in ("json", "rgba"):
                (directory / f"{stem}.{ext}").unlink(missing_ok=True)
        env = dict(environment, THRUM_SCENE="interface", THRUM_INTERFACE_TEXT="status",
                   THRUM_FONT_STORE=str(store), THRUM_MAC_EXTENT_PROOF=str(directory),
                   THRUM_MAC_EXTENT_WINDOW=label, THRUM_V1_FRAMES="100")
        if resized:
            env["THRUM_MAC_EXTENT_RESIZE"] = "x".join(map(str, resized))
        with (directory / "run.log").open("wb") as log:
            subprocess.run(["/usr/bin/caffeinate", "-d", "-i", "-u", game],
                           cwd=root, env=env, check=True, timeout=40, stdout=log, stderr=log)
        window = json.loads((directory / "window.json").read_text())
        if window["points"] != points or window["scale"] != 2 or len(window["insets"]) != 4:
            raise ValueError(f"unexpected AppKit window record: {window}")
        if window["monitor_backing"] != [3456, 2234]:
            raise ValueError(f"status window is not on the built-in display: {window}")
        samples = []
        for stem, expected in [("submitted", points)] + ([("resized", resized)] if resized else []):
            sample = json.loads((directory / f"{stem}.json").read_text())
            if sample["capture_extent"] != [2 * side for side in expected]:
                raise ValueError(f"unexpected {stem} window grid: {sample}")
            samples.append(compare_window(sample, window, directory, stem, oracle, store))
            (directory / f"{stem}.rgba").unlink()
        arms.append({"window": window, "directory": label, "samples": samples})
    return {"target": "mac-metal", "commit": revision, "console": console_observation()[0],
            "mode": "submitted engine still", "arms": arms}


def validate_costs(record, target):
    counts = {"100": 0, "500": 0, "2000": 0}
    for size, layout_ms, paint_ms in record["layout"]:
        counts[str(int(size))] += 1
        if not all(isinstance(value, (int, float)) and value >= 0 for value in [layout_ms, paint_ms]):
            raise ValueError("missing CPU layout or paint duration")
    if list(counts.values()) != [32] * 3:
        raise ValueError("incomplete size arms")
    samples = record["samples"]
    if len(samples) != 32:
        raise ValueError(f"{target}: missing status frame samples")
    for frame in samples:
        for field in ("layouts", "rows", "lists", "upload_bytes", "cpu_ns", "gpu_ns"):
            value = frame[field]
            if not isinstance(value, (float, int)) or not 0 <= value < float("inf"):
                raise ValueError(f"{target}: missing status frame measurement {field}")
    if not any(frame["rows"] > 0 and frame["upload_bytes"] > 0 for frame in samples):
        raise ValueError(f"{target}: status updates never reached the measured frame")


def costs(root, destination, scratch, revision, environment, target, load, digest):
    run_files = []
    if target == "mac-metal":
        subprocess.run(["cargo", "build", "--release", "--locked", "--offline", "-p", "v1-game",
                        "--example", EXAMPLE], cwd=root, check=True)
        store = prepare_store(root, scratch, "release")
        executable = root / "target/release/examples" / EXAMPLE
    for index in (1, 2):
        before = load()
        if target == "mac-metal":
            record = json.loads(subprocess.check_output([executable, store, "costs"], cwd=root, env=environment))
            build = {"binary_sha256": digest(executable)}
        else:
            arm = scratch / f"chrome-{index}"
            subprocess.run(["node", "scripts/check_v1_browser.mjs", "--case", "interface-status-cost",
                            "--output", arm], cwd=root, check=True, env=environment)
            record = json.loads((arm / "interface-status-cost.json").read_text())
            build = record["build"]
        record.update(commit=revision, profile="release", build=build,
                      load_before=before, load_after=load(), run=index)
        validate_costs(record, target)
        name = f"run-{index}.json"
        (destination / name).write_text(json.dumps(record, indent=2) + "\n")
        run_files.append(name)
    return {"target": target, "commit": revision, "runs": run_files}
