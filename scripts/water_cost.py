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
RECORD = ROOT / 'docs/evidence/WATERS/M2-cost.json'


def run(command, **kwargs):
    return subprocess.run(command, cwd=ROOT, check=True, **kwargs)


def measure():
    output = ROOT / 'tmp/waters-M2/cost-current'
    output.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, THRUM_WATER_PROOF='cost', THRUM_WATER_OUTPUT=str(output))
    env.pop('THRUM_ASSET_STORE', None)
    run(['cargo', 'test', '--release', '--locked', '--offline', '-p', 'water-scene', '--test', 'proof_gpu'], env=env)
    data = json.loads((output / 'cost.json').read_text())
    data['commit'] = run(['git', 'rev-parse', 'HEAD'], capture_output=True, text=True).stdout.strip()
    data['host'] = platform.platform()
    data['load'] = run(['uptime'], capture_output=True, text=True).stdout.strip()
    data['build'] = 'release; concurrent machine load, not qualified cadence'
    data['refresh_hz'] = 120
    data['view'] = 'camera at 2 m, bearing 90 degrees, noon; optional calibration float omitted in both arms'
    data['estimates'] = {'surface_vertices': 200000, 'surface_gpu_ms': 'unmeasured in design; largest term', 'broadsides_surface_ms': 10.5}
    data['summary'] = []
    for arm in data['arms']:
        for tick in [True, False]:
            frames = [f for f in arm['frames'] if f['tick_frame'] == tick]
            if not frames:
                raise ValueError('missing tick or repeat frames')
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
    return data


def validate(data):
    assert data['target'] == 'mac-metal' and data['extent'] == [2560, 1440]
    assert data['capture'] is False and data['vertices'] > 0
    assert data['resources'] and data['load'] and data['commit']
    assert {arm['water'] for arm in data['arms']} == {True, False}
    for arm in data['arms']:
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


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['record', 'check'])
    parser.add_argument('--scene', choices=['first-frame'], required=True)
    args = parser.parse_args()
    data = measure()
    validate(data)
    if args.action == 'record':
        RECORD.parent.mkdir(parents=True, exist_ok=True)
        RECORD.write_text(json.dumps(data, indent=2) + '\n')
    else:
        validate(json.loads(RECORD.read_text()))
    print(f'PASS paired first-frame record: {RECORD.relative_to(ROOT)}; fresh sample at tmp/waters-M2/cost-current/cost.json')


if __name__ == '__main__':
    main()
