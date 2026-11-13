"""Design arithmetic witness only; does not execute the codec or engine."""

import math
import struct


def f32(value):
    return struct.unpack("<f", struct.pack("<f", value))[0]


def bits(value):
    return struct.pack("<f", value).hex()


def quantize(value):
    scaled = value * 32767.0
    return math.floor(scaled + 0.5) if scaled >= 0 else math.ceil(scaled - 0.5)


def canonical(value):
    return f32(quantize(value) / 32767.0)


for integer in range(-32767, 32768):
    decoded = f32(integer / 32767.0)
    assert quantize(decoded) == integer
    assert bits(canonical(decoded)) == bits(decoded)
print("PASS: all 65,535 admitted wire integers re-encode to themselves")

boundary_count = 0
for lower in range(-32767, 32767):
    boundary = (lower + 0.5) / 32767.0
    nearest = f32(boundary)
    encoding = struct.unpack("<I", struct.pack("<f", nearest))[0]
    values = [struct.unpack("<f", struct.pack("<I", encoding + delta))[0]
              for delta in (-1, 0, 1)]
    below = max(value for value in values if value < boundary)
    above = min(value for value in values if value > boundary)
    assert quantize(below) == lower
    assert quantize(above) == lower + 1
    for value in (below, above):
        assert bits(canonical(value)) == bits(canonical(canonical(value)))
    boundary_count += 1
print(f"PASS: adjacent binary32 values on both sides of {boundary_count:,} boundaries")

for value in (-1.0, -0.5, -0.0, 0.0, 0.1, 0.5, 1.0):
    local = f32(value)
    decoded = canonical(local)
    assert bits(decoded) == bits(canonical(decoded))
    print(f"axis {value!r}: local={local!r}, decoded={decoded!r}, bits={bits(decoded)}")
