"""Native window evidence refuses false submission/input/clock joins."""
import copy
import unittest
from package_mac_m2_window import joins


class WindowEvidenceTests(unittest.TestCase):
    def setUp(self):
        self.surface = [{'submission_id': '4', 'sequence': '12', 'lease_generation': '1', 'presented': 'true'}]
        self.requests = [{'sequence': '12', 'accepted_yaw_degrees': '0'}]
        self.images = [{'submission_id': '4', 'submit_origin_ns': '100', 'tick': '0', 'device_generation': '1', 'presenter_generation': '1'}]

    def test_image_is_bound_to_actual_surface_submission_and_normal_request(self):
        records, missing = joins(self.surface, self.requests, self.images, ['hash'])
        self.assertEqual(missing, [])
        self.assertEqual(records[0]['sequence'], 12)
        self.assertEqual(records[0]['accepted_yaw_degrees'], 0)
        self.assertEqual(records[0]['lease_generation'], 1)

    def test_occlusion_or_unmatched_image_or_input_cannot_pass(self):
        for change in ('occluded', 'unmatched', 'input', 'hash'):
            surface, requests, images, hashes = copy.deepcopy((self.surface, self.requests, self.images, ['hash']))
            if change == 'occluded': surface[0]['presented'] = 'false'
            if change == 'unmatched': images[0]['submission_id'] = '5'
            if change == 'input': requests.clear()
            if change == 'hash': hashes.clear()
            with self.assertRaises(ValueError):
                joins(surface, requests, images, hashes)

    def test_surface_retirement_remains_an_explicit_missing_image(self):
        surface = self.surface + [dict(self.surface[0], submission_id='5', sequence='13', lease_generation='2')]
        requests = self.requests + [dict(self.requests[0], sequence='13')]
        records, missing = joins(surface, requests, self.images, ['hash'])
        self.assertEqual(missing, [5])
        self.assertEqual(len(records), 1)

    def test_source_order_cannot_be_rewritten_as_movie_time(self):
        surface = self.surface + [dict(self.surface[0], submission_id='5', sequence='13')]
        requests = self.requests + [dict(self.requests[0], sequence='13')]
        images = self.images + [dict(self.images[0], submission_id='5', submit_origin_ns='99')]
        with self.assertRaisesRegex(ValueError, 'nonmonotonic'):
            joins(surface, requests, images, ['one', 'two'])


if __name__ == '__main__':
    unittest.main()
