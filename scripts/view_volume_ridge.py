"""Cookable valley variant with rock openings for the fixed shaft inspection."""
import importlib.util
import json
from pathlib import Path
import shutil


# Two pillars and a cap make the lower opening. A fourth rock forms a saddle
# against the south pillar so the ridge case has an open crest as well.
ROCK_PLACEMENTS = (
    ((10, 112, 127), (110, 188, 95)),
    ((-30, 111, 358), (120, 196, 112)),
    ((-17, 227, 243), (103, 61, 165)),
    ((-59, 119, 487), (120, 161, 103)),
)


def ridge_source(valley: Path, destination: Path) -> Path:
    """Add opaque casters to a diagnostic copy of the shipped valley."""
    shutil.copytree(valley, destination)
    spec = importlib.util.spec_from_file_location('valley_geometry', destination / 'generate.py')
    geometry = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(geometry)
    vertices, normals, uvs, triangles = geometry.terrain()
    rock, rock_normals, _, rock_triangles = geometry.round_mesh(16, 12, rock=True)
    for centre, scale in ROCK_PLACEMENTS:
        base = len(vertices)
        for position, normal in zip(rock, rock_normals, strict=True):
            x, y, z = (c + s * v for c, s, v in zip(centre, scale, position, strict=True))
            vertices.append((x, y, z))
            normals.append(geometry.unit(tuple(v / s for v, s in zip(normal, scale, strict=True))))
            uvs.append(geometry.ground_uv(x + 0.641 * y, z))
        triangles.extend(tuple(base + index for index in triangle) for triangle in rock_triangles)
    material = json.loads((destination / 'terrain.gltf').read_text())['materials'][0]
    geometry.write_mesh('terrain', (vertices, normals, uvs, triangles), material['pbrMetallicRoughness'])
    return destination
