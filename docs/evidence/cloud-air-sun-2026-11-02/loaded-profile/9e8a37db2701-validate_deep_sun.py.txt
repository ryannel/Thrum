"""Offline finite checks of actual deep-Sun GPU readbacks; no GPU or source cook.

Reads receipt.json and four SHA-pinned little-endian binaries. Compression is
checked over entire original PL input intervals, allowing different GPU anchors.
NumPy double field integration is independent of the candidate JS/WGSL. Reported
finite comparisons are not uniform quadrature/footprint/backend certificates.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import sys

import numpy as np

SCHEMA = 'thrum.deep-sun-offline.v1'
EXPONENTS = np.array([1., .25])
COMPRESSION_ALLOCATION = .005
FINITE_F32_TOLERANCE = 2e-6


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def decode_field(payload, dimensions, coefficient_max=.1):
    """RGBA holds consecutive logical Z; physical x fastest, guarded groups."""
    nx, ny, nz = map(int, dimensions)
    require(all(1 <= n <= 256 for n in (nx, ny, nz)), 'logical field dimensions')
    px, py, pz = nx+2, ny+2, (nz+3)//4+2
    require(len(payload) == px*py*pz*8, 'field byte count')
    packed = np.frombuffer(payload, dtype='<f2').reshape(pz, py, px, 4)
    require(np.isfinite(packed).all() and (packed >= 0).all() and (packed <= coefficient_max).all(), 'finite bounded field coefficients')
    require(not any(np.count_nonzero(a) for a in (packed[0], packed[-1], packed[:, 0], packed[:, -1], packed[:, :, 0], packed[:, :, -1])), 'nonzero field guard')
    if nz % 4:
        require(not np.count_nonzero(packed[-2, 1:-1, 1:-1, nz % 4:]), 'nonzero unused Z channels')
    return np.stack([packed[z//4+1, 1:ny+1, 1:nx+1, z % 4].astype(np.float64) for z in range(nz)])


def sample_field(field, world, lo, extent):
    """Double trilinear zero-extended logical samples, boundary coord centres i+.5."""
    points = np.asarray(world, dtype=np.float64)
    require(points.shape[-1] == 3 and np.isfinite(points).all(), 'finite field points')
    shape = points.shape[:-1]
    points = points.reshape(-1, 3)
    dims = np.array(field.shape[::-1])
    coordinate = (points-lo)/extent*dims-.5
    base = np.floor(coordinate).astype(np.int64)
    fraction = coordinate-base
    output = np.zeros(len(points))
    for z in (0, 1):
        for y in (0, 1):
            for x in (0, 1):
                offset = np.array([x, y, z])
                index = base+offset
                valid = ((index >= 0) & (index < dims)).all(axis=1)
                weights = np.where(offset, fraction, 1-fraction).prod(axis=1)
                selected = index[valid]
                output[valid] += weights[valid]*field[selected[:, 2], selected[:, 1], selected[:, 0]]
    return output.reshape(shape)


def scene_from_words(words):
    require(len(words) == 48 and all(type(n) is int and 0 <= n <= 0xffffffff for n in words), 'Scene requires exact48u32')
    u = np.array(words, dtype='<u4')
    f = u.view('<f4').astype(np.float64)
    lo, extent = f[:3], f[4:7]
    dims = u[40:43].astype(np.int64)
    require(np.isfinite(f[:40]).all() and (extent > 0).all() and (dims >= 1).all() and (dims <= 256).all(), 'Scene finite bounds/grid')
    basis = np.stack((f[20:23], f[24:27], f[28:31]))
    require(np.max(np.abs(basis@basis.T-np.eye(3))) < 1e-5 and np.linalg.norm(basis[2]+f[16:19]) < 1e-5, 'orthonormal light frame W=-Sun')
    raw = np.abs(basis)@(extent/2+extent/dims/2)
    half = raw.copy()
    half[:2] /= 1-4/64
    return dict(lo=lo, extent=extent, dims=dims, basis=basis, raw=raw, half=half,
                centre=lo+extent/2, sun=f[16:19], source_time=float(f[3]), bytes=u.tobytes())


def direct_tau(field, scene, origin, intervals):
    """Ordinary Sun-depth midpoint integration of the same readback field."""
    direction, lo, extent = scene['sun'], scene['lo'], scene['extent']
    halo = extent/scene['dims']/2
    lower, upper = lo-halo, lo+extent+halo
    near, far = -math.inf, math.inf
    for k in range(3):
        if abs(direction[k]) < 1e-8:
            if origin[k] < lower[k] or origin[k] > upper[k]:
                return 0.
        else:
            a, b = (lower[k]-origin[k])/direction[k], (upper[k]-origin[k])/direction[k]
            near, far = max(near, min(a, b)), min(far, max(a, b))
    start = max(0., near)
    end = max(start, far)
    if end <= start:
        return 0.
    step = (end-start)/intervals
    world = origin+(start+(np.arange(intervals)+.5)*step)[:, None]*direction
    return float(np.sum(sample_field(field, world, lo, extent))*step)


def curve_error(original, retained):
    """Analytic sup transformed error for arbitrary two PL ordinary-tau curves."""
    a, b = np.asarray(original, float), np.asarray(retained, float)
    for points in (a, b):
        require(points.ndim == 2 and points.shape[1] == 2 and len(points) >= 2 and np.isfinite(points).all(), 'curve shape/finite')
        require((np.diff(points[:, 0]) > 0).all() and (np.diff(points[:, 1]) >= 0).all() and (points[:, 1] >= 0).all(), 'ordered monotone curve')
    tolerance = FINITE_F32_TOLERANCE*max(1., abs(a[-1, 0]))
    require(abs(a[0, 0]-b[0, 0]) <= tolerance and abs(a[-1, 0]-b[-1, 0]) <= tolerance, 'curve endpoint domains differ')
    # Cover the full original domain, including tiny rounded endpoint offsets.
    # Constant endpoint extension matches the stored-curve query contract.
    start, end = a[0, 0], a[-1, 0]
    breaks = np.unique(np.r_[a[:, 0], b[:, 0], start, end])
    breaks = breaks[(breaks >= start) & (breaks <= end)]
    ta, tb = np.interp(breaks, a[:, 0], a[:, 1]), np.interp(breaks, b[:, 0], b[:, 1])
    errors, witnesses = [], []
    for k in EXPONENTS:
        difference = np.exp(-k*np.minimum(ta, tb)) * -np.expm1(-k*np.abs(ta-tb))
        index = int(np.argmax(difference))
        maximum, witness = float(difference[index]), float(breaks[index])
        for i in range(len(breaks)-1):
            d, e = ta[i+1]-ta[i], tb[i+1]-tb[i]
            if d == e or d == 0 or e == 0:
                continue
            delta = d-e
            ratio = math.log1p(delta/e) if abs(delta) < .5*e else math.log(d)-math.log(e)
            u = (ratio/k-(ta[i]-tb[i]))/delta
            if 0 < u < 1:
                aa, bb = ta[i]+d*u, tb[i]+e*u
                error = math.exp(-k*min(aa, bb))*-math.expm1(-k*abs(aa-bb))
                if error > maximum:
                    maximum = error
                    witness = float(breaks[i]+u*(breaks[i+1]-breaks[i]))
        errors.append(maximum)
        witnesses.append(witness)
    return errors, witnesses


def weighted_terms(taus, weights):
    return np.sum(np.exp(-np.asarray(taus)[:, None]*EXPONENTS)*np.asarray(weights)[:, None], axis=0)


def lookup_terms(nodes, metadata, scene, world):
    """Manual four-curve exp/weight contract; None means actual direct fallback."""
    projection = scene['basis']@(np.asarray(world)-scene['centre'])
    guard = 1e-5*np.maximum(scene['raw'], 1.)
    if projection[2] < -scene['raw'][2]-guard[2] or (np.abs(projection[:2]) > scene['half'][:2]+guard[:2]).any():
        return np.ones(2), [], 'conservative_clear'
    g = 64*(projection[:2]/(2*scene['half'][:2])+.5)+.5
    base = np.floor(g).astype(int)
    fraction = g-base
    if (base < 0).any() or (base+1 >= 66).any():
        return None, [], 'unsupported_footprint_direct'
    taps, taus, weights = [], [], []
    for y in (0, 1):
        for x in (0, 1):
            i, j = base+[x, y]
            record = metadata[j, i]
            taps.append([int(i), int(j)])
            if record[1] != 1:
                return None, taps, 'invalid_column_direct'
            count = int(record[0])
            curve = nodes[j, i, :count]
            depth = float(projection[2]+scene['raw'][2])
            taus.append(float(np.interp(depth, curve[:, 0], curve[:, 1])))
            weights.append((fraction[0] if x else 1-fraction[0])*(fraction[1] if y else 1-fraction[1]))
    return weighted_terms(taus, weights), taps, 'four_curve_terms'


def column_terms(field, scene, taps, depth, intervals, sampled_prefix=False):
    """Independent uncompressed column terms before four-tap weighting.

    sampled_prefix=True interpolates cumulative fixed-full-span midpoint samples,
    matching the intended input PL definition without the compressed nodes.
    Otherwise integrate to the actual depth with fine midpoint quadrature.
    """
    span = 2*scene['raw'][2]
    depth = min(span, max(0., depth))
    output = []
    for x, y in taps:
        start = (scene['centre']+scene['basis'][0]*((x-.5)/64-.5)*2*scene['half'][0]
                 +scene['basis'][1]*((y-.5)/64-.5)*2*scene['half'][1]-scene['basis'][2]*scene['raw'][2])
        h = (span if sampled_prefix else depth)/intervals
        world = start+((np.arange(intervals)+.5)*h)[:, None]*scene['basis'][2]
        sigma = sample_field(field, world, scene['lo'], scene['extent'])
        if sampled_prefix:
            prefix = np.r_[0., np.cumsum(sigma*h)]
            tau = float(np.interp(depth, np.arange(intervals+1)*h, prefix))
        else:
            tau = float(np.sum(sigma)*h)
        output.append(tau)
    return output


def query_weights(scene, world):
    projection = scene['basis']@(world-scene['centre'])
    g = 64*(projection[:2]/(2*scene['half'][:2])+.5)+.5
    f = g-np.floor(g)
    return [(f[0] if x else 1-f[0])*(f[1] if y else 1-f[1])
            for y in (0, 1) for x in (0, 1)]


def _binary(root, receipt, name):
    entry = receipt['binaries'][name]
    path = (root/entry['filename']).resolve()
    require(path.is_relative_to(root), 'binary filename outside capture directory')
    data = path.read_bytes()
    require(len(data) == entry['bytes'] and sha(data) == entry['sha256'], f'{name} bytes/SHA mismatch')
    return data


def validate_capture(directory, receipt_name='receipt.json', fine_intervals=4096):
    root = Path(directory).resolve()
    receipt_bytes = (root/receipt_name).read_bytes()
    receipt = json.loads(receipt_bytes)
    require(receipt['schema'] == 'thrum.deep-sun-probe.v1', 'capture schema')
    scene = scene_from_words(receipt['uniform_u32'])
    identity = receipt.get('identity', {})
    if 'uniform_sha256' in identity:
        require(identity['uniform_sha256'] == sha(scene['bytes']), 'Scene uniform SHA mismatch')
    require(list(scene['dims']) == receipt['logical_dimensions'], 'Scene/logical grid mismatch')
    payloads = {name: _binary(root, receipt, name) for name in ('nodes', 'metadata', 'field', 'paths')}
    require(len(payloads['nodes']) == 66*66*48*2*4 and len(payloads['metadata']) == 66*66*8*4, 'deep binary sizes')
    nodes = np.frombuffer(payloads['nodes'], dtype='<f4').reshape(66, 66, 48, 2)
    metadata = np.frombuffer(payloads['metadata'], dtype='<f4').reshape(66, 66, 8)
    require(np.isfinite(metadata).all() and (metadata >= 0).all(), 'nonfinite/negative metadata')
    for word in (0, 1, 4, 5, 6, 7):
        require((metadata[:, :, word] == np.floor(metadata[:, :, word])).all(), 'noninteger metadata count/flag')
    require((metadata[:, :, 0] <= 48).all() and (metadata[:, :, 1] <= 1).all(), 'column count/valid cap')
    require((metadata[:, :, 4] <= 512).all() and (metadata[:, :, 5] <= 2*metadata[:, :, 4]).all() and (metadata[:, :, 6] <= 48).all(), 'column work cap')
    admitted = metadata[:, :, 1] == 1
    require((metadata[admitted, 0] >= 2).all() and (metadata[admitted, 7] == 0).all(), 'admitted flag/count inconsistent')
    for y in range(66):
        for x in range(66):
            count = int(metadata[y, x, 0])
            active = nodes[y, x, :count]
            require(np.isfinite(active).all() and (active[:, 1] >= 0).all(), 'nonfinite/negative active knots')
            require((np.diff(active[:, 0]) > 0).all() and (np.diff(active[:, 1]) >= 0).all(), 'unordered active knots')
            if admitted[y, x]:
                require(active[0, 0] == 0 and active[0, 1] == 0 and abs(float(active[-1, 0])-2*scene['raw'][2]) <= FINITE_F32_TOLERANCE*max(1.,2*scene['raw'][2]), 'admitted curve endpoints')
    physical = [int(scene['dims'][0]+2), int(scene['dims'][1]+2), int((scene['dims'][2]+3)//4+2)]
    require(physical == receipt['field_physical_dimensions'], 'packed physical grid mismatch')
    # Explicit commissioned retained-Q1 calibration exception; never clamp or
    # present this .125 field as the production .1 optical admission.
    fixture = receipt.get('frame', {})
    q1_calibration = (fixture.get('source_model') == 'prepared' and fixture.get('source_stats', {}).get('profile') == 'retained Q1 static geometry comparator')
    coefficient_max = .125 if q1_calibration else .1
    field = decode_field(payloads['field'], scene['dims'], coefficient_max)
    selected = receipt['selected_columns']
    require(0 < len(selected) <= 16 and len(payloads['paths']) == len(selected)*513*16, 'selected path count/size')
    paths = np.frombuffer(payloads['paths'], dtype='<f4').reshape(len(selected), 513, 4).astype(float)
    require(np.isfinite(paths).all() and (paths[:, :, 3] == 1).all(), 'invalid GPU path sample')
    columns = []
    for selection, path in zip(selected, paths):
        x, y = selection['physical_xy']
        require(type(x) is int and type(y) is int and 0 <= x < 66 and 0 <= y < 66 and selection['linear_index'] == y*66+x, 'selected column identity')
        require(np.array_equal(path[0], [0, 0, 0, 1]) and ((path[:, 2] >= 0) & (path[:, 2] <= coefficient_max)).all(), 'path endpoint0/sigma')
        require((np.diff(path[:, 0]) > 0).all() and (np.diff(path[:, 1]) >= 0).all() and (path[:, 1] >= 0).all(), 'ordered nonnegative raw path')
        count = int(metadata[y, x, 0])
        if 'count' in selection:
            require(selection['count'] == count and selection['compression_valid'] == bool(admitted[y, x]), 'selected receipt metadata mismatch')
        curve = nodes[y, x, :count].astype(float)
        errors, witness = curve_error(path[:, :2], curve) if admitted[y, x] else (None, None)
        h = path[1, 0]
        depth_expected = np.arange(1, 513)*h
        start = (scene['centre']+scene['basis'][0]*((x-.5)/64-.5)*2*scene['half'][0]
                 +scene['basis'][1]*((y-.5)/64-.5)*2*scene['half'][1]-scene['basis'][2]*scene['raw'][2])
        world = start+((np.arange(512)+.5)*h)[:, None]*scene['basis'][2]
        sigma_cpu = sample_field(field, world, scene['lo'], scene['extent'])
        tau_cpu = np.r_[0., np.cumsum(sigma_cpu*h)]
        tau_sigma_recorded = np.r_[0., np.cumsum(path[1:, 2]*h)]
        columns.append(dict(physical_xy=[x, y], valid=bool(admitted[y, x]), flags=int(metadata[y, x, 7]), knots=count,
                            compression_error=errors, compression_witness_depth_m=witness,
                            compression_within_finite_allowance=None if errors is None else max(errors) <= .005+FINITE_F32_TOLERANCE,
                            nominal_metadata_bounds=metadata[y, x, 2:4].tolist(),
                            max_depth_arithmetic_error_m=float(np.max(np.abs(path[1:, 0]-depth_expected))),
                            span_contract_error_m=abs(float(path[-1, 0])-2*scene['raw'][2]),
                            max_sigma_gpu_vs_cpu_per_m=float(np.max(np.abs(path[1:, 2]-sigma_cpu))),
                            max_tau_stream_vs_cpu=float(np.max(np.abs(path[:, 1]-tau_cpu))),
                            max_tau_accumulation_vs_recorded_sigma=float(np.max(np.abs(path[:, 1]-tau_sigma_recorded))),
                            max_T_stream_vs_cpu=np.max(np.abs(np.exp(-path[:, 1, None]*EXPONENTS)-np.exp(-tau_cpu[:, None]*EXPONENTS)), axis=0).tolist()))
    queries = []
    rows = receipt['query']['rows']
    require(len(rows) <= 128, 'point query cap')
    for row in rows:
        world = np.array(row['world_m'], float)
        require(world.shape == (3,) and np.isfinite(world).all(), 'query world point')
        tau512 = direct_tau(field, scene, world, 512)
        tau_fine = direct_tau(field, scene, world, fine_intervals)
        manual, taps, mode = lookup_terms(nodes, metadata, scene, world)
        if manual is None:
            manual = np.exp(-EXPONENTS*tau512)
        gpu = np.array(row['terms'], float)
        require(gpu.shape == (2,) and np.isfinite(gpu).all(), 'nonfinite GPU point query')
        if not row['valid']:
            queries.append(dict(world_m=world.tolist(), kind=row.get('kind'), available=False, gpu_terms=gpu.tolist(),
                                CPU_direct_tau512=tau512, CPU_direct_tau_fine=tau_fine))
            continue
        require(((gpu >= 0) & (gpu <= 1)).all(), 'admitted GPU terms outside0..1')
        gpu_tau = float(row['direct_tau_512'])
        require(math.isfinite(gpu_tau) and gpu_tau >= 0, 'invalid GPU direct tau')
        direct_terms = np.exp(-EXPONENTS*tau_fine)
        separation = dict(lateral_terms_error=None, column_quadrature_terms_error=None,
                          compressed_vs_CPU_input_terms_error=None)
        if mode == 'four_curve_terms':
            weights = query_weights(scene, world)
            depth = float(scene['basis'][2]@(world-scene['centre'])+scene['raw'][2])
            coarse = weighted_terms(column_terms(field, scene, taps, depth, 512, True), weights)
            fine = weighted_terms(column_terms(field, scene, taps, depth, fine_intervals), weights)
            separation = dict(lateral_terms_error=np.abs(fine-direct_terms).tolist(),
                              column_quadrature_terms_error=np.abs(coarse-fine).tolist(),
                              compressed_vs_CPU_input_terms_error=np.abs(manual-coarse).tolist())
        queries.append(dict(**separation, world_m=world.tolist(), kind=row.get('kind'), available=True, mode=mode, taps=taps,
                            gpu_terms=gpu.tolist(), manual_terms=manual.tolist(),
                            GPU_vs_manual_terms_error=np.abs(gpu-manual).tolist(),
                            GPU_direct_tau512_vs_CPU_error=abs(gpu_tau-tau512),
                            CPU_direct_tau512=tau512, CPU_direct_tau_fine=tau_fine,
                            source_quadrature_terms_error=np.abs(np.exp(-EXPONENTS*tau512)-direct_terms).tolist(),
                            lateral_plus_compression_vs_fine_point_error=np.abs(manual-direct_terms).tolist(),
                            GPU_terms_vs_fine_point_error=np.abs(gpu-direct_terms).tolist()))
    def max_pairs(rows, key):
        values = [r[key] for r in rows if r.get(key) is not None]
        return np.max(values, axis=0).tolist() if values else None
    checked_columns = [c for c in columns if c['compression_error'] is not None]
    summary = dict(selected_compression_checked=len(checked_columns),
                   selected_compression_finite_pass=(all(c['compression_within_finite_allowance'] for c in checked_columns) if checked_columns else None),
                   selected_compression_error_max=max_pairs(columns, 'compression_error'),
                   GPU_manual_terms_error_max=max_pairs(queries, 'GPU_vs_manual_terms_error'),
                   source_quadrature_terms_error_max=max_pairs(queries, 'source_quadrature_terms_error'),
                   lateral_terms_error_max=max_pairs(queries, 'lateral_terms_error'),
                   column_quadrature_terms_error_max=max_pairs(queries, 'column_quadrature_terms_error'),
                   compressed_vs_CPU_input_terms_error_max=max_pairs(queries, 'compressed_vs_CPU_input_terms_error'),
                   GPU_vs_fine_point_terms_error_max=max_pairs(queries, 'GPU_terms_vs_fine_point_error'),
                   unavailable_point_queries=sum(not q['available'] for q in queries))
    return dict(schema=SCHEMA, capture_receipt_sha256=sha(receipt_bytes), validator_sha256=sha(Path(__file__).read_bytes()),
                identity=receipt.get('identity'), binary_pins=receipt['binaries'],
                scene_u32_sha256=sha(scene['bytes']), source_time_s=scene['source_time'],
                finite_check_policy=dict(compression_allocation=.005, comparison_f32_tolerance=FINITE_F32_TOLERANCE,
                                         source_quadrature_allocation=.005, lateral_allocation=.008, storage_allocation=.002,
                                         fine_midpoint_intervals=fine_intervals, uniform_backend_certificate=False),
                capture_structurally_valid=True, whole_profile_qualified=False,
                field_admission=dict(calibration_exception=q1_calibration, coefficient_limit_per_m=coefficient_max,
                                     actual_max_per_m=float(field.max()), production_sigma_0_1_admitted=bool(field.max() <= .1)),
                column_counts=dict(total=4356, admitted=int(admitted.sum()), unavailable=int((~admitted).sum()),
                                   maximum_retained=int(metadata[:, :, 0].max()),
                                   coefficient_requests=int(metadata[:, :, 4].sum()),
                                   endpoint_visits=int(metadata[:, :, 5].sum()), anchor_builds=int(metadata[:, :, 6].sum())),
                summary=summary, selected_columns=columns, query_rows=queries, probe_counters=receipt.get('query', {}).get('counters'),
                scope='finite actual-buffer/PL compression/packed-field/query checks; no uniform source-quadrature, lateral, storage, radiance or whole-profile pass')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('capture_directory', type=Path)
    parser.add_argument('--receipt', default='receipt.json')
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    try:
        result = validate_capture(args.capture_directory, args.receipt)
        status = 0
    except (ValueError, KeyError, OSError, TypeError) as error:
        result = dict(schema=SCHEMA, capture_structurally_valid=False, error=str(error), whole_profile_qualified=False)
        status = 2
    encoded = json.dumps(result, separators=(',', ':'), allow_nan=False)+'\n'
    if args.output:
        args.output.write_text(encoded)
    else:
        print(encoded, end='')
    return status


if __name__ == '__main__':
    sys.exit(main())
