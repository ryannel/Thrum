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


def measure():
    SCRATCH.mkdir(parents=True, exist_ok=True)
    run(['cargo', 'build', '--release', '--locked', '--offline', '-p', 'v1-game',
         '--example', 'cloud_cost'], timeout=1200)
    run(['cargo', 'build', '--release', '--locked', '--offline', '-p', 'thrum-cook'], timeout=1200)
    store = SCRATCH / 'valley'
    if not store.exists():
        run(['target/release/thrum-cook', 'cook', 'examples/v1-game/assets/showcase', str(store)], timeout=180)
    rows = []
    for selection, scale in [('off', 1), ('shade', 1), ('shade', 8), ('shade', 60)]:
        environment = dict(os.environ, THRUM_SCENE='valley', THRUM_ASSET_STORE=str(store),
                           THRUM_CLOUDS=selection, THRUM_CLOUD_TIME_SCALE=str(scale),
                           THRUM_CLOUD_WIND='40', THRUM_RECIPE='portable')
        row = json.loads(run(['target/release/examples/cloud_cost'], env=environment, timeout=180))
        row.update(selection=selection, weather_speed=scale)
        rows.append(row)
    return rows


def validate(record):
    for key in ['commit', 'load', 'machine', 'build', 'runs', 'investigation']:
        if key not in record:
            raise ValueError(f'cost record lacks {key}')
    if len(record['runs']) != 4:
        raise ValueError('four paired arms required')
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


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=['record', 'check'])
    parser.add_argument('--scene', choices=['shade'], required=True)
    args = parser.parse_args()
    before = os.getloadavg()
    rows = measure()
    if args.command == 'record':
        record = {'commit': run(['git', 'rev-parse', 'HEAD']),
                  'dirty': bool(run(['git', 'status', '--porcelain'])),
                  'load': {'before': before, 'after': os.getloadavg()},
                  'machine': platform.platform(), 'build': 'release, capture off, named GPU passes',
                  'runs': rows, 'investigation': '',
                  'estimates': 'CLOUDS DESIGN cost account: weather <0.1 ms, Sun 0.1–0.2 ms, ground <0.05 ms; profiles and resource bytes recorded separately.'}
        RECORD.write_text(json.dumps(record, indent=2) + '\n')
        print(RECORD.relative_to(ROOT))
    else:
        validate(json.loads(RECORD.read_text()))
        print(json.dumps({'measured_whole_frame_ns': [r['whole_frame_gpu_ns'] for r in rows]}))


if __name__ == '__main__':
    main()
