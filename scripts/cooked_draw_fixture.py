"""Shared deterministic cooked-draw qualification source and toolchain setup."""
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def environment():
    env = dict(os.environ, RUSTC_WRAPPER='')
    rustc = subprocess.check_output(['rustup', 'which', '--toolchain', '1.99.1', 'rustc'], text=True).strip()
    env.update(RUSTC=rustc, PATH=str(Path(rustc).parent) + os.pathsep + env['PATH'])
    return env


def sources(folder, env, cooker=None):
    """One source/store contains the valley and the explicitly selected diagnostic fixture."""
    command = [str(cooker)] if cooker else ['cargo', 'run', '--locked', '--offline', '-p', 'thrum-cook', '--']
    subprocess.run([*command, 'demo', str(folder)], env=env, check=True)
    for source in (ROOT / 'examples/v1-game/assets/showcase').iterdir():
        if source.is_file() and source.suffix in ['.gltf', '.bin', '.png', '.json']:
            shutil.copyfile(source, folder / source.name)
    mesh = json.loads((folder / 'meshes/terrain.gltf').read_text())
    raw = bytearray((folder / 'meshes/terrain.bin').read_bytes())
    # Reflect source positions/normals and winding, then mirror the node back.
    # The cooker must correct that node's winding and normal transform.
    primitive = mesh['meshes'][0]['primitives'][0]
    for semantic in ['POSITION', 'NORMAL']:
        if semantic not in primitive['attributes']:
            continue
        accessor = mesh['accessors'][primitive['attributes'][semantic]]
        view = mesh['bufferViews'][accessor['bufferView']]
        offset = view.get('byteOffset', 0) + accessor.get('byteOffset', 0)
        stride = view.get('byteStride', 12)
        for index in range(accessor['count']):
            at = offset + index * stride
            struct.pack_into('<f', raw, at, -struct.unpack_from('<f', raw, at)[0])
    accessor = mesh['accessors'][primitive['indices']]
    view = mesh['bufferViews'][accessor['bufferView']]
    offset = view.get('byteOffset', 0) + accessor.get('byteOffset', 0)
    fmt = {5123: 'H', 5125: 'I'}[accessor['componentType']]
    size = struct.calcsize(fmt)
    for index in range(0, accessor['count'], 3):
        at = offset + index * size
        a, b, c = struct.unpack_from('<' + fmt * 3, raw, at)
        struct.pack_into('<' + fmt * 3, raw, at, a, c, b)
    mesh['buffers'][0]['uri'] = 'mirror.bin'
    mesh['nodes'][0]['scale'] = [-1, 1, 1]
    (folder / 'meshes/mirror.bin').write_bytes(raw)
    (folder / 'meshes/mirror.gltf').write_text(json.dumps(mesh, indent=2) + '\n')


def cook(folder, store, env, cooker=None):
    command = [str(cooker)] if cooker else ['cargo', 'run', '--locked', '--offline', '-p', 'thrum-cook', '--']
    subprocess.run([*command, 'cook', str(folder), str(store)], env=env, check=True)


def placements(scene):
    if scene == 'diagnostic':
        return ['meshes/terrain', 'meshes/terrain', 'meshes/mirror']
    recipe = json.loads((ROOT / 'examples/v1-game/assets/showcase/scene.json').read_text())
    return [p['mesh'].removesuffix('.gltf') for p in recipe['placements']]
