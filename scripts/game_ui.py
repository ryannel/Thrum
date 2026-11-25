#!/usr/bin/env python3
"""Measure the prepared interface stage on Metal and Chrome WebGPU."""

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import shutil
import statistics
import subprocess
import tempfile


ROOT = Path(__file__).resolve().parents[1]
BEFORE = "cbb05f15bd17a87940bd913b51ffe61713ff822a"
GAME = Path("examples/v1-game")
PASS = "interface stage"
SPRITE_EXTENTS = ((1920, 1080), (3456, 2234))
MODES = ("8-bit", "float")
WORKLOADS = ("absent", "empty", "loaded")
BASE_CARGO = ("cargo", "build", "--release", "--locked", "--offline")
FIXED_ENV = {
    key: value for key, value in os.environ.items()
    if not key.startswith("THRUM_")
}


def run(args, *, cwd=ROOT, env=None):
    subprocess.run(args, cwd=cwd, env=env, check=True)


def output(args, *, cwd=ROOT, env=None):
    return subprocess.check_output(args, cwd=cwd, env=env, text=True).strip()


def load():
    return output(["sysctl", "-n", "vm.loadavg"])


def digest(file):
    with file.open("rb") as handle:
        return hashlib.file_digest(handle, "sha256").hexdigest()


def write(path, value):
    path.write_text(json.dumps(value, indent=2) + "\n")


def read(path):
    return json.loads(path.read_text())


def snapshot_main(directory):
    """The harness is the same in both builds; the old sample has no interface field."""
    directory.mkdir()
    with subprocess.Popen(["git", "archive", BEFORE], cwd=ROOT, stdout=subprocess.PIPE) as archive:
        subprocess.run(["tar", "-xf", "-", "-C", directory], stdin=archive.stdout, check=True)
        if archive.wait():
            raise RuntimeError(f"git archive failed for {BEFORE}")
    example = GAME / "examples/interface_cost.rs"
    shutil.copyfile(ROOT / example, directory / example)
    support = directory / GAME / "examples/support"
    support.mkdir(exist_ok=True)
    (support / "interface_cost.rs").write_text(
        "pub(crate) fn stats(_: &thrum_render::runtime::FrameSample) -> serde_json::Value {\n"
        "    serde_json::Value::Null\n}\n"
    )


def build_native(tree, target, destination):
    env = {**FIXED_ENV, "CARGO_TARGET_DIR": str(target)}
    run([*BASE_CARGO, "-p", "v1-game", "--example", "interface_cost"], cwd=tree, env=env)
    binary = target / "release/examples/interface_cost"
    shutil.copy2(binary, destination)
    return {"binary": destination, "sha256": digest(destination), "profile": "release"}


def cook_store(directory, target):
    run([*BASE_CARGO, "-p", "thrum-cook"], env={**FIXED_ENV, "CARGO_TARGET_DIR": str(target)})
    store = directory / "store"
    run([target / "release/thrum-cook", "cook", GAME / "assets/showcase", store], env=FIXED_ENV)
    return store


def finite(value):
    return isinstance(value, (int, float)) and math.isfinite(value)


def native_arm(binary, revision, name, extent, mode, store=None):
    env = {**FIXED_ENV, "THRUM_SCENE": "sprites", "THRUM_SPRITE_ROWS": "10000"}
    args = [binary["binary"], mode, *map(str, extent)]
    if store:
        env.update(THRUM_SCENE="valley", THRUM_ASSET_STORE=str(store))
        args.append(GAME / "routes/moving-v1.json")
    elif name != "absent":
        env.update(THRUM_SCENE="interface", THRUM_INTERFACE_ROWS="0" if name == "empty" else "2000")
    before = load()
    record = json.loads(output(args, env=env))
    after = load()
    record.update(name=name, commit=revision, profile="release", binary_sha256=binary["sha256"],
                  load_before=before, load_after=after)
    if (record["extent"] != list(extent) or record["mode"] != mode
            or len(record["samples"]) != 32 or bool(record["route_origin"]) != bool(store)):
        raise ValueError("incomplete native cost arm")
    if not all(finite(s["cpu_ns"]) and finite(s["gpu_pass_span_ns"])
               and finite(s["present_ns"]) for s in record["samples"]):
        raise ValueError("missing native frame timestamps")
    active = name != "absent"
    if (PASS in record["passes"]) != (active and mode == "8-bit"):
        raise ValueError("wrong native cost graph")
    for s in record["samples"]:
        if (s["interface"] is not None) != active:
            raise ValueError("missing or unexpected Metal interface record")
        if active and (s["interface"]["extent_mismatch"] or s["interface"]["dropped_rows"]):
            raise ValueError("incomplete paint workload")
        if mode == "8-bit" and active and not finite(s["interface_ns"]):
            raise ValueError("missing interface stage timestamp")
    record["shares_present_pass"] = active and mode == "float"
    record["load_bytes"] = record["store_bytes"] = extent[0] * extent[1] * 4 if active and mode == "8-bit" else 0
    return record


def measure_upload(record, accessor):
    upload = [accessor(s) for s in record["samples"]]
    if record["name"] == "loaded" and (not any(upload) or 0 not in upload):
        raise ValueError("missing unchanged or wholly changed paint-list sample")
    if record["name"] == "empty" and any(upload):
        raise ValueError("empty paint list uploaded bytes after warmup")
    record["uploaded_bytes"] = sorted(set(upload))


def median(record, key):
    return statistics.median(row[key] for row in record["samples"])


def costs_native(destination, revision, scratch):
    old_tree = scratch / "before"
    snapshot_main(old_tree)
    before = build_native(old_tree, scratch / "before-target", scratch / "old-interface-cost")
    target = scratch / "after-target"
    after = build_native(ROOT, target, scratch / "new-interface-cost")
    store = cook_store(scratch, target)
    builds = {"before": (before, BEFORE), "after": (after, revision)}
    comparisons = []
    for index, order in enumerate((("before", "after"), ("after", "before")), 1):
        paired = {}
        for scene, extent in (("sprites", SPRITE_EXTENTS[0]), ("valley", (1280, 720))):
            measurements = [
                native_arm(*builds[arm], "absent", extent, "8-bit",
                           store=store if scene == "valley" else None)
                for arm in order
            ]
            keyed = dict(zip(order, measurements, strict=True))
            paired[scene] = {
                "order": order, **keyed,
                "gpu_span_fraction_change": (
                    median(keyed["after"], "gpu_pass_span_ns")
                    / median(keyed["before"], "gpu_pass_span_ns") - 1
                ),
            }
        cases = []
        for extent in SPRITE_EXTENTS:
            for mode in MODES:
                arms = [native_arm(after, revision, name, extent, mode) for name in WORKLOADS]
                for arm in arms[1:]:
                    measure_upload(arm, lambda sample: sample["interface"]["uploaded_bytes"])
                measurement = {"extent": extent, "mode": mode, "arms": arms}
                if mode == "float":
                    # The transform and paint share one render pass. Subtract the
                    # same pass with an empty list to isolate incremental paint work.
                    measurement["paint_gpu_delta_ns"] = median(arms[2], "present_ns") - median(arms[1], "present_ns")
                    measurement["empty_gpu_delta_ns"] = median(arms[1], "present_ns") - median(arms[0], "present_ns")
                cases.append(measurement)
        record = {"run": index, "paired_no_interface": paired, "cases": cases}
        comparisons.append(record)
        write(destination / f"metal-{index}.json", record)
    return {
        "target": "mac-metal", "commit": revision, "before_commit": BEFORE,
        "runs": [f"metal-{index}.json" for index in (1, 2)],
        "float_gpu_method": "Paired difference of median present-pass time on the same extent and scene",
        "paired_gpu_tripwires": [
            {"run": run_record["run"], "scene": scene,
             "fraction": paired["gpu_span_fraction_change"]}
            for run_record in comparisons
            for scene, paired in run_record["paired_no_interface"].items()
            if paired["gpu_span_fraction_change"] >= 0.25
        ],
    }


def costs_chrome(destination, revision, target, scratch):
    web = target / "v1-game-web-proving"
    run(["sh", GAME / "web/build.sh", "--proving"],
        env={**FIXED_ENV, "CARGO_TARGET_DIR": str(target)})
    for index in (1, 2):
        directory = scratch / f"chrome-{index}"
        before = load()
        run(["node", "scripts/check_v1_browser.mjs", "--case", "interface-stage-cost",
             "--web-root", web, "--output", directory])
        after = load()
        record = read(directory / "interface-stage-cost.json")
        cases = record["rows"]
        if record["commit"] != revision or len(cases) != 6:
            raise ValueError("incomplete Chrome cost record")
        for case in cases:
            active = case["name"] != "absent"
            if ((case["srgb_views"] > 0) != active or len(case["samples"]) != 32
                    or (PASS in case["passes"]) != active):
                raise ValueError("incomplete Chrome interface case")
            if active:
                for sample in case["samples"]:
                    interface = sample["interface"]
                    if not all(finite(interface[key]) for key in ("encode_cpu_ns", "upload_cpu_ns", "gpu_ns")):
                        raise ValueError("incomplete Chrome timestamp record")
                measure_upload(case, lambda sample: sample["interface"]["uploaded_bytes"])
        record.update(load_before=before, load_after=after, profile="release")
        retained = destination / directory.name
        retained.mkdir(exist_ok=True)
        write(retained / "interface-stage-cost.json", record)
        shutil.copy2(directory / "browser-gpu.json", retained / "browser-gpu.json")
    return {"target": "chrome-webgpu", "commit": revision,
            "runs": [f"chrome-{index}/interface-stage-cost.json" for index in (1, 2)]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=["costs"])
    parser.add_argument("--target", required=True, choices=["mac-metal", "chrome-webgpu"])
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    revision = output(["git", "rev-parse", "HEAD"])
    scratch = ROOT / "tmp/game-ui-wp1"
    scratch.mkdir(parents=True, exist_ok=True)
    destination = args.output or Path(".orchestrate/evidence/GAME-UI/WP1") / f"costs-{args.target}"
    destination = destination.resolve()
    destination.mkdir(parents=True, exist_ok=True)
    target = ROOT / "target"
    with tempfile.TemporaryDirectory(prefix="cost-", dir=scratch) as work:
        if args.target == "mac-metal":
            record = costs_native(destination, revision, Path(work))
        else:
            record = costs_chrome(destination, revision, target, Path(work))
    write(destination / "summary.json", record)
    print(destination / "summary.json")


if __name__ == "__main__":
    main()
