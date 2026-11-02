"""Independent scalar compression checks; no source/GPU/quality-time certificate."""
import math
import random
import unittest

import deep_light_reference as ref


def dense_errors(original, retained, subdivisions=32):
    errors = [0.0, 0.0]
    for a, b in zip(original, original[1:]):
        for n in range(subdivisions + 1):
            f = n / subdivisions
            x = a[0] + (b[0] - a[0]) * f
            expected = a[1] + (b[1] - a[1]) * f
            actual = ref.interpolate_tau(retained, x)
            for j, k in enumerate(ref.EXPONENTS):
                errors[j] = max(errors[j], abs(math.exp(-k*expected) - math.exp(-k*actual)))
    return tuple(errors)


def fixtures():
    # Endpoints are explicitly float32: the later GPU consumes the same positions.
    shapes = {
        'clear': lambda x: 0,
        'slab': lambda x: max(0, min(x-24, 64)) * .06,
        'separated_thin_layers': lambda x: (.15*max(0, min(x-20, 1))
                                          + .25*max(0, min(x-100, 1))),
        'dense_tower': lambda x: max(0, min(x-10, 110)) * 2,
        'gradual': lambda x: 12*(x/128)**2,
        'opaque': lambda x: 1000*(x/128)**1.3,
        'grazing': lambda x: max(0, min(x-25, 64))*.06/math.sin(math.radians(.2)),
    }
    return {name: list(ref.float32_samples((i*.5, tau(i*.5)) for i in range(257)))
            for name, tau in shapes.items()}


class LocalAnalyticTests(unittest.TestCase):
    def check_local(self, points):
        local = ref.local_removal_error(*points)
        actual = dense_errors(points, (points[0], points[2]), 4096)
        for j in range(2):
            self.assertLessEqual(actual[j], local.analytic[j]+local.numeric_guard[j])
        self.assertLessEqual(local.locations, 6)
        return local

    def test_equal_zero_and_one_zero_slope(self):
        for points in [((0, 0), (1, 0), (2, 0)), ((0, 0), (1, 2), (2, 4)),
                       ((0, 0), (1, 0), (2, 4)), ((0, 0), (1, 4), (2, 4)),
                       ((0, 700), (1, 701), (2, 701)),
                       ((0, 1e-12), (1e-9, 1.000001e-12), (2e-9, 1.0000021e-12))]:
            self.check_local(points)
        equal = self.check_local(((0, 0), (1, 2), (2, 4)))
        self.assertEqual(equal.analytic, (0, 0))
        self.assertEqual(equal.extrema, 0)

    def test_nonendpoint_extrema_are_required(self):
        points = ((0, 0), (1, 20), (2, 21))
        local = self.check_local(points)
        mid_error = abs(math.exp(-20)-math.exp(-10.5))
        self.assertGreater(local.analytic[0], mid_error*100)
        self.assertGreater(local.extrema, 0)

    def test_near_equal_slopes_and_random_triples(self):
        rng = random.Random(1729)
        for _ in range(120):
            x = 10**rng.uniform(-6, 6)
            b = rng.uniform(.01, .99)
            base = rng.uniform(0, 15)
            d, e = 10**rng.uniform(-9, 2), 10**rng.uniform(-9, 2)
            self.check_local(((0, base), (x*b, base+d), (x, base+d+e)))
        self.check_local(((0, 1), (1, 2), (2, 3+1e-13)))

    def test_duplicate_decreasing_and_nonfinite_rejected(self):
        for points in [((0, 0), (0, 1), (2, 2)), ((0, 0), (1, 2), (2, 1)),
                       ((0, 0), (1, -1), (2, 1)), ((0, 0), (1, math.inf), (2, 2)),
                       ((-1e100, 0), (0, 1), (1e-100, 2))]:
            with self.assertRaises(ValueError):
                ref.local_removal_error(*points)


class StreamingTests(unittest.TestCase):
    def test_named_fixtures_dense_actual_errors_under_accumulated_bounds(self):
        self.records = {}
        for name, points in fixtures().items():
            with self.subTest(name=name):
                result = ref.compress_tau_samples(iter(points))
                actual = dense_errors(points, result.knots)
                self.assertLessEqual(len(result.knots), 48)
                self.assertEqual(result.knots[0], points[0])
                self.assertEqual(result.knots[-1], points[-1])
                self.assertTrue(all(a[0] < b[0] and a[1] <= b[1]
                                    for a, b in zip(result.knots, result.knots[1:])))
                self.assertTrue(all(p in points for p in result.knots))
                for j in range(2):
                    self.assertLessEqual(actual[j], result.bounds[j])
                self.assertLessEqual(result.work['peak_knots'], 49)
                self.assertEqual(result.work['removals'], len(points)-48)
                self.assertEqual(result.work['candidate_evaluations'], 47*(len(points)-48))
                self.assertLessEqual(result.work['transformed_locations'],
                                     6*result.work['candidate_evaluations'])
                self.assertEqual(result.valid, max(result.bounds) <= .005)
                self.records[name] = dict(valid=result.valid, actual=actual, bounds=result.bounds)
        self.assertTrue(ref.compress_tau_samples(fixtures()['clear']).valid)
        self.assertTrue(ref.compress_tau_samples(fixtures()['separated_thin_layers']).valid)
        gradual = ref.compress_tau_samples(fixtures()['gradual'])
        self.assertFalse(gradual.valid, 'selected K48 must report this known smooth-curve failure')
        self.assertGreater(dense_errors(fixtures()['gradual'], gradual.knots)[1], .005)

    def test_initial_gpu_513_float32_endpoints(self):
        points = list(ref.float32_samples((i*.25, 12*(i/512)**2) for i in range(513)))
        result = ref.compress_tau_samples(iter(points))
        actual = dense_errors(points, result.knots, 64)
        self.assertFalse(result.valid)
        self.assertEqual(result.work['removals'], 465)
        self.assertEqual(result.work['candidate_evaluations'], 21855)
        self.assertLessEqual(result.work['transformed_locations'], 131130)
        self.assertTrue(all(a <= b for a, b in zip(actual, result.bounds)))
        self.assertGreaterEqual(result.accumulated_numeric_guard[0], .00093)

    def test_accumulated_error_never_resets_after_append_or_removal(self):
        points = [(i, .08*i + (.03 if i % 2 else 0)) for i in range(70)]
        previous = (0, 0)
        for end in range(3, len(points)+1):
            result = ref.compress_tau_samples(points[:end], max_knots=3)
            self.assertTrue(all(a <= b for a, b in zip(previous, result.accumulated_analytic)))
            previous = result.accumulated_analytic
            actual = dense_errors(points[:end], result.knots, 8)
            self.assertTrue(all(a <= b for a, b in zip(actual, result.bounds)))
        last = ref.local_removal_error(*points[-3:])
        self.assertGreater(result.accumulated_analytic[0], last.analytic[0]*2)

    def test_reduced_depth_not_black_primary_controls_admission(self):
        points = [(0, 8), (1, 8), (2, 16)]
        result = ref.compress_tau_samples(points, max_knots=2)
        self.assertLess(result.bounds[0], .005)
        self.assertGreater(result.bounds[1], .005)
        self.assertFalse(result.valid)
        self.assertEqual(result.reason, 'compression_allocation_exceeded')
        # The blocker is retained in the endpoint tau even when fit is invalid.
        self.assertEqual(result.knots[-1], points[-1])
        with self.assertRaisesRegex(ValueError, 'unavailable'):
            ref.curve_terms(result, 1)

    def test_capacity_invalid_never_silently_returns_clear(self):
        points = [(i, i*.01) for i in range(2049)]
        result = ref.compress_tau_samples(iter(points))
        self.assertFalse(result.valid)
        self.assertEqual(result.reason, 'endpoint_capacity_exceeded')
        self.assertEqual(result.work['input_samples'], 2049)
        self.assertEqual(result.work['appended_samples'], 2048)
        self.assertLessEqual(result.work['peak_knots'], 49)
        self.assertGreaterEqual(result.accumulated_numeric_guard[0], 2000*2e-6)
        with self.assertRaises(ValueError):
            ref.curve_terms(result, 0)

    def test_float32_depth_collision_and_invalid_inputs_visible(self):
        invalid = [[], [(0, 0)], [(0, 0), (0, 1)], [(1, 0), (0, 1)],
                   [(0, 1), (1, 0)], [(0, 0), (1, -1)],
                   [(0, 0), (1, math.nan)], [(0, 0), (math.inf, 1)],
                   [(0, 0), (1, 2, 3)]]
        for points in invalid:
            self.assertFalse(ref.compress_tau_samples(points).valid)
        collided = ref.compress_tau_samples(ref.float32_samples([(1e8, 0), (1e8+1, 1)]))
        self.assertFalse(collided.valid)
        self.assertIn('duplicates', collided.reason)
        for kwargs in [dict(max_knots=1), dict(max_knots=49), dict(max_samples=2049),
                       dict(allocation=.006), dict(allocation=0)]:
            with self.assertRaises(ValueError):
                ref.compress_tau_samples([(0, 0), (1, 1)], **kwargs)

    def test_translated_and_stretched_depths_preserve_same_light_curve(self):
        points = fixtures()['separated_thin_layers']
        translated = [(x*1e4+8e6, tau) for x, tau in points]
        a, b = ref.compress_tau_samples(points), ref.compress_tau_samples(translated)
        self.assertEqual([tau for _, tau in a.knots], [tau for _, tau in b.knots])
        for x in (0, 20.5, 65, 100.5, 128):
            self.assertEqual(ref.curve_terms(a, x), ref.curve_terms(b, x*1e4+8e6))
        with self.assertRaises(ValueError):
            ref.curve_terms(a, -1)


class TauTubeTests(unittest.TestCase):
    def test_delta_guarantees_both_terms_including_opaque(self):
        epsilon = .005-ref.NUMERICAL_GUARD_PER_REMOVAL
        for anchor in (0, 1, 5, 20, 22, 100, 1000):
            delta = ref.tube_delta(anchor, epsilon)
            for k in ref.EXPONENTS:
                base = math.exp(-k*anchor)
                error = base if math.isinf(delta) else base*-math.expm1(-k*delta)
                self.assertLessEqual(error, epsilon+1e-15)
        self.assertTrue(math.isinf(ref.tube_delta(1000, epsilon)))

    def test_named_dense_curves_and_original_negative_now_admit(self):
        for name, points in fixtures().items():
            with self.subTest(name=name):
                result = ref.compress_tau_tube(iter(points))
                self.assertTrue(result.valid, result.reason)
                self.assertEqual(result.method, 'tau_tube')
                self.assertEqual(result.knots[0], points[0])
                self.assertEqual(result.knots[-1], points[-1])
                self.assertTrue(all(p in points for p in result.knots))
                self.assertTrue(all(a[0] < b[0] and a[1] <= b[1]
                                    for a, b in zip(result.knots, result.knots[1:])))
                actual = dense_errors(points, result.knots, 64)
                self.assertTrue(all(a <= b for a, b in zip(actual, result.bounds)))
                self.assertLessEqual(result.work['endpoint_visits'], 2*(len(points)-1))
                self.assertEqual(result.work['emitted_pieces'], len(result.knots)-1)
                self.assertEqual(result.accumulated_numeric_guard, (2e-6, 2e-6))
        points = fixtures()['gradual']
        self.assertFalse(ref.compress_tau_samples(points).valid, 'retain old failure')
        self.assertEqual(len(ref.compress_tau_tube(points).knots), 14)
        points513 = list(ref.float32_samples((i*.25, 12*(i/512)**2) for i in range(513)))
        result = ref.compress_tau_tube(points513)
        self.assertTrue(result.valid)
        self.assertEqual(len(result.knots), 14)
        self.assertTrue(all(a <= b for a, b in zip(dense_errors(points513, result.knots, 64), result.bounds)))

    def test_rejected_endpoint_restarts_at_previous_actual_sample(self):
        points = [(0, 0), (1, 0), (2, .1), (3, .1)]
        result = ref.compress_tau_tube(points)
        self.assertEqual(result.knots, tuple(points))
        self.assertEqual(dense_errors(points, result.knots), (0, 0))
        # An arbitrary feasible slope must not replace the source endpoint tau.
        self.assertEqual(result.knots[1], (1, 0))
        self.assertEqual(ref.compress_tau_tube([(0, 0), (10, .1)]).knots,
                         ((0, 0), (10, .1)))

    def test_many_thin_blockers_overflow_visible_without_truncation_admission(self):
        points = [(i, (i//2)*.02) for i in range(1000)]
        result = ref.compress_tau_tube(iter(points))
        self.assertFalse(result.valid)
        self.assertEqual(result.reason, 'knot_capacity_exceeded')
        self.assertEqual(len(result.knots), 48)
        self.assertLess(result.knots[-1][0], points[-1][0], 'invalid prefix is incomplete')
        self.assertLessEqual(result.work['endpoint_visits'], 2*result.work['input_samples'])
        with self.assertRaises(ValueError):
            ref.curve_terms(result, 10)

    def test_random_monotone_and_grazing_depth_scaling(self):
        rng = random.Random(441)
        for _ in range(50):
            points, tau = [], 0.0
            for i in range(100):
                tau += rng.uniform(0, .006)
                points.append((i*10000+1e7, tau))
            result = ref.compress_tau_tube(points)
            self.assertTrue(result.valid)
            self.assertTrue(all(a <= b for a, b in zip(dense_errors(points, result.knots, 16), result.bounds)))

    def test_bad_samples_and_endpoint_capacity_are_not_clear(self):
        for points in ([], [(0, 0)], [(0, 0), (0, .1)], [(0, 1), (1, 0)],
                       [(0, 0), (1, math.nan)]):
            self.assertFalse(ref.compress_tau_tube(points).valid)
        result = ref.compress_tau_tube((i, .001*i) for i in range(2049))
        self.assertFalse(result.valid)
        self.assertEqual(result.reason, 'endpoint_capacity_exceeded')
        with self.assertRaises(ValueError):
            ref.curve_terms(result, 0)
        with self.assertRaises(ValueError):
            ref.compress_tau_tube([(0, 0), (1, 1)], allocation=1e-6)


if __name__ == '__main__':
    unittest.main()
