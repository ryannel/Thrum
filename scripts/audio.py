#!/usr/bin/env python3
"""Build the import-free audio mix module and run the silent WP1 proofs."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import sys
from datetime import datetime, timezone

ROOT = Path(__file__).resolve().parents[1]
TARGET = ROOT / "target"
WASM = TARGET / "wasm32-unknown-unknown/release/thrum_audio.wasm"
NATIVE = TARGET / "release/examples/audio-proof"
NODE = ROOT / "scripts/audio_module.mjs"
DIGEST = ROOT / "crates/thrum-audio/tests/reference.digest"
CARGO = ("rustup", "run", "stable", "cargo")


def run(*command, capture=False, timeout=600, env=None):
    result = subprocess.run(
        command, cwd=ROOT, text=True, check=True, timeout=timeout, env=env,
        stdout=subprocess.PIPE if capture else None,
    )
    return result.stdout.strip() if capture else None


def cargo_env():
    rustc = run("rustup", "which", "--toolchain", "stable", "rustc", capture=True)
    return dict(os.environ, RUSTC=rustc, RUSTC_WRAPPER=os.environ.get("RUSTC_WRAPPER", ""))


def build_module():
    run(*CARGO, "rustc", "--locked", "--offline", "--target-dir", TARGET,
        "-p", "thrum-audio", "--release", "--lib", "--target", "wasm32-unknown-unknown",
        "--features", "mix-module", "--crate-type", "cdylib", env=cargo_env())


def build_native():
    run(*CARGO, "build", "--locked", "--offline", "--target-dir", TARGET,
        "-p", "thrum-audio", "--release", "--example", "audio-proof", env=cargo_env())


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def metadata():
    return {
        "revision": run("git", "rev-parse", "HEAD", capture=True),
        "utc": datetime.now(timezone.utc).isoformat(),
        "host": platform.platform(),
        "rustc": run("rustup", "run", "stable", "rustc", "--version", capture=True),
        "node": run("node", "--version", capture=True),
        "wasm": str(WASM.relative_to(ROOT)),
        "wasm_sha256": sha256(WASM),
    }


def mismatch(frames, actual, expected):
    build_native()
    original = run(NATIVE, "reference-bits", str(frames), capture=True).splitlines()
    observed = run("node", NODE, WASM, "reference-bits", str(frames), capture=True).splitlines()
    for sample, (native, wasm) in enumerate(zip(original, observed)):
        if native != wasm:
            return f"frame {sample // 2}, channel {sample % 2}, native 0x{native}, wasm 0x{wasm}"
    if len(original) != len(observed):
        return f"native returned {len(original)} samples and Wasm returned {len(observed)}"
    return f"current native and Wasm bits agree at digest {actual}, retained digest is {expected}"


def module():
    build_module()
    expected = DIGEST.read_text().strip()
    data = metadata()
    data.update(target="stand_in:chrome-webgpu", imports=[], retained_digest=expected, pulls={})
    for frames in (128, 512):
        actual = run("node", NODE, WASM, "reference", str(frames), capture=True)
        data["pulls"][frames] = actual
        if actual != expected:
            data["first_difference"] = mismatch(frames, actual, expected)
            print(json.dumps(data, indent=2), flush=True)
            raise ValueError("the mix module's reference digest differs")
    print(json.dumps(data, indent=2))


CAPACITY_REASONS = {
    "one_shots": "48 slots cover forty hits a second with eight slots for overlapping cues.",
    "emitters": "16 slots keep the strongest continuous sounds in the mix.",
    "commands": "256 slots allow a burst of plays, emitter changes and control commands.",
    "outcomes": "512 outcomes retain at least two full command queues between drains.",
    "sounds": "256 identities allow a scene to keep a range of short effects registered.",
}


def timed_process(*command):
    before = os.getloadavg()
    values = json.loads(run(*command, capture=True, timeout=75))
    values.update(load_before=before, load_after=os.getloadavg())
    return values


def costs():
    build_module()
    build_native()
    data = metadata()
    native = timed_process(NATIVE, "costs")
    wasm = timed_process("node", NODE, WASM, "costs")
    data.update(
        target="mac-metal",
        load="As measured on the shared Mac while other work continued.",
        native_sha256=sha256(NATIVE),
        sound_seconds=60,
        frames_per_pull=128,
        full_pool={
            "native": {
                "seconds": native["render_seconds"], "real_time_share": native["render_seconds"] / 60,
                "load_before": native["load_before"], "load_after": native["load_after"],
                "witness": native["witness"],
            },
            "wasm_node": {
                "seconds": wasm["render_seconds"], "real_time_share": wasm["render_seconds"] / 60,
                "load_before": wasm["load_before"], "load_after": wasm["load_after"],
                "witness": wasm["witness"],
            },
        },
        emitter_update={
            "seconds": native["update_seconds"], "emitters": 10000, "moved": 2000,
            "listener_moved": True, "eligible": native["eligible"],
        },
        capacities={
            "pool": {"value": native["one_shots"] + native["emitters"],
                     "reason": "The sum of the two independently reserved voice shares."},
            **{key: {"value": native[key], "reason": reason} for key, reason in CAPACITY_REASONS.items()},
        },
        tuning={key: native[key] for key in ("ramp_frames", "glide_step", "knee", "near", "emitter_margin")},
    )
    print(json.dumps(data, indent=2), flush=True)
    if not all(0 < row["seconds"] <= 60 for row in data["full_pool"].values()):
        raise ValueError("one minute of audio did not render within one minute")
    if native["update_seconds"] <= 0:
        raise ValueError("no emitter update measurement")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("module", "costs"))
    args = parser.parse_args()
    try:
        {"module": module, "costs": costs}[args.command]()
    except (ValueError, OSError, subprocess.SubprocessError) as error:
        print(error, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
