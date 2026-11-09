#!/usr/bin/env python3
"""Qualify cooked content through the ordinary iOS simulator route (function only).

Run under agent_loop.py lock gpu. Owns and deletes a fresh simulator. Never skips.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import runpy
import shutil
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--lifecycle', action='store_true')
    parser.add_argument('--rotation', action='store_true')
    args = parser.parse_args()
    os.chdir(ROOT)
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    report = {'target': 'ios-sim', 'scope': 'simulator function only', 'commands': [], 'samples': []}
    device = None

    def run(command, env=None, timeout=180):
        print(' '.join(command), flush=True)
        result = subprocess.run(command, env=env, text=True, capture_output=True, timeout=timeout)
        report['commands'].append({'argv': command, 'exit': result.returncode, 'stdout': result.stdout, 'stderr': result.stderr})
        result.check_returncode()
        return result.stdout.strip()

    def console(label):
        raw = run(['ioreg', '-n', 'Root', '-d1', '-l'])
        (out / f'console-{label}.txt').write_text(raw)
        locked = '"IOConsoleLocked" = Yes' in raw
        report[f'console_locked_{label}'] = locked
        if not locked and not args.rotation:
            raise RuntimeError('locked-console proof required')

    try:
        if shutil.disk_usage(out).free < 2 * 1024**3:
            raise RuntimeError('needs 2 GiB free')
        report['commit'] = run(['git', 'rev-parse', 'HEAD'])
        report['diff_sha256'] = hashlib.sha256(run(['git', 'diff', 'HEAD']).encode()).hexdigest()
        report['host'] = run(['sw_vers'])
        console('before')
        env = dict(os.environ, RUSTC_WRAPPER='')
        run(['examples/v1-game-ios/ios/build-simulator.sh'], env, 300)
        store = out / 'store'
        rustc = run(['rustup', 'which', '--toolchain', '1.99.1', 'rustc'])
        env.update(RUSTC=rustc, PATH=str(Path(rustc).parent) + os.pathsep + env['PATH'])
        run(['cargo', 'run', '--locked', '--offline', '-p', 'thrum-cook', '--', 'cook', 'examples/v1-game/assets/showcase', str(store)], env, 300)
        report['content_hashes'] = {str(p.relative_to(store)): hashlib.sha256(p.read_bytes()).hexdigest() for p in store.rglob('*') if p.is_file()}
        device = run(['xcrun', 'simctl', 'create', 'Thrum COOKED-DRAW owned', 'com.apple.CoreSimulator.SimDeviceType.iPhone-18-Pro', 'com.apple.CoreSimulator.SimRuntime.iOS-27-2'])
        report['device'] = device
        run(['xcrun', 'simctl', 'bootstatus', device, '-b'])
        report['simulators'] = json.loads(run(['xcrun', 'simctl', 'list', 'devices', '-j']))
        app = Path('target/v1-game-ios-sim/ThrumV1.app').resolve()
        report['binary_sha256'] = hashlib.sha256((app / 'ThrumV1').read_bytes()).hexdigest()
        run(['xcrun', 'simctl', 'install', device, str(app)])
        container = Path(run(['xcrun', 'simctl', 'get_app_container', device, 'org.thrum.v1', 'data']))
        installed = container / 'tmp/cooked-draw-store'
        shutil.copytree(store, installed)
        state = container / 'tmp/thrum-v1-ios-state.txt'
        launch_env = dict(env, SIMCTL_CHILD_THRUM_V1_ROUTE='1', SIMCTL_CHILD_THRUM_SCENE='valley', SIMCTL_CHILD_THRUM_ASSET_STORE=str(installed), SIMCTL_CHILD_THRUM_STARS='off')
        run(['xcrun', 'simctl', 'launch', device, 'org.thrum.v1'], launch_env)

        def read():
            raw = state.read_text() if state.exists() else ''
            if raw.startswith('fault='):
                raise RuntimeError(raw)
            return dict(line.split('=', 1) for line in raw.splitlines() if '=' in line)

        def healthy(s):
            return s.get('cooked_path') == 'Some(Direct)' and s.get('cooked_drawn') == 'Some(1)' and s.get('cooked_errors') == '[]' and s.get('cooked_device_generation') == f"Some({s.get('device_generation')})"

        deadline = time.monotonic() + 60
        while not healthy(read()):
            if time.monotonic() >= deadline:
                raise RuntimeError(f'cooked set did not draw: {read()}')
            time.sleep(.1)
        previous = None
        for index in range(12):
            s = read()
            update_deadline = time.monotonic() + 10
            while previous and any(int(s.get(k, 0)) <= int(previous[k]) for k in ['presented', 'cooked_submitted']):
                if time.monotonic() >= update_deadline:
                    raise RuntimeError('stalled presented or cooked counter')
                time.sleep(.1)
                s = read()
            if not healthy(s) or 'INDIRECT_EXECUTION' in s.get('adapter', ''):
                raise RuntimeError(f'bad cooked state: {s}')
            if previous and any(int(s[k]) <= int(previous[k]) for k in ['presented', 'cooked_submitted']):
                raise RuntimeError('stalled presented or cooked counter')
            (out / f'state-{index:02}.json').write_text(json.dumps(s, indent=2))
            report['samples'].append({'monotonic': time.monotonic(), 'state': s})
            previous = s
            time.sleep(1)
        run(['xcrun', 'simctl', 'io', device, 'screenshot', str(out / 'valley-start.png')])
        if args.lifecycle or args.rotation:
            old_argv = sys.argv
            sys.argv = ['route.py', device, str(state)]
            os.environ['THRUM_V1_ROUTE_OUT'] = str(out / 'route')
            route = runpy.run_path('examples/v1-game-ios/ios/route.py')
            sys.argv = old_argv
            if args.lifecycle:
                before = read()
                original_snapshot = route['background'].__globals__['snapshot']
                def checked_snapshot(name, state, **kwargs):
                    if name.endswith('-background'):
                        current = read()
                        if any(current[k] != state[k] for k in ['presented', 'cooked_submitted', 'device_generation']):
                            raise RuntimeError('background submission or device publication')
                        report['background_held'] = {'before': state, 'after': current}
                    return original_snapshot(name, state, **kwargs)
                route['background'].__globals__['snapshot'] = checked_snapshot
                resumed = route['background'](before, 'cooked-background')
                if not healthy(resumed) or int(resumed['cooked_submitted']) <= int(before['cooked_submitted']):
                    raise RuntimeError('cooked background/foreground did not resume')
                report['resumed'] = resumed
                loss_base = read()
                route['post']('lose-device')
                route['wait'](route['recovered_after'](loss_base), 120, 'cooked replacement')
                deadline = time.monotonic() + 60
                while not healthy(read()):
                    if time.monotonic() >= deadline:
                        raise RuntimeError('cooked set did not restage after replacement')
                    time.sleep(.1)
                after = read()
                if int(after['lost_empty_submits']) or int(after['recover_failed']):
                    raise RuntimeError('loss submission or failed recovery')
                report['lifecycle'] = {'before': before, 'after': after}
            if args.rotation:
                run(['examples/v1-game-ios/ios/build-uitest.sh'], env, 300)
                before = read()
                after = route['rotation'](before)
                if not healthy(after) or int(after['cooked_submitted']) <= int(before['cooked_submitted']) or after['device_generation'] != before['device_generation']:
                    raise RuntimeError('cooked rotation continuity failed')
                report['rotation'] = {'before': before, 'after': after}
        console('after')
        report['pass'] = True
    except BaseException as error:
        report['failure'] = str(error)
        raise
    finally:
        if device:
            subprocess.run(['xcrun', 'simctl', 'terminate', device, 'org.thrum.v1'], capture_output=True, timeout=30)
            run(['xcrun', 'simctl', 'shutdown', device])
            run(['xcrun', 'simctl', 'delete', device])
        (out / 'report.json').write_text(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
