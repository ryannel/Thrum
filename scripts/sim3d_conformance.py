#!/usr/bin/env python3
"""Retain the Mac release collision vectors and compare the three target builds."""

import argparse
import hashlib
import itertools
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

from sim2d_conformance import ROOT, command, device_command, identity
import simulator_lifecycle

SCRATCH = ROOT / "tmp/sim3d"
FIXTURES = ROOT / "examples/sim3d-run/tests/fixtures"
INPUTS = FIXTURES / "pair-inputs.txt"
RETAINED = FIXTURES / "pair-vectors.txt"
CARGO = ["rustup", "run", "stable", "cargo"]
TRIPLES = {"chrome-webgpu": "wasm32-unknown-unknown", "ios-sim": "aarch64-apple-ios-sim"}
TARGETS = ("mac-metal", *TRIPLES)


def cargo_environment():
    env = dict(os.environ, RUSTC=command(["rustup", "which", "--toolchain", "stable", "rustc"]).strip())
    env.setdefault("RUSTC_WRAPPER", "")
    return env


def render_hand_worked_inputs():
    log = subprocess.check_output(
        [*CARGO, "test", "--locked", "--offline", "-p", "thrum-sim3d", "--test", "it",
         "conformance::hand_worked_inputs", "--", "--exact", "--nocapture"],
        cwd=ROOT, text=True, env=cargo_environment())
    rows = [line.removeprefix("input ") for line in log.splitlines() if line.startswith("input ")]
    if not rows or len({row.split(" ", 1)[0] for row in rows}) != len(rows):
        raise ValueError("hand-worked test input IDs must be nonempty and unique")
    return "\n".join(rows) + "\n"


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def build(target, profile):
    args = [*CARGO, "build", "--locked", "--offline", "-p", "sim3d-run"]
    if profile == "release":
        args.append("--release")
    triple = TRIPLES.get(target)
    if triple:
        args += ["--target", triple]
    subprocess.run(args, cwd=ROOT, env=cargo_environment(), check=True)
    target_directory = ROOT / os.environ.get("CARGO_TARGET_DIR", "target")
    directory = target_directory / (triple or "") / profile
    return directory / ("sim3d_run.wasm" if target == "chrome-webgpu" else "sim3d-run")


def execute(target, artifact, scratch):
    if target == "mac-metal":
        return command([artifact]), {}
    if target == "ios-sim":
        lifecycle = scratch / "simulator.jsonl"
        rows = simulator_lifecycle.run_case([artifact], record=lifecycle)
        events = [json.loads(line) for line in lifecycle.read_text().splitlines()]
        return rows, {"lifecycle": events, "scope": "function"}
    # The reused browser driver makes its profile directory under tmp/sim2d.
    (ROOT / "tmp/sim2d").mkdir(parents=True, exist_ok=True)
    binding = Path.home() / ".cargo/bin/wasm-bindgen"
    subprocess.run([binding, "--target", "web", "--out-dir", scratch, artifact], cwd=ROOT, check=True)
    request = json.dumps({"method": "vectors", "args": []})
    result = json.loads(device_command(
        "chrome", ["node", "scripts/sim2d_browser.mjs", scratch, request, "sim3d_run"]))
    return result["result"], {"user_agent": result["userAgent"],
                              "cross_origin_isolated": result["crossOriginIsolated"],
                              "wasm_bindgen": command([binding, "--version"]).strip(),
                              "served_wasm_sha256": digest(scratch / "sim3d_run_bg.wasm")}


def compare(expected, actual):
    for number, (want, got) in enumerate(
            itertools.zip_longest(expected.splitlines(), actual.splitlines()), start=1):
        if want != got:
            vector = (want or got or "<empty>").split(" ", 1)[0]
            raise ValueError(f"vector {vector} differs at row {number}: retained {want!r}, got {got!r}")


def retain():
    INPUTS.write_text(render_hand_worked_inputs())
    artifact = build("mac-metal", "release")
    retained = command([artifact])
    if not retained:
        raise ValueError("Mac release returned no vectors")
    RETAINED.write_text(retained)
    return {"source": identity(), "inputs_sha256": digest(INPUTS), "artifact_sha256": digest(artifact),
            "reference_sha256": digest(RETAINED), "vectors": len(retained.splitlines())}


def vectors(target, reference):
    source = identity()
    expected = reference.read_text()
    if target == "mac-metal":
        compare(INPUTS.read_text(), render_hand_worked_inputs())
    profiles = ["release", "debug"] if target == "mac-metal" else ["release"]
    report = []
    SCRATCH.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=SCRATCH) as temp:
        for profile in profiles:
            artifact = build(target, profile)
            actual, details = execute(target, artifact, Path(temp))
            compare(expected, actual)
            report.append({"target": target, "profile": profile, "result": "pass",
                           "vector_count": len(actual.splitlines()), "artifact_sha256": digest(artifact),
                           "actual_sha256": hashlib.sha256(actual.encode()).hexdigest(), **details})
    return {"source": source, "inputs_sha256": digest(INPUTS),
            "reference_sha256": digest(reference), "runs": report}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("retain", help="regenerate the hand-worked packets and Mac release vectors")
    run = commands.add_parser("vectors", help="compare an actual target with the retained vectors")
    run.add_argument("--target", required=True, choices=TARGETS)
    run.add_argument("--reference", default=RETAINED, type=Path)
    args = parser.parse_args()
    try:
        report = retain() if args.command == "retain" else vectors(args.target, args.reference)
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(error, file=sys.stderr)
        return 1
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
