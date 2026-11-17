#!/usr/bin/env python3
"""Paired Metal recipe costs; run under the GPU lock after building recipe_cost.

The JSONL source is retained compressed. Thermal observations use Foundation's
read-only thermalState, pmset power/thermal reports and host load, never grants.
Shared-load results are descriptive and cannot qualify a recipe promotion.
"""
import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path
import statistics
import subprocess
import time

from t2_load import LoadRecord


def save(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')


def observe(thermal):
    result = {'monotonic': time.monotonic(), 'load': list(os.getloadavg())}
    for name, command in [('thermal_state', [str(thermal)]),
                          ('power', ['pmset', '-g', 'batt']),
                          ('throttling', ['pmset', '-g', 'therm'])]:
        completed = subprocess.run(command, capture_output=True, text=True, timeout=10)
        result[name] = {'exit': completed.returncode, 'stdout': completed.stdout, 'stderr': completed.stderr}
    return result


def summary(rows):
    result = {}
    for key in ['drive_ns', 'control_admission_ns', 'upload_ns', 'encode_ns', 'view_update_ns', 'extract_ns',
                'tick_cpu_ns', 'render_ns', 'gpu_sum_ns', 'present_gpu_ns', 'uploaded_rows', 'uploaded_bytes',
                'moving', 'queued_ticks', 'pending_frames', 'dropped_light_references']:
        values = sorted(row[key] for row in rows if row.get(key) is not None)
        result[key] = {'count': len(values), 'p50': statistics.median(values) if values else None,
                       'p95': values[round(.95 * (len(values) - 1))] if values else None,
                       'min': min(values) if values else None, 'max': max(values) if values else None}
    names = sorted({name for row in rows for name in (row['gpu_pass_ns'] or {})})
    result['gpu_pass_ns'] = {}
    for name in names:
        values = sorted(value for row in rows for key, value in (row['gpu_pass_ns'] or {}).items() if key == name and value is not None)
        result['gpu_pass_ns'][name] = {'count': len(values), 'p50': statistics.median(values) if values else None,
                                      'p95': values[round(.95 * (len(values) - 1))] if values else None}
    result['culling_counts'] = sorted({json.dumps(row['culling_counts'], sort_keys=True) for row in rows})
    result['native_outcomes'] = sorted({str(row['native_segment']) for row in rows})
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--sustained', action='store_true', help='600 seconds after warmup per recipe, at 1080p')
    parser.add_argument('--resume', action='store_true', help='reuse complete same-binary arms, retaining their original thermal observations')
    parser.add_argument('--witness', choices=['off', 'metal', 'wgsl'], default='off')
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=args.resume)
    thermal_source = out / 'thermal.swift'
    thermal_source.write_text('import Foundation\nprint(ProcessInfo.processInfo.thermalState.rawValue)\n')
    thermal = out / 'thermal'
    subprocess.run(['swiftc', '-module-cache-path', str(out / 'swift-cache'), str(thermal_source), '-o', str(thermal)], check=True)
    binary = Path('tmp/apple-tile-m12/recipe_cost').resolve()
    report = {'commit': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
              'diff_sha256': hashlib.sha256(subprocess.check_output(['git', 'diff', 'HEAD'])).hexdigest(),
              'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(), 'target': 'mac-metal',
              'runner_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              'scope': 'shared-load descriptive costs; no speed or promotion; ON is tap/readback only, without writer',
              'thermal_enum': {'0': 'nominal', '1': 'fair', '2': 'serious', '3': 'critical'},
              'thermal_scope': 'Foundation thermal pressure plus pmset reported CPU/scheduler limits; no die temperature or GPU clock sensor',
              'runs': [], 'passed': False}
    if args.resume:
        previous = json.loads((out / 'report.json').read_text())
        if previous['binary_sha256'] != report['binary_sha256']:
            raise RuntimeError('resume binary changed')
        report = previous
        report['passed'] = False
    load = LoadRecord(out)
    try:
        if args.sustained:
            arms = [(1920, 1080, recipe, 0, 'frustum', 600) for recipe in ['portable', 'apple-tile']]
        else:
            arms = [(w, h, recipe, capture, cull, 0)
                    for w, h in [(1280, 720), (1920, 1080)]
                    for capture in [0, 1]
                    for cull in (['direct', 'frustum', 'occlusion'] if args.witness == 'off' else ['frustum'])
                    for recipe in ['portable', 'portable', 'apple-tile', 'portable', 'apple-tile']]
        for index, (w, h, recipe, capture, cull, seconds) in enumerate(arms):
            name = f'{index:03}-{w}x{h}-{recipe}-{cull}-capture{capture}'
            if any(row['name'] == name for row in report['runs']):
                continue
            env = dict(os.environ, THRUM_RECIPE=recipe,
                       RECIPE_WIDTH=str(w), RECIPE_HEIGHT=str(h), RECIPE_SECONDS=str(seconds),
                       RECIPE_SAMPLES='240', RECIPE_WARMUP='120', RECIPE_CAPTURE=str(capture), RECIPE_CULL=cull)
            env['THRUM_NATIVE_WITNESS'] = args.witness
            env.pop('THRUM_SCENE', None)
            observations = [observe(thermal)]
            raw = out / f'{name}.jsonl'
            with raw.open('w') as output, (out / f'{name}.stderr').open('w') as errors:
                process = subprocess.Popen([str(binary)], env=env, stdout=output, stderr=errors)
                try:
                    while process.poll() is None:
                        try:
                            process.wait(timeout=30)
                        except subprocess.TimeoutExpired:
                            observations.append(observe(thermal))
                    if process.returncode:
                        raise RuntimeError(f'{name} exited {process.returncode}; see stderr')
                finally:
                    if process.poll() is None:
                        process.terminate()
                        process.wait(timeout=10)
            observations.append(observe(thermal))
            save(out / f'{name}-thermal.json', observations)
            records = [json.loads(line) for line in raw.read_text().splitlines()]
            header, footer, retired = records[0], records[-2], records[-1]
            rows = records[1:-2]
            if not footer['footer'] or len(rows) != footer['samples'] or footer['actual_seconds'] < seconds:
                raise RuntimeError(f'{name}: incomplete measurement')
            if header['pipelines_prepared'] != footer['pipelines_end'] or footer['pending_frames']:
                raise RuntimeError(f'{name}: creation in play or outstanding work')
            if footer.get('creations_end', {}).get('violations', 0) or any(
                    phase['phase'] == 'Play' and phase['pipeline_events']
                    for phase in footer.get('creations_end', {}).get('phases', [])):
                raise RuntimeError(f'{name}: generation ledger reports a play pipeline creation')
            compressed = raw.with_suffix('.jsonl.gz')
            with gzip.open(compressed, 'wb') as output:
                output.write(raw.read_bytes())
            raw.unlink()
            report['runs'].append({'name': name, 'header': header, 'footer': footer, 'retired': retired,
                                   'summary': summary(rows), 'thermal': observations,
                                   'source': compressed.name, 'sha256': hashlib.sha256(compressed.read_bytes()).hexdigest()})
            save(out / 'report.json', report)
            print(f'{name}: {len(rows)} rows, {footer["actual_seconds"]:.3f}s', flush=True)
        report['passed'] = True
    finally:
        report['load'] = load.finish()
        save(out / 'report.json', report)


if __name__ == '__main__':
    main()
