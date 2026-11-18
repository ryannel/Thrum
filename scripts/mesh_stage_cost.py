#!/usr/bin/env python3
"""Record paired Apple tile valley costs under load, with capture disabled."""

import argparse
import gzip
import json
import os
from pathlib import Path
import statistics
import subprocess
import tempfile

from mesh_stage_proof import ROOT, cook_valley
from t2_load import LoadRecord


def median(rows, field):
    return statistics.median(row[field] for row in rows)


def summarize(frames):
    return {
        "timed_pass_span_ns": median(frames, "timed_pass_span_ns"),
        "frame_cpu_ns": median(frames, "frame_cpu_ns"),
        "upload_cpu_ns": median(frames, "upload_cpu_ns"),
        "encode_cpu_ns": median(frames, "encode_cpu_ns"),
        "view_update_cpu_ns": median(frames, "view_update_cpu_ns"),
        "passes_ns": {
            name: statistics.median(row["passes_ns"][name] for row in frames)
            if frames[0]["passes_ns"][name] is not None else None
            for name in frames[0]["passes_ns"]
        },
        "mesh_buffer_bytes": frames[0]["mesh_buffer_bytes"],
        "forward_draws": frames[0]["forward_draws"],
        "draws_per_sun_map": frames[0]["draws_per_sun_map"],
    }


def run(command, root, env, arm, width, repeat):
    folder = root / f"{width}-{arm}-{repeat}"
    folder.mkdir()
    load = LoadRecord(folder)
    try:
        subprocess.run(command, cwd=ROOT, check=True, env=dict(
            env, THRUM_MESH_STAGE_PROOF_DIR=str(folder), THRUM_MESH_COST_ARM=arm,
            THRUM_MESH_COST_WIDTH=str(width),
        ))
    finally:
        witness = load.finish()
    data = json.loads((folder / "route.json").read_text())
    if len(data["frames"]) != 361 or data["extent"] != [width, width * 9 // 16]:
        raise ValueError("incomplete valley route samples")
    if data["path"] != ("Indirect" if arm == "indexed" else "MeshStages") or data["recipe"] != "AppleTile":
        raise ValueError("measured wrong cooked path or recipe")
    data.update(load=witness, load_samples=(folder / "system-iostat.txt").read_text(),
                repeat=repeat)
    if arm != "counts":
        data["summary"] = summarize(data["frames"])
    return data


def compare(runs):
    indexed, mesh = [[row for row in runs if row["arm"] == arm] for arm in ("indexed", "mesh")]
    evidence = {}
    for key in ("timed_pass_span_ns", "frame_cpu_ns"):
        a, b = [[row["summary"][key] for row in arm] for arm in (indexed, mesh)]
        delta = statistics.mean(b) - statistics.mean(a)
        spread = max(abs(a[1] - a[0]), abs(b[1] - b[0]))
        ratio = delta / statistics.mean(a)
        evidence[key] = dict(delta_ns=delta, spread_ns=spread, fraction=ratio,
                             separated=abs(delta) > spread, regression_tripwire=ratio >= 0.25)
    return evidence


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("scene", choices=["valley"])
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    if os.uname().sysname != "Darwin":
        parser.error("Metal hardware is required")
    parent = ROOT / "tmp/mesh-shading-M2"
    parent.mkdir(parents=True, exist_ok=True)
    root = args.output.resolve() if args.output else Path(tempfile.mkdtemp(prefix="cost-", dir=parent))
    root.mkdir(parents=True, exist_ok=True)
    print(f"cost evidence {root}", flush=True)
    store = root / "store"
    cook_valley(store)
    env = dict(os.environ, THRUM_REQUIRE_GPU="1", THRUM_SCENE="valley", THRUM_STARS="off",
               THRUM_RECONSTRUCTION="off", THRUM_ROUTE_OVERLAY="off",
               THRUM_MESH_STAGE_VALLEY_STORE=str(store))
    command = [
        "cargo", "test", "--release", "--locked", "--offline", "-p", "v1-game",
        "--features", "draw-probe", "--test", "mesh_stage_gpu",
        "valley::cost::route_cost", "--", "--exact", "--ignored", "--nocapture",
    ]
    subprocess.run(command[:command.index("valley::cost::route_cost")] + ["--no-run"],
                   cwd=ROOT, env=env, check=True)
    record = {
        "commit": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        "target": "mac-metal",
        "mode": "release; 60 Hz fixed world route with capture off; every route tick submitted",
        "frame_cpu_scope": "Game frame upload plus encode through queue submission. Query readback and replay are excluded.",
        "timed_pass_span_scope": "First timed pass begin to surface consumer end, including inter-pass time.",
        "outside_timed_pass_span": {
            "both": "Queued view, instance, light and Sun map uniform writes before the first pass.",
            "indexed": "Per-placement indexed cull uniform writes.",
            "mesh": "Placement stage uniform writes when its history flag changes.",
        },
        "counts_scope": "Separate fixed route replay with exact recorded view and caster matrices. "
                        "CPU classifies cooked meshlets and sums their triangles by placement and first rejection reason. "
                        "Each pair gives meshlets and triangles. Forward compute and object culls share the rule; "
                        "indexed casters submit whole meshes. Counts and their readback are outside the timing runs.",
        "extents": [],
    }
    for width in (1280, 1920):
        rows = [run(command, root, env, arm, width, repeat)
                for repeat, order in enumerate((("indexed", "mesh"), ("mesh", "indexed")))
                for arm in order]
        counts = run(command, root, env, "counts", width, 0)
        comparison = compare(rows)
        print(json.dumps({"width": width, "runs": [dict(arm=row["arm"], **row["summary"]) for row in rows],
                          "comparison": comparison}), flush=True)
        record["extents"].append(dict(width=width, timings=rows, accounting=counts, comparison=comparison))
    with gzip.open(root / "valley-cost.json.gz", "wt", encoding="utf-8") as stream:
        json.dump(record, stream, separators=(",", ":"))


if __name__ == "__main__":
    main()
