"""Offline direct-density alternative from genuine fine source exports.

No carrier or history: retain all actual source times, linearly interpolate two
R8 frames. Sub-frame temporal accuracy remains unqualified without finer exports.
"""
from __future__ import annotations

import argparse
from collections import OrderedDict
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import time

import numpy as np

_HELPER = Path(__file__).with_name('cook.py')
_spec = importlib.util.spec_from_file_location('cloud_proof_codec', _HELPER)
_codec = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_codec)
SCHEMA = 'thrum.cloud-source-proof.direct.v1'


def load_fine(path):
    with np.load(path, allow_pickle=False) as data:
        density = np.asarray(data['density'], np.float32)
        source_time = float(data['source_time_s'])
    n = density.shape[0]
    if density.shape != (n, n, n) or n % 8 or not np.isfinite(density).all():
        raise ValueError(f'{path}: expected finite cubic extinction field, side divisible by8')
    if np.any(density < 0) or not np.isfinite(source_time):
        raise ValueError(f'{path}: negative extinction or nonfinite time')
    return density, source_time


def frame_payload(density, code_max):
    if float(density.max()) > code_max:
        raise ValueError(f'source sigma max {float(density.max())} exceeds fixed code max {code_max}; no clipping')
    code = np.rint(density / code_max * 255).astype(np.uint8)
    payload, record = _codec.pack_density(code)
    return payload, record, hashlib.sha256(code.tobytes()).hexdigest()


class Frames:
    """Indexable float32 sigma[T,Z,Y,X], with at most two decoded frames resident."""
    def __init__(self, manifest, payload):
        self.manifest = manifest
        self.payload = payload
        side = manifest['density_side']
        self.shape = (len(manifest['density_frames']), side, side, side)
        self.dtype = np.dtype(np.float32)
        self.ndim = 4
        self.cache = OrderedDict()

    def __len__(self):
        return self.shape[0]

    def frame(self, index):
        index = int(index)
        if index < 0:
            index += len(self)
        if not 0 <= index < len(self):
            raise IndexError(index)
        record = self.manifest['density_frames'][index]
        key = record['chunk_id']
        if key not in self.cache:
            blob = self.payload[record['offset']:record['offset']+record['bytes']]
            if hashlib.sha256(blob).hexdigest() != record['sha256']:
                raise ValueError('density chunk hash mismatch')
            code = _codec.unpack_density(blob, self.shape[1], record)
            sigma = code.astype(np.float32)*np.float32(self.manifest['runtime_sigma_max_per_m']/255)
            sigma.setflags(write=False)
            self.cache[key] = sigma
            while len(self.cache) > 2:
                self.cache.popitem(last=False)
        self.cache.move_to_end(key)
        return self.cache[key]

    def __getitem__(self, key):
        if isinstance(key, tuple):
            index, rest = key[0], key[1:]
            selected = self[index]
            return selected[(slice(None),)+rest] if isinstance(index, slice) else selected[rest]
        if isinstance(key, slice):
            indices = range(*key.indices(len(self)))
            return (np.stack([self.frame(i) for i in indices]) if len(indices) else
                    np.empty((0,)+self.shape[1:], np.float32))
        return self.frame(key)


def read_product(path):
    path = Path(path)
    manifest = json.loads((path/'cloudclip.json').read_text())
    if manifest['schema'] != SCHEMA or manifest['reader_kind'] != 'direct':
        raise ValueError('unsupported direct-density product')
    payload = (path/'density.bin').read_bytes()
    record = manifest['products']['density.bin']
    if len(payload) != record['bytes'] or hashlib.sha256(payload).hexdigest() != record['sha256']:
        raise ValueError('density product length/hash mismatch')
    frames = Frames(manifest, payload)
    return {'density': frames, 'frames': frames, 'read_frame': frames.frame,
            'times': np.asarray(manifest['source_times_s'], np.float64),
            'flow': np.empty((0, 0, 0, 0, 3), np.float32), 'manifest': manifest}


def time_location(times, source_time):
    if not np.isfinite(source_time):
        raise ValueError('nonfinite source time')
    source_time = float(np.clip(source_time, times[0], times[-1]))
    index = int(np.clip(np.searchsorted(times, source_time, side='right')-1, 0, len(times)-2))
    fraction = (source_time-times[index])/(times[index+1]-times[index])
    return index, float(fraction)


def sample_time(product, source_time):
    index, fraction = time_location(product['times'], source_time)
    a = product['frames'][index]
    if fraction == 0:
        return a
    b = product['frames'][index+1]
    if fraction == 1:
        return b
    return (1-fraction)*a+fraction*b


def maximum_errors(records):
    names = ('max_axial_tau_error', 'max_axial_transmittance_abs_error',
             'mean_abs_extinction_error_per_m', 'max_abs_extinction_error_per_m')
    return {name: max(r[name] for r in records) for name in names}


def cadence_errors(paths, product, stride, extent):
    result = []
    times = product['times']
    for first in range(0, len(paths)-stride, stride):
        last = first+stride
        a, _ = load_fine(paths[first])
        b, _ = load_fine(paths[last])
        qa, qb = product['frames'][first], product['frames'][last]
        for index in range(first+1, last):
            reference, source_time = load_fine(paths[index])
            fraction = (source_time-times[first])/(times[last]-times[first])
            native = (1-fraction)*a+fraction*b
            quantized = (1-fraction)*qa+fraction*qb
            result.append({'source_frame': index+1, 'source_time_s': source_time,
                'native_endpoint_lerp': _codec.optical_error(native, reference, extent/a.shape[0]),
                'r8_endpoint_lerp': _codec.optical_error(quantized, reference, extent/a.shape[0])})
    return result


def build(source, out, code_max=.125, extent=512, source_manifest=None):
    if out.exists():
        raise ValueError('output must be a new directory')
    if not np.isfinite(code_max) or code_max <= 0 or not np.isfinite(extent) or extent <= 0:
        raise ValueError('finite positive fixed optical/spatial scales required')
    paths = sorted(source.glob('fine-*.npz'))
    if [p.name for p in paths] != [f'fine-{i:04d}.npz' for i in range(1, 130)]:
        raise ValueError('expected129 actual fine-0001..fine-0129 exports; no invented frames')
    begin = time.perf_counter()
    payload = bytearray()
    chunks = {}
    frame_records, source_records, times, quantization = [], [], [], []
    side = None
    max_positive_column = 0
    collar_max = 0.
    max_sigma = 0.
    endpoint_max = []
    for index, path in enumerate(paths):
        density, source_time = load_fine(path)
        if side is None:
            side = density.shape[0]
        if density.shape != (side, side, side):
            raise ValueError('source grid changes across frames')
        if abs(source_time-index*1.875) > 1e-9:
            raise ValueError('actual source times disagree with declared1.875s exports')
        blob, record, code_hash = frame_payload(density, code_max)
        if code_hash not in chunks:
            record.update(offset=len(payload), sha256=hashlib.sha256(blob).hexdigest(),
                          quantized_frame_sha256=code_hash, chunk_id=len(chunks))
            chunks[code_hash] = record
            payload.extend(blob)
        else:
            existing = chunks[code_hash]
            if bytes(payload[existing['offset']:existing['offset']+existing['bytes']]) != blob:
                raise ValueError('encoded-frame hash collision; exact deduplication refused')
        frame_records.append(dict(chunks[code_hash], source_frame=index+1))
        source_records.append({'frame': index+1, 'sha256': _codec.sha256(path)})
        times.append(source_time)
        q = _codec.unpack_density(blob, side, record).astype(np.float32)*np.float32(code_max/255)
        quantization.append({'source_frame': index+1,
                             **_codec.optical_error(q, density, extent/side)})
        max_sigma = max(max_sigma, float(density.max()))
        for axis in range(3):
            max_positive_column = max(max_positive_column, int(np.count_nonzero(density, axis=axis).max()))
        k = min(8, side//4)
        collar_max = max(collar_max, *[float(a.max()) for a in
            (density[:k], density[-k:], density[:, :k], density[:, -k:], density[:, :, :k], density[:, :, -k:])])
        if index in (0, len(paths)-1):
            endpoint_max.append(float(density.max()))
        if index % 32 == 0:
            print(f'DIRECT_ENCODE {index+1}/{len(paths)}', flush=True)
    out.mkdir(parents=True)
    (out/'density.bin').write_bytes(payload)
    rounding_allowance = 4*np.finfo(np.float32).eps*code_max
    node_error_bound = code_max/510+rounding_allowance
    manifest = {'schema': SCHEMA, 'reader_kind': 'direct',
        'decision_status': 'bounded temporal/residency alternative; source acceptance and engine integration unqualified',
        'compiler_sha256': _codec.sha256(__file__), 'codec_helper_sha256': _codec.sha256(_HELPER),
        'producer': 'actual fine exported authored optical master; not reconstructed coarse-reference samples',
        'density_side': side, 'flow_side': 0, 'flow_intervals': [],
        'extent_m': extent, 'bounds_min_xyz_m': [-extent/2, 0, -extent/2],
        'axes': 'right-handed Y-up; scalar storageXfastest thenYthenZ',
        'voxel_centres': 'bounds_min+(ijk+.5)*extent/side; fixed across allframes',
        'runtime_sigma_max_per_m': code_max, 'source_max_sigma_per_m': max_sigma,
        'density_encoding': 'u16LEbrickindex empty65535;littlebitorderoccupancy;512R8voxels/activebrick',
        'density_frames': frame_records, 'unique_density_chunks': len(chunks),
        'deduplication': 'exact R8 codeframe equality, including exactempty; every genuine timestamp retained',
        'source_times_s': times, 'source_frame_step_s': 1.875, 'source_frames': source_records,
        'runtime_evaluation': 'direct scalar endpoint lerp; no flow, no history, no procedural density modification',
        'time_policy': 'finite clip, endpoint hold outside range; both endpoints must beexactzero; no loop',
        'source_temporal_qualification': '1.875s exports are real; no0.46875s/other subexport reference exists here, so1.875s temporal adequacy is unqualified',
        'resident_cpu_reader': 'encodedclip plus2decodedfloat32frames; eager129floatframes(~1GiB at128^3) notallocated',
        'products': {'density.bin': {'bytes': len(payload), 'sha256': _codec.sha256(out/'density.bin')}},
        'metadata_reserved_bytes': 131072,
        'endpoint_max_sigma_per_m': endpoint_max, 'empty_boundary_collar_max_sigma_per_m': collar_max,
        'quantization_bounds': {'exact_real_arithmetic_half_step_per_m': code_max/510,
            'float32_rounding_allowance_per_m': float(rounding_allowance),
            'node_extinction_abs_error_per_m': float(node_error_bound),
            'maximum_native_positive_axial_column_voxels': max_positive_column,
            'native_axial_tau_error_bound': float(max_positive_column*extent/side*node_error_bound),
            'arbitrary_direction_box_chord_tau_bound': float(np.sqrt(3)*extent*node_error_bound),
            'scope': 'node half-step plus conservative4eps32*codeMax float32 allowance; trilinear interpolation preserves nodalerror bound; axial support-count bound is for native columns, not arbitrary interpolated rays'},
        'quantization_axial_summary': maximum_errors(quantization),
        'rejection_reasons': []}
    if source_manifest is not None:
        manifest['recoverable_master_reference'] = {'file': str(source_manifest),
                                                   'sha256': _codec.sha256(source_manifest)}
    if endpoint_max != [0., 0.]:
        manifest['rejection_reasons'].append('source endpoints not both exactzero')
    if collar_max != 0:
        manifest['rejection_reasons'].append('source eight-voxel boundarycollar notexactzero')
    if manifest['quantization_axial_summary']['max_axial_tau_error'] > .1:
        manifest['rejection_reasons'].append('R8 native axial tauerror exceeds0.1')
    # Initial manifest permits the bounded reader to supply encoded endpoints.
    _codec.write_manifest(out/'cloudclip.json', manifest)
    product = read_product(out)
    coarse4 = cadence_errors(paths, product, 4, extent)
    coarse2 = cadence_errors(paths, product, 2, extent)
    summaries = {}
    for stride, records in ((4, coarse4), (2, coarse2)):
        summaries[str(stride)] = {kind: maximum_errors([r[kind] for r in records])
                    for kind in ('native_endpoint_lerp', 'r8_endpoint_lerp')}
    diagnostics = {'source_temporal_reference_scope': manifest['source_temporal_qualification'],
        'endpoint_R8_vs_actual_fine': quantization,
        'stride4_7_5s_endpoints_vs_actual_quarters': coarse4,
        'stride2_3_75s_endpoints_vs_actual_midpoints': coarse2,
        'cadence_summary': summaries,
        'interpretation': 'coarser-cadence failure/pass measures those coarser recipes only; doesnot qualify1.875s subframes',
        'wall_s': time.perf_counter()-begin}
    (out/'diagnostics.json').write_text(json.dumps(diagnostics, indent=2)+'\n')
    manifest['coarser_unwarped_cadence_summaries'] = summaries
    manifest['evidence_products'] = {'diagnostics.json': {'bytes': (out/'diagnostics.json').stat().st_size,
                                               'sha256': _codec.sha256(out/'diagnostics.json')}}
    manifest['qualification'] = 'rejected' if manifest['rejection_reasons'] else 'quantization/finite-boundary checks passed; temporal/source visuals/runtime unqualified'
    _codec.write_manifest(out/'cloudclip.json', manifest)
    if manifest['metadata_actual_bytes'] > manifest['metadata_reserved_bytes']:
        manifest['rejection_reasons'].append('actual runtime metadata exceeds128KiB reservation')
        manifest['qualification'] = 'rejected'
        _codec.write_manifest(out/'cloudclip.json', manifest)
    if not manifest['actual_source_cap_11MiB']:
        manifest['rejection_reasons'].append('actual direct encoded source exceeds11MiB cap')
        manifest['qualification'] = 'rejected'
        _codec.write_manifest(out/'cloudclip.json', manifest)
    # Decode each true frame independently and compare against its original field.
    product = read_product(out)
    for index, path in enumerate(paths):
        native, _ = load_fine(path)
        expected = np.rint(native/code_max*255).astype(np.uint8).astype(np.float32)*np.float32(code_max/255)
        if not np.array_equal(product['frames'][index], expected):
            raise ValueError(f'direct reader roundtrip failed atframe{index+1}')
    return manifest


def self_test():
    a = np.zeros((8, 8, 8), np.float32)
    a[2, 3, 4] = .1
    blob, record, digest = frame_payload(a, .125)
    record.update(offset=0, sha256=hashlib.sha256(blob).hexdigest(), chunk_id=0)
    empty, er, _ = frame_payload(np.zeros_like(a), .125)
    er.update(offset=len(blob), sha256=hashlib.sha256(empty).hexdigest(), chunk_id=1)
    manifest = {'schema': SCHEMA, 'reader_kind': 'direct', 'density_side': 8,
        'runtime_sigma_max_per_m': .125, 'source_times_s': [0, 1.875, 3.75],
        'density_frames': [record, er, record],
        'products': {'density.bin': {'bytes': len(blob)+len(empty),
                                    'sha256': hashlib.sha256(blob+empty).hexdigest()}}}
    with tempfile.TemporaryDirectory() as directory:
        p = Path(directory)
        (p/'density.bin').write_bytes(blob+empty)
        _codec.write_manifest(p/'cloudclip.json', manifest)
        product = read_product(p)
        assert np.isclose(product['frames'][0, 2, 3, 4], .1, atol=1e-8)
        assert np.array_equal(product['frames'][0], product['frames'][2])
        assert not np.any(product['frames'][1])
        np.testing.assert_allclose(product['frames'][:, 2, 3, 4], [.1, 0, .1], atol=1e-8)
        assert product['frames'][0:0].shape == (0, 8, 8, 8)
        assert np.isclose(sample_time(product, .9375)[2, 3, 4], .05)
        assert time_location(product['times'], 3.75) == (1, 1.)
        assert time_location(product['times'], -1) == (0, 0.)
        assert len(product['frames'].cache) <= 2
        assert (p/'cloudclip.json').stat().st_size == manifest['metadata_actual_bytes']
        (p/'density.bin').write_bytes(bytes(len(blob)+len(empty)))
        try:
            read_product(p)
        except ValueError:
            pass
        else:
            raise AssertionError('corrupt direct payload accepted')
    try:
        frame_payload(np.full_like(a, .126), .125)
    except ValueError:
        pass
    else:
        raise AssertionError('outofrange source silently clipped')
    print('DIRECT_SELF_TEST_PASS: dedup/empty, axes, fractions, boundedreader, corruption and scale rejection')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path)
    parser.add_argument('--out', type=Path)
    parser.add_argument('--code-max', type=float, default=.125)
    parser.add_argument('--extent', type=float, default=512)
    parser.add_argument('--source-manifest', type=Path)
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    if args.self_test:
        self_test()
        return
    if args.source is None or args.out is None:
        parser.error('--source and --out are required')
    manifest = build(args.source, args.out, args.code_max, args.extent, args.source_manifest)
    print('DIRECT_COMPLETE', json.dumps({'out': str(args.out),
        'runtime_source_MiB': manifest['admitted_bytes_actual']/1024**2,
        'unique_frames': manifest['unique_density_chunks'],
        'qualification': manifest['qualification'],
        'cadence': manifest['coarser_unwarped_cadence_summaries']}), flush=True)


if __name__ == '__main__':
    main()
