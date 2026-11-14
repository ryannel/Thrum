"""WATERS design, revision of 14 November 2026 for surf, mirror views and the view from under water:
reproductions N to Q (H to M are in water_proof.py, A to G in ../../SEA/design-proof).
Pure Python, no dependencies. About a minute to run.

Part N  The shore transform. A long-wave component's phase over a bed is a lag added to its deep-water
        phase, from an eikonal solve on the bed's grid. N1: the solve against Snell's closed form on a plane
        beach, at normal and oblique incidence. N2: one lag for a ladder of wavelengths a factor of two
        apart, blended for the wavelengths between. N3: one lag for each sampled stage, blended for the
        stages between. N4: an island: crests turn to face every shore, and how much of a 45 degree train
        each shore is open to.
Part O  Breaking. O1: where a regular wave breaks under linear shoaling, against Komar and Gaughan's
        formula. O2: the larger waves of a set break further out. O3: a crest thrown forward by a shear of
        its own height overhangs and never crosses itself. O4: the swash's closed form (part L) with the
        shore transform switched on.
Part P  The mirror view. A reflected ray is looked up in an image drawn from the camera's mirror image in
        the water's plane, corrected for the surface's own height and slope by a fixed-point step against
        the mirror's depth. Error against the true reflected ray, for flat water, ripples and waves.
Part Q  Under water. Snell's window, the Fresnel split seen from beneath, and the closed form for light
        scattered into a path through water lit from above, against a numerical integral.

Run: python3 surf_view_proof.py
"""
import math

G = 9.81


# ---------------------------------------------------------------- shared wave theory

def ratio(u):
    """k / k0 for deep-water wavenumber k0 on depth d, with u = k0 d: the root of F tanh(F u) = 1."""
    f = 1.0 / math.sqrt(math.tanh(u)) if u > 1e-9 else 1e5
    for _ in range(60):
        t = math.tanh(f * u)
        g = f * t - 1.0
        dg = t + f * u * (1.0 - t * t)
        f -= g / dg
    return f


def shoal(u):
    """Ks = sqrt(Cg0 / Cg) for u = k0 d."""
    f = ratio(u)
    kd = f * u
    n = 0.5 * (1.0 + 2.0 * kd / math.sinh(2.0 * kd)) if kd < 300 else 0.5
    # c / c0 = k0 / k = 1 / f ; Cg = n c ; Cg0 = c0 / 2
    return math.sqrt(0.5 / (n / f))


# ---------------------------------------------------------------- part N

def sweep(phi, speed, h, fixed, passes):
    """Fast sweeping for |grad phi| = speed, first order, Godunov upwind. speed None = land."""
    nx, ny = len(phi), len(phi[0])
    orders = [(range(nx), range(ny)), (range(nx - 1, -1, -1), range(ny)),
              (range(nx), range(ny - 1, -1, -1)), (range(nx - 1, -1, -1), range(ny - 1, -1, -1))]
    for _ in range(passes):
        for xs, ys in orders:
            for i in xs:
                row, frow, srow = phi[i], fixed[i], speed[i]
                for j in ys:
                    if frow[j] or srow[j] is None:
                        continue
                    a = min(phi[i - 1][j] if i > 0 else 1e30, phi[i + 1][j] if i < nx - 1 else 1e30)
                    b = min(row[j - 1] if j > 0 else 1e30, row[j + 1] if j < ny - 1 else 1e30)
                    nh = srow[j] * h
                    if abs(a - b) >= nh:
                        v = min(a, b) + nh
                    else:
                        v = 0.5 * (a + b + math.sqrt(2.0 * nh * nh - (a - b) ** 2))
                    if v < row[j]:
                        row[j] = v
    return phi


def part_n():
    print("Part N: the shore transform")
    print(" N1: an 80 m wave on a plane beach of 1 in 10 (80 m deep at the open edge, depth floored at 0.5 m).")
    print("     Phase from the eikonal solve on the bed's grid against Snell's closed form, in radians.")
    print("     incidence   cells   worst phase error, whole beach   at 2 m depth   crest's turn at 2 m: solve, Snell")
    k0 = 2 * math.pi / 80.0
    floor = 0.5
    for theta in (0.0, 30.0, 60.0):
        for h in (4.0, 2.0):
            nx, ny = int(800 / h) + 1, int(320 / h) + 1
            st = math.sin(math.radians(theta))
            depth = [max(floor, (800.0 - i * h) / 10.0) for i in range(nx)]
            kloc = [k0 * ratio(k0 * d) for d in depth]
            # closed form: phi = k0 sin(theta) y + integral sqrt(k^2 - k0^2 sin^2) dx, by fine quadrature
            exact_x = [0.0] * nx
            sub = 40
            for i in range(1, nx):
                acc = 0.0
                for s in range(sub):
                    x = (i - 1 + (s + 0.5) / sub) * h
                    d = max(floor, (800.0 - x) / 10.0)
                    kk = k0 * ratio(k0 * d)
                    acc += math.sqrt(kk * kk - (k0 * st) ** 2) * h / sub
                exact_x[i] = exact_x[i - 1] + acc
            phi = [[1e30] * ny for _ in range(nx)]
            fixed = [[False] * ny for _ in range(nx)]
            speed = [[kloc[i]] * ny for i in range(nx)]
            for i in range(nx):
                for j in range(ny):
                    if i == 0 or j == 0 or j == ny - 1:
                        phi[i][j] = exact_x[i] + k0 * st * j * h
                        fixed[i][j] = True
            sweep(phi, speed, h, fixed, 3)
            worst, at2, turn = 0.0, 0.0, 0.0
            i2 = min(range(nx), key=lambda i: abs(depth[i] - 2.0))
            for i in range(1, nx - 1):
                for j in range(ny // 4, 3 * ny // 4):
                    e = abs(phi[i][j] - exact_x[i] - k0 * st * j * h)
                    worst = max(worst, e)
                    if i == i2:
                        at2 = max(at2, e)
            j = ny // 2
            gx = (phi[i2 + 1][j] - phi[i2 - 1][j]) / (2 * h)
            gy = (phi[i2][j + 1] - phi[i2][j - 1]) / (2 * h)
            turn = math.degrees(math.atan2(gy, gx))
            snell = math.degrees(math.asin(k0 * st / kloc[i2]))
            print(f"     {theta:4.0f} deg     {h:3.0f} m   {worst:6.3f}                           {at2:6.3f}         "
                  f"{turn:5.1f}, {snell:5.1f} deg from the shore's normal")

    print(" N2: lags are kept for wavelengths a factor of two apart and blended, linearly in the logarithm of the")
    print("     wavelength, for those between. Worst error of the local wavenumber, over depths of 0.5 m to 200 m:")
    rungs = [20.0 * 2 ** i for i in range(6)]
    depths = [0.5 * 1.08 ** i for i in range(80)]
    worst = (0.0, 0.0, 0.0)
    for r in range(len(rungs) - 1):
        ka, kb = 2 * math.pi / rungs[r], 2 * math.pi / rungs[r + 1]
        for s in range(1, 20):
            t = s / 20.0
            k = math.exp((1 - t) * math.log(ka) + t * math.log(kb))
            for d in depths:
                blend = (1 - t) * ratio(ka * d) + t * ratio(kb * d)
                e = abs(blend / ratio(k * d) - 1.0)
                if e > worst[0]:
                    worst = (e, 2 * math.pi / k, d)
    print(f"     {100 * worst[0]:.2f}% (a {worst[1]:.0f} m wave on {worst[2]:.2f} m of water); "
          f"with rungs a factor of four apart: ", end="")
    worst4 = 0.0
    for r in range(0, len(rungs) - 2, 2):
        ka, kb = 2 * math.pi / rungs[r], 2 * math.pi / rungs[r + 2]
        for s in range(1, 20):
            t = s / 20.0
            k = math.exp((1 - t) * math.log(ka) + t * math.log(kb))
            for d in depths:
                worst4 = max(worst4, abs(((1 - t) * ratio(ka * d) + t * ratio(kb * d)) / ratio(k * d) - 1.0))
    print(f"{100 * worst4:.2f}%")

    print(" N3: lags are kept for stages a set spacing apart and blended linearly for the stages between.")
    print("     Worst error of the local wavenumber, depth floored at 0.5 m, wavelengths of 20 m to 640 m:")
    print("     stage spacing   on 0.5 m to 1 m of water   on 1 m to 2 m   deeper than 2 m")
    for spacing in (1.0, 0.5, 0.25):
        bands = [0.0, 0.0, 0.0]
        for lam in rungs:
            k = 2 * math.pi / lam
            for di in range(1, 400):
                d = 0.5 + di * 0.025
                for s in range(1, 10):
                    off = spacing * s / 10.0
                    lo, hi = max(floor, d - off), max(floor, d - off + spacing)
                    t = off / spacing
                    blend = (1 - t) * ratio(k * lo) + t * ratio(k * hi)
                    e = abs(blend / ratio(k * d) - 1.0)
                    b = 0 if d < 1.0 else 1 if d < 2.0 else 2
                    bands[b] = max(bands[b], e)
        print(f"     {spacing:4.2f} m          {100 * bands[0]:5.1f}%                     {100 * bands[1]:5.1f}%          {100 * bands[2]:5.1f}%")

    print(" N4: an island 6 m high in 40 m of water, its shore about 80 m from its centre; an 80 m wave from the west,")
    print("     5 m cells. Around the island on the 4 m depth contour: the angle between the wave's direction and the")
    print("     shore's inward normal, with no transform and with it; the local wavelength against the depth's own;")
    print("     and the share of a 45 degree train's directions with open water behind them (its exposure).")
    h, n = 5.0, 201
    c = (n - 1) / 2

    def bed(x, y):
        return -40.0 + 46.0 * math.exp(-(x * x + y * y) / (2 * 150.0 ** 2))

    k0 = 2 * math.pi / 80.0
    speed = [[None] * n for _ in range(n)]
    phi = [[1e30] * n for _ in range(n)]
    fixed = [[False] * n for _ in range(n)]
    for i in range(n):
        for j in range(n):
            z = bed((i - c) * h, (j - c) * h)
            if z <= 0.5:
                speed[i][j] = k0 * ratio(k0 * max(floor, -z))
            if i in (0, n - 1) or j in (0, n - 1):
                phi[i][j] = k0 * (i - c) * h
                fixed[i][j] = True
    sweep(phi, speed, h, fixed, 6)
    r2 = math.sqrt(-2 * 150.0 ** 2 * math.log(36.0 / 46.0))

    def land(x, y):
        return bed(x, y) > 0.5

    def exposure(x, y, ang):
        open_rays = 0
        for s in range(17):
            a = ang + math.radians(-22.5 + 45.0 * s / 16)
            ok, t = True, 0.0
            while t < 720.0:
                t += 2.5
                px, py = x - t * math.cos(a), y - t * math.sin(a)
                if abs(px) > 500 or abs(py) > 500:
                    break
                if land(px, py):
                    ok = False
                    break
            open_rays += ok
        return open_rays / 17.0

    print("     where on the shore        direction off the normal: plain, transformed   wavelength: solve, depth's own   exposure")
    for az, name in ((180, "windward, facing the wave"), (135, "windward quarter"), (90, "side"), (45, "lee quarter"), (0, "lee")):
        x, y = r2 * math.cos(math.radians(az)), r2 * math.sin(math.radians(az))
        i, j = int(round(x / h + c)), int(round(y / h + c))
        gx = (phi[i + 1][j] - phi[i - 1][j]) / (2 * h)
        gy = (phi[i][j + 1] - phi[i][j - 1]) / (2 * h)
        inward = (-math.cos(math.radians(az)), -math.sin(math.radians(az)))
        mag = math.hypot(gx, gy)
        ang = math.degrees(math.acos(max(-1, min(1, (gx * inward[0] + gy * inward[1]) / mag))))
        plain = math.degrees(math.acos(max(-1, min(1, inward[0]))))
        d = -bed((i - c) * h, (j - c) * h)
        print(f"     {name:26s} {plain:5.0f} deg, {ang:5.0f} deg                              "
              f"{2 * math.pi / mag:5.1f} m, {2 * math.pi / (k0 * ratio(k0 * max(floor, d))):5.1f} m            {exposure(x, y, 0.0):4.2f}")
    for depth in (4.0, 2.0, 1.0):
        rr = math.sqrt(-2 * 150.0 ** 2 * math.log((40.0 - depth) / 46.0))
        worst = 0.0
        for az in range(0, 181, 15):
            x, y = rr * math.cos(math.radians(az)), rr * math.sin(math.radians(az))
            i, j = int(round(x / h + c)), int(round(y / h + c))
            if None in (speed[i + 1][j], speed[i - 1][j], speed[i][j + 1], speed[i][j - 1]) or exposure(x, y, 0.0) == 0.0:
                continue
            gx = (phi[i + 1][j] - phi[i - 1][j]) / (2 * h)
            gy = (phi[i][j + 1] - phi[i][j - 1]) / (2 * h)
            dot = (-gx * math.cos(math.radians(az)) - gy * math.sin(math.radians(az))) / math.hypot(gx, gy)
            worst = max(worst, math.degrees(math.acos(max(-1, min(1, dot)))))
        print(f"     every 15 degrees round the {depth:.0f} m contour, wherever the exposure is above zero: "
              f"at most {worst:.0f} deg off the shore's normal")
    print("     (plain: the deep-water wave, which runs out of the land on the lee side. Transformed: the first arrival")
    print("      round the island, which meets every shore from the sea. Exposure scales the train's energy.)")


# ---------------------------------------------------------------- part O

def part_o():
    print("\nPart O: breaking")
    print(" O1: a regular wave shoals by linear theory and breaks where its height is 0.78 of the depth.")
    print("     deep-water height, period   breaks at height, depth   Komar and Gaughan's height   difference")
    for h0, period in ((0.5, 6.0), (1.0, 8.0), (2.0, 10.0), (1.0, 14.0)):
        k0 = (2 * math.pi / period) ** 2 / G
        lo, hi = 0.05, 200.0
        for _ in range(80):
            d = math.sqrt(lo * hi)
            if h0 * shoal(k0 * d) > 0.78 * d:
                lo = d
            else:
                hi = d
        hb = 0.78 * d
        kg = 0.56 * h0 * (h0 / (2 * math.pi / k0)) ** -0.2
        print(f"     {h0:3.1f} m, {period:4.1f} s               {hb:4.2f} m, {d:4.2f} m            {kg:4.2f} m                      {100 * (hb / kg - 1):+5.1f}%")
    print(" O2: a set. Two trains of 9 s and 10 s in equal parts, 1 m significant height in deep water; a crest's height")
    print("     is twice the envelope where it stands. Where crests of three sizes reach 0.78 of the depth:")
    k0 = (2 * math.pi / 9.5) ** 2 / G
    for name, env in (("the smallest of the set", 0.2), ("a middling crest", 0.5), ("the largest", 0.7)):
        lo, hi = 0.05, 200.0
        for _ in range(80):
            d = math.sqrt(lo * hi)
            if 2 * env * shoal(k0 * d) > 0.78 * d:
                lo = d
            else:
                hi = d
        print(f"     {name:24s} envelope {env:3.1f} m   breaks in {d:4.2f} m of water, {d * 20:5.1f} m from the waterline on a beach of 1 in 20")
    print(" O3: a crest 1.4 m high on a 25 m wave, its front steepened by a phase skew. The section throws each point")
    print("     forward in proportion to its height above the mean level. How far the crest is thrown, the largest")
    print("     overhang (how far the lip stands in front of the face below it), and whether the curve crosses itself:")
    a, k = 0.7, 2 * math.pi / 25.0
    pts = [(i * 0.01 - 12.5) for i in range(2501)]
    for beta in (0.7, 0.95):
        def z(x):
            th = -k * x                      # travelling toward +x; the front is at x > 0
            return a * math.cos(th + beta * math.sin(th))
        steep = max(abs(z(x + 0.005) - z(x - 0.005)) / 0.01 for x in pts)
        for crest in (0.0, 1.4, 2.8, 4.2):
            throw = crest / a
            curve = [(x + throw * max(0.0, z(x)), z(x)) for x in pts]
            overhang, best = 0.0, -1e9
            for (cx, cz) in curve:
                if cx < best:
                    overhang = max(overhang, best - cx)
                best = max(best, cx)
            # a shear is one-to-one on the plane, so the image of a graph cannot cross itself; check anyway
            cross = False
            for i in range(0, len(curve) - 5, 5):
                (x1, z1), (x2, z2) = curve[i], curve[i + 5]
                for j in range(i + 10, len(curve) - 5, 5):
                    (x3, z3), (x4, z4) = curve[j], curve[j + 5]
                    d = (x2 - x1) * (z4 - z3) - (z2 - z1) * (x4 - x3)
                    if abs(d) < 1e-12:
                        continue
                    t = ((x3 - x1) * (z4 - z3) - (z3 - z1) * (x4 - x3)) / d
                    u = ((x3 - x1) * (z2 - z1) - (z3 - z1) * (x2 - x1)) / d
                    if 0 < t < 1 and 0 < u < 1:
                        cross = True
            print(f"     skew {beta:4.2f} (steepest face {steep:4.2f})   crest thrown {crest:3.1f} m   overhang {overhang:4.2f} m   "
                  f"crosses itself: {'yes' if cross else 'no'}")
    print(" O4: part L's beach (1 in 10, a 60 m regular wave) with the shore transform on: the wave's phase carries the")
    print("     lag, its height is shoaled before the limit, and the section is plain inshore of half a swash depth.")
    print("     The beach is sampled every 4 mm of height.")
    print("     sea height   waterline's highest and lowest, plain   with the transform     closed form")
    root8 = 2 * math.sqrt(2)
    gl = 0.6 / root8
    beta = 0.1
    k0 = 2 * math.pi / 60.0
    om = math.sqrt(G * k0)
    for hs in (0.16, 1.0, 2.14):
        s, a0 = 0.7 * hs, hs / root8
        # lag along the beach, by quadrature from 60 m of water
        xs = [-600.0 + i * 0.02 for i in range(int(640 / 0.02))]
        lag, acc = [], 0.0
        for x in xs:
            d = max(0.5, -beta * x)
            acc += (ratio(k0 * d) - 1.0) * 0.02
            lag.append(acc)
        out = []
        for on in (False, True):
            hi, lo = -1e9, 1e9
            for ti in range(240):
                t = ti * (2 * math.pi / om) / 240
                edge = None
                for idx in range(int((600.0 - 9.0 * s / beta - 5) / 0.02), len(xs), 2):
                    x = xs[idx]
                    d = -beta * x
                    grow = shoal(k0 * max(0.5, d)) if on else 1.0
                    amp = min(a0 * grow, max(0.0, 0.6 * (d + s)) / root8)
                    ph = k0 * (x + (lag[idx] if on else 0.0)) - om * t
                    if amp * math.cos(ph) > beta * x:
                        edge = x
                    elif edge is not None:
                        break
                if edge is not None:
                    hi, lo = max(hi, beta * edge), min(lo, beta * edge)
            out.append((hi, lo))
        print(f"     {hs:4.2f} m       {out[0][0]:+6.3f} and {out[0][1]:+6.3f} m                  {out[1][0]:+6.3f} and {out[1][1]:+6.3f} m    "
              f"{gl * s / (1 + gl):+.3f} and {-gl * s / (1 - gl):+.3f} m")


# ---------------------------------------------------------------- part P

def part_p():
    print("\nPart P: the mirror view, in a vertical section. A lake 60 m across to a bank; behind it a hill rising at 0.35")
    print("  to 40 m, and a tree 8 m tall 2 m back from the bank. The mirror image is the scene above the water's plane")
    print("  as seen from the camera's mirror image below it, with its depth, 4,000 samples across.")
    print("  A reflected ray from each surface point is found exactly, and looked up in the mirror: first where the")
    print("  surface point itself lies in the mirror image, then corrected: the ray is taken as far as the mirror's depth")
    print("  says the last sample lay, and the image is read again where that point lies. Error is the distance between")
    print("  the scenery point returned and the true one, as a share of the ray's length.")
    shore = 60.0

    def ground(x):
        return min(40.0, 0.35 * (x - shore)) if x >= shore else -1.0

    tree_x, tree_h = shore + 2.0, 8.0

    def trace(ox, oy, dx, dy):
        """First hit with the hill or the tree above the plane; None for sky."""
        best = None
        if dx > 1e-9:
            t = (tree_x - ox) / dx
            y = oy + t * dy
            if t > 0 and ground(tree_x) <= y <= ground(tree_x) + tree_h and y >= 0:
                best = t
        t, step = 0.0, 0.05
        while t < 400.0:
            t += step
            x, y = ox + t * dx, oy + t * dy
            if (y < -0.5 and dy <= 0) or x > 260 or y > 60:
                break
            if x >= shore and 0 <= y <= ground(x):
                lo, hi = t - step, t
                for _ in range(30):
                    mid = 0.5 * (lo + hi)
                    mx, my = ox + mid * dx, oy + mid * dy
                    if mx >= shore and my <= ground(mx):
                        hi = mid
                    else:
                        lo = mid
                if best is None or hi < best:
                    best = hi
                break
            step = 0.05 + 0.002 * t
        if best is None:
            return None
        return (ox + best * dx, oy + best * dy)

    print("  camera   water                                  corrections   sky and scenery confused   error: median, 95th percentile, worst")
    for hc in (2.0, 10.0):
        cam, mir = (0.0, hc), (0.0, -hc)
        a_lo, a_hi, n = math.radians(-2.0), math.radians(80.0), 4000
        image = []
        for i in range(n):
            a = a_lo + (a_hi - a_lo) * (i + 0.5) / n
            image.append(trace(mir[0], mir[1], math.cos(a), math.sin(a)))

        def read(px, py):
            a = math.atan2(py - mir[1], px - mir[0])
            i = int((a - a_lo) / (a_hi - a_lo) * n)
            return image[i] if 0 <= i < n else None

        for name, amp, lam in (("flat", 0.0, 1.0), ("ripples, 2 cm on 1.5 m", 0.02, 1.5),
                               ("waves, 0.1 m on 8 m", 0.1, 8.0), ("waves, 0.3 m on 20 m", 0.3, 20.0)):
            kap = 2 * math.pi / lam
            truth, pts = [], []
            for s in range(400):
                x = 6.0 + (shore - 8.0) * s / 400.0
                y = amp * math.sin(kap * x)
                sl = amp * kap * math.cos(kap * x)
                nn = math.hypot(sl, 1.0)
                nx, ny = -sl / nn, 1.0 / nn
                vx, vy = x - cam[0], y - cam[1]
                vl = math.hypot(vx, vy)
                vx, vy = vx / vl, vy / vl
                dot = vx * nx + vy * ny
                if dot >= 0:
                    continue  # the surface faces away
                rx, ry = vx - 2 * dot * nx, vy - 2 * dot * ny
                if ry <= 0.01:
                    continue  # the ray runs into the water ahead: another wave's business
                pts.append((x, y, rx, ry))
                truth.append(trace(x, y, rx, ry))
            for iters in (0, 1, 2, 3):
                errs, confused = [], 0
                for (x, y, rx, ry), tr in zip(pts, truth):
                    q = read(x, y)
                    for _ in range(iters):
                        t = 1e4 if q is None else max(0.0, (q[0] - x) * rx + (q[1] - y) * ry)
                        q = read(x + rx * t, y + ry * t)
                    if (q is None) != (tr is None):
                        confused += 1
                    elif q is not None:
                        errs.append(math.hypot(q[0] - tr[0], q[1] - tr[1]) / math.hypot(tr[0] - x, tr[1] - y))
                errs.sort()
                med = errs[len(errs) // 2] if errs else 0.0
                p95 = errs[int(len(errs) * 0.95)] if errs else 0.0
                print(f"  {hc:4.0f} m   {name:38s} {iters}             {100 * confused / len(pts):5.1f}% of rays              "
                      f"{100 * med:5.2f}%, {100 * p95:5.2f}%, {100 * (errs[-1] if errs else 0):6.2f}%")
    print("  (no corrections is the plain planar lookup. Rays still confused after two corrections graze the hill's skyline")
    print("   or the tree's top, where a small shift crosses an edge; the worst errors are single rays at such an edge.)")


# ---------------------------------------------------------------- part Q

def part_q():
    print("\nPart Q: under water")
    n = 1.333
    crit = math.degrees(math.asin(1 / n))
    print(f" Snell's window: a ray from below leaves the water only within {crit:.1f} degrees of the surface's normal;")
    print("   beyond it all of the light is reflected back down. Share reflected, seen from beneath, by angle from the normal:")
    row = []
    for ang in (0, 20, 40, 45, 48, 48.5, 50, 70):
        si = math.sin(math.radians(ang))
        st = n * si
        if st >= 1:
            r = 1.0
        else:
            ci, ct = math.cos(math.radians(ang)), math.sqrt(1 - st * st)
            rs = ((n * ci - ct) / (n * ci + ct)) ** 2
            rp = ((n * ct - ci) / (n * ct + ci)) ** 2
            r = 0.5 * (rs + rp)
        row.append(f"{ang:g} deg {r:.3f}")
    print("   " + ", ".join(row))
    print(" Light scattered into a view path. The water is lit from above by light that falls off as exp(-K z) with depth z;")
    print("   a path of length s from depth z0, rising at sine m to the horizontal, loses exp(-c s') along the way. Closed form")
    print("   J0 exp(-K z0) (1 - exp(-(c - K m) s)) / (c - K m) against a 200,000-step integral:")
    print("   c, K per metre   from depth   sine     length   closed form   integral    difference")
    for c, kd, z0, m, s in ((0.15, 0.08, 5.0, 0.0, 30.0), (0.15, 0.08, 5.0, 0.5, 9.0), (0.15, 0.08, 2.0, -0.7, 20.0),
                            (0.6, 0.3, 1.0, 0.9, 1.1), (0.04, 0.03, 20.0, 1.0, 20.0), (0.3, 0.3, 3.0, 1.0, 3.0)):
        a = c - kd * m
        closed = math.exp(-kd * z0) * (s if abs(a) < 1e-12 else (1 - math.exp(-a * s)) / a)
        steps, acc = 200000, 0.0
        for i in range(steps):
            sp = (i + 0.5) * s / steps
            acc += math.exp(-kd * (z0 - m * sp)) * math.exp(-c * sp) * s / steps
        print(f"   {c:4.2f}, {kd:4.2f}       {z0:4.1f} m       {m:+4.1f}     {s:4.1f} m   {closed:9.5f}     {acc:9.5f}   {abs(closed - acc):.1e}")
    print("   (the last row has c = K m, where the closed form's limit is the path's length times the source)")


if __name__ == "__main__":
    part_n()
    part_o()
    part_p()
    part_q()
