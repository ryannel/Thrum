"""SEA design proof, second set: the mechanisms changed after the design review of 13 November 2026.

Run: uv run --with numpy python docs/evidence/SEA/design-proof/revision_proof.py

A  A hitch: the wake advanced n ticks in one step, against n one-tick steps.
B  The display blend across a span of several ticks, in the tick that follows a hitch.
C  The receiving region of a body's reflection and refraction, against rays reflected and refracted on the CPU.
D  The foam transfer chain: recipients rise with no birth, the inventory falls, one long step equals many short.
E  Gathered scene colour turned back into radiance: exposure and the camera path's air, each applied once.
"""
import math
import sys

import numpy as np

from wake_proof import DT, G, HULL, Level


# --- A ---------------------------------------------------------------------------------------------------------

def step_n(lv, sources, n):
    """One step of n ticks. `sources` are the source transforms at the n tick positions of the step: their mean is
    the pressure impulse along the body's path. Half is applied before the rotation and half after; the border
    damps by the per-tick factor to the power n."""
    hh, ph = np.fft.fft2(lv.h), np.fft.fft2(lv.p)
    kick = 0.0
    if sources:
        kick = -G * (sum(sources) / len(sources)) * lv.band * (0.5 * n * DT)
    c, s = np.cos(lv.w * n * DT), np.sin(lv.w * n * DT)
    ph = ph + kick
    hh, ph = c * hh + lv.k_over_w * s * ph, c * ph - lv.g_over_w * s * hh
    ph = ph + kick
    lv.h = np.fft.ifft2(hh).real * lv.mask ** n
    lv.p = np.fft.ifft2(ph).real * lv.mask ** n


def wake_run(seconds, speed, group, swept=True, n=256, dx=0.5):
    """Hull at `speed` for `seconds`. `group(i)` gives how many ticks the step that starts at tick i covers."""
    lv = Level(n, dx)
    body = np.array([0.0, 0.0])
    ticks = int(round(seconds / DT))
    i = 0
    while i < ticks:
        m = min(group(i), ticks - i)
        path = [body + np.array([speed * DT * (j + 1), 0.0]) for j in range(m)]
        body = path[-1]
        lv.follow(body)
        srcs = [lv.source_hat(p, 0.0, **HULL) for p in (path if swept else path[-1:])]
        step_n(lv, srcs, m)
        i += m
    return lv, body


def part_a(out):
    out("A  hull at 6 m/s for 20 s on one 256 x 0.5 m level; error inside the window, clear of its border,")
    out("   against the same run stepped one tick at a time")
    ref, body = wake_run(20.0, 6.0, lambda i: 1)
    gx, gz = np.meshgrid(body[0] + np.arange(-64.0, 16.0, 0.5), body[1] + np.arange(-40.0, 40.0, 0.5), indexing="ij")
    one = ref.sample(gx, gz)
    rms = float(np.sqrt(np.mean(one ** 2)))

    def err(lv):
        e = lv.sample(gx, gz) - one
        return 100.0 * float(np.sqrt(np.mean(e ** 2))) / rms, 1000.0 * float(np.abs(e).max())

    free = Level(256, 0.5)
    free.h, free.p = ref.h.copy(), ref.p.copy()
    many = Level(256, 0.5)
    many.h, many.p = ref.h.copy(), ref.p.copy()
    step_n(free, None, 8)
    for _ in range(8):
        step_n(many, None, 1)
    d = free.h - many.h
    out(f"   free propagation, one 8-tick step against eight: max difference {float(np.abs(d).max()) * 1000:.2e} mm"
        f" away from the border {float(np.abs(d[40:-40, 40:-40]).max()) * 1000:.2e} mm")
    for n in (2, 4, 8, 16):
        last = int(round(20.0 / DT)) - n
        a = err(wake_run(20.0, 6.0, lambda i, n=n, last=last: n if i == last else 1)[0])
        b = err(wake_run(20.0, 6.0, lambda i, n=n: n if i % 60 == 0 else 1)[0])
        c = err(wake_run(20.0, 6.0, lambda i, n=n: n if i % 60 == 0 else 1, swept=False)[0])
        d = err(wake_run(20.0, 6.0, lambda i, n=n: n)[0])
        out(f"   n={n:2d}: one hitch at the end {a[0]:5.2f}% of RMS (max {a[1]:6.2f} mm) | a hitch every second"
            f" {b[0]:5.2f}% | the same with the source held at its last place {c[0]:5.2f}% | every step n ticks {d[0]:5.2f}%")
    out(f"   reference RMS {rms * 1000:.1f} mm")


# --- B ---------------------------------------------------------------------------------------------------------

def part_b(out):
    out("B  largest error of the time-weighted blend of two endpoints s ticks apart, during the last tick of the")
    out("   span, as a share of a wave's amplitude; the bound is (w dt)^2 / 8 for s = 1 and (w dt)^2 (s - 1) / 2 after")
    for lam in (0.4, 2.0, 20.0, 150.0):
        w = math.sqrt(G * 2.0 * math.pi / lam)
        row = []
        for s in (1, 2, 4, 8):
            span = s * DT
            worst = 0.0
            for phase in np.linspace(0.0, 2.0 * math.pi, 181):
                t = np.linspace(span - DT, span, 201)
                true = np.cos(w * t + phase)
                blend = (1.0 - t / span) * math.cos(phase) + (t / span) * math.cos(w * span + phase)
                worst = max(worst, float(np.abs(blend - true).max()))
            bound = (w * DT) ** 2 / 8.0 if s == 1 else (w * DT) ** 2 * (s - 1) / 2.0
            row.append(f"s={s}: {100 * worst:6.3f}% (bound {100 * bound:6.3f}%)")
        out(f"   wavelength {lam:6.1f} m  " + "  ".join(row))


# --- C ---------------------------------------------------------------------------------------------------------

def hit_box(origin, direction, lo, hi, reach):
    """Slab test: does origin + t direction, 0 < t <= reach, meet the box [lo, hi]?"""
    with np.errstate(divide="ignore", invalid="ignore"):
        inv = 1.0 / direction
        t0, t1 = (lo - origin) * inv, (hi - origin) * inv
    near = np.nanmax(np.minimum(t0, t1), axis=1)
    far = np.nanmin(np.maximum(t0, t1), axis=1)
    return (far >= np.maximum(near, 1e-6)) & (near <= reach) & (far > 0.0)


def screen_rect(lo, hi, cam):
    """Screen bounds (ordinates over focal length) of a world box, for a level camera at `cam` looking along +x.
    A box that reaches the camera's plane is given the whole screen."""
    if lo[0] - cam[0] <= 0.05:
        return (-math.inf, math.inf, -math.inf, math.inf)
    c = np.array([[x, y, z] for x in (lo[0], hi[0]) for y in (lo[1], hi[1]) for z in (lo[2], hi[2])]) - cam
    u, v = c[:, 2] / c[:, 0], c[:, 1] / c[:, 0]
    return (float(u.min()), float(u.max()), float(v.min()), float(v.max()))


def receiving_boxes(lo, hi, amp, tilt, reach):
    """The design's two receiving volumes for a body's box [lo, hi], mean sea level at y = 0."""
    s = math.sin(tilt)
    top = hi[1]
    down = top + 2.0 * amp + 2.0 * reach * s
    side = 2.0 * (top + amp + reach * s) * s
    reflect = (np.array([lo[0] - side, -down, lo[2] - side]), np.array([hi[0] + side, hi[1], hi[2] + side]))
    grow = max(amp - lo[1], 0.0) * math.tan(math.asin(1.0 / 1.33) + tilt)
    refract = (np.array([lo[0] - grow, -amp, lo[2] - grow]), np.array([hi[0] + grow, amp, hi[2] + grow]))
    return reflect, refract


def part_c(out):
    out("C  sea points with random heights within the amplitude and random normals within the slope allowance;")
    out("   each whose reflected or refracted ray meets the body's box must lie inside the receiving rectangle")
    rng = np.random.default_rng(7)
    reach = 64.0
    total_missed = 0
    cases = []
    for cam_h in (1.0, 2.0, 10.0, 40.0):
        for dist in (12.0, 20.0, 100.0):
            for amp, tilt_deg in ((0.0, 0.0), (0.5, 10.0), (3.0, 20.0)):
                cases.append((cam_h, dist, amp, tilt_deg))
    for cam_h, dist, amp, tilt_deg in cases:
        tilt = math.radians(tilt_deg)
        cam = np.array([0.0, cam_h, 0.0])
        lo, hi = np.array([dist - 3.0, -0.8, 4.0 - 1.0]), np.array([dist + 3.0, 2.5, 4.0 + 1.0])
        (rlo, rhi), (flo, fhi) = receiving_boxes(lo, hi, amp, tilt, reach)
        r_rect, f_rect = screen_rect(rlo, rhi, cam), screen_rect(flo, fhi, cam)
        n = 1_500_000
        x = np.stack([rng.uniform(0.3, dist + 70.0, n), rng.uniform(-amp, amp, n) if amp else np.zeros(n),
                      rng.uniform(-70.0, 78.0, n)], axis=1)
        x = x[x[:, 1] < cam_h - 0.05]
        m = len(x)
        cos_t = rng.uniform(math.cos(tilt), 1.0, m)
        sin_t, phi = np.sqrt(1.0 - cos_t ** 2), rng.uniform(0.0, 2.0 * math.pi, m)
        nrm = np.stack([sin_t * np.cos(phi), cos_t, sin_t * np.sin(phi)], axis=1)
        view = x - cam
        view /= np.linalg.norm(view, axis=1, keepdims=True)
        cosi = -np.sum(view * nrm, axis=1)
        front = cosi > 0.0
        refl = view + 2.0 * cosi[:, None] * nrm
        eta = 1.0 / 1.33
        k = 1.0 - eta * eta * (1.0 - cosi ** 2)
        refr = eta * view + (eta * cosi - np.sqrt(np.maximum(k, 0.0)))[:, None] * nrm
        u, v = x[:, 2] / x[:, 0], (x[:, 1] - cam_h) / x[:, 0]

        def inside(rect):
            return (u >= rect[0] - 1e-9) & (u <= rect[1] + 1e-9) & (v >= rect[2] - 1e-9) & (v <= rect[3] + 1e-9)

        got_r = front & hit_box(x, refl, lo, hi, reach)
        got_f = front & hit_box(x, refr, lo, np.array([hi[0], amp, hi[2]]), 1e9)
        miss = int(np.sum(got_r & ~inside(r_rect))) + int(np.sum(got_f & ~inside(f_rect)))
        total_missed += miss
        direct = screen_rect(lo, hi, cam)
        outside_direct = int(np.sum(got_r & ~inside(direct)))

        def area(rect, half_w=1.0, half_h=0.5625):  # a 16:9 screen about 90 degrees wide
            w = max(0.0, min(rect[1], half_w) - max(rect[0], -half_w))
            h = max(0.0, min(rect[3], half_h) - max(rect[2], -half_h))
            return 100.0 * w * h / (4.0 * half_w * half_h)

        out(f"   camera {cam_h:4.1f} m, body {dist:5.1f} m, amplitude {amp:3.1f} m, allowance {tilt_deg:4.1f} deg:"
            f" {int(got_r.sum()):6d} reflect, {int(got_f.sum()):6d} refract, {miss} outside the receiving rectangle;"
            f" {outside_direct:6d} reflections outside the body's own rectangle;"
            f" screen share: body {area(direct):5.2f}%, receiving {area((min(r_rect[0], f_rect[0]), max(r_rect[1], f_rect[1]), min(r_rect[2], f_rect[2]), max(r_rect[3], f_rect[3]))):6.2f}%")
    out(f"   sea points outside their receiving rectangle, all cases: {total_missed}")
    cam = np.array([0.0, 2.0, 0.0])
    lo, hi = np.array([19.5, 0.0, -0.5]), np.array([20.5, 2.0, 0.5])
    (rlo, rhi), _ = receiving_boxes(lo, hi, 0.0, 0.0, reach)
    d, r = screen_rect(lo, hi, cam), screen_rect(rlo, rhi, cam)
    out(f"   the review's case (camera 2 m, body 2 m tall at 20 m, calm): body rows [{d[2]:.3f}, {d[3]:.3f}],"
        f" receiving rows [{r[2]:.3f}, {r[3]:.3f}], which hold the mirror image's [-0.200, -0.100]")
    assert total_missed == 0


# --- D ---------------------------------------------------------------------------------------------------------

def part_d(out):
    out("D  foam with no birth: air -> fresh film -> aged film -> gone, lifetimes 2, 6 and 20 s, yield 1")
    ta, tf, tg = 2.0, 6.0, 20.0
    m = np.array([[-1 / ta, 0, 0], [1 / ta, -1 / tf, 0], [0, 1 / tf, -1 / tg]])
    vals, vecs = np.linalg.eig(m)

    def advance(state, dt):
        return (vecs @ np.diag(np.exp(vals * dt)) @ np.linalg.inv(vecs) @ state).real

    for name, start in (("fresh film only", np.array([0.0, 1.0, 0.0])), ("submerged air only", np.array([1.0, 0.0, 0.0]))):
        s = start.copy()
        rows, last_total, rises = [], s.sum(), False
        for i in range(1, 3601):
            s = advance(s, DT)
            rises |= s.sum() > last_total + 1e-15
            last_total = s.sum()
            if i in (36, 360, 3600):
                rows.append(f"t={i * DT:4.1f} s air {s[0]:.4f} fresh {s[1]:.4f} aged {s[2]:.4f} sum {s.sum():.4f}")
        big = advance(start, 3600 * DT)
        out(f"   {name}: " + " | ".join(rows))
        out(f"      inventory ever rose: {rises}; one 3600-tick step against 3600 steps differs by {float(np.abs(big - s).max()):.1e}")


# --- E ---------------------------------------------------------------------------------------------------------

def part_e(out):
    out("E  a submerged body of radiance 10 toward the camera; exposure 0.25; camera-to-body air T 0.8, S 0.6 (as the")
    out("   forward pass applied them); camera-to-surface air T 0.82, S 0.55; water T 0.5; Fresnel 0.02; water in-scatter 0")
    radiance, exposure, t_cam, s_cam, t_air, s_air, t_water, fresnel = 10.0, 0.25, 0.8, 0.6, 0.82, 0.55, 0.5, 0.02
    copied = (t_cam * radiance + s_cam) * exposure

    def stored(behind):
        return exposure * (t_air * (1.0 - fresnel) * t_water * behind + s_air)

    out(f"   scene colour copied: {copied:.4f}")
    out(f"   stored sea colour, from the true radiance:              {stored(radiance):.4f}")
    out(f"   from the gather (exposure divided out, camera air removed): {stored((copied / exposure - s_cam) / t_cam):.4f}")
    out(f"   from the copied colour used as radiance:                {stored(copied):.4f}")
    out(f"   from the copied colour with exposure divided out only:  {stored(copied / exposure):.4f}")
    for scale in (0.25, 4.0):
        c = (t_cam * radiance + s_cam) * scale
        right = scale * (t_air * (1.0 - fresnel) * t_water * ((c / scale - s_cam) / t_cam) + s_air)
        wrong = scale * (t_air * (1.0 - fresnel) * t_water * c + s_air)
        out(f"   exposure {scale:4.2f}: stored / exposure is {right / scale:.4f} converted, {wrong / scale:.4f} unconverted")
    extinction = 3.668e-5 + 4.4e-6
    out(f"   air left out between the water and a reflected body: over the 64 m reach, T >= {math.exp(-extinction * 64.0):.4f}"
        f" at 429 nm in ATMOSPHERE's air (Rayleigh 3.668e-5 + Mie 4.4e-6 per metre at sea level)")


def main():
    def out(line):
        print(line, flush=True)

    part_e(out)
    part_d(out)
    part_b(out)
    part_c(out)
    part_a(out)


if __name__ == "__main__":
    sys.exit(main())
