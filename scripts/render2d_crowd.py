"""Release Crowd replay receipts and T2 submitted-image captures."""

import json
import math
import os
from pathlib import Path
import shutil
import statistics
import subprocess
import sys
import tempfile
import time
import uuid

from capture_spool import decoded_frames
from capture_v1_runtime import preview
from render2d import EXTENTS, PASSES, ROOT, digest, git, load
from t2_writer_probe import package

PACKAGE = "crowd-2d-view"
COUNTS = (2000, 5000, 10000)
RETAINED = ROOT / "examples/crowd-2d/retained/crowd.txt"
PROOF = ROOT / ".orchestrate/evidence/RENDER-2D/WP5"
METRICS = ("tick_ns", "extract_ns", "order_ns", "upload_bytes", "sprite_gpu_ns",
           "frame_cpu_ns", "frame_gpu_ns", "shaded_pixels")


def invoke(command, timeout=1500, **options):
    subprocess.run(list(map(str, command)), cwd=ROOT, check=True, timeout=timeout, **options)


def build(target, scratch):
    compiler = subprocess.check_output(["rustup", "which", "rustc", "--toolchain", "stable"], text=True).strip()
    cmd = ["rustup", "run", "stable", "cargo", "build", "--locked", "--offline", "--release", "-p", PACKAGE]
    env = dict(os.environ, RUSTC=compiler, RUSTC_WRAPPER="")
    target_root = Path(env.get("CARGO_TARGET_DIR", ROOT / "target"))
    if not target_root.is_absolute():
        target_root = ROOT / target_root
    if target == "mac-metal":
        invoke(cmd, env=env, stdout=sys.stderr)
        return target_root / "release" / PACKAGE, cmd
    cmd += ["--lib", "--target", "wasm32-unknown-unknown"]
    invoke(cmd, env=env, stdout=sys.stderr)
    bindgen = shutil.which("wasm-bindgen") or Path.home() / ".cargo/bin/wasm-bindgen"
    web = scratch / "web"
    invoke([bindgen, "--target", "web", "--out-dir", web,
            target_root / "wasm32-unknown-unknown/release/crowd_2d_view.wasm"])
    shutil.copy(ROOT / "examples/crowd-2d-view/web/index.html", web)
    return web, cmd


def retained():
    folds, ticks = {}, {}
    for line in RETAINED.read_text().splitlines():
        if line.startswith(("fold ", "checkpoint ")):
            kind, count, tick, value = line.split()
            (folds if kind == "fold" else ticks)[int(count), int(tick)] = value
    return folds, ticks


def fold_rows(rows):
    fold = 0xCBF29CE484222325
    for row in rows:
        for byte in int(row["digest"], 16).to_bytes(8, "little"):
            fold = ((fold ^ byte) * 0x100000001B3) & ((1 << 64) - 1)
    return f"{fold:016x}"


def replay(artifact, target, action, count, extent, ticks, where, close=False):
    where.mkdir()
    run_id = uuid.uuid4().hex
    if target == "mac-metal":
        invoke([artifact, action, count, *extent, "close" if close else "whole",
                where if action in ("fixed", "realtime") else where / "replay.json"],
               env=dict(os.environ, THRUM_CROWD_RUN_ID=run_id))
        report = where / ("runtime.json" if action in ("fixed", "realtime") else "replay.json")
    else:
        request = dict(action=action, count=count, extent=extent, ticks=ticks, close=close, run_id=run_id)
        invoke(["node", "scripts/render2d_crowd_browser.mjs", artifact, where, json.dumps(request)])
        report = where / "replay.json"
    data = json.loads(report.read_text())
    if action in ("crowd", "crowd-costs") and data.get("run_id") != run_id:
        raise ValueError("the replay receipt came from a different launch")
    return data


def verify(record, *, target, count, extent, ticks, costs, expected):
    folds, checkpoints = expected
    if (record["workload"], record["target"], record["count"], record["extent"], record["passes"]) != (
            "retained Crowd", target, count, list(extent), list(PASSES)):
        raise ValueError("wrong workload, target, extent or compiled passes")
    if target == "mac-metal" and "backend: Metal" not in record["adapter"]:
        raise ValueError("expected actual Metal")
    if target == "chrome-webgpu" and ("Chrome/" not in record["user_agent"] or "Portable" not in record["recipe"]):
        raise ValueError("expected Chrome and the portable GPU recipe")
    run_id = record.get("run_id")
    if not isinstance(run_id, str) or len(run_id) != 32:
        raise ValueError("missing replay identity")
    pubs, samples = record["publications"], record["samples"]
    if len(pubs) != ticks or len(samples) != ticks:
        raise ValueError("expected one publication and completed submission per replay tick")
    selected, matched = [], {}
    previous_submission = None
    for tick, (publication, sample) in enumerate(zip(pubs, samples), 1):
        if not (publication.get("run_id") == sample.get("run_id") == run_id and
                publication.get("submission") == sample["submission"] and
                publication["tick"] == sample["tick"] == tick and
                publication["live"] == publication["mapped"] == sample["world_rows"] > 0 and
                sample["draws"] == 1 and sample["extent"] == list(extent) and sample["presented"] and
                sample["dropped"] == 0 and (previous_submission is None or sample["submission"] > previous_submission)):
            raise ValueError(f"wrong publication/submission join at {tick}: {publication}, {sample}")
        previous_submission = sample["submission"]
        if tick % 120 == 0:
            expected_digest = checkpoints[count, tick]
            if publication["digest"] != expected_digest:
                raise ValueError(f"retained digest differs at {count}/{tick}")
            matched[tick] = expected_digest
        if costs and 120 <= tick <= 240 and sample["timing_submission"] == sample["submission"]:
            if not all(type(sample.get(key)) in (int, float) and math.isfinite(sample[key])
                       and sample[key] >= 0 for key in METRICS):
                raise ValueError(f"incomplete measurement: {sample}")
            if not sample["shaded_pixels"] or not sample["upload_bytes"] or not sample["frame_gpu_ns"]:
                raise ValueError(f"empty timed submission: {sample}")
            sample["overdraw"] = sample["shaded_pixels"] / math.prod(extent)
            selected.append(sample)
    if costs:
        if len(selected) < 90:
            raise ValueError(f"only {len(selected)} out of 121 replay-joined cost samples")
        record["timings"] = {
            key: {"p50": statistics.median(row[key] for row in selected),
                  "p95": statistics.quantiles([row[key] for row in selected], n=20)[18]}
            for key in (*METRICS, "overdraw")
        }
        record["sampled_submissions"] = [row["submission"] for row in selected]
    else:
        digest_fold = fold_rows(pubs)
        if digest_fold != folds[count, ticks]:
            raise ValueError(f"retained complete timeline fold differs at {count}/{ticks}")
        record["timeline_fold"] = digest_fold
    reused = sum(row["reused"] for row in pubs)
    skipped = sum(row["skipped_interpolation"] for row in pubs)
    if not reused or skipped != reused:
        raise ValueError("no slot reuse, or a reused slot interpolated across identity")
    record.update(reused=reused, skipped_interpolation=skipped, checkpoints=matched)


def capture_output(record, source, out, mode):
    if record["mode"] != mode:
        raise ValueError("wrong T2 capture mode")
    images = record["images"]
    if record["expected"] != len(images) or len(record["hashes"]) != len(images):
        raise ValueError("missing submitted images")
    if len({row["submission"] for row in images}) != len(images):
        raise ValueError("duplicate submitted image")
    if not all(row["presented"] and row["draws"] == 1 and row["world_rows"] >= 10000 for row in images):
        raise ValueError("captured images did not come from the 10,000-body sprite view")
    out.mkdir(parents=True)
    [width, height] = images[0]["extent"]
    spool = source / ("frames.spool" if record["encoding"] == "ZstdDelta" else "frames.rgba")
    frames = decoded_frames(spool, record["encoding"], record["hashes"], width * height * 4, 1600)
    if mode == "fixed":
        for frame in frames:
            package(frame, out / "still.png", width, height)
    else:
        start = images[0]["submit_ns"]
        if images[-1]["submit_ns"] - start < 20_000_000_000:
            raise ValueError("submitted realtime images span less than twenty seconds")
        files = out / "png"
        files.mkdir()
        for row, frame in zip(images, frames, strict=True):
            package(frame, files / f'{row["submission"]:06}.png', width, height)
        records = [{"submission_id": row["submission"], "submit_ns": row["submit_ns"] - start}
                   for row in images]
        movie = {"timebase": "realtime"}
        preview(out, movie, {"interval_end_ns": records[-1]["submit_ns"] + 16_666_667}, records)
        record["preview"] = movie
        for label, index in [("start", 0), ("middle", len(records) // 2), ("end", -1)]:
            frame = files / f'{images[index]["submission"]:06}.png'
            shutil.copy(frame, out / f"{label}.png")
        shutil.rmtree(files)
    (out / "record.json").write_text(json.dumps(record, indent=2) + "\n")


def run(args):
    from render2d import digest
    costs = args.action == "crowd-costs"
    target = args.target
    commit = git("rev-parse", "HEAD")
    expected = retained()
    scratch_root = ROOT / "tmp"
    scratch_root.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(dir=scratch_root, prefix="render2d-crowd-") as folder:
        scratch = Path(folder)
        artifact, command = build(target, scratch)
        binary = artifact if target == "mac-metal" else artifact / "crowd_2d_view_bg.wasm"
        identity = dict(commit=commit, build=command, profile="release", artifact_sha256=digest(binary),
                        retained_sha256=digest(RETAINED))
        if args.action == "band":
            output = scratch / "band.json"
            invoke([artifact, "band", 10000, 320, 200, "whole", output])
            receipt = json.loads(output.read_text()) | identity
            record(receipt, args.output)
            return
        if args.action == "crowd-capture":
            modes = ("fixed", "realtime") if target == "mac-metal" else ("realtime",)
            for mode in modes:
                for close in [False, True]:
                    extent = (3456, 2234) if mode == "fixed" else (1920, 1080) if target == "mac-metal" else (1280, 720)
                    label = f"{mode}-{'close' if close else 'whole'}"
                    source = scratch / label
                    data = replay(artifact, target, mode, 10000, extent, 0, source, close)
                    capture_output(data | identity, source, args.output / label, mode)
                    shutil.rmtree(source)
            return
        extents = EXTENTS if costs and target == "mac-metal" else ((1920, 1080),) if costs else ((320, 200),)
        all_runs = []
        for take in range(1, 3 if costs else 2):
            for extent in extents:
                for count in COUNTS:
                    tick_count = 240 if costs else next(tick for size, tick in expected[0] if size == count)
                    where = scratch / f"take-{take}-{extent[0]}-{count}"
                    started = time.time_ns()
                    load_before = load()
                    data = replay(artifact, target, args.action, count, extent, tick_count, where)
                    verify(data, target=target, count=count, extent=extent, ticks=tick_count, costs=costs, expected=expected)
                    data.update(take=take, started_ns=started, load_before=load_before, load_after=load())
                    all_runs.append(data)
                    print(json.dumps({key: value for key, value in data.items()
                                      if key not in ("publications", "samples", "checkpoints", "sampled_submissions")}),
                          flush=True)
        receipt = identity | {"target": target, "workload": "retained Crowd", "runs": all_runs}
        if costs:
            receipt["tripwires"] = tripwires(all_runs)
        record(receipt, args.output)


def tripwires(runs):
    flags = []
    previous = {}
    for run in runs:
        row = run["timings"]
        key = run["count"], tuple(run["extent"])
        ns = row["frame_cpu_ns"]["p95"] + row["frame_gpu_ns"]["p95"]
        if run["target"] == "mac-metal" and run["count"] == 10000 and ns > 1e9 / 120:
            flags.append(dict(take=run["take"], extent=run["extent"], count=run["count"], cpu_plus_gpu_p95_ns=ns))
        if key in previous and ns > previous[key] * 1.25:
            flags.append(dict(take=run["take"], extent=run["extent"], count=run["count"], paired_p95_ratio=ns / previous[key]))
        previous[key] = ns
    return flags


def record(receipt, output):
    if output:
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(json.dumps(receipt, separators=(",", ":")) + "\n")
    print(json.dumps({key: value for key, value in receipt.items() if key != "runs"}), flush=True)
