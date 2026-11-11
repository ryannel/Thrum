#!/usr/bin/env python3
"""Compare consecutive submitted held-camera frames for APPLE-PRESENTATION MF1."""
import argparse
import hashlib
import json
from pathlib import Path

from t2_writer_probe import package


def crop(raw, width, box):
    x, y, w, h = box
    return b''.join(raw[((y + row) * width + x) * 4:
                       ((y + row) * width + x + w) * 4] for row in range(h))


def difference(before, after):
    peak = changed = above24 = 0
    a = memoryview(before).cast('I')
    b = memoryview(after).cast('I')
    for left, right in zip(a, b, strict=True):
        if left != right:
            delta = max(abs(((left >> shift) & 255) - ((right >> shift) & 255))
                        for shift in (0, 8, 16))
            changed += delta > 0
            above24 += delta > 24
            peak = max(peak, delta)
    return {'changed_rgb_pixels': changed, 'above24_pixels': above24, 'peak_code_delta': peak}


def compare(capture, regions, output):
    manifest = json.loads((capture / 'manifest.json').read_text())
    assert manifest['complete'] and manifest['target'] == 'mac-metal'
    route = manifest['route']
    width, height = route['render_extent']
    assert manifest['header']['display_overlay'] == 'off'
    assert len({tuple(key['eye']) for key in route['keys']}) == 1
    for axis in ['yaw_degrees', 'pitch_degrees']:
        assert len({key[axis] for key in route['keys']}) == 1
    for x, y, w, h in regions.values():
        assert 0 <= x < x + w <= width and 0 <= y < y + h <= height
    output.mkdir(parents=True, exist_ok=False)
    records = manifest['records']
    frames = []
    results = []
    prior = None
    for checkpoint, record in zip(route['checkpoints'], records, strict=True):
        raw = (capture / 'checkpoints' / (checkpoint['name'] + '.rgba8')).read_bytes()
        assert len(raw) == width * height * 4
        assert hashlib.sha256(raw).hexdigest() == record['rgba_sha256']
        if len(frames) < 4:
            frames.append(raw)
        if prior is not None:
            row = {'ticks': [record['tick'] - 1, record['tick']],
                   'whole_frame': difference(prior, raw), 'regions': {}}
            for name, box in regions.items():
                row['regions'][name] = difference(crop(prior, width, box), crop(raw, width, box))
            results.append(row)
        prior = raw
    for name, box in regions.items():
        _, _, w, h = box
        crops = [crop(raw, width, box) for raw in frames]
        strip = b''.join(b''.join(raw[row * w * 4:(row + 1) * w * 4] for raw in crops)
                         for row in range(h))
        package(strip, output / f'{name}-four-consecutive-native.png', w * len(crops), h)
    a, b = frames[:2]
    amplified = bytearray(len(a))
    for index in range(0, len(a), 4):
        for channel in range(3):
            amplified[index + channel] = min(255, 4 * abs(a[index + channel] - b[index + channel]))
        amplified[index + 3] = 255
    package(amplified, output / 'consecutive-difference-amplified4.png', width, height)
    report = {'target': 'mac-metal', 'extent': [width, height], 'pairs': len(results),
              'capture_manifest_sha256': hashlib.sha256((capture / 'manifest.json').read_bytes()).hexdigest(),
              'comparison_script_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              'regions_xywh': regions, 'consecutive_pairs': results}
    (output / 'comparison.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'pairs': len(results), 'extent': [width, height],
                      'whole_frame_above24_range': [min(r['whole_frame']['above24_pixels'] for r in results),
                                                   max(r['whole_frame']['above24_pixels'] for r in results)],
                      'region_above24_max': {name: max(r['regions'][name]['above24_pixels'] for r in results)
                                             for name in regions}}, indent=2))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--capture', type=Path, required=True)
    parser.add_argument('--regions', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    compare(args.capture, json.loads(args.regions.read_text()), args.output)
