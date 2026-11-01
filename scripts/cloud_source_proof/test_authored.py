"""Source contract regressions, including the observed float32 ghost-fog fault."""
import unittest
import numpy as np
from authored import Source, smooth


class AuthoredContract(unittest.TestCase):
    def test_controller_closed_range_monotonicity_and_endpoints(self):
        values=smooth(np.linspace(-.01,1.01,100001,dtype=np.float32))
        self.assertEqual(float(values.min()),0.)
        self.assertEqual(float(values.max()),1.)
        self.assertTrue(np.all(np.diff(values)>=0))

    def test_empty_space_never_creates_density_and_lifecycle_endpoints_are_zero(self):
        source=Source()
        for age in [0,20,60,67.5,90,185,200,240]:
            density=source.field(age)
            self.assertTrue(np.isfinite(density).all())
            self.assertGreaterEqual(float(density.min()),0.)
            self.assertLessEqual(float(density.max()),.125)
            self.assertFalse(np.any(density[source.base==0]))
            if age in [0,240]:self.assertFalse(np.any(density))


if __name__=='__main__':unittest.main()
