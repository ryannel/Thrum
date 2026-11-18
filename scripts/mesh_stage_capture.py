#!/usr/bin/env python3
"""Capture matched Apple tile fixed stops and realtime valley runs through T2."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

from mesh_stage_proof import ROOT, cook_valley


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--fixed-only", action="store_true")
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    store = output / "store"
    cook_valley(store)
    env = dict(os.environ, THRUM_REQUIRE_GPU="1", THRUM_SCENE="valley", THRUM_STARS="off",
               THRUM_RECONSTRUCTION="off", THRUM_ROUTE_OVERLAY="off",
               THRUM_MESH_STAGE_VALLEY_STORE=str(store))
    fixed = output / "fixed"
    fixed.mkdir()
    subprocess.run([
        "cargo", "test", "--release", "--locked", "--offline", "-p", "v1-game",
        "--features", "draw-probe", "--test", "mesh_stage_gpu",
        "valley::fixed_stops", "--", "--exact", "--ignored", "--nocapture",
    ], cwd=ROOT, env=dict(env, THRUM_MESH_STAGE_PROOF_DIR=str(fixed)), check=True)
    report = json.loads((fixed / "fixed-stops.json").read_text())
    for frame in report["frames"]:
        raw = fixed / frame["file"]
        frame["rgba_sha256"] = hashlib.sha256(raw.read_bytes()).hexdigest()
        png = raw.with_suffix(".png")
        subprocess.run(["ffmpeg", "-v", "error", "-f", "rawvideo", "-pixel_format", "rgba",
                        "-video_size", "1920x1080", "-i", str(raw), "-frames:v", "1", str(png)],
                       check=True)
        raw.unlink()
        frame["file"] = png.name
    report["commit"] = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    (fixed / "fixed-stops.json").write_text(json.dumps(report, indent=2) + "\n")
    if args.fixed_only:
        return
    subprocess.run(["cargo", "build", "--release", "--locked", "--offline", "-p", "v1-game",
                    "--features", "draw-probe", "--bin", "v1-game"], cwd=ROOT, env=env, check=True)
    route = json.loads((ROOT / "examples/v1-game/routes/moving-v1.json").read_text())
    route["overlay"] = "off"
    route["render_extent"] = [1280, 720]
    route_file = output / "route.json"
    route_file.write_text(json.dumps(route, indent=2) + "\n")
    for arm in ("mesh", "indexed"):
        arm_env = dict(env)
        arm_env.pop("THRUM_VALLEY_MESH_STAGE_PROBE", None)
        arm_env.pop("THRUM_HARDWARE", None)
        if arm == "mesh":
            arm_env.update(THRUM_HARDWARE="mesh-stages", THRUM_VALLEY_MESH_STAGE_PROBE="culled")
        destination = output / f"realtime-{arm}"
        subprocess.run([
            sys.executable, str(ROOT / "scripts/capture_v1_runtime.py"), "--binary",
            str(ROOT / "target/release/v1-game"), "--store", str(store),
            "--output", str(destination), "--case", "route", "--route", str(route_file),
            "--mode", "realtime", "--recipe", "apple-tile", "--extent", "1280", "720",
            "--admission", "wait",
        ], cwd=ROOT, env=arm_env, check=True)
        path = json.loads((destination / "cooked-path.json").read_text())
        expected = "MeshStages" if arm == "mesh" else "Indirect"
        if path["resolved_path"] != f"Some({expected})" or path["recipe"] != "Some(AppleTile)":
            raise ValueError(f"unexpected resolved path: {path}")


if __name__ == "__main__":
    main()
