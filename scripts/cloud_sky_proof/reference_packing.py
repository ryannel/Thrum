"""Independent finite CPU vectors for CLOUDS' logical four-Z half packing.

No renderer imports. Coordinates are logical voxel boundary space, x is fastest,
then y, then z; RGBA channels hold consecutive logical Z coefficients. Mips use
ceil-halved dimensions and eight logical parents, including zero outside support.
This qualifies arithmetic/layout only, never a cloud family or optical LOD.
"""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
import struct
from typing import Iterable

SCHEMA = "thrum.cloud-packing-reference.v1"
COEFFICIENT_MAX = 0.1
Dimensions = tuple[int, int, int]


def _nearest_even(numerator: int, denominator: int) -> int:
    quotient, remainder = divmod(numerator, denominator)
    twice = 2 * remainder
    return quotient + int(twice > denominator or (twice == denominator and quotient % 2))


def half_bits(value: float) -> int:
    """Round an exact binary64 input directly to binary16, ties to even.

    Integer-ratio rounding avoids relying on a GPU, NumPy or native half packing.
    Nonfinite/overflowing values are rejected rather than silently becoming data.
    """
    value = float(value)
    if not math.isfinite(value):
        raise ValueError("half store requires a finite value")
    sign = 0x8000 if math.copysign(1.0, value) < 0 else 0
    numerator, denominator = abs(value).as_integer_ratio()
    if numerator == 0:
        return sign
    exponent = numerator.bit_length() - denominator.bit_length()
    if exponent >= 0:
        exponent -= int(numerator < denominator << exponent)
    else:
        exponent -= int(numerator << -exponent < denominator)
    scale = 24 if exponent < -14 else 10 - exponent
    scaled_numerator = numerator << max(scale, 0)
    scaled_denominator = denominator << max(-scale, 0)
    significand = _nearest_even(scaled_numerator, scaled_denominator)
    if exponent < -14:
        return sign | significand
    if significand == 2048:
        significand = 1024
        exponent += 1
    if exponent > 15:
        raise OverflowError("half store overflow")
    return sign | ((exponent + 15) << 10) | (significand - 1024)


def half_value(bits: int) -> float:
    """Decode finite binary16 bits without using the platform's half decoder."""
    if not 0 <= bits <= 0xFFFF:
        raise ValueError("half bits outside u16")
    exponent = (bits >> 10) & 31
    fraction = bits & 1023
    if exponent == 31:
        raise ValueError("nonfinite half coefficient")
    value = math.ldexp(fraction, -24) if exponent == 0 else math.ldexp(1024 + fraction, exponent - 25)
    return -value if bits & 0x8000 else value


def validate_dimensions(dimensions: Iterable[int]) -> Dimensions:
    dimensions = tuple(dimensions)
    if len(dimensions) != 3 or any(type(n) is not int or n < 1 for n in dimensions):
        raise ValueError("three positive integer logical dimensions required")
    return dimensions


def coefficient_bits(value: float) -> int:
    if not math.isfinite(value) or not 0 <= value <= COEFFICIENT_MAX:
        raise ValueError("coefficient outside admitted finite 0..0.1 m^-1 range")
    return half_bits(value)


def dense_index(dimensions: Dimensions, x: int, y: int, z: int) -> int:
    nx, ny, _ = dimensions
    return (z * ny + y) * nx + x


def logical_value(bits: list[int], dimensions: Dimensions, x: int, y: int, z: int) -> float:
    if any(i < 0 or i >= n for i, n in zip((x, y, z), dimensions)):
        return 0.0
    return half_value(bits[dense_index(dimensions, x, y, z)])


def point_coefficient(bits: list[int], dimensions: Dimensions, position: Iterable[float]) -> float:
    """Trilinearly reconstruct level zero or an explicitly chosen logical level.

    This does not infer an optical footprint or allow point tau queries to pick a
    coarser mip. All eight logical corner coefficients use zero outside support.
    """
    position = tuple(position)
    if len(position) != 3 or not all(math.isfinite(v) for v in position):
        raise ValueError("three finite boundary-space coordinates required")
    q = [v - 0.5 for v in position]
    lower = [math.floor(v) for v in q]
    fraction = [v - lo for v, lo in zip(q, lower)]
    terms = []
    for dz in range(2):
        for dy in range(2):
            for dx in range(2):
                weight = math.prod(f if offset else 1 - f for f, offset in zip(fraction, (dx, dy, dz)))
                terms.append(weight * logical_value(bits, dimensions, lower[0] + dx, lower[1] + dy, lower[2] + dz))
    return math.fsum(terms)


def logical_mip(bits: list[int], dimensions: Dimensions) -> tuple[list[int], Dimensions]:
    output_dimensions = tuple((n + 1) // 2 for n in dimensions)
    nx, ny, nz = output_dimensions
    output = []
    for z in range(nz):
        for y in range(ny):
            for x in range(nx):
                parents = [logical_value(bits, dimensions, 2*x+dx, 2*y+dy, 2*z+dz)
                           for dz in range(2) for dy in range(2) for dx in range(2)]
                output.append(half_bits(math.fsum(parents) / 8))
    return output, output_dimensions


def packed_dimensions(dimensions: Dimensions) -> Dimensions:
    nx, ny, nz = dimensions
    return nx + 2, ny + 2, (nz + 3) // 4 + 2


def pack_rgba_half(bits: list[int], dimensions: Dimensions) -> bytes:
    """Tight x-fastest little-endian RGBA texels, one guard on every axis.

    A Z guard is a whole four-channel group; unused channels of a last partial
    group are zero. WebGPU row-pitch padding is a harness transfer concern and
    is deliberately absent from this portable expected texture-content payload.
    """
    nx, ny, nz = validate_dimensions(dimensions)
    if len(bits) != nx * ny * nz:
        raise ValueError("logical coefficient count does not match dimensions")
    px, py, pz = packed_dimensions(dimensions)
    result = bytearray(px * py * pz * 8)
    for z in range(nz):
        group, channel = divmod(z, 4)
        for y in range(ny):
            for x in range(nx):
                texel = ((group + 1) * py + y + 1) * px + x + 1
                struct.pack_into("<H", result, texel * 8 + channel * 2, bits[dense_index(dimensions, x, y, z)])
    return bytes(result)


def packed_logical_value(payload: bytes, dimensions: Dimensions, x: int, y: int, z: int) -> float:
    """Decode the guarded packing, including mathematical-floor negative Z."""
    px, py, pz = packed_dimensions(dimensions)
    group, channel = divmod(z, 4)
    gx, gy, gz = x + 1, y + 1, group + 1
    if not (0 <= gx < px and 0 <= gy < py and 0 <= gz < pz):
        return 0.0
    offset = ((gz * py + gy) * px + gx) * 8 + 2 * channel
    return half_value(struct.unpack_from("<H", payload, offset)[0])


def constant_slab(sigma: float, length: float, source_per_m: float = 0.002, incoming: float = 2.0) -> dict:
    """Analytic constant segment L_out=T*L_in+S; optical units stay explicit."""
    if any(not math.isfinite(v) or v < 0 for v in (sigma, length, source_per_m, incoming)):
        raise ValueError("slab inputs must be finite and nonnegative")
    tau = sigma * length
    transmittance = math.exp(-tau)
    source = source_per_m * length if sigma == 0 else source_per_m * (-math.expm1(-tau)) / sigma
    return {"sigma_per_m": sigma, "length_m": length, "source_radiance_per_m": source_per_m,
            "incoming_radiance": incoming, "tau": tau, "T": transmittance, "S": source,
            "outgoing_radiance": transmittance * incoming + source}


def _pattern(dimensions: Dimensions, pattern: str) -> list[int]:
    nx, ny, nz = dimensions
    result = []
    for z in range(nz):
        for y in range(ny):
            for x in range(nx):
                if pattern == "ramp":
                    value = (1 + x + 3*y + 7*z) / 16384
                elif pattern == "impulse":
                    value = 0.0625 if (x, y, z) == (nx//2, ny//2, nz//2) else 0.0
                elif pattern == "clear_cloud_edge":
                    value = 0.05 if x < (nx + 1)//2 and z < (nz + 1)//2 else 0.0
                elif pattern == "rounding_store_edges":
                    if dimensions != (4, 4, 5):
                        raise ValueError("rounding-store fixture is defined on 4x4x5")
                    # Every 2x2x2 block has four copies of each endpoint. Their
                    # mean hits a precise half midpoint, exercising mip stores.
                    pairs = ((0x0000, 0x0001), (0x0001, 0x0002),
                             (0x2C00, 0x2C01), (0x2C01, 0x2C02),
                             (0x03FF, 0x0400), (0x0001, 0x0001),
                             (0x0000, 0x2E66), (0x03FE, 0x03FF))
                    if z < 4:
                        pair = pairs[(z//2)*4 + (y//2)*2 + x//2]
                        value = half_value(pair[x % 2])
                    else:
                        # Odd final slice checks zero-extended parents; include
                        # minimum subnormals crossing the logical Z group edge.
                        value = half_value((1, 2, 3, 4)[x])
                else:
                    raise ValueError(f"unknown fixture pattern: {pattern}")
                result.append(coefficient_bits(value))
    return result


def _sample_positions(dimensions: Dimensions) -> list[tuple[float, float, float]]:
    nx, ny, nz = dimensions
    positions = {(0.5, 0.5, z+0.5) for z in range(nz)}
    positions.update({(nx/2, ny/2, nz/2), (nx-0.5, ny-0.5, nz-0.5),
                      (0.5, 0.5, 3.5), (0.5, 0.5, 3.75), (0.5, 0.5, 4.0),
                      (0.5, 0.5, 4.25), (0.5, 0.5, 4.5), (1.25, 2.125, 4.25),
                      (0.125, 0.8125, 3.9375)})
    for axis, extent in enumerate(dimensions):
        for coordinate in (-0.75, -0.25, 0.0, 0.25, 0.5, extent-0.5, extent, extent+0.25, extent+0.75):
            point = [0.5, 0.5, 0.5]
            point[axis] = coordinate
            positions.add(tuple(point))
    return sorted(positions)


def half_rounding_vectors() -> list[dict]:
    inputs = [0.0, -0.0, 2**-25, 2**-24, 3*2**-25, 1023*2**-24,
              1023.5*2**-24, 2**-14, 0.0625, 0.0625+2**-15,
              0.0625+3*2**-15, 0.1,
              0.0625+2**-15-2**-27, 0.0625+2**-15+2**-27,
              2**-25-2**-48, 2**-25+2**-48]
    return [{"input": value, "input_binary64_hex": value.hex(), "half_bits": half_bits(value),
             "half_value": half_value(half_bits(value))} for value in inputs]


def reference_vectors() -> dict:
    fixtures = []
    for dimensions in ((7, 5, 9), (1, 1, 1), (4, 3, 4), (4, 4, 5)):
        patterns = ["ramp", "impulse", "clear_cloud_edge"]
        if dimensions == (4, 4, 5):
            patterns.append("rounding_store_edges")
        for pattern in patterns:
            bits = _pattern(dimensions, pattern)
            current_dimensions = dimensions
            levels = []
            for level in range(3):
                samples = []
                for position in _sample_positions(current_dimensions):
                    coefficient = point_coefficient(bits, current_dimensions, position)
                    samples.append({"boundary_xyz": position, "coefficient": coefficient,
                                    "stored_half_bits": half_bits(coefficient),
                                    "stored_half_value": half_value(half_bits(coefficient))})
                payload = pack_rgba_half(bits, current_dimensions)
                levels.append({"level": level, "logical_dimensions_xyz": current_dimensions,
                               "dense_half_bits": bits, "physical_dimensions_xyz": packed_dimensions(current_dimensions),
                               "rgba_half_le_hex": payload.hex(), "samples": samples})
                bits, current_dimensions = logical_mip(bits, current_dimensions)
            fixtures.append({"id": f"{pattern}-{'x'.join(map(str, dimensions))}", "pattern": pattern,
                             "dimensions_xyz": dimensions, "levels": levels})
    return {"schema": SCHEMA, "contract": {"ordering": "x-fastest, then y, then z, then RGBA channel",
            "coordinate_space": "logical boundary; voxel i centre = i+0.5",
            "guard_xyz": [1, 1, 1], "guard_z_unit": "packed group of four logical cells",
            "mip_dimensions": "ceil(parent/2), eight parents with zero outside support",
            "levels": 3, "byte_order": "little-endian", "row_pitch": "tight; no transfer padding",
            "point_optics_level": 0, "filtered_mips": "coefficient-only, no optical LOD acceptance claim",
            "half_store": "binary16 nearest/ties-even; expected coefficient float is before output-half store"},
            "rounding": half_rounding_vectors(), "fixtures": fixtures,
            "constant_slabs": [constant_slab(sigma, length) for sigma in (0.0, 2**-14, half_value(half_bits(0.05)))
                               for length in (0.0, 1.0, 100.0)]}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, help="write compact JSON; default stdout")
    arguments = parser.parse_args()
    encoded = json.dumps(reference_vectors(), separators=(",", ":"), allow_nan=False) + "\n"
    if arguments.output:
        arguments.output.write_text(encoded, encoding="utf-8")
    else:
        print(encoded, end="")


if __name__ == "__main__":
    main()
