#!/usr/bin/env python3
"""Compare complete checkpoint bytes and save native-size difference masks.

PNG/PPM inputs retain their declared channels. Raw rgba8 and rgba16f inputs
require an explicit extent. rgba16f compares all eight stored bytes per pixel,
without decoding floats or introducing a tolerance. This tool proves pixel
identity only. The caller must separately establish target and sample identity.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import zlib

from run_v1_showcase import Failure, read_png, read_ppm
from t2_writer_probe import package

MAX_FRAME_BYTES = 128 * 1024 * 1024


def load_pixels(path, encoding, extent):
    """Return the exact stored channels, refusing partial or foreign layouts."""
    data = path.read_bytes()
    if encoding == 'png':
        # The shared decoder handles PNG filters. Verify the container before
        # using it, including CRCs, termination and the complete decoded size.
        if data[:8] != b'\x89PNG\r\n\x1a\n':
            raise ValueError('invalid PNG signature')
        if len(data) < 33 or data[8:16] != struct.pack('>I4s', 13, b'IHDR'):
            raise ValueError('missing PNG header')
        width, height, depth, colour, compression, filtering, interlace = struct.unpack_from('>IIBBBBB', data, 16)
        channels = {2: 3, 6: 4}.get(colour, 0)
        if depth != 8 or not channels or compression or filtering or interlace:
            raise ValueError('unsupported PNG layout')
        if not width or not height or width * height * channels > MAX_FRAME_BYTES:
            raise ValueError('invalid or excessive image extent')
        position = 8
        compressed = bytearray()
        ended = False
        while position + 12 <= len(data):
            size, kind = struct.unpack_from('>I4s', data, position)
            end = position + size + 12
            if end > len(data):
                raise ValueError('truncated PNG chunk')
            payload = data[position + 8:end - 4]
            crc = struct.unpack_from('>I', data, end - 4)[0]
            if zlib.crc32(kind + payload) != crc:
                raise ValueError('PNG CRC mismatch')
            if kind == b'IDAT':
                compressed.extend(payload)
            position = end
            if kind == b'IEND':
                ended = True
                break
        if not ended or position != len(data):
            raise ValueError('missing PNG end or trailing data')
        width, height, channels, pixels = read_png(path)
        if len(zlib.decompress(compressed)) != height * (width * channels + 1):
            raise ValueError('PNG decompressed length mismatch')
        pixel_bytes = channels
    elif encoding == 'ppm':
        width, height, pixel_bytes, pixels = read_ppm(path)
    else:
        if extent is None:
            raise ValueError('raw images require --extent WIDTH HEIGHT')
        width, height = extent
        pixel_bytes = 8 if encoding == 'rgba16f' else 4
        pixels = data
    expected = width * height * pixel_bytes
    if width <= 0 or height <= 0 or not 0 < expected <= MAX_FRAME_BYTES:
        raise ValueError('invalid or excessive image extent')
    if len(pixels) != expected:
        raise ValueError('image length differs from declared extent and format')
    if extent is not None and [width, height] != list(extent):
        raise ValueError('image extent differs from --extent')
    identity = {'file': str(path.resolve()), 'file_sha256': hashlib.sha256(data).hexdigest(),
                'pixels_sha256': hashlib.sha256(pixels).hexdigest(),
                'extent': [width, height], 'bytes_per_pixel': pixel_bytes, 'encoding': encoding}
    return identity, pixels


def compare(reference, candidate, encoding, extent, output):
    """Compare every channel byte and retain a full-frame and cropped mask."""
    if output.exists():
        raise ValueError('output must be a new directory')
    left, expected = load_pixels(reference, encoding, extent)
    right, observed = load_pixels(candidate, encoding, extent)
    if left['extent'] != right['extent'] or left['bytes_per_pixel'] != right['bytes_per_pixel']:
        raise ValueError('checkpoint layouts differ')
    width, height = left['extent']
    stride = left['bytes_per_pixel']
    report = {'reference': left, 'candidate': right, 'equal': expected == observed,
              'scope': 'exact stored pixel bytes only; target/sample/exposure/history identity must be established by the caller',
              'changed_pixels': 0, 'changed_bytes': 0, 'bounds_xyxy_exclusive': None}
    output.mkdir(parents=True)
    if not report['equal']:
        mask = bytearray(b'\0\0\0\xff' * (width * height))
        x0, y0, x1, y1 = width, height, 0, 0
        for pixel in range(width * height):
            offset = pixel * stride
            a, b = expected[offset:offset + stride], observed[offset:offset + stride]
            if a == b:
                continue
            report['changed_pixels'] += 1
            report['changed_bytes'] += sum(a != b for a, b in zip(a, b, strict=True))
            x, y = pixel % width, pixel // width
            x0, y0, x1, y1 = min(x0, x), min(y0, y), max(x1, x + 1), max(y1, y + 1)
            mask[pixel * 4:pixel * 4 + 4] = b'\xff\xff\xff\xff'
        report['bounds_xyxy_exclusive'] = [x0, y0, x1, y1]
        report['mask_sha256'] = package(mask, output / 'difference.png', width, height)
        crop = b''.join(mask[(y * width + x0) * 4:(y * width + x1) * 4] for y in range(y0, y1))
        report['crop_sha256'] = package(crop, output / 'difference-crop.png', x1 - x0, y1 - y0)
    (output / 'comparison.json').write_text(json.dumps(report, indent=2) + '\n')
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--reference', type=Path, required=True)
    parser.add_argument('--candidate', type=Path, required=True)
    parser.add_argument('--encoding', choices=['png', 'ppm', 'rgba8', 'rgba16f'], required=True)
    parser.add_argument('--extent', nargs=2, type=int, metavar=('WIDTH', 'HEIGHT'))
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    try:
        report = compare(args.reference, args.candidate, args.encoding, args.extent, args.output)
    except (ValueError, OSError, Failure, zlib.error, struct.error, IndexError) as error:
        parser.exit(2, f'REFUSED {error}\n')
    print(f'{"PASS" if report["equal"] else "DIFFERENT"} {args.output / "comparison.json"}')
    return 0 if report['equal'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
