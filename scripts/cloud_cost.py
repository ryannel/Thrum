#!/usr/bin/env python3
"""Paired capture-off Weather and Shade cost; timings are investigated tripwires."""
import argparse
import json
import os
import platform
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RECORD = ROOT / 'docs/evidence/CLOUDS/M2-cost.json'
SCRATCH = ROOT / 'tmp/clouds-m2/cost'


def run(command, **kwargs):
    return subprocess.run(command, cwd=ROOT, check=True, text=True,
                          capture_output=True, **kwargs).stdout.strip()


def measure(scene='shade', alternatives=True):
    SCRATCH.mkdir(parents=True, exist_ok=True)
    run(['cargo', 'build', '--release', '--locked', '--offline', '-p', 'v1-game',
         '--example', 'cloud_cost'], timeout=1200)
    run(['cargo', 'build', '--release', '--locked', '--offline', '-p', 'thrum-cook'], timeout=1200)
    store = SCRATCH / 'valley'
    if not store.exists():
        run(['target/release/thrum-cook', 'cook', 'examples/v1-game/assets/showcase', str(store)], timeout=180)
    rows = []
    arms=[('off',1,None,None),('shade',1,None,None),('shade',8,None,None),('shade',60,None,None)]
    if scene=='first-frame':
        arms=[('off',1,None,None),('sky',1,2,3)]
        if alternatives:
            arms += [('sky',1,1,3),('sky',1,4,3),('sky',1,2,1),('sky',1,2,0)]
    for selection, scale, stride, samples in arms:
        environment = dict(os.environ, THRUM_SCENE='valley', THRUM_ASSET_STORE=str(store),
                           THRUM_CLOUDS=selection, THRUM_CLOUD_TIME_SCALE=str(scale),
                           THRUM_CLOUD_WIND='40', THRUM_RECIPE='portable')
        if scene=='first-frame':
            environment.update(THRUM_CAMERA_HEIGHT='260.4846459239257',THRUM_CAMERA_Z='80')
        if stride is not None:
            environment.update(THRUM_CLOUD_REFRESH_STRIDE=str(stride),THRUM_CLOUD_LIGHT_SAMPLES=str(samples))
        row = json.loads(run(['target/release/examples/cloud_cost'], env=environment, timeout=180))
        row.update(selection=selection, weather_speed=scale)
        if stride is not None:
            row.update(refresh_stride=stride,local_light_samples=samples)
        rows.append(row)
    return rows


def validate(record,scene='shade'):
    for key in ['commit', 'load', 'machine', 'build', 'runs', 'investigation']:
        if key not in record:
            raise ValueError(f'cost record lacks {key}')
    if len(record['runs']) != (6 if scene=='first-frame' else 4):
        raise ValueError('required paired/quality arms missing')
    for row in record['runs']:
        for key in ['whole_frame_gpu_ns', 'pass_gpu_ns', 'resources', 'maximum_sun_refill_ns', 'extent', 'capture']:
            if key not in row:
                raise ValueError(f'arm lacks {key}')
        if row['capture'] or row['extent'] != [2560, 1440] or row['whole_frame_gpu_ns'] <= 0:
            raise ValueError('invalid measurement')
        if row['selection'] == 'shade':
            if len([k for k in row['pass_gpu_ns'] if 'cloud ' in k]) != 6 or not row['resources']:
                raise ValueError('missing named cloud passes or resources')
        if row['selection']=='sky':
            expected=['cloud weather map','cloud Sun path near','cloud Sun path middle','cloud Sun path far','cloud ground field','cloud far transmittance','cloud hemisphere light','cloud trace','cloud resolve','cloud compose']
            if any(not any(name.endswith(p) for name in row['pass_gpu_ns']) for p in expected):
                raise ValueError('missing named Sky pass')
            stats=row.get('ray_statistics') or {}
            if any(key not in stats for key in ['rays_traced','mean_samples','p95_samples','refresh_fraction']):
                raise ValueError('missing traced-ray work measurement')
            if stats['rays_traced']<=0 or stats['mean_samples']<=0 or not row['resources'] or not row.get('first_submission'):
                raise ValueError('missing first submission, work or resource record')
        if row['whole_frame_gpu_ns'] > 1e9 / 120 and not record['investigation']:
            raise ValueError('refresh tripwire lacks investigation')
    if scene=='first-frame' and not record.get('quality_observations'):
        raise ValueError('quality alternatives lack visual observations')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=['record', 'check'])
    parser.add_argument('--scene', choices=['shade','first-frame'], required=True)
    args = parser.parse_args()
    before = os.getloadavg()
    rows = measure(args.scene,args.command=='record')
    destination=RECORD if args.scene=='shade' else ROOT/'docs/evidence/CLOUDS/M3-cost.json'
    if args.command == 'record':
        record = {'commit': run(['git', 'rev-parse', 'HEAD']),
                  'dirty': bool(run(['git', 'status', '--porcelain'])),
                  'load': {'before': before, 'after': os.getloadavg()},
                  'machine': platform.platform(), 'build': 'release, capture off, named GPU passes',
                  'runs': rows, 'investigation': '',
                  'estimates': 'CLOUDS DESIGN cost account: weather <0.1 ms, Sun 0.1–0.2 ms, ground <0.05 ms; profiles and resource bytes recorded separately.'}
        if args.scene=='first-frame':
            record['estimates']='CLOUDS DESIGN: weather <0.1 ms, Sun 0.1–0.2 ms, resolve 0.1–0.2 ms, compose 0.2–0.4 ms, total 47 MiB; trace measured here. One half-extent texel per 2x2 block refresh is 1/16 render pixels.'
            record['quality_observations']=''
        destination.write_text(json.dumps(record, indent=2) + '\n')
        print(destination.relative_to(ROOT))
    else:
        validate(json.loads(destination.read_text()),args.scene)
        print(json.dumps({'measured_whole_frame_ns': [r['whole_frame_gpu_ns'] for r in rows]}))


if __name__ == '__main__':
    main()
