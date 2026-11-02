"""Proposed single 256mu x64height Sun-air LUT reference, CPU only.

Calibration is imported unchanged from optical_reference.EARTH. This is a top-
boundary AIR-medium product; opaque planet visibility is a separate exact branch.
Mapping follows Bruneton's public transmittance coordinates, with local-height
stable algebra. No clear-sky image, clouds, fog or multiple scattering is baked.
No shader/candidate implementation or table payload is written by this module.
"""
from dataclasses import asdict
import hashlib
import json
import math
from pathlib import Path

import numpy as np
import optical_reference as optical

ATMOSPHERE = optical.EARTH
MU_SIZE, HEIGHT_SIZE, MIDPOINT_SAMPLES = 256, 64, 512


def require(ok, message):
    if not ok:
        raise ValueError(message)


def horizon_mu(height, atmosphere=ATMOSPHERE):
    h = np.asarray(height, float)
    return -np.sqrt(h*(2*atmosphere.radius_m+h))/(atmosphere.radius_m+h)


def planet_blocked(height, mu, atmosphere=ATMOSPHERE):
    h, mu = np.broadcast_arrays(np.asarray(height, float), np.asarray(mu, float))
    require(np.isfinite(h).all() and np.isfinite(mu).all() and ((h >= 0) & (h <= atmosphere.top_m)).all()
            and ((mu >= -1) & (mu <= 1)).all(), 'height/mu outside admitted finite atmosphere')
    return (mu <= horizon_mu(h, atmosphere)) & (mu <= 0)


def distance_to_top(height, mu, atmosphere=ATMOSPHERE):
    h, mu = np.broadcast_arrays(np.asarray(height, float), np.asarray(mu, float))
    r, top, radius = atmosphere.radius_m+h, atmosphere.top_m, atmosphere.radius_m
    c = (top-h)*(2*radius+top+h)
    b = r*mu
    root = np.sqrt(b*b+c)
    # Divide only in outward branch; top/upward zero length is explicit.
    out = np.array(root-b, copy=True)
    denominator = root+b
    np.divide(c, denominator, out=out, where=(mu >= 0) & (denominator > 0))
    return np.where((h == top) & (mu >= 0), 0., out)


def to_coordinates(height, mu, atmosphere=ATMOSPHERE):
    h, mu = np.broadcast_arrays(np.asarray(height, float), np.asarray(mu, float))
    require(not planet_blocked(h, mu, atmosphere).any(), 'Sun ray intersects opaque planet')
    radius, top = atmosphere.radius_m, atmosphere.top_m
    rho = np.sqrt(h*(2*radius+h))
    cap = math.sqrt(top*(2*radius+top))
    d = distance_to_top(h, mu, atmosphere)
    delta = top-h
    difference = np.array(d-delta, copy=True)
    # Rationalized d-dmin near zenith avoids subtracting two nearby distances.
    b = (radius+h)*mu
    root = np.sqrt(b*b+delta*(2*radius+top+h))
    denominator = root+b+delta
    np.divide(2*(radius+h)*delta*(1-mu), denominator,
              out=difference, where=(mu >= 0) & (denominator > 0))
    return np.stack((difference/(rho+cap-delta), rho/cap), axis=-1)


def from_coordinates(x_mu, x_height, atmosphere=ATMOSPHERE):
    x, y = np.broadcast_arrays(np.asarray(x_mu, float), np.asarray(x_height, float))
    require(np.isfinite(x).all() and np.isfinite(y).all() and ((x >= 0) & (x <= 1)).all()
            and ((y >= 0) & (y <= 1)).all(), 'table coordinate domain')
    radius, top = atmosphere.radius_m, atmosphere.top_m
    cap = math.sqrt(top*(2*radius+top))
    rho = cap*y
    h = rho*rho/(np.sqrt(radius*radius+rho*rho)+radius)
    h = np.where(y == 1, top, h)
    delta = top-h
    d = delta+x*(rho+cap-delta)
    c = delta*(2*radius+top+h)
    denominator = 2*(radius+h)*d
    mu = np.ones_like(h)
    np.divide(c-d*d, denominator, out=mu, where=d > 0)
    mu = np.where(x == 0, 1., mu)
    mu = np.where(x == 1, horizon_mu(h, atmosphere), mu)
    # Inverse-only root roundoff, not clamping external below-ground input.
    return h, np.clip(mu, -1., 1.)


def heights_along(height, mu, distances, atmosphere=ATMOSPHERE):
    h, mu, s = np.broadcast_arrays(height, mu, distances)
    radius = atmosphere.radius_m
    rho = np.sqrt(h*(2*radius+h))
    b = (radius+h)*mu
    n = (s+b)*(s+b)+(rho+b)*(rho-b)
    altitude = n/(np.sqrt(radius*radius+n)+radius)
    require(np.min(altitude) >= -1e-6, 'medium integration crosses ground')
    # Only the existing oracle's accepted sub-micrometre root roundoff policy.
    return np.maximum(altitude, 0.)


def midpoint_top(height, mu, atmosphere=ATMOSPHERE):
    h, mu = np.broadcast_arrays(np.asarray(height, float), np.asarray(mu, float))
    distance = distance_to_top(h, mu, atmosphere)
    u = (np.arange(MIDPOINT_SAMPLES)+.5)/MIDPOINT_SAMPLES
    altitude = heights_along(h[..., None], mu[..., None], distance[..., None]*u, atmosphere)
    rayleigh = np.exp(-altitude/atmosphere.rayleigh_height_m).mean(axis=-1)*distance
    aerosol = np.exp(-altitude/atmosphere.aerosol_height_m).mean(axis=-1)*distance
    tau = (rayleigh[..., None]*np.array(atmosphere.rayleigh_scattering_per_m)
           +aerosol[..., None]*atmosphere.aerosol_extinction_per_m)
    return np.exp(-tau)


def oracle_top(height, mu, order=256, atmosphere=ATMOSPHERE):
    """Independent existing world-coordinate root/radial-height Gaussian oracle.

    Deliberately integrates the medium-only grazing limit without treating the
    top LUT boundary as a visible Sun: callers apply planet_blocked separately.
    """
    h, mu = np.broadcast_arrays(np.asarray(height, float), np.asarray(mu, float))
    shape = h.shape
    origins = np.zeros((h.size, 3));origins[:, 1] = h.ravel()
    directions = np.column_stack((np.sqrt(np.maximum(0., 1-mu.ravel()**2)), mu.ravel(), np.zeros(h.size)))
    distances = []
    for origin, direction in zip(origins, directions):
        roots = optical.ray_sphere_roots(origin, direction, atmosphere.top_m, atmosphere.radius_m)
        require(roots is not None, 'oracle top boundary')
        distances.append(max(0., roots[-1]))
    tau = optical._tau_many(origins, directions, distances, order, atmosphere)
    return np.exp(-tau).reshape(shape+(3,))


def cook_table(atmosphere=ATMOSPHERE):
    """One fixed candidate only; no dimensions/sample-count sweep."""
    table = np.empty((HEIGHT_SIZE, MU_SIZE, 3), dtype=np.float32)
    x = np.linspace(0., 1., MU_SIZE)
    for row in range(HEIGHT_SIZE):
        h, mu = from_coordinates(x, row/(HEIGHT_SIZE-1), atmosphere)
        table[row] = midpoint_top(h, mu, atmosphere)
    require(np.isfinite(table).all() and ((table >= 0) & (table <= 1)).all(), 'invalid air cook')
    return table


def lookup_medium(table, height, mu, atmosphere=ATMOSPHERE):
    coordinate = to_coordinates(height, mu, atmosphere)
    require(coordinate.shape == (2,), 'scalar medium lookup required')
    require((coordinate >= -1e-10).all() and (coordinate <= 1+1e-10).all(), 'unsupported lookup coordinate')
    # This tiny inverse-coordinate policy does not admit blocked rays.
    g = np.clip(coordinate, 0, 1)*np.array([MU_SIZE-1, HEIGHT_SIZE-1])
    lo = np.floor(g).astype(int);hi = np.minimum(lo+1, [MU_SIZE-1, HEIGHT_SIZE-1]);f = g-lo
    return ((1-f[0])*(1-f[1])*table[lo[1], lo[0]]+f[0]*(1-f[1])*table[lo[1], hi[0]]
            +(1-f[0])*f[1]*table[hi[1], lo[0]]+f[0]*f[1]*table[hi[1], hi[0]]).astype(float)


def sun_transmittance(table, height, mu, atmosphere=ATMOSPHERE):
    if planet_blocked(height, mu, atmosphere):
        return np.zeros(3)
    if height == atmosphere.top_m and mu >= 0:
        return np.ones(3)
    return lookup_medium(table, height, mu, atmosphere)


def segment_ratio(table, height, mu, length, reverse=False, atmosphere=ATMOSPHERE):
    """Diagnostic ratio only; not an admitted finite-view path implementation."""
    direction = np.array([math.sqrt(1-mu*mu), mu, 0.])
    origin = np.array([0., height, 0.])
    terminal = optical.trace_terminal(origin, direction, atmosphere)
    require(terminal['status'] == 'ok' and 0 <= length <= terminal['distance_m']+1e-8, 'segment crosses boundary')
    q = origin+length*direction
    hq = float(optical.radial_height(q, atmosphere.radius_m))
    muq = ((atmosphere.radius_m+height)*mu+length)/(atmosphere.radius_m+hq)
    if reverse:
        numerator = lookup_medium(table, hq, -muq, atmosphere)
        denominator = lookup_medium(table, height, -mu, atmosphere)
    else:
        numerator = lookup_medium(table, height, mu, atmosphere)
        denominator = lookup_medium(table, hq, muq, atmosphere)
    require((denominator > 0).all(), 'zero/underflow ratio denominator')
    return numerator/denominator


def finite_report(table):
    # Compare every candidate knot against the independent Gauss256 evaluator.
    worst_cook = np.zeros(3);minimum = np.ones(3)
    x = np.linspace(0., 1., MU_SIZE)
    for row in range(HEIGHT_SIZE):
        h, mu = from_coordinates(x, row/(HEIGHT_SIZE-1))
        oracle = oracle_top(h, mu)
        worst_cook = np.maximum(worst_cook, np.max(np.abs(table[row]-oracle), axis=0))
        minimum = np.minimum(minimum, table[row].min(axis=0))
    half = table.astype(np.float16).astype(float)
    rng = np.random.default_rng(1729)
    cases = []
    for h in [0., .001, 1., 50., 900., 18000., 50000., 99999., 100000.]:
        for elevation in [-15., -5., -1., -.1, 0., .1, 1., 2., 5., 15., 30., 75., 90.]:
            cases.append((h, math.sin(math.radians(elevation))))
        for offset in [-1e-7, 0., 1e-9, 1e-7, 1e-5, .001]:
            cases.append((h, float(horizon_mu(h))+offset))
    for xy in rng.uniform(0, 1, (1024, 2)):
        h, mu = from_coordinates(*xy)
        cases.append((float(h), float(mu)))
    visible = [(h, m) for h, m in cases if not planet_blocked(h, m)]
    reference = oracle_top([x[0] for x in visible], [x[1] for x in visible], 256)
    rows = []
    for (h, mu), true in zip(visible, reference):
        sample = lookup_medium(table, h, mu)
        half_sample = lookup_medium(half, h, mu)
        rows.append(dict(height_m=h, mu=mu, rgb=sample.tolist(), reference=true.tolist(),
                         error=np.abs(sample-true).tolist(), half_error=np.abs(half_sample-true).tolist()))
    worst = [max(rows, key=lambda r:r['error'][channel]) for channel in range(3)]
    # A few finite-segment tests report conditioning rather than claiming ratios.
    ratios = []
    for h, mu, length, reverse in [(0., 1., 1000., False), (900., -.01, 1000., True),
                                  (900., 0., 10000., False), (100000., float(horizon_mu(100000.))+.000001, 1000., False),
                                  (100000., float(horizon_mu(100000.))+.000001, 1000., True)]:
        origin = [0., h, 0.];direction = [math.sqrt(1-mu*mu), mu, 0.]
        true = optical.air_path(origin, direction, length, order=512)['air_transmittance_rgb']
        record = dict(height_m=h, mu=mu, length_m=length, reverse=reverse, reference=true)
        for key, data in [('f32_table', table), ('half_table', half)]:
            try:
                value = segment_ratio(data, h, mu, length, reverse)
                record[key] = dict(value=value.tolist(), error=np.abs(value-true).tolist(), physically_in_range=bool(((value >= 0) & (value <= 1)).all()))
            except ValueError as error:
                record[key] = dict(unavailable=str(error))
        ratios.append(record)
    return dict(schema='thrum.air-sun-reference.v1', status='PROPOSED; finite CPU comparison only',
                tool_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                optical_reference_sha256=hashlib.sha256(Path(optical.__file__).read_bytes()).hexdigest(),
                calibration=asdict(ATMOSPHERE), shape=[64,256,3], midpoint_samples=512,
                candidate_knot_count=16384, coefficient_evaluations=8388608,
                R32_RGBA_bytes=262144, R16_RGBA_bytes=131072,
                max_midpoint_knot_error_vs_Gauss256=worst_cook.tolist(), minimum_table_rgb=minimum.tolist(),
                half_storage_plane_error=np.max(np.abs(half-table), axis=(0,1)).tolist(),
                half_zero_count_rgb=(half == 0).sum(axis=(0,1)).tolist(), visible_offgrid_cases=len(visible),
                blocked_cases=len(cases)-len(visible), max_lookup_error=np.max([r['error'] for r in rows],axis=0).tolist(),
                max_half_lookup_error=np.max([r['half_error'] for r in rows],axis=0).tolist(), worst_cases=worst,
                segment_ratio_cases=ratios, uniform_certificate=False)


if __name__ == '__main__':
    print(json.dumps(finite_report(cook_table()), separators=(',', ':'), allow_nan=False))
