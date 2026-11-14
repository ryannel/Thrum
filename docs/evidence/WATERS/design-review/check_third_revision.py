"""Independent specification checks for the third WATERS design review.

No engine or planner-proof imports. These check arithmetic, not rendering.
"""

import math
import random

G = 9.81
ROOT8 = math.sqrt(8)


def surface(level, bed, heights, gains, phases, margin=1.5):
    energies = [h * gain for h, gain in zip(heights, gains)]
    swash = min(margin, 0.7 * math.sqrt(sum(h * h for h in energies)))
    cap = max(0, 0.6 * (level - bed + swash))
    return level + sum(min(h, cap) * math.cos(p) / ROOT8
                       for h, p in zip(energies, phases)), swash


def joins():
    hs = ROOT8 * 0.5
    sea, _ = surface(0, -0.25, [hs], [1], [0])
    old_chart = min(hs, 0.6 * 0.25) / ROOT8
    chart, _ = surface(0, -0.25, [hs], [1], [0])
    print(f"Previous shallow mouth: sea={sea:.9f}, old chart={old_chart:.9f}, "
          f"step={sea-old_chart:.9f} m; revised step={sea-chart:.9f} m")
    assert abs(sea - old_chart - 0.21) < 1e-12
    rng = random.Random(301)
    worst = 0
    for _ in range(1000):
        level = rng.uniform(-1, 6)
        bed = level - rng.uniform(-1.5, 4)
        own = [rng.random() for _ in range(4)]
        receiving = [rng.random() for _ in range(4)]
        heights = [rng.uniform(0, 2) for _ in range(4)]
        phases = [rng.uniform(0, math.tau) for _ in range(4)]
        # Endpoint weight is one. Reconstruct the mixed row separately.
        mixed = [a + (b - a) for a, b in zip(own, receiving)]
        a = surface(level, bed, heights, receiving, phases)
        b = surface(level, bed, heights, mixed, phases)
        worst = max(worst, *(abs(x-y) for x, y in zip(a, b)))
    print(f"1000 endpoint rows, four layers, varied phase/depth/stage: worst height/swash difference={worst:.3g} m")
    assert worst < 1e-12


HS, BETA, LAMBDA = 2.14, 0.1, 60.0
S = 0.7 * HS
C = 0.6 / ROOT8
K = math.tau / LAMBDA
OMEGA = math.sqrt(G * K)
PERIOD = math.tau / OMEGA
RUNUP = C * S / (1 + C) / BETA
BACK = -C * S / (1 - C) / BETA


def drift(x, t):
    """Analytic derivatives of the limited single-wave height, unlike part M's finite differences."""
    cap = max(0, C * (S - BETA * x))
    amplitude = min(HS / ROOT8, cap)
    derivative = -C * BETA if 0 < cap < HS / ROOT8 else 0
    phase = K * x - OMEGA * t
    height = amplitude * math.cos(phase)
    rise = amplitude * OMEGA * math.sin(phase)
    if height <= BETA * x or rise <= 0:
        return 0
    slope = derivative * math.cos(phase) - amplitude * K * math.sin(phase) - BETA
    weight = min(1, max(0, 1 - x / BACK))
    if slope == 0:
        return 0
    return -math.copysign(1, slope) * weight * min(rise / abs(slope), math.sqrt(G * S))


def patch(cover, ticks, enabled=True, capped=True):
    dx, n = 0.25, 160
    xs = [(i - n // 2 + 0.5) * dx for i in range(n)]
    values = [cover if -1 <= x <= 0 else 0 for x in xs]
    initial = sum(values)
    dt = ticks / 60
    peak = cover
    lost = 0
    for step in range(int(3 * PERIOD / dt)):
        dest = [0.0] * n
        for x, value in zip(xs, values):
            if value == 0:
                continue
            velocity = drift(x, (step + 0.5) * dt) if enabled else 0
            index = (x + velocity * dt - xs[0]) / dx
            lo = math.floor(index)
            fraction = index - lo
            assert 0 <= lo < n - 1
            dest[lo] += (1-fraction) * value
            dest[lo+1] += fraction * value
        peak = max(peak, max(dest))
        if capped:
            lost += sum(max(0, v-1) for v in dest)
            dest = [min(1, v) for v in dest]
        values = dest
    total = sum(values)
    centre = sum(x*v for x, v in zip(xs, values)) / total
    assert abs(total + lost - initial) < 1e-10
    print(f"cover={cover}, ticks={ticks}, drift={enabled}, capped={capped}: "
          f"centre={centre:.6f} m, run-up error={RUNUP-centre:.6f} m, "
          f"retained={100*total/initial:.3f}%, peak before cap={peak:.6f}")
    if enabled and cover == 0.1:
        assert abs(centre - RUNUP) < 0.5 and lost == 0
    if not enabled:
        assert abs(centre + 0.5) < 1e-12


if __name__ == "__main__":
    joins()
    print("Foam: one regular wave, 1:10 beach, no wind/current/births/ageing/spreading.")
    for ticks in (1, 8):
        patch(0.1, ticks, enabled=False)
        patch(0.1, ticks)
        patch(1, ticks, capped=False)
        patch(1, ticks)
    print("Assertions passed. Full-cover loss is the declared saturation sink; thin-patch transport conserves.")
