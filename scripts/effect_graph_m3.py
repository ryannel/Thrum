#!/usr/bin/env python3
"""EFFECT-GRAPH M3 fixed submitted images and paired native boundary-cost records.

Run under agent_loop.py lock gpu, after building the release game and cooker.
"""
import argparse
import contextlib
import hashlib
import json
import os
from pathlib import Path
import statistics
import struct
import subprocess
import sys
import zlib

from capture_spool import decoded_frames


def png(path, raw, width, height):
    def chunk(kind, data):
        return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))
    rows = b''.join(b'\0' + raw[y * width * 4:(y + 1) * width * 4] for y in range(height))
    path.write_bytes(b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 6, 0, 0, 0))
                     + chunk(b'IDAT', zlib.compress(rows)) + chunk(b'IEND', b''))


def selected_image(folder):
    manifest = json.loads((folder / 'manifest.json').read_text())
    runtime = json.loads((folder / 'runtime.json').read_text())
    if not manifest['complete'] or manifest['timebase'] != 'fixed':
        raise ValueError('fixed target capture is incomplete')
    with contextlib.closing(decoded_frames(folder / manifest['spool_file'], runtime['spool_encoding'],
                                          runtime['writer_result']['Ok']['frame_sha256'])) as frames:
        raw = next(frames)
    png(folder / 'selected.png', raw, *manifest['extent'])
    row = {'file': 'selected.png', 'rgba_sha256': hashlib.sha256(raw).hexdigest(),
           'submission': manifest['records'][0], 'source_revision': manifest['source_revision'],
           'binary_sha256': manifest['binary_sha256'], 'mode': 'fixed', 'extent': manifest['extent']}
    (folder / 'selected.json').write_text(json.dumps(row, indent=2) + '\n')
    return row


def capture(args, recipe, selection, case, mode, name, params):
    output = args.output / name
    env = dict(os.environ, THRUM_RECIPE=recipe, THRUM_EFFECT_GRAPH=selection)
    command = [sys.executable, 'scripts/capture_v1_runtime.py', '--binary', str(args.binary),
               '--store', str(args.store), '--output', str(output), '--case', case,
               '--mode', mode, '--params', str(params), '--verify-only']
    with (args.output / f'{name}.log').open('w') as log:
        subprocess.run(command, env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
    manifest = json.loads((output / 'manifest.json').read_text())
    if not manifest['complete']:
        raise ValueError(f'incomplete capture: {name}')
    return output, manifest


def native(args):
    args.output.mkdir(parents=True, exist_ok=False)
    params = Path('examples/v1-game/params/showcase-noon.params.json')
    rows, images = [], []
    for recipe in ['portable', 'apple-tile']:
        for phase in ['noon', 'night-upslope']:
            folder, _ = capture(args, recipe, 'slab', 'pan', 'fixed', f'{recipe}-{phase}',
                                Path(f'examples/v1-game/params/showcase-{phase}.params.json'))
            images.append({'recipe': recipe, 'phase': phase, 'selection': 'slab',
                           'path': str(folder), **selected_image(folder)})
        for round_index in range(2):
            arms = ['simulation-only', 'depth-only-empty', 'slab-empty']
            if round_index % 2:
                arms.reverse()
            for arm in arms:
                folder, manifest = capture(args, recipe, arm, 'off', 'realtime',
                                           f'cost-{recipe}-{round_index}-{arm}', params)
                runtime = json.loads((folder / 'runtime.json').read_text())
                samples = runtime['gpu']
                boundaries = [sample['gpu_effect_boundary_ns'] for sample in samples
                              if sample.get('gpu_effect_boundary_ns') is not None]
                sums = [sum(value for value in sample['gpu_pass_ns'].values() if value is not None)
                        for sample in samples if sample.get('gpu_pass_ns')]
                if len(sums) < 10 or (arm != 'simulation-only' and len(boundaries) < 10):
                    raise ValueError(f'missing measured native rows: {recipe} {arm}')
                rows.append({'recipe': recipe, 'round': round_index, 'arm': arm,
                             'timed_samples': len(sums), 'boundary_samples': len(boundaries),
                             'median_pass_sum_ns': statistics.median(sums),
                             'median_effect_boundary_ns': statistics.median(boundaries) if boundaries else None,
                             'load': manifest.get('system_load'), 'manifest': str(folder / 'manifest.json')})
                (args.output / 'costs.json').write_text(json.dumps(rows, indent=2) + '\n')
    (args.output / 'images.json').write_text(json.dumps(images, indent=2) + '\n')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, default=Path('target/release/v1-game'))
    parser.add_argument('--store', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    native(parser.parse_args())
