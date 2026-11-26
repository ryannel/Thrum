"""Metal launch stage baselines and whole-route creation receipts."""
from collections import Counter
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import subprocess
import tempfile
import time

from apple_presentation import ROOT, RECIPE, read_rows, run, scale_environment
from run_v1_showcase import MOVING_ROUTE
from t2_load import LoadRecord


RECONSTRUCTION = {"portable": ("taau", "Taau"), "apple-tile": ("metalfx", "MetalFx")}
STAGES = ("wgsl_parse", "wgsl_validate", "wgsl_specialize", "wgsl_to_msl",
          "msl_compile", "pipeline_state", "shader_module", "metalfx", "metallib_load")
READY = re.compile(r"^THRUM_METAL_READY (\d+) (\d+) (\d+) (\d+) (\d+)$")
COOKED_DRAW = re.compile(r"^THRUM_METAL_COOKED_DRAW (\d+) (\d+)$")
FINISHED = re.compile(r"^THRUM_METAL_FINISHED (\d+) (\d+) (\d+)$")
STAGE = re.compile(r"^THRUM_METAL_STAGE (\w+) (\d+)$")
CACHE = re.compile(r"^THRUM_METAL_CACHE (.+)$")


def probe_build(scratch):
    wrapper = ROOT / "scripts/apple_metal_probe_rustc.py"
    probe_digest = hashlib.sha256(wrapper.read_bytes()).hexdigest()
    # A new wrapper changes the compilations that Cargo must retain.
    target = ROOT / "target/apple-creation-proof" / probe_digest[:16]
    build_env = {"CARGO_TARGET_DIR": str(target), "RUSTC_WRAPPER": str(wrapper),
                 "THRUM_METAL_PROBE_SOURCE": str(target / "probe-source")}
    started = time.monotonic_ns()
    binary, env, route = scale_environment(scratch, counters=True, build_env=build_env)
    build = {
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "cargo_lock_sha256": hashlib.sha256((ROOT / "Cargo.lock").read_bytes()).hexdigest(),
        "probe_sha256": probe_digest,
        "build_and_cook_ns": time.monotonic_ns() - started,
        "rustc": subprocess.check_output(["rustup", "run", "stable", "rustc", "-V"], text=True).strip(),
        "macOS": platform.mac_ver()[0],
        "os_build": subprocess.check_output(["sw_vers", "-buildVersion"], text=True).strip(),
        "cooked_root": route["cooked_root"],
    }
    # The environment used to run a measured process needs none of Cargo's inputs.
    return binary, env, route, build


def witness(output, binary, env, recipe, proof, cache=None, *, stage_path="wgsl"):
    output.mkdir()
    capture = output / "route"
    select, expected = RECONSTRUCTION[recipe]
    launch = dict(env, THRUM_RECIPE=recipe, THRUM_RECONSTRUCTION=select,
                  THRUM_CAPTURE_SCENE=str(capture), THRUM_CAPTURE_ROUTE=str(MOVING_ROUTE),
                  THRUM_CREATION_PROOF=proof)
    if cache:
        library, directory = cache
        launch.update(DYLD_INSERT_LIBRARIES=str(library), THRUM_METAL_CACHE_PATH=str(directory))
    load = LoadRecord(output)
    try:
        with (output / "process.log").open("w") as log:
            started = time.time_ns()
            process = subprocess.Popen([binary], cwd=ROOT, env=launch, stdout=log, stderr=subprocess.STDOUT)
            try:
                code = process.wait(timeout=300)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
                raise
            if code != 0:
                raise ValueError(f"{recipe} {proof} exited {code}; see {output / 'process.log'}")
    finally:
        observed_load = load.finish()
    header, *frames = read_rows(capture / "route.jsonl")
    if (header["backend"], header["recipe"], header["reconstruction"]) != ("Metal", RECIPE[recipe], expected):
        raise ValueError(f"wrong Metal target or reconstructor: {header}")

    lines = (output / "process.log").read_text().splitlines()
    ready = [(index, *map(int, match.groups())) for index, line in enumerate(lines)
             if (match := READY.fullmatch(line))]
    if len(ready) != 1:
        raise ValueError(f"expected one host readiness marker in {output}")
    boundary, now, generation, submitted, metalfx_calls, metalfx_ns = ready[0]
    if submitted != 0:
        raise ValueError(f"generation was marked ready after {submitted} submissions")
    cooked = [(index, *map(int, match.groups())) for index, line in enumerate(lines)
              if (match := COOKED_DRAW.fullmatch(line))]
    if (len(cooked) != 1 or cooked[0][0] <= boundary
            or cooked[0][2] != header["cooked_generation"]):
        raise ValueError(f"missing first cooked draw after readiness in {output}")
    _, cooked_epoch_ns, _ = cooked[0]
    rows = [(index, match[1], int(match[2])) for index, line in enumerate(lines)
            if (match := STAGE.fullmatch(line))]
    before = {stage: [ns for index, name, ns in rows if index < boundary and name == stage]
              for stage in STAGES}
    after = Counter(name for index, name, _ in rows if index > boundary)
    if set(after) - set(STAGES) or any(name not in STAGES for _, name, _ in rows):
        raise ValueError("unknown Metal boundary event")
    translated = ("wgsl_specialize", "wgsl_to_msl", "msl_compile")
    always = ("wgsl_parse", "wgsl_validate", "pipeline_state", "shader_module")
    required = always + (translated if stage_path == "wgsl" else ("metallib_load",))
    internal = [(index, line.removeprefix("THRUM_METAL_INTERNAL "))
                for index, line in enumerate(lines[:boundary]) if line.startswith("THRUM_METAL_INTERNAL ")]
    first_game_shader = next((index for index, name, _ in rows if name == "wgsl_parse"), boundary)
    translated_internal = {name: [ns for index, stage, ns in rows
                                  if index < first_game_shader and stage == name]
                           for name in translated}
    unexpected = (any(len(before[name]) != len(internal)
                      or before[name] != translated_internal[name] for name in translated)
                  if stage_path == "cooked" else bool(before["metallib_load"]))
    if any(not before[name] for name in required) or unexpected:
        raise ValueError(f"unexpected startup stages in {output}: {list(map(len, before.values()))}; {internal}")
    if metalfx_calls != int(recipe == "apple-tile"):
        raise ValueError(f"MetalFX count did not match the selected launch arm: {metalfx_calls}")
    finished = [list(map(int, match.groups())) for line in lines
                if (match := FINISHED.fullmatch(line))]
    if len(finished) == 1:
        after["metalfx"] += finished[0][2]
    stage_totals = {stage: {"calls": len(times), "ns": sum(times)} for stage, times in before.items()}
    stage_totals["metalfx"] = {"calls": metalfx_calls, "ns": metalfx_ns}
    reported_cache = [match[1] for line in lines if (match := CACHE.fullmatch(line))]
    if cache and reported_cache != [str(cache[1])]:
        raise ValueError(f"Metal did not attest the selected cache: {reported_cache} vs {cache[1]}")
    return dict(pid=process.pid, host_launch_epoch_ns=started, ready_epoch_ns=now,
                launch_to_ready_ns=now - started, device_generation=generation,
                submitted_at_ready=submitted, first_cooked_draw_epoch_ns=cooked_epoch_ns,
                launch_to_first_cooked_draw_ns=cooked_epoch_ns - started, header=header,
                stages=stage_totals, wgpu_internal_started=[label for _, label in internal],
                wgpu_internal_translation_ns=sum(sum(samples) for samples in translated_internal.values()),
                wgsl_translation_ns=sum(sum(times) for stage, times in before.items() if stage.startswith("wgsl_")),
                after_ready=dict.fromkeys(STAGES, 0) | dict(after), finished=finished,
                cache_path=reported_cache[0] if reported_cache else None, load=observed_load,
                routed_ticks=[frame["route_tick"] for frame in frames],
                route_history_sha256=hashlib.sha256(json.dumps([
                    (frame["route_tick"], frame["submission"], frame["jitter_phase"], frame["history"])
                    for frame in frames
                ]).encode()).hexdigest())


def cache_inventory(directory):
    return {str(path.relative_to(directory)): path.stat().st_size
            for path in sorted(directory.rglob("*")) if path.is_file()}


def mac_launch(output):
    results = []
    with tempfile.TemporaryDirectory(prefix="apple-launch-", dir=ROOT / "tmp") as temp:
        scratch = Path(temp).resolve()
        binary, env, _, build = probe_build(scratch)
        library = scratch / "cache-probe.dylib"
        run("xcrun", "clang", "-Wall", "-Werror", "-dynamiclib", "-framework",
            "CoreFoundation", str(ROOT / "scripts/apple_metal_cache_probe.c"), "-o", str(library))
        for pair in (1, 2):
            # An exclusive directory starts empty for this pair. Metal itself
            # chooses the files and confirms its active path from inside the game.
            cache = scratch / f"metal-shader-cache-{pair}"
            cache.mkdir()
            for state in ("cold", "warm"):
                before = cache_inventory(cache)
                if (state == "cold" and list(cache.iterdir())) or (state == "warm" and not before):
                    raise ValueError(f"cache inventory cannot establish a {state} launch: {before}")
                arm = f"{pair}-{state}"
                result = witness(output / arm, binary, env, "apple-tile", "launch", (library, cache))
                after = cache_inventory(cache)
                libraries = any(Path(name).name.startswith("libraries") and size > 0
                                for name, size in after.items())
                functions = any(Path(name).name.startswith("functions") and size > 0
                                for name, size in after.items())
                if not (libraries and functions) or result["finished"] or any(result["after_ready"].values()):
                    raise ValueError(f"incomplete readiness/cache record in {arm}")
                results.append(dict(arm=arm, cache_before=before, cache_after=after, **result))
    return dict(target="mac-metal", build=build, launches=results, timing_decides_pass=False,
                clock="parent pre-spawn and child readiness on local system clock; stage durations use Rust Instant",
                cache_scope="Metal's selected on-disk shader cache for this launch pair; one fresh process per arm",
                scope="offscreen V1 launch to renderer generation readiness before first submission; first cooked draw is a separate endpoint")


def mac_no_compile(output):
    results = []
    with tempfile.TemporaryDirectory(prefix="apple-no-compile-", dir=ROOT / "tmp") as temp:
        binary, env, route, build = probe_build(Path(temp))
        ticks = list(range(route["keys"][-1]["tick"] + 1))
        for recipe, proof in (("portable", "route"), ("apple-tile", "route"),
                              ("apple-tile", "late-pipeline")):
            arm = f"{recipe}-{proof}"
            result = witness(output / arm, binary, env, recipe, proof)
            expected = int(proof == "late-pipeline")
            counts = result["after_ready"]
            want = dict.fromkeys(STAGES, expected)
            want["metalfx"] = 0
            want["metallib_load"] = 0
            if (result["routed_ticks"] != ticks or counts != want or len(result["finished"]) != 1
                    or result["finished"][0][0] < len(ticks) or result["finished"][0][1] != expected):
                raise ValueError(f"{arm} did not detect exactly the expected late creation(s): {result}")
            results.append(dict(arm=arm, **result))
    return dict(target="mac-metal", build=build, routes=results)
