#!/usr/bin/env python3
"""Verify registered exact images on main, and tell a commit whether main's run covers it.

`verify` asks the main-branch check named MAIN_CHECK whether a passing run contains HEAD.
It draws nothing and takes no lock. `verify --full` draws and compares every registered
baseline here: the main-branch check runs it on main's tip, and a planned refresh runs it
on its candidate under the gpu lock. Its compact records live in tmp/baseline-guard;
successful bulk captures are discarded after comparison. `guard` checks registered
references, definitions and comparison files against their recorded pins.
"""
import argparse
from datetime import datetime, timezone
import gzip
import hashlib
import json
import zlib
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]
REGISTRY = '.config/exact-baselines.json'
SCRATCH = 'tmp/baseline-guard'
MAIN_CHECK = 'exact-images'  # the `Main-branch check` line in ORCHESTRATE.md


def git(*args, root=ROOT):
    return subprocess.check_output(['git', *args], cwd=root).decode().strip()


def registry(root=ROOT):
    return json.loads((root / REGISTRY).read_text())


def content_key(root=ROOT):
    paths = registry(root)['covered_paths']
    objects = subprocess.check_output(['git', 'ls-tree', '-r', '-z', 'HEAD', '--', *paths], cwd=root)
    return hashlib.sha256(json.dumps(paths).encode() + b'\0' + objects).hexdigest()


def tree(root, revision):
    rows = subprocess.check_output(['git', 'ls-tree', '-r', '-z', revision], cwd=root)
    return {path.decode(): meta.decode().split()[2]
            for row in rows.split(b'\0') if row for meta, path in [row.split(b'\t', 1)]}


def definition(entry):
    return {key: value for key, value in entry.items() if key not in ('pin', 'refresh')}


def protected(entry, objects):
    reference = entry['reference']
    files = {path: oid for path, oid in objects.items()
             if path == reference or path.startswith(reference.rstrip('/') + '/')}
    if not files:
        raise ValueError(f"missing reference {reference}")
    for path in entry['comparison_files']:
        if path not in objects:
            raise ValueError(f'missing comparison file {path}')
        files[path] = objects[path]
    return files


def pin(entry, objects):
    payload = {'definition': definition(entry), 'files': protected(entry, objects)}
    return hashlib.sha256(json.dumps(payload, sort_keys=True, separators=(',', ':')).encode()).hexdigest()


def guard(root=ROOT):
    """CPU-only check of HEAD and its merge base; no bypass or local state."""
    from repo_check import anchors
    base = git('merge-base', 'HEAD', 'refs/remotes/origin/main', root=root)
    current = json.loads(git('show', f'HEAD:{REGISTRY}', root=root))
    previous = json.loads(git('show', f'{base}:{REGISTRY}', root=root))
    objects, old_objects = tree(root, 'HEAD'), tree(root, base)
    rulings = anchors(git('show', 'HEAD:docs/plans/ARCHITECTURE.md', root=root))
    old = {entry['id']: entry for entry in previous['baselines']}
    active, retired = current['baselines'], current.get('retired', [])
    errors = []

    def fail(entry, message):
        errors.append(f"baseline {entry['id']} owner {entry['owner']}: rule 1: {message}")

    def refresh_valid(entry):
        refresh = entry.get('refresh', {})
        if (not isinstance(refresh, dict) or refresh.get('anchor') not in rulings
                or not refresh.get('module') or not refresh.get('milestone')):
            fail(entry, 'refresh must name an existing module-list anchor, module and milestone')

    identities = [entry['id'] for entry in active + retired]
    if len(identities) != len(set(identities)):
        raise ValueError('rule 1: duplicate registered or retired baseline identity')
    for entry in old.values():
        if entry['id'] not in identities:
            fail(entry, 'registered at merge base but neither registered nor retired')
    for entry in retired:
        refresh_valid(entry)
        if entry['id'] in old and entry['owner'] != old[entry['id']]['owner']:
            fail(entry, 'retirement changes the owner')
    for entry in active:
        refresh_valid(entry)
        before = old.get(entry['id'])
        try:
            computed = pin(entry, objects)
            print(f"{entry['id']} owner {entry['owner']} pin {computed}")
            if entry.get('pin') != computed:
                changes = []
                if before:
                    prior_files = protected(before, old_objects) if before.get('comparison_files') else {
                        path: oid for path, oid in old_objects.items()
                        if path == before['reference'] or path.startswith(before['reference'].rstrip('/') + '/')}
                    now_files = protected(entry, objects)
                    changes.extend(path for path in sorted(prior_files.keys() | now_files.keys())
                                   if prior_files.get(path) != now_files.get(path))
                    changes.extend('definition.' + key for key in sorted(definition(before).keys() | definition(entry).keys())
                                   if definition(before).get(key) != definition(entry).get(key))
                fail(entry, 'pin mismatch; ' + ', '.join(changes or ['protected content differs from pin']))
            if before and entry.get('pin') != before.get('pin') and entry.get('refresh') == before.get('refresh'):
                fail(entry, 'pin changed without a changed refresh')
            paths = [REGISTRY, entry['reference'], *entry['comparison_files']]
            dirty = git('status', '--porcelain', '--untracked-files=all', '--', *paths, root=root)
            if dirty:
                fail(entry, 'uncommitted protected changes: ' + dirty)
        except (KeyError, ValueError) as error:
            paths = [entry.get('reference', ''), *entry.get('comparison_files', [])]
            if before:
                paths.extend([before['reference'], *before.get('comparison_files', [])])
            changed = git('diff', '--name-only', base, 'HEAD', '--', *paths, root=root)
            fail(entry, f'{error}; {changed}')
    if errors:
        raise ValueError('\n'.join(errors))
    print('PASS exact-baseline reference guard')


# Measured failure: 1.31 GiB; compact success: <9 MiB. One day (48 records)
# plus one failed set is about 1.73 GiB, against the 16 GiB free-space preflight.
KEEP_RECORDS = 48


def prune_scratch(root=ROOT):
    """Only prune completed, verifier-owned directories; never follow symlinks."""
    scratch = root / SCRATCH
    if not scratch.exists():
        return
    if scratch.is_symlink() or scratch.resolve() != root.resolve() / SCRATCH:
        raise ValueError('scratch must be inside this checkout')
    completed = []
    for directory in sorted(scratch.glob('verify-*'), reverse=True):
        if not directory.is_dir() or directory.is_symlink():
            continue
        try:
            record = json.loads((directory / 'record.json').read_text())
        except (OSError, ValueError):
            continue
        if 'seconds' in record and (record.get('passed') is True or 'error' in record):
            completed.append((directory, record))
    latest_failure = next((directory for directory, record in completed
                           if not record.get('passed') and not record.get('could_not_judge')), None)
    latest_unjudged = next((directory for directory, record in completed if record.get('could_not_judge')), None)
    for index, (directory, _) in enumerate(completed):
        if directory in (latest_failure, latest_unjudged):
            continue
        if index >= KEEP_RECORDS:
            shutil.rmtree(directory)
        else:
            for child in directory.iterdir():
                if child.is_dir() and not child.is_symlink():
                    shutil.rmtree(child)


def covered_by_main(root=ROOT):
    """Exit status of the shared main-branch run's answer for HEAD: 0 once a pass contains it.

    The orchestrate script sets ORCHESTRATE_SCRIPT for every check it runs. Its statuses pass
    through unchanged: 1 for a commit not on main yet or a failure that held, 75 while a run
    is still under way.
    """
    script = os.environ.get('ORCHESTRATE_SCRIPT')
    if not script:
        print('exact baselines: `verify` reads the main-branch check, which needs the orchestrate script. '
              f'Run it as an item check, or ask directly: python3 <agent_loop.py> main-check {MAIN_CHECK}. '
              'To draw and compare here, for a planned refresh, hold the gpu lock and run verify --full.',
              file=sys.stderr)
        return 2
    return subprocess.run([sys.executable, script, 'main-check', MAIN_CHECK, '--worktree', str(root)]).returncode


def save(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def archive_manifest(directory, output):
    """Retain the complete result before removing successful reconstructible frames."""
    data = (directory / 'manifest.json').read_bytes()
    output.write_bytes(gzip.compress(data, mtime=0))
    shutil.rmtree(directory)


def archive_completed_runs(directory, output):
    """Release only captures whose successful result the unchanged owner published.

    Both owner matrices publish their top-level manifest after finishing a run's
    comparisons or motion verification. Neither reads completed runs again. Keep
    their identities and source manifests before deleting reconstructible pixels.
    An in-progress or failed capture is never removed.
    """
    try:
        report = json.loads((directory / 'manifest.json').read_text())
    except (OSError, json.JSONDecodeError):
        return
    if report.get('failures'):
        return
    for name, row in report.get('runs', {}).items():
        compared = any((key == name or key.startswith(name + '-')) and result.get('equal') is True
                       for key, result in report.get('comparisons', {}).items())
        if not row.get('complete') and not compared:
            continue
        capture = directory / name
        if not capture.is_dir() or capture.resolve().parent != directory.resolve():
            continue
        records = {}
        for path in capture.rglob('*.json'):
            records[str(path.relative_to(capture))] = json.loads(path.read_text())
        if (capture / 'route.jsonl').exists():
            records['route.jsonl'] = [json.loads(line) for line in (capture / 'route.jsonl').read_text().splitlines()]
        archive = output / f'{directory.name}-{name}-records.json.gz'
        archive.write_bytes(gzip.compress(json.dumps(records).encode(), mtime=0))
        shutil.rmtree(capture)


def verify(root=ROOT):
    if git('status', '--porcelain', '--untracked-files=all', root=root):
        raise ValueError('dirty tree: verifier requires a clean checkout (including untracked sources)')
    key = content_key(root)
    prune_scratch(root)
    if shutil.disk_usage(root).free < 16 * 1024**3:
        raise ValueError('LOW_DISK_SPACE: verifier requires 16 GiB free')
    output = root / SCRATCH / ('verify-' + datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%S.%fZ'))
    output.mkdir(parents=True)
    started = time.monotonic()
    report = {'passed': False, 'revision': git('rev-parse', 'HEAD', root=root), 'content_key': key,
              'target': 'mac-metal + chrome-webgpu', 'commands': [], 'comparisons': {},
              'diff_sha256': hashlib.sha256(subprocess.check_output(['git', 'diff', 'HEAD'], cwd=root)).hexdigest(),
              'load': os.getloadavg(), 'disk_free_before_bytes': shutil.disk_usage(root).free}
    environment = {k: v for k, v in os.environ.items() if not k.startswith('THRUM_') and k not in
                   ['CARGO_TARGET_DIR', 'RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC', 'RUSTC_WRAPPER']}
    environment['RUSTC_WRAPPER'] = ''

    def run(name, command, baseline=None):
        start = time.monotonic()
        log = output / (name + '.log')
        print(f'RUN {name}', flush=True)
        with log.open('w') as stream:
            result = subprocess.run(command, cwd=root, env=environment, stdout=stream,
                                    stderr=subprocess.STDOUT, timeout=3000)
            code = result.returncode
        report['commands'].append({'name': name, 'command': command, 'exit': code,
                                   'seconds': time.monotonic() - start, 'log': str(log.relative_to(root))})
        save(output / 'record.json', report)
        if code:
            if code == 75 and name in ('apple_tile_m1', 'apple_tile_m2'):
                manifest = json.loads((output / name / 'manifest.json').read_text())
                if manifest.get('could_not_judge'):
                    from capture_result import CaptureUnjudged
                    raise CaptureUnjudged(f"{name}: {manifest['incomplete_capture']}")
            identity = f"baseline {baseline['id']} owner {baseline['owner']}: " if baseline else ''
            detail = log.read_text()[-3000:]
            raise ValueError(f'{identity}comparison/stage {name} failed (exit {code}): {detail}')

    try:
        entries = registry(root)['baselines']
        report['references'] = {}
        # Fail distinctly on absent/unreadable references before drawing anything.
        for entry in entries:
            reference = root / entry['reference']
            files = [reference] if reference.is_file() else [root / p for p in
                     git('ls-files', '--', entry['reference'], root=root).splitlines()]
            if not files:
                raise ValueError(f"baseline {entry['id']} owner {entry['owner']}: unreadable reference {reference}")
            for path in files:
                try:
                    data = path.read_bytes()
                    if path.suffix == '.gz':
                        gzip.decompress(data)
                    report['references'][str(path.relative_to(root))] = hashlib.sha256(data).hexdigest()
                except (OSError, ValueError, EOFError, zlib.error) as error:
                    raise ValueError(f"baseline {entry['id']} owner {entry['owner']}: unreadable reference {path}: {error}") from error
        run('native-build', ['rustup', 'run', '1.99.1', 'cargo', 'build', '--release', '--locked', '--offline',
                             '-p', 'v1-game', '--features', 'draw-probe,thrum-render/creation-counters'])
        run('cooker-build', ['rustup', 'run', '1.99.1', 'cargo', 'build', '--release', '--locked', '--offline', '-p', 'thrum-cook'])
        run('web-build', ['sh', 'examples/v1-game/web/build.sh'])
        run('cook', ['target/release/thrum-cook', 'cook', 'examples/v1-game/assets/showcase', str(output / 'valley')])
        from run_v1_showcase import store_root
        report.update(native_binary_sha256=sha(root / 'target/release/v1-game'),
                      wasm_artifact_hashes={p.name: sha(p) for p in sorted((root / 'target/v1-game-web').iterdir()) if p.is_file()},
                      cooked_root=store_root(output / 'valley'),
                      compiler=git_version_command(['rustup', 'run', '1.99.1', 'rustc', '-Vv'], root),
                      host=git_version_command(['sw_vers'], root))
        native = entries[0]
        for script in native['comparisons']:
            name = Path(script).stem
            directory = output / name
            run(name, [sys.executable, 'scripts/exact_baselines_native.py', '--matrix', name,
                       '--binary', str(root / 'target/release/v1-game'), '--store', str(output / 'valley'),
                       '--output', str(directory), '--records', str(output)], native)
            manifest = json.loads((directory / 'manifest.json').read_text())
            if not manifest['passed']:
                raise ValueError(f"baseline {native['id']} owner {native['owner']}: {name} did not pass")
            report['comparisons'][name] = {'passed': True, 'comparisons': len(manifest['comparisons'])}
            archive_completed_runs(directory, output)
            archive_manifest(directory, output / (name + '-manifest.json.gz'))
        for entry in entries[1:]:
            name = entry['id']
            directory = output / name
            run(name, ['node', 'scripts/capture_v1_browser.mjs', '--mode', 'fixed', '--scene', entry['scene'],
                       '--source', entry['source'], '--cook', 'target/release/thrum-cook', '--output', str(directory)], entry)
            command = [sys.executable, 'scripts/apple_tile_m1_compare_browser.py', '--reference-digest',
                       '--reference', entry['reference'], '--candidate', str(directory),
                       '--output', str(output / (name + '-comparison.json'))]
            if entry['source'] == 'scene-linear':
                command.append('--linear')
            run(name + '-compare', command, entry)
            report['comparisons'][name] = {'passed': True, 'records': entry['records']}
            archive_manifest(directory, output / (name + '-manifest.json.gz'))
        if git('status', '--porcelain', '--untracked-files=all', root=root) or key != content_key(root):
            raise ValueError('checkout changed while verifier ran; run cannot pass')
        report.update(passed=True, baselines=[entry['id'] for entry in entries])
        report['seconds'] = time.monotonic() - started
        save(output / 'record.json', report)
        print(f'PASS exact baselines; record {output.relative_to(root)}/record.json; {report["seconds"]:.2f}s')
    except Exception as error:
        report.update(error=str(error), seconds=time.monotonic() - started)
        from capture_result import CaptureUnjudged
        if isinstance(error, CaptureUnjudged):
            report.update(could_not_judge=True, incomplete_capture=str(error))
        save(output / 'record.json', report)
        print(f'FAIL {error}\nRecord: {output / "record.json"}', file=sys.stderr)
        return 75 if report.get('could_not_judge') else 1
    return 0


def git_version_command(command, root):
    return subprocess.check_output(command, cwd=root, text=True).strip()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=['verify', 'guard'])
    parser.add_argument('--full', action='store_true',
                        help='with verify: draw and compare every registered baseline at this commit')
    args = parser.parse_args()
    try:
        if args.command == 'verify':
            return verify() if args.full else covered_by_main()
        start = time.monotonic()
        guard()
        print(f'Guard seconds: {time.monotonic() - start:.6f}; load: {os.getloadavg()}')
        return 0
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f'FAIL exact baselines: {error}', file=sys.stderr)
        return 1


if __name__ == '__main__':
    raise SystemExit(main())
