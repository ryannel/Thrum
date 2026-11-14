#!/usr/bin/env python3
"""Retain straight corrected traces explicitly, then compare independent target runs."""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
SCRATCH = ROOT / "tmp/sim2d"
RETAINED = ROOT / "examples/crowd-2d/retained/crowd.txt"
VECTORS = ROOT / "crates/thrum-core/src/profile/vectors.json"
SIZES = [(2000, 3600), (5000, 3600), (10000, 36000)]


def command(args):
    return subprocess.check_output([str(arg) for arg in args], cwd=ROOT, text=True)


def identity():
    return {"commit": command(["git", "rev-parse", "HEAD"]).strip(),
            "dirty": bool(command(["git", "status", "--porcelain"])),
            "utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
            "machine": platform.platform(), "load": os.getloadavg(),
            "rustc": command(["rustup", "run", "1.99.1", "rustc", "--version"]).strip()}


def source_key():
    """Identity of the measured rules, runtime, codec, runner and build settings."""
    files = [ROOT / name for name in ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "scripts/sim2d_conformance.py", "scripts/sim2d_browser.mjs"]]
    for directory in ["crates/thrum-core", "crates/thrum-sim2d", "crates/thrum-network", "examples/crowd-2d", "examples/crowd-2d-run", ".cargo"]:
        files.extend(path for path in (ROOT / directory).rglob("*") if path.is_file())
    digest = hashlib.sha256()
    for path in sorted(files):
        digest.update(str(path.relative_to(ROOT)).encode())
        digest.update(hashlib.sha256(path.read_bytes()).digest())
    return digest.hexdigest()


def build(target, debug=False):
    args = ["rustup", "run", "1.99.1", "cargo", "build", "--locked", "--offline", "-p", "crowd-2d-run"]
    if not debug:
        args.append("--release")
    triple = {"ios-sim": "aarch64-apple-ios-sim", "chrome-webgpu": "wasm32-unknown-unknown"}.get(target)
    if triple:
        args += ["--target", triple]
    environment = dict(os.environ, RUSTC=command(["rustup", "which", "--toolchain", "1.99.1", "rustc"]).strip())
    environment.setdefault("RUSTC_WRAPPER", "")
    subprocess.run(args, cwd=ROOT, env=environment, check=True)
    profile = "debug" if debug else "release"
    if target == "chrome-webgpu":
        web = SCRATCH / "web"
        web.mkdir(parents=True, exist_ok=True)
        wasm = ROOT / "target" / triple / profile / "crowd_2d_run.wasm"
        subprocess.run([str(Path.home() / ".cargo/bin/wasm-bindgen"), "--target", "web", "--out-dir", str(web), str(wasm)], check=True)
        return web, wasm
    binary = ROOT / "target" / (triple or "") / profile / "crowd-2d-run"
    return binary, binary


def simulator():
    devices = json.loads(command(["xcrun", "simctl", "list", "devices", "available", "-j"]))
    matches = [device for group in devices["devices"].values() for device in group if device["name"] == "iPhone 18 Pro"]
    if not matches:
        raise RuntimeError("iPhone 18 Pro simulator is unavailable")
    device = matches[0]
    if device["state"] != "Booted":
        subprocess.run(["xcrun", "simctl", "boot", device["udid"]], check=True)
        subprocess.run(["xcrun", "simctl", "bootstatus", device["udid"], "-b"], check=True)
    return device["udid"]


class Runner:
    def __init__(self, target, debug=False):
        self.target = target
        self.program, artifact = build(target, debug)
        self.artifact_hash = hashlib.sha256(artifact.read_bytes()).hexdigest()
        self.device = simulator() if target == "ios-sim" else None
        self.user_agent = None
        self.isolated = None

    def invoke(self, method, args):
        if self.target == "chrome-webgpu":
            packet = json.loads(command(["node", "scripts/sim2d_browser.mjs", self.program, json.dumps({"method": method, "args": args})]))
            self.user_agent = packet["userAgent"]
            self.isolated = packet["crossOriginIsolated"]
            return packet["result"]
        if method == "vectors":
            arguments = ["vectors"]
        elif method == "costs":
            arguments = [args[0], "costs"]
        else:
            count, ticks, straight, wire, control = args
            arguments = [count, ticks]
            arguments += [flag for flag, enabled in [("--straight", straight), ("--wire", wire), ("--control", control)] if enabled]
        prefix = ["xcrun", "simctl", "spawn", self.device] if self.device else []
        return json.loads(command(prefix + [self.program] + arguments))


def references():
    result = {}
    for line in RETAINED.read_text().splitlines():
        if line.startswith("#") or not line:
            continue
        kind, count, tick, value = line.split()
        entry = result.setdefault(int(count), {"checkpoints": [], "slice": []})
        if kind == "fold":
            entry.update(ticks=int(tick), fold=value)
        elif kind == "checkpoint":
            entry["checkpoints"].append([int(tick), value])
        elif kind == "slice":
            entry["slice"].append(value)
        else:
            raise ValueError(f"Unknown retained row {kind}")
    if set(result) != {count for count, _ in SIZES}:
        raise RuntimeError("Retained crowd sizes are incomplete")
    return result


def compare(row, retained):
    expected = retained[row["bodies"]]
    assert row["ticks"] == expected["ticks"], "trace length"
    assert row["fold"] == expected["fold"], f"timeline fold differs at {row['bodies']}"
    assert row["checkpoints"] == expected["checkpoints"], "checkpoint digest differs"
    for field in ["reused", "hits", "rewinds", "changed_rewinds"]:
        assert row[field] > 0, f"missing {field} witness"
    if row["wire"]:
        assert row["changed_axes"] > 0, "wire route did not change any axis"


def retain():
    runner = Runner("mac-metal")
    rows = []
    for count, ticks in SIZES:
        row = runner.invoke("conformance", [count, ticks, True, False, False])
        assert row["rewinds"] == 0 and row["hits"] > 0 and row["reused"] > 0
        rows.append(row)
    lines = ["# Straight Mac run with all corrected inputs present from the start.", "# " + json.dumps(identity(), sort_keys=True)]
    for row in rows:
        count = row["bodies"]
        lines.append(f"fold {count} {row['ticks']} {row['fold']}")
        lines.extend(f"checkpoint {count} {tick} {digest}" for tick, digest in row["checkpoints"])
        if count == 2000:
            lines.extend(f"slice 2000 {tick} {digest}" for tick, digest in enumerate(row["timeline"][:240], 1))
    RETAINED.write_text("\n".join(lines) + "\n")
    VECTORS.parent.mkdir(parents=True, exist_ok=True)
    VECTORS.write_text(json.dumps(runner.invoke("vectors", [])) + "\n")
    return {"retained": str(RETAINED.relative_to(ROOT)), "folds": {row["bodies"]: row["fold"] for row in rows}}


def run(target):
    retained = references()
    runner = Runner(target)
    assert runner.invoke("vectors", []) == json.loads(VECTORS.read_text()), "profile vectors differ"
    rows = []
    for count, ticks in SIZES:
        row = runner.invoke("conformance", [count, ticks, False, False, False])
        compare(row, retained)
        row.pop("timeline")
        rows.append(row)
    wire = runner.invoke("conformance", [2000, 3600, False, True, False])
    compare(wire, retained)
    wire.pop("timeline")
    rows.append(wire)
    if target == "mac-metal":
        debug = Runner(target, debug=True)
        row = debug.invoke("conformance", [2000, 3600, False, False, False])
        compare(row, retained)
        row.pop("timeline")
        row["build"] = "debug"
        rows.append(row)
    return {"rows": rows, "artifact_sha256": runner.artifact_hash, "simulator": runner.device, "user_agent": runner.user_agent, "profile_vectors": "pass"}


def control(target):
    if target != "chrome-webgpu":
        raise ValueError("The discriminating control target is Chrome")
    mac = Runner("mac-metal")
    chrome = Runner(target)
    a = mac.invoke("conformance", [2000, 240, False, False, True])
    b = chrome.invoke("conformance", [2000, 240, False, False, True])
    assert a["fold"] != b["fold"], "platform sine control stopped discriminating"
    first = next(index + 1 for index, pair in enumerate(zip(a["timeline"], b["timeline"])) if pair[0] != pair[1])
    return {"mac_fold": a["fold"], "chrome_fold": b["fold"], "first_differing_tick": first, "user_agent": chrome.user_agent}


def measure(target):
    if target == "ios-sim":
        raise ValueError("Simulator timing is not a qualified cost target")
    runner = Runner(target)
    runs = []
    for repeat in range(2):
        rows = [runner.invoke("costs", [count]) for count, _ in SIZES]
        for row in rows:
            expected_ticks = dict(SIZES)[row["bodies"]]
            assert row["ticks"] == expected_ticks and row["samples"] == expected_ticks - 120 and row["neighbours_per_tick"] > 0
            for key in ["tick_median_ms", "index_median_ms", "digest_median_ms", "rewind_eight_ms", "replay_eight_ms"]:
                assert isinstance(row[key], (float, int)) and row[key] >= 0, f"missing timing {key}"
            prior = next((old for run in runs for old in run["rows"] if old["bodies"] == row["bodies"]), None)
            row["quarter_slower_tripwire"] = bool(prior and row["tick_median_ms"] > 1.25 * prior["tick_median_ms"] and abs(row["neighbours_per_tick"] / prior["neighbours_per_tick"] - 1) < 0.01)
        for previous, row in zip(rows, rows[1:]):
            row["super_neighbour_scaling_tripwire"] = row["tick_ms_per_neighbour"] > previous["tick_ms_per_neighbour"]
        rows[0]["super_neighbour_scaling_tripwire"] = False
        runs.append({"repeat": repeat + 1, "load": os.getloadavg(), "build": "release", "rows": rows})
    return {"runs": runs, "artifact_sha256": runner.artifact_hash, "user_agent": runner.user_agent, "cross_origin_isolated": runner.isolated, "source_sha256": source_key(), "timings_are_baselines": True}


def costs(target):
    """Check the retained device measurements, independently of their time values."""
    path = ROOT / f"docs/evidence/SIM-2D/M1/costs-{target}.json"
    record = json.loads(path.read_text())
    assert record["target"] == target and record["dirty"] is False
    assert record["source_sha256"] == source_key(), "cost record belongs to different simulation sources"
    assert record["rustc"] == identity()["rustc"], "cost record belongs to another compiler"
    assert len(record["runs"]) == 2
    for run in record["runs"]:
        assert run["build"] == "release" and len(run["load"]) == 3
        assert [(row["bodies"], row["ticks"]) for row in run["rows"]] == SIZES
        for row in run["rows"]:
            assert row["samples"] == row["ticks"] - 120 and row["neighbours_per_tick"] > 0
            for key in ["tick_median_ms", "index_median_ms", "digest_median_ms", "rewind_eight_ms", "replay_eight_ms"]:
                assert isinstance(row[key], (float, int)) and row[key] >= 0
            for key in ["clock_tripwire", "quarter_slower_tripwire", "super_neighbour_scaling_tripwire"]:
                assert isinstance(row[key], bool)
    return {"record": str(path.relative_to(ROOT)), "source_sha256": record["source_sha256"], "measured_commit": record["commit"], "timings_are_baselines": True}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["retain", "run", "control", "measure", "costs"])
    parser.add_argument("--target", choices=["mac-metal", "chrome-webgpu", "ios-sim"], default="mac-metal")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    SCRATCH.mkdir(parents=True, exist_ok=True)
    record = {"action": args.action, "target": args.target, **identity()}
    result = retain() if args.action == "retain" else globals()[args.action](args.target)
    record.update(result)
    destination = args.output or SCRATCH / f"{args.action}-{args.target}.json"
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(json.dumps(record, indent=2) + "\n")
    print(json.dumps({"result": "pass", "record": str(destination.relative_to(ROOT) if destination.is_relative_to(ROOT) else destination)}))


if __name__ == "__main__":
    main()
