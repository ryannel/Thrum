#!/usr/bin/env python3
"""Write the original moving-field art and the two atomic-save PNGs."""

import json
from pathlib import Path
import struct
import sys
import zlib

COLOURS = ((25, 183, 238), (240, 114, 31), (221, 48, 216))
SIDE = 24


def png(width, height, pixel):
    def chunk(kind, data):
        body = kind + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body))

    rows = [b"\0" + bytes(channel for x in range(width) for channel in pixel(x, y))
            for y in range(height)]
    header = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header)
            + chunk(b"IDAT", zlib.compress(b"".join(rows))) + chunk(b"IEND", b""))


def actor(x, y, phase):
    frame, x = divmod(x, SIDE)
    repacked = phase == 2
    lift = frame % 2
    head = 5 <= x <= 18 and 3 + lift <= y <= 16
    shoe = 18 <= y <= 21 and (6 <= x <= 9 or 14 <= x <= 17)
    fin = repacked and 0 <= x <= 23 and 12 <= y <= 15
    if not (head or shoe or fin):
        return (0, 0, 0, 0)
    if 6 <= y <= 8 and x in (9, 10, 14, 15):
        return (251, 240, 162, 255)
    if shoe or x in (5, 18) or y in (3 + lift, 16):
        return (18, 22, 38, 255)
    return (*COLOURS[phase], 255)


def prepare(source, edits):
    field = source / "field"
    control = source / "unrelated"
    for folder in (field, control, edits):
        folder.mkdir(parents=True, exist_ok=True)
    (field / "sheet.json").write_text(json.dumps({
        "kind": "pixel", "density": 540, "page_size": 64,
        "grids": {"walk.png": [SIDE, SIDE]},
    }) + "\n")
    (control / "sheet.json").write_text('{"kind":"pixel","density":1}\n')
    (control / "control.png").write_bytes(png(2, 2, lambda x, y: (255, 255, 255, 255)))
    for path, phase in ((field / "walk.png", 0), (edits / "recolour.png", 1),
                        (edits / "repack.png", 2)):
        path.write_bytes(png(3 * SIDE, SIDE, lambda x, y: actor(x, y, phase)))


if __name__ == "__main__":
    prepare(*map(Path, sys.argv[1:]))
