"""Retain observer costs and every strict over-period interval with its game join."""
import argparse
import importlib.util
import json
from pathlib import Path
import re
import statistics


def named_gpu_samples(arm):
    samples = {}
    unmeasured = set()
    for block in (arm / 'gpu-samples.txt').read_text().split('FrameSample {')[1:]:
        ordinal = int(re.search(r'submission_order: (\d+)', block)[1])
        if ordinal in samples:
            raise ValueError('duplicate GPU sample ordinal')
        passes = [{'name': name, 'milliseconds': float(value) / 1e6}
                  for name, value in re.findall(r'PassTiming\s*\{\s*name: "([^"]+)",\s*nanoseconds: ([\d.]+)', block)]
        missing = re.search(r'unmeasured: \[([^]]*)\]', block)
        if missing:
            unmeasured.update(re.findall(r'"([^"]+)"', missing[1]))
        samples[ordinal] = {'gpu_named_pass_sum_ms': sum(row['milliseconds'] for row in passes) if passes else None,
                            'named_passes': passes}
        for field in ('render_cpu', 'encode_cpu'):
            duration = re.search(r'\b' + field + r': ([\d.]+)(ns|µs|ms|s),', block)
            samples[ordinal][field + '_elapsed_ms'] = (float(duration[1]) * {'ns': 1e-6, 'µs': .001, 'ms': 1, 's': 1000}[duration[2]]
                                                     if duration else None)
    return samples, sorted(unmeasured)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    previous = Path(__file__).resolve().parent.parent / 'mac-m2-presented-20261111/analyse-arms.py'
    spec = importlib.util.spec_from_file_location('mac_m2_arm_costs', previous)
    costs = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(costs)
    report = {}
    for name in ('off-traced', 'on-untraced', 'on-traced'):
        arm = args.root / name
        result = costs.analyse(arm)
        samples, unmeasured = named_gpu_samples(arm)
        markers = costs.table(arm / 'signposts.tsv')
        start, end = float(markers[0]['media_before']), float(markers[-1]['media_after'])
        wanted = {int(row['submission_id']) for row in costs.table(arm / 'surface.tsv')
                  if start <= float(row['media_seconds']) <= end}
        values = [samples[ordinal]['gpu_named_pass_sum_ms'] for ordinal in wanted
                  if ordinal in samples and samples[ordinal]['gpu_named_pass_sum_ms'] is not None]
        # The historical helper reads the old unnamed array. Current FrameSample
        # carries SceneGpuTimings with named passes, including the present pass.
        result['statistics'].pop('gpu_scene_query_ms')
        result.pop('missing_gpu_scene_query_ordinals')
        result['statistics']['gpu_named_pass_sum_ms'] = costs.summary(values)
        result['missing_named_gpu_sample_ordinals'] = sorted(ordinal for ordinal in wanted
                                                           if ordinal not in samples or not samples[ordinal]['named_passes'])
        result['unmeasured_graph_pass_names'] = unmeasured
        result['named_gpu_scope'] = 'sum of reported graph pass queries, including V1 surface present; separate present query is also retained and must not be added again'
        for missed in result.get('marked_missed_refreshes', []):
            missed['neighboring_samples'] = {str(ordinal): samples.get(ordinal)
                                             for ordinal in range(max(0, missed['from'] - 2), missed['to'] + 4)}
            for side, ordinal in (('from_sample', missed['from']), ('to_sample', missed['to'])):
                if missed[side] is not None:
                    missed[side].pop('gpu_scene_query_ms', None)
                    missed[side].update(samples.get(ordinal, {}))
        rows = [line.split() for line in (arm / 'system-iostat.txt').read_text().splitlines()
                if line.strip() and line.strip()[0].isdigit()]
        idle = [float(row[-4]) for row in rows[1:]]
        result['collection_cpu_idle_percent'] = {
            'count': len(idle), 'minimum': min(idle), 'median': statistics.median(idle),
            'scope': 'one-second collection samples including export/finalization, excluding since-boot row'}
        if (arm / 'witness.json').exists():
            witness = json.loads((arm / 'witness.json').read_text())
            drawables = {row['drawable']: row for row in witness['drawables']}
            strict = []
            for interval in witness['intervals_over_one_period']:
                if not interval['overlaps_whole']:
                    continue
                before, after = drawables[interval['from']], drawables[interval['to']]
                item = dict(interval)
                item.update(from_presented_media_ns=before['presented_media_ns'],
                            to_presented_media_ns=after['presented_media_ns'],
                            from_present_call_media_seconds=before['present_call_media_seconds'],
                            to_present_call_media_seconds=after['present_call_media_seconds'],
                            from_sequence=before['sequence'], to_sequence=after['sequence'],
                            from_yaw=before['yaw'], to_yaw=after['yaw'],
                            opportunity_gap_ms=(float(after['opportunity_media_seconds']) - float(before['opportunity_media_seconds'])) * 1000,
                            callback_target_gap_ms=(float(after['callback_target_media_seconds']) - float(before['callback_target_media_seconds'])) * 1000,
                            from_call_to_present_ms=before['call_to_present_ms'],
                            to_call_to_present_ms=after['call_to_present_ms'],
                            deviation_from_one_period_ns=interval['interval_ns'] - float(witness['refresh_period_ns']),
                            gpu_sample_ordinals=[interval['from'], interval['to']],
                            from_named_gpu=samples.get(interval['from']),
                            to_named_gpu=samples.get(interval['to']),
                            host_load=result['load'],
                            interpretation=('Long interval. Neighboring game, opportunity, target and GPU records are retained in marked_missed_refreshes and gpu-samples.txt. Run-wide CPU/disk observations cannot isolate an instantaneous cause.'
                                            if interval['missed_refresh'] else
                                            'Strictly over one nominal period, while rounding to one refresh. The exact deviation is retained. These small timestamp variations are reported separately from the long intervals.'))
                strict.append(item)
            result['every_marked_strict_over_period_interval'] = strict
        report[name] = result
    report['limits'] = ('OFF supplies OS-reported presented cadence. ON intervals are under capture. '
                        'Callback targets, opportunities, present calls, GPU queries, image times and movie PTS retain their own clocks. '
                        'Run order and shared host load confound causal observer-cost percentages. Kernel swaps, scan-out, photons, input-to-photon and physical iPhone timing are unmeasured.')
    args.output.write_text(json.dumps(report, indent=2) + '\n')
