#!/usr/bin/env python3
"""Generate the lake's pine in metres, with separate bark and needle meshes."""
from base64 import b64encode
from json import dumps
from math import cos, sin, sqrt, tau
from pathlib import Path
from struct import pack

MESHES = Path(__file__).parent / 'assets' / 'meshes'


def triangle(rows, points):
    p, q, r = points
    u, v = ([b - a for a, b in zip(p, q)], [b - a for a, b in zip(p, r)])
    n = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]]
    length = sqrt(sum(x*x for x in n))
    if length > 1e-9:
        rows.extend((point, [x / length for x in n]) for point in points)


def ring(radius, y, phase=0, rough=0):
    return [((radius + rough * cos(i * 2.4)) * cos(i * tau / 14 + phase),
             y + rough * sin(i * 1.7),
             (radius + rough * cos(i * 2.4)) * sin(i * tau / 14 + phase)) for i in range(14)]


def frustum(rows, low, high):
    for i in range(len(low)):
        j = (i + 1) % len(low)
        triangle(rows, [low[i], high[i], low[j]])
        triangle(rows, [low[j], high[i], high[j]])


def write(name, rows):
    count = len(rows)
    positions = [position for position, _ in rows]
    normals = [normal for _, normal in rows]
    data = b''.join(pack('<3f', *p) for p in positions + normals)
    data += b''.join(pack('<2f', p[0] / 4, p[1] / 6) for p in positions)
    sizes = [12 * count, 12 * count, 8 * count]
    model = {
        'asset': {'version': '2.0'},
        'buffers': [{'byteLength': len(data), 'uri': 'data:application/octet-stream;base64,' + b64encode(data).decode()}],
        'bufferViews': [{'buffer': 0, 'byteOffset': sum(sizes[:i]), 'byteLength': size} for i, size in enumerate(sizes)],
        'accessors': [{'bufferView': i, 'componentType': 5126, 'count': count, 'type': kind}
                      for i, kind in enumerate(('VEC3', 'VEC3', 'VEC2'))],
        'meshes': [{'primitives': [{'attributes': {'POSITION': 0, 'NORMAL': 1, 'TEXCOORD_0': 2}}]}],
        'nodes': [{'mesh': 0}], 'scenes': [{'nodes': [0]}], 'scene': 0,
    }
    model['accessors'][0].update(min=[min(p[k] for p in positions) for k in range(3)],
                                 max=[max(p[k] for p in positions) for k in range(3)])
    (MESHES / f'{name}.gltf').write_text(dumps(model, separators=(',', ':')) + '\n')


bark, needles = [], []
frustum(bark, ring(0.23, -0.2), ring(0.035, 5.65))
for i, (y, spread, rise) in enumerate([(1.2, 1.73, 2.6), (2.25, 1.48, 2.15), (3.24, 1.15, 1.7),
                                      (4.06, 0.82, 1.55), (4.84, 0.49, 1.35)]):
    phase = 0.23 * i
    skirt = ring(spread, y, phase, spread * 0.12)
    frustum(needles, ring(spread * 0.13, y - 0.18, phase), skirt)
    frustum(needles, skirt, ring(0.025, y + rise, phase))
write('pine-bark', bark)
write('pine-needles', needles)
