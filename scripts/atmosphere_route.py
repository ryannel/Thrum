#!/usr/bin/env python3
"""Capture the G3s showcase stops and matching elapsed-time pans through T2."""
import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

from atmosphere_cost import ROOT, STOPS, preset


def capture(args):
    args.output.mkdir()
    motions = {}
    for name, solar, yaw in [('scrub', 5, -45), ('ground-pan', 17.4, -110)]:
        path = args.output / f'{name}.params.json'
        path.write_text(json.dumps({'world.paused': True, 'sky.solar_time': solar,
                                    'camera.yaw': yaw, 'camera.pitch': 0}) + '\n')
        motions[name] = path
    cases = [(stop, 'checkpoint', preset(stop)) for stop in STOPS]
    cases += [(name, name, parameters) for name, parameters in motions.items()]
    failures = []
    for name, scenario, parameters in cases:
        fixed = scenario == 'checkpoint'
        mode = 'fixed' if fixed else 'realtime'
        output = args.output / name
        common = ['--mode', mode, '--params', str(parameters), '--output', str(output),
                  '--overlay', 'off', '--reconstruction', 'taau']
        environment = dict(os.environ)
        if args.target == 'mac-metal':
            native_case = scenario if fixed else f'atmosphere-{scenario}'
            command = [sys.executable, 'scripts/capture_v1_runtime.py', '--binary', str(args.binary),
                       '--store', str(args.store), '--case', native_case, '--defer', *common]
            if not fixed:
                command += ['--admission', 'wait', '--extent', '1280', '720']
                environment['THRUM_CAPTURE_ENCODING'] = 'raw'
        else:
            command = ['node', 'scripts/capture_v1_browser.mjs', '--cook', str(args.cook),
                       '--web-root', str(args.web_root), '--atmosphere', scenario, *common]
            if not fixed:
                command += ['--retain-raw', 'on']
        if subprocess.run(command, cwd=ROOT, env=environment, check=False).returncode:
            failures.append(name)
    if failures:
        raise SystemExit('Incomplete captures: ' + ', '.join(failures))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--target', choices=['mac-metal', 'chrome-webgpu'], required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--binary', type=Path, default=Path('target/release/v1-game'))
    parser.add_argument('--store', type=Path)
    parser.add_argument('--cook', type=Path, default=Path('target/release/thrum-cook'))
    parser.add_argument('--web-root', type=Path, default=Path('target/v1-game-web'))
    args = parser.parse_args()
    if args.target == 'mac-metal' and args.store is None:
        parser.error('--store is required on Metal')
    capture(args)


if __name__ == '__main__':
    main()
