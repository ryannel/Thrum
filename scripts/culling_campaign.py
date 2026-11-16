#!/usr/bin/env python3
"""Measure one bounded CULLING campaign on main; never integrate or publish."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import threading
import time

from run_v1_showcase import store_root


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def git(*args):
    return subprocess.check_output(['git', *args], text=True).strip()


def save(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')


def completed_worker_rows(output):
    """Count completed render records, excluding metadata and unfinished writes."""
    result = {}
    for path in output.glob('worker-*/rows.jsonl'):
        rows = map(json.loads, path.read_bytes().split(b'\n')[:-1])
        result[str(path.relative_to(output))] = sum('tick' in row and 'mode' in row for row in rows)
    return result


def pair(args, output, turn):
    recipe = ('portable', 'apple-tile')[turn % 2]
    source = ('sdr', 'scene-linear')[(turn // 2) % 2]
    meshes = ('off', 'on')[(turn // 4) % 2]
    route = Path('examples/v1-game/routes/moving-v1.json').resolve()
    spec = json.loads(route.read_text())
    reserve = 16 * 1024**3 + spec['render_extent'][0] * spec['render_extent'][1] * 8 * 4 * (spec['keys'][-1]['tick'] + 1)
    assert shutil.disk_usage(output).free >= reserve, 'insufficient failure-payload reserve'
    common = {k: v for k, v in os.environ.items() if not k.startswith('THRUM_')}
    common.update(THRUM_SCENE='valley', THRUM_ASSET_STORE=str(args.store.resolve()),
                  THRUM_CAPTURE_ROUTE=str(route), THRUM_VALLEY_MESHES=meshes)
    stop = threading.Event()
    active, worker_errors = {}, []
    guard = threading.Lock()
    report = {'turn': turn, 'recipe': recipe, 'source': source, 'meshes': meshes,
              'runs': [], 'worker_rounds': [], 'passed': False}

    def counts():
        return completed_worker_rows(output)

    def worker(number):
        iteration = 0
        while not stop.is_set():
            folder = output / f'worker-{number}-{iteration}'
            with folder.with_suffix('.log').open('w') as log:
                with guard:
                    if stop.is_set():
                        return
                    process = subprocess.Popen([str(args.control.resolve()), '--ignored', '--nocapture'],
                        env=dict(common, THRUM_RECIPE='portable', THRUM_CULLING_TIMING='1',
                                 THRUM_CULLING_OUTPUT=str(folder)), stdout=log, stderr=subprocess.STDOUT)
                    active[number] = process
                try:
                    code = process.wait(timeout=600)
                except subprocess.TimeoutExpired:
                    process.kill()
                    code = process.wait()
                with guard:
                    active.pop(number, None)
                    report['worker_rounds'].append({'worker': number, 'round': iteration, 'exit': code})
                    if code and not stop.is_set():
                        worker_errors.append(f'{number}/{iteration}: {code}')
                        return
            iteration += 1

    threads = [threading.Thread(target=worker, args=(n,), daemon=True) for n in range(2)]
    try:
        for thread in threads:
            thread.start()
        deadline = time.monotonic() + 180
        while not all(sum(v for k, v in counts().items() if k.startswith(f'worker-{n}-')) >= 3 for n in range(2)):
            assert not worker_errors and time.monotonic() < deadline, 'workers failed to start'
            time.sleep(.1)
        for arm in ('candidate', 'control'):
            folder = output / arm
            before = counts()
            with folder.with_suffix('.log').open('w') as log:
                result = subprocess.run([str(getattr(args, arm).resolve()), '--ignored', '--nocapture'],
                    env=dict(common, THRUM_RECIPE=recipe, THRUM_CULLING_SOURCE=source,
                             THRUM_CULLING_CAMPAIGN='1', THRUM_CULLING_OUTPUT=str(folder)),
                    stdout=log, stderr=subprocess.STDOUT, timeout=1200)
            rows = list(map(json.loads, (folder / 'rows.jsonl').read_text().splitlines()))
            final = rows[-1]
            comparisons = [r for r in rows if 'comparison_tick' in r]
            assert final['hosts'] == 4 and final['compared'] == len(comparisons) == 361
            assert [r['comparison_tick'] for r in comparisons] == list(range(180, 541))
            assert final['differing_ticks'] == sum(not r['equal'] for r in comparisons)
            assert result.returncode == (101 if final['differing_ticks'] else 0)
            after = counts()
            progress = [sum(v for k, v in after.items() if k.startswith(f'worker-{n}-')) >
                        sum(v for k, v in before.items() if k.startswith(f'worker-{n}-')) for n in range(2)]
            report['runs'].append({'arm': arm, 'rows_sha256': sha(folder / 'rows.jsonl'),
                'result': final, 'worker_rows_before': before, 'worker_rows_after': after,
                'workers_progressed': progress, 'load': os.getloadavg(), 'exit': result.returncode})
            save(output / 'pair.json', report)
            assert all(progress) and not worker_errors, 'both counted workers must progress'
            assert arm != 'candidate' or not final['differing_ticks'], 'coverage build differs; retain payloads and stop'
        report['passed'] = True
    finally:
        with guard:
            stop.set()
            children = list(active.values())
        for process in children:
            process.terminate()
        for process in children:
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
        for thread in threads:
            thread.join(timeout=12)
        report['worker_errors'] = worker_errors
        report['ended_utc'] = time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())
        save(output / 'pair.json', report)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('candidate', 'control', 'store', 'output', 'loop'):
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--candidate-commit', required=True)
    parser.add_argument('--control-commit', required=True)
    parser.add_argument('--pair', type=int)
    args = parser.parse_args()
    args.output = args.output.resolve()
    args.output.mkdir(parents=True, exist_ok=False)
    if args.pair is not None:
        pair(args, args.output, args.pair)
        return
    subprocess.run(['git', 'merge-base', '--is-ancestor', args.candidate_commit, 'origin/main'], check=True)
    route = Path('examples/v1-game/routes/moving-v1.json')
    assert route.read_bytes() == subprocess.check_output(['git', 'show', f'{args.candidate_commit}:{route}']), 'route differs from campaigned build'
    assert store_root(args.store) == json.loads(route.read_text())['cooked_root']
    changed = git('diff', '--name-only', args.candidate_commit, args.control_commit).splitlines()
    assert set(changed) <= {'crates/thrum-render/src/synthetic.wgsl', 'crates/thrum-render/src/synthetic/secondary.wgsl'} and changed
    record = {'candidate_commit': args.candidate_commit, 'control_commit': args.control_commit,
        'source_diff': git('diff', args.candidate_commit, args.control_commit),
        'binary_sha256': {n: sha(getattr(args, n)) for n in ('candidate', 'control')},
        'profile': 'release', 'features': 'default', 'compiler': subprocess.check_output(['rustup', 'run', '1.99.1', 'rustc', '-Vv'], text=True),
        'route_sha256': sha(route), 'cooked_root': store_root(args.store), 'runner_sha256': sha(Path(__file__)),
        'started_utc': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime()),
        'pairs': [], 'control_differing_routes': 0, 'passed': False}
    record['embedded_shader'] = {}
    for arm in ('candidate', 'control'):
        source = subprocess.check_output(['git', 'show', getattr(args, arm + '_commit') + ':crates/thrum-render/src/synthetic.wgsl'])
        embedded = source in getattr(args, arm).read_bytes()
        record['embedded_shader'][arm] = {'sha256': hashlib.sha256(source).hexdigest(), 'exact_source_in_binary': embedded}
        assert embedded, f'{arm} binary does not contain its committed shader'
    save(args.output / 'report.json', record)
    for turn in range(16):
        folder = args.output / f'pair-{turn:02}'
        command = [sys.executable, str(args.loop), 'lock', 'gpu', '--worktree', '.', '--', sys.executable, __file__]
        for name in ('candidate', 'control', 'store', 'loop'):
            command.extend(['--' + name, str(getattr(args, name).resolve())])
        command.extend(['--output', str(folder), '--candidate-commit', args.candidate_commit,
                        '--control-commit', args.control_commit, '--pair', str(turn)])
        result = subprocess.run(command)
        proof = json.loads((folder / 'pair.json').read_text()) if (folder / 'pair.json').exists() else {}
        record['pairs'].append({'path': str(folder.relative_to(Path.cwd())), 'exit': result.returncode, 'proof': proof})
        save(args.output / 'report.json', record)
        assert result.returncode == 0 and proof['passed'], 'pair failed; preserve all payloads'
        record['control_differing_routes'] += int(proof['runs'][1]['result']['differing_ticks'] > 0)
        record['passed'] = turn >= 7 and record['control_differing_routes'] >= 5
        record['ended_utc'] = proof['ended_utc']
        save(args.output / 'report.json', record)
        print(f"pairs={turn + 1} differing control routes={record['control_differing_routes']}", flush=True)
        if record['passed']:
            break
    if not record['passed']:
        raise SystemExit(2)


if __name__ == '__main__':
    main()
