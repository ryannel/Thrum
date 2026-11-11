#!/usr/bin/env python3
"""Verify registered exact images, or enforce their content-key proof without a GPU.

Run verify under agent_loop.py lock gpu. Its compact records and untracked proofs
live in tmp/baseline-guard; successful bulk captures are discarded after comparison.
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


def git(*args, root=ROOT):
    return subprocess.check_output(['git', *args], cwd=root).decode().strip()


def registry(root=ROOT):
    return json.loads((root / REGISTRY).read_text())


def content_key(root=ROOT):
    paths = registry(root)['covered_paths']
    objects = subprocess.check_output(['git', 'ls-tree', '-r', '-z', 'HEAD', '--', *paths], cwd=root)
    return hashlib.sha256(json.dumps(paths).encode() + b'\0' + objects).hexdigest()


def proof_path(root=ROOT):
    return root / SCRATCH / 'proofs' / (content_key(root) + '.json')


def gate(root=ROOT):
    """Fail closed when remote main or the exact covered-content proof is absent."""
    paths = registry(root)['covered_paths']
    if git('status', '--porcelain', '--untracked-files=all', '--', *paths, root=root):
        raise ValueError('covered paths have uncommitted changes; commit before verifying')
    base = git('merge-base', 'HEAD', 'refs/remotes/origin/main', root=root)
    changed = git('diff', '--name-only', base, 'HEAD', '--', *paths, root=root)
    if not changed:
        print('PASS exact-baseline gate: no covered-path change beyond origin/main')
        return
    path = proof_path(root)
    try:
        proof = json.loads(path.read_text())
    except (OSError, ValueError) as error:
        raise ValueError('no exact-baseline proof for this content; run under the GPU lock: '
                         'python3 scripts/exact_baselines.py verify') from error
    expected = [entry['id'] for entry in registry(root)['baselines']]
    if (proof.get('content_key') != content_key(root) or proof.get('passed') is not True
            or proof.get('baselines') != expected):
        raise ValueError('invalid exact-baseline proof; run python3 scripts/exact_baselines.py verify')
    print(f'PASS exact-baseline gate: proof {path.relative_to(root)}')


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
    proof = proof_path(root)
    # A failed attempt must not leave a previous proof for the attempted content.
    proof.unlink(missing_ok=True)
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
            raise ValueError('checkout changed while verifier ran; no proof written')
        report.update(passed=True, baselines=[entry['id'] for entry in entries])
        report['seconds'] = time.monotonic() - started
        save(output / 'record.json', report)
        proof.parent.mkdir(parents=True, exist_ok=True)
        # Atomic publication only after all native and Chrome comparisons passed.
        pending = proof.with_suffix('.pending')
        save(pending, dict(report, record=str((output / 'record.json').relative_to(root))))
        pending.replace(proof)
        print(f'PASS exact baselines; proof {proof.relative_to(root)}; {report["seconds"]:.2f}s')
    except Exception as error:
        report.update(error=str(error), seconds=time.monotonic() - started)
        save(output / 'record.json', report)
        print(f'FAIL {error}\nRecord: {output / "record.json"}', file=sys.stderr)
        return 1
    return 0


def git_version_command(command, root):
    return subprocess.check_output(command, cwd=root, text=True).strip()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=['verify', 'gate'])
    args = parser.parse_args()
    try:
        if args.command == 'verify':
            return verify()
        start = time.monotonic()
        gate()
        print(f'Gate seconds: {time.monotonic() - start:.6f}; load: {os.getloadavg()}')
        return 0
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f'FAIL exact baselines: {error}', file=sys.stderr)
        return 1


if __name__ == '__main__':
    raise SystemExit(main())
