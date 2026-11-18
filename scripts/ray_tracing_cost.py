#!/usr/bin/env python3
"""Retain contended measurements of the Metal ray structure."""

import argparse
import json
import math
import os
from pathlib import Path
import statistics
import subprocess


ROOT = Path(__file__).resolve().parents[1]
PROFILES = {(10_000, 0), (10_000, 1500), (10_000, 10_000), (100_513, 0)}
SAMPLES = 13


def command(*args):
    return subprocess.check_output(args, cwd=ROOT, text=True).strip()


def parse(output):
    profiles = {}
    for line in output.splitlines():
        fields = line.split()
        if not fields or fields[0] not in {"structure", "rebuild"}:
            continue
        key = tuple(map(int, fields[1:3]))
        if fields[0] == "structure":
            control, traced, resident = map(int, fields[3:6])
            record, submit, gpu, bracket = map(float, fields[6:])
            profiles[key] = {
                "instances": key[0],
                "movers": key[1],
                "plain_scene_allocated_bytes": control,
                "ray_scene_allocated_bytes": traced,
                "resident_delta_bytes": resident,
                "first_build_cpu_us": round(record + submit, 3),
                "first_build_record_cpu_us": record,
                "first_build_finish_submit_cpu_us": submit,
                "first_build_gpu_bracket_us": gpu,
                "no_build_gpu_bracket_us": bracket,
                "rebuilds": [],
            }
        else:
            frame = int(fields[3])
            record, submit, gpu = map(float, fields[4:])
            profiles[key]["rebuilds"].append({
                "frame": frame,
                "cpu_us": round(record + submit, 3),
                "record_cpu_us": record,
                "finish_submit_cpu_us": submit,
                "gpu_bracket_us": gpu,
            })
    if set(profiles) != PROFILES:
        raise ValueError(f"missing structure profile: {PROFILES - profiles.keys()}")
    for profile in profiles.values():
        rows = profile["rebuilds"]
        if [row["frame"] for row in rows] != list(range(SAMPLES)):
            raise ValueError(f"missing rebuild samples: {profile}")
        for field in ("cpu_us", "record_cpu_us", "finish_submit_cpu_us", "gpu_bracket_us"):
            profile[f"median_rebuild_{field}"] = statistics.median(row[field] for row in rows)
        values = [
            profile["first_build_cpu_us"], profile["first_build_gpu_bracket_us"],
            profile["no_build_gpu_bracket_us"], profile["resident_delta_bytes"],
            *(row[field] for row in rows for field in ("cpu_us", "gpu_bracket_us")),
        ]
        if not all(math.isfinite(value) and value > 0 for value in values):
            raise ValueError(f"missing timing or resident measurement: {profile}")
    return list(profiles.values())


def structure(output):
    host = command("system_profiler", "SPHardwareDataType", "SPDisplaysDataType")
    machine = [line.strip() for line in host.splitlines() if line.strip().startswith(
        ("Chip:", "Chipset Model:", "Memory:", "Total Number of Cores:")
    )]
    load_before = os.getloadavg()
    result = subprocess.run(
        ["cargo", "test", "--locked", "--offline", "--release", "-p", "thrum-render",
         "--features", "creation-counters", "--test", "ray_structure_cost_gpu", "--",
         "--ignored", "--nocapture"],
        cwd=ROOT, text=True, stdout=subprocess.PIPE, check=True,
        env=dict(os.environ, THRUM_REQUIRE_GPU="1"),
    )
    record = {
        "revision": command("git", "rev-parse", "HEAD"),
        "os": command("sw_vers", "-productVersion"),
        "machine": machine,
        "load_average_before": load_before,
        "load_average_after": os.getloadavg(),
        "profiles": parse(result.stdout),
    }
    report = json.dumps(record, indent=2) + "\n"
    print(report, end="")
    if output:
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(report)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("profile", choices=["structure"])
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    structure(args.output)


if __name__ == "__main__":
    main()
