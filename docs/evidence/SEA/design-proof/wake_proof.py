"""SEA design proof: does a split-band, body-following FFT eWave produce the wake cues?

Run: uv run --with numpy python docs/evidence/SEA/design-proof/wake_proof.py
Deep water, linear. Equations 19-20 of Tessendorf, "eWave" (2014). The hull is a moving
pressure patch p = rho g d(x), with d a Gaussian draft; it enters as d(phi)/dt = -g h - p/rho.

A  Wedge angle and transverse wavelength at two speeds, single level, against Kelvin theory.
B  The wake outlives its source: a crest keeps travelling after the hull stops.
C  Two levels with complementary bands sum to the single-level field (no wave counted twice).
D  A non-dispersive wave equation at the same two speeds (the wrong substitute).
"""
import math
import sys

import numpy as np

G = 9.80665
DT = 1.0 / 60.0


class Out:
    def append(self, line):
        print(line, flush=True)


class Level:
    """One periodic, body-following window. `band` is the spectral share of the source it owns."""

    def __init__(self, n, dx, band=None, taper_cells=None, anchor=(0.66, 0.5)):
        self.n, self.dx = n, dx
        k1 = 2.0 * math.pi * np.fft.fftfreq(n, d=dx)
        self.kx, self.kz = np.meshgrid(k1, k1, indexing="ij")
        self.k = np.hypot(self.kx, self.kz)
        self.w = np.sqrt(G * self.k)
        with np.errstate(divide="ignore", invalid="ignore"):
            self.k_over_w = np.where(self.k > 0, self.k / self.w, 0.0)
            self.g_over_w = np.where(self.k > 0, G / self.w, 0.0)
        self.c, self.s = np.cos(self.w * DT), np.sin(self.w * DT)
        self.band = np.ones_like(self.k) if band is None else band(self.k)
        t = taper_cells if taper_cells is not None else n // 8
        edge = np.minimum(np.arange(n), n - 1 - np.arange(n)) / float(t)
        ramp = np.clip(edge, 0.0, 1.0)
        m1 = 1.0 - (1.0 - ramp) ** 2 * 0.25  # per-tick damping in the border
        self.mask = np.outer(m1, m1)
        self.h = np.zeros((n, n))
        self.p = np.zeros((n, n))
        # the body sits at a fixed fraction of the window; origin = world position of cell (0,0)
        self.anchor = (anchor[0] * n * dx, anchor[1] * n * dx)
        self.origin = np.array([0.0, 0.0])

    def follow(self, body):
        want = np.array(body) - np.array(self.anchor)
        shift = np.round((want - self.origin) / self.dx).astype(int)
        if shift.any():
            self.h = np.roll(self.h, (-shift[0], -shift[1]), axis=(0, 1))
            self.p = np.roll(self.p, (-shift[0], -shift[1]), axis=(0, 1))
            self.origin = self.origin + shift * self.dx

    def source_hat(self, body, heading, draft, bx, bz):
        """Fourier transform of the Gaussian draft at an exact sub-cell position (m^3 -> grid sum)."""
        ch, sh = math.cos(heading), math.sin(heading)
        ka = self.kx * ch + self.kz * sh
        kb = -self.kx * sh + self.kz * ch
        amp = draft * 2.0 * math.pi * bx * bz * np.exp(-0.5 * ((ka * bx) ** 2 + (kb * bz) ** 2))
        rel = np.array(body) - self.origin
        return amp * np.exp(-1j * (self.kx * rel[0] + self.kz * rel[1])) / (self.dx * self.dx)

    def step(self, src_hat, disperse=True, c0=None):
        hh, ph = np.fft.fft2(self.h), np.fft.fft2(self.p)
        kick = -G * src_hat * self.band * (0.5 * DT) if src_hat is not None else 0.0
        ph = ph + kick
        if disperse:
            c, s, kow, gow = self.c, self.s, self.k_over_w, self.g_over_w
        else:  # constant-speed wave equation, omega = c0 k: the wrong substitute
            w = c0 * self.k
            c, s = np.cos(w * DT), np.sin(w * DT)
            kow = c0 * self.k / G  # dh/dt = (c0 k)^2 / g * phi keeps d(phi)/dt = -g h and gives omega = c0 k
            with np.errstate(divide="ignore", invalid="ignore"):
                gow = np.where(self.k > 0, G / w, 0.0)
        hh, ph = c * hh + kow * s * ph, c * ph - gow * s * hh
        ph = ph + kick
        self.h = np.fft.ifft2(hh).real * self.mask
        self.p = np.fft.ifft2(ph).real * self.mask

    def sample(self, xs, zs):
        """Bilinear sample at world points."""
        u = (np.asarray(xs) - self.origin[0]) / self.dx
        v = (np.asarray(zs) - self.origin[1]) / self.dx
        i0, j0 = np.floor(u).astype(int), np.floor(v).astype(int)
        fu, fv = u - i0, v - j0
        n = self.n
        a = self.h[i0 % n, j0 % n]
        b = self.h[(i0 + 1) % n, j0 % n]
        c = self.h[i0 % n, (j0 + 1) % n]
        d = self.h[(i0 + 1) % n, (j0 + 1) % n]
        return (a * (1 - fu) + b * fu) * (1 - fv) + (c * (1 - fu) + d * fu) * fv


HULL = dict(draft=0.30, bx=1.5, bz=0.5)


def run(levels, speed, seconds, stop_at=None, disperse=True, c0=None):
    body = np.array([0.0, 0.0])
    t = 0.0
    for _ in range(int(round(seconds / DT))):
        u = speed if (stop_at is None or t < stop_at) else 0.0
        body = body + np.array([u * DT, 0.0])
        on = stop_at is None or t < stop_at
        for lv in levels:
            lv.follow(body)
            src = lv.source_hat(body, 0.0, **HULL) if on else None
            lv.step(src, disperse=disperse, c0=c0)
        t += DT
    return body


def wedge_and_wavelength(lv, body, speed):
    """Half-angle of the ridge of largest |h|, and the transverse wavelength on the track."""
    lam_t = 2.0 * math.pi * speed * speed / G
    # the ridge: at stations behind the hull, the lateral offset of max |h|
    stations = np.linspace(1.5 * lam_t, 3.5 * lam_t, 14)
    zs = np.arange(lv.dx, 0.36 * lv.n * lv.dx, lv.dx * 0.5)
    pts = []
    for d in stations:
        # envelope over one transverse wavelength, so a crest/trough phase does not pick the offset
        env = np.zeros_like(zs)
        for dd in np.linspace(-0.5 * lam_t, 0.5 * lam_t, 24):
            env = np.maximum(env, np.abs(lv.sample(np.full_like(zs, body[0] - d - dd), body[1] + zs)))
        pts.append((d, zs[int(np.argmax(env))]))
    d, z = np.array(pts).T
    slope = float(np.polyfit(d, z, 1)[0])
    angle = math.degrees(math.atan(slope))
    # transverse wavelength: zero crossings on the centreline
    xs = np.arange(1.0 * lam_t, 3.5 * lam_t, lv.dx * 0.25)
    line = lv.sample(body[0] - xs, np.full_like(xs, body[1]))
    sign = np.signbit(line)
    cross = np.where(sign[1:] != sign[:-1])[0]
    lam = 2.0 * float(np.mean(np.diff(xs[cross]))) if len(cross) > 2 else float("nan")
    return angle, lam, lam_t


def main():
    out = Out()
    kelvin = math.degrees(math.asin(1.0 / 3.0))

    out.append("A  single level, dispersive (eWave)")
    for speed, n, dx in ((3.0, 512, 0.25), (6.0, 512, 0.5)):
        lv = Level(n, dx)
        body = run([lv], speed, 30.0)
        ang, lam, lam_t = wedge_and_wavelength(lv, body, speed)
        fr = speed / math.sqrt(G * 2.0 * HULL["bx"] * 2.0)
        out.append(
            f"   U={speed:4.1f} m/s  Fr(6 m)={fr:4.2f}  ridge half-angle {ang:5.2f} deg (Kelvin cusp {kelvin:5.2f})"
            f"  transverse wavelength {lam:6.2f} m (2 pi U^2/g = {lam_t:6.2f}, {100 * (lam / lam_t - 1):+5.1f}%)"
            f"  max |h| {np.abs(lv.h).max():.3f} m"
        )

    out.append("D  single level, constant-speed wave equation (wrong substitute), c0 = 2 m/s")
    for speed, n, dx in ((3.0, 512, 0.25), (6.0, 512, 0.5)):
        lv = Level(n, dx)
        body = run([lv], speed, 30.0, disperse=False, c0=2.0)
        ang, lam, lam_t = wedge_and_wavelength(lv, body, speed)
        mach = math.degrees(math.asin(2.0 / speed))
        out.append(f"   U={speed:4.1f} m/s  ridge half-angle {ang:5.2f} deg (Mach {mach:5.2f}); centreline wavelength {lam:6.2f} m against {lam_t:6.2f}")

    out.append("B  source off: hull at 3 m/s for 20 s, then stopped and its pressure removed")
    lv = Level(512, 0.25)
    body = run([lv], 3.0, 20.0)
    lam_t = 2.0 * math.pi * 9.0 / G
    world = np.arange(body[0] - 6.0 * lam_t, body[0] + 3.0 * lam_t, 0.05)

    def line():
        return lv.sample(world, np.full_like(world, body[1]))

    before = line()
    behind = (world > body[0] - 3.0 * lam_t) & (world < body[0] - 1.0 * lam_t)
    w0 = world[behind][int(np.argmax(before[behind]))]
    a0 = float(np.abs(lv.h).max())
    for _ in range(int(round(2.0 / DT))):
        lv.step(None)
    after = line()
    near_crest = np.abs(world - (w0 + 3.0 * 2.0)) < 0.4 * lam_t
    w1 = world[near_crest][int(np.argmax(after[near_crest]))]
    out.append(
        f"   2 s after source-off the tracked crest moved {w1 - w0:5.2f} m (phase speed x 2 s = 6.00 m)"
        f" with height {float(after[near_crest].max()) * 1000:.1f} mm; max |h| {a0:.3f} -> {float(np.abs(lv.h).max()):.3f} m"
    )
    for _ in range(int(round(8.0 / DT))):
        lv.step(None)
    out.append(f"   10 s after source-off max |h| is {float(np.abs(lv.h).max()):.3f} m (waves leave through the absorbing border; none wrap)")

    out.append("C  two levels, complementary bands, against one level (hull at 6 m/s, 20 s, then a 60-degree turn over 6 s)")
    ks, kw = 2.0 * math.pi / 12.0, 2.0 * math.pi / 60.0

    def low(k):
        return 0.5 * (1.0 - np.tanh((k - ks) / kw))

    near = Level(256, 0.25, band=lambda k: 1.0 - low(k))
    far = Level(256, 2.0, band=low)
    near_f, far_f = Level(256, 0.25), Level(256, 2.0)
    ref = Level(1024, 0.25)
    levels = [near, far, near_f, far_f, ref]
    body = np.array([0.0, 0.0])
    heading = 0.0
    for i in range(int(round(26.0 / DT))):
        t = i * DT
        if t > 20.0:
            heading = min(math.radians(60.0), heading + math.radians(10.0) * DT)
        body = body + 6.0 * DT * np.array([math.cos(heading), math.sin(heading)])
        for lv in levels:
            lv.follow(body)
            lv.step(lv.source_hat(body, heading, **HULL))
    # compare inside the near window, clear of its absorbing border
    gx, gz = np.meshgrid(body[0] + np.arange(-32.0, 12.0, 0.25), body[1] + np.arange(-22.0, 22.0, 0.25), indexing="ij")
    one = ref.sample(gx, gz)
    rms = float(np.sqrt(np.mean(one ** 2)))

    def report(name, field):
        e = field - one
        out.append(f"   {name}: RMS error {float(np.sqrt(np.mean(e ** 2))) * 1000:7.2f} mm ({100 * float(np.sqrt(np.mean(e ** 2))) / rms:5.1f}% of reference RMS {rms * 1000:.1f} mm), max {float(np.abs(e).max()) * 1000:7.2f} mm")

    report("two levels, complementary bands", near.sample(gx, gz) + far.sample(gx, gz))
    report("far level alone (2 m cells)    ", far_f.sample(gx, gz))
    report("two full-band levels summed    ", near_f.sample(gx, gz) + far_f.sample(gx, gz))


if __name__ == "__main__":
    sys.exit(main())
