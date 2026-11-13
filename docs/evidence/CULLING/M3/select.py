#!/usr/bin/env python3
"""Select exact 1x M3 crops and retain their immutable submission correspondence.

Usage: python3 docs/evidence/CULLING/M3/select.py --capture tmp/culling-m3/capture-...
The complete local sources remain under the capture folder through review.
"""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[4]
OUT = Path(__file__).resolve().parent


def retain_json(source, destination):
    with gzip.GzipFile(filename=str(destination), mode='wb', mtime=0) as output:
        output.write(source.read_bytes())


def crops(base, output, rows, tick_key, groups):
    output.mkdir(exist_ok=True)
    correspondence = []
    for name, tick, box in groups:
        center = min(range(len(rows)), key=lambda i: abs(rows[i][tick_key] - tick))
        for index, row in enumerate(rows[center - 2:center + 4]):
            source = base / 'png' / f'{row["submission_id"]:06}.png'
            dest = output / f'{name}-{index}.png'
            subprocess.run(['ffmpeg', '-v', 'error', '-y', '-i', str(source), '-vf', f'crop={box}',
                            '-pix_fmt', 'rgba', str(dest)], check=True)
            correspondence.append({'group': name, 'index': index, 'box': box,
                                   'source': str(source.relative_to(ROOT)), 'image': row,
                                   'source_png_sha256': hashlib.sha256(source.read_bytes()).hexdigest(),
                                   'crop_sha256': hashlib.sha256(dest.read_bytes()).hexdigest()})
        subprocess.run(['ffmpeg', '-v', 'error', '-y', '-framerate', '1', '-i', str(output / f'{name}-%d.png'),
                        '-vf', 'tile=2x3', '-frames:v', '1', '-pix_fmt', 'rgba', str(output / f'{name}-1x.png')], check=True)
    (output / 'correspondence.json').write_text(json.dumps(correspondence, indent=2) + '\n')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--capture', required=True, type=Path)
    parser.add_argument('--oracle', required=True, type=Path, help='Chrome cube oracle with exact submitted PNGs')
    parser.add_argument('--native-oracle', required=True, type=Path)
    args = parser.parse_args()
    capture = args.capture.resolve()
    for target in ('native-realtime', 'chrome-fixed', 'chrome-realtime'):
        source = capture / target
        manifest = json.loads((source / 'manifest.json').read_text())
        assert manifest['complete'], f'incomplete {target}'
        dest = OUT / target
        dest.mkdir(exist_ok=True)
        for name in ('manifest.json', 'runtime.json'):
            retain_json(source / name, dest / f'{name}.gz')
        if target.endswith('realtime'):
            shutil.copyfile(source / 'preview-1x.mp4', dest / 'preview-1x.mp4')
        rows = json.loads((source / 'runtime.json').read_text())['images'] if target.startswith('native') else manifest['records']
        key = 'tick' if target.startswith('native') else 'source_tick'
        if target.startswith('native'):
            groups = [('enter', 277, '320:400:1600:200'), ('leave', 291, '320:400:0:200'),
                      ('ridge', 285, '480:240:720:96'), ('cover', 378, '480:320:900:300')]
        else:
            groups = [('enter', 277, '240:280:1040:120'), ('leave', 291, '240:280:0:120'),
                      ('ridge', 285, '400:160:400:64'), ('cover', 378, '400:240:500:200')]
        if target.endswith('realtime'):
            crops(source, dest, rows, key, groups)
    oracle = args.oracle.resolve() / 'default'
    fixed = OUT / 'chrome-fixed'
    receipt = json.loads((oracle / 'moving-route.record.json').read_text())
    (fixed / 'exact-checkpoints.json').write_text(json.dumps(receipt['checkpoints'], indent=2) + '\n')
    for checkpoint in receipt['checkpoints']:
        shutil.copyfile(oracle / 'moving-route-submitted' / f'{checkpoint["name"]}.png', fixed / f'{checkpoint["name"]}.png')
    native = args.native_oracle.resolve() / 'portable'
    fixed = OUT / 'native-fixed'
    fixed.mkdir(exist_ok=True)
    for tick in (225, 350, 376, 386, 530):
        subprocess.run(['ffmpeg', '-v', 'error', '-y', '-i', str(native / f'fixed-{tick}.ppm'),
                        '-pix_fmt', 'rgba', str(fixed / f'checkpoint-{tick}.png')], check=True)
    field = capture / 'field-fixed'
    dest = OUT / 'field-fixed'
    dest.mkdir(exist_ok=True)
    records = []
    for name, start, box in [('enter', 301, '240:200:400:140'), ('exit', 421, '240:200:0:140')]:
        for index, tick in enumerate(range(start, start + 6)):
            f = field / 'meshes-frustum' / f'{tick:05}.ppm'
            o = field / 'meshes-occlusion' / f'{tick:05}.ppm'
            assert f.read_bytes() == o.read_bytes(), f'field modes differ at {tick}'
            output = dest / f'{name}-{index}.png'
            subprocess.run(['ffmpeg', '-v', 'error', '-y', '-i', str(o), '-vf', f'crop={box}',
                            '-pix_fmt', 'rgba', str(output)], check=True)
            records.append({'group': name, 'index': index, 'tick': tick, 'box': box,
                            'source': str(o.relative_to(ROOT)), 'sha256': hashlib.sha256(o.read_bytes()).hexdigest(),
                            'frustum_equals_occlusion': True})
        subprocess.run(['ffmpeg', '-v', 'error', '-y', '-framerate', '1', '-i', str(dest / f'{name}-%d.png'),
                        '-vf', 'tile=2x3', '-frames:v', '1', '-pix_fmt', 'rgba', str(dest / f'{name}-1x.png')], check=True)
    (dest / 'correspondence.json').write_text(json.dumps(records, indent=2) + '\n')
    for tick in (301, 426):
        subprocess.run(['ffmpeg', '-v', 'error', '-y', '-i', str(field / 'meshes-occlusion' / f'{tick:05}.ppm'),
                        '-pix_fmt', 'rgba', str(dest / f'whole-{tick}.png')], check=True)


if __name__ == '__main__':
    main()
