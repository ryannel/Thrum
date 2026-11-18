#!/usr/bin/env python3
"""Metal proofs for cooked ray geometry and fixed traced/raster valley pairs."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
from tempfile import TemporaryDirectory


ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / "docs/evidence/RAY-TRACING/M2"
VIEWS = (
    ("valley", 0, 0),
    ("contact", 0, -13),
    ("west", -12, -8),
    ("east", 13, -8),
)


def command(*args):
    return subprocess.check_output(args, cwd=ROOT, text=True).strip()


def run_test(case, *, release=False, extra_env=None):
    args = ["rustup", "run", "stable", "cargo", "test", "--locked", "--offline"]
    if release:
        args.append("--release")
    args.extend(["-p", "v1-game", "--features", "draw-probe", "--test", "ray_tracing_gpu",
                 "--", f"ray_tracing::{case}", "--exact", "--nocapture"])
    if release:
        args.append("--ignored")
    result = subprocess.run(
        args, cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
        env=dict(os.environ, THRUM_REQUIRE_GPU="1", **(extra_env or {})),
    )
    if result.returncode:
        print(result.stdout, end="")
        result.check_returncode()
    rows = [json.loads(line.split("ray proof ", 1)[1]) for line in result.stdout.splitlines()
            if "ray proof " in line]
    if not rows or "1 passed" not in result.stdout:
        raise ValueError(f"{case} did not run and record evidence on Metal: {result.stdout}")
    return rows


def proof(case):
    reference = None
    if case == "demand":
        scratch = ROOT / "tmp/ray-tracing-m2"
        scratch.mkdir(parents=True, exist_ok=True)
        with TemporaryDirectory(prefix="demand-", dir=scratch) as directory:
            pixels = Path(directory) / "ordinary.rgba"
            rows = run_test(case, extra_env={"THRUM_RAY_DEMAND_BYTES": str(pixels)})
            sha256 = hashlib.sha256(pixels.read_bytes()).hexdigest()
        reference = json.loads((EVIDENCE / "demand-main.json").read_text())
        if sha256 != reference["rgba_sha256"]:
            raise ValueError(f"no-demand bytes differ from {reference['revision']}: {sha256}")
        rows[0]["rgba_sha256"] = sha256
    else:
        rows = run_test(case)
    result = {"revision": command("git", "rev-parse", "HEAD"), "target": "mac-metal",
              "proof": case, "observations": rows}
    if reference:
        result["main_reference"] = reference
    return result


def capture(output, binary, store):
    output.mkdir(parents=True, exist_ok=True)
    report = []
    for view, yaw, pitch in VIEWS:
        params = output / f"{view}.params.json"
        params.write_text(json.dumps({
            "world.paused": True, "sky.solar_time": 12, "camera.yaw": yaw,
            "camera.pitch": pitch,
        }) + "\n")
        pair = {"view": view, "yaw_degrees": yaw, "pitch_degrees": pitch, "frames": {}}
        for mode in ("drawn", "traced"):
            name = f"{view}-{mode}"
            directory = output / name
            env = dict(os.environ, THRUM_HARDWARE="ray-query", THRUM_RECIPE="apple-tile",
                       THRUM_TRACED_VIEW="on" if mode == "traced" else "off",
                       THRUM_ROUTE_OVERLAY="proof", THRUM_VALLEY_MESHES="on")
            subprocess.run(
                ["python3", "scripts/capture_v1_runtime.py", "--binary", str(binary),
                 "--store", str(store), "--output", str(directory), "--params", str(params),
                 "--case", "checkpoint", "--mode", "fixed", "--extent", "1280", "720"],
                cwd=ROOT, env=env, check=True, stdout=subprocess.DEVNULL,
            )
            manifest = json.loads((directory / "manifest.json").read_text())
            runtime = json.loads((directory / "runtime.json").read_text())
            if not (manifest["complete"] and runtime["mode"] == "fixed" and
                    "EXPERIMENTAL_RAY_QUERY" in runtime["enabled_features"] and
                    runtime["backend"] == "Metal" and runtime["recipe"] == "Ok(AppleTile)"):
                raise ValueError(f"incomplete or wrong capture device: {directory}")
            record, = manifest["records"]
            source = directory / "png" / f"{record['submission_id']:06}.png"
            shutil.copyfile(source, output / f"{name}.png")
            pair["frames"][mode] = {
                "file": f"{name}.png", **record, "adapter": runtime["adapter"],
                "backend": runtime["backend"], "recipe": runtime["recipe"],
                "feature": "ray-query required", "binary_sha256": manifest["binary_sha256"],
                "cooked_products_sha256": manifest["cooked_hashes"],
            }
        if pair["frames"]["drawn"]["rgba_sha256"] == pair["frames"]["traced"]["rgba_sha256"]:
            raise ValueError(f"the traced display selection did not take effect for {view}")
        report.append(pair)
    return {"revision": command("git", "rev-parse", "HEAD"), "mode": "fixed",
            "extent": [1280, 720], "views": report}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("case", choices=["edit", "casters", "lifecycle", "demand", "stills"])
    parser.add_argument("--output", type=Path)
    parser.add_argument("--binary", type=Path, default=Path("target/debug/v1-game"))
    parser.add_argument("--store", type=Path)
    args = parser.parse_args()
    if args.case == "stills":
        if args.output is None or args.store is None:
            parser.error("stills requires a T2 output directory and the cooked valley store")
        result = capture(args.output, args.binary, args.store)
        destination = args.output / "capture.json"
    else:
        result = proof(args.case)
        destination = args.output
    text = json.dumps(result, indent=2) + "\n"
    if destination:
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(text)
    print(text, end="")


if __name__ == "__main__":
    main()
