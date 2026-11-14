"""CLOUDS design, second revision proof, 14 November 2026.

Two CPU checks of the mechanisms the design changed after its second review
(docs/evidence/CLOUDS/design-review.md), each on the review's own counterexample and on a wider case.

G  the first order's phase is the droplet's phase averaged over the Sun's disc, not a widened lobe:
   it integrates to 1 over the sphere across the supported droplet range, its mean cosine has a
   closed form, and it changes the point-Sun phase only within about a degree of the Sun;
H  the Sun's path over a placed shape integrates the form that is drawn, so it reads the lent
   volume: under an oblique Sun a footprint cannot give the volume's shade; the volume read at its
   finest level along each column's ray does, and the volume averaged to a column's width does not.

It is evidence for the design, not engine code. Constants are a first profile.
Run: python3 docs/evidence/CLOUDS/design-proof/second_revision_proof.py   (standard library only;
under a minute)
"""
import math

PI = math.pi
RHO = 0.004654                       # the Sun's angular radius, rad
DISC = 2 * PI * (1 - math.cos(RHO))  # its solid angle, sr


# --------------------------------------------------------------------------- G
def fit(d_um):
    """Jendersie and d'Eon 2023, the fits for 5 <= d <= 50 micrometres: g_HG, g_D, alpha, w_D."""
    return (math.exp(-0.0990567 / (d_um - 1.67154)),
            math.exp(-2.20679 / (d_um + 3.91029) - 0.428934),
            math.exp(3.62489 - 8.29288 / (d_um + 5.52825)),
            math.exp(-0.599085 / (d_um - 0.641583) - 0.665888))


def hg(s2, g, spread=0.0):
    return (1 - g * g) / (4 * PI * ((1 - g) ** 2 + g * (4 * s2 + spread)) ** 1.5)


def point_phase(s2, m, spread=0.0):
    """The droplet's phase for a point Sun at s2 = sin^2(theta/2). `spread` is the refuted rule."""
    g_hg, g_d, alpha, w = m
    c = 1 - 2 * s2
    draine = hg(s2, g_d) * (1 + alpha * c * c) / (1 + alpha * (1 + 2 * g_d * g_d) / 3)
    return (1 - w) * hg(s2, g_hg, spread) + w * draine


def point_mean_cosine(m):
    g_hg, g_d, alpha, w = m
    draine = g_d * (1 + alpha * (3 + 2 * g_d * g_d) / 5) / (1 + alpha * (1 + 2 * g_d * g_d) / 3)
    return (1 - w) * g_hg + w * draine


def gauss_legendre(n):
    nodes = []
    for i in range(n):
        x = math.cos(PI * (i + 0.75) / (n + 0.5))
        for _ in range(60):
            p0, p1 = 1.0, x
            for k in range(2, n + 1):
                p0, p1 = p1, ((2 * k - 1) * x * p1 - (k - 1) * p0) / k
            dp = n * (x * p1 - p0) / (x * x - 1)
            x -= p1 / dp
        nodes.append((x, 2 / ((1 - x * x) * dp * dp)))
    return nodes


def disc_rule(nr, nphi):
    """Points of the Sun's disc as (sin r, r, hav(phi), weight); the weights sum to 1."""
    rule = []
    for x, wx in gauss_legendre(nr):
        r = 0.5 * RHO * (x + 1)
        wr = 0.5 * RHO * wx * math.sin(r)
        for j in range(nphi):
            phi = (j + 0.5) * PI / nphi
            rule.append((math.sin(r), r, math.sin(phi / 2) ** 2, wr * 2 * PI / nphi / DISC))
    return rule


FINE, WIDE = disc_rule(24, 48), disc_rule(6, 12)


def disc_phase(theta, m):
    """The phase averaged over the Sun's disc: the mean of the point phase over the directions to it."""
    st, total = math.sin(theta), 0.0
    for sr, r, hav, w in (FINE if theta < 12 * RHO else WIDE):
        # haversine of the angle between the view ray's Sun angle and one point of the disc
        s2 = math.sin((theta - r) / 2) ** 2 + st * sr * hav
        total += w * point_phase(s2, m)
    return total


def sphere(fn, n):
    """Integral over the sphere and mean cosine of fn(theta), by the midpoint rule in u = sin(theta/2)^(1/2)."""
    total = mean = 0.0
    for i in range(n):
        u = (i + 0.5) / n
        s2 = u ** 4
        theta = 2 * math.asin(math.sqrt(s2))
        w = 16 * PI * u ** 3 / n          # d(solid angle) = 4 pi d(s2)
        p = fn(theta)
        total += p * w
        mean += p * (1 - 2 * s2) * w
    return total, mean / total


def spread_integral(m, spread):
    """Closed-form integral of the refuted rule, the lobe widened by `spread`, over the sphere."""
    g, _, _, w = m
    a = (1 - g) ** 2 + g * spread
    return (1 - w) * (1 - g * g) / (2 * g) * (1 / math.sqrt(a) - 1 / math.sqrt(a + 4 * g)) + w


def peak_exact(m):
    """The disc average at the Sun's centre: the lobes' closed-form energy inside the disc over its solid angle."""
    g, _, _, w = m
    inside = (1 - g * g) / (2 * g) * (1 / (1 - g) - 1 / math.sqrt(1 + g * g - 2 * g * math.cos(RHO)))
    draine = sum(wt * (point_phase(math.sin(r / 2) ** 2, m) - (1 - w) * hg(math.sin(r / 2) ** 2, g))
                 for _, r, _, wt in FINE)
    return (1 - w) * inside / DISC + draine


def part_g():
    print("G. The first order's phase, averaged over the Sun's disc (angular radius 0.2667 degrees)")
    print("   integral over the sphere and mean cosine; the closed form is the point Sun's mean cosine times")
    print("   (1 + cos rho) / 2, the disc's own mean cosine")
    print("   diameter   refuted rule   disc average   mean cosine   closed form   point Sun")
    for d in (5.0, 10.0, 20.0, 40.0, 50.0):
        m = fit(d)
        total, mean = sphere(lambda t: disc_phase(t, m), 1500)
        closed = point_mean_cosine(m) * (1 + math.cos(RHO)) / 2
        print(f"   {d:5.0f} um {spread_integral(m, RHO * RHO / 2):13.6f} {total:14.6f} {mean:13.6f} "
              f"{closed:13.6f} {point_mean_cosine(m):11.6f}")
    m50 = fit(50.0)
    a, b = sphere(lambda t: disc_phase(t, m50), 750)[0], sphere(lambda t: disc_phase(t, m50), 3000)[0]
    print(f"   50 um at 750 and 3000 angles: {a:.6f}, {b:.6f}")
    print("   at the Sun's centre, sr^-1: point Sun, disc average by quadrature, by closed form, refuted rule")
    for d in (5.0, 20.0, 50.0):
        m = fit(d)
        print(f"   {d:5.0f} um {point_phase(0.0, m):10.1f} {disc_phase(0.0, m):10.1f} {peak_exact(m):10.1f} "
              f"{point_phase(0.0, m, RHO * RHO / 2):10.1f}")
    print("   where the disc matters: disc average over point Sun, and two closed forms over the disc average")
    print("   (the refuted rule, and the same lobe scaled to unit energy, which is a lobe of lower g)")
    print("   diameter  theta    disc/point   refuted/disc   rescaled/disc")
    for d in (20.0, 50.0):
        m = fit(d)
        g, w = m[0], m[3]
        scale = (1 - w) * ((1 - g * g) / (2 * g) * (1 / math.sqrt((1 - g) ** 2 + g * RHO * RHO / 2)
                                                    - 1 / math.sqrt((1 - g) ** 2 + g * RHO * RHO / 2 + 4 * g)))
        for deg in (0.0, 0.1, 0.2667, 0.5, 1.0, 2.0, 5.0, 20.0):
            theta = math.radians(deg)
            s2 = math.sin(theta / 2) ** 2
            exact = disc_phase(theta, m)
            old = point_phase(s2, m, RHO * RHO / 2)
            lobe = (1 - w) * hg(s2, g, RHO * RHO / 2)
            rescaled = old - lobe + lobe * (1 - w) / scale
            print(f"   {d:5.0f} um {deg:6.2f} {exact / point_phase(s2, m):12.4f} {old / exact:14.4f} "
                  f"{rescaled / exact:15.4f}")
    print("   part E of first_principles_proof.py again, with the disc average in place of the refuted rule:")
    print("   reference = HG(0.86); droplet = a 20 um droplet's phase averaged over the Sun's disc, sr^-1")
    print("   theta   reference    droplet   reference/droplet")
    m20 = fit(20.0)
    for deg in (0.0, 0.5, 2.0, 5.0, 10.0, 20.0, 45.0, 90.0, 140.0, 180.0):
        theta = math.radians(deg)
        ref, dro = hg(math.sin(theta / 2) ** 2, 0.86), disc_phase(theta, m20)
        print(f"   {deg:5.1f} {ref:11.4f} {dro:10.4f} {ref / dro:13.3f}")
    print("   a table over u = sin(theta/2)^(1/2), read with linear interpolation, for a 50 um droplet:")
    n = 1024
    fine = [disc_phase(2 * math.asin((i / n) ** 2), m50) for i in range(n + 1)]
    mids = [disc_phase(2 * math.asin(((i + 0.5) / n) ** 2), m50) for i in range(n)]
    for size, step in ((n, 1), (n // 4, 4), (n // 16, 16)):
        table = fine[::step]
        total = 0.0
        for i in range(size):
            for k in range(8):
                t = (k + 0.5) / 8
                u = (i + t) / size
                total += (table[i] + (table[i + 1] - table[i]) * t) * 16 * PI * u ** 3 / (8 * size)
        worst = 0.0
        for i in range(n):
            j, t = divmod(i + 0.5, step)
            j = int(j)
            read = table[j] + (table[j + 1] - table[j]) * t / step
            if step == 1 or (i + 0.5) % step:
                worst = max(worst, abs(read / mids[i] - 1))
        print(f"      {size + 1:5d} entries: integral {total:.6f}, largest relative error at the midpoints {worst:.2e}")
    print()


# --------------------------------------------------------------------------- H
VOX = 20.0                           # voxel, m
NXV, NZV = 128, 64                   # a shape 2,560 m long and 1,280 m tall, extruded across the Sun
COLUMN = 80.0                        # the product's near column, m
SLICES = 48


def shape_density(x, z):
    """A cumulus that is not uniform: a body, a dense core off-centre, a shelf with clear air under it, a hollow."""
    def ellipse(cx, cz, rx, rz):
        return ((x - cx) / rx) ** 2 + ((z - cz) / rz) ** 2 < 1.0
    d = 0.0
    if ellipse(1100.0, 500.0, 800.0, 450.0) and z >= 100.0:
        d = 0.5
    if ellipse(800.0, 700.0, 300.0, 250.0):
        d = 1.0
    if 1700.0 <= x <= 2400.0 and 820.0 <= z <= 1000.0:
        d = 0.7
    if ellipse(1300.0, 350.0, 200.0, 150.0):
        d = 0.0
    return d


GRID = [[shape_density((i + 0.5) * VOX, (k + 0.5) * VOX) for k in range(NZV)] for i in range(NXV)]


def mip(grid, times):
    for _ in range(times):
        grid = [[(grid[2 * i][2 * k] + grid[2 * i + 1][2 * k] + grid[2 * i][2 * k + 1] + grid[2 * i + 1][2 * k + 1]) / 4
                 for k in range(len(grid[0]) // 2)] for i in range(len(grid) // 2)]
    return grid


def soften(grid, times):
    """A 3 x 3 box filter, `times` over: edges that fade across a few voxels."""
    for _ in range(times):
        nx, nz = len(grid), len(grid[0])
        grid = [[sum(grid[min(nx - 1, max(0, i + a))][min(nz - 1, max(0, k + b))]
                     for a in (-1, 0, 1) for b in (-1, 0, 1)) / 9 for k in range(nz)] for i in range(nx)]
    return grid


def bilinear(grid, vox, x, z):
    nx, nz = len(grid), len(grid[0])
    i, k = math.floor(x / vox - 0.5), math.floor(z / vox - 0.5)
    fx, fz = x / vox - 0.5 - i, z / vox - 0.5 - k
    out = 0.0
    for di, wx in ((0, 1 - fx), (1, fx)):
        for dk, wz in ((0, 1 - fz), (1, fz)):
            if 0 <= i + di < nx and 0 <= k + dk < nz:
                out += wx * wz * grid[i + di][k + dk]
    return out


def footprint(grid, vox):
    """Each column's sum and its lowest and highest occupied height: all a footprint holds."""
    out = []
    for col in grid:
        occupied = [k for k, v in enumerate(col) if v > 0.0]
        out.append((sum(col) * vox, occupied[0] * vox, (occupied[-1] + 1) * vox) if occupied else (0.0, 0.0, 0.0))
    return out


FOOT = footprint(GRID, VOX)


def column_form(x, z):
    """The far form: the footprint's column spread over a vertical profile between its lowest and highest heights."""
    i = math.floor(x / VOX)
    if not 0 <= i < NXV:
        return 0.0
    total, lo, hi = FOOT[i]
    if hi <= lo or not lo < z < hi:
        return 0.0
    u = (z - lo) / (hi - lo)
    return total * 6.0 * u * (1.0 - u) / (hi - lo)


def sun_tau(density, x, z, elev, step):
    """Optical depth (per unit extinction) from (x, z) toward the Sun, which stands over +x."""
    top = NZV * VOX
    n = max(1, int((top - z) / math.sin(elev) / step))
    ds = (top - z) / math.sin(elev) / n
    cx, cz = math.cos(elev) * ds, math.sin(elev) * ds
    return sum(density(x + (s + 0.5) * cx, z + (s + 0.5) * cz) for s in range(n)) * ds


def build_product(elev, density):
    """tau(column, slice): columns are Sun rays indexed where they cross the layer's bottom, marched through `density`."""
    dz = NZV * VOX / SLICES
    lo = -int(NZV * VOX / math.tan(elev) / COLUMN) - 2
    hi = int(NXV * VOX / COLUMN) + 2
    product = {}
    for c in range(lo, hi + 1):
        x0 = c * COLUMN
        tau, column = 0.0, [0.0] * (SLICES + 1)
        for k in range(SLICES - 1, -1, -1):
            for s in range(4):
                z = (k + (s + 0.5) / 4) * dz
                tau += density(x0 + z / math.tan(elev), z) * dz / 4 / math.sin(elev)
            column[k] = tau
        product[c] = column
    return product, lo, hi, dz


def read_product(prod, x, z, elev):
    product, lo, hi, dz = prod
    c = (x - z / math.tan(elev)) / COLUMN
    i = max(lo, min(hi - 1, math.floor(c)))
    f = min(1.0, max(0.0, c - i))
    k = min(SLICES - 1, max(0, math.floor(z / dz)))
    g = min(1.0, max(0.0, z / dz - k))
    a = product[i][k] + (product[i][k + 1] - product[i][k]) * g
    b = product[i + 1][k] + (product[i + 1][k + 1] - product[i + 1][k]) * g
    return a + (b - a) * f


def rms_max(pairs):
    diffs = [abs(a - b) for a, b in pairs]
    return math.sqrt(sum(d * d for d in diffs) / len(diffs)), max(diffs)


def part_h():
    print("H. The Sun's path over a placed shape")
    print("   the review's two shapes: equal footprints, a 45 degree Sun, the ray through their middles")
    a = ((.002, .014, .002, .002), (.002, .002, .014, .002))
    b = ((.002, .002, .014, .002), (.002, .014, .002, .002))
    for name, shape in (("A", a), ("B", b)):
        def volume(x, z, shape=shape):
            i, k = math.floor(x / 100.0), math.floor(z / 100.0)
            return shape[i][k] if 0 <= i < 2 and 0 <= k < 4 else 0.0
        tau = sun_tau(volume, -100.0, 0.0, math.radians(45.0), 0.01)
        column = sum(shape[0]) * 100.0
        print(f"      shape {name}: footprint columns {sum(shape[0]) * 100:.1f} and {sum(shape[1]) * 100:.1f}, "
              f"0 to 400 m; the volume's Sun path {tau:.6f}, transmittance {math.exp(-tau):.6f}")
    def flat(x, z):
        return column / 400.0 if 0.0 <= x < 200.0 and 0.0 <= z < 400.0 else 0.0
    tau = sun_tau(flat, -100.0, 0.0, math.radians(45.0), 0.01)
    print(f"      a path made from the footprint is one number for both: {tau:.6f}, transmittance {math.exp(-tau):.6f}")

    print("   a cumulus of 20 m voxels with a dense core, a shelf and a hollow, extruded across the Sun, with hard")
    print("   edges and with edges softened over about 100 m. Three products of 80 m columns: filled from the volume")
    print("   at its finest level along each column's ray, from the volume averaged to 80 m, and from the column")
    print("   form. Each against a 5 m march of the volume itself averaged over a column's width: differences in")
    print("   transmittance on the ground and at points inside the cloud, as RMS / largest")
    global FOOT
    offsets = [(j - 3) * COLUMN / 7 for j in range(7)]
    for edges, grid in (("hard edges", GRID), ("soft edges", soften(GRID, 3))):
        FOOT = footprint(grid, VOX)
        coarse = mip(grid, 2)
        fills = (("finest level", lambda x, z: bilinear(grid, VOX, x, z)),
                 ("averaged to 80 m", lambda x, z: bilinear(coarse, 4 * VOX, x, z)),
                 ("column form", column_form))
        full = fills[0][1]
        for label, sigma in (("thin, extinction 0.004 1/m", 0.004), ("the type's own, 0.06 1/m", 0.06)):
            for deg in (45.0, 20.0):
                elev = math.radians(deg)
                reach = NZV * VOX / math.tan(elev)
                ground = [(-reach + (r + 0.5) * (reach + NXV * VOX) / 150, 0.0) for r in range(150)]
                inside = [((i + 0.5) * 80.0, (k + 0.5) * 80.0) for i in range(32) for k in range(16)
                          if full((i + 0.5) * 80.0, (k + 0.5) * 80.0) > 0.02]
                truths = [[sum(math.exp(-sigma * sun_tau(full, x + o, z, elev, 5.0)) for o in offsets) / 7
                           for x, z in points] for points in (ground, inside)]
                print(f"   {edges}, {label}, Sun {deg:.0f} deg")
                for name, density in fills:
                    prod = build_product(elev, density)
                    out = [rms_max([(math.exp(-sigma * read_product(prod, x, z, elev)), t)
                                    for (x, z), t in zip(points, truth)])
                           for points, truth in zip((ground, inside), truths)]
                    print(f"      {name:17s} ground {out[0][0]:.3f} / {out[0][1]:.2f}   inside {out[1][0]:.3f} / {out[1][1]:.2f}")
    print("   The product is filled from the forms the weather map names, with their weights, whether or not the")
    print("   trace is declared, so the two profiles compute the same numbers from the same lent volume.")


if __name__ == "__main__":
    part_g()
    part_h()
