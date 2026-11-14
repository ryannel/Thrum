"""SCENE-VIEWS design, 14 November 2026: reproductions A to C. Pure Python, no dependencies, a second to run.

Part A  Jitter. A secondary view at half the main view's size each way is read by projection from the
        main view's pixels, and the main view's reconstruction averages what it reads over its jitter
        sequence. An edge in the secondary view, drawn with no jitter, with a jitter of its own, and
        with the main view's jitter phase at its own extent: where the averaged edge sits against the
        true edge, and how that changes as the edge slides by a fraction of a texel.
Part B  Air. A mirror view's camera stands below its plane. The air on the part of a line of sight
        below the plane is the air of that part's mirror image above it. How far the optical depth of
        that part is off when the altitude is taken as it stands, and when it is folded about the plane.
Part C  Clipping by the projection. An oblique near plane puts the clip plane in the view's matrix, and
        the stored depth then crowds toward one value. The world distance one step of a 32-bit float
        depth spans, against a standard reversed projection.

Run: python3 views_proof.py
"""
import math
import struct


def halton(index, base):
    weight, value = 1.0, 0.0
    while index:
        weight /= base
        value += (index % base) * weight
        index //= base
    return value - 0.5


def part_a():
    print("PART A  an edge in a half-size view, read by projection and averaged over 16 jitter phases")
    phases = [halton(i + 1, 2) for i in range(16)]
    other = [halton(i + 1, 3) for i in range(16)]

    def texel(k, shift, edge):
        """A texel of the view, sampled at its centre, moved by its jitter (in view texels)."""
        return 1.0 if (k + 0.5 + shift) * 2.0 < edge else 0.0

    def read(x, shift, edge):
        """A main-view sample at x (main pixels) reads the view bilinearly at its own projection."""
        u = x / 2.0 - shift - 0.5
        k = math.floor(u)
        t = u - k
        return (1 - t) * texel(k, shift, edge) + t * texel(k + 1, shift, edge)

    def crossing(mode, edge):
        """Where the averaged edge passes one half, in main pixels, from pixel centres."""
        vals = []
        for p in range(8, 40):
            acc = 0.0
            for j, o in zip(phases, other):
                shift = {"none": 0.0, "own": o, "frame": j}[mode]
                acc += read(p + 0.5 + j, shift, edge)
            vals.append(acc / len(phases))
        for i in range(len(vals) - 1):
            if vals[i] >= 0.5 > vals[i + 1]:
                t = (vals[i] - 0.5) / (vals[i] - vals[i + 1])
                return 8 + i + 0.5 + t, vals
        return float("nan"), vals

    for mode, name in (("none", "no jitter in the view"), ("own", "a jitter of the view's own"),
                       ("frame", "the frame's phase at the view's extent")):
        errs, widths = [], []
        for step in range(40):
            edge = 20.0 + step * 0.05
            at, vals = crossing(mode, edge)
            errs.append(at - edge)
            widths.append(sum(1 for v in vals if 0.02 < v < 0.98))
        mean = sum(errs) / len(errs)
        wander = max(errs) - min(errs)
        print(f"  {name:40s} edge placed {mean:+.2f} px from true, wandering {wander:.2f} px as it slides;"
              f" ramp {min(widths)} to {max(widths)} px wide")


def part_b():
    print("PART B  the air below a mirror plane: optical depth of the part of the path under the plane")
    for name, scale in (("molecules, 8 km", 8000.0), ("aerosol, 1.2 km", 1200.0)):
        for cam in (2.0, 10.0, 40.0):
            # a path from the mirrored camera, cam below the plane, rising to the plane: density by height
            n = 2000
            straight = folded = 0.0
            for i in range(n):
                y = -cam + (i + 0.5) * cam / n          # height about the plane
                straight += math.exp(-y / scale)
                folded += math.exp(-abs(y) / scale)
            print(f"  {name:16s} camera {cam:4.0f} m up: taken as it stands, {100 * (straight / folded - 1):.2f}% too"
                  f" dense; the camera itself would stand {cam:.0f} m below the plane")


def f32(x):
    return struct.unpack("f", struct.pack("f", x))[0]


def bump(x):
    """The next 32-bit float above x."""
    return struct.unpack("f", struct.pack("I", struct.unpack("I", struct.pack("f", x))[0] + 1))[0]


def part_c():
    print("PART C  world distance spanned by one step of a 32-bit depth, along a ray 10 degrees above the plane")
    elev = math.radians(10.0)
    for cam in (0.1, 2.0, 40.0):
        for dist in (20.0, 300.0, 3000.0):
            # standard reversed projection: z = near / depth along the axis
            near = 0.1
            d_axis = dist
            z = near / d_axis
            step_std = d_axis * d_axis / near * (bump(z) - f32(z))
            # oblique: z = 1 - s * (height above the clip plane) / (depth along the axis), s = 1
            h = -cam + dist * math.sin(elev)
            if h <= 0:
                continue
            zo = 1.0 - h / d_axis
            ulp = bump(zo) - f32(zo)
            # dz/d(dist) = -cam / dist^2 for a fixed direction
            step_obl = ulp * dist * dist / cam
            print(f"  camera {cam:5.1f} m from the plane, {dist:6.0f} m out: standard {step_std * 1000:9.4f} mm,"
                  f" oblique {step_obl * 1000:12.2f} mm")


if __name__ == "__main__":
    part_a()
    part_b()
    part_c()
