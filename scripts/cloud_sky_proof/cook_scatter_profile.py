"""CPU cook of the selected empirical cloud source bank; no GPU/runtime code.

The reference solves homogeneous slabs with ordinary view extinction. Generated
scattering-event power may exceed incident power; escaped flux is distinct.
A finite table is an empirical approximation, never a uniform energy certificate.
NumPy only; arrays are row-major [mu0, physical vertical optical thickness T].
"""
from __future__ import annotations

import argparse
from dataclasses import dataclass
from functools import lru_cache
import hashlib
import json
import math
from pathlib import Path
import resource
import sys
import time

import numpy as np

TOOL_VERSION = "1.0.0"
SCHEMA = "thrum.cloud-scatter-fit.v1"
CLEAR_T = 1e-4
MAX_T = 1000.0
A_BOUND = 3.724
MAX_RSS = 256 * 1024 * 1024


@dataclass(frozen=True)
class ProfileParameters:
    g: float = .86
    omega: float = 1.0
    kappa: float = .25
    rho: float = .8

    def __post_init__(self):
        if not all(math.isfinite(x) for x in (self.g, self.omega, self.kappa, self.rho)):
            raise ValueError("nonfinite profile parameter")
        if self.g not in (.86, .75):
            raise ValueError("only keyed HG(.86) liquid and HG(.75) ice candidates are admitted")
        if self.omega != 1 or self.kappa != .25 or self.rho != .8:
            raise ValueError("this cook admits omega=1, kappa=.25, rho=.8 only")


@dataclass
class ProfileTable:
    parameters: ProfileParameters
    t_axis: np.ndarray
    mu_axis: np.ndarray
    coefficients: np.ndarray
    metadata: dict


@lru_cache(maxsize=8)
def _cosine_rule(count):
    if not isinstance(count, int) or not 8 <= count <= 1024:
        raise ValueError("cosine quadrature count must be 8..1024")
    nodes, weights = np.polynomial.legendre.leggauss(count)
    return (nodes+1)/2, weights/2


def phase_hg(cosine, g):
    c = np.asarray(cosine, dtype=np.float64)
    if not math.isfinite(g) or abs(g) >= 1 or np.any(~np.isfinite(c)) or np.any(np.abs(c) > 1+1e-12):
        raise ValueError("invalid HG domain")
    c = np.clip(c, -1, 1)
    return (1-g*g)/(4*math.pi*(1+g*g-2*g*c)**1.5)


@lru_cache(maxsize=384)
def _marginals(mu0, g, cosine_count, phi_count):
    if not math.isfinite(mu0) or not 0 <= mu0 <= 1:
        raise ValueError("mu0 must be in 0..1")
    if not isinstance(phi_count, int) or not 16 <= phi_count <= 4096:
        raise ValueError("azimuth count must be 16..4096")
    mu, weights = _cosine_rule(cosine_count)
    phi = np.arange(phi_count)*2*math.pi/phi_count
    horizontal = np.sqrt(1-mu[:, None]**2)*math.sqrt(max(0, 1-mu0*mu0))*np.cos(phi)[None, :]
    # Include the azimuth integral, leaving only outgoing-cosine quadrature.
    up = np.mean(phase_hg(horizontal-mu0*mu[:, None], g), axis=1)*2*math.pi
    down = np.mean(phase_hg(horizontal+mu0*mu[:, None], g), axis=1)*2*math.pi
    return mu, weights, up, down


def difference_integral(length, slope_a, slope_b):
    """(exp(-a*T)-exp(-b*T))/(b-a), including equal positive slopes."""
    length, a, b = np.broadcast_arrays(np.asarray(length, float), np.asarray(slope_a, float), np.asarray(slope_b, float))
    if np.any(~np.isfinite(length)) or np.any(length < 0) or np.any(~np.isfinite(a)) or np.any(a <= 0) or np.any(~np.isfinite(b)) or np.any(b <= 0):
        raise ValueError("finite nonnegative length and positive finite slopes required")
    delta = np.abs(a-b)
    factor = np.empty_like(delta)
    unequal = delta > 0
    factor[unequal] = -np.expm1(-delta[unequal]*length[unequal])/delta[unequal]
    factor[~unequal] = length[~unequal]
    return np.exp(-np.minimum(a, b)*length)*factor


def _validate_thickness(thickness):
    value = np.asarray(thickness, dtype=np.float64)
    if np.any(np.isnan(value)) or np.any(value < 0):
        raise ValueError("T must be nonnegative; positive infinity denotes semi-infinite")
    return value


def flux_components(thickness, mu0, parameters=ProfileParameters(), cosine_count=256, phi_count=1024):
    """Pure scalar/vector F0,F1 and formal-solution fluxes, no fitted A.

    F0 includes direct transmission and primary up/down. F1 is the unit-bank
    up/down sum. At mu0=0 use the finite-slab grazing limit, including bottom
    photons that survive the ordinary view path. At T=0 define clear explicitly.
    """
    original = _validate_thickness(thickness)
    flat = original.reshape(-1)
    mu, weights, p_up, p_down = _marginals(float(mu0), parameters.g, cosine_count, phi_count)
    finite = np.isfinite(flat)
    t = flat[finite, None]
    results = {}
    for name, b, pu, pd in (("primary", 1., p_up, p_down), ("bank", parameters.kappa, np.full_like(mu, .5), np.full_like(mu, .5))):
        up = np.empty_like(flat)
        down = np.zeros_like(flat)
        if mu0 == 0:
            up[:] = .5/b
            down[finite] = np.sum(np.exp(-t/mu[None, :])*(weights*pd/b)[None, :], axis=1)
        else:
            up[finite] = np.sum(-np.expm1(-t*(1/mu+b/mu0)[None, :])*(weights*pu*mu/(mu0+b*mu))[None, :], axis=1)
            down[finite] = np.sum(difference_integral(t, b/mu0, 1/mu[None, :])*(weights*pd/mu0)[None, :], axis=1)
            up[~finite] = np.sum(weights*pu*mu/(mu0+b*mu))
        up[flat == 0] = 0
        down[flat == 0] = 0
        results[name+"_up"] = up
        results[name+"_down"] = down
    direct = np.zeros_like(flat)
    if mu0 > 0:
        direct[finite] = np.exp(-flat[finite]/mu0)
    direct[flat == 0] = 1
    results["direct"] = direct
    results["F0"] = direct+results["primary_up"]+results["primary_down"]
    results["F1"] = results["bank_up"]+results["bank_down"]
    return {name: (float(value[0]) if original.ndim == 0 else value.reshape(original.shape)) for name, value in results.items()}


def infinity_amplitude(mu0, parameters=ProfileParameters(), cosine_count=256, phi_count=1024):
    r0 = flux_components(math.inf, mu0, parameters, cosine_count, phi_count)["primary_up"]
    k = parameters.kappa
    r1 = .5/k if mu0 == 0 else .5*(1/k-mu0/k**2*math.log1p(k/mu0))
    return max(0., (parameters.rho-r0)/r1)


def fit_coefficient(thickness, mu0, parameters=ProfileParameters(), cosine_count=256, phi_count=1024):
    """Exact written cap formula at finite quadrature; never normalize source output."""
    t = _validate_thickness(thickness)
    flux = flux_components(t, mu0, parameters, cosine_count, phi_count)
    f0, f1 = np.asarray(flux["F0"]), np.asarray(flux["F1"])
    if np.any(~np.isfinite(f0)) or np.any(~np.isfinite(f1)) or np.any(f0 > 1+2e-7) or np.any(f1 < 0):
        raise ValueError("invalid reference flux; cannot mask failed quadrature")
    cap = np.zeros_like(t)
    np.divide(np.maximum(0, 1-f0), f1, out=cap, where=f1 > 0)
    gate = -np.expm1(-t)
    value = np.minimum(gate*gate*infinity_amplitude(mu0, parameters, cosine_count, phi_count), cap)
    value = np.where(t < CLEAR_T, 0., value)
    if np.any(~np.isfinite(value)) or np.any(value < 0) or np.any(value >= A_BOUND):
        raise ValueError("coefficient violates hard source bound")
    return float(value) if value.ndim == 0 else value


def source_factor(cos_theta, tau_all, coefficient, parameters=ProfileParameters()):
    if not math.isfinite(tau_all) or tau_all < 0 or not math.isfinite(coefficient) or not 0 <= coefficient < A_BOUND:
        raise ValueError("invalid source input")
    return float(phase_hg(cos_theta, parameters.g))*math.exp(-tau_all)+coefficient/(4*math.pi)*math.exp(-parameters.kappa*tau_all)


def make_axes(mu_count=64, t_count=128):
    if not isinstance(mu_count, int) or not isinstance(t_count, int) or not 2 <= mu_count <= 256 or not 3 <= t_count <= 256:
        raise ValueError("axis counts outside mu=2..256/T=3..256")
    t = np.concatenate(([0.], np.geomspace(CLEAR_T, MAX_T, t_count-1))).astype(np.float32)
    mu = np.linspace(0, 1, mu_count).astype(np.float32)
    if not np.all(np.diff(t) > 0) or not np.all(np.diff(mu) > 0):
        raise ValueError("float32 axes must remain strictly increasing")
    return t, mu


def _validate_axes(t_axis, mu_axis):
    t, mu = np.asarray(t_axis), np.asarray(mu_axis)
    if t.ndim != 1 or mu.ndim != 1 or not 3 <= len(t) <= 256 or not 2 <= len(mu) <= 256:
        raise ValueError("invalid axis dimensions")
    if t.dtype != np.float32 or mu.dtype != np.float32 or not np.all(np.isfinite(t)) or not np.all(np.isfinite(mu)):
        raise ValueError("axes must be exact finite float32 data")
    if t[0] != 0 or t[-1] != MAX_T or mu[0] != 0 or mu[-1] != 1 or not np.all(np.diff(t) > 0) or not np.all(np.diff(mu) > 0):
        raise ValueError("invalid axis bounds/order")


def _validate_table(table):
    _validate_axes(table.t_axis, table.mu_axis)
    values = np.asarray(table.coefficients)
    if values.dtype != np.float32 or values.shape != (len(table.mu_axis), len(table.t_axis)):
        raise ValueError("invalid float32 [mu,T] table layout")
    if not np.all(np.isfinite(values)) or np.any(values < 0) or np.any(values >= A_BOUND):
        raise ValueError("invalid table coefficient/source bound")


def build_table(parameters, t_axis, mu_axis, cosine_count=256, phi_count=1024):
    _validate_axes(t_axis, mu_axis)
    values = np.empty((len(mu_axis), len(t_axis)), dtype=np.float32)
    for row, mu in enumerate(mu_axis):
        values[row] = fit_coefficient(t_axis.astype(float), float(mu), parameters, cosine_count, phi_count)
        # The final physical-T column is also the explicit upper-infinity row.
        values[row, -1] = infinity_amplitude(float(mu), parameters, cosine_count, phi_count)
    if not np.all(np.isfinite(values)) or np.min(values) < 0 or np.max(values) >= A_BOUND:
        raise ValueError("float32 table violates source bound")
    return ProfileTable(parameters, np.asarray(t_axis, np.float32), np.asarray(mu_axis, np.float32), values, {})


def sample_table(table, thickness, mu0):
    """Manual physical-axis interpolation in f64 of exact f32 data.

    GPU f32 arithmetic and rounded world-column errors remain independent probes.
    """
    t, m = np.broadcast_arrays(_validate_thickness(thickness), np.asarray(mu0, float))
    if np.any(~np.isfinite(m)) or np.any(m < 0) or np.any(m > 1):
        raise ValueError("lookup mu0 outside 0..1")
    tc = np.minimum(t, float(table.t_axis[-1]))
    ti = np.clip(np.searchsorted(table.t_axis, tc, side="right")-1, 0, len(table.t_axis)-2)
    mi = np.clip(np.searchsorted(table.mu_axis, m, side="right")-1, 0, len(table.mu_axis)-2)
    ft = (tc-table.t_axis[ti].astype(float))/(table.t_axis[ti+1].astype(float)-table.t_axis[ti].astype(float))
    fm = (m-table.mu_axis[mi].astype(float))/(table.mu_axis[mi+1].astype(float)-table.mu_axis[mi].astype(float))
    a = table.coefficients
    value = (1-fm)*((1-ft)*a[mi, ti]+ft*a[mi, ti+1])+fm*((1-ft)*a[mi+1, ti]+ft*a[mi+1, ti+1])
    value = np.where(t < CLEAR_T, 0., value)
    if np.any(~np.isfinite(value)) or np.any(value < 0) or np.any(value >= A_BOUND):
        raise ValueError("invalid sampled coefficient")
    return float(value) if value.ndim == 0 else value


def assess_table(table, cosine_count=256, phi_count=1024):
    """Repeatable off-grid and named slab witnesses; summary only, not certificate."""
    t = np.unique(np.concatenate((np.array([0, 1e-7, CLEAR_T, .001, .01, .1, .25, .5, 1, 2, 2.4537511066, 3, 5, 10, 30, 100, 999, 1000, 1001, math.inf]),
                                 (table.t_axis[:-1].astype(float)+table.t_axis[1:].astype(float))/2)))
    m = np.unique(np.concatenate((np.array([0, 1e-5, .001, .0075, *[math.sin(math.radians(a)) for a in (2, 5, 15, 30, 75, 90)]]),
                                 (table.mu_axis[:-1].astype(float)+table.mu_axis[1:].astype(float))/2)))
    errors, signed, excess = [], [], []
    worst = None
    named = []
    for mu in m:
        expected = fit_coefficient(t, float(mu), table.parameters, cosine_count, phi_count)
        actual = sample_table(table, t, float(mu))
        flux = flux_components(t, float(mu), table.parameters, cosine_count, phi_count)
        escaped = flux["F0"]+actual*flux["F1"]
        err = actual-expected
        errors.extend(np.abs(err).tolist())
        signed.extend(err.tolist())
        excess.extend(np.maximum(0, escaped-1).tolist())
        index = int(np.argmax(np.abs(err)))
        if worst is None or abs(err[index]) > worst["absolute_A_error"]:
            worst = {"T": None if math.isinf(t[index]) else float(t[index]), "mu0": float(mu),
                     "expected_A": float(expected[index]), "actual_A": float(actual[index]),
                     "absolute_A_error": float(abs(err[index])), "escaped_flux_ratio": float(escaped[index])}
    for degrees in (2, 5, 15, 30, 75, 90):
        mu = math.sin(math.radians(degrees))
        for thickness in (0., 1e-4, .01, .1, 1., 2., 5., 30., math.inf):
            a = sample_table(table, thickness, mu)
            f = flux_components(thickness, mu, table.parameters, cosine_count, phi_count)
            generated = 1+a/table.parameters.kappa if math.isinf(thickness) else -math.expm1(-thickness/mu)+a/table.parameters.kappa*(-math.expm1(-table.parameters.kappa*thickness/mu))
            named.append({"sun_elevation_deg": degrees, "T": None if math.isinf(thickness) else thickness,
                          "A": a, "escaped_flux_ratio": f["F0"]+a*f["F1"], "generated_event_power_ratio": generated})
    return {"sample_count": len(errors), "max_absolute_A_error": max(errors),
            "mean_absolute_A_error": float(np.mean(errors)), "max_positive_A_error": max(signed),
            "max_escaped_flux_excess": max(excess), "worst_A_case": worst,
            "named_flux_cases": named, "uniform_error_bound": "unproven"}


def _node_energy(table, cosine_count=256, phi_count=1024):
    excess = 0.
    for row, mu in enumerate(table.mu_axis):
        f = flux_components(table.t_axis.astype(float), float(mu), table.parameters, cosine_count, phi_count)
        excess = max(excess, float(np.max(f["F0"]+table.coefficients[row]*f["F1"]-1)))
    if excess > 2e-6:
        raise ValueError("stored nodes fail finite-slab cap beyond rounding tolerance")
    return excess


def _flux_convergence(table):
    maxima = {"max_F0_difference": 0., "max_F1_difference": 0., "max_unrounded_A_difference": 0.}
    t = table.t_axis.astype(float)
    for mu in table.mu_axis:
        low = flux_components(t, float(mu), table.parameters, 128, 512)
        high = flux_components(t, float(mu), table.parameters, 256, 1024)
        for name in ("F0", "F1"):
            key = "max_"+name+"_difference"
            maxima[key] = max(maxima[key], float(np.max(np.abs(low[name]-high[name]))))
        delta = np.abs(fit_coefficient(t, float(mu), table.parameters, 128, 512)-fit_coefficient(t, float(mu), table.parameters, 256, 1024))
        maxima["max_unrounded_A_difference"] = max(maxima["max_unrounded_A_difference"], float(np.max(delta)))
    return maxima


def cook_profile(mu_count=64, t_count=128, parameters=ProfileParameters(), refine=True):
    started = time.perf_counter()
    attempts = []
    while True:
        t_axis, mu_axis = make_axes(mu_count, t_count)
        coarse = build_table(parameters, t_axis, mu_axis, 128, 512)
        selected = build_table(parameters, t_axis, mu_axis, 256, 1024)
        assessment = assess_table(selected)
        attempts.append({"mu_count": mu_count, "t_count": t_count,
                         "max_absolute_A_error": assessment["max_absolute_A_error"],
                         "max_escaped_flux_excess": assessment["max_escaped_flux_excess"]})
        # These are bounded table-selection heuristics, not requirements for art.
        acceptable = assessment["max_absolute_A_error"] <= .02 and assessment["max_escaped_flux_excess"] <= .01
        if acceptable or not refine or (mu_count == 256 and t_count == 256):
            break
        mu_count, t_count = min(256, 2*mu_count), min(256, 2*t_count)
    comparison = np.abs(coarse.coefficients.astype(float)-selected.coefficients.astype(float))
    node_excess = _node_energy(selected)
    flux_convergence = _flux_convergence(selected)
    selected.metadata = {"quadrature": {"selected": {"cosine_nodes": 256, "azimuth_samples": 1024},
                         "comparison": {"cosine_nodes": 128, "azimuth_samples": 512},
                         "max_node_A_difference": float(np.max(comparison)),
                         "mean_node_A_difference": float(np.mean(comparison)),
                         "max_node_escaped_flux_excess": node_excess, **flux_convergence},
                         "empirical": assessment,
                         "selection": {"attempts": attempts, "heuristic_A_error": .02, "heuristic_flux_excess": .01,
                                       "heuristics_met": acceptable, "max_axis_count": 256,
                                       "failure_to_meet_heuristics_does_not_block_labelled_appearance_comparison": True},
                         "elapsed_seconds": time.perf_counter()-started}
    return selected


def _sha(data):
    return hashlib.sha256(data).hexdigest()


def write_package(table, out_dir):
    """Write complete known package files, publishing JSON last; preserve other files."""
    _validate_table(table)
    out = Path(out_dir)
    out.mkdir(parents=True, exist_ok=True)
    a_bytes = np.asarray(table.coefficients, dtype="<f4").tobytes(order="C")
    axis_bytes = np.concatenate((table.t_axis, table.mu_axis)).astype("<f4").tobytes()
    tool_bytes = Path(__file__).read_bytes()
    rss = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    rss_bytes = int(rss if sys.platform == "darwin" else rss*1024)
    if rss_bytes >= MAX_RSS:
        raise MemoryError("cook exceeded 256 MiB process peak")
    document = {"schema": SCHEMA, "tool": {"name": "cook_scatter_profile.py", "version": TOOL_VERSION, "sha256": _sha(tool_bytes), "python": sys.version.split()[0], "numpy": np.__version__, "elapsed_clock": "perf_counter wall seconds, cook/refinement/convergence only"},
                "parameters": {"phase": "normalized-HG", "g": table.parameters.g, "omega": 1., "kappa": .25, "rho_opaque_art_target": .8},
                "profile_role": "liquid comparison candidate" if table.parameters.g == .86 else "ice candidate; not activated",
                "counts": {"mu": len(table.mu_axis), "T": len(table.t_axis)},
                "axes": {"T": table.t_axis.astype(float).tolist(), "mu": table.mu_axis.astype(float).tolist()},
                "layout": {"A": "little-endian float32, row-major [mu,T]", "axes": "little-endian float32: all T, then all mu"},
                "files": {"A": {"filename": "A.f32", "bytes": len(a_bytes), "sha256": _sha(a_bytes)},
                          "axes": {"filename": "axes.f32", "bytes": len(axis_bytes), "sha256": _sha(axis_bytes)}},
                "hard_bounds": {"A_exclusive_max": A_BOUND, "actual_A_min": float(np.min(table.coefficients)),
                                "actual_A_max": float(np.max(table.coefficients)), "source_ratio_max_cd_m2": 2_000_000,
                                "phase_peak_max_per_sr": 8., "directional_RGB_irradiance_max_lux": 120000., "external_ambient_max_cd_m2": 960000.},
                "evaluation": {"T_below": CLEAR_T, "T_below_result": 0., "T_above": MAX_T,
                               "T_above_result": "interpolate final T-column A[mu,T_count-1] (semi-infinite)",
                               "interpolation": "textureLoad/manual bilinear; fractions from actual physical T and mu axes, not log(T)",
                               "reference_arithmetic": "float64 lookup of exact float32 axes/values; WGSL arithmetic separately unproven",
                               "body_descriptor": "world-owner-up integral of transformed owner extinction; external product/generation",
                               "phase_dispatch": "ice HG(.75) primary-only until separate bank qualification; no implicit liquid-table reuse"},
                "empirical_certificate": False, "generated_event_power_may_exceed_incident": True,
                "limits": {"process_peak_cap_bytes": MAX_RSS, "measured_process_peak_bytes": rss_bytes,
                           "appearance": "unproven", "GPU": "not run", "uniform_escaped_energy_certificate": "unproven"},
                **table.metadata}
    data = json.dumps(document, indent=2, allow_nan=False).encode()+b"\n"
    if len(data)+len(a_bytes)+len(axis_bytes) > 2*1024*1024:
        raise ValueError("unexpected package size")
    for name, payload in (("A.f32", a_bytes), ("axes.f32", axis_bytes), ("profile.json", data)):
        pending = out/(name+".pending")
        pending.write_bytes(payload)
        pending.replace(out/name)
    return document


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out-dir", required=True)
    parser.add_argument("--mu-count", type=int, default=64)
    parser.add_argument("--t-count", type=int, default=128)
    parser.add_argument("--g", type=float, default=.86)
    parser.add_argument("--omega", type=float, default=1.)
    parser.add_argument("--no-refine", action="store_true", help="retain requested grid; still record empirical errors")
    args = parser.parse_args()
    try:
        parameters = ProfileParameters(g=args.g, omega=args.omega)
        table = cook_profile(args.mu_count, args.t_count, parameters, not args.no_refine)
        doc = write_package(table, args.out_dir)
    except (ValueError, MemoryError) as error:
        parser.error(str(error))
    print(json.dumps({"schema": SCHEMA, "out_dir": str(Path(args.out_dir).resolve()), "counts": doc["counts"],
                      "bytes": {k: v["bytes"] for k,v in doc["files"].items()},
                      "elapsed_seconds": doc["elapsed_seconds"], "process_peak_bytes": doc["limits"]["measured_process_peak_bytes"],
                      "max_A_error": doc["empirical"]["max_absolute_A_error"],
                      "max_escaped_flux_excess": doc["empirical"]["max_escaped_flux_excess"], "empirical_certificate": False}))


if __name__ == "__main__":
    main()
