#!/usr/bin/env python3
"""Paired water cost record. Timing numbers are tripwires, never pass thresholds."""
import argparse
import json
import os
from pathlib import Path
import platform
import statistics
import subprocess

ROOT = Path(__file__).resolve().parents[1]
RECORDS = {'first-frame': ROOT / 'docs/evidence/WATERS/M2-cost.json',
           'island': ROOT / 'docs/evidence/WATERS/M4-cost.json',
           'wake': ROOT / '.orchestrate/evidence/WATERS/M6/cost.json'}


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
        assert data['first_frame_record'] == str(RECORDS['first-frame'].relative_to(ROOT))
        assert set(data['body_pixels']) == {'Sea', 'Lake', 'Pond'}
        assert all(count > 0 for count in data['body_pixels'].values())
        assert len(data['body_vertices']) == 3 and sum(n for _, n in data['body_vertices']) == data['vertices']
        assert all(n > 0 for _, n in data['body_vertices'])


def measure_wake():
    """Compare the same two views, once with the producer and once without it."""
    output = ROOT / 'tmp/waters-M6/cost'
    output.mkdir(parents=True, exist_ok=True)
    store = output / 'store'
    run(['cargo', 'run', '--release', '--locked', '--offline', '-p', 'thrum-cook', '--',
         'cook', 'examples/water/assets', str(store)])
    env = dict(os.environ, THRUM_WATER_PROOF='cost', THRUM_ASSET_STORE=str(store),
               THRUM_WATER_WAKE='trade', THRUM_WATER_VIEW='overhead', THRUM_WATER_SPEED='6')
    env['THRUM_WATER_COMMIT'] = run(['git', 'rev-parse', 'HEAD'], capture_output=True, text=True).stdout.strip()
    env.pop('THRUM_WATER_ISLAND', None)
    env.pop('THRUM_WATER_STONE', None)
    result = {'target': 'mac-metal', 'extent': [2560, 1440], 'capture': False,
              'commit': env['THRUM_WATER_COMMIT'],
              'host': platform.platform(), 'load': run(['uptime'], capture_output=True, text=True).stdout.strip(),
              'views': {}}
    for location in ('sea', 'lake'):
        view = output / location
        env.update(THRUM_WATER_OUTPUT=str(view), THRUM_WATER_LOCATION=location)
        run(['cargo', 'test', '--release', '--locked', '--offline', '-p', 'water-scene',
             '--test', 'proof_gpu'], env=env)
        record = json.loads((view / 'cost.json').read_text())
        assert record['target'] == 'mac-metal' and record['extent'] == [2560, 1440]
        assert record['capture'] is False and record['commit'] == result['commit']
        active, baseline = record['arms']
        assert active['water'] and not baseline['water']
        assert len(active['frames']) == len(baseline['frames'])
        comparisons = []
        timing_faults = 0
        for current, old in zip(active['frames'], baseline['frames'], strict=True):
            assert current['tick'] == old['tick'] and current['alpha'] == old['alpha']
            assert current['interaction']['endpoints']
            assert not any('interaction waves' in label for label in old['passes_ms'])
            solves = current['interaction']['solves']
            assert all(0 <= n <= 16 for n in solves) and sum(solves) > 0
            available = current['gpu_ms'] is not None and old['gpu_ms'] is not None
            if available and current['tick_frame']:
                assert any('interaction waves' in label for label in current['passes_ms'])
            if not available:
                timing_faults += 1
            metric = 'gpu_ms' if available else 'completion_upper_ms'
            scene_ms, old_ms = current[metric], old[metric]
            assert scene_ms > 0 and old_ms > 0
            if scene_ms > 1000 / 120 or scene_ms > old_ms * 1.25:
                wake_ms = sum(value for label, value in current['passes_ms'].items() if 'interaction waves' in label) if available else None
                comparisons.append({
                    'tick': current['tick'], 'metric': metric, 'candidate_ms': scene_ms, 'paired_ms': old_ms,
                    'wake_ms': wake_ms, 'solves': solves,
                    'other_pass_delta_ms': scene_ms - wake_ms - old_ms if available else None,
                    'labelling_cpu_ms': current['interaction']['labelling_cpu_ms'],
                    'investigation': ('The timestamped interaction pass isolates the transforms. The remainder includes surface sampling; compare with repeat frames, where the transforms skip.'
                                      if available else 'The renderer rejected at least one timestamp batch. The paired completion bracket includes observer waits and CPU submission, so the transform contribution cannot be isolated from it. The CPU labelling measurement is independent.'),
                })
        result['views'][location] = {'arms': record['arms'], 'resources': record['resources'],
                                    'timestamp_fault_pairs': timing_faults,
                                    'tripwire_investigations': comparisons}
    path = RECORDS['wake']
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(result, indent=2) + '\n')
    print(f'PASS paired 1440p Metal wake records: {path.relative_to(ROOT)}')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['record', 'check'])
    parser.add_argument('--scene', choices=list(RECORDS), required=True)
    args = parser.parse_args()
    if args.scene == 'wake':
        measure_wake()
        return
    record = RECORDS[args.scene]
    data = measure(args.scene)
    validate(data, args.scene)
    if args.action == 'record':
        record.write_text(json.dumps(data, indent=2) + '\n')
    else:
        validate(json.loads(record.read_text()), args.scene)
    print(f'PASS paired {args.scene} record: {record.relative_to(ROOT)}')


if __name__ == '__main__':
    main()
