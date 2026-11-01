"""Independent float64 optical oracle for the bounded cloud-sky proof.

Authored Earth-like coefficients, not a spectral fit or engine implementation.
No candidate GPU code is imported. Coordinates are local metres with planet centre
(0, -R, 0); all ray directions are unit vectors. NumPy is the sole dependency.
Run with --json for compact, versioned comparison cases on stdout.
"""
from __future__ import annotations

import argparse
from dataclasses import asdict, dataclass
from functools import lru_cache
import json
import math

import numpy as np

SCHEMA = "thrum.cloud-sky-optical-reference.v1"
WEIGHTS = (0.7, 0.2, 0.07, 0.03)
BRANCH_B = (1.0, 0.5, 0.25, 0.125)
LIQUID_G = (0.86, 0.43, 0.215, 0.1075)


@dataclass(frozen=True)
class Atmosphere:
    radius_m: float = 6_371_000.0
    top_m: float = 100_000.0
    rayleigh_scattering_per_m: tuple[float, ...] = (5.802e-6, 13.558e-6, 33.1e-6)
    rayleigh_height_m: float = 8_000.0
    aerosol_scattering_per_m: float = 3.996e-6
    aerosol_extinction_per_m: float = 4.4e-6
    aerosol_height_m: float = 1_200.0
    aerosol_g: float = 0.8

    def __post_init__(self):
        positive = (self.radius_m, self.top_m, self.rayleigh_height_m, self.aerosol_height_m)
        if not all(math.isfinite(x) and x > 0 for x in positive):
            raise ValueError("invalid atmospheric dimensions")
        if len(self.rayleigh_scattering_per_m) != 3 or not all(math.isfinite(x) and x >= 0 for x in self.rayleigh_scattering_per_m):
            raise ValueError("invalid Rayleigh RGB coefficients")
        if not all(math.isfinite(x) for x in (self.aerosol_scattering_per_m, self.aerosol_extinction_per_m, self.aerosol_g)):
            raise ValueError("nonfinite aerosol parameters")
        if not 0 <= self.aerosol_scattering_per_m <= self.aerosol_extinction_per_m or abs(self.aerosol_g) >= 1:
            raise ValueError("invalid aerosol scattering/extinction/phase")


EARTH = Atmosphere()


def _vector(value):
    result = np.asarray(value, dtype=np.float64)
    if result.shape != (3,) or not np.all(np.isfinite(result)):
        raise ValueError("expected finite XYZ vector")
    return result


def _unit(value):
    result = _vector(value)
    norm = float(np.linalg.norm(result))
    if abs(norm - 1.0) > 1e-9:
        raise ValueError("direction must be unit length")
    return result / norm


def radial_height(position, radius_m=EARTH.radius_m):
    """Stable rationalized height; accepts (..., 3) positions."""
    p = np.asarray(position, dtype=np.float64)
    if p.shape[-1:] != (3,) or not np.all(np.isfinite(p)) or radius_m <= 0:
        raise ValueError("invalid position or radius")
    x, y, z = np.moveaxis(p, -1, 0)
    numerator = x*x + z*z + y*(2*radius_m + y)
    denominator = np.sqrt(x*x + z*z + (radius_m+y)**2) + radius_m
    return numerator / denominator


def ray_sphere_roots(origin, direction, altitude_m=0.0, radius_m=EARTH.radius_m):
    """Ordered signed distances to a shell; None is a geometric miss.

    The constant term avoids subtracting two squared planetary radii. Roundoff
    within eight float64 ulps of the discriminant is conservatively tangent.
    This tolerance is numerical, not a license to thicken the planet in f32.
    """
    p, d = _vector(origin), _unit(direction)
    if not math.isfinite(altitude_m) or radius_m <= 0 or radius_m+altitude_m <= 0:
        raise ValueError("invalid sphere")
    x, y, z = p
    b = x*d[0] + (y+radius_m)*d[1] + z*d[2]
    c = x*x + z*z + (y-altitude_m)*(2*radius_m+y+altitude_m)
    disc = b*b-c
    tolerance = 8*(math.ulp(float(b*b)) + math.ulp(float(c)))
    if disc < -tolerance:
        return None
    root = math.sqrt(max(0.0, disc))
    if root == 0:
        return (-float(b), -float(b))
    q = -b-math.copysign(root, b)
    values = (float(q), float(c/q))
    return tuple(sorted(values))


def trace_terminal(origin, direction, atmosphere=EARTH):
    """Return ok planet/top intersection, or unsupported; never silent clear."""
    try:
        p, d = _vector(origin), _unit(direction)
        h = float(radial_height(p, atmosphere.radius_m))
        if h < -1e-7 or h > atmosphere.top_m+1e-7:
            raise ValueError("origin outside admitted atmosphere")
        radial_dot = p[0]*d[0] + (p[1]+atmosphere.radius_m)*d[1] + p[2]*d[2]
        if h <= 1e-7 and radial_dot <= 0:
            return {"status": "ok", "terminal": "planet", "distance_m": 0.0}
        if h >= atmosphere.top_m-1e-7 and radial_dot >= 0:
            return {"status": "ok", "terminal": "top", "distance_m": 0.0}
        outer = ray_sphere_roots(p, d, atmosphere.top_m, atmosphere.radius_m)
        tops = [t for t in outer or () if t > 1e-7]
        if not tops:
            raise ValueError("no atmosphere exit")
        distance = max(tops)
        ground = ray_sphere_roots(p, d, 0, atmosphere.radius_m)
        hits = [t for t in ground or () if t > 1e-7]
        if hits and min(hits) <= distance:
            return {"status": "ok", "terminal": "planet", "distance_m": min(hits)}
        return {"status": "ok", "terminal": "top", "distance_m": distance}
    except ValueError as error:
        return {"status": "unsupported", "reason": str(error)}


@lru_cache(maxsize=16)
def _gauss(order):
    if order < 2 or order > 2048:
        raise ValueError("quadrature order outside 2..2048")
    return np.polynomial.legendre.leggauss(order)


def coefficients(height_m, atmosphere=EARTH):
    h = np.asarray(height_m, dtype=np.float64)
    if np.any(h < -1e-6) or np.any(h > atmosphere.top_m+1e-6):
        raise ValueError("coefficient query outside admitted atmosphere")
    h = np.maximum(h, 0.0)  # only the accepted sub-micrometre root roundoff
    rayleigh = np.exp(-h/atmosphere.rayleigh_height_m)[..., None] * np.asarray(atmosphere.rayleigh_scattering_per_m)
    aerosol_density = np.exp(-h/atmosphere.aerosol_height_m)[..., None]
    aerosol_s = aerosol_density * atmosphere.aerosol_scattering_per_m
    extinction = rayleigh + aerosol_density*atmosphere.aerosol_extinction_per_m
    return extinction, rayleigh, aerosol_s


def _tau_many(origins, directions, distances, order, atmosphere):
    origins = np.asarray(origins, dtype=np.float64).reshape(-1, 3)
    directions = np.broadcast_to(directions, origins.shape)
    distances = np.asarray(distances, dtype=np.float64).reshape(-1)
    nodes, weights = _gauss(order)
    t = distances[:, None]*(nodes[None, :]+1)/2
    points = origins[:, None, :]+t[..., None]*directions[:, None, :]
    sigma, _, _ = coefficients(radial_height(points, atmosphere.radius_m), atmosphere)
    return distances[:, None]/2 * np.sum(sigma*weights[None, :, None], axis=1)


def air_path(origin, direction, distance_m=None, order=128, atmosphere=EARTH):
    terminal = trace_terminal(origin, direction, atmosphere)
    if terminal["status"] != "ok":
        return terminal
    try:
        p, d = _vector(origin), _unit(direction)
        distance = terminal["distance_m"]
        result_terminal = terminal["terminal"]
        if distance_m is not None:
            if not math.isfinite(distance_m) or distance_m < 0:
                raise ValueError("invalid finite segment length")
            if distance_m < distance:
                distance, result_terminal = distance_m, "finite"
        tau = _tau_many([p], d, [distance], order, atmosphere)[0]
        transmittance = np.exp(-tau)
        return {"status": "ok", "terminal": result_terminal, "distance_m": distance,
                "tau_rgb": tau.tolist(), "air_transmittance_rgb": transmittance.tolist(),
                "source_transmittance_rgb": ([0.0]*3 if result_terminal == "planet" else transmittance.tolist())}
    except ValueError as error:
        return {"status": "unsupported", "reason": str(error)}


def phase_hg(cos_theta, g):
    if not math.isfinite(g) or abs(g) >= 1:
        raise ValueError("HG requires -1 < g < 1")
    mu = np.asarray(cos_theta, dtype=np.float64)
    if np.any(~np.isfinite(mu)) or np.any(np.abs(mu) > 1+1e-12):
        raise ValueError("invalid phase cosine")
    mu = np.clip(mu, -1, 1)
    return (1-g*g)/(4*math.pi*(1+g*g-2*g*mu)**1.5)


def phase_rayleigh(cos_theta):
    mu = np.asarray(cos_theta, dtype=np.float64)
    if np.any(~np.isfinite(mu)) or np.any(np.abs(mu) > 1+1e-12):
        raise ValueError("invalid phase cosine")
    return 3*(1+mu*mu)/(16*math.pi)


def air_direct_radiance(origin, direction, sun_direction, irradiance_rgb=(100000.0,)*3,
                        order=128, sun_order=96, atmosphere=EARTH):
    """Fine direct-only air source integral, no disk/ambient/cloud source.

    Independent Gaussian quadrature evaluates view depth at each source node and
    its Sun-to-top/planet path. Geometry shadow is the spherical planet only.
    """
    terminal = trace_terminal(origin, direction, atmosphere)
    if terminal["status"] != "ok":
        return terminal
    try:
        p, d, sun, e = _vector(origin), _unit(direction), _unit(sun_direction), _vector(irradiance_rgb)
        if np.any(e < 0):
            raise ValueError("negative irradiance")
        nodes, weights = _gauss(order)
        length = terminal["distance_m"]
        t = length*(nodes+1)/2
        positions = p+t[:, None]*d
        tau_view = _tau_many(np.broadcast_to(p, positions.shape), d, t, sun_order, atmosphere)
        sunlight = np.zeros((order, 3))
        paths = [trace_terminal(q, sun, atmosphere) for q in positions]
        if any(path["status"] != "ok" for path in paths):
            raise ValueError("unsupported Sun path in direct-air quadrature")
        indices = [i for i, path in enumerate(paths) if path["status"] == "ok" and path["terminal"] == "top"]
        if indices:
            tau_sun = _tau_many(positions[indices], sun, [paths[i]["distance_m"] for i in indices], sun_order, atmosphere)
            sunlight[indices] = np.exp(-tau_sun)*e
        _, rayleigh, aerosol = coefficients(radial_height(positions, atmosphere.radius_m), atmosphere)
        mu = float(np.dot(d, sun))
        source = sunlight*(rayleigh*phase_rayleigh(mu)+aerosol*phase_hg(mu, atmosphere.aerosol_g))
        radiance = length/2*np.sum(weights[:, None]*np.exp(-tau_view)*source, axis=0)
        return {"status": "ok", "terminal": terminal["terminal"], "distance_m": length,
                "direct_radiance_rgb_cd_m2": radiance.tolist(), "order": order, "sun_order": sun_order}
    except ValueError as error:
        return {"status": "unsupported", "reason": str(error)}


def beer_segment(sigma_t, source_per_m, length_m):
    """Scalar constant-coefficient affine (T,S), including zero extinction."""
    if not all(math.isfinite(x) and x >= 0 for x in (sigma_t, source_per_m, length_m)):
        raise ValueError("negative or nonfinite Beer input")
    transmittance = math.exp(-sigma_t*length_m)
    scatter = source_per_m*(-math.expm1(-sigma_t*length_m))/sigma_t if sigma_t else source_per_m*length_m
    return transmittance, scatter


def compose(near, far):
    return near[0]*far[0], near[1]+near[0]*far[1]


def remaining_bound(transmittance, terminal_max, source_max):
    if not all(math.isfinite(x) and x >= 0 for x in (transmittance, terminal_max, source_max)) or transmittance > 1:
        raise ValueError("invalid bound")
    return transmittance*max(terminal_max, source_max)


def slab_parameters(profile="passive"):
    if profile == "passive":
        return np.asarray(WEIGHTS)*np.asarray(BRANCH_B), np.asarray(BRANCH_B), np.asarray(LIQUID_G)
    if profile == "old_unconditional":
        return .8**np.arange(4), .5**np.arange(4), .86*.5**np.arange(4)
    raise ValueError("unsupported slab profile")


def slab_generated_power(elevation_deg, vertical_tau=math.inf, profile="passive"):
    """All-angle generated scatter / incident horizontal beam power."""
    if not 0 < elevation_deg <= 90 or vertical_tau < 0 or math.isnan(vertical_tau):
        raise ValueError("unsupported slab domain")
    amplitude, b, _ = slab_parameters(profile)
    mu0 = math.sin(math.radians(elevation_deg))
    removed = np.ones_like(b) if math.isinf(vertical_tau) else -np.expm1(-b*vertical_tau/mu0)
    return float(np.sum(amplitude/b*removed))


def slab_upward_flux(elevation_deg, vertical_tau=math.inf, profile="passive", mu_order=128, phi_count=512):
    """Escaping top flux / E_perpendicular*sin(elevation).

    Exact vertical formal solution; Gaussian outgoing cosine quadrature and
    periodic azimuth quadrature. No ambient, air, disk or recursive scattering.
    """
    slab_generated_power(elevation_deg, vertical_tau, profile)  # validate same domain
    if phi_count < 8:
        raise ValueError("insufficient azimuth quadrature")
    amplitude, b, gs = slab_parameters(profile)
    nodes, weights = _gauss(mu_order)
    mu, w = (nodes+1)/2, weights/2
    mu0 = math.sin(math.radians(elevation_deg))
    phi = np.arange(phi_count)*2*math.pi/phi_count
    cos_theta = np.sqrt(1-mu[:, None]**2)*math.sqrt(max(0, 1-mu0*mu0))*np.cos(phi)[None, :]-mu0*mu[:, None]
    total = np.zeros_like(cos_theta)
    for amp, scale, g in zip(amplitude, b, gs):
        finite = 1 if math.isinf(vertical_tau) else -np.expm1(-vertical_tau*(1/mu+scale/mu0))[:, None]
        total += amp*phase_hg(cos_theta, float(g))*mu[:, None]/(mu0+scale*mu[:, None])*finite
    return float(np.sum(w[:, None]*total)*2*math.pi/phi_count)



def slab_downward_flux(elevation_deg, vertical_tau, profile="passive", mu_order=128, phi_count=512):
    """Escaping bottom scattered flux / incident horizontal beam power.

    The finite-depth difference of exponentials is evaluated with expm1; this
    remains finite at equal source/view slopes and at large optical depth.
    """
    slab_generated_power(elevation_deg, vertical_tau, profile)
    if phi_count < 8:
        raise ValueError("insufficient azimuth quadrature")
    if math.isinf(vertical_tau):
        return 0.0
    amplitude, b, gs = slab_parameters(profile)
    nodes, weights = _gauss(mu_order)
    mu, w = (nodes+1)/2, weights/2
    mu0 = math.sin(math.radians(elevation_deg))
    phi = np.arange(phi_count)*2*math.pi/phi_count
    cos_theta = np.sqrt(1-mu[:, None]**2)*math.sqrt(max(0, 1-mu0*mu0))*np.cos(phi)[None, :]+mu0*mu[:, None]
    total = np.zeros_like(cos_theta)
    for amp, scale, g in zip(amplitude, b, gs):
        x, y = scale/mu0, 1/mu
        delta = np.abs(x-y)
        factor = np.empty_like(mu)
        same = delta < 1e-12
        factor[same] = vertical_tau*np.exp(-x*vertical_tau)
        factor[~same] = np.exp(-np.minimum(x, y[~same])*vertical_tau)*(-np.expm1(-delta[~same]*vertical_tau))/delta[~same]
        total += amp*phase_hg(cos_theta, float(g))*factor[:, None]/mu0
    return float(np.sum(w[:, None]*total)*2*math.pi/phi_count)


def sun_direction(elevation_deg):
    angle = math.radians(elevation_deg)
    return [math.cos(angle), math.sin(angle), 0.0]


def oracle_cases():
    geometry = []
    for name, origin, direction in (
        ("ground_up", [0, 0, 0], [0, 1, 0]),
        ("ground_tangent", [0, 0, 0], [1, 0, 0]),
        ("ground_down", [0, 0, 0], [0, -1, 0]),
        ("two_m_horizon", [0, 2, 0], [1, 0, 0]),
        ("two_m_down", [0, 2, 0], [0, -1, 0]),
        ("outside_top", [0, 100001, 0], [0, 1, 0]),
    ):
        geometry.append({"id": name, "origin_m": origin, "direction": direction, **trace_terminal(origin, direction)})
    h = 1536.0
    tangent = [EARTH.radius_m/(EARTH.radius_m+h), -math.sqrt(h*(2*EARTH.radius_m+h))/(EARTH.radius_m+h), 0]
    geometry.append({"id": "1536m_planet_tangent", "origin_m": [0, h, 0], "direction": tangent,
                     "planet_roots_m": ray_sphere_roots([0, h, 0], tangent), **trace_terminal([0, h, 0], tangent)})
    shell_radius = EARTH.radius_m+18000
    shell_origin_height = 18001.0
    shell_offset = 1.0
    tangent_angle = math.asin(math.sqrt(shell_offset*(2*shell_radius+shell_offset))/(shell_radius+shell_offset))
    for suffix, offset_deg in (("tangent", 0), ("miss", 1e-5), ("cross", -1e-5)):
        direction = sun_direction(-math.degrees(tangent_angle)+offset_deg)
        if suffix == "tangent":
            direction = [shell_radius/(shell_radius+shell_offset),
                         -math.sqrt(shell_offset*(2*shell_radius+shell_offset))/(shell_radius+shell_offset), 0]
        origin = [0, shell_origin_height, 0]
        geometry.append({"id": "cloud18km_"+suffix, "origin_m": origin, "direction": direction,
                         "shell_altitude_m": 18000, "shell_roots_m": ray_sphere_roots(origin, direction, 18000),
                         **trace_terminal(origin, direction)})
    air = []
    for height, elevation in ((2, 5), (2, 15), (2, 75), (6000, 5), (18000, 5), (2, 0), (2, -5)):
        origin, direction = [0, height, 0], sun_direction(elevation)
        record = {"id": f"h{height}_e{elevation}", "origin_m": origin, "direction": direction,
                  **air_path(origin, direction)}
        if elevation in (5, 75) and height == 2:
            record["direct_air"] = air_direct_radiance(origin, [0, 1, 0], direction)
        air.append(record)
    tau_cases = []
    for name, sigmas, lengths in (("clear", [0], [1000]), ("one_slab", [.01], [200]),
                                 ("two_layers", [.001, 0, .004], [100, 300, 200]),
                                 ("many_layers", [.002 if i % 2 else 0 for i in range(64)], [25]*64)):
        tau = math.fsum(s*d for s, d in zip(sigmas, lengths))
        tau_cases.append({"id": name, "sigma_t_per_m": sigmas, "lengths_m": lengths,
                          "tau": tau, "transmittance": math.exp(-tau),
                          "branch_terms": [math.exp(-b*tau) for b in BRANCH_B]})
    phase = [{"cos_theta": mu, "rayleigh_per_sr": float(phase_rayleigh(mu)),
              "liquid_hg_per_sr": [float(phase_hg(mu, g)) for g in LIQUID_G],
              "weighted_passive_phase_per_sr": float(sum(w*b*phase_hg(mu, g) for w,b,g in zip(WEIGHTS, BRANCH_B, LIQUID_G)))}
             for mu in (-1, 0, .5, 1)]
    slabs = []
    for elevation in (5, 15, 75):
        for profile in ("passive", "old_unconditional"):
            for tau in (.01, 1.0, math.inf):
                amplitude, branch_b, phase_g = slab_parameters(profile)
                upward = slab_upward_flux(elevation, tau, profile)
                downward = slab_downward_flux(elevation, tau, profile)
                direct = 0.0 if math.isinf(tau) else math.exp(-tau/math.sin(math.radians(elevation)))
                slabs.append({"profile": profile, "source_amplitudes": amplitude.tolist(),
                              "branch_b": branch_b.tolist(), "phase_g": phase_g.tolist(), "elevation_deg": elevation,
                              "vertical_tau": None if math.isinf(tau) else tau,
                              "upward_flux_ratio": upward, "downward_flux_ratio": downward,
                              "unscattered_flux_ratio": direct, "total_escaping_ratio": upward+downward+direct,
                              "generated_power_ratio": slab_generated_power(elevation, tau, profile)})
    return {"schema": SCHEMA, "scope": "independent CPU optical invariants; no family/image/performance pass",
            "fixture": {**asdict(EARTH), "ozone": "off", "frame": "centre=(0,-radius_m,0); Y up; metres",
                        "weights": WEIGHTS, "branch_b": BRANCH_B, "liquid_g": LIQUID_G},
            "geometry_cases": geometry, "air_cases": air, "constant_tau_cases": tau_cases,
            "phase_cases": phase, "slab_cases": slabs,
            "exit_cases": [{"id": "mirror_sun_near_leg", "near_transmittance": 1e-8,
                            "reflected_terminal_max_cd_m2": 2e9, "source_max_cd_m2": 1920000,
                            "remaining_bound_cd_m2": remaining_bound(1e-8, 2e9, 1920000),
                            "incorrect_straight_miss_bound_cd_m2": remaining_bound(1e-8, 20000, 1920000),
                            "allowance_cd_m2": .25}]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--json", action="store_true", help="emit versioned compact comparison cases")
    args = parser.parse_args()
    if args.json:
        print(json.dumps(oracle_cases(), separators=(",", ":"), allow_nan=False))
    else:
        parser.print_help()


if __name__ == "__main__":
    main()
