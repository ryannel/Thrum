#!/usr/bin/env python3
"""Retain contended measurements of the Metal ray structure."""

import argparse
import json
import math
import os
from pathlib import Path
import statistics
import subprocess
from statistics import median

from ray_tracing_proof import run_test


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


def valley(output):
    host = command("system_profiler", "SPHardwareDataType", "SPDisplaysDataType")
    machine = [line.strip() for line in host.splitlines() if line.strip().startswith(
        ("Chip:", "Chipset Model:", "Memory:", "Total Number of Cores:")
    )]
    before = os.getloadavg()
    rows = run_test("valley_cost", release=True)
    memory, = [row for row in rows if row["type"] == "memory"]
    initial = [row for row in rows if row["type"] == "mesh_build" and row["phase"] == "initial"]
    edit, = [row for row in rows if row["type"] == "mesh_build" and row["phase"] == "edit"]
    if {row["mesh"] for row in initial} != {"arena mesh 0", "arena mesh 1", "terrain", "rock", "sphere"}:
        raise ValueError(f"missing initial mesh builds: {initial}")
    if edit["mesh"] != "terrain" or not edit["switch_in_this_frame"]:
        raise ValueError(f"expected the terrain edit to build in its switch submission: {edit}")
    if not all(row["triangles"] > 0 and all(math.isfinite(row[field]) and row[field] > 0
               for field in ("gpu_bracket_ns", "control_bracket_ns")) for row in [*initial, edit]):
        raise ValueError("missing mesh build measurement")
    if memory["incremental_resident_bytes"] <= 0:
        raise ValueError("missing resident bytes")
    record = {
        "revision": command("git", "rev-parse", "HEAD"),
        "target": "mac-metal",
        "os": command("sw_vers", "-productVersion"),
        "machine": machine,
        "load_average_before": before,
        "load_average_after": os.getloadavg(),
        "memory": memory,
        "initial": initial,
        "edit": edit,
        "scope": "Under load. Each interval includes a one-instance measurement structure and "
                 "a query that consumes it. The paired control repeats these commands over the "
                 "finished mesh. The subtraction estimates the edit's additional GPU work "
                 "in its switch submission, outside its copies and ordinary scene structure build.",
    }
    text = json.dumps(record, indent=2) + "\n"
    print(text, end="")
    if output:
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(text)


def shadows(output):
    machine = command("system_profiler", "SPHardwareDataType", "SPDisplaysDataType")
    load_before = os.getloadavg()
    rows = run_test("shadow_cost", release=True)
    passes = ("synthetic Sun shadow near", "synthetic Sun shadow far")
    measurements = {}
    for method, lamps in (("Maps", False), ("Traced", False), ("Traced", True)):
        selected = [row for row in rows if (row["method"], row["lamps"]) == (method, lamps)]
        if not selected:
            raise ValueError(f"missing {method} with lamps={lamps}")
        for row in selected:
            timings = row["passes_ns"]
            if not all((name in timings) == (method == "Maps") for name in passes):
                raise ValueError(f"incorrect cascade timings in {row}")
            if "synthetic Sun horizon heights" not in timings:
                raise ValueError(f"missing height pass in {row}")
            if ("ray structure build" in timings) != (method == "Traced"):
                raise ValueError(f"incorrect scene structure graph in {row}")
            if not all(math.isfinite(row[key]) and row[key] > 0
                       for key in ("timed_pass_span_ns", "frame_cpu_ns")):
                raise ValueError(f"missing measurement in {row}")
        measurements[f"{method}, lamps {lamps}"] = {
            key: median(row[key] for row in selected)
            for key in ("timed_pass_span_ns", "frame_cpu_ns")
        }
    record = {
        "revision": command("git", "rev-parse", "HEAD"),
        "target": "mac-metal",
        "os": command("sw_vers", "-productVersion"),
        "machine": [line.strip() for line in machine.splitlines() if line.strip().startswith(
            ("Chip:", "Chipset Model:", "Memory:", "Total Number of Cores:"))],
        "load_average_before": load_before,
        "load_average_after": os.getloadavg(),
        "timed_pass_span_scope": (
            "First timed pass begin through surface consumer end, including the scene structure "
            "build on Traced. Queue::write_buffer transfers run ahead of that interval. Both "
            "methods queue view/instance/light/cluster inputs and the caster heights' uniforms; "
            "Maps also queues the two cascade view records. Lamp quality is held at its "
            "prepared value and adds no repeated queue transfer. CPU time includes queue writes."
        ),
        "median_ns": measurements,
        "samples": rows,
    }
    text = json.dumps(record, indent=2) + "\n"
    print(text, end="")
    if output:
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(text)


def soft(output):
    machine = command("system_profiler", "SPHardwareDataType", "SPDisplaysDataType")
    before = os.getloadavg()
    rows = run_test("shadow_cost", release=True, extra_env={"THRUM_RAY_SOFT_COST": "1"})
    dive, = [row for row in rows if row.get("type") == "dive_account"]
    opaque = dive["ordinary"] + dive["dipping"] + dive["excluded"]
    dive["share_beyond_limit"] = dive["excluded"] / opaque
    dive["share_inside_limit"] = 1.0 - dive["share_beyond_limit"]

    forward = "synthetic forward opaque"
    stages = [
        "traced Sun visibility trace", "traced Sun visibility neighbourhood",
        "traced Sun visibility accumulation", "traced Sun visibility filter",
    ]
    figures = ("timed_pass_span_ns", "frame_cpu_ns")
    arms = {enabled: {} for enabled in (False, True)}
    for row in rows:
        if row is dive:
            continue
        enabled = row["penumbra"]
        timings = row["passes_ns"]
        if row["method"] != "Traced" or row["lamps"] or row["recipe"] != "AppleTile":
            raise ValueError(f"incorrect traced recipe or selection in {row}")
        if not all((stage in timings) == enabled for stage in stages):
            raise ValueError(f"incorrect visibility timing in {row}")
        values = [timings[forward], *(row[key] for key in figures)]
        values.extend(timings[stage] for stage in stages if enabled)
        if not all(math.isfinite(value) and value > 0 for value in values):
            raise ValueError(f"missing required measurement in {row}")
        if not row["history_bytes"] or not row["dive_limit_degrees"]:
            raise ValueError(f"missing history residency or dive limit in {row}")
        arms[enabled][row["tick"]] = row
    if not arms[False] or arms[False].keys() != arms[True].keys():
        raise ValueError("paired timing rows must cover the same route ticks")

    summary = {}
    for enabled, samples in arms.items():
        summary["on" if enabled else "off"] = {
            **{key: median(row[key] for row in samples.values()) for key in figures},
            **{stage: median(row["passes_ns"][stage] for row in samples.values())
               for stage in ([forward, *stages] if enabled else [forward])},
        }
    record = {
        "revision": command("git", "rev-parse", "HEAD"),
        "target": "mac-metal",
        "machine": [line.strip() for line in machine.splitlines() if line.strip().startswith(
            ("Chip:", "Chipset Model:", "Memory:", "Total Number of Cores:"))],
        "os": command("sw_vers", "-productVersion"),
        "load_average_before": before,
        "load_average_after": os.getloadavg(),
        "timed_pass_span_scope": (
            "Contended. First timed pass through surface consumer. The preceding queued "
            "view, instance, light, cluster, caster heights and soft frame uniforms are "
            "outside the span. CPU time includes those writes."
        ),
        "median_ns": summary,
        "dive_account": dive,
        "pairs": [{"tick": tick, "off": arms[False][tick], "on": arms[True][tick]}
                  for tick in sorted(arms[False])],
    }
    rendered = json.dumps(record, indent=2) + "\n"
    print(rendered, end="")
    if output:
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(rendered)


def ambient(output):
    machine = command("system_profiler", "SPHardwareDataType", "SPDisplaysDataType")
    before = os.getloadavg()
    rows = run_test("shadow_cost", release=True, extra_env={"THRUM_RAY_AMBIENT_COST": "1"})
    trace = "traced Sun and ambient visibility trace"
    receiver_stage = "ambient receiver raster"
    filter_stage = "traced ambient visibility filter"
    figures = ("timed_pass_span_ns", "frame_cpu_ns")
    arms = {0: {}, 1: {}, 2: {}}
    for row in rows:
        extent = row["ambient_extent"]
        timings = row["passes_ns"]
        if row["method"] != "Traced" or not row["penumbra"] or row["recipe"] != "AppleTile":
            raise ValueError(f"incorrect traced preparation: {row}")
        if (trace not in timings
                or any((stage in timings) != (extent > 0) for stage in (receiver_stage, filter_stage))
                or "traced Sun visibility trace" in timings or "traced ambient visibility" in timings):
            raise ValueError(f"incorrect ambient visibility stages: {row}")
        values = [row[key] for key in figures]
        values.append(timings[trace])
        if extent > 0:
            values.extend(timings[stage] for stage in (receiver_stage, filter_stage))
        if not all(math.isfinite(value) and value > 0 for value in values):
            raise ValueError(f"missing timing in {row}")
        arms[extent][row["tick"]] = row
    if not arms[0] or not arms[0].keys() == arms[1].keys() == arms[2].keys():
        raise ValueError("all ambient extent arms must cover the same route ticks")

    names = ("off", "half", "full")
    summary = {
        name: {
            **{key: median(row[key] for row in arms[extent].values()) for key in figures},
            trace: median(row["passes_ns"][trace] for row in arms[extent].values()),
            **{stage: median(row["passes_ns"][stage] for row in arms[extent].values())
               for stage in ((receiver_stage, filter_stage) if extent > 0 else ())},
        }
        for extent, name in enumerate(names)
    }
    record = {
        "revision": command("git", "rev-parse", "HEAD"),
        "target": "mac-metal",
        "machine": [line.strip() for line in machine.splitlines() if line.strip().startswith(
            ("Chip:", "Total Number of Cores:", "Memory:"))],
        "os": command("sw_vers", "-productVersion"),
        "load_average_before": before,
        "load_average_after": os.getloadavg(),
        "timed_pass_span_scope": (
            "Contended. First timed pass through surface consumer, including the scene structure "
            "build. Queue writes to view, instance, light, cluster, caster heights, and the soft "
            "Sun and ambient frame uniforms precede that interval in all three arms. Frame CPU "
            "time includes these writes."
        ),
        "median_ns": summary,
        "pairs": [{"tick": tick, **{name: arms[extent][tick] for extent, name in enumerate(names)}}
                  for tick in sorted(arms[0])],
    }
    rendered = json.dumps(record, indent=2) + "\n"
    print(rendered, end="")
    if output:
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(rendered)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("profile", choices=["structure", "valley", "shadows", "soft", "ambient"])
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    {"structure": structure, "valley": valley, "shadows": shadows, "soft": soft, "ambient": ambient}[args.profile](args.output)


if __name__ == "__main__":
    main()
