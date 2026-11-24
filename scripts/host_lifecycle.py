#!/usr/bin/env python3
"""Rebuild and run the Mac, Chrome and iPhone simulator lifecycle checks.

For direct Chrome or simulator runs, set ORCHESTRATE_SCRIPT to agent_loop.py.
"""

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
from host_overload import check_mac_overload
import simulator_lifecycle as simulator
import v1_simulator_route


ROOT = Path(__file__).resolve().parents[1]
MAC_WITNESS = ROOT / 'scripts/host_witness'
ASSETS = ROOT / 'examples/v1-game/assets/showcase'
PRESET = ROOT / 'examples/v1-game/params/showcase-noon.params.json'
WEB_CHECK = ROOT / 'scripts/check_v1_browser.mjs'
WEB_CASES = ('normal', 'controls', 'overload', 'device-loss', 'late-device-loss', 'device-recovery',
             'device-recovery-retry', 'device-recovery-acquire', 'valley-recovery', 'valley-late-attach')
COOKER = ROOT / 'target/debug/thrum-cook'
BUILD_INPUTS = ('Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', '.cargo',
                'crates', 'examples', 'tools')


def run(*arguments, cwd=ROOT, env=None):
    command = [str(argument) for argument in arguments]
    print(' '.join(command), flush=True)
    subprocess.run(command, cwd=cwd, env=env, check=True)


def prepare_mac(folder, revision, env, cooker):
    folder.mkdir()
    metadata = json.loads(subprocess.check_output(
        ['cargo', 'metadata', '--locked', '--offline', '--format-version', '1',
         '--filter-platform', 'aarch64-apple-darwin'], cwd=ROOT, env=env))
    wgpu = next(Path(package['manifest_path']).parent for package in metadata['packages']
                if package['name'] == 'wgpu' and package['version'] == '30.0.1')
    archive = folder / 'source'
    backend = folder / 'wgpu'
    with subprocess.Popen(['git', 'archive', revision, '--', *BUILD_INPUTS],
                          cwd=ROOT, stdout=subprocess.PIPE) as source:
        with tarfile.open(fileobj=source.stdout, mode='r|') as stream:
            stream.extractall(archive, filter='data')
        # tar may stop at the first end marker while Git is still writing padding.
        source.stdout.read()
        if source.wait():
            raise RuntimeError('could not archive the source revision')
    shutil.copytree(wgpu, backend)
    shutil.copyfile(MAC_WITNESS / 'loss_witness.rs',
                    archive / 'crates/thrum-app/src/app/loss_witness.rs')
    run(sys.executable, MAC_WITNESS / 'prepare_live_loss.py', archive, backend)
    run('cargo', 'build', '--offline', '--release', '-p', 'v1-game', '--bin', 'v1-game',
        '--target-dir', folder / 'target', cwd=archive, env=env)
    run('xcrun', 'swiftc', '-module-cache-path', folder / 'swift-cache', '-parse-as-library',
        MAC_WITNESS / 'record_owned_window.swift', '-o', folder / 'record_owned_window')
    run(cooker, 'cook', ASSETS, folder / 'store')


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
    check_mac_overload(folder, PRESET)


def prepare_web(folder, env):
    build = ROOT / 'examples/v1-game/web/build.sh'
    run(build, env=dict(env, THRUM_V1_WEB_OUTPUT=str(folder / 'v1-game-web')))
    run(build, '--dev', env=dict(env, THRUM_V1_WEB_OUTPUT=str(folder / 'v1-game-web-dev')))


def check_web(folder):
    folder.mkdir(parents=True, exist_ok=True)
    for case, web_root in [(case, 'v1-game-web') for case in WEB_CASES] + [
        ('material', 'v1-game-web-dev'),
    ]:
        run('node', WEB_CHECK, '--case', case, '--web-root', folder.parent / web_root,
            '--cook', folder.parent / 'thrum-cook', '--output', folder / case,
            '--verify-owned-shutdown')


def check_ios(folder, *, prepared=False):
    folder.mkdir(parents=True, exist_ok=True)
    env = dict(environment(), CARGO_TARGET_DIR=str(ROOT / 'target'),
               THRUM_V1_ROUTE_OUT=str(folder / 'route'))
    with simulator.supervise(fresh=True) as turn:
        if not prepared:
            turn.run(['bash', v1_simulator_route.ROUTE, '--prepare'], env=env)
        v1_simulator_route.worker(turn, None, env)
        turn.run([sys.executable, ROOT / 'scripts/host_overload.py', folder,
                  ROOT / 'target/v1-game-ios-sim/ThrumV1.app'], env=turn.worker_env(env))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['check', 'prepare', 'run-target'])
    parser.add_argument('--target', action='append', required=True,
                        choices=['mac-metal', 'chrome-webgpu', 'ios-sim'])
    parser.add_argument('--output', type=Path)
    parser.add_argument('--ios-build-prepared', action='store_true', help=argparse.SUPPRESS)
    parser.add_argument('--prepared', action='store_true',
                        help='only run the target checks from a previously prepared output')
    args = parser.parse_args()
    if args.action == 'run-target':
        # Internal callback for a target protected by check_locked.
        if args.output is None or len(args.target) != 1 or args.target[0] == 'mac-metal':
            parser.error('run-target requires one Chrome or simulator target and --output')
        if args.target[0] == 'ios-sim':
            check_ios(args.output, prepared=args.ios_build_prepared)
        else:
            check_web(args.output)
        return
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
        cooker = output / 'thrum-cook'
        if any(target != 'ios-sim' for target in args.target):
            run('cargo', 'build', '--locked', '--offline', '-p', 'thrum-cook', '--bin', 'thrum-cook', env=env)
            shutil.copy2(COOKER, cooker)
        if 'mac-metal' in args.target:
            prepare_mac(output / 'mac', revision, env, cooker)
        if 'chrome-webgpu' in args.target:
            prepare_web(output, env)
    if args.action == 'check':
        if 'mac-metal' in args.target:
            keep_awake = subprocess.Popen(['caffeinate', '-d', '-i'])
            try:
                check_mac(output / 'mac', revision)
            finally:
                keep_awake.terminate()
                keep_awake.wait()
        if 'chrome-webgpu' in args.target:
            # Wake the display as well as holding it awake so Chrome can drive visible RAF.
            run(sys.executable, simulator.orchestration_script(), 'lock', 'chrome', '--',
                'caffeinate', '-d', '-i', '-u', sys.executable, __file__, 'run-target',
                '--target', 'chrome-webgpu', '--output', output / 'chrome')
        if 'ios-sim' in args.target:
            holder, _ = simulator.admission()
            if holder:
                check_ios(output / 'ios')
            else:
                v1_simulator_route.prepare(dict(environment(), CARGO_TARGET_DIR=str(ROOT / 'target')))
                simulator.exec_with_admission([sys.executable, __file__, 'run-target',
                                               '--target', 'ios-sim', '--output', output / 'ios',
                                               '--ios-build-prepared'])


if __name__ == '__main__':
    main()
