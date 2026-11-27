#!/usr/bin/env python3
"""Launch the cooked valley on the Mac or in Chrome along the G7 showcase route, or check it.

Design: docs/design/decisions/G7-SHOWCASE-ROUTE.md. Each command works in a new run
directory under tmp/g7-showcase/runs/ and records the commit, build identities and arguments
in run.json there. The route's values live only in examples/v1-game/params/showcase-*.params.json.

  mac [--stop NAME] [--frames N]   windowed game on a watched copy of the valley source
  chrome [--stop NAME] [--no-open] Chrome page on a watched, served copy; Ctrl-C stops it
  check [--save-baseline] [--baseline PATH] [--source DIR [--recalibrate]]
                                   visible check: the cooked valley on both targets, not a fallback
  startup [--runs N]               Chrome startup proxy: N fresh browsers, navigation to first observed cooked draw
  moving                           moving proof route (seam 8): native and Chrome twice each, compared per tick
  ticks [--route flythrough|held] [--window FIRST LAST] [--phase PHASE]
                                   native fixed route and exact submitted images in inclusive route-tick windows
  motion [--check]                 record or check the six real-time exit tripwire arms
  exit --controls                  apply every exit preset through the Mac and Chrome control paths
  exit --compare                   compare submitted stills across rows and with each row's baseline
  exit --compare --refresh-baseline CAUSE [--refresh-row ROW] [--baseline PATH]
                                   refresh reviewed row grids with the retained thresholds
  exit --calibrate-comparison      stage a first three-row baseline for visual inspection
  frames --target TARGET           record capture-off flythrough and day cycle timing
"""

import argparse
import base64
import hashlib
import http.server
import itertools
import json
import math
import os
import shutil
import shlex
import signal
import tempfile
import struct
import subprocess
import sys
import threading
import time
import traceback
import urllib.parse
import webbrowser
import zlib
from pathlib import Path

import showcase_motion

REPO = Path(__file__).resolve().parent.parent
PARAMS = REPO / 'examples/v1-game/params'
SOURCE = REPO / 'examples/v1-game/assets/showcase'
EXIT_SOURCE = SOURCE.with_name('exit')
RUNS = REPO / 'tmp/g7-showcase/runs'
COOK = REPO / 'target/release/thrum-cook'
GAME = REPO / 'target/release/v1-game'
SHIPPING_WEB = REPO / 'target/v1-game-web'
WEB = REPO / 'target/v1-game-web-proving'
WASM = WEB / 'v1_game_bg.wasm'
CHECKER = REPO / 'scripts/check_v1_browser.mjs'
BASELINE = REPO / 'examples/v1-game/tests/fixtures/showcase/valley-baseline.json'
EXIT_BASELINE = REPO / 'examples/v1-game/tests/fixtures/showcase/exit-baseline.json'
EXIT_FLYTHROUGH = REPO / 'examples/v1-game/routes/exit-flythrough.json'
EXIT_HELD = EXIT_FLYTHROUGH.with_name('exit-held.json')
EXIT_CONFIG = PARAMS / 'showcase-exit.json'
M5_RUNS = REPO / 'tmp/showcase-m5'
TOOLCHAIN = 'stable'

# Route order. Each stop's frame on both targets comes from applying its preset: native capture with
# THRUM_CAPTURE_PARAMS, Chrome through the checker's route case. Values live only in the presets.
ROUTE = ['sunrise', 'noon', 'sunset-clear-sun', 'sunset-sun', 'twilight', 'night-upslope']
# The diagnostic scene is held at the presets of the five stops with a negative control.
DIAGNOSTIC_STOPS = [stop for stop in ROUTE if stop != 'sunset-clear-sun']
# Frame source version, part of the baseline identity: 1 took stops from the fixed captures and the checker's sky
# phases; 2 applies each preset on both targets.
FRAMES = 2
KEYS = {'world.paused', 'sky.solar_time', 'camera.yaw', 'camera.pitch'}

# Comparator version: 2 maps both extents onto the grid by exact area coverage (version 1 used integer cells,
# which misaligned cell edges between 1252 and 1280 pixel frames by up to a pixel).
COMPARATOR = 2
# Thresholds per statistic (median, p95, max of 32x18 cell mean absolute 8-bit RGB difference), fixed by the
# calibration rule max(1, 3 x worst valley-to-native), rounded up, on the first run of this comparator and source
# root that passes both controls. None means that calibration run. Changing them is a recorded decision.
# Comparator 2, source root 59f0484f (authored ground, 80e519ec): worst valley 0.027, 0.062, 0.161 -> the 1.0 floor.
# Recalibrated for source root 70d2baf8 (G7 M4 continuous ground, 512² tile): worst valley 0.027, 0.052, 0.148 ->
# the same 1.0 floor, thresholds unchanged (approved by Thrum Core, G7 NOTES "Root verdict on ground variant E").
# SHOWCASE M9 lookout rim and 6.32 h sunrise, root f34397a1: worst valley 0.027, 0.050, 0.125 -> the 1.0 floor.
THRESHOLDS = [1.0, 1.0, 1.0]
# The source root the thresholds were calibrated on. Another root needs a reviewed recalibration, so a parity
# regression that arrives with a content change is not absorbed.
CALIBRATED_ROOT = 'f34397a181e38e76a57b8da4ebf00426deadc377976c33dd00f7d127718c1eea'
CONTROL_RATIO = 10.0

# The moving proof route (seam 8). The route file is the only source of its keys, phase and checkpoints.
MOVING_ROUTE = REPO / 'examples/v1-game/routes/moving-v1.json'
# Per-tick identity both targets must log bit for bit at every routed tick.
MOVING_IDENTITY = ['alpha_bits', 'rendered_solar_bits', 'committed_solar_bits', 'eye', 'yaw', 'pitch', 'epoch', 'submission']


class Failure(Exception):
    pass


def route():
    found = sorted(path.name for path in PARAMS.glob('showcase-*.params.json'))
    expected = sorted(f'showcase-{stop}.params.json' for stop in ROUTE)
    if found != expected:
        raise Failure(f'route presets {found} differ from the route {expected}')
    stops = {}
    for stop in ROUTE:
        values = json.loads((PARAMS / f'showcase-{stop}.params.json').read_text())
        if set(values) != KEYS or values['world.paused'] is not True:
            raise Failure(f'{stop}: a route preset holds exactly {sorted(KEYS)} with world.paused true')
        stops[stop] = values
    return stops


def web_presets(web_root=WEB):
    """The web root must ship each checked-in route preset byte for byte, so a stale build never shows an old stop."""
    for stop in ROUTE:
        built = web_root / f'showcase-{stop}.params.json'
        if not built.is_file() or sha256(built) != sha256(PARAMS / built.name):
            raise Failure(f'{built} is missing or differs from the checked-in preset; rebuild the web page')


def sha256(path):
    digest = hashlib.sha256()
    with open(path, 'rb') as file:
        for block in iter(lambda: file.read(1 << 20), b''):
            digest.update(block)
    return digest.hexdigest()


def git(*args):
    return subprocess.run(['git', *args], cwd=REPO, capture_output=True, text=True, check=True).stdout.strip()


def new_run(command, parent=RUNS):
    parent.mkdir(parents=True, exist_ok=True)
    run = parent / f'{command}-{time.strftime("%Y%m%d-%H%M%S")}'
    run.mkdir()
    return run


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')


def build(run, web, *, proving=True, creation_counters=False):
    """Build release thrum-cook and v1-game, plus the selected web bundle if asked."""
    started = time.monotonic()
    with open(run / 'build.log', 'w') as log:
        native = ['rustup', 'run', TOOLCHAIN, 'cargo', 'build', '--release', '--locked',
                  '-p', 'thrum-cook', '-p', 'v1-game', '--bin', 'thrum-cook', '--bin', 'v1-game']
        if creation_counters:
            native.extend(('--features', 'v1-game/creation-counters'))
        steps = [native]
        if web:
            steps.append(['sh', 'examples/v1-game/web/build.sh', *(['--proving'] if proving else [])])
        for step in steps:
            if subprocess.run(step, cwd=REPO, stdout=log, stderr=subprocess.STDOUT).returncode:
                raise Failure(f'build failed: {" ".join(step)} (see {run / "build.log"})')
    return time.monotonic() - started


def identities(web, built, *, web_root=WEB):
    # Under --no-build the binaries may predate the commit; only a run that builds proves they match it.
    # The run's main revision is the commit's merge base with main; origin/main itself may move during a session.
    found = {'commit': git('rev-parse', 'HEAD'), 'main_base': git('merge-base', 'HEAD', 'origin/main'),
             'origin_main_at_run': git('rev-parse', 'origin/main'),
             'dirty': bool(git('status', '--porcelain')), 'built_by_this_run': built}
    for name, path in [('thrum_cook', COOK), ('v1_game', GAME)] + ([('wasm', web_root / WASM.name)] if web else []):
        if not path.exists():
            raise Failure(f'{path} is not built; run without --no-build')
        found[f'{name}_sha256'] = sha256(path)
    return found


def copy_source(run, origin=SOURCE, exit_assets=None):
    source = run / 'source'
    shutil.copytree(origin, source)
    if exit_assets is not None:
        for mesh in sorted(exit_assets.glob('exit-*.*')):
            if mesh.suffix in ('.bin', '.gltf'):
                shutil.copyfile(mesh, source / mesh.name)
    return source


def layout(_args):
    command = [sys.executable, str(SOURCE / 'generate_exit.py')]
    if subprocess.run(command, cwd=REPO).returncode:
        raise Failure('exit layout did not verify against the shipped terrain')


def exit_rows(path=EXIT_CONFIG):
    rows = json.loads(path.read_text())['rows']
    if set(rows) != {'mac-apple', 'mac-portable', 'chrome'}:
        raise Failure('exit configuration must declare the two Mac rows and Chrome')
    expected_keys = {'recipe', 'reconstruction', 'display', 'governor', 'culling'}
    for name, row in rows.items():
        selected_keys = expected_keys - {'recipe', 'display'} if name == 'chrome' else expected_keys
        if set(row) != {'select', 'expect'} or set(row['select']) != selected_keys or set(row['expect']) != expected_keys:
            raise Failure(f'{name}: exit configuration omits or adds a selection')
        if not all(isinstance(value, str) and value for value in row['select'].values()):
            raise Failure(f'{name}: an exit selection is empty or invalid')
    return rows


def native_exit_environment(store, row):
    variables = {'recipe': 'THRUM_RECIPE', 'reconstruction': 'THRUM_RECONSTRUCTION',
                 'display': 'THRUM_MAC_EDR', 'governor': 'THRUM_GOVERNOR', 'culling': 'THRUM_CULLING'}
    env = {key: value for key, value in os.environ.items() if not key.startswith('THRUM_')}
    env.update(THRUM_ASSET_STORE=str(store), THRUM_SCENE='exit')
    env.update({variables[key]: value for key, value in row['select'].items()})
    return env


def declarations(controls):
    return {control['key']: {key: value for key, value in control.items()
                             if key not in {'text', 'value'}} for control in controls}


def complete_lines(path):
    return [json.loads(line) for line in path.read_text().split('\n')[:-1]] if path.is_file() else []


def wait_until(process, label, condition, seconds=30):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline and process.poll() is None:
        result = condition()
        if result:
            return result
        time.sleep(0.1)
    raise Failure(f'the running Mac game did not {label}')


def mac_exit_controls(run, store, row, stops, *, edit_source=None):
    from apple_governor_proof import awake_display
    from edit_valley_content import edit

    output = run / 'panel'
    output.mkdir()
    parameters = run / 'stop.params.json'
    if edit_source is not None:
        stops = {name: {**preset, 'profiler.enabled': True} for name, preset in stops.items()}
    initial = {'world.paused': True}
    if edit_source is not None:
        initial['profiler.enabled'] = True
    write_json(parameters, initial)
    env = dict(native_exit_environment(store, row), THRUM_PARAMS=str(parameters),
               THRUM_PROFILE_OUTPUT=str(output), THRUM_REPORT='1', THRUM_MAC_FRAME_RATE='60')
    content_path = run / 'drawn.json'
    frame_path = run / 'frames.jsonl'
    if edit_source is not None:
        env.update(THRUM_SHOWCASE_CONTENT_REPORT=str(content_path), THRUM_GOVERNOR_RECORD='observed',
                   THRUM_MAC_TRANSITION=str(run), THRUM_MAC_TRANSITION_ARM='route')
    result = {}
    log_path = run / 'game.log'
    with awake_display(), log_path.open('w') as log:
        process = subprocess.Popen([str(GAME)], env=env, cwd=REPO, stdout=log, stderr=subprocess.STDOUT)
        try:
            def saved_generation(expected):
                def read():
                    if not content_path.is_file():
                        return None
                    content = json.loads(content_path.read_text())
                    if content['errors']:
                        raise Failure(f'cooked exit error: {content["errors"]}')
                    return content if content['generation'] == expected else None
                return wait_until(process, f'draw cooked generation {expected}', read)

            if edit_source is not None:
                first_content = saved_generation(1)
                ready = wait_until(process, 'present the prepared window',
                                   lambda: next((record for record in reversed(complete_lines(frame_path))
                                                 if record['presented']), None))
            profiles = output / 'samples.jsonl'

            def initial_profile():
                records = complete_lines(profiles)
                if records and records[-1]['frames']:
                    return records[-1]
                return None

            # Startup can advance the clock before the initial pause is applied.
            # Send every stop through the watcher after the paused window has presented.
            profile = wait_until(process, 'present the paused panel', initial_profile)
            last_submission = profile['frames'][0]['submission']
            for index, (stop, preset) in enumerate(stops.items()):
                pending = parameters.with_suffix('.partial')
                write_json(pending, preset)
                pending.replace(parameters)

                def read_preset():
                    records = complete_lines(profiles)
                    saves = [line for line in log_path.read_text().splitlines() if ' (saved): ' in line]
                    if not records or len(saves) <= index:
                        return None
                    if not saves[index].endswith(', 0 refused'):
                        raise Failure(f'{stop}: {saves[index]}')
                    record = records[-1]
                    by_key = {control['key']: control for control in record['model']['controls']}
                    matches = all(
                        by_key[key]['value'] == expected if isinstance(expected, bool) else
                        close(by_key[key]['value'], expected, by_key[key]['step'] / 2 + 1e-4)
                        for key, expected in preset.items())
                    frames = record['frames']
                    if matches and frames and frames[0]['submission'] > last_submission:
                        return record
                    return None

                record = wait_until(process, f'accept {stop} through the live parameter control', read_preset)
                result[stop] = record
                last_submission = record['frames'][0]['submission']
            if edit_source is not None:
                edits = []
                for generation, operation in enumerate(('mesh', 'texture'), start=2):
                    edit(edit_source, operation)
                    receipt = saved_generation(generation)
                    if (receipt['device_generation'] != first_content['device_generation']
                            or receipt['swaps'] != generation
                            or receipt['submitted_frames'] <= first_content['submitted_frames']):
                        raise Failure(f'{operation}: the live cooked publication was not submitted')
                    edits.append({'product': operation, 'observed_draw': receipt})
                pending = parameters.with_suffix('.partial')
                write_json(pending, {'world.paused': False, 'profiler.enabled': True})
                pending.replace(parameters)

                def active_profiler():
                    profile = complete_lines(output / 'samples.jsonl')[-1]
                    model = {control['key']: control['value'] for control in profile['model']['controls']}
                    if model['world.paused'] or not any(frame['completed_ticks'] for frame in profile['frames']):
                        return None
                    rows = dict(model['profiler.rows'])
                    cpu_keys = ('Callback interval', 'World tick', 'Extract', 'Upload', 'Encode + submit')
                    numeric = all(any(character.isdigit() for character in rows[key]) for key in cpu_keys)
                    if numeric and any('ms' in value for key, value in rows.items() if key.startswith('GPU ')):
                        return model['profiler.rows']
                    return None

                profiled = wait_until(process, 'report current CPU stages and GPU passes in the profiler',
                                      active_profiler)
                before = ready['governor']['pipelines']
                measured = [frame for frame in complete_lines(frame_path) if frame['sequence'] >= ready['sequence']]
                if before is None or before[1] != 0 or any(frame['governor']['pipelines'] != before for frame in measured):
                    raise Failure('native pipeline creation changed after the scene was ready')
                write_json(run / 'clauses.json', {'readiness': ready['sequence'], 'pipeline_count': before[0],
                                                 'observed_frame_replies': len(measured), 'profiler': profiled,
                                                 'initial_content': first_content, 'edits': edits})
        finally:
            stop_process(process)
    return result


def chrome_exit_stops(run, source, row, *, lock=True):
    command = ['node', str(CHECKER), '--case', 'exit-stops', '--cook', str(COOK), '--web-root', str(WEB),
               '--assets-source', str(source), '--output', str(run)]
    env = dict(os.environ, THRUM_SHOWCASE_EXIT_QUERY=urllib.parse.urlencode(row['select']))
    log_path = run.with_suffix('.log')
    with log_path.open('w') as log:
        if bounded(locked_capture('chrome', command) if lock else command, log, 900, cwd=REPO, env=env):
            raise Failure(f'Chrome exit stops failed; see {log_path}')
    wasm_hash = sha256(WASM)
    results = json.loads((run / 'run.json').read_text())['results']
    reports = {}
    for result in results:
        if result['actualWasmSha256'] != wasm_hash:
            raise Failure('Chrome ran a different Wasm build')
        stop = result['case'].removeprefix('exit-stop-')
        report = result['route']
        if report['selections'] != row['expect']:
            raise Failure(f'Chrome {stop} resolved {report["selections"]}, expected {row["expect"]}')
        reports[stop] = report
    if set(reports) != set(ROUTE):
        raise Failure('Chrome did not report each of the six exit stops')
    return reports


def exit_showcase(args):
    if (args.refresh_baseline is not None or args.baseline or args.refresh_row) and not args.compare:
        raise Failure('baseline and refresh options require exit --compare')
    if args.refresh_row and args.refresh_baseline is None:
        raise Failure('--refresh-row requires --refresh-baseline')
    if args.refresh_baseline is not None and not args.refresh_baseline.strip():
        raise Failure('--refresh-baseline requires a nonempty reviewed cause')
    rows = exit_rows()
    stops = route()
    if args.row:
        return exit_row(args.row, rows[args.row], stops)
    if args.clauses:
        return exit_clauses(args, rows)
    if args.compare or args.calibrate_comparison:
        return exit_compare(rows, stops, calibrate=args.calibrate_comparison,
                            baseline_path=Path(args.baseline).resolve() if args.baseline else EXIT_BASELINE,
                            refresh=args.refresh_baseline, refresh_row=args.refresh_row)
    run = new_run('exit-controls')
    print(f'exit controls: {run}', flush=True)
    build(run, web=True)
    builds = identities(web=True, built=True)
    source = copy_source(run, exit_assets=EXIT_SOURCE)
    root = json.loads(EXIT_FLYTHROUGH.read_text())['cooked_root']
    store = cook_exit(run, source, root)
    report = {'builds': builds, 'target': 'mac-metal and chrome-webgpu', 'control_declarations': {}}
    for name in ('mac-apple', 'mac-portable'):
        output = run / name
        output.mkdir()
        results = mac_exit_controls(output, store, rows[name], stops)
        write_json(output / 'presets.json', results)
        report['control_declarations'][name] = {
            stop: declarations(record['model']['controls']) for stop, record in results.items()}
    chrome = chrome_exit_stops(run / 'chrome', source, rows['chrome'])
    report['control_declarations']['chrome'] = {
        stop: declarations(record['controls']) for stop, record in chrome.items()}
    reference = report['control_declarations']['mac-portable'][ROUTE[0]]
    for name, models in report['control_declarations'].items():
        for stop, model in models.items():
            if model != reference:
                raise Failure(f'{name} {stop}: declared controls differ from the Mac panel')
    write_json(run / 'run.json', report)
    print(f'PASS: {run / "run.json"}')


def native_exit_stops(run, store, row, stops, *, keys=ROUTE, empty=False):
    presets = [PARAMS / f'showcase-{stop}.params.json' for stop in keys]
    env = dict(native_exit_environment(store, row), THRUM_CAPTURE_SCENE=str(run),
               THRUM_CAPTURE_ROUTE_ONLY='1', THRUM_CAPTURE_SOURCE='submitted-sdr',
               THRUM_CAPTURE_PARAMS=os.pathsep.join(map(str, presets)))
    if row['select']['display'] != 'sdr':
        env['THRUM_CAPTURE_EDR_HEADROOM'] = '1'
    if empty:
        env['THRUM_EXIT_NO_ENTITIES'] = '1'
    with run.with_suffix('.log').open('w') as log:
        if bounded([str(GAME)], log, 120, cwd=REPO, env=env):
            raise Failure(f'{run.name}: native submitted stills failed')
    manifest = json.loads((run / 'stills.json').read_text())
    expected = row['expect']
    resolved = {key: manifest[key] for key in ('recipe', 'reconstruction', 'culling', 'governor')}
    if resolved != {key: f'Ok({value})' if key == 'recipe' else value
                    for key, value in expected.items() if key != 'display'}:
        raise Failure(f'{run.name}: stills resolved different render selections: {resolved}')
    frames = manifest['stills']
    for frame, stop in zip(frames, keys, strict=True):
        preset = stops[stop]
        if (not frame['surface_presented'] or frame['headroom'] != 1
                or frame['source'] != expected['display']
                or not close(frame['time_of_day'] * 24, preset['sky.solar_time'], 0.0051)
                or not close(math.degrees(frame['yaw']), preset['camera.yaw'], 1e-3)
                or not close(math.degrees(frame['pitch']), preset['camera.pitch'], 1e-3)):
            raise Failure(f'{run.name}: submitted frame does not match {stop}')
    return manifest


def exit_row(name, row, stops):
    web = name == 'chrome'
    run = new_run(f'exit-row-{name}')
    print(f'exit row: {run}', flush=True)
    build(run, web=web, creation_counters=not web)
    builds = identities(web=web, built=True)
    source = copy_source(run, exit_assets=EXIT_SOURCE)
    route_file = json.loads(EXIT_FLYTHROUGH.read_text())
    expected_root = route_file['cooked_root']
    store = cook_exit(run, source, expected_root)
    if web:
        stop_reports = chrome_exit_stops(run / 'stops', source, row, lock=False)
        publications = {stop: (report['cooked'].get('drawn'), report['cooked'].get('cooked_root'))
                        for stop, report in stop_reports.items()}
    else:
        stop_reports = native_exit_stops(run / 'stops', store, row, stops)
        publications = {
            stop: (frame.get('cooked_generation'), frame.get('cooked_root'))
            for stop, frame in zip(ROUTE, stop_reports['stills'], strict=True)}
    flythrough = record_exit_phase(run / 'flythrough', source, store, row, builds, web=web, day=False)
    publications['flythrough'] = (flythrough.get('cooked_generation'), flythrough.get('cooked_root'))
    write_json(run / 'run.json', {'builds': builds, 'row': name, 'target': flythrough['target'],
                                 'stops': stop_reports, 'flythrough': flythrough,
                                 'drawn_publications': {phase: {'generation': generation, 'root': root}
                                                        for phase, (generation, root) in publications.items()}})
    expected = (route_file['cooked_generation'], expected_root)
    wrong = [phase for phase, publication in publications.items() if publication != expected]
    if wrong:
        raise Failure(f'{name}: missing or wrong drawn publication for {", ".join(wrong)}; see {run / "run.json"}')
    print(f'PASS: {run / "run.json"}')


def exit_compare(rows, stops, *, calibrate, baseline_path, refresh, refresh_row):
    from apple_display_proof import half_components, HALF, srgb

    root = json.loads(EXIT_FLYTHROUGH.read_text())['cooked_root']
    identity = {**baseline_identity(root), 'exit_rows': rows}
    baseline_bytes = baseline_path.read_bytes()
    baseline = json.loads(baseline_bytes)
    baseline_hash = hashlib.sha256(baseline_bytes).hexdigest()
    retained = baseline.get('exit_comparison')
    if calibrate and retained and retained['identity'] == identity:
        raise Failure('this comparison identity already has a calibration')
    if not calibrate and (not retained or retained['identity'] != identity):
        raise Failure('no retained comparison for these rows, comparator, presets and cooked root')
    refresh_rows = {name for name in rows if refresh is not None and refresh_row in (None, name)}

    run = new_run('exit-compare')
    print(f'exit comparison: {run}', flush=True)
    build(run, web=True)
    builds = identities(web=True, built=True)
    source = copy_source(run, exit_assets=EXIT_SOURCE)
    store = cook_exit(run, source, root)
    transfer = bytes(srgb(value) for value in HALF)

    def capture_native(name, row, *, path=store, keys=ROUTE, empty=False):
        target = run / name
        manifest = native_exit_stops(target, path, row, stops, keys=keys, empty=empty)
        expected_root = store_root(path)
        values = {}
        for frame, key in zip(manifest['stills'], keys, strict=True):
            if frame['cooked_root'] != expected_root:
                raise Failure(f'{name} {key}: captured another cooked publication')
            extent = manifest['extent']
            filename = target / frame['file']
            if frame['source'] == 'Edr':
                pixels = bytes(transfer[word] for word in half_components(filename, extent))
            else:
                pixels = filename.read_bytes()
            values[key] = pixel_grid(filename, *extent, 4, pixels)
        return values

    def capture_rows(suffix=''):
        grids = {name: capture_native(name + suffix, rows[name])
                 for name in ('mac-apple', 'mac-portable')}
        chrome_dir = run / ('chrome' + suffix)
        grids['chrome'] = {}
        for key, result in chrome_exit_stops(chrome_dir, source, rows['chrome']).items():
            if result['cooked']['cooked_root'] != root:
                raise Failure(f'Chrome {key}: captured another cooked publication')
            frame = result['submitted']
            path = chrome_dir / frame['file']
            grids['chrome'][key] = pixel_grid(path, frame['width'], frame['height'], 4, path.read_bytes())
        return grids

    grids = capture_rows()
    repeat = capture_rows('-repeat') if calibrate else None
    empty = capture_native('empty', rows['mac-portable'], empty=True)
    retinted = run / 'retinted'
    retinted.mkdir()
    edited_source = copy_source(retinted, origin=source)
    ground = edited_source / 'ground.png'
    original_ground = sha256(ground)
    ground.write_bytes(solid_texture([214, 150, 96]))
    edited_store = retinted / 'store'
    with (retinted / 'cook.log').open('w') as log:
        if bounded([str(COOK), 'cook', str(edited_source), str(edited_store)], log, 120, cwd=REPO):
            raise Failure('retinted control could not cook')
    if store_root(edited_store) == root:
        raise Failure('retinted control did not change the publication')
    texture = capture_native('texture-control', rows['mac-portable'], path=edited_store, keys=['noon'])
    diagnostic = run / 'diagnostic'
    command = ['node', str(CHECKER), '--case', 'route-diagnostic', '--cook', str(COOK),
               '--web-root', str(WEB), '--assets-source', str(source), '--output', str(diagnostic)]
    with (run / 'diagnostic.log').open('w') as log:
        if bounded(locked_capture('chrome', command), log, 120, cwd=REPO):
            raise Failure('Chrome diagnostic control failed')
    phases = json.loads((diagnostic / 'run.json').read_text())['results'][0]['sky']['phases']
    report = {'builds': builds, 'identity': identity, 'encoding': 'submitted 8-bit sRGB',
              'sources': {'mac-apple': 'EDR at headroom 1, converted from float to sRGB',
                          'mac-portable': 'SDR at headroom 1', 'chrome': 'SDR'},
              'baseline_sha256': baseline_hash,
              'texture_control': {'source': 'ground.png', 'before_sha256': original_ground,
                                  'after_sha256': sha256(ground), 'cooked_root': store_root(edited_store)},
              'cross_row_thresholds': {}, 'pairs': {}, 'controls': {}, 'baselines': {}, 'failures': []}
    verify_diagnostic_poses({phase['name']: phase for phase in phases}, stops, report)
    for key in ROUTE:
        pairs = report['pairs'][key] = {}
        for left, right in itertools.combinations(grids, 2):
            cells = cell_differences(grids[left][key], grids[right][key])
            worst = max(range(len(cells)), key=cells.__getitem__)
            row_key = f'{left} vs {right}'
            pairs[row_key] = {'values': difference_statistics(cells), 'max_cell': [worst % 32, worst // 32]}
        thresholds = report['cross_row_thresholds'][key] = (
            [1.0, *(max(1.0, 3 * max(pair['values'][index] for pair in pairs.values())) for index in (1, 2))]
            if calibrate else retained['cross_row_thresholds'][key])
        for pair, result in pairs.items():
            if any(value > limit for value, limit in zip(result['values'], thresholds, strict=True)):
                report['failures'].append(f'{key}: {pair} exceeded the fixed thresholds')
        values = {'empty': difference(empty[key], grids['mac-portable'][key])}
        if key in DIAGNOSTIC_STOPS:
            fallback = grid(diagnostic / f'route-diagnostic-{key}.png')
            values['diagnostic'] = difference(fallback, grids['mac-portable'][key])
        if key == 'noon':
            values['retinted'] = difference(texture[key], grids['mac-portable'][key])
        controls = report['controls'][key] = {}
        for name, distances in values.items():
            ratios = [value / limit for value, limit in zip(distances, thresholds, strict=True)]
            controls[name] = {'values': distances, 'threshold_ratios': ratios}
            separated = max(ratios) >= CONTROL_RATIO if name == 'diagnostic' else max(ratios) > 1
            if not separated:
                report['failures'].append(f'{key}: {name} control failed to separate')

    saved_rows = {}
    for name, row in grids.items():
        if calibrate:
            repeat_delta = {key: difference(row[key], repeat[name][key]) for key in ROUTE}
            for key, distances in repeat_delta.items():
                if any(value > limit for value, limit in
                       zip(distances, report['cross_row_thresholds'][key], strict=True)):
                    report['failures'].append(f'{name}/{key}: row did not repeat under its cross-row thresholds')
            thresholds = [max(1.0, 3 * max(values[index] for values in repeat_delta.values()))
                          for index in range(3)]
            saved = {'thresholds': thresholds, 'grids': {key: pack(cells) for key, cells in row.items()}}
            saved_rows[name] = saved
        else:
            saved = retained['rows'][name]
        distances = {key: difference(cells, unpack(saved['grids'][key])) for key, cells in row.items()}
        report['baselines'][name] = {'thresholds': saved['thresholds'], 'stops': distances}
        if calibrate:
            report['baselines'][name]['repeat_differences'] = repeat_delta
        over = [key for key, values in distances.items()
                if any(value > limit for value, limit in zip(values, saved['thresholds'], strict=True))]
        report['baselines'][name]['over_thresholds'] = over
        if name not in refresh_rows:
            report['failures'].extend(f'{name}/{key}: exceeded the row baseline thresholds' for key in over)

    if refresh_rows and not report['failures']:
        try:
            report['refresh'] = refresh_exit_rows(baseline_path, baseline, baseline_hash, grids, report,
                                                  refresh_rows, refresh.strip())
        except Failure as error:
            report['failures'].append(str(error))
    write_json(run / 'run.json', report)
    if report['failures']:
        raise Failure(f'{", ".join(report["failures"])}; see {run / "run.json"}')
    if calibrate:
        baseline['exit_comparison'] = {
            'identity': identity, 'cross_row_thresholds': report['cross_row_thresholds'], 'rows': saved_rows,
            'calibration': {'run': run.name, 'builds': builds},
        }
        (run / EXIT_BASELINE.name).write_text(json.dumps(baseline, indent=1) + '\n')
        print(f'Staged for inspection: {run / EXIT_BASELINE.name}')
    else:
        print(f'PASS: {run / "run.json"}')


def refresh_exit_rows(path, baseline, before_hash, grids, report, selected, cause):
    from fcntl import LOCK_EX, flock

    retained = baseline['exit_comparison']
    history = retained.setdefault('refreshes', [])
    updates = {}
    new_grids = {}
    for name in sorted(selected):
        current = retained['rows'][name]['grids']
        replacement = {key: pack(cells) for key, cells in grids[name].items()}
        updated = [key for key in ROUTE if replacement[key] != current[key]]
        if not updated:
            continue
        before_builds = next((entry['after_builds'] for entry in reversed(history) if name in entry['rows']),
                             retained['calibration']['builds'])
        previous = report['baselines'][name]
        updates[name] = {'before_builds': before_builds, 'updated_stops': updated,
                         'grid_difference': previous['stops'], 'over_thresholds': previous['over_thresholds']}
        new_grids[name] = replacement
    if not updates:
        raise Failure('no selected exit row baseline grid moved')
    record = {'cause': cause, 'before_file_sha256': before_hash, 'after_builds': report['builds'], 'rows': updates}
    history.append(record)
    for name, replacement in new_grids.items():
        retained['rows'][name]['grids'] = replacement
    content = (json.dumps(baseline, indent=1) + '\n').encode()
    with path.open('rb') as before:
        flock(before, LOCK_EX)
        if sha256(path) != before_hash:
            raise Failure('the baseline changed during the refresh')
        with tempfile.NamedTemporaryFile(dir=path.parent, prefix=f'.{path.name}.', delete=False) as output:
            temporary = Path(output.name)
            try:
                output.write(content)
                output.close()
                temporary.chmod(path.stat().st_mode)
                temporary.replace(path)
            finally:
                temporary.unlink(missing_ok=True)
    return {**record, 'after_file_sha256': hashlib.sha256(content).hexdigest()}


def exit_clauses(args, rows):
    web = args.target == 'chrome-webgpu'
    run = new_run(f'exit-clauses-{args.target}')
    print(f'exit clauses: {run}', flush=True)
    build(run, web=web, creation_counters=not web)
    builds = identities(web=web, built=True)
    if not web:
        report = {'builds': builds, 'target': args.target, 'rows': {}}
        for name in ('mac-apple', 'mac-portable'):
            output = run / name
            output.mkdir()
            source = copy_source(output, exit_assets=EXIT_SOURCE)
            watcher, _ = watch(output, source, serve=False)
            try:
                presets = mac_exit_controls(output, output / 'store', rows[name], route(), edit_source=source)
            finally:
                stop_process(watcher)
            write_json(output / 'presets.json', presets)
            report['rows'][name] = json.loads((output / 'clauses.json').read_text())
        write_json(run / 'run.json', report)
        print(f'PASS: {run / "run.json"}')
        return
    source = copy_source(run, exit_assets=EXIT_SOURCE)
    row = rows['chrome']
    output = run / 'chrome'
    command = ['node', str(CHECKER), '--case', 'exit-clauses', '--cook', str(COOK), '--web-root', str(WEB),
               '--assets-source', str(source), '--output', str(output)]
    env = dict(os.environ, THRUM_SHOWCASE_EXIT_QUERY=urllib.parse.urlencode(row['select']))
    with (run / 'chrome.log').open('w') as log:
        if bounded(command, log, 180, cwd=REPO, env=env):
            raise Failure(f'Chrome exit clauses failed; see {run / "chrome.log"}')
    result, = json.loads((output / 'run.json').read_text())['results']
    if result['actualWasmSha256'] != builds['wasm_sha256'] or result['exit']['selections'] != row['expect']:
        raise Failure('Chrome exit clauses ran another build or resolved different selections')
    write_json(run / 'run.json', {'builds': builds, 'target': args.target, **result['exit']})
    print(f'PASS: {run / "run.json"}')


def record_exit_phase(output, source, store, row, builds, *, web, day):
    import showcase_frames
    from apple_governor_proof import awake_display

    phase = 'day' if day else 'flythrough'
    route_path = EXIT_HELD if day else EXIT_FLYTHROUGH
    if web:
        command = ['node', 'scripts/capture_v1_browser.mjs', '--mode', 'realtime', '--control', 'off',
                   '--scene', 'exit', '--overlay', 'off', '--exit-record', phase, '--wasm-receipt', 'on',
                   '--route-file', str(route_path), '--web-root', str(WEB), '--cook', str(COOK),
                   '--assets-source', str(source), '--output', str(output)]
        for key, value in row['select'].items():
            command.extend((f'--{key}', value))
        log_path = output.with_suffix('.log')
        with awake_display(), log_path.open('w') as log:
            if bounded(command, log, 240, cwd=REPO):
                raise Failure(f'{output.name}: browser record failed; see {log_path}')
        record = showcase_frames.chrome_record(output, builds['wasm_sha256'], day)
    else:
        record = showcase_frames.mac_window(
            output, GAME, native_exit_environment(store, row), route_path, day=day)
    write_json(output / 'record.json', record)
    if record['selections'] != row['expect']:
        raise Failure(f'{output.name}: resolved {record["selections"]}, expected {row["expect"]}')
    return record


def exit_frames(args):
    rows = exit_rows()
    web = args.target == 'chrome-webgpu'
    run = new_run(f'exit-frames-{args.target}')
    print(f'exit frame records: {run}', flush=True)
    build(run, web=web, creation_counters=not web)
    builds = identities(web=web, built=True)
    source = copy_source(run, exit_assets=EXIT_SOURCE)
    root = json.loads(EXIT_FLYTHROUGH.read_text())['cooked_root']
    store = cook_exit(run, source, root)
    report = {'builds': builds, 'target': args.target, 'rows': {}}
    for name in (('chrome',) if web else ('mac-apple', 'mac-portable')):
        report['rows'][name] = {
            phase: record_exit_phase(run / f'{name}-{phase}', source, store, rows[name], builds, web=web, day=day)
            for phase, day in (('flythrough', False), ('day', True))}
    write_json(run / 'run.json', report)
    print(f'PASS: {run / "run.json"}')


def store_root(store):
    """The generation-1 root of a cooked store, refusing any other generation."""
    head = (store / 'HEAD').read_bytes()
    if head[:8] != b'THRMHEAD' or struct.unpack('<Q', head[16:24])[0] != 1:
        raise Failure(f'the cooked store {store} is not at generation 1')
    return head[24:56].hex()


def watch(run, source, serve):
    """Start thrum-cook watch on the copy; wait for generation 1 (and the origin when serving)."""
    log_path = run / 'cook.log'
    log = open(log_path, 'w')
    command = [str(COOK), 'watch', str(source), str(run / 'store')] + (['--serve', '127.0.0.1:0'] if serve else [])
    process = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT)
    log.close()
    try:
        deadline = time.monotonic() + 120
        while time.monotonic() < deadline:
            if process.poll() is not None:
                raise Failure(f'thrum-cook exited early (see {log_path})')
            text = log_path.read_text()
            origin = next((line.split()[1] for line in text.splitlines() if line.startswith('serving ')), None)
            if 'generation 1' in text and (origin or not serve):
                return process, origin
            time.sleep(0.1)
        raise Failure('thrum-cook did not publish generation 1 within 120 s')
    except BaseException:
        stop_process(process)
        raise


def stop_process(process):
    if process and process.poll() is None:
        process.terminate()
        try:
            process.wait(10)
        except subprocess.TimeoutExpired:
            process.kill()


def mac(args):
    stops = route()
    run = new_run('mac')
    seconds = None if args.no_build else build(run, web=False)
    record = {'command': 'mac', 'stop': args.stop, 'frames': args.frames, 'build_seconds': seconds,
              'builds': identities(web=False, built=not args.no_build), 'preset': stops[args.stop],
              'store': str(run / 'store'), 'scene': 'exit' if args.exit else 'valley'}
    write_json(run / 'run.json', record)
    watcher, _ = watch(run, copy_source(run, exit_assets=EXIT_SOURCE if args.exit else None), serve=False)
    env = dict(os.environ, THRUM_ASSET_STORE=str(run / 'store'), THRUM_SCENE=record['scene'],
               THRUM_PARAMS=str(PARAMS / f'showcase-{args.stop}.params.json'))
    if args.frames:
        env['THRUM_V1_FRAMES'] = str(args.frames)
    print(f'mac: {args.stop}; edits to {run / "source"} cook live; L re-applies the preset. Run: {run}', flush=True)
    game = None
    try:
        with open(run / 'game.log', 'w') as log:
            game = subprocess.Popen([str(GAME)], env=env, stdout=log, stderr=subprocess.STDOUT)
            code = game.wait()
    except KeyboardInterrupt:
        code = 'interrupted'
    finally:
        stop_process(game)
        stop_process(watcher)
    record['exit_code'] = code
    write_json(run / 'run.json', record)
    if code:
        raise Failure(f'v1-game exited {code} (see {run / "game.log"})')


class Logged(http.server.SimpleHTTPRequestHandler):
    """Static handler that appends each request line and status to the run's requests.log."""
    log_path = None

    def log_message(self, *_):
        pass

    def log_request(self, code='-', size='-'):
        with open(self.log_path, 'a') as log:
            log.write(f'{time.time():.3f} {code} {self.requestline}\n')


def chrome(args):
    route()
    run = new_run('chrome')
    seconds = None if args.no_build else build(run, web=True, proving=False)
    record = {'command': 'chrome', 'stop': args.stop, 'build_seconds': seconds,
              'builds': identities(web=True, built=not args.no_build, web_root=SHIPPING_WEB)}
    web_presets(SHIPPING_WEB)
    watcher = server = None
    try:
        watcher, assets = watch(run, copy_source(run, exit_assets=EXIT_SOURCE if args.exit else None), serve=True)
        # build.sh copies the presets into the web root, so the page fetches them beside index.html.
        handler = type('Handler', (Logged,), {'log_path': run / 'requests.log'})
        server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), lambda *a: handler(*a, directory=str(SHIPPING_WEB)))
        threading.Thread(target=server.serve_forever, daemon=True).start()
        inputs = {'assets': assets, 'params': f'showcase-{args.stop}.params.json'}
        if args.exit:
            inputs['scene'] = 'exit'
            if rain := os.environ.get('THRUM_RAIN'):
                inputs['rain'] = rain
        query = urllib.parse.urlencode(inputs)
        url = f'http://127.0.0.1:{server.server_address[1]}/index.html?{query}'
        record['url'] = url
        write_json(run / 'run.json', record)
        print(f'chrome: {url}\nRun: {run}; Ctrl-C stops both servers.', flush=True)
        if not args.no_open:
            webbrowser.get('open -a "Google Chrome" %s').open(url)
        while watcher.poll() is None:
            time.sleep(0.5)
        raise Failure('thrum-cook watch exited')
    except KeyboardInterrupt:
        pass
    finally:
        if server:
            server.shutdown()
        stop_process(watcher)


# --- Pixel comparison (stdlib PNG and PPM readers) ---

def read_png(path):
    data = Path(path).read_bytes()
    if data[:8] != b'\x89PNG\r\n\x1a\n':
        raise Failure(f'{path}: not a PNG')
    pos, idat = 8, []
    while pos < len(data):
        length, kind = struct.unpack('>I4s', data[pos:pos + 8])
        body = data[pos + 8:pos + 8 + length]
        if kind == b'IHDR':
            width, height, depth, colour, _, _, interlace = struct.unpack('>IIBBBBB', body)
            if depth != 8 or colour not in (2, 6) or interlace:
                raise Failure(f'{path}: unsupported PNG layout')
            bpp = 3 if colour == 2 else 4
        elif kind == b'IDAT':
            idat.append(body)
        pos += 12 + length
    raw = zlib.decompress(b''.join(idat))
    stride = width * bpp
    out = bytearray(height * stride)
    previous = bytearray(stride)
    offset = 0
    for y in range(height):
        kind = raw[offset]
        line = bytearray(raw[offset + 1:offset + 1 + stride])
        offset += 1 + stride
        if kind == 1:
            for x in range(bpp, stride):
                line[x] = (line[x] + line[x - bpp]) & 255
        elif kind == 2:
            line = bytearray((a + b) & 255 for a, b in zip(line, previous))
        elif kind == 3:
            for x in range(stride):
                left = line[x - bpp] if x >= bpp else 0
                line[x] = (line[x] + (left + previous[x]) // 2) & 255
        elif kind == 4:
            for x in range(stride):
                a = line[x - bpp] if x >= bpp else 0
                b = previous[x]
                c = previous[x - bpp] if x >= bpp else 0
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[x] = (line[x] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        elif kind != 0:
            raise Failure(f'{path}: unknown PNG filter {kind} on row {y}')
        out[y * stride:(y + 1) * stride] = line
        previous = line
    return width, height, bpp, bytes(out)


def read_ppm(path):
    data = Path(path).read_bytes()
    fields, pos = [], 0
    while len(fields) < 4:
        while data[pos:pos + 1].isspace():
            pos += 1
        if data[pos:pos + 1] == b'#':
            pos = data.index(b'\n', pos)
            continue
        end = pos
        while not data[end:end + 1].isspace():
            end += 1
        fields.append(data[pos:end])
        pos = end
    width, height = int(fields[1]), int(fields[2])
    pixels = data[pos + 1:]
    if fields[0] != b'P6' or fields[3] != b'255' or len(pixels) != width * height * 3:
        raise Failure(f'{path}: not a complete 8-bit binary PPM')
    return width, height, 3, pixels


def coverage(length, cells):
    """Each pixel's (cell, weight) parts when `length` pixels are mapped onto `cells` equal cells.

    Weights are exact area fractions, so frames of different extents average over the same regions
    of the view. Integer cell assignment would move cell edges by up to a pixel between extents.
    """
    parts = []
    for p in range(length):
        start, end = p * cells / length, (p + 1) * cells / length
        first = int(start)
        if first + 1 < end and first + 1 < cells:
            parts.append(((first, first + 1 - start), (first + 1, end - first - 1)))
        else:
            parts.append(((min(first, cells - 1), end - start),))
    return parts


def grid(path, columns=32, rows=18, crop=None):
    width, height, bpp, pixels = read_ppm(path) if str(path).endswith('.ppm') else read_png(path)
    return pixel_grid(path, width, height, bpp, pixels, columns, rows, crop)


def pixel_grid(path, width, height, bpp, pixels, columns=32, rows=18, crop=None):
    if len(pixels) != width * height * bpp:
        raise Failure(f'{path}: pixel bytes differ from {width}x{height}x{bpp}')
    x0, y0, x1, y1 = crop or (0, 0, width, height)
    if not (0 <= x0 < x1 <= width and 0 <= y0 < y1 <= height):
        raise Failure(f'{path}: crop {crop} outside {width}x{height}')
    across, down = coverage(x1 - x0, columns), coverage(y1 - y0, rows)
    sums = [[0.0, 0.0, 0.0, 0.0] for _ in range(columns * rows)]
    for y in range(y1 - y0):
        # Sum each source row into per-column totals first, then spread those over the rows it covers.
        line = [[0.0, 0.0, 0.0, 0.0] for _ in range(columns)]
        base = (y + y0) * width * bpp
        for x in range(x1 - x0):
            o = base + (x + x0) * bpp
            r, g, b = pixels[o], pixels[o + 1], pixels[o + 2]
            for cell, weight in across[x]:
                total = line[cell]
                total[0] += r * weight
                total[1] += g * weight
                total[2] += b * weight
                total[3] += weight
        for row, weight in down[y]:
            for column, total in enumerate(line):
                cell = sums[row * columns + column]
                cell[0] += total[0] * weight
                cell[1] += total[1] * weight
                cell[2] += total[2] * weight
                cell[3] += total[3] * weight
    return [[s[0] / s[3], s[1] / s[3], s[2] / s[3]] for s in sums]


def cell_differences(a, b):
    return [sum(abs(p - q) for p, q in zip(x, y, strict=True)) / 3 for x, y in zip(a, b, strict=True)]


def difference_statistics(cells):
    cells = sorted(cells)
    return [cells[len(cells) // 2], cells[int(len(cells) * 0.95)], cells[-1]]


def difference(a, b):
    return difference_statistics(cell_differences(a, b))


def close(a, b, tolerance):
    return abs(float(a) - float(b)) <= tolerance


def verify_diagnostic_poses(phases, stops, report):
    rows = report['diagnostic_poses'] = {}
    for stop in DIAGNOSTIC_STOPS:
        preset = stops[stop]
        expected = {'hours': preset['sky.solar_time'], 'yaw': preset['camera.yaw'], 'pitch': preset['camera.pitch']}
        applied = {key: phases[stop]['edited']['controls'][key] for key in expected}
        rows[stop] = {'preset': expected, 'applied': applied}
        if not all(close(applied[key], expected[key], tolerance)
                   for key, tolerance in [('hours', 0.0051), ('yaw', 0.5001), ('pitch', 0.5001)]):
            report['failures'].append(f'diagnostic pose drift at {stop}: {applied} vs {expected}')


def pack(cells):
    """A grid as base64 little-endian uint16 hundredths of an 8-bit level (0.005 rounding)."""
    return base64.b64encode(b''.join(struct.pack('<H', round(v * 100)) for cell in cells for v in cell)).decode()


def unpack(text):
    raw = base64.b64decode(text)
    values = [v / 100 for (v,) in struct.iter_unpack('<H', raw)]
    return [values[i:i + 3] for i in range(0, len(values), 3)]


def read_baseline(baseline_path, identity):
    if not baseline_path.exists():
        return None, {'status': 'absent'}
    saved = json.loads(baseline_path.read_text())
    saved_identity = saved['identity']
    differs = sorted(key for key in saved_identity.keys() | identity.keys()
                     if key not in saved_identity or key not in identity or saved_identity[key] != identity[key])
    if differs:
        return saved, {'status': 'other identity', 'differs': differs, 'baseline_identity': saved_identity}
    return saved, None


def compare_baseline(baseline, grids, thresholds):
    """Each stop's difference from the saved baseline, when the baseline is for this identity."""
    saved, unmatched = baseline
    if unmatched:
        # Same comparator, frames and route on other content: the difference locates what the content change moved.
        # It is information only and never judged.
        if unmatched.get('differs') == ['source_root']:
            unmatched['informational'] = {f'{target}/{stop}': difference(cells, unpack(saved['grids'][target][stop]))
                                          for target, stops in grids.items() for stop, cells in stops.items()}
        return unmatched
    result = {'status': 'compared', 'stops': {}, 'over': []}
    for target, stops in grids.items():
        for stop, cells in stops.items():
            values = difference(cells, unpack(saved['grids'][target][stop]))
            result['stops'][f'{target}/{stop}'] = values
            if any(value > limit for value, limit in zip(values, thresholds)):
                result['over'].append(f'{target}/{stop}')
    return result


def baseline_identity(root):
    return {'comparator': COMPARATOR, 'frames': FRAMES, 'source_root': root,
            'route': {stop: sha256(PARAMS / f'showcase-{stop}.params.json') for stop in ROUTE}}


def save_baseline(baseline_path, grids, identity, report, run):
    baseline_path.write_text(json.dumps({
        'about': 'G7 showcase route baseline: per-stop 32x18 grids consumed by scripts/run_v1_showcase.py check. '
                 'Reproduce with `check`; replace only with `check --save-baseline` after reviewing the frames, and '
                 'record why in the working folder, .orchestrate/evidence/v1-workload/.',
        'identity': identity,
        'builds': report['builds'],
        'thresholds': report['thresholds'],
        'run': run.name,
        'grids': {target: {stop: pack(cells) for stop, cells in stops.items()} for target, stops in grids.items()},
    }, indent=1) + '\n')


def bounded(command, log, seconds, **options):
    """Run one check step with a deadline, so a hung step fails the check instead of holding it."""
    try:
        return subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, timeout=seconds, **options).returncode
    except subprocess.TimeoutExpired:
        raise Failure(f'{Path(command[0]).name} exceeded {seconds} s') from None


def solid_texture(rgb):
    def chunk(kind, payload):
        return struct.pack('>I', len(payload)) + kind + payload + struct.pack('>I', zlib.crc32(kind + payload))

    header = struct.pack('>IIBBBBB', 4, 4, 8, 2, 0, 0, 0)
    rows = (b'\0' + bytes(rgb) * 4) * 4
    return b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', header) + chunk(b'IDAT', zlib.compress(rows)) + chunk(b'IEND', b'')


def check(args):
    if args.exit:
        return check_exit(args)
    if args.refresh_baseline is not None:
        raise Failure('--refresh-baseline requires --exit')
    stops = route()
    origin = Path(args.source).resolve()
    baseline_path = Path(args.baseline).resolve()
    if args.recalibrate and args.save_baseline and baseline_path == BASELINE:
        raise Failure('--recalibrate cannot save the canonical baseline: update THRESHOLDS and CALIBRATED_ROOT by review')
    run = new_run('check')
    report = {'command': 'check', 'route': stops, 'comparator': COMPARATOR, 'passed': False, 'failures': [],
              'source': {'path': str(origin), 'canonical': origin == SOURCE}, 'recalibrate': args.recalibrate}
    report['build_seconds'] = None if args.no_build else build(run, web=True)
    report['builds'] = identities(web=True, built=not args.no_build)
    write_json(run / 'run.json', report)
    failures = report['failures']
    timings = report['timings_seconds'] = {}
    try:
        source = copy_source(run, origin)
        started = time.monotonic()
        with open(run / 'cook.log', 'w') as log:
            if bounded([str(COOK), 'cook', str(source), str(run / 'store')], log, 120):
                raise Failure('thrum-cook cook failed')
        timings['cook'] = time.monotonic() - started
        root = report['generation_1_root'] = store_root(run / 'store')
        # Recalibration is only for another source root; on the calibrated one it could only loosen the check.
        if args.recalibrate and root == CALIBRATED_ROOT:
            raise Failure(f'--recalibrate on the calibrated source root {root[:8]}: the fixed thresholds apply')
        identity = baseline_identity(root)
        saved_baseline = read_baseline(baseline_path, identity)
        if origin == SOURCE and baseline_path == BASELINE and not (args.recalibrate or args.save_baseline):
            if unmatched := saved_baseline[1]:
                report['baseline'] = unmatched
                raise Failure(f'canonical baseline was not compared: {unmatched["status"]} {unmatched.get("differs", "")}')

        # Native: THRUM_SCENE=valley makes capture require the store and refuse unless it is drawn.
        native = run / 'native'
        presets = [PARAMS / f'showcase-{stop}.params.json' for stop in ROUTE]
        env = dict(os.environ, THRUM_CAPTURE_SCENE=str(native), THRUM_ASSET_STORE=str(run / 'store'), THRUM_SCENE='valley',
                   THRUM_CAPTURE_ROUTE_ONLY='1', THRUM_CAPTURE_PARAMS=os.pathsep.join(str(path) for path in presets))
        launched = time.time()
        started = time.monotonic()
        with open(run / 'native.log', 'w') as log:
            code = bounded([str(GAME)], log, 300, env=env)
        timings['native_capture_run'] = time.monotonic() - started
        if code:
            raise Failure(f'native capture refused or failed (exit {code}; see {run / "native.log"})')
        manifest = json.loads((native / 'captures.json').read_text())
        poses = {record['file']: record for record in manifest['route']}
        first = min((native / 'route' / file).stat().st_mtime for file in poses)
        timings['native_cold_start_proxy'] = first - launched
        for stop, values in stops.items():
            pose = poses.get(f'showcase-{stop}.ppm')
            # The controls snap solar time to the 0.01 h slider stop, as on every target.
            if not pose or pose['unchanged'] + pose['changed'] != len(KEYS) or not (
                    close(pose['hours'], values['sky.solar_time'], 0.0051) and close(pose['yaw_degrees'], values['camera.yaw'], 1e-3)
                    and close(pose['pitch_degrees'], values['camera.pitch'], 1e-3)):
                failures.append(f'native pose drift at {stop}: {pose} vs {values}')

        # Chrome: the route presets on the cooked valley; the valley case for its live texture edit (the positive
        # control); and the diagnostic scene without ?assets as the negative control.
        chrome_runs = {}
        for case, name in [('route', 'chrome-route'), ('valley', 'chrome-valley'), ('route-diagnostic', 'chrome-diagnostic')]:
            started = time.monotonic()
            with open(run / f'{name}.log', 'w') as log:
                code = bounded(['node', str(CHECKER), '--case', case, '--cook', str(COOK), '--web-root', str(WEB),
                                       '--assets-source', str(source), '--output', str(run / name)],
                               log, 900, cwd=REPO)
            timings[f'{name}_checker_run'] = time.monotonic() - started
            if code:
                raise Failure(f'checker --case {case} failed (see {run / f"{name}.log"})')
            results = json.loads((run / name / 'run.json').read_text())['results']
            for result in results:
                if result['actualWasmSha256'] != report['builds']['wasm_sha256']:
                    raise Failure(f'{result["case"]}: Chrome loaded Wasm {result["actualWasmSha256"]}, not the build')
            if case == 'route':
                chrome_runs[case] = {result['case'].removeprefix('route-'): result['route'] for result in results}
            else:
                chrome_runs[case] = {phase['name']: phase for phase in results[0]['sky']['phases']}
        for name in ['chrome-route', 'chrome-valley']:
            if not (run / name / 'store/cas' / root[:2] / root).is_file():
                failures.append(f'same cooked input: generation-1 root {root} is absent from the {name} store')
        if sorted(chrome_runs['route']) != sorted(ROUTE):
            raise Failure(f'Chrome route stops {sorted(chrome_runs["route"])} differ from the route {sorted(ROUTE)}')
        for stop, values in stops.items():
            stage = chrome_runs['route'][stop]
            want = {'hours': values['sky.solar_time'], 'yaw': values['camera.yaw'], 'pitch': values['camera.pitch']}
            got = stage['applied']
            # The checker refuses any report with a refused entry, and holds one paused tick through the capture.
            if (stage['sha256'] != sha256(PARAMS / f'showcase-{stop}.params.json') or stage['settled'].get('paused') is not True
                    or not (close(got['hours'], want['hours'], 0.0051) and close(got['yaw'], want['yaw'], 0.5001)
                            and close(got['pitch'], want['pitch'], 0.5001))):
                failures.append(f'Chrome route pose drift at {stop}: {got} vs {want}, preset {stage["sha256"][:8]}')
        verify_diagnostic_poses(chrome_runs['route-diagnostic'], stops, report)
        if failures:
            raise Failure('pose drift')

        # Compare matched stops.
        grids = {'native': {stop: grid(native / 'route' / f'showcase-{stop}.ppm') for stop in ROUTE},
                 'chrome': {stop: grid(run / 'chrome-route' / f'route-{stop}.png') for stop in ROUTE}}
        statistics = report['statistics'] = {}
        for stop in ROUTE:
            statistics[stop] = {'valley': difference(grids['chrome'][stop], grids['native'][stop])}
            if stop in DIAGNOSTIC_STOPS:
                diagnostic = grid(run / 'chrome-diagnostic' / f'route-diagnostic-{stop}.png')
                statistics[stop]['diagnostic'] = difference(diagnostic, grids['native'][stop])
        # The edit frames are full-page shots; compare only the canvas, located by the checker's own clip.
        clip = chrome_runs['valley']['phase-paused-scene-noon']['images'][0]['clip']
        canvas = (round(clip['x']), round(clip['y']), round(clip['x'] + clip['width']), round(clip['y'] + clip['height']))
        report['positive_control_crop'] = canvas
        edit = difference(grid(run / 'chrome-valley/valley-before-edit-final.png', crop=canvas),
                          grid(run / 'chrome-valley/valley-edited-final.png', crop=canvas))
        report['positive_control'] = edit
        worst = [max(values['valley'][i] for values in statistics.values()) for i in range(3)]
        rule = [max(1.0, 3 * value) for value in worst]
        report['calibration'] = {'worst_valley': worst, 'rule_thresholds': rule, 'fixed_thresholds': THRESHOLDS}
        if THRESHOLDS and root != CALIBRATED_ROOT and not args.recalibrate:
            raise Failure(f'thresholds were calibrated on source root {CALIBRATED_ROOT[:8]}, not {root[:8]}: '
                          'recalibrate by the design rule (--recalibrate) and record it')
        # --recalibrate judges this run, both controls included, by the rule rather than the fixed thresholds.
        thresholds = rule if args.recalibrate else THRESHOLDS or rule
        report['calibration']['applied'] = 'rule (--recalibrate)' if args.recalibrate else 'fixed' if THRESHOLDS else 'rule (calibration run)'
        report['thresholds'] = thresholds
        names = ['median', 'p95', 'max']
        for stop, values in statistics.items():
            over = [f'{names[i]} {values["valley"][i]:.3f} > {thresholds[i]:.3f}' for i in range(3) if values['valley'][i] > thresholds[i]]
            if over:
                failures.append(f'Chrome fallback or divergence at {stop}: {", ".join(over)}')
            if 'diagnostic' in values and not any(values['diagnostic'][i] >= CONTROL_RATIO * thresholds[i] for i in range(3)):
                failures.append(f'negative control: the diagnostic scene at {stop} is within {CONTROL_RATIO}x of the thresholds')
        if not any(edit[i] > thresholds[i] for i in range(3)):
            failures.append('positive control: the retinted texture did not exceed any threshold')
        baseline = report['baseline'] = compare_baseline(saved_baseline, grids, thresholds)
        baseline['path'] = str(baseline_path.relative_to(REPO)) if baseline_path.is_relative_to(REPO) else str(baseline_path)
        drifted = baseline['status'] == 'compared' and baseline['over']
        # Replacing the baseline is the deliberate answer to drift, so drift alone does not block --save-baseline;
        # every other failure does. The replaced drift stays in run.json.
        if drifted and not args.save_baseline:
            failures.append(f'baseline drift beyond the thresholds at {", ".join(baseline["over"])}')
        if args.save_baseline and not failures:
            save_baseline(baseline_path, grids, identity, report, run)
            baseline['saved'] = baseline['path']
            if drifted:
                baseline['replaced_drift'] = baseline['over']
    except Failure as error:
        failures.append(str(error))
    except Exception:  # Record any other fault in run.json before failing.
        failures.append(traceback.format_exc())
    except KeyboardInterrupt:
        failures.append('interrupted')
    report['passed'] = not failures
    write_json(run / 'run.json', report)
    for stop, values in report.get('statistics', {}).items():
        diagnostic = " ".join(f"{v:7.2f}" for v in values["diagnostic"]) if 'diagnostic' in values else '(no diagnostic phase)'
        print(f'{stop:>16}  valley {" ".join(f"{v:7.3f}" for v in values["valley"])}  diagnostic {diagnostic}')
    if 'thresholds' in report:
        print(f'thresholds {report["thresholds"]}  positive control {report["positive_control"]}')
    baseline = report.get('baseline', {})
    if baseline.get('status') == 'compared':
        worst = [max(values[i] for values in baseline['stops'].values()) for i in range(3)]
        print(f'baseline: worst difference {" ".join(f"{v:.3f}" for v in worst)} over {len(baseline["stops"])} frames')
    elif baseline:
        print(f'baseline: {baseline["status"]} {baseline.get("differs", "")}')
        for name, values in baseline.get('informational', {}).items():
            print(f'  informational difference from it, {name:>24}: {" ".join(f"{v:7.3f}" for v in values)}')
    if baseline.get('saved'):
        replaced = f' (replacing drift at {", ".join(baseline["replaced_drift"])})' if baseline.get('replaced_drift') else ''
        print(f'baseline saved: {baseline["saved"]}{replaced}')
    print(f'{"PASS" if report["passed"] else "FAIL"}: {run / "run.json"}')
    for failure in failures:
        print(f'  {failure}')
    if failures:
        raise SystemExit(1)


def check_exit(args):
    stops = route()
    if args.source != str(SOURCE) or args.baseline != str(BASELINE) or args.recalibrate:
        raise Failure('exit parity uses its own pinned source and baseline')
    if args.save_baseline == EXIT_BASELINE.exists():
        raise Failure('use --save-baseline exactly once to calibrate the exit check')
    if args.refresh_baseline is not None and (args.save_baseline or not args.refresh_baseline.strip()):
        raise Failure('--refresh-baseline requires a calibrated baseline and a nonempty cause')
    run = new_run('exit-check')
    source = copy_source(run, exit_assets=EXIT_SOURCE)
    report = {'passed': False, 'failures': [], 'load_start': os.getloadavg()}
    print(f'exit route parity: {run}', flush=True)
    try:
        if not args.no_build:
            build(run, web=True)
        report['builds'] = identities(web=True, built=not args.no_build)

        def execute(command, label, seconds=300, **options):
            with open(run / f'{label}.log', 'w') as log:
                if bounded(command, log, seconds, cwd=REPO, **options):
                    raise Failure(f'{label} failed (see {run / f"{label}.log"})')

        store = run / 'store'
        execute([str(COOK), 'cook', str(source), str(store)], 'cook')
        root = report['source_root'] = store_root(store)
        presets = [PARAMS / f'showcase-{stop}.params.json' for stop in ROUTE]
        env = dict(os.environ, THRUM_ASSET_STORE=str(store), THRUM_SCENE='exit', THRUM_RECIPE='portable',
                   THRUM_CAPTURE_ROUTE_ONLY='1', THRUM_CAPTURE_PARAMS=os.pathsep.join(map(str, presets)))
        env.pop('THRUM_EXIT_NO_ENTITIES', None)
        execute([str(GAME)], 'native', env=dict(env, THRUM_CAPTURE_SCENE=str(run / 'native')))
        execute([str(GAME)], 'empty', env=dict(env, THRUM_CAPTURE_SCENE=str(run / 'empty'), THRUM_EXIT_NO_ENTITIES='1'))
        retinted_source = run / 'retinted-source'
        shutil.copytree(source, retinted_source)
        (retinted_source / 'white.png').write_bytes(solid_texture([214, 150, 96]))
        retinted_store = run / 'retinted-store'
        execute([str(COOK), 'cook', str(retinted_source), str(retinted_store)], 'retinted-cook')
        report['retinted_root'] = store_root(retinted_store)
        if report['retinted_root'] == root:
            raise Failure('the retinted texture did not change the publication')
        execute([str(GAME)], 'retinted', env=dict(env, THRUM_ASSET_STORE=str(retinted_store),
                THRUM_CAPTURE_SCENE=str(run / 'retinted'),
                THRUM_CAPTURE_PARAMS=str(PARAMS / 'showcase-noon.params.json')))
        for case, name in [('exit-stops', 'chrome'), ('route-diagnostic', 'diagnostic')]:
            execute(['node', str(CHECKER), '--case', case, '--cook', str(COOK), '--web-root', str(WEB),
                     '--assets-source', str(source), '--output', str(run / name)], name, seconds=900)
            results = json.loads((run / name / 'run.json').read_text())['results']
            if any(result['actualWasmSha256'] != report['builds']['wasm_sha256'] for result in results):
                raise Failure(f'{name}: stale Wasm')
            if case == 'route-diagnostic':
                phases = {phase['name']: phase for phase in results[0]['sky']['phases']}
                verify_diagnostic_poses(phases, stops, report)
                if report['failures']:
                    raise Failure('diagnostic pose drift')
        if root != store_root(run / 'chrome/store'):
            raise Failure('Chrome and native cooked roots differ')
        chrome = json.loads((run / 'chrome/run.json').read_text())['results']
        chrome = {result['case'].removeprefix('exit-stop-'): result['route'] for result in chrome}
        poses = json.loads((run / 'native/captures.json').read_text())['route']
        poses = {pose['file']: pose for pose in poses}
        for stop, preset in stops.items():
            pose = poses[f'showcase-{stop}.ppm']
            if not (close(pose['hours'], preset['sky.solar_time'], 0.0051)
                    and close(pose['yaw_degrees'], preset['camera.yaw'], 1e-3)
                    and close(pose['pitch_degrees'], preset['camera.pitch'], 1e-3)):
                raise Failure(f'native pose differs from the preset at {stop}')
            if chrome[stop]['exit']['in_view'] < 1:
                raise Failure(f'Chrome saw no entities at {stop}')
        grids = {'native': {stop: grid(run / 'native/route' / f'showcase-{stop}.ppm') for stop in ROUTE},
                 'chrome': {stop: grid(run / 'chrome' / f'exit-stop-{stop}.png') for stop in ROUTE}}
        stats = report['statistics'] = {}
        for stop, native in grids['native'].items():
            row = stats[stop] = {'parity': difference(grids['chrome'][stop], native),
                                 'empty': difference(grid(run / 'empty/route' / f'showcase-{stop}.ppm'), native),
                                 'in_view': chrome[stop]['exit']['in_view']}
            if stop in DIAGNOSTIC_STOPS:
                diagnostic = run / 'diagnostic' / f'route-diagnostic-{stop}.png'
                row['diagnostic'] = difference(grid(diagnostic), native)
        edit = report['positive_control'] = difference(grids['native']['noon'],
            grid(run / 'retinted/route/showcase-noon.ppm'))
        worst = [max(row['parity'][i] for row in stats.values()) for i in range(3)]
        identity = baseline_identity(root)
        if args.save_baseline:
            thresholds = [max(1.0, 3 * value) for value in worst]
        else:
            fixed, unmatched = read_baseline(EXIT_BASELINE, identity)
            if unmatched:
                raise Failure('exit baseline identity changed')
            thresholds = fixed['thresholds']
        report['thresholds'] = thresholds
        report['worst_parity'] = worst
        failures = report['failures']
        for stop, row in stats.items():
            if any(value > limit for value, limit in zip(row['parity'], thresholds)):
                failures.append(f'{stop}: Chrome/native parity exceeded the threshold')
            if not any(value > limit for value, limit in zip(row['empty'], thresholds)):
                failures.append(f'{stop}: the scene without entities passed the comparison')
            if 'diagnostic' in row and not any(value >= CONTROL_RATIO * limit
                                              for value, limit in zip(row['diagnostic'], thresholds)):
                failures.append(f'{stop}: diagnostic comparison did not reach {CONTROL_RATIO} times the threshold')
        if not any(value > limit for value, limit in zip(edit, thresholds)):
            failures.append('retinted texture passed the comparison')
        if args.save_baseline and not failures:
            save_baseline(EXIT_BASELINE, grids, identity, report, run)
        elif not args.save_baseline:
            baseline = report['baseline'] = compare_baseline((fixed, unmatched), grids, thresholds)
            if args.refresh_baseline is not None and not failures:
                refresh_exit_baseline(fixed, grids, report, run, args.refresh_baseline.strip())
            elif baseline['status'] != 'compared' or baseline['over']:
                failures.append('exit baseline failed')
    except (Failure, OSError, ValueError, KeyError) as error:
        report['failures'].append(str(error))
    finally:
        report['load_end'] = os.getloadavg()
        report['passed'] = not report['failures']
        write_json(run / 'run.json', report)
    if not report['passed']:
        raise Failure(f'{", ".join(report["failures"])} (see {run / "run.json"})')
    print(f'PASS: {run / "run.json"}')


def refresh_exit_baseline(saved, grids, report, run, cause):
    if 'refresh' in saved:
        raise Failure('the M5 exit baseline has already been refreshed')
    delta = report['baseline']
    if not any(any(values) for values in delta['stops'].values()):
        raise Failure('no exit baseline grid moved')
    saved['refresh'] = {
        'cause': cause,
        'before_builds': saved['builds'],
        'after_builds': report['builds'],
        'before_file_sha256': sha256(EXIT_BASELINE),
        'run': run.name,
        'grid_difference': delta['stops'],
        'over_thresholds': delta['over'],
    }
    saved['grids'] = {target: {stop: pack(cells) for stop, cells in stops.items()}
                      for target, stops in grids.items()}
    temporary = EXIT_BASELINE.with_suffix('.partial')
    temporary.write_text(json.dumps(saved, indent=1) + '\n')
    temporary.replace(EXIT_BASELINE)
    delta['refreshed_sha256'] = sha256(EXIT_BASELINE)


def locked_capture(resource, command):
    script = os.environ.get('ORCHESTRATE_SCRIPT')
    if not script:
        raise Failure('set ORCHESTRATE_SCRIPT to agent_loop.py for the capture resource locks')
    return [sys.executable, script, 'lock', resource, '--', *command]


def cook_exit(run, source, expected_root):
    store = run / 'store'
    with (run / 'cook.log').open('w') as log:
        if bounded([str(COOK), 'cook', str(source), str(store)], log, 120, cwd=REPO):
            raise Failure(f'exit cook failed (see {run / "cook.log"})')
    if store_root(store) != expected_root:
        raise Failure('exit cooked root differs from the route')
    return store


def ticks(args):
    route_path = EXIT_HELD if args.route == 'held' else EXIT_FLYTHROUGH
    route = showcase_motion.load(route_path)
    last = route['keys'][-1]['tick']
    windows = args.window
    if any(not 0 <= first <= end <= last for first, end in windows):
        raise Failure(f'--window requires ordered route ticks between 0 and {last}')
    if args.phase is not None and not 0 < args.phase < 1:
        raise Failure('--phase must be between zero and one')
    command = locked_capture('gpu', [str(GAME)])
    run = new_run(f'ticks-{args.route}', M5_RUNS)
    print(run, flush=True)
    if not args.no_build:
        build(run, web=False)
    builds = identities(web=False, built=not args.no_build)
    store = cook_exit(run, copy_source(run, exit_assets=EXIT_SOURCE), route['cooked_root'])
    frames = run / 'frames'
    env = dict(os.environ, WGPU_BACKEND='metal', THRUM_ASSET_STORE=str(store), THRUM_SCENE='exit',
               THRUM_RECIPE='portable', THRUM_RECONSTRUCTION='off', THRUM_ROUTE_OVERLAY='off',
               THRUM_CAPTURE_SOURCE='sdr', THRUM_CAPTURE_DISPLAY_OVERLAY='off',
               THRUM_CAPTURE_ROUTE=str(route_path), THRUM_CAPTURE_SCENE=str(frames),
               THRUM_CAPTURE_WINDOWS=json.dumps(windows))
    env.pop('THRUM_CAPTURE_PHASE', None)
    if args.phase is not None:
        env['THRUM_CAPTURE_PHASE'] = str(args.phase)
    with (run / 'capture.log').open('w') as log:
        if bounded(command, log, 2400, cwd=REPO, env=env):
            raise Failure(f'fixed Metal route failed (see {run / "capture.log"})')
    header, *rows = map(json.loads, (frames / 'route.jsonl').read_text().splitlines())
    phase = args.phase if args.phase is not None else route['phase'][0] / route['phase'][1]
    origin = route['origin_tick']
    if (header['backend'] != 'Metal' or header['mode'] != 'fixed' or len(rows) != last + 1
            or any(row['route_tick'] != i or row['tick'] != origin + i
                   or not math.isclose(row['alpha'], phase, abs_tol=1e-4)
                   or row['submission'] != rows[0]['submission'] + i
                   for i, row in enumerate(rows))):
        raise Failure('fixed Metal route did not retain one submission for every routed tick at the requested phase')
    retained = {}
    for row in rows:
        chosen = any(first <= row['route_tick'] <= end for first, end in windows)
        requested = []
        if chosen:
            requested.append(frames / 'windows' / f'tick-{row["tick"]:06}')
        if row['checkpoint']:
            requested.append(frames / 'checkpoints' / row['checkpoint'])
        for stem in requested:
            identity = showcase_motion.load(stem.with_suffix('.identity.json'))
            pixels = stem.with_suffix('.rgba8')
            if (identity['source_tick'] != row['tick'] or identity['alpha_bits'] != row['alpha_bits']
                    or identity['submission_id'] != row['submission']
                    or pixels.stat().st_size != math.prod(route['render_extent']) * 4):
                raise Failure(f'{stem}: submitted image does not match the route log')
            retained[str(pixels.relative_to(run))] = sha256(pixels)
    write_json(run / 'run.json', {'builds': builds, 'route': args.route, 'route_sha256': sha256(route_path),
                                  'cooked_root': route['cooked_root'], 'phase': phase, 'windows': windows,
                                  'source_ticks': [origin, origin + last], 'retained_rgba_sha256': retained})
    print(f'PASS: {len(rows)} consecutive routed submissions, {len(retained)} images; {run / "run.json"}')


def motion_pair(run, pair, common, on, off, resource, env=None):
    commands = [shlex.join([*common, '--output', str(run / f'{pair}-{mode}'), *options])
                for mode, options in [('on', on), ('off', off)]]
    # The second arm still supplies evidence if the first finished the route with export omissions.
    command = locked_capture(resource, ['/bin/sh', '-c', '\n'.join(commands)])
    with (run / f'{pair}.log').open('w') as log:
        if bounded(command, log, 2400, cwd=REPO, env=env):
            raise Failure(f'{pair} capture-off arm failed (see {run / f"{pair}.log"})')


def motion(args):
    if args.check:
        showcase_motion.verify(showcase_motion.load(showcase_motion.RECORD))
        print(f'PASS: {showcase_motion.RECORD.relative_to(REPO)}')
        return
    if git('status', '--porcelain'):
        raise Failure('commit all tracked and untracked changes before recording motion')
    locked_capture('gpu', [])  # Check lock support before the build.
    run = new_run('motion', M5_RUNS)
    print(run, flush=True)
    revision = git('rev-parse', 'HEAD')
    digest = showcase_motion.input_digest()
    build(run, web=True)
    builds = identities(web=True, built=True)
    source = copy_source(run, exit_assets=EXIT_SOURCE)
    route = showcase_motion.load(EXIT_FLYTHROUGH)
    store = cook_exit(run, source, route['cooked_root'])
    width, height = map(str, route['render_extent'])
    native = [sys.executable, 'scripts/capture_v1_runtime.py', '--mode', 'realtime', '--binary', str(GAME),
              '--extent', width, height, '--recipe', 'portable', '--reconstruction', 'off', '--overlay', 'off',
              '--store', str(store), '--defer']
    env = dict(os.environ, WGPU_BACKEND='metal', THRUM_CAPTURE_ENCODING='raw')
    for pair, options in [('mac-route', ['--case', 'exit-route', '--route', str(EXIT_FLYTHROUGH)]),
                          ('mac-held', ['--case', 'exit-drift'])]:
        motion_pair(run, pair, [*native, *options], [], ['--observer-only'], 'gpu', env)
    chrome = ['node', 'scripts/capture_v1_browser.mjs', '--mode', 'realtime', '--scene', 'exit',
              '--overlay', 'off', '--route-file', str(EXIT_FLYTHROUGH), '--web-root', str(WEB),
              '--assets-source', str(source), '--cook', str(COOK)]
    motion_pair(run, 'chrome-route', chrome, ['--retain-raw', 'on'], ['--control', 'off'], 'chrome')
    if (builds['commit'] != revision or git('rev-parse', 'HEAD') != revision
            or git('status', '--porcelain') or showcase_motion.input_digest() != digest):
        raise Failure('the checkout changed during motion recording')
    arms = {}
    for pair in showcase_motion.COMPARISONS:
        for mode in ('on', 'off'):
            name = f'{pair}-{mode}'
            arms[name] = showcase_motion.record_arm(run / name, pair == 'chrome-route', mode == 'on', builds)
    record = {'builds': builds, 'input_sha256': digest, 'cooked_root': route['cooked_root'], 'arms': arms,
              'timeline_columns': ['elapsed_ms', 'committed_tick', 'phase (capture-on only)'],
              'longest_interval_columns': ['duration_ms', 'start_elapsed_ms', 'start_tick', 'end_tick']}
    showcase_motion.verify(record)
    temporary = run / showcase_motion.RECORD.name
    write_json(temporary, record)
    temporary.replace(showcase_motion.RECORD)
    print(f'PASS: {showcase_motion.RECORD.relative_to(REPO)}')


def spread(values):
    values = sorted(values)
    return {'count': len(values), 'min': values[0], 'median': values[len(values) // 2] if len(values) % 2
            else (values[len(values) // 2 - 1] + values[len(values) // 2]) / 2, 'max': values[-1]} if values else None


def startup(args):
    """Chrome startup proxy: each launch is the checker's route case in a fresh Chrome process and profile."""
    stops = route()
    run = new_run('startup')
    report = {'command': 'startup', 'runs': args.runs, 'passed': False, 'failures': [],
              'build_seconds': None if args.no_build else build(run, web=True)}
    report['builds'] = identities(web=True, built=not args.no_build)
    web_presets()
    source = copy_source(run)
    rows = report['rows'] = []
    launches = report['launches'] = []
    try:
        for launch in range(1, args.runs + 1):
            output = run / f'launch-{launch}'
            started = time.monotonic()
            with open(run / f'launch-{launch}.log', 'w') as log:
                code = bounded(['node', str(CHECKER), '--case', 'route', '--cook', str(COOK), '--web-root', str(WEB),
                                '--assets-source', str(source), '--output', str(output)], log, 900, cwd=REPO)
            seconds = time.monotonic() - started
            if code:
                raise Failure(f'launch {launch}: checker --case route failed (see {run / f"launch-{launch}.log"})')
            results = json.loads((output / 'run.json').read_text())['results']
            root = store_root(output / 'store')
            launches.append({'launch': launch, 'checker_seconds': seconds, 'generation_1_root': root, 'pages': len(results)})
            # Pages run one after another in one browser; the earliest page clock is the launch's first page.
            for order, result in enumerate(sorted(results, key=lambda r: r['route']['startup']['timeOrigin'])):
                stop = result['case'].removeprefix('route-')
                stage, observed = result['route'], result['route']['startup']
                if result['actualWasmSha256'] != report['builds']['wasm_sha256']:
                    raise Failure(f'launch {launch} {stop}: Chrome loaded Wasm {result["actualWasmSha256"]}, not the build')
                if stage['sha256'] != sha256(PARAMS / f'showcase-{stop}.params.json'):
                    raise Failure(f'launch {launch} {stop}: served preset differs from the repository')
                if observed['observeDelayMs']:
                    raise Failure(f'launch {launch} {stop}: the first observation was delayed for testing')
                rows.append({'launch': launch, 'page': order + 1, 'cold': order == 0, 'stop': stop,
                             # Without an observation that establishes no draw, the lower bound is navigation start.
                             'lower_observed': observed['lowerObserved'],
                             'lower_ms': observed['lowerMs'], 'upper_ms': observed['upperMs'],
                             'granularity_ms': observed['granularityMs'], 'median_poll_ms': observed['medianPollMs'],
                             'phases_ms': observed['phases'], 'wasm': observed['timing']['wasm'],
                             'cooked_fetch_ms': [observed['timing']['cooked']['start'], observed['timing']['cooked']['end']],
                             'uploaded_bytes': observed['uploadedBytes'], 'preset_sha256': stage['sha256']})
        roots = {launch['generation_1_root'] for launch in launches}
        if len(roots) != 1:
            raise Failure(f'launches cooked different roots: {sorted(roots)}')
        report['generation_1_root'] = roots.pop()
        report['summary_upper_ms'] = {'cold': spread([row['upper_ms'] for row in rows if row['cold']]),
                                      'warm': spread([row['upper_ms'] for row in rows if not row['cold']])}
        report['summary_granularity_ms'] = spread([row['granularity_ms'] for row in rows if row['lower_observed']])
        report['lower_bound_absent'] = sum(not row['lower_observed'] for row in rows)
        report['clock'] = 'page performance.now(), ms since navigation start'
        report['cache'] = ('fresh Chrome process and profile per launch; HTTP cache disabled for each page; the OS file '
                           'cache and any GPU shader cache outside the profile stay warm')
        report['scope'] = ('Headless CDP polling proxy for navigation to the first observed cooked draw on hardware WebGPU; '
                           'the bracket between the lower and upper bound is the granularity, or only the navigation-start bound when no lower '
                           'observation exists; not physical first-paint latency. Build, cook and checker '
                           'time are recorded apart and not counted.')
    except Failure as error:
        report['failures'].append(str(error))
    except Exception:
        report['failures'].append(traceback.format_exc())
    except KeyboardInterrupt:
        report['failures'].append('interrupted')
    report['passed'] = not report['failures']
    write_json(run / 'run.json', report)
    print(f'{"launch":>6} {"page":>4} {"stop":>16} {"cold":>4} {"lower":>7} {"upper":>7} {"gran":>6}')
    for row in rows:
        lower, gap = (f'{row["lower_ms"]:.1f}', f'{row["granularity_ms"]:.1f}') if row['lower_observed'] else ('(0)', '-')
        print(f'{row["launch"]:>6} {row["page"]:>4} {row["stop"]:>16} {"yes" if row["cold"] else "":>4} '
              f'{lower:>7} {row["upper_ms"]:7.1f} {gap:>6}')
    for kind, values in report.get('summary_upper_ms', {}).items():
        if values:
            print(f'{kind} upper bound ms: n {values["count"]} min {values["min"]:.1f} median {values["median"]:.1f} max {values["max"]:.1f}')
    if report.get('lower_bound_absent'):
        print(f'{report["lower_bound_absent"]} page(s) had no observation before the draw: lower bound is navigation start')
    print(f'{"PASS" if report["passed"] else "FAIL"}: {run / "run.json"}')
    for failure in report['failures']:
        print(f'  {failure}')
    if report['failures']:
        raise SystemExit(1)


def moving_native(run, name, store, route_path=MOVING_ROUTE, refusal=None):
    env = dict(os.environ, THRUM_CAPTURE_SCENE=str(run / name), THRUM_ASSET_STORE=str(store), THRUM_SCENE='valley',
               THRUM_CAPTURE_ROUTE=str(route_path), THRUM_ROUTE_OVERLAY='proof')
    with open(run / f'{name}.log', 'w') as log:
        code = bounded([str(GAME)], log, 300, env=env)
    if refusal is not None:
        # A negative control: the native driver must refuse, for the named reason.
        text = (run / f'{name}.log').read_text()
        if not code or refusal not in text:
            raise Failure(f'native control {name} was not refused for "{refusal}" (exit {code}; see {run / f"{name}.log"})')
        return next(line for line in text.splitlines() if refusal in line)
    if code:
        raise Failure(f'native moving route refused or failed (exit {code}; see {run / f"{name}.log"})')
    lines = [json.loads(line) for line in (run / name / 'route.jsonl').read_text().splitlines()]
    samples = {}
    for line in lines[1:]:
        # The native log carries the committed solar hour on each sample; Chrome reports it per step.
        samples[line['tick']] = line
    return lines[0], samples


def record_waits(directory):
    record = json.loads((directory / 'moving-route.record.json').read_text())
    return {'waits': record['slot_waits'], 'max_ms': record['slot_wait_max_ms']}


def moving_chrome(run, name, source, wasm, last):
    with open(run / f'{name}.log', 'w') as log:
        code = bounded(['node', str(CHECKER), '--case', 'moving-route', '--cook', str(COOK), '--web-root', str(WEB),
                        '--assets-source', str(source), '--output', str(run / name)], log, 600, cwd=REPO)
    if code:
        raise Failure(f'checker --case moving-route failed (see {run / f"{name}.log"})')
    result = json.loads((run / name / 'run.json').read_text())['results'][0]
    if result['actualWasmSha256'] != wasm:
        raise Failure(f'{name}: Chrome loaded Wasm {result["actualWasmSha256"]}, not the build')
    record = json.loads((run / name / 'moving-route.record.json').read_text())
    steps = {step['tick']: step for step in record['steps']}
    samples = {}
    for tick, sample in record['samples'].items():
        tick = int(tick)
        # Ticks after the route's last key are drain steps that collected its last samples.
        if tick in steps and tick <= last:
            samples[tick] = dict(sample, committed_solar_bits=steps[tick]['committed_solar_bits'])
    return record['header'], samples, steps


def moving_controls(run, source, route_file, wasm, native, origin, last):
    """Negative controls: Chrome page refusals, a Chrome-only phase the comparator must reject, and bad route files."""
    controls = {}
    with open(run / 'chrome-controls.log', 'w') as log:
        code = bounded(['node', str(CHECKER), '--case', 'moving-route-controls', '--cook', str(COOK), '--web-root', str(WEB),
                        '--assets-source', str(source), '--output', str(run / 'chrome-controls')], log, 900, cwd=REPO)
    if code:
        raise Failure(f'checker --case moving-route-controls failed (see {run / "chrome-controls.log"})')
    for result in json.loads((run / 'chrome-controls' / 'run.json').read_text())['results']:
        if result['actualWasmSha256'] not in (wasm, None):
            raise Failure(f'{result["case"]}: Chrome loaded Wasm {result["actualWasmSha256"]}, not the build')
        controls[result['case']] = result['route'].get('error') or 'completed'
    # Phase 0.1 on Chrome only: the comparator must reject every routed tick's alpha.
    record = json.loads((run / 'chrome-controls' / 'moving-route-phi.record.json').read_text())
    rejected = sum(1 for tick in range(origin, last + 1)
                   if record['samples'].get(str(tick), {}).get('alpha_bits') != native[tick]['alpha_bits'])
    if record['header']['phase'] == route_file['phase'] or rejected != last - origin + 1:
        raise Failure(f'phase 0.1 control: header {record["header"]["phase"]}, {rejected} of {last - origin + 1} ticks rejected')
    controls['moving-route-phi'] = f'header phase {record["header"]["phase"]}; alpha rejected at {rejected} of {last - origin + 1} ticks'
    # The image side, reported per checkpoint: a 0.4-tick camera offset may sit below the cell metric where the
    # camera moves slowly, so only the alpha comparison is required to reject it.
    controls['moving-route-phi-images'] = {
        checkpoint['name']: difference(grid(run / 'chrome-controls/moving-route-phi-checkpoints' / f'{checkpoint["name"]}.png'),
                                       grid(run / 'native-1/checkpoints' / f'{checkpoint["name"]}.ppm'))
        for checkpoint in route_file['checkpoints']}
    # Bad route files on the native driver.
    for name, change, refusal in [
            ('native-generation', lambda r: r.update(cooked_generation=2), 'is not the route\'s 2'),
            ('native-frame-cap', lambda r: r['keys'][-1].update(tick=600), 'frame cap'),
            ('native-yaw-step', lambda r: r['keys'][1].update(yaw_degrees=200.0), 'yaw step over'),
    ]:
        bad = json.loads(MOVING_ROUTE.read_text())
        change(bad)
        path = run / f'{name}.json'
        write_json(path, bad)
        controls[name] = moving_native(run, name, run / 'store', path, refusal)
    return controls


def moving(args):
    os.environ.setdefault('THRUM_RECONSTRUCTION', 'taau')
    os.environ.setdefault('THRUM_RECIPE', 'portable')
    route_file = json.loads(MOVING_ROUTE.read_text())
    origin, last = route_file['origin_tick'], route_file['origin_tick'] + route_file['keys'][-1]['tick']
    names = [checkpoint['name'] for checkpoint in route_file['checkpoints']]
    run = new_run('moving')
    report = {'command': 'moving', 'route': str(MOVING_ROUTE.relative_to(REPO)), 'route_sha256': sha256(MOVING_ROUTE),
              'comparator': COMPARATOR, 'passed': False, 'failures': [],
              'labels': {'native': 'native submitted SDR output', 'chrome': 'Chrome presented canvas'}}
    report['build_seconds'] = None if args.no_build else build(run, web=True)
    report['builds'] = identities(web=True, built=not args.no_build)
    write_json(run / 'run.json', report)
    failures = report['failures']
    try:
        source = copy_source(run)
        with open(run / 'cook.log', 'w') as log:
            if bounded([str(COOK), 'cook', str(source), str(run / 'store')], log, 120):
                raise Failure('thrum-cook cook failed')
        root = report['generation_1_root'] = store_root(run / 'store')
        if root != route_file['cooked_root']:
            raise Failure(f'the cooked root {root} is not the route\'s {route_file["cooked_root"]}')
        targets = {}
        for name in ['native-1', 'native-2']:
            header, samples = moving_native(run, name, run / 'store')
            targets[name] = {'header': header, 'samples': samples,
                             'images': {n: run / name / 'checkpoints' / f'{n}.ppm' for n in names}}
        for name in ['chrome-1', 'chrome-2']:
            header, samples, steps = moving_chrome(run, name, source, report['builds']['wasm_sha256'], last)
            targets[name] = {'header': header, 'samples': samples,
                             'images': {n: run / name / 'moving-route-checkpoints' / f'{n}.png' for n in names},
                             'slot_waits': record_waits(run / name)}
            if not (run / name / 'store/cas' / root[:2] / root).is_file():
                failures.append(f'same cooked input: generation-1 root {root} is absent from the {name} store')
        report['chrome_slot_waits'] = {name: targets[name]['slot_waits'] for name in ['chrome-1', 'chrome-2']}
        report['controls'] = moving_controls(run, source, route_file, report['builds']['wasm_sha256'], targets['native-1']['samples'],
                                             origin, last)

        # Extent and phase identity.
        for name, target in targets.items():
            header = target['header']
            if header.get('reconstruction') != 'Taau' or (name.startswith('native') and header.get('recipe') != 'Ok(Portable)'):
                failures.append(f'{name}: expected prepared TAAU and forced portable recipe, got {header}')
            if header['render_extent'] != route_file['render_extent'] or header['phase'] != route_file['phase'] \
                    or header['origin_tick'] != origin:
                failures.append(f'{name} header differs from the route: {header}')
            ticks = sorted(target['samples'])
            if ticks != list(range(origin, last + 1)):
                failures.append(f'{name} routed ticks are not {origin}..{last}: {len(ticks)} logged')

        # Per-tick identity: every target against native-1, every field bit for bit.
        reference = targets['native-1']['samples']
        mismatches = report['identity_mismatches'] = {}
        for name, target in targets.items():
            for tick in range(origin, last + 1):
                got, want = target['samples'].get(tick), reference.get(tick)
                if got is None or want is None:
                    continue
                for key in MOVING_IDENTITY:
                    if got.get(key) != want.get(key):
                        mismatches.setdefault(name, []).append({'tick': tick, 'field': key, 'got': got.get(key), 'want': want.get(key)})
        for name, found in mismatches.items():
            failures.append(f'{name}: {len(found)} per-tick identity mismatches against native-1, first {found[0]}')
        report['identity'] = {'fields': MOVING_IDENTITY, 'ticks': last - origin + 1, 'targets': sorted(targets)}

        # Images: repeat runs per target, then Chrome against native under the 32x18 cell metric, labelled.
        images = report['checkpoints'] = {}
        for checkpoint in names:
            entry = images[checkpoint] = {}
            for name, target in targets.items():
                path = target['images'][checkpoint]
                entry[name] = {'file': str(path.relative_to(run)), 'sha256': sha256(path)}
            for kind in ['native', 'chrome']:
                entry[f'{kind}_repeat_identical'] = entry[f'{kind}-1']['sha256'] == entry[f'{kind}-2']['sha256']
                if not entry[f'{kind}_repeat_identical']:
                    entry[f'{kind}_repeat'] = difference(grid(targets[f'{kind}-1']['images'][checkpoint]),
                                                         grid(targets[f'{kind}-2']['images'][checkpoint]))
            entry['chrome_vs_native'] = difference(grid(targets['chrome-1']['images'][checkpoint]),
                                                   grid(targets['native-1']['images'][checkpoint]))
            if any(value > limit for value, limit in zip(entry['chrome_vs_native'], THRESHOLDS)):
                failures.append(f'checkpoint {checkpoint}: Chrome presented canvas vs native submitted SDR output '
                                f'{entry["chrome_vs_native"]} exceeds {THRESHOLDS}')
            for kind in ['native', 'chrome']:
                repeat = entry.get(f'{kind}_repeat')
                if repeat and any(value > limit for value, limit in zip(repeat, THRESHOLDS)):
                    failures.append(f'checkpoint {checkpoint}: {kind} repeat runs differ by {repeat}')
        report['scope'] = ('Moving proof route on the cooked valley: per-tick identity of the actual authored camera, phase and '
                           'solar hour on both targets, and checkpoint images under their actual labels. Chrome images are '
                           'compositor captures of the presented canvas, not the submitted pre-display image. No timing, '
                           'cadence or display claim.')
        report['passed'] = not failures
    except Failure as error:
        failures.append(str(error))
    finally:
        write_json(run / 'run.json', report)
    print(f'{"PASS" if report["passed"] else "FAIL"}: {run / "run.json"}')
    for failure in failures:
        print(f'  {failure}')
    if failures:
        raise SystemExit(1)


def metal_exit_oracle(run, recipe, env, route, command):
    """Run complete fresh worlds in order, retaining one lossless Direct stream per recipe."""
    origin = route['origin_tick']
    ticks = list(range(origin, origin + route['keys'][-1]['tick'] + 1))
    expected_recipe = {'portable': 'Ok(Portable)', 'apple-tile': 'Ok(AppleTile)'}[recipe]
    output = run / recipe
    output.mkdir()
    arms = {}
    with tempfile.TemporaryDirectory(prefix='reference-', dir=output) as scratch:
        reference = Path(scratch) / 'direct.rgba'
        for name in ('Direct', 'Frustum', 'Occlusion', 'DirectControl'):
            arm = output / name
            log_path = output / f'{name}.log'
            arm_env = dict(env, THRUM_CULLING_EXIT_ARM=name, THRUM_CULLING_EXIT_REFERENCE=str(reference),
                           THRUM_CULLING_OUTPUT=str(arm))
            started = time.monotonic()
            with log_path.open('w') as log:
                if bounded(command, log, 1200, env=arm_env, cwd=REPO):
                    raise Failure(f'{recipe} {name} oracle failed: {log_path}')
            rows = [json.loads(line) for line in (arm / 'rows.jsonl').read_text().splitlines()]
            header, result = rows[0], rows[-1]
            if header['recipe'] != expected_recipe:
                raise Failure(f'{recipe} {name}: the Metal oracle ran {header["recipe"]}')
            samples = rows[1:-1]
            if (result.get('equal') is not True or result.get('frames') != len(ticks)
                    or result.get('compared') != (0 if name == 'Direct' else len(ticks))
                    or [row['tick'] for row in samples] != ticks
                    or any(row['mode'] != name for row in samples)):
                raise Failure(f'{recipe} {name}: incomplete submitted route')
            arms[name] = {'header': header, **result, 'elapsed_seconds': time.monotonic() - started}
            if name in ('Frustum', 'Occlusion'):
                if any((row['counts'] or {}).get('active') != 10_000 for row in samples):
                    raise Failure(f'{recipe} {name}: incomplete exit population')
                caster_passes = sum('synthetic Sun caster cull' in (row['pass_gpu_ns'] or {}) for row in samples)
                if caster_passes != len(ticks) or not any(result['hidden_ticks']):
                    raise Failure(f'{recipe} {name}: missing Sun-caster sample or no hidden work')
                arms[name]['sun_caster_cull_samples'] = caster_passes
    return {'compared': len(ticks), 'equal': True, 'differing_ticks': 0, 'arms': arms}


def culling(args):
    """Run the existing mode oracles with the selected moving route and cooked scene."""
    scene = 'exit' if args.exit else 'valley'
    route_path = EXIT_FLYTHROUGH if args.exit else MOVING_ROUTE
    route_file = json.loads(route_path.read_text())
    expected_frames = route_file['keys'][-1]['tick'] + 1
    expected_bytes = math.prod(route_file['render_extent']) * 4
    run = new_run(f'culling-{args.target}-{"exit" if args.exit else "meshes" if args.meshes else "cubes"}')
    report = {'command': 'culling', 'target': args.target, 'meshes': args.meshes,
              'scene': scene, 'commit': git('rev-parse', 'HEAD'), 'route_sha256': sha256(route_path),
              'passed': False}
    print(run, flush=True)
    write_json(run / 'run.json', report)
    try:
        if not args.no_build:
            build(run, web=args.target == 'chrome-webgpu')
        report['builds'] = identities(web=args.target == 'chrome-webgpu', built=not args.no_build)
        source = copy_source(run, exit_assets=EXIT_SOURCE) if args.exit else SOURCE
        if args.target == 'mac-metal':
            with open(run / 'cook.log', 'w') as log:
                if bounded([str(COOK), 'cook', str(source), str(run / 'store')], log, 120):
                    raise Failure(f'{scene} cook failed')
            report['cooked_root'] = store_root(run / 'store')
            if report['cooked_root'] != route_file['cooked_root']:
                raise Failure(f'the cooked {scene} does not match the moving route root')
            with open(run / 'test-build.log', 'w') as log:
                command = ['cargo', 'test', '--locked', '--offline', '--release', '-p', 'v1-game',
                           '--test', 'valley_culling_gpu', '--no-run']
                if bounded(command, log, 1200, cwd=REPO):
                    raise Failure(f'valley test build failed: {run / "test-build.log"}')
            report['oracle_binaries'] = {str(path.relative_to(REPO)): sha256(path)
                                        for path in (REPO / 'target/release/deps').glob('valley_culling_gpu-*')
                                        if path.is_file() and os.access(path, os.X_OK)}
            report['recipes'] = {}
            command = ['cargo', 'test', '--locked', '--offline', '--release', '-p', 'v1-game',
                       '--test', 'valley_culling_gpu', '--', '--ignored', '--nocapture']
            for recipe in ('portable', 'apple-tile'):
                output = run / recipe
                env = dict(os.environ, THRUM_SCENE=scene, THRUM_ASSET_STORE=str(run / 'store'),
                           THRUM_CAPTURE_ROUTE=str(route_path), THRUM_CULLING_OUTPUT=str(output),
                           THRUM_VALLEY_MESHES='on' if args.meshes else 'off', THRUM_RECIPE=recipe,
                           THRUM_CULLING_SOURCE='sdr')
                if args.exit:
                    report['recipes'][recipe] = metal_exit_oracle(run, recipe, env, route_file, command)
                    write_json(run / 'run.json', report)
                    continue
                with open(run / f'{recipe}.log', 'w') as log:
                    if bounded(command, log, 1200, env=env, cwd=REPO):
                        raise Failure(f'{recipe} oracle failed: {run / f"{recipe}.log"}')
                rows = [json.loads(line) for line in (output / 'rows.jsonl').read_text().splitlines()]
                if rows[-1].get('compared') != expected_frames or rows[-1].get('equal') is not True:
                    raise Failure(f'{recipe}: incomplete route')
                report['recipes'][recipe] = {'header': rows[0], **rows[-1]}
        else:
            report['modes'] = {}
            for mode in ('direct', 'default', 'frustum', 'occlusion'):
                output = run / mode
                env = dict(os.environ, THRUM_SCENE=scene, THRUM_ROUTE_CULLING=mode,
                           THRUM_VALLEY_MESHES='on' if args.meshes else 'off', THRUM_CULLING_PROOF='on')
                with open(run / f'{mode}.log', 'w') as log:
                    if bounded(['node', str(CHECKER), '--case', 'moving-route', '--cook', str(COOK),
                                '--web-root', str(WEB), '--assets-source', str(source),
                                '--output', str(output)], log, 600, env=env, cwd=REPO):
                        raise Failure(f'Chrome {mode} oracle failed: {run / f"{mode}.log"}')
                record = json.loads((output / 'moving-route.record.json').read_text())
                direct_record = json.loads((run / 'direct/moving-route.record.json').read_text())
                resolved = 'Direct' if mode == 'direct' else 'Occlusion' if mode == 'occlusion' else 'Frustum'
                if f'culling={resolved}' not in record['header']['culling']:
                    raise Failure(f'Chrome {mode} did not resolve to {resolved}')
                report['modes'][mode] = {'resolved': resolved, 'header': record['header'], 'checkpoints': []}
                if args.exit:
                    report['modes'][mode]['exit'] = record['exit']
                    if (record['exit']['row_generation'] != route_file['cooked_generation']
                            or (mode != 'direct' and record['exit']['active'] != 10_000)
                            or record['exit']['mover'] == record['checkpoints'][0]['mover']):
                        raise Failure(f'Chrome {mode} did not draw the populated exit with moving entities')
                for index, checkpoint in enumerate(route_file['checkpoints']):
                    name = checkpoint['name']
                    captured = record['checkpoints'][index]
                    oracle = direct_record['checkpoints'][index]
                    if not captured.get('sha256') or captured['bytes'] != expected_bytes or captured['sha256'] != oracle.get('sha256'):
                        raise Failure(f'Chrome {mode} submitted bytes differ from Direct at {name}')
                    image = read_png(output / 'moving-route-checkpoints' / f'{name}.png')
                    direct = read_png(run / 'direct/moving-route-checkpoints' / f'{name}.png')
                    if image != direct:
                        raise Failure(f'Chrome {mode} differs from Direct at {name}')
                    report['modes'][mode]['checkpoints'].append(captured if args.exit else name)
        report['passed'] = True
    except Exception as error:
        report['error'] = str(error)
        raise
    finally:
        write_json(run / 'run.json', report)
        print(f'{"PASS" if report["passed"] else "FAIL"}: {run / "run.json"}')


def reconstruction(args):
    """Independent held-state reference on the submitted portable game path."""
    from reconstruction_difference import compare, self_check
    from statistics import median
    self_check()
    run = new_run('reconstruction')
    report = {'command': 'reconstruction', 'target': args.target, 'extent': [640,360],
              'commit': git('rev-parse','HEAD'), 'passed': False, 'cut': args.cut,
              'metric': 'whole-frame submitted 8-bit SDR CIEDE2000, D65, nearest-rank p99',
              'tolerance': 3, 'checkpoints': {}, 'timing': {}, 'load_start': os.getloadavg(),
              'arm_order': ['reference','taau','off','broken'],
              'timing_scope': '640x360 held showcase checkpoints; paused world; capture on in both paired arms; first 16 frames excluded; registered GPU jobs serialized by caller; shared host load',
              'design_1080p_estimate_ms': [0.2,0.5]}
    try:
        if not args.no_build:
            build(run, web=False)
        with open(run/'cook.log','w') as log:
            if bounded([str(COOK),'cook',str(SOURCE),str(run/'store')],log,120):
                raise Failure('valley cook failed')
        report['cooked_root'] = store_root(run/'store')
        env = dict(os.environ, THRUM_SCENE='valley', THRUM_RECIPE='portable',
                   THRUM_ASSET_STORE=str(run/'store'), THRUM_RECONSTRUCTION_OUTPUT=str(run/'frames'))
        with open(run/'oracle.log','w') as log:
            if bounded(['cargo','test','--locked','--offline','--release','-p','v1-game','--lib',
                        'reconstruction_reference_on_submitted_game_frames','--','--ignored'],log,1800,
                       env=env,cwd=REPO/'examples/v1-game'):
                raise Failure(f'game reconstruction oracle failed: {run / "oracle.log"}')
        report['device'] = json.loads((run/'frames/build.json').read_text())
        executable = Path(report['device']['executable'])
        report['oracle_binaries'] = {str(executable.relative_to(REPO)):sha256(executable)}
        for name in ROUTE:
            directory = run/'frames'/name
            frames = [json.loads(line) for line in (directory/'frames.jsonl').read_text().splitlines()]
            held = {(row['state'],json.dumps(row['celestial']),tuple(row['exposure'][:4]),row['cooked_generation']) for row in frames}
            if len(held) != 1 or len(frames) != 4*64:
                raise Failure(f'{name}: reference and judged frames did not hold the same state')
            pair = report['timing'][name] = {}
            for mode in ('taau','off'):
                samples = [row['passes'] for row in frames if row['mode'] == mode and row['frame'] >= 16]
                if any(sample is None for sample in samples):
                    pair[mode] = {'available':False}
                    continue
                totals = [sum(p['ns'] for p in sample if p['name'] != 'V1 surface present') for sample in samples]
                resolve = [p['ns'] for sample in samples for p in sample if p['name'] == 'portable TAAU']
                pair[mode] = {'available':True,'frames':len(samples),'scene_p50_ns':median(totals),
                              'taau_passes':len(resolve),'taau_p50_ns':median(resolve) if resolve else None}
            if pair['taau'].get('taau_p50_ns') is not None:
                heuristic = pair['taau']['taau_p50_ns']*9/1e6
                pair['pixel_ratio_1080p_heuristic_ms'] = heuristic
                pair['tripwire_only_outside_estimate'] = not .2 <= heuristic <= .5
            def pixels(mode, frame=64):
                path = directory/f'{mode}-{frame}.ppm'
                raw = path.read_bytes()
                # The oracle writes a fixed three-line P6 header.
                header = f'P6\n640 360\n255\n'.encode()
                if not raw.startswith(header) or len(raw)-len(header) != 640*360*3:
                    raise Failure(f'invalid full-frame image: {path}')
                return raw[len(header):]
            reference = pixels('reference')
            linear_cycles = json.loads((directory/'linear-cycles.json').read_text())
            if not linear_cycles['within_float_rounding']:
                raise Failure(f'{name}: submitted linear cycle averages differ')
            row = report['checkpoints'][name] = {mode:compare(pixels(mode),reference) for mode in ('taau','off','broken')}
            row['linear_cycles'] = dict(linear_cycles, hashes={str(frame):sha256(directory/f'reference-linear-{frame}.f32le') for frame in (32,64)})
            row['reference_sdr_cycle_equal'] = pixels('reference',32) == reference
            row['pass'] = row['taau']['p99'] <= 3 and row['taau']['p99'] < row['off']['p99']
            row['broken_rejected'] = not (row['broken']['p99'] <= 3 and row['broken']['p99'] < row['off']['p99'])
        report['passed'] = all(row['pass'] and row['broken_rejected'] for row in report['checkpoints'].values())
        report['cut_comparison'] = json.loads((run/'frames/cut.json').read_text())
        report['passed'] &= report['cut_comparison']['cut_equals_reset'] and report['cut_comparison']['suppressed_differs']
        if not report['passed']:
            raise Failure('reconstruction tolerance or history-disabled negative control failed')
    except Exception as error:
        report['error'] = str(error)
        raise
    finally:
        report['load_end'] = os.getloadavg()
        write_json(run/'run.json',report)
        print(f'{"PASS" if report["passed"] else "FAIL"}: {run / "run.json"}')


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    commands = parser.add_subparsers(dest='command', required=True)
    commands.add_parser('layout', help='verify the reproducible exit layout against the shipped terrain')
    fixed = commands.add_parser('ticks', help='record the Metal exit route in fixed mode')
    fixed.add_argument('--route', choices=('flythrough', 'held'), default='flythrough')
    fixed.add_argument('--window', nargs=2, action='append', type=int, default=[], metavar=('FIRST', 'LAST'),
                       help='retain every submission in this inclusive route-tick window (repeatable)')
    fixed.add_argument('--phase', type=float, help='override the route presentation phase')
    fixed.add_argument('--no-build', action='store_true', help='use an existing native build for diagnostics')
    motion_command = commands.add_parser('motion', help='record paired real-time exit motion tripwires')
    motion_command.add_argument('--check', action='store_true', help='check the retained record without building')
    exit_command = commands.add_parser('exit', help='run the exit scene checks')
    exit_mode = exit_command.add_mutually_exclusive_group(required=True)
    exit_mode.add_argument('--controls', action='store_true', help='check the exit controls and presets')
    exit_mode.add_argument('--clauses', action='store_true', help='check the exit time, content and profiler clauses')
    exit_mode.add_argument('--row', choices=['mac-apple', 'mac-portable', 'chrome'], help='run one exit row through the stops and flythrough')
    exit_mode.add_argument('--compare', action='store_true', help='compare the three submitted exit rows')
    exit_mode.add_argument('--calibrate-comparison', action='store_true',
                           help='stage a first three-row baseline after the repeat and controls pass')
    exit_command.add_argument('--refresh-baseline', metavar='CAUSE',
                              help='refresh reviewed grids after parity and controls pass')
    exit_command.add_argument('--refresh-row', choices=['mac-apple', 'mac-portable', 'chrome'],
                              help='refresh only this row and check the other rows against their retained grids')
    exit_command.add_argument('--baseline', help='comparison baseline path (defaults to the checked-in fixture)')
    exit_command.add_argument('--target', choices=['mac-metal', 'chrome-webgpu'])
    frames_command = commands.add_parser('frames', help='record whole exit frames with capture off')
    frames_command.add_argument('--target', required=True, choices=['mac-metal', 'chrome-webgpu'])
    for name in ('mac', 'chrome', 'check', 'startup', 'moving', 'culling', 'reconstruction'):
        command = commands.add_parser(name)
        command.add_argument('--no-build', action='store_true', help='use the existing release and web builds')
        if name in ('mac', 'chrome'):
            command.add_argument('--stop', choices=ROUTE, default=ROUTE[0])
            command.add_argument('--exit', action='store_true', help='launch the populated exit scene')
    commands.choices['mac'].add_argument('--frames', type=int, help='bound the run (THRUM_V1_FRAMES)')
    commands.choices['culling'].add_argument('--target', required=True, choices=['mac-metal', 'chrome-webgpu'])
    commands.choices['culling'].add_argument('--meshes', action='store_true')
    commands.choices['culling'].add_argument('--exit', action='store_true', help='use the populated exit flythrough')
    commands.choices['reconstruction'].add_argument('--target', required=True, choices=['mac-metal'])
    commands.choices['reconstruction'].add_argument('--cut', action='store_true')
    commands.choices['check'].add_argument('--save-baseline', action='store_true',
                                           help='write the baseline after a passing run (a reviewed, recorded act); '
                                                'drift from the old baseline alone does not block it')
    commands.choices['check'].add_argument('--exit', action='store_true', help='check the populated exit scene')
    commands.choices['check'].add_argument('--refresh-baseline', metavar='CAUSE',
                                           help='refresh the bare-default exit grids; use exit --compare '
                                                '--refresh-baseline for the configured rows')
    commands.choices['check'].add_argument('--baseline', default=str(BASELINE),
                                           help='baseline file to compare with or save, relative to the current directory '
                                                '(default: the checked-in one)')
    commands.choices['check'].add_argument('--source', default=str(SOURCE),
                                           help='valley source directory to cook and check (default: the checked-in one)')
    commands.choices['check'].add_argument('--recalibrate', action='store_true',
                                           help='judge this run by the calibration rule instead of the fixed thresholds '
                                                '(another source root); cannot save the canonical baseline')
    commands.choices['startup'].add_argument('--runs', type=int, default=5, help='fresh Chrome launches (default 5)')
    commands.choices['chrome'].add_argument('--no-open', action='store_true', help='print the URL instead of opening Chrome')
    args = parser.parse_args()
    if args.command == 'exit' and bool(args.target) != args.clauses:
        parser.error('exit --clauses requires --target')
    # A terminated launcher stops its watcher, servers and game the same way Ctrl-C does.
    signal.signal(signal.SIGTERM, lambda *_: (_ for _ in ()).throw(KeyboardInterrupt))
    try:
        {'mac': mac, 'chrome': chrome, 'check': check, 'startup': startup, 'moving': moving, 'culling': culling,
         'reconstruction': reconstruction, 'layout': layout, 'ticks': ticks, 'motion': motion,
         'exit': exit_showcase, 'frames': exit_frames}[args.command](args)
    except (Failure, OSError, ValueError, KeyError, TypeError, StopIteration) as error:
        print(f'FAIL: {error}', file=sys.stderr)
        raise SystemExit(1)


if __name__ == '__main__':
    main()
