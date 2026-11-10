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

class PermissionEvidenceTests(unittest.TestCase):
    def test_console_witness_does_not_affect_denied_permission_proof(self):
        from capture_v1_runtime import permission_proof_valid
        observed = {'denied': True, 'error': None, 'api': 'CGPreflightScreenCaptureAccess', 'probe_pid': 23}
        for before, after in [(False, False), (True, True), (True, False), (False, True), (None, None)]:
            manifest = {'permission_before': observed, 'permission_after': observed,
                        'console_locked': before, 'console_locked_after_capture': after}
            self.assertTrue(permission_proof_valid(manifest))
        for field in ['permission_before', 'permission_after']:
            for invalid in [{}, dict(observed, denied=False), dict(observed, denied=None),
                            dict(observed, error='probe failed'), dict(observed, probe_pid=None)]:
                manifest = {'permission_before': observed, 'permission_after': observed}
                manifest[field] = invalid
                self.assertFalse(permission_proof_valid(manifest))

    def test_permission_observations_must_belong_to_the_actual_driver_and_parent(self):
        from capture_v1_runtime import validate_permission_attribution
        observed = {'denied': True, 'error': None, 'api': 'CGPreflightScreenCaptureAccess', 'probe_pid': 23}
        launcher = {'capture_exit_code': 0, 'screen_recording_before': False, 'screen_recording_after': False,
                    'bundle_identifier': 'our-app', 'preflight_api': 'CGPreflightScreenCaptureAccess',
                    'launcher_executable': '/our-app/Launcher', 'launcher_pid': 10, 'capture_driver_pid': 11}
        manifest = {'permission_before': observed, 'permission_after': observed,
                    'driver_identity': {'pid': 11, 'parent_pid': 10, 'executable': '/python',
                                        'responsible_bundle_identifier': 'our-app'}}
        validate_permission_attribution(launcher, manifest, 'our-app')
        for change in [{'screen_recording_after': True}, {'launcher_pid': 99},
                       {'capture_driver_pid': 99}, {'capture_exit_code': False},
                       {'bundle_identifier': 'other-app'}, {'launcher_executable': ''}]:
            with self.subTest(change=change), self.assertRaises(ValueError):
                validate_permission_attribution(dict(launcher, **change), manifest, 'our-app')


if __name__ == '__main__':
    unittest.main()
