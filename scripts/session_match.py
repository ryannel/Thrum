#!/usr/bin/env python3
"""Headless SESSION checks on the registered reference crowd."""

import argparse
import fcntl
import hashlib
import itertools
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import tomllib
from concurrent.futures import ThreadPoolExecutor

import sim2d_conformance as conformance
import session_faults
import simulator_lifecycle


ROOT = conformance.ROOT
SCRATCH = ROOT / "tmp/session-m1"
PACKAGE = "match-proof"
TARGETS = {
    "mac-metal": None,
    "chrome-webgpu": "wasm32-unknown-unknown",
    "ios-sim": "aarch64-apple-ios-sim",
}
CHROME = Path("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome")
IMAGE_TICKS = (0, 18_000, 30_000)
COST_SIZES = (2000, 5000, 10000)
COST_TICK = 240


def build(target, tree=ROOT, output=None, *, sockets=False):
    output = output or (ROOT / "target" / "session-sockets" if sockets else ROOT / "target")
    triple = TARGETS[target]
    channel = tomllib.loads((tree / "rust-toolchain.toml").read_text())["toolchain"]["channel"]
    args = ["rustup", "run", channel, "cargo", "build", "--locked", "--offline", "--release", "-p", PACKAGE,
            "--target-dir", str(output)]
    if sockets:
        # The full match keeps five independently scheduled worlds running on
        # one machine. Optimize across the rule, runtime and session boundaries.
        args += ["--features", "sockets", "--config", 'profile.release.lto="fat"',
                 "--config", "profile.release.codegen-units=1"]
    if triple:
        args += ["--target", triple]
    if target == "chrome-webgpu":
        args.append("--lib")
    rustc = conformance.command(["rustup", "which", "--toolchain", channel, "rustc"]).strip()
    environment = dict(os.environ, RUSTC=rustc)
    environment.setdefault("RUSTC_WRAPPER", "")
    subprocess.run(args, cwd=tree, env=environment, check=True)
    directory = (output / triple if triple else output) / "release"
    return directory / ("match_proof.wasm" if target == "chrome-webgpu" else PACKAGE)


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def dependency_closure(package, *features, edges="normal,build"):
    command = ["cargo", "tree", "--locked", "--offline", "-p", package, "--target", "all",
               "--no-default-features", "-e", edges, "--prefix", "none"]
    if features:
        command += ["--features", ",".join(features)]
    tree = subprocess.check_output(command, cwd=ROOT, text=True)
    packages = {(name, version.removeprefix("v")) for name, version, *_ in
                (line.split() for line in tree.splitlines() if line.strip())}
    assert any(name == package for name, _ in packages), f"missing root: {package}"
    return packages


def closure():
    network = {"thrum-network", "thrum-transport"}
    shipping = {}
    for package in ("v1-game", "v1-game-ios", "thrum-starter", "thrum-stones", "thrum-lighthouse",
                    "mirror-scene", "water-scene"):
        names = {name for name, _ in dependency_closure(package, edges="normal,build,dev")}
        assert not names & network, f"{package} reaches {sorted(names & network)}"
        shipping[package] = {"dependency_count_including_root": len(names), "network_crates": []}

    before = {tuple(pair) for pair in
              json.loads((ROOT / "tests/fixtures/transport/lock-before.json").read_text())}
    transport = dependency_closure("thrum-transport")
    adapters = dependency_closure("thrum-transport", "native", "browser") - transport - before
    assert adapters, "the socket adapters must add crates relative to the pre-TRANSPORT lockfile"
    memory = dependency_closure(PACKAGE)
    assert network <= {name for name, _ in memory}, "the rehearsal must use the session and the message link"
    assert not memory & adapters, f"{PACKAGE} without sockets reaches {sorted(memory & adapters)}"
    sockets = dependency_closure(PACKAGE, "sockets")
    assert adapters <= sockets, f"the sockets control is missing {sorted(adapters - sockets)}"
    return {"shipping": shipping, "rehearsal_package": PACKAGE, "rehearsal_closure": sorted(memory),
            "socket_only_crates_witnessed_with_sockets": sorted(adapters)}


class Runner:
    def __init__(self, target, scratch):
        self.target = target
        built = build(target)
        snapshot = scratch / target
        snapshot.mkdir()
        self.artifact = snapshot / built.name
        shutil.copy2(built, self.artifact)
        self.artifact_hash = sha(self.artifact)
        self.lifecycle = scratch / "simulator-turns.jsonl" if target == "ios-sim" else None
        self.web = None
        self.loaded_wasm_hash = None
        self.user_agent = None
        self.browser_version = None
        if target == "chrome-webgpu":
            self.web = scratch / "web"
            self.web.mkdir()
            bindgen = Path.home() / ".cargo/bin/wasm-bindgen"
            subprocess.run([str(bindgen), "--target", "web", "--out-dir", str(self.web),
                            str(self.artifact)], check=True)
            self.loaded_wasm_hash = sha(self.web / "match_proof_bg.wasm")
            for name in ("browser.js", "image_worker.js"):
                shutil.copyfile(ROOT / "examples/match-proof" / name, self.web / name)
            self.browser_version = conformance.command([CHROME, "--version"]).strip()
            conformance.SCRATCH.mkdir(parents=True, exist_ok=True)

    def browser_call(self, method, arguments, files, *, requests=()):
        names = [file.name for file in files]
        for file in files:
            shutil.copyfile(file, self.web / file.name)
        (self.web / "images.json").write_text(json.dumps({"images": names, "requests": requests}))
        packet = json.loads(conformance.device_command("chrome", [
            "node", "scripts/sim2d_browser.mjs", self.web,
            json.dumps({"method": method, "args": arguments}), "browser",
        ]))
        self.user_agent = packet["userAgent"]
        return packet["result"]

    def invoke(self, operation, count, mixed, data, last=None):
        if self.web:
            assert operation == "restore", "Chrome image capture uses the batch"
            return self.browser_call(operation, [count, mixed, data.name, last], [data])

        data = ",".join(map(str, data)) if operation == "capture" else data
        arguments = [self.artifact, operation, count, str(mixed).lower(), data]
        if last is not None:
            arguments.append(last)
        if self.lifecycle:
            return json.loads(simulator_lifecycle.run_case(arguments, record=self.lifecycle))
        return json.loads(conformance.command(arguments))

    def record(self):
        turns = [json.loads(line) for line in self.lifecycle.read_text().splitlines()] if self.lifecycle else []
        return {
            "artifact_sha256": self.artifact_hash,
            "build": "release",
            "target": self.target,
            "simulator": next((turn["udid"] for turn in turns if turn["event"] == "booted"), None),
            "simulator_turns": turns,
            "browser_version": self.browser_version,
            "user_agent": self.user_agent,
            "loaded_wasm_sha256": self.loaded_wasm_hash,
        }


def reference_rows():
    for trace in conformance.registration()["traces"]:
        path = ROOT / trace["retained"]
        yield trace, path, conformance.references(path)[10_000]


def mac_capture(native, revision, name, mixed):
    # Each target compares with one Mac writer at this revision. Keep distinct
    # target checks from running that same long trace concurrently.
    cache = SCRATCH / "mac-images" / f"{revision}-{native.artifact_hash}"
    cache.mkdir(parents=True, exist_ok=True)
    record = cache / f"{name}.json"
    with (cache / f"{name}.lock").open("w") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        if record.exists():
            return json.loads(record.read_text())
        packet = native.invoke("capture", 10_000, mixed, IMAGE_TICKS)
        temporary = record.with_suffix(".json.partial")
        temporary.write_text(json.dumps(packet))
        temporary.replace(record)
        return packet


def check_trace_images(native, receiver, reference, host, local, restore):
    trace, path, expected = reference
    name = trace["name"]
    target = receiver.target
    assert host["identity"] == local["identity"], "rule-set identity differs across builds"
    assert [row["tick"] for row in host["rows"]] == list(IMAGE_TICKS), "Mac image ticks"
    assert [row["tick"] for row in local["rows"]] == list(IMAGE_TICKS), "target image ticks"
    retained = dict(expected["checkpoints"])
    proofs = []
    for origin, peer in zip(host["rows"], local["rows"], strict=True):
        tick = origin["tick"]
        raw = bytes.fromhex(origin["image"])
        if receiver.web:
            assert peer["equal_to_mac"], f"{name} tick {tick}: Chrome bytes differ"
        else:
            assert raw == bytes.fromhex(peer["image"]), f"{target} {name} tick {tick}: image bytes differ"
        assert raw[8:40].hex() == host["identity"], "image carries another rule-set identity"
        if tick:
            assert origin["reused"] > 0 and origin["despawned"] > 0, (name, tick, origin["reused"], origin["despawned"])
            assert origin["digest"] == retained[tick], f"{name} tick {tick}: writer digest differs"
        result = restore(tick, raw)
        assert result["identity"] == host["identity"] and result["epoch"] != origin["epoch"]
        if receiver is native:
            assert result["pid"] != host["pid"], "restore must run in a separate process"
        assert result["tick"] == tick and result["last"] == expected["ticks"]
        assert result["initial"] == origin["digest"] == peer["digest"], f"{name} tick {tick}: restored digest differs"
        remaining = [row for row in expected["checkpoints"] if row[0] > tick]
        assert len(result["checkpoints"]) == len(remaining), f"{name} tick {tick}: incomplete continuation"
        for actual, retained_row in zip(result["checkpoints"], remaining, strict=True):
            assert actual == retained_row, f"{target} {name} from {tick}: got {actual}, retained {retained_row}"
        proofs.append({
            "trace": name, "configured_bodies": 10_000,
            "retained": str(path.relative_to(ROOT)), "retained_sha256": sha(path),
            "image_sha256": hashlib.sha256(raw).hexdigest(),
            "rule_set": host["identity"],
            "comparison": "source image" if receiver is native else "byte for byte",
            "source_pid": host["pid"], "local_pid": local.get("pid"),
            "source": {key: origin[key] for key in ("tick", "epoch", "digest", "live", "reused", "despawned", "bytes")},
            "restore": {key: result[key] for key in ("tick", "epoch", "initial", "last")},
            "restore_pid": result.get("pid"), "checkpoints": len(remaining),
            "last_digest": remaining[-1][1],
        })
    return proofs


def trace_images(native, receiver, scratch, revision, reference):
    trace, _, expected = reference
    name, mixed = trace["name"], trace["variant"]
    target = receiver.target
    print(f"{target}: write {name} images", file=sys.stderr, flush=True)
    if receiver is native:
        host = local = mac_capture(native, revision, name, mixed)
    elif target == "ios-sim":
        host = mac_capture(native, revision, name, mixed)
        local = receiver.invoke("capture", 10_000, mixed, IMAGE_TICKS)
    else:
        with ThreadPoolExecutor(max_workers=1) as pool:
            pending = pool.submit(mac_capture, native, revision, name, mixed)
            local = receiver.invoke("capture", 10_000, mixed, IMAGE_TICKS)
            host = pending.result()

    def restore(tick, raw):
        image = scratch / f"{name}-{tick}.bin"
        image.write_bytes(raw)
        print(f"{target}: restore {name} at tick {tick} through {expected['ticks']}", file=sys.stderr, flush=True)
        return receiver.invoke("restore", 10_000, mixed, image, expected["ticks"])

    return check_trace_images(native, receiver, reference, host, local, restore)


def browser_images(native, receiver, scratch, revision, references):
    with ThreadPoolExecutor(max_workers=2) as pool:
        captures = [pool.submit(mac_capture, native, revision, trace["name"], trace["variant"])
                    for trace, _, _ in references]
        hosts = [pending.result() for pending in captures]

    files, requests = [], []
    for (trace, _, expected), host in zip(references, hosts, strict=True):
        name, mixed = trace["name"], trace["variant"]
        trace_files = []
        for row in host["rows"]:
            image = scratch / f"{name}-{row['tick']}.bin"
            image.write_bytes(bytes.fromhex(row["image"]))
            trace_files.append(image)
        files.extend(trace_files)
        names = [file.name for file in trace_files]
        requests.append([
            {"method": "capture", "args": [10_000, mixed, IMAGE_TICKS, names]},
            *({"method": "restore", "args": [10_000, mixed, filename, expected["ticks"]]} for filename in names),
        ])

    # Start the longest operations from both traces first. All workers run under
    # one Chrome admission, after the Mac writers have finished.
    ordered = list(itertools.chain.from_iterable(zip(*requests, strict=True)))
    print(f"{receiver.target}: compare images and replay {len(files)} restores in one Chrome run",
          file=sys.stderr, flush=True)
    results = receiver.browser_call("proveImages", [], files, requests=ordered)
    assert len(results) == len(ordered), "incomplete Chrome proof batch"
    proofs = []
    for index, (reference, host) in enumerate(zip(references, hosts, strict=True)):
        local, *restored = results[index::len(references)]
        by_tick = dict(zip(IMAGE_TICKS, restored, strict=True))
        proofs.extend(check_trace_images(native, receiver, reference, host, local,
                                        lambda tick, _raw: by_tick[tick]))
    return proofs


def images(target, scratch, revision):
    native = Runner("mac-metal", scratch)
    receiver = native if target == "mac-metal" else Runner(target, scratch)
    references = list(reference_rows())
    if receiver is native:
        with ThreadPoolExecutor(max_workers=2) as pool:
            pending = [pool.submit(trace_images, native, receiver, scratch, revision, reference)
                       for reference in references]
            proofs = [row for trace in pending for row in trace.result()]
    elif receiver.web:
        proofs = browser_images(native, receiver, scratch, revision, references)
    else:
        proofs = [row for reference in references
                  for row in trace_images(native, receiver, scratch, revision, reference)]
    return {"rows": proofs, "mac_writer": native.record(), **receiver.record()}


def costs(target, scratch, provenance):
    if target == "ios-sim":
        raise ValueError("the simulator is for function only")
    native = Runner("mac-metal", scratch)
    receiver = native if target == "mac-metal" else Runner(target, scratch)
    rows = []
    for run, count in itertools.product((1, 2), COST_SIZES):
        write_load = os.getloadavg()
        host = native.invoke("capture", count, False, [COST_TICK])
        source = host["rows"][0]
        image = scratch / f"cost-{run}-{count}.bin"
        image.write_bytes(bytes.fromhex(source["image"]))
        restore_load = os.getloadavg()
        result = receiver.invoke("restore", count, False, image, COST_TICK)
        assert result["initial"] == source["digest"] and result["identity"] == host["identity"]
        row = {
            "run": run, "configured_bodies": count, "commit": provenance["commit"], "tick": COST_TICK,
            **receiver.record(), "load": restore_load, "identity": host["identity"],
            "image_bytes": image.stat().st_size, "restore_ns": result["restore_ns"],
            "restore_pid": result.get("pid"), "tripwires": [],
        }
        if receiver is native:
            tick_with_write_ns = source["tick_ns"] + source["write_ns"]
            row.update(write_ns=source["write_ns"], write_load=write_load,
                       tick_with_write_ns=tick_with_write_ns, source_pid=host["pid"])
            if tick_with_write_ns * 60 > 1_000_000_000:
                row["tripwires"].append("host tick with image write exceeded its 60 Hz period")
        rows.append(row)

    for first, second in zip(rows[:len(COST_SIZES)], rows[len(COST_SIZES):], strict=True):
        for field in ("image_bytes", "restore_ns", "write_ns", "tick_with_write_ns"):
            if field in first and second[field] * 4 >= first[field] * 5 and second[field] > first[field]:
                second["tripwires"].append(f"{field} at least a quarter above the previous run")
    return {
        "rows": rows, "mac_writer": native.record(), **receiver.record(),
        "tripwire_scope": "Wire bandwidth and join scaling apply to later transport runs.",
    }


def variant_restores(binary, scratch, identities):
    count, tick = COST_SIZES[0], COST_TICK
    results = []
    for source, destination in (("crowd", "mixed"), ("mixed", "crowd")):
        capture = json.loads(conformance.command([binary, "capture", count, str(source == "mixed").lower(), tick]))
        row, = capture["rows"]
        raw = bytes.fromhex(row["image"])
        assert raw[8:40].hex() == capture["identity"] == identities[source]
        image = scratch / f"{source}.bin"
        image.write_bytes(raw)

        def restore(variant):
            return json.loads(conformance.command([binary, "restore", count, str(variant == "mixed").lower(),
                                                   image, tick]))

        matching = restore(source)
        assert matching["initial"] == row["digest"] and matching["identity"] == identities[source]
        refused = restore(destination)
        refusal_pid = refused.pop("pid")
        assert refused == {"refusal": "rule_set"}
        results.append({"source": source, "destination": destination, "tick": tick,
                        "source_identity": identities[source], "destination_identity": identities[destination],
                        "matching_digest": matching["initial"], "refusal": refused["refusal"],
                        "capture_pid": capture["pid"], "matching_pid": matching["pid"], "refusal_pid": refusal_pid})
    return results


def identity(scratch):
    # A separate source tree keeps the single-file controls out of the checkout.
    tree = scratch / "source"
    tracked = subprocess.check_output(["git", "ls-files", "-z"], cwd=ROOT).decode().split("\0")
    for name in filter(None, tracked):
        path = Path(name)
        if path.suffix == ".rs" or path.name in {"Cargo.toml", "Cargo.lock", "rust-toolchain.toml"} or name == ".cargo/config.toml":
            (tree / path).parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / path, tree / path)

    def compiled(output):
        binary = build("mac-metal", tree, scratch / output)
        packet = json.loads(conformance.command([binary, "identity"]))
        identities = {variant: packet[variant] for variant in ("crowd", "mixed")}
        assert identities["crowd"] != identities["mixed"], "Crowd and Mixed share a rule-set identity"
        return binary, identities

    binary, first = compiled("first")
    _, second = compiled("second")
    assert first == second, "independent builds differ"
    restores = variant_restores(binary, scratch, first)
    controls = [
        ("rules", "examples/crowd-2d/src/rules.rs", "N::from_i32(12)", "N::from_i32(13)"),
        ("engine profile", "crates/thrum-core/src/profile.rs", "0xd256_d193", "0xd256_d191"),
        ("engine digest", "crates/thrum-core/src/profile.rs", "0x100_0000_01b3", "0x100_0000_01b5"),
        ("action wire", "examples/crowd-2d-run/src/lib.rs", "Err(InputCodecError::InvalidAction)", "Ok((0, 0))"),
    ]
    changed = []
    for label, filename, before, after in controls:
        path = tree / filename
        original = path.read_text()
        assert before in original, f"missing control input in {filename}"
        path.write_text(original.replace(before, after, 1))
        _, value = compiled("second")
        assert all(value[variant] != original_identity for variant, original_identity in first.items()), (
            f"{label} change did not change both identities"
        )
        changed.append({"control": label, "file": filename, "identities": value})
        path.write_text(original)
    return {"standalone_builds": [first, second], "controls": changed, "variant_restores": restores,
            "source_copy_outside_checkout": True, "build": "release"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["image", "identity", "image-costs", "memory", "sockets", "closure"])
    parser.add_argument("scenario", nargs="?", choices=[
        "steady", "faults", "slow", "listen", "lead", "costs", "prediction", "prediction-costs", *session_faults.SCENARIOS,
    ])
    parser.add_argument("--target", choices=TARGETS, default="mac-metal")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    if args.action == "sockets":
        import session_sockets
        if args.scenario not in session_sockets.SCENARIOS:
            parser.error("sockets requires steady, faults, slow, listen or costs")
        if args.target == "ios-sim" and args.scenario in ("slow", "costs"):
            parser.error("the simulator is for function only")
        session_sockets.proof(args.scenario, args.target, args.output, build)
        return
    if args.action == "memory":
        if args.scenario is None or args.target != "mac-metal":
            parser.error("memory needs a scenario on mac-metal")
        if args.scenario in session_faults.SCENARIOS:
            session_faults.proof(args.scenario, args.output, build)
            return
        if args.scenario in ("prediction", "prediction-costs"):
            from session_prediction import proof
            proof(args.scenario, args.output, build)
            return
        from session_memory import proof
        proof(args.scenario, args.output, build)
        return
    if args.scenario is not None:
        parser.error("only memory takes a scenario")
    provenance = conformance.identity()
    if args.action == "closure":
        if args.target != "mac-metal":
            parser.error("the dependency closure check runs on the Mac and covers all Cargo targets")
        record = closure()
    else:
        SCRATCH.mkdir(parents=True, exist_ok=True)
        scratch_root = None if args.action == "identity" else SCRATCH
        with tempfile.TemporaryDirectory(prefix=f"thrum-{args.action}-{args.target}-", dir=scratch_root) as directory:
            scratch = Path(directory)
            if args.action == "image":
                record = images(args.target, scratch, provenance["commit"])
            elif args.action == "image-costs":
                record = costs(args.target, scratch, provenance)
            else:
                if args.target != "mac-metal":
                    parser.error("the identity build controls run on the Mac")
                assert not scratch.resolve().is_relative_to(ROOT.resolve()), "identity controls require external scratch"
                record = identity(scratch)
    output = args.output or SCRATCH / f"{args.action}-{args.target}.json"
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps({**provenance, "action": args.action, "target": args.target,
                                  **record, "result": "pass"}, indent=2) + "\n")
    print(json.dumps({"result": "pass", "record": str(output)}))


if __name__ == "__main__":
    main()
