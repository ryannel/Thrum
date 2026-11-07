"""Generate assets/meshes/lighthouse.gltf and .bin with the Python standard library only.

A tapered tower (12 m), a gallery deck, a lantern room and a conical cap, as one primitive with
POSITION, NORMAL, TEXCOORD_0 and u16 indices, origin at the tower's base centre, +Y up.
Run: python3 examples/lighthouse/generate.py
"""
import json, math, struct, pathlib

SEG = 40
pos, nor, uv, idx = [], [], [], []


def ring_band(r0, y0, r1, y1, v0, v1):
    """Side of a frustum from (r0, y0) to (r1, y1), smooth normals, outward."""
    dr, dy = r1 - r0, y1 - y0
    ln = math.hypot(dr, dy)
    ny, nr = -dr / ln, dy / ln  # outward normal in (r, y)
    base = len(pos)
    for i in range(SEG + 1):
        a = 2 * math.pi * i / SEG
        c, s = math.cos(a), math.sin(a)
        for r, y, v in ((r0, y0, v0), (r1, y1, v1)):
            pos.append((r * c, y, r * s))
            nor.append((nr * c, ny, nr * s))
            uv.append((i / SEG, v))
    for i in range(SEG):
        a, b, c2, d = base + 2 * i, base + 2 * i + 1, base + 2 * i + 2, base + 2 * i + 3
        idx.extend((a, b, c2, c2, b, d))  # counter-clockwise seen from outside


def disc(r, y, up):
    """Flat cap at height y, facing +Y if up else -Y."""
    base = len(pos)
    pos.append((0.0, y, 0.0)); nor.append((0.0, 1.0 if up else -1.0, 0.0)); uv.append((0.5, 0.5))
    for i in range(SEG + 1):
        a = 2 * math.pi * i / SEG
        pos.append((r * math.cos(a), y, r * math.sin(a)))
        nor.append((0.0, 1.0 if up else -1.0, 0.0))
        uv.append((0.5 + 0.5 * math.cos(a), 0.5 + 0.5 * math.sin(a)))
    for i in range(SEG):
        p, q = base + 1 + i, base + 2 + i
        idx.extend((base, q, p) if up else (base, p, q))


ring_band(1.7, 0.0, 1.05, 12.0, 0.0, 0.8)   # tower
disc(1.75, 12.0, False)                     # gallery underside
ring_band(1.75, 12.0, 1.75, 12.25, 0.8, 0.82)  # gallery rim
disc(1.75, 12.25, True)                     # gallery deck
ring_band(0.85, 12.25, 0.85, 13.9, 0.82, 0.93)  # lantern room
disc(1.0, 13.9, False)                      # cap underside
ring_band(1.0, 13.9, 0.05, 15.1, 0.93, 1.0)  # conical cap

bin_ = bytearray()
views = []
for data, fmt in ((pos, "3f"), (nor, "3f"), (uv, "2f")):
    off = len(bin_)
    for t in data:
        bin_ += struct.pack("<" + fmt, *t)
    views.append((off, len(bin_) - off, 34962))
off = len(bin_)
for i in idx:
    bin_ += struct.pack("<H", i)
views.append((off, len(bin_) - off, 34963))
while len(bin_) % 4:
    bin_ += b"\0"

mins = [min(p[k] for p in pos) for k in range(3)]
maxs = [max(p[k] for p in pos) for k in range(3)]
gltf = {
    "asset": {"version": "2.0", "generator": "thrum-lighthouse stdlib recipe v1"},
    "scene": 0,
    "scenes": [{"nodes": [0]}],
    "nodes": [{"mesh": 0, "name": "lighthouse"}],
    "meshes": [{"name": "lighthouse", "primitives": [{"attributes": {"POSITION": 0, "NORMAL": 1, "TEXCOORD_0": 2}, "indices": 3, "material": 0, "mode": 4}]}],
    "materials": [{"name": "lighthouse", "pbrMetallicRoughness": {"baseColorFactor": [0.9, 0.9, 0.88, 1], "metallicFactor": 0, "roughnessFactor": 0.6}}],
    "buffers": [{"uri": "lighthouse.bin", "byteLength": len(bin_)}],
    "bufferViews": [{"buffer": 0, "byteOffset": o, "byteLength": n, "target": t} for o, n, t in views],
    "accessors": [
        {"bufferView": 0, "componentType": 5126, "count": len(pos), "type": "VEC3", "min": mins, "max": maxs},
        {"bufferView": 1, "componentType": 5126, "count": len(nor), "type": "VEC3"},
        {"bufferView": 2, "componentType": 5126, "count": len(uv), "type": "VEC2"},
        {"bufferView": 3, "componentType": 5123, "count": len(idx), "type": "SCALAR"},
    ],
}
out = pathlib.Path(__file__).parent / "assets" / "meshes"
(out / "lighthouse.bin").write_bytes(bin_)
(out / "lighthouse.gltf").write_text(json.dumps(gltf, separators=(",", ":")))
print(len(pos), "vertices,", len(idx) // 3, "triangles")
