"""WATERS design, 14 November 2026: reproductions H to M (A to G are in ../../SEA/design-proof).
Pure Python, no dependencies. About a minute to run.

Part H  Interaction waves on water of any depth: the finite-depth state update, the depth ladder and its
        per-cell blend, measured as the operator the design selects: per wave vector on uniform depth,
        and as height and potential fields through beds whose depth varies, for steps of one and eight ticks.
Part I  How much of each layer of an open sea's wave field a sheltered body takes: band gains for a
        fetch-limited spectrum, and the depth limit near a shore.
Part J  Dry ground: one transform over a whole window carries a wave across a bar of land; one solve for
        each connected region of water carries none. What thin land inside one region lets through.
Part K  Joins: a channel from a lake to the sea on a 200 m by 120 m island, with the lake's stage and the
        tide moved and a swell crest at the mouth. Steps in the surface where two bodies meet: in the raw
        rows, then in the assembled surface after the depth limit, at shallow ends, with the swash depth
        taken once for each body and taken from the row; and beside a mouth, where the sea's sheet lies
        on a low bank.
Part L  The swash: a regular wave on a plane beach under the depth limit with a swash depth. How far the
        waterline runs up and back.
Part M  Foam in the swash: the drift a rising sheet gives foam, on parcels and on a grid of foam cells,
        against the drift of the current and the wind alone, which is none.

Part H replaces the first version of this file's part H, which blended scalar phases at one depth and
qualified nothing about fields (first design review, finding 4). The assembled surface in part K and all of
part M answer the second design review: its shallow-mouth case is reproduced in K, and M gives the foam the
transport that review found missing.

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
    root8 = 2 * math.sqrt(2)
    margin, ratio = 1.5, 0.7                      # margin height taken for this run; the swash ratio
    gl = 0.6 / root8
    reach = gl / (1 + gl) * margin                # a regular wave's run-up at the largest swash depth
    swell_hs, wind_hs = root8 * crest, 0.40       # the field: the swell, and its short wind waves at sea
    own = {"sea": (1.0, 1.0), "lake": (0.0, 0.16 / wind_hs), "chart": (0.0, 0.07 / wind_hs)}   # (long, short) gains

    def mix(a, b, w):
        return tuple(p + w * (q - p) for p, q in zip(a, b))

    def swash_of(gains):
        return min(margin, ratio * math.hypot(gains[0] * swell_hs, gains[1] * wind_hs))

    def surface(level, gains, bed_z, swash):
        """The assembled surface at a crest of both layers: the level plus each layer held by the depth limit."""
        cap = max(0.0, 0.6 * (level - bed_z + swash))
        return level + (min(gains[0] * swell_hs, cap) + min(gains[1] * wind_hs, cap)) / root8

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

    def chart_row(px, lake, sea, backwater_reach=reach):
        """The chart's mixed row at a place along it: its level and its (long, short) gains. Backwater makes
        the chart the sea's wherever the sea's level plus `backwater_reach` stands at or above the chart's own."""
        lc = chart_level(px)
        m_sea = max(smooth((px - 138) / 12), smooth((sea + backwater_reach - lc + band) / band))
        m_lake = 1 - smooth((px - 48) / 12)
        level = lc + m_sea * (sea - lc)
        level += m_lake * (lake - level)
        return level, mix(mix(own["chart"], own["sea"], m_sea), own["lake"], m_lake)

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
                    level, gains = chart_row(px, lake, sea)
                    gain = gains[0]                        # long-wave gain: 1 at sea, 0 in the chart and the lake
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
    level, gains = chart_row(48.001, lake, sea)
    print(f"    source end: lake {lake:.4f} m, chart {level:.4f} m")
    level, gains = chart_row(149.999, lake, sea)
    print(f"    mouth:      sea {sea + crest:.4f} m at the crest, chart {level + gains[0] * crest:.4f} m")

    print("\n  The assembled surface: the level plus each layer after the depth limit, at a crest of the swell and of the")
    print(f"  short wind waves ({wind_hs:.2f} m at sea, 0.16 m on the lake, 0.07 m on the chart's own gains); swash ratio {ratio}.")
    print("  Swash depth taken once for each body (the chart's is its own waves') against taken from the row at the point.")
    calm = (1.0, 0.0)
    there, per_body = surface(0.0, calm, -0.25, swash_of(calm)), surface(0.0, calm, -0.25, 0.0)
    print(f"  The design review's case, no wind, a mouth 0.25 m deep: sea {there:.9f} m, chart with its own swash depth "
          f"{per_body:.9f} m, step {there - per_body:.9f} m; from the row, step {0.0:.9f} m")
    print("  An end with the same bed and the same joined row either side, by the water's depth there, with the wind waves too:")
    print("  end      depth     joined body    chart, one swash depth a body    step       chart, swash depth from the row    step")
    for name, body, depths in (("mouth", "sea", (4.0, 1.0, 0.25, 0.0)), ("source", "lake", (1.0, 0.25, 0.05, 0.0))):
        for d in depths:
            there = surface(0.0, own[body], -d, swash_of(own[body]))
            per_body = surface(0.0, own[body], -d, swash_of(own["chart"]))
            from_row = surface(0.0, own[body], -d, swash_of(own[body]))
            print(f"  {name:6s}  {d:5.2f} m   {there:8.5f} m     {per_body:8.5f} m                       {there - per_body:7.5f} m  "
                  f"{from_row:8.5f} m                         {there - from_row:7.5f} m")
    print("  On the island, 1 mm either side of each end, through the stages:")
    print("  lake stage  tide    depth at the mouth   step, one a body   step, from the row   depth at the source   step, one a body   step, from the row")
    for lake_stage, tide in ((0.0, 0.0), (1.0, 0.0), (-0.5, 0.0), (0.0, 1.0), (0.0, -1.0), (1.0, 1.0), (-0.5, -1.0), (1.0, -1.0)):
        lake, sea = 5.0 + lake_stage, tide
        out = []
        for px, body, level_there in ((149.999, "sea", sea), (48.001, "lake", lake)):
            bed_z = bed(px, 60.0)
            level, gains = chart_row(px, lake, sea)
            there = surface(level_there, own[body], bed_z, swash_of(own[body]))
            out += [level_there - bed_z, abs(there - surface(level, gains, bed_z, swash_of(own["chart"]))),
                    abs(there - surface(level, gains, bed_z, swash_of(gains)))]
        print(f"  {lake_stage:+5.1f} m    {tide:+4.1f} m   {out[0]:5.2f} m              {out[1]:6.4f} m           {out[2]:6.4f} m            "
              f"{out[3]:5.2f} m               {out[4]:6.4f} m           {out[5]:6.4f} m")
    print("  (the island's source end is too deep for the limit to hold the lake's small waves; the table above it has a shallow sill)")

    print("\n  Beside a mouth: beach sand 2 cm above the channel's own water, which the sea's sheet also reaches. The sea's")
    print("  surface on that sand against the chart's, at a crest, as heights over the sand (nothing where it does not reach).")
    print("  Backwater measured from the sea's still level, and from its level plus the reach of its run-up "
          f"({reach:.4f} m for a margin height of {margin} m):")
    print("  chart's own level over the sea's   weight, still level   sea's sheet   chart's    step      weight, level plus reach   chart's    step")
    for over in (0.02, 0.05, 0.10, 0.15, 0.20, 0.25, 0.30, 0.35):
        sand = over + 0.02
        sea_h = max(surface(0.0, own["sea"], sand, swash_of(own["sea"])) - sand, 0.0)
        row = []
        for r in (0.0, reach):
            w = smooth((r - over + band) / band)
            level, gains = over + w * (0.0 - over), mix(own["chart"], own["sea"], w)
            h = max(surface(level, gains, sand, swash_of(gains)) - sand, 0.0)
            row += [w, h, abs(sea_h - h)]
        print(f"  {over:5.2f} m                            {row[0]:5.3f}                 {sea_h:6.4f} m     {row[1]:6.4f} m   {row[2]:6.4f} m  "
              f"{row[3]:5.3f}                      {row[4]:6.4f} m   {row[5]:6.4f} m")
    print("  (two layers cresting together stand a few centimetres higher than one regular wave's run-up, as the row at")
    print("   0.25 m shows; a chart beside sand they reach has a weight of 0.94 or more)")


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


# ---------------------------------------------------------------- foam in the swash

def part_m():
    print("\nPart M: foam in the swash. A plane beach at a fixed stage, the still waterline at 0, shoreward positive.")
    print("  The sheet's thickness is the surface less the ground. While the sheet rises at a place, a line of equal")
    print("  thickness travels at (rate of rise) / (slope of the thickness), toward thinner water; foam's drift gains that,")
    print("  never more than sqrt(g x swash depth), at full weight above the still waterline and fading to none at the")
    print("  depth the waterline draws back to. While the sheet falls the drift gains nothing. No wind, no current.")
    root8 = 2 * math.sqrt(2)
    gl = 0.6 / root8

    def beach(hs, beta, waves, ratio=0.7):
        """waves: (share of the variance's root, wavelength). Returns the surface, the swash drift and the bounds."""
        s, a0 = ratio * hs, hs / root8
        comps = [(w, 2 * math.pi / lam, math.sqrt(G * 2 * math.pi / lam)) for w, lam in waves]
        draw_back = gl * s / (1 - gl)
        bound = math.sqrt(G * s)

        def eta(x, t):
            amp = min(a0, max(0.0, 0.6 * (-beta * x + s)) / root8)
            return amp * sum(w * math.cos(k * x - om * t) for w, k, om in comps)

        def drift(x, t, on=True):
            e = 1e-3
            if not on or eta(x, t) <= beta * x:
                return 0.0, 0.0
            rise = (eta(x, t + e) - eta(x, t - e)) / (2 * e)
            if rise <= 0:
                return 0.0, 0.0
            hx = (eta(x + e, t) - eta(x - e, t)) / (2 * e) - beta
            d = -beta * x
            weight = 1.0 if d <= 0 else max(0.0, 1 - d / draw_back)
            raw = rise / abs(hx) if hx else 1e9
            return (-1 if hx > 0 else 1) * weight * min(raw, bound), raw
        return eta, drift, s, bound

    def waterline_top(eta, beta, seconds):
        top = -1e9
        for i in range(int(seconds * 30)):
            for j in range(-200, 1400):
                x = j * 0.01
                if eta(x, i / 30) > beta * x:
                    top = max(top, x)
        return top

    print("  parcels, one regular wave of 60 m, stepped each tick; where each lies along the beach after 1, 2 and 3 waves")
    print("  sea height  beach     run-up line   parcel from the draw-back line   from half way down    from the still waterline   fastest   bound")
    for hs, slope in ((2.14, 10), (1.0, 10), (0.16, 10), (2.14, 4), (2.14, 30)):
        beta = 1.0 / slope
        lam = 2.8 if hs < 0.5 else 60.0
        eta, drift, s, bound = beach(hs, beta, [(1.0, lam)])
        period = 2 * math.pi / math.sqrt(G * 2 * math.pi / lam)
        up, back = gl * s / (1 + gl) / beta, -gl * s / (1 - gl) / beta
        cols, fastest = [], 0.0
        for x in (back, back / 2, 0.0):
            t, marks = 0.0, []
            for i in range(int(3 * period / TICK)):
                v = drift(x + drift(x, t)[0] * TICK / 2, t + TICK / 2)[0]
                x, t, fastest = x + v * TICK, t + TICK, max(fastest, abs(v))
                if (i + 1) % int(period / TICK) == 0:
                    marks.append(x)
            cols.append(", ".join(f"{m:5.2f}" for m in marks))
        print(f"  {hs:5.2f} m     1 in {slope:<3d}  {up:5.2f} m       {cols[0]} m             {cols[1]} m   {cols[2]} m        "
              f"{fastest:4.2f}      {bound:4.2f} m/s")
    print("  (a parcel on the draw-back line is never bared and is not carried: seaward of the swash nothing moves foam shoreward)")

    print("  two wave trains, 60 m and 25 m in equal parts, 2.14 m sea, beach of 1 in 10, 60 s:")
    eta, drift, s, bound = beach(2.14, 0.1, [(math.sqrt(0.5), 60.0), (math.sqrt(0.5), 25.0)])
    top = waterline_top(eta, 0.1, 60)
    for x0 in (-2.0, 0.0):
        x, t = x0, 0.0
        for i in range(60 * 60):
            v = drift(x + drift(x, t)[0] * TICK / 2, t + TICK / 2)[0]
            x, t = x + v * TICK, t + TICK
        print(f"    parcel from {x0:+.1f} m ends at {x:5.2f} m; the waterline's highest reach in that time is {top:5.2f} m")

    print("  foam cells of 0.25 m, a 1 m patch of cover laid just below the still waterline, 3 waves of 60 m, 2.14 m sea;")
    print("  each cell's foam is moved along the drift for the whole step and shared between the cells it lands in")
    print("  beach     step       drift                       patch's centre   leading edge   run-up line   foam kept   most in one cell")
    for slope, nt, on in ((10, 1, False), (10, 1, True), (10, 8, True), (4, 1, True), (30, 1, True), (30, 8, True)):
        beta = 1.0 / slope
        eta, drift, s, bound = beach(2.14, beta, [(1.0, 60.0)])
        period = 2 * math.pi / math.sqrt(G * 2 * math.pi / 60.0)
        dx, n = 0.25, 160
        x = [(j - n // 2 + 0.5) * dx for j in range(n)]
        c = [1.0 if -1.0 <= p <= 0.0 else 0.0 for p in x]
        start, dt, t = sum(c), TICK * nt, 0.0
        for _ in range(int(3 * period / dt)):
            new = [0.0] * n
            for j, p in enumerate(x):
                if c[j] == 0.0:
                    continue
                q = (p + drift(p, t + dt / 2, on)[0] * dt - x[0]) / dx
                k = int(math.floor(q))
                if 0 <= k < n:
                    new[k] += c[j] * (1 - (q - k))
                if 0 <= k + 1 < n:
                    new[k + 1] += c[j] * (q - k)
            c, t = new, t + dt
        total = sum(c)
        centre = sum(p * v for p, v in zip(x, c)) / total
        lead = max(p for p, v in zip(x, c) if v > 0.01 * max(c))
        print(f"  1 in {slope:<3d}  {nt} tick{'s' if nt > 1 else ' '}    {'the current and the wind' if not on else 'with the rising sheet   '}   "
              f"{centre:+6.2f} m         {lead:+6.2f} m       {gl * s / (1 + gl) / beta:5.2f} m        {100 * total / start:5.1f} %     {max(c):4.2f}")
    print("  (first row: the transport as reviewed. More than 1 in a cell is foam gathered past full cover, which the design drops.)")


if __name__ == "__main__":
    part_h()
    part_i()
    part_j()
    part_k()
    part_l()
    part_m()
