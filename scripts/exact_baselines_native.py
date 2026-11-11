#!/usr/bin/env python3
"""Run an APPLE-TILE matrix with synchronous retention of completed captures.

The owner run function, comparisons and saved reports execute as supplied by the
owner module. Its report publisher releases verified outputs before returning,
so the next capture's disk preflight observes that released space.
"""
import argparse
import importlib
from pathlib import Path

from exact_baselines import archive_completed_runs

MATRICES = ['apple_tile_m1', 'apple_tile_m2']


def run_matrix(matrix, binary, store, output, records):
    owner = importlib.import_module(matrix)
    original_save = owner.save
    manifest = output.resolve() / 'manifest.json'

    def publish(path, value):
        original_save(path, value)
        if path.resolve() == manifest:
            archive_completed_runs(output, records)

    args = argparse.Namespace(binary=binary, store=store, output=output)
    owner.save = publish
    try:
        return owner.run(args)
    finally:
        owner.save = original_save


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--matrix', choices=MATRICES, required=True)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--store', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--records', type=Path, required=True)
    args = parser.parse_args()
    return run_matrix(args.matrix, args.binary, args.store, args.output, args.records)


if __name__ == '__main__':
    raise SystemExit(main())
