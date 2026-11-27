#!/usr/bin/env python3
"""Build the optional patch mesh with the shipped terrain's ground-texture coordinates."""
import json
import math
from pathlib import Path
import struct

HERE = Path(__file__).resolve().parent
EXIT = HERE.with_name("exit")
ORIGIN = (-4, 2)
SIZE = (144, 112)
SPACING = 0.0625
HOLLOWS = ((3.0, 2.8, 1.65, 0.98, 0.044, 239.938),
           (6.1, 4.35, 1.32, 1.05, 0.040, 239.946))


def radius(x, z, cx, cz, rx, rz):
    dx, dz = x - cx, z - cz
    edge = 1 + 0.055 * math.sin(5 * dx + 4 * dz)
    return math.hypot((dx + 0.27 * dz) / rx, dz / rz) / edge


def relief(x, z, ground):
    terrain, _ = ground(ORIGIN[0] + x, ORIGIN[1] + z)

    def hollow(cx, cz, rx, rz, depth, bank):
        r = radius(x, z, cx, cz, rx, rz)
        # Give each hollow its own bank. The shoulder meets the original
        # floor with zero slope, outside the water and between the pools.
        t = min(1, max(0, (1.72 - r) / 0.67))
        shoulder = t**3 * (10 - 15*t + 6*t*t)
        bowl = depth * max(0, 1 - r*r)**1.1
        rock = 0.0008 * math.cos(2.8 * x + 4.1 * z)
        return shoulder * max(0, bank + rock - bowl - terrain)

    edge = min(x, 9 - x, z, 7 - z)
    t = min(1, max(0, edge / 0.45))
    boundary = t*t * (3 - 2*t)
    raise_by = max(hollow(*bowl) for bowl in HOLLOWS)
    return terrain + boundary * raise_by - 0.0015


def terrain_sample():
    recipe = json.loads((HERE.parent / 'showcase/terrain.gltf').read_text())
    terrain = (HERE.parent / 'showcase/terrain.bin').read_bytes()

    def read(view, layout):
        bounds = recipe['bufferViews'][view]
        start = bounds['byteOffset']
        return list(struct.iter_unpack(layout, terrain[start:start + bounds['byteLength']]))

    positions = read(0, '<3f')
    uv = read(2, '<2f')
    triangles = []
    for indices in read(3, '<3H'):
        vertices = [positions[i] for i in indices]
        xs, _, zs = zip(*vertices)
        if (min(xs) > ORIGIN[0] + (SIZE[0] + 1) * SPACING or max(xs) < ORIGIN[0] - SPACING
                or min(zs) > ORIGIN[1] + (SIZE[1] + 1) * SPACING or max(zs) < ORIGIN[1] - SPACING):
            continue
        (ax, _, az), (bx, _, bz), (cx, _, cz) = vertices
        det = (bz - cz) * (ax - cx) + (cx - bx) * (az - cz)
        if abs(det) > 1e-9:
            triangles.append((vertices, [uv[i] for i in indices], det))

    def interpolate(x, z):
        for ((ax, ay, az), (bx, by, bz), (cx, cy, cz)), texels, det in triangles:
            a = ((bz - cz) * (x - cx) + (cx - bx) * (z - cz)) / det
            b = ((cz - az) * (x - cx) + (ax - cx) * (z - cz)) / det
            c = 1 - a - b
            if min(a, b, c) >= -1e-6:
                y = a * ay + b * by + c * cy
                if y < 230:
                    continue
                weights = (a, b, c)
                texture = [sum(v * weight for v, weight in zip(axis, weights)) for axis in zip(*texels)]
                return y, texture
        raise ValueError(f'No terrain UV at {x}, {z}')

    return interpolate


def generate():
    ground = terrain_sample()
    width, height = SIZE
    positions, normals, uv, indices = [], [], [], []
    for row in range(height + 1):
        for col in range(width + 1):
            x, z = col * SPACING, row * SPACING
            position = [ORIGIN[0] + x, relief(x, z, ground), ORIGIN[1] + z]
            dx = (relief(x + SPACING, z, ground) - relief(x - SPACING, z, ground)) / (2 * SPACING)
            dz = (relief(x, z + SPACING, ground) - relief(x, z - SPACING, ground)) / (2 * SPACING)
            scale = math.sqrt(1 + dx**2 + dz**2)
            positions.append(position)
            normals.append([-dx / scale, 1 / scale, -dz / scale])
            uv.append(ground(position[0], position[2])[1])
            if row < height and col < width:
                a = row * (width + 1) + col
                b, c = a + 1, a + width + 1
                indices.extend((a, c, b, b, c, c + 1))
    # The lattice contains both catchments. Ground beyond the banks
    # belongs to the valley's open runoff rather than the two pools.
    outlets = bytes(
        not any(radius((x + 0.5) * SPACING, (z + 0.5) * SPACING, *bowl[:4]) < 1.14
                for bowl in HOLLOWS)
        for z in range(height) for x in range(width)
    )
    (HERE / 'outlets.bin').write_bytes(outlets)
    streams = [b''.join(struct.pack('<3f', *v) for v in positions),
               b''.join(struct.pack('<3f', *v) for v in normals),
               b''.join(struct.pack('<2f', *v) for v in uv),
               struct.pack(f'<{len(indices)}H', *indices)]
    raw = b''.join(streams)
    (EXIT / 'exit-rain-patch.bin').write_bytes(raw)
    fields = [
        {'buffer': 0, 'byteOffset': sum(map(len, streams[:i])), 'byteLength': len(data),
         'target': 34963 if i == 3 else 34962} for i, data in enumerate(streams)
    ]
    accessors = [
        {'bufferView': 0, 'componentType': 5126, 'count': len(positions), 'type': 'VEC3',
         'min': [min(axis) for axis in zip(*positions)], 'max': [max(axis) for axis in zip(*positions)]},
        {'bufferView': 1, 'componentType': 5126, 'count': len(positions), 'type': 'VEC3'},
        {'bufferView': 2, 'componentType': 5126, 'count': len(positions), 'type': 'VEC2'},
        {'bufferView': 3, 'componentType': 5123, 'count': len(indices), 'type': 'SCALAR'},
    ]
    (EXIT / 'exit-rain-patch.gltf').write_text(json.dumps({
        'asset': {'version': '2.0', 'generator': 'Thrum valley rain patch'},
        'scene': 0, 'scenes': [{'nodes': [0]}], 'nodes': [{'mesh': 0}],
        'meshes': [{'name': 'exit-rain-patch', 'primitives': [{
            'attributes': {'POSITION': 0, 'NORMAL': 1, 'TEXCOORD_0': 2}, 'indices': 3,
        }]}],
        'buffers': [{'uri': 'exit-rain-patch.bin', 'byteLength': len(raw)}],
        'bufferViews': fields, 'accessors': accessors,
    }, separators=(',', ':')) + '\n')


if __name__ == '__main__':
    generate()
