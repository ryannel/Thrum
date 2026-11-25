"""Qualify the quality governor against the authored route on Mac Metal."""

from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
from statistics import median

from apple_presentation import ROOT, mac_binary, read_rows, run
from run_v1_showcase import MOVING_ROUTE, SOURCE, store_root

LOAD = (230, 320)
ROUNDS = 4600
WINDOW_LOAD_ROUNDS = (1000, 1200, 1800, 2600)
PRELOAD_TICKS = (200, LOAD[0])
THERMAL_EXTENT = (960, 540)
LIVE_EXTENT = (1920, 1080)
MISSED_OPPORTUNITY_TOLERANCE = 2
CADENCE = "atmosphere.aerial cadence"


def environment(output):
    binary, env = mac_binary(cook=True, counters=True)
    store = output / "store"
    run(str(ROOT / "target/release/thrum-cook"), "cook", str(SOURCE), str(store))
    route = json.loads(MOVING_ROUTE.read_text())
    if store_root(store) != route["cooked_root"]:
        raise ValueError("cooked store does not match the moving route")
    env.update(THRUM_ASSET_STORE=str(store), THRUM_SCENE="valley",
               THRUM_RECIPE="portable", THRUM_RECONSTRUCTION="taau",
               THRUM_ROUTE_OVERLAY=route["overlay"])
    return binary, env, route


def sample_changes(samples):
    changes = []
    for sample in samples:
        quality = sample["quality"]
        if quality is None:
            continue
        state = (quality["scale"], quality["tier"])
        if not changes or changes[-1][0] != state:
            changes.append((state, sample))
    return [sample for _, sample in changes]


@contextmanager
def awake_display():
    # Wake a sleeping display once and hold it across the arms and retries.
    # Bind the assertion to this process so an interrupted proof also releases it.
    keep_awake = subprocess.Popen(
        ["/usr/bin/caffeinate", "-d", "-i", "-u", "-w", str(os.getpid())])
    try:
        yield
    finally:
        keep_awake.terminate()
        keep_awake.wait()

def require_unlocked_display():
    console = subprocess.check_output(
        ["/usr/sbin/ioreg", "-n", "Root", "-d1"], text=True, timeout=15)
    if re.search(r'"(?:CGSSessionScreenIsLocked|IOConsoleLocked)"\s*=\s*Yes', console):
        raise InvalidDisplayEnvironment(
            "Mac console session is locked; the visible route needs an unlocked display")


def window_arm(output, binary, env, route, name, *, enabled=True, load=0, thermal=False):
    directory = output / name
    directory.mkdir()
    route_path = output / "window-route.json"
    route_path.write_text(json.dumps(dict(route, render_extent=LIVE_EXTENT)))
    launch = dict(env, THRUM_GOVERNOR="on" if enabled else "off",
                  THRUM_GOVERNOR_LOAD=str(load), THRUM_GOVERNOR_ROUTE=str(route_path),
                  THRUM_GOVERNOR_RECORD="thermal" if thermal else "observed",
                  THRUM_MAC_TRANSITION=str(directory), THRUM_MAC_TRANSITION_ARM="baseline",
                  THRUM_V1_FRAMES="1440" if enabled and load else "1040")
    with (directory / "game.log").open("w") as log:
        result = subprocess.run([str(binary)],
                                cwd=ROOT, env=launch, stdout=log, stderr=subprocess.STDOUT,
                                check=False, timeout=100)
    if result.returncode:
        raise ValueError(f"Mac window could not finish {name}: {(directory / 'game.log').read_text()}")
    game = (directory / "game.log").read_text()
    if "adapter=Apple M3 Max" not in game or "x".join(map(str, LIVE_EXTENT)) not in game:
        raise ValueError(f"wrong running Metal target in {directory}")
    rows = read_rows(directory / "frames.jsonl")
    display = read_rows(directory / "display.jsonl")
    route_end = route["origin_tick"] + route["keys"][-1]["tick"]
    if not rows or not display or rows[-1]["world_tick"] < max(route_end, LOAD[1] + 180):
        raise ValueError(f"Metal window did not complete the route in {directory}")
    if any(tuple(row["surface"]) != LIVE_EXTENT for row in rows):
        raise InvalidDisplayEnvironment(
            f"{directory}: drawable extent changed during the ordinary window route")
    samples = [sample for row in rows for sample in row["governor"]["samples"]]
    if not samples or any((sample["quality"] is not None) != enabled for sample in samples):
        raise ValueError(f"governor enabled state or sample coverage is wrong in {directory}")
    if not any(sample["gpu_ns"] for sample in samples):
        raise ValueError(f"missing Metal GPU timestamps in {directory}")
    if load and not any(sample["debug_load_ns"] for sample in samples):
        raise ValueError(f"missing GPU work from the debug load in {directory}")
    return dict(
        directory=str(directory), rows=rows, samples=samples, changes=sample_changes(samples),
        display_hz=round(1 / median(row["target"] - row["media"] for row in display), 2),
        proxy="Display-link opportunities with no render-owner present call (render-side proxy)",
    )


def miss_rate(arm, first, until):
    rows = [row for row in arm["rows"] if first <= row["world_tick"] < until]
    if len(rows) < 12:
        raise ValueError(f"missing display opportunities in {arm['directory']} at {first}..{until}")
    start, end = rows[0], rows[-1]
    count = end["governor"]["missed_present_proxy"] - start["governor"]["missed_present_proxy"]
    ticks = end["sequence"] - start["sequence"]
    if ticks <= 0:
        raise ValueError("display sequence did not advance")
    return dict(missed=count, ticks=ticks, rate=count / ticks)


def exceeds_baseline(arm, baseline):
    # Two opportunities of measurement tolerance at window boundaries.
    return arm["rate"] > baseline["rate"] + MISSED_OPPORTUNITY_TOLERANCE / arm["ticks"]


class InvalidDisplayEnvironment(ValueError):
    """Unloaded display delivery could not support a forced-load comparison."""


def steady_miss_rate(arm, first, until):
    measured = miss_rate(arm, first, until)
    if measured["missed"] > MISSED_OPPORTUNITY_TOLERANCE:
        raise InvalidDisplayEnvironment(
            f"{arm['directory']}: unloaded baseline missed too many opportunities "
            f"at ticks {first}..{until}: {measured}")
    return measured


def matched_preload(arm, steady_arm, reference):
    if abs(arm["display_hz"] - steady_arm["display_hz"]) > steady_arm["display_hz"] * 0.03:
        raise InvalidDisplayEnvironment(
            f"{arm['directory']}: display interval differs from the unloaded run "
            f"({arm['display_hz']} Hz versus {steady_arm['display_hz']} Hz)")
    measured = miss_rate(arm, *PRELOAD_TICKS)
    if exceeds_baseline(measured, reference) or exceeds_baseline(reference, measured):
        raise InvalidDisplayEnvironment(
            f"{arm['directory']}: unloaded delivery {measured} versus {reference}")
    return measured


def validate_recovery(changes, *, scale=False):
    field = "scale" if scale else "tier"
    recoveries = []
    levels = [100, 85, 75, 67, 50] if scale else [0, 1, 2]
    for previous, row in zip(changes, changes[1:]):
        if row["quality"]["reason"] != "Recovery":
            continue
        if previous["quality"][field] == row["quality"][field]:
            continue
        if levels.index(previous["quality"][field]) - levels.index(row["quality"][field]) != 1:
            raise ValueError(f"recovery skipped a prepared {field} step")
        if row["observation_ns"] - previous["observation_ns"] < 1_000_000_000:
            raise ValueError(f"recovery changed {field} within its one-second minimum period")
        recoveries.append(row)
    if not recoveries:
        raise ValueError(f"governor never recovered its {field}")
    return recoveries


def mac_governor(output, *, steady=False, capture=False):
    binary, env, route = environment(output)
    if capture and not steady:
        return mac_governor_capture(output, binary, env, route)

    with awake_display():
        return measure_window(output, binary, env, route, steady=steady)


def measure_window(output, binary, env, route, *, steady):
    require_unlocked_display()
    if steady:
        arm = window_arm(output, binary, env, route, "steady")
        last_tick = route["origin_tick"] + route["keys"][-1]["tick"]
        changes = [row for row in arm["changes"] if row["tick"] <= last_tick]
        if not changes:
            raise ValueError("no recorded quality")
        count = sum(a["quality"]["scale"] != b["quality"]["scale"]
                    for a, b in zip(changes, changes[1:]))
        if count > 2:
            raise ValueError(f"steady route changed scale {count} times in {arm['directory']}")
        return dict(target="mac-metal", extent=LIVE_EXTENT, scale_changes=count, display_hz=arm["display_hz"],
                    changes=changes, binary_sha256=digest(binary))

    interrupted = []
    for number in range(1, 4):
        attempt = output / f"attempt-{number}"
        attempt.mkdir()
        try:
            report = compare_live_load(attempt, binary, env, route)
        except InvalidDisplayEnvironment as error:
            interrupted.append(str(error))
        else:
            return dict(report, invalid_display_attempts=interrupted)
    raise InvalidDisplayEnvironment(
        f"invalid display environment in all forced-load attempts: {interrupted}")


def compare_live_load(output, binary, env, route):
    steady_arm = window_arm(output, binary, env, route, "steady")
    first = LOAD[0] + 30
    baseline = steady_miss_rate(steady_arm, first, LOAD[1])
    preload = {"steady": steady_miss_rate(steady_arm, *PRELOAD_TICKS)}
    controls = {}
    # Select load strength using only the ungoverned run. A GPU that still
    # presents on time at the first setting cannot witness a return to rate.
    for rounds in WINDOW_LOAD_ROUNDS:
        off_arm = window_arm(output, binary, env, route, f"loaded-off-{rounds}",
                             enabled=False, load=rounds)
        preload["loaded-off"] = matched_preload(off_arm, steady_arm, preload["steady"])
        controls[rounds] = miss_rate(off_arm, first, LOAD[1])
        if exceeds_baseline(controls[rounds], baseline):
            break
    else:
        raise ValueError(f"no governor-off load exceeded the baseline: {baseline=}, {controls=}")

    arms = {
        "steady": steady_arm,
        "loaded": window_arm(output, binary, env, route, "loaded", load=rounds),
        "loaded-off": off_arm,
    }
    preload["loaded"] = matched_preload(arms["loaded"], steady_arm, preload["steady"])
    changes = arms["loaded"]["changes"]
    if not changes:
        raise ValueError("missing governed frame samples")
    samples = arms["loaded"]["samples"]
    loaded = [sample for sample in samples if sample["debug_load_ns"]]
    previous = next(sample for sample in reversed(samples) if sample["tick"] < LOAD[0])
    starting_scale = previous["quality"]["scale"]
    drop = next((row for row in changes
                 if row["tick"] >= LOAD[0] and row["quality"]["scale"] < starting_scale), None)
    if drop is None or drop["tick"] > loaded[0]["tick"] + 12:
        raise ValueError(f"no panic step in the routed load window, first step {drop}")
    recoveries = validate_recovery(changes, scale=True)
    if not any(row["tick"] >= LOAD[1] and row["quality"]["scale"] >= starting_scale
               for row in recoveries):
        raise ValueError(f"scale did not recover at least to its starting step {starting_scale}")
    # Keep the same settled interval for all arms and count every later miss,
    # including those near any further quality adjustment.
    comparison = {name: miss_rate(arm, first, LOAD[1]) for name, arm in arms.items()}
    baseline, governed, disabled = (comparison[name] for name in ("steady", "loaded", "loaded-off"))
    if (exceeds_baseline(governed, baseline)
            or not exceeds_baseline(disabled, baseline)
            or disabled["rate"] <= governed["rate"]):
        raise ValueError(f"loaded/off did not falsify a return to baseline proxy rate: {comparison}")
    return dict(target="mac-metal", extent=LIVE_EXTENT, comparison=comparison,
                comparison_ticks=[first, LOAD[1]],
                changes=changes, load=LOAD,
                rounds=rounds, control_trials=controls, preload=preload, preload_ticks=PRELOAD_TICKS,
                proxy=arms["loaded"]["proxy"],
                display_hz=arms["loaded"]["display_hz"],
                arm_display_hz={name: arm["display_hz"] for name, arm in arms.items()},
                binary_sha256=digest(binary))


def mac_governor_capture(output, binary, env, route):
    width, height = LIVE_EXTENT
    rounds = WINDOW_LOAD_ROUNDS[0]
    route_path = output / "capture-route.json"
    route_path.write_text(json.dumps(dict(route, render_extent=LIVE_EXTENT)))
    arms = {}
    for name in ("tap-off", "tap-on"):
        directory = output / name
        flags = ["--observer-only"] if name == "tap-off" else []
        run("python3", "scripts/capture_v1_runtime.py", "--binary", str(binary),
            "--store", env["THRUM_ASSET_STORE"], "--route", str(route_path),
            "--case", "route", "--mode", "realtime", "--extent", str(width), str(height),
            "--reconstruction", "taau", "--recipe", "portable", "--output", str(directory),
            *flags, env=dict(env, THRUM_GOVERNOR="on", THRUM_GOVERNOR_LOAD=str(rounds),
                             THRUM_CAPTURE_ENCODING="raw"), timeout=180)
        runtime = json.loads((directory / "runtime.json").read_text())
        manifest = json.loads((directory / "manifest.json").read_text())
        if (runtime["backend"] != "Metal" or manifest["timebase"] != "realtime"
                or tuple(runtime["extent"]) != LIVE_EXTENT or runtime["requested_hz"] != 60):
            raise ValueError("wrong capture target or timebase")
        if not manifest["complete"]:
            raise ValueError(f"incomplete realtime capture in {directory}")
        quality = runtime["governor"]
        loaded = [row for row in quality if LOAD[0] <= row["tick"] < LOAD[1]]
        initial = next((row["scale"] for row in reversed(quality) if row["tick"] < LOAD[0]), None)
        if initial is None or not loaded or min(row["scale"] for row in loaded) >= initial:
            raise ValueError(f"real-time governor did not lower under load in {directory}")
        restored = next((row for row in quality if row["tick"] >= LOAD[1]
                         and row["scale"] >= initial), None)
        if restored is None:
            raise ValueError(f"real-time governor did not recover in {directory}")
        motion_end = restored["tick"] + 60
        opportunities = runtime["opportunities"]
        bounds = [next((index for index, row in enumerate(opportunities)
                        if row["tick"] is not None and row["tick"] >= tick), None)
                  for tick in (LOAD[0], motion_end)]
        first, until = bounds
        motion = opportunities[first:until]
        if first is None or until is None or not motion or any(
                not row["submitted"] or row["skipped_before"] for row in motion):
            raise ValueError(f"missing render opportunities through load and recovery in {directory}")
        timings = [row for row in opportunities if row["tick"] is not None
                   and 330 <= row["tick"] < 420 and row["submitted"]]
        arms[name] = dict(
            directory=str(directory), submitted=len(quality), captured=len(manifest["records"]),
            motion_ticks=[LOAD[0], motion_end], motion_opportunities=len(motion),
            step_thread_median_ms=median(row["step_thread_cpu_ns"] for row in timings) / 1e6,
            step_wall_median_ms=median(row["step_wall_ns"] for row in timings) / 1e6,
            drain_wall_median_ms=median(row["drain_wall_ns"] for row in timings) / 1e6,
            idle_poll_thread_median_ms=median(
                (row.get("idle_gpu_progress") or {}).get("cpu_ns", 0) for row in timings) / 1e6,
        )
    return dict(target="mac-metal", capture_mode="realtime", extent=LIVE_EXTENT,
                requested_hz=60, rounds=rounds, binary_sha256=digest(binary), arms=arms)


def mac_governor_identity(output):
    binary, env, _route = environment(output)
    arms = {}
    for name, enabled, load in (("off", False, 0), ("on", True, 0), ("loaded", True, ROUNDS)):
        directory = output / name
        launch = dict(env, THRUM_CAPTURE_SCENE=str(directory), THRUM_CAPTURE_ROUTE=str(MOVING_ROUTE),
                      THRUM_GOVERNOR="on" if enabled else "off", THRUM_GOVERNOR_LOAD=str(load),
                      THRUM_GOVERNOR_IDENTITY="1")
        run(str(binary), env=launch, timeout=300)
        header = read_rows(directory / "route.jsonl")[0]
        if header["backend"] != "Metal" or "Apple M3 Max" not in header["adapter"]:
            raise ValueError(f"wrong identity target: {header}")
        arms[name] = json.loads((directory / "authority.json").read_text())
    if (len({(arm["tick"], arm["hash"]) for arm in arms.values()}) != 1
            or arms["loaded"]["loaded_frames"] == 0
            or arms["loaded"]["minimum_scale"] >= 100
            or arms["off"]["loaded_frames"] or arms["on"]["loaded_frames"]):
        raise ValueError(f"authority or forced-load witness mismatch: {arms}")
    return dict(target="mac-metal", arms=arms, binary_sha256=digest(binary))


def mac_governor_thermal(output):
    binary, env, route = environment(output)
    directory = output / "thermal"
    route_path = output / "thermal-route.json"
    route_path.write_text(json.dumps(dict(route, render_extent=THERMAL_EXTENT)))
    launch = dict(env, THRUM_CAPTURE_SCENE=str(directory), THRUM_CAPTURE_ROUTE=str(route_path),
                  THRUM_GOVERNOR_RECORD="thermal")
    run(str(binary), env=launch, timeout=300)
    header = read_rows(directory / "route.jsonl")[0]
    if header["backend"] != "Metal" or "Apple M3 Max" not in header["adapter"]:
        raise ValueError(f"wrong thermal target: {header}")
    trace = json.loads((directory / "thermal.json").read_text())
    states = sample_changes(trace["samples"])
    changes = [row for index, row in enumerate(states)
               if index == 0 or row["quality"]["tier"] != states[index - 1]["quality"]["tier"]]
    actual = [(row["quality"]["tier"], row["quality"]["knobs"][CADENCE],
               row["quality"]["reason"], row["quality"]["thermal"]) for row in changes]
    expected = [(0, 1, "Initial", "Nominal"), (1, 2, "Thermal", "Serious"),
                (2, 4, "Thermal", "Serious"), (1, 2, "Recovery", "Nominal"),
                (0, 1, "Recovery", "Nominal")]
    if actual != expected:
        raise ValueError(f"thermal record did not degrade and restore effect tier: {actual}")
    validate_recovery(changes)
    pipelines = trace["pipelines"]
    if pipelines is None or pipelines[0] != pipelines[1] or pipelines[1][1] != 0:
        raise ValueError(f"thermal record created a pipeline: {pipelines}")
    if not any(sample["quality"]["aerial_age"] and sample["quality"]["tier"] == 2
               for sample in trace["samples"]):
        raise ValueError("aerial cadence did not age its retained output")
    return dict(target="mac-metal", changes=changes, pipeline_counts=pipelines,
                platform_thermal=trace["platform_thermal"], scope="fixed route on the Metal frame host",
                binary_sha256=digest(binary))


def digest(path: Path):
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()
