"""Reviewer check of the proposed phase table, not an engine/GPU test.

Run from the repository root with python3 -B. Reuses the planner's disc
quadrature after inspecting it; independently integrates an f32 table lookup
with composite Simpson quadrature, including f32 coordinate and lerp rounding.
The expected normalization and first moment follow from spherical convolution.
"""
import importlib.util
import math
from pathlib import Path
import struct

source = Path(__file__).resolve().parents[1] / "design-proof/second_revision_proof.py"
spec = importlib.util.spec_from_file_location("cloud_proof", source)
proof = importlib.util.module_from_spec(spec)
spec.loader.exec_module(proof)


def f32(x):
    return struct.unpack("f", struct.pack("f", x))[0]


def lookup(table, u):
    coord = f32(f32(u) * (len(table) - 1))
    index = min(int(coord), len(table) - 2)
    fraction = f32(coord - index)
    return f32(table[index] + f32(f32(table[index + 1] - table[index]) * fraction))


def integrate(table, n):
    energy = moment = 0.0
    for i in range(n + 1):
        u = i / n
        weight = 1 if i in (0, n) else (4 if i % 2 else 2)
        value = lookup(table, u) * 16 * math.pi * u ** 3
        energy += weight * value
        moment += weight * value * (1 - 2 * u ** 4)
    return energy / (3 * n), moment / energy


tables = []
print("1025 f32 entries, coordinate u = sqrt(sin(theta/2)); simulated f32 linear read")
for diameter in (5, 20, 50):
    params = proof.fit(diameter)
    table = [f32(proof.disc_phase(2 * math.asin((i / 1024) ** 2), params))
             for i in range(1025)]
    tables.append(table)
    expected_mean = proof.point_mean_cosine(params) * (1 + math.cos(proof.RHO)) / 2
    results = [integrate(table, n) for n in (16384, 32768)]
    errors = []
    for degrees in (0, 0.25, 0.5, 2):
        theta = math.radians(degrees)
        reference = proof.disc_phase(theta, params)
        errors.append(abs(lookup(table, math.sqrt(math.sin(theta / 2))) / reference - 1))
    print(f"{diameter:2d} um: integrals {results[0][0]:.9f}, {results[1][0]:.9f}; "
          f"mean {results[1][1]:.9f} vs {expected_mean:.9f}; "
          f"max relative probe error {max(errors):.6g}")
    assert abs(results[1][0] - 1) < 0.0002
    assert abs(results[1][1] - expected_mean) < 0.0001
    assert max(errors) < 0.002

mixed = [f32(0.37 * a + 0.63 * b) for a, b in zip(tables[0], tables[2])]
print(f"37% 5 um + 63% 50 um: integral {integrate(mixed, 32768)[0]:.9f}")
assert abs(integrate(mixed, 32768)[0] - 1) < 0.0002
print("PASS: table accuracy is feasible in simulated f32; actual shader remains M1/M5 proof.")
