#!/usr/bin/env python3
"""Collect and verify the sprite scene's release cost record."""

import argparse
import gzip
import hashlib
import json
import math
import os
from pathlib import Path
import re
import statistics
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE = ROOT / "docs/evidence/RENDER-2D/M1"
STARTING = "4392b652fb63994e6ca9090e9b5d20602777337d"
TARGETS = ("mac-metal", "chrome-webgpu")
BUILDS = {
    "mac-metal": "cargo build --locked --offline --release -p v1-game --bin v1-game",
    "chrome-webgpu": "sh examples/v1-game/web/build.sh (locked offline release)",
}
VALLEY_BUILD = "cargo build --locked --offline --release -p v1-game --bench valley_culling"
EXTENTS = ((1920, 1080), (3456, 2234))
SPRITES = "sprites"
TONE = "display-referred tone encode"
PRESENT = "V1 surface present"
CPU = ("tick_cpu_ns", "extract_cpu_ns", "upload_cpu_ns", "encode_cpu_ns", "render_cpu_ns", "wall_ns")
PASSES = (SPRITES, TONE, PRESENT)


def git(*args):
    return subprocess.check_output(["git", *args], cwd=ROOT, text=True).strip()


def digest(path):
    with path.open("rb") as data:
        return hashlib.file_digest(data, "sha256").hexdigest()


def load():
    return {"averages": list(os.getloadavg()), "logical_cpus": os.cpu_count(),
            "processes": subprocess.check_output(["ps", "-A", "-r", "-o", "pcpu=,comm="],
                                                 text=True).splitlines()[:12]}


def read(path):
    return json.loads(path.read_text())


def run(command, **options):
    subprocess.run(command, check=True, cwd=ROOT, timeout=360, **options)


def storage(text, extent, present_bytes):
    pixels = math.prod(extent)
    for pattern, size in (("sprite colour Rgba16Float bytes=", pixels * 8),
                          ("SDR output Rgba8Unorm bytes=", pixels * 4)):
        if list(map(int, re.findall(rf"^{pattern}(\d+)$", text, re.M))) != [size, size]:
            raise ValueError(f"missing two prepared sprite targets of {size} bytes")
    return {"slots": 2, "scene_bytes_per_slot": pixels * 8, "sdr_bytes_per_slot": pixels * 4,
            "present_bytes": present_bytes}


def summarize(rows):
    if len(rows) < 30:
        raise ValueError("fewer than 30 completed samples")
    if len({row["submission"] for row in rows}) != len(rows):
        raise ValueError("repeated sample submission")
    columns = (*CPU, "sprite_gpu_ns", "tone_gpu_ns", "present_gpu_ns", "whole_gpu_ns")
    for row in rows:
        if any(type(row.get(column)) not in (float, int) or not math.isfinite(row[column])
               or row[column] < 0 for column in columns):
            raise ValueError(f"incomplete CPU/GPU sample {row}")
    return {column: {
        "p50": statistics.median(row[column] for row in rows),
        "p95": sorted(row[column] for row in rows)[round((len(rows) - 1) * .95)],
    } for column in columns}


def sample(row, names, present, submission):
    if (not isinstance(names, dict) or
            set(name for name, value in names.items() if value is not None) != set(PASSES)):
        raise ValueError(f"unexpected timed sprite scene passes {names}")
    if names[PRESENT] != present or present is None:
        raise ValueError("present timings are missing or disagree")
    return {**{key: row[key] for key in CPU}, "submission": submission,
            "sprite_gpu_ns": names[SPRITES], "tone_gpu_ns": names[TONE],
            "present_gpu_ns": present, "whole_gpu_ns": sum(names[key] for key in PASSES)}


def metal_take(folder, binary, expected_sha, commit):
    extent_rows = []
    for extent in EXTENTS:
        output = folder / ("x".join(map(str, extent)))
        run([sys.executable, "scripts/capture_v1_runtime.py", "--binary", str(binary),
             "--case", "off", "--mode", "realtime", "--recipe", "portable",
             "--extent", *map(str, extent), "--output", str(output)],
            env={**os.environ, "THRUM_SCENE": "sprites", "THRUM_SPRITE_SCALE": "2"})
        manifest, runtime = read(output / "manifest.json"), read(output / "runtime.json")
        if (manifest["source_revision"] != commit or manifest["binary_sha256"] != expected_sha or
                not manifest["complete"] or runtime["scene_kind"] != "TwoD" or
                runtime["images"] or runtime["backend"] != "Metal"):
            raise ValueError("native timing provenance or capture-off scene is wrong")
        # Use the middle interval, excluding warmup and the final drain.
        completed = [row for row in runtime["gpu"] if row.get("gpu_present_ns") is not None]
        samples = [sample(row, row["gpu_pass_ns"], row["gpu_present_ns"], row["submission_id"])
                   for row in completed[20:110]]
        extent_rows.append({"extent": extent, "load": manifest["system_load"],
                            "storage": storage(runtime["storage"], extent, runtime["present_target_bytes"]),
                            "adapter": runtime["adapter"], "recipe": runtime["recipe"], "samples": samples})
    return extent_rows


def chrome_take(folder, web):
    before = load()
    run(["node", "scripts/check_v1_browser.mjs", "--case", "sprites-2d-cost",
         "--web-root", str(web), "--output", str(folder)])
    after = load()
    record = read(folder / "sprites-2d-cost.json")
    gpu = read(folder / "browser-gpu.json")
    return [{
        "extent": row["extent"], "load": {"before": before, "after": after},
        "storage": storage(row["measurement"]["storage"], row["extent"], math.prod(row["extent"]) * 4),
        "adapter": gpu["gpu"]["gpu"]["devices"], "chrome": gpu["version"]["product"],
        "wasm": row["wasm"], "samples": [
            sample(frame, frame["pass_ns"], frame["present_ns"], frame["submission"])
            for frame in row["measurement"]["samples"]
        ],
        "limits": {name: row["measurement"][name]
                   for name in ("sampled_textures", "array_layers", "storage_buffers")},
    } for row in record["rows"]]


def valley_rows(output):
    records = [json.loads(line) for line in (output / "rows.jsonl").read_text().splitlines()]
    header, *samples, footer = records
    if (header["adapter"] != "Apple M3 Max" or header["extent"] != [1920, 1080]
            or not header["timing_only"] or not footer["timed"]):
        raise ValueError(f"invalid valley route measurement at {output}")
    return header, {f'{row["tick"]}/{row["mode"]}': row["whole_gpu_ns"] for row in samples}


def valley_pair(args, folder):
    before = args.valley_before.resolve()
    binary = args.valley_binary.resolve()
    route = args.valley_route.resolve()
    store = args.valley_store.resolve()
    after = folder / "valley-after"
    before_load = load()
    run([str(binary)], env={
        **os.environ, "THRUM_SCENE": "valley", "THRUM_ASSET_STORE": str(store),
        "THRUM_CAPTURE_ROUTE": str(route), "THRUM_CULLING_OUTPUT": str(after),
        "THRUM_RECIPE": "portable", "THRUM_CULLING_TIMING": "on", "THRUM_VALLEY_MESHES": "off",
    })
    return {
        "build": VALLEY_BUILD,
        "route": "examples/v1-game/routes/moving-v1.json at 1920x1080",
        "route_sha256": digest(route), "cooked_head_sha256": digest(store / "HEAD"),
        "before_commit": STARTING, "before_binary_sha256": digest(args.valley_base_binary),
        "after_binary_sha256": digest(binary),
        "before_load": read(args.valley_before_load), "after_load": [before_load, load()],
        "before": valley_rows(before), "after": valley_rows(after),
    }


def load_averages(values):
    return isinstance(values, list) and len(values) == 3 and all(
        type(value) in (float, int) and math.isfinite(value) and value >= 0 for value in values
    )


def text_rows(rows):
    return isinstance(rows, list) and bool(rows) and all(
        isinstance(row, str) and row.strip() for row in rows
    )


def load_snapshot(snapshot):
    return (isinstance(snapshot, dict) and load_averages(snapshot.get("averages"))
            and type(snapshot.get("logical_cpus")) is int and snapshot["logical_cpus"] > 0
            and text_rows(snapshot.get("processes")))


def validate_row_record(row, target):
    extent = row["extent"]
    pixels = math.prod(extent)
    sizes = {"slots": 2, "scene_bytes_per_slot": pixels * 8, "sdr_bytes_per_slot": pixels * 4,
             "present_bytes": pixels * 4}
    storage_record = row.get("storage")
    if not isinstance(storage_record, dict) or any(
            type(storage_record.get(name)) is not int or storage_record[name] != size
            for name, size in sizes.items()):
        raise ValueError(f"{target} {extent}: missing or incorrect target storage bytes")

    observations = row.get("load")
    if isinstance(observations, dict):
        if target == "chrome-webgpu":
            if all(load_snapshot(observations.get(edge)) for edge in ("before", "after")):
                return
        elif (all(load_averages(observations.get(f"{edge}_load_1_5_15")) for edge in ("before", "after"))
              and text_rows(observations.get("background_process_snapshot"))):
            return
    raise ValueError(f"{target} {extent}: missing before or after machine load observation")


def sha256_field(value):
    return isinstance(value, str) and bool(re.fullmatch("[a-f0-9]{64}", value))


def validate(record, target):
    toolchain = record.get("toolchain")
    if (record["target"] != target or record["profile"] != "release" or
            not re.fullmatch("[a-f0-9]{40}", record["commit"]) or
            not sha256_field(record.get("binary_sha256")) or
            record.get("build") != BUILDS[target] or not isinstance(toolchain, str) or
            not re.match(r"^rustc \d+\.\d+\.\d+\b", toolchain) or
            len(record["runs"]) != 2):
        raise ValueError("missing target, release build identity or separate takes")
    takes = record["take_started_unix_ns"]
    if len(takes) != 2 or not all(type(stamp) is int and stamp > 0 for stamp in takes) or takes[1] <= takes[0]:
        raise ValueError("the two takes need distinct start times")
    summaries = []
    for take in record["runs"]:
        if [tuple(row["extent"]) for row in take] != list(EXTENTS):
            raise ValueError("missing 1080p or full backing size")
        summary = []
        for row in take:
            if not row.get("adapter"):
                raise ValueError("missing adapter record")
            validate_row_record(row, target)
            if target == "chrome-webgpu":
                if row["wasm"] != record["binary_sha256"] or not row["chrome"]:
                    raise ValueError("Chrome loaded a different build")
                if set(row["limits"]) != {"sampled_textures", "array_layers", "storage_buffers"} or not all(
                        type(value) is int and value > 0 for value in row["limits"].values()):
                    raise ValueError("missing Chrome capability limits")
            summary.append(summarize(row["samples"]))
        summaries.append(summary)
    trips = []
    for extent, current, prior in zip(EXTENTS, summaries[1], summaries[0], strict=True):
        for key in ("render_cpu_ns", "sprite_gpu_ns", "whole_gpu_ns"):
            new, old = current[key]["p50"], prior[key]["p50"]
            if new > 1.25 * old:
                trips.append(f"{extent} second take {key}: {new:.0f} vs {old:.0f} ns under recorded load")
    pixels_ratio = math.prod(EXTENTS[1]) / math.prod(EXTENTS[0])
    for index, (small, large) in enumerate(summaries, 1):
        for key in ("sprite_gpu_ns", "whole_gpu_ns"):
            if large[key]["p50"] > small[key]["p50"] * pixels_ratio:
                trips.append(f"take {index} {key}: grows faster than pixel extent at one fixed row")
    if target == "mac-metal":
        pair = record["valley"]
        before_header, before = pair["before"]
        after_header, after = pair["after"]
        for key in ("adapter", "extent", "capture_source", "recipe", "mode", "timing_only"):
            if before_header[key] != after_header[key]:
                raise ValueError(f"unpaired valley settings: {key}")
        if (pair["before_commit"] != STARTING or pair.get("build") != VALLEY_BUILD
                or not pair.get("route") or not all(sha256_field(pair.get(key)) for key in (
                    "route_sha256", "cooked_head_sha256", "before_binary_sha256", "after_binary_sha256"))):
            raise ValueError("missing valley build, route, cooked or load provenance")
        for name, complete in (("before_load", load_averages), ("after_load", load_snapshot)):
            observations = pair.get(name)
            if (not isinstance(observations, list) or len(observations) != 2
                    or not all(complete(entry) for entry in observations)):
                raise ValueError(f"missing valley load observations: {name}")
        if set(before) != set(after) or len(before) < 300 or not all(
                type(value) in (float, int) and math.isfinite(value) and value >= 0 for value in
                [*before.values(), *after.values()]):
            raise ValueError("incomplete paired valley samples")
        for mode in ("Direct", "Frustum", "Occlusion"):
            keys = [key for key in before if key.endswith(f"/{mode}")]
            ratios = [after[key] / before[key] for key in keys if before[key] > 0]
            ratio = statistics.median(ratios) if ratios else 0
            print(f"valley {mode}: {len(keys)} paired ticks, median ratio {ratio:.3f}")
            if ratio > 1.25:
                trips.append(f"valley {mode}: paired median {ratio:.3f} under different recorded loads")
    print(json.dumps({"target": target, "summaries_ns": summaries, "tripwires_flagged": trips,
                      "refresh_tripwire": "10,000 rows is a later milestone; this scene draws one card",
                      "thresholds": "reported observations only"}, indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("costs", "record-costs"))
    parser.add_argument("--target", choices=TARGETS, required=True)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/v1-game")
    parser.add_argument("--web-root", type=Path, default=ROOT / "target/v1-game-web")
    for name in ("before", "before-load", "base-binary", "binary", "route", "store"):
        parser.add_argument(f"--valley-{name}", type=Path)
    args = parser.parse_args()
    retained = EVIDENCE / f"cost-{args.target}.json.gz"
    if args.action == "costs":
        with gzip.open(retained, "rt") as data:
            validate(json.load(data), args.target)
        return
    if not args.output:
        parser.error("record-costs requires a new --output under tmp/")
    folder = args.output.resolve()
    if not folder.is_relative_to(ROOT / "tmp"):
        parser.error("output must be under this checkout's tmp/")
    folder.mkdir(parents=True)
    metal = args.target == "mac-metal"
    if metal and any(getattr(args, f"valley_{name}") is None for name in (
            "before", "before_load", "base_binary", "binary", "route", "store")):
        parser.error("Metal collection requires both valley binaries, starting run, load, route and store")
    commit = git("rev-parse", "HEAD")
    binary = args.binary.resolve() if metal else args.web_root.resolve() / "v1_game_bg.wasm"
    identity = digest(binary)
    compiler = ["rustc"] if metal else ["rustup", "run", "stable", "rustc"]
    record = {"target": args.target, "profile": "release", "commit": commit, "build": BUILDS[args.target],
              "toolchain": subprocess.check_output([*compiler, "--version"], text=True).strip(),
              "binary_sha256": identity, "runs": [], "take_started_unix_ns": []}
    for index in range(1, 3):
        record["take_started_unix_ns"].append(time.time_ns())
        take = folder / f"take-{index}"
        record["runs"].append(metal_take(take, binary, identity, commit) if metal
                              else chrome_take(take, args.web_root.resolve()))
    if metal:
        record["valley"] = valley_pair(args, folder)
    validate(record, args.target)
    with gzip.open(folder / retained.name, "wt") as data:
        json.dump(record, data, separators=(",", ":"))


if __name__ == "__main__":
    main()
