#!/usr/bin/env python3
"""Compare every stored byte and deterministic sample of two Chrome captures."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path

IDENTITY = ['session', 'submission_id', 'tick', 'source_tick', 'source_epoch', 'timeline',
            'alpha', 'history_epoch', 'device_generation', 'presenter_generation', 'surface_presented']


def compare(reference, candidate, output, linear):
    before = json.loads((reference / 'manifest.json').read_text())
    after = json.loads((candidate / 'manifest.json').read_text())
    if not before['complete'] or not after['complete'] or before['mode'] != 'fixed' or after['mode'] != 'fixed':
        raise ValueError('complete fixed captures required')
    if len(before['records']) != len(after['records']):
        raise ValueError('record count changed')
    # The retained pre-M1 SDR record predates the exposure export field. The
    # linear extension records exact GPU words on both sides of its comparison.
    keys = IDENTITY + (['pixel_source', 'exposure_gpu_words'] if linear else [])
    rows = []
    for left, right in zip(before['records'], after['records'], strict=True):
        if any(left.get(key) != right.get(key) for key in keys):
            raise ValueError(f'identity changed at ordinal {left["ordinal"]}')
        if linear and len(left.get('exposure_gpu_words') or []) != 8:
            raise ValueError('missing exact exposure identity')
        first = gzip.decompress((reference / left['file']).read_bytes())
        second = gzip.decompress((candidate / right['file']).read_bytes())
        for raw, row in [(first, left), (second, right)]:
            if hashlib.sha256(raw).hexdigest() != row['rgba_sha256']:
                raise ValueError('source hash verification failed')
        if first != second:
            raise ValueError(f'raw output changed at ordinal {left["ordinal"]}')
        rows.append({'ordinal': left['ordinal'], 'sha256': right['rgba_sha256'], 'bytes': len(first),
                     'identity': {key: right[key] for key in keys}})
    report = {'passed': True, 'reference': str(reference), 'candidate': str(candidate),
              'reference_manifest_sha256': hashlib.sha256((reference / 'manifest.json').read_bytes()).hexdigest(),
              'candidate_manifest_sha256': hashlib.sha256((candidate / 'manifest.json').read_bytes()).hexdigest(),
              'frames': len(rows), 'identity_keys': keys, 'records': rows}
    output.write_text(json.dumps(report, indent=2) + '\n')
    print(f'PASS {len(rows)} exact Chrome records')


def compare_digest(reference, candidate, output, linear, expected_records=481):
    """Compare retained manifest identities/digests with freshly decoded candidates.

    The reference can be a gzip manifest. No reference frame files are needed.
    The existing byte comparison above deliberately remains independent.
    """
    try:
        encoded = reference.read_bytes()
        before = json.loads(gzip.decompress(encoded) if reference.suffix == '.gz' else encoded)
    except (OSError, ValueError, EOFError) as error:
        raise ValueError(f'unreadable retained reference {reference}: {error}') from error
    after = json.loads((candidate / 'manifest.json').read_text())
    if not before['complete'] or not after['complete'] or before['mode'] != 'fixed' or after['mode'] != 'fixed':
        raise ValueError('complete fixed captures required')
    if len(before['records']) != expected_records or len(after['records']) != expected_records:
        raise ValueError(f'expected {expected_records} records in both captures')
    keys = IDENTITY + (['pixel_source', 'exposure_gpu_words'] if linear else [])
    rows = []
    for left, right in zip(before['records'], after['records'], strict=True):
        if any(left.get(key) != right.get(key) for key in keys):
            raise ValueError(f'identity changed at ordinal {left["ordinal"]}')
        if linear and len(left.get('exposure_gpu_words') or []) != 8:
            raise ValueError('missing exact exposure identity')
        frame = (candidate / right['file']).resolve()
        if not frame.is_relative_to(candidate.resolve()):
            raise ValueError('candidate frame escapes capture directory')
        raw = gzip.decompress(frame.read_bytes())
        digest = hashlib.sha256(raw).hexdigest()
        if digest != right['rgba_sha256']:
            raise ValueError(f'candidate source hash verification failed at ordinal {left["ordinal"]}')
        if digest != left['rgba_sha256']:
            raise ValueError(f'decoded image changed at ordinal {left["ordinal"]}: {left["rgba_sha256"]} != {digest}')
        rows.append({'ordinal': left['ordinal'], 'sha256': digest, 'bytes': len(raw),
                     'identity': {key: right[key] for key in keys}})
    report = {'passed': True, 'reference': str(reference), 'candidate': str(candidate),
              'reference_manifest_sha256': hashlib.sha256(encoded).hexdigest(),
              'candidate_manifest_sha256': hashlib.sha256((candidate / 'manifest.json').read_bytes()).hexdigest(),
              'frames': len(rows), 'identity_keys': keys, 'records': rows}
    output.write_text(json.dumps(report, indent=2) + '\n')
    print(f'PASS {len(rows)} exact Chrome digest records')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--reference', type=Path, required=True)
    parser.add_argument('--candidate', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--linear', action='store_true')
    parser.add_argument('--reference-digest', action='store_true')
    args = parser.parse_args()
    comparison = compare_digest if args.reference_digest else compare
    comparison(args.reference, args.candidate, args.output, args.linear)
