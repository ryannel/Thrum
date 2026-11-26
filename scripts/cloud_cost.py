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
FIXTURES = {scene: ROOT / f'tests/fixtures/clouds/{scene}-cost.json' for scene in ('shade', 'first-frame', 'low-sun', 'placed', 'decks', 'overcast-lit')}
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
DECK_PRESETS = ('fair', 'broken', 'overcast')
WORLD_LIGHT_PASSES = ('cloud sky overlay', 'atmosphere diffuse irradiance',
                      *(f'atmosphere reflection mip {mip}' for mip in range(7)))


def run(command, **kwargs):
    try:
        return subprocess.run(command, cwd=ROOT, check=True, text=True,
                              capture_output=True, **kwargs).stdout.strip()
    except subprocess.CalledProcessError as error:
        raise RuntimeError(f'{error}: {error.stderr.strip()}') from error


def measure(scene='shade', alternatives=True):
    scratch = {
        'low-sun': ROOT / 'tmp/clouds-m5/cost',
        'placed': ROOT / 'tmp/clouds-m6/cost',
        'decks': ROOT / 'tmp/clouds-m7/cost',
        'overcast-lit': ROOT / 'tmp/clouds-wp8/cost',
    }.get(scene, SCRATCH)
    scratch.mkdir(parents=True, exist_ok=True)
    run(['cargo', 'build', '--release', '--locked', '--offline', '-p', 'v1-game',
         '--example', 'cloud_cost'], timeout=1200)
    run(['cargo', 'build', '--release', '--locked', '--offline', '-p', 'thrum-cook'], timeout=1200)
    store = scratch / 'valley'
    if not store.exists():
        run(['target/release/thrum-cook', 'cook', 'examples/v1-game/assets/showcase', str(store)], timeout=180)
    placed_store = scratch / 'placed'
    if scene == 'placed':
        run(['cargo', 'run', '--release', '--locked', '--offline', '-p', 'v1-game',
             '--example', 'cook_cloud_library', '--', str(scratch / 'tvol'), str(placed_store)], timeout=1200)
    rows = []
    arms=[('off',1,None,None),('shade',1,None,None),('shade',8,None,None),('shade',60,None,None)]
    if scene in ('first-frame', 'low-sun', 'placed'):
        arms=[('off',1,None,None),('sky',1,None,None)]
        if alternatives and scene != 'placed':
            arms += [('sky',1,2,3),('sky',1,4,3),('sky',1,1,1),('sky',1,1,0)]
    if scene == 'decks':
        arms = [('off', 1, None, None)] + [('sky', 1, None, None) for _ in DECK_PRESETS]
    if scene == 'overcast-lit':
        arms = [('off', 1, None, None), ('sky', 1, None, None), ('world-light', 1, None, None)]
    recipes = ('portable', 'native') if scene == 'placed' else ('portable',)
    for recipe, (index, (selection, scale, stride, samples)) in (
        (recipe, arm) for recipe in recipes for arm in enumerate(arms)
    ):
        environment = dict(os.environ, THRUM_SCENE='valley', THRUM_ASSET_STORE=str(store),
                           THRUM_CLOUDS=selection, THRUM_CLOUD_TIME_SCALE=str(scale),
                           THRUM_CLOUD_WIND='40', THRUM_RECIPE='portable')
        environment.pop('THRUM_CLOUD_PRESET', None)
        environment.pop('THRUM_CLOUD_SEQUENCE', None)
        if scene in ('first-frame', 'low-sun', 'placed', 'decks', 'overcast-lit'):
            environment.pop('THRUM_CLOUD_REFRESH_STRIDE', None)
            environment.pop('THRUM_CLOUD_LIGHT_SAMPLES', None)
            environment.update(THRUM_CAMERA_HEIGHT='260.4846459239257',THRUM_CAMERA_Z='80')
        if stride is not None:
            environment.update(THRUM_CLOUD_REFRESH_STRIDE=str(stride),THRUM_CLOUD_LIGHT_SAMPLES=str(samples))
        command = ['target/release/examples/cloud_cost']
        if scene == 'decks':
            command += ['--device-profile', 'portable', '--noon']
            if selection == 'sky':
                environment['THRUM_CLOUD_PRESET'] = DECK_PRESETS[index - 1]
        if scene == 'overcast-lit':
            command += ['--device-profile', 'portable', '--noon']
            environment['THRUM_CLOUD_PRESET'] = 'overcast'
        if scene in ('low-sun', 'placed'):
            command.append('--low-sun')
        if scene == 'placed':
            command += ['--device-profile', recipe]
            if selection == 'sky':
                command += ['--placed-store', str(placed_store)]
        row = json.loads(run(command, env=environment, timeout=180))
        row.update(selection=selection, weather_speed=scale)
        if scene in ('decks', 'overcast-lit'):
            row.update(preset=environment.get('THRUM_CLOUD_PRESET'))
        if scene == 'placed':
            row.update(recipe=recipe)
        if selection in ('sky', 'world-light'):
            knobs = {knob['name'].rsplit('/', 1)[-1]: knob['value'] for knob in row['quality']}
            row.update(refresh_stride=1 << knobs['cloud refresh exponent'],
                       local_light_samples=knobs['cloud local light samples'])
        rows.append(row)
    return rows


def validate(record,scene='shade'):
    for key in ['commit', 'load', 'machine', 'build', 'runs', 'investigation']:
        if key not in record:
            raise ValueError(f'cost record lacks {key}')
    if scene == 'placed':
        validate_placed(record)
        return
    if scene == 'decks':
        validate_first_frame(record)
        if [row.get('preset') for row in record['runs']] != [None, *DECK_PRESETS]:
            raise ValueError('expected absent, fair, broken and overcast paired arms')
        if any(row.get('view_angles') != record['runs'][0].get('view_angles')
               or (row.get('view_angles') or {}).get('start_solar_hours') != 12
               for row in record['runs']):
            raise ValueError('expected the same noon view in every arm')
        if record.get('deck_tripwires') != deck_tripwires(record['runs']):
            raise ValueError('deck tripwires lack their paired pass and work investigation')
        if 'reference_investigation' in record:
            expected_investigation = investigate_decks(record['runs'], record['deck_tripwires'])
            if record['investigation'] != expected_investigation:
                raise ValueError('deck investigation does not describe the measured pair')
        return
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



def validate_placed(record):
    previous = record['previous']
    validate_first_frame(previous)
    reference = previous['runs'][0]
    for recipe in ('portable', 'native'):
        rows = [row for row in record['runs'] if row.get('recipe') == recipe]
        strips = {'cloud footprints', 'cloud placed weather map', 'cloud placed weather windows', 'placed cloud instances'}
        if {row.get('selection') for row in rows} != {'off', 'sky'}:
            raise ValueError(f'{recipe} is missing a paired placed-cloud cost')
        for row in rows:
            if (row['capture'] or row['extent'] != [2560, 1440] or row['view'] != reference['view']
                    or row['view_angles'] != reference['view_angles'] or 'backend: Metal' not in row['adapter']
                    or not positive(row['whole_frame_gpu_ns']) or not row['first_submission']):
                raise ValueError(f'{recipe} needs the matched 1440p 10° Metal view')
            if row['selection'] == 'off':
                if row['resources']:
                    raise ValueError('absent arm retains cloud storage')
                continue
            names = {name for name, size in row['resources'] if positive(size)}
            if not (strips | SKY_RESOURCES) <= names:
                raise ValueError(f'{recipe} lacks a complete footprint and frame resource inventory')
            forms = ('R8Unorm',) if recipe == 'portable' else ('R8Unorm', 'Bc4RUnorm')
            for product in ('cloud-library', 'cloud-growth'):
                if not any(f'{product} {fmt}' in names for fmt in forms):
                    raise ValueError(f'{recipe} lacks the {product} stack')
            if not any(name.endswith('cloud placed weather map') and positive(time)
                       for name, time in row['pass_gpu_ns'].items()):
                raise ValueError(f'{recipe} lacks the placed map timing')
            if row['whole_frame_gpu_ns'] > 1e9 / 120 and not record['investigation'].strip():
                raise ValueError(f'{recipe} placed cloud exceeded refresh without an investigation')


def deck_tripwires(rows):
    fair = rows[1]
    findings = {}
    for row in rows[1:]:
        passes = row['pass_gpu_ns']
        previous = fair['pass_gpu_ns']
        cloud = {name: value for name, value in passes.items() if name.rsplit(': ', 1)[-1].startswith('cloud ')}
        work = row['ray_statistics']['mean_samples'] * row['ray_statistics']['rays_traced']
        fair_work = fair['ray_statistics']['mean_samples'] * fair['ray_statistics']['rays_traced']
        trace = next((value for name, value in cloud.items() if name.endswith('cloud trace')), 0)
        fair_trace = next((value for name, value in previous.items() if name.endswith('cloud trace')), 0)
        findings[row['preset']] = {
            'refresh_excess': max(row['whole_frame_gpu_ns'], row['first_submission']['whole_frame_gpu_ns']) > 1e9 / 120,
            'largest_cloud_passes': sorted(cloud.items(), key=lambda item: -item[1])[:3],
            'passes_at_least_25_percent_over_fair': {
                name: [previous[name], value] for name, value in cloud.items()
                if name in previous and value >= 1.25 * previous[name]
            },
            'trace_work_ratio_to_fair': work / fair_work,
            'trace_time_ratio_to_fair': trace / fair_trace,
            'trace_cost_outgrows_work': trace / fair_trace >= 1.25 * work / fair_work,
            'resource_bytes': sum(size for _, size in row['resources']),
        }
    return json.loads(json.dumps(findings))


def investigate_decks(rows, tripwires):
    notes = []
    absent = rows[0]
    if max(absent['whole_frame_gpu_ns'], absent['first_submission']['whole_frame_gpu_ns']) > 1e9 / 120:
        notes.append(f"Clouds absent also exceeds 120 Hz: {absent['whole_frame_gpu_ns'] / 1e6:.3f} ms steady, "
                     f"{absent['first_submission']['whole_frame_gpu_ns'] / 1e6:.3f} ms first submission. "
                     "This establishes some frame cost without clouds.")
    for row in rows[1:]:
        preset = row['preset']
        findings = tripwires[preset]
        if findings['refresh_excess']:
            largest = ', '.join(f'{name} {value / 1e6:.3f} ms' for name, value in findings['largest_cloud_passes'])
            notes.append(f"{preset}: exceeds 120 Hz at {row['whole_frame_gpu_ns'] / 1e6:.3f} ms steady or "
                         f"{row['first_submission']['whole_frame_gpu_ns'] / 1e6:.3f} ms on the first submission. "
                         f"Largest cloud pass intervals: {largest}.")
        for name, (fair, measured) in findings['passes_at_least_25_percent_over_fair'].items():
            notes.append(f'{preset}: {name} took {measured / 1e6:.4f} ms compared with fair at {fair / 1e6:.4f} ms. '
                         f'The cloud resource inventory totals {findings["resource_bytes"]:,} bytes, compared with '
                         f'{tripwires["fair"]["resource_bytes"]:,} bytes in fair. The pair measures the pass increase '
                         'under shared load. It does not isolate per-invocation work from contention.')
        if findings['trace_cost_outgrows_work']:
            notes.append(f"{preset}: trace takes {findings['trace_time_ratio_to_fair']:.1%} of fair's time for "
                         f"{findings['trace_work_ratio_to_fair']:.1%} of its counted ray samples. The counter includes "
                         'visits through empty space and omits nested sunlight work. The discrepancy warrants measuring '
                         'lighting work per contributing sample as well as shared-load contention.')
    return '\n'.join(notes) or 'No deck tripwires triggered in this measurement.'


def named_passes(row):
    return {name.rsplit(': ', 1)[-1]: value for name, value in row['pass_gpu_ns'].items()}


def overcast_tripwires(record):
    off, sky, lit = record['runs']
    previous = record['wp7_overcast']['arm']
    triggers = {}
    if max(lit['whole_frame_gpu_ns'], lit['first_submission']['whole_frame_gpu_ns']) > 1e9 / 120:
        triggers['refresh_interval'] = [off['whole_frame_gpu_ns'], sky['whole_frame_gpu_ns'],
                                        lit['whole_frame_gpu_ns'], lit['first_submission']['whole_frame_gpu_ns']]
    measured = named_passes(lit)
    if measured['cloud sky overlay'] > 50_000:
        triggers['overlay_budget'] = [50_000, measured['cloud sky overlay']]
    for label, reference in [('WP7', previous), ('Sky', sky)]:
        baseline = named_passes(reference)
        if lit['whole_frame_gpu_ns'] >= 1.25 * reference['whole_frame_gpu_ns']:
            triggers[f'{label}/whole frame'] = [reference['whole_frame_gpu_ns'], lit['whole_frame_gpu_ns']]
        for name, actual in measured.items():
            if positive(baseline.get(name)) and actual >= 1.25 * baseline[name]:
                triggers[f'{label}/{name}'] = [baseline[name], actual]
        def samples(row):
            work = row['ray_statistics']
            return work['mean_samples'] * work['rays_traced']
        if (samples(reference) > 0 and positive(baseline.get('cloud trace'))
                and measured['cloud trace'] / baseline['cloud trace'] >= 1.25 * samples(lit) / samples(reference)):
            triggers[f'{label}/trace outgrows samples'] = [
                samples(lit) / samples(reference), measured['cloud trace'] / baseline['cloud trace']]
    return triggers


def investigate_overcast(record):
    off, sky, lit = record['runs']
    sky_passes = named_passes(sky)
    notes = {}
    for key, values in overcast_tripwires(record).items():
        if key == 'refresh_interval':
            off_ns, sky_ns, lit_ns, first_ns = values
            largest = sorted(named_passes(lit).items(), key=lambda row: -row[1])[:3]
            leaders = ', '.join(f'{name} {time / 1e6:.3f} ms' for name, time in largest)
            notes[key] = (f'Off {off_ns / 1e6:.3f} ms, Sky {sky_ns / 1e6:.3f} ms, '
                          f'World light {lit_ns / 1e6:.3f} ms steady and {first_ns / 1e6:.3f} ms first. '
                          f'Largest intervals: {leaders}. Frame-wide tuning remains follow-up.')
        elif key == 'overlay_budget':
            notes[key] = (f'The overlay averages {values[1] / 1e3:.1f} µs. This pass includes the shared '
                          'clear-sky hemisphere integration and due coarse cube rows. Filtering is timed '
                          'separately. Further tuning is deferred.')
        elif key.endswith('/trace outgrows samples'):
            notes[key] = (f'Trace work ratio {values[0]:.3f} and time ratio {values[1]:.3f}. '
                          'Counted samples omit nested sunlight work. Separate that work from '
                          'GPU scheduling variability in the performance phase.')
        else:
            _, name = key.split('/', 1)
            paired = sky['whole_frame_gpu_ns'] if name == 'whole frame' else sky_passes.get(name)
            item = (f'{values[0] / 1e3:.1f} to {values[1] / 1e3:.1f} µs. '
                    f'Fresh Sky control: {paired / 1e3:.1f} µs. ' if paired is not None else
                    f'{values[0] / 1e3:.1f} to {values[1] / 1e3:.1f} µs. ')
            if name.startswith('atmosphere reflection') or name == 'atmosphere diffuse irradiance':
                item += ('The registered path reads the cloud cube and composes physical radiance; '
                         'the clear control reads the existing Sky chart. Investigate filtering cost in tuning.')
            else:
                item += ('The matched arms share the pass. The timing alone does not isolate '
                         'scheduling or contention from pass work. Retain this comparison for tuning.')
            notes[key] = item
    return notes


def validate_overcast(record, prior):
    if record['wp7_overcast']['arm'] != prior:
        raise ValueError('cost record needs the committed WP7 overcast reference')
    off, sky, lit = record['runs']
    if [row['selection'] for row in (off, sky, lit)] != ['off', 'sky', 'world-light']:
        raise ValueError('cost record needs matched Off, Sky and World light arms')
    for row in (off, sky, lit):
        if (row['extent'] != [2560, 1440] or row['capture'] or row['preset'] != 'overcast'
                or row['view'] != prior['view'] or row['view_angles'] != prior['view_angles']
                or row['frames'] < 30 or not positive(row['whole_frame_gpu_ns'])):
            raise ValueError('overcast measurement must match the WP7 view and output extent')
    for row in (sky, lit):
        names = named_passes(row)
        expected = set(SKY_PASSES)
        if row is lit:
            expected.remove('cloud hemisphere light')
            expected.update(WORLD_LIGHT_PASSES)
        if any(not positive(names.get(name)) for name in expected):
            raise ValueError(f'{row["selection"]} lacks named cloud or atmosphere pass measurements')
        if (not positive(row['ray_statistics']['rays_traced']) or
                not positive(row['ray_statistics']['mean_samples']) or
                not row['resources'] or any(not positive(size) for _, size in row['resources'])):
            raise ValueError('visible cloud arm lacks ray work or resource bytes')
    if (off['resources'] or any(name.startswith('cloud ') for name in named_passes(off))
            or 'cloud sky overlay' in named_passes(sky)):
        raise ValueError('a control ran a cloud capability it omits')
    named = {name: value for name, value in lit['pass_gpu_ns'].items()
             if name.rsplit(': ', 1)[-1] in WORLD_LIGHT_PASSES}
    if record['world_light_pass_gpu_ns'] != named:
        raise ValueError('named overlay timings must agree with the visible arm')
    triggered = overcast_tripwires(record)
    if record['tripwires'] != triggered or set(record['investigation']) != set(triggered):
        raise ValueError('each measured overcast tripwire needs its investigation')
    if any(not isinstance(note, str) or not note.strip() for note in record['investigation'].values()):
        raise ValueError('overcast tripwire investigations cannot be empty')


def overcast_lit(check):
    revision = run(['git', 'rev-parse', 'HEAD'])
    status = run(['git', 'status', '--porcelain'])
    changes = run(['git', 'diff', '--binary', 'HEAD'])
    load = os.getloadavg()
    prior = json.loads(FIXTURES['decks'].read_text())
    prior_arm = next(row for row in prior['runs'] if row['preset'] == 'overcast')
    if check:
        validate_overcast(json.loads(FIXTURES['overcast-lit'].read_text()), prior_arm)
    rows = measure('overcast-lit', False)
    if (revision, status, changes) != (run(['git', 'rev-parse', 'HEAD']),
                                      run(['git', 'status', '--porcelain']),
                                      run(['git', 'diff', '--binary', 'HEAD'])):
        raise ValueError('checkout changed during World light measurement')
    record = {'commit': revision, 'dirty': bool(status), 'machine': platform.platform(),
              'build': 'release, capture off, portable Metal', 'load': {'before': load, 'after': os.getloadavg()},
              'wp7_overcast': {'commit': prior['commit'], 'record': str(FIXTURES['decks'].relative_to(ROOT)),
                               'arm': prior_arm}, 'runs': rows,
              'world_light_pass_gpu_ns': {name: value for name, value in rows[-1]['pass_gpu_ns'].items()
                                          if name.rsplit(': ', 1)[-1] in WORLD_LIGHT_PASSES}}
    record['tripwires'] = overcast_tripwires(record)
    record['investigation'] = investigate_overcast(record)
    validate_overcast(record, prior_arm)
    return record


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=['record', 'check', 'refuse', 'cadence', 'hold'])
    parser.add_argument('--scene', choices=[*FIXTURES, 'wind', 'scripted'], required=True)
    parser.add_argument('--previous', type=Path, help='previous renderer cost record at the same low-Sun view')
    parser.add_argument('--output', type=Path, help='write a cost record to an alternate destination')
    parser.add_argument('--resume', action='store_true',
                        help='continue a wind series from its retained recordings and the same binary')
    args = parser.parse_args()
    if args.resume and (args.scene != 'wind' or args.output is None):
        parser.error('--resume requires --scene wind and an existing --output')
    if args.command == 'hold' and args.scene == 'scripted':
        from cloud_cadence import scripted_hold
        return scripted_hold()
    if args.command in ('cadence', 'hold') and args.scene == 'wind':
        from cloud_cadence_capture import run_wind
        return run_wind(args.command, args.output, resume=args.resume)
    if args.scene == 'overcast-lit' and args.command in ('check', 'record'):
        report = json.dumps(overcast_lit(args.command == 'check'), indent=2)
        if args.output:
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(report + '\n')
        print(report)
        return
    if args.scene not in FIXTURES or args.command in ('cadence', 'hold'):
        parser.error('cadence requires wind; hold requires wind or scripted')
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
    if args.scene == 'decks':
        deck_revision = run(['git', 'rev-parse', 'HEAD'])
        deck_changes = run(['git', 'diff', '--binary', 'HEAD'])
        deck_status = run(['git', 'status', '--porcelain'])
    rows = measure(args.scene, args.command == 'record')
    if args.scene == 'decks' and (
            deck_revision != run(['git', 'rev-parse', 'HEAD'])
            or deck_changes != run(['git', 'diff', '--binary', 'HEAD'])
            or deck_status != run(['git', 'status', '--porcelain'])):
        raise ValueError('checkout changed during the paired deck measurement')
    if args.command == 'record':
        record = {'commit': run(['git', 'rev-parse', 'HEAD']),
                  'dirty': bool(run(['git', 'status', '--porcelain'])),
                  'load': {'before': before, 'after': os.getloadavg()},
                  'machine': platform.platform(), 'build': 'release, capture off, named GPU passes',
                  'runs': rows, 'investigation': '',
                  'estimates': 'CLOUDS DESIGN cost account: weather <0.1 ms, Sun 0.1–0.2 ms, ground <0.05 ms; profiles and resource bytes recorded separately.'}
        if args.scene in ('first-frame', 'low-sun', 'placed'):
            record['estimates'] = FIRST_FRAME_ESTIMATES
        if args.scene == 'decks':
            record['deck_tripwires'] = deck_tripwires(rows)
            record['investigation'] = investigate_decks(rows, record['deck_tripwires'])
        previous = args.previous or (FIXTURES['low-sun'] if args.scene == 'placed' else None)
        if previous:
            record['previous'] = json.loads(previous.read_text())
        destination = args.output or WORKING / f'{args.scene}-cost.json'
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(json.dumps(record, indent=2) + '\n')
        print(destination)
    else:
        record = json.loads(destination.read_text())
        validate(record, args.scene)
        if args.scene == 'decks':
            tripwires = deck_tripwires(rows)
            fresh = {
                'commit': deck_revision,
                'dirty': bool(deck_status),
                'load': {'before': before, 'after': os.getloadavg()},
                'machine': platform.platform(),
                'build': 'release, capture off, named GPU passes',
                'estimates': record['estimates'],
                'runs': rows,
                'deck_tripwires': tripwires,
                'investigation': investigate_decks(rows, tripwires),
                'reference_investigation': {key: record[key] for key in ('commit', 'investigation')},
            }
            validate(fresh, args.scene)
            print(json.dumps(fresh))
            return
        if args.scene in ('first-frame', 'low-sun', 'placed'):
            validate(dict(record, runs=rows), args.scene)
        if args.scene in ('low-sun', 'placed'):
            print(json.dumps({'commit': run(['git', 'rev-parse', 'HEAD']),
                              'load': {'before': before, 'after': os.getloadavg()}, 'runs': rows}))
        else:
            print(json.dumps({'measured_whole_frame_ns': [r['whole_frame_gpu_ns'] for r in rows]}))


if __name__ == '__main__':
    raise SystemExit(main())
