"""Capture-off Metal timing for the populated V1 valley at the showcase extent."""
from collections import defaultdict
import json
import os
from pathlib import Path
import shutil
from statistics import median
import subprocess
import sys
from tempfile import TemporaryDirectory

ROOT = Path(__file__).resolve().parent.parent
FIXTURE = ROOT / 'tests/fixtures/puddles/valley-cost.json'
EVIDENCE = ROOT / '.orchestrate/evidence/PUDDLES/WP6/valley-cost.json'
EXTENT = [1920, 1080]
REFRESH = 120


def run(command, **kwargs):
    return subprocess.run(command, cwd=ROOT, check=True, **kwargs)


def summarise(trace, tick):
    frames = []
    previous = None
    for frame in trace['gpu']:
        index = frame['tick']
        if previous is not None and index > 0 and (index != previous) == tick:
            if frame['timed_pass_span_ns'] is not None and frame['gpu_pass_ns']:
                frames.append(frame)
        previous = index
    if not frames:
        raise ValueError(f'no timed {"tick" if tick else "repeat"} frames')
    passes = defaultdict(list)
    for frame in frames:
        for name, elapsed in frame['gpu_pass_ns'].items():
            if elapsed is not None:
                passes[name].append(elapsed / 1e6)
    return {
        'tick': tick,
        'samples': len(frames),
        'gpu_ms': median(row['timed_pass_span_ns'] for row in frames) / 1e6,
        'wall_ms': median(row['wall_ns'] for row in frames) / 1e6,
        'tick_cpu_ms': median(row['tick_cpu_ns'] for row in frames) / 1e6,
        'passes_ms': {name: median(values) for name, values in passes.items()},
    }


def investigations(arms):
    results = []
    for dry, wet in zip(arms['off']['timings'], arms['soaked']['timings']):
        for label, record in [('off', dry), ('soaked', wet)]:
            if record['gpu_ms'] > 1000 / REFRESH:
                dominant = sorted(record['passes_ms'].items(), key=lambda entry: -entry[1])[:4]
                water = sum(cost for name, cost in record['passes_ms'].items() if
                            any(word in name for word in ('water', 'shore film', 'rain')))
                results.append({
                    'wire': 'GPU span exceeds a 120 Hz interval',
                    'selection': label,
                    'tick': record['tick'],
                    'measurement': {'gpu_ms': record['gpu_ms'], 'dominant_passes_ms': dominant,
                                    'rain_and_water_pass_sum_ms': water, 'paired_gpu_ms':
                                    (dry if label == 'soaked' else wet)['gpu_ms']},
                    'finding': 'The paired span and named passes account for shared scene work and the extra effect. Shared machine load is recorded alongside each arm.',
                })
        for name, old in dry['passes_ms'].items():
            new = wet['passes_ms'].get(name)
            if old > 0 and new is not None and new >= old * 1.25:
                results.append({
                    'wire': 'paired pass at least 25% slower',
                    'selection': 'soaked',
                    'tick': wet['tick'],
                    'measurement': {'pass': name, 'off_ms': old, 'soaked_ms': new,
                                    'delta_ms': new - old},
                    'finding': ('The forward pass evaluates wetness on each cooked and entity fragment and draws the added ground.'
                                if 'forward' in name else
                                'This pass shares its algorithm with the dry arm. The paired timestamps bound its absolute contribution under the recorded load.'),
                })
    return results


def measure_valley():
    run(['cargo', 'build', '--release', '--locked', '--offline', '-p', 'v1-game', '-p', 'thrum-cook'])
    scratch = ROOT / 'tmp/puddles-wp6'
    scratch.mkdir(parents=True, exist_ok=True)
    with TemporaryDirectory(prefix='cost-', dir=scratch) as temporary:
        work = Path(temporary)
        source = work / 'source'
        source.mkdir()
        for folder in ('showcase', 'exit', 'rain'):
            for asset in (ROOT / 'examples/v1-game/assets' / folder).iterdir():
                if asset.suffix in ('.bin', '.gltf', '.png'):
                    shutil.copyfile(asset, source / asset.name)
        store = work / 'store'
        run(['target/release/thrum-cook', 'cook', str(source), str(store)])
        arms = {}
        for arm in ('off', 'soaked'):
            target = work / arm
            env = dict(os.environ, THRUM_RAIN=arm, THRUM_RAIN_VIEW='patch', THRUM_CULLING='frustum',
                       THRUM_GOVERNOR='off')
            run([sys.executable, 'scripts/capture_v1_runtime.py',
                 '--binary', 'target/release/v1-game', '--store', str(store), '--scene', 'exit',
                 '--case', 'rain-cost', '--mode', 'realtime', '--observer-only',
                 '--recipe', 'portable', '--reconstruction', 'taau', '--extent', *map(str, EXTENT),
                 '--output', str(target)], env=env)
            trace = json.loads((target / 'runtime.json').read_text())
            manifest = json.loads((target / 'manifest.json').read_text())
            if not trace['complete'] or not trace['observer_only'] or trace['backend'] != 'Metal':
                raise ValueError(f'{arm}: missing complete Metal observer arm')
            if trace['extent'] != EXTENT:
                raise ValueError(f'{arm}: wrong showcase extent')
            rain = trace.get('rain')
            if arm == 'soaked':
                if not rain or not rain['submitted_hollow_vertices'] or not rain['resources']:
                    raise ValueError('soaked arm did not submit a nonempty water surface')
            elif rain:
                raise ValueError('rain was prepared in the off arm')
            arms[arm] = {
                'adapter': trace['adapter'], 'recipe': trace['recipe'],
                'timings': [summarise(trace, tick) for tick in (True, False)],
                'rain': rain, 'system_load': manifest['system_load'],
                'source_revision': manifest['source_revision'],
                'build_source_digest': manifest['build_source_digest'],
            }
        return {'extent': EXTENT, 'refresh_hz': REFRESH, 'arms': arms,
                'investigations': investigations(arms),
                'scope': 'Metal offscreen submissions, no capture tap, shared machine load'}


def valley(action):
    measured = measure_valley()
    if action == 'record':
        EVIDENCE.parent.mkdir(parents=True, exist_ok=True)
        EVIDENCE.write_text(json.dumps(measured, indent=2) + '\n', encoding='utf-8')
    else:
        committed = json.loads(FIXTURE.read_text(encoding='utf-8'))
        if committed['extent'] != EXTENT or committed['refresh_hz'] != REFRESH:
            raise ValueError('incomplete valley extent or refresh record')
        for arm in ('off', 'soaked'):
            for value in ('adapter', 'recipe', 'timings', 'rain', 'system_load', 'source_revision', 'build_source_digest'):
                if value not in committed['arms'][arm]:
                    raise ValueError(f'committed valley record is missing {arm}.{value}')
            if {row['tick'] for row in committed['arms'][arm]['timings']} != {True, False}:
                raise ValueError(f'committed valley record does not distinguish {arm} tick and repeat frames')
        recorded = {(row['wire'], row['selection'], row['tick'], row['measurement'].get('pass'))
                    for row in committed['investigations'] if row.get('finding')}
        required = {(row['wire'], row['selection'], row['tick'], row['measurement'].get('pass'))
                    for row in investigations(committed['arms'])}
        if required - recorded:
            raise ValueError('committed valley tripwire is missing its investigation')
    for arm, record in measured['arms'].items():
        for row in record['timings']:
            print(f'{arm}, {"tick" if row["tick"] else "repeat"}: '
                  f'{row["samples"]} samples, median GPU span {row["gpu_ms"]:.3f} ms')
