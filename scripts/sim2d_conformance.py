#!/usr/bin/env python3
"""Retain straight corrected traces explicitly, then compare independent target runs."""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import subprocess
import sys
import tomllib
from copy import deepcopy

ROOT = Path(__file__).resolve().parents[1]
SCRATCH = ROOT / "tmp/sim2d"
# Retained device measurements: the first take (prior-costs) and the one costs checks (costs).
PRIOR_COSTS = ROOT / "tests/fixtures/sim-2d"
RETAINED = ROOT / "examples/crowd-2d/retained/crowd.txt"
MIXED = ROOT / "examples/crowd-2d/retained/mixed.txt"
VECTORS = ROOT / "crates/thrum-core/src/profile/vectors.json"
SIZES = [(2000, 3600), (5000, 3600), (10000, 36000)]
PACKAGE = "crowd-2d"
RUNNER = "crowd-2d-run"


def registration(package=None):
    """Game-owned trace inputs and runner; engine tooling knows no game schema."""
    name = package or PACKAGE
    registry = tomllib.loads((ROOT / ".config/architecture.toml").read_text())["packages"]
    entry = registry.get(name, {})
    if not entry.get("rule_package") or "conformance" not in entry:
        raise ValueError(f"{name}: not a registered rule package with conformance inputs")
    config = entry["conformance"]
    runner = config["runner"]
    if runner not in registry or name not in registry[runner]["allowed_workspace_dependencies"]:
        raise ValueError(f"{name}: runner must be registered and depend on the rule package")
    return config


def command(args):
    return subprocess.check_output([str(arg) for arg in args], cwd=ROOT, text=True)


def device_command(resource, args):
    """Lock each device invocation, never the whole multi-trace cost campaign."""
    script = os.environ.get("ORCHESTRATE_SCRIPT")
    if not script:
        raise RuntimeError("Set ORCHESTRATE_SCRIPT to agent_loop.py; target invocations take their own device locks")
    return command([sys.executable, script, "lock", resource, "--", *args])


def identity():
    return {"commit": command(["git", "rev-parse", "HEAD"]).strip(),
            "dirty": bool(command(["git", "status", "--porcelain"])),
            "utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
            "machine": platform.platform(), "load": os.getloadavg(),
            "rustc": command(["rustup", "run", "stable", "rustc", "--version"]).strip()}


def build_settings(manifest, lock, packages):
    """Keep shared settings and locked packages that can affect the runner build."""
    manifest = deepcopy(manifest)
    for key in ["members", "default-members", "exclude"]:
        manifest["workspace"].pop(key, None)
    names = {name for name, _, _ in packages}
    for profile in manifest.get("profile", {}).values():
        if "package" in profile:
            profile["package"] = {name: value for name, value in profile["package"].items()
                                  if name in names or not all(c.isalnum() or c in "_-" for c in name)}
            if not profile["package"]:
                del profile["package"]
    if "profile" in manifest:
        manifest["profile"] = {name: value for name, value in manifest["profile"].items() if value}
        if not manifest["profile"]:
            del manifest["profile"]
    selected = [{key: value for key, value in entry.items() if key != "dependencies"} for entry in lock["package"]
                if (entry["name"], entry["version"], entry.get("source")) in packages]
    return {"manifest": manifest, "lock_version": lock["version"], "packages": selected}


def runner_graph(tree, packages):
    """Read Cargo's package-scoped tree, preserving active features and edges."""
    selected, nodes, edges, parents = set(), set(), set(), []
    for line in tree.splitlines():
        depth, package, features = line.split("|", 2)
        depth = int(depth)
        name, version = package.split()[:2]
        matches = [p for p in packages.values() if p["name"] == name and p["version"] == version.removeprefix("v")]
        if len(matches) != 1:
            raise ValueError(f"ambiguous Cargo tree package: {package}")
        package = matches[0]
        selected.add(package["id"])
        node = json.dumps({"name": name, "version": package["version"], "source": package["source"],
                           "features": sorted(filter(None, features.split(",")))}, sort_keys=True)
        nodes.add(node)
        if depth:
            edges.add((parents[depth - 1], node))
        parents[depth:] = [node]
    if not selected:
        raise ValueError("Cargo returned no runner dependencies")
    return selected, {"nodes": sorted(nodes), "edges": sorted(edges)}


def source_inputs():
    """Portable, inspectable inputs of the package-scoped runner build."""
    metadata = json.loads(command(["rustup", "run", "stable", "cargo", "metadata", "--locked", "--offline", "--format-version", "1"]))
    packages = {p["id"]: p for p in metadata["packages"]}
    tree = command(["rustup", "run", "stable", "cargo", "tree", "--locked", "--offline", "-p", RUNNER,
                    "--target", "all", "--edges", "normal,build", "--prefix", "depth", "--no-dedupe", "--format", "|{p}|{f}"])
    selected, graph = runner_graph(tree, packages)
    identities = {(packages[p]["name"], packages[p]["version"], packages[p]["source"]) for p in selected}
    settings = build_settings(tomllib.loads((ROOT / "Cargo.toml").read_text()),
                              tomllib.loads((ROOT / "Cargo.lock").read_text()), identities)
    files = [ROOT / name for name in ["rust-toolchain.toml", "scripts/sim2d_conformance.py", "scripts/sim2d_browser.mjs"]]
    directories = [ROOT / ".cargo"] + [Path(packages[p]["manifest_path"]).parent
                                       for p in selected if packages[p]["source"] is None]
    for directory in directories:
        files.extend(path for path in directory.rglob("*") if path.is_file())
    return {"graph": graph, "settings": settings,
            "files": {str(path.relative_to(ROOT)): hashlib.sha256(path.read_bytes()).hexdigest()
                      for path in sorted(files)}}


def source_key(inputs=None):
    """Identity of the runner's normal/build dependency closure, on all targets."""
    return hashlib.sha256(json.dumps(source_inputs() if inputs is None else inputs, sort_keys=True).encode()).hexdigest()


def build(target, debug=False):
    args = ["rustup", "run", "stable", "cargo", "build", "--locked", "--offline", "-p", RUNNER]
    if not debug:
        args.append("--release")
    triple = {"ios-sim": "aarch64-apple-ios-sim", "chrome-webgpu": "wasm32-unknown-unknown"}.get(target)
    if triple:
        args += ["--target", triple]
    environment = dict(os.environ, RUSTC=command(["rustup", "which", "--toolchain", "stable", "rustc"]).strip())
    environment.setdefault("RUSTC_WRAPPER", "")
    subprocess.run(args, cwd=ROOT, env=environment, check=True)
    profile = "debug" if debug else "release"
    if target == "chrome-webgpu":
        web = SCRATCH / "web"
        web.mkdir(parents=True, exist_ok=True)
        wasm = ROOT / "target" / triple / profile / (RUNNER.replace("-", "_") + ".wasm")
        subprocess.run([str(Path.home() / ".cargo/bin/wasm-bindgen"), "--target", "web", "--out-dir", str(web), str(wasm)], check=True)
        return web, wasm
    binary = ROOT / "target" / (triple or "") / profile / RUNNER
    return binary, binary


def simulator():
    devices = json.loads(command(["xcrun", "simctl", "list", "devices", "available", "-j"]))
    matches = [device for group in devices["devices"].values() for device in group if device["name"] == "iPhone 18 Pro"]
    if not matches:
        raise RuntimeError("iPhone 18 Pro simulator is unavailable")
    device = matches[0]
    if device["state"] != "Booted":
        device_command("simulator", ["xcrun", "simctl", "boot", device["udid"]])
        device_command("simulator", ["xcrun", "simctl", "bootstatus", device["udid"], "-b"])
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
            return self.invoke_many([(method, args)])[0]
        if method == "vectors":
            arguments = ["vectors"]
        elif method in ["costs", "measure_trace"]:
            arguments = [args[0], "costs"]
            if method == "measure_trace":
                arguments += [flag for flag, enabled in zip(["--mixed", "--parallel"], args[1:]) if enabled]
        else:
            count, ticks, straight, wire, control = args[:5]
            arguments = [count, ticks]
            arguments += [flag for flag, enabled in [("--straight", straight), ("--wire", wire), ("--control", control)] if enabled]
            if method == "trace":
                arguments += [flag for flag, enabled in zip(["--mixed", "--parallel"], args[5:]) if enabled]
        prefix = ["xcrun", "simctl", "spawn", self.device] if self.device else []
        invoke = prefix + [self.program] + arguments
        return json.loads(device_command("simulator", invoke) if self.device else command(invoke))

    def invoke_many(self, requests, omit_timeline=False):
        """Keep a Chrome conformance run in one queue turn, after compilation."""
        if self.target != "chrome-webgpu":
            return [self.invoke(method, args) for method, args in requests]
        packet = json.loads(device_command("chrome", ["node", "scripts/sim2d_browser.mjs", self.program,
                            json.dumps([{"method": method, "args": args, "omitTimeline": omit_timeline}
                                        for method, args in requests]), RUNNER.replace("-", "_")]))
        self.user_agent = packet["userAgent"]
        self.isolated = packet["crossOriginIsolated"]
        if len(packet["result"]) != len(requests):
            raise RuntimeError("Chrome returned an incomplete conformance batch")
        return packet["result"]


def references(path=RETAINED):
    result = {}
    for line in path.read_text().splitlines():
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
    for field in registration()["witnesses"]:
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


def retain_mixed():
    """Explicitly record only the new trace; Crowd is immutable after M1."""
    runner = Runner("mac-metal")
    rows = [runner.invoke("trace", [count, ticks, True, False, False, True, False]) for count, ticks in SIZES]
    lines = ["# Mixed: straight corrected Mac run, oversize enemies and swept projectiles.", "# " + json.dumps(identity(), sort_keys=True)]
    for row in rows:
        assert row["oversize"] > 0 and row["capsule_hits"] > 0
        count = row["bodies"]
        lines.append(f"fold {count} {row['ticks']} {row['fold']}")
        lines.extend(f"checkpoint {count} {tick} {digest}" for tick, digest in row["checkpoints"])
        if count == 2000:
            lines.extend(f"slice 2000 {tick} {digest}" for tick, digest in enumerate(row["timeline"][:240], 1))
    MIXED.write_text("\n".join(lines) + "\n")
    return {"retained": str(MIXED.relative_to(ROOT)), "folds": {row["bodies"]: row["fold"] for row in rows}}


def run(target, parallel=False):
    config = registration()
    measured_source = source_key()
    runner = Runner(target)
    requests = [("vectors", [])]
    for trace in config["traces"]:
        mixed = trace["variant"]
        requests.extend(("trace", [count, ticks, False, False, False, mixed, parallel]) for count, ticks in SIZES)
        requests.append(("trace", [*SIZES[0], False, True, False, mixed, parallel]))
    results = iter(runner.invoke_many(requests, omit_timeline=True))
    assert next(results) == json.loads((ROOT / config["vectors"]).read_text()), "profile vectors differ"
    rows = []
    for trace in config["traces"]:
        mixed = trace["variant"]
        retained = references(ROOT / trace["retained"])
        for count, ticks in SIZES:
            row = next(results)
            compare(row, retained)
            for field in trace["witnesses"]:
                assert row[field] > 0, f"missing {field} witness"
            if parallel:
                assert row["parallel_chunks"] > 0, "steering did not dispatch native chunks"
            row.pop("timeline", None)
            rows.append(row)
        wire = next(results)
        compare(wire, retained)
        wire.pop("timeline", None)
        rows.append(wire)
    if target == "mac-metal":
        debug = Runner(target, debug=True)
        for trace in config["traces"]:
            row = debug.invoke("trace", [*SIZES[0], False, False, False, trace["variant"], False])
            compare(row, references(ROOT / trace["retained"]))
            row.pop("timeline")
            row["build"] = "debug"
            rows.append(row)
    assert source_key() == measured_source, "simulation sources changed during conformance"
    return {"rows": rows, "source_sha256": measured_source, "artifact_sha256": runner.artifact_hash, "simulator": runner.device, "user_agent": runner.user_agent, "profile_vectors": "pass"}


def control(target):
    if target != "chrome-webgpu":
        raise ValueError("The discriminating control target is Chrome")
    mac = Runner("mac-metal")
    chrome = Runner(target)
    config = registration()["control"]
    request = [config["bodies"], config["ticks"], False, False, True]
    a = mac.invoke("conformance", request)
    b = chrome.invoke("conformance", request)
    assert a["fold"] != b["fold"], "platform sine control stopped discriminating"
    first = next(index + 1 for index, pair in enumerate(zip(a["timeline"], b["timeline"])) if pair[0] != pair[1])
    return {"mac_fold": a["fold"], "chrome_fold": b["fold"], "first_differing_tick": first, "user_agent": chrome.user_agent}


def record_trace(trace_name, destination):
    """Record a new package's straight timeline; never overwrite retained truth."""
    config = registration()
    trace = next((entry for entry in config["traces"] if entry["name"] == trace_name), None)
    if trace is None:
        raise ValueError(f"unknown trace {trace_name!r}")
    retained_paths = {ROOT / entry["retained"] for entry in config["traces"]}
    if destination.resolve() in {path.resolve() for path in retained_paths} or destination.exists():
        raise ValueError("record to a new candidate file, then review and promote it explicitly")
    runner = Runner("mac-metal")
    lines = ["# Straight corrected run: " + json.dumps(identity(), sort_keys=True)]
    for count, ticks in SIZES:
        row = runner.invoke("trace", [count, ticks, True, False, False, trace["variant"], False])
        assert row["rewinds"] == 0 and len(row["timeline"]) == ticks
        lines.append(f"fold {count} {ticks} {row['fold']}")
        lines.extend(f"checkpoint {count} {tick} {digest}" for tick, digest in row["checkpoints"])
        if count == SIZES[0][0]:
            lines.extend(f"slice {count} {tick} {digest}" for tick, digest in enumerate(row["timeline"][:240], 1))
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text("\n".join(lines) + "\n")
    return {"candidate": str(destination)}


def allocation_counts():
    """Count optimized warmed calls; no count is a pass/fail threshold."""
    args = ["rustup", "run", "stable", "cargo", "test", "--locked", "--offline", "--release",
            "-p", "thrum-sim2d", "--test", "allocations", "--", "--nocapture"]
    output = command(args)
    counts = {name: int(count) for name, count in re.findall(r"^(.+): (\d+) allocations per warmed call$", output, re.MULTILINE)}
    assert set(counts) == {"build", "circle", "box", "capsule", "wide circle", "nearest-k"}, "missing allocation observation"
    return {"bodies": 10000, "counts": counts, "command": args,
            "remaining_sources": "None in these warmed calls; first-use Vec growth is outside the warmed scope."}


def measure(target):
    measured_inputs = source_inputs()
    measured_source = source_key(measured_inputs)
    measured_commit = command(["git", "rev-parse", "HEAD"]).strip()
    if target == "ios-sim":
        raise ValueError("Simulator timing is not a qualified cost target")
    previous_path = PRIOR_COSTS / f"prior-costs-{target}.json"
    previous = load_cost_record(previous_path) if previous_path.exists() else None
    previous_hash = hashlib.sha256(previous_path.read_bytes()).hexdigest() if previous else None
    allocations = allocation_counts() if target == "mac-metal" else None
    runner = Runner(target)
    runs = []
    for repeat in range(2):
        rows = []
        for parallel in ([False, True] if target == "mac-metal" else [False]):
            for mixed in [False, True]:
                for count, _ in SIZES:
                    load = os.getloadavg()
                    utc = datetime.datetime.now(datetime.timezone.utc).isoformat()
                    row = runner.invoke("measure_trace", [count, mixed, parallel])
                    row.update(load_before=load, load_after=os.getloadavg(), utc=utc, build="release")
                    rows.append(row)
                    print(f"measured {target} repeat={repeat + 1} mixed={mixed} parallel={parallel} bodies={count}", file=sys.stderr, flush=True)
        for row in rows:
            expected_ticks = dict(SIZES)[row["bodies"]]
            assert row["ticks"] == expected_ticks and row["samples"] == expected_ticks - 120 and row["neighbours_per_tick"] > 0
            for key in ["tick_median_ms", "index_median_ms", "digest_median_ms", "rewind_eight_ms", "replay_eight_ms"]:
                assert isinstance(row[key], (float, int)) and row[key] >= 0, f"missing timing {key}"
            comparisons = []
            for origin, record in [("previous_record", previous), ("current_record", {"commit": measured_commit, "runs": runs})]:
                comparison = last_comparable(row, record, origin)
                if comparison:
                    comparisons.append(comparison)
            row["quarter_slower_comparisons"] = comparisons
            row["quarter_slower_tripwire"] = any(entry["tripped"] for entry in comparisons)
        for index, row in enumerate(rows):
            row["super_neighbour_scaling_tripwire"] = index % 3 != 0 and row["tick_ms_per_neighbour"] > rows[index - 1]["tick_ms_per_neighbour"]
        runs.append({"repeat": repeat + 1, "load": os.getloadavg(), "build": "release", "rows": rows})
    assert source_key() == measured_source, "simulation sources changed during measurement"
    return {"commit": measured_commit, "runs": runs, "allocations": allocations, "previous_record_sha256": previous_hash, "previous_record_commit": command(["git", "log", "-1", "--format=%H", "--", previous_path.relative_to(ROOT)]).strip() if previous else None, "artifact_sha256": runner.artifact_hash, "user_agent": runner.user_agent, "cross_origin_isolated": runner.isolated, "source_inputs": measured_inputs, "source_sha256": measured_source, "timings_are_baselines": True}


def last_comparable(row, record, origin):
    """Name the latest same-size row within 1% of this row's neighbour count."""
    if not record:
        return None
    for run in reversed(record["runs"]):
        for prior in reversed(run["rows"]):
            same_work = origin == "previous_record" or (prior.get("trace", "crowd") == row.get("trace", "crowd") and prior.get("parallel", False) == row.get("parallel", False))
            if same_work and prior["bodies"] == row["bodies"] and abs(row["neighbours_per_tick"] / prior["neighbours_per_tick"] - 1) < 0.01:
                ratio = row["tick_median_ms"] / prior["tick_median_ms"]
                return {"origin": origin, "commit": record["commit"], "repeat": run["repeat"],
                        "trace": prior.get("trace", "crowd"), "parallel": prior.get("parallel", False),
                        "bodies": prior["bodies"], "neighbours_per_tick": prior["neighbours_per_tick"],
                        "tick_median_ms": prior["tick_median_ms"], "ratio": ratio, "tripped": ratio > 1.25}
    return None


def load_cost_record(path):
    """A validation receipt is not a measurement; fail before running expensive work."""
    record = json.loads(path.read_text())
    if not isinstance(record.get("runs"), list) or not record["runs"]:
        raise ValueError(f"{path}: expected a measurement with runs; a costs receipt cannot replace a measure record")
    return record


def costs(target):
    """Check the retained device measurements, independently of their time values."""
    path = PRIOR_COSTS / f"costs-{target}.json"
    record = load_cost_record(path)
    assert record["target"] == target and record["dirty"] is False
    current = source_inputs()
    assert source_key(record["source_inputs"]) == record["source_sha256"], "cost input manifest hash differs"
    changes = [key for key in current if current[key] != record["source_inputs"].get(key)]
    assert record["source_sha256"] == source_key(current), f"cost record belongs to different simulation sources: {changes}"
    assert record["rustc"] == identity()["rustc"], "cost record belongs to another compiler"
    assert len(record["runs"]) == 2
    if target == "mac-metal":
        assert record["allocations"]["bodies"] == 10000
        assert set(record["allocations"]["counts"]) == {"build", "circle", "box", "capsule", "wide circle", "nearest-k"}
        assert all(isinstance(count, int) and count >= 0 for count in record["allocations"]["counts"].values())
    previous = None
    if record["previous_record_commit"]:
        # The prior record is test data with the bytes it had at previous_record_commit.
        retained = (PRIOR_COSTS / f"prior-costs-{target}.json").read_bytes()
        assert hashlib.sha256(retained).hexdigest() == record["previous_record_sha256"]
        previous = json.loads(retained)
    for index, run in enumerate(record["runs"]):
        assert run["build"] == "release" and len(run["load"]) == 3
        assert [(row["bodies"], row["ticks"], row["trace"], row["parallel"]) for row in run["rows"]] == [
            (count, ticks, trace, parallel) for parallel in ([False, True] if target == "mac-metal" else [False])
            for trace in ["crowd", "mixed"] for count, ticks in SIZES]
        for row_index, row in enumerate(run["rows"]):
            assert row["samples"] == row["ticks"] - 120 and row["neighbours_per_tick"] > 0
            assert row["build"] == "release" and row["utc"]
            assert len(row["load_before"]) == len(row["load_after"]) == 3
            assert row["tick_ms_per_neighbour"] == row["tick_median_ms"] / row["neighbours_per_tick"]
            assert row["clock_tripwire"] == (row["tick_median_ms"] > 1000.0 / 60.0)
            assert row["super_neighbour_scaling_tripwire"] == (row_index % 3 != 0 and row["tick_ms_per_neighbour"] > run["rows"][row_index - 1]["tick_ms_per_neighbour"])
            if row["parallel"]:
                assert row["parallel_chunks"] > 0
            for key in ["tick_median_ms", "index_median_ms", "digest_median_ms", "rewind_eight_ms", "replay_eight_ms"]:
                assert isinstance(row[key], (float, int)) and row[key] >= 0
            for key in ["clock_tripwire", "quarter_slower_tripwire", "super_neighbour_scaling_tripwire"]:
                assert isinstance(row[key], bool)
            comparisons = row["quarter_slower_comparisons"]
            expected = [entry for entry in [
                last_comparable(row, previous, "previous_record"),
                last_comparable(row, {"commit": record["commit"], "runs": record["runs"][:index]}, "current_record"),
            ] if entry]
            assert comparisons == expected, "missing or incorrect chronological cost comparator"
            for entry in comparisons:
                assert entry["origin"] in ["previous_record", "current_record"] and entry["commit"]
                assert entry["bodies"] == row["bodies"] and entry["repeat"] in [1, 2]
                assert abs(row["neighbours_per_tick"] / entry["neighbours_per_tick"] - 1) < 0.01
                assert entry["ratio"] == row["tick_median_ms"] / entry["tick_median_ms"]
                assert entry["tripped"] == (entry["ratio"] > 1.25)
            assert row["quarter_slower_tripwire"] == any(entry["tripped"] for entry in comparisons)
    return {"record": str(path.relative_to(ROOT)), "source_sha256": record["source_sha256"], "measured_commit": record["commit"], "timings_are_baselines": True}


def main():
    global PACKAGE, RUNNER, SIZES
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["retain", "retain-mixed", "record", "run", "control", "measure", "costs"])
    parser.add_argument("--trace", help="registered trace to record")
    parser.add_argument("--parallel", action="store_true")
    parser.add_argument("--package", default="crowd-2d", help="registered deterministic rule package")
    parser.add_argument("--target", choices=["mac-metal", "chrome-webgpu", "ios-sim"], default="mac-metal")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    PACKAGE = args.package
    config = registration()
    RUNNER = config["runner"]
    SIZES = [tuple(size) for size in config["sizes"]]
    if PACKAGE != "crowd-2d" and args.action not in ["record", "run", "control"]:
        parser.error("retention and historical cost commands belong to crowd-2d; use run for other registered packages")
    SCRATCH.mkdir(parents=True, exist_ok=True)
    if args.action == "record":
        if not args.trace or not args.output:
            parser.error("record requires --trace and --output for a new candidate file")
        print(json.dumps(record_trace(args.trace, args.output)))
        return
    record = {"action": args.action, "target": args.target, "package": PACKAGE, **identity()}
    protected = [PRIOR_COSTS / f"{kind}costs-{target}.json"
                 for kind in ["prior-", ""] for target in ["mac-metal", "chrome-webgpu"]]
    if args.action != "measure" and args.output and args.output.resolve() in protected:
        parser.error("only measure may write a retained cost record; costs writes a validation receipt")
    if args.action == "retain-mixed":
        result = retain_mixed()
    elif args.action == "run":
        result = run(args.target, args.parallel)
    else:
        result = retain() if args.action == "retain" else globals()[args.action](args.target)
    record.update(result)
    destination = args.output or SCRATCH / f"{args.action}-{args.target}.json"
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(json.dumps(record, indent=2) + "\n")
    print(json.dumps({"result": "pass", "record": str(destination.relative_to(ROOT) if destination.is_relative_to(ROOT) else destination)}))


if __name__ == "__main__":
    main()
