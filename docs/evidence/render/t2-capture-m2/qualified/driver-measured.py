#!/usr/bin/env python3
"""Capture actual iOS simulator submissions with bounded lossless output.

Run under agent_loop.py lock gpu. Console state is a witness. Attributed denied
Screen Recording preflights are required by the actual parent launcher.
"""
import argparse
import hashlib
import json
import math
import os
import runpy
from pathlib import Path
import shutil
import subprocess
import sys
import time

from cooked_draw_ios_sim import console_witness
from cooked_draw_sim_launcher import REPORT_ENV, launch

ROOT = Path(__file__).resolve().parents[1]


def wait(read, predicate, label, timeout=40):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        value = read()
        if predicate(value):
            return value
        time.sleep(.03)
    raise RuntimeError(f'{label} timed out, last observation {value}')


def load(path):
    try:
        return json.loads(path.read_text())
    except (FileNotFoundError, json.JSONDecodeError):
        return {}


def validate_session(runtime, negative=False):
    images = runtime['images']
    orders = [row['submission_id'] for row in images]
    if len(orders) != len(set(orders)) or orders != sorted(orders):
        raise ValueError('misordered or duplicate capture identity')
    if negative:
        if runtime['complete']:
            raise ValueError('incomplete control incorrectly completed')
        return
    if not runtime['complete'] or not images or runtime['expected_submissions'] != len(images):
        raise ValueError('incomplete simulator evidence')
    if any(b != a + 1 for a, b in zip(orders, orders[1:])):
        raise ValueError('missing submitted image')
    if any(b['submit_ns'] <= a['submit_ns'] for a, b in zip(images, images[1:])):
        raise ValueError('nonmonotonic submission clock')
    for row in images:
        if not row['surface_presented'] or row['map_ns'] < row['submit_ns']:
            raise ValueError('image origin/time mismatch')
    submitted = sum(bool(row.get('report', {}).get('submitted'))
                    for row in runtime['opportunities'] if row['phase'] == 'capturing' and row.get('report'))
    if submitted != len(images):
        raise ValueError('opportunity/image coverage mismatch')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--case', choices=['all', 'off', 'pan', 'running', 'stall', 'surface', 'background', 'loss', 'saturation', 'slow', 'ui-regression'], default='all')
    parser.add_argument('--device', help='continue this task’s already owned simulator boot')
    parser.add_argument('--no-build', action='store_true', help='use the current local app and cooker')
    args = parser.parse_args()
    os.chdir(ROOT)
    if REPORT_ENV not in os.environ:
        launch(sys.argv[1:], args.output, driver=Path(__file__).resolve())
        return
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    report = {'target': 'ios-sim', 'scope': 'simulator function only', 'harness_pid': os.getpid(),
              'launcher_pid': os.getppid(), 'commands': [], 'sessions': [], 'route_pass': False,
              'session_output_allowance_bytes': 20 * 1024**3}
    device = None

    def run(command, env=None, timeout=180):
        print(' '.join(command), flush=True)
        result = subprocess.run(command, env=env, capture_output=True, text=True, timeout=timeout)
        report['commands'].append({'argv': command, 'exit': result.returncode,
                                   'stdout': result.stdout, 'stderr': result.stderr})
        result.check_returncode()
        return result.stdout.strip()

    def console(phase):
        try:
            raw = run(['ioreg', '-n', 'Root', '-d1', '-l'])
            observation = console_witness(raw)
        except (OSError, subprocess.SubprocessError, UnicodeError) as error:
            observation = console_witness('', str(error))
        report.setdefault('console_observations', []).append(dict(observation, phase=phase, ns=time.monotonic_ns()))

    try:
        if shutil.disk_usage(out).free < 20 * 1024**3:
            raise RuntimeError('20 GiB native-resolution session allowance unavailable')
        preflight = load(Path(os.environ[REPORT_ENV]))
        if preflight.get('screen_recording_before') is not False or preflight.get('launcher_pid') != os.getppid():
            raise RuntimeError('denied preflight from actual parent launcher required')
        console('before')
        report['commit'] = run(['git', 'rev-parse', 'HEAD'])
        report['diff_sha256'] = hashlib.sha256(run(['git', 'diff', 'HEAD']).encode()).hexdigest()
        report['host'] = run(['sw_vers'])
        report['load_before'] = run(['uptime'])
        report['source_files'] = {}
        for folder in ['crates', 'examples/v1-game', 'examples/v1-game-ios', 'scripts']:
            for path in sorted(Path(folder).rglob('*')):
                if path.is_file() and path.suffix in ('.rs', '.wgsl', '.toml', '.swift', '.py', '.json'):
                    report['source_files'][str(path)] = hashlib.sha256(path.read_bytes()).hexdigest()
        env = dict(os.environ, RUSTC_WRAPPER='')
        rustc = run(['rustup', 'which', '--toolchain', '1.99.1', 'rustc'])
        env.update(RUSTC=rustc, PATH=str(Path(rustc).parent) + os.pathsep + env['PATH'])
        if not args.no_build:
            run(['examples/v1-game-ios/ios/build-simulator.sh'], env, 600)
            run(['cargo', 'build', '--locked', '--offline', '-p', 'thrum-cook'], env, 600)
        store = out / 'store'
        from cooked_draw_fixture import sources, cook
        cooker = ROOT / 'target/debug/thrum-cook'
        report['cooker_sha256'] = hashlib.sha256(cooker.read_bytes()).hexdigest()
        sources(out / 'source', env, cooker=cooker)
        cook(out / 'source', store, env, cooker=cooker)
        report['content_hashes'] = {str(p.relative_to(store)): hashlib.sha256(p.read_bytes()).hexdigest()
                                    for p in store.rglob('*') if p.is_file()}
        app = Path('target/v1-game-ios-sim/ThrumV1.app').resolve()
        report['binary_sha256'] = hashlib.sha256((app / 'ThrumV1').read_bytes()).hexdigest()
        device = args.device or run(['xcrun', 'simctl', 'create', 'Thrum T2 capture owned',
                      'com.apple.CoreSimulator.SimDeviceType.iPhone-18-Pro', 'com.apple.CoreSimulator.SimRuntime.iOS-27-2'])
        report['device'] = device
        run(['xcrun', 'simctl', 'bootstatus', device, '-b'], timeout=600)
        report['simulators'] = json.loads(run(['xcrun', 'simctl', 'list', 'devices', '-j']))
        run(['xcrun', 'simctl', 'install', device, str(app)])
        # A sampled fresh-device launch blocked in SpringBoard's synchronous IconServices request.
        # Reset only this owned device's icon worker before the first app launch.
        run(['xcrun', 'simctl', 'spawn', device, 'launchctl', 'kickstart', '-k',
             'system/com.apple.iconservices.iconservicesagent'])
        container = Path(run(['xcrun', 'simctl', 'get_app_container', device, 'org.thrum.v1', 'data']))
        temp = container / 'tmp'
        installed = temp / 't2-cooked'
        shutil.copytree(store, installed)
        witness = temp / 'thrum-v1-ios-state.txt'
        commands = temp / 'thrum-v1-commands'
        sequence = 0
        actions = []

        def state():
            error = temp / 'thrum-capture-error.txt'
            if error.exists():
                raise RuntimeError(error.read_text())
            raw = witness.read_text() if witness.exists() else ''
            if raw.startswith('fault='):
                raise RuntimeError(raw)
            return dict(line.split('=', 1) for line in raw.splitlines() if '=' in line)

        def post(name):
            nonlocal sequence
            sequence += 1
            commands.mkdir(exist_ok=True)
            partial = temp / 'command.partial'
            partial.write_text(name)
            partial.rename(commands / f'{sequence:06}-{name}')
            actions.append({'name': name, 'host_ns': time.monotonic_ns()})

        def healthy(s):
            return (s.get('cooked_path') == 'Some(Direct)' and s.get('cooked_drawn') == 'Some(1)'
                    and s.get('cooked_errors') == '[]' and int(s.get('presented', 0)) >= 120)

        def observe(path):
            state()  # Capture faults fail immediately, rather than becoming observation timeouts.
            return load(path)

        def finish(path, destination, negative=False):
            post('capture-end')
            runtime = wait(lambda: observe(path / 'runtime.json'), bool, 'bounded capture finalization', 25)
            validate_session(runtime, negative)
            shutil.copytree(path, destination)
            return runtime

        def snapshot_actor():
            before = int(state().get('actor_samples', 0))
            post('sample-actor')
            return wait(state, lambda s: int(s.get('actor_samples', 0)) > before, 'committed actor sample')

        cases = ['off', 'pan', 'pan', 'pan', 'running', 'stall', 'surface', 'background', 'loss', 'saturation', 'slow', 'ui-regression'] if args.case == 'all' else [args.case]
        for index, case in enumerate(cases):
            name = f'{index:02}-{case}'
            session_out = out / name
            session_out.mkdir()
            actions = []
            for path in temp.glob('thrum-capture-*'):
                if path.is_dir():
                    shutil.rmtree(path)
                else:
                    path.unlink()
            shutil.rmtree(commands, ignore_errors=True)
            witness.unlink(missing_ok=True)
            launch_env = dict(env, SIMCTL_CHILD_THRUM_V1_ROUTE='1', SIMCTL_CHILD_THRUM_SCENE='diagnostic' if case == 'running' else 'valley',
                              SIMCTL_CHILD_THRUM_ASSET_STORE=str(installed), SIMCTL_CHILD_THRUM_STARS='off')
            if case == 'pan':
                launch_env['SIMCTL_CHILD_THRUM_COOKED_DRAW_HELD'] = '1'
            launched = run(['xcrun', 'simctl', 'launch', device, 'org.thrum.v1'], launch_env)
            try:
                before = wait(state, healthy, 'native cooked foreground warmup', 30)
            except RuntimeError:
                if state():
                    before = wait(state, healthy, 'continuing cooked warmup', 150)
                else:
                    pid = launched.rsplit(':', 1)[-1].strip()
                    report.setdefault('startup_processes', []).append(run(['ps', '-p', pid, '-o', 'pid,stat,etime,comm']))
                    # A second launch foregrounds the existing process after fresh-device bootstrap.
                    run(['xcrun', 'simctl', 'launch', device, 'org.thrum.v1'], launch_env)
                    before = wait(state, healthy, 'foreground after simulator bootstrap', 150)
            row = {'case': case, 'name': name, 'before': before, 'actions': actions}
            (session_out / 'before.json').write_text(json.dumps(before, indent=2))
            if case in ('off', 'ui-regression'):
                if case == 'ui-regression':
                    run(['examples/v1-game-ios/ios/build-uitest.sh'], env, 300)
                    previous_argv = sys.argv
                    sys.argv = ['route.py', device, str(witness)]
                    os.environ['THRUM_V1_ROUTE_OUT'] = str(session_out / 'route')
                    route = runpy.run_path('examples/v1-game-ios/ios/route.py')
                    sys.argv = previous_argv
                    extent_proof = route['extent_proof']
                    fov = json.loads(Path('examples/v1-game/assets/showcase/scene.json').read_text())['camera']['vertical_fov_degrees']
                    route['rotate'].__globals__['extent_proof'] = lambda s: extent_proof(s, fov)
                    touched = route['touches'](state())
                    after = route['rotation'](touched)
                    row.update(input='real UIKit gestures from the existing XCUITest route',
                               scope='supplemental simulator UI/lifecycle function; source engine tap is disabled')
                else:
                    time.sleep(3)
                    after = wait(state, lambda s: int(s.get('presented', 0)) > int(before['presented']), 'capture-off progress')
                if list(temp.glob('thrum-capture-*')):
                    raise RuntimeError('disabled capture allocated artifacts')
                row.update(after=after, pass_result=True)
            else:
                mode = 'capture-saturate' if case in ('surface', 'background', 'loss', 'saturation') else 'capture-slow' if case == 'slow' else 'capture-begin'
                post(mode)
                path = temp / 'thrum-capture-1'
                wait(lambda: observe(path / 'pending.json'), bool, 'capture start')
                if case in ('surface', 'background', 'loss'):
                    pending = wait(lambda: observe(path / 'pending.json'), lambda p: p.get('pending', [0])[0] >= 3, 'pending readbacks')
                    row['pending_before_transition'] = pending
                    if case == 'background':
                        run(['xcrun', 'simctl', 'launch', device, 'com.apple.Preferences'])
                        suspended = wait(state, lambda s: s.get('state') == 'Suspended' and s.get('world_paused') == 'true', 'background world pause')
                        time.sleep(1)
                        held = state()
                        time.sleep(1)
                        held_after = state()
                        if any(held[k] != held_after[k] for k in ['presented', 'cooked_submitted', 'world_tick', 'device_generation']):
                            raise RuntimeError('background rendering or world advance')
                        row['background'] = {'suspended': suspended, 'before': held, 'after': held_after,
                                             'pending': load(path / 'pending.json')}
                        run(['xcrun', 'simctl', 'launch', device, 'org.thrum.v1'])
                    else:
                        post('replace-surface' if case == 'surface' else 'lose-device')
                    retired = wait(lambda: observe(path / 'runtime.json'), bool, 'retired pending session', 90)
                    validate_session(retired, negative=True)
                    if not retired['cancel_reason']:
                        raise RuntimeError('retired lease did not cancel capture')
                    shutil.copytree(path, session_out / 'retired')
                    recovered = wait(state, lambda s: healthy(s) and int(s.get('presented_epoch', 0)) > int(before['epoch'])
                        and (case != 'loss' or int(s.get('recoveries', 0)) > int(before['recoveries'])), 'foreground recovery', 120)
                    if recovered['lost_empty_submits'] != '0' or recovered['recover_failed'] != '0':
                        raise RuntimeError('lost-queue submission or recovery failure')
                    row['recovered'] = recovered
                    post('capture-begin')
                    path = temp / 'thrum-capture-2'
                    wait(lambda: observe(path / 'pending.json'), bool, 'fresh capture generation')
                if case == 'pan':
                    post('cooked-pan')
                    time.sleep(12.2)
                elif case == 'running':
                    a = snapshot_actor()
                    time.sleep(1)
                    post('capture-move')
                    time.sleep(2)
                    post('capture-rest')
                    b = snapshot_actor()
                    row['actor'] = {'before': a, 'after': b}
                    positions = [[float(v) for v in s['actor_position'].split(',')] for s in (a, b)]
                    if math.dist(*positions) < 1:
                        raise RuntimeError('normal semantic control did not move the committed actor')
                    time.sleep(1.2)
                elif case == 'stall':
                    time.sleep(1)
                    post('stall-render')
                    wait(state, lambda s: int(s.get('stalls', 0)) > int(before['stalls']), 'actual eight-second stall', 20)
                    time.sleep(1.2)
                else:
                    time.sleep(5 if case == 'slow' else 3)
                negative = case in ('saturation', 'slow')
                runtime = finish(path, session_out / 'capture', negative)
                if negative and not (runtime['renderer_omissions'] or runtime['writer_omissions'] or runtime['writer_error']):
                    raise RuntimeError('saturation/slow control did not exercise refusal')
                if not negative:
                    identity = load(session_out / 'capture/identity.json')
                    if any(image['surface_epoch'] != identity['surface_epoch'] or image['device_generation'] != identity['device_generation']
                           for image in runtime['images']):
                        raise RuntimeError('stale image published in fresh session')
                if case == 'stall':
                    gaps = [b['submit_ns'] - a['submit_ns'] for a, b in zip(runtime['images'], runtime['images'][1:])]
                    row['max_image_gap_ns'] = max(gaps)
                    if row['max_image_gap_ns'] < 7_500_000_000:
                        raise RuntimeError('elapsed stall was compressed out of source time')
                row.update(complete=runtime['complete'], pass_result=True, images=len(runtime['images']),
                           after=state(), expected=runtime['expected_submissions'],
                           renderer_omissions=runtime['renderer_omissions'], writer_omissions=runtime['writer_omissions'])
                if int(row['after']['world_tick']) < int(before['world_tick']):
                    raise RuntimeError('authority moved backward')
                if negative and int(row['after']['world_tick']) <= int(before['world_tick']):
                    raise RuntimeError('world stopped during capture saturation')
            (session_out / 'result.json').write_text(json.dumps(row, indent=2) + '\n')
            report['sessions'].append(row)
            run(['xcrun', 'simctl', 'terminate', device, 'org.thrum.v1'])
            console(name)
        console('after')
        report['load_after'] = run(['uptime'])
        report['route_pass'] = True
    except BaseException as error:
        report['failure'] = str(error)
        raise
    finally:
        if device:
            for command in [['terminate', device, 'org.thrum.v1'], ['shutdown', device], ['delete', device]]:
                try:
                    result = subprocess.run(['xcrun', 'simctl', *command], capture_output=True, text=True, timeout=90)
                    report.setdefault('cleanup', []).append({'argv': command, 'exit': result.returncode, 'stderr': result.stderr})
                    if result.returncode and command[0] != 'terminate':
                        report['route_pass'] = False
                except subprocess.SubprocessError as error:
                    report.setdefault('cleanup', []).append({'argv': command, 'error': str(error)})
                    report['route_pass'] = False
        (out / 'report.json').write_text(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
