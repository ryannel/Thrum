"""Run and record Mac controller reconciliation and release measurements."""

from collections import Counter
from concurrent.futures import ThreadPoolExecutor
import hashlib
import itertools
import json
import math
import os
from pathlib import Path
import shutil
import statistics
import sys
import tempfile

import sim2d_conformance as conformance


SCRATCH = conformance.ROOT / "tmp/session-m4"
TARGET = "mac-metal"
PROFILES = ("a0", "harsh")
CAUSES = ("HeldFrame", "AuthoringGap", "Command", "Rule")
SIZES = (2000, 5000, 10000)
FRAME_MS = {"120_hz": 1000 / 120, "60_hz": 1000 / 60}


def run(artifact, profile, repeat, costs):
    ticks = 3600 if costs else 36000
    scenario = "costs" if costs else "long"
    print(f"{profile} run {repeat}: {ticks:,} ticks", file=sys.stderr, flush=True)
    load_before = os.getloadavg()
    record = json.loads(conformance.command([artifact, "prediction", scenario, profile, ticks, 2000, 29721]))
    assert record["schedule"]["host_ticks"] == ticks
    record.update(repeat=repeat, load_before=load_before, load_after=os.getloadavg())
    if costs:
        add_cost_summary(record)
    return record


def time_fields(row):
    # Check record completeness only. A measured value is never a threshold.
    assert set(row) == {"samples", "total", "max"} and row["samples"]
    assert all(isinstance(value, int) for value in row.values())
    return {**row, "mean": row["total"] / row["samples"]}


def add_cost_summary(record):
    assert len(record["branches"]) == 2 and len(record["load_before"]) == len(record["load_after"]) == 3
    corrections = []
    for branch in record["branches"]:
        for field in ("lead_ticks", "rule_tick_ns", "session_advance_ns", "engine_tick_ns"):
            branch[field] = time_fields(branch[field])
        assert set(branch["lead_ms"]) == {"mean", "max"}
        assert all(math.isfinite(value) for value in branch["lead_ms"].values())
        for row in branch["corrections"]:
            assert {"tick", "horizon", "replayed", "cause", "before_bits", "after_bits", "distance"} <= row.keys()
            assert row["cause"] in CAUSES and math.isfinite(row["distance"])
        corrections.extend(branch["corrections"])
    minutes = record["schedule"]["run_us"] / 60_000_000
    causes = Counter(row["cause"] for row in corrections)
    replay = [row["replayed"] for row in corrections]
    record["corrections_per_minute"] = {cause: causes[cause] / minutes for cause in CAUSES}
    record["replay_steps"] = {
        "median": statistics.median(replay) if replay else None,
        "max": max(replay, default=None),
    }


def sim2d_replay():
    # These committed records were captured on their named targets by SIM-2D.
    retained = conformance.PRIOR_COSTS
    records = []
    for target in (TARGET, "chrome-webgpu"):
        source = retained / f"costs-{target}.json"
        data = source.read_bytes()
        record = json.loads(data)
        assert record["target"] == target and not record["dirty"] and len(record["runs"]) == 2
        rows = []
        for run in record["runs"]:
            for trace in ("crowd", "mixed"):
                for parallel in ((False, True) if target == TARGET else (False,)):
                    selected = [row for row in run["rows"] if row["trace"] == trace and row["parallel"] == parallel]
                    assert sorted(row["bodies"] for row in selected) == list(SIZES)
                    for row in selected:
                        value = row["replay_eight_ms"]
                        assert row["build"] == "release" and math.isfinite(value)
                        rows.append({
                            "repeat": run["repeat"], "bodies": row["bodies"], "trace": trace,
                            "parallel": parallel, "replay_eight_ms": value,
                            "load_before": row["load_before"], "load_after": row["load_after"],
                            "frame_periods": {rate: value / frame for rate, frame in FRAME_MS.items()},
                        })
        records.append({
            "target": target, "source": str(source.relative_to(conformance.ROOT)),
            "source_sha256": hashlib.sha256(data).hexdigest(), "commit": record["commit"],
            "artifact_sha256": record["artifact_sha256"], "machine": record["machine"],
            "user_agent": record["user_agent"], "build": "release", "rows": rows,
        })
    return {"frame_ms": FRAME_MS, "records": records}


def proof(scenario, output, build):
    provenance = conformance.identity()
    costs = scenario == "prediction-costs"
    SCRATCH.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=f"{scenario}-", dir=SCRATCH) as directory:
        artifact = Path(directory) / "match-proof"
        shutil.copy2(build(TARGET), artifact)
        artifact_hash = hashlib.sha256(artifact.read_bytes()).hexdigest()
        runs = []
        with ThreadPoolExecutor(max_workers=2) as pool:
            for repeat in (1, 2) if costs else (1,):
                pending = [pool.submit(run, artifact, profile, repeat, costs) for profile in PROFILES]
                runs.extend(job.result() for job in pending)
    assert [(row["repeat"], row["profile"]) for row in runs] == list(itertools.product(
        (1, 2) if costs else (1,), PROFILES))
    assert len({row["pid"] for row in runs}) == len(runs)
    packet = {**provenance, "scenario": scenario, "target": TARGET, "build": "release",
              "artifact_sha256": artifact_hash, "runs": runs, "result": "pass"}
    if costs:
        packet["sim2d_replay"] = sim2d_replay()
        packet["timings_are_baselines"] = True
        packet["measurement_scope"] = {
            "rule_tick_ns": "Canonical profile conversion and one production controller step, including replay steps",
            "session_advance_ns": "Prediction-enabled replica advance, including committed engine steps",
            "engine_tick_ns": "Registered runtime step from within that same session advance",
        }
    output = output or SCRATCH / f"{scenario}.json"
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(packet, indent=2, allow_nan=False) + "\n")
    print(json.dumps({"result": "pass", "record": str(output)}))
