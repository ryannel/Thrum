"""Offline cloud-clip compiler/reader; no Thrum runtime or renderer dependency.

Run under Blender for OpenVDB inputs, or any NumPy Python for full-grid UNI.
The input master is an authored gas optical effect, not condensed cloud water.
"""
from __future__ import annotations

import argparse
import gzip
import hashlib
import json
from pathlib import Path
import struct
import sys
import time

import numpy as np

SCHEMA = 'thrum.cloud-source-proof.clip.v1'
EMPTY = 65535
BRICK = 8


def sha256(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def read_uni(path):
    """Pinned Blender e13b728c6880 MNT3 full-grid float32 cache, X fastest."""
    with gzip.open(path, 'rb') as stream:
        if stream.read(4) != b'MNT3':
            raise ValueError(f'{path}: expected MNT3')
        header = stream.read(288)
        if len(header) != 288:
            raise ValueError(f'{path}: truncated UNI header')
        nx, ny, nz, grid_type, element, width = struct.unpack_from('<6i', header)
        dim_t = struct.unpack_from('<i', header, 276)[0]
        components = {1: 1, 2: 3}.get(element)
        if components is None or width != components * 4 or dim_t != 0:
            raise ValueError(f'{path}: unsupported UNI field type')
        if min(nx, ny, nz) < 1 or max(nx, ny, nz) > 512:
            raise ValueError(f'{path}: unreasonable UNI dimensions')
        data = stream.read(nx * ny * nz * width + 1)
        if len(data) != nx * ny * nz * width:
            raise ValueError(f'{path}: wrong UNI payload length')
    shape = (nz, ny, nx) + ((3,) if components == 3 else ())
    array = np.frombuffer(data, dtype='<f4').reshape(shape).copy()
    if not np.isfinite(array).all():
        raise ValueError(f'{path}: nonfinite UNI field')
    return array, {'grid_type': grid_type, 'element_type': element, 'precision': 'float32',
                   'full_domain_velocity': components == 3}


def read_vdb(path):
    import openvdb
    grids, _ = openvdb.readAll(str(path))
    grids = {g.name: g for g in grids}
    result = []
    for name in ('density', 'velocity'):
        grid = grids[name]
        shape = tuple(int(v) for v in grid['file_base_resolution'])
        array = np.zeros(shape + ((3,) if name == 'velocity' else ()), np.float32)
        grid.copyToArray(array)
        # OpenVDB Python arrays address [x,y,z], unlike the native X-fastest file.
        array = array.transpose((2, 1, 0, 3) if name == 'velocity' else (2, 1, 0)).copy()
        transform = [list(grid.transform.indexToWorld(p)) for p in
                     [(0, 0, 0), (1, 0, 0), (0, 1, 0), (0, 0, 1)]]
        result.append((array, {'transform_samples': transform, 'class': str(grid['class']),
                              'saved_as_half': bool(grid['is_saved_as_half_float']),
                              'full_domain_velocity': False,
                              'reason': 'Blender smoke VDB exporter clips sparse velocity by density'}))
    return result


def read_frame(cache, frame):
    data = Path(cache)
    if (data / 'data').is_dir():
        data /= 'data'
    density = data / f'density_{frame:04d}.uni'
    velocity = data / f'vel_{frame:04d}.uni'
    if density.exists() and velocity.exists():
        d, dm = read_uni(density)
        v, vm = read_uni(velocity)
        if dm['element_type'] != 1 or not vm['grid_type'] & 8:
            raise ValueError('UNI source requires real density and staggered MAC velocity')
        return d, v, {'format': 'UNI', 'density': dm, 'velocity': vm,
                      'files': {str(p.name): sha256(p) for p in (density, velocity)}}
    path = data / f'fluid_data_{frame:04d}.vdb'
    (d, dm), (v, vm) = read_vdb(path)
    return d, v, {'format': 'VDB', 'density': dm, 'velocity': vm,
                  'files': {str(path.name): sha256(path)}}


def center_mac(velocity):
    """MAC components lie on the lower face of their same-index scalar cell."""
    out = np.empty_like(velocity)
    for component, axis in enumerate((2, 1, 0)):
        following = np.take(velocity[..., component],
                            np.minimum(np.arange(velocity.shape[axis]) + 1,
                                       velocity.shape[axis] - 1), axis=axis)
        out[..., component] = .5 * (velocity[..., component] + following)
    return out


def y_up(array, vector=False):
    """Blender XYZ -> engine (x,z,-y); input/output storage ZYX."""
    axes = (1, 0, 2, 3) if vector else (1, 0, 2)
    out = array.transpose(axes)[::-1].copy()
    if vector:
        out = out[..., [0, 2, 1]]
        out[..., 2] *= -1
    return np.ascontiguousarray(out)


def downsample(array, side):
    n = array.shape[0]
    if array.shape[:3] != (n, n, n) or n % side:
        raise ValueError('source must be a cube with integer carrier reduction')
    k = n // side
    tail = array.shape[3:]
    return array.reshape(side, k, side, k, side, k, *tail).mean(axis=(1, 3, 5), dtype=np.float32)


def pack_density(density):
    side = density.shape[0]
    if density.dtype != np.uint8 or density.shape != (side,) * 3 or side % BRICK:
        raise ValueError('expected cubic uint8 density, side divisible by8')
    n = side // BRICK
    bricks = density.reshape(n, BRICK, n, BRICK, n, BRICK).transpose(0, 2, 4, 1, 3, 5)
    bricks = np.ascontiguousarray(bricks).reshape(n ** 3, BRICK ** 3)
    active = np.any(bricks != 0, axis=1)
    index = np.full(n ** 3, EMPTY, '<u2')
    count = int(active.sum())
    if count >= EMPTY:
        raise ValueError('u16 brick index overflow')
    index[active] = np.arange(count, dtype=np.uint16)
    occupancy = np.packbits(active, bitorder='little').tobytes()
    payload = index.tobytes() + occupancy + bricks[active].tobytes()
    return payload, {'index_bytes': index.nbytes, 'occupancy_bytes': len(occupancy),
                     'active_bricks': count, 'bytes': len(payload)}


def unpack_density(payload, side, record):
    n = side // BRICK
    indices = np.frombuffer(payload, '<u2', count=n ** 3)
    occupied = np.unpackbits(np.frombuffer(payload[record['index_bytes']:
                         record['index_bytes'] + record['occupancy_bytes']], np.uint8),
                         bitorder='little')[:n ** 3].astype(bool)
    if not np.array_equal(occupied, indices != EMPTY):
        raise ValueError('density occupancy/index disagreement')
    count = record['active_bricks']
    valid = indices != EMPTY
    if np.any(indices[valid] >= count):
        raise ValueError('density brick index outside payload')
    start = record['index_bytes'] + record['occupancy_bytes']
    if len(payload) != start + count * BRICK ** 3:
        raise ValueError('density payload length disagrees with manifest')
    stored = np.frombuffer(payload, np.uint8, offset=start).reshape(count, BRICK ** 3)
    bricks = np.zeros((n ** 3, BRICK ** 3), np.uint8)
    bricks[valid] = stored[indices[valid]]
    return bricks.reshape(n, n, n, BRICK, BRICK, BRICK).transpose(0, 3, 1, 4, 2, 5).reshape((side,) * 3)


def sample(field, positions, extent):
    """Trilinear scalar/vector sampling; closest-point carrier extension."""
    n = field.shape[0]
    p = np.clip(np.asarray(positions) * n / extent - .5, 0, n - 1)
    lo = np.floor(p).astype(np.int32)
    hi = np.minimum(lo + 1, n - 1)
    frac = p - lo
    out = np.zeros(p.shape[:-1] + field.shape[3:], np.float32)
    for z in range(2):
        for y in range(2):
            for x in range(2):
                w = ((frac[..., 0] if x else 1-frac[..., 0]) *
                     (frac[..., 1] if y else 1-frac[..., 1]) *
                     (frac[..., 2] if z else 1-frac[..., 2]))
                value = field[(hi if z else lo)[..., 2],
                              (hi if y else lo)[..., 1], (hi if x else lo)[..., 0]]
                out += value * (w[..., None] if field.ndim == 4 else w)
    return out


def grid_positions(side, extent):
    c = (np.arange(side, dtype=np.float32) + .5) * extent / side
    z, y, x = np.meshgrid(c, c, c, indexing='ij')
    return np.stack((x, y, z), axis=-1)


def reconstruct(a, b, flow, theta, duration, extent):
    side = a.shape[0]
    support = (a > 0) | (b > 0)
    out = np.zeros_like(a)
    if not np.any(support):
        return out
    # The bounded flow characteristic cannot travel farther than duration*V.
    # Add one optical interpolation cell. The carrier itself remains full-domain.
    bound = float(np.linalg.norm(flow, axis=-1).max())
    padding = int(np.ceil(duration * bound * side / extent)) + 1
    indices = np.where(support)
    lo = [max(0, int(i.min())-padding) for i in indices]
    hi = [min(side, int(i.max())+padding+1) for i in indices]
    slices = tuple(slice(l, h) for l, h in zip(lo, hi))
    coords = [(np.arange(l, h, dtype=np.float32)+.5)*extent/side for l, h in zip(lo, hi)]
    z, y, x = np.meshgrid(*coords, indexing='ij')
    q = np.stack((x, y, z), axis=-1)
    v = sample(flow, q, extent)
    def endpoint(h):
        return q + h * sample(flow, q + .5 * h * v, extent)
    pa = endpoint(-theta * duration)
    pb = endpoint((1-theta) * duration)
    def optical(field, p):
        value = sample(field, p, extent)
        return np.where(np.all((p >= 0) & (p <= extent), axis=-1), value, 0)
    out[slices] = (1-theta) * optical(a, pa) + theta * optical(b, pb)
    return out


def flow_bounds(flow, spacing):
    speed = float(np.linalg.norm(flow, axis=-1).max())
    # A trilinear field's Jacobian is itself the trilinear interpolation of
    # its eight corner Jacobians. Convexity of the spectral norm proves this
    # corner maximum bounds every interior point, including clamped extension.
    shape = tuple(n-1 for n in flow.shape[:3])
    max_norm = 0.
    differences = [np.diff(flow, axis=axis) / spacing for axis in range(3)]
    for z in (0, 1):
        for y in (0, 1):
            for x in (0, 1):
                corner = (z, y, x)
                jacobian = np.empty(shape + (3, 3), np.float32)
                for axis in range(3):
                    slices = tuple(slice(None) if a == axis else
                                   slice(corner[a], corner[a]+shape[a]) for a in range(3))
                    jacobian[..., :, 2-axis] = differences[axis][slices]
                max_norm = max(max_norm, float(np.linalg.norm(jacobian, ord=2, axis=(-2, -1)).max()))
    return {'max_speed_m_per_source_s': speed,
            'lipschitz_spectral_corner_upper_per_s': max_norm}


def integrate_velocity_pair(q, previous, current, duration, extent, steps):
    """RK2 over real adjacent exports, linearly interpolating velocity in time."""
    q = q.copy()
    h = duration/steps
    for step in range(steps):
        t0 = step/steps
        tm = (step+.5)/steps
        v0 = (1-t0)*sample(previous, q, extent)+t0*sample(current, q, extent)
        mid = q+.5*h*v0
        vm = (1-tm)*sample(previous, mid, extent)+tm*sample(current, mid, extent)
        q += h*vm
    return q


def encoded_flow(v):
    scale = np.max(np.abs(v), axis=(0, 1, 2))
    codes = np.rint(v/np.where(scale > 0, scale, 1)*127).astype(np.int8)
    return codes, codes.astype(np.float32)/127*scale, scale


def optical_error(candidate, reference, dx):
    errors, tau_errors = [], []
    for axis in range(3):
        ca = candidate.sum(axis=axis, dtype=np.float64) * dx
        re = reference.sum(axis=axis, dtype=np.float64) * dx
        tau_errors.append(float(np.abs(ca-re).max()))
        errors.append(float(np.abs(np.exp(-ca)-np.exp(-re)).max()))
    return {'max_axial_tau_error': max(tau_errors),
            'max_axial_transmittance_abs_error': max(errors),
            'mean_abs_extinction_error_per_m': float(np.mean(np.abs(candidate-reference))),
            'max_abs_extinction_error_per_m': float(np.max(np.abs(candidate-reference)))}


def read_product(path):
    path = Path(path)
    m = json.loads((path / 'cloudclip.json').read_text())
    if m['schema'] != SCHEMA:
        raise ValueError('unsupported cloudclip schema')
    for name in ('density.bin', 'flow.bin'):
        if sha256(path / name) != m['products'][name]['sha256']:
            raise ValueError(f'{name}: hash mismatch')
    blob = (path / 'density.bin').read_bytes()
    fields = []
    for r in m['density_frames']:
        p = blob[r['offset']:r['offset']+r['bytes']]
        if hashlib.sha256(p).hexdigest() != r['sha256']:
            raise ValueError('density frame hash mismatch')
        fields.append(unpack_density(p, m['density_side'], r).astype(np.float32) *
                      np.float32(m['runtime_sigma_max_per_m'] / 255))
    packed = np.frombuffer((path / 'flow.bin').read_bytes(), np.int8)
    flows = packed.reshape(len(m['flow_intervals']), m['flow_side'], m['flow_side'], m['flow_side'], 3)
    scales = np.asarray([r['component_scale_m_per_s'] for r in m['flow_intervals']], np.float32)
    flow = flows.astype(np.float32) / np.float32(127) * scales[:, None, None, None, :]
    return {'density': np.stack(fields), 'flow': flow,
            'times': np.asarray(m['source_times_s'], np.float64), 'manifest': m}


def write_manifest(path, manifest):
    """Compact exact metadata with fixed-point accounting of its own byte count."""
    binaries = sum(record['bytes'] for record in manifest['products'].values())
    size = 0
    for _ in range(8):
        manifest['metadata_actual_bytes'] = size
        manifest['admitted_bytes_actual'] = binaries+size
        manifest['actual_source_cap_11MiB'] = binaries+size <= 11*1024**2
        manifest['actual_source_cap_16_75MiB'] = binaries+size <= 16.75*1024**2
        payload = (json.dumps(manifest, separators=(',', ':'))+'\n').encode()
        if len(payload) == size:
            Path(path).write_bytes(payload)
            return
        size = len(payload)
    raise ValueError('metadata byte accounting did not converge')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--cache', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--sigma-scale', type=float, default=.15)
    parser.add_argument('--runtime-sigma-max', type=float, default=.5)
    parser.add_argument('--extent', type=float, default=512)
    parser.add_argument('--frames', type=int, default=129)
    parser.add_argument('--source-step', type=float, default=1.875)
    parser.add_argument('--fps', type=float, default=8)
    parser.add_argument('--time-scale', type=float, default=1)
    parser.add_argument('--flow-side', type=int, default=32)
    parser.add_argument('--scratch-dense', type=Path)
    args = parser.parse_args(sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else None)
    if (args.frames-1) % 4:
        raise ValueError('endpoint/quarter proof requires1+4*N real input frames')
    if args.out.exists():
        raise ValueError('output must be a new directory')
    if min(args.sigma_scale, args.runtime_sigma_max, args.extent, args.source_step, args.fps) <= 0:
        raise ValueError('scales and time steps must be positive')
    source_path = args.cache.parent/'source.json'
    source_descriptor = json.loads(source_path.read_text())
    if source_descriptor['frames'] < args.frames or source_descriptor['solver_fps'] != args.fps:
        raise ValueError('source descriptor clock/frame count disagrees with requested cook')
    if source_descriptor['domain_side_local_m'] != args.extent:
        raise ValueError('source descriptor spatial scale disagrees with requested cook')
    declared_step = source_descriptor.get('source_seconds_per_cache_frame', args.source_step)
    if abs(declared_step-args.source_step) > 1e-12:
        raise ValueError('source descriptor source-time mapping disagrees with requested cook')
    args.out.mkdir(parents=True)
    scratch = args.scratch_dense
    if scratch:
        scratch.mkdir(parents=True, exist_ok=False)
    start = time.perf_counter()
    frames, low_flows, sources, measurements = [], [], [], []
    first, previous = None, None
    previous_velocity = None
    trajectory = None
    correspondence = []
    n = None
    all_full = True
    collar_max = 0.
    for f in range(1, args.frames+1):
        raw, velocity, metadata = read_frame(args.cache, f)
        if n is None:
            n = raw.shape[0]
        if raw.shape != (n, n, n) or velocity.shape != (n, n, n, 3):
            raise ValueError('source dimensions/transform changed')
        if np.any(raw < 0) or not np.isfinite(raw).all() or not np.isfinite(velocity).all():
            raise ValueError('source has negative density or nonfinite samples')
        sigma = y_up(raw) * np.float32(args.sigma_scale)
        if float(sigma.max()) > args.runtime_sigma_max:
            raise ValueError('source exceeds fixed optical scale; no silent clipping')
        # Manta advances .1*(25/fps)*time_scale internal units per frame.
        multiplier = (args.extent / n) * (.1*25/args.fps*args.time_scale) / args.source_step
        local_v = y_up(center_mac(velocity), vector=True) * np.float32(multiplier)
        low_flows.append(downsample(local_v, args.flow_side))
        if trajectory is not None and len(trajectory['start']):
            trajectory['fine'] = integrate_velocity_pair(trajectory['fine'], previous_velocity,
                                      local_v, args.source_step, args.extent, 4)
            trajectory['coarse'] = integrate_velocity_pair(trajectory['coarse'], previous_velocity,
                                      local_v, args.source_step, args.extent, 2)
            trajectory['persistent'] &= sample(sigma, trajectory['fine'], args.extent) > .001
        if (f-1) % 4 == 0:
            if trajectory is not None:
                active = trajectory['persistent']
                count = int(np.count_nonzero(active))
                record = {'interval': (f-1)//4-1, 'sampled_starts': len(active),
                          'persistently_nonzero_starts': count,
                          'density_threshold_per_m': .001}
                if count:
                    q0 = trajectory['start'][active]
                    reference = trajectory['fine'][active]
                    native_convergence = np.linalg.norm(reference-trajectory['coarse'][active], axis=-1)
                    v = (low_flows[-5]+low_flows[-1]+2*sum(low_flows[-4:-1]))/8
                    _, carrier, _ = encoded_flow(v)
                    exactish = integrate_velocity_pair(q0, carrier, carrier,
                                     4*args.source_step, args.extent, 16)
                    coarse = integrate_velocity_pair(q0, carrier, carrier,
                                     4*args.source_step, args.extent, 8)
                    runtime = integrate_velocity_pair(q0, carrier, carrier,
                                     4*args.source_step, args.extent, 1)
                    record.update(
                        max_sampled_carrier_fit_error_m=float(np.linalg.norm(exactish-reference, axis=-1).max()),
                        max_sampled_runtime_correspondence_error_m=float(np.linalg.norm(runtime-reference, axis=-1).max()),
                        native_trajectory_2_vs_4_substeps_max_m=float(native_convergence.max()),
                        carrier_trajectory_8_vs_16_substeps_max_m=float(np.linalg.norm(exactish-coarse, axis=-1).max()))
                correspondence.append(record)
            # Deterministic bounded sample from actual nonzero native source cells.
            indices = np.argwhere(sigma > .001)
            stride = max(1, int(np.ceil(len(indices)/4096)))
            indices = indices[::stride]
            q0 = (indices[:, ::-1].astype(np.float32)+.5)*args.extent/n
            trajectory = {'start': q0, 'fine': q0.copy(), 'coarse': q0.copy(),
                          'persistent': np.ones(len(q0), bool)}
        previous_velocity = local_v

        all_full &= bool(metadata['velocity']['full_domain_velocity'])
        sources.append({'frame': f, 'source_s': (f-1)*args.source_step, **metadata})
        if scratch:
            np.savez_compressed(scratch / f'fine-{f:04d}.npz', density=sigma,
                                source_time_s=np.float64((f-1)*args.source_step))
        if (f-1) % 4 == 0:
            frames.append(sigma)
        if first is None:
            first = sigma
        previous = sigma
        # Eight fine cells on every face are the declared validity collar.
        k = min(8, n//4)
        edge = max(float(np.max(sigma[:k])), float(np.max(sigma[-k:])),
                   float(np.max(sigma[:, :k])), float(np.max(sigma[:, -k:])),
                   float(np.max(sigma[:, :, :k])), float(np.max(sigma[:, :, -k:])))
        collar_max = max(collar_max, edge)
        measurements.append({'frame': f, 'mass_like_sigma_cell_sum': float(sigma.sum(dtype=np.float64)),
                             'max_sigma_per_m': float(sigma.max()),
                             'positive_voxels': int(np.count_nonzero(sigma))})
        if f % 16 == 1:
            print(f'READ {f}/{args.frames}', flush=True)
    density_records, density_bytes, decoded = [], bytearray(), []
    for sigma in frames:
        codes = np.rint(sigma / args.runtime_sigma_max * 255).astype(np.uint8)
        payload, record = pack_density(codes)
        record.update(offset=len(density_bytes), sha256=hashlib.sha256(payload).hexdigest())
        density_bytes.extend(payload)
        density_records.append(record)
        decoded.append(codes.astype(np.float32) * np.float32(args.runtime_sigma_max/255))
    flow_records, flow_bytes, decoded_flow = [], bytearray(), []
    for i in range(len(frames)-1):
        # Composite trapezoid over actual source velocities including quarters.
        v = (low_flows[4*i] + low_flows[4*i+4] +
             2*sum(low_flows[4*i+1:4*i+4])) / np.float32(8)
        codes, recovered, scale = encoded_flow(v)
        flow_bytes.extend(codes.tobytes())
        flow_records.append({'component_scale_m_per_s': scale.tolist(),
                             'unquantized_bounds': flow_bounds(v, args.extent/args.flow_side),
                             'quantized_bounds': flow_bounds(recovered, args.extent/args.flow_side),
                             'max_velocity_vector_quant_error': float(np.linalg.norm(recovered-v, axis=-1).max())})
        decoded_flow.append(recovered)
    (args.out/'density.bin').write_bytes(density_bytes)
    (args.out/'flow.bin').write_bytes(flow_bytes)
    quant_errors = [optical_error(d, s, args.extent/n) for d, s in zip(decoded, frames)]
    temporal = []
    if scratch:
        for i, v in enumerate(decoded_flow):
            for j in (1, 2, 3):
                frame = 4*i+j+1
                with np.load(scratch/f'fine-{frame:04d}.npz') as data:
                    ref = data['density']
                    candidate = reconstruct(decoded[i], decoded[i+1], v, j/4,
                                            4*args.source_step, args.extent)
                    temporal.append({'source_frame': frame,
                                     **optical_error(candidate, ref, args.extent/n)})
            print(f'REFERENCE interval{i+1}/{len(decoded_flow)}', flush=True)
    rejects = []
    if not all_full:
        rejects.append('missing full-domain carrier: VDB density clipping; screening only')
    if float(first.max()) != 0 or float(previous.max()) != 0:
        rejects.append('density endpoints are not both exact zero')
    if collar_max != 0:
        rejects.append('nonzero density inside required eight-cell boundary collar')
    if n != 128 or args.frames != 129:
        rejects.append('screening dimensions/time range differ from128^3/240s final regime')
    for i, record in enumerate(flow_records):
        b = record['quantized_bounds']
        if b['max_speed_m_per_source_s'] > 2 or b['lipschitz_spectral_corner_upper_per_s'] > .02:
            rejects.append(f'interval{i}: V/L admission exceeded (trilinear spectral corner bound)')
    if not temporal:
        rejects.append('actual-quarter optical reference not run')
    elif max(e['max_axial_tau_error'] for e in temporal) > .1:
        rejects.append('actual-quarter axial source-ray optical-depth error exceeds0.1')
    if max(e['max_axial_tau_error'] for e in quant_errors) > .1:
        rejects.append('R8 endpoint axial source-ray optical-depth error exceeds0.1')
    if any(r.get('max_sampled_runtime_correspondence_error_m', 0) > 1 for r in correspondence):
        rejects.append('sampled native-velocity runtime correspondence exceeds1m')
    total = len(density_bytes)+len(flow_bytes)
    if total+131072 > 16.75 * 1024**2:
        rejects.append('source binaries exceed16.75MiB portable source cap')
    manifest = {'schema': SCHEMA, 'producer': 'authored Mantaflow gas optical effect; not condensed water',
        'compiler_sha256': sha256(__file__), 'source_descriptor': source_descriptor,
        'source_descriptor_sha256': sha256(source_path), 'density_side': n, 'flow_side': args.flow_side,
        'extent_m': args.extent, 'bounds_min_xyz_m': [-args.extent/2, 0, -args.extent/2],
        'axes': 'Y-up right-handed; x=BlenderX,y=BlenderZ,z=-BlenderY',
        'storage_order': 'X fastest then Y then Z; flowXYZinterleaved',
        'voxel_centres': 'bounds_min+(ijk+.5)*extent/side; fixed across frames',
        'sigma_per_raw_density_per_m': args.sigma_scale,
        'runtime_sigma_max_per_m': args.runtime_sigma_max,
        'source_times_s': [i*4*args.source_step for i in range(len(frames))],
        'raw_manta_velocity_to_local_m_per_source_s': multiplier,
        'mac_recentering': 'component average with positive-axis neighbour; final face uses clamped boundary',
        'flow_temporal_average': 'five actual frame samples, composite trapezoid weights1,2,2,2,1 divided8',
        'flow_encoding': 'XYZsignedint8SNORM127 with perinterval/component scale; no clamp',
        'density_encoding': 'u16LEbrickindex empty65535; littlebitorderoccupancy;512R8voxels/activebrick',
        'density_frames': density_records, 'flow_intervals': flow_records,
        'products': {name: {'bytes': (args.out/name).stat().st_size, 'sha256': sha256(args.out/name)}
                     for name in ('density.bin', 'flow.bin')},
        'source_binary_MiB': total/1024**2, 'metadata_reserved_bytes': 131072,
        'admitted_bytes_with_metadata_reservation': total+131072, 'source_cap_11MiB': total+131072 <= 11*1024**2,
        'source_cap_16_75MiB': total+131072 <= 16.75*1024**2,
        'empty_boundary_collar_max_sigma_per_m': collar_max,
        'endpoint_max_sigma_per_m': [float(first.max()), float(previous.max())],
        'quantization_axial_optical_errors': quant_errors,
        'actual_quarter_reference_errors': temporal,
        'quarter_reference_limit': 'independent source frames; axial optical projections only, not full render or correspondence qualification',
        'sampled_native_velocity_correspondence': correspondence,
        'persistent_correspondence_gate': 'diagnostic: up to4096 deterministic native-positive seeds/interval, retained if sigma>.001 at allfive exports; sampled true native MAC-centered velocities with linear time interpolation.2/4substeps report integration convergence; subexport temporal uncertainty and unsampled structures remain unqualified',
        'qualification': 'rejected' if rejects else 'numerical gates pending optical/correspondence visual review',
        'rejection_reasons': rejects, 'source_frames': sources,
        'fine_frame_measurements': measurements, 'cook_wall_s': time.perf_counter()-start}
    write_manifest(args.out/'cloudclip.json', manifest)
    if scratch:
        np.savez_compressed(scratch/'cooked.npz', density=np.stack(decoded),
                            flow=np.stack(decoded_flow), times=np.asarray(manifest['source_times_s']))
    # Validate reader against exactly the admitted bytes, not pre-encoding arrays.
    reread = read_product(args.out)
    if not np.array_equal(reread['density'], np.stack(decoded)):
        raise ValueError('density reader roundtrip failed')
    if not np.array_equal(reread['flow'], np.stack(decoded_flow)):
        raise ValueError('flow reader roundtrip failed')
    print('COOK_COMPLETE', json.dumps({'out': str(args.out), 'source_MiB': total/1024**2,
                                      'rejection_reasons': rejects}), flush=True)


if __name__ == '__main__':
    main()
