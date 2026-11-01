"""Semantic byte/coordinate guards for the offline clip compiler."""
import gzip
import importlib.util
from pathlib import Path
import struct
import tempfile
import unittest

import numpy as np

COOK = Path(__file__).with_name('cook.py')
spec = importlib.util.spec_from_file_location('cloud_cook', COOK)
cook = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cook)


class ClipBytes(unittest.TestCase):
    def test_sparse_boundaries_empty_sentinel_and_bit_order(self):
        a = np.zeros((16, 16, 16), np.uint8)
        a[0, 0, 7] = 17
        a[15, 15, 15] = 249
        blob, record = cook.pack_density(a)
        indices = np.frombuffer(blob, '<u2', count=8)
        self.assertEqual(indices.tolist(), [0, 65535, 65535, 65535, 65535, 65535, 65535, 1])
        self.assertEqual(blob[16], 129)
        np.testing.assert_array_equal(cook.unpack_density(blob, 16, record), a)
        corrupt = bytearray(blob)
        corrupt[16] ^= 1
        with self.assertRaisesRegex(ValueError, 'occupancy'):
            cook.unpack_density(corrupt, 16, record)

    def test_uni_native_xyz_and_strict_length(self):
        data = np.arange(2*3*4*3, dtype=np.float32).reshape(2, 3, 4, 3)
        header = struct.pack('<6i252siQ', 4, 3, 2, 8, 2, 12, b'fixture', 0, 0)
        self.assertEqual(len(header), 288)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory)/'vel.uni'
            with gzip.open(path, 'wb') as output:
                output.write(b'MNT3'+header+data.astype('<f4').tobytes())
            actual, meta = cook.read_uni(path)
            np.testing.assert_array_equal(actual, data)
            self.assertTrue(meta['full_domain_velocity'])
            with gzip.open(path, 'wb') as output:
                output.write(b'MNT3'+header+data.tobytes()[:-4])
            with self.assertRaisesRegex(ValueError, 'length'):
                cook.read_uni(path)

    def test_axis_rotation_and_mac_center(self):
        a = np.zeros((4, 4, 4), np.float32)
        a[1, 2, 3] = 7
        transformed = cook.y_up(a)
        self.assertEqual(np.argwhere(transformed).tolist(), [[1, 1, 3]])
        v = np.empty((4, 4, 4, 3), np.float32)
        v[...] = [1, 2, 3]
        np.testing.assert_array_equal(cook.y_up(v, True)[0, 0, 0], [1, 3, -2])
        v[..., 0] = np.arange(4)[None, None, :]
        np.testing.assert_array_equal(cook.center_mac(v)[0, 0, :, 0], [.5, 1.5, 2.5, 3])

    def test_trilinear_shear_lipschitz_and_flow_reconstruction(self):
        flow = np.zeros((4, 4, 4, 3), np.float32)
        flow[..., 0] = np.arange(4)[None, :, None] * 2
        bound = cook.flow_bounds(flow, 1)
        self.assertAlmostEqual(bound['lipschitz_spectral_corner_upper_per_s'], 2)
        side = 8
        xyz = cook.grid_positions(side, 8)
        a = xyz[..., 0].copy()
        b = a-1
        uniform = np.zeros((4, 4, 4, 3), np.float32)
        uniform[..., 0] = 1
        result = cook.reconstruct(a, b, uniform, .5, 1, 8)
        np.testing.assert_allclose(result[1:-1, 1:-1, 1:-1],
                                   (a-.5)[1:-1, 1:-1, 1:-1], atol=1e-6)


if __name__ == '__main__':
    unittest.main(argv=[__file__])
