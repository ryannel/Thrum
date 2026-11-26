"""Observe the capture-off showcase on the Mac and in Chrome."""

import json
import signal
import subprocess
import time
from bisect import bisect_left, bisect_right

from apple_governor_proof import awake_display
from t2_load import LoadRecord

MAC_ROUTE_TIMEOUT = 90


def wait_for_first_present(process, output, launched):
    frames = output / 'frames.jsonl'
    deadline = time.monotonic() + 30
    previous = 0.0
    while process.poll() is None and time.monotonic() < deadline:
        elapsed = time.monotonic() - launched
        if frames.is_file():
            lines = frames.read_text().split('\n')[:-1]
            first = next((row for row in map(json.loads, lines) if row['presented']), None)
            if first:
                return {'label': 'process launch to observed first exit-scene present call',
                        'scope': 'polling bracket for the host present call',
                        'lower_ms': previous * 1000, 'upper_ms': elapsed * 1000,
                        'submission': first['submission']}
        previous = elapsed
        time.sleep(0.025)
    raise ValueError(f'the Mac exit scene did not present; see {output / "game.log"}')


def wait_for_route_record(process, output, first, last):
    """Observe the end of the route and completion of every in-route present's timing sample."""
    deadline = time.monotonic() + MAC_ROUTE_TIMEOUT
    pending = set()
    past_end = False
    with (output / 'frames.jsonl').open('rb') as stream:
        while process.poll() is None and time.monotonic() < deadline:
            position = stream.tell()
            line = stream.readline()
            if not line.endswith(b'\n'):
                stream.seek(position)
                time.sleep(0.025)
                continue
            row = json.loads(line)
            if row['presented']:
                if first <= row['world_tick'] <= last:
                    pending.add(row['submission'])
                past_end |= row['world_tick'] > last
            pending.difference_update(sample['submission'] for sample in row['governor']['samples'])
            if past_end and not pending:
                return
    raise ValueError(f'Mac exit window did not finish the route and its timing record; see {output / "game.log"}')


def read_flushed_rows(path):
    with path.open('rb') as stream:
        return [json.loads(line) for line in stream if line.endswith(b'\n')]


def mac_window(output, binary, env, route, *, day):
    output.mkdir()
    display = output / 'display'
    display.mkdir()
    authored = json.loads(route.read_text())
    admitted = output / 'route.json'
    # The actual window uses its native backing extent. Camera, world clock,
    # cooked publication and route ticks come from the authored route.
    admitted.write_text(json.dumps(dict(authored, render_extent=[1920, 1080])) + '\n')
    parameters = output / 'parameters.json'
    parameters.write_text('{"world.paused": true}\n')
    first = authored['origin_tick']
    last = first + authored['keys'][-1]['tick']
    launch = dict(env, THRUM_GOVERNOR_ROUTE=str(admitted), THRUM_SHOWCASE_ROUTE_START_PAUSED='1',
                  THRUM_PARAMS=str(parameters), THRUM_GOVERNOR_RECORD='observed',
                  THRUM_MAC_TRANSITION=str(output), THRUM_MAC_TRANSITION_ARM='route',
                  THRUM_SHOWCASE_CONTENT_REPORT=str(output / 'drawn.json'),
                  THRUM_MAC_EDR_PROOF=str(display), THRUM_REPORT='1', THRUM_MAC_FRAME_RATE='60',
                  # The display proof expects a host quota. Keep its watchdog beyond the
                  # launcher's deadline and let the observed route end determine when to stop.
                  THRUM_V1_FRAMES=str(120 * (MAC_ROUTE_TIMEOUT + 30)))
    if day:
        launch['THRUM_MAC_SOLAR_SWEEP'] = json.dumps([[first, last], [0, 24]])
    with awake_display(), (output / 'game.log').open('w') as log:
        load = LoadRecord(output)
        try:
            launched = time.monotonic()
            with subprocess.Popen([binary], env=launch, stdout=log, stderr=subprocess.STDOUT) as process:
                try:
                    startup = wait_for_first_present(process, output, launched)
                    resume = parameters.with_suffix('.partial')
                    resume.write_text('{"world.paused": false}\n')
                    resume.replace(parameters)
                    # A present quota would stop early on a fast run or time out on a loaded one.
                    # The append-only receipts are flushed by the host before this observer stops it.
                    wait_for_route_record(process, output, first, last)
                    process.terminate()
                    if process.wait(timeout=10) != -signal.SIGTERM:
                        raise ValueError(f'Mac exit window failed after the route; see {output / "game.log"}')
                finally:
                    if process.poll() is None:
                        process.kill()
        finally:
            observed_load = load.finish()
            (output / 'load.json').write_text(json.dumps(observed_load, indent=2) + '\n')
    return mac_record(output, first, last, day, startup, observed_load)


def mac_record(output, first, last, day, startup, load):
    frames = read_flushed_rows(output / 'frames.jsonl')
    callbacks = read_flushed_rows(output / 'display.jsonl')
    presents = [row for row in frames if row['presented']]
    samples = {sample['submission']: sample for row in frames for sample in row['governor']['samples']}
    if not presents or presents[0]['world_tick'] >= first or presents[-1]['world_tick'] <= last:
        raise ValueError(f'Mac run did not bracket the measurement window {first} to {last}')
    measured = [row for row in presents if first <= row['world_tick'] <= last]
    if not measured or any(row['submission'] not in samples for row in measured):
        raise ValueError('completed frame samples do not cover the measured presents')
    if any(not samples[row['submission']]['gpu_pass_ns'] for row in measured):
        raise ValueError('a completed frame has no named GPU timings')
    sampled = {row['submission']: samples[row['submission']] for row in measured}
    by_sequence = {row['sequence']: row for row in frames}
    requested = [row for row in callbacks
                 if measured[0]['sequence'] <= row['sequence'] <= measured[-1]['sequence'] + 1]
    period = 1 / 60
    latest = measured[0]
    misses = []
    timeline = []
    present_index = 0
    for previous, current in zip(requested, requested[1:]):
        start, target = previous['target'], current['target']
        interval = current['media'] - previous['media']
        if interval <= 0 or target <= start:
            raise ValueError('Mac display callbacks and their targets must increase')
        # The window accounts for presents against the display-link target.
        # Anchor absent opportunities to the new target just as the host does.
        opportunities = max(1, int((target - start) / period + 0.5))
        subdivisions = [start, *(target - index * period for index in range(opportunities - 1, 0, -1)),
                        target]
        for begin, end in zip(subdivisions, subdivisions[1:]):
            while present_index < len(presents) and presents[present_index]['present_media'] < begin:
                latest = presents[present_index]
                present_index += 1
            following_present = present_index
            while (following_present < len(presents)
                   and presents[following_present]['present_media'] < end):
                following_present += 1
            interval_presents = presents[present_index:following_present]
            if len(interval_presents) != 1:
                nearby_frames = [by_sequence[row['sequence']] for row in (previous, current)
                                 if row['sequence'] in by_sequence]
                crossing_work = [{'stage': stage, 'submission': frame['submission'],
                                  'wall_ms': (finish - begin_work) * 1000}
                                 for frame in nearby_frames
                                 for stage, begin_work, finish in frame['work']
                                 if begin_work < end <= finish]
                tick = min(last, max(first, latest['world_tick']))
                position = {'world_tick': tick, 'route_tick': tick - first}
                if day:
                    position['solar_hours'] = 24 * (tick - first) / (last - first)
                misses.append({'position': position, 'duration_ms': (end - begin) * 1000,
                               'target_media': end, 'callback_interval_ms': interval * 1000,
                               'previous_submission': latest['submission'],
                               'interval_submissions': [frame['submission'] for frame in interval_presents],
                               'next_submission': presents[following_present]['submission']
                               if following_present < len(presents) else None,
                               'preceding_callback': previous, 'following_callback': current,
                               'nearby_frame_reports': nearby_frames, 'work_crossing_deadline': crossing_work,
                               'previous_sample': samples.get(latest['submission']),
                               'next_sample': samples.get(presents[following_present]['submission'])
                               if following_present < len(presents) else None,
                               'investigation': ('render-owner wall-time work crossed the deadline'
                                                 if crossing_work else 'unresolved under load'),
                               'evidence_read': 'display-link targets and callback intervals, neighbouring render-owner stages and GPU-complete samples'})
        timeline.append({'sequence': previous['sequence'], 'callback_interval_ms': interval * 1000,
                         'opportunities': opportunities})
    prepared = json.loads((output / 'prepared.json').read_text())
    surfaces = read_flushed_rows(output / 'display/frames.jsonl')
    presented_surfaces = {row['format'] for row in surfaces if row.get('presented')}
    display = 'Edr' if presented_surfaces == {'Rgba16Float'} else (
        'Sdr' if presented_surfaces and all(name.startswith(('Bgra8', 'Rgba8'))
                                           for name in presented_surfaces) else None)
    recipe = prepared['recipe'].removeprefix('Ok(').removesuffix(')')
    selections = {'recipe': recipe, 'reconstruction': prepared['reconstruction'], 'display': display,
                  'governor': measured[0]['governor']['state'] is not None,
                  'culling': next(iter(sampled.values()))['culling']}
    if any(sample['culling'] != selections['culling'] for sample in sampled.values()):
        raise ValueError('the culling mode changed during the measured run')
    content_path = output / 'drawn.json'
    content = json.loads(content_path.read_text()) if content_path.is_file() else {}
    return {'selections': selections, 'cooked_root': content.get('cooked_root'),
            'cooked_generation': content.get('generation'),
            'target': 'mac-metal', 'adapter': prepared['adapter'],
            'phase': 'day cycle' if day else 'flythrough', 'capture': 'off', 'startup_proxy': startup,
            'load': load, 'callbacks': timeline, 'misses': misses,
            'first_requested_tick': first, 'last_requested_tick': last,
            'first_measured_tick': measured[0]['world_tick'], 'last_measured_tick': measured[-1]['world_tick'],
            'first_present_after_route': next(row['world_tick'] for row in presents if row['world_tick'] > last),
            'completed_frames': len(sampled), 'frame_timing': 'frames.jsonl'}


def chrome_record(output, wasm_hash, day):
    manifest = json.loads((output / 'manifest.json').read_text())
    runtime = json.loads((output / 'runtime.json').read_text())
    if (manifest.get('instantiated_wasm_sha256') != wasm_hash or not manifest.get('observer_only')
            or not runtime['complete'] or not runtime['startup_proxy']):
        raise ValueError('Chrome did not complete the exit record from this build with capture off')
    route = runtime['effective_route']
    first = route['origin_tick']
    last = first + route['keys'][-1]['tick']
    callbacks = runtime['opportunities']
    if not callbacks[0]['tick'] <= first or callbacks[-1]['tick'] <= last:
        raise ValueError(f'Chrome run did not bracket the measurement window {first} to {last}')
    samples = [row for row in runtime['exit_frames'] if first <= row['tick'] <= last and row['presented']]
    if not samples or not all(row['timing_requested'] for row in samples):
        raise ValueError('Chrome had no completed frames with timestamp queries requested')
    samples.sort(key=lambda row: (row['tick'], row['submission']))
    measured = [row for row in callbacks if first <= row['tick'] <= last]
    misses = []
    for previous, current in zip(measured, measured[1:]):
        interval = current['raf_ms'] - previous['raf_ms']
        if interval <= 0:
            raise ValueError('Chrome animation timestamps are not increasing')
        if interval < 25:
            continue
        position = {'world_tick': current['tick'], 'route_tick': current['tick'] - first}
        if day:
            position['solar_hours'] = 24 * (current['tick'] - first) / (last - first)
        earlier = bisect_left(samples, previous['tick'], key=lambda row: row['tick'])
        later = bisect_right(samples, current['tick'], key=lambda row: row['tick'])
        misses.append({'position': position, 'duration_ms': interval,
                       'preceding_callback': previous, 'following_callback': current,
                       'preceding_driver_ms': previous['drain_finished_ms'] - previous['requested_ms'],
                       'available_frame_samples': samples[max(0, earlier - 1):later + 1],
                       'investigation': 'unresolved under load',
                       'evidence_read': 'animation-frame timestamps, JS driver times and available GPU pass samples'})
    content = runtime.get('exit_cooked', {})
    return {'selections': runtime['exit_selections'], 'cooked_root': content.get('cooked_root'),
            'cooked_generation': content.get('drawn'), 'target': 'chrome-webgpu',
            'phase': 'day cycle' if day else 'flythrough', 'capture': 'off',
            'startup_proxy': runtime['startup_proxy'], 'misses': misses,
            'first_requested_tick': first, 'last_requested_tick': last,
            'first_measured_tick': measured[0]['tick'], 'last_measured_tick': measured[-1]['tick'],
            'completed_frames': len(samples),
            'frames_with_gpu_pass_timing': sum(bool(row['gpu_pass_ns']) for row in samples),
            'load': {'before': manifest['load_before'], 'after': manifest['load_after']},
            'frame_timing': 'runtime.json'}
