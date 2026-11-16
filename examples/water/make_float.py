#!/usr/bin/env python3
"""Regenerate this scene's grey calibration float and white material texel."""
import base64
import json
from pathlib import Path
import struct
import zlib

ROOT = Path(__file__).resolve().parent / 'assets'


def main():
    positions, normals, uvs, indices = [], [], [], []
    for normal, corners in [
        ((1, 0, 0), [(1,-1,-1),(1,1,-1),(1,1,1),(1,-1,1)]),
        ((-1, 0, 0), [(-1,-1,1),(-1,1,1),(-1,1,-1),(-1,-1,-1)]),
        ((0, 1, 0), [(-1,1,-1),(-1,1,1),(1,1,1),(1,1,-1)]),
        ((0, -1, 0), [(-1,-1,1),(-1,-1,-1),(1,-1,-1),(1,-1,1)]),
        ((0, 0, 1), [(-1,-1,1),(1,-1,1),(1,1,1),(-1,1,1)]),
        ((0, 0, -1), [(1,-1,-1),(-1,-1,-1),(-1,1,-1),(1,1,-1)]),
    ]:
        start = len(positions)
        positions.extend([[x * .5 for x in p] for p in corners])
        normals.extend([normal] * 4)
        uvs.extend([(0,0),(0,1),(1,1),(1,0)])
        indices.extend(start + x for x in [0,1,2,0,2,3])
    data = bytearray()
    views, accessors = [], []
    for values, fmt, kind, component in [(positions,'3f','VEC3',5126),(normals,'3f','VEC3',5126),(uvs,'2f','VEC2',5126),([(i,) for i in indices],'H','SCALAR',5123)]:
        offset = len(data)
        for value in values:
            data.extend(struct.pack('<' + fmt, *value))
        views.append({'buffer':0,'byteOffset':offset,'byteLength':len(data)-offset})
        accessor = {'bufferView':len(views)-1,'componentType':component,'count':len(values),'type':kind}
        if len(views) == 1:
            accessor.update(min=[-.5]*3, max=[.5]*3)
        accessors.append(accessor)
    model = {'asset':{'version':'2.0'},'buffers':[{'byteLength':len(data),'uri':'data:application/octet-stream;base64,'+base64.b64encode(data).decode()}],'bufferViews':views,'accessors':accessors,'meshes':[{'primitives':[{'attributes':{'POSITION':0,'NORMAL':1,'TEXCOORD_0':2},'indices':3}]}],'nodes':[{'mesh':0}],'scenes':[{'nodes':[0]}],'scene':0}
    (ROOT/'meshes').mkdir(parents=True, exist_ok=True)
    (ROOT/'meshes/float.gltf').write_text(json.dumps(model, indent=2)+'\n')
    def chunk(kind, payload):
        return struct.pack('>I',len(payload))+kind+payload+struct.pack('>I',zlib.crc32(kind+payload))
    png=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',1,1,8,6,0,0,0))+chunk(b'IDAT',zlib.compress(b'\0\xff\xff\xff\xff'))+chunk(b'IEND',b'')
    (ROOT/'textures').mkdir(exist_ok=True)
    (ROOT/'textures/white.png').write_bytes(png)


if __name__ == '__main__':
    main()
