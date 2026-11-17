#!/usr/bin/env python3
"""Run the capture-off exit mesh sizing experiment on already built targets."""
import argparse
import json
import math
import os
import subprocess
import sys
from pathlib import Path

from run_v1_showcase import COOK, REPO, ROUTE, SOURCE, WASM, git, sha256, store_root, write_json

CANDIDATES = [48, 224, 704]
SCRATCH = REPO / 'tmp/showcase-m5'
EXAMPLE = REPO / 'target/release/examples/exit_sweep'


def prepare():
    from shutil import copyfile

    for tier in range(len(CANDIDATES)):
        source = SCRATCH / f'candidate-{tier}'
        source.mkdir(parents=True, exist_ok=True)
        for path in SOURCE.iterdir():
            if path.suffix in ('.gltf', '.bin', '.png'):
                copyfile(path, source / path.name)
        subprocess.run([sys.executable, str(SOURCE / 'generate_exit.py'), '--stand-ins',
                        str(source), '--tier', str(tier)], check=True)
        subprocess.run([str(COOK), 'cook', str(source), str(SCRATCH / f'store-{tier}')], check=True)


def trials(target, run, web_root):
    output = SCRATCH / f'{target}-{run}'
    output.mkdir()
    commit = git('rev-parse', 'HEAD')
    if git('status', '--porcelain'):
        raise ValueError('commit the sizing build before recording evidence')
    executable = EXAMPLE if target == 'mac-metal' else web_root / WASM.name
    build = sha256(executable)
    rows = []
    for tier, triangles in enumerate(CANDIDATES):
        for recipe in ('portable', 'apple-tile') if target == 'mac-metal' else ('portable',):
            store = SCRATCH / f'store-{tier}'
            stem = f'{recipe}-{triangles}'
            log_path = output / f'{stem}.log'
            load_start = os.getloadavg()
            if target == 'mac-metal':
                command = [str(EXAMPLE), recipe]
                env = dict(os.environ, THRUM_SCENE='exit', THRUM_ASSET_STORE=str(store), THRUM_RECIPE=recipe)
            else:
                mac = json.loads((SCRATCH / f'mac-metal-{run}' / 'result.json').read_text())
                paired = next(row for row in mac if row['recipe'] == recipe and row['triangles'] == triangles)
                command = ['node', 'scripts/check_v1_browser.mjs', '--case', 'exit-sweep',
                           '--cook', str(COOK), '--web-root', str(web_root), '--assets-source',
                           str(SCRATCH / f'candidate-{tier}'), '--output', str(output / stem)]
                env = dict(os.environ, THRUM_EXIT_SWEEP_STOP=paired['stop'])
            with log_path.open('w') as log:
                subprocess.run(command, cwd=REPO, env=env, stdout=log, stderr=subprocess.STDOUT,
                               check=True, timeout=180)
            if target == 'mac-metal':
                header, *samples = map(json.loads, log_path.read_text().splitlines())
                assert header['recipe'] == {'portable': 'Portable', 'apple-tile': 'AppleTile'}[recipe]
                counts = {row['stop']: row['in_view'] for row in samples[:len(ROUTE)]}
                stop = max(ROUTE, key=counts.__getitem__)
                assert all(row['measured_stop'] == stop for row in samples[len(ROUTE):])
                data = samples[len(ROUTE):]
            else:
                record = json.loads((output / stem / 'run.json').read_text())['results'][0]
                assert record['actualWasmSha256'] == build
                assert store_root(output / stem / 'store') == store_root(store)
                stop, counts = paired['stop'], paired['stop_counts']
                data = record['exit']['samples']
                assert all(row['in_view'] == counts[stop] for row in data)
            values = sorted(row['gpu_ns'] for row in data)
            assert len(values) >= 120 and min(values) > 0
            p95 = values[math.ceil(.95 * len(values)) - 1] / 1e6
            rows.append({'target': target, 'run': run, 'commit': commit, 'binary_sha256': build,
                         'cooked_root': store_root(store), 'recipe': recipe, 'triangles': triangles,
                         'stop': stop, 'stop_counts': counts, 'extent': [1920, 1080],
                         'load_start': load_start, 'load_end': os.getloadavg(),
                         'capture': 'off', 'samples': data, 'p95_ms': p95})
            write_json(output / 'result.json', rows)
            print(f'{target} run {run} {recipe} {stop}: {triangles} triangles, p95 {p95:.3f} ms', flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('target', choices=['prepare', 'mac-metal', 'chrome-webgpu'])
    parser.add_argument('--run', type=int, choices=[1, 2])
    parser.add_argument('--web-root', type=Path, default=WASM.parent)
    args = parser.parse_args()
    if args.target == 'prepare':
        prepare()
    else:
        if args.run is None:
            parser.error('--run 1 or 2 is required on a target')
        trials(args.target, args.run, args.web_root)


if __name__ == '__main__':
    main()
