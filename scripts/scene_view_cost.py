#!/usr/bin/env python3
"""Collect and validate capture-off scene-view timing blocks on Metal."""
import argparse
from collections import defaultdict
import gzip
import hashlib
import json
import os
from pathlib import Path
import re
import statistics
import subprocess

# check reads the committed record; run writes a new take to the working folder unless --record names another.
RECORD = Path("tests/fixtures/scene-views/cost.json.gz")
TAKE = Path(".orchestrate/evidence/SCENE-VIEWS/cost.json.gz")
RECIPES = ("portable", "apple-tile")
SCENES = ("mirror", "field")
KNOBS = (
    "view extent percent", "view far metres", "view least pixels",
    "view Sun cascade count", "view Sun reach metres",
)
ARMS = ("off", "one", "two", *(f"{knob} {end}" for knob in KNOBS for end in ("min", "max")))
OPAQUE = "synthetic forward opaque"
VIEW = "secondary view "


def git(*arguments):
    return subprocess.check_output(("git", *arguments), text=True).strip()


def host_load():
    return {
        "load_average": list(os.getloadavg()),
        "logical_cpus": os.cpu_count(),
        "processes": subprocess.check_output(
            ("ps", "-A", "-o", "pcpu=,comm=", "-r"), text=True,
        ).splitlines()[:12],
    }


def collect(recipe, binary, store):
    environment = dict(os.environ, THRUM_RECIPE=recipe, THRUM_MIRROR_COST="1",
                       THRUM_ASSET_STORE=str(store))
    before = host_load()
    header, *lines = subprocess.check_output([binary], env=environment, text=True, timeout=420).splitlines()
    after = host_load()
    blocks = defaultdict(lambda: defaultdict(dict))
    for line in lines:
        scene, arm, number, submission, name, duration = line.split("\t")
        row = blocks[scene][number].setdefault(arm, {"submission": int(submission), "passes_ns": {}})
        if row["submission"] != int(submission) or name in row["passes_ns"]:
            raise ValueError(f"duplicate or disjoint pass: {line}")
        row["passes_ns"][name] = int(duration)
    return {"adapter": header, "load_before": before, "load_after": after, "scenes": blocks}


def sum_passes(row):
    return sum(row["passes_ns"].values())


def view_sum(row):
    return sum(value for name, value in row["passes_ns"].items() if name.startswith(VIEW))


def summary(blocks):
    one = [arms["one"] for arms in blocks.values()]
    off = [arms["off"] for arms in blocks.values()]
    med = statistics.median
    return {
        arm: {
            "whole_gpu_ns_p50": med(sum_passes(row[arm]) for row in blocks.values()),
            "secondary_gpu_ns_p50": med(view_sum(row[arm]) for row in blocks.values()),
            "paired_whole_delta_from_off_ns_p50": med(
                sum_passes(row[arm]) - sum_passes(base) for row, base in zip(blocks.values(), off, strict=True)
            ),
            "paired_secondary_delta_from_one_ns_p50": med(
                view_sum(row[arm]) - view_sum(base) for row, base in zip(blocks.values(), one, strict=True)
            ),
            "main_opaque_gpu_ns_p50": med(row[arm]["passes_ns"][OPAQUE] for row in blocks.values()),
        }
        for arm in ARMS
    }


def check(record):
    if not re.fullmatch("[0-9a-f]{40}", record["commit"]):
        raise ValueError("missing source commit")
    if record["extent"] != [2560, 1440] or record["capture"] != "off" or record["field_instances"] != 100_513:
        raise ValueError("wrong workload, extent or capture state")
    if set(record["runs"]) != set(RECIPES):
        raise ValueError("missing recipe")
    for recipe, run in record["runs"].items():
        if "Apple M3 Max" not in run["adapter"] or "Metal" not in run["adapter"]:
            raise ValueError(f"{recipe}: target identity missing")
        if set(run["scenes"]) != set(SCENES) or not run["load_before"] or not run["load_after"]:
            raise ValueError(f"{recipe}: missing scenes or load")
        for scene, blocks in run["scenes"].items():
            if set(blocks) != {str(i) for i in range(1, 9)}:
                raise ValueError(f"{recipe}/{scene}: incomplete blocks")
            for arms in blocks.values():
                if set(arms) != set(ARMS):
                    raise ValueError(f"{recipe}/{scene}: incomplete paired arms")
                if len({row["submission"] for row in arms.values()}) != len(ARMS):
                    raise ValueError(f"{recipe}/{scene}: reused submissions")
                for arm, row in arms.items():
                    passes = row["passes_ns"]
                    if OPAQUE not in passes or not all(isinstance(value, int) and value >= 0 for value in passes.values()):
                        raise ValueError(f"{recipe}/{scene}/{arm}: invalid GPU times")
                    views = sum(name.startswith(f"{VIEW}opaque") for name in passes)
                    if views != {"off": 0, "two": 2}.get(arm, 1):
                        raise ValueError(f"{recipe}/{scene}/{arm}: wrong slot count")
            print(f"{recipe}/{scene}: {len(blocks)} paired blocks, {len(ARMS)} states per block")
            print(json.dumps(summary(blocks), indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("run", "check"))
    parser.add_argument("--record", type=Path)
    parser.add_argument("--binary", type=Path, default=Path("target/release/mirror-scene"))
    parser.add_argument("--store", type=Path)
    arguments = parser.parse_args()
    if arguments.action == "check":
        with gzip.open(arguments.record or RECORD, "rt", encoding="utf-8") as stream:
            check(json.load(stream))
        return
    if arguments.store is None:
        parser.error("run requires the cooked mirror --store")
    record = {
        "commit": git("rev-parse", "HEAD"),
        "clean_tracked_inputs": not git("status", "--porcelain", "--untracked-files=no"),
        "binary_sha256": hashlib.sha256(arguments.binary.read_bytes()).hexdigest(),
        "cooked_head_sha256": hashlib.sha256((arguments.store / "HEAD").read_bytes()).hexdigest(),
        "extent": [2560, 1440],
        "capture": "off",
        "field_instances": 100_513,
        "runs": {recipe: collect(recipe, arguments.binary, arguments.store) for recipe in RECIPES},
    }
    destination = arguments.record or TAKE
    destination.parent.mkdir(parents=True, exist_ok=True)
    with gzip.open(destination, "wt", encoding="utf-8") as stream:
        json.dump(record, stream, separators=(",", ":"))
    check(record)


if __name__ == "__main__":
    main()
