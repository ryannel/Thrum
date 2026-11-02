"""Bounded CPU candidate tests; no uniform/backend/appearance certificate."""
import math
import unittest

import numpy as np

import air_reference as ref


class AirReferenceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.table = ref.cook_table()

    def test_mapping_roundtrip_and_top_degenerate_branch(self):
        for h in (0., .001, 50., 900., 18000., 99999., 100000.):
            for mu in (0.001, .1, .5, 1.):
                xy = ref.to_coordinates(h, mu)
                hh, mm = ref.from_coordinates(*xy)
                self.assertAlmostEqual(float(hh), h, delta=1e-8)
                if h < 100000.:
                    self.assertAlmostEqual(float(mm), mu, delta=1e-9)
        np.testing.assert_array_equal(ref.sun_transmittance(self.table,100000.,1.),[1,1,1])

    def test_ground_tangent_and_night_never_lookup_as_clear(self):
        for h in (0., 50., 900., 18000., 100000.):
            tangent = float(ref.horizon_mu(h))
            np.testing.assert_array_equal(ref.sun_transmittance(self.table,h,tangent),[0,0,0])
            np.testing.assert_array_equal(ref.sun_transmittance(self.table,h,tangent-.0001),[0,0,0])
            self.assertTrue((ref.sun_transmittance(self.table,h,tangent+1e-7)>0).all())
        with self.assertRaises(ValueError):
            ref.sun_transmittance(self.table,-1.,1.)
        with self.assertRaises(ValueError):
            ref.sun_transmittance(self.table,100001.,1.)

    def test_named_midpoint_and_lookup_checks_against_independent_oracle(self):
        for h, mu in [(0.,1.),(900.,.5),(900.,math.sin(math.radians(.1))),(18000.,-.01),(99999.,.001)]:
            expected = ref.oracle_top(h,mu,512)
            self.assertLess(float(np.max(np.abs(ref.midpoint_top(h,mu)-expected))),3e-5)
            self.assertLess(float(np.max(np.abs(ref.sun_transmittance(self.table,h,mu)-expected))),.001)
        self.assertEqual(ref.ATMOSPHERE.aerosol_extinction_per_m,4.4e-6)
        self.assertAlmostEqual(ref.ATMOSPHERE.aerosol_extinction_per_m-ref.ATMOSPHERE.aerosol_scattering_per_m,.404e-6)

    def test_ratio_is_not_implicitly_qualified_or_clamped(self):
        mu = float(ref.horizon_mu(100000.))+1e-6
        forward = ref.segment_ratio(self.table,100000.,mu,1000.)
        self.assertGreater(float(forward.max()),1.,'retain observed interpolation overshoot')
        half = self.table.astype(np.float16).astype(float)
        with self.assertRaisesRegex(ValueError,'underflow'):
            ref.segment_ratio(half,100000.,mu,1000.)
        reverse = ref.segment_ratio(self.table,100000.,mu,1000.,reverse=True)
        self.assertTrue(((reverse>=0)&(reverse<=1)).all())


if __name__=='__main__':
    unittest.main()
