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


def wait_for_console_lock(observe, timeout=120):
    """Allow bounded manual setup, requiring a real locked observation before continuing."""
    if observe():
        return
    print(f'Mac console is unlocked. Waiting up to {timeout:g} seconds for manual locking.', flush=True)
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        time.sleep(.5)
        if observe():
            return
    raise RuntimeError('locked-console proof required. Please lock the Mac manually and rerun the check')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--lifecycle', action='store_true')
    parser.add_argument('--rotation', action='store_true')
    parser.add_argument('--visuals', action='store_true', help='held views and sampled pan; function only')
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
        def observe():
            raw = run(['ioreg', '-n', 'Root', '-d1', '-l'])
            (out / f'console-{label}.txt').write_text(raw)
            locked = '"IOConsoleLocked" = Yes' in raw
            report[f'console_locked_{label}'] = locked
            report.setdefault('console_observations', []).append(
                {'phase': label, 'monotonic': time.monotonic(), 'locked': locked})
            return locked

        if args.rotation:
            observe()
        else:
            wait_for_console_lock(observe, timeout=120 if label == 'before' else 0)

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
        if args.visuals:
            from cooked_draw_fixture import sources, cook
            sources(out / 'source', env)
            cook(out / 'source', store, env)
        else:
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
        if args.visuals:
            if args.lifecycle or args.rotation:
                raise RuntimeError('held visual proof is separate from ordinary lifecycle/rotation proof')
            launch_env.update(SIMCTL_CHILD_THRUM_COOKED_DRAW_HELD='1')
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
        if args.lifecycle or args.rotation or args.visuals:
            old_argv = sys.argv
            sys.argv = ['route.py', device, str(state)]
            os.environ['THRUM_V1_ROUTE_OUT'] = str(out / 'route')
            route = runpy.run_path('examples/v1-game-ios/ios/route.py')
            sys.argv = old_argv
            original_extent_proof = route['extent_proof']
            selected_fov = json.loads((ROOT / 'examples/v1-game/assets/showcase/scene.json').read_text())['camera']['vertical_fov_degrees']
            route['rotate'].__globals__['extent_proof'] = lambda s: original_extent_proof(s, selected_fov)
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
            if args.visuals:
                report['views'] = []
                def shot(name):
                    before = read()
                    began = time.monotonic()
                    run(['xcrun', 'simctl', 'io', device, 'screenshot', str(out / f'{name}.png')])
                    row = {'name': name, 'begin': began, 'end': time.monotonic(), 'before': before, 'after': read()}
                    if not healthy(row['after']) or int(row['after']['world_tick']) != 0:
                        raise RuntimeError(f'held view is unhealthy or world advanced: {row}')
                    report['views'].append(row)
                    return row
                def pair(name):
                    shot(name + '-a')
                    time.sleep(.5)
                    shot(name + '-b')
                pair('noon-0')
                base = read()
                route['post']('cooked-pan')
                route['wait'](lambda s: float(s.get('cooked_pan_seconds', 0)) > 0, 15, 'cooked pan started')
                pan = []
                for index in range(16):
                    pan.append(shot(f'pan-{index:02}'))
                    time.sleep(.6)
                route['wait'](lambda s: float(s.get('cooked_yaw', 0)) == 36, 15, 'cooked pan end hold')
                pair('noon-36')
                report['pan'] = {'before_command': base, 'after_hold': read(), 'samples': pan,
                                 'largest_begin_gap_seconds': max(b['begin'] - a['begin'] for a,b in zip(pan,pan[1:])),
                                 'scope': 'simctl compositor samples only; witness counters bracket observations, not exact frame IDs; no conclusion about unsampled frames, speed or cadence',
                                 'start_hold_seconds': 2, 'pan_seconds': 8,
                                 'end_hold_observed_min_seconds': max(0, float(read()['cooked_pan_seconds']) - 10)}
                # Relaunch the same installed binary for the start camera's orientation proof.
                def relaunch(scene='valley', solar='12'):
                    run(['xcrun', 'simctl', 'terminate', device, 'org.thrum.v1'])
                    state.unlink(missing_ok=True)
                    view_env = dict(launch_env, SIMCTL_CHILD_THRUM_SCENE=scene,
                                    SIMCTL_CHILD_THRUM_COOKED_DRAW_SOLAR=solar,
                                    SIMCTL_CHILD_THRUM_COOKED_DRAW_DIAGNOSTIC='1')
                    run(['xcrun', 'simctl', 'launch', device, 'org.thrum.v1'], view_env)
                    route['wait'](healthy, 60, 'fresh held cooked view')
                    time.sleep(3)
                relaunch()
                run(['examples/v1-game-ios/ios/build-uitest.sh'], env, 300)
                landscape = route['rotate']('12-landscape', 'testLandscape', read(), landscape=True)
                pair('landscape')
                portrait = route['rotate']('13-portrait', 'testPortrait', landscape, landscape=False)
                pair('portrait')
                report['visual_rotation'] = {'landscape': landscape, 'portrait': portrait}
                relaunch(solar='6.5')
                pair('low-sun')
                relaunch(scene='diagnostic')
                pair('diagnostic')
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
