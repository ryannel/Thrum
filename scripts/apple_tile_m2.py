#!/usr/bin/env python3
"""Qualify M2 letterbox bytes and skipped-view motion, under the GPU lock.

Run apple_tile_m1.py separately for the unchanged complete M0 comparison matrix.
This adds both retained surface format controls and T2 realtime stall recovery.
"""
import argparse
import json
import os
from pathlib import Path

from apple_tile_m0 import ROOT, ROUTE, digest, finish_t2, run_native, run_t2, save, validate_store
from apple_tile_m1 import BASELINE, retained_comparison


def run(args):
    args.binary = args.binary.resolve()
    args.store = args.store.resolve()
    args.output = args.output.resolve()
    route = json.loads(ROUTE.read_text())
    validate_store(args.store, route)
    args.output.mkdir(parents=True, exist_ok=False)
    report = {'passed': False, 'binary_sha256': digest(args.binary), 'runs': {}, 'comparisons': {}, 'failures': []}
    previous = os.environ.get('THRUM_RECIPE')
    try:
        for recipe in ['portable', 'apple-tile']:
            os.environ['THRUM_RECIPE'] = recipe
            for format in ['bgra8', 'bgra8-srgb']:
                name = f'{recipe}-{format}'
                report['runs'][name] = run_native(args, args.output, name, route, extra={
                    'THRUM_CAPTURE_SURFACE_FORMAT': format, 'THRUM_CAPTURE_SURFACE_EXTENT': '1000x800'})
                for checkpoint in [c for c in route['checkpoints'] if c['name'] == 'mirror-light']:
                    key = checkpoint['name']
                    candidate_identity = json.loads((args.output / name / 'checkpoints' / f'{key}.identity.json').read_text())
                    reference_identity = json.loads((BASELINE / f'letterbox-{format}-1' / f'{key}.identity.json').read_text())
                    if candidate_identity != reference_identity:
                        raise ValueError(f'{name} surface sample identity differs')
                    report['comparisons'][f'{name}-{key}'] = retained_comparison(args.output,
                        args.output / name / 'checkpoints' / f'{key}.surface-bgra8',
                        BASELINE / f'letterbox-{format}-1' / f'{key}.surface-bgra8.gz',
                        'bgra8', [1000, 800], f'{name}-{key}')
                save(args.output / 'manifest.json', report)
            for case in ['render-stall', 'route']:
                name = f'{recipe}-{case}-motion'
                options = {'mode': 'realtime', 'case': case}
                if case == 'route':
                    path = args.output / f'{recipe}-runtime-route.json'
                    save(path, dict(route, render_extent=[1920, 1080]))
                    options['route'] = path
                report['runs'][name] = run_t2(args, args.output, name, options)
                report['runs'][name].update(finish_t2(args.output / name, viewing=True))
                save(args.output / 'manifest.json', report)
        report['passed'] = True
    except Exception as error:
        report['failures'].append(str(error))
    finally:
        if previous is None:
            os.environ.pop('THRUM_RECIPE', None)
        else:
            os.environ['THRUM_RECIPE'] = previous
        save(args.output / 'manifest.json', report)
    print(json.dumps({'passed': report['passed'], 'failures': report['failures']}))
    return 0 if report['passed'] else 1


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, default=Path('target/release/v1-game'))
    parser.add_argument('--store', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    raise SystemExit(run(parser.parse_args()))
