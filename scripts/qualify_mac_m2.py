#!/usr/bin/env python3
"""Resumable attended Mac M2 collection. Run under the orchestrate GPU lock.

Never marks an item passed. An agent inspects window evidence before creating
window-acceptance.json; then the SAME command resumes the two trace arms.
No Screen Recording grant is requested. Never retries a failed recording.
"""
from __future__ import annotations

import argparse
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
                if finalization_seconds is not None and saving is None and 'Saving output file' in log.read_text(errors='replace'):
                    saving = time.monotonic()
                    deadline = min(deadline, saving + finalization_seconds)
                    save(log.with_suffix('.timing.json'), {'start_monotonic_s': started,
                         'saving_observed_monotonic_s': saving, 'deadline_monotonic_s': deadline,
                         'finalization_limit_s': finalization_seconds})
                try:
                    return process.wait(timeout=0.05)
                except subprocess.TimeoutExpired:
                    pass
            raise subprocess.TimeoutExpired(command, seconds)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGTERM)
            try:
                process.wait(timeout=3)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait(timeout=3)
            raise RuntimeError(f'owned process deadline ({seconds}s): {log}')


def collect(args, output, name, tap, replace=False, grid=False, traced=False):
    if shutil.disk_usage(output).free < 16 * 1024**3:
        raise RuntimeError('less than 16 GiB free before this arm; no attempt consumed')
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
        ledger = json.loads(LEDGER.read_text()) if LEDGER.exists() else {'commission': '9-November-2026', 'attempts': []}
        if len(ledger['attempts']) >= 2:
            raise RuntimeError('BLOCKED: two-attempt trace allowance exhausted; scoped re-plan required')
        # Durable before launching, including failures or interruption.
        ledger['attempts'].append({'arm': str(arm), 'unix_ns': time.time_ns(), 'hypothesis': HYPOTHESIS, 'status': 'started'})
        LEDGER.parent.mkdir(parents=True, exist_ok=True)
        save(LEDGER, ledger)
        with LEDGER.open('r+') as journal:
            journal.flush()
            os.fsync(journal.fileno())
        save(arm / 'recording-options.json', {'Points of Interest': {'excludeOSLogs': True}})
        command = ['xcrun', 'xctrace', 'record', '--no-prompt', '--template', 'Game Performance',
                   '--recording-options', str(arm / 'recording-options.json'),
                   '--time-limit', '16s', '--output', str(arm / 'pan.trace'),
                   '--target-stdout', str(arm / 'game.log')]
        for key, value in env.items():
            if key.startswith('THRUM_'):
                command += ['--env', f'{key}={value}']
        command += ['--launch', '--', str(args.binary.resolve())]
    save(arm / 'invocation.json', {'command': command, 'record_limit_s': 16 if traced else None,
                                 'finalization_limit_s': 75 if traced else None})
    load = LoadRecord(arm)
    begin = time.monotonic_ns()
    observations = set()
    def inspect_window():
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
                       inspect_window if replace else None, finalization_seconds=75 if traced else None)
        if code:
            raise RuntimeError(f'{name} exited {code}; see process.log')
        if not (arm / 'surface.tsv').exists():
            raise RuntimeError(f'{name} did not finalize its surface/image records')
        if traced:
            for filename, extra in [('toc.xml', ['--toc']), ('swaps.xml', ['--xpath',
                    '/trace-toc/run/data/table[@schema="displayed-surfaces-interval"]']),
                    ('signposts.xml', ['--xpath', "/trace-toc/run/data/table[contains(@schema, 'signpost')]"])]:
                code = bounded(['xcrun', 'xctrace', 'export', '--input', str(arm / 'pan.trace'),
                                '--output', str(arm / filename)] + extra, arm / (filename + '.log'), 30)
                if code or not (arm / filename).exists():
                    raise RuntimeError(f'unreadable {filename}; keep cadence open')
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
            save(LEDGER, ledger)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--binary', type=Path, default=ROOT / 'target/release/v1-game')
    parser.add_argument('--store', type=Path, default=ROOT / 'tmp/mac-m2/valley')
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    try:
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
                                         ('off-probe', 'off', False, False)]:
            arm = output / name
            if not arm.exists():
                collect(args, output, name, tap, replace, grid)
            elif not json.loads((arm / 'collection.json').read_text()).get('collected'):
                raise RuntimeError(f'failed {name}; diagnose before any new run')
        # This is an evidence gate for the resumed builder, not an owner approval prompt.
        acceptance = output / 'window-acceptance.json'
        if not acceptance.exists():
            raise RuntimeError('BLOCKED: builder must inspect M2.3 real-window pattern/lease/replacement/OFF evidence, '
                               'package native sequences, and record window-acceptance.json before tracing; see runbook')
        proof = json.loads(acceptance.read_text())
        if proof.get('binary_sha256') != identity['binary_sha256'] or proof.get('M2.3') != 'pass' or not proof.get('evidence'):
            raise RuntimeError('M2.3 evidence gate missing same-build pass/evidence')
        for name, tap in [('on-traced', 'on'), ('off-traced', 'off')]:
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
