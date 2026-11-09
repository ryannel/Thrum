#!/usr/bin/env python3
"""Verify the native window tap and package its original host-time sequence.

No cadence verdict: images are before present, not compositor pixels. Diagnostic
surface retirement is recorded as a gap; never label that sequence continuous.
"""
import argparse
import csv
import hashlib
import json
from pathlib import Path
import subprocess

from capture_spool import decoded_frames
from capture_v1_runtime import preview
from t2_writer_probe import FRAME_BYTES, package


def table(path):
    with path.open() as stream:
        return list(csv.DictReader(stream, delimiter='\t'))


def joins(surface, requests, images, hashes):
    submitted = {int(row['submission_id']): row for row in surface}
    if len(submitted) != len(surface) or not surface:
        raise ValueError('missing or duplicated surface submission identities')
    if any(row['presented'] != 'true' for row in surface):
        raise ValueError('unpresented output cannot qualify real-window proof')
    requested = {int(row['sequence']): row for row in requests}
    if len(requested) != len(requests):
        raise ValueError('ambiguous request sequence')
    records = []
    previous = -1
    seen = set()
    if len(images) != len(hashes):
        raise ValueError('writer/image coverage differs')
    for image, digest in zip(images, hashes, strict=True):
        identity = int(image['submission_id'])
        if identity in seen or identity not in submitted:
            raise ValueError('duplicate or unmatched captured submission')
        seen.add(identity)
        frame = submitted[identity]
        request = requested.get(int(frame['sequence']))
        if request is None:
            raise ValueError('missing normal AppKit request for submission')
        stamp = int(image['submit_origin_ns'])
        if stamp <= previous:
            raise ValueError('nonmonotonic source image time')
        previous = stamp
        records.append({'submission_id': identity, 'submit_ns': stamp, 'rgba_sha256': digest,
                        'sequence': int(frame['sequence']), 'lease_generation': int(frame['lease_generation']),
                        'accepted_yaw_degrees': float(request['accepted_yaw_degrees']),
                        'tick': int(image['tick']), 'device_generation': int(image['device_generation']),
                        'presenter_generation': int(image['presenter_generation'])})
    return records, sorted(set(submitted) - seen)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    output = args.output.resolve()
    surface = table(output / 'surface.tsv')
    requests = table(output / 'requests.tsv')
    images = table(output / 'images.tsv')
    if not surface or {row['tap_enabled'] for row in surface} != {'true'}:
        raise ValueError('packaging requires enabled capture; OFF has no pixels')
    hashes = (output / 'hashes.txt').read_text().splitlines()
    records, missing = joins(surface, requests, images, hashes)
    retirements = table(output / 'retirements.tsv')
    if missing and not retirements:
        raise ValueError('continuous interval omitted submitted images')
    frame_dir = output / 'png'
    frame_dir.mkdir(exist_ok=True)
    for record, raw in zip(records, decoded_frames(output / 'frames.zst', 'ZstdDelta', hashes), strict=True):
        if len(raw) != FRAME_BYTES or hashlib.sha256(raw).hexdigest() != record['rgba_sha256']:
            raise ValueError('source hash/size mismatch')
        package(raw, frame_dir / f"{record['submission_id']:06}.png")
    if not records:
        raise ValueError('no decoded sources')
    start = records[0]['submit_ns']
    for record in records:
        record['submit_ns'] -= start
    runtime = {'interval_end_ns': records[-1]['submit_ns'] + 8_333_333}
    manifest = {'timebase': 'realtime', 'records': records, 'missing_submissions': missing,
                'continuous': not missing and not retirements, 'retirements': retirements,
                'scope': 'engine output consumed by present, excludes compositor/native widgets/panel',
                'qualified': False}
    # Reuse T2's lossless native PNG checks and host-time-preserving preview/PTS verifier.
    if not (output / 'preview-1x.mp4').exists():
        preview(output, manifest, runtime, records)
    (output / 'window-manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    print(f"Verified {len(records)} originals, {len(missing)} retired/missing; no cadence verdict")


if __name__ == '__main__':
    main()
