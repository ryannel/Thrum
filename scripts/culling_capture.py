#!/usr/bin/env python3
"""Retain M3's T2 fixed and elapsed-time motion proof. Hold the gpu lock externally."""
import json
import os
from pathlib import Path
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]


def main():
    output = ROOT / 'tmp/culling-m3' / time.strftime('capture-%Y%m%d-%H%M%S')
    output.mkdir(parents=True)
    subprocess.run(['cargo', 'build', '--locked', '--offline', '--release', '-p', 'v1-game', '-p', 'thrum-cook'], cwd=ROOT, check=True)
    subprocess.run(['sh', 'examples/v1-game/web/build.sh'], cwd=ROOT, check=True)
    subprocess.run(['target/release/thrum-cook', 'cook', 'examples/v1-game/assets/showcase', str(output / 'store')], cwd=ROOT, check=True)
    fast = json.loads((ROOT / 'examples/v1-game/routes/culling-fast-yaw.json').read_text())
    fast['render_extent'] = [1920, 1080]
    native_route = output / 'fast-yaw-1080.json'
    native_route.write_text(json.dumps(fast, indent=2) + '\n')
    # Native fixed checkpoint stills come from the full-route oracle's T2 tap.
    command = ['python3', 'scripts/capture_v1_runtime.py', '--binary', 'target/release/v1-game',
               '--store', str(output / 'store'), '--case', 'route', '--route', str(native_route),
               '--mode', 'realtime', '--output', str(output / 'native-realtime')]
    subprocess.run(command, cwd=ROOT, check=True)
    for mode in ('fixed', 'realtime'):
        subprocess.run(['node', 'scripts/capture_v1_browser.mjs', '--cook', 'target/release/thrum-cook',
                        '--mode', mode, '--route-file', 'examples/v1-game/routes/culling-fast-yaw.json',
                        '--output', str(output / f'chrome-{mode}')], cwd=ROOT, check=True)
        subprocess.run(['python3', 'scripts/package_v1_browser.py', '--output', str(output / f'chrome-{mode}')], cwd=ROOT, check=True)
    env = dict(os.environ, THRUM_FIELD_FULL_PATH='1', THRUM_FIELD_SEQUENCE=str(output / 'field-fixed'))
    subprocess.run(['cargo', 'test', '--locked', '--offline', '--release', '-p', 'v1-game',
                    '--test', 'field_occlusion_gpu', '--', '--nocapture'], env=env, cwd=ROOT, check=True)
    print(output)


if __name__ == '__main__':
    main()
