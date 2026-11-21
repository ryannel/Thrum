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
# check and refuse compare with the committed records; record writes to the working folder unless --output
# names a fixture to promote.
FIXTURES = {scene: ROOT / f'tests/fixtures/clouds/{scene}-cost.json' for scene in ('shade', 'first-frame', 'low-sun')}
WORKING = ROOT / '.orchestrate/evidence/CLOUDS'
SCRATCH = ROOT / 'tmp/clouds-m2/cost'
SKY_PASSES = (
    'cloud weather map', 'cloud Sun path near', 'cloud Sun path middle',
    'cloud Sun path far', 'cloud ground field', 'cloud far transmittance',
    'cloud hemisphere light', 'cloud trace', 'cloud resolve', 'cloud compose',
)
# The 1440p portable Sky recipe's cloud_resource_bytes inventory. Keep this
# independent of the measured rows so either input can fail on its own.
SKY_RESOURCES = {
    *(f'cloud buffer {index}' for index in range(13)),
    *(f'cloud layer texture {index}' for index in range(6)),
    'cloud texture 0 Rgba32Float',
    'cloud texture 1 Rgba32Float',
    'cloud texture 2 Rgba32Uint',
    *(f'cloud texture {index} Rgba16Float' for index in range(3, 7)),
    'cloud shape noise', 'cloud detail noise', 'cloud hemisphere light',
    'cloud layer parameters', 'cloud phase tables',
}
FIRST_FRAME_ESTIMATES = (
    'CLOUDS DESIGN: weather <0.1 ms, Sun 0.1–0.2 ms, ground <0.05 ms, '
    'resolve 0.1–0.2 ms, compose 0.2–0.4 ms, total 47 MiB. Trace is measured.'
)


def run(command, **kwargs):
    return subprocess.run(command, cwd=ROOT, check=True, text=True,
                          capture_output=True, **kwargs).stdout.strip()


def measure(scene='shade', alternatives=True):
    scratch = ROOT / 'tmp/clouds-m5/cost' if scene == 'low-sun' else SCRATCH
    scratch.mkdir(parents=True, exist_ok=True)
    run(['cargo', 'build', '--release', '--locked', '--offline', '-p', 'v1-game',
         '--example', 'cloud_cost'], timeout=1200)
    run(['cargo', 'build', '--release', '--locked', '--offline', '-p', 'thrum-cook'], timeout=1200)
    store = scratch / 'valley'
    if not store.exists():
        run(['target/release/thrum-cook', 'cook', 'examples/v1-game/assets/showcase', str(store)], timeout=180)
    rows = []
    arms=[('off',1,None,None),('shade',1,None,None),('shade',8,None,None),('shade',60,None,None)]
    if scene in ('first-frame', 'low-sun'):
        arms=[('off',1,None,None),('sky',1,None,None)]
        if alternatives:
            arms += [('sky',1,2,3),('sky',1,4,3),('sky',1,1,1),('sky',1,1,0)]
    for selection, scale, stride, samples in arms:
        environment = dict(os.environ, THRUM_SCENE='valley', THRUM_ASSET_STORE=str(store),
                           THRUM_CLOUDS=selection, THRUM_CLOUD_TIME_SCALE=str(scale),
                           THRUM_CLOUD_WIND='40', THRUM_RECIPE='portable')
        if scene in ('first-frame', 'low-sun'):
            environment.pop('THRUM_CLOUD_REFRESH_STRIDE', None)
            environment.pop('THRUM_CLOUD_LIGHT_SAMPLES', None)
            environment.update(THRUM_CAMERA_HEIGHT='260.4846459239257',THRUM_CAMERA_Z='80')
        if stride is not None:
            environment.update(THRUM_CLOUD_REFRESH_STRIDE=str(stride),THRUM_CLOUD_LIGHT_SAMPLES=str(samples))
        command = ['target/release/examples/cloud_cost']
        if scene == 'low-sun':
            command.append('--low-sun')
        row = json.loads(run(command, env=environment, timeout=180))
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
    if scene in ('first-frame', 'low-sun'):
        validate_first_frame(record)
        if scene == 'low-sun':
            validate_low_sun(record)
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


def validate_sky_resources(resources):
    if not isinstance(resources, list):
        raise ValueError('missing Sky resource inventory')
    names = set()
    for entry in resources:
        if not isinstance(entry, list) or len(entry) != 2:
            raise ValueError('expected a [name, bytes] Sky resource entry')
        name, size = entry
        if not isinstance(name, str) or not name or type(size) is not int or size <= 0:
            raise ValueError(f'invalid Sky resource byte count for {name!r}')
        if name in names:
            raise ValueError(f'duplicate Sky resource measurement: {name}')
        names.add(name)
    if names != SKY_RESOURCES:
        missing = ', '.join(sorted(SKY_RESOURCES - names)) or 'none'
        unexpected = ', '.join(sorted(names - SKY_RESOURCES)) or 'none'
        raise ValueError(f'incomplete Sky resource inventory (missing: {missing}; unexpected: {unexpected})')


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
            validate_sky_resources(row.get('resources'))
        if (max(first['whole_frame_gpu_ns'], row['whole_frame_gpu_ns']) > 1e9 / 120
                and not record['investigation'].strip()):
            raise ValueError('refresh tripwire lacks investigation')


def require_refusal(record, rows, description):
    try:
        validate(dict(record, runs=rows), 'first-frame')
    except ValueError as error:
        return error
    raise ValueError(f'accepted {description}')


def validate_low_sun(record):
    previous = record['previous']
    validate_first_frame(previous)
    for measured in (record, previous):
        rows = measured['runs']
        if len(rows) < 2 or [row['selection'] for row in rows[:2]] != ['off', 'sky']:
            raise ValueError('missing default paired low-Sun arms')
        view = previous['runs'][0]
        for row in rows:
            angles = row.get('view_angles') or {}
            if (row['view'] != view['view'] or angles != view['view_angles']
                    or not math.isclose(angles.get('start_sun_elevation_degrees', -1), 10, abs_tol=0.001)):
                raise ValueError('expected matched 10-degree Sun views')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=['record', 'check', 'refuse'])
    parser.add_argument('--scene', choices=['shade','first-frame','low-sun'], required=True)
    parser.add_argument('--previous', type=Path, help='previous renderer cost record at the same low-Sun view')
    parser.add_argument('--output', type=Path, help='write a cost record to an alternate destination')
    args = parser.parse_args()
    destination = args.output or FIXTURES[args.scene]
    if args.command == 'refuse':
        if args.scene != 'first-frame':
            parser.error('refuse is defined for first-frame')
        record = json.loads(destination.read_text())
        validate(record, args.scene)
        rows = record['runs']
        absent = next(row for row in rows if row['selection'] == 'off')
        substituted = [absent if row['selection'] == 'sky' else row for row in rows]
        error = require_refusal(record, substituted, 'an all-clouds-absent record')
        print(f'Refused clouds-absent substitution: {error}')
        visible_index = next(index for index, row in enumerate(rows) if row['selection'] == 'sky')
        resources = rows[visible_index]['resources']
        for index, (name, _) in enumerate(resources):
            incomplete = rows.copy()
            incomplete[visible_index] = dict(
                rows[visible_index], resources=resources[:index] + resources[index + 1:],
            )
            require_refusal(record, incomplete, f'a Sky inventory without {name}')
        print(f'Refused {len(resources)} single-resource omissions')
        return

    before = os.getloadavg()
    rows = measure(args.scene, args.command == 'record')
    if args.command == 'record':
        record = {'commit': run(['git', 'rev-parse', 'HEAD']),
                  'dirty': bool(run(['git', 'status', '--porcelain'])),
                  'load': {'before': before, 'after': os.getloadavg()},
                  'machine': platform.platform(), 'build': 'release, capture off, named GPU passes',
                  'runs': rows, 'investigation': '',
                  'estimates': 'CLOUDS DESIGN cost account: weather <0.1 ms, Sun 0.1–0.2 ms, ground <0.05 ms; profiles and resource bytes recorded separately.'}
        if args.scene in ('first-frame', 'low-sun'):
            record['estimates'] = FIRST_FRAME_ESTIMATES
        if args.previous:
            record['previous'] = json.loads(args.previous.read_text())
        destination = args.output or WORKING / f'{args.scene}-cost.json'
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(json.dumps(record, indent=2) + '\n')
        print(destination)
    else:
        record = json.loads(destination.read_text())
        validate(record, args.scene)
        if args.scene in ('first-frame', 'low-sun'):
            validate(dict(record, runs=rows), args.scene)
        if args.scene == 'low-sun':
            print(json.dumps({'commit': run(['git', 'rev-parse', 'HEAD']),
                              'load': {'before': before, 'after': os.getloadavg()}, 'runs': rows}))
        else:
            print(json.dumps({'measured_whole_frame_ns': [r['whole_frame_gpu_ns'] for r in rows]}))


if __name__ == '__main__':
    main()
