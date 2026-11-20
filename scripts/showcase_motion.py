"""Collect and check the six SHOWCASE real-time tripwire arms."""

import hashlib
import json
import math
from pathlib import Path
import re
import statistics
import subprocess


ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'tests/fixtures/showcase'
RECORD = FIXTURES / 'motion-record.json'
DIAGNOSTICS = FIXTURES / 'motion-diagnostics.json'
INPUTS = ('crates/', 'examples/v1-game/', 'scripts/', '.cargo/',
          'Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml')
COMPARISONS = {'mac-route': 'elapsed-exit-route', 'mac-held': 'elapsed-exit-drift',
               'chrome-route': 'current-chrome'}


def load(path):
    return json.loads(path.read_text())


def input_digest():
    """Hash tracked paths and their current bytes, including the capture drivers."""
    names = subprocess.check_output(['git', 'ls-files', '-z', '--', *INPUTS], cwd=ROOT)
    digest = hashlib.sha256()
    for name in sorted(set(names.split(b'\0')) - {b''}):
        digest.update(name + b'\0')
        with (ROOT / name.decode()).open('rb') as source:
            digest.update(hashlib.file_digest(source, 'sha256').digest())
    return digest.hexdigest()


def intervals(timeline):
    rows = [[after[0] - before[0], before[0], before[1], after[1]]
            for before, after in zip(timeline, timeline[1:])]
    return statistics.median(row[0] for row in rows), sorted(rows, reverse=True)[:10]


def world_progress(events):
    tick = -1
    end_ms = None
    for elapsed_ms, current in events:
        if current > tick:
            tick, end_ms = current, elapsed_ms
    return {'last_advanced_ms': end_ms, 'last_tick': tick}


def mac_arm(runtime, manifest, capture):
    opportunities = [row for row in runtime['opportunities'] if 'returned_ns' in row]
    if capture:
        timeline = [[row['submit_ns'] / 1e6, row['tick'], row['alpha']]
                    for row in runtime['images']]
    else:
        timeline = [[row['returned_ns'] / 1e6, row['tick']]
                    for row in opportunities if row['submitted']]
    world = world_progress((row['returned_ns'] / 1e6, row['tick']) for row in opportunities)
    world['released_ms'] = next(row['accepted_ns'] / 1e6 for row in runtime['control_actions']
                                if row['kind'] == 'world-pause' and not row['paused'])
    system_load = manifest['system_load']
    return {
        'complete': manifest.get('capture_complete', False) if capture else runtime['complete'],
        'load_before': system_load['before_load_1_5_15'], 'load_after': system_load['after_load_1_5_15'],
        'clock': 'Metal host submission' if capture else 'Metal host return after submission',
        'target': runtime['backend'], 'adapter': runtime['adapter'], 'recipe': runtime['recipe'],
        'timeline': timeline, 'world': world, 'extent': runtime['extent'],
    }


def chrome_arm(runtime, manifest, capture):
    opportunities = runtime['opportunities']
    start_ms = runtime['elapsed_origin_ms']
    if capture:
        timeline = [[row['submit_ms'], row['source_tick'], row['alpha']]
                    for row in manifest['records'] if row is not None]
    else:
        presented = runtime['initial_report']['presented']
        timeline = []
        for row in opportunities:
            if row['presented'] > presented:
                timeline.append([row['returned_ms'] - start_ms, row['tick']])
                presented = row['presented']
        if presented != runtime['final_report']['presented']:
            raise ValueError('the Chrome capture-off submission count disagrees with the runtime')
    world = world_progress((row['returned_ms'] - start_ms, row['tick']) for row in opportunities)
    return {
        'complete': manifest['complete'] if capture else manifest['observer_only'] and runtime['complete'],
        'load_before': manifest['load_before'], 'load_after': manifest['load_after'],
        'clock': 'Chrome page submission' if capture else 'Chrome page return after submission',
        'target': manifest['target'], 'adapter': manifest['gpu']['gpu']['devices'][0]['deviceString'],
        'recipe': 'Portable',
        'timeline': timeline, 'world': world, 'extent': manifest['encoding']['extent'],
    }


def record_arm(directory, chrome, capture, builds):
    manifest = load(directory / 'manifest.json')
    runtime = load(directory / 'runtime.json')
    if (manifest['source_revision'] != builds['commit']
            or manifest['dirty_diff_sha256'] != hashlib.sha256(b'').hexdigest()):
        raise ValueError(f'{directory.name}: capture was not of the clean recording commit')
    if chrome:
        artifact = manifest['artifacts']['/v1_game_bg.wasm']
        if artifact != builds['wasm_sha256'] or (manifest['control'] == 'ordinary') != capture:
            raise ValueError(f'{directory.name}: wrong Wasm or capture control')
        arm = chrome_arm(runtime, manifest, capture)
    else:
        if manifest['binary_sha256'] != builds['v1_game_sha256'] or runtime['observer_only'] == capture:
            raise ValueError(f'{directory.name}: wrong native program or capture control')
        arm = mac_arm(runtime, manifest, capture)
    arm['median_interval_ms'], arm['longest_intervals'] = intervals(arm['timeline'])
    return arm


def verify(record):
    if record['input_sha256'] != input_digest():
        raise ValueError('motion inputs differ from the recording commit; retake all six arms')
    builds = record['builds']
    if (not re.fullmatch(r'[0-9a-f]{40}', builds['commit']) or builds['dirty'] is not False
            or builds['built_by_this_run'] is not True
            or not all(re.fullmatch(r'[0-9a-f]{64}', builds[f'{name}_sha256'])
                       for name in ('v1_game', 'wasm', 'thrum_cook'))):
        raise ValueError('motion needs a clean commit and all three program hashes')
    baseline = {row['name']: row for row in load(DIAGNOSTICS)['arms']}
    route = load(ROOT / 'examples/v1-game/routes/exit-flythrough.json')
    route_end = route['origin_tick'] + route['keys'][-1]['tick']
    expected = {f'{pair}-{mode}' for pair in COMPARISONS for mode in ('on', 'off')}
    if set(record['arms']) != expected:
        raise ValueError('motion needs exactly the six prescribed arms')
    for name, arm in record['arms'].items():
        pair, mode = name.rsplit('-', 1)
        target = 'chrome-webgpu' if pair == 'chrome-route' else 'Metal'
        if (arm['target'] != target or arm['extent'] != route['render_extent']
                or not all(isinstance(arm[key], str) and arm[key] for key in ('clock', 'adapter', 'recipe'))
                or not isinstance(arm['complete'], bool) or (mode == 'off' and not arm['complete'])):
            raise ValueError(f'{name}: missing runtime identity or observer result')
        for key in ('load_before', 'load_after'):
            load_row = arm[key]
            if len(load_row) != 3 or not all(math.isfinite(value) and value >= 0 for value in load_row):
                raise ValueError(f'{name}: missing system load')
        world = arm['world']
        if pair == 'mac-held':
            if (world['last_tick'] < 1200
                    or world['last_advanced_ms'] - world['released_ms'] < 20_000):
                raise ValueError(f'{name}: the held world did not advance through twenty seconds')
        elif world['last_tick'] < route_end or not math.isfinite(world['last_advanced_ms']):
            raise ValueError(f'{name}: the world stopped before the route ended')
        timeline = arm['timeline']
        if len(timeline) < 2 or any(
            len(row) != (3 if mode == 'on' else 2) or not math.isfinite(row[0])
            or type(row[1]) is not int or row[1] < 0 or (mode == 'on' and not 0 <= row[2] <= 1)
            for row in timeline
        ) or any(after[0] <= before[0] or after[1] < before[1]
                 for before, after in zip(timeline, timeline[1:])):
            raise ValueError(f'{name}: invalid submission timeline')
        median, longest = intervals(timeline)
        if median != arm['median_interval_ms'] or longest != arm['longest_intervals']:
            raise ValueError(f'{name}: missing or inconsistent frame intervals')
        if mode == 'on' and median > 2 * baseline[COMPARISONS[pair]]['median_interval_ms']:
            raise ValueError(f'{name}: capture-on median interval regressed to {median:.2f} ms')
