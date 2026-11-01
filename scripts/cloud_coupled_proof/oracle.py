"""Independent float64 optical reference; input decoding alone may reuse the Q1 reader.

R16F input is little endian, tightly packed Z/Y/X (X fastest), cell-centred.
No renderer/shader source is imported. Finite support uses edge-clamped trilinear
values inside its AABB and exact zero outside. World/source time is latched once.
"""
from __future__ import annotations

import argparse
from dataclasses import dataclass
import gzip
import importlib.util
import json
from pathlib import Path

import numpy as np


def unit(direction):
    value = np.asarray(direction, dtype=np.float64)
    length = np.linalg.norm(value)
    if value.shape != (3,) or not np.isfinite(value).all() or length == 0:
        raise ValueError('finite nonzero 3D direction required')
    return value / length


def ray_box(origin, direction, lower, upper, distance=np.inf):
    """Return the finite forward interval, in metres; None for a missed support."""
    origin, lower, upper = map(lambda v: np.asarray(v, np.float64), (origin, lower, upper))
    direction = unit(direction)
    if not np.isfinite(origin).all() or distance < 0 or np.isnan(distance):
        raise ValueError('invalid finite origin/path length')
    near, far = 0.0, float(distance)
    for axis in range(3):
        if direction[axis] == 0:
            if origin[axis] < lower[axis] or origin[axis] > upper[axis]:
                return None
        else:
            ends = (np.asarray([lower[axis], upper[axis]]) - origin[axis]) / direction[axis]
            near, far = max(near, ends.min()), min(far, ends.max())
    return (near, far) if far > near else None


@dataclass
class Field:
    values: np.ndarray
    lower: np.ndarray
    extent: float = 512.0

    def __post_init__(self):
        self.values = np.asarray(self.values, np.float64)
        self.lower = np.asarray(self.lower, np.float64)
        if (self.values.ndim != 3 or len(set(self.values.shape)) != 1
                or not np.isfinite(self.values).all() or np.any(self.values < 0)
                or self.lower.shape != (3,) or not np.isfinite(self.lower).all()
                or not np.isfinite(self.extent) or self.extent <= 0):
            raise ValueError('nonnegative finite cubic density and finite world support required')

    def sample(self, points):
        p = np.asarray(points, np.float64)
        size = self.values.shape[0]
        local = (p - self.lower) / self.extent
        q = np.clip(local * size - 0.5, 0, size - 1)
        lo = np.floor(q).astype(np.int64)
        hi = np.minimum(lo + 1, size - 1)
        f = q - lo
        result = np.zeros(p.shape[:-1], np.float64)
        for z in (0, 1):
            for y in (0, 1):
                for x in (0, 1):
                    weight = ((f[..., 0] if x else 1-f[..., 0])
                              * (f[..., 1] if y else 1-f[..., 1])
                              * (f[..., 2] if z else 1-f[..., 2]))
                    result += weight * self.values[(hi if z else lo)[..., 2],
                                                   (hi if y else lo)[..., 1],
                                                   (hi if x else lo)[..., 0]]
        return np.where(np.all((local >= 0) & (local <= 1), axis=-1), result, 0)

    def tau(self, origin, direction, distance=np.inf, step=1.0):
        if not np.isfinite(step) or step <= 0:
            raise ValueError('positive finite integration step required')
        interval = ray_box(origin, direction, self.lower, self.lower+self.extent, distance)
        if interval is None:
            return 0.0
        begin, end = interval
        count = max(1, int(np.ceil((end-begin)/step)))
        h = (end-begin)/count
        points = np.asarray(origin) + unit(direction) * (begin+(np.arange(count)+0.5)*h)[:, None]
        return float(np.sum(self.sample(points), dtype=np.float64)*h)

    def mips(self):
        fields = [self]
        for _ in range(2):
            v = fields[-1].values
            if v.shape[0] % 2:
                raise ValueError('mip fixture side must be divisible by four')
            n = v.shape[0]//2
            fields.append(Field(v.reshape(n, 2, n, 2, n, 2).mean(axis=(1, 3, 5)), self.lower, self.extent))
        return fields


def segment(sigma_t, source, distance):
    """Exact constant-coefficient affine solution (T,S), including zero sigma."""
    sigma, source = np.broadcast_arrays(np.asarray(sigma_t, np.float64), np.asarray(source, np.float64))
    if distance < 0 or np.any(sigma < 0):
        raise ValueError('nonnegative optical path required')
    transmittance = np.exp(-sigma*distance)
    factor = np.full(sigma.shape, float(distance))
    np.divide(-np.expm1(-sigma*distance), sigma, out=factor, where=sigma != 0)
    return transmittance, source*factor


def compose(front, back):
    """Front then back along camera-to-terminal path; attenuation occurs once."""
    tf, sf = front
    tb, sb = back
    return tf*tb, sf+tf*sb


def hg(mu, g):
    if not -1 < g < 1 or np.any(np.abs(mu) > 1+1e-12):
        raise ValueError('HG requires normalized directions and |g|<1')
    return (1-g*g)/(4*np.pi*(1+g*g-2*g*np.clip(mu, -1, 1))**1.5)


def cloud_source(sigma_s, mu, illuminance, cloud_depths, noncloud_tau=0,
                 geometry_visibility=1, ambient=0, g=0.866617162492502,
                 a=0.8, b=0.2, c=0.5):
    """CLOUDS closure: primary direct term plus three source-only higher orders.

    cloud_depths are mips 0/1/2. Ambient is authoritative incident radiance,
    already evaluated externally; it receives no phase or directional multiplier.
    """
    tau = np.asarray(cloud_depths, np.float64)
    if tau.shape != (3,) or np.any(tau < 0) or np.any(np.asarray(sigma_s) < 0):
        raise ValueError('three nonnegative cloud depths/scattering required')
    primary = np.exp(-tau[0])*hg(mu, g)
    higher = sum(a**n*np.exp(-b**n*tau[min(n, 2)])*hg(mu, g*c**n) for n in (1, 2, 3))
    direct = np.asarray(illuminance)*geometry_visibility*np.exp(-np.asarray(noncloud_tau))
    return np.asarray(sigma_s)*(direct*(primary+higher)+np.asarray(ambient))


def reflect_point(point, plane_point=(0, 0, 0), plane_normal=(0, 1, 0)):
    normal = unit(plane_normal)
    point, plane_point = np.asarray(point, np.float64), np.asarray(plane_point, np.float64)
    return point-2*np.dot(point-plane_point, normal)*normal


def reflect_direction(direction, plane_normal=(0, 1, 0)):
    normal, direction = unit(plane_normal), unit(direction)
    return direction-2*np.dot(direction, normal)*normal


def mirror_hit(origin, direction, plane_point=(0, 0, 0), plane_normal=(0, 1, 0)):
    normal, direction = unit(plane_normal), unit(direction)
    denom = np.dot(direction, normal)
    if abs(denom) < 1e-14:
        return None
    distance = np.dot(np.asarray(plane_point)-origin, normal)/denom
    return np.asarray(origin)+distance*direction if distance >= 0 else None


def read_density(path, side=128):
    path = Path(path)
    data = gzip.decompress(path.read_bytes()) if path.suffix == '.gz' else path.read_bytes()
    if len(data) != side**3*2:
        raise ValueError('R16F byte length does not match the declared cubic side')
    return np.frombuffer(data, dtype='<f2').reshape((side,)*3).astype(np.float64)


def read_clip(path, age):
    """Reuse only the independently hash-checked Q1 file decoder, never optics."""
    reader_path = Path(__file__).parents[1]/'cloud_source_proof'/'ranked.py'
    spec = importlib.util.spec_from_file_location('q1_input_decoder', reader_path)
    decoder = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(decoder)
    source = decoder.read_product(path)
    times = source['times']
    if not times[0] <= age <= times[-1]:
        raise ValueError('source age outside selected clip; no implicit wrapping')
    upper = min(int(np.searchsorted(times, age, side='right')), len(times)-1)
    lower = max(upper-1, 0)
    alpha = 0 if upper == lower else (age-times[lower])/(times[upper]-times[lower])
    return ((1-alpha)*source['read_frame'](lower).astype(np.float64)
            + alpha*source['read_frame'](upper).astype(np.float64))


def sphere_interval(origin, direction, centre, radius, distance=4000):
    direction = unit(direction)
    offset = np.asarray(origin)-np.asarray(centre)
    b = np.dot(offset, direction)
    discriminant = b*b-np.dot(offset, offset)+radius*radius
    if discriminant < 0:
        return None
    root = np.sqrt(discriminant)
    near, far = max(0., -b-root), min(float(distance), -b+root)
    return (near, far) if far > near else None


def air_tau(origin, direction, distance, sigma0, scale_height=8000):
    """Analytic exponential-height atmosphere over a finite above-ground segment."""
    q = -unit(direction)[1]*distance/scale_height
    factor = 1. if abs(q) < 1e-10 else np.expm1(q)/q
    return np.asarray(sigma0)*np.exp(-np.asarray(origin)[1]/scale_height)*distance*factor


def default_scene():
    return {'sun_direction': [-.55, .82, .15], 'sun_rgb': [90000, 85000, 75000],
            'sky_rgb': [1800, 3000, 5000], 'air_sigma0': np.array([6, 12, 24])*1e-6,
            'air_scale_height': 8000., 'light_distance': 4000.,
            'fog_spheres': [{'centre': [-20, 210, 40], 'radius': 105, 'sigma': .0014,
                             'albedo': .9, 'g': .6},
                            {'centre': [65, 160, 110], 'radius': 100, 'sigma': .001,
                             'albedo': .9, 'g': .6}],
            'opaque_spheres': [{'centre': [70, 45, 15], 'radius': 45}],
            'ground_y': 0., 'cloud_albedo': 1.}


def noncloud_light_tau(point, direction, distance, scene):
    tau = air_tau(point, direction, distance, scene['air_sigma0'], scene['air_scale_height'])
    for fog in scene['fog_spheres']:
        interval = sphere_interval(point, direction, fog['centre'], fog['radius'], distance)
        if interval:
            tau += fog['sigma']*(interval[1]-interval[0])
    return tau


def geometry_visibility(point, direction, scene):
    # Analytic sphere visibility is independent of the candidate geometry shadow map.
    for sphere in scene['opaque_spheres']:
        interval = sphere_interval(point, direction, sphere['centre'], sphere['radius'], scene['light_distance'])
        if interval and interval[1] > 1e-6:
            return 0.
    return 1.


def ambient_reference(field, point, scene=None, step=1):
    """Six-axis sky recipe with -Y horizon zero; direct optical quadrature reference."""
    scene = default_scene() | (scene or {})
    total = np.zeros(3)
    for direction in ([1, 0, 0], [-1, 0, 0], [0, 1, 0], [0, 0, 1], [0, 0, -1]):
        tau = (field.tau(point, direction, scene['light_distance'], step)
               + noncloud_light_tau(point, direction, scene['light_distance'], scene))
        total += np.asarray(scene['sky_rgb'])*np.exp(-tau)
    return total/6


def read_ambient(path, lower, extent=512, side=32, reference=5000):
    """GPU RGBA16F normalized ambient cache as an external source, RGB only."""
    path = Path(path)
    raw = gzip.decompress(path.read_bytes()) if path.suffix == '.gz' else path.read_bytes()
    if len(raw) != side**3*8:
        raise ValueError('ambient RGBA16F byte length mismatch')
    values = np.frombuffer(raw, '<f2').reshape(side, side, side, 4).astype(np.float64)
    channels = [Field(values[..., c]*reference, lower, extent) for c in range(3)]
    return lambda point: np.array([channel.sample(point) for channel in channels])


def trace_joint(field, origin, direction, distance=4000, step=1, light_step=1,
                scene=None, ambient=(0, 0, 0), terminal=(0, 0, 0), clip_geometry=True,
                light_depths=None, cloud_on=True):
    """Bounded camera-to-terminal joint solver; returns RGB T,S and completed L.

    Sun photons travel along -L; outgoing-to-camera is -view_direction, hence
    mu=dot(L,view_direction). Air uses normalized Rayleigh, fog uses HG. Only
    clouds receive authored ambient/higher orders. Opaque boundaries terminate
    the ray; reflection callers start the far ray at the actual mirror hit.
    """
    scene = default_scene() | (scene or {})
    origin, direction = np.asarray(origin, np.float64), unit(direction)
    if not 0 <= distance <= 4000 or step <= 0 or light_step <= 0:
        raise ValueError('camera path must be 0..4000m, with positive steps')
    if clip_geometry:
        for sphere in scene['opaque_spheres']:
            interval = sphere_interval(origin, direction, sphere['centre'], sphere['radius'], distance)
            if interval:
                distance = min(distance, interval[0])
        ground = scene.get('ground_y')
        if ground is not None and direction[1] < 0:
            hit = (ground-origin[1])/direction[1]
            if 0 <= hit < distance:
                distance = hit
    boundaries = [0., distance]
    support = ray_box(origin, direction, field.lower, field.lower+field.extent, distance)
    if support:
        boundaries.extend(support)
    for fog in scene['fog_spheres']:
        interval = sphere_interval(origin, direction, fog['centre'], fog['radius'], distance)
        if interval:
            boundaries.extend(interval)
    boundaries = sorted(set(boundaries))
    to_sun = unit(scene['sun_direction'])
    mu = float(np.dot(to_sun, direction))
    rayleigh = 3*(1+mu*mu)/(16*np.pi)
    mips = field.mips()
    transmittance, scattered = np.ones(3), np.zeros(3)
    sample_count = 0
    for begin, end in zip(boundaries[:-1], boundaries[1:]):
        count = max(1, int(np.ceil((end-begin)/step)))
        h = (end-begin)/count
        for t in begin+(np.arange(count)+.5)*h:
            point = origin+direction*t
            cloud = float(field.sample(point)) if cloud_on else 0.
            air = np.asarray(scene['air_sigma0'])*np.exp(-point[1]/scene['air_scale_height'])
            active_fog = [fog for fog in scene['fog_spheres']
                          if np.linalg.norm(point-np.asarray(fog['centre'])) < fog['radius']]
            fog_t = sum(fog['sigma'] for fog in active_fog)
            sigma_t = air+fog_t+cloud
            visibility = geometry_visibility(point, to_sun, scene)
            raw = np.asarray(scene['sun_rgb'])*visibility
            if np.any(raw):
                noncloud = noncloud_light_tau(point, to_sun, scene['light_distance'], scene)
                if light_depths is not None:
                    depths = light_depths(point)
                else:
                    depths = [mip.tau(point, to_sun, scene['light_distance'], light_step)
                              for mip in mips] if cloud else [field.tau(point, to_sun, scene['light_distance'], light_step)]*3
                direct = raw*np.exp(-noncloud-depths[0])
                source = direct*(air*rayleigh+sum(fog['sigma']*fog['albedo']*hg(mu, fog['g'])
                                                  for fog in active_fog))
            else:
                noncloud, depths, source = 0., [0., 0., 0.], np.zeros(3)
            lamb = ambient(point) if callable(ambient) else np.asarray(ambient)
            source += cloud_source(cloud*scene['cloud_albedo'], mu, raw, depths,
                                   noncloud_tau=noncloud, ambient=lamb)
            local_t, local_s = segment(sigma_t, source, h)
            scattered += transmittance*local_s
            transmittance *= local_t
            sample_count += 1
    return {'T': transmittance.tolist(), 'S': scattered.tolist(),
            'radiance': (scattered+transmittance*np.asarray(terminal)).tolist(),
            'distance': float(distance), 'samples': sample_count}


def evaluate_queries(field, queries, ambient=None, scene=None):
    results = []
    if len(queries) > 64:
        raise ValueError('bounded oracle admits at most64 probe rays per invocation')
    for query in queries:
        origin, direction = query['origin'], query['direction']
        distance, step = query.get('max_distance', np.inf), query.get('step', 1.0)
        if query.get('kind') == 'joint':
            options = {'distance': query.get('max_distance', 4000),
                       'light_step': query.get('light_step', 1),
                       'scene': (scene or {}) | query.get('scene', {}),
                       'ambient': ambient if ambient is not None else query.get('cloud_ambient', [0, 0, 0]),
                       'terminal': query.get('terminal', [0, 0, 0]),
                       'clip_geometry': query.get('clip_geometry', True)}
            result = trace_joint(field, origin, direction, step=step, **options)
            options['light_step'] /= 2
            refined = trace_joint(field, origin, direction, step=step/2, **options)
            results.append({'id': query.get('id'), **result, 'refined': refined,
                            'S_quadrature_delta': (np.asarray(result['S'])-refined['S']).tolist()})
            continue
        tau = field.tau(origin, direction, distance, step)
        refined = field.tau(origin, direction, distance, step/2)
        result = {'id': query.get('id'), 'cloud_tau': tau, 'cloud_T': float(np.exp(-tau)),
                  'refined_tau': refined, 'quadrature_delta': abs(tau-refined)}
        if 'gpu_tau' in query:
            result.update(gpu_tau=query['gpu_tau'], gpu_tau_error=float(query['gpu_tau']-refined),
                          gpu_T_error=float(np.exp(-query['gpu_tau'])-np.exp(-refined)))
        if 'air_tau' in query or 'fog_tau' in query:
            joint = refined+np.asarray(query.get('air_tau', 0))+np.asarray(query.get('fog_tau', 0))
            result.update(joint_tau=np.asarray(joint).tolist(), joint_T=np.exp(-joint).tolist())
        results.append(result)
    return results


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument('--density', type=Path)
    source.add_argument('--clip', type=Path)
    parser.add_argument('--age', type=float, default=90)
    parser.add_argument('--side', type=int, default=128)
    parser.add_argument('--world-min', nargs=3, type=float)
    parser.add_argument('--extent', type=float, default=512)
    parser.add_argument('--queries', type=Path, required=True)
    parser.add_argument('--ambient', type=Path)
    parser.add_argument('--ambient-side', type=int, default=32)
    parser.add_argument('--ambient-lref', type=float, default=5000)
    parser.add_argument('--out', type=Path)
    args = parser.parse_args()
    lower = args.world_min if args.world_min else np.array([-256, 120, -256])+np.array([4, 0, 1])*(args.age-90)
    values = read_clip(args.clip, args.age) if args.clip else read_density(args.density, args.side)
    payload = json.loads(args.queries.read_text())
    queries = payload['queries'] if isinstance(payload, dict) else payload
    ambient = read_ambient(args.ambient, lower, args.extent, args.ambient_side, args.ambient_lref) if args.ambient else None
    results = evaluate_queries(Field(values, lower, args.extent), queries, ambient,
                               payload.get('scene') if isinstance(payload, dict) else None)
    text = json.dumps({'world_min': np.asarray(lower).tolist(), 'extent': args.extent,
                       'age': args.age, 'results': results}, indent=2, allow_nan=False)+'\n'
    if args.out:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(text)
    else:
        print(text, end='')


if __name__ == '__main__':
    main()
