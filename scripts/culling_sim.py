#!/usr/bin/env python3
"""Run a requested Occlusion scene on an owned iPhone simulator (function only).

Run under the repository GPU lock. No desktop capture permission is needed.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    os.chdir(ROOT)
    out = args.output or Path('tmp/culling-m2') / f'sim-{time.time_ns()}'
    out = out.resolve()
    out.mkdir(parents=True, exist_ok=False)
    report = {'target': 'ios-sim', 'scope': 'function only', 'commands': [], 'samples': []}
    device = None

    def run(command, env=None, timeout=300):
        result = subprocess.run(command, env=env, text=True, capture_output=True, timeout=timeout)
        report['commands'].append(dict(argv=command, exit=result.returncode, stdout=result.stdout, stderr=result.stderr))
        result.check_returncode()
        return result.stdout.strip()

    try:
        report['commit'] = run(['git', 'rev-parse', 'HEAD'])
        report['diff_sha256'] = hashlib.sha256(run(['git', 'diff', 'HEAD']).encode()).hexdigest()
        run(['examples/v1-game-ios/ios/build-simulator.sh'], dict(os.environ, RUSTC_WRAPPER=''), 900)
        device = run(['xcrun', 'simctl', 'create', 'Thrum CULLING owned', 'com.apple.CoreSimulator.SimDeviceType.iPhone-18-Pro', 'com.apple.CoreSimulator.SimRuntime.iOS-27-2'])
        run(['xcrun', 'simctl', 'bootstatus', device, '-b'])
        app = ROOT / 'target/v1-game-ios-sim/ThrumV1.app'
        report['binary_sha256'] = hashlib.sha256((app / 'ThrumV1').read_bytes()).hexdigest()
        run(['xcrun', 'simctl', 'install', device, str(app)])
        container = Path(run(['xcrun', 'simctl', 'get_app_container', device, 'org.thrum.v1', 'data']))
        state = container / 'tmp/thrum-v1-ios-state.txt'
        # Ordinary diagnostic grid: all entities are submitted by Direct.
        run(['xcrun', 'simctl', 'launch', device, 'org.thrum.v1'], dict(os.environ, SIMCTL_CHILD_THRUM_CULLING='occlusion'))
        deadline = time.monotonic() + 90
        previous = 0
        while len(report['samples']) < 8:
            raw = state.read_text() if state.exists() else ''
            if raw.startswith('fault='):
                raise RuntimeError(raw)
            s = dict(line.split('=', 1) for line in raw.splitlines() if '=' in line)
            presented = int(s.get('presented', 0))
            if presented > previous and s.get('culling_requested') == 'Occlusion':
                if s.get('culling_resolved') != 'Direct' or s.get('culling_reason') != 'MissingIndirectExecution':
                    raise RuntimeError(f'wrong fallback: {s}')
                report['samples'].append(s)
                previous = presented
            if time.monotonic() > deadline:
                raise RuntimeError(f'no progressing fallback: {s}')
            time.sleep(.5)
        run(['xcrun', 'simctl', 'io', device, 'screenshot', str(out / 'direct-fallback.png')])
        report['pass'] = True
    except BaseException as error:
        report.update({'pass': False, 'failure': str(error)})
        raise
    finally:
        if device:
            subprocess.run(['xcrun', 'simctl', 'shutdown', device], capture_output=True, timeout=60)
            subprocess.run(['xcrun', 'simctl', 'delete', device], capture_output=True, timeout=60)
        (out / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
        print(out / 'report.json', flush=True)


if __name__ == '__main__':
    main()
