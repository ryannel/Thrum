#!/usr/bin/env python3
"""Rebuild and run the existing Mac and Chrome device lifecycle checks."""

import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile

from cooked_draw_fixture import environment


ROOT = Path(__file__).resolve().parents[1]
MAC_WITNESS = ROOT / 'docs/evidence/HOST/mac-window-check'
ASSETS = ROOT / 'examples/v1-game/assets/showcase'
PRESET = ROOT / 'examples/v1-game/params/showcase-noon.params.json'
WEB_CHECK = ROOT / 'scripts/check_v1_browser.mjs'
WEB_CASES = ('normal', 'device-loss', 'late-device-loss', 'device-recovery',
             'device-recovery-retry', 'valley-recovery', 'valley-late-attach')
COOKER = ROOT / 'target/debug/thrum-cook'


def run(*arguments, cwd=ROOT, env=None):
    command = [str(argument) for argument in arguments]
    print(' '.join(command), flush=True)
    subprocess.run(command, cwd=cwd, env=env, check=True)


def prepare_mac(folder, revision, env):
    folder.mkdir()
    metadata = json.loads(subprocess.check_output(
        ['cargo', 'metadata', '--locked', '--offline', '--format-version', '1',
         '--filter-platform', 'aarch64-apple-darwin'], cwd=ROOT, env=env))
    wgpu = next(Path(package['manifest_path']).parent for package in metadata['packages']
                if package['name'] == 'wgpu' and package['version'] == '30.0.1')
    archive = folder / 'source'
    backend = folder / 'wgpu'
    with subprocess.Popen(['git', 'archive', revision], cwd=ROOT, stdout=subprocess.PIPE) as source:
        with tarfile.open(fileobj=source.stdout, mode='r|') as stream:
            stream.extractall(archive, filter='data')
        if source.wait():
            raise RuntimeError('could not archive the source revision')
    shutil.copytree(wgpu, backend)
    shutil.copyfile(MAC_WITNESS / 'm1_loss_witness.rs',
                    archive / 'crates/thrum-app/src/app/m1_loss_witness.rs')
    run(sys.executable, MAC_WITNESS / 'prepare_live_loss.py', archive, backend)
    run('cargo', 'build', '--offline', '--release', '-p', 'v1-game', '--bin', 'v1-game',
        '--target-dir', folder / 'target', cwd=archive, env=env)
    run('xcrun', 'swiftc', '-module-cache-path', folder / 'swift-cache', '-parse-as-library',
        MAC_WITNESS / 'record_owned_window.swift', '-o', folder / 'record_owned_window')
    run(COOKER, 'cook', ASSETS, folder / 'store')


def check_mac(folder, revision):
    recorder = folder / 'record_owned_window'
    run(recorder, '--preflight')
    for row in ('control', 'paused', 'running'):
        run('caffeinate', '-d', '-i', sys.executable, MAC_WITNESS / 'run_live_witness.py',
            row, folder / row, '--owner-window', '--store', folder / 'store',
            '--binary', folder / 'target/release/v1-game', '--preset', PRESET,
            '--recorder', recorder, '--revision', revision)
    report = subprocess.check_output(
        [sys.executable, str(MAC_WITNESS / 'audit_live_witness.py'), str(folder)], cwd=ROOT, text=True)
    (folder / 'audit.json').write_text(report)
    print(report, end='')


def prepare_web(env):
    build = ROOT / 'examples/v1-game/web/build.sh'
    run(build, env=env)
    run(build, '--dev', env=env)


def check_web(folder):
    for case, web_root in [(case, 'v1-game-web') for case in WEB_CASES] + [
        ('material', 'v1-game-web-dev'),
    ]:
        run('node', WEB_CHECK, '--case', case, '--web-root', ROOT / 'target' / web_root,
            '--cook', COOKER, '--output', folder / case)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['check', 'prepare'])
    parser.add_argument('--target', action='append', required=True, choices=['mac-metal', 'chrome-webgpu'])
    parser.add_argument('--output', type=Path)
    parser.add_argument('--prepared', action='store_true',
                        help='only run the target checks from a previously prepared output')
    args = parser.parse_args()
    if args.prepared and (args.output is None or args.action != 'check'):
        parser.error('--prepared requires check and an explicit --output')
    scratch = ROOT / 'tmp'
    scratch.mkdir(exist_ok=True)
    if args.output:
        output = args.output.resolve()
        if not output.is_relative_to(scratch) or output == scratch:
            parser.error('--output must be inside the repository tmp/ directory')
        if not args.prepared:
            output.mkdir(parents=True)
    else:
        output = Path(tempfile.mkdtemp(prefix='host-lifecycle-', dir=scratch))
    print(f'Lifecycle evidence directory {output}', flush=True)
    revision = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
    revision_file = output / 'revision'
    if args.prepared and revision_file.read_text().strip() != revision:
        raise RuntimeError('prepared checkout revision differs from HEAD')
    if not args.prepared:
        revision_file.write_text(revision + '\n')
        env = dict(environment(), CARGO_TARGET_DIR=str(ROOT / 'target'))
        run('cargo', 'build', '--locked', '--offline', '-p', 'thrum-cook', '--bin', 'thrum-cook', env=env)
        if 'mac-metal' in args.target:
            prepare_mac(output / 'mac', revision, env)
        if 'chrome-webgpu' in args.target:
            prepare_web(env)
    if args.action == 'check':
        if 'mac-metal' in args.target:
            check_mac(output / 'mac', revision)
        if 'chrome-webgpu' in args.target:
            check_web(output / 'chrome')


if __name__ == '__main__':
    main()
