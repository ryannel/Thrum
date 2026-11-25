#!/usr/bin/env python3
"""Paired water cost record. Timing numbers are tripwires, never pass thresholds."""
import argparse
import json
import os
from pathlib import Path
import platform
import statistics
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
# The first-frame and island records are test data later checks compare with; record writes a new take to the
# working folder, and promoting it is copying it over the fixture.
RECORDS = {'first-frame': ROOT / 'tests/fixtures/waters/first-frame-cost.json',
           'island': ROOT / 'tests/fixtures/waters/island-cost.json',
           'wake': ROOT / '.orchestrate/evidence/WATERS/M6/cost.json',
           'foam': ROOT / '.orchestrate/evidence/WATERS/M7/cost.json',
           'surf': ROOT / '.orchestrate/evidence/WATERS/WP21/surf-cost.json'}
WORKING = ROOT / '.orchestrate/evidence/WATERS'
# The island record taken before the records moved to fixtures names the first-frame record by its old path,
# which ends in this name.
FIRST_FRAME_FORMER = '/WATERS/M2-cost.json'
WAKE_HISTORY = ROOT / 'examples/water/tests/fixtures/wake-cost-prior.json'
SURF_HISTORY = ROOT / 'examples/water/tests/fixtures/surf-cost-prior.json'
WP13_SURF = ROOT / 'examples/water/tests/fixtures/surf-cost-wp13.json'
SURF_CACHE_INVESTIGATION = ROOT / 'examples/water/tests/fixtures/surf-cache-investigation.json'
WAKE_ROUTE = {'wind': 'trade', 'view': 'overhead', 'speed_mps': 6}
WAKE_ARMS = {'with-wake': (True, True), 'without-wake': (True, False),
             'water-absent': (False, False)}


def run(command, **kwargs):
    return subprocess.run(command, cwd=ROOT, check=True, **kwargs)


def measure(scene):
    island = scene == 'island'
    output = ROOT / ('tmp/waters-M4/cost-current' if island else 'tmp/waters-M2/cost-current')
    output.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, THRUM_WATER_PROOF='cost', THRUM_WATER_OUTPUT=str(output))
    if island:
        env['THRUM_WATER_ISLAND'] = 'trade'
    else:
        env.pop('THRUM_WATER_ISLAND', None)
    store = output / 'store'
    run(['cargo', 'run', '--release', '--locked', '--offline', '-p', 'thrum-cook', '--', 'cook', 'examples/water/assets', str(store)])
    env['THRUM_ASSET_STORE'] = str(store)
    run(['cargo', 'test', '--release', '--locked', '--offline', '-p', 'water-scene', '--test', 'proof_gpu'], env=env)
    data = json.loads((output / 'cost.json').read_text())
    data['commit'] = run(['git', 'rev-parse', 'HEAD'], capture_output=True, text=True).stdout.strip()
    data['host'] = platform.platform()
    data['load'] = run(['uptime'], capture_output=True, text=True).stdout.strip()
    data['build'] = 'release; concurrent machine load, not qualified cadence'
    data['refresh_hz'] = 120
    data['view'] = ('40 m island overlook in trade wind at noon; same cooked ground and sky at identical world times'
                    if island else 'camera at 2 m, bearing 90 degrees, noon; camera record gives origin and lens; identical calibration float and sky in both arms at identical world times')
    data['scene'] = scene
    data['estimates'] = {'surface_vertices': 200000, 'surface_gpu_ms': 'unmeasured in design; largest term', 'long_cache_gpu_ms': '0.2-0.3', 'previous_inverted_cache_gpu_ms': 3.6, 'broadsides_surface_ms': 10.5}
    data['summary'] = []
    for arm in data['arms']:
        for tick in [True, False]:
            frames = [f for f in arm['frames'] if f['tick_frame'] == tick]
            if not frames:
                raise ValueError('missing tick or repeat frames')
            if any(f['gpu_ms'] is None or f.get('timing_fault', False) for f in frames):
                raise ValueError('missing GPU timing: inspect cost.json timing_fault; no duration is imputed')
            names = set().union(*(f['passes_ms'] for f in frames))
            passes = {name: statistics.median(f['passes_ms'][name] for f in frames if name in f['passes_ms']) for name in sorted(names)}
            data['summary'].append({'water': arm['water'], 'tick_frame': tick, 'samples': len(frames), 'gpu_ms': statistics.median(f['gpu_ms'] for f in frames), 'cpu_wall_ms': statistics.median(f['cpu_wall_ms'] for f in frames), 'completion_upper_ms': statistics.median(f['completion_upper_ms'] for f in frames), 'passes_ms': passes})
    data['investigations'] = []
    for row in data['summary']:
        if max(row['gpu_ms'], row['completion_upper_ms']) > 1000 / data['refresh_hz']:
            largest = max(row['passes_ms'], key=row['passes_ms'].get)
            paired = next(r for r in data['summary'] if r['water'] != row['water'] and r['tick_frame'] == row['tick_frame'])
            water_ms = sum(v for k, v in row['passes_ms'].items() if k.startswith('effect '))
            data['investigations'].append({'wire': 'whole frame exceeds refresh interval', 'water': row['water'], 'tick_frame': row['tick_frame'], 'measurement': f"GPU pass sum {row['gpu_ms']:.3f} ms; paired arm {paired['gpu_ms']:.3f} ms; effect passes {water_ms:.3f} ms; largest pass {largest}: {row['passes_ms'][largest]:.3f} ms. Completion upper bracket {row['completion_upper_ms']:.3f} ms includes synchronous observer polling and timestamp resolve/readback.", 'disposition': 'Pass timestamps isolate the effect and existing scene work; the completion bracket separately includes untimed work and observer waits. Full-frame tuning remains WATERS M11 and the later performance phase; this concurrent-load sample makes no display-cadence claim.'})
    if island:
        first = json.loads(RECORDS['first-frame'].read_text())
        previous = {row['tick_frame']: {
            label.rsplit(': ', 1)[-1]: timing for label, timing in row['passes_ms'].items()
        } for row in first['summary'] if row['water']}
        data['first_frame_record'] = str(RECORDS['first-frame'].relative_to(ROOT))
        sea_arm = next(arm for arm in data['arms'] if arm['water'])
        data['body_pixels'] = dict(zip((name for name, _ in data['body_vertices']), sea_arm['body_pixels']))
        for row in (row for row in data['summary'] if row['water']):
            old = previous[row['tick_frame']]
            for key, new_time in row['passes_ms'].items():
                old_time = old.get(key.rsplit(': ', 1)[-1])
                if old_time and key.startswith('effect ') and new_time >= old_time * 1.25:
                    data['investigations'].append({
                        'wire': 'pass grew by at least 25% compared to first-frame',
                        'water': True, 'tick_frame': row['tick_frame'],
                        'measurement': f'{key}: {old_time:.3f} -> {new_time:.3f} ms; vertices {first["vertices"]} -> {data["vertices"]}; visible coverage {sea_arm["water_pixels_shaded"]} at the island overlook.',
                        'disposition': 'Paired water-off isolates scene work. View, wave forcing and mapped work differ from first-frame, so this comparison flags the term without attributing the delta to one input.',
                    })
    return data


def validate(data, scene):
    assert data['target'] == 'mac-metal' and data['extent'] == [2560, 1440]
    assert data['capture'] is False and data['vertices'] > 0
    assert data['resources'] and data['load'] and data['commit']
    assert {arm['water'] for arm in data['arms']} == {True, False}
    for arm in data['arms']:
        assert arm['calibration_float'] == (scene == 'first-frame')
        assert arm['frames'] and arm['storage']
        assert {f['tick_frame'] for f in arm['frames']} == {True, False}
        if arm['water']:
            assert arm['water_pixels_shaded'] > 0
            assert all(any('water surface' in p for p in f['passes_ms']) for f in arm['frames'])
        else:
            assert all(not any('water surface' in p for p in f['passes_ms']) for f in arm['frames'])
    for row in data['summary']:
        assert row['gpu_ms'] > 0 and row['passes_ms'] and row['samples'] > 0
        if max(row['gpu_ms'], row['completion_upper_ms']) > 1000 / data['refresh_hz']:
            assert any(i['water'] == row['water'] and i['tick_frame'] == row['tick_frame'] and i['measurement'] and i['disposition'] for i in data['investigations'])
    if scene == 'island':
        named = data['first_frame_record']
        assert named == str(RECORDS['first-frame'].relative_to(ROOT)) or named.endswith(FIRST_FRAME_FORMER)
        assert set(data['body_pixels']) == {'Sea', 'Lake', 'Pond'}
        assert all(count > 0 for count in data['body_pixels'].values())
        assert len(data['body_vertices']) == 3 and sum(n for _, n in data['body_vertices']) == data['vertices']
        assert all(n > 0 for _, n in data['body_vertices'])


def wake_summary(frames):
    summary = {}
    for name, step in [('step', True), ('repeat', False)]:
        group = [frame for frame in frames if frame['tick_frame'] == step]
        timed = [frame for frame in group if frame['gpu_ms'] is not None]
        assert group and all(frame['completion_upper_ms'] > 0 for frame in group)
        assert all(frame['gpu_ms'] > 0 and frame['passes_ms'] for frame in timed)
        passes = {}
        for frame in timed:
            for label, elapsed in frame['passes_ms'].items():
                passes.setdefault(label.rsplit(': ', 1)[-1], []).append(elapsed)
        summary[name] = {
            'samples': len(group), 'timestamped_samples': len(timed),
            'gpu_ms': statistics.median(frame['gpu_ms'] for frame in timed) if timed else None,
            'completion_upper_ms': statistics.median(frame['completion_upper_ms'] for frame in group),
            'passes_ms': {key: statistics.median(values) for key, values in sorted(passes.items())},
        }
    return summary


def wake_view(record, prior):
    summaries = {}
    reference = [(f['tick'], f['alpha'], f['tick_frame']) for f in record['arms'][0]['frames']]
    for (name, (water, wake)), arm in zip(WAKE_ARMS.items(), record['arms'], strict=True):
        assert (arm['water'], arm['wake']) == (water, wake)
        assert bool(arm['water_vertices']) == water and bool(arm['water_resources']) == water
        assert bool(arm['water_pixels_shaded']) == water
        assert [(f['tick'], f['alpha'], f['tick_frame']) for f in arm['frames']] == reference
        for frame in arm['frames']:
            assert ('interaction' in frame) == wake
            if wake:
                interaction = frame['interaction']
                assert interaction['endpoints'] and interaction['labelling_cpu_ms'] >= 0
                assert all(0 <= n <= 16 for n in interaction['solves']) and sum(interaction['solves']) > 0
                assert all(n > 0 for n in interaction['regions'])
            if frame['gpu_ms'] is not None:
                names = [name.rsplit(': ', 1)[-1] for name in frame['passes_ms']]
                assert ('water surface' in names) == water
                assert ('sea long waves' in names) == water
                assert ('interaction waves' in names) == (wake and frame['tick_frame'])
        summaries[name] = wake_summary(arm['frames'])

    comparisons = {}
    investigations = []
    for name, summary in summaries.items():
        if name not in prior:
            comparisons[name] = {'availability': 'No prior water-absent arm for this view. The former baseline still drew ambient water.'}
        else:
            comparisons[name] = {}
        for phase, row in summary.items():
            paired = {arm: summaries[arm][phase] for arm in WAKE_ARMS if arm != name}
            passes = row['passes_ms']
            largest = sorted(passes.items(), key=lambda item: item[1], reverse=True)[:3]
            current = {'arm': name, 'phase': phase, 'current_ms': {
                'gpu': row['gpu_ms'], 'completion_upper': row['completion_upper_ms']}}
            if row['timestamped_samples'] != row['samples']:
                investigations.append(dict(current, wire='missing pass timestamps',
                    measurement={'timestamped_samples': row['timestamped_samples'], 'submitted_samples': row['samples'],
                                 'paired_completion_upper_ms': {arm: pair['completion_upper_ms'] for arm, pair in paired.items()}},
                    disposition='The observer records completion brackets independently. The frame host returned no pass timings for rejected batches; their pass durations are unavailable. Lake timings and CPU labelling remain independently measured.'))
            if max(row['gpu_ms'] or 0, row['completion_upper_ms']) > 1000 / 120:
                investigations.append(dict(current, wire='whole frame exceeds refresh interval',
                    measurement={'largest_timed_passes_ms': dict(largest),
                                 'wake_pass_ms': passes.get('interaction waves'),
                                 'paired_gpu_ms': {arm: pair['gpu_ms'] for arm, pair in paired.items()},
                                 'paired_completion_upper_ms': {arm: pair['completion_upper_ms'] for arm, pair in paired.items()}},
                    disposition='The water-absent arm measures scene work. The water-without-wake arm isolates the incremental interaction cost. Completion includes synchronous observer polling and is a bracket under concurrent load.'))
            if name not in prior:
                continue
            old = prior[name][phase]
            common = passes.keys() & old['passes_ms'].keys()
            comparisons[name][phase] = {
                'previous': old,
                'completion_upper_delta_ms': row['completion_upper_ms'] - old['completion_upper_ms'],
                'gpu_delta_ms': row['gpu_ms'] - old['gpu_ms'] if row['gpu_ms'] is not None and old['gpu_ms'] is not None else None,
                'common_pass_deltas_ms': {key: passes[key] - old['passes_ms'][key] for key in sorted(common)},
            }
            for key in sorted(common):
                before, after = old['passes_ms'][key], passes[key]
                if before > 0 and after >= before * 1.25:
                    inactive = paired['without-wake'] if name == 'with-wake' else paired['with-wake']
                    inactive_ms = inactive['passes_ms'].get(key)
                    if inactive_ms is None:
                        observation = 'The other water arm has no sample for this pass.'
                    else:
                        observation = f'The other water arm measured {inactive_ms * 1000:.1f} µs; the historical change was {(after - before) * 1000:.1f} µs.'
                    investigations.append(dict(current, wire='pass grew by at least 25% from same-view record',
                        measurement={'pass': key, 'prior_ms': before, 'current_ms': after,
                                     'paired_pass_ms': {arm: pair['passes_ms'].get(key) for arm, pair in paired.items()}},
                        disposition=f'{observation} The paired and historical samples were taken under concurrent load.'))
    return {'arms': record['arms'], 'resources': record['resources'], 'summary': summaries,
            'historical_comparison': comparisons, 'tripwire_investigations': investigations}


def measure_wake():
    """Repeat the previous wake views and also measure each scene without water."""
    output = ROOT / 'tmp/waters-M6/cost'
    output.mkdir(parents=True, exist_ok=True)
    store = output / 'store'
    run(['cargo', 'run', '--release', '--locked', '--offline', '-p', 'thrum-cook', '--',
         'cook', 'examples/water/assets', str(store)])
    env = dict(os.environ, THRUM_WATER_PROOF='cost', THRUM_ASSET_STORE=str(store),
               THRUM_WATER_WAKE=WAKE_ROUTE['wind'], THRUM_WATER_VIEW=WAKE_ROUTE['view'],
               THRUM_WATER_SPEED=str(WAKE_ROUTE['speed_mps']))
    env['THRUM_WATER_COMMIT'] = run(['git', 'rev-parse', 'HEAD'], capture_output=True, text=True).stdout.strip()
    env.pop('THRUM_WATER_ISLAND', None)
    env.pop('THRUM_WATER_STONE', None)
    prior = json.loads(WAKE_HISTORY.read_text())
    assert prior['route'] == WAKE_ROUTE and prior['target'] == 'mac-metal'
    assert prior['extent'] == [2560, 1440] and prior['capture'] is False
    result = {'target': 'mac-metal', 'extent': [2560, 1440], 'capture': False, 'route': WAKE_ROUTE,
              'commit': env['THRUM_WATER_COMMIT'],
              'host': platform.platform(), 'load': run(['uptime'], capture_output=True, text=True).stdout.strip(),
              'history': {'record': str(WAKE_HISTORY.relative_to(ROOT)), 'commit': prior['commit'],
                          'host': prior['host'], 'load': prior['load']}, 'views': {}}
    for location in ('sea', 'lake'):
        view = output / location
        env.update(THRUM_WATER_OUTPUT=str(view), THRUM_WATER_LOCATION=location)
        run(['cargo', 'test', '--release', '--locked', '--offline', '-p', 'water-scene',
             '--test', 'proof_gpu'], env=env)
        record = json.loads((view / 'cost.json').read_text())
        assert record['target'] == 'mac-metal' and record['extent'] == [2560, 1440]
        assert record['capture'] is False and record['commit'] == result['commit']
        result['views'][location] = wake_view(record, prior['views'][location])
    path = RECORDS['wake']
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(result, indent=2) + '\n')
    print(f'PASS paired 1440p Metal wake records: {path.relative_to(ROOT)}')


def measure_foam():
    """Run same-view controls and keep timestamp faults and comparison context."""
    output = ROOT / 'tmp/waters-M7/cost'
    output.mkdir(parents=True, exist_ok=True)
    store = output / 'store'
    run(['cargo', 'run', '--release', '--locked', '--offline', '-p', 'thrum-cook', '--',
         'cook', 'examples/water/assets', str(store)])
    env = dict(os.environ, THRUM_WATER_PROOF='cost', THRUM_ASSET_STORE=str(store),
               THRUM_WATER_FOAM='gale', THRUM_WATER_VIEW='overhead', THRUM_WATER_SPEED='6')
    for key in ('THRUM_WATER_ISLAND', 'THRUM_WATER_STONE', 'THRUM_WATER_WAKE', 'THRUM_WATER_BODIES'):
        env.pop(key, None)
    env['THRUM_WATER_COMMIT'] = run(['git', 'rev-parse', 'HEAD'], capture_output=True, text=True).stdout.strip()
    prior_sea = json.loads(WAKE_HISTORY.read_text())
    prior_island = json.loads(RECORDS['island'].read_text())
    result = {'target': 'mac-metal', 'extent': [2560, 1440], 'capture': False,
              'commit': env['THRUM_WATER_COMMIT'], 'host': platform.platform(),
              'load': run(['uptime'], capture_output=True, text=True).stdout.strip(),
              'comparison': 'Each record includes the gale scene at the same camera and ticks with foam off. Earlier modules measured the overhead sea in trade wind and the island from the hill. Those historical records are kept with their differing forcing or camera.',
              'prior': {
                  'sea': {'path': str(WAKE_HISTORY.relative_to(ROOT)), 'route': prior_sea['route'],
                          'commit': prior_sea['commit'], 'summary': prior_sea['views']['sea']},
                  'island': {'path': str(RECORDS['island'].relative_to(ROOT)), 'route': prior_island['view'],
                             'commit': prior_island['commit'], 'summary': prior_island['summary']},
              }, 'views': {}}
    for location in ('sea', 'beach'):
        view = output / location
        env.update(THRUM_WATER_OUTPUT=str(view), THRUM_WATER_LOCATION=location)
        run(['cargo', 'test', '--release', '--locked', '--offline', '-p', 'water-scene',
             '--test', 'proof_gpu'], env=env)
        record = json.loads((view / 'cost.json').read_text())
        assert record['target'] == 'mac-metal' and record['extent'] == [2560, 1440]
        assert record['capture'] is False and record['commit'] == result['commit']
        names = ('foam', 'foam-absent', 'water-absent')
        arms = dict(zip(names, record['arms'], strict=True))
        counters = [[(f['tick'], f['alpha']) for f in arm['frames']] for arm in arms.values()]
        assert counters[0] == counters[1] == counters[2]
        summary = {}
        for (name, arm), (water, foam) in zip(arms.items(), ((True, True), (True, False), (False, False)), strict=True):
            assert (arm['water'], arm['foam']) == (water, foam)
            assert all(('foam' in frame) == foam for frame in arm['frames'])
            if foam:
                assert all(frame['foam']['pattern'] is not None for frame in arm['frames'])
                assert any(frame['foam']['endpoints'][0] < frame['foam']['endpoints'][1] for frame in arm['frames'])
            summary[name] = wake_summary(arm['frames'])
            for frame in arm['frames']:
                if frame['gpu_ms'] is not None:
                    assert any(label.endswith('shore film') for label in frame['passes_ms']) == (foam and location == 'beach')
                    assert any(label.endswith('whitewater') for label in frame['passes_ms']) == (foam and frame['tick_frame'])
        investigations = []
        for phase in ('step', 'repeat'):
            active = summary['foam'][phase]
            control = summary['foam-absent'][phase]
            empty = summary['water-absent'][phase]
            if active['completion_upper_ms'] > 1000 / 120 or (active['gpu_ms'] or 0) > 1000 / 120:
                investigations.append({
                    'wire': '120 Hz interval', 'phase': phase,
                    'completion_ms': {name: rows[phase]['completion_upper_ms'] for name, rows in summary.items()},
                    'water_passes_ms': {label: active['passes_ms'].get(label) for label in ('whitewater', 'shore film', 'water surface')},
                    'disposition': 'The foam-off arm isolates the film and whitewater changes. The water-off arm measures the same opaque and sky work. Completion includes synchronous observer polling under load.',
                })
            if not all(row['timestamped_samples'] == row['samples'] for row in (active, control, empty)):
                investigations.append({
                    'wire': 'timestamp availability', 'phase': phase,
                    'timed': {name: rows[phase]['timestamped_samples'] for name, rows in summary.items()},
                    'faults': {name: sorted(set(str(frame.get('timing_fault')) for frame in arm['frames'] if frame['gpu_ms'] is None))
                               for name, arm in arms.items()},
                    'disposition': 'Compare the presence of the fault in the foam-off and water-off arms. Keep the separate completion brackets where the collector rejects the pass batch.',
                })
            for name, elapsed in active['passes_ms'].items():
                base = control['passes_ms'].get(name, 0)
                if base > 0.05 and elapsed > 1.25 * base:
                    investigations.append({'wire': '25 percent paired pass increase', 'phase': phase,
                                           'pass': name, 'foam_ms': elapsed, 'foam_off_ms': base,
                                           'disposition': 'Water surface includes pattern and endpoint reads. Common passes in all arms show the step and repeat spread under concurrent load.'})
        result['views'][location] = {'summary': summary, 'investigations': investigations, 'record': record}
    path = RECORDS['foam']
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(result, indent=2) + '\n')
    print(f'PASS paired 1440p Metal whitewater records: {path.relative_to(ROOT)}')


def measure_surf_arm(mode, arm=None):
    scratch = ROOT / ('tmp/waters-WP21' if arm else 'tmp/waters-WP19')
    scratch.mkdir(parents=True, exist_ok=True)
    revision = run(['git', 'rev-parse', 'HEAD'], capture_output=True, text=True).stdout.strip()
    load_before = run(['uptime'], capture_output=True, text=True).stdout.strip()
    with tempfile.TemporaryDirectory(dir=scratch, prefix=mode + '-') as directory:
        output = Path(directory)
        store = output / 'store'
        run(['cargo', 'run', '--release', '--locked', '--offline', '-p', 'thrum-cook', '--',
             'cook', 'examples/water/assets', str(store)])
        env = dict(os.environ, THRUM_WATER_PROOF=mode, THRUM_ASSET_STORE=str(store),
                   THRUM_WATER_OUTPUT=str(output), THRUM_REQUIRE_GPU='1')
        if arm:
            env['THRUM_WATER_SURF_ARM'] = arm
        run(['cargo', 'test', '--release', '--locked', '--offline', '-p', 'water-scene',
             '--test', 'proof_gpu', '--', '--nocapture'], env=env)
        record = json.loads((output / (mode + '.json')).read_text())
    record.update(commit=revision, host=platform.platform(), load_before=load_before,
                  load=run(['uptime'], capture_output=True, text=True).stdout.strip(),
                  recipe='portable' if env.get('THRUM_WATER_RECIPE') == 'portable' else 'native')
    return record


def save_surf_record(record, path):
    path.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=path.parent, prefix='surf-record-') as directory:
        staged = Path(directory) / path.name
        staged.write_text(json.dumps(record, indent=2) + '\n')
        staged.replace(path)


def measure_surf_running(arm_name=None):
    record = measure_surf_arm('surf-running', arm_name)
    assert record['target'] == 'mac-metal'
    assert record['step_hz'] == 60 and 0 < record['maximum_age'] <= 8
    problems = []
    for name in (('open_water',) if arm_name else ('open_water', 'breakers')):
        arm = record[name]
        ticks = arm['ticks']
        assert ticks >= 600 and arm['extent'] == record['extent']
        assert arm['wall_seconds'] >= ticks / record['step_hz']
        assert arm['frames_completed'] > 0
        view_times = arm['view_completion_ms']
        assert len(view_times) == arm['frames_completed']
        arm['view_completion_summary_ms'] = {
            'median': statistics.median(view_times), 'max': max(view_times),
        }
        assert len(arm['age']) == record['maximum_age'] + 1 and arm['age'][0] == 0
        expected = 48 * ticks
        arm['delivery_share'] = {
            'one': arm['age'][1] / expected, 'two': arm['age'][2] / expected,
            'older': sum(arm['age'][3:]) / expected, 'not_drained': (expected - sum(arm['age'])) / expected,
        }
        if arm['delivery_share']['one'] + arm['delivery_share']['two'] < 0.90:
            problems.append(f"{name} missed the 90% window: {arm['delivery_share']}")
        for key in ('fallback', 'unavailable', 'blocking_reads', 'refused'):
            if arm[key]:
                problems.append(f"{name} reported {arm[key]} {key}")
        nodes = arm['nodes']
        if len(nodes) != ticks:
            problems.append(f"{name} recorded {len(nodes)} node counts for {ticks} ticks")
        if name == 'breakers':
            if arm['completed_batch_ticks'] != ticks or not all(coarse > 0 and fine > 0 for coarse, fine in nodes):
                problems.append('breakers did not show cuts on every measured tick')
        elif arm['completed_cut_nodes'] != [0, 0]:
            problems.append('open water unexpectedly made cuts')
        arm['node_summary'] = {
            phase: {'min': min(column), 'mean': statistics.mean(column), 'max': max(column)}
            for phase, column in zip(('coarse', 'fine'), zip(*nodes), strict=True)
        }
    if arm_name:
        arm = record['open_water']
        batches = arm['batches']
        assert len(batches) == arm['ticks']
        assert len(arm['views']) == arm['frames_submitted']
        assert sum('completed_s' in view for view in arm['views']) == arm['frames_completed']
        counts = [0] * (record['maximum_age'] + 1)
        late = dict(queue=0, device=0, drain=0)
        refusals = {event['tick']: event for event in arm['tick_refusals']}
        for index, batch in enumerate(batches):
            posted = batch['posted']
            assert posted == batches[0]['posted'] + index
            submitted, published = batch['submitted'], batch['published']
            assert submitted is None or submitted >= posted
            assert published is None or submitted is not None and published >= submitted
            if submitted is None or submitted > posted:
                batch['refusal'] = refusals.get(posted, 'no submission at the posting tick')
            assert len(batch['first_drains']) == 8
            for delivered in batch['first_drains']:
                if delivered is not None:
                    assert published is not None and delivered >= published
                    counts[delivered - posted] += 6
                if delivered is None or delivered - posted > 2:
                    waiting = ('queue' if submitted != posted else
                               'device' if published is None or published >= posted + 2 else 'drain')
                    late[waiting] += 6
        assert counts == arm['age']
        arm['late_stations_by_wait'] = late
        record['problems'] = problems
        path = WORKING / 'WP21/surf-running-open-water.json'
        save_surf_record(record, path)
        assert not problems, '; '.join(problems) + f" ({path.relative_to(ROOT)})"
        print(f'PASS wall-clock Metal open-water query: {path.relative_to(ROOT)}')
        return
    for witness in record['witness']:
        if len(witness['stations']) != 48 or witness['max_error_m'] > record['tolerance_m']:
            problems.append(f"model agreement failed at measured post {witness['post_index']}: "
                            f"{len(witness['stations'])} stations, {witness['max_error_m']} m error")
    assert len(record['witness']) == 3
    control, surf = (record[name] for name in ('open_water', 'breakers'))
    control_late, surf_late = (arm['delivery_share']['older'] + arm['delivery_share']['not_drained']
                              for arm in (control, surf))
    record['investigations'] = []
    if surf_late > control_late:
        record['investigations'].append({
            'wire': 'breakers later than open water',
            'open_water': {key: control[key] for key in ('delivery_share', 'node_summary', 'busy',
                                                        'dropped_views', 'view_completion_summary_ms')},
            'breakers': {key: surf[key] for key in ('delivery_share', 'node_summary', 'busy',
                                                   'dropped_views', 'view_completion_summary_ms')},
            'disposition': 'Both arms draw the same island view. The breakers add the coarse and fine cut; '
                           'frame skips and occupied readback slots distinguish visible backpressure from '
                           'the GPU and OS scheduling variation that also affects the control.',
        })
    record['problems'] = problems
    path = WORKING / 'WP19/surf-running.json'
    save_surf_record(record, path)
    assert not problems, '; '.join(problems) + f" ({path.relative_to(ROOT)})"
    print(f'PASS paired wall-clock Metal surf query: {path.relative_to(ROOT)}')


def surf_cache_investigation(record, table, wp13):
    history = json.loads(SURF_CACHE_INVESTIGATION.read_text())
    assert all(record[key] == history[key] for key in ('camera', 'extent', 'wind'))
    work = record['cache_work']
    assert work['texels_per_full_cell_stage'] == 56 and work['texels_per_lag_cell_stage'] == 48
    footprints = {'own_cell', 'swash_contour', 'across_cell_edge', 'ring_edge', 'previous_stage', 'distant_lattice'}
    for state in ('stable_stage', 'moved_stage'):
        frame = work[state]
        for domain in ('mesh', 'distant_lattice'):
            assert 0 < frame[domain]['evaluated'] <= frame[domain]['allocated']
        assert frame['texels'].keys() == footprints
        assert all(isinstance(count, int) and count >= 0 for count in frame['texels'].values())
    assert work['stable_stage']['texels']['previous_stage'] == 0
    assert work['moved_stage']['texels']['previous_stage'] > 0
    ablations = history['ablations']
    assert ablations.keys() == {'unmodified', 'without_table_reads', 'half_components', 'half_nodes'}
    assert all(len(phase) == 12 and all(ms > 0 for ms in phase)
               for ablation in ablations.values() for phase in ablation['cache_samples_ms'].values())
    label = 'surf field cache'
    return {
        'wire': 'surf field cache cost',
        'history': history,
        'cache_ms': {
            phase: {
                'current': table['surf'][phase]['passes_ms'][label],
                'wp13': wp13['summary']['surf'][phase]['passes_ms'][label],
                'wp19_review_fix': history['wp19_review_fix']['cache_ms'][phase],
                'pre_change': {name: statistics.median(variant['cache_samples_ms'][phase])
                               for name, variant in ablations.items()},
            } for phase in ('step', 'repeat')
        },
        'changes': [
            'Read packed lag layers by sector pair and share the exposure layer across two pairs. The component loop performs no shore-table texture loads.',
            'Prepare the wave and shore parameters for both presentations in one component loop at equal stage. Read the previous stage separately when it changes.',
            'Skip mesh nodes outside the displaced triangle and normal footprint and nodes inside undrawn ring holes. Use only the coarser nodes at a ring edge. Retain the distant lattice cull.',
        ],
        'work': work,
    }


def measure_surf():
    record = measure_surf_arm('surf-cost')
    prior = json.loads(SURF_HISTORY.read_text())
    wp13 = json.loads(WP13_SURF.read_text())
    assert record['target'] == 'mac-metal' and record['extent'] == prior['extent'] == [2560, 1440]
    assert record['capture'] is False and record['camera'] == prior['camera']
    assert record['wind'] == prior['wind'] == 'Gale'
    assert record['camera'] == wp13['camera'] and record['extent'] == wp13['extent']
    record.update(prior=prior, wp13=wp13,
                  comparison='The current arms and WP13 share the camera, gale forcing and extent. '
                             'The M7 reference used the earlier swell forcing.')
    arms = dict(zip(('surf', 'surf-disabled', 'water-absent'), record['arms'], strict=True))
    control = arms['surf-disabled']['frames']
    phases = [(f['tick'], f['alpha']) for f in control]
    table = {}
    query_frames = {}
    for (name, arm), (water, surf) in zip(arms.items(), ((True, True), (True, False), (False, False)), strict=True):
        assert (arm['water'], arm['surf']) == (water, surf)
        assert [(f['tick'], f['alpha']) for f in arm['frames']] == phases
        resources = dict(arm['resources'])
        for key in ('surf lags', 'surf exposures'):
            assert (resources.get(key, 0) > 0) == surf
        cut_scratch = resources.get('water query coarse lattice', 0)
        assert (cut_scratch > 0) == surf
        table[name] = wake_summary(arm['frames'])
        for frame in arm['frames']:
            labels = frame['passes_ms']
            if frame['gpu_ms'] is not None:
                assert any(label.endswith('surf field cache') for label in labels) == surf
            if 'fleet_query' in frame:
                assert name not in query_frames and frame['tick_frame']
                assert frame['fleet_query']['floats'] == 8 and frame['fleet_query']['stations'] == 48
                counts = frame['fleet_query']
                assert counts['cut_nodes'] == counts['coarse_nodes'] + counts['fine_nodes']
                assert (counts['coarse_nodes'] > 0) == (counts['fine_nodes'] > 0) == surf
                if surf:
                    # Coarse nodes alone occupy one 16-byte record each. The
                    # retained allocation also includes the jobs and fine work.
                    assert cut_scratch >= 16 * counts['coarse_nodes'], (
                        f'{cut_scratch} scratch bytes cannot hold {counts["coarse_nodes"]} coarse nodes')
                query_frames[name] = frame['fleet_query']
                elapsed = next((value for label, value in labels.items()
                                if label.endswith('water body query')), None)
                assert elapsed is not None, f"{name} query timestamp unavailable: {frame.get('timing_fault')}"
                query_frames[name]['gpu_ms'] = elapsed
    assert set(query_frames) == {'surf', 'surf-disabled'}
    record['query'] = query_frames
    record['summary'] = table
    investigations = [surf_cache_investigation(record, table, wp13)]
    for phase in ('step', 'repeat'):
        treatment = table['surf'][phase]
        paired = table['surf-disabled'][phase]
        old = prior['summary']['foam'][phase]
        previous = wp13['summary']['surf'][phase]
        if max(treatment['completion_upper_ms'], treatment['gpu_ms'] or 0) > 1000 / 120:
            investigations.append({
                'wire': '120 Hz interval', 'phase': phase,
                'arms': {name: {k: rows[phase][k] for k in ('gpu_ms', 'completion_upper_ms')}
                         for name, rows in table.items()},
                'surf_cache_ms': treatment['passes_ms'].get('surf field cache'),
                'disposition': 'The surf-disabled arm isolates the added field cache and surface path. The water-absent arm keeps the same sky and opaque ground. The completion bracket includes the fixture’s synchronous wait.',
            })
        if any(rows[phase]['timestamped_samples'] != rows[phase]['samples'] for rows in table.values()):
            investigations.append({
                'wire': 'timestamp availability', 'phase': phase,
                'faults': {name: sorted({str(f.get('timing_fault')) for f in arm['frames']
                                         if f['gpu_ms'] is None and f['tick_frame'] == (phase == 'step')})
                           for name, arm in arms.items()},
                'disposition': 'The affected passes have no imputed duration. Completion is reported separately.',
            })
        for label, elapsed in treatment['passes_ms'].items():
            for baseline, row in (('surf-disabled', paired), ('M7', old), ('WP13', previous)):
                before = row['passes_ms'].get(label, 0)
                if before > 0 and elapsed >= 1.25 * before:
                    investigations.append({
                        'wire': '25 percent pass increase', 'phase': phase, 'pass': label,
                        'baseline': baseline, 'before_ms': before, 'current_ms': elapsed,
                        'disposition': 'Use the same-forcing paired arms to distinguish a surf cost from the earlier forcing and shared machine load.',
                    })
    if query_frames['surf']['gpu_ms'] > 0.1:
        investigations.append({
            'wire': 'body query time', 'arms': query_frames, 'wp13_query': wp13['query'],
            'disposition': 'The query target is 0.1 ms outside surf. Both arms queried the same stations and only the surf arm evaluates the highest-surface cut.',
        })
    record['investigations'] = investigations
    path = RECORDS['surf']
    save_surf_record(record, path)
    print(f'PASS paired 1440p Metal surf record: {path.relative_to(ROOT)}')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['record', 'check', 'running'])
    parser.add_argument('--scene', choices=list(RECORDS), required=True)
    parser.add_argument('--arm', choices=['open-water'])
    args = parser.parse_args()
    if args.action == 'running':
        if args.scene != 'surf':
            parser.error('running is only available for surf')
        measure_surf_running(args.arm)
        return
    if args.scene == 'wake':
        measure_wake()
        return
    if args.scene == 'foam':
        measure_foam()
        return
    if args.scene == 'surf':
        measure_surf()
        return
    record = RECORDS[args.scene]
    data = measure(args.scene)
    validate(data, args.scene)
    if args.action == 'record':
        record = WORKING / f'{args.scene}-cost.json'
        record.parent.mkdir(parents=True, exist_ok=True)
        record.write_text(json.dumps(data, indent=2) + '\n')
    else:
        validate(json.loads(record.read_text()), args.scene)
    print(f'PASS paired {args.scene} record: {record.relative_to(ROOT)}')


if __name__ == '__main__':
    main()
