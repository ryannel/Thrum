#!/usr/bin/env python3
"""Native CREATOR M2 evidence through the ordinary kit and valley frame hosts.

Run under agent_loop.py lock gpu. Uses AppKit's offscreen panel rendering, never desktop
capture. A generated disposable consumer calls only the V1 game's public capture_game and
the engine's completed samples; no V1 implementation is modified for the proof.
Default: elapsed offscreen opportunities and offscreen AppKit panels. --window additionally
exercises the ordinary native window shell and requires that it receive display-link callbacks.
"""
import argparse
import hashlib
import json
import os
import resource
from pathlib import Path
import statistics
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]
RUST = Path(subprocess.check_output(["rustup", "which", "--toolchain", "stable", "rustc"], text=True).strip()).parent
ENV = dict(os.environ, PATH=f"{RUST}:{os.environ['PATH']}", RUSTC_WRAPPER="")


def run(command, **kwargs):
    subprocess.run(command, cwd=ROOT, env=ENV, check=True, **kwargs)


def wait_for(test, label, timeout=60):
    end = time.monotonic() + timeout
    while time.monotonic() < end:
        value = test()
        if value:
            return value
        time.sleep(.1)
    raise RuntimeError(f"Timed out waiting for {label}")


def latest(directory):
    try:
        lines = (directory / "samples.jsonl").read_text().splitlines()
        # The writer may currently be appending the last line.
        for line in reversed(lines):
            try:
                record = json.loads(line)
                if len(record["frames"]) >= 120:
                    return record
            except json.JSONDecodeError:
                pass
    except FileNotFoundError:
        pass
    return None


class Scene:
    def __init__(self, command, directory, env, params):
        self.directory = directory
        directory.mkdir(parents=True)
        self.params = directory / "params.json"
        self.params.write_text(json.dumps(params))
        self.log = (directory / "command.log").open("w")
        usage = resource.getrusage(resource.RUSAGE_CHILDREN)
        self.cpu_before = usage.ru_utime + usage.ru_stime
        self.started = time.monotonic()
        self.process = subprocess.Popen(command, cwd=ROOT, env=dict(ENV, **env, THRUM_PROFILE_OUTPUT=str(directory), THRUM_PARAMS=str(self.params)), stdout=self.log, stderr=self.log, start_new_session=True)
        try:
            wait_for(lambda: latest(directory) if self.process.poll() is None else self.fail(), "native completed samples")
        except BaseException:
            try:
                status = directory / "samples.jsonl"
                if status.exists():
                    lines = status.read_text().splitlines()
                    if lines:
                        record = json.loads(lines[-1])
                        print(json.dumps({"native_startup":{key:record.get(key) for key in ["elapsed","display_link_callbacks","render_callbacks","present_calls"]}}), flush=True)
            except (OSError, json.JSONDecodeError):
                pass
            finally:
                self.close()
            raise

    def fail(self):
        raise RuntimeError(f"Native scene exited {self.process.returncode}: {(self.directory / 'command.log').read_text()[-2000:]}")

    def settle(self, params=None, capture=None):
        if params is not None:
            self.params.write_text(json.dumps(params))
        previous = latest(self.directory)
        boundary = previous["elapsed"]
        last_submission = max(frame["submission"] for frame in previous["frames"])
        # Replace all 120 recent frames and allow hot reload to settle before sampling.
        record = wait_for(lambda: (r if (r := latest(self.directory)) and r["elapsed"] > boundary + 3 and min(frame["submission"] for frame in r["frames"]) > last_submission else None), "settled recent window")
        if capture:
            (self.directory / "capture").write_text(capture)
            wait_for(lambda: (self.directory / f"{capture}.png").exists(), "offscreen native panel")
            record = latest(self.directory)
            (self.directory / f"{capture}.json").write_text(json.dumps(record, indent=2))
        return record

    def close(self):
        if self.process.poll() is None:
            os.killpg(self.process.pid, 15)
        self.process.wait(timeout=20)
        self.log.close()
        usage = resource.getrusage(resource.RUSAGE_CHILDREN)
        return {"process_cpu_s":usage.ru_utime + usage.ru_stime - self.cpu_before, "elapsed_s":time.monotonic() - self.started}


def med(record, field):
    return statistics.median(frame[field] for frame in record["frames"])


def forward(record):
    return statistics.median(next(value for name, value in frame["gpu"] if "forward" in name) for frame in record["frames"])


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("output", type=Path)
    parser.add_argument("--window", action="store_true")
    parser.add_argument("--prepare-only", action="store_true")
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    (output / "ownership.json").write_text(json.dumps({"owner":"CREATOR M2", "purpose":"native profiler proof", "peak":"4 GiB including build outputs", "retain":"selected panel PNGs, summary and sample windows", "closeout":"delete generated consumer and redundant logs after promotion"}))
    harness = output / "consumer"
    (harness / "src").mkdir(parents=True, exist_ok=True)
    (harness / "Cargo.toml").write_text(f'''[package]
name = "creator-profile-proof"
version = "0.0.0"
edition = "2024"
[workspace]
[dependencies]
thrum-app = {{ path = "{ROOT / 'crates/thrum-app'}" }}
v1-game = {{ path = "{ROOT / 'examples/v1-game'}" }}
thrum-render = {{ path = "{ROOT / 'crates/thrum-render'}" }}
thrum-controls = {{ path = "{ROOT / 'crates/thrum-controls'}" }}
thrum-core = {{ path = "{ROOT / 'crates/thrum-core'}" }}
wgpu = {{ version = "=30.0.1", default-features = false, features = ["std", "metal"] }}
pollster = "=1.0.1"
serde_json = "=1.0.154"
''')
    (harness / "src/starter.rs").write_text((ROOT / "examples/starter/src/main.rs").read_text() + '\npub fn profile_game() -> Result<thrum_app::sky::SkyGame, String> { scene::build().map_err(|e| e.to_string()) }\n')
    modules = '\n'.join(f'#[path = "{ROOT / source}"] mod {name};' for name, source in [
        ("window", "crates/thrum-app/src/profile/window.rs"),
        ("recording", "crates/thrum-app/src/app/profile_recording.rs"),
        ("probe", "scripts/creator_profile_probe.rs"),
    ])
    (harness / "src/main.rs").write_text(modules + '''
mod starter;
const TABLE: &str = "profiler.rows";
fn main() -> Result<(), String> {
    if std::env::var_os("THRUM_PROFILE_WINDOW").is_none() {
        return if std::env::var_os("THRUM_PROFILE_STARTER").is_some() {
            probe::run(starter::profile_game()?, [1600,900])
        } else { probe::run(v1_game::capture_game()?, [1920,1080]) };
    }
    thrum_app::run_until_closed(thrum_app::Config {
        name: "creator-profile-proof", frames_variable: "THRUM_PROFILE_FRAMES",
        window_title: "Thrum valley profiler proof", title: "Thrum valley",
        render_thread: "creator-profile-render", panel_title: "Thrum demo",
        window_size: [1920, 1080],
    }, v1_game::capture_game);
    Ok(())
}
''')
    build = ["cargo", "build", "--offline", "--release", "--manifest-path", str(harness / "Cargo.toml"), "--target-dir", str(ROOT / "target")]
    run(build)
    run(["cargo", "build", "--locked", "--offline", "--release", "-p", "thrum-starter", "-p", "thrum-cook"])
    for name, source in [("valley", "examples/v1-game/assets/showcase"), ("starter", "examples/starter/assets")]:
        run([str(ROOT / "target/release/thrum-cook"), "cook", source, str(output / f"{name}-store")])
    if args.prepare_only:
        return
    binary = ROOT / "target/release/creator-profile-proof"
    result = {"revision":subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(), "binary_sha256":hashlib.sha256(binary.read_bytes()).hexdigest(), "load":"V1 valley, 10,000 entities, 1920x1080, ordinary Game/FrameHost, compact observer equal in every arm", "mode":"window" if args.window else "elapsed offscreen output and AppKit panel; no OS present or display-link claim", "order":[], "pairs":[], "world":[], "material":[]}
    result["source_sha256"] = {name:hashlib.sha256((ROOT / name).read_bytes()).hexdigest() for name in ["scripts/check_creator_profiler.py", "scripts/creator_profile_probe.rs", "crates/thrum-app/src/profile/window.rs", "crates/thrum-controls/src/macos.rs", "examples/starter/src/main.rs"]}
    result["cooked_heads_sha256"] = {name:hashlib.sha256((output / f"{name}-store/HEAD").read_bytes()).hexdigest() for name in ["valley", "starter"]}
    valley_env = {"THRUM_SCENE":"valley", "THRUM_ASSET_STORE":str(output / "valley-store")}
    if args.window:
        valley_env["THRUM_PROFILE_WINDOW"] = "1"
    for index, on in enumerate([False, True, True, False, False, True]):
        result["order"].append(on)
        scene = Scene([str(binary)], output / f"pair-{index}", dict(valley_env, THRUM_PROFILE_DETAIL="compact"), {"profiler.enabled":on})
        try:
            record = scene.settle()
            (output / f"pair-{index}.json").write_text(json.dumps(record, indent=2))
            result["pairs"].append({"on":on, "encode_us":med(record,"encode_us"), "render_us":med(record,"render_us"), "tick_us":med(record,"tick_us"), "all_timed":all(f["timed"] for f in record["frames"]), "none_timed":all(not f["timed"] for f in record["frames"])})
            assert all(f["timed"] == on for f in record["frames"]), "shell ignored timing demand"
        finally:
            result["pairs"][-1].update(scene.close())
    scene = Scene([str(binary)], output / "world", valley_env, {"profiler.enabled":True, "world.paused":False})
    try:
        for index in range(3):
            control = scene.settle({"profiler.enabled":True, "world.paused":False}, f"control-{index}")
            before = scene.settle(capture=f"before-{index}")
            paused = scene.settle({"profiler.enabled":True, "world.paused":True}, f"paused-{index}")
            result["world"].append({"control_us":med(control,"tick_us"), "before_us":med(before,"tick_us"), "paused_us":med(paused,"tick_us")})
        scene.settle({"profiler.enabled":False,"world.paused":False}, "off")
    finally:
        scene.close()
    material = output / "material.wgsl"
    light = "fn cooked_surface(surface: CookedSurface, at: CookedSurfacePoint) -> CookedSurface { return surface; }\n"
    heavy = """fn cooked_surface(surface: CookedSurface, at: CookedSurfacePoint) -> CookedSurface {
    var out = surface;
    var value = at.world.x;
    for (var i = 0u; i < 512u; i = i + 1u) { value = sin(value + f32(i) * 0.001); }
    out.base_colour = vec4(surface.base_colour.rgb * (0.9 + 0.1 * value), surface.base_colour.a);
    return out;
}
"""
    material.write_text(light)
    starter_command = [str(ROOT / "target/release/thrum-starter")] if args.window else [str(binary)]
    scene = Scene(starter_command, output / "material", {"THRUM_PROFILE_STARTER":"1", "THRUM_ASSET_STORE":str(output / "starter-store"), "THRUM_MATERIAL":str(material)}, {"profiler.enabled":True})
    try:
        for index in range(3):
            material.write_text(light)
            control = scene.settle(capture=f"control-{index}")
            before = scene.settle(capture=f"before-{index}")
            material.write_text(heavy)
            loaded = scene.settle(capture=f"heavy-{index}")
            result["material"].append({"control_ms":forward(control), "before_ms":forward(before), "heavy_ms":forward(loaded)})
    finally:
        scene.close()
    result["world_spread_us"] = max(v[k] for v in result["world"] for k in ["control_us","before_us"]) - min(v[k] for v in result["world"] for k in ["control_us","before_us"])
    result["material_spread_ms"] = max(v[k] for v in result["material"] for k in ["control_ms","before_ms"]) - min(v[k] for v in result["material"] for k in ["control_ms","before_ms"])
    (output / "summary.json").write_text(json.dumps(result, indent=2))
    assert all(v["before_us"]-v["paused_us"] > result["world_spread_us"] for v in result["world"]), "world change did not exceed unchanged spread"
    assert all(v["heavy_ms"]-v["before_ms"] > result["material_spread_ms"] for v in result["material"]), "material change did not exceed unchanged spread"
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
