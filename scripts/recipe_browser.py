#!/usr/bin/env python3
"""Integrated Chrome fixed exact and realtime proof; run under chrome lock.

Use a draw-probe Wasm build for omission controls; ordinary arms must first
match the retained pre-change oracle. Light-list readback runs separately.
"""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import subprocess

from apple_tile_m1_compare_browser import compare_digest
from package_v1_browser import run as package
from compare_v1_pixels import compare


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--web-root', type=Path, default=Path('target/v1-game-web-probe'))
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    registry = json.loads(Path('.config/exact-baselines.json').read_text())
    report = {'revision': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
              'diff_sha256': hashlib.sha256(subprocess.check_output(['git', 'diff', 'HEAD'])).hexdigest(),
              'passed': False, 'runs': [], 'omissions': [],
              'scope': 'ordinary fixed exact, host-time realtime, real-renderer omission controls; overflow lists run separately'}
    try:
        for mode in ['fixed', 'realtime']:
            for scene in ['day', 'night']:
                for source in (['sdr', 'scene-linear'] if mode == 'fixed' else ['sdr']):
                    name = f'{mode}-{scene}-{source}'
                    directory = out / name
                    command = ['node', 'scripts/capture_v1_browser.mjs', '--mode', mode,
                               '--scene', scene, '--source', source, '--cook', 'target/release/thrum-cook',
                               '--web-root', str(args.web_root),
                               '--output', str(directory)]
                    with (out / f'{name}.log').open('w') as log:
                        subprocess.run(command, check=True, stdout=log, stderr=subprocess.STDOUT)
                    manifest = json.loads((directory / 'manifest.json').read_text())
                    if not manifest['complete']:
                        raise RuntimeError(f'{name}: incomplete capture')
                    row = {'name': name, 'command': command, 'records': len(manifest['records'])}
                    if mode == 'fixed':
                        reference = next(b for b in registry['baselines'] if b['id'] == f'chrome-{scene}-{source}')
                        comparison = out / f'{name}-comparison.json'
                        compare_digest(Path(reference['reference']), directory, comparison, source == 'scene-linear')
                        row['exact'] = True
                    if source == 'sdr':
                        package(directory)
                        if mode == 'realtime':
                            subprocess.run(['ffmpeg', '-v', 'error', '-i', str(directory / 'preview-1x.mp4'),
                                            '-vf', 'fps=10,scale=384:216,tile=5x8', '-fps_mode', 'vfr',
                                            str(directory / 'inspection-%02d.png')], check=True)
                    report['runs'].append(row)
                    (out / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
                    print(f'{name}: {row["records"]} complete records', flush=True)
        for control, scene in [('light-off', 'day'), ('mirror-omit', 'day'), ('stars-off', 'night')]:
            route = json.loads(Path('examples/v1-game/routes/moving-v1.json').read_text())
            route['overlay'] = control
            route_file = out / f'{control}-route.json'
            route_file.write_text(json.dumps(route, indent=2) + '\n')
            directory = out / control
            command = ['node', 'scripts/capture_v1_browser.mjs', '--mode', 'fixed', '--scene', scene,
                       '--source', 'sdr', '--cook', 'target/release/thrum-cook', '--web-root', str(args.web_root),
                       '--route-file', str(route_file), '--output', str(directory)]
            with (out / f'{control}.log').open('w') as log:
                subprocess.run(command, check=True, stdout=log, stderr=subprocess.STDOUT)
            reference = next(b for b in registry['baselines'] if b['id'] == f'chrome-{scene}-sdr')
            try:
                compare_digest(Path(reference['reference']), directory, out / f'{control}-unexpected-pass.json', False)
            except ValueError as error:
                if 'decoded image changed at ordinal' not in str(error):
                    raise
                mismatch = str(error)
            else:
                raise RuntimeError(f'{control}: omission unexpectedly matched pre-change baseline')
            positive = out / f'fixed-{scene}-sdr'
            before = json.loads((positive / 'manifest.json').read_text())
            after = json.loads((directory / 'manifest.json').read_text())
            left, right = next((a, b) for a, b in zip(before['records'], after['records'], strict=True)
                               if a['rgba_sha256'] != b['rgba_sha256'])
            ref_raw, candidate_raw = out / f'{control}-reference.rgba', out / f'{control}-candidate.rgba'
            ref_raw.write_bytes(gzip.decompress((positive / left['file']).read_bytes()))
            candidate_raw.write_bytes(gzip.decompress((directory / right['file']).read_bytes()))
            difference = compare(ref_raw, candidate_raw, 'rgba8', route['render_extent'], out / f'{control}-diff')
            if difference['equal']:
                raise RuntimeError('omission diff unexpectedly equal')
            ref_raw.unlink()
            candidate_raw.unlink()
            report['omissions'].append({'control': control, 'scene': scene, 'command': command,
                                        'prechange_comparison_refusal': mismatch,
                                        'ordinal': left['ordinal'], 'native_size_diff': difference})
            (out / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
        report['passed'] = True
    finally:
        (out / 'report.json').write_text(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
