"""Physical invariants and deliberately plausible-wrong transport comparators."""
import gzip
import importlib.util
from pathlib import Path
import tempfile
import unittest

import numpy as np


spec = importlib.util.spec_from_file_location('optical_oracle', Path(__file__).with_name('oracle.py'))
oracle = importlib.util.module_from_spec(spec)
import sys
sys.modules[spec.name] = oracle
spec.loader.exec_module(oracle)


class OpticalInvariants(unittest.TestCase):
    def test_segment_splitting_and_affine_composition(self):
        sigma, source = np.array([0, .03, .4]), np.array([2, 3, 1])
        whole = oracle.segment(sigma, source, 17)
        split = oracle.compose(oracle.segment(sigma, source, 4), oracle.segment(sigma, source, 13))
        np.testing.assert_allclose(split, whole, rtol=1e-14)
        np.testing.assert_allclose(whole[1][0], 34)
        a, b, c = [oracle.segment(s, j, h) for s, j, h in [(.1, 4, 2), (.2, 1, 3), (.03, 2, 8)]]
        np.testing.assert_allclose(oracle.compose(oracle.compose(a, b), c),
                                   oracle.compose(a, oracle.compose(b, c)), rtol=1e-14)

    def test_overlapping_cloud_air_fog_sum_coefficients_before_solution(self):
        # Homogeneous overlap has one exact source function, independent of feature order.
        sigma = np.array([[.2, .2, .2], [.03, .06, .1], [.4, .3, .2]])
        source = np.array([[4, 3, 2], [.1, .3, .8], [1, 2, 3]])
        exact = oracle.segment(sigma.sum(axis=0), source.sum(axis=0), 9)
        t = np.exp(-sigma.sum(axis=0)*9)
        np.testing.assert_allclose(exact[0], t)
        np.testing.assert_allclose(exact[1], (source.sum(axis=0)/sigma.sum(axis=0))*(1-t))
        wrong = oracle.compose(oracle.compose(oracle.segment(sigma[0], source[0], 9),
                                             oracle.segment(sigma[1], source[1], 9)),
                               oracle.segment(sigma[2], source[2], 9))
        self.assertGreater(float(np.max(abs(exact[1]-wrong[1]))), 1)
        np.testing.assert_allclose(exact[0], wrong[0])  # T alone cannot detect the bug.

    def test_trilinear_axes_and_world_translation(self):
        z, y, x = np.mgrid[0:8, 0:8, 0:8]
        field = oracle.Field(1+2*x+3*y+5*z, [12, 27, -4], 8)
        local = np.array([[2.2, 3.3, 4.4], [4.7, 1.2, 2.6]])
        np.testing.assert_allclose(field.sample(local+field.lower+.5), 1+local@[2, 3, 5])
        moved = oracle.Field(field.values, field.lower+[8, -3, 1], 8)
        np.testing.assert_allclose(moved.sample(local+moved.lower+.5), field.sample(local+field.lower+.5))
        self.assertEqual(field.sample([11.99, 29, 0]), 0)

    def test_finite_arbitrary_ray_box_and_inside_receiver_sun_paths(self):
        field = oracle.Field(np.full((8, 8, 8), .125), [0, 0, 0], 8)
        self.assertEqual(oracle.ray_box([-1, 9, 4], [1, 0, 0], [0, 0, 0], [8, 8, 8]), None)
        self.assertEqual(field.tau([-1, 4, 4], [1, 0, 0], .5), 0)
        self.assertAlmostEqual(field.tau([-3, 4, 4], [1, 0, 0]), 1)
        self.assertAlmostEqual(field.tau([4, 4, 4], [0, 1, 0]), .5)
        direction = oracle.unit([1, 2, -1])
        end = 4/direction[1]
        self.assertAlmostEqual(field.tau([4, 4, 4], direction), .125*end)
        self.assertAlmostEqual(field.tau([4, 4, 4], direction, 1.7), .125*1.7)
        # A finite light segment cannot be replaced by the full receiver shadow.
        self.assertGreater(field.tau([4, 4, 4], direction), field.tau([4, 4, 4], direction, 1.7))

    def test_refinement_converges_on_curved_density(self):
        z, y, x = np.mgrid[0:32, 0:32, 0:32]
        values = .05*np.exp(-((x-17)**2+(y-13)**2+(z-15)**2)/45)
        field = oracle.Field(values, [-16, 5, -16], 32)
        query = ([-20, 12, -7], [1, .19, .13])
        truth = field.tau(*query, step=.03125)
        coarse = field.tau(*query, step=1)
        fine = field.tau(*query, step=.125)
        self.assertLess(abs(fine-truth), abs(coarse-truth))
        self.assertLess(abs(fine-truth), 1e-5)

    def test_hg_normalization_and_forward_direction(self):
        mu = np.linspace(-1, 1, 200001)
        value = oracle.hg(mu, .866617162492502)
        self.assertAlmostEqual(np.trapezoid(value, mu)*2*np.pi, 1, places=6)
        self.assertGreater(oracle.hg(1, .8), oracle.hg(-1, .8))
        self.assertAlmostEqual(oracle.hg(.5, 0), 1/(4*np.pi))

    def test_closure_source_off_linear_bound_and_authoritative_ambient(self):
        sigma, tau = .2, [3, 2, 1]
        source = oracle.cloud_source(sigma, .7, [100, 80, 60], tau, ambient=[1, 2, 3])
        no_ambient = oracle.cloud_source(sigma, .7, [100, 80, 60], tau)
        np.testing.assert_allclose(source-no_ambient, sigma*np.array([1, 2, 3]))
        np.testing.assert_array_equal(oracle.cloud_source(sigma, .7, [0, 0, 0], tau), [0, 0, 0])
        np.testing.assert_allclose(oracle.cloud_source(sigma, .7, [200, 160, 120], tau), 2*no_ambient)
        # Setting a=0 isolates exactly one single-scatter term; b never alters camera extinction.
        direct = sigma*100*np.exp(-tau[0])*oracle.hg(.7, .866617162492502)
        self.assertAlmostEqual(oracle.cloud_source(sigma, .7, 100, tau, a=0), direct)
        bound = sigma*100*sum(.8**n*oracle.hg(1, .866617162492502*.5**n) for n in range(4))
        self.assertLess(oracle.cloud_source(sigma, .7, 100, tau), bound)

    def test_mirror_ray_equals_transformed_camera_and_foreground_once(self):
        origin, direction = np.array([1., 4., -3.]), oracle.unit([.2, -1, .6])
        hit = oracle.mirror_hit(origin, direction)
        camera = oracle.reflect_point(origin)
        reflected = oracle.reflect_direction(direction)
        np.testing.assert_allclose(oracle.mirror_hit(camera, reflected), hit, atol=1e-14)
        terminal = hit+reflected*13
        np.testing.assert_allclose(oracle.unit(terminal-camera), reflected, atol=1e-14)
        foreground = oracle.segment(.1, .3, np.linalg.norm(hit-origin))
        background = oracle.segment(.07, .8, 13)
        t, s = oracle.compose(foreground, background)
        radiance = s+t*2
        direct = foreground[1]+foreground[0]*(background[1]+background[0]*2)
        self.assertAlmostEqual(radiance, direct)
        self.assertGreater(abs(foreground[1]+foreground[0]*radiance-radiance), .01)

    def test_raw_and_gzip_density_inputs_have_same_values(self):
        with tempfile.TemporaryDirectory() as directory:
            raw, compressed = Path(directory)/'a.f16', Path(directory)/'a.f16.gz'
            values = np.arange(64, dtype='<f2').reshape(4, 4, 4)/100
            raw.write_bytes(values.tobytes())
            compressed.write_bytes(gzip.compress(values.tobytes()))
            np.testing.assert_array_equal(oracle.read_density(raw, 4), oracle.read_density(compressed, 4))

    def test_analytic_air_tau_and_rayleigh_normalization(self):
        origin, direction, length = [0, 300, 0], oracle.unit([1, .3, 0]), 4000
        distances = (np.arange(40000)+.5)*length/40000
        numerical = np.mean(np.exp(-(origin[1]+direction[1]*distances)/8000))*length
        np.testing.assert_allclose(oracle.air_tau(origin, direction, length, [1, 2, 3]),
                                   numerical*np.array([1, 2, 3]), rtol=1e-11)
        mu = np.linspace(-1, 1, 100001)
        self.assertAlmostEqual(np.trapezoid(3*(1+mu*mu)/(16*np.pi), mu)*2*np.pi, 1, places=9)

    def test_joint_fog_overlap_has_one_common_light_solution(self):
        field = oracle.Field(np.full((8, 8, 8), .02), [-4, 0, -4], 8)
        fog = {'centre': [0, 4, 0], 'radius': 2, 'sigma': .3, 'albedo': .9, 'g': .6}
        scene = {'sun_direction': [0, 1, 0], 'sun_rgb': [100, 90, 80],
                 'opaque_spheres': [], 'air_sigma0': [0, 0, 0], 'fog_spheres': [fog]}
        whole = oracle.trace_joint(field, [0, 4, -3], [0, 0, 1], 6, step=.5,
                                   light_step=.5, scene=scene, ambient=[2, 3, 4])
        # Co-located overlapping constituents must be equivalent to their sum on
        # both the camera path and every incident light path.
        split = scene | {'fog_spheres': [fog | {'sigma': .1}, fog | {'sigma': .2}]}
        divided = oracle.trace_joint(field, [0, 4, -3], [0, 0, 1], 6, step=.5,
                                     light_step=.5, scene=split, ambient=[2, 3, 4])
        np.testing.assert_allclose(divided['S'], whole['S'], rtol=2e-14)
        np.testing.assert_allclose(divided['T'], whole['T'], rtol=2e-14)
        np.testing.assert_allclose(whole['T'], np.exp(-(.02*6+.3*4)))
        dark = oracle.trace_joint(field, [0, 4, -3], [0, 0, 1], 6, scene=scene | {'sun_rgb': [0, 0, 0]},
                                  ambient=[0, 0, 0], terminal=[100, 80, 60])
        np.testing.assert_array_equal(dark['S'], [0, 0, 0])
        np.testing.assert_allclose(dark['radiance'], np.array(dark['T'])*[100, 80, 60])
        env = oracle.trace_joint(field, [0, 4, -3], [0, 0, 1], 6,
                                 scene=scene | {'sun_rgb': [0, 0, 0]}, ambient=[2, 3, 4])
        self.assertTrue(np.all(np.array(env['S']) > 0))

    def test_mirror_joint_near_far_preserves_foreground_exactly_once(self):
        field = oracle.Field(np.full((8, 8, 8), .02), [-4, 0, -4], 8)
        scene = {'sun_rgb': [0, 0, 0], 'air_sigma0': [0, 0, 0], 'opaque_spheres': [],
                 'fog_spheres': [{'centre': [0, 1, 0], 'radius': 2, 'sigma': .03,
                                  'albedo': .9, 'g': .6}]}
        origin, direction = np.array([0., 3., -3.]), oracle.unit([0, -1, 1])
        hit = oracle.mirror_hit(origin, direction)
        reflected = oracle.reflect_direction(direction)
        near_distance = np.linalg.norm(hit-origin)
        near = oracle.trace_joint(field, origin, direction, near_distance, step=.4, scene=scene)
        far = oracle.trace_joint(field, hit, reflected, 4, step=.4, scene=scene, terminal=[50, 60, 70])
        t, s = oracle.compose((np.array(near['T']), np.array(near['S'])),
                              (np.array(far['T']), np.array(far['S'])))
        fog = scene['fog_spheres'][0]
        lengths = [oracle.sphere_interval(p, d, fog['centre'], fog['radius'], length)
                   for p, d, length in [(origin, direction, near_distance), (hit, reflected, 4)]]
        expected = np.exp(-.02*(near_distance+4)-.03*sum(b-a for a, b in lengths))
        np.testing.assert_allclose(t, expected, rtol=1e-14)
        np.testing.assert_allclose(s+t*[50, 60, 70], np.array(near['S'])+np.array(near['T'])*far['radiance'])

    def test_ambient_horizon_and_geometry_source_visibility(self):
        field = oracle.Field(np.zeros((8, 8, 8)), [-4, 0, -4], 8)
        scene = {'air_sigma0': [0, 0, 0], 'fog_spheres': [], 'sky_rgb': [600, 1200, 1800]}
        np.testing.assert_allclose(oracle.ambient_reference(field, [0, 4, 0], scene), [500, 1000, 1500])
        scene = oracle.default_scene() | {'opaque_spheres': [{'centre': [0, 10, 0], 'radius': 1}]}
        self.assertEqual(oracle.geometry_visibility([0, 4, 0], [0, 1, 0], scene), 0)
        self.assertEqual(oracle.geometry_visibility([0, 4, 0], [0, -1, 0], scene), 1)


if __name__ == '__main__':
    unittest.main()
