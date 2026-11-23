#!/usr/bin/env python3
"""Run the view volume's paired target checks, or capture its inspection takes."""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import shutil
import statistics
import subprocess
import sys
import tarfile
import tempfile


ROOT = Path(__file__).resolve().parent.parent
BASE_FIXTURE = ROOT / 'crates/thrum-render/tests/data/view-volume-base.json'
TARGETS = ('mac-metal', 'chrome-webgpu')
LOW_SUN = 12 + 12 / math.pi * math.acos(math.sqrt(2) * math.sin(math.radians(10)))
LOOKOUT_EYE = [0, 244.9226672381398, 16]
# The cooked terrain triangle under x=0, z=16 is at 239.925172 m.
NEAR_EYE = [0, 241.92517205618816, 16]
SHAFT_EYE = [390, 115, 230]
EXTENT = [1280, 720]
AERIAL = 'atmosphere aerial perspective'


def run(*command, cwd=ROOT, env=None, log=None):
    if log is None:
        subprocess.run(command, cwd=cwd, env=env, check=True)
    else:
        with log.open('wb') as output:
            subprocess.run(command, cwd=cwd, env=env, stdout=output,
                           stderr=subprocess.STDOUT, check=True)


def read(path):
    return json.loads(path.read_text())


def save(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def git_commit():
    return subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()


def build(output, targets, cost=False):
    release = ROOT / 'target/release'
    run('cargo', 'build', '--release', '--locked', '--offline',
        '-p', 'v1-game', '-p', 'thrum-cook', '--bin', 'v1-game', '--bin', 'thrum-cook',
        log=output / 'native-build.log')
    if cost and 'mac-metal' in targets:
        run('cargo', 'build', '--release', '--locked', '--offline',
            '-p', 'v1-game', '--example', 'atmosphere_cost', log=output / 'cost-build.log')
    if 'chrome-webgpu' in targets:
        run('sh', 'examples/v1-game/web/build.sh', log=output / 'chrome-build.log')
    store = output / 'store'
    run(release / 'thrum-cook', 'cook', ROOT / 'examples/v1-game/assets/showcase', store,
        log=output / 'cook.log')
    return release / 'v1-game', release / 'thrum-cook', ROOT / 'target/v1-game-web', store


def witness_baseline_archive(base, output):
    # This probe adds a read-only observation after preparation. The base's
    # rendering code, assets, scene defaults and shaders come from the archive.
    def insert(relative, marker, text):
        path = output / relative
        source = path.read_text()
        if source.count(marker) != 1:
            raise ValueError(f'archive probe marker moved: {relative}')
        before, after = source.split(marker, 1)
        path.write_text(f'{before}{text}\n{marker}{after}')

    with subprocess.Popen(['git', 'archive', base], cwd=ROOT, stdout=subprocess.PIPE) as process:
        with tarfile.open(fileobj=process.stdout, mode='r|') as archive:
            archive.extractall(output, filter='data')
        if process.wait():
            raise ValueError('could not extract the recorded base')
    insert('crates/thrum-render/src/synthetic.rs', '    pub fn scene_pass_count(&self) -> u32 {',
           '    #[doc = "Read the prepared shader for the parity check."]\n'
           '    pub(crate) fn scene_shader_source(&self) -> &str { &self.surface.shader_source }')
    insert('crates/thrum-render/src/runtime.rs', '    pub fn scene_graph(&self)',
           '    #[doc = "Read the prepared shader for the parity check."]\n'
           '    pub fn scene_shader_source(&self) -> Option<&str> {\n'
           '        Some(self.prepared.three_d()?.scene_shader_source())\n    }')
    baseline_runtime = output / 'crates/thrum-render/src/runtime.rs'
    before = baseline_runtime.read_text()
    graph_slot = '''        self.prepared
            .graph(&self.slots[0].target, capture)
            .map_err(|error| FrameError::gpu("graph inspection", error))'''
    if before.count(graph_slot) != 1:
        raise ValueError('archive graph probe marker moved')
    baseline_runtime.write_text(before.replace(graph_slot, '''        self.slots.iter()
            .find_map(|slot| self.prepared.graph(&slot.target, capture).ok())
            .ok_or_else(|| FrameError::gpu("graph inspection", RenderError::StaleTarget))'''))
    insert('examples/v1-game/src/browser.rs', '    pub fn set_cloud_capture_eye(',
           '    #[doc = "Read the prepared shader and resource graph."]\n'
           '    pub fn renderer_signature(&self) -> String {\n'
           '        let host = self.page.shell.host();\n'
           '        serde_json::json!({"shader":host.scene_shader_source(),\n'
           '            "graph":host.scene_graph().map(|graph|format!("{graph:?}")).ok()}).to_string()\n'
           '    }')
    insert('examples/v1-game/src/window/capture.rs', '    let (scene_kind, present_target_bytes, storage)',
           '    if std::env::var_os("THRUM_CAPTURE_RENDERER_SIGNATURE").is_some() {\n'
           '        let host = driver.host();\n'
           '        result["renderer_signature"] = json!({"shader": host.scene_shader_source(),\n'
           '            "graph": format!("{:?}", host.scene_graph().map_err(|err|err.to_string())?)});\n'
           '    }')
    baseline_capture = output / 'examples/v1-game/src/window/capture.rs'
    before = baseline_capture.read_text()
    begin = 'fn warmup<G: Game>(driver: &mut Offscreen<G>) -> Result<(), String> {'
    witness_begin = 'fn warmup<G: Game>(driver: &mut Offscreen<G>, parity_checkpoint: bool) -> Result<(), String> {'
    end = '\nfn capture_settings()'
    current = (ROOT / baseline_capture.relative_to(output)).read_text()
    warmup = current.split(witness_begin, 1)[1].split(end, 1)[0]
    start, original = before.split(begin, 1)
    baseline_capture.write_text(start + witness_begin + warmup + end + original.split(end, 1)[1])
    before = baseline_capture.read_text()
    call = '    warmup(driver)?;'
    if before.count(call) != 1:
        raise ValueError('archive warmup marker moved')
    baseline_capture.write_text(before.replace(call, '''    warmup(driver,
        crate::content::launch_value("THRUM_CAPTURE_RENDERER_SIGNATURE").as_deref() == Ok("1"))?;'''))


def checkpoint(output, target, builds, pose, haze=None, cloud=False, signature=False, motion=None,
               cost=False, assets_source=None):
    native, cook, web, store = builds
    mode = 'realtime' if motion else 'fixed'
    scenario = 'ground-pan' if motion else 'checkpoint'
    if cost:
        scenario = 'cost'
    params = output.with_suffix('.params.json')
    save(params, {'world.paused': True, 'sky.solar_time': round(pose['solar_hours'], 2),
                  'camera.yaw': pose['yaw_degrees'], 'camera.pitch': 0})
    common = ['--mode', mode, '--params', params, '--output', output,
              '--overlay', 'off', '--reconstruction', 'taau']
    if target == 'mac-metal':
        env = dict(os.environ)
        if haze:
            env.update(THRUM_HAZE=haze, THRUM_HAZE_TAKE=json.dumps(pose))
        else:
            env.pop('THRUM_HAZE', None)
        if cloud:
            env['THRUM_DIAGNOSTIC_CLOUDS'] = 'on'
        if signature:
            env['THRUM_CAPTURE_RENDERER_SIGNATURE'] = '1'
        run(sys.executable, 'scripts/capture_v1_runtime.py', '--binary', native,
            '--store', store, '--case', f'atmosphere-{scenario}' if motion else scenario,
            '--extent', *map(str, EXTENT), *common, env=env, log=output.with_suffix('.log'))
    else:
        extra = []
        if haze:
            take = output.with_suffix('.take.json')
            save(take, pose)
            extra += ['--haze', haze, '--haze-take', take]
        if cloud:
            extra += ['--diagnostic-clouds', 'on']
        if signature:
            extra += ['--renderer-signature', 'on']
        if cost:
            extra += ['--control', 'off']
        if assets_source:
            extra += ['--assets-source', assets_source]
        run('node', 'scripts/capture_v1_browser.mjs', '--atmosphere', scenario,
            '--cook', cook, '--web-root', web, *common, *extra, log=output.with_suffix('.log'))
        warnings = (output / 'chrome.log').read_text()
        if 'GPUDevice:' in warnings:
            raise ValueError(f'Chrome WebGPU reported validation errors: {output / "chrome.log"}')
        if not cost:
            run(sys.executable, 'scripts/package_v1_browser.py', '--output', output,
                log=output.with_suffix('.package.log'))
    return read(output / 'runtime.json'), read(output / 'manifest.json')


def off_parity(output, targets, builds):
    native, cook, _, store = builds
    base = read(BASE_FIXTURE)['commit']
    output = output.resolve()
    source = output / 'base'
    source.mkdir()
    witness_baseline_archive(base, source)
    base_target = output / 'base-target'
    env = dict(os.environ, CARGO_TARGET_DIR=str(base_target))
    if 'mac-metal' in targets:
        run('cargo', 'build', '--release', '--locked', '--offline', '-p', 'v1-game',
            '--bin', 'v1-game', cwd=source, env=env, log=output / 'base-native-build.log')
    if 'chrome-webgpu' in targets:
        run('sh', 'examples/v1-game/web/build.sh', cwd=source, env=env,
            log=output / 'base-web-build.log')
        base_web = base_target / 'v1-game-web'
        shutil.copyfile(ROOT / 'examples/v1-game/web/capture.mjs', base_web / 'capture.mjs')
    assets = source / 'examples/v1-game/assets/showcase'
    base_store = output / 'base-store'
    run(cook, 'cook', assets, base_store, log=output / 'base-cook.log')
    base_builds = (base_target / 'release/v1-game', cook, base_target / 'v1-game-web', base_store)
    pose = {'eye': LOOKOUT_EYE, 'solar_hours': 12.0, 'yaw_degrees': 0, 'motion': None}
    report = {'base': base, 'commit': git_commit(), 'targets': {}}
    for target in targets:
        prior, reference = checkpoint(output / f'{target}-base', target, base_builds, pose,
                                      signature=True, assets_source=assets)
        current, result = checkpoint(output / f'{target}-current', target, builds, pose, signature=True)
        before, after = prior['renderer_signature'], current['renderer_signature']
        if not before['shader'] or not before['graph'] or before != after:
            raise ValueError(f'{target}: composed shader or graph changed with the view volume omitted')
        first = [row['rgba_sha256'] for row in reference['records']]
        second = [row['rgba_sha256'] for row in result['records']]
        if not first or first != second:
            raise ValueError(f'{target}: submitted output changed with the view volume omitted')
        report['targets'][target] = {
            'frame_hashes': first,
            **{f'{key}_sha256': hashlib.sha256(value.encode()).hexdigest() for key, value in before.items()},
        }
        save(output / 'off-parity.json', report)
    print(json.dumps(report))
    shutil.rmtree(base_target)
    shutil.rmtree(source)


def cost(output, targets, builds):
    _, _, _, store = builds
    pose = {'eye': LOOKOUT_EYE, 'solar_hours': LOW_SUN, 'yaw_degrees': -65, 'motion': None}
    params = output / 'cost.params.json'
    save(params, {'world.paused': True, 'sky.solar_time': round(LOW_SUN, 2),
                  'camera.yaw': -65, 'camera.pitch': 0})
    report = {'commit': git_commit(), 'load_class': 'shared builder load', 'targets': {}}
    for target in targets:
        arms = {}
        for label, haze in [('off', None), ('on', '20,0')]:
            name = output / f'{target}-{label}'
            load_before = os.getloadavg()
            if target == 'mac-metal':
                env = dict(os.environ, THRUM_SCENE='valley', THRUM_RECIPE='portable',
                           THRUM_ASSET_STORE=str(store), THRUM_RECONSTRUCTION='taau')
                if haze:
                    env['THRUM_HAZE'] = haze
                else:
                    env.pop('THRUM_HAZE', None)
                log = name.with_suffix('.json')
                run(ROOT / 'target/release/examples/atmosphere_cost', params, env=env, log=log)
                arm = read(log)
                arm['frames'] = [row for row in arm['frames'] if row['resident']]
            else:
                runtime, _ = checkpoint(name, target, builds, pose, haze=haze, cost=True)
                arm = runtime['haze_cost']
                arm['frames'] = arm['frames'][-64:]
            arm.update(load_before=load_before, load_after=os.getloadavg())
            spans = [row['frame_gpu_ns'] for row in arm['frames']]
            if len(spans) < 24 or any(not isinstance(v, (int, float)) or not math.isfinite(v) or v <= 0 for v in spans):
                raise ValueError(f'{target} {label}: missing whole-frame timestamps')
            if label == 'on':
                if not arm['view_volume'] or any(AERIAL not in row['pass_gpu_ns'] for row in arm['frames']):
                    raise ValueError(f'{target}: the live view volume has no named timestamps or allocation')
                stats = [row['pass_gpu_ns'][AERIAL] for row in arm['frames']]
                arm['view_volume_pass_median_ns'] = statistics.median(stats)
                arm['view_volume_pass_p95_ns'] = statistics.quantiles(stats, n=20)[18]
            elif arm['view_volume']:
                raise ValueError(f'{target}: baseline allocated the view volume')
            arm['frame_median_ns'] = statistics.median(spans)
            arm['frame_p95_ns'] = statistics.quantiles(spans, n=20)[18]
            arms[label] = arm
        report['targets'][target] = arms
        save(output / 'cost.json', report)
    print(f"Capture-off paired target timings and allocations saved to {output / 'cost.json'}")


def inspection(output, targets, builds, group):
    for target in targets:
        target_root = output / target
        target_root.mkdir()
        if group in ('all', 'haze'):
            for height in [2, 40]:
                for hour, sun in [(12, 'noon'), (LOW_SUN, 'low')]:
                    pose = {'eye': [NEAR_EYE[0], NEAR_EYE[1] + height - 2, NEAR_EYE[2]],
                            'solar_hours': hour, 'yaw_degrees': 0, 'motion': None}
                    for km, kind in [(300, 0), (60, 0), (20, 0), (5, 0), (20, 1.3), (5, 1.3)]:
                        checkpoint(target_root / f'haze-{sun}-{height}m-{km}km-{kind}', target,
                                   builds, pose, haze=f'{km},{kind}')
        if group in ('all', 'shafts'):
            for yaw, name in [(-90, 'toward'), (-42, 'across')]:
                for cloud in [False, True]:
                    pose = {'eye': SHAFT_EYE, 'solar_hours': LOW_SUN, 'yaw_degrees': yaw, 'motion': None}
                    checkpoint(target_root / f'shaft-{name}-{cloud}', target, builds, pose,
                               haze='20,0', cloud=cloud)
        if group in ('all', 'motion'):
            for motion in ['pan', 'walk', 'scrub', 'cut']:
                pose = {'eye': SHAFT_EYE, 'solar_hours': LOW_SUN, 'yaw_degrees': -65, 'motion': motion}
                checkpoint(target_root / motion, target, builds, pose, haze='20,0', cloud=True, motion=motion)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('operation', choices=['off-parity', 'cost', 'inspect'])
    parser.add_argument('--scene', choices=['haze'], default='haze')
    parser.add_argument('--target', choices=TARGETS, action='append')
    parser.add_argument('--group', choices=['all', 'haze', 'shafts', 'motion'], default='all')
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    parent = ROOT / 'tmp/atmosphere-view-volume'
    parent.mkdir(parents=True, exist_ok=True)
    output = args.output.resolve() if args.output else Path(tempfile.mkdtemp(prefix=args.operation + '-', dir=parent))
    if not output.is_relative_to((ROOT / 'tmp').resolve()):
        parser.error('use an output folder under this worktree’s tmp directory')
    if args.output:
        output.mkdir()
    targets = args.target or TARGETS
    print(f'View volume artifacts: {output}', flush=True)
    builds = build(output, targets, cost=args.operation == 'cost')
    save(output / 'builds.json', {'commit': git_commit(), 'targets': targets, 'native_sha256': sha(builds[0]),
                                 'chrome_sha256': sha(builds[2] / 'v1_game_bg.wasm') if 'chrome-webgpu' in targets else None})
    if args.operation == 'off-parity':
        off_parity(output, targets, builds)
    elif args.operation == 'cost':
        cost(output, targets, builds)
    else:
        inspection(output, targets, builds, args.group)


if __name__ == '__main__':
    main()
