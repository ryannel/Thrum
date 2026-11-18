#!/usr/bin/env python3
"""Paired capture-off cloud costs. Timings are investigated tripwires."""
import argparse
import json
import math
import os
import platform
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RECORD = ROOT / 'docs/evidence/CLOUDS/M2-cost.json'
SCRATCH = ROOT / 'tmp/clouds-m2/cost'
SKY_PASSES = (
    'cloud weather map', 'cloud Sun path near', 'cloud Sun path middle',
    'cloud Sun path far', 'cloud ground field', 'cloud far transmittance',
    'cloud hemisphere light', 'cloud trace', 'cloud resolve', 'cloud compose',
)
FIRST_FRAME_ESTIMATES = (
    'CLOUDS DESIGN: weather <0.1 ms, Sun 0.1–0.2 ms, ground <0.05 ms, '
    'resolve 0.1–0.2 ms, compose 0.2–0.4 ms, total 47 MiB. Trace is measured.'
)


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
        arms=[('off',1,None,None),('sky',1,None,None)]
        if alternatives:
            arms += [('sky',1,2,3),('sky',1,4,3),('sky',1,1,1),('sky',1,1,0)]
    for selection, scale, stride, samples in arms:
        environment = dict(os.environ, THRUM_SCENE='valley', THRUM_ASSET_STORE=str(store),
                           THRUM_CLOUDS=selection, THRUM_CLOUD_TIME_SCALE=str(scale),
                           THRUM_CLOUD_WIND='40', THRUM_RECIPE='portable')
        if scene=='first-frame':
            environment.pop('THRUM_CLOUD_REFRESH_STRIDE', None)
            environment.pop('THRUM_CLOUD_LIGHT_SAMPLES', None)
            environment.update(THRUM_CAMERA_HEIGHT='260.4846459239257',THRUM_CAMERA_Z='80')
        if stride is not None:
            environment.update(THRUM_CLOUD_REFRESH_STRIDE=str(stride),THRUM_CLOUD_LIGHT_SAMPLES=str(samples))
        row = json.loads(run(['target/release/examples/cloud_cost'], env=environment, timeout=180))
        row.update(selection=selection, weather_speed=scale)
        if selection == 'sky':
            knobs = {knob['name'].rsplit('/', 1)[-1]: knob['value'] for knob in row['quality']}
            row.update(refresh_stride=1 << knobs['cloud refresh exponent'],
                       local_light_samples=knobs['cloud local light samples'])
        rows.append(row)
    return rows


def validate(record,scene='shade'):
    for key in ['commit', 'load', 'machine', 'build', 'runs', 'investigation']:
        if key not in record:
            raise ValueError(f'cost record lacks {key}')
    if scene == 'first-frame':
        validate_first_frame(record)
        return
    if len(record['runs']) != 4:
        raise ValueError('required paired Shade arms missing')
    for row in record['runs']:
        for key in ['whole_frame_gpu_ns', 'pass_gpu_ns', 'resources', 'maximum_sun_refill_ns', 'extent', 'capture']:
            if key not in row:
                raise ValueError(f'arm lacks {key}')
        if row['capture'] or row['extent'] != [2560, 1440] or row['whole_frame_gpu_ns'] <= 0:
            raise ValueError('invalid measurement')
        if row['selection'] == 'shade':
            if len([k for k in row['pass_gpu_ns'] if 'cloud ' in k]) != 6 or not row['resources']:
                raise ValueError('missing named cloud passes or resources')
        if row['whole_frame_gpu_ns'] > 1e9 / 120 and not record['investigation']:
            raise ValueError('refresh tripwire lacks investigation')


def positive(value):
    return type(value) in (int, float) and math.isfinite(value) and value > 0


def validate_first_frame(record):
    if not record.get('estimates', '').strip():
        raise ValueError('missing design estimates')
    rows = record['runs']
    absent = [row for row in rows if row['selection'] == 'off']
    visible = [row for row in rows if row['selection'] == 'sky']
    if len(absent) != 1 or not visible or len(absent) + len(visible) != len(rows):
        raise ValueError('expected a clouds-absent and a clouds-visible measurement')
    reference = absent[0]
    for row in rows:
        first = row['first_submission']
        if (row['capture'] is not False or row['extent'] != [2560, 1440]
                or 'Apple M3 Max' not in row['adapter'] or 'backend: Metal' not in row['adapter']
                or row['adapter'] != reference['adapter'] or row['view'] != reference['view']
                or row['weather_speed'] != 1
                or not positive(first['whole_frame_gpu_ns'])
                or not positive(row['whole_frame_gpu_ns'])):
            raise ValueError('expected paired 1440p capture-off measurements on M3 Max Metal')

        for passes in [first['pass_gpu_ns'], row['pass_gpu_ns']]:
            cloud_times = {name.rsplit(': ', 1)[-1]: duration for name, duration in passes.items()
                           if name.rsplit(': ', 1)[-1].startswith('cloud ')}
            if row['selection'] == 'sky':
                if any(not positive(cloud_times.get(name)) for name in SKY_PASSES):
                    raise ValueError('missing named Sky GPU pass')
            elif cloud_times or row['resources'] or row['ray_statistics'] is not None:
                raise ValueError('cloud work or allocations found in the absent arm')
        if row['selection'] == 'sky':
            stats = row['ray_statistics'] or {}
            if any(not positive(stats.get(key)) for key in
                   ('rays_traced', 'mean_samples', 'p95_samples')):
                raise ValueError('missing traced rays, mean or p95 samples')
            resources = row['resources']
            if not resources or any(not name or type(size) is not int or size <= 0
                                    for name, size in resources):
                raise ValueError('missing per-resource byte counts')
        if (max(first['whole_frame_gpu_ns'], row['whole_frame_gpu_ns']) > 1e9 / 120
                and not record['investigation'].strip()):
            raise ValueError('refresh tripwire lacks investigation')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=['record', 'check', 'refuse'])
    parser.add_argument('--scene', choices=['shade','first-frame'], required=True)
    args = parser.parse_args()
    destination=RECORD if args.scene=='shade' else ROOT/'docs/evidence/CLOUDS/M3-cost.json'
    if args.command == 'refuse':
        if args.scene != 'first-frame':
            parser.error('refuse is defined for first-frame')
        record = json.loads(destination.read_text())
        validate(record, args.scene)
        absent = next(row for row in record['runs'] if row['selection'] == 'off')
        record['runs'] = [absent if row['selection'] == 'sky' else row for row in record['runs']]
        try:
            validate(record, args.scene)
        except ValueError as error:
            print(f'Refused clouds-absent substitution: {error}')
            return
        raise ValueError('accepted an all-clouds-absent record')

    before = os.getloadavg()
    rows = measure(args.scene, args.command == 'record')
    if args.command == 'record':
        record = {'commit': run(['git', 'rev-parse', 'HEAD']),
                  'dirty': bool(run(['git', 'status', '--porcelain'])),
                  'load': {'before': before, 'after': os.getloadavg()},
                  'machine': platform.platform(), 'build': 'release, capture off, named GPU passes',
                  'runs': rows, 'investigation': '',
                  'estimates': 'CLOUDS DESIGN cost account: weather <0.1 ms, Sun 0.1–0.2 ms, ground <0.05 ms; profiles and resource bytes recorded separately.'}
        if args.scene=='first-frame':
            record['estimates'] = FIRST_FRAME_ESTIMATES
        destination.write_text(json.dumps(record, indent=2) + '\n')
        print(destination.relative_to(ROOT))
    else:
        record = json.loads(destination.read_text())
        validate(record, args.scene)
        if args.scene == 'first-frame':
            validate(dict(record, runs=rows), args.scene)
        print(json.dumps({'measured_whole_frame_ns': [r['whole_frame_gpu_ns'] for r in rows]}))


if __name__ == '__main__':
    main()
