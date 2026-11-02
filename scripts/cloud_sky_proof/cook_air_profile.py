"""Cook the one selected Sun-only air profile. CPU float64, no GPU or ratio API."""
from dataclasses import asdict
import argparse
import hashlib
import json
import math
from pathlib import Path
import struct
import time

import numpy as np
import air_reference as reference
import optical_reference as optical

WIDTH, HEIGHT, SAMPLES = 256, 64, 512
PROFILE_ID = 'earth-authored.sun-air.256x64.midpoint512.v1'
MAPPING_ID = 'rho-local-height.top-distance.sun-only.v1'
GPU_TOLERANCE = .0015
POLICY = dict(horizon_ambiguity_mu=1e-6, unit_sun_length_tolerance=1e-5,
              cosine_round_allowance=2e-6, coordinate_round_allowance=2e-6,
              world_abs_xz_max_m=100000., world_y_min_m=0., world_y_max_m=20000.)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def encoded_json(value):
    return (json.dumps(value, separators=(',', ':'), sort_keys=True, allow_nan=False)+'\n').encode()


def f32(value):
    return struct.unpack('<f', struct.pack('<f', float(value)))[0]


def words(values):
    return [struct.unpack('<I', struct.pack('<f', float(v)))[0] for v in values]


def primitive_status(height, mu):
    """Independent f64 admission at exact supplied f32 inputs; no mu clamp."""
    if not math.isfinite(height) or not math.isfinite(mu) or not 0 <= height <= optical.EARTH.top_m or not -1 <= mu <= 1:
        return 'unsupported'
    if height == 0 and mu <= 0:
        return 'planet-blocked'
    if mu < 0:
        horizon = float(reference.horizon_mu(height))
        if abs(mu-horizon) <= POLICY['horizon_ambiguity_mu']:
            return 'unsupported'
        if mu <= horizon:
            return 'planet-blocked'
    return 'visible'


def reference_for_height_mu(height, mu):
    status = primitive_status(height, mu)
    if status == 'unsupported':
        return dict(status=status, valid=0, rgb=None)
    if status == 'planet-blocked':
        return dict(status=status, valid=1, rgb=[0., 0., 0.])
    return dict(status=status, valid=1, rgb=reference.oracle_top(height, mu, 256).tolist())


def reference_for_world(point, sun):
    """Independent existing spherical geometry/quadrature, not shader mapping."""
    if not all(math.isfinite(x) for x in point+sun) or abs(point[0]) > 100000 or abs(point[2]) > 100000 or not 0 <= point[1] <= 20000:
        return dict(status='unsupported', valid=0, rgb=None)
    length = math.sqrt(sum(x*x for x in sun))
    if abs(length-1) > POLICY['unit_sun_length_tolerance']:
        return dict(status='unsupported', valid=0, rgb=None)
    direction = np.asarray(sun)/length
    height = float(optical.radial_height(point))
    radial = np.asarray(point)+[0, optical.EARTH.radius_m, 0]
    mu = float(np.dot(radial, direction)/np.linalg.norm(radial))
    result = reference_for_height_mu(height, mu)
    if result['status'] == 'visible':
        terminal = optical.trace_terminal(point, direction)
        if terminal['status'] != 'ok':
            raise ValueError('independent world oracle: '+terminal.get('reason', 'unsupported terminal'))
        if terminal['terminal'] == 'planet':
            result = dict(status='planet-blocked', valid=1, rgb=[0., 0., 0.])
        else:
            tau = optical._tau_many(np.asarray(point)[None, :], direction[None, :], [terminal['distance_m']], 256, optical.EARTH)[0]
            result['rgb'] = np.exp(-tau).tolist()
    return {**result, 'radial_height_m':height, 'radial_mu':mu}


def query_cases():
    cases = []
    def add_primitive(h, mu, kind):
        h, mu = f32(h), f32(mu)
        cases.append(dict(id=len(cases), kind=kind, mode='height_mu', input_u32=words([h, mu, 0, 0, 0, 0, 0, 0]), expected=reference_for_height_mu(h, mu)))
    # The same named/off-grid population as the retained reference, now tested
    # at exact GPU input bits and with the explicitly selected ambiguity band.
    for h in [0., .001, 1., 50., 900., 18000., 50000., 99999., 100000.]:
        for elevation in [-15., -5., -1., -.1, 0., .1, 1., 2., 5., 15., 30., 75., 90.]:
            add_primitive(h, math.sin(math.radians(elevation)), 'named-Sun')
        for offset in [-1e-7, 0., 1e-9, 1e-7, 1e-5, .001]:
            add_primitive(h, float(reference.horizon_mu(h))+offset, 'horizon-reference-population')
    rng = np.random.default_rng(1729)
    for xy in rng.uniform(0, 1, (1024, 2)):
        h, mu = reference.from_coordinates(*xy)
        add_primitive(float(h), float(mu), 'seeded-offgrid')
    for h in [1., 900., 18000., 100000.]:
        for offset in [-2e-6, -.5e-6, 0., .5e-6, 2e-6]:
            add_primitive(h, float(reference.horizon_mu(h))+offset, 'explicit-ambiguity-band')
    for h, mu in [(-.001, 1), (100001, 1), (1, -1.0001), (1, 1.0001), (math.nan, 0), (1, math.inf), (math.inf, 1)]:
        add_primitive(h, mu, 'unsupported-primitive')
    points = [[0, 0, 0], [0, .001, 0], [512, 1000, 512], [100000, 0, 0], [0, 20000, 0],
              [100000, 20000, -100000], [-100000, 0, 100000], [0, 1, 0], [80000, 0, 0]]
    def add_world(point, sun, kind):
        point, sun = [f32(x) for x in point], [f32(x) for x in sun]
        cases.append(dict(id=len(cases), kind=kind, mode='world', input_u32=words(point+[1]+sun+[0]), expected=reference_for_world(point, sun)))
    for point in points:
        for elevation in [-5., 0., 5., 15., 30., 75., 90.]:
            angle = math.radians(elevation)
            add_world(point, [.9*math.cos(angle), math.sin(angle), math.sqrt(.19)*math.cos(angle)], 'world-named-Sun')
    for point, sun in [([100001, 0, 0], [0, 1, 0]), ([0, -.001, 0], [0, 1, 0]), ([0, 20001, 0], [0, 1, 0]),
                       ([0, 1, 0], [0, 2, 0]), ([0, 1, 0], [0, 0, 0]), ([math.nan, 1, 0], [0, 1, 0]),
                       ([0, 1, 0], [0, math.inf, 0]), ([100000, 0, -100001], [0, 1, 0])]:
        add_world(point, sun, 'unsupported-world')
    return cases


def cook_arrays():
    """One fixed float64 candidate; conversion to half occurs exactly once."""
    table = np.ones((HEIGHT, WIDTH, 4), dtype=np.float64)
    fine = np.empty((HEIGHT, WIDTH, 3), dtype=np.float64)
    x = np.linspace(0, 1, WIDTH)
    for row in range(HEIGHT):
        h, mu = reference.from_coordinates(x, row/(HEIGHT-1))
        table[row, :, :3] = reference.midpoint_top(h, mu)
        fine[row] = reference.oracle_top(h, mu, 256)
    if not np.isfinite(table).all() or not ((table >= 0) & (table <= 1)).all():
        raise ValueError('finite unit-interval table required')
    half = table.astype('<f2')
    return table, half, fine


def cook_package(out_dir):
    out_dir = Path(out_dir)
    table, half, fine = cook_arrays()
    cases = query_cases()
    sources = dict(cooker=sha(Path(__file__).read_bytes()), air_reference=sha(Path(reference.__file__).read_bytes()), optical_reference=sha(Path(optical.__file__).read_bytes()))
    oracle = dict(schema='thrum.air-sun-oracle.v1', profile_id=PROFILE_ID, source_hashes=sources,
                  reference='independent existing spherical Gauss256 geometry/density quadrature; authored calibration',
                  knots=dict(shape=[HEIGHT, WIDTH], order='height-major, mu-minor', reference_rgb=fine.reshape(-1, 3).tolist(),
                             mode='medium-only texture-coordinate knots, INCLUDING visible grazing limit; no planet visibility claim'),
                  queries=cases, policy=POLICY, gpu_total_absolute_tolerance=GPU_TOLERANCE, uniform_certificate=False)
    table_bytes = half.tobytes(order='C')
    oracle_bytes = encoded_json(oracle)
    errors = []
    for case in cases:
        if case['expected']['status'] != 'visible':
            continue
        values = struct.unpack('<8f', struct.pack('<8I', *case['input_u32']))
        if case['mode'] == 'height_mu':
            h, mu = values[:2]
        else:
            h, mu = case['expected']['radial_height_m'], case['expected']['radial_mu']
        actual = reference.lookup_medium(half[:, :, :3].astype(float), h, mu)
        errors.append(np.abs(actual-np.asarray(case['expected']['rgb'])))
    statuses = {name:sum(case['expected']['status'] == name for case in cases) for name in ['visible', 'planet-blocked', 'unsupported']}
    profile = dict(schema='thrum.air-sun-profile.v1', profile_id=PROFILE_ID, mapping_id=MAPPING_ID,
                   calibration={**asdict(optical.EARTH), 'ozone':'off', 'authored_fixture':True},
                   dimensions=dict(mu=WIDTH, height=HEIGHT), format='rgba16float', channels=['T_R', 'T_G', 'T_B', 'one'],
                   layout='little-endian binary16 row-major[height,mu,RGBA]',
                   cook=dict(method='float64 uniform512 midpoint integration, direct conversion to binary16', midpoint_samples=SAMPLES,
                             medium_evaluations=WIDTH*HEIGHT*SAMPLES, density_exponentials=WIDTH*HEIGHT*SAMPLES*2),
                   source_hashes=sources, files=dict(table=dict(filename='transmittance.rgba16f', bytes=len(table_bytes), sha256=sha(table_bytes)),
                                                   oracle=dict(filename='queries.json', bytes=len(oracle_bytes), sha256=sha(oracle_bytes), lazy_probe_only=True)),
                   domain=dict(height_m=[0., 100000.], mu=[-1., 1.], world_frame='planet centre=(0,-R,0); local tangent metres', **POLICY),
                   acceptance=dict(gpu_total_absolute_T=GPU_TOLERANCE, exact_uploaded_half_bytes=True,
                                   experimental_allocations=dict(table_quadrature_storage=.0005, hardware_filtering=.0005, mapping_numerics=.0005)),
                   finite_cpu_evidence=dict(knot_count=WIDTH*HEIGHT, knot_midpoint_error_max_rgb=np.max(np.abs(table[:, :, :3]-fine), axis=(0, 1)).tolist(),
                                            half_plane_error_max_rgb=np.max(np.abs(half[:, :, :3].astype(float)-table[:, :, :3]), axis=(0, 1)).tolist(),
                                            half_zero_count_rgb=(half[:, :, :3] == 0).sum(axis=(0, 1)).tolist(), query_status_counts=statuses,
                                            admitted_half_lookup_error_max_rgb=np.max(errors, axis=0).tolist()),
                   uniform_certificate=False, scope='Sun-air attenuation only; no finite-segment ratio, view-air source, A0, fog, physical sky or performance pass')
    out_dir.mkdir(parents=True, exist_ok=True)
    for name, payload in [('transmittance.rgba16f', table_bytes), ('queries.json', oracle_bytes), ('profile.json', encoded_json(profile))]:
        (out_dir/name).write_bytes(payload)
    return profile


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out-dir', type=Path, required=True)
    args = parser.parse_args()
    started = time.perf_counter()
    profile = cook_package(args.out_dir)
    print(json.dumps(dict(profile=PROFILE_ID, wall_seconds=time.perf_counter()-started, table=profile['files']['table'],
                          oracle_bytes=profile['files']['oracle']['bytes'], finite_cpu_evidence=profile['finite_cpu_evidence']), allow_nan=False))
