#!/usr/bin/env python3
"""Headless SESSION checks on the registered reference crowd."""

import argparse
import hashlib
import itertools
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
from concurrent.futures import ThreadPoolExecutor

import sim2d_conformance as conformance


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


def build(target, tree=ROOT, output=None):
    output = output or (ROOT / "target")
    triple = TARGETS[target]
    args = ["rustup", "run", "1.99.1", "cargo", "build", "--locked", "--offline", "--release", "-p", PACKAGE,
            "--target-dir", str(output)]
    if triple:
        args += ["--target", triple]
    if target == "chrome-webgpu":
        args.append("--lib")
    environment = dict(os.environ, RUSTC=conformance.command(["rustup", "which", "--toolchain", "1.99.1", "rustc"]).strip())
    environment.setdefault("RUSTC_WRAPPER", "")
    subprocess.run(args, cwd=tree, env=environment, check=True)
    directory = (output / triple if triple else output) / "release"
    return directory / ("match_proof.wasm" if target == "chrome-webgpu" else PACKAGE)


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


class Runner:
    def __init__(self, target, scratch):
        self.target = target
        self.artifact = build(target)
        self.artifact_hash = sha(self.artifact)
        self.device = conformance.simulator() if target == "ios-sim" else None
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
            shutil.copyfile(ROOT / "examples/match-proof/browser.js", self.web / "browser.js")
            self.browser_version = conformance.command([CHROME, "--version"]).strip()
            conformance.SCRATCH.mkdir(parents=True, exist_ok=True)

    def invoke(self, operation, count, mixed, data, last=None):
        if self.web:
            images = []
            if operation == "restore":
                images.append(data.name)
                shutil.copyfile(data, self.web / data.name)
                data = data.name
            (self.web / "images.json").write_text(json.dumps(images))
            arguments = [count, mixed, data]
            if last is not None:
                arguments.append(last)
            packet = json.loads(conformance.device_command("chrome", [
                "node", "scripts/sim2d_browser.mjs", self.web,
                json.dumps({"method": operation, "args": arguments}), "browser",
            ]))
            self.user_agent = packet["userAgent"]
            return packet["result"]

        data = ",".join(map(str, data)) if operation == "capture" else data
        arguments = [self.artifact, operation, count, str(mixed).lower(), data]
        if last is not None:
            arguments.append(last)
        if self.device:
            arguments = ["xcrun", "simctl", "spawn", self.device, *arguments]
            return json.loads(conformance.device_command("simulator", arguments))
        return json.loads(conformance.command(arguments))

    def record(self):
        return {
            "artifact_sha256": self.artifact_hash,
            "build": "release",
            "target": self.target,
            "simulator": self.device,
            "browser_version": self.browser_version,
            "user_agent": self.user_agent,
            "loaded_wasm_sha256": self.loaded_wasm_hash,
        }


def reference_rows():
    for trace in conformance.registration()["traces"]:
        path = ROOT / trace["retained"]
        yield trace, path, conformance.references(path)[10_000]


def trace_images(native, receiver, scratch, reference):
    trace, path, expected = reference
    name, mixed = trace["name"], trace["variant"]
    target = receiver.target
    print(f"{target}: write {name} images", file=sys.stderr, flush=True)
    if receiver is native:
        host = local = native.invoke("capture", 10_000, mixed, IMAGE_TICKS)
    else:
        with ThreadPoolExecutor(max_workers=1) as pool:
            pending = pool.submit(native.invoke, "capture", 10_000, mixed, IMAGE_TICKS)
            local = receiver.invoke("capture", 10_000, mixed, IMAGE_TICKS)
            host = pending.result()
    assert host["identity"] == local["identity"], "rule-set identity differs across builds"
    assert [row["tick"] for row in host["rows"]] == list(IMAGE_TICKS), "Mac image ticks"
    assert [row["tick"] for row in local["rows"]] == list(IMAGE_TICKS), "target image ticks"
    retained = dict(expected["checkpoints"])
    proofs = []
    for origin, peer in zip(host["rows"], local["rows"], strict=True):
        tick = origin["tick"]
        raw = bytes.fromhex(origin["image"])
        assert raw == bytes.fromhex(peer["image"]), f"{target} {name} tick {tick}: image bytes differ"
        assert raw[8:40].hex() == host["identity"], "image carries another rule-set identity"
        if tick:
            assert origin["reused"] > 0 and origin["despawned"] > 0, (name, tick, origin["reused"], origin["despawned"])
            assert origin["digest"] == retained[tick], f"{name} tick {tick}: writer digest differs"
        image = scratch / f"{name}-{tick}.bin"
        image.write_bytes(raw)
        print(f"{target}: restore {name} at tick {tick} through {expected['ticks']}", file=sys.stderr, flush=True)
        result = receiver.invoke("restore", 10_000, mixed, image, expected["ticks"])
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


def images(target, scratch):
    native = Runner("mac-metal", scratch)
    receiver = native if target == "mac-metal" else Runner(target, scratch)
    references = list(reference_rows())
    with ThreadPoolExecutor(max_workers=2 if receiver is native else 1) as pool:
        pending = [pool.submit(trace_images, native, receiver, scratch, reference)
                   for reference in references]
        proofs = [row for trace in pending for row in trace.result()]
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

    for first, second in zip(rows[:3], rows[3:], strict=True):
        for field in ("image_bytes", "restore_ns", "write_ns"):
            if field in first and second[field] * 4 >= first[field] * 5 and second[field] > first[field]:
                second["tripwires"].append(f"{field} at least a quarter above the previous run")
    return {
        "rows": rows, "mac_writer": native.record(), **receiver.record(),
        "tripwire_scope": "Wire bandwidth and join scaling apply to later transport runs.",
    }


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
        return json.loads(conformance.command([binary, "identity"]))["identity"]

    first, second = compiled("first"), compiled("second")
    assert first == second, "independent builds differ"
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
        value = compiled("second")
        assert value != first, f"{label} change did not change identity"
        changed.append({"control": label, "file": filename, "identity": value})
        path.write_text(original)
    return {"standalone_builds": [first, second], "controls": changed, "build": "release"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["image", "identity", "image-costs"])
    parser.add_argument("--target", choices=TARGETS, default="mac-metal")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    provenance = conformance.identity()
    SCRATCH.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=f"{args.action}-{args.target}-", dir=SCRATCH) as directory:
        scratch = Path(directory)
        if args.action == "image":
            record = images(args.target, scratch)
        elif args.action == "image-costs":
            record = costs(args.target, scratch, provenance)
        else:
            if args.target != "mac-metal":
                parser.error("the identity build controls run on the Mac")
            record = identity(scratch)
    output = args.output or SCRATCH / f"{args.action}-{args.target}.json"
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps({**provenance, "action": args.action, "target": args.target,
                                  **record, "result": "pass"}, indent=2) + "\n")
    print(json.dumps({"result": "pass", "record": str(output)}))


if __name__ == "__main__":
    main()
