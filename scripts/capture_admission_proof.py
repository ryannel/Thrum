#!/usr/bin/env python3
"""Real Metal admission proofs; invoke under the orchestrate gpu lock.

The loaded proof declares mixed CPU/GPU competition, including normal engine
readbacks. It does not qualify uncaptured cadence. Every attempt is recorded.
"""
import argparse
from datetime import datetime, timezone
import gzip
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import sys

from apple_tile_m0 import digest, save
from capture_v1_runtime import finalize, save_manifest

ROOT = Path(__file__).resolve().parents[1]
TASK = ROOT / 'tmp/t2-capture-m3'


def prepare():
    TASK.mkdir(parents=True, exist_ok=True)
    with (TASK / 'proof-build.log').open('w') as log:
        subprocess.run(['rustup', 'run', 'stable', 'cargo', 'build', '--release', '--locked', '--offline',
            '-p', 'v1-game', '--features', 'draw-probe,thrum-render/creation-counters', '-p', 'thrum-cook'],
            cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True, timeout=600)
        subprocess.run(['target/release/thrum-cook', 'cook', 'examples/v1-game/assets/showcase', str(TASK / 'valley')],
            cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True, timeout=120)


def session(output, name, case='variants', recipe='portable', policy=None, hold=0, source='sdr', verify=True):
    directory = output / name
    command = [sys.executable, 'scripts/capture_v1_runtime.py', '--binary', 'target/release/v1-game',
               '--store', str(TASK / 'valley'), '--output', str(directory), '--case', case,
               '--mode', 'realtime', '--recipe', recipe, '--source', source, '--defer']
    if policy is not None:
        command += ['--admission', policy]
    if case == 'route':
        route = json.loads((ROOT / 'examples/v1-game/routes/moving-v1.json').read_text())
        route['render_extent'] = [1920, 1080]
        save(output / 'runtime-route.json', route)
        command += ['--route', str(output / 'runtime-route.json')]
    environment = {k: v for k, v in os.environ.items() if not k.startswith('THRUM_')}
    environment.update(THRUM_CAPTURE_HOLD_FULL_MS=str(hold), THRUM_CULLING='direct', THRUM_STARS='on')
    with (output / (name + '.log')).open('w') as log:
        result = subprocess.run(command, cwd=ROOT, env=environment, stdout=log, stderr=subprocess.STDOUT, timeout=120)
    runtime = json.loads((directory / 'runtime.json').read_text())
    manifest = json.loads((directory / 'manifest.json').read_text())
    assert runtime['recipe'] == f"Ok({'AppleTile' if recipe == 'apple-tile' else 'Portable'})"
    omissions = [row for row in runtime['opportunities'] if row.get('ring_omissions')]
    assert all(row['admission_usage'][0] == 6 for row in omissions), 'omission with a free staging slot'
    submitted = sum(row['submitted'] for row in runtime['opportunities'])
    if result.returncode == 0:
        assert runtime['complete'] and len(runtime['images']) == submitted
        assert not runtime['ring_omissions'] and not runtime['writer_omissions']
        # Independent zstd CLI/XOR decode verifies every original image hash.
        if verify:
            verify_images(directory, manifest, runtime, submitted)
    else:
        assert result.returncode == 75 and not runtime['complete'] and manifest.get('could_not_judge'), manifest
    row = {'name': name, 'command': command, 'exit': result.returncode, 'submitted': submitted,
           'images': len(runtime['images']), 'ring_omissions': runtime['ring_omissions'],
           'writer_omissions': runtime['writer_omissions'], 'all_omissions_at_full_ring': True,
           'policy': runtime['admission_policy'], 'waits': runtime['admission_waits'],
           'skipped_waiting': runtime['admission_skipped_opportunities'],
           'diagnostic': runtime['admission_diagnostic'], 'source': source, 'verified': result.returncode == 0 and verify,
           'source_sha256': digest(directory / manifest['spool_file']),
           'source_retention': 'complete sources hash-verified then discarded; runtime and manifest retained',
           'runtime_sha256': digest(directory / 'runtime.json'), 'manifest_sha256': digest(directory / 'manifest.json')}
    if verify:
        retire_source(directory, manifest)
    return row


def verify_images(directory, manifest, runtime, submitted):
    finalize(directory, manifest, runtime, verify_only=True)
    save_manifest(directory, manifest)
    assert manifest['complete'] and len(manifest['records']) == submitted
    assert [row['submit_ns'] for row in manifest['records']] == [row['submit_ns'] for row in runtime['images']]


def retire_source(directory, manifest):
    for filename in ['runtime.json', 'manifest.json']:
        path = directory / filename
        (directory / (filename + '.gz')).write_bytes(gzip.compress(path.read_bytes(), mtime=0))
    # No visual-inspection item: keep identities and original hashes, not
    # gigabytes of reconstructible pixels. Incomplete source hashes remain too.
    (directory / manifest['spool_file']).unlink()


def start_load(output):
    processes = []
    try:
        for _ in range(24):
            processes.append(subprocess.Popen([sys.executable, '-c', 'while True: pass'], start_new_session=True))
        for index in range(8):
            directory = output / f'competing-engine-{index}'
            directory.mkdir()
            environment = {k: v for k, v in os.environ.items() if not k.startswith('THRUM_')}
            environment.update(THRUM_CAPTURE_RUNTIME=str(directory), THRUM_CAPTURE_CASE='variants',
                THRUM_CAPTURE_MODE='realtime', THRUM_ASSET_STORE=str(TASK / 'valley'), THRUM_SCENE='valley')
            processes.append(subprocess.Popen([sys.executable, '-c',
                'import subprocess,sys\nwhile True: subprocess.run([sys.argv[1]], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)',
                str(ROOT / 'target/release/v1-game')], env=environment, start_new_session=True))
        return processes
    except BaseException:
        stop_load(processes)
        raise


def stop_load(processes):
    for process in processes:
        if process.poll() is None:
            os.killpg(process.pid, signal.SIGTERM)
    for process in processes:
        process.wait(timeout=20)


def loaded(output, report):
    processes = start_load(output)
    report['competing_load'] = {'cpu_workers': 24, 'engine_workers': 8,
        'engine_workload': 'repeated ordinary 1080p realtime variants, default omit; independent processes, full scene and readbacks',
        'pids': [p.pid for p in processes], 'scope': 'mixed CPU/GPU contention; no isolated GPU-causality or cadence claim'}
    try:
        for attempt in range(3):
            row = session(output, f'omit-control-{attempt}', case='route', verify=False)
            report['sessions'].append(row)
            save(output / 'report.json', report)
            if row['ring_omissions']:
                break
        else:
            raise AssertionError('declared competing load did not reproduce ring exhaustion')
        for matrix, cases in [('m1', ['variants', 'route']), ('m2', ['render-stall', 'route'])]:
            for recipe in ['portable', 'apple-tile']:
                for case in cases:
                    row = session(output, f'{matrix}-{recipe}-{case}', case, recipe, 'wait', verify=False)
                    report['sessions'].append(row)
                    save(output / 'report.json', report)
                    assert row['exit'] == 0, f'{row["name"]} did not complete under wait'
    finally:
        stop_load(processes)
    # Deferred CPU decoding is outside the competing workload. Every measured
    # capture interval above has the same load, without spending GPU lock time
    # starving an unrelated postprocessor of CPU service.
    for row in report['sessions']:
        directory = output / row['name']
        runtime = json.loads((directory / 'runtime.json').read_text())
        manifest = json.loads((directory / 'manifest.json').read_text())
        if row['exit'] == 0:
            verify_images(directory, manifest, runtime, row['submitted'])
            row['verified'] = True
            row['manifest_sha256'] = digest(directory / 'manifest.json')
        retire_source(directory, manifest)
    report['verification_load'] = 'competing workers stopped after all capture intervals, before deferred decoding'


def ring_full(output, report):
    for source in ['sdr', 'scene-linear']:
        for policy in [None, 'wait']:
            row = session(output, f'{source}-{policy or "default"}', policy=policy, hold=350, source=source)
            report['sessions'].append(row)
            save(output / 'report.json', report)
            assert row['diagnostic']['filled'] and row['diagnostic']['released']
            if policy == 'wait':
                assert row['exit'] == 0 and row['skipped_waiting'] > 0
                assert row['skipped_waiting'] == sum(wait['skipped_opportunities'] for wait in row['waits'])
                assert any(wait['end_ns'] - wait['start_ns'] >= 300_000_000 for wait in row['waits'])
            else:
                assert row['exit'] == 75 and row['ring_omissions'] > 0 and row['policy'] == 'omit'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('proof', choices=['loaded', 'ring-full'])
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    prepare()
    output = (args.output or TASK / (args.proof + '-' + datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%S'))).resolve()
    output.mkdir(parents=True, exist_ok=False)
    report = {'passed': False, 'proof': args.proof, 'sessions': [],
        'revision': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
        'diff_sha256': hashlib.sha256(subprocess.check_output(['git', 'diff', 'HEAD'], cwd=ROOT)).hexdigest(),
        'binary_sha256': digest(ROOT / 'target/release/v1-game'), 'load_before': os.getloadavg()}
    try:
        (loaded if args.proof == 'loaded' else ring_full)(output, report)
        report['passed'] = True
    finally:
        report['load_after'] = os.getloadavg()
        save(output / 'report.json', report)
        print(f'{args.proof}: {report["passed"]}; {output / "report.json"}', flush=True)
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
