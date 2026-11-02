"""Finite CPU producer/domain checks; no GPU or atmosphere-model certificate."""
import hashlib
import json
import math
from pathlib import Path
import tempfile
import unittest

import numpy as np
import cook_air_profile as cook
import optical_reference as optical


class SunAirCookTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.table, cls.half, cls.fine = cook.cook_arrays()

    def test_selected_format_and_finite_payload(self):
        self.assertEqual(self.half.shape, (64, 256, 4))
        self.assertEqual(self.half.nbytes, 131072)
        self.assertTrue(np.isfinite(self.half).all())
        self.assertTrue(((self.half >= 0) & (self.half <= 1)).all())
        self.assertTrue((self.half[:, :, 3].view('<u2') == 0x3c00).all())
        self.assertEqual((self.half[:, :, :3] == 0).sum(axis=(0, 1)).tolist(), [0, 0, 110])

    def test_vertical_closed_form_and_top(self):
        p = optical.EARTH
        rayleigh = p.rayleigh_height_m * -math.expm1(-p.top_m/p.rayleigh_height_m)
        aerosol = p.aerosol_height_m * -math.expm1(-p.top_m/p.aerosol_height_m)
        exact = np.exp(-rayleigh*np.asarray(p.rayleigh_scattering_per_m)-aerosol*p.aerosol_extinction_per_m)
        np.testing.assert_allclose(self.table[0, 0, :3], exact, atol=3e-5, rtol=0)
        np.testing.assert_array_equal(self.half[-1, 0], np.ones(4))
        self.assertGreater(float(self.half[-1, -1, 0]), 0)
        self.assertLess(float(self.half[-1, -1, 0]), 1, 'top inward grazing path is not top/outward clear')

    def test_all_knots_against_independent_fine_oracle(self):
        self.assertLess(float(np.max(np.abs(self.table[:, :, :3]-self.fine))), 3e-5)
        self.assertLess(float(np.max(np.abs(self.half[:, :, :3].astype(float)-self.fine))), .0005)

    def test_unsupported_is_distinct_from_planet_and_visible(self):
        self.assertEqual(cook.reference_for_height_mu(-.001, 1), {'status':'unsupported', 'valid':0, 'rgb':None})
        self.assertEqual(cook.reference_for_height_mu(0, 0), {'status':'planet-blocked', 'valid':1, 'rgb':[0., 0., 0.]})
        self.assertEqual(cook.reference_for_height_mu(100000, 1)['rgb'], [1., 1., 1.])
        for h in [1., 900., 18000., 100000.]:
            horizon = float(cook.reference.horizon_mu(h))
            self.assertEqual(cook.primitive_status(h, horizon), 'unsupported')
            self.assertEqual(cook.primitive_status(h, horizon-.5e-6), 'unsupported')
            self.assertEqual(cook.primitive_status(h, horizon-2e-6), 'planet-blocked')
            self.assertEqual(cook.primitive_status(h, horizon+2e-6), 'visible')
        for h, mu in [(math.nan, 0), (1, math.inf), (100001, 1), (1, 1.0001)]:
            self.assertEqual(cook.primitive_status(h, mu), 'unsupported')

    def test_world_bounds_unit_and_local_height(self):
        self.assertEqual(cook.reference_for_world([0, -.001, 0], [0, 1, 0])['status'], 'unsupported')
        self.assertEqual(cook.reference_for_world([100001, 0, 0], [0, 1, 0])['status'], 'unsupported')
        self.assertEqual(cook.reference_for_world([0, 0, 0], [0, 2, 0])['status'], 'unsupported')
        self.assertEqual(cook.reference_for_world([0, 0, 0], [0, -1, 0])['status'], 'planet-blocked')
        tiny = cook.reference_for_world([0, .001, 0], [0, 1, 0])
        self.assertAlmostEqual(tiny['radial_height_m'], .001, places=12)
        far = cook.reference_for_world([100000, 0, 0], [0, 1, 0])
        self.assertGreater(far['radial_height_m'], 700)
        self.assertEqual(far['status'], 'visible')

    def test_reproducible_package_and_hashes(self):
        selected = Path(__file__).parent/'fixtures'/'air-sun'
        with tempfile.TemporaryDirectory() as tmp:
            profile = cook.cook_package(tmp)
            for name in ['profile.json', 'transmittance.rgba16f', 'queries.json']:
                self.assertEqual((Path(tmp)/name).read_bytes(), (selected/name).read_bytes())
            for role in ['table', 'oracle']:
                item = profile['files'][role]
                data = (Path(tmp)/item['filename']).read_bytes()
                self.assertEqual(item['bytes'], len(data))
                self.assertEqual(item['sha256'], hashlib.sha256(data).hexdigest())
            self.assertFalse(profile['uniform_certificate'])
            self.assertEqual(profile['acceptance']['gpu_total_absolute_T'], .0015)
            self.assertLess(max(profile['finite_cpu_evidence']['admitted_half_lookup_error_max_rgb']), .0005)
            oracle = json.loads((Path(tmp)/'queries.json').read_text())
            self.assertEqual(len(oracle['knots']['reference_rgb']), 16384)
            self.assertTrue(any(q['kind']=='explicit-ambiguity-band' and not q['expected']['valid'] for q in oracle['queries']))
            self.assertTrue(any(any(x in (0x7fc00000, 0x7f800000) for x in q['input_u32']) for q in oracle['queries']))


if __name__ == '__main__':
    unittest.main()
