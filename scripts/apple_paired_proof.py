"""Compare the Metal presentation paths on the same routed world and scale trace."""

from collections import defaultdict
import hashlib
import json
import platform
import re
from statistics import median
import subprocess

from apple_governor_proof import awake_display, require_unlocked_display
from apple_presentation import RECIPE, ROOT, SCALE_STEPS, annotate_display_ticks, read_rows, scale_environment, scale_reference
from run_v1_showcase import MOVING_ROUTE
from t2_load import LoadRecord

EXTENT = [1920, 1080]
SCALES = (67, 50)
TIMED = re.compile(r"Timed\(\[(.+), (.+)\]\)")


def distribution(values):
    values = sorted(values)
    if not values:
        return "unmeasured"
    return dict(count=len(values), p50=median(values), p95=values[(95 * len(values) - 1) // 100],
                maximum=values[-1])


def measured_scale(scale, samples, graph_traffic):
    extent = [round(axis * scale / 100) for axis in EXTENT]
    samples = [row for row in samples if row["render_extent"] == extent]
    if not samples:
        raise ValueError(f"no routed samples at {scale}%")
    counters = defaultdict(list)
    for row in samples:
        for name, value in (row["gpu_pass_ns"] or {}).items():
            counters.setdefault(name, [])
            if value is not None:
                counters[name].append(value)
        if row["gpu_present_ns"] is not None:
            counters["surface present"].append(row["gpu_present_ns"])
        if row["gpu_span_ns"] is not None:
            counters["timed pass span"].append(row["gpu_span_ns"])
        segment = row["native_segment"]
        outcome = segment and TIMED.fullmatch(segment["outcome"])
        if outcome:
            counters[segment["pass"]].append(round((float(outcome[2]) - float(outcome[1])) * 1e9))
    return dict(scale=scale, drawn=extent, samples=len(samples), gpu_ns={
        name: distribution(values) for name, values in sorted(counters.items())
    }, graph_declared_attachment_load_store_bytes=graph_traffic,
        graph_traffic_scope="Full declared attachment ranges per pass; render scale changes covered pixels within those ranges",
        measured_device_memory_traffic="unmeasured")


def window_arm(output, binary, env, route, name, recipe, reconstruction, headroom, *,
               replay=True, fullscreen=False, capture=False):
    arm = output / name
    arm.mkdir()
    display = arm / "edr"
    display.mkdir()
    route_document = json.loads(route.read_text())
    start = route_document["origin_tick"]
    launch = dict(env, THRUM_RECIPE=recipe, THRUM_RECONSTRUCTION=reconstruction,
                  THRUM_GOVERNOR_ROUTE=str(route), THRUM_GOVERNOR="off" if replay else "on",
                  THRUM_GOVERNOR_RECORD="replay" if replay else "observed",
                  THRUM_G4_SCALE_ORIGIN=str(start),
                  THRUM_MAC_TRANSITION=str(arm),
                  THRUM_MAC_TRANSITION_ARM="fullscreen" if fullscreen else "route",
                  THRUM_MAC_EDR=headroom, THRUM_MAC_EDR_PROOF=str(display),
                  THRUM_REPORT="1", THRUM_V1_FRAMES="800")
    if capture:
        launch["THRUM_MAC_TRANSITION_TAP"] = "1"
    load = LoadRecord(arm)
    try:
        with (arm / "game.log").open("w") as log:
            subprocess.run([binary], cwd=ROOT, env=launch, stdout=log, stderr=subprocess.STDOUT,
                           check=True, timeout=130)
    finally:
        observed_load = load.finish()
        (arm / "host-load.json").write_text(json.dumps(observed_load, indent=2) + "\n")
    prepared = json.loads((arm / "prepared.json").read_text())
    reconstructor = {"metalfx": "MetalFx", "taau": "Taau"}[reconstruction]
    if (prepared["adapter"], prepared["recipe"], prepared["reconstruction"]) != (
            "Apple M3 Max", RECIPE[recipe], reconstructor):
        raise ValueError(f"{name}: unexpected prepared target or fallback")
    configured = [row for row in read_rows(display / "frames.jsonl") if row.get("presented")]
    expected = "Bgra8" if headroom == "sdr" else "Rgba16Float"
    if not configured or any(not row["format"].startswith(expected) for row in configured):
        raise ValueError(f"{name}: surface did not use {expected}")
    if {row["headroom"] for row in configured} != {1 if headroom == "sdr" else float(headroom)}:
        raise ValueError(f"{name}: surface used unexpected headroom")
    frames = read_rows(arm / "frames.jsonl")
    ticks = annotate_display_ticks(arm, frames, read_rows(arm / "configure.jsonl"), False)
    end = start + route_document["keys"][-1]["tick"]
    if not frames or frames[-1]["world_tick"] < end:
        raise ValueError(f"{name}: the live run ended before the route")
    route_frames = [row for row in frames if start <= row["world_tick"] <= end]
    samples = [sample for row in frames for sample in row["governor"]["samples"]
               if start <= sample["tick"] <= end]
    extent = route_document["render_extent"]
    if not samples or any(row["surface"] != extent for row in route_frames):
        raise ValueError(f"{name}: route lacks samples or changed its surface")
    if replay:
        for sample in samples:
            routed_tick = sample["tick"] - start
            if any(abs(routed_tick - step) <= 4 for step in SCALE_STEPS):
                continue
            scale = next((percent for step, percent in reversed(SCALE_STEPS.items())
                          if routed_tick >= step), 100)
            expected_extent = [(axis * scale + 50) // 100 for axis in extent]
            if sample["render_extent"] != expected_extent:
                raise ValueError(f"{name}: replay diverged at routed tick {routed_tick}")
        graph_traffic = prepared["graph_attachment_load_store_bytes"]
        scales = [measured_scale(scale, samples, graph_traffic) for scale in SCALES]
        for row in scales:
            if row["gpu_ns"].get("surface present") is None or (
                    reconstruction == "metalfx" and
                    row["gpu_ns"].get("MetalFX temporal", "unmeasured") == "unmeasured"):
                raise ValueError(f"{name}: missing GPU timings at {row['scale']}%")
    else:
        scales = []
        if not any(sample["quality"] is not None for sample in samples):
            raise ValueError(f"{name}: the governor was inactive")
    in_route = [tick for tick in ticks if route_frames[0]["sequence"] <= tick["sequence"]
                <= route_frames[-1]["sequence"]]
    presents = sorted(row["present_media"] for row in route_frames if row["presented"])
    report = {key: prepared[key] for key in ("adapter", "recipe", "registry_digest", "reconstruction",
                                             "registry_default_reconstructor")}
    report.update(name=name, surface=extent, drawable_format=configured[0]["format"],
                  forced_headroom=headroom, load=observed_load, scales=scales,
                  creation_counters=f"{name}/creations.txt",
                  graph_declarations=f"{name}/prepared.json",
                  governor_trace=f"{name}/frames.jsonl", display_ticks=f"{name}/display.jsonl",
                  display_intervals_ms=distribution([(b - a) * 1000 for a, b in zip(presents, presents[1:])]),
                  ticks_without_present=sum(tick["outcome"] != "presented" for tick in in_route),
                  route_display_ticks=len(in_route))
    return report


def float_cost(sdr, floating):
    rows = []
    for base, half in zip(sdr["scales"], floating["scales"], strict=True):
        baseline = base["gpu_ns"]["surface present"]["p50"]
        float_time = half["gpu_ns"]["surface present"]["p50"]
        rows.append(dict(scale=base["scale"], sdr_p50_ns=baseline, float_p50_ns=float_time,
                         float_minus_sdr_ns=float_time - baseline))
    return dict(scope="Apple Tile and MetalFX in both arms; forced SDR display record at H=1, "
                      "float arm retains a float drawable using potential headroom",
                surface_present_pass=rows)


def mac_fullscreen_motion(output):
    binary, env, route = scale_environment(output, counters=True)
    displays = json.loads(subprocess.check_output(["system_profiler", "SPDisplaysDataType", "-json"]))
    screen, = [screen for gpu in displays["SPDisplaysDataType"]
               for screen in gpu.get("spdisplays_ndrvs", [])
               if screen.get("spdisplays_connection_type") == "spdisplays_internal"]
    extent = [int(value) for value in screen["_spdisplays_pixels"].split(" x ")]
    route_path = output / "fullscreen-route.json"
    route_path.write_text(json.dumps(dict(route, render_extent=extent)) + "\n")
    with awake_display():
        require_unlocked_display()
        control = window_arm(output, binary, env, route_path, "capture-off", "apple-tile",
                             "metalfx", "2", replay=False, fullscreen=True)
        capture = window_arm(output, binary, env, route_path, "capture-on", "apple-tile",
                             "metalfx", "2", replay=False, fullscreen=True, capture=True)
    return dict(target="mac-metal", timebase="realtime", extent=extent,
                binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
                observer_only=control, capture=capture)


def mac_paired(output):
    binary, env, route = scale_environment(output, counters=True)
    route_path = output / "moving-1080p.json"
    route_path.write_text(json.dumps(dict(route, render_extent=EXTENT)) + "\n")
    arms = []
    with awake_display():
        require_unlocked_display()
        for repeat in (1, 2):
            for name, recipe, reconstructor, headroom in (
                ("apple", "apple-tile", "metalfx", "2"),
                ("portable", "portable", "taau", "sdr"),
            ):
                arms.append(window_arm(output, binary, env, route_path, f"{repeat}-{name}",
                                       recipe, reconstructor, headroom))
        sdr = window_arm(output, binary, env, route_path, "sdr", "apple-tile", "metalfx", "sdr")
        floating = window_arm(output, binary, env, route_path, "float-h1", "apple-tile", "metalfx", "1")
        governor = window_arm(output, binary, env, route_path, "governor", "apple-tile",
                              "metalfx", "2", replay=False)
    colour = {}
    for recipe in RECIPE:
        reference = output / f"reference-{recipe}"
        reference.mkdir()
        colour[recipe] = scale_reference(reference, env, metalfx=recipe == "apple-tile")
    return dict(target="mac-metal", system=platform.platform(),
                binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
                route_sha256=hashlib.sha256(MOVING_ROUTE.read_bytes()).hexdigest(),
                scale_trace=SCALE_STEPS, arms=arms, governor=governor,
                drawable_controls=[sdr, floating], float_drawable_cost_at_h1=float_cost(sdr, floating),
                fixed_reference_extent=[640, 360], fixed_reference_colour=colour)
