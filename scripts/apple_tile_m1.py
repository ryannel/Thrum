#!/usr/bin/env python3
"""Qualify prepared APPLE-TILE recipes against retained M0 Metal bytes.

Run under the repository GPU lock with a draw-probe/creation-counters binary.
Outputs must be new and remain under the task's owned scratch directory.
"""
import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path
import subprocess

from apple_tile_m0 import ROOT, ROUTE, digest, run_native, run_t2, finish_t2, save, validate_store
from compare_v1_pixels import compare, load_pixels

BASELINE = ROOT / 'docs/evidence/render/apple-tile-m0/native-baseline'


def retained_comparison(directory, candidate, reference, encoding, extent, name):
    raw = directory / (name + '.reference')
    pixels = load_pixels(reference, 'png', extent)[1] if reference.suffix == '.png' else (gzip.decompress(reference.read_bytes()) if reference.suffix == '.gz' else reference.read_bytes())
    raw.write_bytes(pixels)
    result = compare(raw, candidate, encoding, extent, directory / 'comparisons' / name)
    result['retained_reference'] = str(reference.relative_to(ROOT))
    result['retained_sha256'] = digest(reference)
    if not result['equal']:
        raise ValueError(f'exact retained comparison failed: {name}')
    return result


def run(args):
    args.binary = args.binary.resolve()
    args.store = args.store.resolve()
    args.output = args.output.resolve()
    route = json.loads(ROUTE.read_text())
    validate_store(args.store, route)
    args.output.mkdir(parents=True, exist_ok=False)
    sources = subprocess.check_output(['git', 'ls-files', '-co', '--exclude-standard', 'crates', 'examples', 'scripts', 'Cargo.lock'], text=True).splitlines()
    report = {'passed': False, 'revision': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
              'diff_sha256': hashlib.sha256(subprocess.check_output(['git', 'diff', 'HEAD'])).hexdigest(),
              'binary_sha256': digest(args.binary), 'route_sha256': digest(ROUTE),
              'source_files': {p: digest(ROOT / p) for p in sources if (ROOT / p).is_file()},
              'runs': {}, 'comparisons': {}, 'failures': []}
    previous = os.environ.get('THRUM_RECIPE')
    try:
        for recipe in ['portable', 'apple-tile', 'portable']:
            repeat = 2 if recipe == 'portable' and 'portable-sdr' in report['runs'] else 1
            os.environ['THRUM_RECIPE'] = recipe
            for source, encoding, baseline in [('sdr', 'rgba8', 'sdr-1'), ('scene-linear', 'rgba16f', 'scene-linear-1')]:
                name = f'{recipe}-{source}' + ('-repeat' if repeat == 2 else '')
                report['runs'][name] = run_native(args, args.output, name, route, source=source)
                expected = 'AppleTile' if recipe == 'apple-tile' else 'Portable'
                if report['runs'][name]['header']['recipe'] != f'Ok({expected})':
                    raise ValueError('recipe identity mismatch')
                for checkpoint in route['checkpoints']:
                    key = checkpoint['name']
                    identity = json.loads((args.output / name / 'checkpoints' / f'{key}.identity.json').read_text())
                    retained_identity = json.loads((BASELINE / baseline / f'{key}.identity.json').read_text())
                    if identity != retained_identity:
                        raise ValueError(f'retained sample/exposure identity mismatch: {name}-{key}')
                    report['comparisons'][f'{name}-{key}'] = retained_comparison(args.output,
                        args.output / name / 'checkpoints' / f'{key}.{encoding}',
                        BASELINE / baseline / (f'{key}.png' if encoding == 'rgba8' else f'{key}.{encoding}.gz'), encoding, route['render_extent'], f'{name}-{key}')
                save(args.output / 'manifest.json', report)
        for recipe in ['portable', 'apple-tile']:
            os.environ['THRUM_RECIPE'] = recipe
            for time, params in [('sun', 'showcase-sunset-clear-sun'), ('night', 'showcase-night-upslope')]:
                for source, encoding in [('sdr', 'rgba8'), ('linear', 'rgba16f')]:
                    name = f'{recipe}-{time}-{source}'
                    options = {'mode': 'still', 'params': ROOT / f'examples/v1-game/params/{params}.params.json'}
                    if source == 'linear':
                        options['source'] = 'scene-linear'
                    report['runs'][name] = run_t2(args, args.output, name, options)
                    report['runs'][name].update(finish_t2(args.output / name))
                    reference = BASELINE / f'{time}-{source}' / f'checkpoint-0.{encoding}.gz'
                    if encoding == 'rgba8':
                        reference = BASELINE / f'{time}-{source}' / 'checkpoint-0.png'
                    report['comparisons'][name] = retained_comparison(args.output,
                        args.output / name / f'checkpoint-0.{encoding}', reference, encoding, [1920, 1080], name)
                    save(args.output / 'manifest.json', report)
            for case in ['variants', 'route']:
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
