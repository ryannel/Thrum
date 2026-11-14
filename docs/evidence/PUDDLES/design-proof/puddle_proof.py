"""PUDDLES design, 14 November 2026: reproductions 1 to 6.
Pure Python, no dependencies. Under a minute to run. Its output is puddle-proof.txt.

Part 1  Hollows. A bed with several hollows is cut into a tree of hollows by a flood from its pits. Rain
        fills them, they spill into each other, merge, and part again as they dry. Checked at every step:
        the water a point test gives (the level of the cell's hollow above the cell's bed) against a flood
        from the pits; each hollow's level across every merge and parting; and the water account.
Part 2  How long a puddle takes to fill and to dry, from the rain rate, its catchment and its losses.
Part 3  Rain rings. The response of water to a drop's impulse, with gravity, surface tension, depth and
        viscosity: the least group speed and the calm disc it leaves, the rings' size from the drop's
        momentum, the share of the drop's energy they carry, what depth does to them, and the slope they
        give a surface at a rain rate.
Part 4  Why wet ground is dark: light returned inside the water film, by albedo and by the share returned.
Part 5  Wet ground through a shower: the surface water and the pores as two stores, under open sky and
        under part shelter.
Part 6  Rings from a footfall in a puddle: a fine interaction level with a ladder of depths from 5 mm, the
        speed of its blended step against the true speed, and what a blend of two ticks costs at 120 Hz.

Run: python3 puddle_proof.py
"""
import heapq
import math

G = 9.81
RHO = 1000.0
TEN = 7.28e-5      # surface tension over density, m3/s2
NU = 1.0e-6        # kinematic viscosity, m2/s


def rnd(seed):
    s = seed & 0xFFFFFFFF
    while True:
        s = (1664525 * s + 1013904223) & 0xFFFFFFFF
        yield s / 4294967296.0


# ---------------------------------------------------------------------------------------------- part 1
def make_bed(n, cell):
    """A yard 3.2 m square: three bowls, a rut, a slope and small relief. Heights in metres."""
    r = rnd(7)
    waves = [(next(r) * 6.28, next(r) * 6.28, 2 + int(next(r) * 5), 2 + int(next(r) * 5)) for _ in range(6)]
    bed = [[0.0] * n for _ in range(n)]
    for j in range(n):
        for i in range(n):
            x, y = (i + 0.5) * cell, (j + 0.5) * cell
            z = 0.060 + 0.004 * x                                   # a fall of 4 mm a metre
            for (cx, cy, rad, dep) in ((0.9, 0.9, 0.55, 0.045), (1.75, 1.05, 0.40, 0.030),
                                       (2.3, 2.2, 0.60, 0.050), (1.0, 2.3, 0.35, 0.018)):
                d2 = ((x - cx) ** 2 + (y - cy) ** 2) / (rad * rad)
                z -= dep * math.exp(-d2)
            z -= 0.012 * math.exp(-((y - 1.6) / 0.12) ** 2) * (1 if 0.4 < x < 2.9 else 0)   # a rut
            for (pa, pb, ka, kb) in waves:
                z += 0.0012 * math.sin(pa + ka * 6.28 * x / 3.2) * math.sin(pb + kb * 6.28 * y / 3.2)
            bed[j][i] = z
    return bed


class Tree:
    """The tree of hollows of a bed. Node 0 is outside (the rectangle's edge). Leaves are pits."""

    def __init__(self, bed, cell):
        n = len(bed)
        self.n, self.cell, self.area = n, cell, cell * cell
        z = [bed[j][i] for j in range(n) for i in range(n)]
        self.z = z
        nb = lambda c: [c + d for d, ok in ((-1, c % n > 0), (1, c % n < n - 1), (-n, c >= n), (n, c < n * n - n)) if ok]
        self.nb = nb
        key = lambda c: (z[c], c)
        label = [-1] * (n * n)
        heap = []
        edge = [c for c in range(n * n) if c % n in (0, n - 1) or c // n in (0, n - 1)]
        for c in edge:
            label[c] = 0
            heapq.heappush(heap, (key(c), c))
        pits = [c for c in range(n * n) if label[c] < 0 and all(key(c) < key(m) for m in nb(c))]
        self.pit = {0: None}
        for k, c in enumerate(pits):
            label[c] = k + 1
            self.pit[k + 1] = c
            heapq.heappush(heap, (key(c), c))
        outlets = {}
        while heap:
            _, c = heapq.heappop(heap)
            for m in nb(c):
                if label[m] < 0:
                    label[m] = label[c]
                    heapq.heappush(heap, (key(m), m))
                elif label[m] != label[c]:
                    pair = (min(label[c], label[m]), max(label[c], label[m]))
                    e = max(z[c], z[m])
                    if pair not in outlets or e < outlets[pair][0]:
                        outlets[pair] = (e, c if label[c] == pair[0] else m, m if label[c] == pair[0] else c)
        self.label = label
        leaves = len(pits)
        self.parent, self.kids, self.sill, self.over = {}, {}, {}, {}
        root = list(range(leaves + 1))

        def find(a):
            while root[a] != a:
                root[a] = root[root[a]]
                a = root[a]
            return a
        top = {k: k for k in range(leaves + 1)}          # union-find root -> its tree node
        nxt = leaves + 1
        for (a, b), (e, ca, cb) in sorted(outlets.items(), key=lambda it: (it[1][0], it[0])):
            ra, rb = find(a), find(b)
            if ra == rb:
                continue
            na, nb_ = top[ra], top[rb]
            if na == 0 or nb_ == 0:                      # spills outside
                node = nb_ if na == 0 else na
                self.sill[node], self.parent[node], self.over[node] = e, 0, 0
                root[ra] = root[rb] = min(ra, rb)
                top[min(ra, rb)] = 0
                continue
            self.kids[nxt] = (na, nb_)
            self.parent[na] = self.parent[nb_] = nxt
            self.sill[na] = self.sill[nb_] = e
            self.over[na], self.over[nb_] = b, a         # the leaf across the sill
            root[ra] = root[rb] = min(ra, rb)
            top[min(ra, rb)] = nxt
            nxt += 1
        self.nodes = [k for k in range(1, nxt)]
        self.leaves = [k for k in range(1, leaves + 1)]
        self.cells = {k: [] for k in self.nodes}
        for c, l in enumerate(label):
            k = l
            while k and k in self.cells:
                self.cells[k].append(c)
                k = self.parent.get(k, 0)
        self.sorted, self.prefix = {}, {}
        for k in self.nodes:
            s = sorted(z[c] for c in self.cells[k])
            p = [0.0]
            for v in s:
                p.append(p[-1] + v)
            self.sorted[k], self.prefix[k] = s, p
        self.cap = {k: self.volume(k, self.sill[k]) for k in self.nodes}
        self.catch = {k: sum(1 for c in range(n * n) if label[c] == k) * self.area for k in self.leaves}

    def count_below(self, k, level):
        s, lo, hi = self.sorted[k], 0, len(self.sorted[k])
        while lo < hi:
            mid = (lo + hi) // 2
            if s[mid] < level:
                lo = mid + 1
            else:
                hi = mid
        return lo

    def volume(self, k, level):
        m = self.count_below(k, level)
        return (level * m - self.prefix[k][m]) * self.area

    def level(self, k, vol):
        s, p = self.sorted[k], self.prefix[k]
        if vol <= 0.0:
            return s[0]
        lo, hi = 1, len(s)
        while lo < hi:                                   # least m whose first m cells hold vol below s[m]
            mid = (lo + hi) // 2
            if (s[mid] * mid - p[mid]) * self.area >= vol:
                hi = mid
            else:
                lo = mid + 1
        return (vol / self.area + p[lo]) / lo

    def surface(self, k, level):
        return self.count_below(k, level) * self.area


class Fill:
    """Water in the tree: a set of filled nodes, none an ancestor of another, each with a volume."""

    def __init__(self, tree):
        self.t = tree
        self.vol = {k: 0.0 for k in tree.leaves}
        self.sunk = 0.0
        self.merges = self.partings = 0

    def holder(self, leaf):
        k = leaf
        while k not in self.vol:
            k = self.t.parent[k]
        return k

    def settle(self):
        t = self.t
        changed = True
        while changed:
            changed = False
            for k in sorted(self.vol):
                if k not in self.vol:
                    continue
                if k in t.kids and self.vol[k] < t.cap[t.kids[k][0]] + t.cap[t.kids[k][1]]:
                    a, b = t.kids[k]                      # the level has fallen to the sill inside: part
                    short = t.cap[a] + t.cap[b] - self.vol[k]
                    sa, sb = t.surface(a, t.sill[a]), t.surface(b, t.sill[b])
                    del self.vol[k]
                    self.vol[a] = t.cap[a] - short * sa / (sa + sb)
                    self.vol[b] = t.cap[b] - short * sb / (sa + sb)
                    self.partings += 1
                    changed = True
                    continue
                over = self.vol[k] - t.cap[k]
                if over <= 0.0:
                    continue
                if t.parent[k] == 0:                      # over the rectangle's edge
                    self.vol[k] = t.cap[k]
                    self.sunk += over
                    changed = True
                    continue
                p = t.parent[k]
                a, b = t.kids[p]
                sib = b if k == a else a
                if sib in self.vol and self.vol[sib] >= t.cap[sib]:
                    self.vol[p] = self.vol.pop(a) + self.vol.pop(b)    # both full: they are one puddle
                    self.merges += 1
                else:
                    self.vol[k] = t.cap[k]
                    self.vol[self.holder(t.over[k])] += over
                changed = True

    def step(self, rain, loss, dt):
        """rain and loss in metres a second. Returns (rain in, water lost)."""
        t = self.t
        got = 0.0
        for leaf in t.leaves:
            v = rain * t.catch[leaf] * dt
            self.vol[self.holder(leaf)] += v
            got += v
        gone = 0.0
        for k in self.vol:
            lvl = t.level(k, self.vol[k])
            take = min(self.vol[k], loss * t.surface(k, lvl) * dt)
            self.vol[k] -= take
            gone += take
        self.settle()
        return got, gone

    def levels(self):
        held = {k: self.t.level(k, v) for k, v in self.vol.items()}
        return {leaf: held[self.holder(leaf)] for leaf in self.t.leaves}


def flood_wet(tree, fill):
    """Wet cells by a flood from each filled node's pits, never crossing ground above its level."""
    wet = set()
    for k, v in fill.vol.items():
        if v <= 0.0:
            continue
        lvl = tree.level(k, v)
        stack = [tree.pit[l] for l in tree.leaves if fill.holder(l) == k]
        own = set(tree.cells[k])
        seen = set(c for c in stack if tree.z[c] < lvl)
        stack = list(seen)
        while stack:
            c = stack.pop()
            for m in tree.nb(c):
                if m not in seen and m in own and tree.z[m] < lvl:
                    seen.add(m)
                    stack.append(m)
        wet |= seen
    return wet


def run_yard(tree, dt, report):
    n = tree.n
    fill = Fill(tree)
    rain, loss = 30e-3 / 3600, 0.0
    total_in = total_gone = 0.0
    worst_mismatch = 0
    prev = fill.levels()
    worst_step = worst_event_step = 0.0
    events_seen = 0
    log = []
    steps = int(5200 / dt)
    for step in range(1, steps + 1):
        sec = step * dt
        if sec >= 2600:
            rain, loss = 0.0, 40e-3 / 3600            # the rain stops; the yard dries and drains
        m0, p0 = fill.merges, fill.partings
        got, gone = fill.step(rain, loss, dt)
        total_in += got
        total_gone += gone
        now = fill.levels()
        jump = max(abs(now[l] - prev[l]) for l in tree.leaves)
        event = fill.merges != m0 or fill.partings != p0
        if event:
            events_seen += 1
            worst_event_step = max(worst_event_step, jump)
        else:
            worst_step = max(worst_step, jump)
        prev = now
        if report and (step % 40 == 0 or event):
            point = set(c for c in range(n * n) if tree.label[c] and now[tree.label[c]] > tree.z[c]
                        and fill.vol[fill.holder(tree.label[c])] > 0.0)
            worst_mismatch = max(worst_mismatch, len(point ^ flood_wet(tree, fill)))
        if report and sec in (600, 1300, 2599, 3300, 4200, 5200):
            puddles = sum(1 for v in fill.vol.values() if v > 1e-7)
            wet = sum(tree.surface(k, tree.level(k, v)) for k, v in fill.vol.items() if v > 0)
            log.append((int(sec), puddles, wet, sum(fill.vol.values()) * 1000))
    held = sum(fill.vol.values())
    return fill, log, worst_mismatch, worst_step, worst_event_step, events_seen, total_in, total_gone, held


def part1():
    print("PART 1  Hollows that fill, spill, merge and part")
    n, cell = 64, 0.05
    tree = Tree(make_bed(n, cell), cell)
    print(f"  bed {n} x {n} cells of {cell * 100:.0f} cm; {len(tree.leaves)} hollows, {len(tree.nodes)} nodes in the tree")
    deep = sorted(((tree.sill[k] - tree.sorted[k][0]) * 1000, tree.cap[k] * 1000, k) for k in tree.leaves)[-5:]
    print("  the five deepest hollows (depth to sill, litres to sill):",
          ", ".join(f"{d:.0f} mm {c:.1f} l" for d, c, _ in reversed(deep)))
    print("  rain of 30 mm/h for 2,600 s, then none and a loss of 40 mm/h from every water surface")
    fill, log, mismatch, step_, event_step, events, tin, gone, held = run_yard(tree, 1.0, True)
    for sec, puddles, wet, litres in log:
        print(f"  t={sec:5d} s  puddles {puddles:3d}  water surface {wet:5.2f} m2  held {litres:6.1f} l")
    print(f"  merges {fill.merges}, partings {fill.partings}, steps with either {events}")
    print(f"  point test against a flood from the pits, worst count of cells that differ: {mismatch}")
    print(f"  water account: in {tin * 1000:.3f} l = held {held * 1000:.3f} + lost {gone * 1000:.3f} "
          f"+ over the edge {fill.sunk * 1000:.3f}; residual {(tin - held - gone - fill.sunk) * 1000:.2e} l")
    for dt in (1.0, 0.25, 1 / 16):
        f2, _, _, s2, e2, _, _, _, _ = run_yard(tree, dt, False)
        print(f"  steps of {dt:6.4f} s: largest change of any hollow's level in one step {s2 * 1000:.4f} mm with no merge "
              f"or parting, {e2 * 1000:.4f} mm in a step with one ({f2.merges} merges, {f2.partings} partings)")
    print(f"  as basins with a seed each, {len(tree.kids)} pairs of fills would overlap at some level and be refused")
    return tree


# ---------------------------------------------------------------------------------------------- part 2
def part2():
    print("PART 2  How long a puddle takes")
    print("  a pothole 0.5 m2 at its sill, 20 mm deep at its deepest, 4.0 l to its sill")
    for ground, k_inf in (("paving, no soak", 0.0), ("packed earth, soaks 5 mm/h", 5.0), ("sand, soaks 60 mm/h", 60.0)):
        row = []
        for rate in (2.0, 10.0, 50.0):
            for ratio in (1.0, 6.0):
                run = max(0.0, rate - k_inf) * (ratio - 1.0) * 0.5 + max(0.0, rate - k_inf) * 0.5   # l/h
                t = 4.0 / run * 60 if run > 0 else float("inf")
                row.append(f"{t:6.0f}" if t < 1e5 else "  none")
        print(f"  {ground:28s} minutes to fill at 2, 10, 50 mm/h (catchment 1x, 6x its area): " + " ".join(row))
    for name, loss in (("evaporation 0.3 mm/h", 0.3), ("evaporation and soak 5 mm/h", 5.3), ("a demo's drying rate 600 mm/h", 600.0)):
        print(f"  20 mm of water gone in {20.0 / loss * 60:8.1f} minutes at {name}")


# ---------------------------------------------------------------------------------------------- part 3
def j0(x):
    ax = abs(x)
    if ax < 8.0:
        y = x * x
        a = 57568490574.0 + y * (-13362590354.0 + y * (651619640.7 + y * (-11214424.18 + y * (77392.33017 + y * -184.9052456))))
        b = 57568490411.0 + y * (1029532985.0 + y * (9494680.718 + y * (59272.64853 + y * (267.8532712 + y))))
        return a / b
    zz = 8.0 / ax
    y = zz * zz
    xx = ax - 0.785398164
    a = 1.0 + y * (-0.1098628627e-2 + y * (0.2734510407e-4 + y * (-0.2073370639e-5 + y * 0.2093887211e-6)))
    b = -0.1562499995e-1 + y * (0.1430488765e-3 + y * (-0.6911147651e-5 + y * (0.7621095161e-6 - y * 0.934935152e-7)))
    return math.sqrt(0.636619772 / ax) * (math.cos(xx) * a - zz * math.sin(xx) * b)


def omega(k, d):
    return math.sqrt((G * k + TEN * k ** 3) * math.tanh(k * d))


def group(k, d):
    h = k * 1e-4
    return (omega(k + h, d) - omega(k - h, d)) / (2 * h)


def damp(k, d):
    w = omega(k, d)
    x = 2 * k * d
    bottom = k * math.sqrt(NU * w / 2) / math.sinh(x) if x < 50 else 0.0
    return 2 * NU * k * k + bottom


def ring(r, t, d, a, impulse, ks):
    """Height at radius r, time t after an impulse spread as a Gaussian of radius a."""
    s = 0.0
    dk = ks[1] - ks[0]
    for k in ks:
        w = omega(k, d)
        s += impulse * math.exp(-0.5 * k * k * a * a) * (k * math.tanh(k * d) / w) * math.sin(w * t) \
            * math.exp(-damp(k, d) * t) * j0(k * r) * k
    return -s * dk / (2 * math.pi * RHO)


def drop(diam_mm):
    v = 9.65 - 10.3 * math.exp(-0.6 * diam_mm)         # Atlas et al. 1973, from memory
    m = RHO * math.pi / 6 * (diam_mm * 1e-3) ** 3
    return m, v


def part3():
    print("PART 3  Rain rings")
    for d in (1.0, 0.02, 0.01, 0.005, 0.002):
        best = min(((group(k, d), k) for k in [20 + 2 * i for i in range(1500)]), key=lambda p: p[0])
        cmin = min((omega(k, d) / k, k) for k in [20 + 2 * i for i in range(1500)])
        print(f"  depth {d * 1000:6.0f} mm: least group speed {best[0] * 100:5.1f} cm/s at a wavelength of "
              f"{2 * math.pi / best[1] * 100:4.1f} cm; least phase speed {cmin[0] * 100:5.1f} cm/s at {2 * math.pi / cmin[1] * 100:4.1f} cm")
    ks = [1.0 + 1.5 * i for i in range(1400)]
    m, v = drop(3.0)
    imp = m * v
    a = 3 * 1.5e-3                                       # the crater: three drop radii
    print(f"  a 3 mm drop: {m * 1e6:.1f} mg at {v:.2f} m/s, momentum {imp * 1e6:.1f} uN s, energy {0.5 * m * v * v * 1e6:.0f} uJ")
    for t in (0.2, 0.4, 0.8):
        prof = [(0.004 * i, ring(0.004 * i, t, 1.0, a, imp, ks)) for i in range(1, 90)]
        peak = max(abs(h) for _, h in prof)
        calm = next(r for r, h in prof if abs(h) > 0.1 * peak)
        outer = max(r for r, h in prof if abs(h) > 0.1 * peak)
        crests = sum(1 for i in range(1, len(prof) - 1) if prof[i][1] > prof[i - 1][1] and prof[i][1] > prof[i + 1][1]
                     and prof[i][1] > 0.1 * peak)
        where = max(prof, key=lambda p: abs(p[1]))[0]
        print(f"  deep water, {t:.1f} s after: calm inside {calm * 100:4.1f} cm ({calm / t * 100:4.1f} cm/s), rings out to "
              f"{outer * 100:4.1f} cm, {crests} crests, highest {peak * 1000:.3f} mm at {where * 100:.1f} cm")
    # energy the rings carry, against the drop's
    for mult in (1.0, 2.0, 3.0, 4.0):
        aa = mult * 1.5e-3
        e = sum((imp * math.exp(-0.5 * k * k * aa * aa)) ** 2 * k * k for k in ks) * 1.5 / (4 * math.pi * RHO)
        print(f"  footprint {mult:.0f} drop radii: the rings carry {e / (0.5 * m * v * v) * 100:5.1f}% of the drop's energy")
    # the same wave train for another drop?
    for diam in (1.5, 3.0, 5.0):
        mm, vv = drop(diam)
        aa = 3 * diam * 0.5e-3
        prof = [ring(0.004 * i, 0.4, 1.0, aa, mm * vv, ks) for i in range(1, 90)]
        peak = max(abs(h) for h in prof)
        calm = next(0.004 * (i + 1) for i, h in enumerate(prof) if abs(h) > 0.1 * peak)
        print(f"  a {diam:.1f} mm drop at 0.4 s: highest ring {peak * 1000:.3f} mm, {peak / (mm * vv) :.1f} m per N s, calm inside {calm * 100:.1f} cm")
    # depth
    for d in (0.05, 0.02, 0.01, 0.005, 0.002, 0.001):
        prof = [ring(0.004 * i, 0.4, d, a, imp, ks) for i in range(1, 90)]
        print(f"  depth {d * 1000:4.0f} mm, 0.4 s after: highest ring {max(abs(h) for h in prof) * 1000:.3f} mm "
              f"({max(abs(h) for h in prof) / (0.6 * d) * 100:5.1f}% of 0.6 of the depth)")
    # slope a rain rate gives: independent rings add their squared slopes
    def one_drop_slope(diam, d):
        mm, vv = drop(diam)
        aa = 3 * diam * 0.5e-3
        s = 0.0
        for k in ks:
            w = omega(k, d)
            amp = mm * vv * math.exp(-0.5 * k * k * aa * aa) * (k * math.tanh(k * d) / w) / RHO
            s += k ** 3 * amp * amp / (4 * damp(k, d))
        return s * 1.5 / (2 * math.pi)
    for rate in (2.0, 10.0, 50.0):
        lam = 4.1 * rate ** -0.21                         # Marshall and Palmer 1948, from memory
        flux = mss = big = water = 0.0
        for i in range(1, 120):
            diam = 0.05 * i
            mm, vv = drop(diam)
            if vv <= 0:
                continue
            nd = 8000.0 * math.exp(-lam * diam) * 0.05 * vv   # drops a m2 a second in this size class
            flux += nd
            water += nd * mm / RHO
            mss += nd * one_drop_slope(diam, 0.05)
            if diam >= 2.0:
                big += nd
        print(f"  rain {rate:4.0f} mm/h: {flux:6.0f} drops a m2 a second ({big:4.0f} of 2 mm and over), "
              f"{water * 3.6e6:5.1f} mm/h of water, mean square slope {mss:.4f} (rms {math.degrees(math.sqrt(mss)):.1f} deg)")
    for cap in (16, 64, 256):
        print(f"  {cap:3d} rings a m2 a second, each drawn for 1.0 s out to 0.20 m: {cap * math.pi * 0.2 ** 2 * 0.5:5.1f} "
              f"rings over a point on average (a ring's disc grows from nothing, so half its last area)")


# ---------------------------------------------------------------------------------------------- part 4
def part4():
    print("PART 4  Why wet ground is dark")
    r_l = 0.02
    print("  wet over dry albedo, by dry albedo and the share of light the film returns to the ground:")
    for p in (0.47, 0.60, 0.70, 0.80):
        row = " ".join(f"{(1 - r_l) * (1 - p) / (1 - p * a):.2f}" for a in (0.05, 0.1, 0.2, 0.4, 0.6, 0.8))
        print(f"  share {p:.2f}: albedo 0.05 0.1 0.2 0.4 0.6 0.8 -> {row}")
    a = (0.30, 0.22, 0.14)
    for p in (0.47, 0.70):
        w = [(1 - r_l) * (1 - p) * c / (1 - p * c) for c in a]
        sat = lambda c: (max(c) - min(c)) / max(c)
        print(f"  a brown of {a} at share {p}: {tuple(round(c, 3) for c in w)}; saturation {sat(a):.2f} -> {sat(w):.2f}")


# ---------------------------------------------------------------------------------------------- part 5
def wet_run(reach, pore_cap, hold, soak, evap, shower, rate=10.0, total=3600):
    """Surface water (mm) and the pores' fill (0..1) each second, for ground a share of the rain reaches."""
    film = pores = 0.0
    out = []
    for sec in range(total):
        rain = rate if sec < shower else 0.0
        film += reach * rain / 3600
        take = min(film, soak / 3600, (1 - pores) * pore_cap)
        film -= take
        pores += take / pore_cap
        film = min(film, hold)
        if rain == 0.0:
            film = max(0.0, film - evap / 3600)
        if film <= 0.0 and rain == 0.0:
            pores -= pores * evap / pore_cap / 3600
        out.append((film, pores))
    return out


def part5():
    print("PART 5  Wet ground through a shower of 10 mm/h for ten minutes, then drying at 6 mm/h (nothing dries while it rains)")
    grounds = (("paving: pores 0.3 mm, relief holds 1.0 mm", 0.3, 1.0, 30.0),
               ("packed earth: pores 3 mm, relief holds 0.5 mm", 3.0, 0.5, 15.0))
    for name, cap, hold, soak in grounds:
        print(f"  {name}")
        runs = {r: wet_run(r, cap, hold, soak, 6.0, 600) for r in (1.0, 0.5, 0.25, 0.4)}
        for r in (1.0, 0.5, 0.25):
            run = runs[r]
            first_film = next((i for i, (f, _) in enumerate(run) if f > 0.05), None)
            film_gone = next((i for i in range(600, 3600) if run[i][0] <= 0.0), None)
            half_dry = next((i for i in range(600, 3600) if run[i][1] <= 0.5 * run[599][1]), None)
            print(f"    reach {r:4.2f}: sheen from {first_film} s; at 600 s film {run[599][0]:.2f} mm, pores {run[599][1]:.2f}; "
                  f"film gone at {film_gone} s; pores half dry at {half_dry} s")
        worst_f = worst_p = 0.0
        for i in range(3600):
            mix_f = runs[0.25][i][0] + (0.4 - 0.25) / 0.25 * (runs[0.5][i][0] - runs[0.25][i][0])
            mix_p = runs[0.25][i][1] + (0.4 - 0.25) / 0.25 * (runs[0.5][i][1] - runs[0.25][i][1])
            worst_f = max(worst_f, abs(mix_f - runs[0.4][i][0]))
            worst_p = max(worst_p, abs(mix_p - runs[0.4][i][1]))
        scale_p = max(abs(0.4 * runs[1.0][i][1] - runs[0.4][i][1]) for i in range(3600))
        print(f"    reach 0.40 read between the stores for 0.25 and 0.5: off by {worst_f:.3f} mm of film and {worst_p:.3f} "
              f"of the pores at worst; as 0.4 of the open store it is off by {scale_p:.2f} of the pores")


# ---------------------------------------------------------------------------------------------- part 6
def part6():
    print("PART 6  Rings from a footfall: a fine interaction level")
    dt = 1 / 60
    ladder = [0.005, 0.01, 0.02, 0.04, 0.08, 0.16, None]     # None is deep water

    def rot(k, d, steps):
        dd = k if d is None else k * math.tanh(k * d)
        gg = G + TEN * k * k
        w = math.sqrt(gg * dd)
        c, s = math.cos(w * dt * steps), math.sin(w * dt * steps)
        return ((c, dd / w * s), (-gg / w * s, c))
    for steps in (1, 2, 8):
        worst_c = worst_loss = 0.0
        where = None
        stalled = 0
        for lam in [0.08 + 0.02 * i for i in range(47)]:
            k = 2 * math.pi / lam
            for i in range(0, 400):
                d = 0.005 * (1.0 / 0.005) ** (i / 399)   # 5 mm to 1 m; the deep operator serves from 0.32 m
                lo = max(j for j in range(len(ladder) - 1) if ladder[j] <= min(d, 0.3199))
                d1, d2 = ladder[lo], ladder[lo + 1]
                top = 0.32 if d2 is None else d2
                w2 = min(1.0, (math.sqrt(d) - math.sqrt(d1)) / (math.sqrt(top) - math.sqrt(d1)))
                a, b = rot(k, d1, steps), rot(k, d2, steps)
                m = [[(1 - w2) * a[r][c] + w2 * b[r][c] for c in range(2)] for r in range(2)]
                tr, det = m[0][0] + m[1][1], m[0][0] * m[1][1] - m[0][1] * m[1][0]
                disc = tr * tr / 4 - det
                if disc >= 0:
                    stalled += 1                         # the blended step no longer turns: the wave does not travel
                    continue
                mod, arg = math.sqrt(det), math.atan2(math.sqrt(-disc), tr / 2)
                true = omega(k, d)
                turn = true * dt * steps                 # the angle is known only to a whole turn: take the nearest
                arg = min((sign * arg + 2 * math.pi * n for sign in (1, -1) for n in range(0, 4)), key=lambda x: abs(x - turn))
                err = abs(arg / (dt * steps) - true) / true
                loss = 1 - mod ** (1 / (dt * steps))
                if err > worst_c:
                    worst_c, where = err, (lam, d)
                worst_loss = max(worst_loss, loss)
        print(f"  steps of {steps} tick(s): phase speed within {worst_c * 100:.1f}% (worst at a {where[0] * 100:.0f} cm wave on "
              f"{where[1] * 1000:.0f} mm of water); never grows; loses at most {worst_loss * 100:.1f}% a second; "
              f"{stalled} cases where the wave does not travel")
    for lam, d in ((0.08, 0.03), (0.2, 0.03), (0.5, 0.05)):
        w = omega(2 * math.pi / lam, d)
        print(f"  a {lam * 100:.0f} cm wave on {d * 1000:.0f} mm: {w / (2 * math.pi / lam) * 100:.0f} cm/s, "
              f"a blend of two ticks misstates it by {(w / 60) ** 2 / 8 * 100:.1f}% of its height at most")
    print("  a 256 x 256 level of 2 cm cells is a window 5.12 m across")
    ks = [0.25 + 0.25 * i for i in range(480)]
    imp = RHO * 0.02 * 0.03 * 1.0                        # the water under a foot (0.02 m2, 30 mm deep) at 1 m/s
    for t, r in ((0.5, 0.25), (1.0, 0.5), (2.0, 1.0)):
        h = max(abs(ring(r + 0.01 * i, t, 0.03, 0.07, imp, ks)) for i in range(-8, 9))
        print(f"  a footfall of {imp:.1f} N s over 7 cm on 30 mm of water: rings {h * 1000:.1f} mm high near {r:.2f} m after {t:.1f} s")


if __name__ == "__main__":
    part1()
    part2()
    part3()
    part4()
    part5()
    part6()
