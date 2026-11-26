"""Cook the counted V1 pipeline stages, then verify their loading and preparation."""

from collections import Counter
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
from pathlib import Path
import re
import tempfile
import time

from apple_creation_proof import READY, cache_inventory, probe_build, witness
from apple_presentation import ROOT, run

PIPELINE = re.compile(r"THRUM_METALLIB_PIPELINE (\w+) (ThreadId\(\d+\)) ([^ ]+) ?(.*)")
COUNTED = re.compile(r"; pipelines (\w+)/[^;]*?: render (\d+) compute (\d+) mesh (\d+)")


def manifest(directory, expected, *, parallel=False):
    lines = (directory / "process.log").read_text().splitlines()
    ready = next(i for i, line in enumerate(lines) if READY.fullmatch(line))
    ledger, = [line for line in lines if line.startswith("THRUM_METAL_READY_LEDGER ")]
    prefix = "THRUM_METAL_READY_PIPELINES "
    named, = [json.loads(line.removeprefix(prefix)) for line in lines if line.startswith(prefix)]
    counts = Counter()
    for phase, render, compute, mesh in COUNTED.findall(ledger):
        if phase == "Build":
            for kind, value in zip(("render", "compute", "mesh"), (render, compute, mesh)):
                counts[kind] += int(value)
    pipelines, stages, threads = Counter(), Counter(), set()
    for index, line in enumerate(lines):
        if line.startswith("THRUM_METALLIB_EXPORT_ERROR"):
            raise ValueError(line)
        if not (match := PIPELINE.fullmatch(line)):
            continue
        kind, thread, recorded, label = match.groups()
        if index >= ready:
            raise ValueError(f"pipeline {label} created after readiness: {directory}")
        keys = []
        for item in recorded.split(","):
            key, path = item.split(":")
            if not re.fullmatch(r"[0-9a-f]{64}", key) or path != expected:
                raise ValueError(f"{label} has an unexpected {path} stage {key}: {directory}")
            stages[key] += 1
            keys.append(key)
        threads.add(thread)
        pipelines[(kind, label, tuple(keys))] += 1
    if not stages or Counter({kind: sum(count for row, count in pipelines.items() if row[0] == kind)
                              for kind in counts}) != counts:
        raise ValueError(f"pipeline manifest did not cover the creation counters: {counts} {pipelines}")
    labels = Counter()
    for (_, label, _), count in pipelines.items():
        labels[label] += count
    if labels != named:
        raise ValueError(f"stage manifest names disagree with the creation counters in {directory}")
    if parallel and len(threads) < 2:
        raise ValueError("no worker thread prepared a counted pipeline")
    return pipelines, stages, sorted(threads)


def describe(pipelines, stages, threads):
    return dict(pipelines=[dict(kind=kind, label=label, stages=keys, count=count)
                           for (kind, label, keys), count in sorted(pipelines.items())],
                stage_events=sum(stages.values()), distinct_stages=len(stages), threads=threads)


def cook_recipe(output, scratch, binary, env, recipe):
    source = scratch / f"stages-{recipe}"
    store = output / f"libraries-{recipe}"
    store.mkdir()
    start = time.monotonic_ns()
    arm = output / f"{recipe}-harvest"
    harvest = witness(arm, binary, dict(env, THRUM_METALLIB_HARVEST=str(source)), recipe, "launch")
    harvest_ns = time.monotonic_ns() - start
    pipelines, stages, threads = manifest(arm, "wgsl")
    if set(p.name for p in source.iterdir()) != stages.keys():
        raise ValueError("backend stage exports did not cover the counted pipeline manifest")
    start = time.monotonic_ns()
    cooker = binary.with_name("thrum-cook")

    def compile_stage(key):
        blob = store / f"{key}.blob"
        run(str(cooker), "metallib", str(source / key), str(blob), timeout=90)
        return key, dict(bytes=blob.stat().st_size, sha256=hashlib.sha256(blob.read_bytes()).hexdigest())

    with ThreadPoolExecutor(max_workers=4) as workers:
        blobs = dict(workers.map(compile_stage, sorted(stages)))
    cook_ns = time.monotonic_ns() - start
    reflection = {key: (source / key / "options.json").read_text() for key in sorted(stages)}
    return store, pipelines, dict(harvest=harvest, harvest_ns=harvest_ns, cook_ns=cook_ns,
                                  **describe(pipelines, stages, threads), blobs=blobs,
                                  compile_options=reflection)


def parity(output, *, build_only=False):
    recipes = {}
    with tempfile.TemporaryDirectory(prefix="apple-metallib-", dir=ROOT / "tmp") as temp:
        scratch = Path(temp)
        binary, env, route, build = probe_build(scratch)
        for recipe in ("portable", "apple-tile"):
            store, baked, cook = cook_recipe(output, scratch, binary, env, recipe)
            recipes[recipe] = {"cook": cook}
            if build_only:
                continue
            arms = {}
            for path in ("wgsl", "cooked"):
                arm = output / f"{recipe}-{path}"
                launch = dict(env, THRUM_METALLIB_MANIFEST="1")
                if path == "cooked":
                    launch["THRUM_METALLIB_STORE"] = str(store)
                result = witness(arm, binary, launch, recipe, "metallib", stage_path=path)
                pipelines, stages, threads = manifest(arm, path)
                if pipelines != baked or any(result["after_ready"].values()):
                    raise ValueError(f"{recipe} {path} did not finish the manifest before readiness")
                if result["routed_ticks"] != list(range(route["keys"][-1]["tick"] + 1)):
                    raise ValueError(f"{recipe} {path} did not finish the whole route")
                hashes = {}
                for checkpoint in route["checkpoints"]:
                    name = checkpoint["name"]
                    relative = Path("route") / "checkpoints" / f"{name}.surface-bgra8"
                    raw = (arm / relative).read_bytes()
                    if path == "cooked" and raw != (output / f"{recipe}-wgsl" / relative).read_bytes():
                        raise ValueError(f"{recipe}: cooked library changed checkpoint {name}")
                    hashes[name] = hashlib.sha256(raw).hexdigest()
                arms[path] = dict(**describe(pipelines, stages, threads), result=result, checkpoints=hashes)
            recipes[recipe]["arms"] = arms
    return dict(target="mac-metal", build=build, recipes=recipes)


def launch_paths(output):
    results = []
    recipe = "apple-tile"
    with tempfile.TemporaryDirectory(prefix="apple-launch-paths-", dir=ROOT / "tmp") as temp:
        scratch = Path(temp)
        binary, env, _, build = probe_build(scratch)
        store, baked, cook = cook_recipe(output, scratch, binary, env, recipe)
        probe = scratch / "cache-probe.dylib"
        run("xcrun", "clang", "-Wall", "-Werror", "-dynamiclib", "-framework",
            "CoreFoundation", str(ROOT / "scripts/apple_metal_cache_probe.c"), "-o", str(probe))
        for mode in ("wgsl", "cooked", "parallel"):
            expected = "wgsl" if mode == "wgsl" else "cooked"
            launch = dict(env, THRUM_METALLIB_MANIFEST="1")
            if expected == "cooked":
                launch["THRUM_METALLIB_STORE"] = str(store)
            if mode == "parallel":
                launch["THRUM_METALLIB_PARALLEL"] = "1"
            for repeat in (1, 2):
                cache = scratch / f"cache-{mode}-{repeat}"
                cache.mkdir()
                for temperature in ("cold", "warm"):
                    before = cache_inventory(cache)
                    if bool(before) != (temperature == "warm"):
                        raise ValueError(f"unexpected {temperature} Metal cache inventory: {before}")
                    arm = output / f"{mode}-{repeat}-{temperature}"
                    result = witness(arm, binary, launch, recipe, "launch", (probe, cache), stage_path=expected)
                    after = cache_inventory(cache)
                    if not after or any(result["after_ready"].values()) or result["finished"]:
                        raise ValueError(f"incomplete readiness receipt in {arm}")
                    pipelines, stages, threads = manifest(arm, expected, parallel=mode == "parallel")
                    if pipelines != baked:
                        raise ValueError(f"manifest differs in {arm}")
                    results.append(dict(
                        mode=mode, repeat=repeat, temperature=temperature, cache_before=before,
                        cache_after=after, **describe(pipelines, stages, threads), **result,
                    ))
    return dict(target="mac-metal", build=build, cook=cook, launches=results, timing_decides_pass=False)
