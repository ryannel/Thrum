"""Arithmetic/layout witnesses for the independent packing vector producer."""

import json
import math
from pathlib import Path
import random
import struct
import subprocess
import sys
import tempfile
import unittest

import reference_packing as reference


class HalfRoundingTests(unittest.TestCase):
    def test_declared_ties_and_subnormals_have_known_bits(self):
        witnesses = [(0.0, 0x0000), (-0.0, 0x8000), (2**-25, 0x0000),
                     (2**-24, 0x0001), (3*2**-25, 0x0002),
                     (1023*2**-24, 0x03FF), (1023.5*2**-24, 0x0400),
                     (2**-14, 0x0400), (0.0625, 0x2C00),
                     (0.0625+2**-15, 0x2C00), (0.0625+3*2**-15, 0x2C02),
                     (0.1, 0x2E66)]
        for value, expected in witnesses:
            with self.subTest(value=value):
                self.assertEqual(reference.half_bits(value), expected)
        midpoint = 0.0625 + 2**-15
        self.assertEqual(reference.half_bits(math.nextafter(midpoint, -math.inf)), 0x2C00)
        self.assertEqual(reference.half_bits(math.nextafter(midpoint, math.inf)), 0x2C01)

    def test_integer_rounder_matches_independent_standard_half_codec(self):
        rng = random.Random(83017)
        values = [rng.random() * 0.1 for _ in range(4096)]
        values += [math.ldexp(1.0, exponent) for exponent in range(-30, -3)]
        for value in values:
            expected = struct.unpack("<H", struct.pack("<e", value))[0]
            self.assertEqual(reference.half_bits(value), expected)
        for bits in range(0x2E67):
            self.assertEqual(reference.half_value(bits), struct.unpack("<e", struct.pack("<H", bits))[0])

    def test_invalid_coefficients_cannot_become_texture_data(self):
        for value in (-0.001, 0.10001, math.inf, -math.inf, math.nan):
            with self.subTest(value=value), self.assertRaises(ValueError):
                reference.coefficient_bits(value)
        with self.assertRaises(OverflowError):
            reference.half_bits(65520.0)
        for dimensions in ((0, 1, 1), (1, 1), (True, 1, 1), (1.5, 1, 1)):
            with self.assertRaises(ValueError):
                reference.validate_dimensions(dimensions)


class LayoutAndSamplingTests(unittest.TestCase):
    def setUp(self):
        self.vectors = reference.reference_vectors()

    def test_every_logical_coefficient_roundtrips_and_all_padding_is_zero(self):
        for fixture in self.vectors["fixtures"]:
            for level in fixture["levels"]:
                dimensions = tuple(level["logical_dimensions_xyz"])
                nx, ny, nz = dimensions
                px, py, pz = level["physical_dimensions_xyz"]
                payload = bytes.fromhex(level["rgba_half_le_hex"])
                self.assertEqual(len(payload), px * py * pz * 8)
                for gz in range(pz):
                    for gy in range(py):
                        for gx in range(px):
                            for channel in range(4):
                                x, y, z = gx-1, gy-1, 4*(gz-1)+channel
                                actual = struct.unpack_from("<H", payload, (((gz*py+gy)*px+gx)*4+channel)*2)[0]
                                expected = (level["dense_half_bits"][(z*ny+y)*nx+x]
                                            if 0 <= x < nx and 0 <= y < ny and 0 <= z < nz else 0)
                                self.assertEqual(actual, expected, (fixture["id"], level["level"], x, y, z))
                for z in range(-5, nz+5):
                    self.assertEqual(reference.packed_logical_value(payload, dimensions, 0, 0, z),
                                     reference.logical_value(level["dense_half_bits"], dimensions, 0, 0, z))

    def test_ramp_is_exact_at_centres_and_z_group_boundary(self):
        dimensions = (7, 5, 9)
        bits = reference._pattern(dimensions, "ramp")
        self.assertEqual(reference.point_coefficient(bits, dimensions, (1.5, 1.5, 2.5)), 19/16384)
        self.assertEqual(reference.point_coefficient(bits, dimensions, (0.5, 0.5, 3.5)), 22/16384)
        self.assertEqual(reference.point_coefficient(bits, dimensions, (0.5, 0.5, 4.5)), 29/16384)
        self.assertEqual(reference.point_coefficient(bits, dimensions, (0.5, 0.5, 4.0)), 25.5/16384)
        self.assertEqual(reference.point_coefficient(bits, dimensions, (1.25, 2.125, 4.25)),
                         (1 + 0.75 + 3*1.625 + 7*3.75)/16384)

    def test_zero_extension_filters_support_boundary_without_clamp(self):
        bits = [reference.half_bits(0.0625)]
        dimensions = (1, 1, 1)
        self.assertEqual(reference.point_coefficient(bits, dimensions, (0.0, 0.5, 0.5)), 0.03125)
        self.assertEqual(reference.point_coefficient(bits, dimensions, (-0.25, 0.5, 0.5)), 0.015625)
        self.assertEqual(reference.point_coefficient(bits, dimensions, (-0.5, 0.5, 0.5)), 0)
        self.assertEqual(reference.point_coefficient(bits, dimensions, (1, 1, 1)), 0.0625/8)
        self.assertEqual(reference.point_coefficient(bits, dimensions, (2, 2, 2)), 0)

    def test_centre_shift_and_wrong_group_antipatterns_are_detected(self):
        dimensions = (7, 5, 9)
        bits = reference._pattern(dimensions, "ramp")
        actual = reference.point_coefficient(bits, dimensions, (0.5, 0.5, 4.0))
        wrong_unshifted = reference.point_coefficient(bits, dimensions, (1.0, 1.0, 4.5))
        self.assertNotEqual(actual, wrong_unshifted)
        payload = reference.pack_rgba_half(bits, dimensions)
        px, py, _ = reference.packed_dimensions(dimensions)
        wrong_truncated_group = struct.unpack_from("<H", payload, ((1*py+1)*px+1)*8)[0]
        self.assertNotEqual(reference.half_value(wrong_truncated_group), 29/16384)
        self.assertEqual(reference.packed_logical_value(payload, dimensions, 0, 0, 4), 29/16384)

    def test_mip_parents_are_logical_cells_not_packed_groups(self):
        dimensions = (4, 4, 5)
        bits = reference._pattern(dimensions, "ramp")
        mip, mip_dimensions = reference.logical_mip(bits, dimensions)
        self.assertEqual(mip_dimensions, (2, 2, 3))
        self.assertEqual(reference.half_value(mip[0]), 6.5/16384)
        # Native packed-Z mipmapping would average channel R at logical z=0,4,
        # rather than averaging the adjacent logical parents z=0,1.
        wrong_parents = [(1+x+3*y+7*z)/16384 for x in (0, 1) for y in (0, 1) for z in (0, 4)]
        self.assertNotEqual(reference.half_bits(sum(wrong_parents)/8), mip[0])
        # Last odd slice keeps z=4 and zero z=5; do not clamp/duplicate z=4.
        self.assertEqual(reference.half_value(mip[8]), 15.5/16384)

    def test_odd_and_singleton_mips_keep_zero_boundary_and_round_each_store(self):
        dimensions = (7, 5, 9)
        bits = reference._pattern(dimensions, "impulse")
        mip, dimensions = reference.logical_mip(bits, dimensions)
        self.assertEqual(dimensions, (4, 3, 5))
        mip, dimensions = reference.logical_mip(mip, dimensions)
        self.assertEqual(dimensions, (2, 2, 3))
        value = [reference.half_bits(0.0625)]
        for expected in (0.0625/8, 0.0625/64):
            value, tiny_dimensions = reference.logical_mip(value, (1, 1, 1))
            self.assertEqual(tiny_dimensions, (1, 1, 1))
            self.assertEqual(reference.half_value(value[0]), expected)
        # A single minimum subnormal among eight parents averages to a value
        # below the half-way point and must round to zero at the mip store.
        value, _ = reference.logical_mip([1], (1, 1, 1))
        self.assertEqual(value, [0])

    def test_actual_texture_fixture_contains_subnormals_and_mip_ties_both_parities(self):
        fixture = next(f for f in self.vectors["fixtures"] if f["pattern"] == "rounding_store_edges")
        self.assertEqual(fixture["dimensions_xyz"], (4, 4, 5))
        level0, level1, _ = fixture["levels"]
        self.assertTrue({0x0001, 0x03FF, 0x0400, 0x2C00, 0x2C01, 0x2C02}.issubset(level0["dense_half_bits"]))
        # First two Z slices form four midpoint blocks; next two include the
        # normal/subnormal boundary and one exact min-subnormal store.
        self.assertEqual(level1["dense_half_bits"][:8],
                         [0x0000, 0x0002, 0x2C00, 0x2C02,
                          0x0400, 0x0001, 0x2A66, 0x03FE])
        payload = bytes.fromhex(level0["rgba_half_le_hex"])
        # z=3 is channel A, z=4 is the next group's R. Check tiny coefficients
        # survive actual payload packing rather than only pack-to-u16 vectors.
        self.assertEqual(reference.packed_logical_value(payload, (4, 4, 5), 0, 0, 4), 2**-24)
        self.assertEqual(reference.packed_logical_value(payload, (4, 4, 5), 1, 0, 4), 2*2**-24)
        # Ties-away would produce 1 for the first midpoint; truncation would
        # produce 1 for the second. The expected pair rejects both shortcuts.
        self.assertNotEqual(level1["dense_half_bits"][:2], [1, 2])
        self.assertNotEqual(level1["dense_half_bits"][:2], [0, 1])

    def test_coeff_average_mip_does_not_claim_transmittance_equivalence(self):
        point_depths = [0.0, 8.0]
        filtered_transmittance = sum(math.exp(-tau) for tau in point_depths)/2
        mean_depth_transmittance = math.exp(-sum(point_depths)/2)
        self.assertGreater(filtered_transmittance, 0.5)
        self.assertLess(mean_depth_transmittance, 0.02)
        self.assertNotAlmostEqual(filtered_transmittance, mean_depth_transmittance)
        self.assertEqual(self.vectors["contract"]["point_optics_level"], 0)


class OpticalAndArtifactTests(unittest.TestCase):
    def test_constant_slab_affine_composition_and_zero_extinction_limit(self):
        clear = reference.constant_slab(0, 10)
        self.assertEqual((clear["tau"], clear["T"], clear["S"]), (0, 1, 0.02))
        full = reference.constant_slab(0.0625, 100)
        near = reference.constant_slab(0.0625, 25)
        far = reference.constant_slab(0.0625, 75)
        self.assertAlmostEqual(full["tau"], 6.25)
        self.assertAlmostEqual(full["T"], near["T"] * far["T"])
        self.assertAlmostEqual(full["S"], near["S"] + near["T"] * far["S"])
        self.assertAlmostEqual(full["outgoing_radiance"], full["T"]*2 + full["S"])

    def test_cli_exports_small_repeatable_json_with_all_requested_shapes(self):
        script = Path(__file__).with_name("reference_packing.py")
        with tempfile.TemporaryDirectory(prefix="thrum-packing-reference-") as temporary:
            output = Path(temporary)/"vectors.json"
            subprocess.run([sys.executable, str(script), "--output", str(output)], check=True)
            actual = json.loads(output.read_text())
            expected = json.loads(json.dumps(reference.reference_vectors()))
            self.assertEqual(actual, expected)
            self.assertLess(output.stat().st_size, 1024*1024)
            self.assertEqual(len(actual["fixtures"]), 13)
            self.assertEqual({tuple(fixture["dimensions_xyz"]) for fixture in actual["fixtures"]},
                             {(7, 5, 9), (1, 1, 1), (4, 3, 4), (4, 4, 5)})
            self.assertTrue(all(len(fixture["levels"]) == 3 for fixture in actual["fixtures"]))


if __name__ == "__main__":
    unittest.main()
