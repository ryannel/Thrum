#!/usr/bin/env python3
"""Metal proofs for cooked ray geometry and fixed traced/raster valley pairs."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
from tempfile import TemporaryDirectory


ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / "tests/fixtures/ray-tracing"
VIEWS = (
    ("valley", 0, 0),
    ("contact", 0, -13),
    ("west", -12, -8),
    ("east", 13, -8),
)


def command(*args):
    return subprocess.check_output(args, cwd=ROOT, text=True).strip()


def run_test(case, *, release=False, extra_env=None):
    args = ["rustup", "run", "stable", "cargo", "test", "--locked", "--offline"]
    if release:
        args.append("--release")
    args.extend(["-p", "v1-game", "--features", "draw-probe", "--test", "ray_tracing_gpu",
                 "--", f"ray_tracing::{case}", "--exact", "--nocapture"])
    if release:
        args.append("--ignored")
    result = subprocess.run(
        args, cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
        env=dict(os.environ, THRUM_REQUIRE_GPU="1", **(extra_env or {})),
    )
    if result.returncode:
        print(result.stdout, end="")
        result.check_returncode()
    rows = [json.loads(line.split("ray proof ", 1)[1]) for line in result.stdout.splitlines()
            if "ray proof " in line]
    if not rows or "1 passed" not in result.stdout:
        raise ValueError(f"{case} did not run and record evidence on Metal: {result.stdout}")
    return rows


def proof(case):
    reference = None
    if case == "demand":
        scratch = ROOT / "tmp/ray-tracing-m2"
        scratch.mkdir(parents=True, exist_ok=True)
        with TemporaryDirectory(prefix="demand-", dir=scratch) as directory:
            pixels = Path(directory) / "ordinary.rgba"
            rows = run_test(case, extra_env={"THRUM_RAY_DEMAND_BYTES": str(pixels)})
            sha256 = hashlib.sha256(pixels.read_bytes()).hexdigest()
        reference = json.loads((EVIDENCE / "demand-main.json").read_text())
        if sha256 != reference["rgba_sha256"]:
            raise ValueError(f"no-demand bytes differ from {reference['revision']}: {sha256}")
        rows[0]["rgba_sha256"] = sha256
    else:
        rows = run_test(case)
    result = {"revision": command("git", "rev-parse", "HEAD"), "target": "mac-metal",
              "proof": case, "observations": rows}
    if reference:
        result["main_reference"] = reference
    return result


def capture(output, binary, store):
    output.mkdir(parents=True, exist_ok=True)
    report = []
    for view, yaw, pitch in VIEWS:
        params = output / f"{view}.params.json"
        params.write_text(json.dumps({
            "world.paused": True, "sky.solar_time": 12, "camera.yaw": yaw,
            "camera.pitch": pitch,
        }) + "\n")
        pair = {"view": view, "yaw_degrees": yaw, "pitch_degrees": pitch, "frames": {}}
        for mode in ("drawn", "traced"):
            name = f"{view}-{mode}"
            directory = output / name
            env = dict(os.environ, THRUM_HARDWARE="ray-query", THRUM_RECIPE="apple-tile",
                       THRUM_TRACED_VIEW="on" if mode == "traced" else "off",
                       THRUM_ROUTE_OVERLAY="proof", THRUM_VALLEY_MESHES="on")
            subprocess.run(
                ["python3", "scripts/capture_v1_runtime.py", "--binary", str(binary),
                 "--store", str(store), "--output", str(directory), "--params", str(params),
                 "--case", "checkpoint", "--mode", "fixed", "--extent", "1280", "720"],
                cwd=ROOT, env=env, check=True, stdout=subprocess.DEVNULL,
            )
            manifest = json.loads((directory / "manifest.json").read_text())
            runtime = json.loads((directory / "runtime.json").read_text())
            if not (manifest["complete"] and runtime["mode"] == "fixed" and
                    "EXPERIMENTAL_RAY_QUERY" in runtime["enabled_features"] and
                    runtime["backend"] == "Metal" and runtime["recipe"] == "Ok(AppleTile)"):
                raise ValueError(f"incomplete or wrong capture device: {directory}")
            record, = manifest["records"]
            source = directory / "png" / f"{record['submission_id']:06}.png"
            shutil.copyfile(source, output / f"{name}.png")
            pair["frames"][mode] = {
                "file": f"{name}.png", **record, "adapter": runtime["adapter"],
                "backend": runtime["backend"], "recipe": runtime["recipe"],
                "feature": "ray-query required", "binary_sha256": manifest["binary_sha256"],
                "cooked_products_sha256": manifest["cooked_hashes"],
            }
        if pair["frames"]["drawn"]["rgba_sha256"] == pair["frames"]["traced"]["rgba_sha256"]:
            raise ValueError(f"the traced display selection did not take effect for {view}")
        report.append(pair)
    return {"revision": command("git", "rev-parse", "HEAD"), "mode": "fixed",
            "extent": [1280, 720], "views": report}


def shadow_capture(output, binary, exit_store, lamp_store, motion):
    """Use the authored exit route for Sun shadows and the valley for local lights."""
    route_path = ROOT / "examples/v1-game/routes/exit-flythrough.json"
    route = json.loads(route_path.read_text())
    output.mkdir(parents=True, exist_ok=True)
    rows = []

    def render(name, store, case, solar, trace_sun, lamps, eye=None, yaw=0, pitch=0):
        mode = "realtime" if motion else "fixed"
        directory = output / name
        params = output / f"{name}.params.json"
        params.write_text(json.dumps({
            "world.paused": True, "sky.solar_time": solar, "camera.yaw": yaw, "camera.pitch": pitch,
        }) + "\n")
        env = dict(os.environ, THRUM_HARDWARE="ray-query", THRUM_RECIPE="apple-tile",
                   THRUM_SUN_SHADOW_METHOD="traced" if trace_sun else "maps",
                   THRUM_LAMP_SHADOWS="on" if lamps else "off")
        if store == lamp_store:
            env["THRUM_ROUTE_OVERLAY"] = "lamp-stop"
        if eye is not None:
            env["THRUM_CAPTURE_FIXED_EYE"] = json.dumps(eye)
        capture_args = [
            "python3", "scripts/capture_v1_runtime.py", "--binary", str(binary),
            "--store", str(store), "--output", str(directory), "--params", str(params),
            "--case", case, "--mode", mode, "--extent", "1280", "720",
        ]
        if not motion:
            capture_args.extend(["--scene", "exit" if store == exit_store else "valley"])
        if motion:
            capture_args.extend(["--route", str(route_path), "--defer"])
        if not (directory / "manifest.json").exists():
            subprocess.run(capture_args, cwd=ROOT, env=env, check=True)
        if motion:
            return
        manifest = json.loads((directory / "manifest.json").read_text())
        runtime = json.loads((directory / "runtime.json").read_text())
        resolved = "Traced" if trace_sun else "Maps"
        selections = [row["shadow_selection"] for row in runtime["images"] if "shadow_selection" in row]
        if not selections:
            selections = [row["shadow_selection"] for row in runtime["gpu"]]
        if not (manifest["complete"] and runtime["mode"] == "fixed"
                and runtime["backend"] == "Metal" and runtime["recipe"] == "Ok(AppleTile)"
                and "EXPERIMENTAL_RAY_QUERY" in runtime["enabled_features"]
                and selections and all(row["sun"] == resolved and row["lamps"] == lamps
                                       and row["fallback"] is None for row in selections)):
            raise ValueError(f"wrong device or shadow selection: {name}")
        record = manifest["records"][-1]
        image = f"{name}.png"
        shutil.copyfile(directory / "png" / f"{record['submission_id']:06}.png", output / image)
        rows.append({"view": name, "solar_hour": solar, "eye": eye, "yaw": yaw, "pitch": pitch,
                     "adapter": runtime["adapter"], "registry_digest": runtime["registry_digest"],
                     "recipe": runtime["recipe"], "selection": selections[-1],
                     "binary_sha256": manifest["binary_sha256"], "record": record, "image": image,
                     "source_revision": manifest["source_revision"], "source_digest": manifest["build_source_digest"]})

    if motion:
        render("route", exit_store, "exit-route", route["start"]["solar_hours"], True, False)
        return None
    stops = [("contact", route["keys"][0])]
    for stop in route["checkpoints"]:
        tick = stop["tick"]
        before, after = next((a, b) for a, b in zip(route["keys"], route["keys"][1:])
                             if a["tick"] <= tick <= b["tick"])
        alpha = (tick - before["tick"]) / (after["tick"] - before["tick"])
        def lerp(a, b):
            return a + (b - a) * alpha
        stops.append((stop["name"], {
            "eye": [lerp(a, b) for a, b in zip(before["eye"], after["eye"])],
            "yaw_degrees": lerp(before["yaw_degrees"], after["yaw_degrees"]),
            "pitch_degrees": lerp(before["pitch_degrees"], after["pitch_degrees"]),
        }))
    for solar, time in ((12, "noon"), (17.4, "low"), (17.9, "dusk")):
        for stop, key in stops:
            for traced in (False, True):
                method = "traced" if traced else "maps"
                render(f"{time}-{stop}-{method}", exit_store, "checkpoint", solar, traced, False,
                       key["eye"], key["yaw_degrees"], key["pitch_degrees"])
    for lamps in (False, True):
        render("lamps-on" if lamps else "lamps-off", lamp_store, "checkpoint", 0, False, lamps,
               eye=[0.0, 242.4, 2.5], yaw=0, pitch=-18)
    return {"revision": command("git", "rev-parse", "HEAD"), "mode": "fixed",
            "extent": [1280, 720], "views": rows}


def stops():
    route = json.loads((ROOT / "examples/v1-game/routes/exit-flythrough.json").read_text())
    poses = {"contact": route["keys"][0]}
    for stop in route["checkpoints"]:
        tick = stop["tick"]
        before, after = next(
            (a, b) for a, b in zip(route["keys"], route["keys"][1:])
            if a["tick"] <= tick <= b["tick"]
        )
        fraction = (tick - before["tick"]) / (after["tick"] - before["tick"])

        def between(a, b):
            return a + fraction * (b - a)

        poses[stop["name"]] = {
            "eye": [between(a, b) for a, b in zip(before["eye"], after["eye"])],
            **{key: between(before[key], after[key])
               for key in ("yaw_degrees", "pitch_degrees")},
        }
    return poses


def capture_soft(args):
    args.output.mkdir(parents=True, exist_ok=True)
    directory = args.output / args.name
    params = args.output / f"{args.name}.params.json"
    pose = stops()[args.stop] if args.eye is None else {
        "eye": args.eye, "pitch_degrees": args.pitch, "yaw_degrees": args.yaw,
    }
    params.write_text(json.dumps({
        "world.paused": True, "sky.solar_time": args.solar,
        "camera.yaw": pose["yaw_degrees"], "camera.pitch": pose["pitch_degrees"],
    }) + "\n")
    env = dict(os.environ, THRUM_HARDWARE="ray-query", THRUM_RECIPE="apple-tile",
               THRUM_SUN_SHADOW_METHOD="traced", THRUM_SUN_PENUMBRA=args.penumbra,
               THRUM_CAPTURE_RENDERER_SIGNATURE="1", THRUM_CAPTURE_FIXED_EYE=json.dumps(pose["eye"]),
               THRUM_SUN_PROOF_GEOMETRY=args.geometry)
    cmd = [
        "python3", "scripts/capture_v1_runtime.py", "--binary", str(args.binary),
        "--store", str(args.store), "--output", str(directory), "--params", str(params),
        "--scene", "exit", "--case", args.scenario, "--mode", args.mode,
        "--extent", "1280", "720", "--source", args.source,
    ]
    if args.route:
        from run_v1_showcase import store_root

        env.pop("THRUM_CAPTURE_FIXED_EYE", None)
        route = json.loads((ROOT / "examples/v1-game/routes/exit-flythrough.json").read_text())
        route["cooked_root"] = store_root(args.store)
        route_file = args.output / f"{args.name}.route.json"
        route_file.write_text(json.dumps(route, indent=2) + "\n")
        cmd.extend(["--route", str(route_file)])
    if args.verify_only:
        cmd.append("--verify-only")
    subprocess.run(cmd, cwd=ROOT, env=env, check=True)
    runtime = json.loads((directory / "runtime.json").read_text())
    records = runtime["images"]
    assert runtime["mode"] == args.mode and runtime["complete"], directory
    assert "renderer_signature" in runtime, directory
    assert runtime["backend"] == "Metal" and runtime["recipe"] == "Ok(AppleTile)", directory
    assert "EXPERIMENTAL_RAY_QUERY" in runtime["enabled_features"], directory
    assert records and all(row["shadow_selection"]["sun"] == "Traced" for row in records), directory
    expected = args.penumbra == "on"
    assert runtime.get("sun_penumbra", {}).get("enabled", False) == expected, directory


def soft_capture(argv):
    parser = argparse.ArgumentParser(description="Metal T2 Sun penumbra samples")
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/v1-game")
    parser.add_argument("--store", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--name", required=True)
    parser.add_argument("--stop", choices=stops(), default="contact")
    parser.add_argument("--solar", type=float, default=17.4)
    parser.add_argument("--penumbra", choices=("on", "off"), default="on")
    parser.add_argument("--mode", choices=("fixed", "realtime"), default="fixed")
    parser.add_argument("--scenario", default="checkpoint")
    parser.add_argument("--route", action="store_true")
    parser.add_argument("--verify-only", action="store_true")
    parser.add_argument("--source", choices=("sdr", "scene-linear"), default="sdr")
    parser.add_argument("--eye", type=float, nargs=3)
    parser.add_argument("--yaw", type=float, default=0)
    parser.add_argument("--pitch", type=float, default=0)
    parser.add_argument("--geometry", choices=("off", "fast", "ambient", "joined", "separate"), default="off")
    capture_soft(parser.parse_args(argv))



def main():
    if sys.argv[1:2] == ["soft-capture"]:
        soft_capture(sys.argv[2:])
        return
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("case", choices=["edit", "casters", "lifecycle", "demand", "stills",
                                         "contributions", "sun-out", "shadow-stills", "shadow-motion"])
    parser.add_argument("--output", type=Path)
    parser.add_argument("--binary", type=Path, default=Path("target/debug/v1-game"))
    parser.add_argument("--store", type=Path)
    parser.add_argument("--lamp-store", type=Path)
    args = parser.parse_args()
    if args.case == "stills":
        if args.output is None or args.store is None:
            parser.error("stills requires a T2 output directory and the cooked valley store")
        result = capture(args.output, args.binary, args.store)
        destination = args.output / "capture.json"
    elif args.case in ("shadow-stills", "shadow-motion"):
        motion = args.case == "shadow-motion"
        if args.output is None or args.store is None or (not motion and args.lamp_store is None):
            parser.error("shadow capture needs an output folder, an exit store and, for stills, a valley lamp store")
        result = shadow_capture(args.output, args.binary, args.store, args.lamp_store, motion)
        if result is None:
            return
        destination = args.output / "capture.json"
    else:
        result = proof(args.case.replace("-", "_"))
        destination = args.output
    text = json.dumps(result, indent=2) + "\n"
    if destination:
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(text)
    print(text, end="")


if __name__ == "__main__":
    main()
