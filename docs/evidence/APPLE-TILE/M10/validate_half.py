#!/usr/bin/env python3
"""Validate every half texel in completed M10 captures, sharing exact hash twins."""
import argparse
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / 'scripts'))
from capture_spool import decoded_frames

BAD_HIGH = bytes(int(0x7c <= high <= 0x7f or high >= 0x81) for high in range(256))


def valid(raw):
    high = raw[1::2]
    if high.translate(BAD_HIGH).find(b'\x01') >= 0:
        return False
    index = high.find(b'\x80')
    while index >= 0:
        if raw[2 * index] != 0:  # Negative zero is nonnegative; negative subnormals are not.
            return False
        index = high.find(b'\x80', index + 1)
    return True


def run(directory):
    for word, expected in [(0, True), (0x8000, True), (0x8001, False),
                           (0x7bff, True), (0x7c00, False), (0xfc00, False)]:
        assert valid(word.to_bytes(2, 'little')) == expected
    report = json.loads((directory / 'manifest.json').read_text())
    assert report['passed'], 'Run the complete capture campaign first'
    result = {'passed': False, 'revision': report['revision'],
              'binary_sha256': report['binary_sha256'],
              'checker_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              'capture_manifest_sha256': hashlib.sha256((directory / 'manifest.json').read_bytes()).hexdigest(),
              'directory': str(directory), 'runs': {}, 'unique_streams': {}}
    checked = result['unique_streams']
    for name in report['runs']:
        folder = directory / name
        manifest = json.loads((folder / 'manifest.json').read_text())
        if manifest['bytes_per_pixel'] != 8:
            continue
        runtime = json.loads((folder / 'runtime.json').read_text())
        hashes = runtime['writer_result']['Ok']['frame_sha256']
        key = hashlib.sha256(json.dumps(hashes).encode()).hexdigest()
        if key not in checked:
            frame_bytes = manifest['extent'][0] * manifest['extent'][1] * 8
            count = 0
            for index, raw in enumerate(decoded_frames(folder / manifest['spool_file'],
                    runtime['spool_encoding'], hashes, frame_bytes)):
                if not valid(raw):
                    raise ValueError(f'{name}: invalid half texel in frame {index}')
                count += 1
            assert count == len(hashes)
            checked[key] = {'representative': name, 'frames': count,
                            'half_texels': count * frame_bytes // 2}
        result['runs'][name] = {'stream': key, 'frames': len(hashes)}
    result['passed'] = True
    return result


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    result = run(args.directory)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({'passed': result['passed'], 'runs': len(result['runs']),
                      'unique_streams': len(result['unique_streams'])}))
