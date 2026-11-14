"""Independent, standard-library-only WATERS design counterexamples.

This is not engine code or GPU/visual qualification. The FFT case is a 1D
cross-section of a 2D field constant along a straight, domain-spanning land bar.
It gives the design a correct finite-depth h/phi step, zeros land every tick,
and completely absorbs the adjacent wet cell (stronger than partial damping).
"""
import cmath
import math


def fft(values, inverse=False):
    n = len(values)
    if n == 1:
        return values[:]
    even = fft(values[::2], inverse)
    odd = fft(values[1::2], inverse)
    sign = 1 if inverse else -1
    turns = [cmath.exp(sign * 2j * math.pi * j / n) * odd[j] for j in range(n // 2)]
    return [even[j] + turns[j] for j in range(n // 2)] + [even[j] - turns[j] for j in range(n // 2)]


def dry_bar():
    n, dx, dt, g, depth = 256, 0.25, 1 / 60, 9.81, 4.0
    x = [(j - n // 2) * dx for j in range(n)]
    initial = [math.exp(-((p + 4) / 0.5) ** 2) if p < -1.25 else 0.0 for p in x]
    factors = []
    for j in range(n):
        k = 2 * math.pi * abs(j if j <= n // 2 else j - n) / (n * dx)
        D = k * math.tanh(k * depth)
        w = math.sqrt(g * D)
        factors.append((math.cos(w * dt), D * math.sin(w * dt) / w if w else 0,
                        -g * math.sin(w * dt) / w if w else -g * dt))
    for barrier in (False, True):
        h, phi = initial[:], [0.0] * n
        peak_other_side = 0.0
        for step in range(600):
            H, P = fft(h), fft(phi)
            hh = fft([a * v + b * p for (a, b, c), v, p in zip(factors, H, P)], True)
            pp = fft([a * p + c * v for (a, b, c), v, p in zip(factors, H, P)], True)
            for j, p in enumerate(x):
                # Full zero on the two-metre land bar AND its adjacent wet cell.
                damping = 0.0 if barrier and abs(p) <= 1.25 else math.exp(-max(0, abs(p) - 24) ** 2 * dt)
                h[j], phi[j] = hh[j].real / n * damping, pp[j].real / n * damping
            peak_other_side = max(peak_other_side, max(abs(v) for p, v in zip(x, h) if 1.5 <= p <= 12))
        print(f"dry bar={barrier}: max |height| in opposite water over 10 s / initial peak = {peak_other_side:.6f}")


def local_cases():
    print("Join: equal mean levels, swell crest 0.5 m at the mouth.")
    print("  sea long-wave swell gain 1 -> height +0.5 m; chart swell gain 0 -> height 0 m")
    print("  hard chart-over-sea ownership leaves a 0.5 m displacement jump.")
    print("Shore: one regular wave, spectral significant height 4*sigma = 2*sqrt(2)*A, limited to 0.6*d.")
    print(f"  A/d <= {0.6 / (2 * math.sqrt(2)):.6f} < 1; surface remains above the bed at every wet point.")
    print("  At d=0, A=0; outside the mean shore the map supplies no water. No run-up or recession.")
    depths = [0.125 * 2**i for i in range(6)]
    print(f"Six factor-two depth bins from 0.125 m: {depths}; cannot bracket 10 m water.")
    print("Finite-depth extension using unchanged eWave equations 19/20 and replacing omega only:")
    k, d, dt, g = math.pi, 0.25, 1 / 60, 9.81
    w = math.sqrt(g * k * math.tanh(k * d))
    det = math.cos(w * dt)**2 + g*k/w**2 * math.sin(w * dt)**2
    print(f"  2 m wave at 0.25 m depth: determinant={det:.9f}; amplitude multiplier/sec={det**30:.6f}")
    print("  This is NOT an exact rotation. A finite-depth operator needs its coefficients specified too.")
    print("Part H instead blends scalar exp(-i*omega*dt), not h/phi update matrices or spatial fields.")


if __name__ == '__main__':
    local_cases()
    dry_bar()
