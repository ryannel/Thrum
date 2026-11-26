#!/usr/bin/env python3
"""Capture CLOUDS WP10 through the native and Chrome T2 entry points."""
import argparse
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent.parent
SOLAR = {'noon': 12, 'low': 17.052276611328125, 'sunset': 18.06}
EYES = {2: [0, 241.92267, 16], 40: [0, 279.92267, 16]}
SHADE_EYE = [-100, 2500, -4000]
COST_EYE = [0, 260.48465, 80]


def views(group):
    if group.startswith('gallery') or group == 'chrome':
        indices = {
            'gallery-fair': range(6),
            'gallery-mediocris': range(6, 9),
            'gallery-congestus': range(9, 12),
            'chrome': (4, 8, 9),
        }[group]
        for index in indices:
            for sun in ('noon', 'low'):
                for height in EYES:
                    side = '-side-lit' if sun == 'low' else ''
                    yaw = 20 if side else -35
                    yield f'shape-{index:02}-{sun}-{height:02}', f'library-{index}{side}', sun, height, 1, 0, yaw, 18
    if group in ('mix', 'chrome'):
        for sun in (SOLAR if group == 'mix' else ('noon',)):
            for height in EYES:
                yield f'mixed-{sun}-{height:02}', 'mixed', sun, height, 4, 10, -39, 13
        for sun in (('noon', 'low') if group == 'mix' else ('noon',)):
            for height in EYES:
                yield f'full-{sun}-{height:02}', 'library-only', sun, height, 60, 0, -39, 10
        yield 'shade-mixed', 'mixed', 'noon', SHADE_EYE, 4, 10, -75, -27
        yield 'shade-procedural', 'library-procedural', 'noon', SHADE_EYE, 4, 10, -75, -27
    if group == 'cost-view':
        yield 'library-cost', 'library-cost', 'low', COST_EYE, 1, 0, -79.844116, 10
    if group == 'motion':
        yield 'growth-1x', 'library-growth', 'noon', 2, 1, 0, -35, 13
        yield 'growth-30x', 'library-growth', 'noon', 2, 30, 0, -35, 13
        yield 'mixed-8x', 'mixed', 'low', 40, 8, 10, -39, 13


def capture(args, view):
    name, placement, sun, height, speed, wind, yaw, pitch = view
    destination = args.output / name
    if destination.exists():
        raise ValueError(f'{destination} already exists')
    moving = args.group == 'motion'
    params = {'world.paused': True, 'sky.solar_time': SOLAR[sun],
              'camera.yaw': yaw, 'camera.pitch': pitch}
    extent = [960, 540] if moving else [1280, 720]
    eye = EYES[height] if isinstance(height, int) else height
    if args.group == 'chrome':
        config = destination.with_suffix('.view.json')
        config.write_text(json.dumps(dict(extent=extent, preset='fair', weather_rate=speed,
                                         wind=wind, placement=None if placement == 'mixed' else placement,
                                         eye=eye, params=params)) + '\n')
        command = ['node', 'scripts/capture_v1_browser.mjs', '--mode', 'fixed', '--scene', 'noon',
                   '--overlay', 'off', '--reconstruction', 'taau', '--clouds', 'world-light',
                   '--cloud-view', str(config), '--cloud-store', str(args.library),
                   '--web-root', str(args.web_root), '--cook', 'target/release/thrum-cook']
        env = os.environ
    else:
        config = destination.with_suffix('.params.json')
        config.write_text(json.dumps(params) + '\n')
        env = {key: value for key, value in os.environ.items()
               if not key.startswith(('THRUM_CAPTURE_', 'THRUM_CLOUD_'))}
        env.update(THRUM_CAPTURE_FIXED_EYE=json.dumps(eye), THRUM_CLOUDS='world-light',
                   THRUM_CLOUD_STORE=str(args.library), THRUM_CLOUD_PLACEMENT=placement,
                   THRUM_CLOUD_TIME_SCALE=str(speed), THRUM_CLOUD_WIND=str(wind), THRUM_CLOUD_PRESET='fair')
        case = 'clouds-growth' if name == 'growth-1x' else 'clouds-placed' if moving else 'checkpoint'
        command = ['python3', 'scripts/capture_v1_runtime.py', '--binary', 'target/release/v1-game',
                   '--case', case, '--mode', 'realtime' if moving else 'fixed', '--recipe', 'portable',
                   '--reconstruction', 'taau', '--extent', *map(str, extent), '--params', str(config),
                   '--store', str(args.valley)]
        if moving:
            command.append('--defer')
    print(f'{name}: {placement}, {sun}, {height} m, {speed}x', flush=True)
    subprocess.run([*command, '--output', str(destination)], cwd=ROOT, env=env, check=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('group', choices=('gallery-fair', 'gallery-mediocris', 'gallery-congestus',
                                         'mix', 'motion', 'chrome', 'cost-view'))
    parser.add_argument('--only', help='capture one named view from the group')
    parser.add_argument('--sun', choices=SOLAR, help='capture only this solar elevation')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--library', type=Path, required=True)
    parser.add_argument('--valley', type=Path, default=ROOT / 'tmp/clouds-wp10/cost/valley')
    parser.add_argument('--web-root', type=Path, default=ROOT / 'target/v1-game-web-proving')
    args = parser.parse_args()
    for name in ('output', 'library', 'valley', 'web_root'):
        setattr(args, name, getattr(args, name).resolve())
    args.output.mkdir(parents=True, exist_ok=True)
    selected = [view for view in views(args.group)
                if (not args.only or view[0] == args.only) and (not args.sun or view[2] == args.sun)]
    if not selected:
        parser.error('--only did not select a known view')
    for view in selected:
        capture(args, view)


if __name__ == '__main__':
    main()
