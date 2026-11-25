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
import tempfile
import time

ROOT = Path(__file__).resolve().parent.parent
FIXTURES = ROOT / "tests/fixtures/render-2d"
STARTING = "4392b652fb63994e6ca9090e9b5d20602777337d"
TARGETS = ("mac-metal", "chrome-webgpu")
BUILDS = {
    "mac-metal": "cargo build --locked --offline --release -p v1-game --bin v1-game",
    "chrome-webgpu": "sh examples/v1-game/web/build.sh --proving (locked offline release)",
}
LEGACY_CHROME_BUILD = "sh examples/v1-game/web/build.sh (locked offline release)"
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
    build = record.get("build")
    build_matches = build == BUILDS[target] or (
        target == "chrome-webgpu" and build == LEGACY_CHROME_BUILD
    )
    if (record["target"] != target or record["profile"] != "release" or
            not re.fullmatch("[a-f0-9]{40}", record["commit"]) or
            not sha256_field(record.get("binary_sha256")) or
            not build_matches or not isinstance(toolchain, str) or
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


FIELD_COUNTS = (2000, 5000, 10000)
FIELD_COLUMNS = (*CPU, "order_cpu_ns", "sprite_gpu_ns", "whole_gpu_ns", "cpu_plus_gpu_ns",
                 "upload_bytes", "world_upload_bytes", "order_upload_bytes", "shaded_pixels")


def field_samples(rows, count, target, *, require_fractional=True):
    """Check the scene and measure complete frames that uploaded the whole moving field."""
    expanded = []
    seen_ticks = set()
    fractional_frames = 0
    for row in rows:
        sprite = row.get("sprites")
        if not sprite or (sprite["world_rows"], sprite["frame_rows"], sprite["draws"],
                          sprite["dropped"], sprite["surface_presented"]) != (count, 2, 1, 0, True):
            raise ValueError(f"wrong submitted sprite workload at {target}: {sprite}")
        if sprite["moving"] == count and 0 < sprite["alpha"] < 1:
            fractional_frames += 1
        if sprite["moving"] != count or sprite["tick"] in seen_ticks:
            continue
        if sprite["world_upload_rows"] == 0:
            # A second display sample at a committed tick may carry no row writes.
            continue
        if sprite["world_upload_rows"] != count or sprite["world_upload_bytes"] % count:
            raise ValueError("an all-moving tick did not write exactly the world field")
        if sprite["order_upload_bytes"] != (count + 2) * 4:
            raise ValueError("the queue did not write the full tick order")
        if target == "mac-metal":
            measured = sample(row, row["gpu_pass_ns"], row["gpu_present_ns"], row["submission_id"])
            span = row["timed_pass_span_ns"]
        else:
            measured = sample(row, row["pass_ns"], row["present_ns"], row["submission"])
            span = row["gpu_span_ns"]
        if span is None:
            raise ValueError("missing complete frame GPU timestamp span")
        measured.update(sprite, gpu_pass_span_ns=span)
        measured["cpu_plus_gpu_ns"] = measured["render_cpu_ns"] + span
        if any(type(measured.get(key)) not in (int, float) or not math.isfinite(measured[key])
               or measured[key] < 0 for key in FIELD_COLUMNS):
            raise ValueError(f"incomplete field timing: {measured}")
        if not measured["world_upload_bytes"] or not measured["shaded_pixels"]:
            raise ValueError("missing writes or shaded pixels for the field")
        expanded.append(measured)
        seen_ticks.add(sprite["tick"])
    if len(expanded) < 32 or (require_fractional and fractional_frames < 10):
        raise ValueError(f"incomplete advancing/interpolating workload: {len(expanded)} ticks, "
                         f"{fractional_frames} fractional frames, {target} {count} rows")
    return expanded[:90], fractional_frames


def native_field_run(folder, count, extent, binary, commit, sha256):
    output = folder / f"{count}-{extent[0]}x{extent[1]}"
    run([sys.executable, "scripts/capture_v1_runtime.py", "--binary", str(binary),
         "--case", "sprites-field-cost", "--mode", "realtime", "--recipe", "portable",
         "--extent", *map(str, extent), "--output", str(output)],
        env={**os.environ, "THRUM_SCENE": "sprites", "THRUM_SPRITE_ROWS": str(count)})
    manifest, runtime = read(output / "manifest.json"), read(output / "runtime.json")
    if (manifest["source_revision"] != commit or manifest["binary_sha256"] != sha256 or
            not manifest["complete"] or runtime["scene_kind"] != "TwoD" or
            tuple(runtime["scene_pass_names"]) != PASSES or
            not runtime["observer_only"] or runtime["backend"] != "Metal"):
        raise ValueError("wrong native scene, build or capture arm")
    samples, fractional_frames = field_samples(
        [row for row in runtime["gpu"] if row.get("gpu_present_ns") is not None], count, "mac-metal")
    return {"count": count, "extent": extent, "load": manifest["system_load"], "adapter": runtime["adapter"],
            "recipe": runtime["recipe"], "samples": samples, "fractional_frames": fractional_frames,
            "pass_names": PASSES, "world_overload_recoveries": runtime["world_overload_recoveries"]}


def browser_field_run(folder, web, sha256):
    before = load()
    run(["node", "scripts/check_v1_browser.mjs", "--case", "sprites-field-cost",
         "--web-root", str(web), "--output", str(folder)])
    after = load()
    gpu = read(folder / "browser-gpu.json")
    shutdown = read(folder / "run.json").get("chrome_shutdown")
    browser_record = read(folder / "sprites-field-cost.json")
    rows = browser_record["rows"]
    if [row["world_rows"] for row in rows] != list(FIELD_COUNTS):
        raise ValueError("missing Chrome workload")
    result = []
    for row in rows:
        report = row["measurement"]
        if tuple(report["passes"]) != PASSES:
            raise ValueError("wrong WebGPU graph")
        # Browser retention selects upload samples. Fractional poses are
        # checked against all completed display frames by the page qualifier.
        samples, _ = field_samples(report["samples"], row["world_rows"], "chrome-webgpu",
                                   require_fractional=False)
        if report["fractional_frames"] < 10:
            raise ValueError("missing Chrome interpolation observations")
        if not row["wasm"] or digest(web / "v1_game_bg.wasm") != sha256:
            raise ValueError("wrong WebGPU build receipt")
        result.append({"count": row["world_rows"], "extent": row["extent"],
                       "load": {"before": before, "after": after},
                       "chrome": gpu["version"]["product"], "adapter": gpu["gpu"]["gpu"]["devices"],
                       "schedule": browser_record["schedule"], "chrome_shutdown": shutdown,
                       "wasm": row["wasm"], "pass_names": PASSES, "samples": samples,
                       "fractional_frames": report["fractional_frames"]})
    return result


def field_tripwires(runs, target):
    flagged = []
    timed = ("extract_cpu_ns", "order_cpu_ns", "sprite_gpu_ns", "gpu_pass_span_ns", "cpu_plus_gpu_ns")
    first = {}
    for take, rows in enumerate(runs, start=1):
        previous = {}
        for row in rows:
            key = tuple(row["extent"])
            metrics = row["summary"] = {
                field: {"p50": statistics.median(frame[field] for frame in row["samples"]),
                        "p95": statistics.quantiles([frame[field] for frame in row["samples"]], n=20,
                                                    method="inclusive")[18]}
                for field in (*FIELD_COLUMNS, "gpu_pass_span_ns")
            }
            name = f"take {take}, {row['count']} rows, {key[0]}x{key[1]}"
            if row.get("world_overload_recoveries"):
                flagged.append(f"{name}: {len(row['world_overload_recoveries'])} world overload recoveries")
            if target == "mac-metal" and row["count"] == 10000:
                over = sum(frame["cpu_plus_gpu_ns"] > 1e9 / 120 for frame in row["samples"])
                if over:
                    flagged.append(f"{name}: {over} samples exceeded the 120 Hz CPU + GPU tripwire")
            if key in previous:
                smaller = previous[key]
                work_ratio = (row["count"] * metrics["shaded_pixels"]["p50"] /
                              (smaller["count"] * smaller["summary"]["shaded_pixels"]["p50"]))
                for field in timed:
                    if metrics[field]["p50"] > work_ratio * smaller["summary"][field]["p50"]:
                        flagged.append(f"{name}: {field} grew faster than rows times shaded pixels")
            previous[key] = row
            comparable = (key, row["count"])
            if take == 1:
                first[comparable] = metrics
            else:
                for field in timed:
                    if metrics[field]["p50"] > 1.25 * first[comparable][field]["p50"]:
                        flagged.append(f"{name}: {field} median over 25% slower than take 1")
    return flagged


def build_proving_web():
    run(["sh", "examples/v1-game/web/build.sh", "--proving"], stdout=sys.stderr,
        env={**os.environ, "CARGO_TARGET_DIR": "target", "RUSTC_WRAPPER": ""})
    return ROOT / "target/v1-game-web-proving"


def prove_field(args, scratch):
    metal = args.target == "mac-metal"
    if args.action == "field" and not metal:
        raise ValueError("use the ordinary sprites-field browser case for Chrome")
    binary = args.binary.resolve()
    web = args.web_root.resolve()
    build = BUILDS[args.target]
    if metal:
        run(BUILDS[args.target].split(), stdout=sys.stderr)
        identity = digest(binary)
    else:
        if web == ROOT / "target/v1-game-web":
            web = build_proving_web()
        else:
            build = f"{build}, prebuilt page at {web.relative_to(ROOT)}"
        identity = digest(web / "v1_game_bg.wasm")
    commit = git("rev-parse", "HEAD")
    compiler = ["rustc"] if metal else ["rustup", "run", "stable", "rustc"]
    result = {"target": args.target, "profile": "release", "commit": commit,
              "build": build, "artifact_sha256": identity,
              "toolchain": subprocess.check_output([*compiler, "--version"], text=True).strip(),
              "time_units": "nanoseconds", "coverage": "clipped pixel centres of axis-aligned quads including overdraw",
              "run_started_unix_ns": [], "runs": []}
    if args.action == "field":
        result["runs"].append([native_field_run(scratch, 10000, EXTENTS[0], binary, commit, identity)])
    else:
        for index in (1, 2):
            take = scratch / f"take-{index}"
            result["run_started_unix_ns"].append(time.time_ns())
            if metal:
                result["runs"].append([native_field_run(take, count, extent, binary, commit, identity)
                                       for extent in EXTENTS for count in FIELD_COUNTS])
            else:
                result["runs"].append(browser_field_run(take, web, identity))
        result["tripwires_flagged"] = field_tripwires(result["runs"], args.target)
    if args.output:
        args.output.write_text(json.dumps(result, indent=2) + "\n")
    for take, rows in enumerate(result["runs"], start=1):
        for row in rows:
            print(json.dumps({key: value for key, value in row.items() if key not in ("samples", "load")}
                             | {"take": take, "measured_ticks": len(row["samples"]), "commit": commit,
                                "build": result["build"], "artifact_sha256": identity}))
    print(json.dumps({"tripwires_flagged": result.get("tripwires_flagged", [])}))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("field", "costs", "record-costs", "window", "crowd", "crowd-costs", "band", "crowd-capture", "crowd-still", "edr", "edr-crowd", "edr-observed"))
    parser.add_argument("--target", choices=TARGETS, required=True)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--allow-occluded", action="store_true", help="accept observed window geometry on an occluded surface")
    parser.add_argument("--web-root", type=Path, default=ROOT / "target/v1-game-web")
    for name in ("before", "before-load", "base-binary", "binary", "route", "store"):
        parser.add_argument(f"--valley-{name}", type=Path)
    args = parser.parse_args()
    if args.action in ("edr", "edr-crowd", "edr-observed"):
        import render2d_edr
        if args.target != "mac-metal":
            parser.error(f"{args.action} requires Metal")
        (ROOT / "tmp").mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(prefix=f"render2d-{args.action}-", dir=ROOT / "tmp") as scratch:
            try:
                record = render2d_edr.proof(args.action, Path(scratch))
            finally:
                if args.output:
                    render2d_edr.retain(Path(scratch), args.output.with_suffix(".frames"))
        record.update(target=args.target, commit=git("rev-parse", "HEAD"))
        if args.output:
            args.output.write_text(json.dumps(record, indent=2) + "\n")
        print(json.dumps(record, separators=(",", ":")))
        return
    if args.action in ("crowd", "crowd-costs", "band", "crowd-capture", "crowd-still"):
        import render2d_crowd
        if args.action in ("band", "crowd-still") and args.target != "mac-metal":
            parser.error(f"{args.action} requires Metal")
        if args.action in ("crowd-capture", "crowd-still") and not args.output:
            parser.error(f"{args.action} requires a new --output folder")
        render2d_crowd.run(args)
        return
    if args.action == "window":
        import render2d_crisp
        if args.target != "mac-metal":
            parser.error(f"{args.action} is a Metal proof")
        (ROOT / "tmp").mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(prefix="render2d-window-", dir=ROOT / "tmp") as scratch:
            record = render2d_crisp.window(Path(scratch), args.binary, allow_occluded=args.allow_occluded)
        if args.output:
            args.output.write_text(json.dumps(record, indent=2) + "\n")
        print(json.dumps(record, separators=(",", ":")))
        return
    if args.allow_occluded:
        parser.error("--allow-occluded applies only to the window proof")
    args.binary = args.binary or ROOT / "target/release/v1-game"
    retained = FIXTURES / f"cost-{args.target}.json.gz"
    if args.action in ("field", "costs"):
        if args.action == "costs":
            with gzip.open(retained, "rt") as data:
                validate(json.load(data), args.target)
        (ROOT / "tmp").mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(prefix="render2d-field-", dir=ROOT / "tmp") as scratch:
            prove_field(args, Path(scratch))
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
    web = args.web_root.resolve()
    if not metal and web == ROOT / "target/v1-game-web":
        web = build_proving_web()
    binary = args.binary.resolve() if metal else web / "v1_game_bg.wasm"
    identity = digest(binary)
    compiler = ["rustc"] if metal else ["rustup", "run", "stable", "rustc"]
    record = {"target": args.target, "profile": "release", "commit": commit, "build": BUILDS[args.target],
              "toolchain": subprocess.check_output([*compiler, "--version"], text=True).strip(),
              "binary_sha256": identity, "runs": [], "take_started_unix_ns": []}
    for index in range(1, 3):
        record["take_started_unix_ns"].append(time.time_ns())
        take = folder / f"take-{index}"
        record["runs"].append(metal_take(take, binary, identity, commit) if metal
                              else chrome_take(take, web))
    if metal:
        record["valley"] = valley_pair(args, folder)
    validate(record, args.target)
    with gzip.open(folder / retained.name, "wt") as data:
        json.dump(record, data, separators=(",", ":"))


if __name__ == "__main__":
    main()
