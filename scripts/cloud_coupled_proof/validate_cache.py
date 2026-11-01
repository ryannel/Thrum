"""Read actual GPU cache exports and compare with independent metre-scale optics.

The captured scene owns the light/world transform. This reader reconstructs
physical node positions and samples GPU products, but imports no shader source.
"""
from __future__ import annotations

import argparse
import hashlib
import gzip
import json
from pathlib import Path

import numpy as np

from oracle import Field, ambient_reference, default_scene, read_clip, read_density, unit


def product_bytes(path):
    path = Path(path)
    return path.read_bytes() if path.exists() else gzip.decompress(Path(str(path)+'.gz').read_bytes())


def digest(path):
    return hashlib.sha256(product_bytes(path)).hexdigest()


def rgba(path, side):
    values = np.frombuffer(product_bytes(path), '<f2')
    if values.size != side**3*4:
        raise ValueError('RGBA16F cache byte length disagrees with declared dimensions')
    return values.reshape(side, side, side, 4).astype(np.float64)


def sample_grid(values, uv):
    """Independent trilinear cache reader, with hardware clamp-to-edge semantics."""
    q = np.clip(np.asarray(uv)*np.array(values.shape[:3][::-1])-.5,
                0, np.array(values.shape[:3][::-1])-1)
    lo = np.floor(q).astype(int)
    hi = np.minimum(lo+1, np.array(values.shape[:3][::-1])-1)
    weight = q-lo
    total = np.zeros(values.shape[3:])
    for corner in np.ndindex(2, 2, 2):
        index = np.where(corner, hi, lo)
        w = np.prod(np.where(corner, weight, 1-weight))
        total += w*values[index[2], index[1], index[0]]
    return total


def cache_depth(values, point, scene):
    basis = scene['basis']
    centre = np.asarray(scene['cloud_min'])+256
    local = np.array([np.dot(np.asarray(point)-centre, basis[k]) for k in ['u', 'v', 'w']])
    uv = local/(2*np.array(basis['half']))+.5
    if np.any(uv[:2] < 0) or np.any(uv[:2] > 1) or uv[2] <= 0:
        return np.zeros(3)
    uv[2] = min(uv[2], 1)
    interior = values.shape[0]-2
    return sample_grid(values, (uv*interior+1)/(interior+2))[:3]


def half_mips(field):
    fields = [field]
    for _ in range(2):
        a = fields[-1].values
        side = a.shape[0]//2
        average = a.reshape(side, 2, side, 2, side, 2).mean(axis=(1, 3, 5))
        fields.append(Field(average.astype(np.float16).astype(np.float64), field.lower, field.extent))
    return fields


def reference_depths(fields, point, sun, step):
    return np.array([field.tau(point, sun, step=step) for field in fields])


def depth_result(identifier, point, cached, fields, sun):
    primary = reference_depths(fields, point, sun, 1)
    refined = reference_depths(fields, point, sun, .5)
    result = {'id': identifier, 'point': np.asarray(point).tolist(), 'gpu_tau_mips': cached.tolist(),
              'reference_tau_1m': primary.tolist(), 'reference_tau_half_m': refined.tolist(),
              'quadrature_delta': abs(primary-refined).tolist(),
              'gpu_tau_error': (cached-refined).tolist(),
              'gpu_T_error': (np.exp(-cached)-np.exp(-refined)).tolist()}
    result['higher_order_T_error'] = [(np.exp(-scale*cached[mip])-np.exp(-scale*refined[mip]))
                                     for scale, mip in [(.2, 1), (.04, 2), (.008, 2)]]
    return result


def select_points(field, scene):
    active = np.argwhere(field.values > 0)
    values = field.values[tuple(active.T)]
    selected = [active[np.argmax(values)]]
    # Density ranks and spatial ordering supply both dense interiors and thin edges.
    for index in np.argsort(values)[np.linspace(0, len(active)-1, 6, dtype=int)]:
        selected.append(active[index])
    points = [('support_centre', field.lower+256),
              ('outside_source_facing', field.lower+256+unit(scene['sun'])*900),
              ('outside_transverse', field.lower+256+np.asarray(scene['basis']['u'])*900)]
    sun = unit(scene['sun'])
    for index, cell in enumerate(selected):
        point = field.lower+(cell[::-1]+.5)*field.extent/field.values.shape[0]
        points.append((f'density_rank_{index}', point))
        points.append((f'ground_receiver_{index}', point-sun*(point[1]/sun[1])))
    return points


def validate(probe, clip, ambient_active_local=None, baseline=None):
    manifest_path = Path(str(probe)+'.json')
    captured = json.loads(manifest_path.read_text())
    scene = captured['scene']
    density_path, prefix_path, ambient_path = [Path(str(probe)+suffix) for suffix in
                                             ['-density.f16', '-prefix.f16', '-ambient.f16']]
    gpu = (rgba(density_path, 128)[..., 0] if len(product_bytes(density_path)) == 128**3*8
           else read_density(density_path))
    decoded = read_clip(clip, scene['age'])
    rounded = decoded.astype(np.float16).astype(np.float64)
    mismatch = gpu != rounded
    field = Field(gpu, scene['cloud_min'])
    fields = half_mips(field)
    prefix_side = next(resource['size'][0] for resource in captured['identity']['resource_bytes']
                       if resource['name'] == 'directional prefix')
    interior = prefix_side-2
    prefix, ambient = rgba(prefix_path, prefix_side), rgba(ambient_path, 32)
    mip_parity = []
    for level in (1, 2):
        path = Path(str(probe)+f'-mip{level}.f16')
        if path.exists() or Path(str(path)+'.gz').exists():
            actual = rgba(path, 128 >> level)[..., 0]
            expected = fields[level].values
            mip_parity.append({'level': level, 'mismatched_voxels': int(np.count_nonzero(actual != expected)),
                               'max_sigma_error': float(np.max(abs(actual-expected))), 'sha256': digest(path)})
            fields[level] = Field(actual, field.lower, field.extent)
    if not np.isfinite(prefix).all() or not np.isfinite(ambient).all():
        raise ValueError('actual GPU caches contain nonfinite values')
    points = select_points(field, scene)
    queries = [depth_result(identifier, point, cache_depth(prefix, point, scene), fields, scene['sun'])
               for identifier, point in points]
    # Direct node reads isolate cache integration/half storage from interpolation.
    active_nodes = np.argwhere(prefix[1:-1, 1:-1, 1:-1, 0] > 0)+1
    node_depths = prefix[tuple(active_nodes.T)][:, 0]
    ranks = np.argsort(node_depths)[np.linspace(0, len(active_nodes)-1, 10, dtype=int)]
    basis = scene['basis']
    matrix = np.array([basis[k] for k in ['u', 'v', 'w']])
    nodes = []
    for rank in ranks:
        zyx = active_nodes[rank]
        normalized = (zyx[::-1]-.5)/interior*2-1
        point = field.lower+256+(normalized*np.asarray(basis['half']))@matrix
        nodes.append(depth_result(f'node_zyx_{zyx.tolist()}', point, prefix[tuple(zyx)][:3], fields, scene['sun']))
    optical_scene = default_scene() | scene | {'sun_direction': scene['sun']}
    ambient_results = []
    # Exact ambient nodes avoid assigning spatial cache interpolation to coarse quadrature.
    ambient_points = [point for identifier, point in points if identifier.startswith('density_rank_')]
    ambient_points += [field.lower+np.array([8, 8, 8]), field.lower+np.array([504, 504, 504])]
    for index, point in enumerate(ambient_points):
        cell = np.clip(np.floor((point-field.lower)/16).astype(int), 0, 31)
        node_point = field.lower+(cell+.5)*16
        stored = ambient[cell[2], cell[1], cell[0], :3]*5000
        required = ambient_active_local is None or (np.all(node_point-field.lower >= np.array(ambient_active_local[:3]))
                                                    and np.all(node_point-field.lower <= np.array(ambient_active_local[3:])))
        if not required:
            ambient_results.append({'id': f'ambient_node_{index}', 'point': node_point.tolist(),
                                    'required': False, 'gpu_radiance': stored.tolist(),
                                    'status': 'intentionally inactive outside declared cloud-query domain; no physical-error comparison'})
            continue
        reference = ambient_reference(field, node_point, optical_scene, step=1)
        refined = ambient_reference(field, node_point, optical_scene, step=.5)
        ambient_results.append({'id': f'ambient_node_{index}', 'point': node_point.tolist(), 'required': True,
                                'gpu_radiance': stored.tolist(), 'reference_1m': reference.tolist(),
                                'reference_half_m': refined.tolist(),
                                'quadrature_delta': abs(reference-refined).tolist(),
                                'gpu_error': (stored-refined).tolist(),
                                'relative_error': ((stored-refined)/np.maximum(refined, 1)).tolist()})
    def maximum(records, key):
        values = [record[key] for record in records if key in record]
        return float(np.max(np.abs(values))) if values else 0.
    parity = {}
    if baseline is not None:
        for product, side in [('density', 128), ('mip1', 64), ('mip2', 32), ('prefix', prefix_side)]:
            current = rgba(Path(str(probe)+f'-{product}.f16'), side)
            previous = rgba(Path(str(baseline)+f'-{product}.f16'), side)
            parity[product] = {'mismatched_half_values': int(np.count_nonzero(current != previous)),
                               'max_absolute_error': float(np.max(abs(current-previous)))}
        previous = rgba(Path(str(baseline)+'-ambient.f16'), 32)
        z, y, x = np.mgrid[0:32, 0:32, 0:32]
        local = np.stack([x, y, z], axis=-1)*16+8
        active = np.ones((32, 32, 32), bool) if ambient_active_local is None else np.all(
            (local >= ambient_active_local[:3]) & (local <= ambient_active_local[3:]), axis=-1)
        parity['ambient'] = {'retained_nodes': int(active.sum()), 'inactive_nodes': int((~active).sum()),
                             'retained_mismatched_half_values': int(np.count_nonzero(ambient[active] != previous[active])),
                             'retained_max_absolute_error': float(np.max(abs(ambient[active]-previous[active]))),
                             'inactive_nonzero_half_values': int(np.count_nonzero(ambient[~active])),
                             'baseline_path': str(baseline),
                             'baseline_ambient_sha256': digest(Path(str(baseline)+'-ambient.f16'))}
    return {'scope': 'Actual GPU caches versus independent float64 optics; no visual/device performance claim',
            'source_identity': {'expected_density_sha256': captured['identity']['source_density_sha256'],
                                'actual_selected_density_sha256': digest(Path(clip)/'density.bin')},
            'captured_identity': captured['identity'], 'scene': scene,
            'inputs': {path.name: digest(path) for path in [manifest_path, density_path, prefix_path, ambient_path]},
            'density': {'finite': bool(np.isfinite(gpu).all()), 'nonzero_voxels': int(np.count_nonzero(gpu)),
                        'decoded_half_mismatch_voxels': int(mismatch.sum()),
                        'decoded_half_max_error': float(np.max(abs(gpu-rounded))),
                        'source_to_half_max_sigma_error': float(np.max(abs(gpu-decoded)))},
            'mip_parity': mip_parity, 'prefix_side': prefix_side, 'baseline_parity': parity,
            'ambient_active_domain': {'local_bounds': ambient_active_local,
                                     'policy': 'Only nodes in declared expanded cloud-query support are physically required; '
                                               'outside zero is inactive, not an optical ambient approximation.'},
            'prefix_limits': 'Mip0 is actual GPU density; mip1/2 references recursively average2x2x2 and roundhalf. '
                             + ('Standalone mip readbacks checked.' if len(mip_parity) == 2 else
                                'GPU mip arrays were not exported, so standalone mip parity is unqualified.')+' '
                             'Physical node and sampled-query errors are reported separately; no acceptance threshold inferred.',
            'prefix_world_queries': queries, 'prefix_exact_nodes': nodes, 'ambient_nodes': ambient_results,
            'summary': {'max_world_tau_error': maximum(queries, 'gpu_tau_error'),
                        'max_world_T_error': maximum(queries, 'gpu_T_error'),
                        'max_node_tau_error': maximum(nodes, 'gpu_tau_error'),
                        'max_node_T_error': maximum(nodes, 'gpu_T_error'),
                        'max_reference_quadrature_delta': maximum(queries+nodes, 'quadrature_delta'),
                        'max_ambient_radiance_error': maximum(ambient_results, 'gpu_error'),
                        'max_ambient_relative_error': maximum(ambient_results, 'relative_error')}}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--probe', type=Path, required=True, help='Export stem, without.json')
    parser.add_argument('--clip', type=Path, default=Path('docs/evidence/cloud-source-2026-11-01/selected-clip'))
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--baseline', type=Path)
    parser.add_argument('--ambient-active-local', type=float, nargs=6, metavar=('X0', 'Y0', 'Z0', 'X1', 'Y1', 'Z1'))
    args = parser.parse_args()
    result = validate(args.probe, args.clip, args.ambient_active_local, args.baseline)
    args.out.mkdir(parents=True, exist_ok=True)
    (args.out/'cache-validation.json').write_text(json.dumps(result, indent=2, allow_nan=False)+'\n')
    summary = result['summary']
    lines = ['# Actual GPU cache validation', '',
             f"Source density half parity: {result['density']['decoded_half_mismatch_voxels']} mismatched voxels.", '',
             '| Quantity | Maximum absolute error |', '| --- | ---: |']
    lines += [f'| {key} | {value:.9g} |' for key, value in summary.items()]
    lines += ['', result['prefix_limits'], '',
              'Ambient comparison uses exact32³ cache-node positions against1m/0.5m cloud integration, '
              'analytic atmosphere/fog paths and the declared five-valid-axis/6 horizon quadrature. '
              'GPU quadrature and half storage are both included in error; use the captured shader identity for its exact recipe.', '',
              'These bounded samples establish neither full-domain maxima nor visual acceptance. '
              'The original captured identity and every input hash are retained in the JSON.']
    if args.ambient_active_local:
        lines += ['', 'Ambient physical-error checks apply only within the declared local support '
                  f'{args.ambient_active_local}; outside zeros are intentionally inactive.']
    if args.baseline:
        lines += ['', 'Baseline full-array and retained-ambient parity:', '',
                  '```json', json.dumps(result['baseline_parity'], indent=2), '```']
    (args.out/'cache-validation.md').write_text('\n'.join(lines)+'\n')
    print(json.dumps({'density': result['density'], 'summary': summary}, indent=2))


if __name__ == '__main__':
    main()
