"""Independent SCENE-VIEWS design counterexample; no engine or GPU measurement.

Run from the repository root:
  python3 docs/evidence/SCENE-VIEWS/design-review/fold_transport.py

Compare altitude-only folding with transport along the physical broken ray.
Use Thrum's HG convention and g=0.8 from atmosphere/common.wgsl. Both
integrals have identical folded density, extinction and incident Sun light.
Only the direction entering the phase function differs. The illustrative
single-band coefficients below do not predict an engine pixel's brightness.
"""

import math


def hg(mu):
    g = 0.8
    return (1 - g * g) / (4 * math.pi * (1 + g * g - 2 * g * mu) ** 1.5)


def integrate(start, end, phase):
    # Mirrored camera (0,-40), surface (40,0), receiver (41,1).
    # All distances are metres. The physical camera is (0,40).
    # Sun direction and unfolded ray are both (1,1)/sqrt(2).
    # Before the surface, the physical ray direction is (1,-1)/sqrt(2).
    steps = 20000
    dt = (end - start) / steps
    transmission, scatter = 1.0, 0.0
    for i in range(steps):
        t = start + (i + 0.5) * dt
        height = abs(-40.0 + t / math.sqrt(2.0))
        density = math.exp(-height / 1200.0)
        sigma_s, sigma_t = 4e-6 * density, 5e-6 * density
        segment_t = math.exp(-sigma_t * dt)
        scatter += transmission * (sigma_s / sigma_t) * phase * (1 - segment_t)
        transmission *= segment_t
    return transmission, scatter


def main():
    crossing = 40 * math.sqrt(2)
    endpoint = 41 * math.sqrt(2)
    t_cs, s_cs = integrate(0, crossing, hg(0.0))
    _, wrong_cs = integrate(0, crossing, hg(1.0))
    t_sb, s_sb = integrate(crossing, endpoint, hg(1.0))
    physical_s = s_cs + t_cs * s_sb
    altitude_only_s = wrong_cs + t_cs * s_sb
    # WATERS DESIGN Units removes camera-to-surface transport to get the
    # surface-to-body radiance. Set body emission to zero to isolate scatter.
    recovered_s = (altitude_only_s - s_cs) / t_cs
    print('Camera (0,40); mirror camera (0,-40); water (40,0); receiver (41,1)')
    print('Sun direction (1,1)/sqrt(2); fold height 0; distances in metres')
    print(f'HG phase, physical first segment mu=0: {hg(0.0):.9f}')
    print(f'HG phase, altitude-only mu=1:         {hg(1.0):.9f}')
    print(f'First-segment phase ratio: {hg(1.0) / hg(0.0):.6f}')
    print(f'Both models total transmission: {t_cs * t_sb:.9f}')
    print(f'Physical folded-path in-scatter: {physical_s:.9f}')
    print(f'Altitude-only in-scatter:        {altitude_only_s:.9f}')
    print(f'True surface-to-body in-scatter: {s_sb:.9f}')
    print(f'Recovered by WATERS from altitude-only colour: {recovered_s:.9f}')
    print(f'Recovered / true in-scatter: {recovered_s / s_sb:.6f}')
    assert hg(1.0) / hg(0.0) > 260
    assert recovered_s / s_sb > 35
    print('Verified: equal folded density does not imply equal transport.')


if __name__ == '__main__':
    main()
