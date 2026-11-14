"""Reproductions for the first-principles revision of the CLOUDS design, 14 November 2026.

Standard library only. Run: python3 first_principles_proof.py > first-principles-proof.txt

Part E. The first scattering order's phase: the retained reference's Henyey-Greenstein lobe
        against the droplet's own phase (Jendersie and d'Eon 2023, HG + Draine), at the angles
        where a silver lining is seen.
Part F. Drift shown at the committed tick against drift shown at the display's time, in pixels,
        on a 120 Hz display fed by a 60 Hz tick.
"""
import math

PI = math.pi
WEIGHTS = (0.7, 0.2, 0.07, 0.03)          # scripts/reference/cloud/optical_reference.py
LIQUID_G = (0.86, 0.43, 0.215, 0.1075)
SUN_RADIUS = 0.004654                      # rad
SPREAD = SUN_RADIUS ** 2 / 2               # a uniform disc widens the squared angle by rho^2/2


def hg(s2, g, spread=0.0):
    base = (1 - g) ** 2 + g * (4 * s2 + spread)
    return (1 - g * g) / (4 * PI * base ** 1.5)


def draine(s2, g, alpha):
    c = 1 - 2 * s2
    return hg(s2, g) * (1 + alpha * c * c) / (1 + alpha * (1 + 2 * g * g) / 3)


def fit(d_um):
    """Jendersie and d'Eon 2023, the fits for 5 <= d <= 50 micrometres."""
    return (math.exp(-0.0990567 / (d_um - 1.67154)),
            math.exp(-2.20679 / (d_um + 3.91029) - 0.428934),
            math.exp(3.62489 - 8.29288 / (d_um + 5.52825)),
            math.exp(-0.599085 / (d_um - 0.641583) - 0.665888))


def droplet(s2, d_um, spread=SPREAD):
    g_hg, g_d, alpha, w = fit(d_um)
    return (1 - w) * hg(s2, g_hg, spread) + w * draine(s2, g_d, alpha)


def s2_of(deg):
    return math.sin(math.radians(deg) / 2) ** 2


def integral(phase, n=400000):
    """Integral over the sphere and the mean cosine, by the midpoint rule in u = theta^(1/4)."""
    total = mean = 0.0
    top = PI ** 0.25
    for i in range(n):
        u = (i + 0.5) / n * top
        theta = u ** 4
        dtheta = 4 * u ** 3 * top / n
        p = phase(math.sin(theta / 2) ** 2)
        w = 2 * PI * math.sin(theta) * dtheta
        total += p * w
        mean += p * math.cos(theta) * w
    return total, mean / total


def part_e():
    print("Part E. First-order phase, sr^-1, at scattering angle theta (0 = toward the Sun)")
    print("  reference = HG(0.86); droplet = HG + Draine for a 20 um droplet, averaged over the Sun's disc")
    print("  mix = the four orders as a thin edge returns them: 0.7 * first + 0.2 HG(.43) + 0.07 HG(.215) + 0.03 HG(.1075)")
    print("  theta   reference    droplet   droplet/reference   mix(reference)   mix(droplet)")
    rows = {}
    for deg in (0.5, 2, 5, 10, 20, 45, 90, 140, 180):
        s2 = s2_of(deg)
        ref = hg(s2, LIQUID_G[0])
        dro = droplet(s2, 20.0)
        later = sum(w * hg(s2, g) for w, g in zip(WEIGHTS[1:], LIQUID_G[1:]))
        rows[deg] = (ref, dro, WEIGHTS[0] * ref + later, WEIGHTS[0] * dro + later)
        print(f"  {deg:5.1f} {ref:11.4f} {dro:10.4f} {dro / ref:13.2f} {rows[deg][2]:16.4f} {rows[deg][3]:14.4f}")
    print("  Rim against body: the thin edge 5 degrees from the Sun over the same edge at 45 degrees")
    print(f"    reference mix {rows[5][2] / rows[45][2]:.1f} times; droplet mix {rows[5][3] / rows[45][3]:.1f} times")
    for name, d in (("10 um", 10.0), ("20 um", 20.0), ("40 um", 40.0)):
        total, mean = integral(lambda s2, d=d: droplet(s2, d, 0.0))
        print(f"  droplet {name}: integral over the sphere {total:.4f}, mean cosine {mean:.4f}")
    total, mean = integral(lambda s2: hg(s2, LIQUID_G[0]))
    print(f"  reference HG(0.86): integral {total:.4f}, mean cosine {mean:.4f}")
    peak = droplet(0.0, 20.0, 0.0)
    print(f"  forward peak of the 20 um droplet: {peak:.0f} sr^-1 as a point Sun, {droplet(0.0, 20.0):.0f} averaged over the disc")
    print()


def part_f():
    print("Part F. Drift of a cloud 1,500 m away, across the view, in pixels of a 1440-row frame with a 60 degree field")
    print("  The tick is 60 Hz and the display 120 Hz, so each tick is shown twice.")
    px_per_rad = 1440 / math.radians(60)
    print("  wind  time-lapse   metres a tick   at the tick: steps, px   at the display's time: steps, px")
    for wind in (10.0, 40.0):
        for rate in (1, 8, 60):
            per_tick = wind * rate / 60
            px = per_tick / 1500 * px_per_rad
            print(f"  {wind:4.0f} {rate:8d}x {per_tick:14.2f}        0 then {px:6.2f}              {px / 2:5.2f} then {px / 2:5.2f}")
    print("  Evolution at the tick, 60x: a cell that lives 12 minutes changes its envelope by at most")
    life = 12 * 60.0
    print(f"    {PI / 2 / life * 100:.2f}% of its range in one tick of 1 s of weather (a raised-cosine envelope).")
    print()


if __name__ == "__main__":
    part_e()
    part_f()
