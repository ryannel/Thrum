#!/usr/bin/env python3
"""Generated CONTENT-LOOP browser inputs; no external assets or Python packages."""
import argparse
from pathlib import Path
import struct
import zlib


def volume(path, seed):
    size = 128
    data = bytes((x + y + z + seed) % 256
                 for z in range(size) for y in range(size) for x in range(size))
    path.write_bytes(b"TVOL" + struct.pack("<III", size, size, size) + data)


def generate(source, edit):
    source.mkdir(parents=True, exist_ok=True)
    if edit:
        volume(source / "cloud0.tvol", 97)
        return
    for index in range(12):
        volume(source / f"cloud{index}.tvol", index)
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
    pixels = b"".join(b"\0" + bytes([200, 40, 20, 255]) * 32 for _ in range(32))
    png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", 32, 32, 8, 6, 0, 0, 0))
    (source / "foam.linear.png").write_bytes(png + chunk(b"IDAT", zlib.compress(pixels)) + chunk(b"IEND", b""))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("--edit", action="store_true")
    args = parser.parse_args()
    generate(args.source, args.edit)
