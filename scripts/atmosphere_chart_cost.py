"""Execute and validate paired Metal route timing traces for the Sun-following charts."""
import json
import math
import os
import platform
import statistics
import subprocess
import time
from pathlib import Path

from apple_governor_proof import awake_display
from run_v1_showcase import MOVING_ROUTE
from run_v1_showcase import SOURCE, store_root
from t2_load import LoadRecord

ROOT = Path(__file__).resolve().parent.parent
STAGES = (
    "atmosphere sky view",
    "atmosphere diffuse irradiance",
    *(f"atmosphere reflection mip {i}" for i in range(7)),
)
WHOLE = {"initialization", "camera cut", "air change", "Sun jump", "recovery"}


def jsonl(path):
    return [json.loads(line) for line in path.read_text().splitlines()]


def stage_rows(directory, cadence):
    frames = jsonl(directory / "frames.jsonl")
    samples = sorted((sample for frame in frames for sample in frame["governor"]["samples"]),
                     key=lambda sample: sample["submission"])
    if not samples or len({sample["submission"] for sample in samples}) != len(samples):
        raise ValueError("missing or duplicated submitted-frame timing")
    rows = []
    for sample in samples:
        quality = sample["quality"]
        actual = quality["knobs"].get("atmosphere.chart cadence") if quality else 1
        reason = quality.get("chart_update") if quality else None
        position = quality.get("chart_position") if quality else None
        order, timestamps = sample["gpu_pass_order"], sample["gpu_pass_ns"]
        if order is None or timestamps is None:
            raise ValueError(f"missing GPU execution trace for submission {sample['submission']}")
        stages = [stage for stage in order if stage in STAGES]
        times = [timestamps[stage] for stage in stages]
        if len(stages) != len(set(stages)) or any(
                not isinstance(ns, (int, float)) or ns < 0 or not math.isfinite(ns) for ns in times):
            raise ValueError(f"invalid chart GPU execution trace for submission {sample['submission']}")
        if actual is None and reason == "initialization":
            actual = 1  # The first held generation can initialize before its tier applies.
        if actual not in (1, 4, 6, 8):
            raise ValueError(f"missing or invalid chart cadence for submission {sample['submission']}")
        if reason:
            if reason not in WHOLE or tuple(stages) != STAGES:
                raise ValueError(f"invalid whole update at submission {sample['submission']}: {reason}")
        elif cadence == 1 and not rows and tuple(stages) == STAGES:
            reason = "initialization"
        elif actual > 1:
            effective = position[0] if position else actual
            if len(stages) > math.ceil(len(STAGES) / effective):
                raise ValueError(f"ordinary submission {sample['submission']} exceeds cadence {effective} bound")
        rows.append({"submission": sample["submission"], "tick": sample["tick"], "cadence": actual,
                     "position": position, "whole_update": reason, "stages": stages, "chart_ns": sum(times)})
    return rows


def complete_refreshes(rows, cadence, route):
    first = route["origin_tick"]
    last = first + route["keys"][-1]["tick"]
    moving = [r for r in rows if first <= r["tick"] < last and r["cadence"] == cadence]
    matched = []
    for start, row in enumerate(moving):
        window = moving[start:start + cadence]
        neighbors = len(window) == cadence and all(
            b["submission"] == a["submission"] + 1 for a, b in zip(window, window[1:]))
        if not neighbors or any(r["whole_update"] for r in window):
            continue
        actual = tuple(stage for frame in window for stage in frame["stages"])
        if cadence == 1:
            if actual != STAGES:
                continue
        else:
            expected = [[cadence, i] for i in range(cadence)]
            if [r["position"] for r in window] != expected:
                continue
            if actual != STAGES:
                raise ValueError(f"incomplete chart stages in moving refresh {row['submission']}: {actual}")
        matched.append([window[0]["submission"], window[-1]["submission"]])
    return matched


def run_take(output, binary, store, route, cadence):
    directory = output / f"cadence-{cadence}"
    directory.mkdir()
    env = {key: value for key, value in os.environ.items() if not key.startswith("THRUM_")}
    env.update(
        THRUM_ASSET_STORE=str(store), THRUM_SCENE="valley", THRUM_RECIPE="portable",
        THRUM_RECONSTRUCTION="taau", THRUM_GOVERNOR="off" if cadence == 1 else "on",
        THRUM_GOVERNOR_ROUTE=str(route), THRUM_GOVERNOR_RECORD="observed",
        THRUM_MAC_TRANSITION=str(directory), THRUM_MAC_TRANSITION_ARM="route",
        THRUM_MAC_FRAME_RATE="60", THRUM_MAC_EDR="sdr", THRUM_ROUTE_OVERLAY="off",
        THRUM_V1_FRAMES="550", THRUM_REPORT="1",
    )
    load = LoadRecord(directory)
    try:
        with awake_display(), (directory / "game.log").open("w") as log:
            subprocess.run([binary], cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT,
                           check=True, timeout=130)
    finally:
        (directory / "host-load.json").write_text(json.dumps(load.finish(), indent=2) + "\n")
    prepared = json.loads((directory / "prepared.json").read_text())
    if prepared["adapter"] != "Apple M3 Max":
        raise ValueError("route did not run on Apple M3 Max Metal")
    rows = stage_rows(directory, cadence)
    route_data = json.loads(route.read_text())
    refreshes = complete_refreshes(rows, cadence, route_data)
    if not refreshes:
        raise ValueError(f"no complete ordinary cadence-{cadence} refresh in the moving route")
    (directory / "chart-frames.json").write_text(json.dumps(rows, separators=(",", ":")) + "\n")
    chart_times = sorted(row["chart_ns"] for row in rows)
    return {"host_load": json.loads((directory / "host-load.json").read_text()),
            "samples": len(rows), "p50_ns": statistics.median(chart_times),
            "p95_ns": chart_times[(95 * len(rows) - 1) // 100],
            "max_stages": max(len(row["stages"]) for row in rows), "complete_refreshes": refreshes}


def measure(output=None):
    output = (output or (ROOT / "tmp" / "atmosphere-wp12" / f"cost-{time.time_ns()}")).resolve()
    output.mkdir(parents=True)
    subprocess.run(["cargo", "build", "--release", "--locked", "--offline", "-p", "v1-game",
                    "-p", "thrum-cook"], cwd=ROOT, check=True)
    target = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target")).resolve()
    binary, cook = (target / "release" / name for name in ("v1-game", "thrum-cook"))
    store = output / "store"
    subprocess.run([cook, "cook", SOURCE, store], cwd=ROOT, check=True)
    route = json.loads(MOVING_ROUTE.read_text())
    if store_root(store) != route["cooked_root"]:
        raise ValueError("cooked valley root differs from the route")
    route["render_extent"] = [1920, 1080]
    route["overlay"] = "off"
    path = output / "route.json"
    path.write_text(json.dumps(route) + "\n")
    result = {
        "revision": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        "host": platform.platform(), "cooked_root": route["cooked_root"], "chart_stages": STAGES,
        "takes": {cadence: run_take(output, binary, store, path, cadence) for cadence in (1, 4)},
    }
    if result["takes"][4]["p50_ns"] >= result["takes"][1]["p50_ns"]:
        result["tripwire"] = "Governed median did not fall. Check the raw pass and load traces."
    (output / "chart-cost.json").write_text(json.dumps(result, indent=2) + "\n")
    print(output.relative_to(ROOT) if output.is_relative_to(ROOT) else output)
    return result
