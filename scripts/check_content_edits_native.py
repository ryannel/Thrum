#!/usr/bin/env python3
"""Run the watched valley with the native ordinary frame driver and optional T2 readback."""
import argparse
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import sys
import time

from edit_valley_content import edit

ROOT = Path(__file__).resolve().parents[1]


def wait_until(test, child, timeout):
    deadline = time.monotonic() + timeout
    while not test():
        if child.poll() is not None or time.monotonic() >= deadline:
            raise RuntimeError(f"readiness deadline, child status {child.poll()}")
        time.sleep(0.01)


def generation(store):
    head = store / "HEAD"
    if not head.exists():
        return 0
    return struct.unpack_from("<Q", head.read_bytes(), 16)[0]


def run(args):
    output = args.output.resolve()
    output.relative_to(ROOT / "tmp")
    output.mkdir(parents=True, exist_ok=False)
    source, store = output / "source", output / "store"
    shutil.copytree(ROOT / "examples/v1-game/assets/showcase", source)
    edit(source, "prepare")
    command = [
        sys.executable, str(ROOT / "scripts/capture_v1_runtime.py"), "--binary",
        str(args.binary.resolve()), "--store", str(store), "--output", str(output / "frames"),
        "--case", "content-edits", "--mode", "realtime", "--reconstruction", "taau",
        "--overlay", "proof", "--extent", "960", "540",
    ]
    if not args.capture:
        command.append("--observer-only")
    else:
        command.extend(["--defer", "--verify-only"])
    environment = dict(os.environ, THRUM_VALLEY_MESHES="on", THRUM_CAPTURE_ENCODING="raw")
    events = []
    load_before = os.getloadavg()
    with (output / "cook.log").open("w") as cook_log, (output / "driver.log").open("w") as driver_log:
        with subprocess.Popen(
            [str(args.cooker.resolve()), "watch", str(source), str(store)],
            cwd=ROOT, stdout=cook_log, stderr=subprocess.STDOUT,
        ) as cooker:
            try:
                wait_until(lambda: generation(store) == 1, cooker, 30)
                with subprocess.Popen(
                    command, cwd=ROOT, env=environment, stdout=driver_log, stderr=subprocess.STDOUT,
                ) as driver:
                    try:
                        wait_until(lambda: (output / "frames/measurement-ready.txt").exists(), driver, 75)
                        origin = time.monotonic_ns()
                        for expected, (at, product) in enumerate([(2.5, "mesh"), (5, "texture"), (7.5, "volume")], 2):
                            wait_until(lambda: time.monotonic_ns() - origin >= at * 1e9, driver, 12)
                            begin = time.monotonic_ns()
                            edit(source, product)
                            wait_until(lambda: generation(store) >= expected, cooker, 3)
                            if generation(store) != expected:
                                raise RuntimeError(f"{product} published more than once")
                            events.append({
                                "product": product, "generation": expected,
                                "save_ns": begin - origin, "publication_ns": time.monotonic_ns() - origin,
                            })
                        if driver.wait(timeout=120):
                            raise RuntimeError(f"T2 driver failed; see {output / 'driver.log'}")
                    finally:
                        if driver.poll() is None:
                            driver.terminate()
            finally:
                cooker.terminate()
    runtime = json.loads((output / "frames/runtime.json").read_text())
    submitted = [row for row in runtime["opportunities"] if row["submitted"]]
    switches = []
    previous = 1
    for row in submitted:
        content = row["content"]
        current = content["placement"]
        if any(current != content[reader] for reader in ("row", "lentTexture", "lentVolume")) or content["errors"]:
            raise RuntimeError(f"publication mismatch in submission {row['submission_id']}: {content}")
        if content["reconstruction"] != "Taau" or content["copyBytes"] > (8 << 20) or content["stagingPeakBytes"] > (32 << 20):
            raise RuntimeError(f"wrong reconstruction or exceeded staging bound: {content}")
        if current != previous:
            if current != previous + 1:
                raise RuntimeError(f"publication skipped: {previous} -> {current}")
            switches.append(row)
            previous = current
    if len(switches) != len(events):
        raise RuntimeError("all three saves must switch")
    for event, switched in zip(events, switches, strict=True):
        event["switch_submission"] = switched["submission_id"]
        event["switch_ns"] = switched["returned_ns"]
    steady = []
    for row in runtime["opportunities"][1:]:
        start = row["request_ns"]
        window = next(
            (event for event in events if event["save_ns"] - 50_000_000 <= start <= event["switch_ns"] + 150_000_000),
            None,
        )
        key = "step_wall_ns"
        if window:
            window[key] = max(window.get(key, 0), row[key])
        else:
            steady.append(row[key])
    record = {
        "commit": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        "target": "mac-metal", "capture": args.capture, "events": events, "submitted": len(submitted),
        "steady_max_step_wall_ns": max(steady), "load_before": load_before, "load_after": os.getloadavg(),
        "edit_clock": "relative to measurement-ready before the prime submission; 50 ms early margin in comparisons",
        "late": [event["product"] for event in events if event["step_wall_ns"] > max(steady)],
    }
    (output / "content-edits.json").write_text(json.dumps(record, indent=2) + "\n")
    print(json.dumps(record, indent=2))
    if not args.capture and record["late"]:
        raise SystemExit("edit step took longer than the steady maximum")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--binary", type=Path, default=Path("target/release/v1-game"))
    parser.add_argument("--cooker", type=Path, default=Path("target/release/thrum-cook"))
    parser.add_argument("--capture", action="store_true")
    run(parser.parse_args())
