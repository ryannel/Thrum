"""Execute and validate paired Metal route timing traces for the Sun-following charts."""
import json
import math
import os
import platform
import statistics
import subprocess
import time
from collections import Counter
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
WHOLE = {"initialization", "camera cut", "air change", "Sun jump", "recovery", "motion cut"}


def jsonl(path):
    return [json.loads(line) for line in path.read_text().splitlines()]


def stage_rows(directory, cadence):
    frames = jsonl(directory / "frames.jsonl")
    closing = json.loads((directory / "final-samples.json").read_text())
    samples = [sample for frame in frames for sample in frame["governor"]["samples"]]
    samples.extend(closing["samples"])
    samples.sort(key=lambda sample: sample["submission"])
    if not samples or closing["last_submission"] is None or [
            sample["submission"] for sample in samples] != list(range(closing["last_submission"] + 1)):
        raise ValueError("missing or duplicated submitted-frame timing")
    rows = []
    for sample in samples:
        quality = sample["quality"]
        actual = quality["knobs"].get("atmosphere.chart cadence") if quality else 1
        reason = quality.get("chart_update") if quality else None
        position = quality.get("chart_position") if quality else None
        change = quality.get("chart_change") if quality else None
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
            if reason not in WHOLE or tuple(stages) != STAGES or position is not None:
                raise ValueError(f"invalid whole update at submission {sample['submission']}: {reason}")
        elif cadence == 1 and not rows and tuple(stages) == STAGES:
            reason = "initialization"
        elif actual > 1:
            if position is None:
                if stages:
                    raise ValueError(f"unlabelled chart stages on submission {sample['submission']}")
            else:
                spread, index = position
                if not (isinstance(change, (int, float)) and math.isfinite(change) and change >= 0):
                    raise ValueError(f"missing refresh change on submission {sample['submission']}")
                expected = next((size for size in range(actual, 1, -1) if size * change <= 1), 1)
                if not 0 <= index < spread or (index == 0 and spread != expected):
                    raise ValueError(f"incorrect refresh spread on submission {sample['submission']}")
                if len(stages) > math.ceil(len(STAGES) / spread):
                    raise ValueError(f"submission {sample['submission']} exceeds spread {spread} bound")
        rows.append({"submission": sample["submission"], "tick": sample["tick"], "cadence": actual,
                     "position": position, "change": change, "whole_update": reason, "stages": stages,
                     "chart_ns": sum(times)})
    return rows


def refresh_counts(rows, cadence, route):
    first = route["origin_tick"]
    last = first + route["keys"][-1]["tick"]
    moving = [r for r in rows if first <= r["tick"] < last]
    if not moving or any(row["cadence"] != cadence for row in moving):
        raise ValueError(f"moving segment did not hold the chart knob at {cadence}")
    matched = []
    started, interrupted, unfinished = Counter(), Counter(), Counter()
    for start, row in enumerate(rows):
        if cadence == 1:
            if tuple(row["stages"]) == STAGES and not row["whole_update"]:
                matched.append({"spread": 1, "first": row["submission"], "last": row["submission"]})
                started[1] += 1
        else:
            if row["position"] is None or row["position"][1] != 0:
                continue
            spread = row["position"][0]
            started[spread] += 1
            window = rows[start:start + spread]
            stop = next((index for index, frame in enumerate(window) if frame["whole_update"]), None)
            prefix = window[:stop]
            if any(frame["position"] != [spread, index] or frame["change"] != row["change"]
                   for index, frame in enumerate(prefix)):
                raise ValueError(f"inconsistent refresh positions at submission {row['submission']}")
            if stop is not None:
                interrupted[spread] += 1
                continue
            if len(window) < spread:
                unfinished[spread] += 1
                continue
            actual = tuple(stage for frame in window for stage in frame["stages"])
            if all(first <= frame["tick"] < last for frame in window) and actual != STAGES:
                raise ValueError(f"incomplete chart stages in moving refresh {row['submission']}: {actual}")
            matched.append({"spread": spread, "first": row["submission"], "last": window[-1]["submission"]})
    moving_ids = {row["submission"] for row in moving}
    moving_matched = [refresh for refresh in matched
                      if all(submission in moving_ids
                             for submission in range(refresh["first"], refresh["last"] + 1))]
    moving_counts = Counter(refresh["spread"] for refresh in moving_matched)
    by_spread = Counter(refresh["spread"] for refresh in matched)
    if cadence == 4 and 4 * moving_counts[4] < 0.75 * len(moving):
        raise ValueError(f"four-submission refreshes cover only {4 * moving_counts[4]}/{len(moving)} submissions")
    if not moving_matched:
        raise ValueError(f"no complete ordinary cadence-{cadence} refresh in the moving route")
    return {
        "complete_refreshes": matched,
        "refreshes_by_spread": {spread: by_spread[spread] for spread in range(1, cadence + 1)},
        "refresh_outcomes_by_spread": {
            spread: {"started": started[spread], "completed": by_spread[spread],
                     "interrupted": interrupted[spread], "unfinished": unfinished[spread]}
            for spread in range(1, cadence + 1)
        },
        "moving_submissions": len(moving),
        "moving_complete_refreshes_by_spread": {
            spread: moving_counts[spread] for spread in range(1, cadence + 1)
        },
    }


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
    if cadence == 4:
        env["THRUM_MAC_CHART_CADENCE"] = "4"
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
    refreshes = refresh_counts(rows, cadence, route_data)
    (directory / "chart-frames.json").write_text(json.dumps(rows, separators=(",", ":")) + "\n")
    chart_times = sorted(row["chart_ns"] for row in rows)
    return {"host_load": json.loads((directory / "host-load.json").read_text()),
            "samples": len(rows), "p50_ns": statistics.median(chart_times),
            "p95_ns": chart_times[(95 * len(rows) - 1) // 100],
            "max_stages": max(len(row["stages"]) for row in rows), **refreshes,
            "whole_updates": dict(Counter(row["whole_update"] for row in rows if row["whole_update"]))}


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
    route["start"]["solar_hours"] = 16.0
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
