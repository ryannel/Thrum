#!/usr/bin/env python3
"""Design proof for the whole-atmosphere revision of 14 November 2026 (fog, mist, haze, low cloud).

Pure Python, no dependencies. First-profile constants; the shapes are not the look.
Run: python3 docs/evidence/CLOUDS/design-proof/whole_atmosphere_proof.py
"""
import math

def smoothstep(a, b, x):
    t = min(1.0, max(0.0, (x - a) / (b - a)))
    return t * t * (3 - 2 * t)

def hash01(i, seed=0):
    x = (i * 0x9E3779B1 + seed * 0x85EBCA6B) & 0xFFFFFFFF
    x ^= x >> 16; x = (x * 0x7FEB352D) & 0xFFFFFFFF
    x ^= x >> 15; x = (x * 0x846CA68B) & 0xFFFFFFFF
    x ^= x >> 16
    return x / 2**32

print("I. The handover between the view volume and the trace")
# A ray through a uniform medium of extinction s and length L, lit by a source J(t) per unit extinction.
# The near integrator takes s*w_n and the far one s*w_f, w_n + w_f = 1, with w_f a ramp over [D0, D1].
# They compose as ordered segments: T = T_n*T_f, S = S_n + T_n*S_f. Exact: one integrator over s.
def handover(s, D0, D1, L, J, n=30000):
    dt = L / n
    tn = tf = te = 0.0; Sn = Sf = Se = 0.0
    for i in range(n):
        t = (i + 0.5) * dt
        wf = smoothstep(D0, D1, t) if D1 > D0 else (1.0 if t >= D0 else 0.0)
        wn = 1 - wf
        Sn += math.exp(-tn - 0.5 * s * wn * dt) * s * wn * dt * J(t)
        Sf += math.exp(-tf - 0.5 * s * wf * dt) * s * wf * dt * J(t)
        Se += math.exp(-te - 0.5 * s * dt) * s * dt * J(t)
        tn += s * wn * dt; tf += s * wf * dt; te += s * dt
    return abs(math.exp(-tn - tf) - math.exp(-te)), abs(Sn + math.exp(-tn) * Sf - Se), Se
sources = (("uniform light", lambda t: 1.0),
           ("a shadow edge at 1500 m, 1 to 0.1", lambda t: 1.0 if t < 1500 else 0.1),
           ("light falling 1 to 0.1 over 1000-2000 m", lambda t: 1.0 - 0.9 * smoothstep(1000, 2000, t)))
for name, J in sources:
    print(f"  {name}:")
    for vis in (400, 2000, 10000, 40000):
        s = 3.912 / vis
        row = []
        for D0, D1 in ((1500, 1500), (1400, 1600), (1000, 2000)):
            dT, dS, Se = handover(s, D0, D1, 30000.0, J)
            row.append(f"ramp {D1-D0:4.0f} m: S off by {dS:.4f}")
        print(f"    visual range {vis:5d} m: T off by {dT:.0e}; " + "; ".join(row) + f" (of {Se:.3f})")

print("\nII. Mist that pools in hollows")
# Ground: fractal relief. Datum: the ground averaged over a wide window (the basin's level).
# Fog top = datum - H0 + amount * H ; fog depth = max(0, top - ground).
N, dx = 4096, 10.0
def ground(i):
    x = i * dx; g = 0.0
    for o, (lam, amp) in enumerate(((6000, 120), (2500, 60), (900, 25), (300, 8), (90, 2))):
        g += amp * math.sin(2 * math.pi * x / lam + 6.283 * hash01(o, 7))
    return g
G = [ground(i) for i in range(N)]
W = 150  # +-1500 m window
pre = [0.0]
for g in G: pre.append(pre[-1] + g)
def datum(i):
    a, b = max(0, i - W), min(N, i + W + 1)
    return (pre[b] - pre[a]) / (b - a)
Dm = [datum(i) for i in range(N)]
H0, H = 60.0, 140.0
def depth(i, a): return max(0.0, Dm[i] - H0 + a * H - G[i])
prev = None
for a in (0.0, 0.2, 0.4, 0.6, 0.8, 1.0):
    d = [depth(i, a) for i in range(N)]
    cov = sum(1 for v in d if v > 0) / N
    ridge = [d[i] for i in range(N) if G[i] - Dm[i] > 40]
    hollow = [d[i] for i in range(N) if G[i] - Dm[i] < -40]
    print(f"  amount {a:.1f}: ground under mist {100*cov:5.1f}%, mean depth in hollows {sum(hollow)/len(hollow):6.1f} m, on ridges {sum(ridge)/len(ridge):5.1f} m")
step = max(abs(depth(i, 0.500 + 0.001) - depth(i, 0.500)) for i in range(N))
step2 = max(abs(depth(i, 0.500 + 0.0005) - depth(i, 0.500)) for i in range(N))
print(f"  largest change of depth for an amount step of 0.001: {step:.3f} m; of 0.0005: {step2:.3f} m (halves)")
slope_top = max(abs(Dm[i+1] - Dm[i]) / dx for i in range(N-1))
slope_g = max(abs(G[i+1] - G[i]) / dx for i in range(N-1))
print(f"  steepest slope of the mist top {100*slope_top:.1f}% against the ground's {100*slope_g:.1f}%")

print("\nIII. Mist through a day: forms on a clear calm night, burns off after sunrise")
# One integrated value, the ground cooling q in 0..1. Target: high when the Sun is down, the sky clear and the air calm.
def day(moisture, coverage, wind, days=3):
    q = 0.0; dt = 60.0; out = {}
    for k in range(int(days * 86400 / dt)):
        t = k * dt; hour = (t / 3600.0) % 24
        elev = 45.0 * math.sin(2 * math.pi * (hour - 6) / 24)      # degrees; sunrise 06:00
        night = 1 - smoothstep(-6.0, 10.0, elev)
        calm = 1 - smoothstep(2.0, 8.0, wind)
        target = night * (1 - 0.85 * coverage) * calm
        tau = 3 * 3600.0 if target > q else 3600.0 / (0.25 + smoothstep(0, 30, elev))
        q += (target - q) * (1 - math.exp(-dt / tau))
        if k >= (days - 1) * 86400 / dt and k % 120 == 0:
            out[int(hour)] = smoothstep(0.0, 1.0, (q + moisture - 1) / 0.5)
    return out
for name, m, c, w in (("moist, clear, calm", 0.7, 0.0, 1.0), ("moist, overcast, calm", 0.7, 1.0, 1.0),
                      ("moist, clear, 9 m/s wind", 0.7, 0.0, 9.0), ("dry, clear, calm", 0.1, 0.0, 1.0)):
    o = day(m, c, w)
    print(f"  {name:26s} amount at " + " ".join(f"{h:02d}h:{o[h]:.2f}" for h in (18, 20, 22, 0, 2, 4, 6, 8, 10, 12, 14)))
def series(moisture, coverage, wind):
    q = 0.0; dt = 60.0; s = []
    for k in range(int(2 * 86400 / dt)):
        hour = (k * dt / 3600.0) % 24
        elev = 45.0 * math.sin(2 * math.pi * (hour - 6) / 24)
        night = 1 - smoothstep(-6.0, 10.0, elev); calm = 1 - smoothstep(2.0, 8.0, wind)
        target = night * (1 - 0.85 * coverage) * calm
        tau = 3 * 3600.0 if target > q else 3600.0 / (0.25 + smoothstep(0, 30, elev))
        q += (target - q) * (1 - math.exp(-dt / tau))
        s.append(smoothstep(0.0, 1.0, (q + moisture - 1) / 0.5))
    return s
s = series(0.7, 0.0, 1.0)
print(f"  largest change of amount in one minute: {max(abs(s[i+1]-s[i]) for i in range(len(s)-1)):.4f}")

print("\nIV. What the view volume resolves")
for nx, ny, nz in ((160, 90, 64), (160, 90, 128), (128, 72, 64)):
    near, far = 0.5, 32000.0
    tile_px = 2560 / nx
    rad = 2 * math.tan(math.radians(25)) / ny       # 50 degree vertical field of view
    rows = []
    for dist in (20, 100, 500, 2000, 8000):
        k = math.log(dist / near) / math.log(far / near) * nz
        thick = dist * ((far / near) ** (1 / nz) - 1)
        rows.append(f"{dist} m: {dist*rad:5.1f} m across, {thick:6.1f} m deep")
    print(f"  {nx}x{ny}x{nz} ({nx*ny*nz/1e6:.2f} M cells, {tile_px:.0f} px tiles at 2560 wide): " + "; ".join(rows))

for nz, hand in ((96, 500.0), (96, 1500.0)):
    k = 0.75 * nz
    r = (hand / 0.5) ** (1 / k)
    print(f"  {nz} slices with three quarters inside a handover at {hand:.0f} m: " + "; ".join(f"{d} m: {d*(r-1):5.1f} m deep" for d in (20, 100, 500, 1500) if d <= hand))

print("\nV. The Sun's path through a fog bank: the ground band")
# A bank 60 m deep from x = 0 to 3000 m with soft ends and top; ground flat. tau toward the Sun from the ground.
s0, Hb = 3.912 / 150.0, 60.0
def sig(x, h):
    if h < 0 or h > Hb * 1.3: return 0.0
    return s0 * smoothstep(-200, 200, x) * (1 - smoothstep(2800, 3200, x)) * (1 - smoothstep(0.6 * Hb, 1.3 * Hb, h))
def tau_true(x, h, elev, ds=1.0):
    c, s_ = math.cos(elev), math.sin(elev); t = 0.0; r = 0.5 * ds
    while h + r * s_ < Hb * 1.3:
        t += sig(x + r * c, h + r * s_) * ds; r += ds   # the Sun is toward +x
    return t
for elev_deg in (30, 10, 4):
    e = math.radians(elev_deg)
    col = 80.0; K = 9                                   # columns 80 m wide, 9 nodes over the band
    hs = [Hb * 1.3 * k / (K - 1) for k in range(K)]
    xs = [-1500 + col * i for i in range(int(6000 / col) + 1)]
    grid = [[tau_true(x, h, e) for h in hs] for x in xs]
    def prod(x, h):
        fx = (x + 1500) / col; i = min(len(xs) - 2, int(fx)); u = fx - i
        fh = h / (Hb * 1.3) * (K - 1); k = min(K - 2, int(fh)); v = fh - k
        return ((grid[i][k] * (1 - u) + grid[i+1][k] * u) * (1 - v) + (grid[i][k+1] * (1 - u) + grid[i+1][k+1] * u) * v)
    e2p = e2f = 0.0; n = 0; mxp = 0.0
    for j in range(400):
        x = -1400 + 5800 * hash01(j, 3); h = 0.0 if j % 2 else Hb * hash01(j, 5)
        tt = math.exp(-tau_true(x, h, e))
        tv = 0.0; hh = h + 0.5
        while hh < Hb * 1.3: tv += sig(x, hh); hh += 1.0
        tp = math.exp(-tv / math.sin(e)); tq = math.exp(-prod(x, h))
        e2p += (tp - tt) ** 2; e2f += (tq - tt) ** 2; mxp = max(mxp, abs(tp - tt)); n += 1
    print(f"  Sun {elev_deg:2d} deg: RMS in transmittance, level-layer rule {math.sqrt(e2p/n):.3f} (largest {mxp:.2f}); product of 80 m columns and {K} heights {math.sqrt(e2f/n):.3f}")

print("\nVI. Haze as visibility")
ray = 1.16e-5; now = 4.4e-6
print(f"  today's fixed aerosol, {now:.1e} per m at the ground: visual range {3.912/(ray+now)/1000:.0f} km")
for v in (150, 60, 20, 8, 3):
    print(f"  visual range {v:3d} km needs aerosol extinction {3.912/(v*1000)-ray:.2e} per m, {(3.912/(v*1000)-ray)/now:6.1f} times today's")
for v, name in ((10000, "thin mist"), (1000, "mist/fog boundary"), (200, "fog"), (50, "thick fog")):
    s = 3.912 / v
    print(f"  {name:18s} visual range {v:5d} m: extinction {s:.2e} per m; liquid water {s*2*1000*8e-6/3*1000:.3f} g/m3 at 8 um droplets")
