"""SEA design, 14 November 2026: two reproductions for the first-principles revision.

Part F  Phase of a short wave after 24 hours: f32 seconds against a whole-cycle loop with integer phase.
Part G  A wave shown at the display's time: evaluated there, against a blend of two tick endpoints;
        and a query answer carried to its use tick by the time derivative.

Run: python3 first_principles_proof.py   (numpy only)
"""
import numpy as np

G = 9.81
TICK = 1.0 / 60.0


def part_f():
    print("Part F: phase after 24 hours")
    loop = 1024.0                      # the profile's loop, seconds
    w0 = 2 * np.pi / loop
    rng = np.random.default_rng(7)
    lengths = np.array([20.0, 9.0, 2.0, 0.4, 0.07])
    worst_f32 = worst_loop = 0.0
    for lam in lengths:
        w = np.sqrt(G * 2 * np.pi / lam)
        n = int(round(w / w0))        # whole cycles per loop
        wq = n * w0
        t = 86400.0 + rng.uniform(0, 1000, 20000)        # frame times, f64 seconds
        ref = np.mod(wq * t, 2 * np.pi)                   # f64 reference
        # (a) f32 seconds, f32 multiply: what a shader given `time` does
        a = np.mod((np.float32(wq) * t.astype(np.float32)).astype(np.float64), 2 * np.pi)
        ea = np.abs(np.angle(np.exp(1j * (a - ref))))
        # (b) loop time as a 32-bit fraction of the loop, integer multiply that wraps
        u = np.floor(np.mod(t, loop) / loop * 2**32).astype(np.uint64)
        turns = ((u * np.uint64(n)) & np.uint64(0xFFFFFFFF)).astype(np.float64) / 2**32
        b = (np.float32(2 * np.pi) * turns.astype(np.float32)).astype(np.float64)
        eb = np.abs(np.angle(np.exp(1j * (b - ref))))
        worst_f32 = max(worst_f32, ea.max()); worst_loop = max(worst_loop, eb.max())
        print(f"  wavelength {lam:5.2f} m  omega {w:6.3f}  cycles/loop {n:5d}  "
              f"speed change {abs(wq - w) / w * 100:.4f}%  f32 seconds {ea.max():.4f} rad  loop phase {eb.max():.2e} rad")
    print(f"  worst: f32 seconds {worst_f32:.3f} rad, loop phase {worst_loop:.2e} rad")


def part_g():
    print("Part G: display time and query carry, as a share of the wave's amplitude")
    print("  wavelength   blend of endpoints 1 tick apart   evaluated at display time   carry 2 ticks   carry 3 ticks")
    for lam in (150.0, 20.0, 9.0, 2.0, 0.4):
        w = np.sqrt(G * 2 * np.pi / lam)
        ph = np.linspace(0, 2 * np.pi, 4001)[:, None]
        a = np.linspace(0, 1, 201)[None, :]
        blend = (1 - a) * np.cos(ph) + a * np.cos(ph + w * TICK)
        exact = np.cos(ph + a * w * TICK)
        e_blend = np.abs(blend - exact).max()
        carry = []
        for ticks in (2, 3):
            dt = ticks * TICK
            c = np.cos(ph) - w * dt * np.sin(ph)          # height carried by its time derivative
            carry.append(np.abs(c - np.cos(ph + w * dt)).max())
        print(f"  {lam:7.1f} m   {e_blend * 100:8.4f}%                         0                           "
              f"{carry[0] * 100:7.3f}%       {carry[1] * 100:7.3f}%")
    print("  texture reads a sea pixel makes for the ambient layers: one set when they are evaluated at the")
    print("  display's time, two sets when two tick endpoints are blended in the surface pass.")


if __name__ == "__main__":
    part_f()
    part_g()
