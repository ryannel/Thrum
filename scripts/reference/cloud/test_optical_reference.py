"""Correctness tests for the independent optical oracle; no timing claims."""
import dataclasses
import json
import math
import unittest

import numpy as np

import optical_reference as ref


class GeometryTests(unittest.TestCase):
    def test_stable_radial_height_tiny_height_and_remote_shell(self):
        self.assertAlmostEqual(float(ref.radial_height([0, 1e-8, 0])), 1e-8, delta=1e-20)
        x, height = 780000.0, 17356.547878167476
        y = math.sqrt((ref.EARTH.radius_m+height)**2-x*x)-ref.EARTH.radius_m
        self.assertAlmostEqual(float(ref.radial_height([x, y, 0])), height, delta=2e-9)

    def test_planet_top_and_zero_length_terminals(self):
        self.assertEqual(ref.trace_terminal([0, 0, 0], [0, -1, 0])["terminal"], "planet")
        self.assertEqual(ref.trace_terminal([0, 0, 0], [1, 0, 0])["distance_m"], 0)
        self.assertEqual(ref.trace_terminal([0, 100000, 0], [0, 1, 0])["distance_m"], 0)
        self.assertAlmostEqual(ref.trace_terminal([0, 2, 0], [0, -1, 0])["distance_m"], 2)
        self.assertAlmostEqual(ref.trace_terminal([0, 2, 0], [0, 1, 0])["distance_m"], 99998)

    def test_tangent_and_near_tangent_side_classification(self):
        radius, h = ref.EARTH.radius_m, 1536
        angle = math.acos(radius/(radius+h))
        tangent = [radius/(radius+h), -math.sqrt(h*(2*radius+h))/(radius+h), 0]
        exact = ref.ray_sphere_roots([0, h, 0], tangent)
        self.assertIsNotNone(exact)
        self.assertAlmostEqual(exact[0], math.sqrt(h*(2*radius+h)), delta=.01)
        self.assertIsNone(ref.ray_sphere_roots([0, h, 0], ref.sun_direction(math.degrees(-angle)+1e-5)))
        self.assertIsNotNone(ref.ray_sphere_roots([0, h, 0], ref.sun_direction(math.degrees(-angle)-1e-5)))

    def test_invalid_fixture_parameters_are_rejected(self):
        for kwargs in ({"radius_m": -1}, {"aerosol_scattering_per_m": 1},
                       {"aerosol_g": 1}, {"rayleigh_scattering_per_m": (1, -1, 0)}):
            with self.assertRaises(ValueError):
                ref.Atmosphere(**kwargs)

    def test_unsupported_is_not_clear(self):
        for p, d in (([0, -1, 0], [0, 1, 0]), ([0, 100001, 0], [0, 1, 0]), ([0, 2, 0], [0, 2, 0])):
            result = ref.air_path(p, d)
            self.assertEqual(result["status"], "unsupported")
            self.assertNotIn("air_transmittance_rgb", result)
        self.assertEqual(ref.air_path([0, 2, 0], [0, 1, 0], -1)["status"], "unsupported")

    def test_visible_point_ambient_blocker_beyond_768km(self):
        radius = ref.EARTH.radius_m
        p = np.array([400000., math.sqrt((radius+6000)**2-400000**2)-radius, 0])
        direction = np.array([(p[1]+radius)/(radius+6000), -p[0]/(radius+6000), 0])
        distance = max(ref.ray_sphere_roots(p, direction, 18000))
        self.assertGreater((p+distance*direction)[0], 790000)
        self.assertIsNone(ref.ray_sphere_roots(p, direction))
        camera = np.array([0, 1536, 0])
        view = (p-camera)/np.linalg.norm(p-camera)
        self.assertEqual(ref.trace_terminal(camera, view)["terminal"], "top")


class TransportTests(unittest.TestCase):
    def test_vertical_air_matches_analytic_exponential(self):
        result = ref.air_path([0, 0, 0], [0, 1, 0])
        a = ref.EARTH
        analytic = np.array(a.rayleigh_scattering_per_m)*a.rayleigh_height_m*(-math.expm1(-a.top_m/a.rayleigh_height_m))
        analytic += a.aerosol_extinction_per_m*a.aerosol_height_m*(-math.expm1(-a.top_m/a.aerosol_height_m))
        np.testing.assert_allclose(result["tau_rgb"], analytic, rtol=2e-12, atol=1e-14)

    def test_horizon_and_low_sun_are_spherical_and_converged(self):
        for elev in (0, 5, 15, 75):
            a = ref.air_path([0, 2, 0], ref.sun_direction(elev), order=64)
            b = ref.air_path([0, 2, 0], ref.sun_direction(elev), order=128)
            self.assertEqual(a["terminal"], "top")
            np.testing.assert_allclose(a["tau_rgb"], b["tau_rgb"], rtol=2e-7, atol=1e-9)
        horizon = ref.air_path([0, 2, 0], [1, 0, 0])
        self.assertLess(horizon["air_transmittance_rgb"][2], .01)

    def test_planet_blocks_sun_but_not_air_optical_depth(self):
        result = ref.air_path([0, 2, 0], ref.sun_direction(-5))
        self.assertEqual(result["terminal"], "planet")
        self.assertEqual(result["source_transmittance_rgb"], [0, 0, 0])
        self.assertTrue(all(x > 0 for x in result["air_transmittance_rgb"]))

    def test_direct_air_dark_clear_and_quadrature(self):
        zero = ref.air_direct_radiance([0, 2, 0], [0, 1, 0], ref.sun_direction(5), (0, 0, 0))
        self.assertEqual(zero["direct_radiance_rgb_cd_m2"], [0, 0, 0])
        clear = dataclasses.replace(ref.EARTH, rayleigh_scattering_per_m=(0, 0, 0), aerosol_scattering_per_m=0, aerosol_extinction_per_m=0)
        self.assertEqual(ref.air_path([0, 2, 0], [0, 1, 0], atmosphere=clear)["air_transmittance_rgb"], [1, 1, 1])
        self.assertEqual(ref.air_direct_radiance([0, 2, 0], [0, 1, 0], ref.sun_direction(5), atmosphere=clear)["direct_radiance_rgb_cd_m2"], [0, 0, 0])
        a = ref.air_direct_radiance([0, 2, 0], [0, 1, 0], ref.sun_direction(5), order=64, sun_order=64)
        b = ref.air_direct_radiance([0, 2, 0], [0, 1, 0], ref.sun_direction(5), order=128, sun_order=128)
        np.testing.assert_allclose(a["direct_radiance_rgb_cd_m2"], b["direct_radiance_rgb_cd_m2"], rtol=2e-5)

    def test_beer_dark_clear_and_layer_semigroup(self):
        self.assertEqual(ref.beer_segment(0, 0, 100), (1, 0))
        self.assertEqual(ref.beer_segment(0, 3, 10), (1, 30))
        self.assertEqual(ref.beer_segment(.01, 0, 100)[1], 0)
        whole = ref.beer_segment(.003, 4, 1000)
        pieces = (1, 0)
        for _ in range(100):
            pieces = ref.compose(pieces, ref.beer_segment(.003, 4, 10))
        np.testing.assert_allclose(pieces, whole, rtol=2e-14)
        self.assertNotEqual(ref.compose((.5, 10), (.8, 1))[1], ref.compose((.8, 1), (.5, 10))[1])

    def test_mirror_solar_exit_bound_uses_downstream_terminal(self):
        near = ref.beer_segment(math.log(1e8), 0, 1)
        actual_reflected_sun = near[0]*2e9
        correct = ref.remaining_bound(near[0], 2e9, 1920000)
        incorrect = ref.remaining_bound(near[0], 20000, 1920000)
        self.assertAlmostEqual(actual_reflected_sun, 20)
        self.assertGreater(correct, .25)
        self.assertLess(incorrect, .25)
        self.assertGreaterEqual(correct, actual_reflected_sun)


class PhaseSlabTests(unittest.TestCase):
    def test_phase_normalization(self):
        mu, weights = np.polynomial.legendre.leggauss(256)
        for g in (-.8, 0, .8, .86, *ref.LIQUID_G):
            self.assertAlmostEqual(float(2*math.pi*np.sum(weights*ref.phase_hg(mu, g))), 1, delta=3e-12)
        self.assertAlmostEqual(float(2*math.pi*np.sum(weights*ref.phase_rayleigh(mu))), 1, delta=2e-14)
        with self.assertRaises(ValueError):
            ref.phase_hg(0, 1)

    def test_new_passivity_old_unconditional_counterexample(self):
        for elevation in (5, 15, 75):
            for tau in (.0001, .1, 1, 20, math.inf):
                generated = ref.slab_generated_power(elevation, tau)
                escaped = ref.slab_upward_flux(elevation, tau)
                self.assertLessEqual(generated, 1+1e-14)
                self.assertGreaterEqual(escaped, 0)
                self.assertLessEqual(escaped, generated+1e-10)
                downward = ref.slab_downward_flux(elevation, tau)
                direct = 0 if math.isinf(tau) else math.exp(-tau/math.sin(math.radians(elevation)))
                self.assertLessEqual(escaped+downward, generated+1e-8)
                self.assertLessEqual(escaped+downward+direct, 1+1e-8)
            old = ref.slab_upward_flux(elevation, profile="old_unconditional")
            self.assertGreater(old, 0)
        self.assertGreater(ref.slab_upward_flux(5, profile="old_unconditional"), 2)

    def test_zero_depth_slab_is_clear_and_bottom_flux_converges(self):
        for elevation in (5, 15, 75):
            self.assertEqual(ref.slab_upward_flux(elevation, 0), 0)
            self.assertEqual(ref.slab_downward_flux(elevation, 0), 0)
            a = ref.slab_downward_flux(elevation, 1, mu_order=64, phi_count=256)
            b = ref.slab_downward_flux(elevation, 1, mu_order=128, phi_count=512)
            self.assertAlmostEqual(a, b, delta=2e-6)

    def test_slab_flux_convergence_and_thin_directional_redistribution(self):
        for elevation in (5, 15, 75):
            a = ref.slab_upward_flux(elevation, profile="old_unconditional", mu_order=64, phi_count=256)
            b = ref.slab_upward_flux(elevation, profile="old_unconditional", mu_order=128, phi_count=512)
            self.assertAlmostEqual(a, b, delta=2e-6)
        self.assertAlmostEqual(sum(w*b for w,b in zip(ref.WEIGHTS, ref.BRANCH_B)), .82125)
        ratios = [sum(w*b*ref.phase_hg(mu, g) for w,b,g in zip(ref.WEIGHTS, ref.BRANCH_B, ref.LIQUID_G))/ref.phase_hg(mu, .86) for mu in (1, 0, -1)]
        self.assertLess(ratios[0], .71)
        self.assertGreater(ratios[1], 1.42)
        self.assertGreater(ratios[2], 1.68)

    def test_unsupported_slab_domain(self):
        for elevation, tau in ((0, 1), (91, 1), (5, -1)):
            with self.assertRaises(ValueError):
                ref.slab_upward_flux(elevation, tau)
        with self.assertRaises(ValueError):
            ref.slab_parameters("unknown")

    def test_many_constant_layers_match_direct_depth(self):
        op = (1, 0)
        depth = 0
        for i in range(64):
            sigma = .002 if i % 2 else 0
            depth += sigma*25
            op = ref.compose(op, ref.beer_segment(sigma, 0, 25))
        self.assertAlmostEqual(op[0], math.exp(-depth), delta=2e-15)
        self.assertEqual(op[1], 0)

    def test_json_schema_is_finite_and_contains_comparison_families(self):
        cases = ref.oracle_cases()
        self.assertEqual(cases["schema"], ref.SCHEMA)
        encoded = json.dumps(cases, allow_nan=False)
        self.assertNotIn("NaN", encoded)
        for key in ("geometry_cases", "air_cases", "constant_tau_cases", "phase_cases", "slab_cases", "exit_cases"):
            self.assertTrue(cases[key])
        self.assertTrue(any(x["status"] == "unsupported" for x in cases["geometry_cases"]))


if __name__ == "__main__":
    unittest.main()
