#!/usr/bin/env python3
"""Capture two live cooked-row edits in the moving valley through T2 realtime capture.

Build first, then run this command under agent_loop.py lock gpu. All source edits
are in the new output directory. The engine capture remains the ordinary T2 path.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]


def run(args):
    output = args.output.resolve()
    output.relative_to(ROOT / "tmp")
    output.mkdir(parents=True, exist_ok=False)
    source, store = output / "source", output / "store"
    shutil.copytree(ROOT / "examples/v1-game/assets/showcase", source)
    cooker = args.cooker.resolve()
    def cook():
        result = subprocess.run([str(cooker), "cook", str(source), str(store)], capture_output=True, text=True, check=True)
        return {"head": (store / "HEAD").read_bytes().hex(), "output": result.stdout}
    first = cook()
    gltf_path = source / "rock.gltf"
    original = json.loads(gltf_path.read_text())
    accessor = original["accessors"][original["meshes"][0]["primitives"][0]["attributes"]["POSITION"]]
    view = original["bufferViews"][accessor["bufferView"]]
    assert accessor["type"] == "VEC3" and accessor["componentType"] == 5126
    bin_path = source / original["buffers"][view["buffer"]]["uri"]
    original_bin = bin_path.read_bytes()
    start = view.get("byteOffset", 0) + accessor.get("byteOffset", 0)
    environment = dict(os.environ, THRUM_VALLEY_MESHES="on", THRUM_ROUTE_OVERLAY="proof")
    command = [sys.executable, str(ROOT / "scripts/capture_v1_runtime.py"), "--binary", str(args.binary.resolve()), "--store", str(store), "--output", str(output / "capture"), "--case", "pan", "--mode", "realtime", "--admission", "omit"]
    events = []
    with (output / "driver.log").open("w") as log:
        capture = subprocess.Popen(command, cwd=ROOT, env=environment, stdout=log, stderr=subprocess.STDOUT)
        try:
            deadline = time.monotonic() + 75
            ready = output / "capture/measurement-ready.txt"
            while not ready.exists():
                if capture.poll() is not None or time.monotonic() >= deadline:
                    raise RuntimeError("capture did not reach its measurement interval")
                time.sleep(0.01)
            origin = time.monotonic_ns()
            for at, scale in [(0.8, 2.0), (2.4, 0.4)]:
                while (time.monotonic_ns() - origin) / 1e9 < at:
                    time.sleep(0.005)
                gltf = json.loads(json.dumps(original))
                changed_accessor = gltf["accessors"][gltf["meshes"][0]["primitives"][0]["attributes"]["POSITION"]]
                changed = bytearray(original_bin)
                for index in range(accessor["count"]):
                    offset = start + index * view.get("byteStride", 12) + 4
                    struct.pack_into("<f", changed, offset, struct.unpack_from("<f", original_bin, offset)[0] * scale)
                for key in ("min", "max"):
                    changed_accessor[key][1] *= scale
                begin = time.monotonic_ns()
                bin_path.write_bytes(changed)
                gltf_path.write_text(json.dumps(gltf))
                publication = cook()
                events.append({"height_scale": scale, "save_ns_after_ready": begin - origin, "published_ns_after_ready": time.monotonic_ns() - origin, "binary_sha256": hashlib.sha256(changed).hexdigest(), **publication})
            if capture.wait(timeout=90):
                raise RuntimeError("T2 capture failed; see driver.log")
        finally:
            if capture.poll() is None:
                capture.terminate()
                capture.wait(timeout=15)
    record = {"commit": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(), "target": "mac-metal", "mode": "realtime", "camera": "T2 pan", "initial": first, "edits": events, "clock_limit": "edit times are relative to measurement-ready, immediately before the prime submission; inspect the submitted frame sequence for the switch", "command": command}
    (output / "edits.json").write_text(json.dumps(record, indent=2) + "\n")
    print(json.dumps(record, indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--binary", type=Path, default=Path("target/release/v1-game"))
    parser.add_argument("--cooker", type=Path, default=Path("target/debug/thrum-cook"))
    run(parser.parse_args())
