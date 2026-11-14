"""RIVERS design, 14 November 2026: reproductions 1 to 7.
Pure Python, no dependencies. About half a minute to run. Its output is river-proof.txt.

Part 1  The scheme: a finite-volume march of the shallow-water equations with an HLL flux, a well-balanced
        reconstruction and implicit Manning friction. Water at rest round dry ground; a steady flow over a
        bump with a hydraulic jump against its closed form; a steady flow with friction against a bed made
        for a known depth, slow and fast, at three cell sizes; a thin sheet on a steep slope against
        Manning's normal depth, with Audusse's reconstruction and with Chen and Noelle's. The scheme is
        first order in space, so its errors here are a ceiling for the solver the design asks for.
Part 2  A stream in section: riffle, pool behind a sill, chute, ledge, run. Discharge, the pool's level, the
        energy account, and what the flow's roughness and its whitewater power come to along it.
Part 3  A reach that answers its source: solved at several levels of the lake above its sill, and the row
        between two neighbouring solutions against a solve at that level.
Part 4  A fall: the sheet's speed, throw and thickness by height, the share of it past its breakup length,
        the plunge's power, the depth the scheme gives at a lip, and the dent the plunge would make if all
        its momentum went to the surface.
Part 5  Reaching the steady state: steps from a dry start and from a filled start, with one time step for
        all cells and with each cell's own.
Part 6  A stream in plan: a narrows and a rock. Discharge through sections, the speed-up, the rise in front
        of the rock, and where the flow loses head.
Part 7  One mean current for an interaction window: the drift it gives rings in a pool beside a riffle,
        against a mean weighted by where the waves are.

Run: python3 river_proof.py
"""
import math

G = 9.81
RHO = 1000.0
DRY = 1e-6


def hll(hl, ul, hr, ur):
    """Mass and momentum flux between two reconstructed states."""
    if hl <= DRY and hr <= DRY:
        return 0.0, 0.0
    cl = math.sqrt(G * hl) if hl > 0 else 0.0
    cr = math.sqrt(G * hr) if hr > 0 else 0.0
    if hl <= DRY:
        sl, sr = ur - 2 * cr, ur + cr
    elif hr <= DRY:
        sl, sr = ul - cl, ul + 2 * cl
    else:
        us = 0.5 * (ul + ur) + cl - cr
        cs = 0.5 * (cl + cr) + 0.25 * (ul - ur)
        sl, sr = min(ul - cl, us - cs), max(ur + cr, us + cs)
    fl0, fl1 = hl * ul, hl * ul * ul + 0.5 * G * hl * hl
    fr0, fr1 = hr * ur, hr * ur * ur + 0.5 * G * hr * hr
    if sl >= 0:
        return fl0, fl1
    if sr <= 0:
        return fr0, fr1
    d = sr - sl
    return ((sr * fl0 - sl * fr0 + sl * sr * (hr - hl)) / d,
            (sr * fl1 - sl * fr1 + sl * sr * (hr * ur - hl * ul)) / d)


def face(hl, ul, zl, hr, ur, zr, recon):
    """Flux through a face with the bed source folded in: (mass, momentum seen by the left cell,
    momentum seen by the right cell)."""
    if recon == "audusse":
        zf = max(zl, zr)
        sl = max(0.0, hl + zl - zf)
        sr = max(0.0, hr + zr - zf)
    else:  # Chen and Noelle 2017
        zf = min(max(zl, zr), min(hl + zl, hr + zr))
        sl = min(hl + zl - zf, hl)
        sr = min(hr + zr - zf, hr)
    f0, f1 = hll(sl, ul, sr, ur)
    return f0, f1 + 0.5 * G * (hl + sl) * (zf - zl), f1 + 0.5 * G * (hr + sr) * (zf - zr)


def solve1d(z, dx, n, up, down, h, q, recon="cn", local=True, cfl=0.45, max_steps=200000, tol=1e-7,
            want_q=None):
    """March to a steady state. up: ("q", value) an inflow, ("level", value) a reservoir, ("state", h, q).
    down: ("level", value) or ("free",). Returns h, q, steps, cell updates."""
    N = len(z)
    h = list(h)
    q = list(q)
    updates = 0
    for step in range(1, max_steps + 1):
        u = [q[i] / h[i] if h[i] > DRY else 0.0 for i in range(N)]
        # ghosts
        if up[0] == "q":
            gh, gu, gz = h[0], (up[1] / h[0] if h[0] > DRY else 0.0), z[0]
        elif up[0] == "level":
            gz = z[0]
            gh, gu = max(0.0, up[1] - gz), 0.0
        elif up[0] == "wall":
            gh, gu, gz = h[0], -u[0], z[0]
        else:
            gh, gu, gz = up[1], up[2] / up[1], z[0]
        if down[0] == "level":
            dz = z[-1]
            dh, du = max(0.0, down[1] - dz), u[-1]
        elif down[0] == "wall":
            dh, du, dz = h[-1], -u[-1], z[-1]
        elif down[0] == "brink":                        # a free overfall: critical where the flow above is slow
            dz = z[-1]
            if h[-1] > DRY and abs(u[-1]) < math.sqrt(G * h[-1]):
                dh = (q[-1] * q[-1] / G) ** (1.0 / 3.0)
                du = q[-1] / dh if dh > DRY else 0.0
            else:
                dh, du = h[-1], u[-1]
        else:
            dh, du, dz = h[-1], u[-1], z[-1]
        F0 = [0.0] * (N + 1)
        FL = [0.0] * (N + 1)
        FR = [0.0] * (N + 1)
        f = face(gh, gu, gz, h[0], u[0], z[0], recon)
        if up[0] == "q":
            f = (up[1], f[1], f[2])
        F0[0], FL[0], FR[0] = f
        for i in range(N - 1):
            F0[i + 1], FL[i + 1], FR[i + 1] = face(h[i], u[i], z[i], h[i + 1], u[i + 1], z[i + 1], recon)
        F0[N], FL[N], FR[N] = face(h[-1], u[-1], z[-1], dh, du, dz, recon)
        sp = [abs(u[i]) + math.sqrt(G * h[i]) if h[i] > DRY else 0.0 for i in range(N)]
        if local:
            dts = []
            for i in range(N):
                s = max(sp[max(0, i - 1)], sp[i], sp[min(N - 1, i + 1)], 0.3)
                dts.append(cfl * dx / s)
        else:
            dts = [cfl * dx / max(max(sp), 0.3)] * N
        change = 0.0
        for i in range(N):
            dt = dts[i]
            hn = h[i] - dt / dx * (F0[i + 1] - F0[i])
            qn = q[i] - dt / dx * (FL[i + 1] - FR[i])
            if hn <= DRY:
                hn = max(hn, 0.0)
                qn = 0.0
            else:
                # friction linearised about the old discharge, so the steady state does not depend on dt
                qn = qn / (1.0 + dt * G * n * n * abs(q[i]) / hn ** (7.0 / 3.0))
            change = max(change, abs(hn - h[i]) / dt, abs(qn - q[i]) / dt)
            h[i], q[i] = hn, qn
        updates += N
        if change < tol:
            if want_q is None or max(abs(F0[i] - want_q) for i in range(N + 1)) < 1e-4 * abs(want_q):
                return h, q, step, updates, F0
    return h, q, max_steps, updates, F0


def bisect(f, a, b, it=200):
    fa = f(a)
    for _ in range(it):
        m = 0.5 * (a + b)
        fm = f(m)
        if (fm > 0) == (fa > 0):
            a, fa = m, fm
        else:
            b = m
    return 0.5 * (a + b)


# ------------------------------------------------------------------------------------------------ part 1
def part1():
    print("PART 1  the scheme")
    # 1a water at rest round dry ground
    N, dx = 100, 0.25
    z = [0.5 * math.exp(-((i + 0.5) * dx - 12.5) ** 2 / 8.0) + 0.3 * math.sin(i * 0.9) ** 2 * 0.2 for i in range(N)]
    level = 0.35
    h = [max(0.0, level - zi) for zi in z]
    hh, qq, steps, _, _ = solve1d(z, dx, 0.03, ("wall",), ("wall",), h, [0.0] * N, max_steps=2000, tol=-1.0)
    dry = sum(1 for x in h if x == 0.0)
    print("  1a at rest, %d of %d cells dry, after 2000 steps: largest level change %.2e m, largest speed %.2e m/s"
          % (dry, N, max(abs(hh[i] + z[i] - level) for i in range(N) if h[i] > 0),
             max(abs(qq[i] / hh[i]) if hh[i] > DRY else 0.0 for i in range(N))))

    # 1b steady flow over a bump with a jump (the SWASHES transcritical case)
    L, qin, hd = 25.0, 0.18, 0.33
    def bump(x):
        return 0.2 - 0.05 * (x - 10.0) ** 2 if 8.0 < x < 12.0 else 0.0
    hc = (qin * qin / G) ** (1.0 / 3.0)
    Ec = hc + 0.2 + qin * qin / (2 * G * hc * hc)
    Ed = hd + qin * qin / (2 * G * hd * hd)
    def sub(E, zb):
        return bisect(lambda x: x + qin * qin / (2 * G * x * x) + zb - E, hc, 5.0)
    def sup(E, zb):
        return bisect(lambda x: x + qin * qin / (2 * G * x * x) + zb - E, 1e-4, hc)
    def rh(x):
        h1, h2 = sup(Ec, bump(x)), sub(Ed, bump(x))
        return qin * qin * (1 / h1 - 1 / h2) + 0.5 * G * (h1 * h1 - h2 * h2)
    xs = bisect(rh, 11.27, 11.99)      # no slow branch exists nearer the crest than 11.25 m
    def exact(x):
        if x < 10.0:
            return sub(Ec, bump(x))
        if x < xs:
            return sup(Ec, bump(x))
        return sub(Ed, bump(x))
    for dx in (0.25, 0.125):
        N = int(L / dx)
        z = [bump((i + 0.5) * dx) for i in range(N)]
        h, q, steps, _, _ = solve1d(z, dx, 0.0, ("q", qin), ("level", hd), [max(0.0, hd - zi) for zi in z],
                                    [0.0] * N, max_steps=60000, tol=1e-7)
        err = [abs(h[i] - exact((i + 0.5) * dx)) for i in range(N)]
        away = [err[i] for i in range(N) if abs((i + 0.5) * dx - xs) > 3 * dx]
        jump = min(range(N - 1), key=lambda i: h[i] - h[i + 1])
        print("  1b bump with a jump, cells of %.3f m: mean depth error %.4f m (%.1f%% of the critical depth), "
              "largest away from the jump %.4f m; jump at %.2f m against %.2f m; %d steps"
              % (dx, sum(err) / N, 100 * sum(err) / N / hc, max(away), (jump + 1) * dx, xs, steps))

    # 1c steady flow with friction over a bed made for a known depth
    for name, base, amp, qin, n, dx in (("slow", 0.50, 0.10, 0.30, 0.03, 0.5), ("fast", 0.12, 0.02, 0.30, 0.03, 0.5),
                                        ("fast", 0.12, 0.02, 0.30, 0.03, 0.25), ("fast", 0.12, 0.02, 0.30, 0.03, 0.125)):
        L = 100.0
        N = int(L / dx)
        def depth(x):
            return base + amp * math.sin(2 * math.pi * x / L)
        def ddepth(x):
            return amp * 2 * math.pi / L * math.cos(2 * math.pi * x / L)
        def slope(x):
            hx = depth(x)
            return (qin * qin / (G * hx ** 3) - 1.0) * ddepth(x) - n * n * qin * qin / hx ** (10.0 / 3.0)
        z, acc, m = [], 0.0, 40
        zx = 0.0
        fine = dx / m
        zs = [0.0]
        for k in range(N * m):
            x = k * fine
            zx += 0.5 * (slope(x) + slope(x + fine)) * fine
            zs.append(zx)
        z = [zs[i * m + m // 2] for i in range(N)]
        fr = qin / depth(0) / math.sqrt(G * depth(0))
        if fr < 1:
            up, down = ("q", qin), ("level", depth(L) + zs[-1])
        else:
            up, down = ("state", depth(0), qin), ("free",)
        h, q, steps, _, _ = solve1d(z, dx, n, up, down, [depth((i + 0.5) * dx) * 0.8 for i in range(N)],
                                    [qin] * N, max_steps=60000, tol=1e-8)
        err = max(abs(h[i] - depth((i + 0.5) * dx)) for i in range(2, N - 2))
        print("  1c friction, %s (Froude %.2f), cells of %.3f m: largest depth error %.4f m, %.1f%% of the depth; "
              "%d steps" % (name, fr, dx, err, 100 * err / base, steps))

    # 1d a thin sheet on a steep slope: the bed falls more across a cell than the water is deep
    for S, dx in ((0.3, 0.25), (0.3, 0.5), (1.0, 0.25)):
        qin, n = 0.02, 0.05
        dn = (qin * n / math.sqrt(S)) ** 0.6
        N = 80
        z = [-(i + 0.5) * dx * S for i in range(N)]
        out = []
        for recon in ("audusse", "cn"):
            h, q, steps, _, _ = solve1d(z, dx, n, ("q", qin), ("free",), [0.01] * N, [0.0] * N, recon=recon,
                                        max_steps=40000, tol=1e-8)
            mid = h[N // 2:N - 5]
            out.append(sum(mid) / len(mid))
        print("  1d slope %.1f, cells of %.2f m (bed falls %.3f m a cell), normal depth %.4f m: "
              "Audusse %.4f m (%+.0f%%), Chen and Noelle %.4f m (%+.0f%%)"
              % (S, dx, S * dx, dn, out[0], 100 * (out[0] / dn - 1), out[1], 100 * (out[1] / dn - 1)))


# ------------------------------------------------------------------------------------------------ part 2
def stream_bed(dx):
    """120 m of stream in section. Returns bed heights at cell centres and named stations."""
    N = int(120 / dx)
    z = []
    for i in range(N):
        x = (i + 0.5) * dx
        if x < 25:
            zz = 12.0 - 0.03 * x                       # riffle, 3%
        elif x < 45:
            crest = 12.0 - 0.75                        # sill crest at 45 m
            zz = crest - 0.8 * math.sin(math.pi * (x - 25) / 20.0) ** 2   # pool
        elif x < 60:
            zz = 11.25 - 0.12 * (x - 45)               # chute, 12%
        elif x < 85:
            zz = 9.45 - 0.4 - 0.01 * (x - 60)          # ledge of 0.4 m, then a run at 1%
        else:
            zz = 8.80 - 0.005 * (x - 85)               # glide at 0.5%
        z.append(zz)
    return z


def filled_start(z, film=0.01):
    """Pools filled to their sills, a film elsewhere."""
    N = len(z)
    spill = [0.0] * N
    m = -1e9
    for i in range(N - 1, -1, -1):
        m = max(m, z[i])
        spill[i] = m
    return [max(film, spill[i] - z[i]) for i in range(N)]


def losses(z, h, q, dx, n):
    """Head, friction power and whole power lost per square metre, as head flux in m2/s."""
    N = len(z)
    H = [z[i] + h[i] + (q[i] / h[i]) ** 2 / (2 * G) if h[i] > DRY else z[i] for i in range(N)]
    whole, fric = [0.0] * N, [0.0] * N
    for i in range(1, N - 1):
        whole[i] = -q[i] * (H[i + 1] - H[i - 1]) / (2 * dx)
        if h[i] > DRY:
            fric[i] = abs(q[i]) * n * n * (q[i] / h[i]) ** 2 / h[i] ** (4.0 / 3.0)
    return H, whole, fric


C_CHOP = 2.0          # the chop's rms height is this times depth times friction slope
LIMIT = 0.6           # WATERS' depth limit on a layer's significant height


def part2():
    print("PART 2  a stream in section")
    dx, qin, n = 0.5, 0.15, 0.04
    z = stream_bed(dx)
    N = len(z)
    lake = z[-1] + 0.45
    h, q, steps, upd, F0 = solve1d(z, dx, n, ("q", qin), ("level", lake), filled_start(z), [0.0] * N,
                                   max_steps=80000, tol=1e-7, want_q=qin)
    print("  %d cells of %.2f m, %.2f m2/s, Manning %.2f: steady after %d steps" % (N, dx, qin, n, steps))
    print("  discharge through every face within %.3f%% of the inflow"
          % (100 * max(abs(f - qin) for f in F0) / qin))
    pool = [i for i in range(N) if 27 < (i + 0.5) * dx < 43]
    lv = [h[i] + z[i] for i in pool]
    crest = 11.25
    print("  the pool's level is flat to %.1f mm and stands %.3f m over its sill (critical depth there %.3f m)"
          % (1000 * (max(lv) - min(lv)), sum(lv) / len(lv) - crest, (qin * qin / G) ** (1 / 3.0)))
    H, whole, fric = losses(z, h, q, dx, n)
    tot = sum(whole[1:N - 1]) * dx
    print("  energy account: head lost cell by cell sums to %.4f m3/s.m, discharge times the head between the "
          "ends is %.4f (%.1f%% apart)" % (tot, qin * (H[1] - H[N - 2]), 100 * abs(tot / (qin * (H[1] - H[N - 2])) - 1)))
    cap = LIMIT / (4 * C_CHOP)
    print("  station              depth  speed  Froude  fr.slope  chop rms  white W/m2")
    for name, x in (("riffle, 3%", 15), ("pool", 35), ("chute, 12%", 52), ("below the ledge", 61),
                    ("run, 1%", 75), ("glide, 0.5%", 100)):
        i = int(x / dx)
        u = q[i] / h[i]
        sf = n * n * u * u / h[i] ** (4.0 / 3.0)
        chop = C_CHOP * h[i] * min(sf, cap)
        form = max(0.0, whole[i] - fric[i])
        white = RHO * G * (form + abs(q[i]) * max(0.0, sf - cap))
        print("  %-20s %5.3f  %5.2f  %5.2f   %7.4f   %5.1f mm  %8.1f"
              % (name, h[i], u, u / math.sqrt(G * h[i]), sf, 1000 * chop, white))
    led = [RHO * G * max(0.0, whole[i] - fric[i]) for i in range(int(58 / dx), int(66 / dx))]
    print("  the ledge and the jump below it: up to %.0f W/m2 of head lost beyond friction; elsewhere under "
          "%.0f W/m2 outside the chute"
          % (max(led), max(RHO * G * max(0.0, whole[i] - fric[i]) for i in range(2, N - 2)
                           if not 44 < (i + 0.5) * dx < 67)))
    return z, h, q, dx, n


# ------------------------------------------------------------------------------------------------ part 3
def part3():
    print("PART 3  a reach that answers its source")
    dx, n = 0.5, 0.04
    N = 160
    z = []
    for i in range(N):
        x = (i + 0.5) * dx
        z.append(2.0 - 0.3 * max(0.0, 6.0 - x) ** 2 / 36.0 * 0 - (0.0 if x < 6 else 0.02 * (x - 6))
                 + 0.15 * math.exp(-((x - 40) / 4.0) ** 2))
    sill = max(z[:20])
    heads = [0.05 + 0.075 * k for k in range(9)]            # the lake above the sill, 0.05 to 0.65 m
    sol = {}
    for hd in heads:
        h0 = [max(0.01, sill + hd - zi) if i < 12 else 0.02 for i, zi in enumerate(z)]
        h, q, steps, _, F0 = solve1d(z, dx, n, ("level", sill + hd), ("free",), h0, [0.0] * N,
                                     max_steps=60000, tol=1e-7)
        sol[hd] = (h, q, F0[N // 2])
    print("  head over the sill and discharge: " + ", ".join("%.3f m %.4f m2/s" % (hd, sol[hd][2]) for hd in heads[::2]))
    print("  discharge over head to the power 1.5: " + ", ".join("%.2f" % (sol[hd][2] / hd ** 1.5) for hd in heads[::2]))
    for stride, label in ((2, "samples 0.15 m apart"), (4, "samples 0.30 m apart")):
        worst_l = worst_q = 0.0
        for k in range(0, len(heads) - stride, stride):
            a, b = heads[k], heads[k + stride]
            for j in range(k + 1, k + stride):
                m = heads[j]
                w = (m - a) / (b - a)
                for i in range(12, N - 2):
                    hb = (1 - w) * sol[a][0][i] + w * sol[b][0][i]
                    ub = (1 - w) * sol[a][1][i] / sol[a][0][i] + w * sol[b][1][i] / sol[b][0][i]
                    worst_l = max(worst_l, abs(hb - sol[m][0][i]))
                    um = sol[m][1][i] / sol[m][0][i]
                    worst_q = max(worst_q, abs(ub - um) / um)
        print("  %s: the row between two solutions is within %.1f mm of a solve at that level in depth "
              "and %.1f%% in speed" % (label, 1000 * worst_l, 100 * worst_q))
    a, b, m = heads[0], heads[8], heads[4]
    i = 60
    print("  left as prepared at the middle level while the lake moves over its range: the reach is wrong by "
          "%.0f%% to %.0f%% in discharge" % (100 * (sol[m][2] / sol[a][2] - 1), 100 * (sol[m][2] / sol[b][2] - 1)))


# ------------------------------------------------------------------------------------------------ part 4
def part4():
    print("PART 4  a fall")
    print("  unit discharge, height: brink speed, time, throw, speed at the foot, thickness there, "
          "breakup length, share of the height past it, power a metre of lip")
    for qq in (0.005, 0.05, 0.2, 1.0):
        hc = (qq * qq / G) ** (1 / 3.0)
        hb = 0.715 * hc
        ub = qq / hb
        lb = 6.0 * qq ** 0.32
        for height in (1.0, 3.0, 8.0, 12.0):
            t = math.sqrt(2 * height / G)
            v = math.hypot(ub, G * t)
            print("  %5.3f m2/s %4.0f m: %4.2f m/s  %4.2f s  %4.2f m  %5.2f m/s  %5.1f mm  %4.2f m  %3.0f%%  %7.0f W"
                  % (qq, height, ub, t, ub * t, v, 1000 * qq / v, lb, 100 * max(0.0, 1 - lb / height),
                     RHO * G * qq * height))
    for S, n, qin in ((0.005, 0.04, 0.15), (0.005, 0.04, 0.6), (0.05, 0.04, 0.15)):
        out = []
        for dx in (0.5, 0.25):
            N = int(30 / dx)
            z = [-(i + 0.5) * dx * S for i in range(N)]
            h, q, steps, _, _ = solve1d(z, dx, n, ("q", qin), ("brink",), [0.2] * N, [0.0] * N,
                                        max_steps=60000, tol=1e-7)
            hc = (qin * qin / G) ** (1 / 3.0)
            out.append("%.2f at %.2f m cells" % (h[-1] / hc, dx))
        dn = (qin * n / math.sqrt(S)) ** 0.6
        print("  the last cell above a lip, slope %.3f, %.2f m2/s, Froude %.2f upstream: depth over the critical "
              "depth %s" % (S, qin, qin / dn / math.sqrt(G * dn), ", ".join(out)))
    N, dx, S, n, qin = 120, 0.25, 0.005, 0.04, 0.15
    z = [-(i + 0.5) * dx * S for i in range(N)] + [-30 * S - 3.0] * 3
    h, q, steps, _, _ = solve1d(z, dx, n, ("q", qin), ("free",), [0.2] * N + [0.0] * 3, [0.0] * (N + 3),
                                max_steps=60000, tol=1e-7)
    print("  with a dry cell 3 m lower in place of the lip's face, the last cell above stands at %.2f of the "
          "critical depth, and a sheet launched from it would be thrown %.1f times too far"
          % (h[N - 1] / (qin * qin / G) ** (1 / 3.0), 0.715 * (qin * qin / G) ** (1 / 3.0) / h[N - 1]))
    qq, height, foot = 0.2, 8.0, 0.4
    v = math.sqrt(2 * G * height)
    print("  if all the momentum of %.1f m2/s falling %.0f m pressed on %.1f m of surface, the dent would be "
          "%.1f m deep: the plunge goes under, and only a share of its unsteady part makes waves"
          % (qq, height, foot, qq * v / (G * foot)))


# ------------------------------------------------------------------------------------------------ part 5
def part5(z, dx, n):
    print("PART 5  reaching the steady state")
    qin = 0.15
    N = len(z)
    lake = z[-1] + 0.45
    rows = []
    for name, start, local in (("dry start, one time step", [0.0] * N, False),
                               ("dry start, each cell's own step", [0.0] * N, True),
                               ("filled start, one time step", filled_start(z), False),
                               ("filled start, each cell's own step", filled_start(z), True)):
        h, q, steps, upd, F0 = solve1d(z, dx, n, ("q", qin), ("level", lake), start, [0.0] * N, local=local,
                                       max_steps=400000, tol=1e-7, want_q=qin)
        rows.append((name, steps, upd))
        print("  %-36s %7d steps, %5.1f million cell updates" % (name, steps, upd / 1e6))
    coarse = [0.5 * (z[2 * i] + z[2 * i + 1]) for i in range(N // 2)]
    h, q, s1, u1, _ = solve1d(coarse, 2 * dx, n, ("q", qin), ("level", lake), filled_start(coarse), [0.0] * (N // 2),
                              max_steps=400000, tol=1e-7, want_q=qin)
    h2 = [max(0.0, h[i // 2] + coarse[i // 2] - z[i]) for i in range(N)]
    q2 = [q[i // 2] if h2[i] > DRY else 0.0 for i in range(N)]
    h, q, s2, u2, _ = solve1d(z, dx, n, ("q", qin), ("level", lake), h2, q2, max_steps=400000, tol=1e-7, want_q=qin)
    print("  %-36s %7d steps coarse and %d fine, %5.1f million cell updates" %
          ("a grid twice as coarse first", s1, s2, (u1 + u2) / 1e6))
    wet = 350 * 4 / 0.25 ** 2
    best = min(r[1] for r in rows)
    steps = best * (350 / 0.25) / N                     # steps grow with the stream's length in cells
    print("  a 350 m stream 4 m wide at 0.25 m cells has %.0f wet cells; at %d steps for this section's %d "
          "cells, and so about %.0f for its 1,400, that is %.0f million cell updates for each flow sample"
          % (wet, best, N, steps, wet * steps / 1e6))


# ------------------------------------------------------------------------------------------------ part 6
def part6():
    print("PART 6  a stream in plan: a narrows and a rock")
    dx, n, Q = 0.5, 0.04, 0.6
    NX, NY = 64, 16
    yc = NY * dx / 2
    z = [[0.0] * NY for _ in range(NX)]
    for i in range(NX):
        x = (i + 0.5) * dx
        narrow = math.exp(-((x - 12.0) / 2.5) ** 2)
        a = 0.06 + 0.5 * narrow                         # the valley's cross-section steepens at the narrows
        for j in range(NY):
            y = (j + 0.5) * dx
            zz = 2.0 - 0.02 * x + a * (y - yc) ** 2
            zz += 0.7 * math.exp(-(((x - 22.0) ** 2 + (y - yc - 0.2) ** 2) / 0.5))   # a rock
            z[i][j] = zz
    lake = 2.0 - 0.02 * NX * dx + 0.30
    h = [[max(0.02 if abs((j + 0.5) * dx - yc) < 2.0 else 0.0, lake - z[i][j]) for j in range(NY)] for i in range(NX)]
    hu = [[0.0] * NY for _ in range(NX)]
    hv = [[0.0] * NY for _ in range(NX)]
    src = [(1, j) for j in range(NY) if abs((j + 0.5) * dx - yc) < 1.0]
    add = Q / len(src) / (dx * dx)
    cfl = 0.4
    for step in range(1, 9001):
        # x faces: i-1/2 for i in 0..NX, y faces likewise
        FX = [[(0.0, 0.0, 0.0, 0.0)] * NY for _ in range(NX + 1)]
        FY = [[(0.0, 0.0, 0.0, 0.0)] * (NY + 1) for _ in range(NX)]
        for j in range(NY):                             # the wall at the head of the valley
            hr = h[0][j]
            if hr > DRY:
                ur, vr = hu[0][j] / hr, hv[0][j] / hr
                f0, fl, fr = face(hr, -ur, z[0][j], hr, ur, z[0][j], "cn")
                FX[0][j] = (0.0, fl, fr, 0.0)
        for i in range(NX):                             # the walls along its sides
            for j, jj in ((0, 0), (NY, NY - 1)):
                hh = h[i][jj]
                if hh > DRY:
                    vv = hv[i][jj] / hh
                    if j == 0:
                        f0, fl, fr = face(hh, -vv, z[i][jj], hh, vv, z[i][jj], "cn")
                    else:
                        f0, fl, fr = face(hh, vv, z[i][jj], hh, -vv, z[i][jj], "cn")
                    FY[i][j] = (0.0, fl, fr, 0.0)
        for i in range(1, NX + 1):
            for j in range(NY):
                hl = h[i - 1][j]
                if i < NX:
                    hr, zr = h[i][j], z[i][j]
                    ur = hu[i][j] / hr if hr > DRY else 0.0
                    vr = hv[i][j] / hr if hr > DRY else 0.0
                else:                                   # the lake
                    zr = z[i - 1][j]
                    hr = max(0.0, lake - zr)
                    ur = hu[i - 1][j] / hl if hl > DRY else 0.0
                    vr = 0.0
                if hl <= DRY and hr <= DRY:
                    continue
                ul = hu[i - 1][j] / hl if hl > DRY else 0.0
                vl = hv[i - 1][j] / hl if hl > DRY else 0.0
                f0, fl, fr = face(hl, ul, z[i - 1][j], hr, ur, zr, "cn")
                FX[i][j] = (f0, fl, fr, f0 * (vl if f0 > 0 else vr))
        for i in range(NX):
            for j in range(1, NY):
                hl, hr = h[i][j - 1], h[i][j]
                if hl <= DRY and hr <= DRY:
                    continue
                vl = hv[i][j - 1] / hl if hl > DRY else 0.0
                vr = hv[i][j] / hr if hr > DRY else 0.0
                ul = hu[i][j - 1] / hl if hl > DRY else 0.0
                ur = hu[i][j] / hr if hr > DRY else 0.0
                f0, fl, fr = face(hl, vl, z[i][j - 1], hr, vr, z[i][j], "cn")
                FY[i][j] = (f0, fl, fr, f0 * (ul if f0 > 0 else ur))
        change = 0.0
        for i in range(NX):
            for j in range(NY):
                hh = h[i][j]
                a, b, c, d = FX[i][j], FX[i + 1][j], FY[i][j], FY[i][j + 1]
                if hh <= DRY and a[0] == 0.0 and b[0] == 0.0 and c[0] == 0.0 and d[0] == 0.0 and (i, j) not in src:
                    continue
                s = 0.3
                for (ii, jj) in ((i, j), (max(0, i - 1), j), (min(NX - 1, i + 1), j), (i, max(0, j - 1)), (i, min(NY - 1, j + 1))):
                    hn = h[ii][jj]
                    if hn > DRY:
                        s = max(s, math.hypot(hu[ii][jj], hv[ii][jj]) / hn + math.sqrt(G * hn))
                dt = cfl * dx / s / 2
                hn = hh - dt / dx * (b[0] - a[0] + d[0] - c[0])
                un = hu[i][j] - dt / dx * (b[1] - a[2] + d[3] - c[3])
                vn = hv[i][j] - dt / dx * (d[1] - c[2] + b[3] - a[3])
                if (i, j) in src:
                    hn += dt * add
                if hn <= DRY:
                    hn, un, vn = max(hn, 0.0), 0.0, 0.0
                else:
                    k = 1.0 + dt * G * n * n * math.hypot(hu[i][j], hv[i][j]) / hn ** (7.0 / 3.0)
                    un, vn = un / k, vn / k
                change = max(change, abs(hn - hh) / dt)
                h[i][j], hu[i][j], hv[i][j] = hn, un, vn
        if step % 500 == 0 and change < 2e-5:
            break
    print("  %d by %d cells of %.1f m, %.2f m3/s from a spring: stopped after %d steps, largest level rate %.1e m/s"
          % (NX, NY, dx, Q, step, change))
    sect = [sum(FX[i][j][0] for j in range(NY)) * dx for i in (6, 16, 24, 36, 50, 62)]
    print("  discharge through six sections: " + ", ".join("%.3f" % s for s in sect)
          + " m3/s (within %.1f%% of the spring)" % (100 * max(abs(s - Q) for s in sect) / Q))
    def width(i):
        return sum(1 for j in range(NY) if h[i][j] > 0.01) * dx
    def axis(i):
        j = NY // 2
        return h[i][j], hu[i][j] / h[i][j]
    for name, i in (("above the narrows", 12), ("in the narrows", 24), ("below it", 34)):
        d, u = axis(i)
        print("  %-18s wet width %.1f m, depth %.2f m, speed %.2f m/s, Froude %.2f"
              % (name, width(i), d, u, u / math.sqrt(G * d)))
    # the rock at x = 22 m, cell 44
    i0, j0 = 44, NY // 2
    ahead = max(h[i][j] + z[i][j] for i in range(40, 44) for j in range(j0 - 1, j0 + 2) if h[i][j] > 0.01)
    far = h[36][j0] + z[36][j0]
    u_far = hu[36][j0] / h[36][j0]
    drop_bed = 0.02 * (42 - 36) * dx
    print("  in front of the rock the water stands %.3f m above the level the slope alone would give; "
          "the speed's head there is %.3f m" % (ahead - (far - drop_bed), u_far ** 2 / (2 * G)))
    # where head is lost beyond friction
    def head(i, j):
        hh = h[i][j]
        return z[i][j] + hh + (hu[i][j] ** 2 + hv[i][j] ** 2) / (hh * hh * 2 * G) if hh > 0.02 else None
    form = [[0.0] * NY for _ in range(NX)]
    total = 0.0
    for i in range(3, NX - 1):
        for j in range(1, NY - 1):
            H0 = head(i, j)
            if H0 is None:
                continue
            gx = gy = 0.0
            a, b = head(i - 1, j), head(i + 1, j)
            if a is not None and b is not None:
                gx = (b - a) / (2 * dx)
            a, b = head(i, j - 1), head(i, j + 1)
            if a is not None and b is not None:
                gy = (b - a) / (2 * dx)
            whole = -(hu[i][j] * gx + hv[i][j] * gy)
            sp = math.hypot(hu[i][j], hv[i][j]) / h[i][j]
            fr = sp * h[i][j] * n * n * sp * sp / h[i][j] ** (4.0 / 3.0)
            form[i][j] = max(0.0, whole - fr)
            total += form[i][j]
    near = sum(form[i][j] for i in range(42, 54) for j in range(NY))
    nar = sum(form[i][j] for i in range(22, 34) for j in range(NY))
    wet = sum(1 for i in range(3, NX - 1) for j in range(NY) if h[i][j] > 0.02)
    wr = sum(1 for i in range(42, 54) for j in range(NY) if h[i][j] > 0.02)
    wn = sum(1 for i in range(22, 34) for j in range(NY) if h[i][j] > 0.02)
    print("  head lost beyond friction: %.0f%% of it in the 6 m round and behind the rock (%.0f%% of the wet "
          "cells), %.0f%% in the 6 m below the narrows (%.0f%% of them)"
          % (100 * near / total, 100 * wr / wet, 100 * nar / total, 100 * wn / wet))
    peak = max(form[i][j] for i in range(42, 54) for j in range(NY))
    print("  largest by the rock: %.0f W/m2" % (RHO * G * peak))


# ------------------------------------------------------------------------------------------------ part 7
def part7(z, h, q, dx):
    print("PART 7  one mean current for a window")
    N = len(z)
    win = [i for i in range(N) if 4 < (i + 0.5) * dx < 68]            # a 64 m window over riffle, pool and chute
    plain = sum(q[i] / h[i] for i in win) / len(win)
    pool = [i for i in win if 30 < (i + 0.5) * dx < 40]
    local = sum(q[i] / h[i] for i in pool) / len(pool)
    print("  a 64 m window over the riffle, the pool and the chute: the mean current of its wet cells is "
          "%.2f m/s; in the pool the water moves at %.2f m/s" % (plain, local))
    print("  rings from a stone in the pool, carried by the plain mean, slide %.1f m downstream in 5 s; "
          "carried by a mean weighted by the waves' own energy, which is all in the pool, %.2f m; "
          "they should slide %.2f m" % (plain * 5, local * 5, local * 5))
    c = math.sqrt(G * 0.6)
    print("  a ring's own speed on the pool's 0.6 m of water is at most %.1f m/s, so the plain mean's drift "
          "is %.0f%% of it" % (c, 100 * (plain - local) / c))


if __name__ == "__main__":
    part1()
    z, h, q, dx, n = part2()
    part3()
    part4()
    part5(z, dx, n)
    part6()
    part7(z, h, q, dx)
