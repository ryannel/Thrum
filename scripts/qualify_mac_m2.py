#!/usr/bin/env python3
"""Resumable attended Mac M2 collection. Run under the orchestrate GPU lock.

Never marks an item passed. An agent inspects window evidence before creating
window-acceptance.json; then the SAME command resumes the two trace arms.
No Screen Recording grant is requested. Never retries a failed recording.
"""
from __future__ import annotations

import argparse
import ctypes
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import re
import subprocess
import time

from capture_v1_runtime import digest
from t2_load import LoadRecord

ROOT = Path(__file__).resolve().parents[1]
LEDGER = ROOT / 'tmp/mac-m2/trace-commission.json'
HYPOTHESIS = ('Retained slot-events.txt shows the target already exited normally, then Game Performance '
              'hung saving and export reported Document Missing Template Error. Therefore target exit is '
              'not a new hypothesis. The current template exposes Points of Interest excludeOSLogs=false. '
              'Test the same Game Performance template with excludeOSLogs=true and no desktop recorder: '
              'less unrelated global log material may avoid finalization failure while preserving the '
              'independent displayed-surfaces-interval table. Success requires readable TOC and actual '
              'app/window swaps within deadlines. Failure rejects this configuration; no unchanged retry.')
OS_LOG_HYPOTHESIS = ('The first 11 November probe finalized and exported pan signposts and built-in display '
                    'identity, but both raw display-surface-swap/VSync and derived displayed-surfaces-interval '
                    'contained zero events with excludeOSLogs=true. Test excludeOSLogs=false on the same '
                    'Game Performance template and capture-OFF release assembly, retaining the absence of a '
                    'desktop recorder. OS-log exclusion may have removed the required display event source. '
                    'Success requires fresh app/window-attributed swaps and bounded finalization. Empty swaps '
                    'or another finalization failure rejects this hypothesis and exhausts the commission.')


def save(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')


def preflight(output):
    probe = subprocess.check_output(['/usr/sbin/ioreg', '-n', 'Root', '-d1'], text=True)
    locked = '\"IOConsoleLocked\" = Yes' in probe
    record = {'unix_ns': time.time_ns(), 'console_locked': locked,
              'screen_recording_requested': False, 'record_attempt_started': False}
    save(output / 'preflight.json', record)
    if locked:
        raise RuntimeError('BLOCKED: unlock and activate the built-in panel; no game/trace launched, no attempt consumed')
    swift = '''import CoreGraphics
var ids = [CGDirectDisplayID](repeating: 0, count: 16)
var count: UInt32 = 0
CGGetActiveDisplayList(16, &ids, &count)
for id in ids.prefix(Int(count)) where CGDisplayIsBuiltin(id) != 0 {
    if let mode = CGDisplayCopyDisplayMode(id) {
        print("display=\\(id) width=\\(mode.pixelWidth) height=\\(mode.pixelHeight) hz=\\(mode.refreshRate)")
    }
}
print("screen_recording_preflight=\\(CGPreflightScreenCaptureAccess())")
'''
    found = subprocess.run(['/usr/bin/swift', '-e', swift], text=True, capture_output=True, timeout=30)
    (output / 'display.txt').write_text(found.stdout + found.stderr)
    if found.returncode or 'display=' not in found.stdout:
        raise RuntimeError('BLOCKED: active built-in display unavailable')
    options = subprocess.run(['xcrun', 'xctrace', 'record', '--template', 'Game Performance',
                              '--show-recording-options'], text=True, capture_output=True, timeout=30)
    (output / 'recording-options.txt').write_text(options.stdout + options.stderr)
    if options.returncode:
        raise RuntimeError('BLOCKED: Instruments setup/authorization unavailable; no recording attempted')


def bounded(command, log, seconds, environment=None, observe=None, finalization_seconds=None):
    """Only terminate the process group created here; never enumerate/kill others."""
    with log.open('wb') as stream:
        process = subprocess.Popen(command, cwd=ROOT, env=environment, stdout=stream,
                                   stderr=subprocess.STDOUT, start_new_session=True)
        try:
            if observe is None and finalization_seconds is None:
                return process.wait(timeout=seconds)
            started = time.monotonic()
            deadline = started + seconds
            saving = None
            while time.monotonic() < deadline:
                if observe is not None:
                    observe()
                if finalization_seconds is not None and saving is None and any(event in log.read_text(errors='replace') for event in ('Target app exited, ending recording', 'Recording completed. Saving output file')):
                    saving = time.monotonic()
                    deadline = min(deadline, saving + finalization_seconds)
                    save(log.with_suffix('.timing.json'), {'start_monotonic_s': started,
                         'recording_stop_observed_monotonic_s': saving, 'deadline_monotonic_s': deadline,
                         'finalization_limit_s': finalization_seconds})
                try:
                    return process.wait(timeout=0.05)
                except subprocess.TimeoutExpired:
                    pass
            raise subprocess.TimeoutExpired(command, seconds)
        except subprocess.TimeoutExpired as error:
            raise RuntimeError(f'owned process deadline ({seconds}s): {log}') from error
        finally:
            if process.poll() is None:
                os.killpg(process.pid, signal.SIGTERM)
                try:
                    process.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait(timeout=3)


def panel_witness(output):
    panel = ctypes.CDLL('/System/Library/Frameworks/CoreGraphics.framework/CoreGraphics')
    for function in ('CGDisplayIsActive', 'CGDisplayIsAsleep', 'CGDisplayIsBuiltin'):
        getattr(panel, function).argtypes = [ctypes.c_uint32]
        getattr(panel, function).restype = ctypes.c_uint32
    panel.CGMainDisplayID.restype = ctypes.c_uint32
    last_panel_observation = 0.0
    def observe():
        nonlocal last_panel_observation
        if time.monotonic() - last_panel_observation >= 0.5:
            last_panel_observation = time.monotonic()
            raw = subprocess.check_output(['/usr/sbin/ioreg', '-n', 'Root', '-d1'], text=True, timeout=2)
            locked = re.search(r'^\s*"IOConsoleLocked"\s*=\s*(Yes|No)\s*$', raw, re.MULTILINE)
            display = panel.CGMainDisplayID()
            state = {'unix_ns': time.time_ns(), 'display_id': display,
                     'console_locked': locked[1] == 'Yes' if locked else None,
                     'builtin': bool(panel.CGDisplayIsBuiltin(display)),
                     'active': bool(panel.CGDisplayIsActive(display)),
                     'asleep': bool(panel.CGDisplayIsAsleep(display))}
            with (output / 'panel-witness.jsonl').open('a') as witness:
                witness.write(json.dumps(state) + '\n')
            if state['console_locked'] is not False or not state['builtin'] or not state['active'] or state['asleep']:
                raise RuntimeError('BLOCKED: local console unlock and awake active built-in panel required; owned recording stopped')
    return observe


def collect(args, output, name, tap, replace=False, grid=False, traced=False, include_os_logs=False, presented=False):
    if shutil.disk_usage(output).free < 16 * 1024**3:
        raise RuntimeError('less than 16 GiB free before this arm; no attempt consumed')
    ledger_path = ROOT / 'tmp/mac-m2/presented-commission-20261111.json' if presented else LEDGER
    allowance = 6 if presented else 2
    if args.post_apple_tile_m3:
        ledger_path = ROOT / 'tmp/mac-m3/recording-commission.json'
        allowance = 2
    hypothesis = ('The full Game Performance 16s window retained the full marked interval but exceeded its bounded process deadline. Use only Metal Application, Display and Points of Interest to reduce unrelated system-wide event and finalization volume, keeping the same 16s window and game. Require complete markers, drawable joins and bounded completion.' if args.presented_minimal else 'Explicit --window 16s replaces the template 5s ring buffer; retained ca-client-presented-handler rows must span both Whole markers. Empty display tables are recorded, not a failure.') if presented else OS_LOG_HYPOTHESIS if include_os_logs else HYPOTHESIS
    if presented and args.presented_deferred:
        hypothesis = 'The three-instrument 16s ring window finalized within bounds but retained only the final 1.075s of drawable events. Remove --window entirely and use the installed Blank template with windowLimit=0 (non-windowed default), retaining the same Metal Application/Display/Points of Interest instruments and game. Require both Whole markers inside retained presented times and bounded completion.'
    if args.post_apple_tile_m3:
        hypothesis = ('Fresh post-APPLE-TILE M3 qualification using Mac M2\'s accepted '
                      'minimal/deferred configuration. One ON and one OFF recording, '
                      'with no diagnostic attempts. Require complete Whole coverage, '
                      'validated presented-time conversion and bounded completion.')
    if traced:
        ledger = json.loads(ledger_path.read_text()) if ledger_path.exists() else {'commission': '11-November-2026-presented-time-ab66bca5' if presented else '9-November-2026', 'attempts': []}
        if args.post_apple_tile_m3:
            ledger['commission'] = 'PLATFORM-RUNTIME-MAC-M3-one-recording-per-traced-arm'
            if any(Path(attempt['arm']).name == name for attempt in ledger['attempts']):
                raise RuntimeError('BLOCKED: M3 grants one recording per traced arm; scoped re-plan required')
        if len(ledger['attempts']) >= allowance:
            raise RuntimeError(f'BLOCKED: {allowance}-attempt trace allowance exhausted; scoped re-plan required')
        if presented and any(a['hypothesis'] == hypothesis and a['status'].startswith('failed') for a in ledger['attempts']):
            raise RuntimeError('BLOCKED: unchanged failed recording configuration; change it before another attempt')
    arm = output / name
    arm.mkdir()
    env = {key: value for key, value in os.environ.items() if not key.startswith('THRUM_')}
    env.update(THRUM_MAC_QUALIFY=str(arm), THRUM_MAC_TAP=tap, THRUM_V1_FRAMES='10000',
               THRUM_PARAMS=str(ROOT / 'examples/v1-game/params/showcase-noon.params.json'))
    if not grid:
        env.update(THRUM_SCENE='valley', THRUM_ASSET_STORE=str(args.store.resolve()))
    if replace:
        env['THRUM_MAC_REPLACE'] = '1'
    if grid:
        env['THRUM_MAC_PATTERN'] = '1'
    save(arm / 'environment.json', {key: value for key, value in env.items() if key.startswith('THRUM_')})
    command = [str(args.binary.resolve())]
    if traced:
        # Durable before launching, including failures or interruption.
        ledger['attempts'].append({'arm': str(arm), 'unix_ns': time.time_ns(), 'hypothesis': hypothesis, 'status': 'started'})
        ledger_path.parent.mkdir(parents=True, exist_ok=True)
        save(ledger_path, ledger)
        with ledger_path.open('r+') as journal:
            journal.flush()
            os.fsync(journal.fileno())
        save(arm / 'recording-options.json', {'Points of Interest': {'excludeOSLogs': not include_os_logs}})
        command = ['xcrun', 'xctrace', 'record', '--no-prompt', '--template', 'Game Performance',
                   '--recording-options', str(arm / 'recording-options.json'),
                   '--time-limit', '16s', '--output', str(arm / 'pan.trace'),
                   '--target-stdout', str(arm / 'game.log')]
        if presented:
            if not args.presented_deferred:
                command += ['--window', '16s']
            if args.presented_minimal:
                command[command.index('Game Performance')] = '/Applications/Xcode.app/Contents/Applications/Instruments.app/Contents/Packages/Base.instrdst/Contents/Templates/Blank.tracetemplate'
                command += ['--instrument', 'Metal Application', '--instrument', 'Display', '--instrument', 'Points of Interest']
        for key, value in env.items():
            if key.startswith('THRUM_'):
                command += ['--env', f'{key}={value}']
        command += ['--launch', '--', str(args.binary.resolve())]
    (arm / 'collector.py').write_bytes(Path(__file__).read_bytes())
    save(arm / 'invocation.json', {'command': command, 'record_limit_s': 16 if traced else None,
                                 'finalization_limit_s': 75 if traced else None,
                                 'collector_sha256': digest(arm / 'collector.py'),
                                 'panel_observation_period_s': 0.5})
    load = LoadRecord(arm)
    begin = time.monotonic_ns()
    observations = set()
    observe_panel = panel_witness(arm)
    def inspect_window():
        observe_panel()
        if not replace or 'screen_recording_preflight=true' not in (output / 'display.txt').read_text():
            return
        elapsed = (time.monotonic_ns() - begin) / 1e9
        metadata = arm / 'window.txt'
        for target in (2, 6):
            if target in observations or elapsed < target or not metadata.exists():
                continue
            observations.add(target)
            number = re.search(r'^window_number=(\d+)$', metadata.read_text(), re.MULTILINE)
            if number:
                capture = subprocess.run(['/usr/sbin/screencapture', '-x', '-l', number[1],
                    str(arm / f'window-{target}s.png')], capture_output=True, text=True, timeout=5)
                save(arm / f'window-{target}s.json', {'exit': capture.returncode, 'stderr': capture.stderr,
                    'scope': 'separate attended OS-window observation with existing grant'})
    try:
        code = bounded(command, arm / 'process.log', 91 if traced else 30, env,
                       inspect_window, finalization_seconds=75 if traced else None)
        if code:
            raise RuntimeError(f'{name} exited {code}; see process.log')
        if not (arm / 'surface.tsv').exists():
            raise RuntimeError(f'{name} did not finalize its surface/image records')
        if traced:
            for filename, extra in [('toc.xml', ['--toc']), ('swaps.xml', ['--xpath',
                    '/trace-toc/run/data/table[@schema="displayed-surfaces-interval"]']),
                    ('signposts.xml', ['--xpath', "/trace-toc/run/data/table[contains(@schema, 'signpost')]"])]:
                code = bounded(['xcrun', 'xctrace', 'export', '--input', str(arm / 'pan.trace'),
                                '--output', str(arm / filename)] + extra, arm / (filename + '.log'), 30,
                               observe=inspect_window)
                if code or not (arm / filename).exists():
                    raise RuntimeError(f'unreadable {filename}; keep cadence open')
            if presented:
                for filename, schema in [('presented.xml', 'ca-client-presented-handler'), ('displays.xml', 'device-display-info'), ('raw-displays.xml', 'display-surface-swap" or @schema="display-vsyncs-interval" or @schema="display-surface-queue')]:
                    code = bounded(['xcrun', 'xctrace', 'export', '--input', str(arm / 'pan.trace'), '--output', str(arm / filename), '--xpath', f'/trace-toc/run/data/table[@schema="{schema}"]'], arm / (filename + '.log'), 30, observe=inspect_window)
                    if code:
                        raise RuntimeError(f'failed {filename}')
            ledger['attempts'][-1]['status'] = 'exported; attribution and full-pan join unjudged'
        save(arm / 'collection.json', {'elapsed_ns': time.monotonic_ns() - begin,
                                      'collected': True, 'qualified': False})
    except Exception as error:
        save(arm / 'collection.json', {'collected': False, 'error': str(error), 'qualified': False})
        if traced:
            ledger['attempts'][-1]['status'] = 'failed; no unchanged retry'
        raise
    finally:
        save(arm / 'load.json', load.finish())
        if traced:
            save(ledger_path, ledger)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--binary', type=Path, default=ROOT / 'target/release/v1-game')
    parser.add_argument('--store', type=Path, default=ROOT / 'tmp/mac-m2/valley')
    parser.add_argument('--identity', type=Path, help='identity.json from this build’s freshly qualified window arms; required for presented arms')
    parser.add_argument('--window-only', action='store_true', help='collect only untraced window proofs without spending a trace recording')
    parser.add_argument('--post-apple-tile-m3', action='store_true',
                        help='M3 commission: one recording per traced arm, no diagnostic attempts')
    parser.add_argument('--diagnostic-with-os-logs', action='store_true',
                        help='use the remaining diagnostic attempt for OFF with OS logs after the empty-display probe')
    parser.add_argument('--presented-deferred', action='store_true', help='omit --window; installed Blank template has windowLimit=0')
    parser.add_argument('--presented-minimal', action='store_true', help='Blank template plus Metal Application, Display and Points of Interest; fewer system-wide sources')
    parser.add_argument('--presented-arm', choices=['on-traced', 'off-traced', 'on-untraced'], help='11 November reviewed presented-time commission; one arm per invocation')
    args = parser.parse_args()
    if args.post_apple_tile_m3 and not (args.presented_arm and args.presented_minimal and args.presented_deferred):
        parser.error('--post-apple-tile-m3 requires a presented arm and the qualified minimal/deferred configuration')
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    try:
        if args.presented_arm:
            panel_witness(output)()
            if args.identity is None:
                raise RuntimeError('--identity from fresh same-build window qualification is required')
            retained = json.loads(args.identity.read_text())
            if digest(args.binary) != retained['binary_sha256']:
                raise RuntimeError('retained M2.1 binary changed')
            if digest(ROOT / 'examples/v1-game/params/showcase-noon.params.json') != retained['preset_sha256']:
                raise RuntimeError('retained preset changed')
            actual = {str(p.relative_to(args.store)): digest(p) for p in sorted(args.store.rglob('*')) if p.is_file()}
            if actual != retained['cooked_hashes']:
                raise RuntimeError('retained cooked content changed')
            save(output / 'identity.json', retained)
            collect(args, output, args.presented_arm, 'off' if args.presented_arm == 'off-traced' else 'on', traced=args.presented_arm != 'on-untraced', presented=True)
            return 0
        preflight(output)
        if shutil.disk_usage(output).free < 16 * 1024**3:
            raise RuntimeError('less than 16 GiB free; reclaim only owned reconstructible outputs first')
        identity = {'binary_sha256': digest(args.binary), 'preset_sha256': digest(ROOT / 'examples/v1-game/params/showcase-noon.params.json'),
                    'source_revision': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
                    'dirty_diff_sha256': hashlib.sha256(subprocess.check_output(['git', 'diff', 'HEAD'])).hexdigest(),
                    'cooked_hashes': {str(p.relative_to(args.store)): digest(p) for p in sorted(args.store.rglob('*')) if p.is_file()},
                    'hypothesis': HYPOTHESIS, 'qualified': False}
        pin = output / 'identity.json'
        if pin.exists():
            retained = json.loads(pin.read_text())
            matched_fields = ('binary_sha256', 'preset_sha256', 'cooked_hashes', 'hypothesis')
            if any(retained.get(key) != identity[key] for key in matched_fields):
                raise RuntimeError('build/content changed: use a fresh output and rerun M2.1; trace budget is not renewed')
            save(output / 'resume-source.json', identity)
        else:
            save(pin, identity)
        for name, tap, replace, grid in [('grid-replacement', 'on', True, True),
                                         ('valley-replacement', 'on', True, False),
                                         ('on-untraced', 'on', False, False),
                                         ('off-probe', 'off', False, False),
                                         ('on-untraced-current', 'on', False, False)]:
            arm = output / name
            if not arm.exists():
                collect(args, output, name, tap, replace, grid)
            elif not json.loads((arm / 'collection.json').read_text()).get('collected'):
                raise RuntimeError(f'failed {name}; diagnose before any new run')
        if args.window_only:
            print('Untraced window arms collected; builder inspection remains required. No trace recording consumed.')
            return 0
        # This is an evidence gate for the resumed builder, not an owner approval prompt.
        acceptance = output / 'window-acceptance.json'
        if not acceptance.exists():
            raise RuntimeError('BLOCKED: builder must inspect M2.3 real-window pattern/lease/replacement/OFF evidence, '
                               'package native sequences, and record window-acceptance.json before tracing; see runbook')
        proof = json.loads(acceptance.read_text())
        if proof.get('binary_sha256') != identity['binary_sha256'] or proof.get('M2.3') != 'pass' or not proof.get('evidence'):
            raise RuntimeError('M2.3 evidence gate missing same-build pass/evidence')
        if args.diagnostic_with_os_logs:
            probe = output / 'on-traced'
            if not (probe / 'collection.json').exists() or not json.loads((probe / 'collection.json').read_text()).get('collected'):
                raise RuntimeError('inspect the retained first diagnostic before the OS-log diagnostic')
            collect(args, output, 'off-traced', 'off', traced=True, include_os_logs=True)
            print('Second diagnostic collected; fresh attribution and clock alignment remain unjudged.')
            return 0
        for name, tap in [('on-traced', 'on'), ('off-traced', 'off')]:
            if name == 'off-traced':
                # Preserve the lock while the builder judges the fresh probe's
                # actual app/window swaps before spending the second attempt.
                tooling = output / 'tooling-acceptance.json'
                deadline = time.monotonic() + 120
                observe_probe = panel_witness(output / 'on-traced')
                while not tooling.exists() and time.monotonic() < deadline:
                    observe_probe()
                    time.sleep(0.5)
                if not tooling.exists():
                    raise RuntimeError('M2.4 probe collected; inspect fresh TOC/app/window swaps and record '
                                       'tooling-acceptance.json before the OFF trace; no second attempt consumed')
                proof = json.loads(tooling.read_text())
                if proof.get('binary_sha256') != identity['binary_sha256'] or proof.get('M2.4') != 'pass' or not proof.get('evidence'):
                    raise RuntimeError('M2.4 fresh attributed-swap proof required before the OFF trace')
                preflight(output)
            arm = output / name
            if not arm.exists():
                collect(args, output, name, tap, traced=True)
            elif not json.loads((arm / 'collection.json').read_text()).get('collected'):
                raise RuntimeError('failed trace: unchanged retries forbidden; retain ledger and seek changed hypothesis')
        print('Collected matched arms. BLOCKED until fresh swap attribution/full-pan clock join and 1x inspection are judged; no item was auto-passed.')
        return 0
    except (OSError, ValueError, RuntimeError, subprocess.SubprocessError) as error:
        print(str(error))
        return 2


if __name__ == '__main__':
    raise SystemExit(main())
