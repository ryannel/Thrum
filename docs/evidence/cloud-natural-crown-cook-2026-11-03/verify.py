#!/usr/bin/env python3
"""Verify selected frozen natural-crown evidence and restore its two native fields."""
import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import resource
import shutil

ROOT = Path(__file__).resolve().parent
EXPECTED_FREEZE = 'c099851c2aa79c6aa7cfeb954656ed27e796f365e28b9274bebb6cba7554b1bc'
EXPECTED_META = '29a35c2c171c339142db94e30cce12d1dbb350f2d449500d8192d7459cc2b339'
EXPECTED_FIELDS = {
    'density.r16f': '6cf3de4e99ea6a5f79d32e1506e9375aadd1aaef6f770615ecd2b8341537ee09',
    'distance.r16f': '3dfb512a6c5367fea155801fea924ac22781b5d3f18d8eb5a429741f0e73f0ab',
}
FIELD_BYTES = 884736


def digest(path):
    h = hashlib.sha256()
    with path.open('rb') as source:
        for block in iter(lambda: source.read(65536), b''):
            h.update(block)
    return h.hexdigest()


def local_file(name):
    rel = PurePosixPath(name)
    assert not rel.is_absolute() and '..' not in rel.parts and str(rel) == name
    path = ROOT.joinpath(*rel.parts)
    assert path.is_file() and not path.is_symlink(), name
    return path


def verify_inventory():
    manifest = json.loads((ROOT / 'manifest.json').read_text())
    for name, record in manifest['files'].items():
        path = local_file(name)
        assert path.stat().st_size == record['bytes'], name
        assert digest(path) == record['sha256'], name
    assert digest(ROOT / 'frozen/FREEZE.json') == EXPECTED_FREEZE
    assert digest(ROOT / 'frozen/natural-crown.json') == EXPECTED_META
    frozen = json.loads((ROOT / 'frozen/FREEZE.json').read_text())
    assert frozen['rho_sha256'] == EXPECTED_FIELDS['density.r16f']
    assert frozen['distance_sha256'] == EXPECTED_FIELDS['distance.r16f']
    assert frozen['metadata_sha256'] == EXPECTED_META
    return manifest


def stream_field(name, dest=None):
    path = local_file('selected/' + name + '.gz')
    hash_value = hashlib.sha256()
    count = 0
    with gzip.open(path, 'rb') as source:
        target = dest.open('xb') if dest else None
        try:
            while block := source.read(65536):
                hash_value.update(block)
                count += len(block)
                if target:
                    target.write(block)
        finally:
            if target:
                target.close()
    assert count == FIELD_BYTES and hash_value.hexdigest() == EXPECTED_FIELDS[name], name
    return count


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, help='new directory for restored fields and original metadata; refuses existing paths')
    args = parser.parse_args()
    manifest = verify_inventory()
    if args.output is None:
        for name in EXPECTED_FIELDS:
            stream_field(name)
    else:
        out = args.output.expanduser().absolute()
        assert not out.exists() and not out.is_symlink(), 'restore output already exists'
        assert out.parent.is_dir(), 'restore parent must exist'
        os.mkdir(out, 0o700)
        for name in EXPECTED_FIELDS:
            stream_field(name, out / name)
        for name in ('natural-crown.json', 'field.json', 'certificates.json', 'verification.json'):
            shutil.copyfile(local_file('frozen/' + name), out / name)
        assert digest(out / 'natural-crown.json') == EXPECTED_META
        print('restored:', out)
    print(json.dumps({'verified_files': len(manifest['files']), 'freeze_sha256': EXPECTED_FREEZE,
                      'field_sha256': EXPECTED_FIELDS, 'peak_rss_bytes': resource.getrusage(resource.RUSAGE_SELF).ru_maxrss}, sort_keys=True))


if __name__ == '__main__':
    main()
