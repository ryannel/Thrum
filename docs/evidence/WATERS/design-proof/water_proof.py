"""WATERS design, 14 November 2026: reproductions H to L (A to G are in ../../SEA/design-proof).
Pure Python, no dependencies. About a minute to run.

Part H  Interaction waves on water of any depth: the finite-depth state update, the depth ladder and its
        per-cell blend, measured as the operator the design selects: per wave vector on uniform depth,
        and as height and potential fields through beds whose depth varies, for steps of one and eight ticks.
Part I  How much of each layer of an open sea's wave field a sheltered body takes: band gains for a
        fetch-limited spectrum, and the depth limit near a shore.
Part J  Dry ground: one transform over a whole window carries a wave across a bar of land; one solve for
        each connected region of water carries none. What thin land inside one region lets through.
Part K  Joins: a channel from a lake to the sea on a 200 m by 120 m island, with the lake's stage and the
        tide moved and a swell crest at the mouth. Steps in the surface where two bodies meet.
Part L  The swash: a regular wave on a plane beach under the depth limit with a swash depth. How far the
        waterline runs up and back.

Part H replaces the first version of this file's part H, which blended scalar phases at one depth and
qualified nothing about fields (design review, finding 4).

Run: python3 water_proof.py
"""
import cmath
import math
import random

G = 9.81
TICK = 1.0 / 60.0
NEAR = [0.25, 0.5, 1.0, 2.0, 4.0, 8.0]                  # last entry is the deep operator
FAR = [1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0, 128.0]     # last entry is the deep operator


def geom(a, b, n):
    return [a * (b / a) ** (i / (n - 1)) for i in range(n)]


def smooth(t):
    t = min(1.0, max(0.0, t))
    return t * t * (3 - 2 * t)


# ---------------------------------------------------------------- the interaction operator

def weights(d, ladder):
    """Blend weights for a cell of depth d: (operator, weight). Operator -1 is the rest operator
    (no propagation), the last ladder entry is the deep operator. Linear in the square root of depth
    between two ladder depths, linear in depth below the first."""
    if d <= 0:
        return []
    if d <= ladder[0]:
        return [(-1, 1 - d / ladder[0]), (0, d / ladder[0])]
    if d >= ladder[-1]:
        return [(len(ladder) - 1, 1.0)]
    i = max(j for j in range(len(ladder)) if ladder[j] <= d)
    w = (math.sqrt(d) - math.sqrt(ladder[i])) / (math.sqrt(ladder[i + 1]) - math.sqrt(ladder[i]))
    return [(i, 1 - w), (i + 1, w)]


def turn(k, depth_index, ladder, dt):
    """The finite-depth update for one wave vector: h' = c h + (D/w) s phi, phi' = c phi - (g/w) s h,
    with D = k tanh(k d) and w = sqrt(g D). Returns (c, (D/w) s, -(g/w) s)."""
    if depth_index < 0:
        return 1.0, 0.0, -G * dt
    D = k if depth_index == len(ladder) - 1 else k * math.tanh(k * ladder[depth_index])
    w = math.sqrt(G * D)
    if w == 0:
        return 1.0, 0.0, -G * dt
    return math.cos(w * dt), D * math.sin(w * dt) / w, -G * math.sin(w * dt) / w


def blended_mode(k, d, ladder, nt):
    """Effective frequency and amplitude factor a second of the blended update on uniform depth d."""
    dt = TICK * nt
    m = [[0.0, 0.0], [0.0, 0.0]]
    for b, w in weights(d, ladder):
        c, hp, ph = turn(k, b, ladder, dt)
        m[0][0] += w * c
        m[0][1] += w * hp
        m[1][0] += w * ph
        m[1][1] += w * c
    tr = m[0][0] + m[1][1]
    det = m[0][0] * m[1][1] - m[0][1] * m[1][0]
    lam = tr / 2 + cmath.sqrt(tr * tr / 4 - det)
    return abs(cmath.phase(lam)) / dt, abs(lam) ** (1 / dt)


def true_omega(k, d):
    return math.sqrt(G * k * math.tanh(k * d))


def make_fft(n):
    bits = n.bit_length() - 1
    rev = [int(format(i, "0%db" % bits)[::-1], 2) for i in range(n)]
    tw = [cmath.exp(-2j * math.pi * i / n) for i in range(n // 2)]

    def fft(a, inverse=False):
        a = [a[r] for r in rev]
        m = 1
        while m < n:
            step = n // (2 * m)
            for s in range(0, n, 2 * m):
                for j in range(m):
                    w = tw[j * step].conjugate() if inverse else tw[j * step]
                    u, v = a[s + j], a[s + j + m] * w
                    a[s + j], a[s + j + m] = u + v, u - v
            m *= 2
        return [x / n for x in a] if inverse else a
    return fft


class Level:
    """One interaction level on a 1D section: height h and potential phi on n cells of size dx over a bed.
    A step transforms once, applies each needed depth's update, and blends them cell by cell.
    `regions` is a list of cell masks; each is solved alone and keeps only its own cells."""

    def __init__(self, n, dx, depth, ladder, nt=1, damp=None):
        self.n, self.dx, self.depth, self.ladder, self.dt = n, dx, depth, ladder, TICK * nt
        self.fft = make_fft(n)
        ks = [abs(2 * math.pi * (j if j <= n // 2 else j - n) / (n * dx)) for j in range(n)]
        self.w = [weights(d, ladder) for d in depth]
        self.used = sorted({b for ws in self.w for b, _ in ws if b >= 0})
        self.fac = {b: [turn(k, b, ladder, self.dt) for k in ks] for b in self.used}
        self.damp = damp or [1.0] * n
        self.h, self.phi = [0.0] * n, [0.0] * n

    def step(self, regions=None):
        n = self.n
        nh, nph = [0.0] * n, [0.0] * n
        for mask in regions or [[1] * n]:
            hin = [a * m for a, m in zip(self.h, mask)]
            pin = [a * m for a, m in zip(self.phi, mask)]
            Z = self.fft([complex(a, b) for a, b in zip(hin, pin)])
            H = [(Z[j] + Z[-j % n].conjugate()) / 2 for j in range(n)]
            P = [(Z[j] - Z[-j % n].conjugate()) / 2j for j in range(n)]
            outs = {}
            for b in self.used:
                f = self.fac[b]
                outs[b] = self.fft([(f[j][0] * H[j] + f[j][1] * P[j]) + 1j * (f[j][0] * P[j] + f[j][2] * H[j])
                                    for j in range(n)], True)
            for j in range(n):
                if not mask[j]:
                    continue
                a = b_ = 0.0
                for b, w in self.w[j]:
                    if b < 0:
                        a += w * hin[j]
                        b_ += w * (pin[j] - G * self.dt * hin[j])
                    else:
                        a += w * outs[b][j].real
                        b_ += w * outs[b][j].imag
                nh[j], nph[j] = a * self.damp[j], b_ * self.damp[j]
        self.h, self.phi = nh, nph


def wavenumber(om, d):
    k = om * om / G
    for _ in range(80):
        k = om * om / (G * math.tanh(k * d))
    return k


def group_speed(om, d):
    k = wavenumber(om, d)
    return 0.5 * om / k * (1 + 2 * k * d / math.sinh(2 * k * d))


def packet(level, x, x0, lam, sigma, d0, direction=1):
    k = 2 * math.pi / lam
    D = k * math.tanh(k * d0)
    om = math.sqrt(G * D)
    for j, xx in enumerate(x):
        e = math.exp(-((xx - x0) / sigma) ** 2 / 2)
        level.h[j] = e * math.cos(k * (xx - x0))
        level.phi[j] = direction * (om / D) * e * math.sin(k * (xx - x0))
    return om


def centroid(level, x):
    s = sum(h * h for h in level.h)
    return sum(xx * h * h for xx, h in zip(x, level.h)) / s, s


def part_h():
    print("Part H: interaction waves at depth, as the selected operator")
    k, d, dt = math.pi, 0.25, TICK
    w = true_omega(k, d)
    literal = math.cos(w * dt) ** 2 + G * k / w ** 2 * math.sin(w * dt) ** 2
    c, hp, ph = turn(k, 0, [d, 1e9], dt)
    print("  the update, 2 m wave on 0.25 m of water, one tick:")
    print(f"    eWave eq. 19 and 20 with only omega replaced: determinant {literal:.9f}, amplitude x{literal ** 30:.4f} a second")
    print(f"    with D = k tanh(kd) in the height row:          determinant {c * c - hp * ph:.9f}")
    print("  uniform depth, each wave vector, weights linear in the square root of depth")
    print("  level  depths             wavelengths    step      phase speed       group speed       loss a second   growth")
    for name, ladder, lams, dlo in (("near", NEAR, geom(0.5, 16, 30), 0.25), ("far", FAR, geom(12, 256, 30), 1.0),
                                    ("far", FAR, geom(12, 256, 30), 0.25)):
        for nt in (1, 8):
            wp = wg = 0.0
            lo, hi = 1.0, 1.0
            for lam in lams:
                k = 2 * math.pi / lam
                dk = k * 1e-4
                for d in geom(dlo, ladder[-1] * 2, 240):
                    o, a = blended_mode(k, d, ladder, nt)
                    wp = max(wp, abs(o / true_omega(k, d) - 1))
                    cg_e = (blended_mode(k + dk, d, ladder, nt)[0] - blended_mode(k - dk, d, ladder, nt)[0]) / (2 * dk)
                    cg_t = (true_omega(k + dk, d) - true_omega(k - dk, d)) / (2 * dk)
                    wg = max(wg, abs(cg_e / cg_t - 1))
                    lo, hi = min(lo, a), max(hi, a)
            print(f"  {name:5s}  {dlo:4.2f} m and deeper  {lams[0]:4.1f} to {lams[-1]:5.1f} m  {nt} tick{'s' if nt > 1 else ' '}"
                  f"   within {100 * wp:5.2f} %    within {100 * wg:5.2f} %    {100 * (1 - lo):6.3f} %        {hi:.7f}")
    print("  fields over a bed that varies (height and potential stepped as the design's step does)")
    n, dx = 512, 0.25
    x = [j * dx for j in range(n)]
    dep = lambda xx: 6.0 if xx < 30 else max(6.0 - (xx - 30) / 10, 0.0) if xx < 100 else 0.0
    depth = [dep(xx) for xx in x]
    damp = [math.exp(-TICK * 6.0 * (1 - d / NEAR[0])) if 0 < d < NEAR[0] else 1.0 for d in depth]
    gauges = {}
    for nt in (1, 8):
        lv = Level(n, dx, depth, NEAR, nt, [v ** nt for v in damp])
        om = packet(lv, x, 14.0, 6.0, 5.0, 6.0)
        ray, gauge = 14.0, []
        if nt == 1:
            print("    near level, 6 m wave packet up a 1 in 10 beach from 6 m of water (ray: the true group speed integrated):")
        for s in range(int(60 * 60 / nt)):
            lv.step()
            for _ in range(8 * nt):
                ray += group_speed(om, max(dep(ray), 1e-3)) * TICK / 8
            gauge.append(lv.h[int(70 / dx)])
            if nt == 1 and s % 600 == 599 and s < 3000:
                c, e = centroid(lv, x)
                print(f"      t {(s + 1) / 60:4.0f} s  packet at {c:6.2f} m, ray at {ray:6.2f} m, depth there {dep(c):4.2f} m, sum of h^2 {e:6.2f}")
        gauges[nt] = gauge
        if nt == 1:
            c, e = centroid(lv, x)
            print(f"      t   60 s  after the shore: sum of h^2 {e:5.2f}, {100 * e / 17.72:.1f} % of what arrived, travelling back")
    a = [gauges[1][i * 8 + 7] for i in range(len(gauges[8]))]
    rms = math.sqrt(sum((p - q) ** 2 for p, q in zip(a, gauges[8])) / len(a))
    print(f"    the same run in steps of 8 ticks, height at the 2 m depth gauge: differs by {100 * rms / math.sqrt(sum(p * p for p in a) / len(a)):.2f} % of its RMS")
    n, dx = 256, 2.0
    x = [j * dx for j in range(n)]
    dep = lambda xx: 0.125 if xx < 40 else 0.125 + (10 - 0.125) * (xx - 40) / 40 if xx < 80 else 10.0
    depth = [dep(xx) for xx in x]
    damp = [math.exp(-TICK * 4 * (1 - min(j, n - 1 - j) / 20) ** 2) if min(j, n - 1 - j) < 20 else 1.0 for j in range(n)]
    print("    far level, a window holding 0.125 m shallows and 10 m of water, packets crossing the 10 m part:")
    for lam, sigma in ((30.0, 30.0), (60.0, 45.0), (100.0, 60.0)):
        lv = Level(n, dx, depth, FAR, 1, damp)
        om = packet(lv, x, 400.0, lam, sigma, 10.0, -1)
        for s in range(60 * 14):
            lv.step()
            if s == 60 * 4 - 1:
                c1, _ = centroid(lv, x)
            if s == 60 * 14 - 1:
                c2, _ = centroid(lv, x)
        v, true = (c1 - c2) / 10.0, group_speed(om, 10.0)
        print(f"      {lam:5.0f} m wave: packet speed {v:.3f} m/s, true group speed {true:.3f} m/s, {100 * (v / true - 1):+.2f} %"
              f"   (depths solved: {', '.join(str(FAR[b]) for b in lv.used)} m)")
    print("    rough beds, 0 to 9 m deep with dry patches, no damping, 120 s: largest sum of h^2 in each 20 s, over the initial")
    n, dx = 512, 0.25
    for seed, nt in ((3, 1), (7, 1), (7, 8)):
        random.seed(seed)
        d, depth = 3.0, []
        for j in range(n):
            d = min(9.0, max(-1.5, d + random.uniform(-0.45, 0.45)))
            depth.append(max(d, 0.0))
        lv = Level(n, dx, depth, NEAR, nt)
        lv.h = [(math.exp(-((j * dx - 40)) ** 2) + 0.5 * math.exp(-((j * dx - 90) / 3.0) ** 2)) if depth[j] > 0 else 0.0 for j in range(n)]
        lv.h = [h + (0.3 * math.sin(j * dx * 1.7) if depth[j] > 0 else 0.0) for j, h in enumerate(lv.h)]
        e0, peak, out = sum(h * h for h in lv.h), 0.0, []
        per = 1200 // nt
        for s in range(60 * 120 // nt):
            lv.step()
            peak = max(peak, sum(h * h for h in lv.h))
            if s % per == per - 1:
                out.append(peak / e0)
                peak = 0.0
        print(f"      bed {seed}, {sum(1 for v in depth if v <= 0)} dry cells of {n}, steps of {nt} tick{'s' if nt > 1 else ''}: " + ", ".join(f"{v:.3f}" for v in out))


# ---------------------------------------------------------------- shelter

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


# ---------------------------------------------------------------- dry ground

def part_j():
    print("\nPart J: dry ground in a 64 m window of 0.25 m cells, a pulse 4 m from the land, 10 s")
    print("  (largest height reached in the water beyond the land, over the pulse's initial peak)")
    n, dx = 256, 0.25
    x = [(j - n // 2) * dx for j in range(n)]

    def run(depth_of, split, land_half):
        depth = [depth_of(p) for p in x]
        damp = []
        for p, d in zip(x, depth):
            v = math.exp(-max(0.0, abs(p) - 24) ** 2 * TICK)
            if d > 0 and land_half is not None and abs(p) <= land_half + dx + 1e-9:
                v = 0.0                                   # the wet cell beside the land absorbs everything
            damp.append(v)
        lv = Level(n, dx, depth, NEAR, 1, damp)
        lv.h = [math.exp(-((p + 4) / 0.5) ** 2) if depth[j] > 0 and p < 0 else 0.0 for j, p in enumerate(x)]
        regions = None
        if split:
            regions = [[1 if p < 0 and d > 0 else 0 for p, d in zip(x, depth)], [1 if p > 0 and d > 0 else 0 for p, d in zip(x, depth)]]
        far_side = [j for j, p in enumerate(x) if land_half is not None and land_half + 0.5 <= p <= 12 or land_half is None and 1.5 <= p <= 12]
        peak = 0.0
        for _ in range(600):
            lv.step(regions)
            peak = max(peak, max(abs(lv.h[j]) for j in far_side))
        return peak

    wall = lambda half, deep: (lambda p: 0.0 if abs(p) <= half else deep)
    print(f"  no land, 4 m deep:                                        {run(lambda p: 4.0, False, None):.6f}")
    print(f"  2 m bar, 4 m deep to its foot, one transform:             {run(wall(1.0, 4.0), False, 1.0):.6f}   (the design review's case)")
    print(f"  the same bar, each side its own solve:                    {run(wall(1.0, 4.0), True, 1.0):.6f}")
    print("  thin land inside one connected region, which one transform still spans:")
    for half, deep in ((1.0, 1.0), (2.0, 4.0), (4.0, 4.0), (8.0, 4.0)):
        print(f"    {2 * half:4.0f} m of land, {deep:.0f} m deep to its foot:                    {run(wall(half, deep), False, half):.6f}")
    for slope in (4.0, 10.0):
        shelf = lambda p, s=slope: 0.0 if abs(p) <= 1.0 else min(4.0, (abs(p) - 1.0) / s)
        print(f"    2 m of land with beaches of 1 in {slope:.0f} each side:              {run(shelf, False, 1.0):.6f}")


# ---------------------------------------------------------------- joins

def part_k():
    print("\nPart K: a channel from a lake to the sea; cells of 1 m; steps in the surface between neighbouring wet cells")
    nx, ny, crest, band = 200, 120, 0.5, 0.25
    slope = (-2.0 - 4.2) / 108.0

    def terrain(px, py):
        t = 7.0 if px < 60 else max(7.0 - 0.1 * (px - 60), -5.0)
        r = math.hypot(px - 30, py - 60)
        return min(t, 7.0 - 4.0 * (1 - (r / 20) ** 2)) if r < 20 else t

    cut = lambda px: 4.2 + slope * (px - 42)
    in_strip = lambda px, py: 42 <= px <= 150 and abs(py - 60) <= 2.5
    in_chart = lambda px, py: 48 <= px <= 150 and abs(py - 60) <= 2.5
    bed = lambda px, py: min(terrain(px, py), cut(px)) if in_strip(px, py) else terrain(px, py)
    chart_level = lambda px: cut(px) + 0.4
    cells = [(i, j) for i in range(nx) for j in range(ny)]
    centre = lambda i, j: (i + 0.5, j + 0.5)

    def build(lake_stage, tide, joined):
        lake, sea = 5.0 + lake_stage, tide
        fill, stack = set(), [(30, 60)]
        while stack:
            c = stack.pop()
            if c in fill or not (0 <= c[0] < nx and 0 <= c[1] < ny):
                continue
            px, py = centre(*c)
            if in_chart(px, py) or bed(px, py) >= lake:
                continue
            fill.add(c)
            stack += [(c[0] + 1, c[1]), (c[0] - 1, c[1]), (c[0], c[1] + 1), (c[0], c[1] - 1)]
        out = {}
        for c in cells:
            px, py = centre(*c)
            b = bed(px, py)
            if in_chart(px, py):
                lc = chart_level(px)
                if joined:
                    m_sea = max(smooth((px - 138) / 12), smooth((sea - lc + band) / band))
                    m_lake = 1 - smooth((px - 48) / 12)
                    level = lc + m_sea * (sea - lc)
                    level += m_lake * (lake - level)
                    gain = m_sea * (1 - m_lake)            # long-wave gain: 1 at sea, 0 in the chart and the lake
                else:                                      # the chart as prepared for the reference stages
                    level = lc + smooth((0.0 - lc + band) / band) * (0.0 - lc)
                    level += (1 - smooth((px - 48) / 12)) * (5.0 - level)
                    gain = 0.0
                if level > b:
                    out[c] = ("chart", level, gain)
            elif c in fill:
                out[c] = ("lake", lake, 0.0)
            elif b < sea and px > 90:
                out[c] = ("sea", sea, 1.0)
        return out, fill

    def measure(lake_stage, tide, joined):
        water, fill = build(lake_stage, tide, joined)
        step = inside = floating = 0.0
        stray = 0
        for (i, j), (body, level, gain) in water.items():
            for c in ((i + 1, j), (i, j + 1), (i - 1, j), (i, j - 1)):
                if c in water:
                    other, level2, gain2 = water[c]
                    d = abs((level + gain * crest) - (level2 + gain2 * crest))
                    if other != body:
                        step = max(step, d)
                        if {body, other} == {"chart", "lake"} and max(i, c[0]) != 48:
                            stray += 1
                    else:
                        inside = max(inside, d)
                elif body == "chart" and 0 <= c[0] < nx:
                    floating = max(floating, level - bed(*centre(*c)))
        return step, inside, floating, stray

    print("  a 0.5 m swell crest stands at the mouth; the surface is the level plus the long-wave gain times the crest")
    print("  lake stage  tide    hard precedence, one stage a chart        joins in the map")
    print("                      step between bodies   chart over its bank   step between bodies   chart over its bank   largest step inside a body")
    for lake_stage, tide in ((0.0, 0.0), (1.0, 0.0), (-0.5, 0.0), (0.0, 1.0), (0.0, -1.0), (1.0, 1.0), (-0.5, -1.0), (1.0, -1.0)):
        old, new = measure(lake_stage, tide, False), measure(lake_stage, tide, True)
        print(f"  {lake_stage:+5.1f} m    {tide:+4.1f} m   {old[0]:6.3f} m              {max(old[2], 0):6.3f} m              "
              f"{new[0]:6.3f} m              {max(new[2], 0):6.3f} m              {new[1]:5.3f} m")
        assert new[3] == 0, "the lake met the chart away from its source end"
    print("  (a step between bodies under joins is the 1 m cell: the blend weight is 1 on the end's edge and the")
    print("   sample is half a cell inside it. 1 mm either side of each end, lake +1 m and tide -1 m:)")
    lake, sea = 6.0, -1.0
    lc = chart_level(48.001)
    print(f"    source end: lake {lake:.4f} m, chart {lc + (1 - smooth(0.001 / 12)) * (lake - lc):.4f} m")
    lc = chart_level(149.999)
    m = max(smooth((149.999 - 138) / 12), smooth((sea - lc + band) / band))
    print(f"    mouth:      sea {sea + crest:.4f} m at the crest, chart {lc + m * (sea - lc) + m * crest:.4f} m")


# ---------------------------------------------------------------- the swash

def part_l():
    print("\nPart L: a regular wave on a 1 in 10 beach at a fixed stage; the still waterline is at 0")
    print("  each layer's significant height is held to 0.6 of (depth + swash depth); swash depth = 0.7 of the body's")
    print("  significant height; a regular wave of amplitude A has significant height 2 sqrt(2) A")
    print("  sea height   swash depth   waterline's highest and lowest, as height on the beach      along the beach   closed form")
    beta, lam = 0.1, 60.0
    k = 2 * math.pi / lam
    for hs, ratio in ((2.14, 0.0), (0.16, 0.7), (1.0, 0.7), (2.14, 0.7)):
        s, a0 = ratio * hs, hs / (2 * math.sqrt(2))
        hi, lo = -1e9, 1e9
        for step in range(240):
            phase = 2 * math.pi * step / 240
            edge = None
            for i in range(-4000, 4001):                  # x in cm, shoreward positive; bed height = beta * x
                xx = i * 0.01
                d = -beta * xx
                amp = min(a0, max(0.0, 0.6 * (d + s)) / (2 * math.sqrt(2)))
                if amp * math.cos(k * xx - phase) > beta * xx:
                    edge = xx
                elif edge is not None:
                    break
            if edge is not None:
                hi, lo = max(hi, beta * edge), min(lo, beta * edge)
        g = 0.6 / (2 * math.sqrt(2))
        print(f"  {hs:5.2f} m      {s:5.2f} m       {hi:+6.3f} m and {lo:+6.3f} m                               "
              f"{(hi - lo) / beta:5.2f} m          {g * s / (1 + g):+.3f} and {-g * s / (1 - g):+.3f} m")
    print("  (first row: the limit with no swash depth, as first designed: the waterline does not move)")


if __name__ == "__main__":
    part_h()
    part_i()
    part_j()
    part_k()
    part_l()
