"""WATERS design, 14 November 2026: two reproductions, parts H and I (A to G are in ../../SEA/design-proof).
Pure Python, no dependencies.

Part H  Waves on water of any depth from a few uniform-depth operators blended per cell (depth bins):
        the speed error and the false damping of the blend, for bins a factor of two apart.
Part I  How much of each layer of an open sea's wave field a sheltered body takes: band gains for a
        fetch-limited spectrum, and the depth limit near a shore.

Run: python3 water_proof.py
"""
import cmath
import math

G = 9.81
TICK = 1.0 / 60.0


def omega(k, d):
    return math.sqrt(G * k * math.tanh(k * d))


def part_h():
    print("Part H: depth bins a factor of two apart, weights linear in log depth, one 60 Hz tick")
    print("  wavelength   worst speed error   worst false damping per second   at depth")
    bins = [0.125 * 2 ** i for i in range(12)]          # 0.125 m .. 256 m
    worst_all = (0.0, 0.0)
    for lam in (0.5, 1.0, 2.0, 4.0, 12.0, 30.0, 100.0):
        k = 2 * math.pi / lam
        worst = (0.0, 0.0, 0.0)
        for i in range(len(bins) - 1):
            d1, d2 = bins[i], bins[i + 1]
            for s in range(1, 20):
                w2 = s / 20.0
                d = d1 * (d2 / d1) ** w2                 # linear in log depth
                a = (1 - w2) * cmath.exp(-1j * omega(k, d1) * TICK) + w2 * cmath.exp(-1j * omega(k, d2) * TICK)
                w_eff = -cmath.phase(a) / TICK
                speed = abs(w_eff / omega(k, d) - 1)
                damp = 1 - abs(a) ** 60
                if speed > worst[0]:
                    worst = (speed, max(worst[1], damp), d)
                elif damp > worst[1]:
                    worst = (worst[0], damp, worst[2])
        worst_all = (max(worst_all[0], worst[0]), max(worst_all[1], worst[1]))
        print(f"  {lam:7.1f} m   {100 * worst[0]:8.2f} %          {100 * worst[1]:10.4f} %                  {worst[2]:.2f} m")
    print(f"  worst over all: speed {100 * worst_all[0]:.2f} %, damping {100 * worst_all[1]:.4f} % a second")
    print("  one operator for the whole window at its mean depth, a pond from 0.25 m to 2 m deep, 2 m wave:")
    k = 2 * math.pi / 2.0
    mean = 1.0
    for d in (0.25, 0.5, 2.0):
        print(f"    at {d:.2f} m the wave travels {100 * (omega(k, mean) / omega(k, d) - 1):+.0f} % off its true speed")


def jonswap(f, fp, alpha=1.0, gamma=3.3):
    s = 0.07 if f <= fp else 0.09
    r = math.exp(-((f - fp) ** 2) / (2 * s * s * fp * fp))
    return alpha * G * G * (2 * math.pi) ** -4 * f ** -5 * math.exp(-1.25 * (fp / f) ** 4) * gamma ** r


def band_variance(fp, hs, lam_hi, lam_lo, n=4000):
    """Variance between two wavelengths for a JONSWAP shape scaled to a significant height."""
    def integral(f0, f1):
        tot = 0.0
        for i in range(n):
            f = f0 + (i + 0.5) * (f1 - f0) / n
            tot += jonswap(f, fp) * (f1 - f0) / n
        return tot
    f_of = lambda lam: math.sqrt(G / (2 * math.pi * lam))
    whole = integral(0.3 * fp, 12.0)
    scale = (hs / 4.0) ** 2 / whole
    lo_f, hi_f = max(f_of(lam_hi), 0.3 * fp), f_of(lam_lo)
    return scale * integral(lo_f, hi_f) if hi_f > lo_f else 0.0


def fetch_sea(u, fetch):
    """JONSWAP fetch-limited growth (Hasselmann et al. 1973), capped at the fully developed sea."""
    x = G * fetch / (u * u)
    hs = min(1.6e-3 * math.sqrt(x), 0.21) * u * u / G
    tp = min(0.286 * x ** (1 / 3), 7.14) * u / G
    return hs, 1.0 / tp


def part_i():
    print("\nPart I: band gains for a wind of 10 m/s (amplitude share of the open sea's field)")
    bands = (("long waves, 20 m and up", 1e4, 20.0), ("cascade 20 to 2.5 m", 20.0, 2.5),
             ("cascade 2.5 to 0.6 m", 2.5, 0.6), ("cascade 0.6 to 0.07 m", 0.6, 0.07))
    u = 10.0
    hs_open, fp_open = fetch_sea(u, 1e9)
    open_var = [band_variance(fp_open, hs_open, hi, lo) for _, hi, lo in bands]
    print(f"  open sea: significant height {hs_open:.2f} m, peak wavelength {G / (2 * math.pi * fp_open ** 2):.0f} m")
    for fetch in (200.0, 1000.0, 5000.0, 50000.0):
        hs, fp = fetch_sea(u, fetch)
        gains = []
        for (name, hi, lo), ov in zip(bands, open_var):
            v = band_variance(fp, hs, hi, lo)
            gains.append(min(1.0, math.sqrt(v / ov)) if ov > 0 else 0.0)
        print(f"  fetch {fetch / 1000:5.1f} km: height {hs:.2f} m, peak {G / (2 * math.pi * fp * fp):5.1f} m, gains "
              + ", ".join(f"{g:.2f}" for g in gains))
    print("  depth limit, significant height at most 0.6 of the depth (Thornton and Guza 1983):")
    for d in (10.0, 3.0, 1.0, 0.3, 0.05):
        print(f"    depth {d:5.2f} m: a 2.1 m sea is held to {min(1.0, 0.6 * d / hs_open):.2f} of its height")


if __name__ == "__main__":
    part_h()
    part_i()
