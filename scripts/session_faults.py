"""Collect native conditioned-link fault and baseline-cost records from match-proof."""

import hashlib
import itertools
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time

import sim2d_conformance as conformance

ROOT = conformance.ROOT
SCRATCH = ROOT / "tmp/session-faults"
EVIDENCE = ROOT / ".orchestrate/evidence/SESSION/M3"
SCENARIOS = ("join", "reconnect", "repair", "slow", "lag", "join-pair", "join-costs")
COST_SIZES = (2_000, 5_000, 10_000)


def run(binary, scenario, count, provenance):
    start = time.monotonic()
    load = os.getloadavg()
    print(f"mac-metal {scenario} at {count:,} bodies", flush=True)
    proof = subprocess.run([binary, "fault", scenario, str(count)], check=True,
                           stdout=subprocess.PIPE, text=True)
    return {"commit": provenance["commit"], "load_before": load,
            "load_after": os.getloadavg(), "elapsed_s": time.monotonic() - start,
            "configured_bodies": count, **json.loads(proof.stdout)}


def comparable_fields(row):
    retention = ("largest_retention_bytes", "largest_retention_sets",
                 "largest_staging_frames", "largest_link_queue_bytes")
    retained = {key: row[key] for key in retention}
    ordinary = row["ordinary_host_ticks_ns"]
    retained["ordinary_host_tick.average_ns"] = ordinary["total"] / ordinary["samples"]
    retained["ordinary_host_tick.max_ns"] = ordinary["max"]
    for direction, count in zip(("to_host", "from_host"), row["bytes_by_direction"], strict=True):
        retained[f"bytes_per_second.{direction}"] = count * 1_000_000 / row["measured_until_us"]
    for name in ("join", "repair"):
        shot = row[name]
        retained[f"{name}.catch_up_steps"] = shot["catch_up_steps"]
        retained[f"{name}.catch_up_ns"] = shot["catch_up_calls_ns"]["total"]
        retained[f"{name}.catch_up_engine_ns"] = shot["catch_up_engine_ns"]
        for field in ("restored_to_playing_us", "hello_to_playing_us",
                      "restore_to_playing_us", "injection_to_playing_us"):
            if field in shot:
                retained[f"{name}.{field}"] = shot[field]
        for key in ("image_bytes", "tick_ns", "image_write_ns"):
            retained[f"{name}.{key}"] = shot["write_tick"][key]
    for field in ("peak", "longest_above_ticks"):
        retained[f"stall.{field}"] = row["stall_backlog"][field]
        retained[f"join.{field}"] = row["join"]["backlog"][field]
    return retained


def costs(binary, provenance):
    rows = []
    for run_id, count in itertools.product((1, 2), COST_SIZES):
        row = run(binary, "join-costs", count, provenance)
        row["run"] = run_id
        flags = []
        assert row["ordinary_host_ticks_ns"]["samples"] > 0
        assert row["largest_retention_bytes"] > 0 and row["largest_retention_sets"] > 0
        assert row["matched_after_repair"] > 0
        period = row["schedule"]["tick_us"] * 1_000
        for key in ("join", "repair"):
            event = row[key]
            tick = event["write_tick"]
            assert tick["image_bytes"] == event["image_bytes"] > 0
            assert tick["writing_tick"] == tick["baseline_tick"] + 1
            assert tick["image_write_ns"] > 0 and tick["tick_ns"] >= tick["image_write_ns"]
            assert event["catch_up_calls_ns"]["samples"] > 0
            assert event["catch_up_steps"] > 0
            assert 0 < event["catch_up_engine_ns"] <= event["catch_up_calls_ns"]["total"]
            if tick["tick_ns"] > period:
                flags.append({"tripwire": "image-writing tick exceeded its period", "phase": key,
                              "observed_ns": tick["tick_ns"], "period_ns": period})
        for direction, size in zip(("to_host", "from_host"), row["bytes_by_direction"], strict=True):
            rate = size * 1_000_000 / row["measured_until_us"]
            if rate > 8_000:
                flags.append({"tripwire": "A0 peer bandwidth above 64 kbit/s", "direction": direction,
                              "observed_bytes_per_second": rate})
        if run_id > 1:
            prior = next(old for old in rows if old["configured_bodies"] == count)
            for field, value in comparable_fields(row).items():
                previous = comparable_fields(prior)[field]
                if value > previous * 1.25:
                    flags.append({"tripwire": "row more than a quarter above comparable run",
                                  "field": field, "previous": previous, "observed": value})
        if count != COST_SIZES[0]:
            small = rows[-1]
            current, previous = row["join"], small["join"]
            if (current["hello_to_playing_us"] * previous["image_bytes"]
                    > previous["hello_to_playing_us"] * current["image_bytes"]):
                flags.append({"tripwire": "join time grew faster than image size",
                              "comparison_bodies": small["configured_bodies"]})
        row["tripwires"] = flags
        rows.append(row)
    return rows


def proof(scenario, output, build):
    provenance = conformance.identity()
    SCRATCH.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=f"{scenario}-", dir=SCRATCH) as directory:
        binary = Path(directory) / "match-proof"
        shutil.copy2(build("mac-metal"), binary)
        artifact = hashlib.sha256(binary.read_bytes()).hexdigest()
        if scenario == "join-costs":
            records = costs(binary, provenance)
        else:
            count = 10_000 if scenario in ("join", "join-pair") else 2_000
            records = [run(binary, scenario, count, provenance)]
    path = Path(output) if output else EVIDENCE / f"memory-{scenario}.json"
    path.parent.mkdir(parents=True, exist_ok=True)
    record = {**provenance, "build": "release", "target": "mac-metal", "artifact_sha256": artifact,
              "conditioner": {"one_way_ms": 50, "jitter_ms": 10, "loss_ppm": 20_000, "duplicates": True},
              "scenario": scenario, "runs": records, "result": "pass"}
    temporary = path.with_suffix(f".{os.getpid()}.partial")
    temporary.write_text(json.dumps(record, indent=2) + "\n")
    temporary.replace(path)
    print(json.dumps({"result": "pass", "record": str(path)}))
