"""Generate the mirror scene's post and terrain meshes with the standard library."""

import json
import math
from pathlib import Path
import struct


def main():
    x, y, z = 0.18, 2.45, 0.18
    faces = [
        ((0, 0, 1), [(-x, 0, z), (x, 0, z), (x, y, z), (-x, y, z)]),
        ((0, 0, -1), [(x, 0, -z), (-x, 0, -z), (-x, y, -z), (x, y, -z)]),
        ((1, 0, 0), [(x, 0, z), (x, 0, -z), (x, y, -z), (x, y, z)]),
        ((-1, 0, 0), [(-x, 0, -z), (-x, 0, z), (-x, y, z), (-x, y, -z)]),
        ((0, 1, 0), [(-x, y, z), (x, y, z), (x, y, -z), (-x, y, -z)]),
        ((0, -1, 0), [(-x, 0, -z), (x, 0, -z), (x, 0, z), (-x, 0, z)]),
    ]
    positions = [point for _, corners in faces for point in corners]
    normals = [normal for normal, corners in faces for _ in corners]
    uvs = [(0, 0), (1, 0), (1, 1), (0, 1)] * len(faces)
    indices = [4 * i + j for i in range(len(faces)) for j in [0, 1, 2, 0, 2, 3]]
    write("post", positions, normals, uvs, indices)
    write("ground", [(-4000, 0, 4000), (4000, 0, 4000), (4000, 0, -4000), (-4000, 0, -4000)],
          [(0, 1, 0)] * 4, [(0, 0), (1, 0), (1, 1), (0, 1)], [0, 1, 2, 0, 2, 3])
    bank()
    hill()


def bank():
    # The slope faces the pool and both eyes. Descend into the plain behind it.
    ridge_height, ridge_distance, far_edge = 2.4, 10, 40
    for_shore = [(0, 0), (ridge_height, -ridge_distance), (-0.065, -far_edge)]
    positions, normals, uvs, indices = [], [], [], []
    for (y0, z0), (y1, z1) in zip(for_shore, for_shore[1:]):
        rise, run = y1 - y0, z0 - z1
        length = math.hypot(rise, run)
        start = len(positions)
        positions.extend([(-44, y0, z0), (44, y0, z0), (44, y1, z1), (-44, y1, z1)])
        normals.extend([(0, run / length, rise / length)] * 4)
        uvs.extend([(0, 0), (1, 0), (1, 1), (0, 1)])
        indices.extend(start + index for index in [0, 1, 2, 0, 2, 3])
    write("bank", positions, normals, uvs, indices)


def hill():
    side, half = 65, 340

    def height(x, z):
        taper = max(0, 1 - (x / half) ** 2) * max(0, 1 - (z / half) ** 2)
        ridge = 105 * math.exp(-((x - 60) / 150) ** 2 - ((z + 60) / 160) ** 2)
        spur = 65 * math.exp(-((x + 125) / 110) ** 2 - ((z - 50) / 160) ** 2)
        return 1.55 * taper * (ridge + spur) * (1 + 0.08 * math.sin(x / 41 + z / 77))

    points, normals, uvs, indices = [], [], [], []
    for j in range(side):
        for i in range(side):
            x, z = [(v / (side - 1) * 2 - 1) * half for v in [i, j]]
            points.append((x, height(x, z), z))
            n = (height(x - 1, z) - height(x + 1, z), 2, height(x, z - 1) - height(x, z + 1))
            normals.append(tuple(v / math.hypot(*n) for v in n))
            uvs.append((i / (side - 1), j / (side - 1)))
            if i + 1 < side and j + 1 < side:
                a = j * side + i
                indices.extend([a, a + side, a + 1, a + 1, a + side, a + side + 1])
    write("hill", points, normals, uvs, indices)


def write(name, positions, normals, uvs, indices):
    binary = bytearray()
    views = []
    accessors = []
    for rows, fmt, kind, component, target in [
        (positions, "3f", "VEC3", 5126, 34962),
        (normals, "3f", "VEC3", 5126, 34962),
        (uvs, "2f", "VEC2", 5126, 34962),
        ([(i,) for i in indices], "H", "SCALAR", 5123, 34963),
    ]:
        data = b"".join(struct.pack("<" + fmt, *row) for row in rows)
        accessors.append(dict(bufferView=len(views), componentType=component, count=len(rows), type=kind))
        views.append(dict(buffer=0, byteOffset=len(binary), byteLength=len(data), target=target))
        binary.extend(data)
    accessors[0].update(min=[min(axis) for axis in zip(*positions)],
                        max=[max(axis) for axis in zip(*positions)])
    gltf = dict(
        asset=dict(version="2.0", generator="mirror pool mesh recipe"),
        scene=0, scenes=[dict(nodes=[0])], nodes=[dict(mesh=0, name=name)],
        meshes=[dict(primitives=[dict(attributes=dict(POSITION=0, NORMAL=1, TEXCOORD_0=2), indices=3)])],
        buffers=[dict(uri=f"{name}.bin", byteLength=len(binary))],
        bufferViews=views, accessors=accessors,
    )
    output = Path(__file__).parent / "assets" / "meshes"
    (output / f"{name}.bin").write_bytes(binary)
    (output / f"{name}.gltf").write_text(json.dumps(gltf, separators=(",", ":")) + "\n")


if __name__ == "__main__":
    main()
