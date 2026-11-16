#!/usr/bin/env python3
"""Capture one native recovery arm, or compare exact submitted checkpoints.

Run `capture` under agent_loop.py lock gpu. Each arm uses the ordinary V1 game
and T2 tap. `compare` is CPU-only and compares every verified full-frame hash.
Realtime arms are reviewed separately and never treated as deterministic pairs.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
from types import SimpleNamespace

from apple_tile_m0 import ROUTE, digest, finish_t2, run_t2, save, validate_store


def capture(args):
    args.binary = args.binary.resolve()
    args.store = args.store.resolve()
    args.output = args.output.resolve()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    expected_content = json.loads(ROUTE.read_text())
    validate_store(args.store, expected_content)
    environment = dict(os.environ)
    record = {'passed': False, 'revision': subprocess.check_output(
        ['git', 'rev-parse', 'HEAD'], text=True).strip(),
        'binary_sha256': digest(args.binary),
        'cooked_root': expected_content['cooked_root'],
        'recipe': args.recipe, 'selection': args.selection, 'arm': args.arm,
        'mode': args.mode, 'source': args.source}
    try:
        os.environ.update(THRUM_RECIPE=args.recipe,
                          THRUM_NATIVE_WITNESS=args.selection,
                          THRUM_CAPTURE_NATIVE_RECOVERY=args.arm)
        result = run_t2(args, args.output.parent, args.output.name,
                        {'mode': args.mode, 'case': 'pan', 'source': args.source,
                         'admission': 'wait' if args.mode == 'realtime' else 'omit'})
        result.update(finish_t2(args.output, viewing=args.mode == 'realtime'))
        runtime = json.loads((args.output / 'runtime.json').read_text())
        actions = [row for row in runtime['diagnostic_actions']
                   if row['kind'] == 'native-provider-checkpoint']
        if [row['fault'] for row in actions] != ['Surface', 'CommandError', 'DeviceLoss', 'Reject']:
            raise ValueError('did not exercise all four recovery checkpoints')
        if args.arm == 'faults':
            if actions[-1]['new_generation'] != (2 if args.selection == 'off' else 3):
                raise ValueError('wrong device recovery generation')
            if args.selection != 'off' and runtime['native_witness']['selection'] != 'Wgsl':
                raise ValueError('provider rejection failed to select prepared twin')
        record.update(result, actions=actions, passed=True)
    except Exception as error:
        record['error'] = str(error)
    finally:
        os.environ.clear()
        os.environ.update(environment)
        save(args.output.parent / f'{args.output.name}-proof.json', record)
    print(json.dumps({key: record[key] for key in ['passed', 'revision']} |
                     {'error': record.get('error')}))
    return 0 if record['passed'] else 1


def compare(args):
    runs = [json.loads((path / 'runtime.json').read_text())
            for path in [args.control, args.candidate]]
    proofs = [json.loads((path.parent / f'{path.name}-proof.json').read_text())
              for path in [args.control, args.candidate]]
    if not all(row['passed'] and row['mode'] == 'fixed' for row in proofs):
        raise ValueError('exact comparison requires complete verified fixed arms')
    if proofs[0]['binary_sha256'] != proofs[1]['binary_sha256']:
        raise ValueError('arms use different binaries')
    if any(proofs[0][key] != proofs[1][key] for key in ['recipe', 'source', 'cooked_root']):
        raise ValueError('arms use different recipe, source stage or cooked content')
    hashes = [run['writer_result']['Ok']['frame_sha256'] for run in runs]
    identities = [[{'tick': image['tick'], 'epoch': image['epoch'], 'timeline': image['timeline'],
                    'alpha': image['alpha'], 'history_epoch': image['history_epoch'],
                    'exposure': (image['exposure_gpu_words'][:5] + image['exposure_gpu_words'][7:])
                        if image.get('exposure_gpu_words') else None}
                   for image in run['images']] for run in runs]
    # Absolute device/presenter/history serials intentionally change at recovery.
    # Both arms reset at the same deterministic opportunity; phase is compared
    # relative to those checkpoints, not across unrelated generation identifiers.
    failures = [i for i, (left, right) in enumerate(zip(hashes[0], hashes[1])) if left != right]
    result = {'passed': len(hashes[0]) == len(hashes[1]) and not failures and identities[0] == identities[1],
              'control': str(args.control), 'candidate': str(args.candidate),
              'frames': list(map(len, hashes)), 'different_frames': failures,
              'sample_and_exposure_equal': identities[0] == identities[1],
              'binary_sha256': proofs[0]['binary_sha256'],
              'contract': 'exact full-frame bytes; deterministic opportunity and reset-relative phase; no tolerance'}
    save(args.output, result)
    print(json.dumps(result))
    return 0 if result['passed'] else 1


def campaign(args):
    """One recipe's bounded capture-only GPU campaign; no build or merge inside."""
    args.output = args.output.resolve()
    args.output.mkdir(parents=True, exist_ok=False)
    summary = {'passed': False, 'recipe': args.recipe, 'arms': [], 'comparisons': []}
    matrix = []
    for source in ['sdr', 'scene-linear']:
        matrix.extend((source, 'off', arm, 'fixed') for arm in ['control', 'faults'])
    for source in ['native-output', 'native-history']:
        matrix.extend((source, selection, arm, 'fixed') for selection, arm in
                      [('metal', 'control'), ('metal', 'faults'), ('wgsl', 'faults')])
    matrix.extend(('sdr', selection, arm, 'realtime') for selection in ['off', 'metal']
                  for arm in ['control', 'faults'])
    try:
        for source, selection, arm, mode in matrix:
            name = f'{mode}-{source}-{selection}-{arm}'
            current = SimpleNamespace(**vars(args) | {'source': source, 'selection': selection,
                                      'arm': arm, 'mode': mode, 'output': args.output / name})
            if capture(current):
                raise ValueError(f'{name} failed; its source and failure record are retained')
            summary['arms'].append(name)
            save(args.output / 'campaign.json', summary)
        for source in ['sdr', 'scene-linear', 'native-output', 'native-history']:
            selection = 'off' if source in ['sdr', 'scene-linear'] else 'metal'
            pairs = [(f'fixed-{source}-{selection}-control', f'fixed-{source}-{selection}-faults')]
            if selection == 'metal':
                pairs.append((f'fixed-{source}-metal-faults', f'fixed-{source}-wgsl-faults'))
            for left, right in pairs:
                path = args.output / f'{left}-vs-{right}.json'
                if compare(SimpleNamespace(control=args.output / left, candidate=args.output / right, output=path)):
                    raise ValueError(f'{left}/{right} exact mismatch')
                summary['comparisons'].append(path.name)
        summary['passed'] = True
    except Exception as error:
        summary['error'] = str(error)
    save(args.output / 'campaign.json', summary)
    print(json.dumps(summary))
    return 0 if summary['passed'] else 1


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    run = commands.add_parser('capture')
    run.add_argument('--binary', type=Path, default=Path('target/release/v1-game'))
    run.add_argument('--store', type=Path, required=True)
    run.add_argument('--output', type=Path, required=True)
    run.add_argument('--recipe', choices=['portable', 'apple-tile'], required=True)
    run.add_argument('--selection', choices=['off', 'metal', 'wgsl'], required=True)
    run.add_argument('--arm', choices=['control', 'faults'], required=True)
    run.add_argument('--mode', choices=['fixed', 'realtime'], required=True)
    run.add_argument('--source', choices=['sdr', 'scene-linear', 'native-output', 'native-history'], required=True)
    pair = commands.add_parser('compare')
    pair.add_argument('--control', type=Path, required=True)
    pair.add_argument('--candidate', type=Path, required=True)
    pair.add_argument('--output', type=Path, required=True)
    suite = commands.add_parser('campaign')
    suite.add_argument('--binary', type=Path, required=True)
    suite.add_argument('--store', type=Path, required=True)
    suite.add_argument('--output', type=Path, required=True)
    suite.add_argument('--recipe', choices=['portable', 'apple-tile'], required=True)
    arguments = parser.parse_args()
    raise SystemExit({'capture': capture, 'compare': compare, 'campaign': campaign}[arguments.command](arguments))
