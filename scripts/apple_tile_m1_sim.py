#!/usr/bin/env python3
"""Run APPLE-TILE M1 diagnostic simulator function and lifecycle evidence.

Uses an owned iPhone 18 Pro instance and the existing shell route's lease checks.
Build the simulator app first. Run under the repository GPU lock.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import runpy
import subprocess
import sys
import time


def native_background_race(route, base, device):
    """Hold an encoded native frame, then observe its refusal while suspended."""
    num = route['num']
    route['start_stall']()
    held = route['read']()
    route['simctl']('launch', device, 'com.apple.Preferences')
    paused = route['wait'](
        lambda s: s.get('state') == 'Suspended' and
            num(s, 'pause_replies') > num(base, 'pause_replies'),
        20, 'native-race world pause')
    refused = route['wait'](
        lambda s: s.get('state') == 'Suspended' and
            num(s, 'stalls') > num(base, 'stalls'),
        20, 'encoded native frame refusal while backgrounded')
    for counter in ['native_submitted', 'empty_submits', 'presented']:
        if num(refused, counter) != num(held, counter):
            raise ValueError(f'background native race advanced {counter}')
    if num(refused, 'native_discarded') <= num(held, 'native_discarded'):
        raise ValueError('background lease did not discard its encoded native candidate')
    route['snapshot']('m10-native-refused-background', refused)
    time.sleep(1)
    stable = route['read']()
    for counter in ['native_encoded', 'native_submitted', 'empty_submits', 'presented']:
        if num(stable, counter) != num(refused, counter):
            raise ValueError(f'background native witness was not quiescent: {counter}')
    route['foreground']()
    resumed = route['wait'](
        lambda s: num(s, 'resume_replies') > num(base, 'resume_replies') and
            route['presents_after'](refused)(s) and route['settled'](s),
        20, 'native provider reset and presentation after resume')
    if resumed['pid'] != base['pid'] or resumed['resume_epoch'] != paused['pause_epoch']:
        raise ValueError('native background race replaced the world/process')
    if resumed['resume_tick'] != paused['pause_tick']:
        raise ValueError('native background race advanced the paused world')
    route['snapshot']('m10-native-resumed', resumed)
    return resumed, {'held': held, 'paused': paused, 'refused': refused,
                     'stable': stable, 'resumed': resumed}


def run(output, native_witness=False):
    output = output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    report = {'passed': False, 'target': 'ios-sim', 'scope': 'diagnostic endpoint and lifecycle function only',
              'commands': [], 'started_monotonic': time.monotonic()}
    device = None

    def command(args, env=None, check=True):
        result = subprocess.run(args, env=env, text=True, capture_output=True, timeout=180)
        report['commands'].append({'argv': args, 'exit': result.returncode, 'stdout': result.stdout, 'stderr': result.stderr})
        if check:
            result.check_returncode()
        return result.stdout.strip()

    try:
        report['revision'] = command(['git', 'rev-parse', 'HEAD'])
        report['diff_sha256'] = hashlib.sha256(subprocess.check_output(['git', 'diff', 'HEAD'])).hexdigest()
        binary = Path('target/v1-game-ios-sim/ThrumV1.app').resolve()
        report['binary_sha256'] = hashlib.sha256((binary / 'ThrumV1').read_bytes()).hexdigest()
        device = command(['xcrun', 'simctl', 'create', 'Thrum APPLE-TILE M1 owned', 'com.apple.CoreSimulator.SimDeviceType.iPhone-18-Pro', 'com.apple.CoreSimulator.SimRuntime.iOS-27-2'])
        report['device'] = device
        command(['xcrun', 'simctl', 'bootstatus', device, '-b'])
        report['devices'] = json.loads(command(['xcrun', 'simctl', 'list', 'devices', '-j']))
        command(['xcrun', 'simctl', 'install', device, str(binary)])
        container = Path(command(['xcrun', 'simctl', 'get_app_container', device, 'org.thrum.v1', 'data']))
        state = container / 'tmp/thrum-v1-ios-state.txt'
        environment = dict(os.environ, SIMCTL_CHILD_THRUM_V1_ROUTE='1')
        if native_witness:
            environment['SIMCTL_CHILD_THRUM_NATIVE_WITNESS'] = 'metal'
        environment.pop('SIMCTL_CHILD_THRUM_SCENE', None)
        environment.pop('THRUM_SCENE', None)
        command(['xcrun', 'simctl', 'launch', device, 'org.thrum.v1'], environment)
        old_argv = sys.argv
        os.environ['THRUM_V1_ROUTE_OUT'] = str(output / 'route')
        sys.argv = ['route.py', device, str(state)]
        try:
            route = runpy.run_path('examples/v1-game-ios/ios/route.py')
        finally:
            sys.argv = old_argv
        route['identity']()
        current = route['startup']()
        report['initial'] = current
        if native_witness and current.get('native_selection') != 'Metal':
            raise ValueError('native Metal witness was not active on the simulator')
        adapter = current.get('adapter', '')
        if 'recipe=Ok(Portable)' not in adapter or 'registry=' not in adapter:
            raise ValueError('missing truthful default registry selection')
        if not native_witness:
            for step in ['stall', 'hide', 'replace', 'stalled_replace', 'raced_present']:
                current = route[step](current)
        current = route['background'](current, 'm1-background')
        before_race = current
        if native_witness:
            current, report['native_race'] = native_background_race(route, current, device)
        else:
            current = route['background'](current, 'm1-background-stall', during_stall=True)
        if native_witness:
            for counter in ['native_discarded', 'native_resets']:
                if int(current.get(counter, '0')) <= int(before_race.get(counter, '0')):
                    raise ValueError(f'encoded-native background race did not advance {counter}')
            if current.get('native_selection') != 'Metal':
                raise ValueError('resume lost the native provider')
            report['race_before'] = before_race
            report['native_witness'] = True
        route['snapshot']('m1-final', current)
        report['final'] = current
        report['passed'] = True
    except (Exception, SystemExit) as error:
        report['error'] = str(error)
    finally:
        if device:
            command(['xcrun', 'simctl', 'shutdown', device], check=False)
            command(['xcrun', 'simctl', 'delete', device], check=False)
        report['ended_monotonic'] = time.monotonic()
        (output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'passed': report['passed'], 'error': report.get('error')}))
    return 0 if report['passed'] else 1


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--native-witness', action='store_true', help='M10 encoded native background race')
    args = parser.parse_args()
    raise SystemExit(run(args.output, args.native_witness))
