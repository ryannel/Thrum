#!/usr/bin/env python3
"""Compare retained raw diagnostic bytes without a tolerance or image conversion."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path


def read(path):
    return gzip.decompress(path.read_bytes()) if path.suffix == '.gz' else path.read_bytes()


def compare(left, right, width, pixel_bytes):
    assert len(left) == len(right) and len(left) % pixel_bytes == 0
    different = [i for i in range(len(left) // pixel_bytes)
                 if left[i * pixel_bytes:(i + 1) * pixel_bytes] != right[i * pixel_bytes:(i + 1) * pixel_bytes]]
    return {'left_sha256': hashlib.sha256(left).hexdigest(),
            'right_sha256': hashlib.sha256(right).hexdigest(),
            'bytes': len(left), 'different_pixels': len(different),
            'different_bytes': sum(a != b for a, b in zip(left, right)),
            'first_pixels': [{'xy': [i % width, i // width],
                              'left': left[i * pixel_bytes:(i + 1) * pixel_bytes].hex(),
                              'right': right[i * pixel_bytes:(i + 1) * pixel_bytes].hex()}
                             for i in different[:64]]}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('left', type=Path)
    parser.add_argument('right', type=Path)
    parser.add_argument('--width', type=int, default=1280)
    parser.add_argument('--pixel-bytes', type=int, default=4)
    args = parser.parse_args()
    print(json.dumps(compare(read(args.left), read(args.right), args.width, args.pixel_bytes), indent=2))
