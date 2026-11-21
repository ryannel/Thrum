#!/usr/bin/env python3
"""PUDDLES cost record: preparing a hollows body of 640,000 cells and a fill step.

  python3 scripts/rain_cost.py record   measure in a release build and write the record to the working folder
  python3 scripts/rain_cost.py check    measure again; fail only if it cannot be measured
                                        or the committed record lacks a field

The committed record is test data in tests/fixtures/puddles/; promoting a new take is copying it there.

No timing figure decides anything (brief decision 3): `check` compares no numbers.
"""
import json
import os
import platform
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
RECORD = REPO / "tests/fixtures/puddles/rain-cost.json"
TAKE = REPO / ".orchestrate/evidence/PUDDLES/rain-cost.json"
COMMAND = ["cargo", "run", "--release", "--locked", "--offline", "-q",
           "-p", "thrum-rain-model", "--example", "rain_cost"]
MEASURED = ["cells", "prepare_seconds", "hollows", "nodes", "bytes", "curve_bytes",
            "curve_bytes_per_node", "fill_step"]
STEP = ["hollows_holding_water", "seconds_one_thread", "seconds_all_threads", "threads"]
CONTEXT = ["commit", "dirty", "build", "load_average", "machine", "date"]


def git(*args):
    return subprocess.run(["git", *args], cwd=REPO, check=True, capture_output=True,
                          text=True).stdout.strip()


def measure():
    """Run the release example; return its record or raise with the reason."""
    result = subprocess.run(COMMAND, cwd=REPO, capture_output=True, text=True, timeout=1200)
    if result.returncode != 0:
        raise RuntimeError(f"measurement failed:\n{result.stderr[-2000:]}")
    lines = [line for line in result.stdout.splitlines() if line.startswith("{")]
    if not lines:
        raise RuntimeError(f"no record printed:\n{result.stdout[-2000:]}")
    measured = json.loads(lines[-1])
    missing(measured, MEASURED, STEP)
    return measured


def missing(record, top, step):
    absent = [k for k in top if k not in record]
    absent += [f"fill_step.{k}" for k in step if k not in record.get("fill_step", {})]
    if absent:
        raise RuntimeError(f"record lacks {', '.join(absent)}")


def record():
    measured = measure()
    measured.update({
        "commit": git("rev-parse", "HEAD"),
        "dirty": bool(git("status", "--porcelain", "--", "crates", "scripts")),
        "build": "release (cargo run --release)",
        "load_average": [round(v, 2) for v in os.getloadavg()],
        "machine": f"{platform.machine()} {platform.mac_ver()[0] or platform.platform()}",
        "date": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "note": "taken while other builders share the machine; a tripwire, not a budget",
    })
    TAKE.parent.mkdir(parents=True, exist_ok=True)
    TAKE.write_text(json.dumps(measured, indent=2) + "\n", encoding="utf-8")
    print(TAKE.relative_to(REPO))


def check():
    measured = measure()
    committed = json.loads(RECORD.read_text(encoding="utf-8"))
    missing(committed, MEASURED + CONTEXT, STEP)
    print(f"measured: prepare {measured['prepare_seconds']:.3f} s, "
          f"{measured['hollows']} hollows, {measured['nodes']} nodes, {measured['bytes']} bytes; "
          f"fill step {measured['fill_step']['seconds_one_thread'] * 1e6:.1f} us; "
          f"committed record from {committed['commit'][:12]}")


def main():
    if len(sys.argv) != 2 or sys.argv[1] not in ("record", "check"):
        print(__doc__, file=sys.stderr)
        return 2
    try:
        (record if sys.argv[1] == "record" else check)()
    except (RuntimeError, OSError, ValueError, subprocess.SubprocessError) as error:
        print(f"rain_cost: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
