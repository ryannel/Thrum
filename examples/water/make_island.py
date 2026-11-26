#!/usr/bin/env python3
"""Make the island's ground triangles and their cooked-mesh source."""
import argparse
import json
from math import cos, exp, floor, hypot, sin
from pathlib import Path
import struct

POND_PATCH = (84, 124, 77, 109)
POND_SUBDIVISIONS = 4


def smoothstep(value):
    t = min(max(value, 0), 1)
    return t * t * (3 - 2 * t)


def height(x, z):
    # Trade wind blows towards the west beach. Its headland extends into
    # the sea to the north, with a sheltered side around the tip.
    headland = 36 * exp(-((z + 95) / 34) ** 2)
    radius = hypot((x - 124 + headland) / 230, z / 176)
    coast = 22 * (1 - radius)
    # Give swell room to refract on the exposed shelf before the depth limit.
    # Keep the existing beach slope at the waterline and blend into deep water.
    depth = max(-coast, 0)
    shelf = smoothstep((depth - 2) / 5) * (1 - smoothstep((depth - 13) / 11))
    exposed = smoothstep((-x - 100) / 45) * smoothstep((95 - abs(z)) / 55)
    coast += 0.44 * depth * shelf * exposed
    hills = (17 + 5 * sin(x / 57) * cos(z / 43)
             + 17 * exp(-((x - 257) / 65) ** 2 - ((z + 77) / 48) ** 2)
             + 15 * exp(-((x - 282) / 39) ** 2 - ((z - 11) / 62) ** 2)
             + 12 * exp(-((x - 35) / 48) ** 2 - ((z + 48) / 52) ** 2))
    hills += 0.85 * sin(x / 12 + z / 24) * cos(z / 19 - x / 51)
    hills += 0.46 * sin(x / 4.1 - z / 8.3) * sin(z / 6.8 + x / 7.6)
    rise = min(max(coast / 9, 0), 1)
    ground = coast + rise * rise * (3 - 2 * rise) * (hills - coast)
    point = 3.6 * exp(-((x - 153) / 13) ** 2 - ((z - 1) / 7) ** 2)
    lake = 8.2 + 0.12 * hypot(x - 183, (z - 5) * 1.16) + point
    hollow = 10.7 + 0.12 * hypot(x - 235, (z - 5) * 1.22)
    bend = sin((z - 93) / 9) * 2.3
    shelf_blend = min(max((x - 96) / 11, 0), 1)
    shelf = 0.72 + 0.28 * shelf_blend**2 * (3 - 2 * shelf_blend)
    outer_bank = 8.0 + 0.24 * shelf * hypot(x - 104 + bend, (z - 93) * 1.13)
    # A rounded bowl gives the close camera a fair curve through the right
    # turn. Ease back into the outer ground above the wet bank.
    pond_radius = hypot((x - 104 + 1.3 * sin((z - 93) / 11)) / 13, (z - 93) / 12)
    bowl = 8.0 + 2.6 * pond_radius**2
    outer_blend = min(max((pond_radius - 1.32) / 0.42, 0), 1)
    pond = bowl + (outer_bank - bowl) * outer_blend**2 * (3 - 2 * outer_blend)
    terrain = min(ground, lake, hollow, pond)
    # Relief behind the far shore stays above the lake's stage range. The
    # waterline and the shallow shelf keep their original surveyed heights.
    backshore = smoothstep((terrain - 13.0) / 2.2)
    ridge = (13 * exp(-((x - 198) / 26) ** 2 - ((z + 42) / 11) ** 2)
             + 9 * exp(-((x - 226) / 15) ** 2 - ((z + 31) / 11) ** 2))
    return terrain + backshore * ridge


def channel_height(x, z):
    """The hand-cut lake outlet, using the island's existing terrain elsewhere."""
    ground = height(x, z)
    side = abs(z - 5)
    # Tie the raised outlet banks back into the surveyed lake rim.
    shoulder = smoothstep((x - 200) / 9) * (1 - smoothstep((x - 222) / 15))
    shoulder *= smoothstep((side - 5.7) / 1.2) * (1 - smoothstep((side - 28) / 14))
    ground += shoulder * max(0, 14.2 - ground)
    if not (199 < x < 385 and side < 29):
        return ground
    along = min(max((x - 210) / 140, 0), 1)
    level = 11.75 - 12.55 * along
    # The mouth sill remains wet at the lowest tide, with a shallow bed.
    floor = level - 0.85 + 0.70 * smoothstep((x - 341) / 9)
    floor -= 0.65 * smoothstep((x - 350) / 10)
    # Taper the wetted banks at the narrows without moving the chart edges.
    narrow = exp(-((x - 256) / 16) ** 2)
    ends = max(1 - smoothstep((x - 210) / 9), smoothstep((x - 342) / 6))
    half_width = 4.7 - 2.35 * narrow + 1.1 * ends + (1 - ends) * 0.38 * sin((x - 219) / 6)
    offset = 0.72 * sin((x - 231) / 9) * (1 - ends) * (1 - 0.85 * narrow)
    # The tideway flares gradually, giving the drowned reach room to
    # carry floating objects around the incoming swell without stranding.
    widening = 7.6 * smoothstep((x - 321) / 28)
    half_width += widening
    channel_side = abs(z - 5 - offset)
    wall = max(0.85, level + 1.0)
    wall += 1.35 * (1 - smoothstep((x - 210) / 17))
    cut = floor + (wall - floor) * smoothstep((channel_side - half_width + 0.85) / (7.4 + widening - half_width + 0.85))
    cut = min(cut, ground) if x > 352 else cut
    weight = smoothstep((x - 199) / 9) * (1 - smoothstep((x - 362) / 23))
    fade = 10.0 + 2.1 * sin(x * 0.12) + 1.2 * sin(x * 0.30 + z)
    weight *= 1 - smoothstep((side - 7.5 - widening) / fade)
    return ground + weight * (cut - ground)


def append_grid(points, normals, uvs, indices, xs, zs, terrain, pond=False):
    base = len(points)
    nx, nz = len(xs), len(zs)
    x0, x1, z0, z1 = POND_PATCH
    for j, z in enumerate(zs):
        for i, x in enumerate(xs):
            y = terrain(x, z)
            # Keep the fine patch on the same straight edges as the coarse
            # triangles. Its interior uses the authored continuous bank.
            if pond and x in (x0, x1):
                lo = floor(z)
                y = terrain(x, lo) + (terrain(x, lo + 1) - terrain(x, lo)) * (z - lo)
            elif pond and z in (z0, z1):
                lo = floor(x)
                y = terrain(lo, z) + (terrain(lo + 1, z) - terrain(lo, z)) * (x - lo)
            points.append((x, y, z))
            dx = terrain(x + 0.1, z) - terrain(x - 0.1, z)
            dz = terrain(x, z + 0.1) - terrain(x, z - 0.1)
            norm = hypot(dx, 0.2, dz)
            normals.append((-dx / norm, 0.2 / norm, -dz / norm))
            uvs.append((x / 120, z / 120))
            outside_patch = not (x0 <= x < x1 and z0 <= z < z1)
            if i + 1 < nx and j + 1 < nz and (pond or outside_patch):
                a = base + j * nx + i
                b = a + nx
                indices.extend([(a,), (b,), (a + 1,), (a + 1,), (b,), (b + 1,)])


def main():
    args = argparse.ArgumentParser(description=__doc__)
    args.add_argument('--channel', action='store_true', help='author the island variant with the chart outlet')
    terrain = channel_height if args.parse_args().channel else height
    name = 'channel-island' if terrain is channel_height else 'island'
    # Keep the denser triangles local to the pond. The wider island uses the
    # original one-metre inland grid and larger seabed triangles.
    xs = sorted(set(range(-320, 401, 4)) | set(range(82, 281)))
    zs = sorted(set(range(-225, 226, 3)) | set(range(-34, 118)))
    points, normals, uvs, indices = [], [], [], []
    append_grid(points, normals, uvs, indices, xs, zs, terrain)
    x0, x1, z0, z1 = POND_PATCH
    n = POND_SUBDIVISIONS
    patch_xs = [x / n for x in range(x0 * n, x1 * n + 1)]
    patch_zs = [z / n for z in range(z0 * n, z1 * n + 1)]
    append_grid(points, normals, uvs, indices, patch_xs, patch_zs, terrain, pond=True)

    data = bytearray()
    views, accessors = [], []
    # Positions then indices also form the CPU map's source triangles.
    for values, fmt, kind, component in [
        (points, '3f', 'VEC3', 5126), (indices, 'I', 'SCALAR', 5125),
        (normals, '3f', 'VEC3', 5126), (uvs, '2f', 'VEC2', 5126),
    ]:
        offset = len(data)
        for value in values:
            data.extend(struct.pack('<' + fmt, *value))
        views.append({'buffer': 0, 'byteOffset': offset, 'byteLength': len(data) - offset})
        accessors.append({'bufferView': len(views) - 1, 'componentType': component,
                          'count': len(values), 'type': kind})
    accessors[0].update(min=[min(axis) for axis in zip(*points)],
                        max=[max(axis) for axis in zip(*points)])
    model = {
        'asset': {'version': '2.0'},
        'buffers': [{'byteLength': len(data), 'uri': f'{name}.bin'}],
        'bufferViews': views, 'accessors': accessors,
        'meshes': [{'primitives': [{'attributes': {'POSITION': 0, 'NORMAL': 2, 'TEXCOORD_0': 3},
                                    'indices': 1}]}],
        'nodes': [{'mesh': 0}], 'scenes': [{'nodes': [0]}], 'scene': 0,
    }
    root = Path(__file__).resolve().parent / 'assets/meshes'
    (root / f'{name}.gltf').write_text(json.dumps(model, indent=2) + '\n')
    (root / f'{name}.bin').write_bytes(data)


if __name__ == '__main__':
    main()
