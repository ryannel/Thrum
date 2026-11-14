"""WATERS design, second revision of 14 November 2026, for light under water: reproductions R to T
(N to Q are in surf_view_proof.py, H to M in water_proof.py, A to G in ../../SEA/design-proof).
Pure Python, no dependencies. About half a minute to run.

Part R  Caustics. Sunlight refracted through a rippled surface, in a vertical section. The reference
        carries every wave from 2 cm to 9 m, on a grid sixteen times finer than the map's, with the
        Sun's disc. The map is the design's: cells of the finest cascade (9 m / 256), waves of 7 cm and
        longer, each cell's light spread over the stretch it lands on, then blurred by depth times the
        slope of the waves the field does not resolve.
        R1: one sinusoid focuses at the depth the curvature gives. R2: light is conserved, and the
        map's contrast against the reference by depth and wind. R3: a ladder of depths, read between
        its rungs, a factor of 2 and of the square root of 2 apart. R4: shallower than the first rung
        the pattern scales with depth. R5: the substitutes: one plane read at every depth, and a
        backward lookup of one sheet.
Part S  Shafts. The same field, read through the water's volume along view rays from a lens 3 m
        down. S1: how much the Sun's scattered light varies from ray to ray, looking up along the
        refracted Sun, level, and across the beams. S2: a march of 16 and of 32 jittered steps
        through the ladder against a fine integral through the exact field.
Part T  The meter. The light a level incident meter reads at the lens, by depth and water type, and
        how far the picture darkens for a camera that follows that light by a share.

Run: python3 under_light_proof.py
"""
import math
import random

N_W = 1.333
TEX = 9.0 / 256          # the finest cascade's cell, metres


# ---------------------------------------------------------------- refraction and the surface

def refract(sin_i, cos_i, slope):
    """A ray going down at the zenith angle, meeting a surface of this slope in the section.
    Returns the refracted ray's horizontal travel per metre of depth and the flux it carries per
    metre of still surface (the tilt toward the Sun and Fresnel's loss)."""
    inv = 1.0 / math.sqrt(1.0 + slope * slope)
    nx, nz = -slope * inv, inv
    ix, iz = sin_i, -cos_i
    ci = -(ix * nx + iz * nz)
    eta = 1.0 / N_W
    k = 1.0 - eta * eta * (1.0 - ci * ci)
    ct = math.sqrt(k)
    g = eta * ci - ct
    tx, tz = eta * ix + g * nx, eta * iz + g * nz
    rs = ((ci - N_W * ct) / (ci + N_W * ct)) ** 2
    rp = ((N_W * ci - ct) / (N_W * ci + ct)) ** 2
    return tx / -tz, (cos_i + slope * sin_i) * (1.0 - 0.5 * (rs + rp))


def waves(wind, lam_lo, lam_hi, seed, count, gain=1.0):
    """A section of a wind-ruffled surface: equal slope variance in each octave from 2 cm to 9 m,
    summing to half of Cox and Munk's mean square slope for this wind (one of two directions)."""
    rnd = random.Random(seed)
    mss = 0.5 * (0.003 + 0.00512 * wind)
    whole = math.log(9.0 / 0.02)
    k_lo, k_hi = 2 * math.pi / lam_hi, 2 * math.pi / lam_lo
    out = []
    for i in range(count):
        f0 = k_lo * (k_hi / k_lo) ** (i / count)
        f1 = k_lo * (k_hi / k_lo) ** ((i + 1) / count)
        k = math.sqrt(f0 * f1) * (1 + 0.1 * (rnd.random() - 0.5))
        m = mss * math.log(f1 / f0) / whole
        out.append((gain * math.sqrt(2 * m) / k, k, rnd.random() * 2 * math.pi))
    return out


def sample(comps, xs):
    hs, ss, cs = [], [], []
    for x in xs:
        h = s = c = 0.0
        for a, k, p in comps:
            ph = k * x + p
            sn = math.sin(ph)
            h += a * sn
            s += a * k * math.cos(ph)
            c -= a * k * k * sn
        hs.append(h)
        ss.append(s)
        cs.append(c)
    return hs, ss, cs


def carry(gx, gh, gs, depth, zen, lo, bins):
    """The map's mechanism. Each cell between two grid points is carried down its refracted rays
    to this depth, and its light is spread evenly over the stretch it lands on. Sheets that cross
    add. Bins follow the mean refracted ray, so a beam keeps its bin at every depth."""
    si, co = math.sin(zen), math.cos(zen)
    m, w0 = refract(si, co, 0.0)
    land, wt = [], []
    for x, h, s in zip(gx, gh, gs):
        t, w = refract(si, co, s)
        land.append(x + (h + depth) * t - depth * m)
        wt.append(w)
    out = [0.0] * bins
    for i in range(len(gx) - 1):
        a, b = land[i], land[i + 1]
        if a > b:
            a, b = b, a
        flux = 0.5 * (wt[i] + wt[i + 1]) * (gx[i + 1] - gx[i])
        ia, ib = (a - lo) / TEX, (b - lo) / TEX
        if ib - ia < 1e-9:
            j = int(math.floor(ia))
            if 0 <= j < bins:
                out[j] += flux
            continue
        for j in range(int(math.floor(ia)), int(math.floor(ib)) + 1):
            ov = min(ib, j + 1) - max(ia, j)
            if ov > 0 and 0 <= j < bins:
                out[j] += flux * ov / (ib - ia)
    return [v / (w0 * TEX) for v in out]


def blur(v, sigma):
    """A Gaussian of this many bins; beyond the ends the light is taken as even."""
    if sigma < 0.05:
        return v[:]
    r = int(3 * sigma) + 1
    k = [math.exp(-0.5 * (i / sigma) ** 2) for i in range(-r, r + 1)]
    s = sum(k)
    k = [x / s for x in k]
    n = len(v)
    out = []
    for i in range(n):
        acc = 0.0
        for j, w in enumerate(k):
            q = i + j - r
            acc += w * (v[q] if 0 <= q < n else 1.0)
        out.append(acc)
    return out


def reference(xs, hs, ss, depth, zen, lo, bins):
    """Every wave, on the fine grid, from three directions across the Sun's disc."""
    acc = [0.0] * bins
    m0 = refract(math.sin(zen), math.cos(zen), 0.0)[0]
    for off in (-0.0035, 0.0, 0.0035):
        m1 = refract(math.sin(zen + off), math.cos(zen + off), 0.0)[0]
        f = carry(xs, hs, ss, depth, zen + off, lo + depth * (m0 - m1), bins)
        acc = [a + b for a, b in zip(acc, f)]
    return [a / 3 for a in acc]


def stats(est, tru, a, b):
    e, t = est[a:b], tru[a:b]
    n = len(e)
    me, mt = sum(e) / n, sum(t) / n
    ve = sum((x - me) ** 2 for x in e) / n
    vt = sum((x - mt) ** 2 for x in t) / n
    cov = sum((x - me) * (y - mt) for x, y in zip(e, t)) / n
    return me, mt, math.sqrt(ve), math.sqrt(vt), cov / math.sqrt(ve * vt + 1e-30)


ZEN = math.radians(30)
M0 = refract(math.sin(ZEN), math.cos(ZEN), 0.0)[0]
SHIFT = (refract(math.sin(ZEN), math.cos(ZEN), 0.01)[0] - M0) / 0.01   # travel per slope per metre
LENGTH, LO = 40.0, 8.0
BINS = int(24.0 / TEX)
A, B = 80, BINS - 80
STEP = 4                          # the fields below are a quarter of an octave apart
FIRST = 0.25                      # the ladder's first rung, metres


class Field:
    """One wind's surface, its reference and its map at depths a quarter octave apart."""

    def __init__(self, wind, gain=1.0):
        full = waves(wind, 0.02, 9.0, 7, 64, gain)
        self.res = [c for c in full if 2 * math.pi / c[1] >= 0.07]
        unres = [c for c in full if 2 * math.pi / c[1] < 0.07]
        self.sig = math.sqrt(sum(0.5 * (a * k) ** 2 for a, k, _ in unres))
        dx = TEX / 16
        self.xs = [i * dx for i in range(int(LENGTH / dx))]
        self.hs, self.ss, _ = sample(full, self.xs)
        self.gx = self.xs[::16]
        self.gh, self.gs, self.gc = sample(self.res, self.gx)
        self.rms = math.sqrt(sum(s * s for s in self.ss) / len(self.ss))
        self.maps = {}

    def depth(self, j):
        return FIRST * 2 ** (j / STEP)

    def map_at(self, d):
        key = round(d, 4)
        if key not in self.maps:
            raw = carry(self.gx, self.gh, self.gs, d, ZEN, LO, BINS)
            sun = d * 0.0035 * SHIFT / 0.25
            self.maps[key] = blur(raw, math.hypot(d * abs(SHIFT) * self.sig, sun) / TEX)
        return self.maps[key]

    def ladder(self, d, per_octave=2):
        """The design's read: rungs a fixed ratio apart, mixed in the logarithm of depth; shallower
        than the first rung the departure from even light scales with depth."""
        if d <= FIRST:
            base = self.map_at(FIRST)
            return [1.0 + (v - 1.0) * d / FIRST for v in base]
        f = math.log2(d / FIRST) * per_octave
        j = int(math.floor(f))
        t = f - j
        lo_map = self.map_at(FIRST * 2 ** (j / per_octave))
        hi_map = self.map_at(FIRST * 2 ** ((j + 1) / per_octave))
        return [(1 - t) * a + t * b for a, b in zip(lo_map, hi_map)]


def part_r(fields):
    print("PART R  caustics")
    print(f"  Sun 30 degrees from the zenith; a ray travels {M0:.3f} m a metre of depth, and {SHIFT:.3f} m more")
    print("  for each unit of slope. Map cell", round(TEX * 100, 2), "cm.")

    print("R1  one sinusoid: where its light is brightest, against 1 / (shift x a k^2)")
    for lam, amp in ((0.5, 0.01), (2.0, 0.05)):
        k = 2 * math.pi / lam
        dx = lam / 400
        xs = [i * dx for i in range(int(12 * lam / dx))]
        hs, ss, _ = sample([(amp, k, 0.3)], xs)
        best = (0.0, 0.0)
        focus = 1.0 / (SHIFT * amp * k * k)
        for i in range(10, 61):
            d = focus * i / 40
            bins = int(8 * lam / (lam / 40))
            land = [0.0] * bins
            si, co = math.sin(ZEN), math.cos(ZEN)
            for x, h, s in zip(xs, hs, ss):
                t, w = refract(si, co, s)
                q = x + (h + d) * t - d * M0
                b = int((q - 2 * lam) / (lam / 40))
                if 0 <= b < bins:
                    land[b] += w * dx
            peak = max(land)
            if peak > best[0]:
                best = (peak, d)
        print(f"    wavelength {lam} m, amplitude {amp} m: brightest at {best[1]:.2f} m, predicted {focus:.2f} m")

    print("R2  light conserved, and contrast (rms about the mean) by depth: reference, map, map with no blur")
    for wind, fld in fields.items():
        print(f"    wind {wind} m/s: rms slope {fld.rms:.3f}, of which unresolved {fld.sig:.3f}")
        for j in range(0, 6 * STEP + 1, STEP):
            d = fld.depth(j)
            ref = reference(fld.xs, fld.hs, fld.ss, d, ZEN, LO, BINS)
            mp = fld.map_at(d)
            raw = carry(fld.gx, fld.gh, fld.gs, d, ZEN, LO, BINS)
            me, mt, se, st, c = stats(mp, ref, A, B)
            _, _, sr, _, cr = stats(raw, ref, A, B)
            print(f"      {d:5.2f} m  mean {mt:.3f} / {me:.3f}   contrast {st:.3f} / {se:.3f} / {sr:.3f}"
                  f"   map to reference r = {c:.2f} (no blur {cr:.2f})")

    print("R3  the ladder read midway between two rungs, against the map made at that depth")
    for wind, fld in fields.items():
        for name, per in (("a factor of 2", 1), ("root 2", 2)):
            worst_c, worst_k = 1.0, 1.0
            for j in range(0, 6 * STEP):
                d = fld.depth(j + (STEP // per) / 2)
                if abs(math.log2(d / FIRST) * per % 1 - 0.5) > 0.01:
                    continue
                direct = fld.map_at(d)
                read = fld.ladder(d, per)
                _, _, sr, sd, c = stats(read, direct, A, B)
                worst_c = min(worst_c, c)
                worst_k = min(worst_k, sr / sd)
            print(f"    wind {wind} m/s, rungs {name} apart: worst r = {worst_c:.2f}, contrast kept {worst_k:.2f}")

    print("R4  shallower than the first rung: the first rung scaled by depth, against the map made there")
    fld = fields[4.0]
    for d in (0.03, 0.06, 0.125):
        direct = fld.map_at(d)
        read = fld.ladder(d)
        _, _, sr, sd, c = stats(read, direct, A, B)
        print(f"    {d:5.3f} m  contrast {sd:.3f} made, {sr:.3f} scaled, r = {c:.2f}")

    print("R5  substitutes, against the map made at each depth (wind 4 m/s)")
    one = fld.map_at(1.0)
    for d in (0.25, 4.0, 16.0):
        direct = fld.map_at(d)
        _, _, so, sd, c = stats(one, direct, A, B)
        print(f"    one plane made at 1 m, read at {d:5.2f} m: contrast {so:.3f} for {sd:.3f}, r = {c:.2f}")
    for d in (0.25, 1.0, 4.0):
        # backward: from the receiver, step back up the mean ray, correct twice by the slope found,
        # and take one sheet's brightness from the curvature there
        direct = carry(fld.gx, fld.gh, fld.gs, d, ZEN, LO, BINS)
        back = []
        g0, dx = fld.gx[0], fld.gx[1] - fld.gx[0]

        def at(arr, x):
            f = (x - g0) / dx
            i = max(0, min(len(arr) - 2, int(math.floor(f))))
            t = f - i
            return (1 - t) * arr[i] + t * arr[i + 1]

        for b in range(BINS):
            q = LO + (b + 0.5) * TEX
            x = q
            for _ in range(2):
                x = q - d * SHIFT * at(fld.gs, x)
            jac = abs(1.0 + d * SHIFT * at(fld.gc, x))
            back.append(min(8.0, 1.0 / max(jac, 1e-3)))
        mb, md, sb, sd, c = stats(back, direct, A, B)
        print(f"    backward, one sheet, at {d:4.2f} m: mean {mb:.2f} for {md:.2f}, contrast {sb:.2f} for {sd:.2f}, r = {c:.2f}")
    for gain in (0.3, 0.05):
        calm = Field(1.0, gain)
        print(f"    the 1 m/s surface at {gain} of its height (rms slope {calm.rms:.4f}): the map's contrast at"
              f" 1 m is {stats(calm.map_at(1.0), calm.map_at(1.0), A, B)[2]:.3f}; a played texture keeps its own")


# ---------------------------------------------------------------- shafts

def part_s(fld):
    print("PART S  shafts (wind 4 m/s, lens 3 m down)")
    c_att, k_att = 0.25, 0.08                 # beam and diffuse attenuation, per metre: a clear sea
    mu = 1.0 / math.sqrt(1.0 + M0 * M0)       # cosine of the refracted Sun from the vertical
    kappa = (c_att - k_att) / mu              # the unscattered share falls as exp(-kappa z)
    z0, x0 = 3.0, 20.0
    exact = {}

    def field_exact(q, depth):
        key = round(depth / 0.05)
        if key not in exact:
            exact[key] = fld.map_at(max(0.01, key * 0.05))
        return read(exact[key], q)

    def read(arr, q):
        f = (q - LO) / TEX - 0.5
        i = int(math.floor(f))
        if i < 0 or i + 1 >= len(arr):
            return 1.0
        t = f - i
        return (1 - t) * arr[i] + t * arr[i + 1]

    cache = {}

    def ladder_span(q, depth, span):
        """The ladder at this depth, averaged over the stretch of the map one step spans."""
        key = round(depth, 2)
        if key not in cache:
            arr = fld.ladder(max(0.01, key))
            pre = [0.0]
            for v in arr:
                pre.append(pre[-1] + v)
            cache[key] = (arr, pre)
        arr, pre = cache[key]
        if span < TEX:
            return read(arr, q)
        a = int(round((q - span / 2 - LO) / TEX))
        b = int(round((q + span / 2 - LO) / TEX))
        if a < 0 or b > len(arr) or b <= a:
            return 1.0
        return (pre[b] - pre[a]) / (b - a)

    def ray(angle, field, steps=None, rnd=None, spans=True):
        """The Sun's share of the light scattered into a view ray, as a ratio to even light.
        angle is from the horizontal, toward +x; the ray ends at the surface or after 18 m."""
        dxr, dzr = math.cos(angle), math.sin(angle)
        length = min(18.0, z0 / dzr if dzr > 1e-6 else 18.0)
        num = den = 0.0
        if steps is None:
            n = int(length / 0.02)
            for i in range(n):
                s = (i + 0.5) * length / n
                depth = z0 - s * dzr
                w = math.exp(-(k_att + kappa) * depth - c_att * s)
                num += w * field(x0 + s * dxr - depth * M0, depth)
                den += w
            return num / den
        # steps placed evenly in the path's own weight exp(-c s), one jittered sample in each;
        # with spans, each sample is the field averaged over the map its step crosses
        total = 1.0 - math.exp(-c_att * length)
        for i in range(steps):
            u = (i + rnd.random()) / steps
            s = -math.log(1.0 - u * total) / c_att
            s_lo = -math.log(1.0 - i / steps * total) / c_att
            s_hi = -math.log(max(1e-12, 1.0 - (i + 1) / steps * total)) / c_att
            depth = z0 - s * dzr
            w = math.exp(-(k_att + kappa) * depth)
            q = x0 + s * dxr - depth * M0
            if spans:
                num += w * field(q, depth, (s_hi - s_lo) * abs(dxr + dzr * M0))
            else:
                num += w * field(q, depth, 0.0)
            den += w
        return num / den

    theta_r = math.atan(M0)
    looks = (("up along the refracted Sun", math.pi / 2 + theta_r),
             ("up, 5 degrees off it", math.pi / 2 + theta_r - math.radians(5)),
             ("up, 10 degrees off it", math.pi / 2 + theta_r - math.radians(10)),
             ("up, 20 degrees off it", math.pi / 2 + theta_r - math.radians(20)),
             ("level", math.radians(1.0)))
    print("S1  ray-to-ray variation of the Sun's scattered light over a 6 degree fan (rms about its mean)")
    fans = {}
    for name, centre in looks:
        vals = [ray(centre + math.radians(-3 + 6 * i / 119), field_exact) for i in range(120)]
        mean = sum(vals) / len(vals)
        rms = math.sqrt(sum((v - mean) ** 2 for v in vals) / len(vals))
        fans[name] = (centre, vals)
        print(f"    {name:28s} mean {mean:.3f}, rms {rms:.3f}, range {min(vals):.2f} to {max(vals):.2f}")
    print("S2  a march through the ladder against the fine integral through the exact field")
    rnd = random.Random(3)
    for name in ("up along the refracted Sun", "up, 10 degrees off it", "level"):
        centre, vals = fans[name]
        mean = sum(vals) / len(vals)
        spread = math.sqrt(sum((v - mean) ** 2 for v in vals) / len(vals))
        for steps, spans in ((16, False), (16, True), (32, True)):
            err = 0.0
            for i, v in enumerate(vals):
                est = ray(centre + math.radians(-3 + 6 * i / 119), ladder_span, steps, rnd, spans)
                err += (est - v) ** 2
            how = "each sample averaged over its step" if spans else "point samples"
            print(f"    {name:28s} {steps} steps, {how}: rms error {math.sqrt(err / len(vals)):.3f}"
                  f" against a shaft contrast of {spread:.3f}")


# ---------------------------------------------------------------- the meter

def part_t():
    print("PART T  the meter under water")
    lum = (0.2126, 0.7152, 0.0722)
    types = (("clear sea", (0.35, 0.07, 0.04)), ("coast", (0.45, 0.16, 0.20)), ("lake", (0.9, 0.5, 0.8)))
    print("  the light at the lens as a share of the light above (surface passes 0.97), in stops,")
    print("  and how far the picture darkens when the camera follows that light by a share")
    for name, k in types:
        for z in (0.1, 1.0, 3.0, 10.0, 20.0):
            f = 0.97 * sum(w * math.exp(-kk * z) for w, kk in zip(lum, k))
            ev = math.log2(f)
            row = ", ".join(f"share {a}: {ev * (1 - a):6.2f}" for a in (0.0, 0.5, 0.75, 1.0))
            print(f"    {name:9s} {z:5.1f} m  light {f:9.2e}  ({ev:6.2f} stops)   picture: {row}")
    print("  across the surface: the lens's share under water w takes the factor as mix(1, f, w), so a")
    print("  lens half under at 0 m reads", round(0.5 * (1 + 0.97), 3), "and no step is larger than",
          round(-math.log2(0.97), 3), "stops")


if __name__ == "__main__":
    fields = {w: Field(w) for w in (1.0, 4.0, 8.0)}
    part_r(fields)
    part_s(fields[4.0])
    part_t()
