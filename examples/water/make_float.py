#!/usr/bin/env python3
"""Regenerate the water scene's floats and white material texel."""
import base64
import json
import math
from pathlib import Path
import struct
import zlib

ROOT = Path(__file__).resolve().parent / 'assets'


def write_float(name, rings, sectors, half_height):
    positions, normals, uvs, indices = [], [], [], []
    for ring in range(rings + 1):
        latitude = math.pi * ring / rings
        for sector in range(sectors + 1):
            longitude = 2 * math.pi * sector / sectors
            normal = (math.sin(latitude) * math.cos(longitude), math.cos(latitude),
                      math.sin(latitude) * math.sin(longitude))
            positions.append([0.5 * normal[0], half_height * normal[1], 0.5 * normal[2]])
            if half_height != 0.5:
                normal = (normal[0], normal[1] * 0.5 / half_height, normal[2])
                magnitude = math.hypot(*normal)
                normal = tuple(value / magnitude for value in normal)
            normals.append(normal)
            uvs.append((sector / sectors, ring / rings))
    for ring in range(rings):
        for sector in range(sectors):
            a = ring * (sectors + 1) + sector
            b = a + sectors + 1
            if ring > 0:
                indices.extend([a, a + 1, b])
            if ring < rings - 1:
                indices.extend([a + 1, b + 1, b])
    data = bytearray()
    views, accessors = [], []
    for values, fmt, kind, component in [(positions,'3f','VEC3',5126),(normals,'3f','VEC3',5126),(uvs,'2f','VEC2',5126),([(i,) for i in indices],'H','SCALAR',5123)]:
        offset = len(data)
        for value in values:
            data.extend(struct.pack('<' + fmt, *value))
        views.append({'buffer':0,'byteOffset':offset,'byteLength':len(data)-offset})
        accessor = {'bufferView':len(views)-1,'componentType':component,'count':len(values),'type':kind}
        if len(views) == 1:
            accessor.update(min=[-0.5, -half_height, -0.5], max=[0.5, half_height, 0.5])
        accessors.append(accessor)
    model = {'asset':{'version':'2.0'},'buffers':[{'byteLength':len(data),'uri':'data:application/octet-stream;base64,'+base64.b64encode(data).decode()}],'bufferViews':views,'accessors':accessors,'meshes':[{'primitives':[{'attributes':{'POSITION':0,'NORMAL':1,'TEXCOORD_0':2},'indices':3}]}],'nodes':[{'mesh':0}],'scenes':[{'nodes':[0]}],'scene':0}
    (ROOT/'meshes').mkdir(parents=True, exist_ok=True)
    (ROOT/'meshes'/f'{name}.gltf').write_text(json.dumps(model, indent=2)+'\n')


def main():
    # The sphere exposes angular illumination for the calibration scene.
    write_float('float', 32, 64, 0.5)
    # The sunk float presents a broad sun-facing crown and curved shoulders.
    write_float('sunk-float', 16, 48, 0.22)
    def chunk(kind, payload):
        return struct.pack('>I',len(payload))+kind+payload+struct.pack('>I',zlib.crc32(kind+payload))
    png=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',1,1,8,6,0,0,0))+chunk(b'IDAT',zlib.compress(b'\0\xff\xff\xff\xff'))+chunk(b'IEND',b'')
    (ROOT/'textures').mkdir(exist_ok=True)
    (ROOT/'textures/white.png').write_bytes(png)


if __name__ == '__main__':
    main()
