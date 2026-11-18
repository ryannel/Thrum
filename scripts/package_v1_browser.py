#!/usr/bin/env python3
"""Verify lossless Chrome source records and package their declared timeline."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
from capture_v1_runtime import preview
from t2_writer_probe import package


def run(output):
    manifest = json.loads((output / 'manifest.json').read_text())
    runtime = json.loads((output / 'runtime.json').read_text())
    if not manifest['complete'] or not runtime['complete'] or not manifest['records']:
        raise ValueError('incomplete browser evidence')
    width, height = runtime['extent']
    frames = output / 'png'
    frames.mkdir(exist_ok=True)
    records = []
    start = manifest['records'][0]['submit_ms']
    frame_bytes = width * height * 4
    if manifest.get('spool_encoding') == 'raw':
        spool = output / manifest['records'][0]['file']
        with spool.open('rb') as source:
            for ordinal, row in enumerate(manifest['records']):
                if row['file'] != spool.name or row['offset'] != ordinal * frame_bytes:
                    raise ValueError('raw browser spool index does not match submitted order')
                raw = source.read(frame_bytes)
                append_frame(frames, records, row, raw, width, height, start)
            if source.read(1):
                raise ValueError('raw browser spool has unindexed bytes')
    else:
        for row in manifest['records']:
            raw = gzip.decompress((output / row['file']).read_bytes())
            append_frame(frames, records, row, raw, width, height, start)
    packaged = {'timebase': manifest['mode']}
    preview(output, packaged, {'interval_end_ns': records[-1]['submit_ns'] + 16_666_667}, records)
    packaged.update(source_manifest_sha256=hashlib.sha256((output / 'manifest.json').read_bytes()).hexdigest(),
                    full_frames_verified=len(records), terminal_duplicate_is_submission=False)
    (output / 'packaging.json').write_text(json.dumps(packaged, indent=2) + '\n')


def append_frame(frames, records, row, raw, width, height, start):
    if len(raw) != width * height * 4 or hashlib.sha256(raw).hexdigest() != row['rgba_sha256']:
        raise ValueError('source size/hash mismatch')
    if package(raw, frames / f"{row['submission_id']:06}.png", width, height) != row['rgba_sha256']:
        raise ValueError('PNG round-trip mismatch')
    records.append({'submission_id': row['submission_id'],
                    'submit_ns': round((row['submit_ms'] - start) * 1_000_000),
                    'sample_ns': round(len(records) * 1_000_000_000 / 60)})


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', required=True, type=Path)
    run(parser.parse_args().output)
