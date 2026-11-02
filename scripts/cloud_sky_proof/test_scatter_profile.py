"""Focused CPU source-fit tests; no family appearance or GPU qualification."""
import hashlib
import json
import math
from pathlib import Path
import tempfile
import unittest

import numpy as np

import cook_scatter_profile as fit


class FluxTests(unittest.TestCase):
    def test_profile_admission_and_ice_key(self):
        self.assertEqual(fit.ProfileParameters(g=.75).g, .75)
        for args in ({"omega": .9}, {"g": .9}, {"kappa": .5}, {"rho": 1}, {"g": math.nan}):
            with self.assertRaises(ValueError):
                fit.ProfileParameters(**args)

    def test_equal_slope_and_factored_bottom_solution(self):
        self.assertAlmostEqual(float(fit.difference_integral(2, .5, .5)), 2*math.exp(-1))
        self.assertAlmostEqual(float(fit.difference_integral(2, .5, .5+1e-12)), 2*math.exp(-1), delta=2e-12)
        for a,b,t in ((1, 2, 3), (2, 1, 3), (.0001, 50000, 1000)):
            actual = float(fit.difference_integral(t, a, b))
            expected = (math.exp(-a*t)-math.exp(-b*t))/(b-a)
            self.assertAlmostEqual(actual, expected, delta=1e-14)
        self.assertEqual(float(fit.difference_integral(0, 1, 1)), 0)

    def test_grazing_retains_finite_bottom_escape(self):
        for t in (.001, .1, 2):
            exact = fit.flux_components(t, 0)
            near = fit.flux_components(t, 1e-8)
            self.assertGreater(exact["primary_down"], 0)
            self.assertAlmostEqual(exact["primary_up"], .5)
            self.assertAlmostEqual(exact["bank_up"], 2)
            for key in ("F0", "F1", "primary_down", "bank_down"):
                self.assertAlmostEqual(exact[key], near[key], delta=3e-6)
        self.assertAlmostEqual(fit.infinity_amplitude(0), .15)

    def test_clear_and_exact_primary(self):
        for m in (0, .1, 1):
            f = fit.flux_components(0, m)
            self.assertEqual(f["F0"], 1)
            self.assertEqual(f["F1"], 0)
            self.assertEqual(fit.fit_coefficient(0, m), 0)
            self.assertEqual(fit.fit_coefficient(1e-7, m), 0)
        for cosine in (-1, 0, 1):
            self.assertAlmostEqual(fit.source_factor(cosine, 2, 0), float(fit.phase_hg(cosine, .86))*math.exp(-2))

    def test_finite_cap_and_generated_power_not_limited_to_one(self):
        for m in (0, .001, *[math.sin(math.radians(x)) for x in (2,5,15,30,75,90)]):
            t = np.concatenate(([0], np.geomspace(1e-6, 1000, 80), [math.inf]))
            a = fit.fit_coefficient(t, m)
            f = fit.flux_components(t, m)
            self.assertTrue(np.all(f["F0"]+a*f["F1"] <= 1+1e-9))
            self.assertTrue(np.all((a >= 0) & (a < fit.A_BOUND)))
        a = fit.fit_coefficient(math.inf, math.sin(math.radians(75)))
        self.assertGreater(1+a/.25, 10, "generated bounce power is explicitly allowed above input")

    def test_known_opaque_and_finite_fit_values(self):
        expected = {5: (.5812496915,.4345684677), 15: (1.2242461044,.9153015674),
                    30: (2.0444058740,1.4649954930), 75: (3.5717544557,1.6753516850)}
        for degrees, values in expected.items():
            m = math.sin(math.radians(degrees))
            self.assertAlmostEqual(fit.infinity_amplitude(m), values[0], delta=2e-7)
            self.assertAlmostEqual(fit.fit_coefficient(2, m), values[1], delta=2e-7)
            f = fit.flux_components(math.inf, m)
            self.assertAlmostEqual(f["primary_up"]+fit.infinity_amplitude(m)*f["bank_up"], .8, delta=2e-13)

    def test_quadrature_convergence_at_required_angles(self):
        for degree in (2,5,15,30,75,90):
            mu = math.sin(math.radians(degree))
            low = fit.fit_coefficient(np.array([.01, .1, 1, 2, 5, math.inf]), mu, cosine_count=128, phi_count=512)
            high = fit.fit_coefficient(np.array([.01, .1, 1, 2, 5, math.inf]), mu)
            np.testing.assert_allclose(low, high, atol=2e-6, rtol=0)

    def test_unsupported_is_not_clear(self):
        for t, m in ((-1,.5),(math.nan,.5),(1,-.1),(1,1.1),(1,math.nan)):
            with self.assertRaises(ValueError):
                fit.fit_coefficient(t,m)
        with self.assertRaises(ValueError):
            fit.source_factor(0, -1, 0)


class TableTests(unittest.TestCase):
    def test_float32_axes_strict_order_and_count_limits(self):
        t, m = fit.make_axes()
        self.assertEqual((len(t),len(m)), (128,64))
        self.assertEqual(t.dtype, np.float32)
        self.assertTrue(np.all(np.diff(t)>0) and np.all(np.diff(m)>0))
        self.assertEqual((t[0],t[-1],m[0],m[-1]), (0,1000,0,1))
        for counts in ((1,3),(2,2),(257,128)):
            with self.assertRaises(ValueError):
                fit.make_axes(*counts)

    def test_physical_T_interpolation_not_log_fraction(self):
        table=fit.ProfileTable(fit.ProfileParameters(),np.array([0,1,3],np.float32),np.array([0,1],np.float32),
                               np.array([[0,1,3],[0,1,3]],np.float32),{})
        self.assertAlmostEqual(fit.sample_table(table,2,.4),2)
        self.assertEqual(fit.sample_table(table,1e-7,.5),0)
        self.assertEqual(fit.sample_table(table,1001,.5),3)
        self.assertEqual(fit.sample_table(table,math.inf,.5),3)
        with self.assertRaises(ValueError):
            fit.sample_table(table,1,1.1)

    def test_format_roundtrip_hashes_and_row_order(self):
        t,m=fit.make_axes(4,8)
        table=fit.build_table(fit.ProfileParameters(),t,m,64,256)
        table.metadata={"elapsed_seconds":0,"quadrature":{"scope":"small unit fixture"}}
        with tempfile.TemporaryDirectory() as path:
            doc=fit.write_package(table,path)
            loaded=json.loads((Path(path)/'profile.json').read_text())
            self.assertEqual(loaded['schema'],fit.SCHEMA)
            self.assertFalse(loaded['empirical_certificate'])
            for record in doc['files'].values():
                payload=(Path(path)/record['filename']).read_bytes()
                self.assertEqual(hashlib.sha256(payload).hexdigest(),record['sha256'])
                self.assertEqual(len(payload),record['bytes'])
            a=np.fromfile(Path(path)/'A.f32',dtype='<f4').reshape(len(m),len(t))
            axes=np.fromfile(Path(path)/'axes.f32',dtype='<f4')
            np.testing.assert_array_equal(a,table.coefficients)
            np.testing.assert_array_equal(axes,np.concatenate((t,m)))
            self.assertEqual(loaded['axes']['T'],t.astype(float).tolist())
            self.assertEqual(loaded['axes']['mu'],m.astype(float).tolist())

    def test_bad_output_payload_cannot_publish(self):
        t,m=fit.make_axes(3,5)
        good=fit.build_table(fit.ProfileParameters(),t,m,32,128)
        for value in (math.nan,-.01,fit.A_BOUND):
            bad=fit.ProfileTable(good.parameters,t,m,good.coefficients.copy(),{})
            bad.coefficients[1,2]=value
            with tempfile.TemporaryDirectory() as path:
                with self.assertRaises(ValueError):
                    fit.write_package(bad,path)
                self.assertFalse((Path(path)/'profile.json').exists())
        with self.assertRaises(ValueError):
            fit.build_table(fit.ProfileParameters(),np.array([0,1,3],np.float32),m)

    def test_source_bound_survives_ice_phase_and_empirical_error(self):
        for g in (.86,.75):
            p=fit.ProfileParameters(g=g)
            self.assertLess(float(fit.phase_hg(1,g)),8)
            self.assertLess(120000*(8+fit.A_BOUND/(4*math.pi))+960000,2e6)
            for mu in (0,.5,1):
                self.assertLess(fit.infinity_amplitude(mu,p),fit.A_BOUND)


if __name__=='__main__':
    unittest.main()
