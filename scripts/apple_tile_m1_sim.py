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


def run(output):
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
        adapter = current.get('adapter', '')
        if 'recipe=Ok(Portable)' not in adapter or 'registry=' not in adapter:
            raise ValueError('missing truthful default registry selection')
        for step in ['stall', 'hide', 'replace', 'stalled_replace', 'raced_present']:
            current = route[step](current)
        current = route['background'](current, 'm1-background')
        current = route['background'](current, 'm1-background-stall', during_stall=True)
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
    raise SystemExit(run(parser.parse_args().output))
