"""Compare captured GPU common-light/path probes with independent optical solves."""
from __future__ import annotations

import argparse
import json
from pathlib import Path

import numpy as np

from oracle import (Field, default_scene, geometry_visibility, noncloud_light_tau,
                    reflect_direction, trace_joint, unit)
from validate_cache import cache_depth, digest, rgba, sample_grid


def errors(actual, reference):
    a, r = np.asarray(actual), np.asarray(reference)
    return {'absolute': (a-r).tolist(), 'relative': ((a-r)/np.maximum(abs(r), 1)).tolist()}


def validate(probe, baseline=None):
    path = Path(str(probe)+'-queries.json')
    capture = json.loads(path.read_text())
    raw_scene = capture['scene']
    scene = default_scene() | raw_scene | {'sun_direction': raw_scene['sun']}
    lower = np.array(raw_scene['cloud_min'])
    field = Field(rgba(Path(str(probe)+'-density.f16'), 128)[..., 0], lower)
    prefix_side = next(r['size'][0] for r in capture['identity']['resource_bytes'] if r['name'] == 'directional prefix')
    prefix = rgba(Path(str(probe)+'-prefix.f16'), prefix_side)
    ambient_grid = rgba(Path(str(probe)+'-ambient.f16'), 32)
    ambient = lambda p: sample_grid(ambient_grid, (p-lower)/512)[:3]*5000
    depth = lambda p: cache_depth(prefix, p, raw_scene)
    light_records, traces = [], []
    if len(capture['queries']) > 64:
        raise ValueError('at most64 bounded GPU queries admitted')
    for query in capture['queries']:
        result = np.array(query['result']).reshape(8, 4)
        origin, ray = np.array(query['origin']), unit(query['ray'])
        sun = unit(scene['sun_direction'])
        noncloud = noncloud_light_tau(origin, sun, 4000, scene)
        visibility = geometry_visibility(origin, sun, scene)
        cached = depth(origin)
        physical_tau = field.tau(origin, sun, step=.5)
        common_cached = np.array(scene['sun_rgb'])*visibility*np.exp(-noncloud-result[4, 0])
        common_physical = np.array(scene['sun_rgb'])*visibility*np.exp(-noncloud-physical_tau)
        light_records.append({'name': query['name'], 'origin': origin.tolist(),
                              'gpu_visibility': result[3, 3], 'reference_visibility': visibility,
                              'gpu_sigma': result[4, 3], 'reference_sigma': float(field.sample(origin)),
                              'gpu_tau_mips': result[4, :3].tolist(), 'cache_reader_tau': cached.tolist(),
                              'physical_primary_tau': physical_tau,
                              'gpu_ambient': result[5, :3].tolist(), 'cache_reader_ambient': ambient(origin).tolist(),
                              'gpu_common_incident': result[3, :3].tolist(),
                              'reference_using_gpu_tau': common_cached.tolist(),
                              'reference_using_physical_tau': common_physical.tolist(),
                              'incident_closure_error': errors(result[3, :3], common_cached),
                              'incident_physical_error': errors(result[3, :3], common_physical),
                              'noncloud_tau_error': (result[7, :3]+result[5, 3]-noncloud).tolist()})
        if query['options'][2] < .5:
            continue
        if result[2, 3] != 1:
            raise ValueError(f'GPU trace reported invalid: {query["name"]}')
        # The actual GPU terminal isolates volume transport from terminal material
        # and mirror-secondary evaluation; the latter is cross-checked separately.
        options = {'distance': float(result[0, 3]), 'scene': scene, 'ambient': ambient,
                   'terminal': result[2, :3], 'clip_geometry': False,
                   'cloud_on': bool(query['options'][1])}
        fine_cached = trace_joint(field, origin, ray, step=1, light_depths=depth, **options)
        coarse_cached = trace_joint(field, origin, ray, step=4, light_depths=depth, **options)
        fine_physical = trace_joint(field, origin, ray, step=1, light_step=.5, **options)
        traces.append({'name': query['name'], 'origin': origin.tolist(), 'ray': ray.tolist(),
                       'gpu_valid': result[2, 3], 'gpu_distance': result[0, 3], 'gpu_hit_kind': result[1, 3],
                       'gpu_S': result[0, :3].tolist(), 'gpu_T': result[1, :3].tolist(),
                       'gpu_terminal': result[2, :3].tolist(), 'gpu_composed': result[6, :3].tolist(),
                       'cpu_1m_with_actual_caches': fine_cached,
                       'cpu_4m_with_actual_caches': coarse_cached,
                       'cpu_1m_direct_sun_tau_actual_ambient': fine_physical,
                       'gpu_S_error_vs_1m_cached': errors(result[0, :3], fine_cached['S']),
                       'gpu_T_error_vs_1m': errors(result[1, :3], fine_cached['T']),
                       'gpu_composed_error_vs_1m_cached': errors(result[6, :3], fine_cached['radiance']),
                       'view_4m_minus_1m_S': (np.array(coarse_cached['S'])-fine_cached['S']).tolist(),
                       'cache_minus_direct_1m_S': (np.array(fine_cached['S'])-fine_physical['S']).tolist(),
                       'composition_algebra_error': (result[6, :3]-(result[0, :3]+result[1, :3]*result[2, :3])).tolist()})
    by_name = {q['name']: q for q in traces}
    reflection = []
    for q in traces:
        if q['gpu_hit_kind'] != 3:
            continue
        secondary_name = 'mirror-secondary-'+q['name'].removeprefix('main-')
        if secondary_name not in by_name:
            continue
        secondary = by_name[secondary_name]
        origin, ray = np.array(q['origin']), np.array(q['ray'])
        hit = origin+ray*q['gpu_distance']
        reflected = reflect_direction(ray)
        expected_origin = hit+reflected*.001
        near = q['cpu_1m_with_actual_caches']
        far = secondary['cpu_1m_with_actual_caches']
        full_cpu = np.array(near['S'])+np.array(near['T'])*far['radiance']
        reflection.append({'name': q['name'], 'secondary': secondary_name,
                           'mirror_hit': hit.tolist(),
                           'secondary_origin_error': (np.array(secondary['origin'])-expected_origin).tolist(),
                           'secondary_ray_error': (np.array(secondary['ray'])-reflected).tolist(),
                           'gpu_terminal_minus_separate_secondary': (np.array(q['gpu_terminal'])-secondary['gpu_composed']).tolist(),
                           'cpu_near_far_composed': full_cpu.tolist(),
                           'gpu_full_error': errors(q['gpu_composed'], full_cpu)})
    def max_error(records, key, component='absolute'):
        return float(np.max(np.abs([q[key][component] for q in records]))) if records else 0
    parity = []
    if baseline is not None:
        previous = json.loads(Path(str(baseline)+'-queries.json').read_text())
        old = {q['name']: q for q in previous['queries']}
        for query in capture['queries']:
            a, b = np.array(query['result']).reshape(8, 4), np.array(old[query['name']]['result']).reshape(8, 4)
            parity.append({'name': query['name'], 'S_T_terminal_composed_mismatches': int(np.count_nonzero(a[[0, 1, 2, 6]] != b[[0, 1, 2, 6]])),
                           'S_T_terminal_composed_max_error': float(np.max(abs(a[[0, 1, 2, 6]]-b[[0, 1, 2, 6]]))),
                           'common_incident_prefix_sigma_mismatches': int(np.count_nonzero(a[[3, 4]] != b[[3, 4]])),
                           'ambient_required': bool(a[4, 3] > 0),
                           'ambient_difference': (a[5, :3]-b[5, :3]).tolist(),
                           'note': 'Ambient at zero cloud coefficient is not consumed by transport.'})
    return {'scope': 'Actual GPU query traces; independent float64 coefficient/source solves. '
                     'Actual GPU terminal and ambient isolate view/light-cache transport; no image or performance claim.',
            'query_input_sha256': digest(path), 'captured_identity': capture['identity'], 'scene': raw_scene,
            'common_light': light_records, 'traces': traces, 'reflection_composition': reflection,
            'baseline_parity': {'baseline': str(baseline) if baseline else None, 'queries': parity},
            'summary': {'all_trace_valid': all(q['gpu_valid'] == 1 for q in traces),
                        'max_incident_closure_relative_error': max_error(light_records, 'incident_closure_error', 'relative'),
                        'max_incident_physical_relative_error': max_error(light_records, 'incident_physical_error', 'relative'),
                        'max_gpu_view_S_relative_error_vs_1m_cached': max_error(traces, 'gpu_S_error_vs_1m_cached', 'relative'),
                        'max_gpu_view_T_absolute_error': max_error(traces, 'gpu_T_error_vs_1m'),
                        'max_gpu_composed_relative_error_vs_1m_cached': max_error(traces, 'gpu_composed_error_vs_1m_cached', 'relative'),
                        'max_mirror_full_relative_error': max_error(reflection, 'gpu_full_error', 'relative')}}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--probe', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--baseline', type=Path)
    args = parser.parse_args()
    result = validate(args.probe, args.baseline)
    args.out.mkdir(parents=True, exist_ok=True)
    (args.out/'query-validation.json').write_text(json.dumps(result, indent=2, allow_nan=False)+'\n')
    lines = ['# Actual GPU query validation', '', '| Quantity | Result |', '| --- | ---: |']
    lines += [f'| {k} | {v} |' for k, v in result['summary'].items()]
    lines += ['', result['scope'], '',
              'Per-ray records separate GPU versus1m view integration with the same actual light/ambient caches, '
              '4m versus1m view quadrature, and cached versus direct fine Sun depth. '
              'Mirror records compare the separate secondary ray and compose camera-to-plane foreground exactly once.']
    if args.baseline:
        lines += ['', 'Baseline query parity:', '', '```json', json.dumps(result['baseline_parity'], indent=2), '```']
    (args.out/'query-validation.md').write_text('\n'.join(lines)+'\n')
    print(json.dumps(result['summary'], indent=2))


if __name__ == '__main__':
    main()
