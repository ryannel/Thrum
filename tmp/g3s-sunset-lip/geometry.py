"""Compare the saved normal Metal frame to rays through its exact cooked terrain source."""
from bisect import bisect_right
import csv
import json
import math
from pathlib import Path
import struct
import sys

ROOT = Path(__file__).resolve().parents[2]
RUN = ROOT / 'tmp/g3s-star-catalog/integration/tmp/g7-showcase/runs/check-20261107-193208'
SOURCE, IMAGES = (map(Path, sys.argv[1:]) if len(sys.argv) == 3
                  else (RUN / 'source', RUN / 'native' / 'route'))
SCENE = json.loads((SOURCE / 'scene.json').read_text())
GLTF = json.loads((SOURCE / 'terrain.gltf').read_text())
BUFFERS = [(SOURCE / buf['uri']).read_bytes() for buf in GLTF['buffers']]
PRIMITIVE = GLTF['meshes'][0]['primitives'][0]


def read_accessor(index, format):
    spec = GLTF['accessors'][index]
    view = GLTF['bufferViews'][spec['bufferView']]
    start = view.get('byteOffset', 0) + spec.get('byteOffset', 0)
    assert view.get('byteStride', struct.calcsize(format)) == struct.calcsize(format)
    size = spec['count'] * struct.calcsize(format)
    return list(struct.iter_unpack(format, BUFFERS[view['buffer']][start:start + size]))


positions = read_accessor(PRIMITIVE['attributes']['POSITION'], '<3f')
normals = read_accessor(PRIMITIVE['attributes']['NORMAL'], '<3f')
index_format = {5123: '<H', 5125: '<I'}[GLTF['accessors'][PRIMITIVE['indices']]['componentType']]
indices = [i for i, in read_accessor(PRIMITIVE['indices'], index_format)]
NX = next(i for i, p in enumerate(positions) if p[2] != positions[0][2])
xs = [p[0] for p in positions[:NX]]
zs = [-p[2] for p in positions[::NX]]
assert xs == sorted(xs) and zs == sorted(zs)
camera = SCENE['camera']
eye = camera['position_m']
assert SCENE['placements'][0]['mesh'] == 'terrain.gltf'
assert SCENE['placements'][0]['translation_m'] == [0, 0, 0]
assert SCENE['placements'][0]['scale'] == [1, 1, 1]


def sub(a, b):
    return tuple(x - y for x, y in zip(a, b))


def cross(a, b):
    ax, ay, az = a
    bx, by, bz = b
    return ay * bz - az * by, az * bx - ax * bz, ax * by - ay * bx


def dot(a, b):
    return sum(x * y for x, y in zip(a, b))


def unit(a):
    length = math.sqrt(dot(a, a))
    return tuple(x / length for x in a)


def triangle(ray, vertices, origin):
    a, b, c = (positions[i] for i in vertices)
    u, v = sub(b, a), sub(c, a)
    p = cross(ray, v)
    det = dot(u, p)
    if abs(det) < 1e-12:
        return
    delta = sub(origin, a)
    w1 = dot(delta, p) / det
    q = cross(delta, u)
    w2 = dot(ray, q) / det
    t = dot(v, q) / det
    if min(w1, w2, 1 - w1 - w2) >= -1e-9 and t > 0:
        n = unit(tuple(sum(weight * normals[i][axis]
                           for weight, i in zip((1 - w1 - w2, w1, w2), vertices))
                       for axis in range(3)))
        return t, n, unit(cross(u, v))


def trace(ray, origin=eye):
    events = {0, camera['far_m']}
    for coords, offset, rate in [(xs, origin[0], ray[0]), (zs, -origin[2], -ray[2])]:
        if abs(rate) > 1e-12:
            events.update(t for p in coords if 0 < (t := (p - offset) / rate) < camera['far_m'])
    events = sorted(events)
    for a, b in zip(events, events[1:]):
        t = (a + b) / 2
        i = bisect_right(xs, origin[0] + t * ray[0]) - 1
        j = bisect_right(zs, -origin[2] - t * ray[2]) - 1
        if not (0 <= i < NX - 1 and 0 <= j < len(zs) - 1):
            continue
        offset = 6 * (j * (NX - 1) + i)
        hits = [hit for k in (0, 3)
                if (hit := triangle(ray, indices[offset+k:offset+k+3], origin)) is not None
                and a - 1e-6 <= hit[0] <= b + 1e-6]
        if hits:
            return min(hits)


def image(stop):
    header, size, maximum, pixels = (IMAGES / f'showcase-{stop}.ppm').read_bytes().split(b'\n', 3)
    assert header == b'P6' and maximum == b'255'
    return (*map(int, size.split()), pixels)


def rows():
    for stop, yaw, pitch in [('sunset-clear-sun', -90, 0), ('sunset-sun', -90, 0), ('noon', 0, 0)]:
        w, h, pixels = image(stop)
        yaw, pitch = map(math.radians, (yaw, pitch))
        forward = (math.cos(pitch) * math.sin(yaw), math.sin(pitch), -math.cos(pitch) * math.cos(yaw))
        right = unit(cross(forward, (0, 1, 0)))
        up = cross(right, forward)
        tan_y = math.tan(math.radians(camera['vertical_fov_degrees']) / 2)
        for x in [w // 4, w // 2, 3 * w // 4]:
            for y in range(390, 500):
                sx = (2 * (x + .5) - w) / h * tan_y
                sy = (1 - 2 * (y + .5) / h) * tan_y
                ray = unit(tuple(f + sx * r + sy * u for f, r, u in zip(forward, right, up)))
                hit = trace(ray)
                if hit is None:
                    continue
                distance, normal, face_normal = hit
                offset = 3 * (y * w + x)
                r, g, b = pixels[offset:offset + 3]
                yield (stop, x, y, round(distance, 3), *(round(v, 6) for v in normal), round(-dot(normal, ray), 6), round(-dot(face_normal, ray), 6), r, g, b)


if __name__ == '__main__':
    writer = csv.writer(sys.stdout)
    writer.writerow(('stop', 'x', 'y', 'range_m', 'normal_x', 'normal_y', 'normal_z', 'shading_normal_dot_view', 'face_normal_dot_view', 'r', 'g', 'b'))
    writer.writerows(rows())
