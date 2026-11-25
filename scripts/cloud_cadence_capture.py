"""Collect one complete Mac wind series and its optional timing diagnostics."""
from collections import defaultdict
from datetime import datetime, timezone
from decimal import Decimal
import gzip
import json
import os
import plistlib
from pathlib import Path
import re
import select
import shutil
import subprocess
import tempfile
import time

from capture_v1_runtime import digest
from cloud_cadence import ARMS, HZ, InvalidMeasurement, NS, PERIOD, analyze_arm, differences, interval
from mac_m2_presented import table
from qualify_mac_m2 import ROOT, bounded, panel_witness, save
from t2_load import LoadRecord

WORKING = ROOT / '.orchestrate/evidence/CLOUDS'
SCRATCH = ROOT / 'tmp/clouds-wp19'
BIN = ROOT / 'target/release/v1-game'
INSTRUMENT = ROOT / 'scripts/host_witness/presented_time.instrpkg'
INSTRUMENT_PACKAGE = SCRATCH / 'ThrumPresentedTime.instrdst'
TRACE_TEMPLATE = SCRATCH / 'PresentedTime.tracetemplate'
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
    prepare_instrument()
    return store


def instrument_packages():
    developer = subprocess.check_output(['xcode-select', '-p'], text=True, timeout=5).strip()
    return Path(developer).parent / 'Applications/Instruments.app/Contents/Packages'


def prepare_instrument():
    packages = instrument_packages()
    subprocess.run(['xcrun', 'instrumentbuilder', '-L', packages, '-l', 'GPU',
                    '-i', INSTRUMENT, '-o', INSTRUMENT_PACKAGE],
                   cwd=ROOT, check=True, timeout=30)
    blank = packages / 'Base.instrdst/Contents/Templates/Blank.tracetemplate'
    archive = plistlib.loads(blank.read_bytes())
    objects = archive['$objects']
    command = objects[archive['$top']['templateRunCommand'].data]
    configuration = objects[command['$3'].data]
    if not configuration['supportsImmediateMode'] or configuration['recordingMode'] != 2:
        raise InvalidMeasurement('unrecognized Xcode Blank trace recording configuration')
    # Use the installed template with Immediate mode and no rolling window.
    configuration['recordingMode'] = 1
    TRACE_TEMPLATE.write_bytes(plistlib.dumps(archive, fmt=plistlib.FMT_BINARY, sort_keys=False))


def console_unlocked():
    raw = subprocess.check_output(['/usr/sbin/ioreg', '-n', 'Root', '-d1'], text=True, timeout=3)
    return '"IOConsoleLocked" = No' in raw and '"CGSSessionScreenIsLocked"=Yes' not in raw


def export(trace, output, arguments):
    command = ['xcrun', 'xctrace', 'export', '--input', str(trace), '--output', str(output), *arguments]
    if bounded(command, output.with_suffix('.export.log'), 40) or not output.is_file():
        raise InvalidMeasurement(f'Instruments export failed for {output.name}')


def discard_kernel_scratch(arm, pid, began, ended):
    temporary = Path(tempfile.gettempdir())
    for path in temporary.glob('instruments*.ktrace'):
        try:
            birth = path.stat().st_birthtime
        except FileNotFoundError:
            continue
        if not began <= birth <= ended:
            continue
        # Identify the recorder's scratch by the owned game's PID in the
        # raw event stream, beyond the creation time and filename pattern.
        command = ['/usr/bin/ktrace', 'trace', '-R', path, '-p', str(pid), '--ndjson']
        sample = bounded_trace_identity(command)
        if sample and sample['pid'] == pid and sample['execname'] == 'v1-game':
            save(arm / 'kernel-scratch.json', {
                'path': str(path), 'bytes': path.stat().st_size,
                'game_pid': pid, 'trace_event_time': sample['walltime'],
            })
            path.unlink()


def bounded_trace_identity(command):
    # Stop at the first event of the requested process, including when
    # other users' trace files happened to start at the same time.
    try:
        with subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
                              text=True) as reader:
            try:
                ready, _, _ = select.select([reader.stdout], [], [], 6)
                line = reader.stdout.readline() if ready else ''
                return json.loads(line) if line else None
            finally:
                if reader.poll() is None:
                    reader.terminate()
    except (OSError, ValueError):
        return None


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
    command = []
    if diagnostic:
        settings['THRUM_MAC_CADENCE_TIMING'] = '1'
        env.update(settings)
        command += [str(BIN)]
    else:
        options = arm / 'recording-options.json'
        save(options, {'Points of Interest': {'excludeOSLogs': True}})
        command += ['xcrun', 'xctrace', 'record', '--no-prompt', '--template', str(TRACE_TEMPLATE),
                    '--recording-options', str(options), '--package', str(INSTRUMENT_PACKAGE),
                    '--instrument', 'Thrum Presented Time', '--instrument', 'Points of Interest',
                    '--time-limit', '36s', '--output', str(trace),
                    '--target-stdout', str(game)]
        for key, value in settings.items():
            command += ['--env', f'{key}={value}']
        command += ['--launch', '--', str(BIN)]
    save(arm / 'launch.json', {'command': command, 'settings': settings, 'diagnostic': diagnostic})
    # Wake before the panel preflight and release the display assertion when
    # the game exits. Export and Instruments finalization need no active panel.
    recorder = game if diagnostic else arm / 'recorder.log'
    power = subprocess.Popen(['/usr/bin/caffeinate', '-d', '-i', '-u'])
    load = None
    began = time.time()
    try:
        observe_panel = panel_witness(arm)

        def observe():
            if power.poll() is not None:
                return
            if (arm / 'refresh-counts.txt').exists() or (
                not diagnostic and recorder.exists() and 'ending recording' in recorder.read_text(errors='replace')
            ):
                power.terminate()
            else:
                observe_panel()

        observe_panel()
        load = LoadRecord(arm)
        code = bounded(command, recorder, 155 if not diagnostic else 33, environment=env,
                       observe=observe, finalization_seconds=None if diagnostic else 125)
        if code or not (arm / 'refresh-account.tsv').is_file():
            raise InvalidMeasurement(f'{arm.name}: window did not finish the cadence record (exit {code})')
    finally:
        if power.poll() is None:
            power.terminate()
        power.wait(timeout=3)
        if load:
            save(arm / 'load.json', load.finish())

    ended = time.time()
    identity = (arm / 'window.txt').read_text()
    if not re.search(r'^extent=1920x1080$', identity, re.MULTILINE) or f'Some({HZ}000)))' not in identity:
        raise InvalidMeasurement(f'{arm.name}: the window was not 1920x1080 at {HZ} Hz')
    if diagnostic:
        return
    export(trace, arm / 'toc.xml', ['--toc'])
    toc = (arm / 'toc.xml').read_text()
    exported = []
    for filename, predicate in TRACE_TABLES.items():
        schemas = re.findall(r'@schema="([^"]+)"', predicate)
        if schemas and not any(f'schema="{schema}"' in toc for schema in schemas):
            continue
        export(trace, arm / filename, ['--xpath', f'/trace-toc/run/data/table[{predicate}]'])
        exported.append(filename)
    result = read_witness(arm, wind)
    for filename in ('toc.xml', *exported):
        source = arm / filename
        with source.open('rb') as original, gzip.open(source.with_suffix('.xml.gz'), 'wb') as retained:
            shutil.copyfileobj(original, retained)
        source.unlink()
    # The successful export retains the OS rows, signposts and display tables.
    # Keep an unsuccessful trace for setup diagnosis.
    shutil.rmtree(trace)
    discard_kernel_scratch(arm, int(re.search(r'^pid=(\d+)$', identity, re.MULTILINE)[1]),
                           began, ended)
    return result


def read_witness(arm, wind):
    table_path = arm / 'presented.xml'
    if not table_path.is_file():
        table_path = table_path.with_suffix('.xml.gz')
    witness = ['python3', str(ROOT / 'scripts/mac_m2_presented.py'), '--input', str(table_path),
               '--arm', str(arm), '--output', str(arm / 'witness.json'), '--measured-interval']
    # The witness's qualification also asks for no unmatched rows. The wind
    # record explicitly retains unmatched submissions and uses only valid times.
    if bounded(witness, arm / 'witness.log', 40) not in (0, 2) or not (arm / 'witness.json').is_file():
        raise InvalidMeasurement(f'{arm.name}: presented-time witness did not produce a result')
    return analyze_arm(arm, wind)


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


def series(output, *, diagnostics, resume=False):
    store = prepare()
    output.mkdir(parents=True, exist_ok=resume)
    head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
    dirty = subprocess.check_output(['git', 'status', '--porcelain'], cwd=ROOT, text=True).splitlines()
    destination = output / 'series.json'
    identity = {
        'binary_sha256': digest(BIN), 'cooked_head_sha256': digest(store / 'HEAD'),
        'instrument_sha256': digest(INSTRUMENT),
    }
    if resume:
        record = json.loads(destination.read_text())
        if any(record.get(key) != value for key, value in identity.items()):
            raise InvalidMeasurement('a resumed series must use the same game, content and instrument')
        if len(record['runs']) == len(ARMS):
            raise InvalidMeasurement('all the wind arms are already recorded')
        record.setdefault('collection_revisions', [record['revision']]).append(head)
        record.pop('setup_failure', None)
    else:
        record = {
            'revision': head, **identity, 'started': datetime.now(timezone.utc).isoformat(),
            'dirty': dirty, 'xcode': subprocess.check_output(
                ['xcodebuild', '-version'], cwd=ROOT, text=True).splitlines(),
            'runs': [], 'cloud_runs': [],
        }
    try:
        for index in range(len(record['runs']), len(ARMS)):
            wind = ARMS[index]
            name = f'{index:02d}-{wind[0]}'
            arm = output / name
            if resume and arm.exists():
                print(f'{name}: validating the retained recording', flush=True)
                run = read_witness(arm, wind)
            else:
                print(f'{name}: recording the built-in panel', flush=True)
                run = collect(arm, store, wind)
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


def run_wind(command, output, *, resume=False):
    stamp = datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%S.%fZ')
    work_package = 'WP19' if command == 'cadence' else 'WP20'
    output = (output or WORKING / work_package / f'{command}-{stamp}').resolve()
    record = series(output, diagnostics=command == 'cadence', resume=resume)
    print(output / 'series.json')
    for run in record['cloud_runs']:
        print(f'{run["arm"]}: {run["holds"]=}, {run["neighbours_lost"]=}')
    return 1 if command == 'hold' and any(not run['holds'] for run in record['cloud_runs']) else 0
