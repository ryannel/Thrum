"""Finite offline oracle tests; synthetic readbacks do not establish GPU evidence."""
import json
import math
from pathlib import Path
import tempfile
import unittest

import numpy as np

import validate_deep_sun as ref


def pack(logical):
    nz, ny, nx = logical.shape
    physical = np.zeros(((nz+3)//4+2, ny+2, nx+2, 4), dtype='<f2')
    for z in range(nz):
        physical[z//4+1, 1:ny+1, 1:nx+1, z % 4] = logical[z]
    return physical.tobytes()


def scene_words():
    words = np.zeros(48, dtype='<u4')
    f = words.view('<f4')
    f[4:7] = [2, 3, 5]
    f[16:19] = [0, -1, 0]
    f[20:23], f[24:27], f[28:31] = [1, 0, 0], [0, 0, 1], [0, 1, 0]
    f[23], f[27], f[31] = 900, 900, 900  # legacy padding MUST be ignored
    words[40:43] = [2, 3, 5]
    return words.tolist()


class FieldTests(unittest.TestCase):
    def test_channel_mapping_centres_bounds_and_guards(self):
        expected = np.fromfunction(lambda z, y, x: (1+x+3*y+7*z)/2048, (5, 3, 2)).astype('<f2').astype(float)
        actual = ref.decode_field(pack(expected), [2, 3, 5])
        np.testing.assert_array_equal(actual, expected)
        points = np.array([[x+.5, y+.5, z+.5] for z in range(5) for y in range(3) for x in range(2)])
        np.testing.assert_array_equal(ref.sample_field(actual, points, np.zeros(3), np.array([2, 3, 5])), expected.ravel())
        self.assertEqual(ref.sample_field(actual, [[-.5, .5, .5]], np.zeros(3), np.array([2, 3, 5]))[0], 0)
        self.assertEqual(ref.sample_field(actual, [[-.25, .5, .5]], np.zeros(3), np.array([2, 3, 5]))[0], .25*expected[0, 0, 0])
        broken = bytearray(pack(expected));broken[0:2] = np.array([.01], dtype='<f2').tobytes()
        with self.assertRaisesRegex(ValueError, 'guard'):
            ref.decode_field(broken, [2, 3, 5])
        broken = bytearray(pack(expected))
        # Nonfinite in an active RGBA channel is never masked/clamped.
        physical_index = ((1*5+1)*4+1)*4
        array = np.frombuffer(broken, dtype='<f2');array[physical_index] = np.inf
        with self.assertRaises(ValueError):
            ref.decode_field(broken, [2, 3, 5])

    def test_named_calibration_limit_is_explicit_and_never_clamps(self):
        payload = pack(np.full((1, 1, 1), .125))
        with self.assertRaises(ValueError):
            ref.decode_field(payload, [1, 1, 1])
        actual = ref.decode_field(payload, [1, 1, 1], coefficient_max=.125)
        self.assertEqual(float(actual.max()), .125)

    def test_double_direct_integral_and_scene_bits(self):
        scene = ref.scene_from_words(scene_words())
        np.testing.assert_array_equal(scene['raw'], [1.5, 3, 2])
        # Half-voxel tent edges plus the constant interior integrate to3m.
        field = np.full((5, 3, 2), .02)
        tau = ref.direct_tau(field, scene, np.array([1., 5., 2.5]), 4096)
        self.assertAlmostEqual(tau, .06, delta=1e-8)
        bad = scene_words();bad[29] = 0
        with self.assertRaises(ValueError):
            ref.scene_from_words(bad)


class CurveAndFilterTests(unittest.TestCase):
    def test_full_PL_extrema_not_only_knots(self):
        original = np.array([[0, 0], [1, 20], [2, 21]])
        retained = np.array([[0, 0], [2, 21]])
        error, witnesses = ref.curve_error(original, retained)
        x = np.linspace(0, 2, 20001)
        a, b = np.interp(x, original[:, 0], original[:, 1]), np.interp(x, retained[:, 0], retained[:, 1])
        actual = np.max(np.abs(np.exp(-a[:, None]*ref.EXPONENTS)-np.exp(-b[:, None]*ref.EXPONENTS)), axis=0)
        np.testing.assert_allclose(actual, error, atol=1e-6)
        self.assertTrue(0 < witnesses[0] < 1)
        equal, _ = ref.curve_error([[0, 0], [1, 1], [2, 2]], [[0, 0], [.5, .5], [2, 2]])
        self.assertLess(max(equal), 1e-15, 'different valid anchors are permitted')

    def test_tau_filtering_and_reusing_primary_effective_tau_both_fail(self):
        correct = ref.weighted_terms([0, 20], [.5, .5])
        mean_tau_terms = np.exp(-10*ref.EXPONENTS)
        self.assertGreater(correct[0]-mean_tau_terms[0], .49)
        reused_primary_tau = math.exp(.25*math.log(correct[0]))
        self.assertGreater(abs(reused_primary_tau-correct[1]), .3)

    def test_manual_four_curve_weights_and_invalid_tap(self):
        scene = ref.scene_from_words(scene_words())
        nodes = np.zeros((66, 66, 48, 2), dtype='<f4')
        nodes[:, :, 1, 0] = 4
        meta = np.zeros((66, 66, 8), dtype='<f4');meta[:, :, :2] = [2, 1]
        nodes[33, 33, 1, 1] = 20
        world = scene['centre']+scene['basis'][2]*2
        terms, taps, mode = ref.lookup_terms(nodes, meta, scene, world)
        np.testing.assert_allclose(terms, ref.weighted_terms([0, 0, 0, 20], [.25]*4))
        self.assertEqual(mode, 'four_curve_terms')
        self.assertEqual(taps, [[32, 32], [33, 32], [32, 33], [33, 33]])
        meta[33, 33, 1] = 0
        self.assertIsNone(ref.lookup_terms(nodes, meta, scene, world)[0])
        far = scene['centre']+scene['basis'][0]*100
        np.testing.assert_array_equal(ref.lookup_terms(nodes, meta, scene, far)[0], [1, 1])


class CaptureTests(unittest.TestCase):
    def test_full_synthetic_readback_hashes_and_tampering(self):
        scene = ref.scene_from_words(scene_words())
        nodes = np.zeros((66, 66, 48, 2), dtype='<f4');nodes[:, :, 1, 0] = 4
        meta = np.zeros((66, 66, 8), dtype='<f4');meta[:, :, :] = [2, 1, .004998, .004998, 512, 512, 1, 0]
        paths = np.zeros((1, 513, 4), dtype='<f4');paths[0, :, 0] = np.arange(513)/128;paths[0, :, 3] = 1
        payloads = dict(nodes=nodes.tobytes(), metadata=meta.tobytes(), field=pack(np.zeros((5, 3, 2))), paths=paths.tobytes())
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binaries = {}
            for name, payload in payloads.items():
                filename = name+'.bin';(root/filename).write_bytes(payload)
                binaries[name] = dict(filename=filename, bytes=len(payload), sha256=ref.sha(payload))
            receipt = dict(schema='thrum.deep-sun-probe.v1', uniform_u32=scene_words(), logical_dimensions=[2, 3, 5],
                           field_physical_dimensions=[4, 5, 4], binaries=binaries,
                           identity={'synthetic_test': True}, selected_columns=[dict(physical_xy=[32, 32], linear_index=2144)],
                           query=dict(rows=[dict(world_m=scene['centre'].tolist(), kind='synthetic', terms=[1, 1], direct_tau_512=0, valid=1)]))
            (root/'receipt.json').write_text(json.dumps(receipt))
            result = ref.validate_capture(root)
            self.assertTrue(result['capture_structurally_valid'])
            self.assertFalse(result['whole_profile_qualified'])
            self.assertEqual(result['selected_columns'][0]['compression_error'], [0, 0])
            self.assertEqual(result['query_rows'][0]['GPU_vs_manual_terms_error'], [0, 0])
            self.assertEqual(result['query_rows'][0]['lateral_terms_error'], [0, 0])
            (root/binaries['nodes']['filename']).write_bytes(b'bad')
            with self.assertRaisesRegex(ValueError, 'SHA mismatch'):
                ref.validate_capture(root)


if __name__ == '__main__':
    unittest.main()
