"""CLOUDS design proof, 13 November 2026.

A short CPU sketch of three mechanisms in docs/plans/CLOUDS/DESIGN.md:

A-C  the weather field: convective cells on a space-time jittered lattice in the air frame,
     plus a stratiform deck, as a stateless function of seed, coverage, travel and time;
D    how many density samples a view ray needs when it skips empty air with a coarse map;
E    one Sun-path optical-depth volume that serves self-shadow and ground shade.

It is evidence for the design, not engine code. Constants here are a first profile.
Run: uv run --with numpy python docs/evidence/CLOUDS/design-proof/cloud_field_proof.py
"""
import numpy as np

R_EARTH = 6.36e6
LC = 1600.0    # cell lattice pitch, m: a profile constant, never a preset value
PG = 1800.0    # generation period, s: a profile constant
BASE = 1000.0  # shared condensation level, m
HMAX = 1200.0  # greatest cloud depth above the base, m
SIG0 = 0.05    # extinction of the cloud interior, 1/m (0.4 g/m3 liquid water, 12 um droplets)
SEED = 7


def h32(a, b, c, d):
    """Integer hash of three lattice integers and a stream id; the same arithmetic runs in WGSL."""
    m = np.uint64(0xFFFFFFFF)
    a = np.asarray(a, dtype=np.int64).astype(np.uint64) & m
    b = np.asarray(b, dtype=np.int64).astype(np.uint64) & m
    c = np.asarray(c, dtype=np.int64).astype(np.uint64) & m
    x = (a * np.uint64(0x9E3779B1) + b * np.uint64(0x85EBCA77) + c * np.uint64(0xC2B2AE3D)
         + np.uint64(d + 131 * SEED) * np.uint64(0x27D4EB2F)) & m
    x ^= x >> np.uint64(15)
    x = (x * np.uint64(0x2C1B3C6D)) & m
    x ^= x >> np.uint64(12)
    x = (x * np.uint64(0x297A2D39)) & m
    x ^= x >> np.uint64(15)
    return x


def u01(a, b, c, d):
    return h32(a, b, c, d).astype(np.float64) / 4294967296.0


def smooth(e0, e1, x):
    t = np.clip((x - e0) / (e1 - e0), 0.0, 1.0)
    return t * t * (3.0 - 2.0 * t)


def value_noise(x, z, w, stream):
    """Trilinear value noise on an integer lattice, smooth-faded."""
    xi, zi, wi = np.floor(x), np.floor(z), np.floor(w)
    fx, fz, fw = x - xi, z - zi, w - wi
    fx, fz, fw = fx * fx * (3 - 2 * fx), fz * fz * (3 - 2 * fz), fw * fw * (3 - 2 * fw)
    out = 0.0
    for dx in (0, 1):
        for dz in (0, 1):
            for dw in (0, 1):
                wgt = (fx if dx else 1 - fx) * (fz if dz else 1 - fz) * (fw if dw else 1 - fw)
                out = out + wgt * u01(xi + dx, zi + dz, wi + dw, stream)
    return out


def p_cell(c):
    return 0.95 * smooth(0.0, 0.6, c)


def field(xa, za, t, c):
    """Air-frame point and weather time -> (dimensional profile d in 0..1, cloud depth above base in m)."""
    xa, za, t, c = np.broadcast_arrays(np.asarray(xa, float), np.asarray(za, float),
                                       np.asarray(t, float), np.asarray(c, float))
    ci, cj = np.floor(xa / LC), np.floor(za / LC)
    d = np.zeros(xa.shape)
    top = np.zeros(xa.shape)
    pc = p_cell(c)
    for di in (-1, 0, 1):
        for dj in (-1, 0, 1):
            i, j = ci + di, cj + dj
            phase = u01(i, j, 0, 1)
            g0 = np.floor(t / PG - phase)
            for dg in (0, -1):
                g = g0 + dg
                life = PG * (0.4 + 0.6 * u01(i, j, g, 3))            # 12 to 30 minutes
                birth = (g + phase + u01(i, j, g, 2)) * PG              # anywhere in the period
                s = (t - birth) / life
                env = np.where((s > 0) & (s < 1), np.sin(np.pi * np.clip(s, 0, 1)) ** 1.2, 0.0)
                amp = smooth(0.0, 1.0, (pc - u01(i, j, g, 4)) / 0.15)
                e = env * amp
                cx = (i + u01(i, j, g, 5)) * LC
                cz = (j + u01(i, j, g, 6)) * LC
                size = 0.30 + 0.50 * u01(i, j, g, 7) ** 2
                rad = LC * size * (0.5 + 0.5 * e)
                r2 = ((xa - cx) ** 2 + (za - cz) ** 2) / (rad * rad)
                dc = e * np.clip(1.0 - r2, 0.0, 1.0)
                hc = (250.0 + 900.0 * u01(i, j, g, 8) ** 2) * dc ** 0.7
                top = np.maximum(top, hc)
                d = np.maximum(d, dc)
    wd = smooth(0.45, 1.0, c)
    n = 0.65 * value_noise(xa / 6000.0, za / 6000.0, t / 3600.0, 20) \
        + 0.35 * value_noise(xa / 1500.0, za / 1500.0, t / 1800.0, 21)
    q = smooth(0.0, 1.0, (n - (1.0 - 1.3 * wd)) / 0.2)
    hd = 350.0 * q ** 0.7 * (0.6 + 0.4 * n)
    top = np.maximum(top, hd)
    d = np.maximum(d, q)
    return d, top


def profile(hf):
    return smooth(0.0, 0.08, hf) * (1.0 - smooth(0.6, 1.0, hf))


def sigma(x, y, z, t, c):
    d, top = field(x, z, t, c)
    hf = np.where(top > 0, (y - BASE) / np.maximum(top, 1e-6), -1.0)
    inside = (hf > 0) & (hf < 1) & (d > 0)
    return np.where(inside, SIG0 * profile(np.clip(hf, 0, 1)) * np.clip(3.0 * d, 0, 1), 0.0)


def column_tau(d, top):
    return SIG0 * top * 0.76 * np.clip(3.0 * d, 0, 1)


rng = np.random.default_rng(1)
T0 = 5.0 * 3600.0

print("A. Coverage control, 40 km square at 200 m, weather time 5 h")
print("   c     cloudy columns   mean tau of cloudy   mean depth m")
gx, gz = np.meshgrid(np.arange(-20e3, 20e3, 200.0), np.arange(-20e3, 20e3, 200.0))
for c in (0.0, 0.15, 0.3, 0.5, 0.7, 0.85, 1.0):
    d, top = field(gx, gz, T0, c)
    tau = column_tau(d, top)
    cloudy = tau > 1.0
    print(f"   {c:4.2f}  {cloudy.mean():10.3f}      {tau[cloudy].mean() if cloudy.any() else 0:10.1f}"
          f"      {top[cloudy].mean() if cloudy.any() else 0:10.0f}")

print("\nB. Continuity. 400 air-frame points, 2 h at 1 s steps; largest change of column tau in one second")
px, pz = rng.uniform(-20e3, 20e3, 400), rng.uniform(-20e3, 20e3, 400)
ts = T0 + np.arange(0, 7200.0, 1.0)
for c in (0.3, 0.7):
    d, top = field(px[:, None], pz[:, None], ts[None, :], c)
    tau = column_tau(d, top)
    print(f"   c={c}: max |d tau| per second {np.abs(np.diff(tau, axis=1)).max():.3f}"
          f" of a largest tau {tau.max():.1f}; columns that were ever cloudy {(tau.max(axis=1) > 1).mean():.2f}")
ramp = 0.3 + 0.4 * np.clip((ts - T0) / 60.0, 0, 1)
d, top = field(px[:, None], pz[:, None], ts[None, :120], ramp[None, :120])
tau = column_tau(d, top)
print(f"   coverage ramped 0.3 to 0.7 in 60 s: max |d tau| per second {np.abs(np.diff(tau, axis=1)).max():.3f}")
trav = 40.0 * (ts[:600] - T0)
d0, top0 = field(px[:, None] - trav[None, :], pz[:, None], ts[None, :600], 0.3)
d1, top1 = field(px[:, None], pz[:, None], ts[None, :600], 0.3)
moved = column_tau(*field(px + 0.0 - trav[-1], pz, ts[599], 0.3))
print(f"   40 m/s wind for 10 min: the field at x equals the still field at x - travel exactly:"
      f" {np.array_equal(column_tau(d0, top0)[:, -1], moved)}")

print("\nC. Births in a 48 km square of sites over 6 h at c=0.3, in one-minute bins")
si, sj = np.meshgrid(np.arange(30), np.arange(30))
births, hops = [], []
prev = None
G0 = int(T0 / PG)
for g in range(G0 - 2, G0 + 22):
    present = (p_cell(0.3) - u01(si, sj, g, 4)) / 0.15 > 0.5
    birth = (g + u01(si, sj, 0, 1) + u01(si, sj, g, 2)) * PG
    births.append(birth[present])
    pos = np.stack([u01(si, sj, g, 5), u01(si, sj, g, 6)])
    if prev is not None:
        hops.append(np.hypot(*(pos - prev)) * LC)
    prev = pos
births = np.concatenate(births)
lo = G0 * PG
counts, _ = np.histogram(births, bins=np.arange(lo, lo + 6 * 3600.0 + 1, 60.0))
spec = np.abs(np.fft.rfft(counts - counts.mean())) ** 2
k_gen = round(len(counts) * 60.0 / PG)
print(f"   births per minute: mean {counts.mean():.2f}, variance/mean {counts.var() / counts.mean():.2f}"
      f" (1 is a Poisson process), max {counts.max()}")
print(f"   spectrum of the counts over its mean: at the generation period {spec[k_gen] / spec[1:].mean():.2f},"
      f" largest anywhere {spec[1:].max() / spec[1:].mean():.2f} (white noise of this length gives about {np.log(len(spec) - 1):.1f})")
print(f"   mean move of a site's cloud from one generation to the next: {np.mean(hops):.0f} m"
      f" (lattice pitch {LC:.0f} m)")

print("\nD. Density samples per sky ray, eye 2 m, pitch 15 deg, 90x58 deg view, 96x54 rays")
CELL = 1000.0
cx = np.arange(-2e3, 262e3, CELL)
cz = np.arange(-190e3, 190e3, CELL)
sub = (np.arange(6) + 0.5) / 6 * CELL
az = np.radians(np.linspace(-45, 45, 96))
el = np.radians(np.linspace(-14, 44, 54))
AZ, EL = np.meshgrid(az, el)
AZ, EL = AZ.ravel(), EL.ravel()
sky = EL > np.radians(0.05)
dirx, dirz, sinE, cosE = np.cos(AZ) * np.cos(EL), np.sin(AZ) * np.cos(EL), np.sin(EL), np.cos(EL)
r0 = R_EARTH + 2.0


def reach(h):
    return -r0 * sinE + np.sqrt(np.maximum((r0 * sinE) ** 2 + (R_EARTH + h) ** 2 - r0 * r0, 0.0))


for c, name in ((0.3, "fair"), (0.7, "broken"), (1.0, "overcast")):
    occ = np.zeros((len(cx), len(cz)))
    for ox in sub:
        for oz in sub:
            X, Z = np.meshgrid(cx + ox, cz + oz, indexing="ij")
            d, top = field(X, Z, T0, c)
            occ = np.maximum(occ, np.where(d > 0, top, 0.0))
    s = reach(BASE).copy()
    s_out = np.minimum(reach(BASE + HMAX), 250e3)
    T = np.ones(len(s))
    n_skip, n_fine, n_full = np.zeros(len(s)), np.zeros(len(s)), np.zeros(len(s))
    live = sky & (s < s_out)
    while live.any():
        idx = np.nonzero(live)[0]
        ss = s[idx]
        x, z = dirx[idx] * ss, dirz[idx] * ss
        alt = np.sqrt(r0 * r0 + ss * ss + 2 * r0 * ss * sinE[idx]) - R_EARTH
        ix = np.clip(((x - cx[0]) / CELL).astype(int), 0, len(cx) - 1)
        iz = np.clip(((z - cz[0]) / CELL).astype(int), 0, len(cz) - 1)
        cap = occ[ix, iz]
        empty = (cap <= 0) | (alt > BASE + cap)
        step = np.clip(0.004 * ss, 10.0, 400.0)
        # empty cell: go to where the ray leaves it
        tx = np.where(dirx[idx] > 0, (cx[ix] + CELL - x), (cx[ix] - x)) / np.where(dirx[idx] == 0, 1e-9, dirx[idx])
        tz = np.where(dirz[idx] > 0, (cz[iz] + CELL - z), (cz[iz] - z)) / np.where(dirz[idx] == 0, 1e-9, dirz[idx])
        leave = np.minimum(np.where(tx > 0, tx, 1e9), np.where(tz > 0, tz, 1e9)) + 1.0
        sg = np.zeros(len(idx))
        f = ~empty
        if f.any():
            sg[f] = sigma(x[f], alt[f], z[f], T0, c)
        n_skip[idx[empty]] += 1
        n_fine[idx[f]] += 1
        n_full[idx[f & (sg > 0)]] += 1
        T[idx] *= np.exp(-sg * step)
        s[idx] = ss + np.where(empty, np.maximum(leave, step), step)
        live[idx] = (s[idx] < s_out[idx]) & (T[idx] > 0.02)
    k = sky
    reads = n_skip[k] + n_fine[k] + 3 * n_full[k]
    print(f"   {name:9s} coarse {n_skip[k].mean():5.1f}  fine {n_fine[k].mean():5.1f}  in cloud {n_full[k].mean():5.1f}"
          f"  (p95 {np.percentile(n_fine[k], 95):4.0f} fine, {np.percentile(n_full[k], 95):4.0f} in cloud);"
          f" texture reads per ray {reads.mean():5.1f}; rays stopped opaque {(T[k] < 0.02).mean():.2f}")
    for w, h in ((640, 360), (320, 180)):
        rays = w * h * sky.mean()
        print(f"      {w}x{h} trace: {rays * reads.mean() / 1e6:6.1f} M reads a frame,"
              f" {rays * (n_fine[k] + n_skip[k]).mean() / 1e6:5.1f} M map lookups, {rays * n_full[k].mean() / 1e6:5.1f} M lit samples")

print("\nE. One Sun-path volume against a 5 m brute-force march, as error in transmittance exp(-tau)")
ytop = BASE + HMAX


def sun_volume(c, cot, sin_e, pitch, nc, ns):
    u = (np.arange(nc) - nc / 2 + 0.5) * pitch
    U, V = np.meshgrid(u, u, indexing="ij")
    dy = HMAX / ns
    vol = np.zeros((nc, nc, ns + 1))
    acc = np.zeros((nc, nc))
    for ks in range(ns):
        y = ytop - (ks + 0.5) * dy
        acc = acc + sigma(U - cot * (ytop - y), y, V, T0, c) * dy / sin_e
        vol[:, :, ks + 1] = acc
    return vol


def lookup(vol, P, cot, pitch, nc, ns):
    dy = HMAX / ns
    uu = (P[0] + cot * (ytop - P[1])) / pitch + nc / 2 - 0.5
    vv = P[2] / pitch + nc / 2 - 0.5
    kk = np.clip((ytop - np.clip(P[1], BASE, ytop)) / dy, 0, ns - 0.001)
    i, j, k = uu.astype(int), vv.astype(int), kk.astype(int)
    a, b, w = uu - i, vv - j, kk - k
    out = 0.0
    for di, wa in ((0, 1 - a), (1, a)):
        for dj, wb in ((0, 1 - b), (1, b)):
            for dk, wk in ((0, 1 - w), (1, w)):
                out = out + wa * wb * wk * vol[i + di, j + dj, k + dk]
    return out


def brute(P, c, cot, sin_e):
    tau = np.zeros(P.shape[1])
    for y in np.arange(ytop - 2.5, BASE, -5.0):
        m = y > P[1]
        tau[m] += sigma(P[0][m] + cot * (y - P[1][m]), y, P[2][m], T0, c) * 5.0 / sin_e
    return tau


print("   c    Sun   columns  slices | in cloud: RMS  p95   mean signed | ground: RMS  p95 | fill, M evaluations")
for c, elev, configs in ((0.3, 60.0, ((80.0, 200, 32), (80.0, 200, 64), (40.0, 200, 64))),
                         (0.3, 20.0, ((80.0, 200, 32), (80.0, 200, 64), (40.0, 200, 64))),
                         (0.3, 6.0, ((80.0, 200, 32), (80.0, 200, 64))),
                         (0.7, 20.0, ((80.0, 200, 64),))):
    e = np.radians(elev)
    cot, sin_e = 1.0 / np.tan(e), np.sin(e)
    # sample in the volume's own coordinates so every Sun ray crosses the window
    uv = rng.uniform(-3e3, 3e3, (2, 80000))
    yy = rng.uniform(BASE, ytop, 80000)
    P = np.stack([uv[0] - cot * (ytop - yy), yy, uv[1]])
    P = P[:, sigma(P[0], P[1], P[2], T0, c) > 0][:, :3000]
    gu = rng.uniform(-3e3, 3e3, (2, 3000))
    G = np.stack([gu[0] - cot * ytop, np.zeros(3000), gu[1]])
    ref_c, ref_g = np.exp(-brute(P, c, cot, sin_e)), np.exp(-brute(G, c, cot, sin_e))
    for pitch, nc, ns in configs:
        vol = sun_volume(c, cot, sin_e, pitch, nc, ns)
        ec = np.exp(-lookup(vol, P, cot, pitch, nc, ns)) - ref_c
        eg = np.exp(-lookup(vol, G, cot, pitch, nc, ns)) - ref_g
        # a short local march of three samples toward the Sun, then the volume at its far end
        loc = ""
        for reach_m in (120.0, 240.0):
            tau, dyl = np.zeros(P.shape[1]), np.minimum(reach_m * sin_e, ytop - P[1])
            for q in range(3):
                yq = P[1] + (q + 0.5) / 3 * dyl
                tau += sigma(P[0] + cot * (yq - P[1]), yq, P[2], T0, c) * dyl / 3 / sin_e
            Q = np.stack([P[0] + cot * dyl, P[1] + dyl, P[2]])
            el = np.exp(-(tau + lookup(vol, Q, cot, pitch, nc, ns))) - ref_c
            loc += f"  +3 local over {reach_m:.0f} m: RMS {np.sqrt((el ** 2).mean()):.3f} p95 {np.percentile(np.abs(el), 95):.3f}"
        print(f"   {c}  {elev:4.1f}  {nc}x{nc} at {pitch:.0f} m  {ns:3d}  |  {np.sqrt((ec ** 2).mean()):.3f}  "
              f"{np.percentile(np.abs(ec), 95):.3f}  {ec.mean():+.3f}  |  {np.sqrt((eg ** 2).mean()):.3f}  "
              f"{np.percentile(np.abs(eg), 95):.3f}  |  {nc * nc * ns / 1e6:.2f} |{loc}")
