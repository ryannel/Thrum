"""CLOUDS design revision proof, 14 November 2026.

Four small CPU checks of the mechanisms the design changed after its first review
(docs/evidence/CLOUDS/design-review.md). Each sets the rule the review refuted beside the rule
the design now states, on a sky or a shape that is not uniform.

A  the Sun-path product's columns keep a floor elevation, so nothing switches as the Sun sinks, and
   the factor fades to a far mean transmittance across a border of the field, beside a broken sky;
B  a placed cell's column query is its shape's own column, so what is drawn is bounded at
   every point for a shape that is not a dome;
C  the Sun's path integrates a coarse density that needs no detail noise;
D  a stacked volume is read slab by slab down to a stated coarsest level.

It is evidence for the design, not engine code. Constants are a first profile.
Run: python3 docs/evidence/CLOUDS/design-proof/revision_proof.py   (standard library only)
"""
import math
import random

random.seed(11)


def smooth(e0, e1, x):
    t = min(1.0, max(0.0, (x - e0) / (e1 - e0)))
    return t * t * (3.0 - 2.0 * t)


# --------------------------------------------------------------------------- A
BASE, TOP = 1000.0, 2000.0          # cloud layer, m
FLOOR = math.radians(2.0)           # the lowest elevation the product's columns follow
PITCH = 400.0                        # lattice pitch of the test sky, m
NSITE = 4096
OPEN = 300                           # sites of open sky before the cloud bank: 120 km


def make_sky(closed):
    """Extinction (1/m) at lattice points, linear between them: open sky, then gaps, edges and cores."""
    s = []
    for i in range(NSITE):
        r = random.random()
        if closed:
            s.append(0.008 + 0.03 * r)
        elif i < OPEN:
            s.append(0.0)
        else:
            s.append(0.0 if r < 0.45 else 0.04 * (r - 0.45) / 0.55)
    prefix = [0.0]
    for i in range(NSITE):
        prefix.append(prefix[-1] + 0.5 * (s[i] + s[(i + 1) % NSITE]) * PITCH)
    return s, prefix


def antiderivative(sky, x):
    s, prefix = sky
    period = NSITE * PITCH
    whole = math.floor(x / period)
    x -= whole * period
    i = min(NSITE - 1, int(x / PITCH))
    f = x / PITCH - i
    a, b = s[i], s[(i + 1) % NSITE]
    return whole * prefix[-1] + prefix[i] + PITCH * (a * f + 0.5 * (b - a) * f * f)


def slant_tau(sky, x, elev):
    """Optical depth from a ground receiver at x along a ray at elevation elev, through the layer."""
    t = math.tan(elev)
    span = antiderivative(sky, x + TOP / t) - antiderivative(sky, x + BASE / t)
    return span / math.cos(elev)


def field(sky, x, elev):
    return math.exp(-slant_tau(sky, x, elev))


def window_stats(sky, lo, hi, elev, n=4000):
    """Mean transmittance and mean optical depth of the field over a window of receivers."""
    taus = [slant_tau(sky, lo + (k + 0.5) * (hi - lo) / n, elev) for k in range(n)]
    return sum(math.exp(-t) for t in taus) / n, sum(taus) / n


def largest_step(fn, start, end, n):
    prev, worst = fn(start), 0.0
    for k in range(1, n + 1):
        cur = fn(start + (end - start) * k / n)
        worst = max(worst, abs(cur - prev))
        prev = cur
    return worst


print("A. Sun-path factor under open sky beside a broken cloud bank")
broken = make_sky(False)
far_T_low, far_tau_low = window_stats(broken, 0.0, 400000.0, FLOOR)


def elevation_old(x):
    """The refuted rule: the field down to 2 deg, one far mean optical depth below it."""
    return lambda e: field(broken, x, e) if e >= FLOOR else math.exp(-far_tau_low)


def elevation_new(x):
    """The product's columns follow the Sun down to 2 deg and keep that slope below it."""
    return lambda e: field(broken, x, max(e, FLOOR))


print("   Sun sinking from 8 deg to 1 deg: largest change between consecutive Sun steps,")
print("   for 20000 / 40000 / 80000 steps")
probes = (("under open sky", 20000.0), ("ray enters the bank", 100000.0), ("gap in the bank", None))
gap = next(x for x in (130000.0 + 37.0 * k for k in range(200000))
           if field(broken, x, math.radians(8.0)) > 0.5)
for label, rule in (("old: the field, then exp(-far mean tau) below 2 deg", elevation_old),
                    ("new: the columns' elevation is max(Sun, 2 deg); no switch", elevation_new)):
    print(f"   {label}")
    for name, x in probes:
        fn = rule(gap if x is None else x)
        steps = [largest_step(fn, math.radians(8.0), math.radians(1.0), n) for n in (20000, 40000, 80000)]
        print(f"      {name:20s} {steps[0]:.6f}  {steps[1]:.6f}  {steps[2]:.6f}"
              f"   ratios {steps[0] / max(steps[1], 1e-300):.2f}, {steps[1] / max(steps[2], 1e-300):.2f}")

print("   a receiver leaving the field's region at a 60 deg Sun: largest change between receiver steps,")
print("   for 4000 / 8000 / 16000 steps over 4 km")
E60, BORDER = math.radians(60.0), 2000.0
EDGE = next(x for x in (200000.0 + 53.0 * k for k in range(200000)) if field(broken, x, E60) > 0.99)
far_T, far_tau = window_stats(broken, EDGE - 160000.0, EDGE, E60)


def edge_old(x):
    return field(broken, x, E60) if x < EDGE else math.exp(-far_tau)


def edge_new(x):
    w = 1.0 - smooth(EDGE - BORDER, EDGE, x)
    return far_T + (field(broken, x, E60) - far_T) * w


for label, fn in (("old: the field, then exp(-far mean tau) outside", edge_old),
                  ("new: mix(far mean transmittance, field, border weight)", edge_new)):
    steps = [largest_step(fn, EDGE - 3000.0, EDGE + 1000.0, n) for n in (4000, 8000, 16000)]
    print(f"      {label}: {steps[0]:.6f}  {steps[1]:.6f}  {steps[2]:.6f}"
          f"   ratios {steps[0] / steps[1]:.2f}, {steps[1] / steps[2]:.2f}")
print(f"      far value over the 160 km window: mean transmittance {far_T:.4f}; "
      f"exp(-mean optical depth) would be {math.exp(-far_tau):.2e}")

closed = make_sky(True)
thinnest = min(closed[0]) * (TOP - BASE)
c_far, _ = window_stats(closed, 0.0, 160000.0, FLOOR, 2000)
worst = c_far
for k in range(2001):
    e = math.radians(20.0 - 21.0 * k / 2000)
    for x in (1234.0, 77777.0, 400000.0):
        worst = max(worst, field(closed, x, max(e, FLOOR)))
print(f"   closed deck, thinnest column tau {thinnest:.2f}: largest factor and far value from a 20 deg Sun "
      f"to below the horizon {worst:.2e}, against exp(-thinnest) {math.exp(-thinnest):.2e}")

# --------------------------------------------------------------------------- B
print("B. A placed cell's query is its shape's own column")
NX, NY, NZ = 20, 20, 12
DZ = 1.0


def shape_density(i, j, k):
    """An L-shaped tower with an overhang and a hollow: nothing like a dome."""
    arm_a = 3 <= i <= 15 and 3 <= j <= 7
    arm_b = 3 <= i <= 7 and 3 <= j <= 16
    tower = 4 <= i <= 6 and 12 <= j <= 15
    overhang = 9 <= i <= 14 and 9 <= j <= 12 and 7 <= k <= 9
    hollow = 6 <= i <= 8 and 4 <= j <= 6 and 2 <= k <= 4
    d = 0.0
    if (arm_a or arm_b) and 1 <= k <= 5:
        d = 0.6 + 0.4 * (((i * 7 + j * 13 + k * 5) % 10) / 10.0)
    if tower and 1 <= k <= 10:
        d = 1.0
    if overhang:
        d = 0.8
    if hollow:
        d = 0.0
    return d


grid = [[[shape_density(i, j, k) for k in range(NZ)] for j in range(NY)] for i in range(NX)]
foot = [[sum(grid[i][j]) * DZ for j in range(NY)] for i in range(NX)]


def tex(g, i, j, k):
    if 0 <= i < NX and 0 <= j < NY and 0 <= k < NZ:
        return g[i][j][k]
    return 0.0


def bilinear(f, x, y):
    i, j = math.floor(x - 0.5), math.floor(y - 0.5)
    fx, fy = x - 0.5 - i, y - 0.5 - j
    out = 0.0
    for di, wx in ((0, 1 - fx), (1, fx)):
        for dj, wy in ((0, 1 - fy), (1, fy)):
            if 0 <= i + di < NX and 0 <= j + dj < NY:
                out += wx * wy * f[i + di][j + dj]
    return out


def trilinear(g, x, y, z):
    i, j, k = math.floor(x - 0.5), math.floor(y - 0.5), math.floor(z - 0.5)
    fx, fy, fz = x - 0.5 - i, y - 0.5 - j, z - 0.5 - k
    out = 0.0
    for di, wx in ((0, 1 - fx), (1, fx)):
        for dj, wy in ((0, 1 - fy), (1, fy)):
            for dk, wz in ((0, 1 - fz), (1, fz)):
                out += wx * wy * wz * tex(g, i + di, j + dj, k + dk)
    return out


def erosion(x, y, z):
    """A factor in [0, 1], different at every point: noise only removes."""
    return 0.5 + 0.5 * math.sin(12.9898 * x + 78.233 * y + 37.719 * z) ** 2


def volume_column(x, y, eroded):
    n = (NZ + 1) * 8                      # kinks of the trilinear read fall on sample-cell edges
    total, h = 0.0, (NZ + 1.0) / n
    for s in range(n):
        z = -0.5 + (s + 0.5) * h
        total += trilinear(grid, x, y, z) * (erosion(x, y, z) if eroded else 1.0) * h
    return total


def column_form(x, y, eroded):
    """The far form: the footprint's column spread over a vertical profile, then eroded."""
    q = bilinear(foot, x, y)
    n, total = 48, 0.0
    for s in range(n):
        u = (s + 0.5) / n
        profile = 6.0 * u * (1.0 - u)            # integrates to 1 over the column
        total += q * profile * (erosion(x, y, u * NZ) if eroded else 1.0) / n
    return total


points = [(random.uniform(0, NX), random.uniform(0, NY)) for _ in range(1500)]
exact = max(abs(volume_column(x, y, False) - bilinear(foot, x, y)) for x, y in points)
print(f"   volume form before erosion against the query, 1500 points: largest difference {exact:.2e}")
worst_over, clear_drawn = -1e9, 0
for x, y in points:
    q = bilinear(foot, x, y)
    v, c = volume_column(x, y, True), column_form(x, y, True)
    for w in (0.0, 0.25, 0.5, 1.0):
        drawn = w * v + (1.0 - w) * c
        worst_over = max(worst_over, drawn - q)
        if q == 0.0 and drawn > 0.0:
            clear_drawn += 1
print(f"   eroded volume form, column form and blends at weights 0, 0.25, 0.5, 1: "
      f"largest (drawn - query) {worst_over:.2e}; drawn where the query is clear: {clear_drawn}")

mean_foot = sum(map(sum, foot)) / (NX * NY)
cx, cy, rad = NX / 2.0, NY / 2.0, NX / 2.0


def dome(x, y):
    r2 = ((x - cx) ** 2 + (y - cy) ** 2) / rad ** 2
    return max(0.0, 1.0 - r2)


dome_mean = sum(dome(i + 0.5, j + 0.5) for i in range(NX) for j in range(NY)) / (NX * NY)
k_dome = mean_foot / dome_mean
over = [bilinear(foot, x, y) - k_dome * dome(x, y) for x, y in points]
print(f"   the refuted rule, a dome of the same mean column as the query: placed column exceeds it at "
      f"{sum(1 for o in over if o > 1e-9) / len(over):.0%} of points, by up to {max(over):.2f} "
      f"(mean column {mean_foot:.2f})")

coarse = [[[sum(tex(grid, 2 * i + a, 2 * j + b, 2 * k + c) for a in (0, 1) for b in (0, 1) for c in (0, 1)) / 8
            for k in range(NZ // 2)] for j in range(NY // 2)] for i in range(NX // 2)]


def coarse_column(x, y):
    i, j = math.floor(x / 2 - 0.5), math.floor(y / 2 - 0.5)
    fx, fy = x / 2 - 0.5 - i, y / 2 - 0.5 - j
    out = 0.0
    for di, wx in ((0, 1 - fx), (1, fx)):
        for dj, wy in ((0, 1 - fy), (1, fy)):
            if 0 <= i + di < NX // 2 and 0 <= j + dj < NY // 2:
                out += wx * wy * sum(coarse[i + di][j + dj]) * 2 * DZ
    return out


filt = max(coarse_column(x, y) - bilinear(foot, x, y) for x, y in points)
print(f"   a read one mip coarser is a local average and exceeds the point query by up to {filt:.2f} "
      f"at an edge: that is the GPU's measured bound, not the model's")

# --------------------------------------------------------------------------- C
print("C. The Sun's path integrates a coarse density that needs no detail noise")
SIG, STRENGTH, EDGE_W, COL = 0.05, 0.6, 150.0, 80.0
noise = [random.random() for _ in range(1 << 16)]
MEAN_N = sum(noise) / len(noise)


def detail(x, y):
    """Value noise with an 8 m lattice."""
    i, j = math.floor(x / 8.0), math.floor(y / 8.0)
    fx, fy = x / 8.0 - i, y / 8.0 - j
    def n(a, b):
        return noise[(a * 7919 + b * 104729) & 0xFFFF]
    return (n(i, j) * (1 - fx) + n(i + 1, j) * fx) * (1 - fy) + (n(i, j + 1) * (1 - fx) + n(i + 1, j + 1) * fx) * fy


def eroded(profile, n):
    """Edge erosion: noise removes extinction where the profile is low, and none in the interior."""
    cut = n * STRENGTH * (1.0 - profile)
    return max(0.0, (profile - cut) / (1.0 - cut)) if cut < 1.0 else 0.0


def path_tau(y, mode):
    total, n = 0.0, 150
    for s in range(n):
        x = (s + 0.5) * EDGE_W / n
        p = smooth(0.0, EDGE_W, x)
        d = {"full": eroded(p, detail(x, y)), "coarse": eroded(p, MEAN_N), "none": p}[mode]
        total += SIG * d * EDGE_W / n
    return total


rows = []
for c in range(40):
    ys = [c * 400.0 + (k + 0.5) * COL / 16 for k in range(16)]
    full = [path_tau(y, "full") for y in ys]
    rows.append((sum(math.exp(-t) for t in full) / 16, math.exp(-path_tau(ys[0], "coarse")),
                 math.exp(-path_tau(ys[0], "none")), max(full) - min(full)))
rms = math.sqrt(sum((a - b) ** 2 for a, b, _, _ in rows) / len(rows))
rms_none = math.sqrt(sum((a - c) ** 2 for a, _, c, _ in rows) / len(rows))
print(f"   150 m edge paths: tau with no erosion {path_tau(0.0, 'none'):.2f}, with detail erosion "
      f"{min(path_tau(c * 400.0, 'full') for c in range(40)):.2f} to "
      f"{max(path_tau(c * 400.0, 'full') for c in range(40)):.2f}, coarse {path_tau(0.0, 'coarse'):.2f}")
print(f"   transmittance of an 80 m column, mean of the full density's paths against one coarse path: "
      f"RMS {rms:.4f}; against a path with erosion left out: RMS {rms_none:.4f}")
print("   the coarse path reads the profile and one constant, so both profiles compute the same number")

# --------------------------------------------------------------------------- D
print("D. A stacked volume read slab by slab")
SLABS, DEPTH = 12, 128
levels = [[(255 if s % 2 else 0) for s in range(SLABS) for _ in range(DEPTH)]]
while len(levels[-1]) > 1:
    z = levels[-1]
    levels.append([(z[2 * i] + z[min(2 * i + 1, len(z) - 1)] + 1) // 2 for i in range(max(1, len(z) // 2))])


def read(level, slab, u, clamp_level):
    """Linear read along the stacking axis at one level; u in [0, 1] inside the slab."""
    z = levels[level]
    per = DEPTH >> level
    half = 0.5 / (DEPTH >> clamp_level)
    u = min(1.0 - half, max(half, u))
    pos = (slab + u) * per - 0.5
    i = math.floor(pos)
    f = pos - i
    a = z[min(len(z) - 1, max(0, i))]
    b = z[min(len(z) - 1, max(0, i + 1))]
    return a + (b - a) * f


CAP = int(math.log2(DEPTH))
worst_in, worst_unclamped = 0.0, 0.0
for level in range(CAP + 1):
    for slab in range(SLABS):
        want = 255 if slab % 2 else 0
        for k in range(201):
            u = k / 200
            lo_l, hi_l = level, min(CAP, level + 1)
            blended = 0.5 * read(lo_l, slab, u, hi_l) + 0.5 * read(hi_l, slab, u, hi_l)
            worst_in = max(worst_in, abs(blended - want))
            pos = (slab + u) * (DEPTH >> level) - 0.5
            i = math.floor(pos)
            z = levels[level]
            raw = z[min(len(z) - 1, max(0, i))] * (1 - (pos - i)) + z[min(len(z) - 1, max(0, i + 1))] * (pos - i)
            worst_unclamped = max(worst_unclamped, abs(raw - want))
print(f"   twelve slabs of 128, alternately 0 and 255; levels 0 to {CAP}, each blended with the next, "
      f"coordinate held half a texel of the coarser level inside the slab: largest error {worst_in:g}")
print(f"   the same reads with no clamp: largest error {worst_unclamped:g}")
print(f"   level {CAP + 1} holds {len(levels[CAP + 1])} texels with values {levels[CAP + 1]}: "
      f"past the cap every texel mixes two shapes")
