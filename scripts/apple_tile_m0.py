#!/usr/bin/env python3
"""Drive APPLE-TILE M0 fixed checkpoints through the existing cooked game.

Build the binary and cooker first. Run this command under the orchestrate GPU
lock. Source archives and all failures remain in a new task output directory.
"""
import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

from compare_v1_pixels import compare
from capture_spool import decoded_frames
from capture_v1_runtime import finalize, save_manifest
from t2_writer_probe import package
from run_v1_showcase import MOVING_IDENTITY, store_root

ROOT = Path(__file__).resolve().parents[1]
ROUTE = ROOT / 'examples/v1-game/routes/moving-v1.json'


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def save(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')


def validate_store(store, route):
    if not store.is_dir():
        raise ValueError('missing cooked store')
    if store_root(store) != route['cooked_root']:
        raise ValueError('cooked root differs from the route')


def run_native(args, directory, name, route, source='sdr', extra=None):
    if shutil.disk_usage(directory).free < 16 * 1024**3:
        raise ValueError('less than 16 GiB free before capture')
    route_path = directory / f'{name}-route.json'
    save(route_path, route)
    output = directory / name
    environment = dict(os.environ)
    for key in ['THRUM_CAPTURE_RUNTIME', 'THRUM_CAPTURE_PARAMS', 'THRUM_CAPTURE_MOTION_FRAMES', 'THRUM_CAPTURE_PHASE', 'THRUM_CAPTURE_ROUTE_MODE', 'THRUM_G3T_OVERLAY_CONTROL', 'THRUM_G3T_OMIT_PLACEMENT']:
        environment.pop(key, None)
    environment.update(THRUM_CAPTURE_SCENE=str(output), THRUM_CAPTURE_ROUTE=str(route_path),
                       THRUM_CAPTURE_SOURCE=source, THRUM_ASSET_STORE=str(args.store),
                       THRUM_SCENE='valley', THRUM_ROUTE_OVERLAY=route['overlay'], THRUM_STARS='on', THRUM_CULLING='direct',
                       THRUM_CAPTURE_PRESENT_CONTROL='on', THRUM_CAPTURE_SURFACE_FORMAT='bgra8',
                       THRUM_CAPTURE_SURFACE_EXTENT='1280x720')
    environment.update(extra or {})
    with (directory / f'{name}.log').open('wb') as log:
        result = subprocess.run([str(args.binary)], env=environment, cwd=ROOT,
                                stdout=log, stderr=subprocess.STDOUT, timeout=120)
    if result.returncode:
        raise ValueError(f'{name} failed with exit {result.returncode}; see its log')
    records = [json.loads(line) for line in (output / 'route.jsonl').read_text().splitlines()]
    if records[0].get('backend') != 'Metal':
        raise ValueError(f'{name} did not use Metal')
    if not records[0].get('draw_probe_enabled'):
        raise ValueError('M0 fixtures require a binary built with --features draw-probe')
    first, last = route['origin_tick'], route['origin_tick'] + route['keys'][-1]['tick']
    if [row['tick'] for row in records[1:]] != list(range(first, last + 1)):
        raise ValueError(f'{name} did not record every routed tick')
    encoding = 'rgba8' if source == 'sdr' else 'rgba16f'
    for checkpoint in route['checkpoints']:
        path = output / 'checkpoints' / f'{checkpoint["name"]}.{encoding}'
        if not path.is_file():
            raise ValueError(f'{name} omitted checkpoint {checkpoint["name"]}')
        identity = json.loads((output / 'checkpoints' / f'{checkpoint["name"]}.identity.json').read_text())
        if len(identity.get('exposure_gpu_words') or []) != 8:
            raise ValueError(f'{name} omitted exact GPU exposure identity')
    return {'header': records[0], 'samples': records[1:], 'route_sha256': digest(route_path),
            'environment': {k: v for k, v in environment.items() if k.startswith('THRUM_')},
            'disk_free_after_bytes': shutil.disk_usage(directory).free}


def compare_routes(directory, first, second, route, encoding, surface=False, expect_difference=False):
    reports = []
    suffix = 'surface-bgra8' if surface else encoding
    for checkpoint in route['checkpoints']:
        name = checkpoint['name']
        extent = [1000, 800] if surface else route['render_extent']
        report = compare(directory / first / 'checkpoints' / f'{name}.{suffix}',
                         directory / second / 'checkpoints' / f'{name}.{suffix}',
                         'bgra8' if surface else encoding, extent,
                         directory / 'comparisons' / f'{first}-{second}-{name}-{suffix}')
        left_identity = json.loads((directory / first / 'checkpoints' / f'{name}.identity.json').read_text())
        right_identity = json.loads((directory / second / 'checkpoints' / f'{name}.identity.json').read_text())
        if not expect_difference and left_identity != right_identity:
            raise ValueError(f'{first}/{second} checkpoint identity differs at {name}')
        report['reference_identity'] = left_identity
        report['candidate_identity'] = right_identity
        reports.append(report)
    if expect_difference:
        if not any(not row['equal'] for row in reports):
            raise ValueError(f'{second} negative control produced no pixel difference')
    elif not all(row['equal'] for row in reports):
        raise ValueError(f'{first}/{second} exact checkpoint comparison failed')
    return reports


def run_t2(args, directory, name, options, stars='on'):
    output = directory / name
    command = [sys.executable, str(ROOT / 'scripts/capture_v1_runtime.py'), '--binary', str(args.binary),
               '--store', str(args.store), '--output', str(output), '--defer']
    for key, value in options.items():
        command.extend([f'--{key}', str(value)])
    environment = dict(os.environ, THRUM_STARS=stars, THRUM_CULLING='direct', THRUM_CAPTURE_ENCODING='zstd-delta')
    for key in ['THRUM_G3T_OVERLAY_CONTROL', 'THRUM_G3T_OMIT_PLACEMENT', 'THRUM_CAPTURE_PHASE', 'THRUM_CAPTURE_ROUTE']:
        environment.pop(key, None)
    with (directory / f'{name}.log').open('wb') as log:
        result = subprocess.run(command, env=environment, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, timeout=180)
    manifest = json.loads((output / 'manifest.json').read_text())
    if result.returncode or not manifest.get('capture_complete'):
        raise ValueError(f'{name} T2 capture failed; see its log and manifest')
    return {'directory': str(output), 'capture_complete': True, 'options': {k: str(v) for k, v in options.items()},
            'stars': stars, 'disk_free_after_bytes': shutil.disk_usage(directory).free}


def finish_t2(directory, viewing=False):
    manifest = json.loads((directory / 'manifest.json').read_text())
    runtime = json.loads((directory / 'runtime.json').read_text())
    finalize(directory, manifest, runtime, verify_only=not viewing)
    save_manifest(directory, manifest)
    if not manifest['complete']:
        raise ValueError(f'{directory} source verification or packaging incomplete')
    frame_bytes = manifest['extent'][0] * manifest['extent'][1] * manifest['bytes_per_pixel']
    selected = {0, len(runtime['images']) - 1}
    if viewing:
        selected.add(len(runtime['images']) // 2)
        for index in range(1, len(runtime['images'])):
            if runtime['images'][index]['history_epoch'] != runtime['images'][index - 1]['history_epoch']:
                selected.update(range(max(0, index - 2), min(len(runtime['images']), index + 3)))
    selected_paths = []
    if not viewing:
        source = directory / manifest['spool_file']
        expected = runtime['writer_result']['Ok']['frame_sha256']
        for index, raw in enumerate(decoded_frames(source, runtime['spool_encoding'], expected, frame_bytes)):
            if index not in selected:
                continue
            suffix = 'rgba16f' if manifest['bytes_per_pixel'] == 8 else 'rgba8'
            path = directory / f'checkpoint-{index}.{suffix}'
            path.write_bytes(raw)
            selected_paths.append({'file': path.name, 'sha256': digest(path), 'image': runtime['images'][index]})
            if suffix == 'rgba8':
                package(raw, directory / f'checkpoint-{index}.png', *manifest['extent'])
    else:
        retained = set()
        for index in selected:
            image = runtime['images'][index]
            name = f'{image["submission_id"]:06}.png'
            retained.add(name)
            selected_paths.append({'file': 'png/' + name, 'sha256': digest(directory / 'png' / name), 'image': image})
        # Every expanded image was hash-verified by finalize. The compressed source
        # and exact selected PNGs remain; the viewing movie owns its host-time PTS.
        removed = 0
        for path in (directory / 'png').glob('*.png'):
            if path.name not in retained:
                path.unlink()
                removed += 1
        manifest['expanded_png_cleanup'] = {'removed_verified_duplicates': removed, 'retained': sorted(retained),
                                           'rebuild': 'rerun capture_v1_runtime.py --finalize with the retained source'}
        save_manifest(directory, manifest)
    return {'complete': True, 'manifest_sha256': digest(directory / 'manifest.json'),
            'images': len(runtime['images']), 'selected': selected_paths,
            'exposure_packets': sum(len(row.get('exposure_gpu_words') or []) == 8 for row in runtime['images']),
            'disk_free_after_bytes': shutil.disk_usage(directory).free}


def run(args):
    args.binary = args.binary.resolve()
    args.store = args.store.resolve()
    args.output = args.output.resolve()
    args.route = args.route.resolve()
    route = json.loads(args.route.read_text())
    validate_store(args.store, route)
    if args.output.exists():
        raise ValueError('output must be a new directory')
    if not args.binary.is_file():
        raise ValueError('missing game binary')
    args.output.mkdir(parents=True)
    source_names = subprocess.check_output(['git', 'ls-files', '-co', '--exclude-standard',
                                           'crates', 'examples', 'scripts', 'Cargo.lock'], cwd=ROOT,
                                          text=True).splitlines()
    source_hashes = {name: digest(ROOT / name) for name in source_names if (ROOT / name).is_file()}
    report = {'passed': False,
              'source_revision': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
              'dirty_diff_sha256': hashlib.sha256(subprocess.check_output(['git', 'diff', 'HEAD'])).hexdigest(),
              'compiler': subprocess.check_output(['rustup', 'run', '1.99.1', 'rustc', '-vV'], text=True),
              'source_hashes': source_hashes, 'binary_sha256': digest(args.binary),
              'cooked_root': route['cooked_root'], 'route_sha256': digest(args.route),
              'headroom': 1, 'runs': {}, 'comparisons': {}, 't2': {}, 'failures': [],
              'disk_free_before_bytes': shutil.disk_usage(args.output).free}
    try:
        for source, encoding in [('sdr', 'rgba8'), ('scene-linear', 'rgba16f')]:
            for repeat in [1, 2]:
                name = f'{source}-{repeat}'
                report['runs'][name] = run_native(args, args.output, name, route, source)
                save(args.output / 'manifest.json', report)
            a, b = (report['runs'][f'{source}-{repeat}']['samples'] for repeat in [1, 2])
            if [[row[key] for key in MOVING_IDENTITY] for row in a] != [[row[key] for key in MOVING_IDENTITY] for row in b]:
                raise ValueError(f'{source} repeat sample identity mismatch')
            report['comparisons'][source] = compare_routes(args.output, f'{source}-1', f'{source}-2', route, encoding)
        report['runs']['phase-control'] = run_native(args, args.output, 'phase-control', route, extra={'THRUM_CAPTURE_PHASE': '0.1'})
        report['comparisons']['phase-control'] = compare_routes(args.output, 'sdr-1', 'phase-control', route, 'rgba8', expect_difference=True)
        for name, extra in [('mirror-control', {'THRUM_G3T_OVERLAY_CONTROL': 'mirror-omit'}),
                            ('contribution-control', {'THRUM_G3T_OMIT_PLACEMENT': '0'}),
                            ('overflow', {'THRUM_G3T_OVERLAY_CONTROL': 'overflow'}),
                            ('weak-control', {'THRUM_G3T_OVERLAY_CONTROL': 'weak-off'})]:
            report['runs'][name] = run_native(args, args.output, name, route, extra=extra)
            if not report['runs'][name]['header']['draw_probe_enabled']:
                raise ValueError('omission control requires the draw-probe build')
        for name in ['mirror-control', 'contribution-control']:
            report['comparisons'][name] = compare_routes(args.output, 'sdr-1', name, route, 'rgba8', expect_difference=True)
        for name in ['overflow', 'weak-control']:
            if not any(row['dropped_light_references'] > 0 for row in report['runs'][name]['samples']):
                raise ValueError(f'{name} did not exercise a cluster overflow')
        report['comparisons']['weak-control'] = compare_routes(args.output, 'overflow', 'weak-control', route, 'rgba8', expect_difference=True)
        for format in ['bgra8', 'bgra8-srgb']:
            for repeat in [1, 2]:
                name = f'letterbox-{format}-{repeat}'
                report['runs'][name] = run_native(args, args.output, name, route, extra={
                    'THRUM_CAPTURE_SURFACE_FORMAT': format, 'THRUM_CAPTURE_SURFACE_EXTENT': '1000x800'})
            report['comparisons'][format] = compare_routes(args.output, f'letterbox-{format}-1', f'letterbox-{format}-2', route, 'rgba8', surface=True)
        runtime_route = dict(route, render_extent=[1920, 1080])
        runtime_route_path = args.output / 'runtime-route.json'
        save(runtime_route_path, runtime_route)
        night = ROOT / 'examples/v1-game/params/showcase-night-upslope.params.json'
        sun = ROOT / 'examples/v1-game/params/showcase-sunset-clear-sun.params.json'
        cases = [('sun-sdr', {'mode': 'still', 'params': sun}, 'on'),
                 ('sun-linear', {'mode': 'still', 'params': sun, 'source': 'scene-linear'}, 'on'),
                 ('night-sdr', {'mode': 'still', 'params': night}, 'on'),
                 ('night-sdr-off', {'mode': 'still', 'params': night}, 'off'),
                 ('night-linear', {'mode': 'still', 'params': night, 'source': 'scene-linear'}, 'on'),
                 ('night-linear-off', {'mode': 'still', 'params': night, 'source': 'scene-linear'}, 'off'),
                 ('variant-motion', {'mode': 'realtime', 'case': 'variants'}, 'on'),
                 ('route-motion', {'mode': 'realtime', 'case': 'route', 'route': runtime_route_path}, 'on')]
        for name, options, stars in cases:
            report['t2'][name] = run_t2(args, args.output, name, options, stars)
            save(args.output / 'manifest.json', report)
        for name, options, stars in cases:
            report['t2'][name].update(finish_t2(args.output / name, viewing=options['mode'] == 'realtime'))
            save(args.output / 'manifest.json', report)
        for source, suffix in [('sdr', 'rgba8'), ('linear', 'rgba16f')]:
            report['comparisons']['stars-' + source] = compare(args.output / f'night-{source}' / f'checkpoint-0.{suffix}',
                args.output / f'night-{source}-off' / f'checkpoint-0.{suffix}', suffix, [1920, 1080],
                args.output / 'comparisons' / ('stars-' + source))
            if report['comparisons']['stars-' + source]['equal']:
                raise ValueError(f'night star omission had no nonzero {source} pixels')
        motion = json.loads((args.output / 'route-motion/runtime.json').read_text())
        if len({row['history_epoch'] for row in motion['images']}) < 2:
            raise ValueError('realtime route did not expose the authored cut')
        variants = json.loads((args.output / 'variant-motion/runtime.json').read_text())
        if len([row for row in variants.get('control_actions', []) if row['kind'] == 'prepared-star-variant-switch']) != 2:
            raise ValueError('prepared day/night/day switch did not execute')
        report['passed'] = True
    except Exception as error:
        report['failures'].append(str(error))
    report['disk_free_after_bytes'] = shutil.disk_usage(args.output).free
    save(args.output / 'manifest.json', report)
    print(f'{"PASS" if report["passed"] else "FAILED"} M0 capture proofs {args.output / "manifest.json"}')
    return 0 if report['passed'] else 1


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, default=Path('target/release/v1-game'))
    parser.add_argument('--store', type=Path, required=True)
    parser.add_argument('--route', type=Path, default=ROUTE)
    parser.add_argument('--output', type=Path, required=True)
    arguments = parser.parse_args()
    try:
        raise SystemExit(run(arguments))
    except (ValueError, OSError) as error:
        parser.exit(2, f'REFUSED {error}\n')
