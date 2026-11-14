"""Independent algebraic checks for CLOUDS design f2418fb348cd, 2026-11-14.

Run with Python's standard library. These are specification counterexamples,
not measurements of engine output or of the accepted library.
"""
import math


def fit(d):
    # Published large-particle fit, also used by first_principles_proof.py.
    return (
        math.exp(-0.0990567 / (d - 1.67154)),
        math.exp(-2.20679 / (d + 3.91029) - 0.428934),
        math.exp(3.62489 - 8.29288 / (d + 5.52825)),
        math.exp(-0.599085 / (d - 0.641583) - 0.665888),
    )


def phase(theta, d, spread):
    gh, gd, alpha, weight = fit(d)
    c = math.cos(theta)
    s2 = math.sin(theta / 2) ** 2
    hg = (1 - gh * gh) / (4 * math.pi * ((1 - gh) ** 2 + gh * (4 * s2 + spread)) ** 1.5)
    dr = (1 - gd * gd) / (4 * math.pi * ((1 - gd) ** 2 + 4 * gd * s2) ** 1.5)
    dr *= (1 + alpha * c * c) / (1 + alpha * (1 + 2 * gd * gd) / 3)
    return (1 - weight) * hg + weight * dr


def quadrature(d, spread, n):
    # theta = u^4 resolves the forward lobe without a uniform-angle grid.
    end = math.pi ** 0.25
    total = 0.0
    for i in range(n):
        u = (i + 0.5) * end / n
        theta = u ** 4
        total += phase(theta, d, spread) * 2 * math.pi * math.sin(theta) * 4 * u ** 3 * end / n
    return total


def exact_integral(d, spread):
    gh, _, _, weight = fit(d)
    a = (1 - gh) ** 2 + gh * spread
    # Integrate HG over cos(theta) from -1 to 1. Draine is normalized.
    hg_integral = (1 - gh * gh) / (2 * gh) * (1 / math.sqrt(a) - 1 / math.sqrt(a + 4 * gh))
    return (1 - weight) * hg_integral + weight


print("A. Sun-disc spreading: the rendered formula versus unit normalization")
spread = 0.004654 ** 2 / 2
for d in (5, 10, 20, 40, 50):
    raw = exact_integral(d, 0)
    exact = exact_integral(d, spread)
    q1, q2 = quadrature(d, spread, 50000), quadrature(d, spread, 100000)
    assert abs(q2 - exact) < 1e-7
    print(f"diameter {d:2d} um: point {raw:.9f}; spread exact {exact:.9f}; quadratures {q1:.9f}, {q2:.9f}; loss {100*(1-exact):.3f}%")


print("\nB. Equal footprints do not determine oblique optical depth")
# Two x columns, four height voxels, each voxel 100 m. Both shapes are
# extruded in the third dimension. Extinction is in m^-1. Every x column
# has the same sum and occupied bottom/top in both shapes.
a = ((.002, .014, .002, .002), (.002, .002, .014, .002))
b = ((.002, .002, .014, .002), (.002, .014, .002, .002))


def footprint(grid):
    return tuple((round(sum(col) * 100, 10), 0, 400) for col in grid)


assert footprint(a) == footprint(b)
print("Footprints (tau, bottom m, top m):", footprint(a))
# A 45-degree Sun ray x = height - 100: x in [0,100) traverses height
# voxel 1; x in [100,200) traverses height voxel 2. Outside x=[0,200]
# density is zero. Exact integration of the piecewise-constant voxels.
for name, grid in (("A", a), ("B", b)):
    tau = 100 * math.sqrt(2) * (grid[0][1] + grid[1][2])
    print(f"shape {name}: oblique tau {tau:.9f}, T {math.exp(-tau):.9f}")
print("A footprint-only shadow path receives identical inputs for A and B, but their volume Sun paths differ.")
print("The distinction persists with shared constant erosion and can be smoothed; detail-noise omission is not its cause.")
