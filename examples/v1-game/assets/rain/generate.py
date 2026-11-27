#!/usr/bin/env python3
"""Build the optional patch mesh with the shipped terrain's ground-texture coordinates."""
import json
import math
from pathlib import Path
import struct

HERE = Path(__file__).resolve().parent
ORIGIN = (-4, 2)
SIZE = (72, 56)
SPACING = 0.125


def relief(x, z):
    edge = ((x - 4.5) / 4.32)**4 + ((z - 3.5 + 0.08 * math.cos(2.7 * x)) / 3.4)**4
    irregular = 0.06 * math.cos(2.8 * x + 2.1 * z) * math.sin(1.3 * z - 2.0 * x)
    skirt = min(1, max(0, (1 - edge + irregular) / 0.64))
    shoulder = skirt * skirt * (3 - 2 * skirt)

    def bowl(cx, cz, rx, rz, depth):
        dx, dz = x - cx, z - cz
        r = math.hypot((dx + 0.27 * dz) / rx, dz / rz)
        border = 1 + 0.1 * math.sin(6 * dx + 4 * dz)
        return depth * min(1, max(0, 1 - (r / border)**2))

    return (239.80 + 0.21 * shoulder
            - bowl(3.0, 2.8, 1.65, 0.98, 0.065)
            - bowl(6.1, 4.35, 1.32, 1.05, 0.056)
            + shoulder * (0.002 * math.cos(2.8 * x + 4.1 * z)
                          + 0.001 * math.sin(7.2 * z - 3.3 * x)))


def ground_uv():
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
        if (min(xs) > ORIGIN[0] + SIZE[0] * SPACING or max(xs) < ORIGIN[0]
                or min(zs) > ORIGIN[1] + SIZE[1] * SPACING or max(zs) < ORIGIN[1]):
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
                if a * ay + b * by + c * cy < 230:
                    continue
                return [sum(v * weight for v, weight in zip(axis, (a, b, c))) for axis in zip(*texels)]
        raise ValueError(f'No terrain UV at {x}, {z}')

    return interpolate


def generate():
    texcoord = ground_uv()
    width, height = SIZE
    positions, normals, uv, indices = [], [], [], []
    for row in range(height + 1):
        for col in range(width + 1):
            x, z = col * SPACING, row * SPACING
            position = [ORIGIN[0] + x, relief(x, z), ORIGIN[1] + z]
            dx = (relief(x + SPACING, z) - relief(x - SPACING, z)) / (2 * SPACING)
            dz = (relief(x, z + SPACING) - relief(x, z - SPACING)) / (2 * SPACING)
            scale = math.sqrt(1 + dx**2 + dz**2)
            positions.append(position)
            normals.append([-dx / scale, 1 / scale, -dz / scale])
            uv.append(texcoord(position[0], position[2]))
            if row < height and col < width:
                a = row * (width + 1) + col
                b, c = a + 1, a + width + 1
                indices.extend((a, c, b, b, c, c + 1))
    streams = [b''.join(struct.pack('<3f', *v) for v in positions),
               b''.join(struct.pack('<3f', *v) for v in normals),
               b''.join(struct.pack('<2f', *v) for v in uv),
               struct.pack(f'<{len(indices)}H', *indices)]
    raw = b''.join(streams)
    (HERE / 'rain-patch.bin').write_bytes(raw)
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
    (HERE / 'rain-patch.gltf').write_text(json.dumps({
        'asset': {'version': '2.0', 'generator': 'Thrum valley rain patch'},
        'scene': 0, 'scenes': [{'nodes': [0]}], 'nodes': [{'mesh': 0}],
        'meshes': [{'name': 'rain-patch', 'primitives': [{
            'attributes': {'POSITION': 0, 'NORMAL': 1, 'TEXCOORD_0': 2}, 'indices': 3,
        }]}],
        'buffers': [{'uri': 'rain-patch.bin', 'byteLength': len(raw)}],
        'bufferViews': fields, 'accessors': accessors,
    }, separators=(',', ':')) + '\n')


if __name__ == '__main__':
    generate()
