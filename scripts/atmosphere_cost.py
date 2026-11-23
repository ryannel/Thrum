#!/usr/bin/env python3
"""Record and check the paired Metal atmosphere cost on the six-stop route."""
import argparse
import hashlib
import json
import math
import os
import platform
import statistics
import subprocess
from pathlib import Path

from run_v1_showcase import store_root

ROOT = Path(__file__).resolve().parent.parent
RECORD = ROOT / 'examples/v1-game/tests/fixtures/atmosphere-route-cost.json'
STOPS = ('sunrise', 'noon', 'sunset-clear-sun', 'sunset-sun', 'twilight', 'night-upslope')
PASSES = (
    'atmosphere transmittance', 'atmosphere multiple scattering', 'atmosphere ground irradiance',
    'atmosphere sky view', 'atmosphere diffuse irradiance', 'atmosphere aerial perspective',
    'atmosphere DFG integration', 'atmosphere stars', 'synthetic Sun air shadow heights',
    *(f'atmosphere reflection mip {i}' for i in range(7)),
)
ORDER = ('starting', 'final', 'final', 'starting')


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def preset(stop):
    return ROOT / 'examples/v1-game/params' / f'showcase-{stop}.params.json'


def measure(args):
    binaries = {'starting': args.starting.resolve(), 'final': args.final.resolve()}
    environment = dict(os.environ, THRUM_SCENE='valley', THRUM_RECONSTRUCTION='taau',
                       THRUM_RECIPE='portable', THRUM_ASSET_STORE=str(args.store.resolve()))
    report = {
        'scope': 'Capture-off 1080p Metal with an offscreen presenter, whole GPU timed-pass span and named graph timestamps. '
                 'Each stop has a fresh renderer and a settled cooked-valley interval. '
                 'Startup work before confirmed cooked residency is labelled on each frame.',
        'host': platform.platform(), 'load_class': 'shared builder load',
        'starting_revision': args.starting_revision, 'final_revision': args.final_revision,
        'cooked_root': store_root(args.store.resolve()),
        'harness_sha256': sha(ROOT / 'examples/v1-game/examples/atmosphere_cost.rs'),
        'history': [
            'M1 reported a noon scene GPU pass-sum p95 change of 22.66 to 25.15 ms. '
            'See .orchestrate/evidence/v1-workload/G3S-HANDOFF.md, lines 847–877.',
            'M2 recorded midnight 1080p scene GPU pass-sum p50/p95 of 3.94–4.39/4.52–9.17 ms '
            'with stars on and 5.76–5.89/6.81–10.59 ms with stars off, under shared load of 27.67–29.37. '
            'See .orchestrate/evidence/v1-workload/G3S-HANDOFF.md, lines 704–722.',
        ],
        'runs': [],
    }
    for arm in ORDER:
        binary = binaries[arm]
        run = {'arm': arm, 'binary_sha256': sha(binary), 'stops': {}}
        for name in STOPS:
            load_before = os.getloadavg()
            row = subprocess.run([binary, preset(name)], cwd=ROOT, env=environment, text=True,
                                 stdout=subprocess.PIPE, check=True, timeout=60)
            result = json.loads(row.stdout)
            result.update(load_before=load_before, load_after=os.getloadavg(),
                          preset_sha256=sha(preset(name)))
            if name == 'noon' and all('atmosphere stars' not in frame['pass_gpu_ns']
                                      for frame in result['frames']):
                result['stars'] = 'daylight-suppressed'
            run['stops'][name] = result
        report['runs'].append(run)
        args.output.write_text(json.dumps(report, separators=(',', ':')) + '\n')


def summary(stop, row):
    frames = row['frames']
    measured = [frame for frame in frames if frame['resident']]
    spans = [frame['frame_gpu_ns'] for frame in measured]
    if len(spans) < 32 or not all(isinstance(v, (int, float)) and math.isfinite(v) and v > 0 for v in spans):
        raise ValueError('missing whole-frame GPU timestamps')
    result = {'whole frame': statistics.median(spans)}
    for name in PASSES:
        # Warm-only timings where available. Startup-only pass durations remain
        # separate from the warm frame span and from skipped passes.
        values = ([frame['pass_gpu_ns'][name] for frame in measured if name in frame['pass_gpu_ns']]
                  or [frame['pass_gpu_ns'][name] for frame in frames if name in frame['pass_gpu_ns']])
        if stop == 'noon' and name == 'atmosphere stars' and row.get('stars') == 'daylight-suppressed':
            if values:
                raise ValueError('noon star omission disagrees with the GPU timestamps')
            continue
        if not values or not all(isinstance(v, (int, float)) and math.isfinite(v) and v >= 0 for v in values):
            raise ValueError(f'missing GPU timing for {name}')
        result[name] = statistics.median(values)
    return result


def validate(record):
    if tuple(run['arm'] for run in record['runs']) != ORDER:
        raise ValueError('missing alternating start/final timing arms')
    if not all(record[key] for key in ('starting_revision', 'final_revision', 'cooked_root', 'harness_sha256',
                                       'host', 'load_class', 'history')):
        raise ValueError('missing build, load class or history')
    if record['harness_sha256'] != sha(ROOT / 'examples/v1-game/examples/atmosphere_cost.rs'):
        raise ValueError('route timing harness differs from the retained measurement')
    configs = {}
    values = {arm: {stop: [] for stop in STOPS} for arm in set(ORDER)}
    for run in record['runs']:
        if not run['binary_sha256'] or set(run['stops']) != set(STOPS):
            raise ValueError('incomplete six-stop arm')
        for stop, row in run['stops'].items():
            if (not row['completed'] or row['backend'] != 'Metal' or row['adapter'] != 'Apple M3 Max'
                    or row['extent'] != [1920, 1080] or row['reconstruction'] != 'taau' or row['capture']):
                raise ValueError(f'{stop}: invalid route cost setup')
            if not all(len(row[key]) == 3 and all(math.isfinite(x) and x >= 0 for x in row[key])
                       for key in ('load_before', 'load_after')):
                raise ValueError(f'{stop}: missing machine load')
            expected = json.loads(preset(stop).read_text())
            for key, setting in [('solar_hours', 'sky.solar_time'), ('yaw', 'camera.yaw'), ('pitch', 'camera.pitch')]:
                if not math.isclose(row[key], expected[setting], abs_tol=1e-4):
                    raise ValueError(f'{stop}: applied pose differs from the preset')
            setup = (row['recipe'], row['preset_sha256'])
            if configs.setdefault(stop, setup) != setup or setup[0] != 'Portable' or setup[1] != sha(preset(stop)):
                raise ValueError(f'{stop}: cost setups differ')
            values[run['arm']][stop].append(summary(stop, row))
    alerts = []
    for stop in STOPS:
        starting_names = set(values['starting'][stop][0])
        if any(set(row) != starting_names for arm in ('starting', 'final') for row in values[arm][stop]):
            raise ValueError(f'{stop}: named pass measurements differ across arms')
        for name in sorted(starting_names):
            before, after = (statistics.median(row[name] for row in values[arm][stop])
                             for arm in ('starting', 'final'))
            if before > 0 and after >= 1.25 * before:
                alerts.append(f'{stop}: {name}')
    if not isinstance(record.get('scrub_hitch_seen'), bool):
        raise ValueError('missing motion observation for the scrub')
    if record['scrub_hitch_seen']:
        alerts.append('scrub hitch')
    missing = [alert for alert in alerts if not record.get('investigations', {}).get(alert)]
    if missing:
        raise ValueError('missing investigation: ' + ', '.join(missing))
    return alerts


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=['record', 'check'])
    parser.add_argument('--scene', choices=['route'], required=True)
    for argument in ('starting', 'final', 'store', 'output'):
        parser.add_argument(f'--{argument}', type=Path)
    parser.add_argument('--starting-revision')
    parser.add_argument('--final-revision')
    args = parser.parse_args()
    if args.command == 'record':
        if not all((args.starting, args.final, args.store, args.output,
                    args.starting_revision, args.final_revision)):
            parser.error('record requires both built executables, revisions, cooked store and output')
        measure(args)
    else:
        alerts = validate(json.loads(RECORD.read_text()))
        print(json.dumps({'passed': True, 'stops': STOPS, 'paired_arms': ORDER, 'investigated': alerts}))


if __name__ == '__main__':
    main()
