#!/usr/bin/env python3
"""Paired water cost record. Timing numbers are tripwires, never pass thresholds."""
import argparse
import json
import math
import os
from pathlib import Path
import platform
import re
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
           'surf': ROOT / '.orchestrate/evidence/WATERS/WP21/surf-cost.json',
           'channel': ROOT / '.orchestrate/evidence/WATERS/WP9/channel-cost.json',
           'river-calm': ROOT / '.orchestrate/evidence/RIVERS/WP5/river-calm-cost.json',
           'river': ROOT / '.orchestrate/evidence/RIVERS/WP4/river-cost.json',
           'rain-puddles': ROOT / 'tests/fixtures/puddles/yard-water-cost.json',
           'rain-rings': ROOT / 'tests/fixtures/puddles/rain-rings-cost.json',
           'rain-footfall': ROOT / '.orchestrate/evidence/PUDDLES/WP8/rain-footfall-cost.json',
           'under': ROOT / 'tests/fixtures/waters/dive-cost-prior.json',
           'optics': ROOT / 'tests/fixtures/waters/optics-cost-prior.json'}
WORKING = ROOT / '.orchestrate/evidence/WATERS'
# The island record taken before the records moved to fixtures names the first-frame record by its old path,
# which ends in this name.
FIRST_FRAME_FORMER = '/WATERS/M2-cost.json'
WAKE_HISTORY = ROOT / 'examples/water/tests/fixtures/wake-cost-prior.json'
SURF_HISTORY = ROOT / 'examples/water/tests/fixtures/surf-cost-prior.json'
RIVER_HISTORY = ROOT / 'examples/water/tests/fixtures/river-cost-wp4.json'
WP13_SURF = ROOT / 'examples/water/tests/fixtures/surf-cost-wp13.json'
SURF_CACHE_INVESTIGATION = ROOT / 'examples/water/tests/fixtures/surf-cache-investigation.json'
WAKE_ROUTE = {'wind': 'trade', 'view': 'overhead', 'speed_mps': 6}
WAKE_ARMS = {'with-wake': (True, True), 'without-wake': (True, False),
             'water-absent': (False, False)}


def run(command, **kwargs):
    return subprocess.run(command, cwd=ROOT, check=True, **kwargs)


def measure(scene):
    island = scene == 'island'
    channel = scene == 'channel'
    calm_river = scene == 'river-calm'
    river = scene in ('river', 'river-calm')
    output = ROOT / ({
        'river-calm': 'tmp/rivers-wp5/cost-current',
        'river': 'tmp/rivers-wp4/cost-current', 'channel': 'tmp/waters-wp9/cost-current', 'island': 'tmp/waters-M4/cost-current',
    }.get(scene, 'tmp/waters-M2/cost-current'))
    output.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, THRUM_WATER_PROOF='cost', THRUM_WATER_OUTPUT=str(output))
    if channel:
        env['THRUM_WATER_CHANNEL'] = 'trade'
    else:
        env.pop('THRUM_WATER_CHANNEL', None)
    if river:
        env['THRUM_WATER_RIVER'] = 'calm' if calm_river else 'trade'
    elif channel:
        env.pop('THRUM_WATER_RIVER', None)
    if island:
        env['THRUM_WATER_ISLAND'] = 'trade'
    else:
        env.pop('THRUM_WATER_ISLAND', None)
    store = output / 'store'
    run(['cargo', 'run', '--release', '--locked', '--offline', '-p', 'thrum-cook', '--', 'cook', 'examples/water/assets', str(store)])
    env['THRUM_ASSET_STORE'] = str(store)
    products = None
    if river:
        with tempfile.TemporaryDirectory(prefix='solve-', dir=output) as directory:
            solved = run(['cargo', 'run', '--release', '--locked', '--offline', '-p', 'water-scene',
                          '--', '--prepare-rivers', directory], text=True, capture_output=True)
            products = river_products(solved, Path(directory))
            # Keep measured products beside the cooked store for both GPU arms.
            river_store = output / 'rivers'
            river_store.mkdir(exist_ok=True)
            for product in products:
                key = product['key']
                (Path(directory) / f'{key}.river').replace(river_store / f'{key}.river')
            env['THRUM_WATER_RIVER_STORE'] = str(river_store)
    run(['cargo', 'test', '--release', '--locked', '--offline', '-p', 'water-scene', '--test', 'proof_gpu'], env=env)
    data = json.loads((output / 'cost.json').read_text())
    data['commit'] = run(['git', 'rev-parse', 'HEAD'], capture_output=True, text=True).stdout.strip()
    data['host'] = platform.platform()
    data['load'] = run(['uptime'], capture_output=True, text=True).stdout.strip()
    data['build'] = 'release; concurrent machine load, not qualified cadence'
    data['refresh_hz'] = 120
    data['view'] = ('40 m island overlook in trade wind at noon; same cooked ground and sky at identical world times'
                    if island else 'camera at 2 m, bearing 90 degrees, noon; camera record gives origin and lens; identical calibration float and sky in both arms at identical world times')
    if channel:
        data['view'] = 'Narrows camera under trade wind; same cooked outlet terrain, float, post and sky in both arms'
    if river:
        data['view'] = ('Spring sill and narrows in a calm, with identical terrain, floats and sky in both arms'
                        if calm_river else
                        'Both rivers from above the lake in trade wind, with identical terrain, floats and sky in both arms')
        data['river_files'] = products
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
    if channel:
        prior = json.loads(RECORDS['island'].read_text())
        data['island_record'] = {
            'path': str(RECORDS['island'].relative_to(ROOT)),
            'commit': prior['commit'], 'view': prior['view'], 'summary': prior['summary'],
        }
        data['investigations'].extend(channel_investigations(data['summary'], prior['summary']))
    if scene == 'river':
        # Keep the current channel comparison in memory and preserve its saved record.
        channel_record = measure('channel')
        data['channel_record'] = {key: channel_record[key] for key in ('view', 'commit', 'summary')}
        for row, label, baseline, duration in channel_growth(data['summary'], channel_record['summary']):
            opposite = next(arm for arm in data['summary']
                            if arm['water'] != row['water'] and arm['tick_frame'] == row['tick_frame'])
            data['investigations'].append({
                'wire': 'pass grew by at least 25% compared to channel',
                'water': row['water'], 'tick_frame': row['tick_frame'], 'pass': label,
                'measurement': f'{label} grew from {baseline:.3f} ms to {duration:.3f} ms.',
                'paired_ms': opposite['passes_ms'].get(label),
                'disposition': ('The river overview shows more terrain and wet coverage than the channel narrows. '
                                'The paired water-off timing shows which changes also occur without water. '
                                'This comparison does not isolate coverage from the cost of mixing samples.'),
            })
    if calm_river:
        prior = json.loads(RIVER_HISTORY.read_text())
        data['wp4'] = prior
        for row, label, baseline, duration in channel_growth(data['summary'], prior['summary']):
            data['investigations'].append({
                'wire': 'pass grew by at least 25% compared to WP4',
                'water': row['water'], 'tick_frame': row['tick_frame'], 'pass': label,
                'measurement': f'{label} grew from {baseline:.3f} ms to {duration:.3f} ms.',
                'disposition': ('The paired dry arm measures scene work for the closer bank camera. '
                                'The new flow transforms and foam step have their own timestamps. '
                                'Wind, depth coverage and camera differ from WP4, so shared passes also reflect scene changes.'),
            })
        for row in data['summary']:
            if row['water']:
                new_passes = {label: ms for label, ms in row['passes_ms'].items()
                              if 'river flow bands' in label or 'whitewater' in label}
                data['investigations'].append({
                    'wire': 'added river passes', 'water': True, 'tick_frame': row['tick_frame'],
                    'measurement': new_passes,
                    'disposition': ('The flow transforms reuse wind FFT pipelines and use independent scratch for their cadence. The whitewater transport cost belongs '
                                    'to tick frames. No dedicated texture binding was added to the surface.'),
                })
    return data


def river_products(solved, directory):
    by_name = {}
    for name, gauge, seconds, steps, rate, imbalance in re.findall(
        r'river sample (.+?) gauge=([\d.]+) seconds=([\d.]+) steps=(\d+) '
        r'level_rate=([\d.e+-]+) imbalance=([\d.e+-]+)', solved.stderr
    ):
        by_name.setdefault(name, []).append({
            'gauge': float(gauge), 'seconds': float(seconds), 'steps': int(steps),
            'level_rate': float(rate), 'imbalance': float(imbalance),
        })
    return [
        {'name': name, 'key': key, 'bytes': (directory / f'{key}.river').stat().st_size,
         'samples': by_name[name]}
        for name, key in zip(('Spring stream', 'Lake river'), solved.stdout.splitlines(), strict=True)
    ]


def comparable_pass(label):
    # Contribution indices depend on what the scene declares. Fixed scene
    # passes retain their complete name.
    return label.rsplit(': ', 1)[-1] if label.startswith('effect ') else label


def channel_growth(summary, island):
    prior = {(row['water'], row['tick_frame']): row['passes_ms'] for row in island}
    for row in summary:
        old = {comparable_pass(label): duration
               for label, duration in prior[row['water'], row['tick_frame']].items()}
        for label, duration in row['passes_ms'].items():
            baseline = old.get(comparable_pass(label))
            if baseline and duration >= baseline * 1.25:
                yield row, label, baseline, duration


def channel_investigations(summary, island):
    arms = {(row['water'], row['tick_frame']): row for row in summary}
    fixed_output = {'synthetic tone encode', 'V1 surface present'}
    for row, label, baseline, duration in channel_growth(summary, island):
        paired = arms[not row['water'], row['tick_frame']]
        other = paired['passes_ms'].get(label)
        control = f' Paired arm {other:.3f} ms.' if other is not None else ' Absent in the paired arm.'
        if label in fixed_output:
            disposition = ('Both scenes run this pass at the same 2560×1440 output extent. '
                           'Use the paired arm to see whether the common output timing also grew '
                           'with water absent. The attributed timing and completion bracket '
                           'under shared load do not isolate scheduling or attribution from '
                           'input-dependent work.')
            if other is not None and other < baseline * 1.25:
                disposition += (' There is no corresponding increase over the island comparison '
                                'baseline in the channel opposite arm. The increase does not '
                                'track the number of pixels encoded or presented.')
        elif label.startswith('effect '):
            disposition = ('Compared the same named contribution with the island. The camera, '
                           'wet coverage and participating chart differ. The paired arm '
                           'shows whether the pass is absent when water is removed.')
        else:
            disposition = ('Compared the same named scene pass and tick kind, and reported '
                           'the paired arm under the same load. Island and channel have '
                           'different cameras and visible content. Timing attribution and '
                           'content-dependent work are unresolved by this comparison.')
        yield {
            'wire': 'pass grew by at least 25% compared to island',
            'water': row['water'], 'tick_frame': row['tick_frame'], 'pass': label,
            'island_ms': baseline, 'channel_ms': duration, 'paired_ms': other,
            'measurement': (f'{label}: {baseline:.3f} -> {duration:.3f} ms.' + control
                            + f" Channel frame pass sum {row['gpu_ms']:.3f} ms, "
                            + f"completion upper bracket {row['completion_upper_ms']:.3f} ms."),
            'disposition': disposition,
        }


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
    if scene == 'channel':
        assert data['island_record']['summary']
        investigated = {(item['water'], item['tick_frame'], item['pass'])
                        for item in data['investigations']
                        if item['wire'] == 'pass grew by at least 25% compared to island'
                        and item.get('measurement') and item.get('disposition')}
        for row, label, _, _ in channel_growth(data['summary'], data['island_record']['summary']):
            assert (row['water'], row['tick_frame'], label) in investigated, label
        assert dict(data['body_vertices'])['Lake outlet'] > 0
    if scene == 'river':
        assert [len(file['samples']) for file in data['river_files']] == [3, 5]
        assert all(file['bytes'] > 0 and all(sample['steps'] > 0 for sample in file['samples'])
                   for file in data['river_files'])
        investigated = {(item['water'], item['tick_frame'], item['pass'])
                        for item in data['investigations']
                        if item['wire'] == 'pass grew by at least 25% compared to channel'
                        and item.get('measurement') and item.get('disposition')}
        for row, label, _, _ in channel_growth(data['summary'], data['channel_record']['summary']):
            assert (row['water'], row['tick_frame'], label) in investigated, label
        for name in ['Spring stream', 'Lake river']:
            assert any(n.startswith(name) and count > 0 for n, count in data['body_vertices'])
    if scene == 'river-calm':
        assert data['extent'] == [2560, 1440] == data['wp4']['extent']
        wet = [row for row in data['summary'] if row['water']]
        assert all(any('river flow bands' in label for label in row['passes_ms']) for row in wet)
        assert any('whitewater' in label for row in wet for label in row['passes_ms'])
        investigated = {(item['water'], item['tick_frame'], item['pass'])
                        for item in data['investigations']
                        if item['wire'] == 'pass grew by at least 25% compared to WP4'}
        for row, label, _, _ in channel_growth(data['summary'], data['wp4']['summary']):
            assert (row['water'], row['tick_frame'], label) in investigated, label
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


def late_stations_by_interval(arm, maximum_age):
    ticks = arm['ticks']
    batches = arm['batches']
    assert len(batches) == ticks
    assert len(arm['views']) == arm['frames_submitted']
    assert sum('completed_s' in view for view in arm['views']) == arm['frames_completed']
    counts = [0] * (maximum_age + 1)
    late = dict(before_submission=0, between_submission_and_publication=0, after_publication=0)
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
                # Publication records the latest tick whose drain had begun.
                if submitted != posted:
                    interval = 'before_submission'
                elif published is None or published >= posted + 2:
                    interval = 'between_submission_and_publication'
                else:
                    interval = 'after_publication'
                late[interval] += 6
    assert counts == arm['age']
    assert sum(late.values()) == 48 * ticks - sum(counts[1:3])
    return late


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
        arm['late_stations_by_interval'] = late_stations_by_interval(arm, record['maximum_age'])
    if arm_name:
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
        comparison_keys = ('delivery_share', 'node_summary', 'busy', 'dropped_views',
                           'view_completion_summary_ms', 'late_stations_by_interval')
        record['investigations'].append({
            'wire': 'breakers later than open water',
            'open_water': {key: control[key] for key in comparison_keys},
            'breakers': {key: surf[key] for key in comparison_keys},
            'disposition': 'Both arms draw the same island view. Compare their submission, publication '
                           'and first-drain ticks. The interval between submission and publication contains '
                           'device work and any delay before the host polls the result.',
        })
    record['problems'] = problems
    path = WORKING / 'WP19/surf-running.json'
    save_surf_record(record, path)
    assert not problems, '; '.join(problems) + f" ({path.relative_to(ROOT)})"
    print(f'PASS paired wall-clock Metal surf query: {path.relative_to(ROOT)}')


def measured_cache_ms(value, source, phase):
    if type(value) not in (int, float) or not math.isfinite(value) or value < 0:
        raise ValueError(f'{source}: missing or invalid surf field cache timing for {phase}: {value!r}')
    return value


def surf_cache_investigation(record, table, wp13):
    history = json.loads(SURF_CACHE_INVESTIGATION.read_text())
    assert all(record[key] == history[key] for key in ('camera', 'extent', 'wind'))
    work = record['cache_work']
    assert work['texels_per_full_cell_stage'] == 56 and work['texels_per_lag_cell_stage'] == 48
    footprints = {'own_cell', 'swash_contour', 'across_cell_edge', 'ring_edge', 'previous_stage', 'distant_lattice'}
    for state in ('stable_stage', 'moved_stage'):
        frame = work[state]
        for domain in ('mesh', 'distant_lattice'):
            evaluated, allocated = (frame[domain][key] for key in ('evaluated', 'allocated'))
            assert type(evaluated) is int and type(allocated) is int and 0 < evaluated <= allocated
        assert frame['texels'].keys() == footprints
        assert all(type(count) is int and count >= 0 for count in frame['texels'].values())
    assert work['stable_stage']['texels']['previous_stage'] == 0
    assert work['moved_stage']['texels']['previous_stage'] > 0
    ablations = history['ablations']
    assert ablations.keys() == {'unmodified', 'without_table_reads', 'half_components', 'half_nodes'}
    label = 'surf field cache'
    timings = {}
    for phase in ('step', 'repeat'):
        pre_change = {}
        for name, variant in ablations.items():
            samples = variant['cache_samples_ms'].get(phase)
            if not isinstance(samples, list) or len(samples) != 12:
                raise ValueError(f'{name}: expected 12 pre-change cache measurements for {phase}')
            pre_change[name] = statistics.median(measured_cache_ms(ms, name, phase) for ms in samples)
        timings[phase] = {
            'current': measured_cache_ms(table['surf'][phase]['passes_ms'].get(label), 'current', phase),
            'wp13': measured_cache_ms(wp13['summary']['surf'][phase]['passes_ms'].get(label), 'WP13', phase),
            'wp19_review_fix': measured_cache_ms(history['wp19_review_fix']['cache_ms'].get(phase),
                                                 'WP19 review fix', phase),
            'pre_change': pre_change,
        }
    return {
        'wire': 'surf field cache cost',
        'history': history,
        'cache_ms': timings,
        'changes': [
            'Read packed lag layers by sector pair and share the exposure layer across two pairs. The component loop performs no shore-table texture loads.',
            'Prepare the wave and shore parameters for both presentations in one component loop at equal stage. Read the previous stage separately when it changes.',
            'Skip mesh nodes outside the displaced triangle and normal footprint and nodes inside undrawn ring holes. Use only the coarser nodes at a ring edge. Retain the distant lattice cull.',
        ],
        'work': work,
    }


def surf_pass_investigation(label, phase, arms):
    samples_by_arm = {}
    for name, arm in arms.items():
        samples = [ms for frame in arm['frames'] if frame['tick_frame'] == (phase == 'step')
                   for key, ms in frame['passes_ms'].items() if key.rsplit(': ', 1)[-1] == label]
        if samples:
            samples_by_arm[name] = dict(count=len(samples), low_ms=min(samples),
                                        median_ms=statistics.median(samples), high_ms=max(samples))
    if label == 'water body query':
        finding = 'The paired arms queried the same 48 stations. The surf arm also cut the surface; see query node counts.'
    elif label == 'whitewater':
        finding = 'The surf arm evaluates transformed shore birth. Compare the sampled spread with the same-forcing whitewater arm.'
    elif set(samples_by_arm) == set(arms):
        finding = 'The pass layout is common to all three arms. Timing spreads show the native counters under shared load; there is no isolated pass substitution.'
    else:
        finding = 'The pass runs in the two water arms. The paired samples quantify the increase with surf enabled; the shared run does not isolate contention.'
    return dict(arms=samples_by_arm, disposition=finding)


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
                'largest_passes': {name: sorted(rows[phase]['passes_ms'].items(),
                                                key=lambda item: item[1], reverse=True)[:5]
                                   for name, rows in table.items()},
                'disposition': 'Compare the same-view completion brackets by arm. Device pass medians report individual timings and can overlap. The water-absent arm keeps the same sky and opaque ground.',
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
                        **surf_pass_investigation(label, phase, arms),
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


def optics_key(view):
    return f"{view['view']}/{view['wind']}/{view['body_count']}"


def optics_summary(data, states):
    assert data['target'] == 'mac-metal' and data['extent'] == [2560, 1440]
    assert data['capture'] is False and data['commit']
    views = {optics_key(view): view for view in data['views']}
    expected = {f'sea/{wind}/{count}' for wind in ('Calm', 'Gale') for count in (0, 1, 9)}
    assert views.keys() == expected | {'lake/Calm/None'}
    summary = {}
    for named, view in views.items():
        arms = {row['enabled']: row for row in view['arms']}
        assert arms.keys() == states
        sources = None
        observed_view = None
        summary[named] = {}
        for enabled, arm in arms.items():
            frames = arm['frames']
            timeline = [(frame['tick'], frame['alpha']) for frame in frames]
            assert sources is None or sources == timeline
            sources = timeline
            if view['body_count'] is not None:
                witness = arm['visibility']
                assert len(witness['fleet_centres_ndc']) == view['body_count'], named
                plane = witness['separating_plane']
                assert 0 <= plane < 6 and len(witness['caster_clip_corners']) == 8
                for x, y, z, w in witness['caster_clip_corners']:
                    assert (w+x, w-x, w+y, w-y, z, w-z)[plane] < 0, named
                assert all(-1 < x < 1 and -1 < y < 1 and 0 < z < 1
                           for x, y, z in witness['fleet_centres_ndc']), named
                assert observed_view is None or observed_view == witness, named
                observed_view = witness
            phases = wake_summary(frames)
            for row in phases.values():
                assert row['timestamped_samples'] > 0, f'{named}: {arm["timing"]}'
                assert 'water surface' in row['passes_ms']
                capture_needed = enabled and view['body_count'] != 0
                assert ('water gather' in row['passes_ms']) == capture_needed, named
            summary[named]['optics' if enabled else 'control'] = phases
    return summary


def measure_optics(action):
    output = ROOT / 'tmp/wp8/paired-cost'
    output.mkdir(parents=True, exist_ok=True)
    store = output / 'store'
    run(['cargo', 'run', '--release', '--locked', '--offline', '-p', 'thrum-cook', '--',
         'cook', 'examples/water/assets', str(store)])
    commit = run(['git', 'rev-parse', 'HEAD'], capture_output=True, text=True).stdout.strip()
    env = dict(os.environ, THRUM_WATER_PROOF='optics-cost', THRUM_WATER_COMMIT=commit,
               THRUM_WATER_OUTPUT=str(output), THRUM_ASSET_STORE=str(store))
    env['THRUM_WATER_OPTICS_COST_ARM'] = 'prior' if action == 'record' else 'paired'
    run(['cargo', 'test', '--release', '--locked', '--offline', '-p', 'water-scene',
         '--test', 'proof_gpu'], env=env)
    data = json.loads((output / 'optics-cost.json').read_text())
    data['host'] = platform.platform()
    data['load'] = run(['uptime'], capture_output=True, text=True).stdout.strip()
    data['summary'] = optics_summary(data, {False} if action == 'record' else {False, True})
    destination = WORKING / 'WP8'
    destination.mkdir(parents=True, exist_ok=True)
    if action == 'record':
        path = destination / f'optics-cost-prior-{commit[:12]}.json'
    else:
        prior = json.loads(RECORDS['optics'].read_text())
        data['prior_record'] = str(RECORDS['optics'].relative_to(ROOT))
        data['prior_summary'] = optics_summary(prior, {False})
        data['investigations'] = optics_investigations(data['summary'], data['prior_summary'])
        path = destination / f'optics-cost-{commit[:12]}.json'
    path.write_text(json.dumps(data, indent=2) + '\n')
    print(f'PASS paired optics record {path.relative_to(ROOT)}')


def optics_investigations(summary, prior):
    investigations = []
    for view, pair in summary.items():
        for phase, current in pair['optics'].items():
            control = pair['control'][phase]
            historical = prior[view]['control'][phase]
            surface = current['passes_ms']['water surface']
            unchanged_surface = control['passes_ms']['water surface']
            gather = current['passes_ms'].get('water gather', 0.0)
            facts = dict(view=view, phase=phase, surface_ms=surface, gather_ms=gather,
                         rays_and_composition_delta_ms=surface-unchanged_surface,
                         control_surface_ms=unchanged_surface,
                         prior_surface_ms=historical['passes_ms']['water surface'],
                         completion_upper_ms=current['completion_upper_ms'],
                         paired_completion_upper_ms=control['completion_upper_ms'])
            if current['timestamped_samples'] != current['samples']:
                investigations.append(dict(facts, wire='timestamp batch rejected',
                    disposition='Completion brackets retain every submitted frame. Named pass medians '
                                'use only intact, validated query batches.'))
            if max(current['gpu_ms'], current['completion_upper_ms']) > 1000 / 120:
                investigations.append(dict(facts, wire='frame exceeds one refresh interval',
                    disposition='The capture-off pair isolates optics. Gather timestamps measure the '
                                'copy and inverse air directly. Rays and composition share the water '
                                'surface pass; the matching surface delta includes both. Completion '
                                'also includes synchronous observer polling under shared machine load.'))
            for name, value in current['passes_ms'].items():
                previous = historical['passes_ms'].get(name)
                if previous is not None and value >= previous * 1.25:
                    investigations.append(dict(facts, wire='pass grows by at least 25% from the prior view',
                        pass_name=name, prior_ms=previous, current_ms=value,
                        paired_ms=control['passes_ms'].get(name),
                        disposition='The same-view control repeats the earlier disabled profile. '
                                    'Only the water surface and the private gather gain optical work; '
                                    'common-pass variation reflects this run and shared device load.'))
    return investigations


def dive_summary(record, location):
    assert record['target'] == 'mac-metal' and record['extent'] == [2560, 1440]
    assert record['capture'] is False and record['commit']
    assert record['location'] == location and record['wind'] == 'Calm'
    summary = {}
    expected = {'above': -2.0, 'half': 0.0, 'under': 2.0}
    assert len(record['views']) == len(expected)
    for view in record['views']:
        name = view['view']
        assert name not in summary and view['depth_m'] == expected[name]
        frames = view['frames']
        assert len(frames) == 12 and all(f['gpu_ms'] is not None and f['gpu_ms'] > 0 for f in frames)
        passes = {}
        for frame in frames:
            assert any(label.endswith(': water surface') for label in frame['passes_ms'])
            assert any(label.endswith(': water gather') for label in frame['passes_ms'])
            for label, elapsed in frame['passes_ms'].items():
                passes.setdefault(label, []).append(elapsed)
        terms = {label: statistics.median(passes[label]) for label in sorted(passes)}
        summary[name] = {'samples': len(frames), 'gpu_ms': statistics.median(f['gpu_ms'] for f in frames),
                         'cpu_wall_ms': statistics.median(f['cpu_wall_ms'] for f in frames),
                         'pass_samples': {label: len(passes[label]) for label in terms},
                         'passes_ms': terms}
    return summary


def dive_investigations(summary, prior):
    investigations = []
    above = summary['above']
    prior_passes = {comparable_pass(label): (label, duration)
                    for label, duration in prior['passes_ms'].items()}
    above_passes = {comparable_pass(label): duration for label, duration in above['passes_ms'].items()}
    for view, current in summary.items():
        water = {comparable_pass(key): duration for key, duration in current['passes_ms'].items()
                 if key.endswith((': water gather', ': water surface'))}
        facts = dict(view=view, gpu_ms=current['gpu_ms'], above_gpu_ms=above['gpu_ms'],
                     prior_above_gpu_ms=prior['gpu_ms'], cpu_wall_ms=current['cpu_wall_ms'],
                     water_gather_ms=water['water gather'], water_surface_ms=water['water surface'])
        if current['gpu_ms'] > 1000 / 120:
            largest = max(current['passes_ms'], key=current['passes_ms'].get)
            investigations.append(dict(facts, wire='GPU pass sum exceeds one refresh interval',
                largest_pass=largest, largest_ms=current['passes_ms'][largest],
                disposition='The pass sum isolates named GPU work. The CPU bracket includes the '
                            'diagnostic observer waits. This capture-off record measures shared device load.'))
        for label, value in current['passes_ms'].items():
            name = comparable_pass(label)
            historical = prior_passes.get(name)
            if historical is not None and historical[1] > 0 and value >= historical[1] * 1.25:
                prior_label, previous = historical
                if label.endswith(': water gather'):
                    disposition = ('Mapped optics can gather above water. A submerged lens widens it to the '
                                   'full viewport. The matching current above arm measures the ordinary gather.')
                elif label.endswith(': water surface'):
                    disposition = ('The water surface pass includes the lens composition and underside '
                                   'shader where active. The current above arm measures the same location '
                                   'with the lens dry. Pass timestamps do not split the two draws.')
                else:
                    disposition = ('This pass has no underwater shader. Compare the current above arm to '
                                   'separate this run from the earlier record. Lower camera positions also '
                                   'change the opaque and atmospheric work presented to the renderer.')
                investigations.append(dict(facts, wire='pass grows at least 25% from the prior above view',
                    pass_name=label, prior_pass_name=prior_label, prior_ms=previous, current_ms=value,
                    paired_above_ms=above_passes.get(name), disposition=disposition))
    return investigations


def dive_wake_summary(record, commit):
    assert record['target'] == 'mac-metal' and record['extent'] == [2560, 1440]
    assert record['capture'] is False and record['commit'] == commit
    assert record['height_m'] == 5.0 and record['speed_mps'] == 6.0 and record['wind'] == 'Calm'
    assert [arm['live'] for arm in record['arms']] == [False, True]
    summary = {}
    for arm in record['arms']:
        frames = arm['frames']
        assert len(frames) == 12 and all(frame['completion_upper_ms'] > 0 for frame in frames)
        assert all(frame['underwater_admitted'] is arm['live'] for frame in frames)
        timed = [frame for frame in frames if frame['gpu_ms'] is not None]
        terms = {}
        for frame in timed:
            passes = frame['passes_ms']
            assert any(label.endswith(': water surface') for label in passes)
            assert any(label.endswith(': water lens cut') for label in passes) == arm['live']
            if arm['live']:
                assert any(label.endswith(': water gather') for label in passes)
            for label, elapsed in passes.items():
                terms.setdefault(label, []).append(elapsed)
        summary['live' if arm['live'] else 'clear'] = {
            'gpu_ms': statistics.median(frame['gpu_ms'] for frame in timed) if timed else None,
            'cpu_wall_ms': statistics.median(frame['cpu_wall_ms'] for frame in frames),
            'completion_upper_ms': statistics.median(frame['completion_upper_ms'] for frame in frames),
            'timestamped_samples': len(timed),
            'timing_stats': arm['timing_stats'],
            'passes_ms': {label: statistics.median(samples) for label, samples in terms.items()},
        }
    clear, live = summary['clear'], summary['live']
    investigations = []
    for arm, measurement in summary.items():
        if measurement['timestamped_samples'] != 12:
            investigations.append(dict(arm=arm, wire='missing pass timestamps',
                timing_stats=measurement['timing_stats'],
                disposition='The native host refused those batches. Keep the independently measured submission-to-completion bracket and leave missing GPU pass durations empty.'))
        if max(measurement['gpu_ms'] or 0.0, measurement['completion_upper_ms']) > 1000 / 120:
            largest = max(measurement['passes_ms'], key=measurement['passes_ms'].get, default=None)
            investigations.append(dict(arm=arm, wire='measured frame or completion bracket exceeds one refresh interval',
                gpu_ms=measurement['gpu_ms'], largest_pass=largest,
                largest_ms=measurement['passes_ms'].get(largest),
                completion_upper_ms=measurement['completion_upper_ms'],
                disposition='Both arms run at the same extent and craft speed. The completion bracket includes the observer waits and shared device load.'))
    for label, current in live['passes_ms'].items():
        previous = clear['passes_ms'].get(label)
        if previous is None or current >= 1.25 * previous:
            explanation = ('The live window adds the wake update, vertex cut and full-screen gather. '
                           'The dry lens keeps no composition pixel. Both arms include the ordinary surface.')
            investigations.append(dict(arm='live', wire='additional pass or at least 25% growth',
                pass_name=label, clear_ms=previous, live_ms=current, disposition=explanation))
    return dict(record=record, summary=summary, investigations=investigations)


def measure_dive(action):
    prior = None if action == 'record' else json.loads(RECORDS['under'].read_text())
    if prior is not None:
        assert prior['extent'] == [2560, 1440] and prior['target'] == 'mac-metal'
        assert prior['capture'] is False and prior['wind'] == 'Calm'
        assert set(prior['locations']) == {'lake', 'sea'}
    commit = run(['git', 'rev-parse', 'HEAD'], capture_output=True, text=True).stdout.strip()
    proof = ['cargo', 'test', '--release', '--locked', '--offline', '-p', 'water-scene',
             '--test', 'proof_gpu']
    scratch = ROOT / 'tmp/waters-wp15'
    scratch.mkdir(parents=True, exist_ok=True)
    locations = {}
    wake = None
    with tempfile.TemporaryDirectory(prefix='dive-cost-', dir=scratch) as staging:
        store = Path(staging) / 'store'
        run(['cargo', 'run', '--release', '--locked', '--offline', '-p', 'thrum-cook', '--',
             'cook', 'examples/water/assets', str(store)])
        env = dict(os.environ, THRUM_WATER_PROOF='dive-cost', THRUM_WATER_COMMIT=commit,
                   THRUM_WATER_OUTPUT=staging, THRUM_ASSET_STORE=str(store),
                   THRUM_REQUIRE_GPU='1', THRUM_WATER_DIVE='calm')
        for location in ('lake', 'sea'):
            run(proof, env=dict(env, THRUM_WATER_LOCATION=location))
            inputs = json.loads((Path(staging) / f'{location}-dive-cost.json').read_text())
            summary = dive_summary(inputs, location)
            locations[location] = {'commit': commit, 'above': summary['above']}
            if prior is not None:
                history = prior['locations'][location]
                assert history['commit'] and history['above']['samples'] == 12
                locations[location].update(views=inputs['views'], summary=summary, prior=history,
                    investigations=dive_investigations(summary, history['above']))
        if prior is not None:
            run(proof, env=dict(env, THRUM_WATER_PROOF='dive-wake-cost'))
            wake = dive_wake_summary(json.loads((Path(staging) / 'wake-dive-cost.json').read_text()), commit)
    record = dict(commit=commit, extent=[2560, 1440], target='mac-metal', capture=False, wind='Calm',
                  host=platform.platform(), load=run(['uptime'], capture_output=True, text=True).stdout.strip(),
                  locations=locations)
    if wake is not None:
        record['wake'] = wake
    destination = WORKING / 'WP15'
    destination.mkdir(parents=True, exist_ok=True)
    path = destination / f'dive-cost{"-prior" if prior is None else ""}-{commit[:12]}.json'
    path.write_text(json.dumps(record, indent=2) + '\n')
    print(f'PASS paired 1440p Metal dive cost record: {path.relative_to(ROOT)}')


def rain_yard_cost(action):
    output = ROOT / 'tmp/puddles-wp3/cost-current'
    store = output / 'store'
    output.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, THRUM_WATER_RAIN='calm', THRUM_WATER_PROOF='rain-puddles-cost',
               THRUM_WATER_OUTPUT=str(output), THRUM_ASSET_STORE=str(store), THRUM_REQUIRE_GPU='1')
    run(['cargo', 'run', '--release', '--locked', '--offline', '-p', 'thrum-cook', '--',
         'cook', 'examples/water/assets', str(store)])
    run(['cargo', 'test', '--release', '--locked', '--offline', '-p', 'water-scene',
         '--test', 'proof_gpu'], env=env)
    data = json.loads((output / 'rain-puddles-cost.json').read_text())
    data['commit'] = run(['git', 'rev-parse', 'HEAD'], capture_output=True, text=True).stdout.strip()
    data['load'] = run(['uptime'], capture_output=True, text=True).stdout.strip()
    data['host'] = platform.platform()
    baseline = json.loads(RECORDS['island'].read_text())
    data['island_record'] = {'path': str(RECORDS['island'].relative_to(ROOT)),
                             'commit': baseline['commit'], 'summary': baseline['summary']}
    prior = {(row['water'], row['tick_frame']): row for row in baseline['summary']}
    for fill in data['fills']:
        bare, water = fill['arms']
        fill['yard_pixels'] = water['reactive_pixels'] - bare['reactive_pixels']
        fill['summary'] = []
        for arm in fill['arms']:
            for tick in (False, True):
                frames = [row for row in arm['frames'] if row['tick_frame'] == tick]
                if not frames or any(row.get('gpu_ms') is None for row in frames):
                    raise ValueError(f"missing GPU timings in {fill['name']} at tick={tick}")
                old = prior[arm['water'], tick]
                passes = {key: statistics.median(row['passes_ms'][key] for row in frames
                                                  if key in row['passes_ms'])
                          for key in sorted(set().union(*(row['passes_ms'] for row in frames)))}
                gpu_ms = statistics.median(row['gpu_ms'] for row in frames)
                upper = statistics.median(row['completion_upper_ms'] for row in frames)
                largest = max(passes, key=passes.get)
                investigation = []
                if max(gpu_ms, upper) > 1000 / 120:
                    investigation.append(
                        f"Pass sum {gpu_ms:.3f} ms, largest pass {largest} "
                        f"{passes[largest]:.3f} ms. Synchronous completion bracket {upper:.3f} ms "
                        "includes polling and shared-device wait.")
                if gpu_ms > old['gpu_ms'] * 1.25:
                    investigation.append(
                        f"Island {old['gpu_ms']:.3f} ms vs yard {gpu_ms:.3f} ms. "
                        f"Yard draws {water['vertices']} vertices and {fill['yard_pixels']} water "
                        "pixels. Different cameras and shared load limit attribution.")
                fill['summary'].append({'water': arm['water'], 'tick_frame': tick, 'gpu_ms': gpu_ms,
                                        'completion_upper_ms': upper, 'passes_ms': passes,
                                        'investigations': investigation})
    validate_rain_yard(data)
    if action == 'record':
        path = ROOT / '.orchestrate/evidence/PUDDLES/WP3/yard-water-cost.json'
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(data, indent=2) + '\n')
    else:
        path = RECORDS['rain-puddles']
        validate_rain_yard(json.loads(path.read_text()))
    print(f'PASS paired rain yard record: {path.relative_to(ROOT)}')


def validate_rain_yard(data):
    assert data['target'] == 'mac-metal' and data['extent'] == [2560, 1440] and data['capture'] is False
    assert data['commit'] and data['load'] and data['island_record']['summary']
    assert [fill['name'] for fill in data['fills']] == ['first', 'separate', 'brim']
    assert data['dry_vertices'] == [0, 0]
    counts = [fill['arms'][1]['vertices'] for fill in data['fills']]
    assert 0 < counts[0] < counts[1] < counts[2] < 128_000
    for fill in data['fills']:
        bare, water = fill['arms']
        assert not bare['water'] and water['water'] and fill['yard_pixels'] > 0
        assert not bare['vertices'] and not bare['hollows_bytes']
        assert water['vertices'] > 0 and water['hollows_bytes'] > 0 and water['levels'] == bare['levels']
        assert len(fill['summary']) == 4
        assert all(row['gpu_ms'] > 0 and row['passes_ms'] for row in fill['summary'])


def rain_rings_cost(action):
    output = ROOT / 'tmp/puddles-wp4/cost'
    output.mkdir(parents=True, exist_ok=True)
    store = output / 'store'
    env = dict(os.environ, THRUM_WATER_PROOF='rain-rings-cost',
               THRUM_WATER_OUTPUT=str(output), THRUM_ASSET_STORE=str(store), THRUM_REQUIRE_GPU='1')
    run(['cargo', 'run', '--release', '--locked', '--offline', '-p', 'thrum-cook', '--',
         'cook', 'examples/water/assets', str(store)])
    run(['cargo', 'test', '--release', '--locked', '--offline', '-p', 'water-scene',
         '--test', 'proof_gpu'], env=env)
    data = json.loads((output / 'rain-rings-cost.json').read_text())
    prior_path = RECORDS['rain-puddles']
    prior = json.loads(prior_path.read_text())
    data['commit'] = run(['git', 'rev-parse', 'HEAD'], capture_output=True, text=True).stdout.strip()
    data['load'] = run(['uptime'], capture_output=True, text=True).stdout.strip()
    data['host'] = platform.platform()
    data['wp3'] = {'path': str(prior_path.relative_to(ROOT)), 'commit': prior['commit'],
                   'brim': prior['fills'][-1]['summary']}
    controls = {}
    for arm in data['arms']:
        for view in arm['views']:
            summary = []
            for tick in (False, True):
                frames = [row for row in view['frames'] if row['tick_frame'] == tick]
                assert frames and all(row['gpu_ms'] is not None for row in frames)
                passes = {name.split(': ')[-1]: statistics.median(row['passes_ms'][name] for row in frames
                                                                  if name in row['passes_ms'])
                          for name in sorted(set().union(*(row['passes_ms'] for row in frames)))}
                gpu = statistics.median(row['gpu_ms'] for row in frames)
                upper = statistics.median(row['completion_upper_ms'] for row in frames)
                upload = statistics.median(row['upload_cpu_ms'] for row in frames)
                key = (view['name'], tick)
                investigations = []
                if max(gpu, upper) > 1000 / 120:
                    largest = max(passes, key=passes.get)
                    investigations.append(f'Pass sum {gpu:.3f} ms. Largest pass {largest} '
                                          f'{passes[largest]:.3f} ms. Synchronous completion '
                                          f'{upper:.3f} ms includes polling and shared GPU wait.')
                if arm['rings']:
                    control = controls[key]
                    rain_ms = sum(value for name, value in passes.items() if name.startswith('rain '))
                    investigations.append(f'Same-view rain dispatch and mips total {rain_ms:.3f} ms. '
                                          f'CPU upload was {upload:.3f} ms, versus '
                                          f'{control["upload_cpu_ms"]:.3f} ms with bands disabled.')
                    for name, previous in control['passes_ms'].items():
                        if passes.get(name, 0) > 1.25 * previous:
                            context = ('the same water surface now reads two sets of slopes and moments'
                                       if name == 'water surface' else
                                       'the pass has the same inputs and commands in both arms, '
                                       'so the observed difference includes shared device load')
                            investigations.append(f'{name} rose from {previous:.4f} to '
                                                  f'{passes[name]:.4f} ms; {context}.')
                row = {'tick_frame': tick, 'gpu_ms': gpu, 'completion_upper_ms': upper,
                       'upload_cpu_ms': upload, 'passes_ms': passes, 'investigations': investigations}
                if arm['water'] and not arm['rings']:
                    controls[key] = row
                summary.append(row)
            view['summary'] = summary
    for row, investigations in rain_brim_investigations(data):
        row['investigations'].extend(investigations)
    summarize_fleet(data['fleet'])
    validate_rain_rings(data)
    evidence = ROOT / '.orchestrate/evidence/PUDDLES/WP4/rain-rings-cost.json'
    evidence.parent.mkdir(parents=True, exist_ok=True)
    evidence.write_text(json.dumps(data, indent=2) + '\n')
    if action == 'check':
        validate_rain_rings(json.loads(RECORDS['rain-rings'].read_text()))
    print(f'PASS paired Metal rain record: {evidence.relative_to(ROOT)}')


def rain_brim_investigations(data):
    """Compare like-sized yard views at matching frame kinds and water settings."""
    prior = {(row['water'], row['tick_frame']): row for row in data['wp3']['brim']}
    yard = {(arm['water'], arm['rings']): arm['views'][0]['summary'] for arm in data['arms']}
    for (water, rings), rows in yard.items():
        for row, control in zip(rows, yard[water, False], strict=True):
            old = prior[water, row['tick_frame']]
            investigations = []
            for name, previous in old['passes_ms'].items():
                current = row['passes_ms'].get(name, 0)
                if previous <= 0 or current < 1.25 * previous:
                    continue
                dry_band_ms = control['passes_ms'][name]
                if dry_band_ms >= 1.25 * previous:
                    attribution = ('the increase is present with bands disabled, so the band '
                                   'dispatch cannot account for it; compare matched captures at '
                                   'similar shared-device load if the increase persists')
                elif name == 'water surface':
                    attribution = ('the increase appears with bands enabled, where surface '
                                   'sampling adds ring slopes and moments; isolate that sampling '
                                   'under matched load if the increase persists')
                else:
                    attribution = ('the increase appears with bands enabled although this pass '
                                   'does not sample rain bands; repeat at matched shared-device '
                                   'load to distinguish GPU scheduling from a pass regression')
                investigations.append(
                    f'WP3 yard brim {name} rose from {previous:.4f} to {current:.4f} ms '
                    f'(current bands-off control {dry_band_ms:.4f} ms); {attribution}.')
            yield row, investigations


def summarize_fleet(fleet):
    dry, wet = fleet['arms']
    for arm in (dry, wet):
        times = arm['drain_ns']
        arm['median_ns'] = statistics.median(times)
        arm['max_ns'] = max(times)
        arm['investigations'] = []
    for tick, elapsed in enumerate(wet['drain_ns']):
        if elapsed <= fleet['step_ns'] / 10:
            continue
        wet['investigations'].append(
            f'At tick {tick + 2} the complete ready-answer drain took {elapsed / 1e6:.3f} ms; '
            f'dry at the same tick took {dry["drain_ns"][tick] / 1e6:.3f} ms. '
            f'The wet median was {wet["median_ns"] / 1e6:.3f} ms and dry median '
            f'{dry["median_ns"] / 1e6:.3f} ms. The interval excludes drawing and GPU readback; '
            'in rain it includes schedule listing, binning, and lookup of the 54 footprints.')


def validate_rain_rings(data):
    assert data['target'] == 'mac-metal' and data['extent'] == [2560, 1440]
    assert data['capture'] is False and data['rate_mm_h'] == 50
    assert data['commit'] and data['load'] and data['wp3']['brim']
    assert [(arm['water'], arm['rings']) for arm in data['arms']] == [
        (False, False), (True, False), (True, True)]
    assert len({tuple(arm['fills'][-1]['levels']) for arm in data['arms']}) == 1
    for arm in data['arms']:
        assert len(arm['fills']) >= 3
        assert any(row['label_ms'] > 0 for row in arm['fills']) == arm['water']
        assert [view['name'] for view in arm['views']] == ['yard', 'lake']
        assert bool(arm['resources']) == arm['water']
        for view in arm['views']:
            assert [row['tick_frame'] for row in view['summary']] == [False, True]
            for row in view['summary']:
                passes = row['passes_ms']
                assert row['gpu_ms'] > 0 and passes
                assert ('water surface' in passes) == arm['water']
                assert ('rain bands' in passes) == arm['rings']
                if arm['rings'] or max(row['gpu_ms'], row['completion_upper_ms']) > 1000 / 120:
                    assert row['investigations']
    fleet = data['fleet']
    assert fleet['stations'] == 54 and fleet['step_ns'] == 16_666_667
    assert fleet['footprint_prepare_ms'] > 0
    dry, wet = fleet['arms']
    assert [dry['rate_mm_h'], wet['rate_mm_h']] == [0, 50]
    assert len(dry['drain_ns']) == len(wet['drain_ns']) >= 60
    for arm in (dry, wet):
        assert min(arm['drain_ns']) > 0
        assert arm['median_ns'] == statistics.median(arm['drain_ns'])
        assert arm['max_ns'] == max(arm['drain_ns'])
    crossings = sum(elapsed > fleet['step_ns'] / 10 for elapsed in wet['drain_ns'])
    assert len(wet['investigations']) == crossings
    for row, investigations in rain_brim_investigations(data):
        assert all(entry in row['investigations'] for entry in investigations)


def compare_footfall_cost(row, control, comparison, fine=False):
    passes = row['passes_ms']
    investigations = row['investigations']
    row.setdefault('comparisons', {})[comparison] = {
        'gpu_ms_delta': row['gpu_ms'] - control['gpu_ms'],
        'completion_upper_ms_delta': row['completion_upper_ms'] - control['completion_upper_ms'],
        'added_passes_ms': {name: duration for name, duration in passes.items()
                            if name not in control['passes_ms']},
    }
    for name, old in control['passes_ms'].items():
        new = passes.get(name, 0)
        if old > 0 and new >= old * 1.25:
            context = ({
                'interaction waves': 'the same pass adds the fine region/depth solves listed above',
                'water surface': 'the surface samples two extra endpoint planes inside the fine window',
            }.get(name, 'the pass does not use fine state and runs under shared load') if fine else
                'the paired arm measures the same opaque yard and walking marker under shared load')
            investigations.append(f'{comparison}: {name} rose from {old:.4f} to {new:.4f} ms. {context}.')
    if fine and row['tick_frame'] and row['labelling_cpu_ms'] > max(0.05, control['labelling_cpu_ms'] * 1.25):
        investigations.append(f'{comparison}: labelling {row["labelling_cpu_ms"]:.3f} ms vs '
                              f'{control["labelling_cpu_ms"]:.3f} ms adds a 256² window.')


def rain_footfall_cost():
    output = ROOT / 'tmp/puddles-wp8/cost'
    output.mkdir(parents=True, exist_ok=True)
    store = output / 'store'
    env = dict(os.environ, THRUM_WATER_PROOF='rain-footfall-cost',
               THRUM_WATER_OUTPUT=str(output), THRUM_ASSET_STORE=str(store), THRUM_REQUIRE_GPU='1')
    run(['cargo', 'run', '--release', '--locked', '--offline', '-p', 'thrum-cook', '--',
         'cook', 'examples/water/assets', str(store)])
    run(['cargo', 'test', '--release', '--locked', '--offline', '-p', 'water-scene',
         '--test', 'proof_gpu'], env=env)
    data = json.loads((output / 'rain-footfall-cost.json').read_text())
    data['commit'] = run(['git', 'rev-parse', 'HEAD'], capture_output=True, text=True).stdout.strip()
    data['load'] = run(['uptime'], capture_output=True, text=True).stdout.strip()
    data['host'] = platform.platform()
    wp4_file = RECORDS['rain-rings']
    wp4 = json.loads(wp4_file.read_text())
    data['wp4'] = {'path': str(wp4_file.relative_to(ROOT)), 'commit': wp4['commit'],
                   'rate_mm_h': wp4['rate_mm_h'], 'yard': wp4['arms'][-1]['views'][0]['summary'],
                   'comparison': 'WP4 used 50 mm/h and a stationary overhead camera. The matched '
                                 'WP8 arms use 10 mm/h and the moving marker at eye height.'}
    assert data['target'] == 'mac-metal' and data['extent'] == [2560, 1440]
    assert data['capture'] is False and data['rate_mm_h'] == 10
    arms = {(arm['water'], arm['fine']): arm for arm in data['arms']}
    assert list(arms) == [(False, False), (True, False), (True, True)]
    assert len(arms) == len(data['arms'])
    schedules = [[(frame['elapsed_s'], frame['tick_frame']) for frame in arm['frames']]
                 for arm in arms.values()]
    assert all(schedule == schedules[0] for schedule in schedules)
    for arm in arms.values():
        arm['summary'] = []
        for tick_frame in (False, True):
            frames = [frame for frame in arm['frames'] if frame['tick_frame'] == tick_frame]
            assert len(frames) == 24 and all(frame['gpu_ms'] > 0 for frame in frames)
            assert all((frame['fine'] is not None) == arm['fine'] for frame in frames)
            for frame in frames:
                labels = {name.split(': ')[-1] for name in frame['passes_ms']}
                for label in ('rain bands', 'water gather', 'water surface'):
                    assert (label in labels) == arm['water']
            names = set().union(*(frame['passes_ms'] for frame in frames))
            passes = {name.split(': ')[-1]: statistics.median(
                frame['passes_ms'][name] for frame in frames if name in frame['passes_ms'])
                for name in sorted(names)}
            row = {key: statistics.median(frame[key] for frame in frames) for key in
                   ('gpu_ms', 'completion_upper_ms', 'upload_cpu_ms', 'labelling_cpu_ms')}
            row.update(tick_frame=tick_frame, passes_ms=passes, investigations=[])
            if arm['fine'] and tick_frame:
                assert any(frame['fine']['regions'] > 0 and frame['fine']['solves'] > 0 for frame in frames)
                assert all(frame['fine']['steps'] <= 2 for frame in frames)
                row['fine_regions'] = sorted({frame['fine']['regions'] for frame in frames})
                row['fine_solves'] = sorted({frame['fine']['solves'] for frame in frames})
                row['fine_steps'] = sorted({frame['fine']['steps'] for frame in frames})
            investigations = row['investigations']
            if max(row['gpu_ms'], row['completion_upper_ms']) > 1000 / 120:
                largest = max(passes, key=passes.get)
                investigations.append(
                    f'Largest pass {largest} was {passes[largest]:.3f} ms. The GPU pass sum was '
                    f'{row["gpu_ms"]:.3f} ms and synchronous completion was '
                    f'{row["completion_upper_ms"]:.3f} ms, including polling and shared-device wait.')
            arm['summary'].append(row)
    without_water, base_levels, fine_level = arms.values()
    for candidate, control, comparison in [
        (base_levels, without_water, 'water without the fine level versus water absent'),
        (fine_level, base_levels, 'fine level enabled versus disabled'),
        (fine_level, without_water, 'water with the fine level versus water absent'),
    ]:
        for row, baseline in zip(candidate['summary'], control['summary'], strict=True):
            compare_footfall_cost(row, baseline, comparison, fine=(control is base_levels))
    evidence = RECORDS['rain-footfall']
    evidence.parent.mkdir(parents=True, exist_ok=True)
    evidence.write_text(json.dumps(data, indent=2) + '\n')
    print(f'PASS paired Metal footfall record: {evidence.relative_to(ROOT)}')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['record', 'check', 'running'])
    parser.add_argument('--scene', choices=list(RECORDS), required=True)
    parser.add_argument('--arm', choices=['open-water'])
    args = parser.parse_args()
    if args.arm and args.action != 'running':
        parser.error('--arm is only available for a running proof')
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
    if args.scene == 'optics':
        measure_optics(args.action)
        return
    if args.scene == 'under':
        measure_dive(args.action)
        return
    if args.scene == 'surf':
        measure_surf()
        return
    if args.scene == 'rain-footfall':
        rain_footfall_cost()
        return
    if args.scene == 'rain-rings':
        rain_rings_cost(args.action)
        return
    if args.scene == 'rain-puddles':
        rain_yard_cost(args.action)
        return
    record = RECORDS[args.scene]
    data = measure(args.scene)
    validate(data, args.scene)
    if args.scene in ('channel', 'river', 'river-calm'):
        record = Path(os.environ.get('THRUM_WATER_OUTPUT', record))
        if record.is_dir():
            record /= f'{args.scene}-cost.json'
        record.parent.mkdir(parents=True, exist_ok=True)
        record.write_text(json.dumps(data, indent=2) + '\n')
    elif args.action == 'record':
        record = WORKING / f'{args.scene}-cost.json'
        record.parent.mkdir(parents=True, exist_ok=True)
        record.write_text(json.dumps(data, indent=2) + '\n')
    else:
        validate(json.loads(record.read_text()), args.scene)
    print(f'PASS paired {args.scene} record: {record.relative_to(ROOT)}')


if __name__ == '__main__':
    main()
