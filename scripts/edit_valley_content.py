#!/usr/bin/env python3
"""Prepare and edit the generated inputs used by the live valley preview."""
import argparse
import json
import struct
import zlib
from pathlib import Path


def save(path, data):
    staging = path.with_name("." + path.name + ".save")
    staging.write_bytes(data)
    staging.replace(path)


def ground():
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))

    size = 512
    colors = [bytes((78, 184, 203)), bytes((221, 185, 98))]
    strips = [b"\0" + (color * 32 + colors[1 - i] * 32) * 8 for i, color in enumerate(colors)]
    rows = b"".join(strips[y // 32 % 2] for y in range(size))
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", size, size, 8, 2, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(rows))
        + chunk(b"IEND", b"")
    )


def volume(size, edited):
    plane = bytes(
        (220 if edited else 32) + (x * 17 // size + y * 19 // size)
        for y in range(size)
        for x in range(size)
    )
    return b"TVOL" + struct.pack("<III", size, size, size) + plane * size


def edit(source, operation, size=128):
    if operation == "mesh":
        path = source / "rock.gltf"
        mesh = json.loads(path.read_bytes())
        mesh["nodes"][0]["scale"] = [1.0, 2.5, 1.0]
        save(path, json.dumps(mesh).encode())
    elif operation == "texture":
        save(source / "ground.png", ground())
    else:
        save(source / "preview-cloud.tvol", volume(size, operation == "volume"))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("operation", choices=["prepare", "mesh", "texture", "volume"])
    parser.add_argument("--volume-size", type=int, default=128)
    args = parser.parse_args()
    edit(args.source, args.operation, args.volume_size)
