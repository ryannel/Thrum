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
import shutil
import stat
import subprocess
import sys
import tempfile
import time

from capture_v1_runtime import digest
from cloud_cadence import (
    ARMS, HZ, InvalidMeasurement, NS, OccludedMeasurement, PERIOD, analyze_arm, differences, interval,
)
from mac_m2_presented import table
from qualify_mac_m2 import ROOT, bounded, panel_witness, save
from t2_load import LoadRecord

WORKING = ROOT / '.orchestrate/evidence/CLOUDS'
SCRATCH = ROOT / 'tmp/clouds-wp19'
BIN = ROOT / 'target/release/v1-game'
INSTRUMENT = ROOT / 'scripts/host_witness/presented_time.instrpkg'
INSTRUMENT_PACKAGE = SCRATCH / 'ThrumPresentedTime.instrdst'
TRACE_TEMPLATE = SCRATCH / 'PresentedTime.tracetemplate'


class RecordingTimeout(InvalidMeasurement):
    """Instruments timed out before providing its witness."""


def prepare():
    require_unlocked_panel(wake=True)
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
    return '"IOConsoleLocked" = No' in raw and not re.search(
        r'"CGSSessionScreenIsLocked"\s*=\s*Yes', raw
    )


def require_unlocked_panel(*, wake=False):
    # A sleeping display can set the screen-lock flag even with locking off.
    if wake or not console_unlocked():
        subprocess.run(['/usr/bin/caffeinate', '-u', '-t', '3'], check=True, timeout=6)
    if not console_unlocked():
        raise InvalidMeasurement('local console must be unlocked for the built-in panel')


def export(trace, output, arguments):
    command = ['xcrun', 'xctrace', 'export', '--input', str(trace), '--output', str(output), *arguments]
    if bounded(command, output.with_suffix('.export.log'), 40) or not output.is_file():
        raise InvalidMeasurement(f'Instruments export failed for {output.name}')


def writable_recordings(fields, temporary):
    """Read lsof's NUL-delimited file records for writable kernel traces."""
    records = []
    file = {}
    for field in fields.split('\0'):
        field = field.lstrip('\n')
        if not field:
            continue
        key, value = field[0], field[1:]
        if key in ('p', 'f'):
            records.append(file)
            file = {}
        else:
            file[key] = value
    records.append(file)
    results = set()
    for file in records:
        if file.get('a') not in ('w', 'u') or file.get('t') != 'REG' or not {'D', 'i', 'n'} <= file.keys():
            continue
        path = Path(file['n'])
        if path.parent.resolve() == temporary and re.fullmatch(r'instruments[^/]+\.ktrace', path.name):
            results.add((temporary / path.name, int(file['D'], 0), int(file['i'])))
    return results


def recording_scratch(pid):
    """Observe writes by xctrace and its descendants while they are alive."""
    processes = subprocess.check_output(['/bin/ps', '-axo', 'pid=,ppid='], text=True, timeout=4)
    children = defaultdict(list)
    for child, parent in map(str.split, processes.splitlines()):
        children[parent].append(child)
    pids = [str(pid)]
    for parent in pids:
        pids.extend(children[parent])
    found = subprocess.run(['/usr/sbin/lsof', '-n', '-p', ','.join(pids), '-F0faDint'],
                           text=True, capture_output=True, timeout=5).stdout
    return writable_recordings(found, Path(tempfile.gettempdir()).resolve())


def kernel_recording(candidates):
    if len(candidates) != 1:
        raise InvalidMeasurement('Instruments did not establish one owned kernel recording')
    (recording,) = candidates
    path, device, inode = recording
    try:
        info = path.stat(follow_symlinks=False)
    except FileNotFoundError as error:
        raise InvalidMeasurement('the owned kernel recording is no longer retained') from error
    if not stat.S_ISREG(info.st_mode) or (info.st_dev, info.st_ino) != (device, inode):
        raise InvalidMeasurement('the owned kernel recording path changed identity')
    return path


def collect(arm, store, wind, *, diagnostic=False):
    require_unlocked_panel()
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
                    '--instrument', 'Thrum Presentation Events', '--instrument', 'Points of Interest',
                    '--time-limit', '36s', '--output', str(trace),
                    '--target-stdout', str(game)]
        for key, value in settings.items():
            command += ['--env', f'{key}={value}']
        command += ['--launch', '--', str(BIN)]
    save(arm / 'launch.json', {'command': command, 'settings': settings, 'diagnostic': diagnostic})
    # Wake before the panel preflight and release the display assertion when
    # the game exits. Export and Instruments finalization need no active panel.
    recorder = game if diagnostic else arm / 'recorder.log'
    power = subprocess.Popen(['/usr/bin/caffeinate', '-d', '-i', '-u', '-t', '40'])
    load = None
    recorder_pid = None
    candidates = set()
    next_ownership_probe = 0.0

    def launched(pid):
        nonlocal recorder_pid
        recorder_pid = pid

    try:
        observe_panel = panel_witness(arm)

        def observe():
            nonlocal next_ownership_probe
            if power.poll() is not None:
                return
            if (arm / 'refresh-counts.txt').exists() or (
                not diagnostic and recorder.exists() and 'ending recording' in recorder.read_text(errors='replace')
            ):
                power.terminate()
            else:
                observe_panel()
                if (not diagnostic and not candidates and recorder_pid is not None
                        and (arm / 'window.txt').exists() and time.monotonic() >= next_ownership_probe):
                    next_ownership_probe = time.monotonic() + 2.0
                    try:
                        candidates.update(recording_scratch(recorder_pid))
                    except (OSError, subprocess.SubprocessError):
                        pass

        observe_panel()
        load = LoadRecord(arm)
        code = bounded(command, recorder, 155 if not diagnostic else 33, environment=env,
                       observe=observe, finalization_seconds=None if diagnostic else 125,
                       on_start=launched)
        if code or not (arm / 'refresh-account.tsv').is_file():
            raise InvalidMeasurement(f'{arm.name}: window did not finish the cadence record (exit {code})')
    except RuntimeError as error:
        if diagnostic or not isinstance(error.__cause__, subprocess.TimeoutExpired):
            raise
        try:
            raw = kernel_recording(candidates)
        except InvalidMeasurement:
            pass
        else:
            save(arm / 'interrupted-scratch.json', {'path': str(raw), 'recorder_pid': recorder_pid,
                 'owned_device': raw.stat().st_dev, 'owned_inode': raw.stat().st_ino})
            kernel_recording(candidates).unlink()
        raise RecordingTimeout(f'{arm.name}: {error}') from error
    finally:
        if power.poll() is None:
            power.terminate()
        power.wait(timeout=3)
        if load:
            save(arm / 'load.json', load.finish())

    identity = (arm / 'window.txt').read_text()
    if not re.search(r'^extent=1920x1080$', identity, re.MULTILINE) or f'Some({HZ}000)))' not in identity:
        raise InvalidMeasurement(f'{arm.name}: the window was not 1920x1080 at {HZ} Hz')
    if diagnostic:
        return
    export(trace, arm / 'toc.xml', ['--toc'])
    export(trace, arm / 'signposts.xml',
           ['--xpath', "/trace-toc/run/data/table[contains(@schema, 'signpost')]"])
    export(trace, arm / 'source-presented-events.xml',
           ['--xpath', "/trace-toc/run/data/table[@schema='kdebug' and contains(@codes, '0xca')]"])
    pid = int(re.search(r'^pid=(\d+)$', identity, re.MULTILINE)[1])
    raw = kernel_recording(candidates)

    # The live capture requests only the game's Core Animation events. Ask
    # Instruments to populate its standard presented-handler table offline,
    # so the live observer never records Metal call stacks.
    modeled = arm / 'modeled.trace'
    command = ['xcrun', 'xctrace', 'import', '--input', str(raw), '--output', str(modeled),
               '--template', str(TRACE_TEMPLATE), '--package', str(INSTRUMENT_PACKAGE),
               '--instrument', 'Thrum Presented Time']
    save(arm / 'modeling.json', {'command': command, 'game_pid': pid, 'raw_bytes': raw.stat().st_size,
                                'recorder_pid': recorder_pid, 'owned_device': raw.stat().st_dev,
                                'owned_inode': raw.stat().st_ino})
    if bounded(command, arm / 'modeling.log', 60):
        raise InvalidMeasurement(f'{arm.name}: offline Instruments modeling failed')
    export(modeled, arm / 'presented.xml',
           ['--xpath', '/trace-toc/run/data/table[@schema="ca-client-presented-handler"]'])
    for filename in ('toc.xml', 'signposts.xml', 'source-presented-events.xml', 'presented.xml'):
        source = arm / filename
        with source.open('rb') as original, gzip.open(source.with_suffix('.xml.gz'), 'wb') as retained:
            shutil.copyfileobj(original, retained)
        source.unlink()
    shutil.rmtree(trace)
    shutil.rmtree(modeled)
    kernel_recording(candidates).unlink()
    return read_witness(arm, wind)


def read_witness(arm, wind):
    table_path = arm / 'presented.xml'
    if not table_path.is_file():
        table_path = table_path.with_suffix('.xml.gz')
    witness = ['python3', str(ROOT / 'scripts/mac_m2_presented.py'), '--input', str(table_path),
               '--arm', str(arm), '--output', str(arm / 'witness.json'), '--measured-interval']
    for suffix in ('.xml', '.xml.gz'):
        events = arm / ('source-presented-events' + suffix)
        if events.is_file():
            witness += ['--source-events', str(events)]
            break
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


def save_series(destination, record):
    # A cancelled collector must leave either the last complete receipt or a
    # fully written new one. The per-arm raw exports remain separately usable.
    staged = destination.with_suffix('.next.json')
    save(staged, record)
    staged.replace(destination)


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
            save_series(destination, record)
        for index in range(1, len(ARMS), 2):
            before, cloud, after = record['runs'][index - 1:index + 2]
            verdict, entries = differences(cloud, before, after)
            record['cloud_runs'].append(dict(arm=cloud['arm'], entries=entries, **verdict))
            if diagnostics and any(entry['excess'] for entry in entries):
                arm = output / f'{cloud["arm"]}-diagnostic'
                print(f'{arm.name}: recording GPU pass timing', flush=True)
                collect(arm, store, ARMS[index], diagnostic=True)
                diagnose(arm, entries)
            save_series(destination, record)
        if digest(BIN) != record['binary_sha256']:
            raise InvalidMeasurement('the game binary changed while the series was running')
    except Exception as error:
        record['setup_failure'] = str(error)
        raise
    finally:
        save_series(destination, record)
    return record


def run_wind(command, output, *, resume=False):
    diagnostics = command == 'cadence'
    work_package = 'WP19' if diagnostics else 'WP20'
    # The shared-load cadence check can encounter transient recording faults. Preserve a
    # rejected attempt and start at the first arm again. Explicit destinations
    # and the attended hold sitting keep their single-attempt behavior.
    attempts = 3 if diagnostics and output is None and not resume else 1
    for attempt in range(1, attempts + 1):
        stamp = datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%S.%fZ')
        destination = (output or WORKING / work_package / f'{command}-{stamp}').resolve()
        try:
            record = series(destination, diagnostics=diagnostics, resume=resume)
        except (OccludedMeasurement, RecordingTimeout) as error:
            if attempt == attempts:
                raise
            print(f'{error}. Rejected series: {destination / "series.json"}. '
                  'Starting a new whole wind series.', file=sys.stderr, flush=True)
        else:
            break
    print(destination / 'series.json')
    for run in record['cloud_runs']:
        print(f'{run["arm"]}: {run["holds"]=}, {run["neighbours_lost"]=}')
    return 1 if command == 'hold' and any(not run['holds'] for run in record['cloud_runs']) else 0
