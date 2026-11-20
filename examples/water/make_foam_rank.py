#!/usr/bin/env python3
"""Regenerate the water example's original ranked cellular foam texture."""
from math import hypot, sin, tau
from pathlib import Path
from random import Random
from struct import pack
from zlib import compress, crc32


def chunk(tag, value):
    return pack("!I", len(value)) + tag + value + pack("!I", crc32(tag + value))


def make(size):
    random = Random(4160)
    sites = [(random.random(), random.random()) for _ in range(72)]
    values = []
    for y in range(size):
        for x in range(size):
            u, v = x / size, y / size
            u, v = u + 0.013 * sin(tau * (3 * v + u)), v + 0.011 * sin(tau * (3 * u + v))
            near = sorted(
                hypot((u - a + 0.5) % 1 - 0.5, (v - b + 0.5) % 1 - 0.5)
                for a, b in sites
            )
            values.append(near[1] - near[0] + random.random() * 0.004)
    rank = [0] * len(values)
    for tier, pixel in enumerate(sorted(range(len(values)), key=values.__getitem__)):
        rank[pixel] = tier * 255 // (len(values) - 1)
    pixels = bytearray()
    for row in range(size):
        pixels.append(0)
        for column in range(size):
            shade = rank[row * size + column]
            pixels.extend([shade] * 4)
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", pack("!IIBBBBB", size, size, 8, 6, 0, 0, 0))
        + chunk(b"IDAT", compress(pixels, 9))
        + chunk(b"IEND", b"")
    )


if __name__ == "__main__":
    Path(__file__).with_name("assets").joinpath("textures/foam-rank.png").write_bytes(make(128))
