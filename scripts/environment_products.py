#!/usr/bin/env python3
"""ATMOSPHERE M5 target parity and paired cost. Run under agent_loop.py lock gpu."""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
import tarfile
import time

ROOT = Path(__file__).resolve().parents[1]
BASE = json.loads((ROOT / 'docs/evidence/ATMOSPHERE/m5-h2/base.json').read_text())['base_commit']
TASK = ROOT / 'tmp/atmosphere-m5'


def run(command, cwd=ROOT, env=None):
    subprocess.run([str(value) for value in command], cwd=cwd, env=env, check=True)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def build_base():
    source = TASK / 'base'
    if not source.exists():
        source.mkdir(parents=True)
        archive = subprocess.check_output(['git', 'archive', BASE], cwd=ROOT)
        with tarfile.open(fileobj=io.BytesIO(archive)) as tree:
            tree.extractall(source, filter='data')
    env = dict(os.environ, RUSTC_WRAPPER='')
    run(['rustup', 'run', '1.99.1', 'cargo', 'build', '--locked', '--offline', '-p', 'v1-game', '-p', 'thrum-cook'], source, env)
    return source


def native_stills(binary, store, output):
    presets = sorted((ROOT / 'examples/v1-game/params').glob('showcase-*.params.json'))
    env = dict(os.environ, THRUM_ASSET_STORE=str(store), THRUM_SCENE='valley',
               THRUM_CAPTURE_SCENE=str(output), THRUM_CAPTURE_ROUTE_ONLY='1',
               THRUM_CAPTURE_PARAMS=os.pathsep.join(str(path) for path in presets))
    env.pop('THRUM_DIAGNOSTIC_CLOUDS', None)
    run([binary], env=env)
    manifest = json.loads((output / 'captures.json').read_text())
    if manifest['adapter'] != 'Apple M3 Max' or manifest.get('cloud_shade', 'off') != 'off':
        raise RuntimeError('Parity requires the recorded Metal adapter and no provider')
    return {path.name: digest(path) for path in sorted((output / 'route').glob('*.ppm'))}


def equal_route(first, second):
    return bool(first) and first == second


def off_parity(target, output):
    base = build_base()
    env = dict(os.environ, RUSTC_WRAPPER='')
    run(['rustup', 'run', '1.99.1', 'cargo', 'build', '--locked', '--offline', '-p', 'v1-game', '-p', 'thrum-cook'], env=env)
    store = TASK / 'store'
    if not (store / 'manifest.json').exists():
        run([ROOT / 'target/debug/thrum-cook', 'cook', ROOT / 'examples/v1-game/assets/showcase', store])
    if target == 'chrome-webgpu':
        run(['sh', 'examples/v1-game/web/build.sh'], base, env)
        run(['sh', 'examples/v1-game/web/build.sh'], env=env)
        env['THRUM_ENVIRONMENT_BASE_WEB'] = str(base / 'target/v1-game-web')
        run(['node', 'scripts/check_v1_browser.mjs', '--case', 'environment-off-parity', '--output', output / 'browser'], env=env)
        return
    shader_files = [
        'crates/thrum-render/src/synthetic.wgsl',
        'crates/thrum-render/src/synthetic_sun.wgsl',
        'crates/thrum-render/src/synthetic_aerial.wgsl',
        'crates/thrum-render/src/synthetic_environment.wgsl',
        'crates/thrum-asset-gpu/src/cooked.wgsl',
    ]
    sources = {name: digest(ROOT / name) for name in shader_files}
    for name in shader_files:
        if (ROOT / name).read_bytes() != (base / name).read_bytes():
            raise RuntimeError(f'No-provider shader source differs at {name}')
    marker = '/// The forward WGSL for a scene'
    composer = 'crates/thrum-render/src/synthetic/scene_source.rs'
    if (ROOT / composer).read_text().split(marker)[1] != (base / composer).read_text().split(marker)[1]:
        raise RuntimeError('No-provider composer differs from the recorded base')
    arms = {}
    for name, binary in [('base', base / 'target/debug/v1-game'), ('claim', ROOT / 'target/debug/v1-game'), ('rerun', ROOT / 'target/debug/v1-game')]:
        arms[name] = {'binary_sha256': digest(binary), 'images': native_stills(binary, store, output / name)}
    if not equal_route(arms['base']['images'], arms['claim']['images']) or not equal_route(arms['claim']['images'], arms['rerun']['images']):
        raise RuntimeError('No-provider route bytes differ. Inspect every arm before claiming parity.')
    # Exercise the identical comparator with a one-byte perturbation of an actual source image.
    sample_name = next(iter(arms['claim']['images']))
    sample = (output / 'claim/route' / sample_name).read_bytes()
    changed = bytearray(sample)
    changed[-1] ^= 1
    perturbed = dict(arms['claim']['images'])
    perturbed[sample_name] = hashlib.sha256(changed).hexdigest()
    negative_rejected = not equal_route(arms['claim']['images'], perturbed)
    if not negative_rejected:
        raise RuntimeError('Parity negative control passed incorrectly')
    result = {'target': target, 'base': BASE, 'claim': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
              'arms': arms, 'shader_sources': sources, 'same_build_control': True, 'one_byte_negative_rejected': negative_rejected,
              'comparison': 'Complete engine-tap PPM bytes, zero tolerance, zero masks, same cooked store'}
    (output / 'parity.json').write_text(json.dumps(result, indent=2) + '\n')


def cost(target, output):
    if target == 'chrome-webgpu':
        run(['sh', 'examples/v1-game/web/build.sh'], env=dict(os.environ, RUSTC_WRAPPER=''))
        run(['node', 'scripts/check_v1_browser.mjs', '--case', 'environment-cost', '--output', output / 'browser'])
        return
    run(['rustup', 'run', '1.99.1', 'cargo', 'build', '--locked', '--offline', '--release', '-p', 'v1-game', '-p', 'thrum-cook'], env=dict(os.environ, RUSTC_WRAPPER=''))
    store = TASK / 'store'
    if not store.exists():
        run([ROOT / 'target/release/thrum-cook', 'cook', ROOT / 'examples/v1-game/assets/showcase', store])
    arms = []
    for index, provider in enumerate(['off', 'on', 'on', 'off']):
        directory = output / f'{index:02}-{provider}'
        env = dict(os.environ, THRUM_DIAGNOSTIC_CLOUDS=provider)
        run(['python3', 'scripts/capture_v1_runtime.py', '--output', directory, '--binary', 'target/release/v1-game', '--store', store, '--mode', 'realtime', '--defer'], env=env)
        manifest = json.loads((directory / 'manifest.json').read_text())
        runtime = json.loads((directory / 'runtime.json').read_text())
        if not runtime['complete'] or runtime['diagnostic_clouds'] != (provider == 'on'):
            raise RuntimeError('Incomplete or wrongly configured cost arm')
        limit = runtime['max_sampled_textures_per_shader_stage']
        counts = {name: count + (provider == 'on') for name, count in json.loads((ROOT / 'docs/evidence/ATMOSPHERE/m5-h2/base.json').read_text())['sampled_texture_counts'].items()}
        if any(count > limit for count in counts.values()):
            raise RuntimeError('Forward pipeline exceeds recorded device texture limit')
        arms.append({'provider': provider, 'order': index, 'manifest': manifest, 'gpu': runtime['gpu'],
                     'sampled_texture_counts': counts, 'capability_limit': limit})
    (output / 'cost.json').write_text(json.dumps({'target': target, 'arms': arms, 'load_class': 'exclusive shared GPU lock',
                                               'scope': 'Paired order-alternated observations, capture on, no performance gate'}, indent=2) + '\n')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('case', choices=['off-parity', 'cost'])
    parser.add_argument('--target', required=True, choices=['mac-metal', 'chrome-webgpu'])
    args = parser.parse_args()
    TASK.mkdir(parents=True, exist_ok=True)
    output = TASK / f'{args.case}-{args.target}-{time.time_ns()}'
    output.mkdir()
    (off_parity if args.case == 'off-parity' else cost)(args.target, output)
    print(f'PASS {args.case} {args.target} {output}')


if __name__ == '__main__':
    main()
