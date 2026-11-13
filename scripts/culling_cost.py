#!/usr/bin/env python3
"""Capture-off paired valley route costs. Run under agent_loop.py lock gpu."""
import json
import os
from pathlib import Path
import statistics
import run_v1_showcase as showcase


def quantile(values, q):
    values = sorted(values)
    return values[round((len(values) - 1) * q)]


def summary(rows):
    ticks = {}
    for row in rows[1:-1]:
        ticks.setdefault(row['tick'], {})[row['mode']] = row
    groups = {'entire_route': [], 'occlusion_hides_slots': [], 'open_no_occlusion_saving': []}
    for tick, modes in ticks.items():
        if len(modes) != 3 or any(row['whole_gpu_ns'] is None for row in modes.values()):
            raise showcase.Failure(f'missing paired timing/count at {tick}')
        pair = {mode: row['whole_gpu_ns'] / 1e6 for mode, row in modes.items()}
        groups['entire_route'].append(pair)
        key = 'occlusion_hides_slots' if modes['Occlusion']['counts']['drawn'] < modes['Frustum']['counts']['drawn'] else 'open_no_occlusion_saving'
        groups[key].append(pair)
    result = {}
    for name, pairs in groups.items():
        result[name] = {'ticks': len(pairs)}
        if not pairs:
            continue
        for mode in ('Direct', 'Frustum', 'Occlusion'):
            values = [p[mode] for p in pairs]
            result[name][mode] = {'p50_ms': statistics.median(values), 'p95_ms': quantile(values, .95)}
        result[name]['paired_O_minus_F_ms'] = statistics.median(p['Occlusion'] - p['Frustum'] for p in pairs)
        result[name]['paired_F_minus_D_ms'] = statistics.median(p['Frustum'] - p['Direct'] for p in pairs)
    result['nonempty_prepasses'] = sum(m['Occlusion']['counts']['prepass'] > 0 for m in ticks.values())
    return result


def main():
    run = showcase.new_run('culling-cost')
    route = json.loads(showcase.MOVING_ROUTE.read_text())
    route['render_extent'] = [1920, 1080]
    route_path = run / 'route.json'
    showcase.write_json(route_path, route)
    report = {'commit': showcase.git('rev-parse', 'HEAD'), 'contended': True,
              'capture': 'off', 'extent': route['render_extent'], 'order': 'rotating each tick',
              'warmup_ticks': 179, 'runs': {}}
    report['builds'] = showcase.identities(web=False, built=False)
    with open(run / 'cook.log', 'w') as log:
        if showcase.bounded([str(showcase.COOK), 'cook', str(showcase.SOURCE), str(run / 'store')], log, 120):
            raise showcase.Failure('cook failed')
    report['cooked_root'] = showcase.store_root(run / 'store')
    if report['cooked_root'] != route['cooked_root']:
        raise showcase.Failure('moving route cooked root mismatch')
    for number in (1, 2):
        for recipe in ('portable', 'apple-tile'):
            name = f'{recipe}-{number}'
            env = dict(os.environ, THRUM_SCENE='valley', THRUM_ASSET_STORE=str(run / 'store'),
                       THRUM_CAPTURE_ROUTE=str(route_path), THRUM_CULLING_OUTPUT=str(run / name),
                       THRUM_RECIPE=recipe, THRUM_CULLING_TIMING='on', THRUM_VALLEY_MESHES='off')
            with open(run / f'{name}.log', 'w') as log:
                if showcase.bounded(['cargo', 'bench', '--locked', '--offline', '-p', 'v1-game',
                                     '--bench', 'valley_culling'], log, 1200, env=env, cwd=showcase.REPO):
                    raise showcase.Failure(f'bench failed: {run / name}')
            rows = [json.loads(line) for line in (run / name / 'rows.jsonl').read_text().splitlines()]
            report['runs'][name] = {'header': rows[0], 'summary': summary(rows)}
            showcase.write_json(run / 'run.json', report)
    print(run / 'run.json')


if __name__ == '__main__':
    main()
