#!/usr/bin/env python3
"""COOKED-DRAW submitted-output parity checkpoint, fail closed on the first mismatch.

The current checkpoint proves/refutes the first required valley view. Remaining
views and base/Chrome arms are pending the recorded draw-partition blocker.
Run under the repository GPU lock; output must be new.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
import subprocess
import zlib

ROOT = Path(__file__).resolve().parents[1]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def png(path, rgba, width=1920, height=1080):
    def chunk(kind, data):
        return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))
    raw = b''.join(b'\0' + rgba[y * width * 4:(y + 1) * width * 4] for y in range(height))
    path.write_bytes(b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 6, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(raw)) + chunk(b'IEND', b''))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode', choices=['paths', 'indirect'])
    parser.add_argument('--target', required=True, choices=['mac-metal', 'chrome-webgpu'])
    parser.add_argument('--base')
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    if args.mode != 'paths' or args.target != 'mac-metal':
        parser.error('base/Chrome arms are not implemented in this blocked checkpoint')
    os.chdir(ROOT)
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    env = dict(os.environ, RUSTC_WRAPPER='')
    rustc = subprocess.check_output(['rustup', 'which', '--toolchain', '1.99.1', 'rustc'], text=True).strip()
    env.update(RUSTC=rustc, PATH=str(Path(rustc).parent) + os.pathsep + env['PATH'])
    subprocess.run(['cargo', 'build', '--locked', '--offline', '-p', 'v1-game', '--bin', 'cooked_draw_probe'], env=env, check=True)
    store = out / 'store'
    subprocess.run(['cargo', 'run', '--locked', '--offline', '-p', 'thrum-cook', '--', 'cook', 'examples/v1-game/assets/showcase', str(store)], env=env, check=True)
    binary = Path('target/debug/cooked_draw_probe').resolve()
    report = {'target': args.target, 'view': 'valley noon yaw 0 degrees', 'extent': [1920, 1080],
              'commit': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
              'diff_sha256': hashlib.sha256(subprocess.check_output(['git', 'diff', 'HEAD'])).hexdigest(),
              'binary_sha256': digest(binary),
              'source_hashes': {str(p): digest(p) for root in [Path('crates'), Path('examples')] for p in root.rglob('*') if p.suffix in ['.rs', '.wgsl', '.toml']},
              'content_hashes': {str(p.relative_to(store)): digest(p) for p in store.rglob('*') if p.is_file()},
              'arms': [], 'pass': False}
    images = []
    for arm in ['indirect', 'direct', 'indirect-repeat', 'direct-repeat']:
        folder = out / arm
        run_env = dict(env, THRUM_SCENE='valley', THRUM_ASSET_STORE=str(store), THRUM_STARS='off',
                       COOKED_DRAW_OUTPUT=str(folder), COOKED_DRAW_MASK_INDIRECT=str(int(arm.startswith('direct'))))
        subprocess.run([str(binary)], env=run_env, check=True)
        row = json.loads((folder / 'report.json').read_text())
        expected = 'Some(Direct)' if arm.startswith('direct') else 'Some(Indirect)'
        if row['path'] != expected or row['drawn'] != 1:
            raise RuntimeError(f'wrong path or undrawn set: {row}')
        raw = (folder / 'output.rgba').read_bytes()
        png(out / f'{arm}.png', raw)
        images.append(raw)
        report['arms'].append(dict(row, arm=arm, rgba_sha256=digest(folder / 'output.rgba')))
    report['same_path_repeat_equal'] = images[0] == images[2] and images[1] == images[3]
    report['paths_equal'] = images[0] == images[1]
    changed = [i for i in range(0, len(images[0]), 4) if images[0][i:i+4] != images[1][i:i+4]]
    report['changed_pixels'] = len(changed)
    mask = bytearray(b'\0\0\0\xff' * (1920 * 1080))
    for i in changed:
        mask[i:i+4] = b'\xff\0\xff\xff'
    png(out / 'difference.png', mask)
    (out / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    if not report['same_path_repeat_equal'] or not report['paths_equal']:
        raise SystemExit(f"FAIL: deterministic={report['same_path_repeat_equal']}, changed pixels={len(changed)}; see {out / 'report.json'}")
    raise SystemExit('First view passed, but remaining M1.3 views are not implemented: milestone proof incomplete')


if __name__ == '__main__':
    main()
