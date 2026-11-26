#!/usr/bin/env python3
"""Author the optional island yard as cooked-mesh sources."""

import json
import math
import struct
from pathlib import Path

from make_island import height as island_height
from make_island import smoothstep

ORIGIN = (146.0, 42.0)
SIDE = (10, 8)
DATUM = 16.55
SUBDIVISIONS = 20
TRACK_EDGE = 3
POTHOLE = (3.2, 5.2)
SHALLOW_DIP = (5.75, 5.35)
ROOF = ((6.7, 6.15), (9.25, 7.45))
POST_WIDTH = 0.13


def dip(x, z, centre, radii, depth, angle=0.0):
    """A level pit with curved sides that fade into the surrounding grade."""
    dx, dz = x - centre[0], z - centre[1]
    along = math.cos(angle) * dx + math.sin(angle) * dz
    across = -math.sin(angle) * dx + math.cos(angle) * dz
    radius = math.hypot(along / radii[0], across / radii[1])
    return -depth * (1.0 - smoothstep(radius))


def paving(x, z):
    # The two depressions overlap at their lowest pass. The larger catches
    # the sloping apron and the shallow one fills later.
    pothole = dip(x, z, POTHOLE, (2.2, 1.5), 0.150, 0.09)
    shallow = dip(x, z, SHALLOW_DIP, (2.2, 1.34), 0.104, -0.04)
    slope = 0.013 * smoothstep((x - 4.8) / 1.5) * smoothstep((z - TRACK_EDGE) / 0.45)
    joint = min(abs(math.remainder(x, 0.5)), abs(math.remainder(z, 0.5)))
    relief = 0.002 * (1.0 - smoothstep(joint / 0.05))
    relief *= smoothstep((z - TRACK_EDGE) / 0.25)
    return DATUM + min(pothole, shallow) + slope - relief


def track(x, z):
    # Two ruts leave a crown between the wheel paths and a dry shoulder.
    bend = 0.13 * math.sin(0.8 * x)
    length = smoothstep((x - 0.65) / 1.5) * smoothstep((9.4 - x) / 1.8)
    rut = min(
        dip(x, z + bend, (3.1, 1.17), (3.1, 0.34), 0.11),
        dip(x, z + bend, (5.9, 2.17), (3.1, 0.34), 0.087),
    )
    texture = 0.0016 * math.sin(14.0 * x + 21.0 * z) * math.sin(18.0 * x - 10.0 * z)
    shoulder = smoothstep(min(z, TRACK_EDGE - z) / 0.25)
    return DATUM + rut + texture * length * shoulder


def terrain(x, z, grade):
    world = (ORIGIN[0] + x, ORIGIN[1] + z)
    # The outer skirt meets the original island below its opaque triangles.
    apron = smoothstep(min(x, SIDE[0] - x, z, SIDE[1] - z) / 0.7)
    base = island_height(*world) - 0.05
    return base + apron * (grade(x, z) - base)


class Mesh:
    """Positions and indices are first, so a reader can use the same ground triangles."""

    def __init__(self):
        self.positions = []
        self.indices = []
        self.normals = []
        self.uvs = []

    def grid(self, z_start, z_end, grade):
        n = SUBDIVISIONS
        width = SIDE[0] * n + 1
        for j in range(z_start * n, z_end * n + 1):
            for i in range(width):
                x, z = i / n, j / n
                step = 0.01
                dx = terrain(x + step, z, grade) - terrain(x - step, z, grade)
                dz = terrain(x, z + step, grade) - terrain(x, z - step, grade)
                length = math.hypot(dx, 2 * step, dz)
                self.positions.append((ORIGIN[0] + x, terrain(x, z, grade), ORIGIN[1] + z))
                self.normals.append((-dx / length, 2 * step / length, -dz / length))
                self.uvs.append((x, z))
                if i + 1 < width and j < z_end * n:
                    a = len(self.positions) - 1
                    b = a + width
                    self.indices.extend((a, b, a + 1, a + 1, b, b + 1))

    def box(self, low, high):
        for axis in range(3):
            u, v = (axis + 1) % 3, (axis + 2) % 3
            for positive in (False, True):
                start = len(self.positions)
                normal = tuple((1.0 if positive else -1.0) if a == axis else 0.0 for a in range(3))
                for iu, iv in ((0, 0), (1, 0), (0, 1), (1, 1)):
                    point = list(high if positive else low)
                    point[u] = (low, high)[iu][u]
                    point[v] = (low, high)[iv][v]
                    self.positions.append((ORIGIN[0] + point[0], point[1], ORIGIN[1] + point[2]))
                    self.normals.append(normal)
                    self.uvs.append((iu, iv))
                order = (0, 1, 2, 1, 3, 2) if positive else (0, 2, 1, 1, 2, 3)
                self.indices.extend(start + index for index in order)

    def save(self, folder, name):
        # Round positions to the asset's precision before writing accessor bounds.
        position = struct.Struct("<3f")
        points = [position.unpack(position.pack(*p)) for p in self.positions]
        payload = bytearray()
        views, accessors = [], []
        for values, layout, kind, component in (
            (points, "3f", "VEC3", 5126),
            (self.indices, "I", "SCALAR", 5125),
            (self.normals, "3f", "VEC3", 5126),
            (self.uvs, "2f", "VEC2", 5126),
        ):
            offset = len(payload)
            for value in values:
                args = (value,) if isinstance(value, int) else value
                payload.extend(struct.pack("<" + layout, *args))
            views.append(dict(buffer=0, byteOffset=offset, byteLength=len(payload) - offset))
            accessors.append(dict(bufferView=len(views) - 1, componentType=component,
                                  count=len(values), type=kind))
        accessors[0].update(min=list(map(min, zip(*points))), max=list(map(max, zip(*points))))
        model = {
            "asset": {"version": "2.0"},
            "buffers": [{"byteLength": len(payload), "uri": name + ".bin"}],
            "bufferViews": views,
            "accessors": accessors,
            "meshes": [{"primitives": [{"attributes": {"POSITION": 0, "NORMAL": 2, "TEXCOORD_0": 3},
                                       "indices": 1}]}],
            "nodes": [{"mesh": 0}], "scenes": [{"nodes": [0]}], "scene": 0,
        }
        (folder / (name + ".gltf")).write_text(json.dumps(model, indent=2) + "\n", encoding="utf-8")
        (folder / (name + ".bin")).write_bytes(payload)


def main():
    folder = Path(__file__).resolve().parent / "assets/meshes"
    mesh = Mesh()
    mesh.grid(TRACK_EDGE, SIDE[1], paving)
    mesh.save(folder, "rain-yard-paving")
    mesh = Mesh()
    mesh.grid(0, TRACK_EDGE, track)
    mesh.save(folder, "rain-yard-track")

    (x0, z0), (x1, z1) = ROOF
    underside = DATUM + 2.2
    roof = Mesh()
    roof.box((x0, underside, z0), (x1, underside + 0.10, z1))
    roof.save(folder, "rain-yard-roof")

    posts = Mesh()
    for x in (x0 + 0.12, x1 - 0.12 - POST_WIDTH):
        for z in (z0 + 0.10, z1 - 0.10 - POST_WIDTH):
            posts.box((x, paving(x, z) - 0.04, z), (x + POST_WIDTH, underside, z + POST_WIDTH))
    x, z = POTHOLE
    posts.box((x, paving(x, z) - 0.04, z), (x + POST_WIDTH, DATUM + 1.25, z + POST_WIDTH))
    posts.save(folder, "rain-yard-posts")

    wall = Mesh()
    wall.box((0.9, DATUM - 0.16, 7.04), (5.65, DATUM + 1.3, 7.27))
    wall.save(folder, "rain-yard-wall")


if __name__ == "__main__":
    main()
