#!/usr/bin/env python3
"""M10 labelled native/twin submitted-output proof. Run under the shared GPU lock.

Keeps every T2 source and provenance record. Fixed paused-world samples compare
exact hashes; realtime motion has separate host-time semantics and no parity claim.
"""
import argparse
import array
import json
import os
from pathlib import Path
import subprocess

from apple_tile_m0 import ROOT, ROUTE, digest, run_t2, finish_t2, save, validate_store


def run(args):
    args.binary = args.binary.resolve()
    args.store = args.store.resolve()
    args.output = args.output.resolve()
    validate_store(args.store, json.loads(ROUTE.read_text()))
    args.output.mkdir(parents=True, exist_ok=False)
    report = {'passed': False, 'revision': subprocess.check_output(
        ['git', 'rev-parse', 'HEAD'], text=True).strip(),
        'binary_sha256': digest(args.binary), 'runs': {}, 'comparisons': {}}
    prior = dict(os.environ)
    try:
        for recipe in ['portable', 'apple-tile']:
            os.environ['THRUM_RECIPE'] = recipe
            for mode in ['still', 'fixed']:
                for source in ['sdr', 'scene-linear', 'native-output', 'native-history']:
                    runs = []
                    for selection in ['wgsl', 'metal']:
                        os.environ['THRUM_NATIVE_WITNESS'] = selection
                        name = f'{recipe}-{mode}-{source}-{selection}'
                        directory = args.output / name
                        result = run_t2(args, args.output, name,
                                        {'mode': mode, 'case': 'pan', 'source': source})
                        result.update(finish_t2(directory))
                        runtime = json.loads((directory / 'runtime.json').read_text())
                        expected = 'Wgsl' if selection == 'wgsl' else 'Metal'
                        if runtime['native_witness']['selection'] != expected:
                            raise ValueError(f'{name}: wrong native selection')
                        for checkpoint in result['selected']:
                            path = directory / checkpoint['file']
                            if path.suffix == '.rgba16f':
                                values = array.array('H', path.read_bytes())
                                if any((v & 0x7fff) > 0x7bff or
                                       (v & 0x8000 and v & 0x7fff) for v in values):
                                    raise ValueError(f'{name}: nonfinite or negative half texel')
                        report['runs'][name] = result
                        runs.append(runtime)
                        save(args.output / 'manifest.json', report)
                    left, right = runs
                    hashes = [r['writer_result']['Ok']['frame_sha256'] for r in runs]
                    identities = [[{k: row[k] for k in (
                        'submission_id', 'tick', 'history_epoch', 'exposure_gpu_words') if k in row}
                        for row in r['images']] for r in runs]
                    if hashes[0] != hashes[1] or identities[0] != identities[1]:
                        raise ValueError(f'{recipe}/{mode}/{source}: native/twin exact mismatch')
                    report['comparisons'][f'{recipe}-{mode}-{source}'] = {
                        'exact': True, 'frames': len(hashes[0]), 'identities_equal': True}
            os.environ['THRUM_NATIVE_WITNESS'] = 'metal'
            name = f'{recipe}-realtime-native'
            route = args.output / 'runtime-route.json'
            save(route, dict(json.loads(ROUTE.read_text()), render_extent=[1920, 1080]))
            report['runs'][name] = run_t2(args, args.output, name,
                {'mode': 'realtime', 'case': 'route', 'route': route})
            report['runs'][name].update(finish_t2(args.output / name, viewing=True))
        report['passed'] = True
    except Exception as error:
        report['error'] = str(error)
    finally:
        os.environ.clear()
        os.environ.update(prior)
        save(args.output / 'manifest.json', report)
    print(json.dumps({'passed': report['passed'], 'error': report.get('error')}))
    return 0 if report['passed'] else 1


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, default=Path('target/release/v1-game'))
    parser.add_argument('--store', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    raise SystemExit(run(parser.parse_args()))
