#!/usr/bin/env python3
"""M1 fixed-time valley motion witness using the existing submitted route tap.

Run `capture` under the repository GPU lock. Run `package` afterwards without
that lock: it validates every identity and writes independently decoded PNGs
and a fixed-sample-time movie. No elapsed-time or physical-display claim.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess

from run_v1_showcase import store_root
from t2_writer_probe import package

ROOT = Path(__file__).resolve().parents[1]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def save(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')


def capture(args):
    output, route_path = args.output.resolve(), args.route.resolve()
    route = json.loads(route_path.read_text())
    if output.exists():
        raise ValueError('capture output must be a new directory')
    output.parent.mkdir(parents=True, exist_ok=True)
    if shutil.disk_usage(output.parent).free < 24 * 1024**3:
        raise ValueError('less than 24 GiB free before the all-tick route')
    if store_root(args.store.resolve()) != route['cooked_root']:
        raise ValueError('cooked root differs from the checked route')
    ticks = [row['tick'] for row in route['checkpoints']]
    if not ticks or ticks != list(range(ticks[0], ticks[-1] + 1)):
        raise ValueError('the witness must capture every tick in one continuous window')
    names = subprocess.check_output(['git', 'ls-files', '-z', '--cached', '--others',
        '--exclude-standard', 'crates', 'examples', 'scripts', 'Cargo.toml', 'Cargo.lock',
        'rust-toolchain.toml', '.cargo'], cwd=ROOT).decode().split('\0')
    manifest = {
        'source_revision': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
        'dirty_diff_sha256': hashlib.sha256(subprocess.check_output(['git', 'diff', 'HEAD'], cwd=ROOT)).hexdigest(),
        'source_files': {name: digest(ROOT / name) for name in names
            if name and (ROOT / name).is_file() and Path(name).suffix in ('.rs', '.wgsl', '.toml', '.lock', '.py')},
        'binary_sha256': digest(args.binary.resolve()), 'route_sha256': digest(route_path),
        'cooked_root': route['cooked_root'], 'route': route,
        'mode': 'fixed', 'target': 'mac-metal', 'capture_source': 'submitted engine SDR',
        'scope': 'one submitted game frame per sampled tick at fixed presentation phase; no endpoint redraw, elapsed-time or panel claim',
        'capture_window': [ticks[0], ticks[-1]],
        'capture_complete': False, 'complete': False,
    }
    environment = dict(os.environ)
    for key in ['THRUM_CAPTURE_RUNTIME', 'THRUM_CAPTURE_PARAMS', 'THRUM_CAPTURE_MOTION_FRAMES',
                'THRUM_CAPTURE_PHASE', 'THRUM_G3T_OVERLAY_CONTROL', 'THRUM_G3T_OMIT_PLACEMENT']:
        environment.pop(key, None)
    environment.update(THRUM_CAPTURE_SCENE=str(output), THRUM_CAPTURE_ROUTE=str(route_path),
        THRUM_CAPTURE_SOURCE='sdr', THRUM_ASSET_STORE=str(args.store.resolve()), THRUM_SCENE='valley',
        THRUM_ROUTE_OVERLAY=route['overlay'], THRUM_RECIPE='portable', THRUM_STARS='on',
        THRUM_CAPTURE_DISPLAY_OVERLAY=args.overlay, THRUM_CAPTURE_PRESENT_CONTROL='off')
    manifest['environment'] = {key: value for key, value in environment.items() if key.startswith('THRUM_')}
    pending = output.with_name(output.name + '.manifest.partial.json')
    log_path = output.with_name(output.name + '.game.log')
    save(pending, manifest)
    with log_path.open('wb') as log:
        result = subprocess.run([str(args.binary.resolve())], cwd=ROOT, env=environment,
            stdout=log, stderr=subprocess.STDOUT, timeout=600)
    output.mkdir(exist_ok=True)
    shutil.move(log_path, output / 'game.log')
    save(output / 'manifest.json', manifest)
    pending.unlink()
    if result.returncode:
        raise ValueError(f'game exited {result.returncode}; see {output / "game.log"}')
    manifest['capture_complete'] = True
    save(output / 'manifest.json', manifest)
    print(json.dumps({'capture_complete': True, 'output': str(output)}))


def package_capture(args):
    output = args.output.resolve()
    manifest = json.loads((output / 'manifest.json').read_text())
    if not manifest['capture_complete'] or manifest['complete']:
        raise ValueError('capture must be complete and not yet packaged')
    route = manifest['route']
    rows = [json.loads(line) for line in (output / 'route.jsonl').read_text().splitlines()]
    header, samples = rows[0], rows[1:]
    if header['backend'] != 'Metal' or header['recipe'] != 'Ok(Portable)' or header['mode'] != 'fixed':
        raise ValueError('wrong target, recipe or capture mode')
    if header['display_overlay'] != manifest['environment']['THRUM_CAPTURE_DISPLAY_OVERLAY']:
        raise ValueError('wrong display selector')
    last = route['keys'][-1]['tick']
    if [row['route_tick'] for row in samples] != list(range(last + 1)):
        raise ValueError('missing or repeated route sample')
    if len({row['submission'] for row in samples}) != len(samples):
        raise ValueError('repeated submitted frame')
    nominal_phase = route['phase'][0] / route['phase'][1]
    width, height = route['render_extent']
    png = output / 'png'
    png.mkdir(exist_ok=True)
    records = []
    for index, checkpoint in enumerate(route['checkpoints']):
        sample = samples[checkpoint['tick']]
        stem = output / 'checkpoints' / checkpoint['name']
        identity = json.loads(stem.with_suffix('.identity.json').read_text())
        if identity['submission_id'] != sample['submission'] or identity['source_tick'] != sample['tick']:
            raise ValueError('source/submission identity mismatch')
        if (identity['alpha_bits'] != sample['alpha_bits']
                or abs(sample['alpha'] - nominal_phase) > 1e-4
                or identity['history_epoch'] != sample['epoch']):
            raise ValueError('phase or camera-history identity mismatch')
        if len(identity['exposure_gpu_words']) != 8 or identity['capture_source'] != 'Sdr':
            raise ValueError('missing submitted exposure or wrong image stage')
        raw = stem.with_suffix('.rgba8').read_bytes()
        if len(raw) != width * height * 4:
            raise ValueError('invalid full-frame byte count')
        image = png / f'{index:04}.png'
        original = package(raw, image, width, height)
        records.append({'tick': checkpoint['tick'], 'sample_seconds': index / route['tick_hz'],
            'rgba_sha256': original, 'png_sha256': digest(image), 'identity': identity})
    subprocess.run(['ffmpeg', '-y', '-v', 'error', '-framerate', str(route['tick_hz']), '-i',
        str(png / '%04d.png'), '-c:v', 'libx264', '-crf', '16', '-pix_fmt', 'yuv420p',
        str(output / 'preview-fixed-1x.mp4')], check=True)
    subprocess.run(['ffmpeg', '-y', '-v', 'error', '-i', str(output / 'preview-fixed-1x.mp4'),
        '-vf', 'fps=4,scale=480:270,tile=4x4', '-frames:v', '1', str(output / 'contact.png')], check=True)
    count = int(subprocess.check_output(['ffprobe', '-v', 'error', '-count_frames',
        '-select_streams', 'v:0', '-show_entries', 'stream=nb_read_frames', '-of',
        'default=noprint_wrappers=1:nokey=1', str(output / 'preview-fixed-1x.mp4')], text=True))
    if count != len(records):
        raise ValueError('preview omitted or duplicated a source frame')
    manifest.update(complete=True, header=header, records=records,
        packaging_script_sha256=digest(Path(__file__)),
        preview_sha256=digest(output / 'preview-fixed-1x.mp4'), preview_frames=count)
    save(output / 'manifest.json', manifest)
    print(json.dumps({'complete': True, 'frames': count, 'target': header['adapter']}))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['capture', 'package'])
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--binary', type=Path, default=Path('target/release/v1-game'))
    parser.add_argument('--store', type=Path)
    parser.add_argument('--route', type=Path)
    parser.add_argument('--overlay', choices=['motion', 'off'], default='motion')
    options = parser.parse_args()
    if options.action == 'capture':
        if options.store is None or options.route is None:
            parser.error('capture requires --store and --route')
        capture(options)
    else:
        package_capture(options)
