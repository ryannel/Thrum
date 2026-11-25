"""Collect one complete Mac wind series and its optional timing diagnostics."""
from collections import defaultdict
from datetime import datetime, timezone
from decimal import Decimal
import gzip
import json
import os
from pathlib import Path
import re
import shutil
import subprocess

from capture_v1_runtime import digest
from cloud_cadence import ARMS, HZ, InvalidMeasurement, NS, PERIOD, analyze_arm, differences, interval
from mac_m2_presented import table
from qualify_mac_m2 import ROOT, bounded, panel_witness, save
from t2_load import LoadRecord

WORKING = ROOT / '.orchestrate/evidence/CLOUDS/WP19'
SCRATCH = ROOT / 'tmp/clouds-wp19'
BIN = ROOT / 'target/release/v1-game'
TRACE_TABLES = {
    'signposts.xml': "contains(@schema, 'signpost')",
    'presented.xml': '@schema="ca-client-presented-handler"',
    'displays.xml': '@schema="device-display-info"',
    'raw-displays.xml': (
        '@schema="display-surface-swap" or @schema="display-vsyncs-interval" or '
        '@schema="display-surface-queue" or @schema="displayed-surfaces-interval"'
    ),
}


def prepare():
    if not console_unlocked():
        raise InvalidMeasurement('local console must be unlocked for the built-in panel')
    subprocess.run(['cargo', 'build', '--release', '--locked', '--offline',
                    '-p', 'v1-game', '--bin', 'v1-game', '-p', 'thrum-cook', '--bin', 'thrum-cook'],
                   cwd=ROOT, check=True, timeout=900)
    SCRATCH.mkdir(parents=True, exist_ok=True)
    store = SCRATCH / 'valley'
    subprocess.run([ROOT / 'target/release/thrum-cook', 'cook',
                    ROOT / 'examples/v1-game/assets/showcase', store],
                   cwd=ROOT, check=True, timeout=180)
    return store


def console_unlocked():
    raw = subprocess.check_output(['/usr/sbin/ioreg', '-n', 'Root', '-d1'], text=True, timeout=3)
    return '"IOConsoleLocked" = No' in raw and '"CGSSessionScreenIsLocked"=Yes' not in raw


def export(trace, output, arguments):
    command = ['xcrun', 'xctrace', 'export', '--input', str(trace), '--output', str(output), *arguments]
    if bounded(command, output.with_suffix('.export.log'), 40) or not output.is_file():
        raise InvalidMeasurement(f'Instruments export failed for {output.name}')


def collect(arm, store, wind, *, diagnostic=False):
    if not console_unlocked():
        raise InvalidMeasurement('local console must be unlocked for the built-in panel')
    arm.mkdir()
    name, speed, scale, turns = wind
    noon = arm / 'noon.params.json'
    save(noon, {'world.paused': False, 'sky.solar_time': 12, 'camera.yaw': 0, 'camera.pitch': 5})
    settings = dict(THRUM_SCENE='valley', THRUM_ASSET_STORE=str(store), THRUM_PARAMS=str(noon),
                    THRUM_CLOUDS='off' if name == 'off' else 'sky', THRUM_CLOUD_PRESET='fair',
                    THRUM_CLOUD_WIND=str(speed), THRUM_CLOUD_TIME_SCALE=str(scale),
                    THRUM_MAC_QUALIFY=str(arm), THRUM_MAC_QUALIFY_MODE='cadence',
                    THRUM_MAC_TAP='off', THRUM_RECIPE='apple-tile', THRUM_RECONSTRUCTION='taau')
    if turns:
        settings['THRUM_CLOUD_SEQUENCE'] = 'wind-turn'
    env = {key: value for key, value in os.environ.items() if not key.startswith('THRUM_')}
    game = arm / 'game.log'
    trace = arm / 'cadence.trace'
    command = ['/usr/bin/caffeinate', '-d', '-i', '-u']
    if diagnostic:
        settings['THRUM_MAC_CADENCE_TIMING'] = '1'
        env.update(settings)
        command += [str(BIN)]
    else:
        developer = subprocess.check_output(['xcode-select', '-p'], text=True, timeout=5).strip()
        blank = Path(developer).parent / (
            'Applications/Instruments.app/Contents/Packages/Base.instrdst/Contents/Templates/Blank.tracetemplate'
        )
        options = arm / 'recording-options.json'
        save(options, {'Points of Interest': {'excludeOSLogs': True}})
        command += ['xcrun', 'xctrace', 'record', '--no-prompt', '--template', str(blank),
                    '--recording-options', str(options), '--instrument', 'Metal Application',
                    '--instrument', 'Display', '--instrument', 'Points of Interest',
                    '--time-limit', '29s', '--output', str(trace), '--target-stdout', str(game)]
        for key, value in settings.items():
            command += ['--env', f'{key}={value}']
        command += ['--launch', '--', str(BIN)]
    save(arm / 'launch.json', {'command': command, 'settings': settings, 'diagnostic': diagnostic})
    # Wake before the panel preflight and hold it only during the recording.
    subprocess.run(['/usr/bin/caffeinate', '-u', '-t', '2'], check=True, timeout=4)
    observe_panel = panel_witness(arm)
    observe_panel()
    load = LoadRecord(arm)
    try:
        code = bounded(command, game if diagnostic else arm / 'recorder.log',
                       100 if not diagnostic else 33, environment=env, observe=observe_panel,
                       finalization_seconds=None if diagnostic else 70)
        if code or not (arm / 'refresh-account.tsv').is_file():
            raise InvalidMeasurement(f'{arm.name}: window did not finish the cadence record (exit {code})')
    finally:
        save(arm / 'load.json', load.finish())

    identity = (arm / 'window.txt').read_text()
    if not re.search(r'^extent=1920x1080$', identity, re.MULTILINE) or f'Some({HZ}000)))' not in identity:
        raise InvalidMeasurement(f'{arm.name}: the window was not 1920x1080 at {HZ} Hz')
    if diagnostic:
        return
    export(trace, arm / 'toc.xml', ['--toc'])
    for filename, predicate in TRACE_TABLES.items():
        export(trace, arm / filename, ['--xpath', f'/trace-toc/run/data/table[{predicate}]'])
    witness = ['python3', str(ROOT / 'scripts/mac_m2_presented.py'), '--input',
               str(arm / 'presented.xml'), '--arm', str(arm), '--output', str(arm / 'witness.json')]
    # The witness's qualification also asks for no unmatched rows. The wind
    # record explicitly retains unmatched submissions and uses only valid times.
    if bounded(witness, arm / 'witness.log', 40) not in (0, 2) or not (arm / 'witness.json').is_file():
        raise InvalidMeasurement(f'{arm.name}: presented-time witness did not produce a result')
    result = analyze_arm(arm, wind)
    for filename in ('toc.xml', *TRACE_TABLES):
        source = arm / filename
        with source.open('rb') as original, gzip.open(source.with_suffix('.xml.gz'), 'wb') as retained:
            shutil.copyfileobj(original, retained)
        source.unlink()
    # The successful export retains the OS rows, signposts and display tables.
    # Keep an unsuccessful trace for setup diagnosis.
    shutil.rmtree(trace)
    return result


def diagnose(arm, entries):
    start, _ = interval(arm)
    by_submission = defaultdict(list)
    for row in table(arm / 'cadence-passes.tsv'):
        by_submission[row['submission_id']].append(
            (row['pass'], float(row['nanoseconds']) / 1e6))
    rows = table(arm / 'refresh-account.tsv')
    for entry in entries:
        if not entry['excess']:
            continue
        if entry['kind'] == 'stall':
            matching = [row for row in rows
                        if row['place'] != 'served' and abs(
                            float((Decimal(row['target_media_seconds']) * NS - start) / NS)
                            - entry['seconds']) <= 1.0]
        else:
            matching = [row for row in rows if row['place'] == entry['place']]
        # The witness is off during this diagnostic. A loss only after the
        # present call therefore cannot be reproduced by this account alone.
        observed = []
        for row in matching:
            passes = sorted(by_submission[row['submission_id'] or row['interval_submission_id']],
                            key=lambda pair: -pair[1])
            observed.append({'target_media_seconds': row['target_media_seconds'],
                             'place': row['place'], 'submission_id': row['submission_id'],
                             'passes_ms': passes})
        entry['diagnostic'] = {
            'arm': arm.name, 'gpu_pass_timing': True, 'period_ms': float(PERIOD / Decimal(1e6)),
            'result': 'not distinguished' if matching or entry.get('place') == 'after_present_call'
            else 'not reproduced',
            'matching_account_rows': observed,
            'scope': 'GPU pass durations and present calls with timestamps enabled; no OS cadence witness',
        }


def series(output, *, diagnostics):
    store = prepare()
    output.mkdir(parents=True, exist_ok=False)
    head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
    dirty = subprocess.check_output(['git', 'status', '--porcelain'], cwd=ROOT, text=True).splitlines()
    record = {'revision': head, 'binary_sha256': digest(BIN), 'started': datetime.now(timezone.utc).isoformat(),
              'dirty': dirty, 'runs': [], 'cloud_runs': []}
    destination = output / 'series.json'
    try:
        for index, wind in enumerate(ARMS):
            name = f'{index:02d}-{wind[0]}'
            print(f'{name}: recording the built-in panel', flush=True)
            run = collect(output / name, store, wind)
            record['runs'].append(dict(arm=name, **run))
            save(destination, record)
        for index in range(1, len(ARMS), 2):
            before, cloud, after = record['runs'][index - 1:index + 2]
            verdict, entries = differences(cloud, before, after)
            record['cloud_runs'].append(dict(arm=cloud['arm'], entries=entries, **verdict))
            if diagnostics and any(entry['excess'] for entry in entries):
                arm = output / f'{cloud["arm"]}-diagnostic'
                print(f'{arm.name}: recording GPU pass timing', flush=True)
                collect(arm, store, ARMS[index], diagnostic=True)
                diagnose(arm, entries)
            save(destination, record)
        if digest(BIN) != record['binary_sha256']:
            raise InvalidMeasurement('the game binary changed while the series was running')
    except Exception as error:
        record['setup_failure'] = str(error)
        raise
    finally:
        save(destination, record)
    return record


def run_wind(command, output):
    stamp = datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%S.%fZ')
    output = (output or WORKING / f'{command}-{stamp}').resolve()
    record = series(output, diagnostics=command == 'cadence')
    print(output / 'series.json')
    for run in record['cloud_runs']:
        print(f'{run["arm"]}: {run["holds"]=}, {run["neighbours_lost"]=}')
    return 1 if command == 'hold' and any(not run['holds'] for run in record['cloud_runs']) else 0
