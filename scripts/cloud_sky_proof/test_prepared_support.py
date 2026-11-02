"""Exact support-map tests; no GPU/optical quality claim."""
import unittest

import numpy as np

import prepared_support as ref


def direct_positive(original, output_side):
    """Independent scalar eight-neighbour trilinear positivity oracle."""
    side = len(original)
    result = np.zeros((output_side,)*3, bool)
    for z in range(output_side):
        for y in range(output_side):
            for x in range(output_side):
                q = (np.array([z, y, x])+.5)*side/output_side-.5
                lo = np.floor(q).astype(int)
                f = q-lo
                value = 0.
                for dz in (0, 1):
                    for dy in (0, 1):
                        for dx in (0, 1):
                            offset = np.array([dz, dy, dx]);index = lo+offset
                            if ((index >= 0) & (index < side)).all():
                                weight = np.prod(np.where(offset, f, 1-f))
                                value += weight*original[tuple(index)]
                result[z, y, x] = value > 0
    return result


class PreparedSupportTests(unittest.TestCase):
    def test_exact_positive_sample_indices_with_edge_clipping(self):
        original = np.zeros((4,)*3, bool)
        original[0, 0, 0] = True
        original[2, 1, 3] = True
        for side in (4, 8):
            np.testing.assert_array_equal(ref.field_support_mask(original, side), direct_positive(original, side))

    def test_reconstructed_field_hats_require_two_or_three_metre_extension(self):
        original = np.zeros((4,)*3, bool);original[1, 1, 1] = True
        a = ref.reconstructed_bounds(ref.field_support_mask(original, 4), extent_m=16)
        b = ref.reconstructed_bounds(ref.field_support_mask(original, 8), extent_m=16)
        self.assertEqual(a['min_m'], [2, 2, 2])
        self.assertEqual(a['max_m'], [10, 10, 10])
        self.assertEqual(b['min_m'], [1, 1, 1])
        self.assertEqual(b['max_m'], [11, 11, 11])
        # Cell hull is[4,8]; sample-centre or original-only bounds would crop hats.
        self.assertEqual(b['min_xyz'], [1, 1, 1])
        self.assertEqual(b['max_xyz'], [4, 4, 4])

    def test_nonnegative_temporal_mix_cannot_escape_endpoint_union(self):
        a = np.zeros((4,)*3, float);b = a.copy()
        a[1, 1, 1], b[2, 2, 2] = .125, .001
        union = ref.field_support_mask((a > 0) | (b > 0), 8)
        for theta in (0, .00001, .25, .5, 1):
            actual = direct_positive((1-theta)*a+theta*b, 8)
            self.assertFalse((actual & ~union).any())

    def test_unsupported_resolution_rejects(self):
        with self.assertRaises(ValueError):
            ref.field_support_mask(np.zeros((4,)*3, bool), 16)
        self.assertIsNone(ref.mask_bounds(np.zeros((4,)*3, bool)))


if __name__ == '__main__':
    unittest.main()
