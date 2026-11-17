#!/usr/bin/env python3
"""Author the exit layout and the stand-in meshes for the sizing sweep."""
import argparse
import bisect
import json
import math
import random
import struct
from pathlib import Path

import generate

GAME = Path(__file__).resolve().parents[2]
SOURCE = Path(__file__).resolve().parent
EXIT = GAME / 'assets/exit'
ROUTE = GAME / 'routes/exit-flythrough.json'
COUNTS = (10_000, 1_500)
MESH_NAMES = ('exit-boulder', 'exit-scrub', 'exit-cactus', 'exit-standing-rock', 'exit-balloon')
SWEEP = ((8, 4), (16, 8), (32, 12))


class Terrain:
    """Sample the actual position and index accessors of the shipped glTF."""

    def __init__(self):
        doc = json.loads((SOURCE / 'terrain.gltf').read_text())
        primitive = doc['meshes'][0]['primitives'][0]
        blob = (SOURCE / doc['buffers'][0]['uri']).read_bytes()

        def read(index, fmt, component):
            accessor = doc['accessors'][index]
            view = doc['bufferViews'][accessor['bufferView']]
            assert accessor['componentType'] == component and 'byteStride' not in view
            start = view.get('byteOffset', 0) + accessor.get('byteOffset', 0)
            end = start + accessor['count'] * struct.calcsize(fmt)
            return list(struct.iter_unpack(fmt, blob[start:end]))

        self.positions = read(primitive['attributes']['POSITION'], '<3f', 5126)
        indices = read(primitive['indices'], '<H', 5123)
        self.triangles = [tuple(item[0] for item in indices[i:i + 3])
                          for i in range(0, len(indices), 3)]
        self.xs = sorted({x for x, _, _ in self.positions})
        self.depths = sorted({-z for _, _, z in self.positions})
        assert len(self.triangles) == 2 * (len(self.xs) - 1) * (len(self.depths) - 1)

    def height(self, x, z):
        assert self.xs[0] <= x < self.xs[-1] and self.depths[0] <= -z < self.depths[-1]
        i = bisect.bisect_right(self.xs, x) - 1
        j = bisect.bisect_right(self.depths, -z) - 1
        start = 2 * (j * (len(self.xs) - 1) + i)
        for triangle in self.triangles[start:start + 2]:
            a, b, c = (self.positions[index] for index in triangle)
            determinant = (b[2] - c[2]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[2] - c[2])
            u = ((b[2] - c[2]) * (x - c[0]) + (c[0] - b[0]) * (z - c[2])) / determinant
            v = ((c[2] - a[2]) * (x - c[0]) + (a[0] - c[0]) * (z - c[2])) / determinant
            if min(u, v, 1 - u - v) >= -1e-8:
                return u * a[1] + v * b[1] + (1 - u - v) * c[1]
        raise ValueError(f'point outside shipped terrain triangles at {(x, z)}')


def path_distance(x, z, keys):
    distance = math.hypot(x, z - 16)
    for a, b in zip(keys, keys[1:]):
        dx, dz = b[0] - a[0], b[2] - a[2]
        phase = ((x - a[0]) * dx + (z - a[2]) * dz) / (dx * dx + dz * dz)
        t = min(1, max(0, phase))
        distance = min(distance, math.hypot(x - a[0] - t * dx, z - a[2] - t * dz))
    return distance


def route_eyes():
    return [key['eye'] for key in json.loads(ROUTE.read_text())['keys']]


def loop_position(center, extent, period, offset, tick):
    angle = math.tau * ((tick + offset) % period) / period
    return (center[0] + extent[0] * math.cos(angle),
            center[1] + extent[1] * math.sin(angle),
            center[2] + extent[2] * math.sin(angle))


def generate_layout(terrain, keys):
    rng = random.Random(0x76A11E7)
    uniform = rng.uniform
    entities, loops = [], []
    for _ in range(COUNTS[1]):
        # Broad, uneven altitude bands with a smaller group over each near slope.
        x = rng.gauss(0, 1390)
        z = rng.gauss(-2280, 1390)
        if rng.random() < .2:
            x, z = rng.gauss(-320, 260), rng.gauss(-650, 310)
        x, z = min(6700, max(-6700, x)), min(2800, max(-6700, z))
        extent = tuple(round(value, 3) for value in (uniform(15, 80), uniform(6, 18), uniform(15, 80)))
        period, offset = rng.randrange(16_000, 36_000), rng.randrange(36_000)
        scale = round(uniform(3.5, 10), 3)
        top = max(terrain.height(x + dx, z + dz)
                  for dx in (-extent[0], 0, extent[0]) for dz in (-extent[2], 0, extent[2]))
        center = tuple(round(value, 4) for value in (x, top + uniform(65, 295), z))
        loops.append([center, extent, period, offset])
        position = loop_position(center, extent, period, offset, 0)
        entities.append([4, rng.randrange(4), *map(lambda v: round(v, 4), position), 0, scale])
    for _ in range(COUNTS[0] - COUNTS[1]):
        # Smooth falloff around the lookout and route. Patch weights give the small
        # scrub and cacti different scatter from the much wider rock distribution.
        while True:
            mesh = rng.choices(range(4), weights=(26, 43, 20, 11))[0]
            reach = (1200, 125, 240, 1800)[mesh]
            scale = round(uniform(.56, 1.8) * (2.4, .55, .85, 2.1)[mesh], 3)
            key = rng.choice(keys)
            sigma = reach * uniform(.35, 1.1)
            x = round(rng.gauss(key[0], sigma), 4)
            z = round(rng.gauss(key[2], sigma), 4)
            if abs(x) > 8000 or abs(z) > 8000 or path_distance(x, z, keys) < 10 + 2 * scale:
                continue
            if any(math.hypot(x - px, z - pz) < 3 + 2 * scale for px, pz in [(-3.4, -5), (3.8, -7)]):
                continue
            if rng.random() < .48 + .46 * generate.noise(x, z, reach * .28):
                break
        y = round(terrain.height(x, z), 4)
        entities.append([mesh, rng.randrange(4), x, y, z, round(uniform(-math.pi, math.pi), 6), scale])
    return {'entities': entities, 'loops': loops}


def layout_bytes(layout):
    return ('{"entities":[\n' + ',\n'.join(json.dumps(row, separators=(',', ':'))
                                           for row in layout['entities'])
            + '\n],"loops":[\n' + ',\n'.join(json.dumps(row, separators=(',', ':'))
                                             for row in layout['loops']) + '\n]}\n').encode()


def verify_layout(layout, terrain, keys):
    entities, loops = layout['entities'], layout['loops']
    assert (len(entities), len(loops)) == COUNTS
    min_clearance = math.inf
    for index, (mesh, _, x, y, z, _, scale) in enumerate(entities):
        if index < len(loops):
            assert mesh == 4
            authored = loops[index]
            assert math.dist((x, y, z), loop_position(*authored, 0)) < .0001
            for step in range(360):
                position = loop_position(*authored, step * authored[2] / 360)
                px, py, pz = position
                assert -7000 < px < 7000 and -7000 < pz < 3200
                clearance = py - scale * 2 - terrain.height(px, pz)
                min_clearance = min(min_clearance, clearance)
                assert clearance > 12, (index, step, clearance)
        else:
            assert mesh < 4 and abs(y - terrain.height(x, z)) <= .000051
            assert path_distance(x, z, keys) >= 10 + 2 * scale
    return min_clearance


def write_stand_ins(output, tier):
    generate.ROOT = output
    output.mkdir(parents=True, exist_ok=True)
    segments, rings = SWEEP[tier]
    points, normals, uv, triangles = generate.round_mesh(segments, rings)
    material = {'baseColorFactor': [1, 1, 1, 1], 'metallicFactor': 0, 'roughnessFactor': .9}
    for mesh, name in enumerate(MESH_NAMES):
        # Stand-ins have the same local envelope across all sweep tiers.
        # The rocks and vegetation are rooted. The balloon is centred on its path.
        shaped = [(x, y if mesh == 4 else (y + 1) * .5, z) for x, y, z in points]
        shaped_normals = [generate.unit((x, y * (1 if mesh == 4 else 2), z)) for x, y, z in normals]
        generate.write_mesh(name, (shaped, shaped_normals, uv, triangles), material)
    print(f'{len(triangles)} triangles per prop at tier {tier}')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--write-layout', action='store_true')
    parser.add_argument('--stand-ins', type=Path)
    parser.add_argument('--tier', type=int, choices=range(len(SWEEP)))
    args = parser.parse_args()
    if args.stand_ins is not None:
        if args.tier is None:
            parser.error('--stand-ins requires --tier')
        write_stand_ins(args.stand_ins, args.tier)
        return
    terrain, keys = Terrain(), route_eyes()
    layout = generate_layout(terrain, keys)
    destination = EXIT / 'layout.json'
    if args.write_layout:
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(layout_bytes(layout))
    clearance = verify_layout(json.loads(destination.read_bytes()), terrain, keys)
    assert destination.read_bytes() == layout_bytes(layout), 'exit layout does not regenerate byte for byte'
    print(f'{COUNTS[0]} entities, {COUNTS[1]} loops, minimum sampled terrain clearance {clearance:.2f} m')


if __name__ == '__main__':
    main()
